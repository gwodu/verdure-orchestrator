//! Model registry + on-disk management. Models are DATA (like the porter
//! catalog): each entry says where to download it and what it's for. Both the
//! transcription panel and the global Storage screen read this one registry, so
//! space management (size + one-click remove) works uniformly for every model
//! as the set grows (knowledge/18 — "cheapest", and the floor user's tight disk).
//!
//! Downloads use base-macOS `curl` (no extra dependency). Models live as single
//! files in the app data folder, so removal fully reclaims the space.

use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
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
#[derive(Debug)]
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
// AVAILABLE RAM (see `chat_model_for_available`): a model needs more memory than an
// 8 GB machine has alongside the WebView — it OOMs and ggml aborts the process — so
// tight devices fall back to a smaller model. The base is now ByteShape's low-bit
// Qwen3 4B (a real 4B in ~1.4 GB — see `BYTESHAPE_QWEN3_4B`); the Qwen2.5 0.5B/1.5B
// pair are the always-present floor/fallback, and Gemma 4 E2B is a roomy-tier option
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

/// The base chat model going forward: ByteShape's ShapeLearn-quantized **Qwen3 4B
/// Instruct-2507** (Apache 2.0). A real 4B — far more coherent than the Qwen 0.5B
/// floor that produced garbage — squeezed to ~1.4 GB by per-tensor low-bit
/// quantization, so it can plausibly fit an 8 GB machine's AVAILABLE RAM. The file
/// is a standard `Q3_K_S` K-quant (ByteShape brands the CPU-preferred K-quants
/// "KQ"), so **stock** llama.cpp GGML kernels decode it — no fork — and Qwen3 (2025)
/// is within our pinned `llama-cpp-2`'s architecture support. We take the smallest
/// K-quant (2.77 bpw) as the safest first pick for the RAM-starved floor machine;
/// step up (`Q3_K_S-3.19bpw` …) for quality once it's confirmed to fit. [OQ-64]
pub const BYTESHAPE_QWEN3_4B: ModelInfo = ModelInfo {
    id: "qwen3-4b-instruct-2507-byteshape",
    name: "Chat model (Qwen3 4B)",
    modality: "Chat",
    file: "Qwen3-4B-Instruct-2507-Q3_K_S-2.77bpw.gguf",
    url: "https://huggingface.co/byteshape/Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen3-4B-Instruct-2507-Q3_K_S-2.77bpw.gguf",
    approx_mb: 1400,
    // Provisional: ~1.4 GB weights + KV/compute at 1024 ctx (llm.rs N_CTX) + llama
    // overhead ≈ ~2.3 GB. Calibrate on-device like the transcription predictor
    // (knowledge/19). NOTE: the 1.4 GB of weights is a hard floor — this can never
    // fit a machine with less than ~2.3 GB free, so on the 8 GB Mac (often ~1.5 GB
    // free) the gate still falls back to Qwen until RAM is freed. Deliberately keeps
    // a safety margin above the raw estimate: undershooting risks an OOM abort.
    runtime_mb: 2300,
    prompt_style: Some(PromptStyle::ChatMl),
};

/// Use the ByteShape Qwen3 4B as the base chat model. Enabled for the on-device
/// trial: it leads the pick when it fits, with the Qwen models as automatic
/// RAM-gated fallback, so enabling it can't strand a machine that's too tight — it
/// simply isn't picked there. This is NOT a "verified" claim (cf. Gemma's gate on an
/// unproven runtime feature); the on-device run IS the verification. Flip to `false`
/// to revert instantly if a run shows it fails to load on our pinned llama.cpp. [OQ-64]
const USE_BYTESHAPE_QWEN3_4B: bool = true;

/// Pick the largest chat model that fits the AVAILABLE memory right now, leaving
/// headroom for the WebView + OS. Returns `None` when nothing fits — the caller
/// then declines honestly instead of loading a model that would OOM-abort the app
/// (knowledge/21). Keying on *available* (not total) RAM is the fix for the 8 GB
/// chat crash: a 0.5B model "fits" 8 GB total but not ~1 GB free.
pub fn chat_model_for_available(available_mb: u64) -> Option<&'static ModelInfo> {
    chat_candidates()
        .into_iter()
        .find(|m| (m.runtime_mb as u64) <= available_mb)
}

/// The chat models we'll consider, most capable first. A model behind a flag stays
/// OUT until it's cleared for on-device use, so we never auto-select one that might
/// fail to load and OOM-abort the app. The Qwen pair is always present as the
/// floor/fallback. One list (not nested ifs) so adding a gated model is one line and
/// so every caller — the RAM picker AND the strict/preferred pick — shares one order.
fn chat_candidates() -> Vec<&'static ModelInfo> {
    let mut candidates: Vec<&'static ModelInfo> = Vec::new();
    if USE_BYTESHAPE_QWEN3_4B {
        candidates.push(&BYTESHAPE_QWEN3_4B); // the base chat model when it fits ([OQ-64])
    }
    if GEMMA4_E2B_VERIFIED {
        candidates.push(&GEMMA4_E2B); // roomy tier, only once verified ([OQ-61])
    }
    candidates.push(&QWEN_CHAT);
    candidates.push(&QWEN_CHAT_SMALL);
    candidates
}

/// The preferred (most capable) chat model, regardless of whether it fits right now.
/// This is the model "best model only" mode insists on, so the user can test it
/// deliberately instead of silently getting a smaller fallback. The Qwen floor
/// guarantees the list is never empty.
pub fn preferred_chat_model() -> &'static ModelInfo {
    chat_candidates()
        .into_iter()
        .next()
        .expect("the Qwen floor is always a candidate")
}

/// Why chat can't run: the chosen model needs more free memory than the machine has.
/// Carries the numbers so the UI can explain honestly (design.md — honest
/// performance) rather than failing blankly or silently downgrading.
#[derive(Debug)]
pub struct ChatModelTooBig {
    pub model_name: &'static str,
    pub needed_mb: u32,
    pub available_mb: u64,
    /// The user asked for the best model only (no silent fallback to a smaller one).
    pub strict: bool,
}

/// Choose which chat model to load. With `allow_fallback` we take the largest model
/// that fits available memory (the friendly default). Without it ("best model only")
/// we insist on the preferred model and, if it doesn't fit, return the shortfall so
/// the caller can tell the user exactly why — never a silent downgrade. Either way we
/// never return a model that wouldn't fit, since loading one OOM-aborts the app.
pub fn choose_chat_model(
    available_mb: u64,
    allow_fallback: bool,
) -> Result<&'static ModelInfo, ChatModelTooBig> {
    if allow_fallback {
        chat_model_for_available(available_mb).ok_or(ChatModelTooBig {
            // Even the smallest floor model won't fit — report against it.
            model_name: QWEN_CHAT_SMALL.name,
            needed_mb: QWEN_CHAT_SMALL.runtime_mb,
            available_mb,
            strict: false,
        })
    } else {
        let preferred = preferred_chat_model();
        if (preferred.runtime_mb as u64) <= available_mb {
            Ok(preferred)
        } else {
            Err(ChatModelTooBig {
                model_name: preferred.name,
                needed_mb: preferred.runtime_mb,
                available_mb,
                strict: true,
            })
        }
    }
}

/// The chat models the user chooses from on first launch (Small · Balanced ·
/// Powerful), smallest first. This is the explicit MENU — distinct from
/// `chat_candidates` (the internal auto-picker). All three are proven small
/// footprints so any of them is a safe pick on a modern device. Friendly tier
/// names + copy live in the UI (design.md — the words belong with the surface).
pub const CHAT_MODEL_CHOICES: &[&ModelInfo] = &[&QWEN_CHAT_SMALL, &QWEN_CHAT, &BYTESHAPE_QWEN3_4B];

/// One chat model as the picker needs it: what it is, how big the download is, and
/// whether it fits THIS device right now (memory + disk). Honest space up front
/// (design.md) so the user chooses with real numbers, not guesses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatModelOption {
    pub id: String,
    pub name: String,
    /// One-time download size, MB (what the user is spending on disk).
    pub download_mb: u32,
    /// Resident footprint estimate, MB (what it needs free to run).
    pub runtime_mb: u32,
    pub installed: bool,
    /// Enough free memory to run it without OOM-aborting.
    pub fits_memory: bool,
    /// Already downloaded, or enough free disk to download it.
    pub fits_disk: bool,
}

/// Build the picker's options for this device: each choice with its size and
/// whether it fits the memory/disk free right now.
pub fn chat_model_options(
    data_dir: &Path,
    available_mb: u64,
    free_disk_mb: u64,
) -> Vec<ChatModelOption> {
    CHAT_MODEL_CHOICES
        .iter()
        .map(|m| {
            let installed = model_path(data_dir, m).exists();
            ChatModelOption {
                id: m.id.to_string(),
                name: m.name.to_string(),
                download_mb: m.approx_mb,
                runtime_mb: m.runtime_mb,
                installed,
                fits_memory: (m.runtime_mb as u64) <= available_mb,
                fits_disk: installed || (m.approx_mb as u64) <= free_disk_mb,
            }
        })
        .collect()
}

/// Resolve the user's chosen chat model for loading. If `id` names one of the
/// choices and it fits available memory, use it. If it's chosen but won't fit,
/// return the shortfall so the UI can say so honestly (never load one that would
/// OOM-abort). If `id` is empty/unknown (no choice yet, or a stale value), fall
/// back to auto-picking the largest that fits.
pub fn chat_model_choice(
    id: &str,
    available_mb: u64,
) -> Result<&'static ModelInfo, ChatModelTooBig> {
    match CHAT_MODEL_CHOICES.iter().copied().find(|m| m.id == id) {
        Some(m) if (m.runtime_mb as u64) <= available_mb => Ok(m),
        Some(m) => Err(ChatModelTooBig {
            model_name: m.name,
            needed_mb: m.runtime_mb,
            available_mb,
            strict: true,
        }),
        None => chat_model_for_available(available_mb).ok_or(ChatModelTooBig {
            model_name: QWEN_CHAT_SMALL.name,
            needed_mb: QWEN_CHAT_SMALL.runtime_mb,
            available_mb,
            strict: false,
        }),
    }
}

/// Every model Verdure can download today. Gemma 4 E2B is listed so the Storage
/// screen can show + manage it uniformly, but the chat picker won't auto-select it
/// until it's verified (see `chat_model_for_available`).
pub const MODELS: &[ModelInfo] =
    &[WHISPER_BASE_EN, BYTESHAPE_QWEN3_4B, QWEN_CHAT_SMALL, QWEN_CHAT, GEMMA4_E2B];

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

/// Fetch `url` into `dest`, following redirects (Hugging Face `resolve` URLs
/// redirect to a CDN). Two implementations, one contract:
///
/// - **macOS**: base-system `curl` (no extra dependency, the path verified on the
///   founder's Mac). Kept byte-identical to the original — don't churn a working,
///   on-device-verified download path for the sake of uniformity.
/// - **iOS (and anything else)**: in-process HTTP via `ureq` (rustls). iOS apps
///   cannot spawn processes — there is no `/usr/bin/curl` to shell out to — so the
///   download must happen in-process. [OQ-65]
#[cfg(target_os = "macos")]
fn download(url: &str, dest: &Path) -> Result<(), String> {
    let output = Command::new("/usr/bin/curl")
        .args(["-fL", "--retry", "3", "-o", &dest.to_string_lossy(), url])
        .output()
        .map_err(|e| format!("the download couldn't start: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn download(url: &str, dest: &Path) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("the download couldn't start: {e}"))?;
    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("couldn't create the download file: {e}"))?;
    // Stream body → disk; a multi-GB model must never be buffered in memory
    // (ggml aborts on failed allocations, and iOS jetsam-kills memory hogs).
    std::io::copy(&mut response.into_reader(), &mut file)
        .map_err(|e| format!("the download was interrupted: {e}"))?;
    Ok(())
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
    if let Err(msg) = download(info.url, &part) {
        let _ = std::fs::remove_file(&part);
        return Err(format!("{} couldn't be downloaded. {}", info.name, msg));
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
        // Plenty free → the base 4B chat model (ByteShape Qwen3), when enabled.
        let roomy = chat_model_for_available(8000).unwrap().id;
        if USE_BYTESHAPE_QWEN3_4B {
            assert_eq!(roomy, BYTESHAPE_QWEN3_4B.id);
        } else {
            assert_eq!(roomy, QWEN_CHAT.id);
        }
        // Tight (the 8 GB Mac with the WebView running) → the small model only.
        assert_eq!(
            chat_model_for_available(1500).unwrap().id,
            QWEN_CHAT_SMALL.id
        );
        // Almost nothing free → decline rather than OOM-abort the app.
        assert!(chat_model_for_available(500).is_none());
    }

    #[test]
    fn byteshape_4b_is_the_base_when_it_fits_and_falls_back_when_it_does_not() {
        // Only meaningful while the base model is enabled.
        if !USE_BYTESHAPE_QWEN3_4B {
            return;
        }
        // Enough for the 4B → it's the pick, ahead of the Qwen fallback.
        assert_eq!(
            chat_model_for_available(BYTESHAPE_QWEN3_4B.runtime_mb as u64).unwrap().id,
            BYTESHAPE_QWEN3_4B.id
        );
        // Just below the 4B's footprint → it's skipped, and whatever IS picked
        // genuinely fits. This is the core safety invariant (never hand
        // `load_from_file` a model larger than available RAM — that OOM-aborts the
        // app), asserted without hard-coding which fallback wins, so it survives
        // runtime_mb re-tuning.
        let just_under = BYTESHAPE_QWEN3_4B.runtime_mb as u64 - 1;
        let pick = chat_model_for_available(just_under).unwrap();
        assert_ne!(pick.id, BYTESHAPE_QWEN3_4B.id);
        assert!((pick.runtime_mb as u64) <= just_under);
    }

    #[test]
    fn preferred_chat_model_is_the_top_candidate() {
        // The preferred model is the most capable candidate, independent of RAM —
        // it's what "best model only" insists on. Matches the head of the pick order.
        let preferred = preferred_chat_model();
        assert_eq!(preferred.id, chat_model_for_available(u64::MAX).unwrap().id);
        if USE_BYTESHAPE_QWEN3_4B {
            assert_eq!(preferred.id, BYTESHAPE_QWEN3_4B.id);
        }
    }

    #[test]
    fn best_model_only_uses_the_preferred_or_explains_why_not() {
        let preferred = preferred_chat_model();
        // Fits → we get the preferred model, never a downgrade.
        let ok = choose_chat_model(preferred.runtime_mb as u64, false).unwrap();
        assert_eq!(ok.id, preferred.id);
        // Doesn't fit → Err carrying the shortfall, flagged strict, and NO fallback
        // (even though a smaller model would have fit that memory).
        let err = choose_chat_model(preferred.runtime_mb as u64 - 1, false).unwrap_err();
        assert!(err.strict);
        assert_eq!(err.model_name, preferred.name);
        assert_eq!(err.needed_mb, preferred.runtime_mb);
    }

    #[test]
    fn fallback_downgrades_and_only_fails_when_nothing_fits() {
        // With fallback on and no room for the preferred model, we still get a
        // smaller one that fits (never the preferred, never a non-fitting model).
        let preferred = preferred_chat_model();
        let tight = QWEN_CHAT_SMALL.runtime_mb as u64; // fits only the floor
        let pick = choose_chat_model(tight, true).unwrap();
        assert!((pick.runtime_mb as u64) <= tight);
        if USE_BYTESHAPE_QWEN3_4B {
            assert_ne!(pick.id, preferred.id);
        }
        // Nothing fits → Err flagged non-strict (the "close some apps" message).
        let err = choose_chat_model(QWEN_CHAT_SMALL.runtime_mb as u64 - 1, true).unwrap_err();
        assert!(!err.strict);
    }

    #[test]
    fn explicit_choice_used_when_it_fits_declined_when_it_does_not() {
        let big = &BYTESHAPE_QWEN3_4B;
        // Chosen + fits → exactly that model, no auto-substitution.
        assert_eq!(
            chat_model_choice(big.id, big.runtime_mb as u64).unwrap().id,
            big.id
        );
        // Chosen + doesn't fit → honest error (strict), never a silent smaller model.
        let err = chat_model_choice(big.id, big.runtime_mb as u64 - 1).unwrap_err();
        assert!(err.strict);
        assert_eq!(err.model_name, big.name);
    }

    #[test]
    fn unknown_choice_falls_back_to_auto_pick() {
        // No choice yet / stale id → largest that fits, non-strict error only if
        // nothing fits at all.
        assert_eq!(
            chat_model_choice("", 8000).unwrap().id,
            chat_model_for_available(8000).unwrap().id
        );
        assert!(chat_model_choice("nope", 100).unwrap_err().strict == false);
    }

    #[test]
    fn chat_options_report_fit_against_the_device() {
        // Roomy machine, empty dir → every choice fits memory, none installed.
        let opts = chat_model_options(Path::new("/tmp/verdure-none-xyz"), 64_000, 64_000);
        assert_eq!(opts.len(), CHAT_MODEL_CHOICES.len());
        assert!(opts.iter().all(|o| o.fits_memory && o.fits_disk && !o.installed));
        // Tiny free memory → the Powerful (4B) no longer fits; the Small one still does.
        let tight = chat_model_options(Path::new("/tmp/verdure-none-xyz"), 1200, 64_000);
        let small = tight.iter().find(|o| o.id == QWEN_CHAT_SMALL.id).unwrap();
        let powerful = tight.iter().find(|o| o.id == BYTESHAPE_QWEN3_4B.id).unwrap();
        assert!(small.fits_memory);
        assert!(!powerful.fits_memory);
    }

    #[test]
    fn byteshape_4b_carries_the_chatml_prompt_style() {
        // Qwen3 uses ChatML, so the existing framing applies — no new plumbing.
        assert_eq!(BYTESHAPE_QWEN3_4B.prompt_style, Some(PromptStyle::ChatMl));
        assert_eq!(find("qwen3-4b-instruct-2507-byteshape").unwrap().modality, "Chat");
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
