//! Shared utilities for speaker and text-to-speech adapters:
//! markdown stripping, language detection, Alexa/Spotify command building,
//! Bluetooth speaker disconnect, and atempo speed adjustment.

use std::process::{Command, Stdio};
use std::sync::LazyLock;

use regex::Regex;
use whichlang::detect_language;

static RE_LINK:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap());
static RE_URL:     LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://\S+").unwrap());
static RE_BOLD:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*+([^*]*)\*+").unwrap());
static RE_HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^#+\s+").unwrap());
static RE_BULLET:  LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[-*]\s+").unwrap());
static RE_CODE:    LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`[^`]*`").unwrap());
static RE_QUOTE:   LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap());

pub fn strip_markdown(text: &str) -> String {
    let s = RE_LINK.replace_all(text, "$1");
    let s = RE_URL.replace_all(&s, "");
    let s = RE_BOLD.replace_all(&s, "$1");
    let s = RE_HEADING.replace_all(&s, "");
    let s = RE_BULLET.replace_all(&s, "");
    let s = RE_CODE.replace_all(&s, "");
    s.trim().to_string()
}

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

pub fn alexa_spotify_title(text: &str) -> Option<(String, String)> {
    let lower = text.to_lowercase();
    if !lower.contains("alexa") || !lower.contains("spotify") {
        return None;
    }
    let m = RE_QUOTE.captures(text)?;
    let title = m.get(1).or_else(|| m.get(2))?.as_str().to_string();
    let lang = if detect_lang(&title) == "es" { "es".into() } else { "en".into() };
    Some((title, lang))
}

pub fn build_alexa_command(title: &str, lang: &str) -> String {
    match lang {
        "es" => format!("Alexa, pon {} en Spotify", title),
        _    => format!("Alexa, play {} on Spotify", title),
    }
}

pub fn disconnect_bt_speaker() {
    let mac = match std::env::var("BT_SPEAKER_MAC") {
        Ok(m) if !m.is_empty() => m,
        _ => {
            eprintln!("[bt: BT_SPEAKER_MAC not set, skipping disconnect]");
            return;
        }
    };
    eprintln!("[bt: disconnecting {mac} after inactivity]");
    let _ = Command::new("bluetoothctl")
        .args(["disconnect", &mac])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

pub fn apply_atempo(bytes: Vec<u8>, speed: f32) -> Vec<u8> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let input_path  = format!("/tmp/tts_atempo_in_{nanos}.mp3");
    let output_path = format!("/tmp/tts_atempo_out_{nanos}.mp3");
    if std::fs::write(&input_path, &bytes).is_err() {
        return bytes;
    }
    let ok = Command::new("ffmpeg")
        .args([
            "-y", "-loglevel", "quiet",
            "-i", &input_path,
            "-af", &format!("atempo={speed}"),
            "-f", "mp3", &output_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = std::fs::remove_file(&input_path);
    if ok {
        let result = std::fs::read(&output_path).unwrap_or(bytes);
        let _ = std::fs::remove_file(&output_path);
        result
    } else {
        bytes
    }
}
