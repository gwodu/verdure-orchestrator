//! Local text generation (knowledge/26). The pure half — how a conversation
//! is framed for each model family, and the arithmetic that sizes the context
//! window and trims history to fit — is lifted from the app's `llm.rs`, where
//! it was tuned against real beta reports (knowledge/22), and is tested here
//! on every platform. The engine is behind a feature:
//!
//! * `llama` — llama.cpp through `llama-cpp-2`, pinned to the exact version
//!   the app has verified on macOS, iOS and Android.

pub mod context;
#[cfg(feature = "llama")]
pub mod llama;
pub mod prompt;

pub use context::{
    context_for_budget, fit_messages, max_prompt_tokens, reply_budget, MAX_CONTEXT, MIN_CONTEXT,
};
pub use prompt::{format_chat, PromptStyle};
