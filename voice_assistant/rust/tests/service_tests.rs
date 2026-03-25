//! Unit-level tests for VoiceListenerService.
//! Detroit School: hand-rolled fakes, no mock library.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::thread;

use voice_assistant::domain::{
    model::{AudioCapture, Language, WakeWord},
    ports::{AudioCapturer, AudioSpeaker, EchoRef, OrderHandler, Transcriber},
    service::VoiceListenerService,
};

// ── Fakes ─────────────────────────────────────────────────────────────────────

fn fake_audio() -> AudioCapture {
    AudioCapture::new(vec![0u8; 4], 16_000, 2)
}

// Shared echo-reference store so tests can inspect it after the service call.
type SharedEchoRef = Arc<Mutex<Option<EchoRef>>>;

struct FakeCapturer {
    captures:       VecDeque<Option<AudioCapture>>,
    echo_ref_store: SharedEchoRef,
}

impl FakeCapturer {
    fn new(captures: Vec<Option<AudioCapture>>, store: SharedEchoRef) -> Self {
        Self { captures: captures.into(), echo_ref_store: store }
    }
}

impl AudioCapturer for FakeCapturer {
    fn capture(&mut self, _t: Option<u64>, _p: Option<u64>, _pause: Option<u64>) -> Option<AudioCapture> {
        self.captures.pop_front().flatten()
    }
    fn calibrate(&mut self, _: f64) {}
    fn mute(&mut self)   {}
    fn unmute(&mut self) {}
    fn set_echo_reference(&mut self, r: Option<EchoRef>) {
        *self.echo_ref_store.lock().unwrap() = r;
    }
}

struct FakeTranscriber {
    texts: Mutex<VecDeque<Option<String>>>,
}

impl FakeTranscriber {
    fn new(texts: Vec<Option<&'static str>>) -> Arc<Self> {
        Arc::new(Self {
            texts: Mutex::new(texts.into_iter().map(|s| s.map(String::from)).collect()),
        })
    }
}

impl Transcriber for FakeTranscriber {
    fn transcribe(&self, _: &AudioCapture, _: &Language) -> Option<String> {
        self.texts.lock().unwrap().pop_front().flatten()
    }
}

struct FakeOrderHandler(String);
impl OrderHandler for FakeOrderHandler {
    fn handle(&self, _: &str) -> String { self.0.clone() }
    fn reset_session(&self) {}
}

struct FakeSpeaker {
    pub beep_count: AtomicUsize,
    pub stopped:    AtomicBool,
}

impl FakeSpeaker {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            beep_count: AtomicUsize::new(0),
            stopped:    AtomicBool::new(false),
        })
    }
}

impl AudioSpeaker for FakeSpeaker {
    fn speak(&self, _: &str, _: &Language, cb: Option<Box<dyn FnOnce() + Send>>) {
        if let Some(f) = cb { f(); }
    }
    fn stop(&self) { self.stopped.store(true, Ordering::SeqCst); }
    fn beep(&self) { self.beep_count.fetch_add(1, Ordering::SeqCst); }
    fn play_melody(&self, stop: Arc<AtomicBool>) {
        while !stop.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn get_echo_reference(&self) -> Option<EchoRef> { None }
}

/// Speaker that blocks in `speak()` until `stop()` is called.
struct BlockingFakeSpeaker {
    stopped:    Arc<AtomicBool>,
    beep_count: AtomicUsize,
}

impl BlockingFakeSpeaker {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            stopped:    Arc::new(AtomicBool::new(false)),
            beep_count: AtomicUsize::new(0),
        })
    }
}

impl AudioSpeaker for BlockingFakeSpeaker {
    fn speak(&self, _: &str, _: &Language, cb: Option<Box<dyn FnOnce() + Send>>) {
        if let Some(f) = cb { f(); }
        while !self.stopped.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn stop(&self) { self.stopped.store(true, Ordering::SeqCst); }
    fn beep(&self) { self.beep_count.fetch_add(1, Ordering::SeqCst); }
    fn play_melody(&self, stop: Arc<AtomicBool>) {
        while !stop.load(Ordering::SeqCst) {
            thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    fn get_echo_reference(&self) -> Option<EchoRef> { None }
}

// ── builder ───────────────────────────────────────────────────────────────────

fn make_service(
    captures:       Vec<Option<AudioCapture>>,
    transcriptions: Vec<Option<&'static str>>,
    speaker:        Arc<dyn AudioSpeaker>,
    response:       &str,
) -> (VoiceListenerService, SharedEchoRef) {
    let store = Arc::new(Mutex::new(None));
    let svc = VoiceListenerService::new(
        Box::new(FakeCapturer::new(captures, Arc::clone(&store))),
        FakeTranscriber::new(transcriptions),
        Arc::new(FakeOrderHandler(response.into())),
        speaker,
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );
    (svc, store)
}

fn make_interruptible(
    captures:       Vec<Option<AudioCapture>>,
    transcriptions: Vec<Option<&'static str>>,
    speaker:        Arc<dyn AudioSpeaker>,
) -> (VoiceListenerService, SharedEchoRef) {
    make_service(captures, transcriptions, speaker, "respuesta")
}

// ── wait_for_wake_word ────────────────────────────────────────────────────────

#[test]
fn returns_none_when_utterance_is_only_the_wake_word() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio())], vec![Some("claudito")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.wait_for_wake_word(), None);
}

#[test]
fn returns_inline_order_when_wake_word_and_order_in_same_utterance() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio())], vec![Some("claudito pon música")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.wait_for_wake_word(), Some("pon música".into()));
}

#[test]
fn skips_none_captures_and_keeps_listening() {
    let (mut svc, _) = make_service(
        vec![None, Some(fake_audio())], vec![Some("claudito")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.wait_for_wake_word(), None);
}

#[test]
fn skips_utterances_without_wake_word() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio()), Some(fake_audio())],
        vec![Some("hola mundo"), Some("claudito")],
        FakeSpeaker::new(),
        "ok",
    );
    assert_eq!(svc.wait_for_wake_word(), None);
}

#[test]
fn accepts_fuzzy_match_of_wake_word() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio())], vec![Some("clauditto")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.wait_for_wake_word(), None); // detected, no inline order
}

// ── listen_for_order ──────────────────────────────────────────────────────────

#[test]
fn returns_transcribed_text_on_first_attempt() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio())], vec![Some("enciende la luz")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.listen_for_order(), Some("enciende la luz".into()));
}

#[test]
fn returns_none_when_all_captures_time_out() {
    let (mut svc, _) = make_service(vec![None, None], vec![], FakeSpeaker::new(), "ok");
    assert_eq!(svc.listen_for_order(), None);
}

#[test]
fn retries_when_capture_returns_none() {
    let (mut svc, _) = make_service(
        vec![None, Some(fake_audio())], vec![Some("apaga la luz")], FakeSpeaker::new(), "ok"
    );
    assert_eq!(svc.listen_for_order(), Some("apaga la luz".into()));
}

#[test]
fn retries_when_transcription_returns_none() {
    let (mut svc, _) = make_service(
        vec![Some(fake_audio()), Some(fake_audio())],
        vec![None, Some("qué hora es")],
        FakeSpeaker::new(),
        "ok",
    );
    assert_eq!(svc.listen_for_order(), Some("qué hora es".into()));
}

#[test]
fn beeps_once_per_attempt() {
    let speaker = FakeSpeaker::new();
    let sp_ref  = Arc::clone(&speaker);
    let (mut svc, _) = make_service(
        vec![None, Some(fake_audio())], vec![Some("hola")], speaker, "ok"
    );
    svc.listen_for_order();
    assert_eq!(sp_ref.beep_count.load(Ordering::SeqCst), 2);
}

// ── speak_interruptible ───────────────────────────────────────────────────────

fn run_interruptible(
    speaker:        Arc<dyn AudioSpeaker>,
    captures:       Vec<Option<AudioCapture>>,
    transcriptions: Vec<Option<&'static str>>,
) -> (bool, SharedEchoRef) {
    let (mut svc, store) = make_interruptible(captures, transcriptions, Arc::clone(&speaker));
    let stop   = Arc::new(AtomicBool::new(false));
    let sc     = Arc::clone(&stop);
    let sp     = Arc::clone(&speaker);
    let handle = thread::spawn(move || sp.play_melody(sc));
    let interrupted = svc.speak_interruptible("respuesta", stop, handle);
    (interrupted, store)
}

#[test]
fn returns_false_when_speech_ends_without_interruption() {
    let (interrupted, _) = run_interruptible(FakeSpeaker::new(), vec![], vec![]);
    assert!(!interrupted);
}

#[test]
fn returns_true_when_wake_word_heard_during_speech() {
    let speaker = BlockingFakeSpeaker::new();
    let (interrupted, _) = run_interruptible(
        speaker, vec![Some(fake_audio())], vec![Some("claudito")]
    );
    assert!(interrupted);
}

#[test]
fn stops_speaker_when_wake_word_interrupts() {
    let speaker = BlockingFakeSpeaker::new();
    let sp_ref  = Arc::clone(&speaker);
    run_interruptible(speaker, vec![Some(fake_audio())], vec![Some("claudito")]);
    assert!(sp_ref.stopped.load(Ordering::SeqCst));
}

#[test]
fn echo_reference_is_cleared_after_speech() {
    let (_, store) = run_interruptible(FakeSpeaker::new(), vec![], vec![]);
    assert!(store.lock().unwrap().is_none());
}

#[test]
fn echo_reference_is_cleared_even_when_interrupted() {
    let speaker = BlockingFakeSpeaker::new();
    let (_, store) = run_interruptible(
        speaker, vec![Some(fake_audio())], vec![Some("claudito")]
    );
    assert!(store.lock().unwrap().is_none());
}

#[test]
fn does_not_interrupt_on_unrelated_speech_then_interrupts_on_wake_word() {
    let speaker = BlockingFakeSpeaker::new();
    let sp_ref  = Arc::clone(&speaker);
    let (interrupted, _) = run_interruptible(
        speaker,
        vec![Some(fake_audio()), Some(fake_audio())],
        vec![Some("hola mundo"), Some("claudito")],
    );
    assert!(interrupted);
    assert!(sp_ref.stopped.load(Ordering::SeqCst));
}
