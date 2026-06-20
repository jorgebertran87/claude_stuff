//! Audio output adapter: plays synthesized MP3 bytes via rodio (pure Rust).

use std::io::Cursor;

use shaku::Component;

use crate::domain::ports::AudioPlayer;
use crate::infrastructure::tts::speaker_utils::disconnect_bt_speaker;

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
