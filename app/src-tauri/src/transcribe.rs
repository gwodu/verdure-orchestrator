//! Transcription via whisper.cpp (the `whisper-rs` crate), used by live
//! dictation (`live.rs`) — the first model wrapped directly inside Verdure (knowledge/18). No external app:
//! whisper.cpp is compiled into this binary and runs locally on the CPU. Audio
//! never leaves the machine.

use std::path::Path;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Load the ggml speech model at `model_path` into a reusable context. Loading is
/// the slow part (hundreds of MB off disk), so callers that transcribe repeatedly
/// — live capture (`live.rs`) — load ONCE and reuse the context across segments.
pub(crate) fn load(model_path: &Path) -> Result<WhisperContext, String> {
    let path_str = model_path
        .to_str()
        .ok_or_else(|| "model path is not valid text".to_string())?;
    WhisperContext::new_with_params(path_str, WhisperContextParameters::default())
        .map_err(|e| format!("couldn't load the speech model: {e}"))
}

/// Transcribe 16 kHz mono f32 PCM with an already-loaded context (see `load`).
/// A fresh state per call keeps each chunk independent — right for live capture,
/// where each segment is decoded on its own.
pub(crate) fn transcribe_with(ctx: &WhisperContext, samples: &[f32]) -> Result<String, String> {
    let mut state = ctx
        .create_state()
        .map_err(|e| format!("couldn't start transcription: {e}"))?;

    // Greedy decoding, English, quiet (no stdout spew from whisper.cpp).
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some("en"));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state
        .full(params, samples)
        .map_err(|e| format!("transcription failed: {e}"))?;

    let mut text = String::new();
    let n = state.full_n_segments();
    for i in 0..n {
        if let Some(segment) = state.get_segment(i) {
            if let Ok(s) = segment.to_str() {
                text.push_str(s);
            }
        }
    }
    Ok(text.trim().to_string())
}
