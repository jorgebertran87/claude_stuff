//! Piper TTS engine — offline text-to-speech via Piper + ffmpeg.
//!
//! This is the low-level engine driver. Adapters (PiperTextSynthesizer,
//! PiperSpeaker, GttsSpeaker) call `tts_segment` which synthesises text
//! to MP3 bytes using a local Piper voice model.

use std::io::Write;
use std::process::{Command, Stdio};

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

/// Synthesise text via Piper, falling back to English if the requested language fails.
pub fn tts_segment(text: &str, lang: &str) -> Result<AudioSegment, String> {
    match piper_synthesize(text, lang) {
        Ok(bytes) => Ok(AudioSegment::from_bytes(bytes)),
        Err(_) => piper_synthesize(text, "en")
            .map(AudioSegment::from_bytes)
            .map_err(|e| format!("TTS fallback failed: {e}")),
    }
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
