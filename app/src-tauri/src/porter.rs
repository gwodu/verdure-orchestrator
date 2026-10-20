//! The porter — execute a tool's recipe: install it (if needed), then launch it.
//!
//! This is the "one click" in "ports it in one click" (knowledge/17). It knows
//! how to RUN recipes; the recipes (catalog.rs) say what to run. For a macOS
//! .dmg port the steps are: download the official .dmg, mount it, copy the .app
//! into /Applications, unmount, clear the quarantine flag, launch.
//!
//! Everything here uses only base-macOS tools at fixed paths (curl, hdiutil, cp,
//! xattr, open) — NO Homebrew and NO Xcode Command Line Tools. A Finder-launched
//! app gets a minimal PATH, so absolute paths matter.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::catalog::Tool;

/// The result of a successful port, reported back to the UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PortOutcome {
    /// True if we installed it just now; false if it was already present.
    pub installed_now: bool,
    /// True once the launch command was issued successfully.
    pub launched: bool,
    /// A short, human, jargon-free summary for the UI.
    pub message: String,
}

/// Everything that can go wrong, in language we can show a non-technical user.
#[derive(Debug)]
pub(crate) enum PortError {
    /// A step of the install (download / mount / copy) failed; carries detail.
    InstallFailed(String),
    /// The launch step failed.
    LaunchFailed(String),
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortError::InstallFailed(detail) => write!(f, "Setup didn't finish. {detail}"),
            PortError::LaunchFailed(detail) => {
                write!(f, "It installed, but couldn't open automatically. {detail}")
            }
        }
    }
}

impl std::error::Error for PortError {}

/// Where an installed .app lives, by its bundle name.
fn installed_app_path(app_name: &str) -> PathBuf {
    PathBuf::from("/Applications").join(format!("{app_name}.app"))
}

/// Is the tool already installed? For a .dmg port: does its .app exist?
fn is_installed(tool: &Tool) -> bool {
    installed_app_path(tool.launch.app_name()).exists()
}

/// Run a base-macOS command, mapping any failure to InstallFailed with context.
fn run(label: &str, program: &str, args: &[&str]) -> Result<(), PortError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| PortError::InstallFailed(format!("{label}: {e}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(PortError::InstallFailed(format!("{label}: {}", tail(&output.stderr))))
    }
}

/// Download + mount + copy + unmount + de-quarantine. `is_arm64` picks the build.
fn install<F: Fn(&str)>(tool: &Tool, is_arm64: bool, progress: &F) -> Result<(), PortError> {
    let app = tool.launch.app_name();
    let url = tool.install.download_url(is_arm64);
    let tmp = std::env::temp_dir();
    let dmg = tmp.join(format!("verdure-{}.dmg", tool.id));
    let mnt = tmp.join(format!("verdure-mnt-{}", tool.id));

    let dmg_s = dmg.to_string_lossy().to_string();
    let mnt_s = mnt.to_string_lossy().to_string();

    progress(&format!(
        "Downloading {app} — this is a big file and can take a few minutes…"
    ));
    run(
        "download",
        "/usr/bin/curl",
        &["-fL", "--retry", "3", "-o", &dmg_s, url],
    )?;

    progress(&format!("Installing {app}…"));
    // Clean any leftover mountpoint from a previous attempt.
    let _ = Command::new("/usr/bin/hdiutil").args(["detach", &mnt_s, "-force"]).output();
    run(
        "mount",
        "/usr/bin/hdiutil",
        &["attach", &dmg_s, "-nobrowse", "-noverify", "-mountpoint", &mnt_s],
    )?;

    // Copy the .app out of the mounted image into /Applications (replace any
    // partial leftover first so cp doesn't merge into a stale bundle).
    let src = mnt.join(format!("{app}.app"));
    let dest = installed_app_path(app);
    let _ = std::fs::remove_dir_all(&dest);
    let copy_result = run(
        "copy",
        "/bin/cp",
        &["-R", &src.to_string_lossy(), "/Applications/"],
    );

    // Always try to unmount, even if the copy failed.
    let _ = Command::new("/usr/bin/hdiutil").args(["detach", &mnt_s, "-force"]).output();
    let _ = std::fs::remove_file(&dmg);
    copy_result?;

    // Clear the quarantine flag so the unsigned-by-us downloaded app opens
    // without a Gatekeeper prompt. Best-effort; never fail the port for it.
    let _ = Command::new("/usr/bin/xattr")
        .args(["-cr", &dest.to_string_lossy()])
        .output();

    Ok(())
}

/// Run the launch recipe.
fn launch(tool: &Tool) -> Result<(), PortError> {
    let (prog, args) = tool.launch.launch_command();
    let output = Command::new(prog)
        .args(args)
        .output()
        .map_err(|e| PortError::LaunchFailed(e.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(PortError::LaunchFailed(tail(&output.stderr)))
    }
}

/// The full one-click port: check → (download + install) → launch, reporting
/// each stage to `progress` so the UI keeps the user informed (skills/design.md).
pub(crate) fn port<F: Fn(&str)>(tool: &Tool, progress: F) -> Result<PortOutcome, PortError> {
    let is_arm64 = crate::device::is_apple_silicon();

    progress(&format!("Checking if {} is already set up…", tool.name));
    let already = is_installed(tool);

    let installed_now = if already {
        false
    } else {
        install(tool, is_arm64, &progress)?;
        true
    };

    progress(&format!("Opening {}…", tool.name));
    launch(tool)?;

    let message = if installed_now {
        format!("{} is set up and open. It runs entirely on your computer.", tool.name)
    } else {
        format!("{} was already set up — opened it for you.", tool.name)
    };
    Ok(PortOutcome {
        installed_now,
        launched: true,
        message,
    })
}

/// Keep the last ~500 bytes of captured output — enough to explain a failure
/// without dumping a wall of text at the user.
fn tail(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    let trimmed = s.trim();
    const MAX: usize = 500;
    if trimmed.len() <= MAX {
        trimmed.to_string()
    } else {
        let start = trimmed.len() - MAX;
        format!("…{}", &trimmed[start..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_app_path_is_in_applications() {
        assert_eq!(
            installed_app_path("Buzz"),
            PathBuf::from("/Applications/Buzz.app")
        );
    }

    #[test]
    fn tail_keeps_short_text_whole() {
        assert_eq!(tail(b"  boom  "), "boom");
    }

    #[test]
    fn tail_truncates_long_text_from_the_front() {
        let long = "x".repeat(1000);
        let out = tail(long.as_bytes());
        assert!(out.starts_with('…'));
        assert_eq!(out.chars().count(), 501); // 500 chars + the ellipsis
    }

    #[test]
    fn errors_render_human() {
        let msg = PortError::InstallFailed("network down".into()).to_string();
        assert!(msg.contains("Setup didn't finish"));
        assert!(!msg.contains("Err"));
    }
}
