//! `verdure-core` — the Verdure SDK's pure domain (knowledge/26).
//!
//! Nothing in this crate opens a file, a socket or a model. It holds the
//! vocabulary (tiers, tasks, bundles, policy), the rules (which bundle fits
//! this device; is this licence valid; is this JSON what the vendor asked for)
//! and the **ports** — the traits the I/O crates implement (`verdure-hw`,
//! `verdure-download`, `verdure-asr`, `verdure-llm`, `verdure-rag`,
//! `verdure-vault`). That is the hexagon from `skills/architecture.md`: the
//! domain never imports an adapter, so a backend can be swapped without
//! touching anything here, and this crate compiles for every target Verdure
//! will ever ship to.
//!
//! Tests here run against fakes and take milliseconds. Anything that needs a
//! real model lives in the engine crates behind a feature.

pub mod capabilities;
pub mod engine;
pub mod error;
pub mod event;
pub mod extract;
pub mod license;
pub mod memory;
pub mod policy;
pub mod registry;
pub mod signing;
pub mod task;
pub mod tier;

pub use capabilities::Capabilities;
pub use engine::{
    AsrEngine, AsrEvent, AsrOptions, AsrSession, AudioQuality, CloudFallback, Control, Embedder,
    GenRequest, LlmEngine, Msg, Role,
};
pub use error::VError;
pub use event::{Event, Progress};
pub use extract::{extract, ExtractOptions, Extraction};
pub use license::{Issuer, LicenseClaims, Verifier};
pub use policy::{Fallback, NetworkPolicy, Policy, Retention, Storage, Telemetry};
pub use registry::{Bundle, BundleRef, Manifest, ModelKind, ModelRef, Resolution};
pub use task::Task;
pub use tier::{DeviceProfile, FormFactor, GpuClass, Tier};

/// Seconds since the Unix epoch. Reading a clock is not I/O in the hexagon's
/// sense, but callers that want determinism (tests, replays) pass their own
/// `now` to the functions that need one instead of calling this.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
