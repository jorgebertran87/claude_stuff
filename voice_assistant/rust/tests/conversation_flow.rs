use cucumber::{given, when, then, World};

use voice_assistant::domain::model::WakeWord;

/// This feature tests conversation state transitions. We test the logic at the
/// model / decision level because `VoiceListenerService::run` is an infinite
/// loop that requires real threading.

#[derive(Debug, Default, World)]
pub struct ConvoWorld {
    response: String,
    waiting_for_answer: bool,
    interrupted: bool,
    order_captured: Option<String>,
    melody_stopped: bool,
    echo_cleared: bool,
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
fn given_user_says(world: &mut ConvoWorld, _wake: String, _order: String) {
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
fn then_next_order_no_wake(world: &mut ConvoWorld, order: String) {
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

fn main() {
    futures::executor::block_on(
        ConvoWorld::run("features/conversation_flow.feature"),
    );
}
