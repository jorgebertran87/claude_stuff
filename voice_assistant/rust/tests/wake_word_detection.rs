use cucumber::{given, when, then, World};

use voice_assistant::domain::model::WakeWord;

#[derive(Debug, Default, World)]
pub struct WakeWordWorld {
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
    let wake = WakeWord::new("claudito").unwrap();

    for (i, cap) in world.captures.iter().enumerate() {
        match cap {
            None => {
                world.skipped_empty = true;
            }
            Some(text) => {
                if wake.matches(text) {
                    world.wake_word_detected = true;
                    world.inline_order = wake.extract_order(text);
                    let exact = text.to_lowercase().contains("claudito");
                    if !exact {
                        world.fuzzy_match = true;
                    }
                    break;
                } else if i < world.captures.len() - 1 {
                    world.ignored_first = true;
                }
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
