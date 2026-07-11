//! Google Web Speech API transcriber (same endpoint used by speech_recognition).

use std::process::{Command, Stdio};

use shaku::Component;

use crate::domain::model::{AudioCapture, Language};
use crate::domain::ports::Transcriber;

/// Built-in default API key used when `GOOGLE_SPEECH_API_KEY` is not set.
/// This key is shared for development convenience; set the env var in production.
const DEFAULT_SPEECH_API_KEY: &str = "AIzaSyBOti4mM-6x9WDnZIjIeyEU21OpBXqWBgw";

fn speech_api_key() -> String {
    std::env::var("GOOGLE_SPEECH_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SPEECH_API_KEY.to_string())
}

#[derive(Component)]
#[shaku(interface = Transcriber)]
pub struct GoogleTranscriber;

impl GoogleTranscriber {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self)
    }
}

impl Transcriber for GoogleTranscriber {
    fn transcribe(&self, audio: &AudioCapture, language: &Language) -> Option<String> {
        let wav_path  = "/tmp/transcribe_input.wav";
        let flac_path = "/tmp/transcribe_input.flac";

        std::fs::write(wav_path, &audio.raw).ok()?;

        // Convert WAV → FLAC (Google Speech API requires FLAC)
        let ok = Command::new("ffmpeg")
            .args(["-y", "-i", wav_path, flac_path])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !ok { return None; }

        let flac_bytes = std::fs::read(flac_path).ok()?;
        if flac_bytes.is_empty() { return None; }

        let lang = &language.code;
        let key = speech_api_key();
        let url = format!(
            "https://www.google.com/speech-api/v2/recognize\
             ?output=json&lang={lang}&key={key}"
        );

        let resp = ureq::post(&url)
            .set("Content-Type", &format!("audio/x-flac; rate={}", audio.sample_rate))
            .send_bytes(&flac_bytes)
            .ok()?;

        let body = resp.into_string().ok()?;
        parse_transcript(&body)
    }
}
fn parse_transcript(body: &str) -> Option<String> {
    // Google returns one JSON object per line; the last non-empty result wins.
    // Format: {"result":[{"alternative":[{"transcript":"text",...}],"final":true}]}
    for line in body.lines().rev() {
        if let Some(start) = line.find("\"transcript\":\"") {
            let rest = &line[start + 14..];
            if let Some(end) = rest.find('"') {
                let text = rest[..end].trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    None
}

// ── WhisperTranscriber ────────────────────────────────────────────────────────

/// Default model path when `WHISPER_MODEL` is not set.
const DEFAULT_WHISPER_MODEL: &str = "/app/models/ggml-tiny.bin";

fn whisper_model_path() -> String {
    std::env::var("WHISPER_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_WHISPER_MODEL.to_string())
}

#[derive(Component)]
#[shaku(interface = Transcriber)]
pub struct WhisperTranscriber;

impl WhisperTranscriber {
    pub fn new() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self)
    }
}

impl Transcriber for WhisperTranscriber {
    fn transcribe(&self, audio: &AudioCapture, language: &Language) -> Option<String> {
        let wav_path = "/tmp/whisper_input.wav";

        std::fs::write(wav_path, &audio.raw).ok()?;

        let model = whisper_model_path();
        let lang = language.lang_prefix();

        let output = Command::new("whisper-cli")
            .args([
                "-m", &model,
                "-l", lang,
                "-f", wav_path,
                "--no-timestamps",
                "--print-progress", "false",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.is_empty() { None } else { Some(text) }
    }
}
