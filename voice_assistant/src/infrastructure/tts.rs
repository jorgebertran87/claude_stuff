use std::io::Read;

const GTTS_SUPPORTED: &[&str] = &[
    "af", "ar", "bg", "bn", "bs", "ca", "cs", "cy", "da", "de", "el", "en",
    "eo", "es", "et", "fi", "fr", "gu", "hi", "hr", "hu", "hy", "id", "is",
    "it", "ja", "jw", "km", "kn", "ko", "la", "lv", "mk", "ml", "mr", "my",
    "ne", "nl", "no", "pl", "pt", "ro", "ru", "si", "sk", "sq", "sr", "su",
    "sv", "sw", "ta", "te", "th", "tl", "tr", "uk", "ur", "vi", "zh-CN", "zh-TW",
];

const MAX_TTS_CHARS: usize = 180;

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

pub fn tts_segment(text: &str, lang: &str) -> Result<AudioSegment, String> {
    match fetch_tts(text, lang) {
        Ok(bytes) => Ok(AudioSegment::from_bytes(bytes)),
        Err(_) => fetch_tts(text, "en")
            .map(AudioSegment::from_bytes)
            .map_err(|e| format!("TTS fallback failed: {e}")),
    }
}

pub fn tts_chunks(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    let sentences = split_sentences(text);
    let mut current = String::new();
    for sentence in sentences {
        if current.len() + sentence.len() <= MAX_TTS_CHARS {
            current.push_str(&sentence);
        } else {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
            if sentence.len() > MAX_TTS_CHARS {
                let mut word_buf = String::new();
                for word in sentence.split_whitespace() {
                    if word_buf.len() + word.len() + 1 > MAX_TTS_CHARS {
                        if !word_buf.trim().is_empty() {
                            chunks.push(word_buf.trim().to_string());
                        }
                        word_buf = word.to_string();
                    } else {
                        if !word_buf.is_empty() { word_buf.push(' '); }
                        word_buf.push_str(word);
                    }
                }
                current = word_buf;
            } else {
                current = sentence;
            }
        }
    }
    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    if chunks.is_empty() { chunks.push(text.to_string()); }
    chunks
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        if matches!(c, '.' | '!' | '?') {
            if chars.peek().map(|&n| n == ' ' || n == '\n').unwrap_or(true) {
                result.push(current.clone());
                current.clear();
            }
        } else if c == '\n' {
            result.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() { result.push(current); }
    result
}

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
