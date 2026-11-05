//! Honest device readings → `DeviceProfile` (knowledge/26 §4.5; the port is
//! `DeviceProbe` in knowledge/14). Each OS reads *available* memory its own
//! truthful way, lifted from the app's `device.rs` where the macOS formula
//! was checked against Activity Monitor to the megabyte and the Android one
//! was corrected on a real phone (knowledge/21):
//!
//! * macOS — `vm_stat` free + inactive + speculative pages (never "free"
//!   alone, never + purgeable: `[OQ-77]`), `df`, `pmset`.
//! * iOS — `os_proc_available_memory()`: this app's headroom before jetsam,
//!   stricter and truer than system-wide free.
//! * Android — a budget of 60 % of total with `MemAvailable` as a floor,
//!   because the low-memory killer frees what `MemAvailable` doesn't count.
//! * Linux — `/proc/meminfo` `MemAvailable`, statvfs.
//! * Windows — `sysinfo`: physical memory the system can hand out now (what
//!   Task Manager calls "Available"), and free space on the volume the data
//!   folder is on, found by the longest matching mount point. First run on a
//!   real Windows machine: the SDK's Windows build (knowledge/26, "Windows").
//!
//! No fingerprint identifiers are read, ever: nothing here identifies the
//! device, only sizes it (knowledge/06).

use std::path::Path;

use verdure_core::{DeviceProfile, FormFactor, GpuClass};

/// Read the device. `data_dir` is where bundles will live, so free disk is
/// measured on its volume. Every reading degrades to a conservative default
/// rather than failing the probe.
pub fn probe(data_dir: &Path) -> DeviceProfile {
    platform::probe(data_dir)
}

/// `std::env::consts::ARCH` normalised to the two names the tiers use.
fn arch() -> String {
    match std::env::consts::ARCH {
        "aarch64" | "arm64" => "aarch64".to_string(),
        other => other.to_string(),
    }
}

fn cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

// ---- pure parsers (unit-tested on every platform) ----------------------

/// `vm_stat` → available bytes: free + inactive + speculative pages. Purgeable
/// is deliberately NOT added — those pages already sit in active/inactive, so
/// adding them double-counts; this form equals total − (active + wired +
/// compressor), which is what Activity Monitor shows (app `device.rs`).
pub fn parse_vm_stat_available_bytes(vm_stat: &str) -> Option<u64> {
    let page_size: u64 = vm_stat
        .split("page size of ")
        .nth(1)
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.parse().ok())?;
    let mut pages = 0u64;
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

/// The "Available" column (KB) of `df -k`.
pub fn parse_df_avail_kb(df: &str) -> Option<u64> {
    df.lines()
        .nth(1)?
        .split_whitespace()
        .nth(3)
        .and_then(|s| s.parse().ok())
}

/// `pmset -g` says Low Power Mode is on.
pub fn parse_low_power(pmset: &str) -> bool {
    pmset
        .lines()
        .any(|l| l.contains("lowpowermode") && l.split_whitespace().last() == Some("1"))
}

/// `MemAvailable:` (kB) from `/proc/meminfo`.
pub fn parse_meminfo_kb(meminfo: &str, key: &str) -> Option<u64> {
    meminfo
        .lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
}

/// Android's allocatable budget: 60 % of total **minus what this process already
/// holds**, never below `MemAvailable`. The first Pixel run (2026-11-03) reported
/// the same 4543 MB "available" before and after loading 1.3 GB of models — the
/// budget ignored our own resident set, so a second `load` would have been
/// admitted into memory we were already using.
pub fn android_budget_mb(total_mb: u64, mem_available_mb: u64, self_resident_mb: u64) -> u64 {
    (total_mb * 6 / 10)
        .saturating_sub(self_resident_mb)
        .max(mem_available_mb)
}

/// Which of `mounts` holds `path`, by the longest mount point that is a
/// prefix of it. Windows paths, compared the way Windows compares them: no
/// case, `/` and `\` alike, and the `\\?\` form that canonicalised paths
/// carry treated as the plain path. Returns the index into `mounts`.
///
/// The first version asked `path.starts_with(mount)` on the path as given. A
/// relative data folder (what a Python script passes) or a `\\?\C:\…` one
/// (what `canonicalize` returns) matched no drive, free disk read as 0 MB,
/// and every bundle was refused for lack of space on a disk with room.
pub fn windows_volume_for(path: &str, mounts: &[&str]) -> Option<usize> {
    fn norm(p: &str) -> String {
        let p = p.replace('/', "\\");
        let p = match p.strip_prefix("\\\\?\\UNC\\") {
            Some(rest) => format!("\\\\{rest}"),
            None => p.strip_prefix("\\\\?\\").unwrap_or(&p).to_string(),
        };
        let mut p = p.to_lowercase();
        if !p.ends_with('\\') {
            p.push('\\');
        }
        p
    }
    let path = norm(path);
    mounts
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.is_empty() && path.starts_with(&norm(m)))
        .max_by_key(|(_, m)| norm(m).len())
        .map(|(i, _)| i)
}

/// What the compiled engines need from an x86-64 processor, and whether this
/// one has it. `None` when it can run them; otherwise a sentence for a log.
///
/// The Windows library is built for the level every x86-64 processor since
/// 2013–2015 has (AVX2, FMA, F16C, BMI2 — Intel Haswell, AMD Excavator/Zen):
/// whisper.cpp and llama.cpp use those instructions unconditionally, so on an
/// older or budget processor (Sandy/Ivy Bridge, pre-2021 Celeron and Pentium,
/// Atom) the first inference would kill the host process with an illegal
/// instruction. The SDK refuses to load there and says why. The build level
/// is asserted on the shipped library by `build-sdk-windows.yml`, so this
/// rule and the binary cannot drift apart silently.
pub fn x86_engine_shortfall(avx2: bool, fma: bool, f16c: bool, bmi2: bool) -> Option<String> {
    let missing: Vec<&str> = [("AVX2", avx2), ("FMA", fma), ("F16C", f16c), ("BMI2", bmi2)]
        .iter()
        .filter(|(_, has)| !has)
        .map(|(name, _)| *name)
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "this processor cannot run the on-device models: it lacks {} (needed: an x86-64 processor from about 2014 or later)",
            missing.join(", ")
        )
    })
}

/// Can the engines in this build run on this processor? `None` = yes.
/// Checked where we ship one x86-64 binary to processors we have never seen
/// (Windows); elsewhere the build is made for the machine or the platform
/// fixes the instruction set (arm64 phones, Apple Silicon).
pub fn engine_cpu_shortfall() -> Option<String> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        x86_engine_shortfall(
            std::is_x86_feature_detected!("avx2"),
            std::is_x86_feature_detected!("fma"),
            std::is_x86_feature_detected!("f16c"),
            std::is_x86_feature_detected!("bmi2"),
        )
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    {
        None
    }
}

/// This process's resident set in MB from `/proc/self/statm` (second field,
/// pages). Linux and Android only; `None` elsewhere or if unreadable.
#[cfg(any(target_os = "android", target_os = "linux"))]
fn self_resident_mb() -> Option<u64> {
    let statm = std::fs::read_to_string("/proc/self/statm").ok()?;
    let pages: u64 = statm.split_whitespace().nth(1)?.parse().ok()?;
    // SAFETY: sysconf takes an integer name and returns an integer; nothing owned.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let page = if page > 0 { page as u64 } else { 4096 };
    Some(pages * page / 1_048_576)
}

#[cfg(any(target_os = "ios", target_os = "android", target_os = "linux"))]
fn statvfs_free_bytes(path: &Path) -> Option<u64> {
    let c = std::ffi::CString::new(path.to_str()?).ok()?;
    // SAFETY: `statvfs` is a plain C struct of integers, so all-zero is a valid
    // start; statvfs fills the struct we own on the stack; `c` outlives the
    // call; nothing is allocated across the boundary.
    let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
    let rc = unsafe { libc::statvfs(c.as_ptr(), &mut sv) };
    (rc == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::process::Command;

    fn run(bin: &str, args: &[&str]) -> Option<String> {
        Command::new(bin)
            .args(args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    }

    fn sysctl_int(key: &str) -> Option<u64> {
        run("/usr/sbin/sysctl", &["-n", key]).and_then(|s| s.trim().parse().ok())
    }

    pub fn probe(data_dir: &Path) -> DeviceProfile {
        let total_ram_mb = sysctl_int("hw.memsize")
            .map(|b| b / 1_048_576)
            .unwrap_or(8192);
        let available_ram_mb = run("/usr/bin/vm_stat", &[])
            .and_then(|s| parse_vm_stat_available_bytes(&s))
            .map(|b| b / 1_048_576)
            .unwrap_or(total_ram_mb / 2);
        let free_disk_mb = run("/bin/df", &["-k", &data_dir.to_string_lossy()])
            .and_then(|s| parse_df_avail_kb(&s))
            .map(|kb| kb / 1024)
            .unwrap_or(0);
        let apple_silicon = sysctl_int("hw.optional.arm64") == Some(1);
        DeviceProfile {
            os: "macos".into(),
            arch: arch(),
            form_factor: FormFactor::Laptop,
            cores: sysctl_int("hw.logicalcpu").unwrap_or(cores() as u64) as u32,
            total_ram_mb,
            available_ram_mb,
            free_disk_mb,
            gpu: if apple_silicon {
                GpuClass::AppleSilicon
            } else {
                GpuClass::Integrated
            },
            low_power: run("/usr/bin/pmset", &["-g"])
                .map(|s| parse_low_power(&s))
                .unwrap_or(false),
        }
    }
}

#[cfg(target_os = "ios")]
mod platform {
    use super::*;

    fn total_ram_bytes() -> Option<u64> {
        let name = std::ffi::CString::new("hw.memsize").ok()?;
        let mut value: u64 = 0;
        let mut size = std::mem::size_of::<u64>();
        // SAFETY: sysctlbyname writes at most `size` bytes into `value`, which
        // we own; `name` outlives the call; nothing is allocated across it.
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

    // Apple's public API (os/proc.h, iOS 13+): how much THIS process may still
    // allocate before jetsam. No arguments, integer by value — nothing owned.
    extern "C" {
        fn os_proc_available_memory() -> usize;
    }

    pub fn probe(data_dir: &Path) -> DeviceProfile {
        let total_ram_mb = total_ram_bytes().map(|b| b / 1_048_576).unwrap_or(4096);
        // SAFETY: see the declaration — a plain integer return.
        let headroom = unsafe { os_proc_available_memory() } as u64 / 1_048_576;
        DeviceProfile {
            os: "ios".into(),
            arch: arch(),
            form_factor: FormFactor::Phone,
            cores: cores(),
            total_ram_mb,
            // 0 on the simulator → a conservative quarter; overshooting on
            // iOS means the OS kills the app, not an error dialog.
            available_ram_mb: if headroom > 0 {
                headroom
            } else {
                total_ram_mb / 4
            },
            free_disk_mb: statvfs_free_bytes(data_dir)
                .map(|b| b / 1_048_576)
                .unwrap_or(0),
            gpu: GpuClass::AppleSilicon,
            low_power: false,
        }
    }
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;

    pub fn probe(data_dir: &Path) -> DeviceProfile {
        // SAFETY: `sysinfo` is a plain C struct of integers; all-zero is a valid
        // start; the syscall fills the struct we own on the stack.
        let mut info: libc::sysinfo = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::sysinfo(&mut info) };
        let total_ram_mb = if rc == 0 {
            (info.totalram as u64) * (info.mem_unit as u64) / 1_048_576
        } else {
            4096
        };
        let mem_available_mb = std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|s| parse_meminfo_kb(&s, "MemAvailable:"))
            .map(|kb| kb / 1024)
            .unwrap_or(total_ram_mb / 4);
        DeviceProfile {
            os: "android".into(),
            arch: arch(),
            form_factor: FormFactor::Phone,
            cores: cores(),
            total_ram_mb,
            available_ram_mb: android_budget_mb(
                total_ram_mb,
                mem_available_mb,
                self_resident_mb().unwrap_or(0),
            ),
            free_disk_mb: statvfs_free_bytes(data_dir)
                .map(|b| b / 1_048_576)
                .unwrap_or(0),
            gpu: GpuClass::Integrated,
            low_power: false,
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    pub fn probe(data_dir: &Path) -> DeviceProfile {
        let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let total_ram_mb = parse_meminfo_kb(&meminfo, "MemTotal:")
            .map(|kb| kb / 1024)
            .unwrap_or(8192);
        let available_ram_mb = parse_meminfo_kb(&meminfo, "MemAvailable:")
            .map(|kb| kb / 1024)
            .unwrap_or(total_ram_mb / 2);
        DeviceProfile {
            os: "linux".into(),
            arch: arch(),
            form_factor: FormFactor::Laptop,
            cores: cores(),
            total_ram_mb,
            available_ram_mb,
            free_disk_mb: statvfs_free_bytes(data_dir)
                .map(|b| b / 1_048_576)
                .unwrap_or(0),
            // A discrete GPU is not detected yet (`[OQ-109]`); the CPU path is
            // the honest default on the school-lab machines we target.
            gpu: GpuClass::Integrated,
            low_power: false,
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    pub fn probe(data_dir: &Path) -> DeviceProfile {
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let total_ram_mb = sys.total_memory() / 1_048_576;
        let available_ram_mb = sys.available_memory() / 1_048_576;
        // The data folder as an absolute path (a relative one is resolved
        // against the working directory, as the OS will when files are
        // written), then the volume whose mount point is its longest prefix.
        let abs = std::path::absolute(data_dir).unwrap_or_else(|_| data_dir.to_path_buf());
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mounts: Vec<String> = disks
            .iter()
            .map(|d| d.mount_point().to_string_lossy().into_owned())
            .collect();
        let mount_refs: Vec<&str> = mounts.iter().map(String::as_str).collect();
        let free_disk_mb = windows_volume_for(&abs.to_string_lossy(), &mount_refs)
            .and_then(|i| disks.iter().nth(i))
            .map(|d| d.available_space() / 1_048_576)
            .unwrap_or(0);
        DeviceProfile {
            os: "windows".into(),
            arch: arch(),
            form_factor: FormFactor::Laptop,
            cores: cores(),
            total_ram_mb,
            available_ram_mb,
            free_disk_mb,
            gpu: GpuClass::Integrated,
            low_power: false,
        }
    }
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "linux",
    target_os = "windows"
)))]
mod platform {
    use super::*;
    pub fn probe(_data_dir: &Path) -> DeviceProfile {
        DeviceProfile {
            os: std::env::consts::OS.into(),
            arch: arch(),
            form_factor: FormFactor::Laptop,
            cores: cores(),
            total_ram_mb: 4096,
            available_ram_mb: 2048,
            free_disk_mb: 0,
            gpu: GpuClass::None,
            low_power: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_stat_available_is_the_complement_of_used() {
        const PAGE: u64 = 4096;
        let (free, active, inactive, speculative, wired, compressor, purgeable) =
            (3u64, 514u64, 506u64, 9u64, 691u64, 287u64, 24u64);
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
        assert_eq!(parse_vm_stat_available_bytes(&sample), Some(total - used));
        assert_eq!(parse_vm_stat_available_bytes("garbage"), None);
    }

    #[test]
    fn df_pmset_and_meminfo_parse() {
        let df = "Filesystem 1024-blocks      Used Available Capacity Mounted on\n/dev/disk3s1  489620264 480000000   2600000      99% /\n";
        assert_eq!(parse_df_avail_kb(df), Some(2_600_000));
        assert!(parse_low_power(" lowpowermode         1"));
        assert!(!parse_low_power(
            " lowpowermode         0\n hibernatemode 3"
        ));
        let mi = "MemTotal:        8000000 kB\nMemFree:          200000 kB\nMemAvailable:    3500000 kB\n";
        assert_eq!(parse_meminfo_kb(mi, "MemAvailable:"), Some(3_500_000));
        assert_eq!(parse_meminfo_kb(mi, "MemTotal:"), Some(8_000_000));
        assert_eq!(parse_meminfo_kb("", "MemAvailable:"), None);
    }

    #[test]
    fn android_budget_never_reports_less_than_mem_available() {
        // The on-device correction: a 4 GB phone reporting ~1 GB MemAvailable
        // wrongly blocked every model; the budget is 60 % with that as a floor.
        assert_eq!(android_budget_mb(4000, 1000, 0), 2400);
        assert_eq!(android_budget_mb(4000, 3000, 0), 3000);
    }

    #[test]
    fn android_budget_shrinks_by_what_this_process_already_holds() {
        // The Pixel 8a numbers: 7573 MB total → 4543 budget; after loading 1.3 GB
        // (PSS 683 MB idle, 952 MB during extract) the figure must fall, not stay.
        assert_eq!(android_budget_mb(7573, 2000, 0), 4543);
        assert_eq!(android_budget_mb(7573, 2000, 683), 3860);
        assert_eq!(android_budget_mb(7573, 2000, 952), 3591);
        // Never below the kernel's own MemAvailable, and never underflows.
        assert_eq!(android_budget_mb(4000, 1500, 2400), 1500);
        assert_eq!(android_budget_mb(4000, 100, 9000), 100);
    }

    #[test]
    fn a_windows_data_folder_finds_its_volume_however_it_is_written() {
        let mounts = ["C:\\", "D:\\", "C:\\mnt\\models\\"];
        // Plain, lower-case drive, forward slashes, no trailing slash.
        assert_eq!(
            windows_volume_for("C:\\Users\\Ada\\AppData\\Local\\app", &mounts),
            Some(0)
        );
        assert_eq!(windows_volume_for("d:/data/verdure", &mounts), Some(1));
        // What `canonicalize` returns on Windows.
        assert_eq!(
            windows_volume_for("\\\\?\\C:\\Users\\Ada", &mounts),
            Some(0)
        );
        // A volume mounted in a folder wins over the drive it sits on.
        assert_eq!(
            windows_volume_for("C:\\mnt\\models\\verdure", &mounts),
            Some(2)
        );
        // ...but a folder that merely starts with the same letters does not.
        assert_eq!(
            windows_volume_for("C:\\mnt\\models2\\verdure", &mounts),
            Some(0)
        );
        // Spaces and non-ASCII names are just path characters.
        assert_eq!(
            windows_volume_for("C:\\Users\\José Álvarez\\数据", &mounts),
            Some(0)
        );
        // A network share has no local volume: unknown, not "drive C".
        assert_eq!(
            windows_volume_for("\\\\server\\share\\verdure", &mounts),
            None
        );
        assert_eq!(
            windows_volume_for("\\\\?\\UNC\\server\\share\\x", &["\\\\server\\share\\"]),
            Some(0)
        );
        assert_eq!(windows_volume_for("E:\\x", &mounts), None);
        assert_eq!(windows_volume_for("C:\\x", &[]), None);
    }

    #[test]
    fn an_old_processor_is_refused_with_what_it_lacks() {
        assert_eq!(x86_engine_shortfall(true, true, true, true), None);
        // Ivy Bridge (2012): AVX and F16C, but no AVX2, FMA or BMI2.
        let msg = x86_engine_shortfall(false, false, true, false).unwrap();
        assert!(msg.contains("AVX2, FMA, BMI2"), "{msg}");
        assert!(!msg.contains("F16C"), "{msg}");
        assert!(msg.contains("2014"), "{msg}");
        // One missing feature is enough.
        assert!(x86_engine_shortfall(true, false, true, true)
            .unwrap()
            .contains("FMA"));
    }

    #[test]
    fn this_machine_can_run_the_engines_it_was_built_for() {
        // Off Windows there is no rule; on Windows x86-64 the CI runner (and
        // any machine the library is tested on) must pass it.
        assert_eq!(engine_cpu_shortfall(), None);
    }

    #[test]
    fn the_live_probe_returns_sane_numbers_on_this_machine() {
        let p = probe(&std::env::temp_dir());
        assert!(p.total_ram_mb >= 1024, "{p:?}");
        assert!(p.available_ram_mb <= p.total_ram_mb, "{p:?}");
        assert!(p.cores >= 1);
        assert_eq!(p.os, std::env::consts::OS);
        // The temp folder is on a real volume with room for a test run: 0 here
        // means the volume was not found, which refuses every bundle.
        assert!(p.free_disk_mb > 0, "{p:?}");
        // The tier is derivable from any live reading.
        let _ = p.tier();
    }
}
