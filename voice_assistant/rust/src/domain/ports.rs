use std::sync::{Arc, atomic::AtomicBool};
use crate::domain::model::{AudioCapture, Language};

pub type EchoRef = (Vec<u8>, u32, u16); // (raw_bytes, sample_rate, sample_width)

pub trait AudioCapturer: Send {
    fn capture(
        &mut self,
        timeout_ms:         Option<u64>,
        phrase_time_limit_ms: Option<u64>,
        pause_threshold_ms: Option<u64>,
    ) -> Option<AudioCapture>;

    fn calibrate(&mut self, duration_secs: f64);
    fn mute(&mut self);
    fn unmute(&mut self);
    fn set_echo_reference(&mut self, reference: Option<EchoRef>);
}

pub trait Transcriber: Send + Sync {
    fn transcribe(&self, audio: &AudioCapture, language: &Language) -> Option<String>;
}

pub trait OrderHandler: Send + Sync {
    fn handle(&self, order: &str) -> String;
    fn reset_session(&self);
}

pub trait AudioSpeaker: Send + Sync {
    fn speak(
        &self,
        text:               &str,
        language:           &Language,
        on_playback_start:  Option<Box<dyn FnOnce() + Send>>,
    );
    fn stop(&self);
    fn beep(&self);
    fn play_melody(&self, stop_signal: Arc<AtomicBool>);
    fn get_echo_reference(&self) -> Option<EchoRef>;
}
