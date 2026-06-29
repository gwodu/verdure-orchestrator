//! Model registry + on-disk management. Models are DATA (like the porter
//! catalog): each entry says where to download it and what it's for. Both the
//! transcription panel and the global Storage screen read this one registry, so
//! space management (size + one-click remove) works uniformly for every model
//! as the set grows (knowledge/18 — "cheapest", and the floor user's tight disk).
//!
//! Downloads use base-macOS `curl` (no extra dependency). Models live as single
//! files in the app data folder, so removal fully reclaims the space.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// How a chat model wants its prompt framed. Families differ in turn markers and
/// BOS convention; using the wrong one yields garbage output, so it travels with
/// the model as data (knowledge/03 — model details). Non-chat models carry `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptStyle {
    /// Qwen 2.5 etc. — ChatML (`<|im_start|>`…`<|im_end|>`), no leading BOS.
    ChatMl,
    /// Gemma — `<start_of_turn>`/`<end_of_turn>`, leading BOS, no system role.
    Gemma,
}

/// A downloadable model: what it is, and where to get it.
pub struct ModelInfo {
    pub id: &'static str,
    /// User-facing name (jargon-free; skills/design.md).
    pub name: &'static str,
    /// What the user uses it for, e.g. "Transcription".
    pub modality: &'static str,
    /// File name in the app data folder.
    pub file: &'static str,
    pub url: &'static str,
    /// Approximate download size, for the one-time "~NN MB" message.
    pub approx_mb: u32,
    /// Rough RAM footprint (MB) while loaded + running: weights + KV-cache +
    /// compute buffers + runtime overhead. Used to decide whether a model fits
    /// the machine's AVAILABLE memory before we load it — loading one that
    /// doesn't fit makes ggml abort the whole process (knowledge/21 gap #4).
    /// Provisional estimates, to be calibrated on-device like the transcription
    /// predictor (knowledge/19). 0 = not gated this way.
    pub runtime_mb: u32,
    /// Prompt framing for chat models; `None` for non-chat models (e.g. Whisper).
    pub prompt_style: Option<PromptStyle>,
}

/// base.en: a good speed/quality balance for English on a CPU. Official
/// whisper.cpp weights on Hugging Face. [OQ-51]: pick by device tier later.
pub const WHISPER_BASE_EN: ModelInfo = ModelInfo {
    id: "whisper-base-en",
    name: "English speech model",
    modality: "Transcription",
    file: "ggml-base.en.bin",
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
    approx_mb: 142,
    runtime_mb: 600,
    prompt_style: None,
};

// Chat models behind the conversational front door (knowledge/18). We pick by
// AVAILABLE RAM (see `chat_model_for_available`): a 1.5B model needs more memory
// than an 8 GB machine has alongside the WebView — it OOMs and ggml aborts the
// process — so low-RAM devices get the 0.5B model. The Qwen2.5 Instruct pair are
// good tool-callers for their size; Gemma 4 E2B (below) is the roomy-tier option,
// gated until verified. Sampling is plain (no grammar) — see `llm.rs`. [OQ-55].

/// Smaller chat model for memory-constrained machines (≤ 8 GB). ~400 MB.
pub const QWEN_CHAT_SMALL: ModelInfo = ModelInfo {
    id: "qwen2.5-0.5b-instruct",
    name: "Chat model (Qwen2.5 0.5B)",
    modality: "Chat",
    file: "qwen2.5-0.5b-instruct-q4_k_m.gguf",
    url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
    approx_mb: 400,
    // ~469 MB weights + KV/compute at 2048 ctx + llama overhead. On an 8 GB Intel
    // Mac this still aborted with only ~2 GB free — so the gate is on AVAILABLE,
    // not total, RAM (knowledge/21).
    runtime_mb: 1100,
    prompt_style: Some(PromptStyle::ChatMl),
};

/// Larger chat model for roomier machines. ~1.1 GB download.
pub const QWEN_CHAT: ModelInfo = ModelInfo {
    id: "qwen2.5-1.5b-instruct",
    name: "Chat model (Qwen2.5 1.5B)",
    modality: "Chat",
    file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
    url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
    approx_mb: 1100,
    runtime_mb: 2400,
    prompt_style: Some(PromptStyle::ChatMl),
};

/// Roomy-tier chat model: Gemma 4 E2B (Google, 2026-04-02, Apache 2.0). Sparse
/// activation (Per-Layer Embeddings) → ~2.3B *effective* params, so short-prompt
/// replies feel close to a 1.5B while carrying more capability — the appeal for the
/// chat front door (knowledge/03, the speed axis). BUT the Q4_K_M file is ~3.1 GB,
/// and the advertised <1.5 GB *active* footprint only materialises if the runtime
/// implements PLE offload — unverified in our pinned llama-cpp-2. So `runtime_mb`
/// is set to the full resident cost (conservative), and the model is held OUT of
/// the live pick (see `GEMMA4_E2B_VERIFIED`) until an on-device run confirms it
/// loads and offloads — loading one that doesn't OOM-aborts the whole app. [OQ-61]
pub const GEMMA4_E2B: ModelInfo = ModelInfo {
    id: "gemma-4-e2b-it",
    name: "Chat model (Gemma 4 E2B)",
    modality: "Chat",
    file: "gemma-4-E2B-it-Q4_K_M.gguf",
    url: "https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf",
    approx_mb: 3100,
    // Full resident estimate: ~3.1 GB weights + KV/compute at 2048 ctx + overhead.
    // Revisit DOWN once PLE offload is confirmed (then it can drop toward ~1.5 GB).
    runtime_mb: 4000,
    prompt_style: Some(PromptStyle::Gemma),
};

/// Whether Gemma 4 E2B has been verified on-device — that our pinned llama.cpp
/// actually loads it AND does the PLE offload that delivers its advertised active
/// footprint. Until this is `true`, the picker never auto-selects it, so we never
/// hand a user a model that might fail to load and OOM-abort the app. Flip to
/// `true` only after an on-device run confirms both ([OQ-61]).
const GEMMA4_E2B_VERIFIED: bool = false;

/// Pick the largest chat model that fits the AVAILABLE memory right now, leaving
/// headroom for the WebView + OS. Returns `None` when nothing fits — the caller
/// then declines honestly instead of loading a model that would OOM-abort the app
/// (knowledge/21). Keying on *available* (not total) RAM is the fix for the 8 GB
/// chat crash: a 0.5B model "fits" 8 GB total but not ~1 GB free.
pub fn chat_model_for_available(available_mb: u64) -> Option<&'static ModelInfo> {
    // Most capable first, so a roomy machine gets the better model. Gemma 4 E2B
    // leads the list ONLY once verified on-device ([OQ-61]); until then it stays
    // out so we never auto-select a model that might OOM-abort the app.
    let candidates: &[&ModelInfo] = if GEMMA4_E2B_VERIFIED {
        &[&GEMMA4_E2B, &QWEN_CHAT, &QWEN_CHAT_SMALL]
    } else {
        &[&QWEN_CHAT, &QWEN_CHAT_SMALL]
    };
    candidates
        .iter()
        .copied()
        .find(|m| (m.runtime_mb as u64) <= available_mb)
}

/// Every model Verdure can download today. Gemma 4 E2B is listed so the Storage
/// screen can show + manage it uniformly, but the chat picker won't auto-select it
/// until it's verified (see `chat_model_for_available`).
pub const MODELS: &[ModelInfo] = &[WHISPER_BASE_EN, QWEN_CHAT_SMALL, QWEN_CHAT, GEMMA4_E2B];

/// Look up a model by id.
pub fn find(id: &str) -> Option<&'static ModelInfo> {
    MODELS.iter().find(|m| m.id == id)
}

/// Where a model lives once installed.
pub fn model_path(data_dir: &Path, info: &ModelInfo) -> PathBuf {
    data_dir.join(info.file)
}

/// One model's presence + size, for the UI (transcribe panel + Storage screen).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub modality: String,
    pub installed: bool,
    pub size_bytes: u64,
}

/// List every known model with whether it's downloaded and its size.
pub fn list(data_dir: &Path) -> Vec<ModelEntry> {
    MODELS
        .iter()
        .map(|m| {
            let (installed, size_bytes) = match std::fs::metadata(model_path(data_dir, m)) {
                Ok(meta) => (true, meta.len()),
                Err(_) => (false, 0),
            };
            ModelEntry {
                id: m.id.to_string(),
                name: m.name.to_string(),
                modality: m.modality.to_string(),
                installed,
                size_bytes,
            }
        })
        .collect()
}

/// Ensure a model is present, downloading it once if needed. Reports progress.
pub fn ensure<F: Fn(&str)>(
    data_dir: &Path,
    info: &ModelInfo,
    progress: &F,
) -> Result<PathBuf, String> {
    let path = model_path(data_dir, info);
    // A truncated/corrupt model makes the inference engine (ggml) ABORT the whole
    // process, not return an error — so we validate the size (≥70% of expected)
    // before trusting an existing file, and re-download if it looks wrong.
    let min_bytes = (info.approx_mb as u64) * 1024 * 1024 * 7 / 10;
    if path.exists() {
        let ok = std::fs::metadata(&path)
            .map(|m| m.len() >= min_bytes)
            .unwrap_or(false);
        if ok {
            return Ok(path);
        }
        let _ = std::fs::remove_file(&path); // incomplete/corrupt — start over
    }

    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("couldn't create the app folder: {e}"))?;

    progress(&format!(
        "Getting the {} ready — a one-time ~{} MB download…",
        info.name.to_lowercase(),
        info.approx_mb
    ));

    // Download to a .part file, rename on success — an interrupted download is
    // never mistaken for a complete model.
    let part = path.with_extension("part");
    let output = Command::new("/usr/bin/curl")
        .args(["-fL", "--retry", "3", "-o", &part.to_string_lossy(), info.url])
        .output()
        .map_err(|e| format!("the download couldn't start: {e}"))?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!(
            "{} couldn't be downloaded. {}",
            info.name,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    std::fs::rename(&part, &path).map_err(|e| format!("couldn't save the model: {e}"))?;

    // Verify the finished file is a plausible size; a truncated download would
    // otherwise crash the inference engine.
    let actual = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    if actual < min_bytes {
        let _ = std::fs::remove_file(&path);
        return Err(format!(
            "the download looks incomplete ({} MB of ~{} MB). Please check your connection and try again.",
            actual / 1024 / 1024,
            info.approx_mb
        ));
    }
    Ok(path)
}

/// Completely remove a downloaded model (and any interrupted .part), reclaiming
/// the space. It re-downloads on next use.
pub fn remove(data_dir: &Path, id: &str) -> Result<(), String> {
    let info = find(id).ok_or_else(|| format!("unknown model: {id}"))?;
    let path = model_path(data_dir, info);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("couldn't remove the model: {e}"))?;
    }
    let _ = std::fs::remove_file(path.with_extension("part"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_whisper_model() {
        assert_eq!(find("whisper-base-en").unwrap().modality, "Transcription");
        assert!(find("nope").is_none());
    }

    #[test]
    fn model_path_uses_the_file_name() {
        let p = model_path(Path::new("/tmp/verdure"), &WHISPER_BASE_EN);
        assert_eq!(p, PathBuf::from("/tmp/verdure/ggml-base.en.bin"));
    }

    #[test]
    fn list_reports_not_installed_for_empty_dir() {
        let entries = list(Path::new("/tmp/verdure-does-not-exist-xyz"));
        assert_eq!(entries.len(), MODELS.len());
        assert!(entries.iter().all(|e| !e.installed && e.size_bytes == 0));
    }

    #[test]
    fn remove_unknown_id_errors() {
        assert!(remove(Path::new("/tmp"), "nope").is_err());
    }

    #[test]
    fn chat_model_picks_by_available_memory() {
        // Plenty free → the larger model.
        assert_eq!(chat_model_for_available(8000).unwrap().id, QWEN_CHAT.id);
        // Tight (the 8 GB Mac with the WebView running) → the small model only.
        assert_eq!(
            chat_model_for_available(1500).unwrap().id,
            QWEN_CHAT_SMALL.id
        );
        // Almost nothing free → decline rather than OOM-abort the app.
        assert!(chat_model_for_available(500).is_none());
    }

    #[test]
    fn unverified_gemma_stays_out_of_the_live_pick() {
        // Even with abundant memory, the gate keeps Gemma 4 E2B out of the live
        // pick until it's verified on-device ([OQ-61]) — never auto-select a model
        // that might fail to load and OOM-abort the app.
        if !GEMMA4_E2B_VERIFIED {
            let pick = chat_model_for_available(64_000).unwrap();
            assert_ne!(pick.id, GEMMA4_E2B.id);
        }
    }

    #[test]
    fn gemma_carries_the_gemma_prompt_style() {
        // The template family travels with the model so inference frames it right.
        assert_eq!(GEMMA4_E2B.prompt_style, Some(PromptStyle::Gemma));
        assert_eq!(QWEN_CHAT.prompt_style, Some(PromptStyle::ChatMl));
        assert_eq!(WHISPER_BASE_EN.prompt_style, None);
    }
}
