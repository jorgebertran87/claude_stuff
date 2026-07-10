use std::sync::{Arc, atomic::AtomicBool};
use crate::domain::model::Language;
use crate::domain::ports::audio_capturer::EchoRef;

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
