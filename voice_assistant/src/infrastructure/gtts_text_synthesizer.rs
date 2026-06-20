//! Google Translate TTS adapter (online).
//! Pre-Piper fallback: fetches TTS audio from Google Translate.

use shaku::Component;

use crate::domain::ports::TextSynthesizer;
use crate::infrastructure::speaker_utils::{
    alexa_spotify_title, apply_atempo, build_alexa_command, detect_lang, strip_markdown,
};
use crate::infrastructure::tts::{tts_chunks, tts_segment};

pub fn synthesize_text(text: &str) -> Vec<u8> {
    let clean = strip_markdown(text);
    if clean.trim().is_empty() {
        return Vec::new();
    }
    let lang = detect_lang(&clean);
    let mut all_bytes: Vec<u8> = Vec::new();
    for chunk in tts_chunks(&clean) {
        match tts_segment(&chunk, &lang) {
            Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
            Err(e) => eprintln!("[tts error: {e}]"),
        }
    }
    if all_bytes.is_empty() {
        return Vec::new();
    }
    apply_atempo(all_bytes, 1.2)
}

pub fn synthesize_alexa_spotify(text: &str) -> Vec<u8> {
    let unified = alexa_spotify_title(text)
        .map(|(title, lang)| build_alexa_command(&title, &lang))
        .unwrap_or_else(|| strip_markdown(text));
    synthesize_text(&unified)
}

#[derive(Component)]
#[shaku(interface = TextSynthesizer)]
pub struct GttsTextSynthesizer;

impl TextSynthesizer for GttsTextSynthesizer {
    fn synthesize_text(&self, text: &str) -> Vec<u8> {
        synthesize_text(text)
    }

    fn synthesize_alexa_spotify(&self, text: &str) -> Vec<u8> {
        synthesize_alexa_spotify(text)
    }
}
