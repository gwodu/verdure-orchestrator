//! Sample-rate and channel plumbing (lifted from the app's `audio.rs`).

/// Every engine we ship consumes 16 kHz mono.
pub const WHISPER_RATE: u32 = 16_000;

/// Average interleaved channels into one.
pub fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Linear-interpolation resampling. Good enough for speech recognition
/// (the model's own front end low-passes); not for music.
pub fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
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
    fn resampling_scales_length_and_passes_same_rate_through() {
        let input: Vec<f32> = (0..100).map(|i| i as f32).collect();
        assert_eq!(resample_linear(&input, 16_000, 16_000), input);
        assert!((resample_linear(&input, 32_000, 16_000).len() as i32 - 50).abs() <= 1);
        assert!((resample_linear(&input[..50], 8_000, 16_000).len() as i32 - 100).abs() <= 1);
        assert!(resample_linear(&[], 48_000, 16_000).is_empty());
        // Interpolated values stay within the input's range.
        let out = resample_linear(&input, 44_100, 16_000);
        assert!(out.iter().all(|v| (0.0..=99.0).contains(v)));
    }

    #[test]
    fn downmix_averages_channels() {
        assert_eq!(downmix_to_mono(&[0.0, 2.0, 4.0, 6.0], 2), vec![1.0, 5.0]);
        assert_eq!(downmix_to_mono(&[1.0, 2.0, 3.0], 1), vec![1.0, 2.0, 3.0]);
    }
}
