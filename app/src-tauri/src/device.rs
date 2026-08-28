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
#[cfg(target_os = "macos")]
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
    /// Which OS this is ("macos" | "ios" | "android"), so the UI can offer the
    /// right way to free disk space — each platform has a different honest answer
    /// ([OQ-67]). From `std::env::consts::OS`, so it can't drift from the build.
    pub os: &'static str,
}

/// Detect this machine's specs for the performance predictor.
#[cfg(target_os = "macos")]
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

/// Detect this machine's specs — iOS. Every iPhone/iPad CPU is Apple ARM, and
/// there are no shell tools to call (apps can't spawn processes), so the readings
/// come from syscalls instead of `/usr/sbin/sysctl` ([OQ-65]).
#[cfg(target_os = "ios")]
pub fn detect_spec() -> DeviceSpec {
    DeviceSpec {
        arch: "apple-silicon".to_string(),
        cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4),
        ram_gb: ios::total_ram_bytes()
            .map(|b| (b as f64 / 1_073_741_824.0).round() as u32)
            .unwrap_or(4),
    }
}

/// Read an integer sysctl value (fixed path, PATH-independent).
#[cfg(target_os = "macos")]
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
#[cfg(target_os = "macos")]
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
        os: std::env::consts::OS,
    }
}

/// Take a live snapshot of free resources — iOS. The key difference from the Mac:
/// `available_ram_mb` is **this app's** remaining allocation headroom before iOS
/// terminates it (jetsam), via `os_proc_available_memory()` — a stricter and more
/// truthful gate than system-wide free RAM, which is exactly what the chat model
/// picker needs to avoid being killed mid-load ([OQ-65], knowledge/11).
#[cfg(target_os = "ios")]
pub fn system_resources(data_dir: &Path) -> SystemResources {
    let total_ram_mb = ios::total_ram_bytes().map(|b| b / 1_048_576).unwrap_or(4096);
    let available_ram_mb = ios::available_ram_bytes()
        .map(|b| b / 1_048_576)
        // If the headroom can't be read, assume a conservative quarter of total —
        // overshooting on iOS means the OS kills the app, not an error dialog.
        .unwrap_or(total_ram_mb / 4);
    SystemResources {
        arch: "apple-silicon".to_string(),
        cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4),
        total_ram_mb,
        available_ram_mb,
        free_disk_mb: ios::free_disk_bytes(data_dir).map(|b| b / 1_048_576).unwrap_or(0),
        // Reading Low Power Mode needs an ObjC bridge (NSProcessInfo) — not worth
        // the dependency for the test build; "off" only skips a speed warning.
        low_power: false,
        os: std::env::consts::OS,
    }
}

/// iOS syscall probes. All readings go through the audited `libc` wrapper crate —
/// no hand-rolled bindings beyond the one documented Apple API below
/// (skills/testing.md, FFI rules 1 & 4).
#[cfg(target_os = "ios")]
mod ios {
    /// Total physical RAM — the same `hw.memsize` fact the Mac reads via the
    /// sysctl *binary*, read via the sysctl *syscall* (no spawning on iOS).
    pub fn total_ram_bytes() -> Option<u64> {
        let name = std::ffi::CString::new("hw.memsize").ok()?;
        let mut value: u64 = 0;
        let mut size = std::mem::size_of::<u64>();
        // SAFETY: sysctlbyname writes at most `size` bytes into `value`, which we
        // own on the stack; `name` outlives the call; nothing is allocated across
        // the boundary, so there is nothing to free.
        let rc = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                &mut value as *mut u64 as *mut libc::c_void,
                &mut size,
                std::ptr::null_mut(),
                0,
            )
        };
        (rc == 0).then_some(value)
    }

    // The one hand-written binding: Apple's public API (os/proc.h, iOS 13+) for
    // "how much can THIS process still allocate before jetsam kills it". Takes no
    // arguments, returns a plain integer — no ownership crosses the boundary.
    extern "C" {
        fn os_proc_available_memory() -> usize;
    }

    /// This app's remaining allocation headroom, in bytes. `None` when the OS
    /// reports 0 (e.g. simulator), so callers fall back conservatively.
    pub fn available_ram_bytes() -> Option<u64> {
        // SAFETY: no-argument call returning an integer by value; nothing owned.
        let bytes = unsafe { os_proc_available_memory() } as u64;
        (bytes > 0).then_some(bytes)
    }

    /// Free space on the volume holding `path`, via statvfs (there is no `df`).
    pub fn free_disk_bytes(path: &std::path::Path) -> Option<u64> {
        let c = std::ffi::CString::new(path.to_str()?).ok()?;
        let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: statvfs fills the struct we own on the stack; `c` outlives the
        // call; nothing is allocated across the boundary.
        let rc = unsafe { libc::statvfs(c.as_ptr(), &mut sv) };
        (rc == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
    }
}

/// Detect this machine's specs — Android. An ARM device; readings come from
/// Linux syscalls/procfs, no shell tools ([OQ-66]).
#[cfg(target_os = "android")]
pub fn detect_spec() -> DeviceSpec {
    DeviceSpec {
        arch: "arm".to_string(),
        cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4),
        ram_gb: android::total_ram_bytes()
            .map(|b| (b as f64 / 1_073_741_824.0).round() as u32)
            .unwrap_or(4),
    }
}

/// Take a live snapshot of free resources — Android. `available_ram_mb` here is an
/// *allocatable budget*, not raw `MemAvailable`. On Android that distinction
/// matters: `/proc/meminfo MemAvailable` systematically UNDERSTATES what a
/// foreground app can use, because the low-memory killer reclaims background apps
/// on demand when the foreground app allocates — memory `MemAvailable` doesn't
/// count. Gating on the raw number wrongly blocked every model on a phone that
/// reported ~1 GB free (observed on-device 2026-07-31). So we budget against total
/// RAM with headroom for the OS + our WebView, and never report less than the real
/// `MemAvailable`. Still an estimate to calibrate on real devices ([OQ-66]).
#[cfg(target_os = "android")]
pub fn system_resources(data_dir: &Path) -> SystemResources {
    let total_ram_mb = android::total_ram_bytes().map(|b| b / 1_048_576).unwrap_or(4096);
    let mem_available_mb = android::available_ram_mb().unwrap_or(total_ram_mb / 4);
    // ~60% of total is a realistic ceiling a foreground app can claim once Android
    // reclaims cache + background apps; keep the honest MemAvailable as a floor.
    let available_ram_mb = (total_ram_mb * 6 / 10).max(mem_available_mb);
    SystemResources {
        arch: "arm".to_string(),
        cores: std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4),
        total_ram_mb,
        available_ram_mb,
        free_disk_mb: android::free_disk_bytes(data_dir).map(|b| b / 1_048_576).unwrap_or(0),
        // Reading battery-saver state needs a Java bridge; not worth it for the
        // test build — "off" only skips a speed warning.
        low_power: false,
        os: std::env::consts::OS,
    }
}

/// Android syscall/procfs probes. Everything goes through the audited `libc`
/// wrapper crate — no hand-rolled bindings (skills/testing.md, FFI rules 1 & 4).
#[cfg(target_os = "android")]
mod android {
    /// Total physical RAM via the sysinfo(2) syscall.
    pub fn total_ram_bytes() -> Option<u64> {
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        // SAFETY: sysinfo fills the struct we own on the stack; nothing is
        // allocated across the boundary, so there is nothing to free.
        let rc = unsafe { libc::sysinfo(&mut info) };
        (rc == 0).then(|| (info.totalram as u64) * (info.mem_unit as u64))
    }

    /// Memory the kernel can hand out right now: MemAvailable from /proc/meminfo
    /// (readable inside the app sandbox; no syscall wrapper needed).
    pub fn available_ram_mb() -> Option<u64> {
        let text = std::fs::read_to_string("/proc/meminfo").ok()?;
        super::parse_meminfo_available_kb(&text).map(|kb| kb / 1024)
    }

    /// Free space on the volume holding `path`, via statvfs (same call as iOS).
    pub fn free_disk_bytes(path: &std::path::Path) -> Option<u64> {
        let c = std::ffi::CString::new(path.to_str()?).ok()?;
        let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
        // SAFETY: statvfs fills the struct we own on the stack; `c` outlives the
        // call; nothing is allocated across the boundary.
        let rc = unsafe { libc::statvfs(c.as_ptr(), &mut sv) };
        (rc == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
    }
}

/// Parse `MemAvailable:` (kB) out of /proc/meminfo. Pure — unit-tested.
#[cfg(any(target_os = "android", test))]
fn parse_meminfo_available_kb(meminfo: &str) -> Option<u64> {
    meminfo
        .lines()
        .find(|l| l.starts_with("MemAvailable:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
}

/// Detect the device's tier — Android. ARM phones/tablets vary widely; a middle
/// default keeps the legacy recommendation honest without overpromising. The RAM
/// gate does the real per-device sizing at model-pick time ([OQ-66]).
#[cfg(target_os = "android")]
pub fn detect_tier() -> &'static str {
    "L2-high"
}

/// Never on Android — used by the legacy porter to pick Mac download flavors.
#[cfg(target_os = "android")]
pub(crate) fn is_apple_silicon() -> bool {
    false
}

/// Available RAM (MB) from `vm_stat`: free + reclaimable pages × page size.
#[cfg(target_os = "macos")]
fn read_available_ram_mb() -> Option<u64> {
    let out = Command::new("/usr/bin/vm_stat").output().ok()?;
    parse_available_bytes(&String::from_utf8_lossy(&out.stdout)).map(|b| b / 1_048_576)
}

/// Free space (MB) on the volume containing `path`, via `df -k`.
#[cfg(target_os = "macos")]
fn read_free_disk_mb(path: &Path) -> Option<u64> {
    let out = Command::new("/bin/df")
        .args(["-k", &path.to_string_lossy()])
        .output()
        .ok()?;
    parse_df_avail_kb(&String::from_utf8_lossy(&out.stdout)).map(|kb| kb / 1024)
}

/// True if macOS Low Power Mode is on (`pmset -g`).
#[cfg(target_os = "macos")]
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
///
/// **Why not "Pages free".** On a working Mac free is almost nothing — 15 MB was
/// measured on the founder's 8 GB machine while 2224 MB was genuinely available.
/// macOS deliberately fills spare RAM with file cache and reclaims it on demand,
/// so *reclaimable* memory is the honest number and "free" would gate out every
/// model. This is also why a "clear cache to free memory" button would be a
/// placebo (decided 2026-08-28): that memory is already counted here, and
/// flushing it only makes the machine slower by discarding warm cache.
///
/// **Why purgeable is NOT added (fixed 2026-08-28, `[OQ-77]`).** It used to be,
/// which double-counted: purgeable pages already sit in the active/inactive
/// lists. Dropping the term is not an arbitrary trim — it makes this function
/// agree exactly with macOS's own accounting, because the page categories
/// partition physical memory:
///
/// ```text
/// total = free + active + inactive + speculative + wired + compressor
/// ⇒  free + inactive + speculative  ≡  total − (active + wired + compressor)
/// ```
///
/// and the right-hand side is what Activity Monitor reports as available. Both
/// forms were computed on the founder's Mac and agreed to the megabyte (1895 MB,
/// against 1976 MB for the old formula), with the categories summing to 8190 MB
/// on an 8 GB machine — so the identity holds against real data, not just on
/// paper. `available_ram_is_the_complement_of_used` locks it in.
///
/// **Residual imprecision, still optimistic:** *dirty* inactive pages are counted
/// as available although they must be compressed or paged out first. That is
/// bounded and much smaller than the double-count was, and correcting it is not
/// possible from `vm_stat` alone. Deliberately left rather than papered over with
/// a fudge factor: a safety margin belongs in `model.rs`'s `runtime_mb`, where it
/// is visible and adjustable, not hidden inside a measurement (knowledge/06 —
/// honest numbers).
#[cfg(any(target_os = "macos", test))]
fn parse_available_bytes(vm_stat: &str) -> Option<u64> {
    let page_size: u64 = vm_stat
        .split("page size of ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())?;
    let mut pages = 0u64;
    // NOT "Pages purgeable" — see the doc comment. Purgeable pages already sit in
    // the active/inactive lists, so adding them double-counts.
    for line in vm_stat.lines() {
        if ["Pages free:", "Pages inactive:", "Pages speculative:"]
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
#[cfg(any(target_os = "macos", test))]
fn parse_df_avail_kb(df: &str) -> Option<u64> {
    let data = df.lines().nth(1)?;
    data.split_whitespace().nth(3).and_then(|s| s.parse().ok())
}

/// Parse `pmset -g` for Low Power Mode being on. Pure — unit-tested.
#[cfg(any(target_os = "macos", test))]
fn parse_low_power(pmset: &str) -> bool {
    pmset
        .lines()
        .any(|l| l.contains("lowpowermode") && l.split_whitespace().last() == Some("1"))
}

/// Detect the device's tier as a decider `Tier` string. Always returns a safe,
/// real tier — never fails to the user.
#[cfg(target_os = "macos")]
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

/// Detect the device's tier — iOS. Every iPad/iPhone is Apple Silicon-class for
/// the decider's purposes; the RAM gate (jetsam-aware, above) does the real
/// per-device sizing at pick time ([OQ-65]).
#[cfg(target_os = "ios")]
pub fn detect_tier() -> &'static str {
    "L3"
}

/// True on Apple Silicon. `sysctl hw.optional.arm64` is "1" there, absent/"0"
/// on Intel. sysctl lives at a fixed path, so this works under the minimal PATH
/// a Finder-launched app inherits. Also used by the porter to pick the right
/// architecture's download (x64 vs ARM64).
#[cfg(target_os = "macos")]
pub(crate) fn is_apple_silicon() -> bool {
    Command::new("/usr/sbin/sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

/// True on Apple Silicon — iOS: always (there are no Intel iPads).
#[cfg(target_os = "ios")]
pub(crate) fn is_apple_silicon() -> bool {
    true
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
        // 16 KB pages; free(100) + inactive(50) + speculative(10) = 160 pages.
        // Active and wired must be ignored — and so must PURGEABLE, which is not
        // a separate pool: those pages are already inside active/inactive, so
        // adding them counted the same memory twice ([OQ-77], fixed 2026-08-28).
        let sample = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\n\
            Pages free:                          100.\n\
            Pages active:                       9999.\n\
            Pages inactive:                       50.\n\
            Pages speculative:                    10.\n\
            Pages wired down:                   8888.\n\
            Pages purgeable:                      40.\n";
        assert_eq!(parse_available_bytes(sample), Some(160 * 16384));
    }

    /// The reason the fix above is correct rather than merely smaller: macOS's
    /// page categories partition physical memory, so what we compute is exactly
    /// the complement of what is in use — which is what Activity Monitor shows.
    /// If someone ever re-adds `purgeable` (it is a tempting-looking line in
    /// `vm_stat`), this fails and says why.
    #[test]
    fn available_ram_is_the_complement_of_used() {
        const PAGE: u64 = 4096;
        let (free, active, inactive, speculative, wired, compressor, purgeable) =
            (3_u64, 514_u64, 506_u64, 9_u64, 691_u64, 287_u64, 24_u64);
        let sample = format!(
            "Mach Virtual Memory Statistics: (page size of {PAGE} bytes)\n\
             Pages free:                {free}.\n\
             Pages active:              {active}.\n\
             Pages inactive:            {inactive}.\n\
             Pages speculative:         {speculative}.\n\
             Pages wired down:          {wired}.\n\
             Pages purgeable:           {purgeable}.\n\
             Pages occupied by compressor: {compressor}.\n"
        );
        let total = (free + active + inactive + speculative + wired + compressor) * PAGE;
        let used = (active + wired + compressor) * PAGE;
        assert_eq!(
            parse_available_bytes(&sample),
            Some(total - used),
            "available must equal total minus what is in use; adding purgeable \
             breaks this because those pages are already in active/inactive"
        );
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

    #[test]
    fn parses_meminfo_available() {
        // The Android available-RAM probe: MemAvailable, in kB. Other fields
        // (MemFree, MemTotal) must be ignored — MemAvailable is the kernel's own
        // "can hand out now" estimate, the honest gating number.
        let sample = "MemTotal:        8000000 kB\n\
            MemFree:          200000 kB\n\
            MemAvailable:    3500000 kB\n\
            Buffers:          100000 kB\n";
        assert_eq!(parse_meminfo_available_kb(sample), Some(3_500_000));
        assert_eq!(parse_meminfo_available_kb("MemTotal: 8000000 kB"), None);
        assert_eq!(parse_meminfo_available_kb(""), None);
    }
}
