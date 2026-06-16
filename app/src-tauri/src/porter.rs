//! The porter — execute a tool's recipe: install it (if needed), then launch it.
//!
//! This is the "one click" in "ports it in one click" (knowledge/17). It knows
//! how to RUN recipes; the recipes themselves (and their command construction)
//! live in catalog.rs. Keeping execution here and construction there means the
//! testable logic (what command to run) is pure, and this file only does I/O.
//!
//! macOS reality this file exists to handle: an app launched from Finder gets a
//! minimal PATH that usually excludes Homebrew, so we must locate `brew`
//! explicitly rather than trust PATH.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::catalog::Tool;

/// The result of a successful port, reported back to the UI.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortOutcome {
    /// True if we installed it just now; false if it was already present.
    pub installed_now: bool,
    /// True once the launch command was issued successfully.
    pub launched: bool,
    /// A short, human, jargon-free summary for the UI.
    pub message: String,
}

/// Everything that can go wrong, in language we can show a non-technical user.
#[derive(Debug)]
pub enum PortError {
    /// Homebrew isn't installed — the one prerequisite for cask ports today.
    BrewMissing,
    /// The install step ran but failed; carries the captured stderr tail.
    InstallFailed(String),
    /// The launch step failed.
    LaunchFailed(String),
}

impl std::fmt::Display for PortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortError::BrewMissing => write!(
                f,
                "Homebrew is needed to set this up automatically. You can install it from brew.sh, then try again."
            ),
            PortError::InstallFailed(detail) => {
                write!(f, "Setup didn't finish. {detail}")
            }
            PortError::LaunchFailed(detail) => {
                write!(f, "It installed, but couldn't open automatically. {detail}")
            }
        }
    }
}

impl std::error::Error for PortError {}

/// Locate the `brew` executable without relying on PATH. Checks the two standard
/// Homebrew prefixes (Apple Silicon, then Intel), then falls back to PATH.
pub fn find_brew() -> Option<PathBuf> {
    const KNOWN: [&str; 2] = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];
    for candidate in KNOWN {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    // Last resort: trust PATH (covers non-standard installs).
    Command::new("brew")
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|_| PathBuf::from("brew"))
}

/// Is the tool already installed? Runs the recipe's check command; exit 0 ⇒ yes.
fn is_installed(tool: &Tool, brew: &Path) -> bool {
    let (prog, args) = tool.install.check_command(brew);
    Command::new(prog)
        .args(args)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Run the install recipe. Captures stderr so a failure can be explained.
fn install(tool: &Tool, brew: &Path) -> Result<(), PortError> {
    let (prog, args) = tool.install.install_command(brew);
    let output = Command::new(prog)
        .args(args)
        .output()
        .map_err(|e| PortError::InstallFailed(e.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(PortError::InstallFailed(tail(&output.stderr)))
    }
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

/// The full one-click port: check → (install) → launch, reporting each stage to
/// `progress` so the UI can keep the user informed (skills/design.md "Feedback").
pub fn port<F: Fn(&str)>(tool: &Tool, progress: F) -> Result<PortOutcome, PortError> {
    let brew = find_brew().ok_or(PortError::BrewMissing)?;

    progress(&format!("Checking if {} is already set up…", tool.name));
    let already = is_installed(tool, &brew);

    let installed_now = if already {
        false
    } else {
        progress(&format!(
            "Setting up {} — downloading and installing. This can take a minute…",
            tool.name
        ));
        install(tool, &brew)?;
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
    fn brew_missing_message_is_human() {
        let msg = PortError::BrewMissing.to_string();
        assert!(msg.contains("Homebrew"));
        assert!(!msg.contains("Err")); // no Rust jargon leaking to the user
    }
}
