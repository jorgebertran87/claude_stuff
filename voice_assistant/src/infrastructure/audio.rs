//! Microphone capture via cpal with basic voice-activity detection and
//! optional acoustic echo cancellation.

use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use shaku::Component;

use crate::domain::model::AudioCapture;
use crate::domain::ports::{AudioCapturer, EchoRef};
use crate::infrastructure::speech::cancel_echo;

/// Amplitude threshold for voice-onset detection (fraction of i16 max).
const VAD_THRESHOLD: f64 = 0.02;

/// Silence below this fraction of i16 max is considered "no signal."
const SILENCE_THRESHOLD: f64 = 0.015;

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

    /// Blocking capture loop — collects samples until silence or timeout.
    fn record_loop(
        &self,
        timeout_ms:           u64,
        phrase_time_limit_ms: u64,
        pause_threshold_ms:   u64,
    ) -> Option<Vec<i16>> {
        let host = cpal::default_host();
        let device = host.default_input_device()?;

        let config = cpal::StreamConfig {
            channels: 1,
            sample_rate: cpal::SampleRate(16_000),
            buffer_size: cpal::BufferSize::Default,
        };

        let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<i16>>(32);
        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let _ = tx.try_send(data.to_vec());
                },
                |err| eprintln!("[audio capture error: {err}]"),
                None,
            )
            .ok()?;

        stream.play().ok()?;

        let mut all_samples: Vec<i16> = Vec::new();
        let mut silence_ms: u64 = 0;
        let mut voice_heard = false;
        let max_dur = Duration::from_millis(phrase_time_limit_ms.max(timeout_ms));
        let pause_dur = Duration::from_millis(pause_threshold_ms);
        let start = Instant::now();

        // Poll interval — roughly one chunk's worth
        let poll = Duration::from_millis(50);

        while start.elapsed() < max_dur {
            match rx.recv_timeout(poll) {
                Ok(chunk) => {
                    let amp = rms_amplitude(&chunk);

                    if !voice_heard && amp > VAD_THRESHOLD {
                        voice_heard = true;
                    }

                    if voice_heard {
                        all_samples.extend_from_slice(&chunk);

                        if amp < SILENCE_THRESHOLD {
                            silence_ms += poll.as_millis() as u64;
                            if silence_ms >= pause_threshold_ms {
                                break;
                            }
                        } else {
                            silence_ms = 0;
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    if voice_heard {
                        silence_ms += poll.as_millis() as u64;
                        if silence_ms >= pause_threshold_ms {
                            break;
                        }
                    }
                    // If we never heard voice and timeout expired, return None
                    if !voice_heard && start.elapsed() >= Duration::from_millis(timeout_ms) {
                        drop(stream);
                        return None;
                    }
                }
            }
        }

        drop(stream);

        if all_samples.is_empty() {
            None
        } else {
            Some(all_samples)
        }
    }

    /// Encode raw i16 samples as a WAV byte vector (16-bit mono PCM).
    fn encode_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let data_size = (samples.len() * 2) as u32;
        let file_size = 44 + data_size;

        let mut wav = Vec::with_capacity(file_size as usize);

        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(file_size - 8).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // fmt  chunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());          // chunk size
        wav.extend_from_slice(&1u16.to_le_bytes());            // PCM
        wav.extend_from_slice(&1u16.to_le_bytes());            // mono
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes());            // block align
        wav.extend_from_slice(&16u16.to_le_bytes());           // bits per sample

        // data chunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_size.to_le_bytes());
        for s in samples {
            wav.extend_from_slice(&s.to_le_bytes());
        }

        wav
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
        let timeout   = timeout_ms.unwrap_or(8_000);
        let phrase    = phrase_time_limit_ms.unwrap_or(8_000);
        let pause     = pause_threshold_ms.unwrap_or(1_500);

        let samples = self.record_loop(timeout, phrase, pause)?;
        let raw = Self::encode_wav(&samples, 16_000);
        Some(AudioCapture::new(raw, 16_000, 2))
    }

    fn calibrate(&self, _duration_secs: f64) {
        // cpal auto-configures; nothing to calibrate.
    }
    fn mute(&self)   {}
    fn unmute(&self) {}

    fn set_echo_reference(&self, reference: Option<EchoRef>) {
        *self.echo_reference.lock().unwrap() = reference;
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn rms_amplitude(samples: &[i16]) -> f64 {
    if samples.is_empty() { return 0.0; }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
    (sum_sq / samples.len() as f64).sqrt() / 32768.0
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_wav_produces_valid_header() {
        let samples = vec![0i16; 100];
        let wav = MicrophoneCapturer::encode_wav(&samples, 16000);
        // RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        // fmt chunk
        assert_eq!(&wav[12..16], b"fmt ");
        // data chunk
        assert_eq!(&wav[36..40], b"data");
        // data size = 100 samples * 2 bytes
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_size, 200);
        // total = 12(RIFF) + 24(fmt) + 8(data_header) + 200(data) = 244
        let file_size = u32::from_le_bytes([wav[4], wav[5], wav[6], wav[7]]);
        assert_eq!(file_size + 8, 244);
    }

    #[test]
    fn encode_wav_empty_returns_minimal_wav() {
        let samples: Vec<i16> = vec![];
        let wav = MicrophoneCapturer::encode_wav(&samples, 16000);
        assert_eq!(wav.len(), 44);
        let data_size = u32::from_le_bytes([wav[40], wav[41], wav[42], wav[43]]);
        assert_eq!(data_size, 0);
    }

    #[test]
    fn rms_amplitude_silence_is_zero() {
        let samples = vec![0i16; 100];
        assert_eq!(rms_amplitude(&samples), 0.0);
    }

    #[test]
    fn rms_amplitude_full_scale_is_one() {
        let samples = vec![i16::MAX; 100];
        let rms = rms_amplitude(&samples);
        assert!(rms > 0.9 && rms <= 1.0, "expected near 1.0, got {rms}");
    }

    #[test]
    fn rms_amplitude_empty_is_zero() {
        assert_eq!(rms_amplitude(&[]), 0.0);
    }
}

// ── SoxMicrophoneCapturer (sox rec-based, pre-cpal) ──────────────────────────

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
