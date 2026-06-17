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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_api_key_uses_env_var_when_set() {
        std::env::set_var("GOOGLE_SPEECH_API_KEY", "env-key-123");
        assert_eq!(speech_api_key(), "env-key-123");
        std::env::remove_var("GOOGLE_SPEECH_API_KEY");
    }

    #[test]
    fn speech_api_key_falls_back_to_default_when_env_empty() {
        std::env::set_var("GOOGLE_SPEECH_API_KEY", "");
        assert_eq!(speech_api_key(), DEFAULT_SPEECH_API_KEY);
        std::env::remove_var("GOOGLE_SPEECH_API_KEY");
    }

    #[test]
    fn speech_api_key_falls_back_to_default_when_env_absent() {
        std::env::remove_var("GOOGLE_SPEECH_API_KEY");
        assert_eq!(speech_api_key(), DEFAULT_SPEECH_API_KEY);
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
