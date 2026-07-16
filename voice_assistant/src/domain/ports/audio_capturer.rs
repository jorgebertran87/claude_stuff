use crate::domain::model::AudioCapture;

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

    /// Apply echo cancellation to a raw audio buffer using the stored reference.
    fn apply_echo_cancellation(
        &self,
        raw:          &[u8],
        sample_rate:  u32,
        sample_width: u16,
    ) -> Vec<u8>;
}
