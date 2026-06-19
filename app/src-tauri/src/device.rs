//! Device detection — pick the user's hardware tier WITHOUT asking them a
//! technical question (skills/design.md, "Assume zero technical knowledge").
//!
//! The website asks the user to pick a device card; the desktop porter can do
//! better — it is already running on the machine, so it detects instead of asks,
//! removing a whole screen. The result is one of the decider's Tier strings
//! (knowledge/02-hardware-tiers.md), fed straight into `recommend()`.
//!
//! Provisional and deliberately coarse for the MVP: Apple Silicon vs Intel is
//! the one distinction that changes which tools are feasible on a Mac (it drives
//! the decider's DeviceClass). RAM-level refinement within a tier is a known
//! follow-up — see knowledge/17 "Small known follow-ups".

use std::process::Command;

use serde::Serialize;

/// The detected machine specs the performance predictor needs (mirrors the
/// decider's `DeviceSpec`). Detected, never asked (skills/design.md).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSpec {
    /// "apple-silicon" | "intel" (a Mac is one of these).
    pub arch: String,
    /// Logical CPU cores.
    pub cores: u32,
    /// Total RAM in GB.
    pub ram_gb: u32,
}

/// Detect this machine's specs for the performance predictor.
pub fn detect_spec() -> DeviceSpec {
    DeviceSpec {
        arch: if is_apple_silicon() {
            "apple-silicon".to_string()
        } else {
            "intel".to_string()
        },
        cores: sysctl_int("hw.logicalcpu").unwrap_or(4) as u32,
        ram_gb: sysctl_int("hw.memsize")
            .map(|b| (b as f64 / 1_073_741_824.0).round() as u32)
            .unwrap_or(8),
    }
}

/// Read an integer sysctl value (fixed path, PATH-independent).
fn sysctl_int(key: &str) -> Option<u64> {
    Command::new("/usr/sbin/sysctl")
        .args(["-n", key])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
}

/// Detect the device's tier as a decider `Tier` string. Always returns a safe,
/// real tier — never fails to the user.
pub fn detect_tier() -> &'static str {
    if is_apple_silicon() {
        // Apple Silicon → DeviceClass apple-silicon (knowledge/03, matrix.ts).
        "L3"
    } else {
        // Intel Mac (the founder's 2019 machine) → a capable CPU laptop. This
        // keeps transcription/text "recommended" and graphics-bound work honest.
        "L2-high"
    }
}

/// True on Apple Silicon. `sysctl hw.optional.arm64` is "1" there, absent/"0"
/// on Intel. sysctl lives at a fixed path, so this works under the minimal PATH
/// a Finder-launched app inherits. Also used by the porter to pick the right
/// architecture's download (x64 vs ARM64).
pub(crate) fn is_apple_silicon() -> bool {
    Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detected_tier_is_a_known_tier() {
        // Whatever this machine is, detection must yield a tier the decider
        // recognizes — never an empty or bogus string.
        let tier = detect_tier();
        assert!(
            ["L1", "L2-low", "L2-high", "L3", "L3+"].contains(&tier),
            "unexpected tier: {tier}"
        );
    }
}
