//! Local chat LLM via llama.cpp (the `llama-cpp-2` crate) — the conversational
//! front door (knowledge/18). Compiled into the binary like whisper; a small
//! GGUF chat model (Qwen2.5 1.5B) is downloaded on first use and runs on the CPU,
//! fully local. Tool-calling (grammar-constrained) lands in the next increment;
//! this is plain chat generation.

use std::num::NonZeroU32;
use std::path::Path;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;

const N_CTX: u32 = 4096;
const MAX_NEW_TOKENS: i32 = 512;

/// Qwen2.5 chat template — wraps the user's message so the instruct model
/// behaves like an assistant. Special tokens are parsed by the tokenizer.
fn format_prompt(user: &str) -> String {
    format!(
        "<|im_start|>system\nYou are Verdure, a concise, friendly assistant that runs entirely on the user's computer.<|im_end|>\n<|im_start|>user\n{user}<|im_end|>\n<|im_start|>assistant\n"
    )
}

/// Generate a single reply to `user_message` using the GGUF model at `model_path`.
/// CPU, local, greedy decoding. Single-turn for v1 (no history yet).
pub fn chat(model_path: &Path, user_message: &str) -> Result<String, String> {
    let backend =
        LlamaBackend::init().map_err(|e| format!("couldn't start the chat engine: {e}"))?;

    let model = LlamaModel::load_from_file(&backend, model_path, &LlamaModelParams::default())
        .map_err(|e| format!("couldn't load the chat model: {e}"))?;

    let ctx_params = LlamaContextParams::default().with_n_ctx(NonZeroU32::new(N_CTX));
    let mut ctx = model
        .new_context(&backend, ctx_params)
        .map_err(|e| format!("couldn't start a chat session: {e}"))?;

    let prompt = format_prompt(user_message);
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

    let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
    let mut output = String::new();
    let mut n_cur = batch.n_tokens();

    for _ in 0..MAX_NEW_TOKENS {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        if let Ok(piece) = model.token_to_str(token, Special::Tokenize) {
            output.push_str(&piece);
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

    Ok(output.trim().to_string())
}
