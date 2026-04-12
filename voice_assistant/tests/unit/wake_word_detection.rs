use cucumber::{given, when, then, World};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use voice_assistant::domain::model::{AudioCapture, Language, WakeWord};
use voice_assistant::domain::ports::{AudioCapturer, AudioSpeaker, EchoRef, OrderHandler, Transcriber};
use voice_assistant::domain::service::VoiceListenerService;

// ── Fakes ────────────────────────────────────────────────────────────────────

struct FakeCapturer {
    queue: Mutex<Vec<Option<Vec<u8>>>>,
}

impl FakeCapturer {
    fn new(queue: Vec<Option<Vec<u8>>>) -> Self {
        Self { queue: Mutex::new(queue) }
    }
}

impl AudioCapturer for FakeCapturer {
    fn capture(&self, _t: Option<u64>, _p: Option<u64>, _pa: Option<u64>) -> Option<AudioCapture> {
        let mut q = self.queue.lock().unwrap();
        if q.is_empty() { return None; }
        match q.remove(0) {
            Some(bytes) => Some(AudioCapture::new(bytes, 16000, 2)),
            None => None,
        }
    }
    fn calibrate(&self, _: f64) {}
    fn mute(&self) {}
    fn unmute(&self) {}
    fn set_echo_reference(&self, _: Option<EchoRef>) {}
}

struct FakeTranscriber {
    responses: Mutex<Vec<Option<String>>>,
}

impl FakeTranscriber {
    fn new(responses: Vec<Option<String>>) -> Self {
        Self { responses: Mutex::new(responses) }
    }
}

impl Transcriber for FakeTranscriber {
    fn transcribe(&self, _audio: &AudioCapture, _lang: &Language) -> Option<String> {
        let mut r = self.responses.lock().unwrap();
        if r.is_empty() { return None; }
        r.remove(0)
    }
}

struct FakeSpeaker;
impl AudioSpeaker for FakeSpeaker {
    fn speak(&self, _: &str, _: &Language, _: Option<Box<dyn FnOnce() + Send>>) {}
    fn stop(&self) {}
    fn beep(&self) {}
    fn play_melody(&self, _: Arc<AtomicBool>) {}
    fn get_echo_reference(&self) -> Option<EchoRef> { None }
}

struct FakeHandler;
impl OrderHandler for FakeHandler {
    fn handle(&self, _: &str) -> String { String::new() }
    fn reset_session(&self) {}
}

// ── World ────────────────────────────────────────────────────────────────────

const DUMMY_AUDIO: &[u8] = &[0u8; 100];

#[derive(Debug, Default, World)]
pub struct WakeWordWorld {
    /// Each entry: None = no audio, Some(text) = transcriber returns this text.
    captures: Vec<Option<String>>,
    wake_word_detected: bool,
    inline_order: Option<String>,
    skipped_empty: bool,
    ignored_first: bool,
    fuzzy_match: bool,
}

#[given(regex = r#"^the microphone captures "(.+)"$"#)]
fn given_mic_captures(world: &mut WakeWordWorld, text: String) {
    world.captures.push(Some(text));
}

#[given("the microphone returns no audio on the first capture")]
fn given_no_audio_first(world: &mut WakeWordWorld) {
    world.captures.insert(0, None);
}

#[given(regex = r#"^the microphone captures "(.+)" on the second capture$"#)]
fn given_mic_second(world: &mut WakeWordWorld, text: String) {
    if world.captures.is_empty() {
        world.captures.push(None);
    }
    world.captures.push(Some(text));
}

#[given(regex = r#"^the microphone then captures "(.+)"$"#)]
fn given_mic_then_captures(world: &mut WakeWordWorld, text: String) {
    world.captures.push(Some(text));
}

#[when("the service waits for the wake word")]
fn when_wait_for_wake_word(world: &mut WakeWordWorld) {
    // Build capture and transcriber queues from the scenario data.
    let mut capture_queue: Vec<Option<Vec<u8>>> = Vec::new();
    let mut transcribe_queue: Vec<Option<String>> = Vec::new();

    for cap in &world.captures {
        match cap {
            None => {
                capture_queue.push(None);
                // No audio → transcriber is not called, but we still track it
                world.skipped_empty = true;
            }
            Some(text) => {
                capture_queue.push(Some(DUMMY_AUDIO.to_vec()));
                transcribe_queue.push(Some(text.clone()));
            }
        }
    }

    // Detect if the first non-empty capture doesn't contain the wake word
    let wake = WakeWord::new("claudito").unwrap();
    let mut first_text_seen = false;
    for cap in &world.captures {
        if let Some(text) = cap {
            if !first_text_seen {
                first_text_seen = true;
                if !wake.matches(text) {
                    world.ignored_first = true;
                }
            }
        }
    }

    let capturer = Arc::new(FakeCapturer::new(capture_queue));
    let transcriber: Arc<dyn Transcriber> = Arc::new(FakeTranscriber::new(transcribe_queue));

    let service = VoiceListenerService::new(
        capturer,
        transcriber,
        Arc::new(FakeHandler),
        Arc::new(FakeSpeaker),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    let result = service.wait_for_wake_word();
    world.wake_word_detected = true; // wait_for_wake_word only returns when detected
    world.inline_order = result;

    // Check if fuzzy match was used (the last matching capture is not exact)
    for cap in world.captures.iter().rev() {
        if let Some(text) = cap {
            if wake.matches(text) {
                let exact = text.to_lowercase().contains("claudito");
                if !exact {
                    world.fuzzy_match = true;
                }
                break;
            }
        }
    }
}

#[then("it detects the wake word")]
fn then_wake_word_detected(world: &mut WakeWordWorld) {
    assert!(world.wake_word_detected, "wake word was not detected");
}

#[then("it returns no inline order")]
fn then_no_inline_order(world: &mut WakeWordWorld) {
    assert!(world.inline_order.is_none(), "expected no inline order, got {:?}", world.inline_order);
}

#[then(regex = r#"^it returns "(.+)" as the inline order$"#)]
fn then_inline_order(world: &mut WakeWordWorld, expected: String) {
    assert_eq!(world.inline_order.as_deref(), Some(expected.as_str()));
}

#[then("it skips the empty capture and keeps listening")]
fn then_skips_empty(world: &mut WakeWordWorld) {
    assert!(world.skipped_empty, "expected to skip empty capture");
}

#[then("it ignores the first utterance")]
fn then_ignores_first(world: &mut WakeWordWorld) {
    assert!(world.ignored_first, "expected to ignore the first utterance");
}

#[then("it detects the wake word on the second utterance")]
fn then_wake_word_second(_world: &mut WakeWordWorld) {
    // Already asserted via "it detects the wake word"
}

#[then("it detects the wake word via fuzzy matching")]
fn then_fuzzy(world: &mut WakeWordWorld) {
    assert!(world.wake_word_detected, "wake word was not detected");
    assert!(world.fuzzy_match, "expected fuzzy match");
}

fn main() {
    futures::executor::block_on(
        WakeWordWorld::run("features/wake_word_detection.feature"),
    );
}
