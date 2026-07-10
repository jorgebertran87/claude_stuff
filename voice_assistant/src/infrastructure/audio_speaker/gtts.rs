//! Google TTS + ffplay speaker adapter (pre-Piper, pre-rodio).
//! Synthesises text via Google Translate TTS and plays with ffplay.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use shaku::Component;

use crate::domain::model::Language;
use crate::domain::ports::{AudioSpeaker, EchoRef};
use crate::infrastructure::tts::speaker_utils::{
    alexa_spotify_title, build_alexa_command, strip_markdown,
};
use crate::infrastructure::shared::speaker::disconnect_bt_speaker;
use crate::infrastructure::tts::engine::tts_segment;
use crate::infrastructure::tts::text_chunking::tts_chunks;

#[derive(Component)]
#[shaku(interface = AudioSpeaker)]
pub struct GTTSSpeaker {
    #[shaku(default)]
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
        let (unified, lang) = match alexa_spotify_title(text) {
            Some((title, ref tl)) => (build_alexa_command(&title, tl), tl.clone()),
            None => (strip_markdown(text), language.lang_prefix().to_string()),
        };

        let mut all_bytes: Vec<u8> = Vec::new();
        for piece in tts_chunks(&unified) {
            match tts_segment(&piece, &lang) {
                Ok(seg) => all_bytes.extend_from_slice(seg.raw_data()),
                Err(e)  => eprintln!("TTS error: {e}"),
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

    fn disconnect(&self) {
        disconnect_bt_speaker();
    }
}
