//! Microphone capture with optional acoustic echo cancellation.

use crate::domain::model::AudioCapture;
use crate::domain::ports::{AudioCapturer, EchoRef};
use crate::infrastructure::speech::cancel_echo;

pub struct MicrophoneCapturer {
    echo_reference: Option<EchoRef>,
}

impl MicrophoneCapturer {
    pub fn new() -> Self {
        Self { echo_reference: None }
    }

    /// Apply echo cancellation to a raw audio buffer using the stored reference.
    pub fn apply_echo_cancellation(
        &self,
        raw:         &[u8],
        sample_rate: u32,
        sample_width: u16,
    ) -> Vec<u8> {
        let Some((ref ref_bytes, ref_rate, _)) = self.echo_reference else {
            return raw.to_vec();
        };

        let mic_samples  = bytes_to_i16(raw);
        let ref_samples  = bytes_to_i16(ref_bytes);

        // Resample reference if needed (linear interpolation)
        let ref_resampled = if *ref_rate != sample_rate {
            resample(&ref_samples, *ref_rate, sample_rate)
        } else {
            ref_samples
        };

        let cleaned = cancel_echo(&mic_samples, &ref_resampled, 0.95);
        i16_to_bytes(&cleaned)
    }
}

impl Default for MicrophoneCapturer {
    fn default() -> Self { Self::new() }
}

impl AudioCapturer for MicrophoneCapturer {
    fn capture(
        &mut self,
        _timeout_ms:           Option<u64>,
        _phrase_time_limit_ms: Option<u64>,
        _pause_threshold_ms:   Option<u64>,
    ) -> Option<AudioCapture> {
        // Real implementation would read from microphone hardware.
        None
    }

    fn calibrate(&mut self, _duration_secs: f64) {}
    fn mute(&mut self)   {}
    fn unmute(&mut self) {}

    fn set_echo_reference(&mut self, reference: Option<EchoRef>) {
        self.echo_reference = reference;
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

pub fn bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}

pub fn i16_to_bytes(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

fn resample(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    let n_out = (samples.len() as u64 * to_rate as u64 / from_rate as u64) as usize;
    (0..n_out)
        .map(|i| {
            let src = i as f64 * from_rate as f64 / to_rate as f64;
            let lo  = src.floor() as usize;
            let hi  = (lo + 1).min(samples.len().saturating_sub(1));
            let t   = src.fract() as f32;
            (samples[lo] as f32 * (1.0 - t) + samples[hi] as f32 * t) as i16
        })
        .collect()
}
