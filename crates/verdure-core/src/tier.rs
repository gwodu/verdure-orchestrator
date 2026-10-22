//! Device tiers — the vocabulary of knowledge/02, kept as the code's names.
//!
//! The SDK spec (knowledge/26) arrived with its own labels (T0/T1 phones,
//! L0/L1/L2 laptops). One vocabulary rule (`skills/architecture.md`): the
//! knowledge base's names win. The mapping, recorded once here and in `26`:
//!
//! | spec | here | what it is |
//! |---|---|---|
//! | T0 | `P1` | 4 GB phone, Unisoc T606 class (Itel A70) |
//! | T1 | `P2` / `P3` | 6–8 GB phone, Helio G85/G99, Dimensity 6300, iPhone 12+ |
//! | L0 | `L1` | 4 GB laptop, Intel gen 6–8 |
//! | L1 | `L2` | 8 GB laptop: refurb Intel MacBook, school-lab PC |
//! | L2 | `L3` | 16 GB+ Apple Silicon or dGPU |

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    P1,
    P2,
    P3,
    L1,
    L2,
    L3,
}

impl Tier {
    pub fn form_factor(self) -> FormFactor {
        match self {
            Tier::P1 | Tier::P2 | Tier::P3 => FormFactor::Phone,
            Tier::L1 | Tier::L2 | Tier::L3 => FormFactor::Laptop,
        }
    }

    /// This tier and every lower tier of the same form factor, highest first —
    /// the order `recommend()` walks when the top bundle doesn't fit.
    pub fn descending(self) -> Vec<Tier> {
        let family: &[Tier] = match self.form_factor() {
            FormFactor::Phone => &[Tier::P3, Tier::P2, Tier::P1],
            FormFactor::Laptop => &[Tier::L3, Tier::L2, Tier::L1],
        };
        family.iter().copied().filter(|t| *t <= self).collect()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::P1 => "P1",
            Tier::P2 => "P2",
            Tier::P3 => "P3",
            Tier::L1 => "L1",
            Tier::L2 => "L2",
            Tier::L3 => "L3",
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Tier {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_ascii_uppercase().as_str() {
            "P1" => Tier::P1,
            "P2" => Tier::P2,
            "P3" => Tier::P3,
            "L1" => Tier::L1,
            "L2" => Tier::L2,
            "L3" => Tier::L3,
            other => return Err(format!("unknown tier {other}")),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormFactor {
    Phone,
    Laptop,
}

/// What the GPU is, as far as it changes which engine path is trustworthy.
/// `AppleSilicon` means Metal is correct AND fast; `Integrated` on x86 means
/// Metal/Vulkan may be present but the CPU is the honest choice (the Intel-Mac
/// Metal bug, knowledge/21 `[OQ-71]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuClass {
    None,
    Integrated,
    AppleSilicon,
    Discrete,
}

/// A normalised reading of the device, produced by `verdure-hw`. Numbers are
/// what the OS can honestly hand out *now*, not spec-sheet totals — the app's
/// gates size everything from `available_ram_mb` (knowledge/21, 22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceProfile {
    /// "macos" | "ios" | "android" | "windows" | "linux".
    pub os: String,
    /// "aarch64" | "x86_64" | …
    pub arch: String,
    pub form_factor: FormFactor,
    pub cores: u32,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub free_disk_mb: u64,
    pub gpu: GpuClass,
    pub low_power: bool,
}

impl DeviceProfile {
    /// knowledge/02's tier for this device. RAM and form factor decide; Apple
    /// Silicon lifts a phone one tier because an A-series chip at 4 GB does
    /// what a Unisoc does at 6 (the spec puts the iPhone 12 in T1, not T0).
    pub fn tier(&self) -> Tier {
        let gb = self.total_ram_mb as f64 / 1024.0;
        match self.form_factor {
            FormFactor::Phone => {
                let base = if gb <= 4.5 {
                    Tier::P1
                } else if gb <= 6.5 {
                    Tier::P2
                } else {
                    Tier::P3
                };
                if self.gpu == GpuClass::AppleSilicon {
                    match base {
                        Tier::P1 => Tier::P2,
                        _ => Tier::P3,
                    }
                } else {
                    base
                }
            }
            FormFactor::Laptop => {
                if gb < 6.0 {
                    Tier::L1
                } else if gb < 12.0 {
                    Tier::L2
                } else {
                    Tier::L3
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(os: &str, ff: FormFactor, ram_mb: u64, gpu: GpuClass) -> DeviceProfile {
        DeviceProfile {
            os: os.into(),
            arch: "aarch64".into(),
            form_factor: ff,
            cores: 8,
            total_ram_mb: ram_mb,
            available_ram_mb: ram_mb / 2,
            free_disk_mb: 10_000,
            gpu,
            low_power: false,
        }
    }

    #[test]
    fn the_spec_device_lab_lands_in_knowledge_02_tiers() {
        // T0 → P1
        assert_eq!(
            dev("android", FormFactor::Phone, 4096, GpuClass::Integrated).tier(),
            Tier::P1
        );
        // T1 → P2 / P3
        assert_eq!(
            dev("android", FormFactor::Phone, 6144, GpuClass::Integrated).tier(),
            Tier::P2
        );
        assert_eq!(
            dev("android", FormFactor::Phone, 8192, GpuClass::Integrated).tier(),
            Tier::P3
        );
        // iPhone 12 (4 GB) is T1 in the spec: Apple Silicon lifts it.
        assert_eq!(
            dev("ios", FormFactor::Phone, 3900, GpuClass::AppleSilicon).tier(),
            Tier::P2
        );
        assert_eq!(
            dev("ios", FormFactor::Phone, 6144, GpuClass::AppleSilicon).tier(),
            Tier::P3
        );
        // L0 → L1, L1 → L2, L2 → L3
        assert_eq!(
            dev("linux", FormFactor::Laptop, 4096, GpuClass::Integrated).tier(),
            Tier::L1
        );
        assert_eq!(
            dev("macos", FormFactor::Laptop, 8192, GpuClass::Integrated).tier(),
            Tier::L2
        );
        assert_eq!(
            dev("macos", FormFactor::Laptop, 16384, GpuClass::AppleSilicon).tier(),
            Tier::L3
        );
        // An 8 GB M1 Air is still L2 by RAM — honest, the model that matters is RAM-bound.
        assert_eq!(
            dev("macos", FormFactor::Laptop, 8192, GpuClass::AppleSilicon).tier(),
            Tier::L2
        );
    }

    #[test]
    fn descending_walks_down_within_the_family() {
        assert_eq!(Tier::L3.descending(), vec![Tier::L3, Tier::L2, Tier::L1]);
        assert_eq!(Tier::P2.descending(), vec![Tier::P2, Tier::P1]);
        assert_eq!(Tier::L1.descending(), vec![Tier::L1]);
    }

    #[test]
    fn tiers_parse_and_print() {
        for t in [Tier::P1, Tier::P2, Tier::P3, Tier::L1, Tier::L2, Tier::L3] {
            assert_eq!(t.to_string().parse::<Tier>().unwrap(), t);
        }
        assert!(
            "T0".parse::<Tier>().is_err(),
            "the spec's labels are not the code's"
        );
    }
}
