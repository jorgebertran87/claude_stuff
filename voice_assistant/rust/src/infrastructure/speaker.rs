//! TTS pipeline: markdown stripping, language detection, audio generation, playback.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use regex::Regex;
use whichlang::detect_language;

use crate::domain::model::Language;
use crate::domain::ports::{AudioSpeaker, EchoRef};

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
        Lang::Rus => "ru",
        Lang::Swe => "sv",
        Lang::Tur => "tr",
        Lang::Kor => "ko",
        Lang::Ara => "ar",
        Lang::Hin => "hi",
        Lang::Cmn => "zh-CN",
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
    let re = Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap();
    let m  = re.captures(text)?;
    // group 1 = double-quoted title, group 2 = single-quoted title
    let title = m.get(1).or_else(|| m.get(2))?.as_str().to_string();
    let before = text[..m.get(0)?.start()].to_string();
    let after  = text[m.get(0)?.end()..].to_string();
    let mut title_lang = detect_lang(&title);
    if title_lang != "es" {
        title_lang = "en".to_string(); // Force English for non-Spanish titles to ensure gTTS support
    }
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

// ── TTS chunk splitter ────────────────────────────────────────────────────────

const MAX_TTS_CHARS: usize = 180;

/// Split text into chunks ≤ MAX_TTS_CHARS, breaking at sentence boundaries first,
/// then at word boundaries. Mirrors how gTTS splits long strings internally.
fn tts_chunks(text: &str) -> Vec<String> {
    let mut chunks: Vec<String> = Vec::new();
    // Split at sentence-ending punctuation, keeping the delimiter
    let sentences = split_sentences(text);
    let mut current = String::new();
    for sentence in sentences {
        if current.len() + sentence.len() <= MAX_TTS_CHARS {
            current.push_str(&sentence);
        } else {
            if !current.trim().is_empty() {
                chunks.push(current.trim().to_string());
            }
            // If the sentence itself exceeds the limit, split at word boundaries
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

/// Synthesize arbitrary text to MP3 bytes at 1.2× speed.
/// Strips markdown, detects language, chunks, and concatenates segments.
/// Returns an empty `Vec` if synthesis fails entirely.
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

/// Generate TTS audio bytes for an Alexa+Spotify order, using the same multilingual
/// segment logic as `GTTSSpeaker::speak()`, with the same 1.2× speed applied.
/// Returns an empty `Vec` if synthesis fails entirely.
pub fn synthesize_alexa_spotify(text: &str) -> Vec<u8> {
    let segments = alexa_spotify_parts(text)
        .unwrap_or_else(|| vec![(strip_markdown(text), "es".to_string())]);

    // Collect segments paired with their per-segment speed:
    // Spanish parts → 1.2×, English title → 1.0×.
    let mut segment_bytes: Vec<(Vec<u8>, f32)> = Vec::new();
    for (chunk, chunk_lang) in &segments {
        if chunk.trim().is_empty() { continue; }
        let speed = if chunk_lang == "en" { 1.0_f32 } else { 1.2_f32 };
        for piece in tts_chunks(chunk) {
            match tts_segment(&piece, chunk_lang) {
                Ok(seg) => segment_bytes.push((seg.raw_data().to_vec(), speed)),
                Err(e)  => eprintln!("[tts error: {e}]"),
            }
        }
    }

    match segment_bytes.len() {
        0 => Vec::new(),
        1 => {
            let (bytes, speed) = segment_bytes.remove(0);
            apply_atempo(bytes, speed)
        }
        // Use ffmpeg per-segment atempo + concat filter so PCM is joined before
        // re-encoding — avoids pops/gaps from raw MP3 byte concatenation.
        _ => ffmpeg_concat_and_speed(segment_bytes),
    }
}

/// Decode MP3 segments to PCM, apply per-segment atempo, concatenate, re-encode.
/// Each entry is `(mp3_bytes, speed)`. Falls back to raw-concat + 1.2× if ffmpeg fails.
fn ffmpeg_concat_and_speed(segments: Vec<(Vec<u8>, f32)>) -> Vec<u8> {
    let output_path = "/tmp/tts_concat_out.mp3";
    let input_paths: Vec<String> = (0..segments.len())
        .map(|i| format!("/tmp/tts_seg_{i}.mp3"))
        .collect();

    // Pre-collect fallback bytes before consuming segments for file writes.
    let fallback: Vec<u8> = segments.iter().flat_map(|(b, _)| b.iter().copied()).collect();

    for (path, (bytes, _)) in input_paths.iter().zip(segments.iter()) {
        if std::fs::write(path, bytes).is_err() {
            return apply_atempo(fallback, 1.2);
        }
    }

    let n = input_paths.len();
    let mut cmd_args: Vec<String> = vec!["-y".into(), "-loglevel".into(), "quiet".into()];
    for path in &input_paths {
        cmd_args.extend(["-i".into(), path.clone()]);
    }
    // For "Alexa, pon [title] en Spotify" the comma pause is generated naturally
    // by gTTS inside the first segment; no artificial gaps are added between
    // segments — the gTTS trailing silence on each clip is already enough.
    let last = n - 1;
    let mut filter_parts: Vec<String> = segments.iter().enumerate()
        .map(|(i, (_, speed))| format!("[{i}:a]atempo={speed}[a{i}]"))
        .collect();
    let tagged: String = (0..n).map(|i| format!("[a{i}]")).collect();
    filter_parts.push(format!("{tagged}concat=n={n}:v=0:a=1[out]"));
    let filter = filter_parts.join(";");

    cmd_args.extend([
        "-filter_complex".into(), filter,
        "-map".into(), "[out]".into(),
        "-f".into(), "mp3".into(),
        output_path.into(),
    ]);

    let ok = Command::new("ffmpeg")
        .args(&cmd_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    for path in &input_paths {
        let _ = std::fs::remove_file(path);
    }

    if ok {
        if let Ok(out) = std::fs::read(output_path) {
            return out;
        }
    }

    apply_atempo(fallback, 1.2)
}

/// Apply an ffmpeg `atempo` filter to MP3 bytes, returning the processed bytes.
/// Falls back to the original bytes if ffmpeg fails.
fn apply_atempo(bytes: Vec<u8>, speed: f32) -> Vec<u8> {
    let input_path  = "/tmp/tts_atempo_in.mp3";
    let output_path = "/tmp/tts_atempo_out.mp3";
    if std::fs::write(input_path, &bytes).is_err() {
        return bytes;
    }
    let ok = Command::new("ffmpeg")
        .args([
            "-y", "-loglevel", "quiet",
            "-i", input_path,
            "-af", &format!("atempo={speed}"),
            "-f", "mp3", output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        std::fs::read(output_path).unwrap_or(bytes)
    } else {
        bytes
    }
}

// ── GTTSSpeaker ───────────────────────────────────────────────────────────────

pub struct GTTSSpeaker {
    current_pid: Arc<Mutex<Option<u32>>>,
}

impl GTTSSpeaker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { current_pid: Arc::new(Mutex::new(None)) })
    }

    fn play_bytes(&self, bytes: &[u8], on_start: Option<Box<dyn FnOnce() + Send>>) {
        let tmp = "/tmp/voice_response.mp3";
        let _ = std::fs::write(tmp, bytes);

        if let Some(cb) = on_start {
            cb();
        }

        if let Ok(mut child) = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                   "-af", "atempo=1.2",
                   tmp])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            *self.current_pid.lock().unwrap() = Some(child.id());
            let _ = child.wait();
            *self.current_pid.lock().unwrap() = None;
        }
    }
}

impl AudioSpeaker for GTTSSpeaker {
    fn speak(&self, text: &str, language: &Language, on_playback_start: Option<Box<dyn FnOnce() + Send>>) {
        let lang = language.lang_prefix();
        let segments = alexa_spotify_parts(text)
            .unwrap_or_else(|| vec![(strip_markdown(text), lang.to_string())]);

        let mut all_bytes: Vec<u8> = Vec::new();
        for (chunk, chunk_lang) in &segments {
            if chunk.trim().is_empty() { continue; }
            for piece in tts_chunks(chunk) {
                match tts_segment(&piece, chunk_lang) {
                    Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
                    Err(e)  => eprintln!("TTS error: {e}"),
                }
            }
        }

        if !all_bytes.is_empty() {
            self.play_bytes(&all_bytes, on_playback_start);
        }
    }

    fn stop(&self) {
        if let Some(pid) = *self.current_pid.lock().unwrap() {
            let _ = Command::new("kill")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }

    fn beep(&self) {
        let _ = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                   "-f", "lavfi", "-i", "sine=frequency=440:duration=0.2"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    fn play_melody(&self, stop_signal: Arc<AtomicBool>) {
        while !stop_signal.load(Ordering::SeqCst) {
            let _ = Command::new("ffplay")
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet",
                       "-f", "lavfi", "-i", "sine=frequency=520:duration=0.4"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            thread::sleep(Duration::from_millis(200));
        }
    }

    fn get_echo_reference(&self) -> Option<EchoRef> {
        None
    }
}
