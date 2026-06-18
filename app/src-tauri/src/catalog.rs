//! The port catalog — the supply side as DATA.
//!
//! Each entry says how to install and launch one existing open-source tool.
//! Adding a ported tool should be a data entry here, never new control flow —
//! that is the whole point of keeping the recipe declarative (knowledge/15
//! "the supply-side database", knowledge/17 "more ported OSS tools = data
//! entries"). The porter (porter.rs) interprets these recipes; it knows nothing
//! about any specific tool.
//!
//! Install method = a DIRECT .dmg download from the tool's official release,
//! NOT Homebrew. Homebrew requires the Xcode Command Line Tools (it shells out
//! to `xcrun`/`git`) and isn't present on a non-technical user's machine — which
//! defeats the whole "one click, no toolchain" promise. A direct download +
//! drag-to-Applications uses only base macOS tools (knowledge/15 "standalone-app
//! port"). See [OQ-51] about resolving "latest" instead of a pinned version.

/// One portable tool: how to get it, how to start it.
pub struct Tool {
    /// Stable id the frontend uses to request a port (e.g. "buzz").
    pub id: &'static str,
    /// Human name shown in the UI.
    pub name: &'static str,
    /// One plain-language line about what it does (jargon-free; skills/design.md).
    /// Not yet surfaced by the backend (the frontend has its own copy); kept as
    /// tool metadata for when the porter exposes a catalog listing.
    #[allow(dead_code)]
    pub blurb: &'static str,
    pub install: InstallMethod,
    pub launch: LaunchMethod,
}

/// How to install a tool. One variant today (a macOS .dmg) — the obvious
/// extension point for Windows installers / Linux packages later. Data-shaped,
/// not an abstraction layer, so it stays honest to "no abstraction without a
/// second implementation" (skills/architecture.md).
pub enum InstallMethod {
    /// Download a macOS .dmg straight from the tool's official release and copy
    /// the .app into /Applications. Arch-specific URLs (the founder's Mac is x64).
    MacDmg {
        url_x64: &'static str,
        url_arm64: &'static str,
    },
}

/// How to start an installed tool.
pub enum LaunchMethod {
    /// A macOS .app bundle, launched with `open -a <AppName>`. The app name is
    /// also the bundle name the porter copies out of the .dmg.
    MacOsApp { app_name: &'static str },
}

impl InstallMethod {
    /// The official download URL for this machine's architecture. Pure — takes
    /// the arch so it can be unit-tested without probing the host.
    pub fn download_url(&self, is_arm64: bool) -> &'static str {
        match self {
            InstallMethod::MacDmg { url_x64, url_arm64 } => {
                if is_arm64 {
                    url_arm64
                } else {
                    url_x64
                }
            }
        }
    }
}

impl LaunchMethod {
    /// The .app bundle name (without extension) — used both to launch and to
    /// locate the bundle inside a mounted .dmg.
    pub fn app_name(&self) -> &'static str {
        match self {
            LaunchMethod::MacOsApp { app_name } => app_name,
        }
    }

    /// The program + args that LAUNCH the tool. `open` is always at /usr/bin/open
    /// on macOS, independent of PATH (which is minimal for Finder-launched apps).
    pub fn launch_command(&self) -> (String, Vec<String>) {
        match self {
            LaunchMethod::MacOsApp { app_name } => {
                ("/usr/bin/open".into(), vec!["-a".into(), (*app_name).into()])
            }
        }
    }
}

/// The MVP's first (and, for v1.0, only one-click) port: transcription via Buzz.
/// MIT, Whisper-based, ships an official macOS .dmg per arch — the simplest
/// standalone-app port and the smallest thing that proves the loop (knowledge/17).
const BUZZ: Tool = Tool {
    id: "buzz",
    name: "Buzz",
    blurb: "Turns recordings into text, right on your computer.",
    install: InstallMethod::MacDmg {
        url_x64: "https://github.com/chidiwilliams/buzz/releases/download/v1.4.4/Buzz-1.4.4-mac-X64.dmg",
        url_arm64: "https://github.com/chidiwilliams/buzz/releases/download/v1.4.4/Buzz-1.4.4-mac-ARM64.dmg",
    },
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
    fn download_url_picks_the_right_arch() {
        let x64 = BUZZ.install.download_url(false);
        let arm = BUZZ.install.download_url(true);
        assert!(x64.contains("X64") && x64.ends_with(".dmg"));
        assert!(arm.contains("ARM64") && arm.ends_with(".dmg"));
        assert_ne!(x64, arm);
    }

    #[test]
    fn launch_command_opens_the_app_by_name() {
        let (prog, args) = BUZZ.launch.launch_command();
        assert_eq!(prog, "/usr/bin/open");
        assert_eq!(args, vec!["-a", "Buzz"]);
    }

    #[test]
    fn app_name_is_buzz() {
        assert_eq!(BUZZ.launch.app_name(), "Buzz");
    }
}
