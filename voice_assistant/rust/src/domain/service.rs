use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;

use crate::domain::model::{Language, WakeWord};
use crate::domain::ports::{AudioCapturer, AudioSpeaker, OrderHandler, Transcriber};

const ORDER_TIMEOUT_MS:       u64   = 10_000;
const ORDER_RETRIES:          usize = 2;
const ORDER_PAUSE_THRESHOLD:  u64   = 2_000;

pub struct VoiceListenerService {
    capturer:      Box<dyn AudioCapturer>,
    transcriber:   Arc<dyn Transcriber>,
    order_handler: Arc<dyn OrderHandler>,
    speaker:       Arc<dyn AudioSpeaker>,
    wake_word:     WakeWord,
    language:      Language,
}

impl VoiceListenerService {
    pub fn new(
        capturer:      Box<dyn AudioCapturer>,
        transcriber:   Arc<dyn Transcriber>,
        order_handler: Arc<dyn OrderHandler>,
        speaker:       Arc<dyn AudioSpeaker>,
        wake_word:     WakeWord,
        language:      Language,
    ) -> Self {
        Self { capturer, transcriber, order_handler, speaker, wake_word, language }
    }

    // ── public methods ────────────────────────────────────────────────────────

    pub fn wait_for_wake_word(&mut self) -> Option<String> {
        eprintln!("[listening for wake word \"{}\"]", self.wake_word.value);
        loop {
            let audio = self.capturer.capture(None, Some(8_000), None);
            let audio = match audio { Some(a) => a, None => continue };
            let text  = self.transcriber.transcribe(&audio, &self.language);
            if let Some(ref t) = text {
                eprintln!("[heard: {t:?}]");
                if self.wake_word.matches(t) {
                    return self.wake_word.extract_order(t);
                }
            }
        }
    }

    pub fn listen_for_order(&mut self) -> Option<String> {
        eprintln!("[listening for order]");
        for attempt in 0..ORDER_RETRIES {
            self.speaker.beep();
            let audio = self.capturer.capture(
                Some(ORDER_TIMEOUT_MS),
                None,
                Some(ORDER_PAUSE_THRESHOLD),
            );
            let audio = match audio {
                Some(a) => a,
                None => {
                    if attempt < ORDER_RETRIES - 1 {
                        eprintln!("I didn't catch that.");
                    }
                    continue;
                }
            };
            if let Some(t) = self.transcriber.transcribe(&audio, &self.language) {
                return Some(t);
            }
        }
        None
    }

    pub fn handle_with_melody(
        &mut self,
        order: &str,
    ) -> (String, Arc<AtomicBool>, thread::JoinHandle<()>) {
        let stop_signal   = Arc::new(AtomicBool::new(false));
        let stop_clone    = Arc::clone(&stop_signal);
        let speaker_clone = Arc::clone(&self.speaker);

        let melody_thread = thread::spawn(move || {
            speaker_clone.play_melody(stop_clone);
        });

        let response = self.order_handler.handle(order);
        (response, stop_signal, melody_thread)
    }

    pub fn run(&mut self) {
        println!("Voice Order Listener");
        println!("====================");
        println!("Press Ctrl+C to quit.\n");

        let mut waiting_for_answer = false;
        loop {
            let order = if waiting_for_answer {
                self.listen_for_order()
            } else {
                let inline = self.wait_for_wake_word();
                inline.or_else(|| self.listen_for_order())
            };

            if let Some(ref order_text) = order {
                println!("Order received: {order_text:?}");
                let (response, stop_melody, melody_thread) =
                    self.handle_with_melody(order_text);
                println!("Claudito: {response}");
                let interrupted =
                    self.speak_interruptible(&response, stop_melody, melody_thread);
                if interrupted {
                    waiting_for_answer = true;
                } else {
                    waiting_for_answer = response.trim_end().ends_with('?');
                }
            } else {
                waiting_for_answer = false;
            }
        }
    }

    pub fn speak_interruptible(
        &mut self,
        response:      &str,
        stop_melody:   Arc<AtomicBool>,
        melody_thread: thread::JoinHandle<()>,
    ) -> bool {
        let speaker_clone  = Arc::clone(&self.speaker);
        let response_owned = response.to_string();
        let lang_clone     = self.language.clone();
        let stop_clone     = Arc::clone(&stop_melody);

        let speak_handle = thread::spawn(move || {
            speaker_clone.speak(
                &response_owned,
                &lang_clone,
                Some(Box::new(move || stop_clone.store(true, Ordering::SeqCst))),
            );
        });

        // Wait for melody to stop (it stops when on_playback_start fires)
        let _ = melody_thread.join();

        self.capturer.set_echo_reference(self.speaker.get_echo_reference());

        let mut interrupted = false;
        while !speak_handle.is_finished() {
            let audio = self.capturer.capture(Some(1_000), Some(2_000), None);
            if let Some(ref a) = audio {
                if let Some(ref t) = self.transcriber.transcribe(a, &self.language) {
                    if self.wake_word.matches(t) {
                        self.speaker.stop();
                        interrupted = true;
                        break;
                    }
                }
            }
        }

        self.capturer.set_echo_reference(None);
        let _ = speak_handle.join();
        interrupted
    }
}
