//! TTS pipeline: markdown stripping, language detection, audio generation.

use regex::Regex;
use whichlang::detect_language;

// Supported gTTS language codes (subset; extended as needed).
const GTTS_SUPPORTED: &[&str] = &[
    "af", "ar", "bg", "bn", "bs", "ca", "cs", "cy", "da", "de", "el", "en",
    "eo", "es", "et", "fi", "fr", "gu", "hi", "hr", "hu", "hy", "id", "is",
    "it", "ja", "jw", "km", "kn", "ko", "la", "lv", "mk", "ml", "mr", "my",
    "ne", "nl", "no", "pl", "pt", "ro", "ru", "si", "sk", "sq", "sr", "su",
    "sv", "sw", "ta", "te", "th", "tl", "tr", "uk", "ur", "vi", "zh-CN", "zh-TW",
];

// ── AudioSegment ─────────────────────────────────────────────────────────────

/// Lightweight audio container — stores raw bytes (MP3 from Google TTS).
/// `len()` returns byte count, used as a proxy for duration in tests.
#[derive(Debug, Clone)]
pub struct AudioSegment {
    bytes: Vec<u8>,
}

impl AudioSegment {
    pub fn from_bytes(bytes: Vec<u8>) -> Self { Self { bytes } }
    pub fn len(&self) -> usize              { self.bytes.len() }
    pub fn is_empty(&self) -> bool          { self.bytes.is_empty() }
    pub fn raw_data(&self) -> &[u8]         { &self.bytes }

    pub fn concat(&self, other: &Self) -> Self {
        let mut bytes = self.bytes.clone();
        bytes.extend_from_slice(&other.bytes);
        Self { bytes }
    }
}

// ── public helpers ────────────────────────────────────────────────────────────

/// Strip common Markdown constructs so TTS reads clean prose.
pub fn strip_markdown(text: &str) -> String {
    let s = Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap().replace_all(text, "$1");
    let s = Regex::new(r"https?://\S+").unwrap().replace_all(&s, "");
    let s = Regex::new(r"\*+([^*]*)\*+").unwrap().replace_all(&s, "$1");
    let s = Regex::new(r"(?m)^#+\s+").unwrap().replace_all(&s, "");
    let s = Regex::new(r"(?m)^[-*]\s+").unwrap().replace_all(&s, "");
    let s = Regex::new(r"`[^`]*`").unwrap().replace_all(&s, "");
    s.trim().to_string()
}

/// Generate a TTS `AudioSegment` for `text` in `lang`.
/// Falls back to English if the language code is not supported by gTTS.
pub fn tts_segment(text: &str, lang: &str) -> Result<AudioSegment, String> {
    match fetch_tts(text, lang) {
        Ok(bytes) => Ok(AudioSegment::from_bytes(bytes)),
        Err(_) => {
            // Fall back to English for unsupported / rejected language codes
            fetch_tts(text, "en")
                .map(AudioSegment::from_bytes)
                .map_err(|e| format!("TTS fallback failed: {e}"))
        }
    }
}

/// Detect language of `text` and return a BCP-47-ish code (e.g. `"en"`, `"es"`).
/// Returns `"en"` when detection is uncertain.
pub fn detect_lang(text: &str) -> String {
    use whichlang::Lang;
    match detect_language(text) {
        Lang::Eng => "en",
        Lang::Spa => "es",
        Lang::Fra => "fr",
        Lang::Deu => "de",
        Lang::Por => "pt",
        Lang::Ita => "it",
        Lang::Nld => "nl",
        Lang::Pol => "pl",
        Lang::Rus => "ru",
        Lang::Swe => "sv",
        Lang::Tur => "tr",
        Lang::Kor => "ko",
        Lang::Ara => "ar",
        Lang::Hin => "hi",
        Lang::Zho => "zh-CN",
        Lang::Jpn => "ja",
        _ => "en",
    }
    .to_string()
}

/// If `text` contains "alexa" and "spotify" and a quoted song title, return
/// `[(chunk, lang_code), …]` with Spanish for the framing and detected language
/// for the title; otherwise `None`.
pub fn alexa_spotify_parts(text: &str) -> Option<Vec<(String, String)>> {
    let lower = text.to_lowercase();
    if !lower.contains("alexa") || !lower.contains("spotify") {
        return None;
    }
    let re = Regex::new(r#"(["\'])(.+?)\1"#).unwrap();
    let m  = re.captures(text)?;
    let title  = m.get(2)?.as_str().to_string();
    let before = text[..m.get(0)?.start()].to_string();
    let after  = text[m.get(0)?.end()..].to_string();
    let title_lang = detect_lang(&title);
    Some(vec![
        (before, "es".into()),
        (title,  title_lang),
        (after,  "es".into()),
    ])
}

// ── private ───────────────────────────────────────────────────────────────────

fn fetch_tts(text: &str, lang: &str) -> Result<Vec<u8>, String> {
    let lang_key = lang.split('-').next().unwrap_or(lang).to_lowercase();
    let lang_check = if lang.contains('-') { lang } else { lang_key.as_str() };
    if !GTTS_SUPPORTED.iter().any(|&s| s.eq_ignore_ascii_case(lang_check)) {
        return Err(format!("Language not supported: {lang}"));
    }
    let url = format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob",
        urlencode(text),
        lang,
    );
    let response = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0")
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read error: {e}"))?;
    if bytes.is_empty() {
        return Err("Empty TTS response".into());
    }
    Ok(bytes)
}

fn urlencode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
                vec![c]
            } else if c == ' ' {
                vec!['+']
            } else {
                c.to_string()
                    .bytes()
                    .flat_map(|b| format!("%{:02X}", b).chars().collect::<Vec<_>>())
                    .collect()
            }
        })
        .collect()
}

// Need std::io::Read for read_to_end
use std::io::Read;
