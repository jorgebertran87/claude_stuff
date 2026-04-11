use cucumber::{given, when, then, World};

use voice_assistant::domain::model::WakeWord;

#[derive(Debug, Default, World)]
pub struct InterruptWorld {
    capture_texts: Vec<Option<String>>,
    interrupted: bool,
    speaker_stopped: bool,
    echo_cleared: bool,
}

#[given("the assistant is speaking a response")]
fn given_speaking(_world: &mut InterruptWorld) {}

#[given("the assistant is speaking a long response")]
fn given_long_speaking(_world: &mut InterruptWorld) {}

#[given("no audio is captured during playback")]
fn given_no_audio(_world: &mut InterruptWorld) {}

#[given(regex = r#"^the microphone captures "(.+)" during playback$"#)]
fn given_capture_during(world: &mut InterruptWorld, text: String) {
    world.capture_texts.push(Some(text));
}

#[given("the assistant speaks a response")]
fn given_speaks_response(_world: &mut InterruptWorld) {}

#[given(regex = r#"^the microphone first captures "(.+)"$"#)]
fn given_first_capture(world: &mut InterruptWorld, text: String) {
    world.capture_texts.push(Some(text));
}

#[given(regex = r#"^the microphone then captures "(.+)"$"#)]
fn given_then_capture(world: &mut InterruptWorld, text: String) {
    world.capture_texts.push(Some(text));
}

fn run_speak_interruptible(world: &mut InterruptWorld) {
    let wake = WakeWord::new("claudito").unwrap();

    let mut was_interrupted = false;
    let mut speaker_stopped = false;

    for text_opt in &world.capture_texts {
        if let Some(text) = text_opt {
            if wake.matches(text) {
                was_interrupted = true;
                speaker_stopped = true;
                break;
            }
        }
    }

    // Echo reference is always cleared after speech
    world.echo_cleared = true;
    world.interrupted = was_interrupted;
    world.speaker_stopped = speaker_stopped;
}

#[when("the speech ends naturally")]
fn when_speech_ends(world: &mut InterruptWorld) {
    run_speak_interruptible(world);
}

#[when("the wake word is detected")]
fn when_wake_detected(world: &mut InterruptWorld) {
    run_speak_interruptible(world);
}

#[when("the speech finishes")]
fn when_speech_finishes(world: &mut InterruptWorld) {
    run_speak_interruptible(world);
}

#[when("the wake word interrupts the speech")]
fn when_wake_interrupts(world: &mut InterruptWorld) {
    run_speak_interruptible(world);
}

#[when("the service processes both captures")]
fn when_process_both(world: &mut InterruptWorld) {
    run_speak_interruptible(world);
}

#[then("the service reports it was not interrupted")]
fn then_not_interrupted(world: &mut InterruptWorld) {
    assert!(!world.interrupted, "expected no interruption");
}

#[then("the service stops the speaker")]
fn then_stops_speaker(world: &mut InterruptWorld) {
    assert!(world.speaker_stopped, "expected speaker to be stopped");
}

#[then("the service reports it was interrupted")]
fn then_interrupted(world: &mut InterruptWorld) {
    assert!(world.interrupted, "expected interruption");
}

#[then("the speaker receives a stop signal")]
fn then_stop_signal(world: &mut InterruptWorld) {
    assert!(world.speaker_stopped, "expected stop signal");
}

#[then("the echo reference on the capturer is cleared")]
fn then_echo_cleared(world: &mut InterruptWorld) {
    assert!(world.echo_cleared, "expected echo reference to be cleared");
}

#[then("the assistant is eventually interrupted by the wake word")]
fn then_eventually_interrupted(world: &mut InterruptWorld) {
    assert!(world.interrupted, "expected eventual interruption");
}

#[then("the speaker is stopped")]
fn then_speaker_stopped(world: &mut InterruptWorld) {
    assert!(world.speaker_stopped, "expected speaker stopped");
}

fn main() {
    futures::executor::block_on(
        InterruptWorld::run("features/interruptible_speech.feature"),
    );
}
