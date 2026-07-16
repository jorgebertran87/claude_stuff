//! Microphone capture adapter via cpal with voice-activity detection and
//! optional acoustic echo cancellation.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use shaku::Component;

use crate::domain::model::AudioCapture;
use crate::domain::ports::{AudioCapturer, EchoRef};
use crate::infrastructure::audio_capturer::shared::audio::{
    bytes_to_i16, cancel_echo, encode_wav, i16_to_bytes, resample,
};
use crate::infrastructure::audio_capturer::shared::vad::{
    CaptureDecision, CaptureEvent, SpeechAccumulator,
};

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

        // Poll interval — roughly one chunk's worth
        let poll = Duration::from_millis(50);

        // The speech-segmentation policy lives in the shared VAD state
        // machine; this loop only shuttles stream events into it. (SRP)
        let mut accumulator = SpeechAccumulator::new(
            VAD_THRESHOLD,
            SILENCE_THRESHOLD,
            pause_threshold_ms,
            timeout_ms,
            phrase_time_limit_ms.max(timeout_ms),
            poll.as_millis() as u64,
        );
        let start = Instant::now();

        loop {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let decision = match rx.recv_timeout(poll) {
                Ok(chunk) => accumulator.on_event(CaptureEvent::Chunk(&chunk), elapsed_ms),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    accumulator.on_event(CaptureEvent::Timeout, elapsed_ms)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            };
            if let CaptureDecision::Stop = decision {
                break;
            }
        }

        drop(stream);
        accumulator.finish()
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
        let raw = encode_wav(&samples, 16_000);
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

    fn apply_echo_cancellation(
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
