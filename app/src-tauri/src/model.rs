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
};

/// The chat model behind the conversational front door (knowledge/18). Qwen2.5
/// 1.5B Instruct (Q4_K_M GGUF) — good chat + tool-calling for its size, ~1 GB,
/// usable on a CPU. [OQ-55].
pub const QWEN_CHAT: ModelInfo = ModelInfo {
    id: "qwen2.5-1.5b-instruct",
    name: "Chat model (Qwen2.5 1.5B)",
    modality: "Chat",
    file: "qwen2.5-1.5b-instruct-q4_k_m.gguf",
    url: "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf",
    approx_mb: 1100,
};

/// Every model Verdure can download today.
pub const MODELS: &[ModelInfo] = &[WHISPER_BASE_EN, QWEN_CHAT];

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
}
