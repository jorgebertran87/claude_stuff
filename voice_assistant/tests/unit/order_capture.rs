use cucumber::{given, when, then, World};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use voice_assistant::domain::model::{AudioCapture, Language, WakeWord};
use voice_assistant::domain::ports::{AudioCapturer, AudioSpeaker, EchoRef, OrderHandler, Transcriber};
use voice_assistant::domain::service::VoiceListenerService;

// ── Fake capturer ────────────────────────────────────────────────────────────

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

// ── Fake transcriber ─────────────────────────────────────────────────────────

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

// ── Fake speaker ─────────────────────────────────────────────────────────────

struct FakeSpeaker {
    beep_count: AtomicUsize,
}

impl FakeSpeaker {
    fn new() -> Self {
        Self { beep_count: AtomicUsize::new(0) }
    }
}

impl AudioSpeaker for FakeSpeaker {
    fn speak(&self, _text: &str, _lang: &Language, _cb: Option<Box<dyn FnOnce() + Send>>) {}
    fn stop(&self) {}
    fn beep(&self) {
        self.beep_count.fetch_add(1, Ordering::SeqCst);
    }
    fn play_melody(&self, _stop: Arc<std::sync::atomic::AtomicBool>) {}
    fn get_echo_reference(&self) -> Option<EchoRef> { None }
}

// ── Fake handler ─────────────────────────────────────────────────────────────

struct FakeHandler;
impl OrderHandler for FakeHandler {
    fn handle(&self, _order: &str) -> String { String::new() }
    fn reset_session(&self) {}
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct OrderWorld {
    capture_queue: Vec<Option<Vec<u8>>>,
    transcription_queue: Vec<Option<String>>,
    result: Option<Option<String>>,
    beep_count: usize,
}

const DUMMY_AUDIO: &[u8] = &[0u8; 100];

#[given("the microphone captures a valid audio clip")]
fn given_valid_audio(world: &mut OrderWorld) {
    world.capture_queue.push(Some(DUMMY_AUDIO.to_vec()));
}

#[given(regex = r#"^the transcription returns "(.+)"$"#)]
fn given_transcription(world: &mut OrderWorld, text: String) {
    world.transcription_queue.push(Some(text));
}

#[given("the microphone returns no audio on every attempt")]
fn given_no_audio_all(world: &mut OrderWorld) {
    world.capture_queue.push(None);
    world.capture_queue.push(None);
}

#[given("the microphone returns no audio on the first attempt")]
fn given_no_audio_first(world: &mut OrderWorld) {
    world.capture_queue.push(None);
}

#[given("the microphone captures a valid audio clip on the second attempt")]
fn given_valid_second(world: &mut OrderWorld) {
    world.capture_queue.push(Some(DUMMY_AUDIO.to_vec()));
}

#[given("the microphone captures a valid audio clip on both attempts")]
fn given_valid_both(world: &mut OrderWorld) {
    world.capture_queue.push(Some(DUMMY_AUDIO.to_vec()));
    world.capture_queue.push(Some(DUMMY_AUDIO.to_vec()));
}

#[given("the first transcription returns nothing")]
fn given_first_transcript_empty(world: &mut OrderWorld) {
    world.transcription_queue.push(None);
}

#[given(regex = r#"^the second transcription returns "(.+)"$"#)]
fn given_second_transcript(world: &mut OrderWorld, text: String) {
    world.transcription_queue.push(Some(text));
}

#[when("the service listens for an order")]
fn when_listen(world: &mut OrderWorld) {
    let speaker = Arc::new(FakeSpeaker::new());
    let transcriber: Arc<dyn Transcriber> = Arc::new(
        FakeTranscriber::new(world.transcription_queue.drain(..).collect()),
    );

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer::new(world.capture_queue.drain(..).collect())),
        transcriber,
        Arc::new(FakeHandler),
        speaker.clone(),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    world.result = Some(service.listen_for_order());
    world.beep_count = speaker.beep_count.load(Ordering::SeqCst);
}

#[then(regex = r#"^it returns "(.+)"$"#)]
fn then_returns(world: &mut OrderWorld, expected: String) {
    assert_eq!(
        world.result.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
    );
}

#[then("it returns no order after exhausting all retries")]
fn then_no_order(world: &mut OrderWorld) {
    assert!(
        world.result.as_ref().unwrap().is_none(),
        "expected no order"
    );
}

#[then(regex = r#"^it retries and returns "(.+)"$"#)]
fn then_retries_and_returns(world: &mut OrderWorld, expected: String) {
    assert_eq!(
        world.result.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
    );
}

#[then(regex = r"^the speaker has beeped exactly (\d+) times$")]
fn then_beep_count(world: &mut OrderWorld, expected: usize) {
    assert_eq!(world.beep_count, expected, "beep count mismatch");
}

fn main() {
    futures::executor::block_on(
        OrderWorld::run("features/order_capture.feature"),
    );
}
