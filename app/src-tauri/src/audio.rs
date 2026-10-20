//! Audio helpers for live dictation (`live.rs`): bring captured mic audio to the
//! 16 kHz mono f32 PCM that whisper.cpp expects. Pure functions, unit-tested.
//!
//! Resampling is intentionally simple (linear interpolation, no anti-alias
//! filter): Whisper is robust to it. A proper resampler (rubato) is a later
//! upgrade if accuracy needs it. (A file decoder built on symphonia lived here
//! until 2026-10-20; it served the drop-a-file screen removed in July.)

/// Whisper's required input sample rate.
pub(crate) const WHISPER_RATE: u32 = 16_000;

/// Average channels down to mono. Reused by live capture to downmix mic frames.
pub(crate) fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Linear-interpolation resample from `src_rate` to `dst_rate`. Reused by live
/// capture to bring a finished mic segment to Whisper's 16 kHz.
pub(crate) fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = dst_rate as f64 / src_rate as f64;
    let out_len = ((input.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let a = input.get(idx).copied().unwrap_or(0.0);
        let b = input.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_rate_is_unchanged() {
        let input = vec![0.1, 0.2, 0.3, 0.4];
        assert_eq!(resample_linear(&input, 16_000, 16_000), input);
    }

    #[test]
    fn downsampling_halves_the_length() {
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let out = resample_linear(&input, 32_000, 16_000);
        assert!((out.len() as i32 - 50).abs() <= 1, "got {}", out.len());
    }

    #[test]
    fn upsampling_doubles_the_length() {
        let input: Vec<f32> = (0..50).map(|i| i as f32).collect();
        let out = resample_linear(&input, 8_000, 16_000);
        assert!((out.len() as i32 - 100).abs() <= 1, "got {}", out.len());
    }

    #[test]
    fn stereo_downmix_averages_channels() {
        // L,R interleaved: (0,2),(4,6) -> 1, 5
        let interleaved = vec![0.0, 2.0, 4.0, 6.0];
        assert_eq!(downmix_to_mono(&interleaved, 2), vec![1.0, 5.0]);
    }

    #[test]
    fn mono_passthrough() {
        let mono = vec![1.0, 2.0, 3.0];
        assert_eq!(downmix_to_mono(&mono, 1), mono);
    }
}
