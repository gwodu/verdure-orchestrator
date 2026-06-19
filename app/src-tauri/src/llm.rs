//! Local chat LLM via llama.cpp (the `llama-cpp-2` crate) — the conversational
//! front door (knowledge/18). Compiled into the binary like whisper; a small
//! GGUF chat model is downloaded on first use. The model converses AND drives
//! orchestration via grammar-constrained (GBNF) tool-calling, so calls are always
//! well-formed and the only question is which tool — never whether it parses.
//!
//! This is the de-risk increment: referencing llama-cpp-2 forces its bundled
//! llama.cpp C++ to compile and CROSS-COMPILE to x86_64 (the founder's Intel Mac)
//! on the Apple-Silicon CI runner — the same risk we retired for whisper, helped
//! by the deployment-target fix in .cargo/config.toml. Chat + tool-calling land
//! next, once CI proves the runtime builds.

/// Returns true once llama.cpp is linked in. Naming a llama-cpp-2 type pulls in
/// llama-cpp-sys-2 (the bundled C++), so CI builds and cross-compiles it.
pub fn engine_linked() -> bool {
    let _ = std::any::type_name::<llama_cpp_2::llama_backend::LlamaBackend>();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llama_engine_links() {
        assert!(engine_linked());
    }
}
