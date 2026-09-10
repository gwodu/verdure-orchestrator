//! Local chat LLM via llama.cpp (the `llama-cpp-2` crate) — the conversational
//! front door (knowledge/18, 20). The model converses and proposes a JSON tool
//! decision (answer, or invoke a capability); the deterministic classifier in the
//! UI (`classifyIntent`) is the real router, so the model's JSON is a hint, not a
//! contract. Runs on the CPU, local.
//!
//! NOTE — no GBNF grammar sampler. We previously grammar-locked the JSON, but
//! llama.cpp's grammar code (`llama_grammar_reject_candidates`) calls
//! `ggml_abort()` when it reaches a state with no valid continuation (e.g. a
//! multi-byte token the grammar can't extend) — and an abort kills the WHOLE app
//! (observed on-device 2026-06-23: chat crashed there, NOT on memory). Plain
//! sampling + lenient parsing + the UI cross-check is safer and simpler. A grammar
//! can come back only once the model runs in an isolated process (knowledge/21).

use std::num::NonZeroU32;
use std::path::Path;
use std::sync::OnceLock;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;

use crate::model::{ModelInfo, PromptStyle};

// Kept modest on purpose: ggml aborts the whole process on a failed allocation,
// and the floor target is an 8 GB machine where the WebView already uses ~1.5 GB.
// 1024 ctx + a 512 batch keeps the KV + compute buffers small — lowered from 2048
// to shrink the resident footprint of the 4B base model (`BYTESHAPE_QWEN3_4B`) so
// the RAM gate can actually pick it on the 8 GB Mac; the model's `runtime_mb` gate
// estimate is set to match. Chat prompts + replies are short, so 1024 is ample. If
// conversational memory (`[OQ-59]`) needs a longer window later, raise both together.
const N_BATCH: u32 = 512;
/// Never run below this: a window too small to hold the instruction plus one
/// exchange isn't a degraded chat, it's a broken one — the RAM gate declines
/// honestly instead (knowledge/22).
const MIN_CONTEXT: u32 = 512;
/// Cap the window by TIME, not just memory. Prefill is linear in tokens and runs
/// on the CPU, so a roomy device that could afford 32k tokens of KV cache would
/// still make the user wait minutes for the first reply. 4096 keeps a long
/// conversation responsive on the floor machine (knowledge/22).
const MAX_CONTEXT: u32 = 4096;
/// Headroom left for the OS + WebView when sizing the window, MB. The 8 GB Mac
/// runs the WebView alongside us; overshooting here is what ggml-aborts the app.
const CONTEXT_HEADROOM_MB: u64 = 700;
/// Slack on top of the reply budget and the (measured) system instruction, to
/// cover turn markers and tokenizer variance. The instruction itself is no longer
/// guessed at — it is tokenized at call time, so growing it can't silently start
/// crowding out history (it grew from ~30 to ~90 tokens on 2026-08-14).
const RESERVE_SLACK_TOKENS: usize = 48;
/// Per-turn template overhead (`<|im_start|>user … <|im_end|>` ×2 and friends).
/// Deliberately generous — undercounting overflows the window, overcounting only
/// costs one turn of history.
const TURN_MARKUP_TOKENS: usize = 12;
// A concise chat reply is short; capping the budget keeps a worst-case generation
// snappy on the CPU floor machine. EOG normally stops us well before this.
const MAX_NEW_TOKENS: i32 = 256;
// Fixed seed → the same message gives the same reply, which is reproducible and
// easy to reason about. The sampler is lightly stochastic (temp), so this only
// pins which of several good continuations we take.
const SEED: u32 = 0x5EED;

/// The model's reply. `action` is always "answer" now — chat is a plain
/// conversation. (The JSON tool-calling wrapper was removed: on-device 2026-07-27
/// the tiny model over-triggered the `set_up_transcription` action on ordinary
/// questions AND leaked malformed JSON into the chat. Dictation is the mic button;
/// capabilities-as-tools returns later, likely constrained + process-isolated.
/// [OQ-55].) The struct is kept so the frontend contract is unchanged.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDecision {
    pub action: String,
    pub reply: String,
}

/// Plain conversational instruction — no JSON, no tools. Same across families;
/// only the turn framing differs (knowledge/03).
/// The instruction, and the one lever we have against confabulation.
///
/// A beta tester reported "it keeps giving fake facts" (2026-08-14). That is the
/// characteristic failure of this model tier, not a bug we can patch out — but the
/// prompt was making it worse by asking only for answers that are "direct, clear
/// and concise", which reads as *always produce a confident answer*. Small models
/// comply, and invent one when they don't know.
///
/// So it licenses uncertainty — but BRIEFLY. The first attempt at this (2026-08-14)
/// spelled the idea out in five clauses and made replies measurably worse on the
/// capable 4B, not just the tiny models (founder report from the iPad, 2026-08-16):
/// every instruction token competes for attention with the user's actual question,
/// and these models follow a short rule better than a careful one. **Keep this
/// short.** If it needs to grow, measure reply quality before and after — the
/// reserve arithmetic in `decide` already tracks its token cost automatically.
const SYSTEM: &str = "You are Verdure, a helpful personal AI running entirely on the user's own device. \
Answer directly and concisely. \
If you don't know something, say so instead of guessing.";

/// One completed exchange. The conversation the user can see IS the memory —
/// the frontend owns the transcript and passes back the part that still fits
/// (knowledge/22).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Turn {
    pub user: String,
    pub assistant: String,
}

/// How large a context this device can afford for this model — pure arithmetic
/// over the KV-cache cost (knowledge/22, "Sizing the budget").
///
/// `spare_mb` is what's left after the weights and OS headroom; `kv_kb_per_token`
/// is the model's own per-token KV cost. Clamped to `[MIN_CONTEXT, MAX_CONTEXT]`
/// and never above what the model was trained for — a longer window than training
/// produces confident nonsense, not more memory.
pub fn context_for_budget(spare_mb: u64, kv_kb_per_token: u32, trained_ctx: u32) -> u32 {
    let per_token = kv_kb_per_token.max(1) as u64;
    let affordable = spare_mb.saturating_mul(1024) / per_token;
    let ceiling = trained_ctx.min(MAX_CONTEXT);
    (affordable.min(ceiling as u64) as u32).max(MIN_CONTEXT).min(ceiling.max(MIN_CONTEXT))
}

/// Memory left for the KV cache once the weights and OS headroom are accounted
/// for. Saturating: a machine tighter than the headroom yields 0, and the caller
/// then runs at `MIN_CONTEXT` or the RAM gate declines outright.
pub fn spare_for_context_mb(available_mb: u64, weights_mb: u32) -> u64 {
    available_mb
        .saturating_sub(weights_mb as u64)
        .saturating_sub(CONTEXT_HEADROOM_MB)
}

/// How many prompt tokens can be decoded while still leaving room to answer.
/// Pure, so the clamp that prevents "ran out of context" is unit-tested.
pub(crate) fn max_prompt_tokens(n_ctx: u32) -> usize {
    // Leave the reply budget plus a couple of tokens of slack; never return 0,
    // or a prompt would be clamped out of existence.
    (n_ctx as usize).saturating_sub(MAX_NEW_TOKENS as usize + 8).max(16)
}

/// Which of the most recent turns fit — returns the index into `history` of the
/// first turn to KEEP (`history.len()` = keep none). Pure: the token cost
/// function is injected, so eviction is unit-tested without loading a model.
///
/// Walks newest→oldest and stops at the first turn that doesn't fit, rather than
/// skipping it and trying older ones: a conversation with a hole in the middle
/// reads as confusion to the model, and recency is what the user means by
/// "remember" (knowledge/22).
pub(crate) fn first_kept_turn(
    history: &[Turn],
    new_message: &str,
    n_ctx: u32,
    reserved: usize,
    cost: &dyn Fn(&str) -> usize,
) -> usize {
    let budget = n_ctx as usize;
    let mut used = reserved + cost(new_message);
    let mut first = history.len();
    for (i, turn) in history.iter().enumerate().rev() {
        let turn_cost = cost(&turn.user) + cost(&turn.assistant) + TURN_MARKUP_TOKENS;
        if used + turn_cost > budget {
            break;
        }
        used += turn_cost;
        first = i;
    }
    first
}

/// Frame the prompt for the model's family, with the retained history ahead of
/// the new message. Getting this wrong yields garbage, so the family travels with
/// the model (`PromptStyle`) rather than being assumed. Turns are always whole —
/// a dangling half-turn corrupts the structure the model relies on.
fn format_chat_prompt(history: &[Turn], user: &str, style: PromptStyle) -> String {
    let mut p = String::new();
    match style {
        // ChatML — Qwen et al. No BOS (the tokenizer adds none for this family).
        // The two ChatML arms differ only in what follows the final assistant
        // marker, so they share one body: a reasoning model gets a pre-filled
        // EMPTY thinking block, which is its documented "answer directly" switch
        // (see `PromptStyle::ChatMlNoThink`). Sharing the body means history
        // framing can never drift between them.
        PromptStyle::ChatMl | PromptStyle::ChatMlNoThink => {
            p.push_str(&format!("<|im_start|>system\n{SYSTEM}<|im_end|>\n"));
            for t in history {
                p.push_str(&format!(
                    "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>\n",
                    t.user, t.assistant
                ));
            }
            p.push_str(&format!(
                "<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
            ));
            if style == PromptStyle::ChatMlNoThink {
                p.push_str("<think>\n\n</think>\n\n");
            }
        }
        // Gemma has no system role, so the instruction rides the first user turn.
        // The leading BOS is added by the tokenizer (AddBos::Always), not written
        // here — writing a literal `<bos>` too would double it.
        // The instruction rides whichever turn comes first — the oldest retained
        // one when there's history, otherwise the new message itself.
        PromptStyle::Gemma => match history.split_first() {
            None => {
                p.push_str(&format!("<start_of_turn>user\n{SYSTEM}\n\n{user}<end_of_turn>\n"));
                p.push_str("<start_of_turn>model\n");
            }
            Some((first, rest)) => {
                p.push_str(&format!(
                    "<start_of_turn>user\n{SYSTEM}\n\n{}<end_of_turn>\n<start_of_turn>model\n{}<end_of_turn>\n",
                    first.user, first.assistant
                ));
                for t in rest {
                    p.push_str(&format!(
                        "<start_of_turn>user\n{}<end_of_turn>\n<start_of_turn>model\n{}<end_of_turn>\n",
                        t.user, t.assistant
                    ));
                }
                p.push_str(&format!("<start_of_turn>user\n{user}<end_of_turn>\n"));
                p.push_str("<start_of_turn>model\n");
            }
        },
    }
    p
}

/// BOS convention for the family — see `format_chat_prompt`.
fn add_bos_for(style: PromptStyle) -> AddBos {
    match style {
        PromptStyle::ChatMl | PromptStyle::ChatMlNoThink => AddBos::Never,
        PromptStyle::Gemma => AddBos::Always,
    }
}

/// The llama backend, initialised once per process. Previously re-initialised on
/// every message; it is a process-global by nature, and the resident model below
/// has to outlive any single call anyway.
static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

fn backend() -> Result<&'static LlamaBackend, String> {
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    let b = LlamaBackend::init().map_err(|e| format!("couldn't start the chat engine: {e}"))?;
    Ok(BACKEND.get_or_init(|| b))
}

/// How the model is loaded — specifically, whether any layers go to the GPU.
///
/// `LlamaModelParams::default()` sets `n_gpu_layers = -1`, meaning **offload
/// everything to the GPU**. On Apple platforms llama.cpp's GPU backend is Metal,
/// and it is compiled in unconditionally (`llama-cpp-sys-2`'s build script only
/// disables Metal for watchOS), so our `default-features = false` never made this
/// "plain CPU" the way `Cargo.toml` claimed for two months.
///
/// On Apple Silicon that is right and fast. **On an Intel Mac it produces
/// garbage** — the cause of `[OQ-71]`: every chat model returned token salad or a
/// single repeating token on the founder's 2019 Intel Mac, while the identical
/// model file answered correctly through upstream's prebuilt macOS **x64**
/// binary, which ships CPU-only. Metal's kernels target Apple Silicon; on an
/// Intel Mac's integrated GPU the arithmetic comes back wrong rather than
/// failing loudly, which is why this looked like a model problem for days.
///
/// So: keep the GPU everywhere it is trustworthy, and stay on the CPU on
/// x86_64 macOS. The reference build gets ~42 tok/s on that machine's CPU, so
/// this costs very little.
///
/// **`with_n_gpu_layers(0)` alone is not enough** — measured. It keeps the
/// *weights* off the GPU, but llama.cpp still registers Metal as a device and
/// schedules compute onto it (observed: a 436 MiB Metal compute buffer and 302
/// graph splits), and the output stayed wrong — merely a different flavour of
/// wrong. The device list is what actually decides the backend, so we pin the
/// model to the CPU device explicitly.
fn model_params() -> LlamaModelParams {
    let params = LlamaModelParams::default();

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let params = {
        let cpu: Vec<usize> = crate::llm::cpu_device_indices();
        // Belt and braces: no weight offload AND only the CPU device visible.
        let p = params.with_n_gpu_layers(0);
        match p.with_devices(&cpu) {
            Ok(p) => p,
            // If the device list is ever unavailable, a CPU-only weight
            // placement is still better than silently trusting Metal here.
            Err(_) => LlamaModelParams::default().with_n_gpu_layers(0),
        }
    };

    params
}

/// Indices of the CPU backend devices llama.cpp has registered.
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
fn cpu_device_indices() -> Vec<usize> {
    llama_cpp_2::list_llama_ggml_backend_devices()
        .into_iter()
        .filter(|d| d.backend.eq_ignore_ascii_case("CPU"))
        .map(|d| d.index)
        .collect()
}

/// A chat model kept loaded across messages (knowledge/22, part 2). Re-reading
/// 0.4–1.4 GB of weights from disk per message was the dominant cost and made
/// sending history unaffordable; holding the weights removes it.
///
/// Only the WEIGHTS are resident, not the llama context/KV cache — keeping the
/// cache alive would let each turn prefill only its new tokens, but `LlamaContext`
/// borrows both backend and model, so storing it needs a self-referential struct.
/// Not justified before the simple version is measured (knowledge/22).
pub struct ResidentModel {
    id: String,
    model: LlamaModel,
}

/// Load `info`'s weights if they aren't already the resident ones. Returns the
/// resident model, reloading only when the user switches models.
pub fn ensure_resident<'a>(
    slot: &'a mut Option<ResidentModel>,
    info: &ModelInfo,
    path: &Path,
) -> Result<&'a ResidentModel, String> {
    let stale = slot.as_ref().map(|r| r.id != info.id).unwrap_or(true);
    if stale {
        // Drop the old weights BEFORE loading the new ones: holding both at once
        // is what OOM-aborts a tight machine (knowledge/21 gap #2).
        *slot = None;
        let model = LlamaModel::load_from_file(backend()?, path, &model_params())
            .map_err(|e| format!("couldn't load the chat model: {e}"))?;
        *slot = Some(ResidentModel {
            id: info.id.to_string(),
            model,
        });
    }
    Ok(slot.as_ref().expect("just populated"))
}

/// Answer `user_message` in the context of `history`. Plain sampling (no grammar,
/// no JSON/tool-calling) — the generated text IS the reply, shown as-is.
///
/// `n_ctx` is sized per device+model by `context_for_budget`; history is trimmed
/// to whole turns that fit it (knowledge/22).
pub fn decide(
    resident: &ResidentModel,
    history: &[Turn],
    user_message: &str,
    style: PromptStyle,
    n_ctx: u32,
) -> Result<ToolDecision, String> {
    let model = &resident.model;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx.max(MIN_CONTEXT)))
        .with_n_batch(N_BATCH);
    let mut ctx = model
        .new_context(backend()?, ctx_params)
        .map_err(|e| format!("couldn't start a chat session: {e}"))?;

    // Cost history in REAL tokens (the model's own tokenizer), not an estimate —
    // an undercount here would overflow the window mid-generation. Falls back to
    // a conservative chars/3 if tokenising a fragment ever fails.
    let cost = |s: &str| -> usize {
        model
            .str_to_token(s, AddBos::Never)
            .map(|t| t.len())
            .unwrap_or_else(|_| s.len() / 3 + 1)
    };
    // Reserve = the reply budget + what the instruction actually costs on THIS
    // model's tokenizer + slack. Measured, not assumed.
    let reserved = MAX_NEW_TOKENS as usize + cost(SYSTEM) + RESERVE_SLACK_TOKENS;
    let keep_from = first_kept_turn(history, user_message, n_ctx, reserved, &cost);
    let prompt = format_chat_prompt(&history[keep_from..], user_message, style);
    let mut tokens = model
        .str_to_token(&prompt, add_bos_for(style))
        .map_err(|e| format!("couldn't read the message: {e}"))?;

    // Last-resort clamp. Eviction drops whole TURNS, so it cannot shrink a single
    // enormous message — and a prompt that doesn't fit used to surface a raw
    // engine error to the user ("ran out of context", reported by a beta tester on
    // build 11). Keep the TAIL: the newest words plus the assistant cue, which is
    // what the model needs to answer. A clipped instruction beats a hard failure.
    let cap = max_prompt_tokens(n_ctx);
    if tokens.len() > cap {
        tokens = tokens.split_off(tokens.len() - cap);
    }

    // Prefill in n_batch-sized chunks. The batch is a fixed-capacity buffer, NOT
    // the context: a 512-token batch cannot swallow a 2000-token conversation, and
    // trying was the actual bug behind that report. Chunking decouples how much
    // history we can carry from how much we can decode at once.
    let mut batch = LlamaBatch::new(N_BATCH as usize, 1);
    let last_idx = tokens.len().saturating_sub(1);
    let mut pos: i32 = 0;
    for chunk in tokens.chunks(N_BATCH as usize) {
        batch.clear();
        for token in chunk {
            let is_last = pos as usize == last_idx;
            batch
                .add(*token, pos, &[0], is_last)
                .map_err(|e| e.to_string())?;
            pos += 1;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("chat failed: {e}"))?;
    }

    // Real sampling — NOT pure greedy, and crucially NO grammar sampler (see the
    // module note: the grammar path can `ggml_abort()` the whole app). Greedy
    // argmax on a tiny q4 model collapses into a repetition loop (observed
    // 2026-06-26: "uids!!!!!!…"). Worse, the loop never reaches an end-of-turn
    // token, so generation runs the full budget every time — garbage AND slow. A
    // repetition penalty breaks the loop and a light temperature keeps replies
    // natural; the model can then actually stop. Order matters: penalties →
    // top_k → top_p → temp → dist (final pick). Qwen's recommended settings,
    // with a slightly firmer repeat penalty because q4 0.5B is loop-prone. [OQ-55]
    let mut sampler = LlamaSampler::chain_simple([
        LlamaSampler::penalties(64, 1.1, 0.0, 0.0),
        LlamaSampler::top_k(40),
        LlamaSampler::top_p(0.9, 1),
        LlamaSampler::temp(0.6),
        LlamaSampler::dist(SEED),
    ]);

    let mut reply = String::new();
    // Position of the next token in the CONTEXT, not in the batch — `batch` now
    // holds only the final prefill chunk, so its length is no longer the count of
    // everything decoded. `pos` tracked that across the chunks.
    let mut n_cur = pos;
    for _ in 0..MAX_NEW_TOKENS {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        if let Ok(piece) = model.token_to_str(token, Special::Tokenize) {
            reply.push_str(&piece);
        }
        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| e.to_string())?;
        n_cur += 1;
        ctx.decode(&mut batch)
            .map_err(|e| format!("chat failed: {e}"))?;
        if n_cur >= n_ctx as i32 {
            break;
        }
    }

    // The generated text is the reply — shown verbatim. No JSON, no tool routing.
    Ok(ToolDecision {
        action: "answer".to_string(),
        reply: reply.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(u: &str, a: &str) -> Turn {
        Turn {
            user: u.to_string(),
            assistant: a.to_string(),
        }
    }
    /// One "token" per word — enough to exercise selection deterministically.
    fn words(s: &str) -> usize {
        s.split_whitespace().count().max(1)
    }

    #[test]
    fn context_scales_with_what_the_device_can_spare() {
        // Roomy + a cheap KV cache (0.5B, ~12 KB/token) → capped by MAX_CONTEXT,
        // not by memory: prefill time is the binding constraint on a CPU.
        assert_eq!(context_for_budget(4000, 12, 32768), MAX_CONTEXT);
        // Tight machine with an expensive KV cache (4B, ~144 KB/token): 200 MB
        // buys ~1422 tokens, so we get a real but smaller window.
        let tight = context_for_budget(200, 144, 32768);
        assert!(tight > MIN_CONTEXT && tight < MAX_CONTEXT, "got {tight}");
        // Nothing spare → never below the floor (the RAM gate declines instead).
        assert_eq!(context_for_budget(0, 144, 32768), MIN_CONTEXT);
        // Never exceed what the model was trained for, however much RAM there is.
        assert_eq!(context_for_budget(100_000, 12, 2048), 2048);
    }

    #[test]
    fn prompt_cap_always_leaves_room_to_answer() {
        // The regression this guards (beta report, build 11): a prompt that fills
        // the window left nothing to generate into and surfaced a raw engine
        // error. The cap must always reserve the reply budget…
        for n_ctx in [MIN_CONTEXT, 1024, 2048, MAX_CONTEXT] {
            let cap = max_prompt_tokens(n_ctx);
            assert!(
                cap + MAX_NEW_TOKENS as usize <= n_ctx as usize,
                "n_ctx={n_ctx} cap={cap} leaves no room to reply"
            );
            assert!(cap > 0, "n_ctx={n_ctx} clamped the prompt out of existence");
        }
        // …and stay sane even if a device somehow reports a tiny window.
        assert!(max_prompt_tokens(64) >= 16);
    }

    #[test]
    fn spare_memory_subtracts_weights_and_headroom() {
        assert_eq!(spare_for_context_mb(4000, 1400), 4000 - 1400 - CONTEXT_HEADROOM_MB);
        // Tighter than the headroom itself → 0, not an underflow panic.
        assert_eq!(spare_for_context_mb(500, 1400), 0);
    }

    #[test]
    fn history_is_kept_newest_first_and_whole() {
        let history = vec![
            turn("oldest question here", "oldest answer here"),
            turn("middle question here", "middle answer here"),
            turn("newest question here", "newest answer here"),
        ];
        // A representative reserve (reply budget + instruction + slack); the real
        // one is measured against the model's tokenizer at call time.
        const RESERVE: usize = 300;
        // Budget for everything → keep all three.
        assert_eq!(first_kept_turn(&history, "hi", 4096, RESERVE, &words), 0);
        // A window that fits only the reserve + the newest turn or two: whatever
        // is kept must be a SUFFIX (the most recent), never a hole in the middle.
        let keep = first_kept_turn(&history, "hi", RESERVE as u32 + 30, RESERVE, &words);
        assert!(keep > 0 && keep <= history.len(), "got {keep}");
        // No room for any history at all → keep none, still answer the question.
        assert_eq!(
            first_kept_turn(&history, "hi", RESERVE as u32, RESERVE, &words),
            history.len()
        );
        // A bigger instruction (larger reserve) must cost HISTORY, never the reply
        // — the regression guard for growing the system prompt again. The reserve
        // has to leave less than one turn (18 "tokens" here) for this to bite; an
        // earlier version left 96 and the turns legitimately fit.
        let tight = first_kept_turn(&history, "hi", 4096, 4090, &words);
        assert_eq!(tight, history.len(), "a large reserve must evict history");
    }

    #[test]
    fn prompt_carries_prior_turns_in_order() {
        let history = vec![turn("my name is Ada", "Nice to meet you, Ada.")];
        let p = format_chat_prompt(&history, "what is my name?", PromptStyle::ChatMl);
        assert!(p.contains("my name is Ada"), "history missing: {p}");
        assert!(p.contains("Nice to meet you, Ada."));
        // The new question comes last, and the model is cued to answer it.
        let hist_at = p.find("my name is Ada").unwrap();
        let new_at = p.find("what is my name?").unwrap();
        assert!(hist_at < new_at, "history must precede the new message");
        assert!(p.trim_end().ends_with("<|im_start|>assistant"));
        // Every turn marker is balanced — a dangling half-turn corrupts the
        // structure the model relies on (knowledge/22).
        assert_eq!(p.matches("<|im_start|>").count(), p.matches("<|im_end|>").count() + 1);
    }

    #[test]
    fn gemma_prompt_carries_history_without_a_system_role() {
        let history = vec![turn("my name is Ada", "Nice to meet you, Ada.")];
        let p = format_chat_prompt(&history, "what is my name?", PromptStyle::Gemma);
        assert!(p.contains("my name is Ada"));
        assert!(p.contains("Nice to meet you, Ada."));
        // The instruction rides the FIRST user turn (Gemma has no system role),
        // and it must appear exactly once however long the history gets.
        assert_eq!(p.matches(SYSTEM).count(), 1);
        assert!(!p.contains("<|im_start|>"));
        assert!(p.trim_end().ends_with("<start_of_turn>model"));
    }

    /// Reproduction harness for `[OQ-71]` — garbage chat output on Intel macOS.
    ///
    /// Runs OUR real inference path against a real model, with no Tauri, no
    /// WebView, no frontend: the "simplest possible test case" rule from
    /// `skills/debugging.md`. Garbage here ⇒ the fault is in this file or in
    /// llama.cpp as we build it. Coherent here ⇒ the fault is above us.
    ///
    /// `#[ignore]` because it needs a multi-hundred-MB model; run it explicitly:
    ///   VERDURE_REPRO_MODEL=/path/to/model.gguf \
    ///     cargo test --lib repro_oq71 -- --ignored --nocapture
    ///
    /// **Getting a model to point it at.** The 469 MB Qwen2.5 0.5B copy this was
    /// developed against was deleted on 2026-08-22 to reclaim disk; the harness is
    /// kept because it is the regression check for `[OQ-71]`. Two ways back:
    ///   * Any `.gguf` already installed by the app works — check
    ///     `~/Library/Application Support/ai.verdure.porter/`. Match the prompt
    ///     style below to the model (`ChatMl` for Qwen, `Gemma` for Gemma).
    ///   * Or re-fetch the original (`model::QWEN_CHAT_SMALL`, 491,400,032 bytes):
    ///     curl -L -o /tmp/qwen05b.gguf \
    ///       https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf
    ///
    /// Keep while `[OQ-71]` can regress — the Metal fix it guards is a one-line
    /// backend choice in `model_params()`, easy to undo by accident.
    #[test]
    #[ignore]
    fn repro_oq71_generates_from_a_real_model() {
        let Ok(path) = std::env::var("VERDURE_REPRO_MODEL") else {
            panic!("set VERDURE_REPRO_MODEL to a .gguf path");
        };
        let path = std::path::Path::new(&path);
        assert!(path.is_file(), "no such model: {}", path.display());

        eprintln!("arch={} os={}", std::env::consts::ARCH, std::env::consts::OS);

        // The prompt style is a variable of the experiment, not a constant: a
        // reasoning model framed as a plain ChatML one spends its whole reply
        // budget thinking out loud. Default ChatML; override for another family:
        //   VERDURE_REPRO_STYLE=nothink|gemma
        let style = match std::env::var("VERDURE_REPRO_STYLE").unwrap_or_default().as_str() {
            "nothink" => PromptStyle::ChatMlNoThink,
            "gemma" => PromptStyle::Gemma,
            _ => PromptStyle::ChatMl,
        };
        eprintln!("style={style:?}");

        let info = &crate::model::QWEN_CHAT_SMALL; // id only; style is chosen above
        let mut slot = None;
        let resident = ensure_resident(&mut slot, info, path).expect("model loads");

        // Fixed window so context size is not a variable in this experiment.
        let out = decide(resident, &[], "How do the northern lights work?", style, 1024)
            .expect("generation succeeds");

        eprintln!("--- reply ---\n{}\n-------------", out.reply);
        assert!(!out.reply.trim().is_empty(), "empty reply");
    }

    #[test]
    fn chatml_prompt_frames_the_turn() {
        let p = format_chat_prompt(&[], "hi", PromptStyle::ChatMl);
        assert!(p.contains("<|im_start|>system"));
        assert!(p.contains("<|im_start|>user\nhi<|im_end|>"));
        assert!(p.trim_end().ends_with("<|im_start|>assistant"));
        // No BOS is written into the string; the tokenizer adds none for ChatML.
        assert_eq!(add_bos_for(PromptStyle::ChatMl), AddBos::Never);
    }

    #[test]
    fn no_think_style_prefills_an_empty_thinking_block() {
        // Qwen 3.5's documented "answer directly" switch is a PRE-FILLED EMPTY
        // thinking block after the assistant marker. Without it the model emits
        // hundreds of tokens of scratchpad and the answer never arrives — measured
        // on-device 2026-09-10, and invisible to any test that only checks that a
        // reply is non-empty.
        let p = format_chat_prompt(&[], "hi", PromptStyle::ChatMlNoThink);
        assert!(p.trim_end().ends_with("</think>"), "no-think block missing: {p}");
        assert!(p.contains("<|im_start|>assistant\n<think>\n\n</think>"));
        // Everything else must match plain ChatML exactly — the two arms share a
        // body precisely so history framing cannot drift between them.
        let plain = format_chat_prompt(&[], "hi", PromptStyle::ChatMl);
        assert_eq!(p.strip_suffix("<think>\n\n</think>\n\n").unwrap(), plain);
        assert_eq!(add_bos_for(PromptStyle::ChatMlNoThink), AddBos::Never);
    }

    #[test]
    fn a_cheaper_kv_cache_buys_a_bigger_window() {
        // The payoff of the hybrid architecture, in the one function that spends
        // it: the same spare RAM buys a far longer conversation at 12 KB/token
        // than at 144 KB/token (knowledge/22).
        let spare = 64; // MB
        let hybrid = context_for_budget(spare, 12, 262_144);
        let dense = context_for_budget(spare, 144, 32_768);
        assert!(hybrid > dense, "hybrid {hybrid} should exceed dense {dense}");
    }

    #[test]
    fn gemma_prompt_uses_turn_markers_not_chatml() {
        let p = format_chat_prompt(&[], "hi", PromptStyle::Gemma);
        assert!(p.contains("<start_of_turn>user"));
        assert!(p.contains("hi<end_of_turn>"));
        assert!(p.trim_end().ends_with("<start_of_turn>model"));
        // Gemma has no system role, and the BOS is added by the tokenizer — so
        // neither a ChatML marker nor a literal <bos> should appear in the text.
        assert!(!p.contains("<|im_start|>"));
        assert!(!p.contains("<bos>"));
        assert_eq!(add_bos_for(PromptStyle::Gemma), AddBos::Always);
    }
}
