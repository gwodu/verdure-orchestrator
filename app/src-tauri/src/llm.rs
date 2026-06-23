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

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;

// Kept modest on purpose: ggml aborts the whole process on a failed allocation,
// and the floor target is an 8 GB machine where the WebView already uses ~1.5 GB.
// 2048 ctx + a 512 batch keeps the KV + compute buffers small. [OQ-55]
const N_CTX: u32 = 2048;
const N_BATCH: u32 = 512;
const MAX_NEW_TOKENS: i32 = 512;

/// The model's decision: which action, and the user-facing text.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDecision {
    /// "answer" | "set_up_transcription"
    pub action: String,
    /// The answer text, or a confirmation line for a tool action.
    pub reply: String,
}

fn format_tool_prompt(user: &str) -> String {
    format!(
        "<|im_start|>system\nYou are Verdure, a local personal AI. Decide what the user wants and reply ONLY in JSON. If they want to turn audio, recordings, voice notes, meetings, or dictation into text, reply {{\"action\":\"set_up_transcription\"}}. Otherwise answer helpfully and concisely with {{\"action\":\"answer\",\"reply\":\"...\"}}.<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
    )
}

/// Decide what to do about `user_message` — answer, or invoke a tool. The output
/// is grammar-locked to valid JSON, then parsed.
pub fn decide(model_path: &Path, user_message: &str) -> Result<ToolDecision, String> {
    let backend =
        LlamaBackend::init().map_err(|e| format!("couldn't start the chat engine: {e}"))?;
    let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())
        .map_err(|e| format!("couldn't load the chat model: {e}"))?;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(N_CTX))
        .with_n_batch(N_BATCH);
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| format!("couldn't start a chat session: {e}"))?;

    let prompt = format_tool_prompt(user_message);
    let tokens = model
        .str_to_token(&prompt, AddBos::Never)
        .map_err(|e| format!("couldn't read the message: {e}"))?;

    let mut batch = LlamaBatch::new(512, 1);
    let last_idx = tokens.len() as i32 - 1;
    for (i, token) in (0_i32..).zip(tokens.iter()) {
        batch
            .add(*token, i, &[0], i == last_idx)
            .map_err(|e| e.to_string())?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| format!("chat failed: {e}"))?;

    // Plain greedy sampling — deterministic, and crucially NO grammar sampler
    // (see the module note: the grammar path can `ggml_abort()` the whole app).
    let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);

    let mut json = String::new();
    let mut n_cur = batch.n_tokens();
    for _ in 0..MAX_NEW_TOKENS {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        if let Ok(piece) = model.token_to_str(token, Special::Tokenize) {
            json.push_str(&piece);
        }
        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| e.to_string())?;
        n_cur += 1;
        ctx.decode(&mut batch)
            .map_err(|e| format!("chat failed: {e}"))?;
        if n_cur >= N_CTX as i32 {
            break;
        }
    }

    parse_decision(&json)
}

/// Parse the model's output into a decision. Without a grammar the model may wrap
/// its JSON in prose (or skip JSON entirely), so this is lenient: pull the first
/// `{...}` object if there is one, else treat the whole text as a plain answer.
/// Tool ROUTING does not depend on this being perfect — the UI cross-checks intent
/// deterministically (`classifyIntent`).
fn parse_decision(text: &str) -> Result<ToolDecision, String> {
    let trimmed = text.trim();

    if let Some(obj) = extract_json_object(trimmed) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&obj) {
            let raw_action = value.get("action").and_then(|a| a.as_str()).unwrap_or("answer");
            // Only honor the one tool action we know; anything else is an answer.
            let action = if raw_action == "set_up_transcription" {
                "set_up_transcription"
            } else {
                "answer"
            };
            let mut reply = value
                .get("reply")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            if action == "set_up_transcription" && reply.is_empty() {
                reply = "I can set you up to turn recordings into text — right here on your computer."
                    .to_string();
            }
            if action == "answer" && reply.is_empty() {
                // Valid JSON but no usable reply — fall back to the raw text.
                reply = trimmed.to_string();
            }
            return Ok(ToolDecision { action: action.to_string(), reply });
        }
    }

    // No usable JSON — the model just chatted. Show what it said.
    Ok(ToolDecision {
        action: "answer".to_string(),
        reply: trimmed.to_string(),
    })
}

/// Pull the first balanced-ish JSON object out of a string (first `{` … last `}`).
/// Good enough for the single flat object the model emits; prose around it is fine.
fn extract_json_object(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| s[start..=end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_answer() {
        let d = parse_decision(r#"{"action":"answer","reply":"hello there"}"#).unwrap();
        assert_eq!(d.action, "answer");
        assert_eq!(d.reply, "hello there");
    }

    #[test]
    fn parses_a_tool_call_and_fills_a_default_reply() {
        let d = parse_decision(r#"{"action":"set_up_transcription"}"#).unwrap();
        assert_eq!(d.action, "set_up_transcription");
        assert!(!d.reply.is_empty());
    }

    #[test]
    fn plain_text_without_json_becomes_an_answer() {
        // No grammar → the model might just talk. We show what it said, not an error.
        let d = parse_decision("Sure! The capital of France is Paris.").unwrap();
        assert_eq!(d.action, "answer");
        assert_eq!(d.reply, "Sure! The capital of France is Paris.");
    }

    #[test]
    fn extracts_json_wrapped_in_prose() {
        let d = parse_decision("Here you go: {\"action\":\"answer\",\"reply\":\"hi\"} hope that helps")
            .unwrap();
        assert_eq!(d.action, "answer");
        assert_eq!(d.reply, "hi");
    }

    #[test]
    fn unknown_action_falls_back_to_answer() {
        let d = parse_decision(r#"{"action":"launch_missiles","reply":"no"}"#).unwrap();
        assert_eq!(d.action, "answer");
    }
}
