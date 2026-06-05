//! Audio output adapter: plays synthesized MP3 bytes via ffplay and releases
//! the Bluetooth speaker after inactivity.

use std::process::{Command, Stdio};

use shaku::Component;

use crate::domain::ports::AudioPlayer;
use crate::infrastructure::speaker::disconnect_bt_speaker;

/// Write `bytes` to a temp MP3 file and play it through ffplay (blocking).
pub fn play_audio_bytes(bytes: &[u8]) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp = format!("/tmp/tts_telegram_play_{nanos}.mp3");
    if let Err(e) = std::fs::write(&tmp, bytes) {
        eprintln!("[play_audio_bytes: failed to write tmp file: {e}]");
        return;
    }
    match Command::new("ffplay")
        .args(["-nodisp", "-autoexit", "-loglevel", "warning", &tmp])
        .stdout(Stdio::null())
        .status()
    {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("[play_audio_bytes: ffplay exited with {status}]"),
        Err(e) => eprintln!("[play_audio_bytes: failed to spawn ffplay: {e}]"),
    }
    let _ = std::fs::remove_file(&tmp);
}

// ── FfplayAudioPlayer ─────────────────────────────────────────────────────────

#[derive(Component)]
#[shaku(interface = AudioPlayer)]
pub struct FfplayAudioPlayer;

impl AudioPlayer for FfplayAudioPlayer {
    fn play(&self, bytes: &[u8]) {
        play_audio_bytes(bytes);
    }

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}
