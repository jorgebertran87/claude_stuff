use cucumber::{given, when, then, World};
use shaku::HasComponent;

use voice_assistant::container;
use voice_assistant::domain::model::Language;
use voice_assistant::domain::ports::AudioSpeaker;

#[derive(World)]
pub struct AudioSpeakerWorld {
    speaker: Option<std::sync::Arc<dyn AudioSpeaker>>,
    echo_ref: Option<Option<(Vec<u8>, u32, u16)>>,
    language: String,
}

impl std::fmt::Debug for AudioSpeakerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSpeakerWorld")
            .field("language", &self.language)
            .finish()
    }
}

impl Default for AudioSpeakerWorld {
    fn default() -> Self {
        Self {
            speaker: None,
            echo_ref: None,
            language: String::new(),
        }
    }
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given("the AudioSpeaker is resolved from the DI container")]
fn given_speaker_resolved(world: &mut AudioSpeakerWorld) {
    let module = container::test_module();
    world.speaker = Some(HasComponent::<dyn AudioSpeaker>::resolve(&module));
}

#[given(regex = r#"^the language is "([^"]+)"$"#)]
fn given_language(world: &mut AudioSpeakerWorld, lang: String) {
    world.language = lang;
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("the speaker is stopped")]
fn when_stop(world: &mut AudioSpeakerWorld) {
    world.speaker.as_ref().unwrap().stop();
}

#[when("the echo reference is requested")]
fn when_get_echo_ref(world: &mut AudioSpeakerWorld) {
    world.echo_ref = Some(world.speaker.as_ref().unwrap().get_echo_reference());
}

#[when("the speaker beeps")]
fn when_beep(world: &mut AudioSpeakerWorld) {
    world.speaker.as_ref().unwrap().beep();
}

#[when(regex = r#"^the speaker speaks "(.+)"$"#)]
fn when_speak(world: &mut AudioSpeakerWorld, text: String) {
    let lang = Language::new(&world.language).unwrap();
    world.speaker.as_ref().unwrap().speak(&text, &lang, None);
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the speaker is available")]
fn then_speaker_available(world: &mut AudioSpeakerWorld) {
    assert!(world.speaker.is_some(), "AudioSpeaker should resolve from container");
}

#[then("no panic occurs")]
fn then_no_panic(_world: &mut AudioSpeakerWorld) {
    // If we got here, no panic occurred in the When steps
}

#[then("the result is None")]
fn then_echo_ref_none(world: &mut AudioSpeakerWorld) {
    assert!(world.echo_ref.as_ref().unwrap().is_none(), "expected None echo reference");
}

fn main() {
    futures::executor::block_on(AudioSpeakerWorld::run(
        "features/audio_speaker.feature",
    ));
}
