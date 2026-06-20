//! Piper-based audio speaker adapter.
//! Synthesises text with Piper TTS and plays via rodio (pure Rust).

use std::io::Cursor;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

use shaku::Component;

use crate::domain::model::Language;
use crate::domain::ports::{AudioSpeaker, EchoRef};
use crate::infrastructure::speaker_utils::{
    alexa_spotify_title, build_alexa_command, disconnect_bt_speaker, strip_markdown,
};
use crate::infrastructure::piper_engine::tts_segment;

#[derive(Component)]
#[shaku(interface = AudioSpeaker)]
pub struct PiperSpeaker {
    #[shaku(default)]
    stop_signal: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

impl PiperSpeaker {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { stop_signal: Arc::new(Mutex::new(None)) })
    }

    fn play_bytes(&self, bytes: &[u8], on_start: Option<Box<dyn FnOnce() + Send>>) {
        if bytes.is_empty() {
            return;
        }
        if let Some(cb) = on_start {
            cb();
        }

        let stop = Arc::new(AtomicBool::new(false));
        *self.stop_signal.lock().unwrap() = Some(Arc::clone(&stop));
        let owned = bytes.to_vec();

        thread::spawn(move || {
            match rodio::Decoder::new(Cursor::new(owned)) {
                Ok(source) => {
                    if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                        if let Ok(sink) = rodio::Sink::try_new(&handle) {
                            sink.append(source);
                            while !sink.empty() && !stop.load(Ordering::SeqCst) {
                                thread::sleep(Duration::from_millis(50));
                            }
                            if stop.load(Ordering::SeqCst) {
                                sink.stop();
                            }
                        }
                    }
                }
                Err(e) => eprintln!("[playback: decode error: {e}]"),
            }
        });
    }
}

impl AudioSpeaker for PiperSpeaker {
    fn speak(&self, text: &str, language: &Language, on_playback_start: Option<Box<dyn FnOnce() + Send>>) {
        let (unified, lang) = match alexa_spotify_title(text) {
            Some((title, ref tl)) => (build_alexa_command(&title, tl), tl.clone()),
            None => (strip_markdown(text), language.lang_prefix().to_string()),
        };

        // Piper handles full text without chunking (no 200-char limit like Google TTS).
        match tts_segment(&unified, &lang) {
            Ok(seg) if !seg.is_empty() => self.play_bytes(seg.raw_data(), on_playback_start),
            Err(e) => eprintln!("TTS error: {e}"),
            _ => {}
        }
    }

    fn stop(&self) {
        if let Some(stop) = self.stop_signal.lock().unwrap().take() {
            stop.store(true, Ordering::SeqCst);
        }
    }

    fn beep(&self) {
        if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(sink) = rodio::Sink::try_new(&handle) {
                use rodio::Source;
                let source = rodio::source::SineWave::new(440.0)
                    .take_duration(Duration::from_millis(200))
                    .amplify(0.5);
                sink.append(source);
                sink.sleep_until_end();
            }
        }
    }

    fn play_melody(&self, stop_signal: Arc<AtomicBool>) {
        while !stop_signal.load(Ordering::SeqCst) {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                if let Ok(sink) = rodio::Sink::try_new(&handle) {
                    use rodio::Source;
                    let source = rodio::source::SineWave::new(520.0)
                        .take_duration(Duration::from_millis(400))
                        .amplify(0.3);
                    sink.append(source);
                    // Check stop during play
                    while !sink.empty() && !stop_signal.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(50));
                    }
                    if stop_signal.load(Ordering::SeqCst) {
                        sink.stop();
                        break;
                    }
                }
            }
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
