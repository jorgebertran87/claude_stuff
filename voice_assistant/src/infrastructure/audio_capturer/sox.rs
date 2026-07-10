//! Sox-based microphone capture adapter (pre-cpal).
//! Uses the `rec` command from sox for audio capture with silence detection.

use std::process::{Command, Stdio};
use std::sync::Mutex;

use shaku::Component;

use crate::domain::model::AudioCapture;
use crate::domain::ports::{AudioCapturer, EchoRef};
use crate::infrastructure::audio_capturer::cpal::{bytes_to_i16, i16_to_bytes, resample};
use crate::infrastructure::transcriber::speech::cancel_echo;

#[derive(Component)]
#[shaku(interface = AudioCapturer)]
pub struct SoxMicrophoneCapturer {
    #[shaku(default)]
    echo_reference: Mutex<Option<EchoRef>>,
}

impl SoxMicrophoneCapturer {
    pub fn new() -> Self {
        Self { echo_reference: Mutex::new(None) }
    }

    /// Apply echo cancellation to a raw audio buffer using the stored reference.
    pub fn apply_echo_cancellation(
        &self,
        raw:          &[u8],
        sample_rate:  u32,
        _sample_width: u16,
    ) -> Vec<u8> {
        let Some((ref_samples, ref_rate)) = ({
            let guard = self.echo_reference.lock().unwrap();
            guard.as_ref().map(|(bytes, rate, _)| (bytes_to_i16(bytes), *rate))
        }) else {
            return raw.to_vec();
        };

        let mic_samples = bytes_to_i16(raw);

        let ref_resampled = if ref_rate != sample_rate {
            resample(&ref_samples, ref_rate, sample_rate)
        } else {
            ref_samples
        };

        let cleaned = cancel_echo(&mic_samples, &ref_resampled, 0.95);
        i16_to_bytes(&cleaned)
    }
}

impl Default for SoxMicrophoneCapturer {
    fn default() -> Self { Self::new() }
}

impl AudioCapturer for SoxMicrophoneCapturer {
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
