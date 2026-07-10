/// Port for synthesizing text to MP3 audio bytes.
pub trait TextSynthesizer: Send + Sync {
    fn synthesize_text(&self, text: &str) -> Vec<u8>;
    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8>;
}
