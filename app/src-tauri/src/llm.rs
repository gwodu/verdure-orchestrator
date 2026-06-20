//! Local chat LLM via llama.cpp (the `llama-cpp-2` crate) — the conversational
//! front door (knowledge/18, 20). The model both converses AND orchestrates: it
//! decides whether to answer or to invoke a capability, emitting a tool call
//! that a **GBNF grammar forces to be valid JSON** — so a small model can't
//! malform the call; it only chooses *which* action. Runs on the CPU, local.

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

// The tool surface, as a grammar: output is EITHER the transcription tool call
// OR an answer with free text. The model cannot produce anything else — invalid
// tool calls are impossible by construction (knowledge/18). Add a tool = add a
// branch here + a case in the frontend.
const TOOL_GRAMMAR: &str = r#"
root   ::= setup | answer
setup  ::= "{\"action\":\"set_up_transcription\"}"
answer ::= "{\"action\":\"answer\",\"reply\":\"" chars "\"}"
chars  ::= char*
char   ::= [^"\\] | "\\" ["\\/bfnrt]
"#;

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

    // Grammar sampler first (masks invalid tokens), then greedy among what's left.
    let grammar = LlamaSampler::grammar(&model, TOOL_GRAMMAR, "root")
        .map_err(|e| format!("grammar error: {e}"))?;
    let mut sampler = LlamaSampler::chain_simple([grammar, LlamaSampler::greedy()]);

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

/// Parse the grammar-locked JSON into a decision. Defaults to a plain answer if
/// anything is off (grammar makes that essentially impossible, but be safe).
fn parse_decision(json: &str) -> Result<ToolDecision, String> {
    let value: serde_json::Value = serde_json::from_str(json.trim())
        .map_err(|_| format!("the model returned something unexpected: {json}"))?;
    let action = value
        .get("action")
        .and_then(|a| a.as_str())
        .unwrap_or("answer")
        .to_string();
    let mut reply = value
        .get("reply")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();
    if action == "set_up_transcription" && reply.is_empty() {
        reply = "I can set you up to turn recordings into text — right here on your computer.".to_string();
    }
    Ok(ToolDecision { action, reply })
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
}
