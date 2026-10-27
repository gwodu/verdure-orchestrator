//! The bundle registry: what a `manifest.json` says, and how `recommend()`
//! turns (task, device, licence) into one bundle — or an honest "nothing
//! fits" with the reasons (knowledge/26 §4.4).
//!
//! A manifest is data a vendor hosts and signs (`verdure-download` fetches
//! and verifies it). Weights are never inside an app binary (Play and App
//! Store size rules, knowledge/26 Part 2); the manifest tells the runtime
//! where each file is, how big, and its SHA-256.

use serde::{Deserialize, Serialize};

use crate::license::LicenseClaims;
use crate::memory;
use crate::task::Task;
use crate::tier::Tier;
use crate::VError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelKind {
    Asr,
    Llm,
    Embedder,
}

/// One downloadable model file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRef {
    pub id: String,
    pub kind: ModelKind,
    /// Jargon-free name for the vendor's own UI, e.g. "English speech model".
    pub name: String,
    /// File name inside the bundle directory.
    pub file: String,
    pub url: String,
    /// Hex SHA-256 of the file.
    pub sha256: String,
    pub size_bytes: u64,
    /// Engine hint: "whisper" | "llama" | "onnx" | …
    pub engine: String,
    /// Chat-prompt family for LLMs ("chatml", "chatml_nothink", "gemma").
    #[serde(default)]
    pub prompt_style: Option<String>,
    #[serde(default)]
    pub kv_kb_per_token: u32,
    #[serde(default)]
    pub trained_ctx: u32,
    /// SPDX-ish licence id: "apache-2.0", "mit", "cc-by-4.0", "llama-community", "gemma-terms".
    pub licence: String,
}

impl ModelRef {
    pub fn size_mb(&self) -> u64 {
        self.size_bytes / 1_048_576
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub task: Task,
    /// Device tiers this bundle is built for.
    pub tiers: Vec<Tier>,
    pub models: Vec<ModelRef>,
    /// Model licences that need the vendor's acknowledgement in the licence
    /// token before this bundle may be used (Llama, Gemma — knowledge/09).
    #[serde(default)]
    pub requires_acknowledgement: Vec<String>,
    /// Attribution strings the vendor must surface (CC-BY Parakeet).
    #[serde(default)]
    pub attributions: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

impl Bundle {
    pub fn download_bytes(&self) -> u64 {
        self.models.iter().map(|m| m.size_bytes).sum()
    }
    pub fn model(&self, kind: ModelKind) -> Option<&ModelRef> {
        self.models.iter().find(|m| m.kind == kind)
    }
    /// Resident memory needed with the LLM loaded and the ASR beside it. The
    /// rule is applied to the LLM (the big one); the ASR and embedder add
    /// their size. Memory pressure unloads the LLM first (knowledge/26).
    pub fn required_mb(&self) -> u64 {
        let llm = self
            .model(ModelKind::Llm)
            .map(|m| memory::required_mb(m.size_mb()))
            .unwrap_or(400);
        let others: u64 = self
            .models
            .iter()
            .filter(|m| m.kind != ModelKind::Llm)
            .map(|m| m.size_mb() + 100)
            .sum();
        llm + others
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// Manifest format version; this crate reads 1.
    pub schema: u32,
    pub issued_at: u64,
    pub bundles: Vec<Bundle>,
}

impl Manifest {
    pub fn parse(json: &[u8]) -> Result<Manifest, VError> {
        let m: Manifest = serde_json::from_slice(json)
            .map_err(|e| VError::InvalidInput(format!("manifest: {e}")))?;
        if m.schema != 1 {
            return Err(VError::InvalidInput(format!(
                "manifest schema {} is not supported (this runtime reads 1)",
                m.schema
            )));
        }
        Ok(m)
    }
    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.bundles.iter().find(|b| b.id == id)
    }
}

/// What `recommend()` hands back: the chosen bundle and the story of how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleRef {
    pub id: String,
    pub task: Task,
    /// The tier the bundle was built for (≤ the device's tier).
    pub tier: Tier,
    pub download_bytes: u64,
    pub required_mb: u64,
    pub attributions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub bundle: BundleRef,
    /// Set when a bundle built for a lower tier than the device's was chosen.
    pub downgraded_from: Option<Tier>,
    /// Why each better-placed bundle was passed over, one sentence each —
    /// the text a vendor's support screen shows.
    pub skipped: Vec<String>,
}

/// Disk headroom kept free after a download (MB).
const DISK_MARGIN_MB: u64 = 500;

/// Everything that would stop `bundle` from being used on this device, one
/// sentence each; empty means it fits. One function, so `recommend()`'s
/// walk and a developer's explicit pick cannot disagree about what "fits".
pub fn problems(
    bundle: &Bundle,
    available_ram_mb: u64,
    free_disk_mb: u64,
    licence: &LicenseClaims,
) -> Vec<String> {
    let mut out = Vec::new();
    if !licence.entitles(&bundle.id) {
        out.push(format!("{}: not included in this licence", bundle.id));
    }
    for missing in bundle
        .requires_acknowledgement
        .iter()
        .filter(|a| !licence.acknowledgements.contains(a))
    {
        out.push(format!(
            "{}: the {missing} licence has not been acknowledged",
            bundle.id
        ));
    }
    let needed = bundle.required_mb();
    if needed > available_ram_mb {
        out.push(format!(
            "{}: needs {needed} MB of memory, {available_ram_mb} MB available",
            bundle.id
        ));
    }
    let disk_needed = bundle.download_bytes() / 1_048_576 + DISK_MARGIN_MB;
    if disk_needed > free_disk_mb {
        out.push(format!(
            "{}: needs {disk_needed} MB of storage, {free_disk_mb} MB free",
            bundle.id
        ));
    }
    out
}

/// A developer's explicit pick, judged: the bundle, the numbers, and every
/// problem spelled out. Unlike `resolve`, nothing is substituted — the
/// developer asked for this bundle and gets the truth about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FitReport {
    pub bundle: BundleRef,
    pub fits: bool,
    pub available_ram_mb: u64,
    pub free_disk_mb: u64,
    pub problems: Vec<String>,
}

/// Judge one named bundle for this device and licence.
pub fn fit(
    manifest: &Manifest,
    bundle_id: &str,
    available_ram_mb: u64,
    free_disk_mb: u64,
    licence: &LicenseClaims,
) -> Result<FitReport, VError> {
    let b = manifest
        .bundle(bundle_id)
        .ok_or_else(|| VError::NoBundle(format!("{bundle_id} is not in the manifest")))?;
    let problems = problems(b, available_ram_mb, free_disk_mb, licence);
    Ok(FitReport {
        bundle: bundle_ref(b, *b.tiers.iter().max().unwrap_or(&Tier::P1)),
        fits: problems.is_empty(),
        available_ram_mb,
        free_disk_mb,
        problems,
    })
}

fn bundle_ref(b: &Bundle, tier: Tier) -> BundleRef {
    BundleRef {
        id: b.id.clone(),
        task: b.task,
        tier,
        download_bytes: b.download_bytes(),
        required_mb: b.required_mb(),
        attributions: b.attributions.clone(),
    }
}

/// Pick the best bundle for `task` on a device of `device_tier` with
/// `available_ram_mb` and `free_disk_mb`, within what `licence` allows.
/// Walks the tier down and explains every refusal.
pub fn resolve(
    manifest: &Manifest,
    task: Task,
    device_tier: Tier,
    available_ram_mb: u64,
    free_disk_mb: u64,
    licence: &LicenseClaims,
) -> Result<Resolution, VError> {
    let mut reasons: Vec<String> = Vec::new();
    for tier in device_tier.descending() {
        let candidates = manifest
            .bundles
            .iter()
            .filter(|b| b.task == task && b.tiers.contains(&tier));
        for b in candidates {
            let mut p = problems(b, available_ram_mb, free_disk_mb, licence);
            if !p.is_empty() {
                // One sentence per bundle in the walk: the first problem is
                // the decisive one (licence before memory before disk).
                reasons.push(p.remove(0));
                continue;
            }
            let downgraded_from = (tier != device_tier).then_some(device_tier);
            return Ok(Resolution {
                bundle: bundle_ref(b, tier),
                downgraded_from,
                skipped: reasons,
            });
        }
    }
    if reasons.is_empty() {
        reasons.push(format!(
            "the manifest has no {task} bundle for {} devices",
            device_tier.form_factor_name()
        ));
    }
    Err(VError::NoBundle(reasons.join("; ")))
}

impl Tier {
    fn form_factor_name(self) -> &'static str {
        match self.form_factor() {
            crate::tier::FormFactor::Phone => "phone",
            crate::tier::FormFactor::Laptop => "laptop",
        }
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub fn model(id: &str, kind: ModelKind, mb: u64, licence: &str) -> ModelRef {
        ModelRef {
            id: id.into(),
            kind,
            name: id.into(),
            file: format!("{id}.bin"),
            url: format!("https://cdn.example.org/{id}.bin"),
            sha256: "00".repeat(32),
            size_bytes: mb * 1_048_576,
            engine: match kind {
                ModelKind::Asr => "whisper".into(),
                _ => "llama".into(),
            },
            prompt_style: matches!(kind, ModelKind::Llm).then(|| "chatml".to_string()),
            kv_kb_per_token: 12,
            trained_ctx: 32_768,
            licence: licence.into(),
        }
    }

    /// Three scribe bundles down the laptop ladder and one phone bundle.
    pub fn manifest() -> Manifest {
        Manifest {
            schema: 1,
            issued_at: 1,
            bundles: vec![
                Bundle {
                    id: "scribe-L3".into(),
                    task: Task::Scribe,
                    tiers: vec![Tier::L3],
                    models: vec![
                        model("whisper-large-turbo", ModelKind::Asr, 1600, "mit"),
                        model("qwen3-8b", ModelKind::Llm, 5000, "apache-2.0"),
                    ],
                    requires_acknowledgement: vec![],
                    attributions: vec![],
                    notes: String::new(),
                },
                Bundle {
                    id: "scribe-L2".into(),
                    task: Task::Scribe,
                    tiers: vec![Tier::L2],
                    models: vec![
                        model("whisper-small", ModelKind::Asr, 480, "mit"),
                        model("llama-3.2-3b", ModelKind::Llm, 2000, "llama-community"),
                    ],
                    requires_acknowledgement: vec!["llama-community".into()],
                    attributions: vec![],
                    notes: String::new(),
                },
                Bundle {
                    id: "scribe-L1".into(),
                    task: Task::Scribe,
                    tiers: vec![Tier::L1, Tier::L2],
                    models: vec![
                        model("whisper-base", ModelKind::Asr, 150, "mit"),
                        model("qwen2.5-1.5b", ModelKind::Llm, 1000, "apache-2.0"),
                    ],
                    requires_acknowledgement: vec![],
                    attributions: vec![],
                    notes: String::new(),
                },
                Bundle {
                    id: "chw-P1".into(),
                    task: Task::ChwForms,
                    tiers: vec![Tier::P1, Tier::P2],
                    models: vec![
                        model("moonshine-tiny", ModelKind::Asr, 30, "mit"),
                        model("qwen2.5-1.5b", ModelKind::Llm, 1000, "apache-2.0"),
                    ],
                    requires_acknowledgement: vec![],
                    attributions: vec!["Speech model: Moonshine (MIT)".into()],
                    notes: String::new(),
                },
            ],
        }
    }

    pub fn licence_all() -> LicenseClaims {
        LicenseClaims {
            vendor_id: "test".into(),
            seats: 0,
            issued_at: 0,
            expires_at: 0,
            bundles: vec!["*".into()],
            acknowledgements: vec!["llama-community".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn the_top_bundle_is_chosen_when_it_fits() {
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Tier::L3,
            16_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(r.bundle.id, "scribe-L3");
        assert_eq!(r.bundle.tier, Tier::L3);
        assert!(r.downgraded_from.is_none() && r.skipped.is_empty());
        // 5000 × 1.5 + 400 for the LLM, plus 1600 + 100 for the ASR.
        assert_eq!(r.bundle.required_mb, 7900 + 1700);
    }

    #[test]
    fn a_tight_device_walks_down_and_says_why() {
        // An L3 machine with only 3 GB free right now (a browser is open).
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Tier::L3,
            3_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(r.bundle.id, "scribe-L1", "{r:?}");
        assert_eq!(r.downgraded_from, Some(Tier::L3));
        assert_eq!(
            r.skipped[0],
            "scribe-L3: needs 9600 MB of memory, 3000 MB available"
        );
        assert!(
            r.skipped[1].starts_with("scribe-L2: needs"),
            "{:?}",
            r.skipped
        );
    }

    #[test]
    fn licence_entitlement_and_acknowledgements_gate_bundles() {
        let only_l1 = LicenseClaims {
            bundles: vec!["scribe-L1".into()],
            ..licence_all()
        };
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Tier::L3,
            16_000,
            20_000,
            &only_l1,
        )
        .unwrap();
        assert_eq!(r.bundle.id, "scribe-L1");
        assert_eq!(r.downgraded_from, Some(Tier::L3));
        assert_eq!(
            r.skipped,
            vec![
                "scribe-L3: not included in this licence",
                "scribe-L2: not included in this licence"
            ]
        );

        let no_ack = LicenseClaims {
            acknowledgements: vec![],
            ..licence_all()
        };
        let r = resolve(&manifest(), Task::Scribe, Tier::L2, 8_000, 20_000, &no_ack).unwrap();
        assert_eq!(
            r.bundle.id, "scribe-L1",
            "the Llama bundle needs an acknowledgement"
        );
        // scribe-L1 is built for L2 too, so this is not a downgrade — but the
        // vendor still learns why the Llama bundle was passed over.
        assert_eq!(r.downgraded_from, None);
        assert_eq!(
            r.skipped,
            vec!["scribe-L2: the llama-community licence has not been acknowledged"]
        );
    }

    #[test]
    fn disk_is_checked_with_a_margin_and_nothing_fits_is_explained() {
        let err = resolve(
            &manifest(),
            Task::Scribe,
            Tier::L1,
            4_000,
            1_000,
            &licence_all(),
        )
        .unwrap_err();
        match err {
            VError::NoBundle(why) => assert!(
                why.contains("scribe-L1: needs 1650 MB of storage, 1000 MB free"),
                "{why}"
            ),
            other => panic!("{other:?}"),
        }
        let err = resolve(
            &manifest(),
            Task::Tutor,
            Tier::L3,
            16_000,
            20_000,
            &licence_all(),
        )
        .unwrap_err();
        assert_eq!(
            err,
            VError::NoBundle("the manifest has no tutor bundle for laptop devices".into())
        );
        // A phone never gets a laptop bundle, even a small one.
        let err = resolve(
            &manifest(),
            Task::Scribe,
            Tier::P3,
            6_000,
            20_000,
            &licence_all(),
        )
        .unwrap_err();
        assert!(matches!(err, VError::NoBundle(w) if w.contains("phone")));
    }

    #[test]
    fn a_bundle_may_serve_several_tiers() {
        let r = resolve(
            &manifest(),
            Task::ChwForms,
            Tier::P2,
            3_000,
            5_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(r.bundle.id, "chw-P1");
        assert_eq!(
            r.bundle.tier,
            Tier::P2,
            "built for P2 as well, so not a downgrade"
        );
        assert!(r.downgraded_from.is_none() && r.skipped.is_empty());
        assert_eq!(
            r.bundle.attributions,
            vec!["Speech model: Moonshine (MIT)".to_string()]
        );
    }

    #[test]
    fn a_developer_pick_is_judged_not_substituted() {
        // The L3 bundle on a tight machine: resolve would walk down; fit says no, with numbers.
        let r = fit(&manifest(), "scribe-L3", 3_000, 20_000, &licence_all()).unwrap();
        assert!(!r.fits);
        assert_eq!(r.bundle.id, "scribe-L3");
        assert_eq!(
            r.problems,
            vec!["scribe-L3: needs 9600 MB of memory, 3000 MB available"]
        );
        // Every problem is listed, not just the first.
        let no_ack = LicenseClaims {
            bundles: vec!["scribe-L1".into()],
            acknowledgements: vec![],
            ..licence_all()
        };
        let r = fit(&manifest(), "scribe-L2", 1_000, 100, &no_ack).unwrap();
        assert_eq!(r.problems.len(), 4, "{:?}", r.problems);
        // A pick that fits reports so, with the tier it was built for.
        let r = fit(&manifest(), "chw-P1", 3_000, 5_000, &licence_all()).unwrap();
        assert!(r.fits && r.problems.is_empty());
        assert_eq!(r.bundle.tier, Tier::P2, "the highest tier it serves");
        assert!(matches!(
            fit(&manifest(), "nope", 1, 1, &licence_all()),
            Err(VError::NoBundle(_))
        ));
    }

    #[test]
    fn manifests_parse_and_refuse_unknown_schemas() {
        let json = serde_json::to_vec(&manifest()).unwrap();
        assert_eq!(Manifest::parse(&json).unwrap(), manifest());
        let mut v: serde_json::Value = serde_json::from_slice(&json).unwrap();
        v["schema"] = 2.into();
        assert!(
            matches!(Manifest::parse(&serde_json::to_vec(&v).unwrap()), Err(VError::InvalidInput(m)) if m.contains("schema 2"))
        );
        assert!(Manifest::parse(b"{").is_err());
    }
}
