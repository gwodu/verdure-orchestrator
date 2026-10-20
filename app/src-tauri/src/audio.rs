//! Audio decoding — turn whatever file the user dropped (mp3, m4a, wav, flac…)
//! into the 16 kHz mono f32 PCM that whisper.cpp expects. Pure Rust (symphonia),
//! so there's no ffmpeg/external tool to install — in keeping with the pivot's
//! "one interface, no extra apps" goal (knowledge/18).
//!
//! Resampling is intentionally simple for v1 (linear interpolation, no
//! anti-alias filter): Whisper is robust to it, and "dead-simple" is the bar the
//! founder set. A proper resampler (rubato) is a later upgrade if accuracy needs it.

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Whisper's required input sample rate.
const TARGET_RATE: u32 = 16_000;

/// Decode an audio file to 16 kHz mono f32 samples, ready for `transcribe`.
pub(crate) fn decode_to_mono_16k(path: &Path) -> Result<Vec<f32>, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("couldn't open the file: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| "this audio format isn't supported yet".to_string())?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no audio track in this file".to_string())?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("couldn't read this audio: {e}"))?;

    let mut src_rate = track.codec_params.sample_rate.unwrap_or(TARGET_RATE);
    let mut channels = 1usize;
    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break, // end of stream
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                let spec = *audio_buf.spec();
                src_rate = spec.rate;
                channels = spec.channels.count().max(1);
                let mut sb = SampleBuffer::<f32>::new(audio_buf.capacity() as u64, spec);
                sb.copy_interleaved_ref(audio_buf);
                interleaved.extend_from_slice(sb.samples());
            }
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue, // skip bad packet
            Err(_) => break,
        }
    }

    if interleaved.is_empty() {
        return Err("couldn't find any audio in this file".to_string());
    }

    let mono = downmix_to_mono(&interleaved, channels);
    Ok(resample_linear(&mono, src_rate, TARGET_RATE))
}

/// Whisper's required input sample rate, shared with live capture (`live.rs`).
pub(crate) const WHISPER_RATE: u32 = TARGET_RATE;

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
