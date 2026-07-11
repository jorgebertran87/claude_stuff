//! Whisper.cpp local transcriber.

use std::process::{Command, Stdio};

use shaku::Component;

use crate::domain::model::{AudioCapture, Language};
use crate::domain::ports::Transcriber;

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
