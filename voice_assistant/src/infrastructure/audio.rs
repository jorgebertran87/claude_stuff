//! Microphone capture with optional acoustic echo cancellation.

use std::process::{Command, Stdio};
use std::sync::Mutex;

use shaku::Component;

use crate::domain::model::AudioCapture;
use crate::domain::ports::{AudioCapturer, EchoRef};
use crate::infrastructure::speech::cancel_echo;

#[derive(Component)]
#[shaku(interface = AudioCapturer)]
pub struct MicrophoneCapturer {
    #[shaku(default)]
    echo_reference: Mutex<Option<EchoRef>>,
}

impl MicrophoneCapturer {
    pub fn new() -> Self {
        Self { echo_reference: Mutex::new(None) }
    }

    /// Apply echo cancellation to a raw audio buffer using the stored reference.
    pub fn apply_echo_cancellation(
        &self,
        raw:         &[u8],
        sample_rate: u32,
        _sample_width: u16,
    ) -> Vec<u8> {
        let guard = self.echo_reference.lock().unwrap();
        let Some((ref ref_bytes, ref_rate, _)) = *guard else {
            return raw.to_vec();
        };

        let mic_samples  = bytes_to_i16(raw);
        let ref_samples  = bytes_to_i16(ref_bytes);

        // Resample reference if needed (linear interpolation)
        let ref_resampled = if ref_rate != sample_rate {
            resample(&ref_samples, ref_rate, sample_rate)
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
        &self,
        timeout_ms:           Option<u64>,
        phrase_time_limit_ms: Option<u64>,
        pause_threshold_ms:   Option<u64>,
    ) -> Option<AudioCapture> {
        let max_secs   = phrase_time_limit_ms.or(timeout_ms).unwrap_or(8_000) / 1_000;
        let pause_secs = pause_threshold_ms.unwrap_or(1_500) as f64 / 1_000.0;
        let tmp        = "/tmp/voice_capture.wav";

        // `rec` (sox): wait for voice onset, then record until pause_secs of silence.
        // silence 1 0.1 2%  → start when 1 sample above 2% amplitude within 0.1 s
        // silence 1 <pause> 2% → stop after <pause> s of silence below 2%
        // trim 0 <max_secs>  → hard cap on duration
        let pause_arg = format!("{pause_secs:.1}");
        let max_arg   = format!("{max_secs}");

        let ok = Command::new("rec")
            .args([
                "-q",
                "-c", "1", "-r", "16000",
                "-e", "signed-integer", "-b", "16",
                tmp,
                "silence", "1", "0.1", "2%",
                "1", &pause_arg, "2%",
                "trim", "0", &max_arg,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !ok { return None; }
        let bytes = std::fs::read(tmp).ok()?;
        if bytes.len() <= 44 { return None; }   // only WAV header → silence
        Some(AudioCapture::new(bytes, 16_000, 2))
    }

    fn calibrate(&self, _duration_secs: f64) {
        // sox `rec` adapts automatically; nothing to calibrate.
    }
    fn mute(&self)   {}
    fn unmute(&self) {}

    fn set_echo_reference(&self, reference: Option<EchoRef>) {
        *self.echo_reference.lock().unwrap() = reference;
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
