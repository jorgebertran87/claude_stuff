//! Voice-activity detection: a pure speech-accumulation state machine that
//! decides when a spoken phrase starts and ends, independent of any audio
//! backend. Capture adapters feed it stream events and act on its decisions.

use crate::infrastructure::shared::audio::rms_amplitude;

/// An event observed by a capture loop.
pub enum CaptureEvent<'a> {
    /// A chunk of samples arrived from the input stream.
    Chunk(&'a [i16]),
    /// No samples arrived within one poll interval.
    Timeout,
}

/// What the capture loop should do after an event.
#[derive(Debug, PartialEq, Eq)]
pub enum CaptureDecision {
    Continue,
    Stop,
}

/// Accumulates speech samples between voice onset and a sustained pause.
///
/// Thresholds are normalized RMS amplitudes (fraction of i16 max). All
/// durations are milliseconds; `elapsed_ms` is measured by the caller from
/// the start of the capture.
pub struct SpeechAccumulator {
    vad_threshold:      f64,
    silence_threshold:  f64,
    pause_threshold_ms: u64,
    timeout_ms:         u64,
    max_duration_ms:    u64,
    poll_ms:            u64,
    samples:            Vec<i16>,
    silence_ms:         u64,
    voice_heard:        bool,
}

impl SpeechAccumulator {
    pub fn new(
        vad_threshold:      f64,
        silence_threshold:  f64,
        pause_threshold_ms: u64,
        timeout_ms:         u64,
        max_duration_ms:    u64,
        poll_ms:            u64,
    ) -> Self {
        Self {
            vad_threshold,
            silence_threshold,
            pause_threshold_ms,
            timeout_ms,
            max_duration_ms,
            poll_ms,
            samples: Vec::new(),
            silence_ms: 0,
            voice_heard: false,
        }
    }

    /// Advance the state machine with one capture-loop event.
    pub fn on_event(&mut self, event: CaptureEvent, elapsed_ms: u64) -> CaptureDecision {
        if elapsed_ms >= self.max_duration_ms {
            return CaptureDecision::Stop;
        }

        match event {
            CaptureEvent::Chunk(chunk) => {
                let amp = rms_amplitude(chunk);

                if !self.voice_heard && amp > self.vad_threshold {
                    self.voice_heard = true;
                }

                if self.voice_heard {
                    self.samples.extend_from_slice(chunk);

                    if amp < self.silence_threshold {
                        self.silence_ms += self.poll_ms;
                        if self.silence_ms >= self.pause_threshold_ms {
                            return CaptureDecision::Stop;
                        }
                    } else {
                        self.silence_ms = 0;
                    }
                }
            }
            CaptureEvent::Timeout => {
                if self.voice_heard {
                    self.silence_ms += self.poll_ms;
                    if self.silence_ms >= self.pause_threshold_ms {
                        return CaptureDecision::Stop;
                    }
                }
                // Never heard voice and the listening window expired.
                if !self.voice_heard && elapsed_ms >= self.timeout_ms {
                    return CaptureDecision::Stop;
                }
            }
        }

        CaptureDecision::Continue
    }

    /// Consume the accumulator, yielding the captured speech (if any).
    pub fn finish(self) -> Option<Vec<i16>> {
        if self.samples.is_empty() {
            None
        } else {
            Some(self.samples)
        }
    }
}
