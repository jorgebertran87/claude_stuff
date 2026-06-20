//! Piper-based offline text-to-speech adapter.
//! Uses local Piper TTS models — no network required.

use shaku::Component;

use crate::domain::ports::TextSynthesizer;
use super::speaker_utils::{
    alexa_spotify_title, build_alexa_command, detect_lang, strip_markdown,
};
use super::piper_engine::tts_segment;

/// Piper can handle arbitrary-length text — no chunking needed.
/// Returns raw MP3 bytes directly (no atempo re-encode — speed is
/// controlled via Piper's --length_scale instead).
fn synthesize_text_piper(text: &str) -> Vec<u8> {
    let clean = strip_markdown(text);
    if clean.trim().is_empty() {
        return Vec::new();
    }
    let lang = detect_lang(&clean);
    match tts_segment(&clean, &lang) {
        Ok(seg) if !seg.is_empty() => seg.raw_data().to_vec(),
        Err(e) => {
            eprintln!("[tts error: {e}]");
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn synthesize_alexa_spotify_piper(text: &str) -> Vec<u8> {
    let unified = alexa_spotify_title(text)
        .map(|(title, lang)| build_alexa_command(&title, &lang))
        .unwrap_or_else(|| strip_markdown(text));
    synthesize_text_piper(&unified)
}

#[derive(Component)]
#[shaku(interface = TextSynthesizer)]
pub struct PiperTextSynthesizer;

impl TextSynthesizer for PiperTextSynthesizer {
    fn synthesize_text(&self, text: &str) -> Vec<u8> {
        synthesize_text_piper(text)
    }

    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8> {
        synthesize_alexa_spotify_piper(text)
    }
}
