//! Cut a continuous mono stream into utterance-sized segments at natural
//! pauses, so a segment never splits a word (lifted from the app's `live.rs`,
//! where it has run on three platforms). Pure: the engine only feeds it
//! samples. Whisper is not a streaming model, so this "segment and commit"
//! cadence is what makes live dictation keep up on a CPU — text lands a few
//! seconds after each pause.

pub struct Segmenter {
    threshold: f32,
    min_samples: usize,
    max_samples: usize,
    silence_samples: usize,
    buf: Vec<f32>,
    trailing_quiet: usize,
    voiced: bool,
    /// Samples consumed so far, for timestamps.
    consumed: u64,
    seg_start: u64,
}

/// A finished segment with its position in the stream.
pub struct Segment {
    pub samples: Vec<f32>,
    pub start_sample: u64,
    pub end_sample: u64,
}

impl Segmenter {
    /// Defaults: cut on ~0.6 s of silence once a segment is ≥ ~1 s; force-cut
    /// at ~12 s; a sample is "voiced" above 0.012.
    pub fn new(sample_rate: u32) -> Self {
        Self::with_params(sample_rate, 0.012, 1.0, 12.0, 0.6)
    }

    pub fn with_params(
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
            consumed: 0,
            seg_start: 0,
        }
    }

    /// Feed samples; returns a completed segment at a cut point. Pure silence
    /// that reaches the maximum length is dropped, never transcribed.
    pub fn push(&mut self, chunk: &[f32]) -> Option<Segment> {
        for &s in chunk {
            if self.buf.is_empty() {
                self.seg_start = self.consumed;
            }
            self.buf.push(s);
            self.consumed += 1;
            if s.abs() >= self.threshold {
                self.trailing_quiet = 0;
                self.voiced = true;
            } else {
                self.trailing_quiet += 1;
            }
        }
        if self.voiced
            && self.buf.len() >= self.min_samples
            && self.trailing_quiet >= self.silence_samples
        {
            return Some(self.take());
        }
        if self.buf.len() >= self.max_samples {
            if self.voiced {
                return Some(self.take());
            }
            self.reset();
        }
        None
    }

    /// On stop: whatever is buffered, if it contains speech.
    pub fn flush(&mut self) -> Option<Segment> {
        if self.voiced && !self.buf.is_empty() {
            Some(self.take())
        } else {
            self.reset();
            None
        }
    }

    fn take(&mut self) -> Segment {
        let samples = std::mem::take(&mut self.buf);
        let seg = Segment {
            samples,
            start_sample: self.seg_start,
            end_sample: self.consumed,
        };
        self.reset();
        seg
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.trailing_quiet = 0;
        self.voiced = false;
    }
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
    fn cuts_on_a_pause_after_speech_with_timestamps() {
        let mut seg = Segmenter::new(RATE);
        assert!(seg.push(&loud(2 * RATE as usize)).is_none());
        let out = seg
            .push(&quiet((0.7 * RATE as f32) as usize))
            .expect("a pause completes a segment");
        assert!(out.samples.len() >= 2 * RATE as usize);
        assert_eq!(out.start_sample, 0);
        assert_eq!(out.end_sample, out.samples.len() as u64);
        // The next segment starts where this one ended.
        seg.push(&loud(2 * RATE as usize));
        let next = seg.push(&quiet(RATE as usize)).unwrap();
        assert_eq!(next.start_sample, out.end_sample);
    }

    #[test]
    fn does_not_cut_before_the_minimum_length() {
        let mut seg = Segmenter::new(RATE);
        assert!(seg.push(&loud((0.2 * RATE as f32) as usize)).is_none());
        assert!(seg.push(&quiet((0.6 * RATE as f32) as usize)).is_none());
    }

    #[test]
    fn force_cuts_a_nonstop_talker_and_discards_pure_silence() {
        let mut seg = Segmenter::new(RATE);
        assert!(seg.push(&loud(13 * RATE as usize)).is_some());
        let mut seg = Segmenter::new(RATE);
        assert!(seg.push(&quiet(13 * RATE as usize)).is_none());
        assert!(seg.flush().is_none());
    }

    #[test]
    fn flush_returns_trailing_speech_once() {
        let mut seg = Segmenter::new(RATE);
        seg.push(&loud((1.5 * RATE as f32) as usize));
        assert!(seg.flush().is_some());
        assert!(seg.flush().is_none());
    }
}
