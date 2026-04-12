use std::sync::{Arc, atomic::AtomicBool};
use crate::domain::model::{AudioCapture, Language};

pub type EchoRef = (Vec<u8>, u32, u16); // (raw_bytes, sample_rate, sample_width)

pub trait AudioCapturer: Send + Sync {
    fn capture(
        &self,
        timeout_ms:           Option<u64>,
        phrase_time_limit_ms: Option<u64>,
        pause_threshold_ms:   Option<u64>,
    ) -> Option<AudioCapture>;

    fn calibrate(&self, duration_secs: f64);
    fn mute(&self);
    fn unmute(&self);
    fn set_echo_reference(&self, reference: Option<EchoRef>);
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
    /// Disconnect the physical audio output device (e.g. Bluetooth speaker).
    /// Default implementation is a no-op for non-BT speakers.
    fn disconnect(&self) {}
}

/// Port for accessing Google Sheets data and managing OAuth credentials.
pub trait GoogleSheetsGateway: Send + Sync {
    fn auth_url(&self) -> Option<String>;
    fn exchange_code(&self, code: &str) -> Result<(), String>;
    fn fetch_as_text(&self) -> Result<String, String>;
}

/// Port for synthesizing text to MP3 audio bytes.
pub trait TextSynthesizer: Send + Sync {
    fn synthesize_text(&self, text: &str) -> Vec<u8>;
    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8>;
}

/// Port for analyzing images using an AI model.
pub trait ImageAnalyzer: Send + Sync {
    fn analyze(&self, bytes: &[u8], caption: &str, model: &str) -> String;
}
