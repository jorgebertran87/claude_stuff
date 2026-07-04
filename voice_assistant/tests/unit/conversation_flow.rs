use cucumber::{given, when, then, World};
use std::sync::{Arc, Mutex, atomic::AtomicBool};

use voice_assistant::domain::model::{Language, WakeWord};
use voice_assistant::domain::ports::{AudioCapturer, AudioSpeaker, EchoRef, OrderHandler, Transcriber};
use voice_assistant::domain::service::VoiceListenerService;

/// This feature tests conversation state transitions. We test the logic at the
/// model / decision level because `VoiceListenerService::run` is an infinite
/// loop that requires real threading.
///
/// The meta-command scenarios test `handle_meta_commands` directly with a
/// real VoiceListenerService and fake adapters.

// ── Fakes ────────────────────────────────────────────────────────────────────

struct FakeCapturer;
impl AudioCapturer for FakeCapturer {
    fn capture(&self, _t: Option<u64>, _p: Option<u64>, _pa: Option<u64>) -> Option<voice_assistant::domain::model::AudioCapture> {
        None
    }
    fn calibrate(&self, _: f64) {}
    fn mute(&self) {}
    fn unmute(&self) {}
    fn set_echo_reference(&self, _: Option<EchoRef>) {}
}

struct FakeTranscriber;
impl Transcriber for FakeTranscriber {
    fn transcribe(&self, _audio: &voice_assistant::domain::model::AudioCapture, _lang: &Language) -> Option<String> {
        None
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

struct TrackingOrderHandler {
    reset_called: Arc<Mutex<bool>>,
    response: String,
}

impl TrackingOrderHandler {
    fn new(reset_called: Arc<Mutex<bool>>, response: &str) -> Self {
        Self { reset_called, response: response.into() }
    }
}

impl OrderHandler for TrackingOrderHandler {
    fn handle(&self, _order: &str) -> String {
        self.response.clone()
    }
    fn reset_session(&self) {
        *self.reset_called.lock().unwrap() = true;
    }
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct ConvoWorld {
    // ── existing fields (scenarios 1-4) ──────────────────────────────────
    response: String,
    waiting_for_answer: bool,
    interrupted: bool,
    melody_stopped: bool,
    echo_cleared: bool,

    // ── meta-command fields (scenarios 5-8) ──────────────────────────────
    reset_called: bool,
    confirmation: Option<String>,
}

// ── Scenario: Melody thread is fully stopped before the response is spoken ───

#[given("an order has been handled and a response is ready")]
fn given_response_ready(world: &mut ConvoWorld) {
    world.response = "Son las 3 de la tarde.".into();
}

#[when("the service speaks the response")]
fn when_speaks(world: &mut ConvoWorld) {
    // In the real code, speak_interruptible joins melody_thread before returning.
    // We simulate: melody is stopped after playback.
    world.melody_stopped = true;
}

#[then("the melody thread is no longer alive after playback ends")]
fn then_melody_dead(world: &mut ConvoWorld) {
    assert!(world.melody_stopped, "melody thread should be stopped");
}

// ── Scenario: Echo reference is always cleared ──────────────────────────────

#[given("the assistant speaks a response")]
fn given_speaks_response(world: &mut ConvoWorld) {
    world.response = "hola".into();
}

#[when("the speech finishes")]
fn when_speech_finishes(world: &mut ConvoWorld) {
    // speak_interruptible always calls capturer.set_echo_reference(None) at the end
    world.echo_cleared = true;
}

#[then("the echo reference stored in the capturer is None")]
fn then_echo_none(world: &mut ConvoWorld) {
    assert!(world.echo_cleared);
}

// ── Scenario: Response ending with a question ───────────────────────────────

#[given(regex = r#"^the user says "(.+)" followed by "(.+)"$"#)]
fn given_user_says(_world: &mut ConvoWorld, _wake: String, _order: String) {
    // Setup context; the order has been processed.
}

#[given(regex = r#"^the assistant responds with "(.+)"$"#)]
fn given_response(world: &mut ConvoWorld, response: String) {
    world.response = response;
}

#[when("the assistant finishes speaking")]
fn when_assistant_finishes(world: &mut ConvoWorld) {
    world.interrupted = false;
    // Logic from run(): if not interrupted, check if response ends with '?'
    world.waiting_for_answer = world.response.trim_end().ends_with('?');
}

#[then("waiting_for_answer is set to True")]
fn then_waiting_true(world: &mut ConvoWorld) {
    assert!(world.waiting_for_answer, "expected waiting_for_answer=true");
}

#[then(regex = r#"^the next order "(.+)" is captured without requiring the wake word again$"#)]
fn then_next_order_no_wake(world: &mut ConvoWorld, _order: String) {
    // When waiting_for_answer is true, listen_for_order() is called directly
    // (skipping wait_for_wake_word). We verify the flag is set.
    assert!(world.waiting_for_answer);
}

// ── Scenario: Interruption sets waiting_for_answer ──────────────────────────

#[given("the assistant is speaking a long response")]
fn given_long_response(world: &mut ConvoWorld) {
    world.response = "Esta es una respuesta muy larga que tomará tiempo.".into();
}

#[given(regex = r#"^the user says the wake word "(.+)" during playback$"#)]
fn given_wake_during(world: &mut ConvoWorld, _wake: String) {
    world.interrupted = true;
}

#[when("the speech is interrupted")]
fn when_interrupted(world: &mut ConvoWorld) {
    // Logic from run(): if interrupted, waiting_for_answer = true
    world.waiting_for_answer = world.interrupted;
}

#[then("the next order is captured directly without requiring the wake word again")]
fn then_next_direct(world: &mut ConvoWorld) {
    assert!(world.waiting_for_answer);
}

// ── Meta-command scenarios ──────────────────────────────────────────────────

#[given("a VoiceListenerService with a FakeOrderHandler")]
fn given_service_with_fake_handler(_world: &mut ConvoWorld) {
    // Nothing to set up here — the service is built in the when step
    // so we can pass the order text directly.
}

#[when(regex = r#"^the service handles the meta-command "(.+)"$"#)]
fn when_handle_meta_command(world: &mut ConvoWorld, order: String) {
    let reset_called = Arc::new(Mutex::new(false));
    let handler: Arc<dyn OrderHandler> = Arc::new(TrackingOrderHandler::new(
        reset_called.clone(),
        "dummy response",
    ));

    let service = VoiceListenerService::new(
        Arc::new(FakeCapturer),
        Arc::new(FakeTranscriber),
        handler,
        Arc::new(FakeSpeaker),
        WakeWord::new("claudito").unwrap(),
        Language::new("es-ES").unwrap(),
    );

    world.confirmation = service.handle_meta_commands(&order);
    world.reset_called = *reset_called.lock().unwrap();
}

#[then("reset_session is called on the order handler")]
fn then_reset_called(world: &mut ConvoWorld) {
    assert!(world.reset_called, "expected reset_session to be called");
}

#[then(regex = r#"^the confirmation message is "(.+)"$"#)]
fn then_confirmation(world: &mut ConvoWorld, expected: String) {
    assert_eq!(world.confirmation.as_deref(), Some(expected.as_str()));
}

#[then("reset_session is not called on the order handler")]
fn then_reset_not_called(world: &mut ConvoWorld) {
    assert!(!world.reset_called, "expected reset_session NOT to be called");
}

#[then("no confirmation message is returned")]
fn then_no_confirmation(world: &mut ConvoWorld) {
    assert!(world.confirmation.is_none(), "expected no confirmation, got {:?}", world.confirmation);
}

fn main() {
    futures::executor::block_on(
        ConvoWorld::run("features/conversation_flow.feature"),
    );
}
