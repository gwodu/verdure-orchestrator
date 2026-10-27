//! The Verdure SDK's public surface — one Rust API, generated bindings for
//! Swift, Kotlin and Python through UniFFI (knowledge/26 §4.3). Everything a
//! vendor's app calls is in this file; the other crates are implementation.
//!
//! Shape rules: records and enums are plain data; `Verdure` and `AsrStream`
//! are objects; the three traits a vendor implements (`ProgressListener`,
//! `TokenSink`, `CloudFallback`) are callback interfaces. No platform `cfg`
//! inside an export block (UniFFI's documented caveat) — platform differences
//! live in `verdure-hw` and the engines.
//!
//! Calls are synchronous and may block (a download, a generation): the host
//! calls them off its UI thread, as it would any file I/O. UniFFI can wrap
//! async later without changing the Rust; the spec's `async fn` shapes were
//! traded for a smaller, more portable v0.1 (knowledge/26).

uniffi::setup_scaffolding!();

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use verdure_core::{
    AsrOptions, Event, LlmEngine, Manifest, Policy, Progress, Task as CoreTask, VError,
};

/// The issuer key compiled into this build. Tokens must be signed by its
/// private half (`verdure license issue`). **This is the development key**:
/// a release build replaces `keys/issuer.pub` — recorded in knowledge/26.
pub const ISSUER_PUBLIC_KEY: &str = include_str!("../keys/issuer.pub");

// ---- data -------------------------------------------------------------

#[derive(Debug, Clone, uniffi::Record)]
pub struct PolicyConfig {
    /// Allow bundle downloads (the only network use). Off = fully offline.
    pub network_downloads: bool,
    /// Hosts downloads may contact; empty = any https host.
    pub allowed_hosts: Vec<String>,
    pub local_counters: bool,
    pub encrypted_storage: bool,
    /// None = session only.
    pub retention_days: Option<u32>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        PolicyConfig {
            network_downloads: true,
            allowed_hosts: vec![],
            local_counters: false,
            encrypted_storage: true,
            retention_days: None,
        }
    }
}

impl From<&PolicyConfig> for Policy {
    fn from(c: &PolicyConfig) -> Policy {
        use verdure_core::{NetworkPolicy, Retention, Storage, Telemetry};
        Policy {
            network: if c.network_downloads {
                NetworkPolicy::DownloadsOnly
            } else {
                NetworkPolicy::None
            },
            telemetry: if c.local_counters {
                Telemetry::LocalCountersOnly
            } else {
                Telemetry::Off
            },
            storage: if c.encrypted_storage {
                Storage::EncryptedAppSandbox
            } else {
                Storage::PlainAppSandbox
            },
            retention: match c.retention_days {
                None => Retention::SessionOnly,
                Some(d) => Retention::Days(d),
            },
            fallback: verdure_core::Fallback::Disabled,
            allowed_hosts: c.allowed_hosts.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Task {
    Scribe,
    Tutor,
    ChwForms,
}

impl From<Task> for CoreTask {
    fn from(t: Task) -> CoreTask {
        match t {
            Task::Scribe => CoreTask::Scribe,
            Task::Tutor => CoreTask::Tutor,
            Task::ChwForms => CoreTask::ChwForms,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Capabilities {
    /// knowledge/02 tier: "P1".."P3" phones, "L1".."L3" laptops.
    pub tier: String,
    pub os: String,
    pub arch: String,
    pub cores: u32,
    pub total_ram_mb: u64,
    pub available_ram_mb: u64,
    pub free_disk_mb: u64,
    pub gpu: String,
    pub low_power: bool,
    pub backends: Vec<String>,
    pub attributions: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct BundleRef {
    pub id: String,
    pub task: Task,
    pub tier: String,
    pub download_bytes: u64,
    pub required_mb: u64,
    pub attributions: Vec<String>,
    /// The device's tier when a lower tier's bundle was chosen.
    pub downgraded_from: Option<String>,
    /// Why better-placed bundles were passed over, one sentence each.
    pub skipped: Vec<String>,
}

/// A developer's explicit bundle choice, judged for this device.
#[derive(Debug, Clone, uniffi::Record)]
pub struct FitReport {
    pub bundle: BundleRef,
    pub fits: bool,
    pub available_ram_mb: u64,
    pub free_disk_mb: u64,
    /// Every problem, one sentence each; empty when it fits.
    pub problems: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Doc {
    pub id: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Hit {
    pub doc_id: String,
    pub title: String,
    pub chunk_index: u32,
    pub text: String,
    pub score: f32,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct Extraction {
    pub json: String,
    pub attempts: u32,
    pub used_grammar: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct AsrConfig {
    pub lang_hints: Vec<String>,
    pub keyterms: Vec<String>,
    /// Pairs applied to final text: [["b p", "BP"], …].
    pub replacements: Vec<Vec<String>>,
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum TranscriptEvent {
    Partial {
        text: String,
    },
    Final {
        text: String,
        start_ms: u64,
        end_ms: u64,
    },
}

#[derive(Debug, Clone, uniffi::Enum)]
pub enum DeleteScope {
    /// One retrieval collection.
    Collection { name: String },
    /// Every retrieval collection.
    Indexes,
    /// Everything Verdure stored for this install: indexes, vault (key
    /// destroyed first), downloaded bundles.
    All,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum VerdureError {
    #[error("{0}")]
    License(String),
    #[error("{0}")]
    Policy(String),
    #[error("{0}")]
    Network(String),
    #[error("{0}")]
    Integrity(String),
    #[error("{0}")]
    Storage(String),
    #[error("{0}")]
    NoBundle(String),
    #[error("{0}")]
    Memory(String),
    #[error("{0}")]
    Disk(String),
    #[error("{0}")]
    EngineUnavailable(String),
    #[error("{0}")]
    Engine(String),
    #[error("{0}")]
    InvalidSchema(String),
    #[error("{0}")]
    SchemaInvalid(String),
    #[error("{0}")]
    InvalidInput(String),
    #[error("cancelled")]
    Cancelled,
}

impl From<VError> for VerdureError {
    fn from(e: VError) -> Self {
        let msg = e.to_string();
        match e {
            VError::License(_) => VerdureError::License(msg),
            VError::PolicyDenied(_) => VerdureError::Policy(msg),
            VError::Network(_) => VerdureError::Network(msg),
            VError::Integrity { .. } => VerdureError::Integrity(msg),
            VError::Storage(_) => VerdureError::Storage(msg),
            VError::NoBundle(_) => VerdureError::NoBundle(msg),
            VError::NotEnoughMemory { .. } => VerdureError::Memory(msg),
            VError::NotEnoughDisk { .. } => VerdureError::Disk(msg),
            VError::EngineUnavailable(_) => VerdureError::EngineUnavailable(msg),
            VError::Engine(_) => VerdureError::Engine(msg),
            VError::InvalidSchema(_) => VerdureError::InvalidSchema(msg),
            VError::SchemaInvalid { .. } => VerdureError::SchemaInvalid(msg),
            VError::InvalidInput(_) => VerdureError::InvalidInput(msg),
            VError::Cancelled => VerdureError::Cancelled,
        }
    }
}

// ---- callbacks the vendor implements ----------------------------------

#[uniffi::export(with_foreign)]
pub trait ProgressListener: Send + Sync {
    fn on_progress(&self, done: u64, total: u64);
    fn on_status(&self, message: String);
    fn is_cancelled(&self) -> bool;
}

#[uniffi::export(with_foreign)]
pub trait TokenSink: Send + Sync {
    /// Return false to stop generation.
    fn on_token(&self, piece: String) -> bool;
}

/// The vendor's cloud. Verdure calls into it when the local engine cannot
/// serve a request and the vendor switched the hook on; Verdure itself opens
/// no socket for this.
#[uniffi::export(with_foreign)]
pub trait CloudFallback: Send + Sync {
    fn transcribe(&self, pcm_16k_mono: Vec<f32>) -> Result<String, VerdureError>;
    fn extract(&self, text: String, schema_json: String) -> Result<String, VerdureError>;
}

struct ListenerBridge(Arc<dyn ProgressListener>);

impl Progress for ListenerBridge {
    fn on_event(&self, event: Event) {
        match event {
            Event::Progress { done, total } => self.0.on_progress(done, total),
            Event::Downgraded { from, to, reason } => self
                .0
                .on_status(format!("Using the {to} bundle instead of {from}: {reason}")),
            Event::Throttled { threads } => self
                .0
                .on_status(format!("Running cooler on {threads} threads.")),
            Event::ModelUnloaded { reason } => self
                .0
                .on_status(format!("The language model was unloaded: {reason}")),
            Event::Status(s) => self.0.on_status(s),
        }
    }
    fn is_cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}

// ---- the runtime -------------------------------------------------------

struct Inner {
    policy: Policy,
    claims: verdure_core::LicenseClaims,
    data_dir: PathBuf,
    manifest: Option<Manifest>,
    rag: Option<verdure_rag::Store>,
    llm: Option<Arc<dyn LlmEngine>>,
    asr: Option<Arc<dyn verdure_core::AsrEngine>>,
    fallback: Option<Arc<dyn CloudFallback>>,
    loaded_bundle: Option<String>,
}

#[derive(uniffi::Object)]
pub struct Verdure {
    inner: Mutex<Inner>,
}

fn backends() -> Vec<String> {
    let mut b = Vec::new();
    if cfg!(feature = "whisper") {
        b.push("whisper".to_string());
    }
    if cfg!(feature = "llama") {
        b.push("llama".to_string());
    }
    b
}

#[uniffi::export]
impl Verdure {
    /// Start the runtime: verify the licence against the built-in issuer key,
    /// read the device. No network, no model load — those come with
    /// `download` and `load`, so starting is instant.
    #[uniffi::constructor]
    pub fn start(
        license_key: String,
        policy: PolicyConfig,
        data_dir: String,
    ) -> Result<Arc<Self>, VerdureError> {
        Self::start_with_issuer(license_key, policy, data_dir, ISSUER_PUBLIC_KEY.to_string())
    }

    /// `start` with an explicit issuer key — for vendors who issue their own
    /// fleet tokens (`[OQ-105]`) and for tests.
    #[uniffi::constructor]
    pub fn start_with_issuer(
        license_key: String,
        policy: PolicyConfig,
        data_dir: String,
        issuer_public_key: String,
    ) -> Result<Arc<Self>, VerdureError> {
        let claims = verdure_core::Verifier::new(&issuer_public_key)
            .verify(&license_key, verdure_core::unix_now())?;
        let data_dir = PathBuf::from(data_dir);
        std::fs::create_dir_all(&data_dir).map_err(|e| {
            VerdureError::Storage(format!("couldn't create {}: {e}", data_dir.display()))
        })?;
        Ok(Arc::new(Verdure {
            inner: Mutex::new(Inner {
                policy: Policy::from(&policy),
                claims,
                data_dir,
                manifest: None,
                rag: None,
                llm: None,
                asr: None,
                fallback: None,
                loaded_bundle: None,
            }),
        }))
    }

    pub fn capabilities(&self) -> Capabilities {
        let inner = self.inner.lock().unwrap();
        let profile = verdure_hw::probe(&inner.data_dir);
        let caps = verdure_core::Capabilities::from_profile(profile, backends());
        let mut attributions = caps.attributions.clone();
        if let (Some(m), Some(id)) = (&inner.manifest, &inner.loaded_bundle) {
            if let Some(b) = m.bundle(id) {
                attributions.extend(b.attributions.iter().cloned());
            }
        }
        Capabilities {
            tier: caps.tier.to_string(),
            os: caps.device.os.clone(),
            arch: caps.device.arch.clone(),
            cores: caps.device.cores,
            total_ram_mb: caps.device.total_ram_mb,
            available_ram_mb: caps.device.available_ram_mb,
            free_disk_mb: caps.device.free_disk_mb,
            gpu: format!("{:?}", caps.device.gpu).to_lowercase(),
            low_power: caps.device.low_power,
            backends: caps.backends,
            attributions,
            notes: caps.notes,
        }
    }

    /// Fetch and verify the vendor's signed manifest (the one network call
    /// besides bundle files; the policy decides whether the host is allowed).
    pub fn set_bundle_source(
        &self,
        manifest_url: String,
        public_key: String,
    ) -> Result<(), VerdureError> {
        let mut inner = self.inner.lock().unwrap();
        let m = verdure_download::fetch_manifest(&inner.policy, &manifest_url, &public_key)?;
        inner.manifest = Some(m);
        Ok(())
    }

    /// Use a signed manifest the app ships inside itself — the fully offline
    /// path (manifest and signature as text, key as base64).
    pub fn load_manifest(
        &self,
        manifest_json: String,
        signature: String,
        public_key: String,
    ) -> Result<(), VerdureError> {
        let m = verdure_download::verify_manifest_bytes(
            manifest_json.as_bytes(),
            &signature,
            &public_key,
        )?;
        self.inner.lock().unwrap().manifest = Some(m);
        Ok(())
    }

    /// The bundle for `task` that fits this device now, within the licence.
    pub fn recommend(&self, task: Task) -> Result<BundleRef, VerdureError> {
        let inner = self.inner.lock().unwrap();
        let manifest = inner.manifest.as_ref().ok_or_else(|| {
            VerdureError::NoBundle(
                "no manifest loaded: call set_bundle_source or load_manifest first".into(),
            )
        })?;
        let profile = verdure_hw::probe(&inner.data_dir);
        let r = verdure_core::registry::resolve(
            manifest,
            task.into(),
            profile.tier(),
            profile.available_ram_mb,
            profile.free_disk_mb,
            &inner.claims,
        )?;
        Ok(BundleRef {
            id: r.bundle.id,
            task,
            tier: r.bundle.tier.to_string(),
            download_bytes: r.bundle.download_bytes,
            required_mb: r.bundle.required_mb,
            attributions: r.bundle.attributions,
            downgraded_from: r.downgraded_from.map(|t| t.to_string()),
            skipped: r.skipped,
        })
    }

    /// Judge a bundle the developer picked themselves — no substitution, no
    /// walk-down. The vendor already chose the models by writing the
    /// manifest; this is the developer's hand on the dial at runtime: pin
    /// `scribe-L2` and learn, in sentences, whether this device can run it.
    pub fn fit(&self, bundle_id: String) -> Result<FitReport, VerdureError> {
        let inner = self.inner.lock().unwrap();
        let manifest = inner
            .manifest
            .as_ref()
            .ok_or_else(|| VerdureError::NoBundle("no manifest loaded".into()))?;
        let profile = verdure_hw::probe(&inner.data_dir);
        let r = verdure_core::registry::fit(
            manifest,
            &bundle_id,
            profile.available_ram_mb,
            profile.free_disk_mb,
            &inner.claims,
        )?;
        Ok(FitReport {
            bundle: BundleRef {
                id: r.bundle.id,
                task: match r.bundle.task {
                    CoreTask::Scribe => Task::Scribe,
                    CoreTask::Tutor => Task::Tutor,
                    CoreTask::ChwForms => Task::ChwForms,
                },
                tier: r.bundle.tier.to_string(),
                download_bytes: r.bundle.download_bytes,
                required_mb: r.bundle.required_mb,
                attributions: r.bundle.attributions,
                downgraded_from: None,
                skipped: vec![],
            },
            fits: r.fits,
            available_ram_mb: r.available_ram_mb,
            free_disk_mb: r.free_disk_mb,
            problems: r.problems,
        })
    }

    /// Download (or resume) every file of `bundle_id`, verifying each.
    pub fn download(
        &self,
        bundle_id: String,
        listener: Arc<dyn ProgressListener>,
    ) -> Result<(), VerdureError> {
        let inner = self.inner.lock().unwrap();
        let manifest = inner
            .manifest
            .as_ref()
            .ok_or_else(|| VerdureError::NoBundle("no manifest loaded".into()))?;
        let bundle = manifest
            .bundle(&bundle_id)
            .ok_or_else(|| VerdureError::NoBundle(format!("{bundle_id} is not in the manifest")))?;
        if !inner.claims.entitles(&bundle_id) {
            return Err(VerdureError::License(format!(
                "{bundle_id} is not included in this licence"
            )));
        }
        verdure_download::download_bundle(
            &inner.policy,
            bundle,
            &inner.data_dir,
            &ListenerBridge(listener),
        )?;
        Ok(())
    }

    /// True when every file of the bundle is on disk and verified.
    pub fn is_downloaded(&self, bundle_id: String) -> bool {
        let inner = self.inner.lock().unwrap();
        let Some(bundle) = inner.manifest.as_ref().and_then(|m| m.bundle(&bundle_id)) else {
            return false;
        };
        let dir = verdure_download::bundle_dir(&inner.data_dir, &bundle_id);
        bundle.models.iter().all(|m| {
            let p = dir.join(&m.file);
            p.is_file()
                && verdure_download::file_sha256(&p)
                    .map(|h| h == m.sha256.to_ascii_lowercase())
                    .unwrap_or(false)
        })
    }

    /// Load the bundle's engines into memory (after `download`). Refuses,
    /// with the numbers, when the device cannot hold them right now.
    pub fn load(&self, bundle_id: String) -> Result<(), VerdureError> {
        self.load_with(bundle_id, false)
    }

    /// `load` without the memory gate. For a developer who has measured their
    /// own device and disagrees with the rule: the engine may abort the host
    /// process if the model does not fit, and nothing will catch it until the
    /// engine runs isolated. The licence still applies.
    pub fn load_overcommit(&self, bundle_id: String) -> Result<(), VerdureError> {
        self.load_with(bundle_id, true)
    }
}

impl Verdure {
    fn load_with(&self, bundle_id: String, overcommit: bool) -> Result<(), VerdureError> {
        let mut inner = self.inner.lock().unwrap();
        let manifest = inner
            .manifest
            .clone()
            .ok_or_else(|| VerdureError::NoBundle("no manifest loaded".into()))?;
        let bundle = manifest
            .bundle(&bundle_id)
            .ok_or_else(|| VerdureError::NoBundle(format!("{bundle_id} is not in the manifest")))?;
        if !inner.claims.entitles(&bundle_id) {
            return Err(VerdureError::License(format!(
                "{bundle_id} is not included in this licence"
            )));
        }
        let profile = verdure_hw::probe(&inner.data_dir);
        let needed = bundle.required_mb();
        if !overcommit && needed > profile.available_ram_mb {
            return Err(VError::NotEnoughMemory {
                needed_mb: needed,
                available_mb: profile.available_ram_mb,
            }
            .into());
        }
        // Drop the old engines BEFORE loading new ones: holding both is what
        // OOM-aborts a tight machine.
        inner.llm = None;
        inner.asr = None;
        let dir = verdure_download::bundle_dir(&inner.data_dir, &bundle_id);
        let threads = profile.cores.clamp(1, 8);
        for m in &bundle.models {
            let path = dir.join(&m.file);
            match m.kind {
                verdure_core::ModelKind::Llm => inner.llm = Some(load_llm(&path, m, threads)?),
                verdure_core::ModelKind::Asr => inner.asr = Some(load_asr(&path, m, threads)?),
                verdure_core::ModelKind::Embedder => {}
            }
        }
        inner.loaded_bundle = Some(bundle_id);
        Ok(())
    }
}

#[uniffi::export]
impl Verdure {
    /// Open a live transcription stream.
    pub fn transcribe(&self, config: AsrConfig) -> Result<Arc<AsrStream>, VerdureError> {
        let inner = self.inner.lock().unwrap();
        let engine = inner.asr.as_ref().ok_or_else(|| {
            VerdureError::EngineUnavailable(
                "no speech model is loaded: call load(bundle) first".into(),
            )
        })?;
        let opts = AsrOptions {
            lang_hints: config.lang_hints,
            keyterms: config.keyterms,
            replacements: config
                .replacements
                .into_iter()
                .filter(|p| p.len() == 2)
                .map(|p| (p[0].clone(), p[1].clone()))
                .collect(),
            ..AsrOptions::default()
        };
        let session = engine.open(&opts)?;
        Ok(Arc::new(AsrStream {
            session: Mutex::new(Some(session)),
        }))
    }

    /// A conversation turn; pieces stream to `sink` as they are generated.
    pub fn chat(
        &self,
        messages: Vec<ChatMessage>,
        sink: Option<Arc<dyn TokenSink>>,
    ) -> Result<String, VerdureError> {
        let llm = self.llm()?;
        let msgs: Vec<verdure_core::Msg> = messages
            .into_iter()
            .map(|m| verdure_core::Msg {
                role: match m.role {
                    Role::System => verdure_core::Role::System,
                    Role::User => verdure_core::Role::User,
                    Role::Assistant => verdure_core::Role::Assistant,
                },
                content: m.content,
            })
            .collect();
        let req = verdure_core::GenRequest {
            messages: msgs,
            max_tokens: 512,
            temperature: 0.6,
            grammar: None,
        };
        let reply = llm.generate(&req, &mut |piece| match &sink {
            Some(s) if !s.on_token(piece.to_string()) => verdure_core::Control::Stop,
            _ => verdure_core::Control::Continue,
        })?;
        Ok(reply)
    }

    /// Free text → JSON matching `schema_json`, or an error naming what was
    /// wrong. Falls back to the vendor's cloud only when they set one and no
    /// local engine is loaded.
    pub fn extract(&self, text: String, schema_json: String) -> Result<Extraction, VerdureError> {
        let fallback = self.inner.lock().unwrap().fallback.clone();
        match self.llm() {
            Ok(llm) => {
                let out = verdure_core::extract(
                    llm.as_ref(),
                    &text,
                    &schema_json,
                    &verdure_core::ExtractOptions::default(),
                    &verdure_core::event::NoProgress,
                )?;
                Ok(Extraction {
                    json: out.json,
                    attempts: out.attempts,
                    used_grammar: out.used_grammar,
                    warnings: out.warnings,
                })
            }
            Err(e) => match fallback {
                Some(f) => {
                    let json = f.extract(text, schema_json.clone())?;
                    let schema: serde_json::Value = serde_json::from_str(&schema_json)
                        .map_err(|e| VerdureError::InvalidSchema(e.to_string()))?;
                    let value: serde_json::Value = serde_json::from_str(&json).map_err(|e| {
                        VerdureError::SchemaInvalid(format!(
                            "the fallback returned invalid JSON: {e}"
                        ))
                    })?;
                    let v = verdure_grammar::validate(&schema, &value);
                    if !v.is_empty() {
                        return Err(VerdureError::SchemaInvalid(
                            v.iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join("; "),
                        ));
                    }
                    Ok(Extraction {
                        json,
                        attempts: 1,
                        used_grammar: false,
                        warnings: vec!["answered by the vendor's cloud fallback".into()],
                    })
                }
                None => Err(e),
            },
        }
    }

    /// Index documents into a retrieval collection (chunked, BM25, plus
    /// embeddings when the loaded bundle has an embedder).
    pub fn index(&self, collection: String, docs: Vec<Doc>) -> Result<u64, VerdureError> {
        let mut inner = self.inner.lock().unwrap();
        let store = rag(&mut inner)?;
        let docs: Vec<verdure_rag::Doc> = docs
            .into_iter()
            .map(|d| verdure_rag::Doc {
                id: d.id,
                title: d.title,
                text: d.text,
            })
            .collect();
        Ok(store.index(&collection, &docs)?)
    }

    pub fn query(
        &self,
        collection: String,
        question: String,
        k: u32,
    ) -> Result<Vec<Hit>, VerdureError> {
        let mut inner = self.inner.lock().unwrap();
        let store = rag(&mut inner)?;
        Ok(store
            .query(&collection, &question, k)?
            .into_iter()
            .map(|h| Hit {
                doc_id: h.doc_id,
                title: h.title,
                chunk_index: h.chunk_index,
                text: h.text,
                score: h.score,
            })
            .collect())
    }

    pub fn delete(&self, scope: DeleteScope) -> Result<(), VerdureError> {
        let mut inner = self.inner.lock().unwrap();
        match scope {
            DeleteScope::Collection { name } => {
                rag(&mut inner)?.delete_collection(&name)?;
            }
            DeleteScope::Indexes => {
                rag(&mut inner)?.delete_all()?;
            }
            DeleteScope::All => {
                inner.rag = None;
                inner.llm = None;
                inner.asr = None;
                inner.loaded_bundle = None;
                let dir = inner.data_dir.clone();
                // Key first (crypto-shred), then the files.
                let vault = verdure_vault::Vault::open(
                    dir.join("vault"),
                    Box::new(verdure_vault::FileKeyStore::new(dir.join("vault.key"))),
                )?;
                vault.shred()?;
                for sub in ["index.sqlite", "index.sqlite-wal", "index.sqlite-shm"] {
                    let _ = std::fs::remove_file(dir.join(sub));
                }
                let _ = std::fs::remove_dir_all(dir.join("bundles"));
                let _ = std::fs::remove_dir_all(dir.join("vault"));
            }
        }
        Ok(())
    }

    /// Give Verdure the vendor's cloud to fall back on, or take it away.
    pub fn set_cloud_fallback(&self, fallback: Option<Arc<dyn CloudFallback>>) {
        let mut inner = self.inner.lock().unwrap();
        inner.policy.fallback = if fallback.is_some() {
            verdure_core::Fallback::VendorHook
        } else {
            verdure_core::Fallback::Disabled
        };
        inner.fallback = fallback;
    }

    /// The licence this runtime started with: vendor id and seat cap.
    pub fn licence_summary(&self) -> String {
        let inner = self.inner.lock().unwrap();
        let c = &inner.claims;
        let seats = if c.seats == 0 {
            "unlimited seats".to_string()
        } else {
            format!("{} seats", c.seats)
        };
        let until = if c.expires_at == 0 {
            "no expiry".to_string()
        } else {
            format!("expires at {}", c.expires_at)
        };
        format!("{} · {seats} · {until}", c.vendor_id)
    }
}

impl Verdure {
    fn llm(&self) -> Result<Arc<dyn LlmEngine>, VerdureError> {
        self.inner.lock().unwrap().llm.clone().ok_or_else(|| {
            VerdureError::EngineUnavailable(
                "no language model is loaded: call load(bundle) first".into(),
            )
        })
    }
}

fn rag(inner: &mut Inner) -> Result<&mut verdure_rag::Store, VerdureError> {
    if inner.rag.is_none() {
        let path = inner.data_dir.join("index.sqlite");
        inner.rag = Some(verdure_rag::Store::open(&path, None)?);
    }
    Ok(inner.rag.as_mut().unwrap())
}

#[cfg(feature = "llama")]
fn load_llm(
    path: &std::path::Path,
    m: &verdure_core::ModelRef,
    threads: u32,
) -> Result<Arc<dyn LlmEngine>, VerdureError> {
    let style = m
        .prompt_style
        .as_deref()
        .and_then(verdure_llm::PromptStyle::parse)
        .unwrap_or(verdure_llm::PromptStyle::ChatMl);
    let n_ctx = if m.trained_ctx > 0 {
        m.trained_ctx.min(verdure_llm::MAX_CONTEXT)
    } else {
        2048
    };
    let opts = verdure_llm::llama::LoadOptions {
        style,
        n_ctx,
        threads,
        ..Default::default()
    };
    Ok(Arc::new(verdure_llm::llama::LlamaEngine::load(path, opts)?))
}

#[cfg(not(feature = "llama"))]
fn load_llm(
    _path: &std::path::Path,
    m: &verdure_core::ModelRef,
    _threads: u32,
) -> Result<Arc<dyn LlmEngine>, VerdureError> {
    Err(VerdureError::EngineUnavailable(format!(
        "this build has no language-model engine for {} (built without the `llama` feature)",
        m.id
    )))
}

#[cfg(feature = "whisper")]
fn load_asr(
    path: &std::path::Path,
    _m: &verdure_core::ModelRef,
    threads: u32,
) -> Result<Arc<dyn verdure_core::AsrEngine>, VerdureError> {
    Ok(Arc::new(verdure_asr::whisper::WhisperAsr::load(
        path, threads,
    )?))
}

#[cfg(not(feature = "whisper"))]
fn load_asr(
    _path: &std::path::Path,
    m: &verdure_core::ModelRef,
    _threads: u32,
) -> Result<Arc<dyn verdure_core::AsrEngine>, VerdureError> {
    Err(VerdureError::EngineUnavailable(format!(
        "this build has no speech engine for {} (built without the `whisper` feature)",
        m.id
    )))
}

/// A live transcription session: push microphone samples, receive events.
#[derive(uniffi::Object)]
pub struct AsrStream {
    session: Mutex<Option<Box<dyn verdure_core::AsrSession>>>,
}

#[uniffi::export]
impl AsrStream {
    /// Mono `f32` samples in [-1, 1] at `sample_rate`; any rate, resampled inside.
    pub fn push_pcm(&self, pcm: Vec<f32>, sample_rate: u32) -> Vec<TranscriptEvent> {
        let mut guard = self.session.lock().unwrap();
        match guard.as_mut() {
            Some(s) => s
                .push_pcm(&pcm, sample_rate)
                .into_iter()
                .filter_map(convert_event)
                .collect(),
            None => Vec::new(),
        }
    }

    /// Flush the last utterance and release the engine for this stream.
    pub fn finish(&self) -> Vec<TranscriptEvent> {
        let mut guard = self.session.lock().unwrap();
        let events = guard.as_mut().map(|s| s.finish()).unwrap_or_default();
        *guard = None;
        events.into_iter().filter_map(convert_event).collect()
    }
}

fn convert_event(e: verdure_core::AsrEvent) -> Option<TranscriptEvent> {
    match e {
        verdure_core::AsrEvent::Partial { text } => Some(TranscriptEvent::Partial { text }),
        verdure_core::AsrEvent::Final {
            text,
            start_ms,
            end_ms,
        } => Some(TranscriptEvent::Final {
            text,
            start_ms,
            end_ms,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use verdure_core::signing::KeyPair;
    use verdure_core::{Issuer, LicenseClaims};

    fn issuer() -> Issuer {
        Issuer::new(KeyPair::generate().unwrap())
    }

    fn token(issuer: &Issuer) -> String {
        issuer.issue(&LicenseClaims {
            vendor_id: "test-vendor".into(),
            seats: 0,
            issued_at: 0,
            expires_at: 0,
            bundles: vec!["*".into()],
            acknowledgements: vec![],
        })
    }

    fn manifest_json() -> String {
        serde_json::json!({
            "schema": 1, "issued_at": 1,
            "bundles": [{
                "id": "chw-P1", "task": "chw_forms", "tiers": ["P1","P2","P3","L1","L2","L3"],
                "models": [{
                    "id": "tiny-llm", "kind": "llm", "name": "Tiny", "file": "tiny.gguf",
                    "url": "https://cdn.example.org/tiny.gguf", "sha256": "00", "size_bytes": 1048576,
                    "engine": "llama", "prompt_style": "chatml", "licence": "apache-2.0"
                }],
                "attributions": ["Speech model: Moonshine (MIT)"]
            }]
        })
        .to_string()
    }

    fn start(dir: &std::path::Path) -> (Arc<Verdure>, Issuer) {
        let iss = issuer();
        let v = Verdure::start_with_issuer(
            token(&iss),
            PolicyConfig::default(),
            dir.to_string_lossy().into_owned(),
            iss.public_key(),
        )
        .unwrap();
        (v, iss)
    }

    #[test]
    fn start_verifies_the_licence_and_reads_the_device() {
        let dir = tempfile::tempdir().unwrap();
        let (v, _) = start(dir.path());
        let caps = v.capabilities();
        assert!(
            ["P1", "P2", "P3", "L1", "L2", "L3"].contains(&caps.tier.as_str()),
            "{caps:?}"
        );
        assert!(caps.total_ram_mb > 0 && caps.cores > 0);
        assert_eq!(caps.backends, backends());
        assert!(v
            .licence_summary()
            .starts_with("test-vendor · unlimited seats"));

        let stranger = issuer();
        let err = Verdure::start_with_issuer(
            token(&stranger),
            PolicyConfig::default(),
            dir.path().to_string_lossy().into_owned(),
            issuer().public_key(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, VerdureError::License(_)), "{err:?}");
    }

    #[test]
    fn a_shipped_manifest_is_trusted_only_when_signed_and_recommend_walks_it() {
        let dir = tempfile::tempdir().unwrap();
        let (v, _) = start(dir.path());
        assert!(
            matches!(v.recommend(Task::ChwForms), Err(VerdureError::NoBundle(_))),
            "no manifest yet"
        );
        let kp = KeyPair::generate().unwrap();
        let m = manifest_json();
        assert!(matches!(
            v.load_manifest(m.clone(), "bad".into(), kp.public_b64()),
            Err(VerdureError::Integrity(_))
        ));
        v.load_manifest(m.clone(), kp.sign(m.as_bytes()), kp.public_b64())
            .unwrap();
        let r = v.recommend(Task::ChwForms).unwrap();
        assert_eq!(r.id, "chw-P1");
        assert_eq!(r.attributions, vec!["Speech model: Moonshine (MIT)"]);
        assert!(r.downgraded_from.is_none());
        assert!(matches!(
            v.recommend(Task::Tutor),
            Err(VerdureError::NoBundle(_))
        ));
        assert!(!v.is_downloaded("chw-P1".into()));
    }

    #[test]
    fn a_developer_can_pin_a_bundle_and_is_told_the_truth_about_it() {
        let dir = tempfile::tempdir().unwrap();
        let (v, _) = start(dir.path());
        let kp = KeyPair::generate().unwrap();
        let m = manifest_json();
        v.load_manifest(m.clone(), kp.sign(m.as_bytes()), kp.public_b64())
            .unwrap();
        let r = v.fit("chw-P1".into()).unwrap();
        assert_eq!(r.bundle.id, "chw-P1");
        assert!(r.available_ram_mb > 0);
        // A 1 MB model fits any machine this test runs on.
        assert!(r.fits && r.problems.is_empty(), "{r:?}");
        assert!(matches!(
            v.fit("nope".into()),
            Err(VerdureError::NoBundle(_))
        ));
        // Loading a pinned bundle that is not downloaded fails at the engine,
        // not at the gate, in both modes; without an engine built it says so.
        for load in [Verdure::load, Verdure::load_overcommit] {
            let err = load(&v, "chw-P1".into()).err().unwrap();
            assert!(
                matches!(
                    err,
                    VerdureError::EngineUnavailable(_) | VerdureError::Engine(_)
                ),
                "{err:?}"
            );
        }
    }

    #[test]
    fn retrieval_works_without_any_engine_and_delete_all_shreds() {
        let dir = tempfile::tempdir().unwrap();
        let (v, _) = start(dir.path());
        let n = v
            .index(
                "guides".into(),
                vec![Doc {
                    id: "epi".into(),
                    title: "Immunisation".into(),
                    text: "Measles at nine months.".into(),
                }],
            )
            .unwrap();
        assert_eq!(n, 1);
        let hits = v
            .query("guides".into(), "when is measles?".into(), 3)
            .unwrap();
        assert_eq!(hits[0].doc_id, "epi");
        v.delete(DeleteScope::All).unwrap();
        assert!(v
            .query("guides".into(), "measles".into(), 3)
            .unwrap()
            .is_empty());
        assert!(!dir.path().join("vault.key").exists());
    }

    #[test]
    fn without_an_engine_extract_says_so_and_a_fallback_is_validated() {
        let dir = tempfile::tempdir().unwrap();
        let (v, _) = start(dir.path());
        let schema = r#"{"type":"object","properties":{"a":{"type":"integer"}},"required":["a"],"additionalProperties":false}"#;
        let err = v.extract("x".into(), schema.into()).err().unwrap();
        assert!(matches!(err, VerdureError::EngineUnavailable(_)), "{err:?}");
        assert!(matches!(
            v.chat(vec![], None),
            Err(VerdureError::EngineUnavailable(_))
        ));
        assert!(matches!(
            v.transcribe(AsrConfig {
                lang_hints: vec![],
                keyterms: vec![],
                replacements: vec![]
            }),
            Err(VerdureError::EngineUnavailable(_))
        ));

        struct Cloud(&'static str);
        impl CloudFallback for Cloud {
            fn transcribe(&self, _: Vec<f32>) -> Result<String, VerdureError> {
                Ok("".into())
            }
            fn extract(&self, _: String, _: String) -> Result<String, VerdureError> {
                Ok(self.0.to_string())
            }
        }
        v.set_cloud_fallback(Some(Arc::new(Cloud(r#"{"a": "not a number"}"#))));
        assert!(
            matches!(
                v.extract("x".into(), schema.into()),
                Err(VerdureError::SchemaInvalid(_))
            ),
            "the fallback's output is checked too"
        );
        v.set_cloud_fallback(Some(Arc::new(Cloud(r#"{"a": 3}"#))));
        let out = v.extract("x".into(), schema.into()).unwrap();
        assert_eq!(out.json, r#"{"a": 3}"#);
        assert_eq!(
            out.warnings,
            vec!["answered by the vendor's cloud fallback"]
        );
        v.set_cloud_fallback(None);
        assert!(matches!(
            v.extract("x".into(), schema.into()),
            Err(VerdureError::EngineUnavailable(_))
        ));
    }

    #[test]
    fn policy_config_maps_to_the_core_policy() {
        let p = Policy::from(&PolicyConfig::default());
        assert_eq!(p, Policy::default());
        let off = Policy::from(&PolicyConfig {
            network_downloads: false,
            retention_days: Some(7),
            ..PolicyConfig::default()
        });
        assert_eq!(off.network, verdure_core::NetworkPolicy::None);
        assert_eq!(off.retention, verdure_core::Retention::Days(7));
    }
}
