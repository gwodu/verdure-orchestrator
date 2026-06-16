//! The port catalog — the supply side as DATA.
//!
//! Each entry says how to install and launch one existing open-source tool.
//! Adding a ported tool should be a data entry here, never new control flow —
//! that is the whole point of keeping the recipe declarative (knowledge/15
//! "the supply-side database", knowledge/17 "more ported OSS tools = data
//! entries"). The porter (porter.rs) interprets these recipes; it knows nothing
//! about any specific tool.
//!
//! The command-construction logic lives on the method enums as pure functions
//! so it can be unit-tested without touching the real system.

use std::path::Path;

/// One portable tool: how to get it, how to start it.
pub struct Tool {
    /// Stable id the frontend uses to request a port (e.g. "buzz").
    pub id: &'static str,
    /// Human name shown in the UI.
    pub name: &'static str,
    /// One plain-language line about what it does (jargon-free; skills/design.md).
    pub blurb: &'static str,
    pub install: InstallMethod,
    pub launch: LaunchMethod,
}

/// How to install a tool. One variant today (Homebrew cask) — the obvious
/// extension point for DMG/script/pip ports later. This is data-shaped, not an
/// abstraction layer, so it stays honest to "no abstraction without a second
/// implementation" (skills/architecture.md): when the second install method
/// lands, it becomes a real seam.
pub enum InstallMethod {
    /// A macOS app delivered via a Homebrew cask. `cask` is the cask token.
    HomebrewCask { cask: &'static str },
}

/// How to start an installed tool.
pub enum LaunchMethod {
    /// A macOS .app bundle, launched with `open -a <AppName>`.
    MacOsApp { app_name: &'static str },
}

impl InstallMethod {
    /// The program + args that INSTALL the tool. Pure — takes the resolved brew
    /// path so it can be tested without a real Homebrew on the machine.
    pub fn install_command(&self, brew: &Path) -> (String, Vec<String>) {
        match self {
            InstallMethod::HomebrewCask { cask } => (
                brew.to_string_lossy().into_owned(),
                vec!["install".into(), "--cask".into(), (*cask).into()],
            ),
        }
    }

    /// The program + args that report whether the tool is already installed.
    /// Exit code 0 ⇒ installed. Pure, for the same reason as above.
    pub fn check_command(&self, brew: &Path) -> (String, Vec<String>) {
        match self {
            InstallMethod::HomebrewCask { cask } => (
                brew.to_string_lossy().into_owned(),
                vec!["list".into(), "--cask".into(), (*cask).into()],
            ),
        }
    }
}

impl LaunchMethod {
    /// The program + args that LAUNCH the tool. `open` is always at /usr/bin/open
    /// on macOS, independent of PATH (which is minimal for Finder-launched apps).
    pub fn launch_command(&self) -> (String, Vec<String>) {
        match self {
            LaunchMethod::MacOsApp { app_name } => (
                "/usr/bin/open".into(),
                vec!["-a".into(), (*app_name).into()],
            ),
        }
    }
}

/// The MVP's first (and, for v1.0, only one-click) port: transcription via Buzz.
/// MIT, Whisper-based, runs on a plain Intel CPU — the simplest standalone-app
/// port and the smallest thing that proves the loop (knowledge/17).
const BUZZ: Tool = Tool {
    id: "buzz",
    name: "Buzz",
    blurb: "Turns recordings into text, right on your computer.",
    install: InstallMethod::HomebrewCask { cask: "buzz" },
    launch: LaunchMethod::MacOsApp { app_name: "Buzz" },
};

/// Every tool Verdure can port today.
pub const CATALOG: &[Tool] = &[BUZZ];

/// Look up a tool by its stable id.
pub fn find(id: &str) -> Option<&'static Tool> {
    CATALOG.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finds_buzz_by_id() {
        let tool = find("buzz").expect("buzz is in the catalog");
        assert_eq!(tool.name, "Buzz");
    }

    #[test]
    fn unknown_id_is_none() {
        assert!(find("does-not-exist").is_none());
    }

    #[test]
    fn install_command_is_brew_cask_install() {
        let brew = PathBuf::from("/usr/local/bin/brew");
        let (prog, args) = BUZZ.install.install_command(&brew);
        assert_eq!(prog, "/usr/local/bin/brew");
        assert_eq!(args, vec!["install", "--cask", "buzz"]);
    }

    #[test]
    fn check_command_is_brew_cask_list() {
        let brew = PathBuf::from("/opt/homebrew/bin/brew");
        let (prog, args) = BUZZ.install.check_command(&brew);
        assert_eq!(prog, "/opt/homebrew/bin/brew");
        assert_eq!(args, vec!["list", "--cask", "buzz"]);
    }

    #[test]
    fn launch_command_opens_the_app_by_name() {
        let (prog, args) = BUZZ.launch.launch_command();
        assert_eq!(prog, "/usr/bin/open");
        assert_eq!(args, vec!["-a", "Buzz"]);
    }
}
