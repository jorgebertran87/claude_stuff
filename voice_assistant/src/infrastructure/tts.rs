use std::io::{Read, Write};
use std::process::{Command, Stdio};

const MAX_TTS_CHARS: usize = 200;

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
    match piper_synthesize(text, lang) {
        Ok(bytes) => Ok(AudioSegment::from_bytes(bytes)),
        Err(_) => piper_synthesize(text, "en")
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

/// Model directory — set via `PIPER_MODEL_DIR` env var, defaults to `/app/piper_models`.
fn piper_model_dir() -> String {
    std::env::var("PIPER_MODEL_DIR")
        .unwrap_or_else(|_| "/app/piper_models".to_string())
}

/// Map a language code prefix to the best available Piper voice model file name.
fn piper_voice_for_lang(lang_prefix: &str) -> String {
    match lang_prefix {
        "es" => "es_ES-sharvard-medium.onnx",
        _    => "en_US-lessac-medium.onnx",
    }
    .to_string()
}

/// Synthesise text to MP3 bytes via Piper TTS (offline).
fn piper_synthesize(text: &str, lang: &str) -> Result<Vec<u8>, String> {
    let lang_prefix = lang.split('-').next().unwrap_or(lang).to_lowercase();
    let voice = piper_voice_for_lang(&lang_prefix);
    let model_path = format!("{}/{}", piper_model_dir(), voice);

    if !std::path::Path::new(&model_path).exists() {
        return Err(format!("Piper voice model not found: {model_path}"));
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let wav_path = format!("/tmp/piper_out_{nanos}.wav");
    let mp3_path = format!("/tmp/piper_out_{nanos}.mp3");

    // Piper: text → WAV. Slow down Spanish slightly for clarity.
    let length_scale = if lang_prefix == "es" { "1.15" } else { "1.0" };
    let mut child = Command::new("piper")
        .args(["--model", &model_path, "--length_scale", length_scale, "--output_file", &wav_path])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Piper: failed to spawn: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).map_err(|e| format!("Piper: stdin write: {e}"))?;
    }

    let status = child.wait().map_err(|e| format!("Piper: wait: {e}"))?;
    if !status.success() {
        let _ = std::fs::remove_file(&wav_path);
        return Err("Piper exited with error".into());
    }

    // ffmpeg: WAV → MP3
    let ffmpeg_ok = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "quiet", "-i", &wav_path, "-f", "mp3", &mp3_path])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = std::fs::remove_file(&wav_path);

    if !ffmpeg_ok {
        let _ = std::fs::remove_file(&mp3_path);
        return Err("Piper: WAV→MP3 conversion failed".into());
    }

    let bytes = std::fs::read(&mp3_path).map_err(|e| format!("Piper: read mp3: {e}"))?;
    let _ = std::fs::remove_file(&mp3_path);

    if bytes.is_empty() {
        Err("Piper: empty output".into())
    } else {
        Ok(bytes)
    }
}

// ── Google Translate TTS (pre-Piper, online) ──────────────────────────────────

/// Supported languages for Google Translate TTS.
const GTTS_SUPPORTED: &[&str] = &[
    "af", "ar", "bg", "bn", "bs", "ca", "cs", "cy", "da", "de", "el", "en",
    "eo", "es", "et", "fi", "fr", "gu", "hi", "hr", "hu", "hy", "id", "is",
    "it", "ja", "jw", "km", "kn", "ko", "la", "lv", "mk", "ml", "mr", "my",
    "ne", "nl", "no", "pl", "pt", "ro", "ru", "si", "sk", "sq", "sr", "su",
    "sv", "sw", "ta", "te", "th", "tl", "tr", "uk", "ur", "vi", "zh-CN", "zh-TW",
];

/// Fetch TTS audio from Google Translate (online, pre-Piper).
pub fn fetch_tts(text: &str, lang: &str) -> Result<Vec<u8>, String> {
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
