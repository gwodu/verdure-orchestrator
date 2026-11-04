//! The bundle registry: what a `manifest.json` says, and how `recommend()`
//! turns (task, device, licence) into one bundle — or an honest "nothing
//! fits" with the reasons (knowledge/26 §4.4).
//!
//! A manifest is data a vendor hosts and signs (`verdure-download` fetches
//! and verifies it). Weights are never inside an app binary (Play and App
//! Store size rules, knowledge/26 Part 2); the manifest tells the runtime
//! where each file is, how big, and its SHA-256.

use std::collections::BTreeMap;

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

/// How soon the transcript must arrive — the one question about speed a
/// developer answers. `Live`: it keeps pace with the speaker (an ambient
/// scribe, dictation). `Recorded`: the speech is already captured and a
/// transcript later is fine, so a slower, better model may be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pace {
    Live,
    Recorded,
}

/// Processing seconds per second of speech above which a bundle is not
/// offered for `Pace::Live`. The manifest's `speed` numbers are measured
/// flat out and sustained (a throttled phone), and real speech has pauses:
/// on the Pixel 8a, Whisper base at 1.25× kept pace through a live visit
/// while Whisper small at 3.2× finished twenty minutes after a ten-minute
/// one (knowledge/26, "Small on the Pixel"). Provisional — one phone, two
/// bundles (`[OQ-120]`).
pub const LIVE_MAX_SPEED: f32 = 1.5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub task: Task,
    /// Device tiers this bundle is built for.
    pub tiers: Vec<Tier>,
    pub models: Vec<ModelRef>,
    /// Measured speech-recognition speed per tier, as × real time
    /// (processing seconds per second of speech, fed flat out, sustained),
    /// keyed by tier name: `{"P3": 3.2, "L2": 0.75}`. A tier that was not
    /// measured is simply absent, and nothing is assumed about it.
    #[serde(default)]
    pub speed: BTreeMap<String, f32>,
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
    /// The vendor's measured speed on `tier` (× real time), if measured.
    pub fn speed_on(&self, tier: Tier) -> Option<f32> {
        self.speed.get(tier.as_str()).copied()
    }
    /// Whether the bundle keeps pace with live speech on `tier`: `None`
    /// when unmeasured — an unknown is not held against a bundle.
    pub fn keeps_pace_on(&self, tier: Tier) -> Option<bool> {
        self.speed_on(tier).map(|s| s <= LIVE_MAX_SPEED)
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        for b in &m.bundles {
            for (tier, speed) in &b.speed {
                let t: Tier = tier.parse().map_err(|_| {
                    VError::InvalidInput(format!(
                        "manifest: bundle {}: speed names {tier:?}, which is not a tier",
                        b.id
                    ))
                })?;
                if !b.tiers.contains(&t) {
                    return Err(VError::InvalidInput(format!(
                        "manifest: bundle {}: speed is given for {tier}, a tier the bundle is not built for",
                        b.id
                    )));
                }
                if !(speed.is_finite() && *speed > 0.0) {
                    return Err(VError::InvalidInput(format!(
                        "manifest: bundle {}: speed on {tier} must be a positive number of × real time, not {speed}",
                        b.id
                    )));
                }
            }
        }
        Ok(m)
    }
    pub fn bundle(&self, id: &str) -> Option<&Bundle> {
        self.bundles.iter().find(|b| b.id == id)
    }
}

/// What `recommend()` hands back: the chosen bundle and the story of how.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BundleRef {
    pub id: String,
    pub task: Task,
    /// The tier the bundle was built for (≤ the device's tier).
    pub tier: Tier,
    pub download_bytes: u64,
    pub required_mb: u64,
    pub attributions: Vec<String>,
    /// Measured speech speed on `tier`, × real time; `None` = not measured.
    pub speed: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
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

/// Everything that would stop `bundle` from being used on this device for
/// this `pace`, one sentence each; empty means it fits. One function, so
/// `recommend()`'s walk and a developer's explicit pick cannot disagree
/// about what "fits". `tier` is the one the speed is judged on.
pub fn problems(
    bundle: &Bundle,
    tier: Tier,
    pace: Pace,
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
    if pace == Pace::Live && bundle.keeps_pace_on(tier) == Some(false) {
        out.push(format!(
            "{}: runs at {:.1}× real time on {tier}, too slow to keep pace with live speech",
            bundle.id,
            bundle.speed_on(tier).unwrap_or_default()
        ));
    }
    out
}

/// A developer's explicit pick, judged: the bundle, the numbers, and every
/// problem spelled out. Unlike `resolve`, nothing is substituted — the
/// developer asked for this bundle and gets the truth about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitReport {
    pub bundle: BundleRef,
    pub fits: bool,
    pub available_ram_mb: u64,
    pub free_disk_mb: u64,
    pub problems: Vec<String>,
}

/// Judge one named bundle for this device, pace and licence. Speed is
/// judged on `device_tier`; the report's `tier` is the highest the bundle
/// is built for.
pub fn fit(
    manifest: &Manifest,
    bundle_id: &str,
    device_tier: Tier,
    pace: Pace,
    available_ram_mb: u64,
    free_disk_mb: u64,
    licence: &LicenseClaims,
) -> Result<FitReport, VError> {
    let b = manifest
        .bundle(bundle_id)
        .ok_or_else(|| VError::NoBundle(format!("{bundle_id} is not in the manifest")))?;
    let problems = problems(
        b,
        device_tier,
        pace,
        available_ram_mb,
        free_disk_mb,
        licence,
    );
    let mut bundle = bundle_ref(b, *b.tiers.iter().max().unwrap_or(&Tier::P1));
    bundle.speed = b.speed_on(device_tier);
    Ok(FitReport {
        bundle,
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
        speed: b.speed_on(tier),
    }
}

/// Pick the best bundle for `task` at `pace` on a device of `device_tier`
/// with `available_ram_mb` and `free_disk_mb`, within what `licence`
/// allows. Walks the tier down, in the manifest's order within a tier, and
/// explains every refusal.
pub fn resolve(
    manifest: &Manifest,
    task: Task,
    pace: Pace,
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
            let mut p = problems(b, tier, pace, available_ram_mb, free_disk_mb, licence);
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
                    speed: BTreeMap::new(),
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
                    speed: BTreeMap::from([("L2".to_string(), 0.75)]),
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
                    speed: BTreeMap::from([("L1".to_string(), 0.6), ("L2".to_string(), 0.28)]),
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
                    speed: BTreeMap::new(),
                },
            ],
        }
    }

    /// The phone ladder the Pixel 8a measured (knowledge/26, "Small on the
    /// Pixel"): the better speech model first, as a vendor would list it,
    /// then the one that keeps pace.
    pub fn phone_manifest() -> Manifest {
        Manifest {
            schema: 1,
            issued_at: 1,
            bundles: vec![
                Bundle {
                    id: "scribe-small".into(),
                    task: Task::Scribe,
                    tiers: vec![Tier::P3],
                    models: vec![
                        model("whisper-small", ModelKind::Asr, 465, "mit"),
                        model("qwen2.5-0.5b", ModelKind::Llm, 469, "apache-2.0"),
                    ],
                    requires_acknowledgement: vec![],
                    attributions: vec![],
                    notes: String::new(),
                    speed: BTreeMap::from([("P3".to_string(), 3.2)]),
                },
                Bundle {
                    id: "scribe-floor".into(),
                    task: Task::Scribe,
                    tiers: vec![Tier::P1, Tier::P2, Tier::P3],
                    models: vec![
                        model("whisper-base", ModelKind::Asr, 141, "mit"),
                        model("qwen2.5-0.5b", ModelKind::Llm, 469, "apache-2.0"),
                    ],
                    requires_acknowledgement: vec![],
                    attributions: vec![],
                    notes: String::new(),
                    speed: BTreeMap::from([("P3".to_string(), 1.25)]),
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
            Pace::Live,
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
            Pace::Live,
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
            Pace::Live,
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
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Pace::Live,
            Tier::L2,
            8_000,
            20_000,
            &no_ack,
        )
        .unwrap();
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
            Pace::Live,
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
            Pace::Live,
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
            Pace::Live,
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
            Pace::Live,
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
        let r = fit(
            &manifest(),
            "scribe-L3",
            Tier::L3,
            Pace::Live,
            3_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
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
        let r = fit(
            &manifest(),
            "scribe-L2",
            Tier::L2,
            Pace::Live,
            1_000,
            100,
            &no_ack,
        )
        .unwrap();
        assert_eq!(r.problems.len(), 4, "{:?}", r.problems);
        // A pick that fits reports so, with the tier it was built for.
        let r = fit(
            &manifest(),
            "chw-P1",
            Tier::P2,
            Pace::Live,
            3_000,
            5_000,
            &licence_all(),
        )
        .unwrap();
        assert!(r.fits && r.problems.is_empty());
        assert_eq!(r.bundle.tier, Tier::P2, "the highest tier it serves");
        assert_eq!(
            r.bundle.speed, None,
            "never measured, so nothing is claimed"
        );
        assert!(matches!(
            fit(
                &manifest(),
                "nope",
                Tier::P1,
                Pace::Live,
                1,
                1,
                &licence_all()
            ),
            Err(VError::NoBundle(_))
        ));
    }

    /// The measurement this rule covers (knowledge/26, "Small on the Pixel",
    /// 2026-11-04): Whisper small at 3.2× real time on a Pixel 8a finished a
    /// ten-minute visit twenty minutes late; Whisper base at 1.25× kept pace.
    #[test]
    fn a_bundle_too_slow_for_live_speech_is_passed_over_with_its_speed_and_kept_for_recordings() {
        let live = resolve(
            &phone_manifest(),
            Task::Scribe,
            Pace::Live,
            Tier::P3,
            4_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(live.bundle.id, "scribe-floor");
        assert_eq!(live.bundle.speed, Some(1.25));
        assert_eq!(live.downgraded_from, None, "same tier, so not a downgrade");
        assert_eq!(
            live.skipped,
            vec!["scribe-small: runs at 3.2× real time on P3, too slow to keep pace with live speech"]
        );
        let recorded = resolve(
            &phone_manifest(),
            Task::Scribe,
            Pace::Recorded,
            Tier::P3,
            4_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(recorded.bundle.id, "scribe-small");
        assert_eq!(recorded.bundle.speed, Some(3.2));
        assert!(recorded.skipped.is_empty());
        // Memory still comes first: a device that cannot hold small says so, not "too slow".
        let tight = resolve(
            &phone_manifest(),
            Task::Scribe,
            Pace::Recorded,
            Tier::P3,
            1_500,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(tight.bundle.id, "scribe-floor");
        assert!(
            tight.skipped[0].starts_with("scribe-small: needs"),
            "{:?}",
            tight.skipped
        );
        // The developer's pin gets the same judgement, and the same escape.
        let pin = fit(
            &phone_manifest(),
            "scribe-small",
            Tier::P3,
            Pace::Live,
            4_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert!(!pin.fits);
        assert_eq!(pin.problems, vec!["scribe-small: runs at 3.2× real time on P3, too slow to keep pace with live speech"]);
        assert_eq!(pin.bundle.speed, Some(3.2));
        let pin = fit(
            &phone_manifest(),
            "scribe-small",
            Tier::P3,
            Pace::Recorded,
            4_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert!(pin.fits, "{:?}", pin.problems);
    }

    #[test]
    fn an_unmeasured_speed_is_not_held_against_a_bundle() {
        // scribe-L3 has no speed entry: it is chosen for live use and says so with `None`.
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Pace::Live,
            Tier::L3,
            16_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!((r.bundle.id.as_str(), r.bundle.speed), ("scribe-L3", None));
        // scribe-L1 measured on L2 at 0.28: reported on the tier it was chosen for.
        let r = resolve(
            &manifest(),
            Task::Scribe,
            Pace::Live,
            Tier::L2,
            3_000,
            20_000,
            &licence_all(),
        )
        .unwrap();
        assert_eq!(
            (r.bundle.id.as_str(), r.bundle.speed),
            ("scribe-L1", Some(0.28))
        );
        assert_eq!(
            phone_manifest().bundles[0].keeps_pace_on(Tier::P2),
            None,
            "a tier the bundle is not measured on"
        );
    }

    #[test]
    fn a_manifest_speed_must_name_a_tier_the_bundle_is_built_for_and_be_positive() {
        let mut v = serde_json::to_value(phone_manifest()).unwrap();
        v["bundles"][0]["speed"] = serde_json::json!({"L2": 0.75});
        let err = Manifest::parse(&serde_json::to_vec(&v).unwrap()).unwrap_err();
        assert!(
            matches!(&err, VError::InvalidInput(m) if m.contains("scribe-small: speed is given for L2, a tier the bundle is not built for")),
            "{err}"
        );
        v["bundles"][0]["speed"] = serde_json::json!({"P3": 0});
        assert!(
            matches!(Manifest::parse(&serde_json::to_vec(&v).unwrap()), Err(VError::InvalidInput(m)) if m.contains("positive"))
        );
        v["bundles"][0]["speed"] = serde_json::json!({"phone": 1});
        assert!(
            matches!(Manifest::parse(&serde_json::to_vec(&v).unwrap()), Err(VError::InvalidInput(m)) if m.contains("not a tier"))
        );
        // Absent is fine and means unmeasured.
        v["bundles"][0].as_object_mut().unwrap().remove("speed");
        let m = Manifest::parse(&serde_json::to_vec(&v).unwrap()).unwrap();
        assert!(m.bundles[0].speed.is_empty());
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
