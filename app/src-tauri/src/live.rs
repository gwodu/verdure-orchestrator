//! Live (microphone) transcription — speak and watch the text appear, entirely
//! on-device (knowledge/18 — wrap the model, no separate app). The companion to
//! `transcribe.rs`'s drop-a-file flow.
//!
//! Whisper is not a streaming model, and on a CPU (the founder's 2019 Intel Mac
//! runs base.en at ~3.9× real-time) re-transcribing a growing buffer every second
//! would fall behind. So we **segment and commit**: capture the mic, cut the
//! stream into utterance-sized segments at natural pauses (a simple energy gate,
//! so we never split a word), and transcribe each segment ONCE as it completes.
//! Text lands a few seconds after each pause — the normal live-captioning cadence,
//! and it keeps up with real time on this hardware.
//!
//! The Whisper context is loaded ONCE per session (in `lib.rs`) and reused across
//! every segment — no per-segment model reload.
//!
//! Robustness note (knowledge/21 gap #4): ggml can `abort()` the whole process on
//! a model error, and live mode runs Whisper repeatedly, so it raises that
//! exposure. The durable fix is running the runtime in a separate process; until
//! then we lean on the model-file validation in `model::ensure`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use tauri::{AppHandle, Emitter, Runtime};
use whisper_rs::WhisperContext;

use crate::audio;
use crate::transcribe;

/// Splits a continuous mono stream into utterance-sized segments at natural
/// pauses, so a finished segment never cuts a word in half. **Pure** (no audio
/// device, no I/O) — the cut logic is fully unit-tested; the live worker only
/// feeds it samples.
///
/// Energy-gated VAD-lite: a sample counts as "voiced" if its magnitude exceeds
/// `threshold`. A run of quiet longer than `silence` ends the current segment
/// (once it is at least `min` long); `max` force-cuts a non-stop talker so they
/// still get timely text. Pure silence is discarded, not transcribed.
pub(crate) struct Segmenter {
    threshold: f32,
    min_samples: usize,
    max_samples: usize,
    silence_samples: usize,
    buf: Vec<f32>,
    trailing_quiet: usize,
    voiced: bool,
}

impl Segmenter {
    /// Build a segmenter for a given source sample rate, with sensible defaults:
    /// cut on ~0.6 s of silence once a segment is ≥ ~1 s, force-cut at ~12 s.
    pub(crate) fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, 0.012, 1.0, 12.0, 0.6)
    }

    fn with_params(
        sample_rate: u32,
        threshold: f32,
        min_secs: f32,
        max_secs: f32,
        silence_secs: f32,
    ) -> Self {
        let s = sample_rate as f32;
        Segmenter {
            threshold,
            min_samples: (min_secs * s) as usize,
            max_samples: (max_secs * s) as usize,
            silence_samples: (silence_secs * s) as usize,
            buf: Vec::new(),
            trailing_quiet: 0,
            voiced: false,
        }
    }

    /// Feed a chunk of mono samples. Returns a completed segment when a cut point
    /// is reached (a pause after speech, or the max length), else `None`. Pure
    /// silence that hits the max length is dropped (returns `None`).
    pub(crate) fn push(&mut self, chunk: &[f32]) -> Option<Vec<f32>> {
        for &s in chunk {
            self.buf.push(s);
            if s.abs() >= self.threshold {
                self.trailing_quiet = 0;
                self.voiced = true;
            } else {
                self.trailing_quiet += 1;
            }
        }

        // A pause after real speech: a natural, word-safe cut.
        if self.voiced
            && self.buf.len() >= self.min_samples
            && self.trailing_quiet >= self.silence_samples
        {
            return Some(self.take());
        }

        // Force a cut so a continuous talker still gets text — but never ship a
        // segment of pure silence to the model; just reset.
        if self.buf.len() >= self.max_samples {
            if self.voiced {
                return Some(self.take());
            }
            self.reset();
        }
        None
    }

    /// On stop, hand back whatever is buffered if it contains speech.
    pub(crate) fn flush(&mut self) -> Option<Vec<f32>> {
        if self.voiced && !self.buf.is_empty() {
            Some(self.take())
        } else {
            self.reset();
            None
        }
    }

    fn take(&mut self) -> Vec<f32> {
        let out = std::mem::take(&mut self.buf);
        self.reset();
        out
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.trailing_quiet = 0;
        self.voiced = false;
    }
}

/// Run a live-transcription session until `stop` is set. Blocking — meant to own a
/// dedicated thread (the cpal `Stream` and the loaded `WhisperContext` both stay
/// on this one thread; nothing model-side ever crosses a thread boundary). The
/// model file at `model_path` is already downloaded + size-validated by the
/// caller; we LOAD it here. Emits to the UI:
///   - `live:status`  — human status lines ("Listening… speak now.")
///   - `live:segment` — a finished chunk of transcript (string)
///   - `live:error`   — capture/model couldn't start / failed (string); session ends
///   - `live:stopped` — the session has ended and the mic is released
pub(crate) fn run<R: Runtime>(app: AppHandle<R>, model_path: PathBuf, stop: Arc<AtomicBool>) {
    let ctx = match transcribe::load(&model_path) {
        Ok(c) => c,
        Err(e) => {
            let _ = app.emit("live:error", e);
            return;
        }
    };

    let (tx, rx) = std::sync::mpsc::channel::<Vec<f32>>();

    let (stream, sample_rate) = match build_stream(&app, tx) {
        Ok(s) => s,
        Err(e) => {
            let _ = app.emit("live:error", e);
            return;
        }
    };
    if let Err(e) = stream.play() {
        let _ = app.emit("live:error", format!("couldn't start the microphone: {e}"));
        return;
    }

    let _ = app.emit("live:status", "Listening… speak now.");
    capture_loop(&app, &ctx, &rx, &stop, sample_rate);

    drop(stream); // release the microphone before signalling done
    let _ = app.emit("live:stopped", ());
}

/// The receive/segment/transcribe loop. Split out so the timeout-driven control
/// flow stays readable; `recv_timeout` lets us notice `stop` even in silence.
fn capture_loop<R: Runtime>(
    app: &AppHandle<R>,
    ctx: &WhisperContext,
    rx: &Receiver<Vec<f32>>,
    stop: &AtomicBool,
    sample_rate: u32,
) {
    let mut seg = Segmenter::new(sample_rate);
    while !stop.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(chunk) => {
                if let Some(segment) = seg.push(&chunk) {
                    emit_segment(app, ctx, &segment, sample_rate);
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if let Some(segment) = seg.flush() {
        emit_segment(app, ctx, &segment, sample_rate);
    }
}

/// Resample a finished segment to Whisper's 16 kHz, transcribe it with the resident
/// context, and emit the text (skipping empty/no-speech results).
fn emit_segment<R: Runtime>(
    app: &AppHandle<R>,
    ctx: &WhisperContext,
    segment: &[f32],
    sample_rate: u32,
) {
    let samples = audio::resample_linear(segment, sample_rate, audio::WHISPER_RATE);
    match transcribe::transcribe_with(ctx, &samples) {
        Ok(text) if !text.trim().is_empty() => {
            let _ = app.emit("live:segment", text);
        }
        Ok(_) => {} // no speech in this chunk — nothing to add
        Err(e) => {
            let _ = app.emit("live:status", format!("(skipped a bit — {e})"));
        }
    }
}

/// Open the default input device and build a running input stream that downmixes
/// every frame to mono and forwards it to `tx`. Returns the stream (kept alive by
/// the caller) and the device's sample rate (for the segmenter + resampler).
fn build_stream<R: Runtime>(
    app: &AppHandle<R>,
    tx: Sender<Vec<f32>>,
) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no microphone was found on this computer".to_string())?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("couldn't read the microphone settings: {e}"))?;

    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let app_err = app.clone();
    let err_fn = move |err| {
        let _ = app_err.emit("live:status", format!("microphone hiccup — {err}"));
    };

    let stream = match format {
        cpal::SampleFormat::F32 => build_typed::<f32, _>(&device, &config, channels, tx, err_fn),
        cpal::SampleFormat::I16 => build_typed::<i16, _>(&device, &config, channels, tx, err_fn),
        cpal::SampleFormat::U16 => build_typed::<u16, _>(&device, &config, channels, tx, err_fn),
        other => Err(format!(
            "this microphone's audio format ({other:?}) isn't supported yet"
        )),
    }?;
    Ok((stream, sample_rate))
}

/// Build the input stream for a concrete sample type, converting frames to mono
/// f32 in the (real-time) audio callback. Kept generic so f32/i16/u16 mics share
/// one path. `audio::downmix_to_mono` is the same averaging the file path uses.
fn build_typed<T, E>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    tx: Sender<Vec<f32>>,
    err_fn: E,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
    E: FnMut(cpal::StreamError) + Send + 'static,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let interleaved: Vec<f32> = data.iter().map(|s| f32::from_sample(*s)).collect();
                let mono = audio::downmix_to_mono(&interleaved, channels);
                // A dropped chunk only costs a sliver of audio; never panic the
                // audio thread if the worker has gone away.
                let _ = tx.send(mono);
            },
            err_fn,
            None,
        )
        .map_err(|e| format!("couldn't open the microphone: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 16_000;

    fn loud(n: usize) -> Vec<f32> {
        vec![0.5; n]
    }
    fn quiet(n: usize) -> Vec<f32> {
        vec![0.0; n]
    }

    #[test]
    fn cuts_on_a_pause_after_speech() {
        let mut seg = Segmenter::new(RATE);
        // 2 s of speech, then 0.7 s of silence (> the 0.6 s gate).
        assert!(seg.push(&loud(2 * RATE as usize)).is_none());
        let out = seg
            .push(&quiet((0.7 * RATE as f32) as usize))
            .expect("a pause after speech should complete a segment");
        // The segment spans the speech plus the trailing silence that triggered it.
        assert!(out.len() >= 2 * RATE as usize);
    }

    #[test]
    fn does_not_cut_before_the_minimum_length() {
        let mut seg = Segmenter::new(RATE);
        // A short word (0.3 s) then a long pause — still under the 1 s minimum.
        assert!(seg.push(&loud((0.3 * RATE as f32) as usize)).is_none());
        assert!(seg.push(&quiet(RATE as usize)).is_none());
    }

    #[test]
    fn force_cuts_a_nonstop_talker_at_the_max() {
        let mut seg = Segmenter::new(RATE);
        // 13 s of unbroken speech exceeds the 12 s force-cut.
        let out = seg.push(&loud(13 * RATE as usize));
        assert!(out.is_some(), "a continuous talker must still get a segment");
    }

    #[test]
    fn discards_pure_silence() {
        let mut seg = Segmenter::new(RATE);
        // 13 s of silence: past the max, but no speech — nothing to transcribe.
        assert!(seg.push(&quiet(13 * RATE as usize)).is_none());
        assert!(seg.flush().is_none());
    }

    #[test]
    fn flush_returns_trailing_speech() {
        let mut seg = Segmenter::new(RATE);
        seg.push(&loud((1.5 * RATE as f32) as usize));
        let out = seg.flush().expect("buffered speech should flush on stop");
        assert!(!out.is_empty());
        // After a flush the segmenter is empty again.
        assert!(seg.flush().is_none());
    }
}
