//! Speech → text on the device (knowledge/26, Part 3 §2). The pure half —
//! cutting a live stream into utterances at pauses, resampling to 16 kHz,
//! biasing with keyterms and applying the vendor's replacements — is here and
//! tested on every platform. The engine that turns an utterance into words is
//! behind a feature:
//!
//! * `whisper` — whisper.cpp through `whisper-rs`, the engine the app has run
//!   on macOS, iOS and Android (knowledge/21). Multilingual, so it is also the
//!   path for the African-language bundles until a Sahara-class model exists.
//!
//! sherpa-onnx (Parakeet, Moonshine, SenseVoice) is the second `AsrEngine`
//! the spec names; it is a new native crate and a founder decision
//! (`[OQ-107]`), so it is not here yet. Nothing above this crate would change
//! when it arrives — that is what the port is for.

pub mod audio;
pub mod segment;
pub mod text;
#[cfg(feature = "whisper")]
pub mod whisper;

pub use audio::{downmix_to_mono, resample_linear, WHISPER_RATE};
pub use segment::Segmenter;
pub use text::{apply_replacements, clean_transcript, keyterms_prompt};
