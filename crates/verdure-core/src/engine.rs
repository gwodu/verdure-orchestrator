//! The ports: what the domain needs from an ASR engine, an LLM, an embedder
//! and the vendor's cloud. Written in engine-neutral terms on purpose
//! (knowledge/14, "design the trait to not smell like llama.cpp"): an ONNX
//! engine, Core ML or a remote fallback must be able to implement these
//! without contortion.

use crate::VError;

/// How the ASR should listen. Mirrors what scribe vendors already expose
/// (Corti's keyterms and replacements, Nabla's locales) so an integrator maps
/// their session config one-to-one (knowledge/26, Part 1).
#[derive(Debug, Clone, PartialEq)]
pub struct AsrOptions {
    /// BCP-47-ish hints: "en", "sw", "ha". Empty = engine default.
    pub lang_hints: Vec<String>,
    pub diarize: bool,
    /// Words to bias recognition toward (drug names, local place names).
    pub keyterms: Vec<String>,
    /// Exact replacements applied to final text ("B P" → "BP").
    pub replacements: Vec<(String, String)>,
    /// Voice-activity gating on by default: silence is never transcribed.
    pub vad: bool,
}

impl Default for AsrOptions {
    fn default() -> Self {
        AsrOptions {
            lang_hints: vec![],
            diarize: false,
            keyterms: vec![],
            replacements: vec![],
            vad: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioQuality {
    Good,
    Quiet,
    Clipping,
    Noisy,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsrEvent {
    /// Interim text that may still change.
    Partial {
        text: String,
    },
    /// Committed text for one utterance.
    Final {
        text: String,
        start_ms: u64,
        end_ms: u64,
    },
    Speaker {
        id: u32,
        start_ms: u64,
    },
    AudioQuality(AudioQuality),
}

/// One open microphone session. PCM is mono `f32` in `[-1, 1]` at any rate;
/// the engine resamples.
pub trait AsrSession: Send {
    fn push_pcm(&mut self, pcm: &[f32], sample_rate: u32) -> Vec<AsrEvent>;
    /// Flush what is buffered; the session is finished afterwards.
    fn finish(&mut self) -> Vec<AsrEvent>;
}

pub trait AsrEngine: Send + Sync {
    fn id(&self) -> &str;
    fn open(&self, opts: &AsrOptions) -> Result<Box<dyn AsrSession>, VError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Msg {
    pub role: Role,
    pub content: String,
}

impl Msg {
    pub fn system(s: impl Into<String>) -> Msg {
        Msg {
            role: Role::System,
            content: s.into(),
        }
    }
    pub fn user(s: impl Into<String>) -> Msg {
        Msg {
            role: Role::User,
            content: s.into(),
        }
    }
    pub fn assistant(s: impl Into<String>) -> Msg {
        Msg {
            role: Role::Assistant,
            content: s.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenRequest {
    pub messages: Vec<Msg>,
    pub max_tokens: u32,
    pub temperature: f32,
    /// A GBNF grammar to constrain sampling. Engines that cannot constrain
    /// must return `VError::EngineUnavailable`, never silently ignore it.
    pub grammar: Option<String>,
}

/// What a token sink says after each piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Continue,
    Stop,
}

pub trait LlmEngine: Send + Sync {
    fn id(&self) -> &str;
    /// Generate, streaming pieces to `on_token`, returning the whole reply.
    fn generate(
        &self,
        req: &GenRequest,
        on_token: &mut dyn FnMut(&str) -> Control,
    ) -> Result<String, VError>;
    fn count_tokens(&self, text: &str) -> usize;
    fn context_window(&self) -> u32;
}

pub trait Embedder: Send + Sync {
    fn id(&self) -> &str;
    fn dimension(&self) -> usize;
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, VError>;
}

/// The vendor's own cloud, as a hook. Verdure calls these; it never opens the
/// socket. Errors are the vendor's strings, passed through.
pub trait CloudFallback: Send + Sync {
    fn transcribe(&self, pcm_16k_mono: &[f32]) -> Result<String, String>;
    fn extract(&self, text: &str, schema_json: &str) -> Result<String, String>;
}

/// A scripted LLM for tests and the CLI's dry runs: returns the next reply in
/// its queue and remembers every request it saw.
pub struct FakeLlm {
    replies: std::sync::Mutex<std::collections::VecDeque<String>>,
    pub requests: std::sync::Mutex<Vec<GenRequest>>,
}

impl FakeLlm {
    pub fn scripted<I: IntoIterator<Item = S>, S: Into<String>>(replies: I) -> FakeLlm {
        FakeLlm {
            replies: std::sync::Mutex::new(replies.into_iter().map(Into::into).collect()),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl LlmEngine for FakeLlm {
    fn id(&self) -> &str {
        "fake"
    }
    fn generate(
        &self,
        req: &GenRequest,
        on_token: &mut dyn FnMut(&str) -> Control,
    ) -> Result<String, VError> {
        self.requests.lock().unwrap().push(req.clone());
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| VError::Engine("fake engine has no more scripted replies".into()))?;
        for word in reply.split_inclusive(' ') {
            if on_token(word) == Control::Stop {
                break;
            }
        }
        Ok(reply)
    }
    fn count_tokens(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
    fn context_window(&self) -> u32 {
        4096
    }
}
