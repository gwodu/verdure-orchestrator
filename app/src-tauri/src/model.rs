//! Speech-model management — fetch the Whisper ggml model on first use and keep
//! it in the app's data folder. ~142 MB, downloaded once. Uses base-macOS `curl`
//! (no extra dependency), matching the "no toolchain for the user" goal.

use std::path::{Path, PathBuf};
use std::process::Command;

/// base.en: a good speed/quality balance for English on a CPU. Official
/// whisper.cpp weights on Hugging Face. [OQ-51]: pick by device tier later.
const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
const MODEL_FILE: &str = "ggml-base.en.bin";

/// Where the model lives once installed.
pub fn model_path(data_dir: &Path) -> PathBuf {
    data_dir.join(MODEL_FILE)
}

/// Ensure the model is present, downloading it once if needed. Reports progress.
pub fn ensure_model<F: Fn(&str)>(data_dir: &Path, progress: &F) -> Result<PathBuf, String> {
    let path = model_path(data_dir);
    if path.exists() {
        return Ok(path);
    }

    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("couldn't create the app folder: {e}"))?;

    progress("Getting the speech model ready — a one-time ~140 MB download…");

    // Download to a .part file and rename on success, so an interrupted download
    // is never mistaken for a complete model.
    let part = path.with_extension("part");
    let output = Command::new("/usr/bin/curl")
        .args([
            "-fL",
            "--retry",
            "3",
            "-o",
            &part.to_string_lossy(),
            MODEL_URL,
        ])
        .output()
        .map_err(|e| format!("the download couldn't start: {e}"))?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!(
            "the speech model couldn't be downloaded. {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    std::fs::rename(&part, &path).map_err(|e| format!("couldn't save the model: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_path_is_in_the_data_dir() {
        let p = model_path(Path::new("/tmp/verdure"));
        assert_eq!(p, PathBuf::from("/tmp/verdure/ggml-base.en.bin"));
    }
}
