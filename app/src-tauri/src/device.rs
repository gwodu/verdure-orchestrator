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

use std::path::Path;
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

/// A LIVE snapshot of the machine's resources — not just what it has, but what
/// is FREE right now. This is the context that lets Verdure orchestrate honestly:
/// pick a model that actually fits the available memory (a model sized to total
/// RAM can still OOM-abort when most of it is already in use — see knowledge/21),
/// warn about disk, and degrade gracefully instead of crashing. "You at the core
/// of context" starts with the machine you're on (knowledge/20).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemResources {
    /// "apple-silicon" | "intel".
    pub arch: String,
    /// Logical CPU cores.
    pub cores: u32,
    /// Total physical RAM, MB.
    pub total_ram_mb: u64,
    /// Memory that can be made available now (free + reclaimable), MB. This is the
    /// number that decides whether a model will fit without aborting.
    pub available_ram_mb: u64,
    /// Free space on the volume holding the app's data (where models download), MB.
    pub free_disk_mb: u64,
    /// macOS Low Power Mode — throttles CPU, worth knowing before a long run.
    pub low_power: bool,
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

/// Take a live snapshot of free resources. `data_dir` is the folder models live
/// in — we report free space on its volume. Every reading degrades safely: if a
/// tool can't be read, that field falls back rather than failing the snapshot.
pub fn system_resources(data_dir: &Path) -> SystemResources {
    let total_ram_mb = sysctl_int("hw.memsize").map(|b| b / 1_048_576).unwrap_or(8192);
    let available_ram_mb = read_available_ram_mb()
        // If vm_stat can't be read, assume half of total is free — conservative
        // enough to avoid a confident-but-wrong "it'll fit".
        .unwrap_or(total_ram_mb / 2);
    let free_disk_mb = read_free_disk_mb(data_dir).unwrap_or(0);
    SystemResources {
        arch: if is_apple_silicon() { "apple-silicon" } else { "intel" }.to_string(),
        cores: sysctl_int("hw.logicalcpu").unwrap_or(4) as u32,
        total_ram_mb,
        available_ram_mb,
        free_disk_mb,
        low_power: read_low_power(),
    }
}

/// Available RAM (MB) from `vm_stat`: free + reclaimable pages × page size.
fn read_available_ram_mb() -> Option<u64> {
    let out = Command::new("/usr/bin/vm_stat").output().ok()?;
    parse_available_bytes(&String::from_utf8_lossy(&out.stdout)).map(|b| b / 1_048_576)
}

/// Free space (MB) on the volume containing `path`, via `df -k`.
fn read_free_disk_mb(path: &Path) -> Option<u64> {
    let out = Command::new("/bin/df")
        .args(["-k", &path.to_string_lossy()])
        .output()
        .ok()?;
    parse_df_avail_kb(&String::from_utf8_lossy(&out.stdout)).map(|kb| kb / 1024)
}

/// True if macOS Low Power Mode is on (`pmset -g`).
fn read_low_power() -> bool {
    Command::new("/usr/bin/pmset")
        .args(["-g"])
        .output()
        .ok()
        .map(|o| parse_low_power(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or(false)
}

/// Parse `vm_stat` output into available bytes (free + inactive + speculative +
/// purgeable pages, the memory macOS can hand out). Pure — unit-tested.
fn parse_available_bytes(vm_stat: &str) -> Option<u64> {
    let page_size: u64 = vm_stat
        .split("page size of ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())?;
    let mut pages = 0u64;
    for line in vm_stat.lines() {
        if ["Pages free:", "Pages inactive:", "Pages speculative:", "Pages purgeable:"]
            .iter()
            .any(|p| line.starts_with(p))
        {
            if let Some(rest) = line.rsplit(':').next() {
                let digits: String = rest.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(v) = digits.parse::<u64>() {
                    pages += v;
                }
            }
        }
    }
    Some(pages * page_size)
}

/// Parse the "Available" column (KB) from `df -k` output. Pure — unit-tested.
fn parse_df_avail_kb(df: &str) -> Option<u64> {
    let data = df.lines().nth(1)?;
    data.split_whitespace().nth(3).and_then(|s| s.parse().ok())
}

/// Parse `pmset -g` for Low Power Mode being on. Pure — unit-tested.
fn parse_low_power(pmset: &str) -> bool {
    pmset
        .lines()
        .any(|l| l.contains("lowpowermode") && l.split_whitespace().last() == Some("1"))
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

    #[test]
    fn parses_available_ram_from_vm_stat() {
        // 16 KB pages; free(100) + inactive(50) + speculative(10) + purgeable(40)
        // = 200 pages × 16384 = 3_276_800 bytes. Active/wired must be ignored.
        let sample = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\n\
            Pages free:                          100.\n\
            Pages active:                       9999.\n\
            Pages inactive:                       50.\n\
            Pages speculative:                    10.\n\
            Pages wired down:                   8888.\n\
            Pages purgeable:                      40.\n";
        assert_eq!(parse_available_bytes(sample), Some(200 * 16384));
    }

    #[test]
    fn available_ram_is_none_without_a_page_size() {
        assert_eq!(parse_available_bytes("garbage output"), None);
    }

    #[test]
    fn parses_df_available_column() {
        let sample = "Filesystem 1024-blocks      Used Available Capacity Mounted on\n\
            /dev/disk3s1  489620264 480000000   2600000      99% /\n";
        // 4th column (Available, KB).
        assert_eq!(parse_df_avail_kb(sample), Some(2_600_000));
    }

    #[test]
    fn detects_low_power_mode() {
        assert!(parse_low_power(" lowpowermode         1"));
        assert!(!parse_low_power(" lowpowermode         0"));
        assert!(!parse_low_power(" hibernatemode        3"));
    }
}
