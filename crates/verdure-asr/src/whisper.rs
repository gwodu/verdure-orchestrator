//! whisper.cpp as an `AsrEngine`. One resident `WhisperContext` per engine;
//! one `Segmenter` per session; each finished utterance is transcribed once
//! and emitted as `AsrEvent::Final` with stream-relative timestamps.
//!
//! FFI note (skills/testing.md): all C++ is reached through `whisper-rs`;
//! nothing here is `unsafe`. A ggml abort still kills the host process, which
//! is why process isolation stays the P0 (`[OQ-58]`).

use std::path::Path;

use verdure_core::{AsrEngine, AsrEvent, AsrOptions, AsrSession, VError};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::audio::{resample_linear, WHISPER_RATE};
use crate::segment::Segmenter;
use crate::text::{apply_replacements, clean_transcript, keyterms_prompt};

pub struct WhisperAsr {
    ctx: std::sync::Arc<WhisperContext>,
    threads: i32,
}

impl WhisperAsr {
    pub fn load(model_path: &Path, threads: u32) -> Result<WhisperAsr, VError> {
        let path = model_path
            .to_str()
            .ok_or_else(|| VError::InvalidInput("model path is not valid text".into()))?;
        let ctx = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|e| VError::Engine(format!("couldn't load the speech model: {e}")))?;
        Ok(WhisperAsr {
            ctx: std::sync::Arc::new(ctx),
            threads: threads.max(1) as i32,
        })
    }
}

impl AsrEngine for WhisperAsr {
    fn id(&self) -> &str {
        "whisper"
    }
    fn open(&self, opts: &AsrOptions) -> Result<Box<dyn AsrSession>, VError> {
        Ok(Box::new(WhisperSession {
            ctx: self.ctx.clone(),
            threads: self.threads,
            opts: opts.clone(),
            seg: None,
            rate: 0,
        }))
    }
}

struct WhisperSession {
    ctx: std::sync::Arc<WhisperContext>,
    threads: i32,
    opts: AsrOptions,
    seg: Option<Segmenter>,
    rate: u32,
}

impl WhisperSession {
    fn transcribe(&self, samples: &[f32], start_sample: u64, end_sample: u64) -> Vec<AsrEvent> {
        let samples = resample_linear(samples, self.rate, WHISPER_RATE);
        let mut state = match self.ctx.create_state() {
            Ok(s) => s,
            Err(e) => {
                return vec![AsrEvent::Partial {
                    text: format!("(engine error: {e})"),
                }]
            }
        };
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let lang = self.opts.lang_hints.first().map(String::as_str);
        params.set_language(lang.or(Some("auto")));
        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        let prompt = keyterms_prompt(&self.opts.keyterms);
        if let Some(p) = &prompt {
            params.set_initial_prompt(p);
        }
        if state.full(params, &samples).is_err() {
            return Vec::new();
        }
        let mut text = String::new();
        for i in 0..state.full_n_segments() {
            if let Some(s) = state.get_segment(i) {
                if let Ok(t) = s.to_str_lossy() {
                    text.push_str(&t);
                }
            }
        }
        let text = apply_replacements(&clean_transcript(&text), &self.opts.replacements);
        if text.is_empty() {
            return Vec::new();
        }
        let ms = |s: u64| s * 1000 / self.rate.max(1) as u64;
        vec![AsrEvent::Final {
            text,
            start_ms: ms(start_sample),
            end_ms: ms(end_sample),
        }]
    }
}

impl AsrSession for WhisperSession {
    fn push_pcm(&mut self, pcm: &[f32], sample_rate: u32) -> Vec<AsrEvent> {
        if self.seg.is_none() || self.rate != sample_rate {
            self.rate = sample_rate;
            self.seg = Some(Segmenter::new(sample_rate));
        }
        let seg = self.seg.as_mut().unwrap();
        match seg.push(pcm) {
            Some(s) => self.transcribe(&s.samples, s.start_sample, s.end_sample),
            None => Vec::new(),
        }
    }
    fn finish(&mut self) -> Vec<AsrEvent> {
        match self.seg.as_mut().and_then(|s| s.flush()) {
            Some(s) => self.transcribe(&s.samples, s.start_sample, s.end_sample),
            None => Vec::new(),
        }
    }
}
