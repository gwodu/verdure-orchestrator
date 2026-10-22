//! `capabilities()` — what this device honestly is, for the vendor's own
//! logic and support screens. Tier and profile come from `verdure-hw`;
//! `notes` carry the sentences a support agent would want ("downgraded to L1
//! because…"); `attributions` is what the vendor must display for CC-BY
//! weights once a bundle is chosen.

use serde::{Deserialize, Serialize};

use crate::tier::{DeviceProfile, Tier};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    pub tier: Tier,
    pub device: DeviceProfile,
    /// Engines compiled into this build: "whisper", "llama", …
    pub backends: Vec<String>,
    pub attributions: Vec<String>,
    pub notes: Vec<String>,
}

impl Capabilities {
    pub fn from_profile(device: DeviceProfile, backends: Vec<String>) -> Self {
        let tier = device.tier();
        let mut notes = Vec::new();
        if device.low_power {
            notes.push("Low Power Mode is on; generation will be slower.".to_string());
        }
        if device.gpu == crate::tier::GpuClass::Integrated
            && device.arch == "x86_64"
            && device.os == "macos"
        {
            notes.push(
                "Intel Mac: inference runs on the CPU (the GPU path returns wrong numbers here)."
                    .to_string(),
            );
        }
        Capabilities {
            tier,
            device,
            backends,
            attributions: Vec::new(),
            notes,
        }
    }
}
