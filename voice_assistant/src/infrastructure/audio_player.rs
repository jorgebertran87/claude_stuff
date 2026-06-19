//! Audio output adapter: plays synthesized MP3 bytes via rodio (pure Rust).

use std::io::Cursor;

use shaku::Component;

use crate::domain::ports::AudioPlayer;
use crate::infrastructure::speaker::disconnect_bt_speaker;

/// Write `bytes` to a temporary MP3 buffer and play it through rodio (blocking).
pub fn play_audio_bytes(bytes: &[u8]) {
    if bytes.is_empty() {
        return;
    }
    match rodio::Decoder::new(Cursor::new(bytes.to_vec())) {
        Ok(source) => {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                if let Ok(sink) = rodio::Sink::try_new(&handle) {
                    sink.append(source);
                    sink.sleep_until_end();
                }
            }
        }
        Err(e) => eprintln!("[play_audio_bytes: decode error: {e}]"),
    }
}

// ── RodioAudioPlayer ─────────────────────────────────────────────────────────

#[derive(Component)]
#[shaku(interface = AudioPlayer)]
pub struct RodioAudioPlayer;

impl AudioPlayer for RodioAudioPlayer {
    fn play(&self, bytes: &[u8]) {
        play_audio_bytes(bytes);
    }

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}

// ── FfplayAudioPlayer (ffplay-based, pre-rodio) ──────────────────────────────

/// Write `bytes` to a temp MP3 file and play it through ffplay (blocking).
pub fn play_audio_bytes_ffplay(bytes: &[u8]) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = format!("/tmp/tts_telegram_play_{nanos}.mp3");
    if let Err(e) = std::fs::write(&tmp, bytes) {
        eprintln!("[play_audio_bytes: failed to write tmp file: {e}]");
        return;
    }
    match std::process::Command::new("ffplay")
        .args(["-nodisp", "-autoexit", "-loglevel", "warning", &tmp])
        .stdout(std::process::Stdio::null())
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("[play_audio_bytes: ffplay exited with {status}]"),
        Err(e) => eprintln!("[play_audio_bytes: failed to spawn ffplay: {e}]"),
    }
    let _ = std::fs::remove_file(&tmp);
}

#[derive(Component)]
#[shaku(interface = AudioPlayer)]
pub struct FfplayAudioPlayer;

impl AudioPlayer for FfplayAudioPlayer {
    fn play(&self, bytes: &[u8]) {
        play_audio_bytes_ffplay(bytes);
    }

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}
