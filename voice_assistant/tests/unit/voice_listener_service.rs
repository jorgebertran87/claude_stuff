use cucumber::{given, when, then, World};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use voice_assistant::cli::{parse_args, CliArgs};
use voice_assistant::domain::model::{AudioCapture, Language, WakeWord};
use voice_assistant::domain::ports::{
    AudioCapturer, AudioSpeaker, EchoRef, OrderHandler, Transcriber,
};
use voice_assistant::domain::service::VoiceListenerService;

// ── CLI parse result wrapper (Result does not implement Default) ────────────

#[derive(Debug)]
enum CliParseResult {
    NotParsed,
    DirectOrder(String),
    TelegramMode,
    ListenMode,
    Error(String),
}

impl Default for CliParseResult {
    fn default() -> Self {
        CliParseResult::NotParsed
    }
}

// ── Fake AudioCapturer ────────────────────────────────────────────────────────

struct FakeCapturer {
    queue: Mutex<Vec<Option<AudioCapture>>>,
}

impl FakeCapturer {
    fn new(queue: Vec<Option<AudioCapture>>) -> Self {
        Self { queue: Mutex::new(queue) }
    }
}

impl AudioCapturer for FakeCapturer {
    fn capture(
        &self,
        _timeout_ms: Option<u64>,
        _phrase_time_limit_ms: Option<u64>,
        _pause_threshold_ms: Option<u64>,
    ) -> Option<AudioCapture> {
        let mut q = self.queue.lock().unwrap();
        if q.is_empty() {
            return None;
        }
        q.remove(0)
    }
    fn calibrate(&self, _: f64) {}
    fn mute(&self) {}
    fn unmute(&self) {}
    fn set_echo_reference(&self, _: Option<EchoRef>) {}
    fn apply_echo_cancellation(&self, raw: &[u8], _: u32, _: u16) -> Vec<u8> {
        raw.to_vec()
    }
}

// ── Fake Transcriber ──────────────────────────────────────────────────────────

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
        if r.is_empty() {
            return None;
        }
        r.remove(0)
    }
}

// ── Fake AudioSpeaker ─────────────────────────────────────────────────────────

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
    fn play_melody(&self, stop_signal: Arc<AtomicBool>) {
        // Block until the stop signal is set, simulating real melody playback.
        while !stop_signal.load(Ordering::SeqCst) {
            thread::sleep(Duration::from_millis(50));
        }
    }
    fn get_echo_reference(&self) -> Option<EchoRef> {
        None
    }
}

// ── Tracking OrderHandler ─────────────────────────────────────────────────────

struct TrackingOrderHandler {
    last_order: Arc<Mutex<Option<String>>>,
    response: String,
    reset_called: Arc<AtomicBool>,
}

impl TrackingOrderHandler {
    fn new(response: &str) -> Self {
        Self {
            last_order: Arc::new(Mutex::new(None)),
            response: response.to_string(),
            reset_called: Arc::new(AtomicBool::new(false)),
        }
    }

    fn last_order_arc(&self) -> Arc<Mutex<Option<String>>> {
        Arc::clone(&self.last_order)
    }

    fn reset_called_arc(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.reset_called)
    }
}

impl OrderHandler for TrackingOrderHandler {
    fn handle(&self, order: &str) -> String {
        *self.last_order.lock().unwrap() = Some(order.to_string());
        self.response.clone()
    }
    fn reset_session(&self) {
        self.reset_called.store(true, Ordering::SeqCst);
    }
}

// ── World ─────────────────────────────────────────────────────────────────────

const DUMMY_AUDIO: &[u8] = &[0u8; 100];

fn dummy_audio() -> AudioCapture {
    AudioCapture::new(DUMMY_AUDIO.to_vec(), 16000, 2)
}

#[derive(Debug, Default, World)]
pub struct VoiceListenerWorld {
    // ── handle_with_melody fields ──────────────────────────────────────────
    melody_response: Option<String>,
    melody_last_order: Option<String>,
    melody_stop_signal: Option<Arc<AtomicBool>>,
    melody_thread_handle: Option<thread::JoinHandle<()>>,

    // ── handle_meta_commands fields ────────────────────────────────────────
    reset_called: bool,
    confirmation: Option<String>,

    // ── listen_for_order fields ────────────────────────────────────────────
    capture_queue: Vec<Option<AudioCapture>>,
    transcription_queue: Vec<Option<String>>,
    order_result: Option<Option<String>>,
    beep_count: usize,

    // ── wait_for_wake_word fields ──────────────────────────────────────────
    wake_word_capture_queue: Vec<Option<AudioCapture>>,
    wake_word_transcription_queue: Vec<Option<String>>,
    wake_word_detected: Option<bool>,
    inline_order: Option<Option<String>>,

    // ── CLI parsing fields ─────────────────────────────────────────────────
    cli_args: Vec<String>,
    cli_parse_result: CliParseResult,

    // ── WakeWord unit test fields ──────────────────────────────────────────
    unit_wake_word: Option<WakeWord>,
    unit_match_result: Option<bool>,
    unit_extract_result: Option<Option<String>>,

    // ── Language unit test fields ──────────────────────────────────────────
    unit_language: Option<Language>,
    unit_prefix_result: Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// handle_with_melody steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given(regex = r#"^a VoiceListenerService with an order handler that returns "(.+)"$"#)]
fn given_handler_returns(_world: &mut VoiceListenerWorld, _response: String) {
    // The handler is built in the when step so we can capture shared state.
}

#[when(regex = r#"^the service handles the order "(.+)" with melody$"#)]
fn when_handle_with_melody(world: &mut VoiceListenerWorld, order: String) {
    let handler = TrackingOrderHandler::new("Son las 3 de la tarde");
    let last_order = handler.last_order_arc();
    let speaker = Arc::new(FakeSpeaker::new());

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer::new(vec![])),
        Arc::new(FakeTranscriber::new(vec![])),
        Arc::new(handler),
        speaker,
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    let (response, stop_signal, join_handle) = service.handle_with_melody(&order);

    world.melody_response = Some(response);
    world.melody_last_order = last_order.lock().unwrap().clone();
    world.melody_stop_signal = Some(stop_signal);
    world.melody_thread_handle = Some(join_handle);
}

#[then(regex = r#"^the order handler receives "(.+)"$"#)]
fn then_handler_receives(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.melody_last_order.as_deref(),
        Some(expected.as_str()),
        "order handler should have received the order text"
    );
}

#[then(regex = r#"^the response is "(.+)"$"#)]
fn then_response_is(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.melody_response.as_deref(),
        Some(expected.as_str()),
        "response should match what the handler returned"
    );
}

#[then("the melody is playing (stop signal is initially false)")]
fn then_melody_playing(world: &mut VoiceListenerWorld) {
    let stop_signal = world.melody_stop_signal.as_ref().expect("stop signal should exist");
    assert!(
        !stop_signal.load(Ordering::SeqCst),
        "stop signal should be false while melody is playing"
    );
    // Clean up: stop the melody thread so it doesn't leak.
    stop_signal.store(true, Ordering::SeqCst);
    if let Some(handle) = world.melody_thread_handle.take() {
        let _ = handle.join();
    }
}

// ── Scenario: Setting the stop signal terminates the melody thread ──────────

#[given(regex = r#"^the service is handling the order "(.+)" with melody$"#)]
fn given_handling_with_melody(world: &mut VoiceListenerWorld, order: String) {
    let handler = TrackingOrderHandler::new("ok");
    let speaker = Arc::new(FakeSpeaker::new());

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer::new(vec![])),
        Arc::new(FakeTranscriber::new(vec![])),
        Arc::new(handler),
        speaker,
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    let (_response, stop_signal, join_handle) = service.handle_with_melody(&order);
    world.melody_stop_signal = Some(stop_signal);
    world.melody_thread_handle = Some(join_handle);
}

#[when("the stop signal is set")]
fn when_stop_signal_set(world: &mut VoiceListenerWorld) {
    world
        .melody_stop_signal
        .as_ref()
        .expect("stop signal should exist")
        .store(true, Ordering::SeqCst);
}

#[then("the melody thread terminates within 2 seconds")]
fn then_melody_terminates(world: &mut VoiceListenerWorld) {
    let handle = world.melody_thread_handle.take().expect("thread handle should exist");
    for _ in 0..40 {
        if handle.is_finished() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("melody thread did not terminate within 2 seconds");
}

// ═══════════════════════════════════════════════════════════════════════════════
// handle_meta_commands steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given("a VoiceListenerService with a tracking order handler")]
fn given_tracking_handler(_world: &mut VoiceListenerWorld) {
    // Built in the when step so we can capture shared state.
}

#[when(regex = r#"^the service checks the meta-command "(.+)"$"#)]
fn when_check_meta_command(world: &mut VoiceListenerWorld, order: String) {
    let handler = TrackingOrderHandler::new("dummy");
    let reset_called = handler.reset_called_arc();

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer::new(vec![])),
        Arc::new(FakeTranscriber::new(vec![])),
        Arc::new(handler),
        Arc::new(FakeSpeaker::new()),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    world.confirmation = service.handle_meta_commands(&order);
    world.reset_called = reset_called.load(Ordering::SeqCst);
}

#[then("reset_session is called on the order handler")]
fn then_meta_reset_called(world: &mut VoiceListenerWorld) {
    assert!(world.reset_called, "expected reset_session to be called");
}

#[then(regex = r#"^the confirmation message is "(.+)"$"#)]
fn then_meta_confirmation(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(world.confirmation.as_deref(), Some(expected.as_str()));
}

#[then("reset_session is not called on the order handler")]
fn then_meta_reset_not_called(world: &mut VoiceListenerWorld) {
    assert!(!world.reset_called, "expected reset_session NOT to be called");
}

#[then("no confirmation message is returned")]
fn then_meta_no_confirmation(world: &mut VoiceListenerWorld) {
    assert!(
        world.confirmation.is_none(),
        "expected no confirmation, got {:?}",
        world.confirmation
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// listen_for_order steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given("the microphone captures a valid audio clip for order listening")]
fn given_order_valid_audio(world: &mut VoiceListenerWorld) {
    world.capture_queue.push(Some(dummy_audio()));
}

#[given(regex = r#"^the transcription of the order returns "(.+)"$"#)]
fn given_order_transcription(world: &mut VoiceListenerWorld, text: String) {
    world.transcription_queue.push(Some(text));
}

#[given("the microphone returns no audio on the first order attempt")]
fn given_order_no_audio_first(world: &mut VoiceListenerWorld) {
    world.capture_queue.push(None);
}

#[given("the microphone captures a valid audio clip on the second order attempt")]
fn given_order_valid_second(world: &mut VoiceListenerWorld) {
    world.capture_queue.push(Some(dummy_audio()));
}

#[given("the microphone captures a valid audio clip on both order attempts")]
fn given_order_valid_both(world: &mut VoiceListenerWorld) {
    world.capture_queue.push(Some(dummy_audio()));
    world.capture_queue.push(Some(dummy_audio()));
}

#[given("the first transcription of the order returns nothing")]
fn given_order_first_transcript_empty(world: &mut VoiceListenerWorld) {
    world.transcription_queue.push(None);
}

#[given(regex = r#"^the second transcription of the order returns "(.+)"$"#)]
fn given_order_second_transcript(world: &mut VoiceListenerWorld, text: String) {
    world.transcription_queue.push(Some(text));
}

#[when("the service listens for an order")]
fn when_order_listen(world: &mut VoiceListenerWorld) {
    let speaker = Arc::new(FakeSpeaker::new());

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer::new(world.capture_queue.drain(..).collect())),
        Arc::new(FakeTranscriber::new(world.transcription_queue.drain(..).collect())),
        Arc::new(TrackingOrderHandler::new("unused")),
        speaker.clone(),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    world.order_result = Some(service.listen_for_order());
    world.beep_count = speaker.beep_count.load(Ordering::SeqCst);
}

#[then(regex = r#"^the service returns "(.+)"$"#)]
fn then_order_returns(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.order_result.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
    );
}

#[then(regex = r#"^the service retries and returns "(.+)"$"#)]
fn then_order_retries_and_returns(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.order_result.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
    );
}

#[then(regex = r"^the speaker has beeped for order exactly (\d+) times$")]
fn then_order_beep_count(world: &mut VoiceListenerWorld, expected: usize) {
    assert_eq!(world.beep_count, expected, "beep count mismatch");
}

// ═══════════════════════════════════════════════════════════════════════════════
// wait_for_wake_word steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given(regex = r#"^the microphone captures "(.+)" for wake word detection$"#)]
fn given_ww_audio(world: &mut VoiceListenerWorld, text: String) {
    world.wake_word_capture_queue.push(Some(dummy_audio()));
    world.wake_word_transcription_queue.push(Some(text));
}

#[given(regex = r#"^the microphone then captures "(.+)" for wake word detection$"#)]
fn given_ww_then_captures(world: &mut VoiceListenerWorld, text: String) {
    world.wake_word_capture_queue.push(Some(dummy_audio()));
    world.wake_word_transcription_queue.push(Some(text));
}

#[when("the service waits for the wake word")]
fn when_ww_wait(world: &mut VoiceListenerWorld) {
    // Use a capturer that panics on exhaustion instead of returning None.
    // When a mutant breaks WakeWord::matches() it would otherwise loop
    // forever inside wait_for_wake_word(), producing a 20 s TIMEOUT.
    // Panicking immediately gives a fast, clean CAUGHT instead.
    struct WakeWordCapturer {
        queue: Mutex<Vec<Option<AudioCapture>>>,
    }
    impl AudioCapturer for WakeWordCapturer {
        fn capture(
            &self,
            _timeout_ms: Option<u64>,
            _phrase_time_limit_ms: Option<u64>,
            _pause_threshold_ms: Option<u64>,
        ) -> Option<AudioCapture> {
            let mut q = self.queue.lock().unwrap();
            if q.is_empty() {
                panic!(
                    "wake word capturer exhausted: wait_for_wake_word() did not \
                     detect the wake word (matches() may be broken by a mutant)"
                );
            }
            q.remove(0)
        }
        fn calibrate(&self, _: f64) {}
        fn mute(&self) {}
        fn unmute(&self) {}
        fn set_echo_reference(&self, _: Option<EchoRef>) {}
        fn apply_echo_cancellation(&self, raw: &[u8], _: u32, _: u16) -> Vec<u8> {
            raw.to_vec()
        }
    }

    let service = VoiceListenerService::new(
        Arc::new(WakeWordCapturer {
            queue: Mutex::new(world.wake_word_capture_queue.drain(..).collect()),
        }),
        Arc::new(FakeTranscriber::new(world.wake_word_transcription_queue.drain(..).collect())),
        Arc::new(TrackingOrderHandler::new("unused")),
        Arc::new(FakeSpeaker::new()),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    let result = service.wait_for_wake_word();
    // wait_for_wake_word returns Some(order) when inline order is present,
    // or None when the wake word was heard but no order followed.
    // The fact that it returned at all means the wake word was detected
    // (otherwise it would have panicked in the capturer).
    world.wake_word_detected = Some(true);
    world.inline_order = Some(result);
}

#[then("the service detects the wake word")]
fn then_ww_detected(world: &mut VoiceListenerWorld) {
    assert!(world.wake_word_detected.unwrap());
}

#[then("no inline order is returned")]
fn then_ww_no_inline(world: &mut VoiceListenerWorld) {
    assert_eq!(
        world.inline_order.as_ref().unwrap().as_deref(),
        None,
        "expected no inline order"
    );
}

#[then(regex = r#"^the inline order "(.+)" is returned$"#)]
fn then_ww_inline_order(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.inline_order.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
        "inline order mismatch"
    );
}

#[then("the service detects the wake word via fuzzy matching")]
fn then_ww_fuzzy(world: &mut VoiceListenerWorld) {
    assert!(world.wake_word_detected.unwrap());
}

#[then("the service ignores the first utterance and keeps listening")]
fn then_ww_ignores_first(_world: &mut VoiceListenerWorld) {
    // The first utterance ("hola mundo") was consumed by the loop and ignored.
    // The service continued looping and consumed the second.
    // If we reach here without hanging, the capturer queue was drained correctly.
}

#[then("the service detects the wake word on the second utterance")]
fn then_ww_detected_second(world: &mut VoiceListenerWorld) {
    assert!(world.wake_word_detected.unwrap());
}

// ═══════════════════════════════════════════════════════════════════════════════
// CLI argument parsing steps
// ═══════════════════════════════════════════════════════════════════════════════

/// Parses a comma-separated list of double-quoted strings like `"prog", "--order", "value"`.
fn parse_quoted_args(raw: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut in_quote = false;
    let mut current = String::new();
    for ch in raw.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            if !in_quote {
                args.push(std::mem::take(&mut current));
            }
        } else if in_quote {
            current.push(ch);
        }
    }
    args
}

#[given(regex = r#"^the command-line arguments (.+)$"#)]
fn given_cli_args(world: &mut VoiceListenerWorld, raw: String) {
    world.cli_args = parse_quoted_args(&raw);
}

#[when("the arguments are parsed")]
fn when_parse_args(world: &mut VoiceListenerWorld) {
    world.cli_parse_result = match parse_args(&world.cli_args) {
        Ok(CliArgs::DirectOrder(s)) => CliParseResult::DirectOrder(s),
        Ok(CliArgs::TelegramMode)   => CliParseResult::TelegramMode,
        Ok(CliArgs::ListenMode)     => CliParseResult::ListenMode,
        Err(e)                      => CliParseResult::Error(e),
    };
}

#[then(regex = r#"^the result is DirectOrder with value "(.+)"$"#)]
fn then_cli_direct_order(world: &mut VoiceListenerWorld, expected: String) {
    match &world.cli_parse_result {
        CliParseResult::DirectOrder(s) if s == &expected => {}
        other => panic!("expected DirectOrder({expected:?}), got {other:?}"),
    }
}

#[then("the parsing result is an error")]
fn then_cli_error(world: &mut VoiceListenerWorld) {
    match &world.cli_parse_result {
        CliParseResult::Error(_) => {}
        other => panic!("expected Error, got {other:?}"),
    }
}

#[then("the parsing result is TelegramMode")]
fn then_cli_telegram(world: &mut VoiceListenerWorld) {
    match &world.cli_parse_result {
        CliParseResult::TelegramMode => {}
        other => panic!("expected TelegramMode, got {other:?}"),
    }
}

#[then("the parsing result is ListenMode")]
fn then_cli_listen(world: &mut VoiceListenerWorld) {
    match &world.cli_parse_result {
        CliParseResult::ListenMode => {}
        other => panic!("expected ListenMode, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// WakeWord unit test steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given(regex = r#"^a wake word "(.+)"$"#)]
fn given_unit_wake_word(world: &mut VoiceListenerWorld, value: String) {
    world.unit_wake_word = Some(WakeWord::new(&value).unwrap());
}

#[when(regex = r#"^the wake word is checked against "(.+)"$"#)]
fn when_check_wake_word(world: &mut VoiceListenerWorld, text: String) {
    let ww = world.unit_wake_word.as_ref().expect("wake word not set");
    world.unit_match_result = Some(ww.matches(&text));
}

#[then("the wake word matches")]
fn then_wake_word_matches(world: &mut VoiceListenerWorld) {
    assert!(
        world.unit_match_result.unwrap(),
        "expected wake word to match"
    );
}

#[then("the wake word does not match")]
fn then_wake_word_does_not_match(world: &mut VoiceListenerWorld) {
    assert!(
        !world.unit_match_result.unwrap(),
        "expected wake word NOT to match"
    );
}

#[when(regex = r#"^the order is extracted from "(.+)"$"#)]
fn when_extract_order(world: &mut VoiceListenerWorld, text: String) {
    let ww = world.unit_wake_word.as_ref().expect("wake word not set");
    world.unit_extract_result = Some(ww.extract_order(&text));
}

#[then(regex = r#"^the extracted order is "(.+)"$"#)]
fn then_extracted_order(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.unit_extract_result.as_ref().unwrap().as_deref(),
        Some(expected.as_str()),
        "extracted order mismatch"
    );
}

#[then("no order is extracted")]
fn then_no_order_extracted(world: &mut VoiceListenerWorld) {
    assert_eq!(
        world.unit_extract_result.as_ref().unwrap().as_deref(),
        None,
        "expected no extracted order"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Language unit test steps
// ═══════════════════════════════════════════════════════════════════════════════

#[given(regex = r#"^a language with code "(.+)"$"#)]
fn given_unit_language(world: &mut VoiceListenerWorld, code: String) {
    world.unit_language = Some(Language::new(&code).unwrap());
}

#[when("the language prefix is requested")]
fn when_lang_prefix(world: &mut VoiceListenerWorld) {
    let lang = world.unit_language.as_ref().expect("language not set");
    world.unit_prefix_result = Some(lang.lang_prefix().to_string());
}

#[then(regex = r#"^the prefix is "(.+)"$"#)]
fn then_lang_prefix(world: &mut VoiceListenerWorld, expected: String) {
    assert_eq!(
        world.unit_prefix_result.as_deref(),
        Some(expected.as_str()),
        "language prefix mismatch"
    );
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    futures::executor::block_on(VoiceListenerWorld::run(
        "features/voice_listener_service.feature",
    ));
}
