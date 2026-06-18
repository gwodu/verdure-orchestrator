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
