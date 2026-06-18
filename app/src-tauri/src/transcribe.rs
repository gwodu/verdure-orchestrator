//! Transcription via whisper.cpp (the `whisper-rs` crate) — the first model
//! wrapped directly inside the Verdure shell (knowledge/18). No external app:
//! whisper.cpp is compiled into this binary and runs locally on the CPU. Audio
//! never leaves the machine.
//!
//! This is the de-risk increment. The biggest unknown in the pivot is whether
//! whisper.cpp's C++ cross-compiles to x86_64 (the founder's Intel Mac) on the
//! Apple-Silicon CI runner. Referencing `whisper-rs` here forces that C++ to
//! build for the target, so CI proves it before we invest in the full
//! decode → resample → transcribe pipeline and the UI.

/// Returns true once whisper.cpp is linked in. Constructing a whisper-rs type
/// forces the bundled C++ engine to compile + link for the build target — the
/// exact thing we're de-risking. Replaced by the real `transcribe()` next.
pub fn engine_linked() -> bool {
    let _params = whisper_rs::WhisperContextParameters::default();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whisper_engine_links() {
        assert!(engine_linked());
    }
}
