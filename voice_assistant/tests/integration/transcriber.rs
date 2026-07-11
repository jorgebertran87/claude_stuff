use cucumber::{given, when, then, World};
use shaku::HasComponent;

use voice_assistant::container;
use voice_assistant::domain::model::{AudioCapture, Language};
use voice_assistant::domain::ports::Transcriber;

#[derive(World)]
pub struct TranscriberWorld {
    transcriber: Option<std::sync::Arc<dyn Transcriber>>,
    audio: Option<AudioCapture>,
    language: String,
    result: Option<Option<String>>,
}

impl std::fmt::Debug for TranscriberWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriberWorld")
            .field("language", &self.language)
            .field("result", &self.result)
            .finish()
    }
}

impl Default for TranscriberWorld {
    fn default() -> Self {
        Self {
            transcriber: None,
            audio: None,
            language: String::new(),
            result: None,
        }
    }
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given("the Transcriber is resolved from the DI container")]
fn given_transcriber_resolved(world: &mut TranscriberWorld) {
    let module = container::test_module();
    world.transcriber = Some(HasComponent::<dyn Transcriber>::resolve(&module));
}

#[given(regex = r#"^the audio file "([^"]+)" at (\d+) Hz mono 16-bit$"#)]
fn given_audio_file(world: &mut TranscriberWorld, filename: String, sample_rate: u32) {
    let bytes = std::fs::read(format!("tests/files/{filename}"))
        .or_else(|_| std::fs::read(&filename))
        .unwrap_or_else(|_| panic!("Cannot read audio file: {filename}"));
    world.audio = Some(AudioCapture::new(bytes, sample_rate, 2));
}

#[given(regex = r#"^the language is "([^"]+)"$"#)]
fn given_language(world: &mut TranscriberWorld, lang: String) {
    world.language = lang;
}

#[given(regex = r"^an AudioCapture with zero bytes of audio at (\d+) Hz$")]
fn given_zero_bytes(world: &mut TranscriberWorld, sample_rate: u32) {
    world.audio = Some(AudioCapture::new(vec![], sample_rate, 2));
}

#[given(regex = r"^an AudioCapture with only the 44-byte WAV header at (\d+) Hz$")]
fn given_wav_header_only(world: &mut TranscriberWorld, sample_rate: u32) {
    let mut header = vec![0u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    world.audio = Some(AudioCapture::new(header, sample_rate, 2));
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("the Transcriber transcribes the audio")]
fn when_transcribe(world: &mut TranscriberWorld) {
    let transcriber = world.transcriber.as_ref().unwrap();
    let lang = Language::new(&world.language).unwrap();
    let audio = world.audio.as_ref().unwrap();
    world.result = Some(transcriber.transcribe(audio, &lang));
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the result is a non-empty string")]
fn then_non_empty(world: &mut TranscriberWorld) {
    let r = world.result.as_ref().unwrap();
    assert!(r.is_some(), "expected Some(string), got None");
    let text = r.as_ref().unwrap();
    assert!(!text.is_empty(), "transcription should not be empty");
}

#[then("the result is None")]
fn then_none(world: &mut TranscriberWorld) {
    let r = world.result.as_ref().unwrap();
    assert!(r.is_none(), "expected None, got {:?}", r);
}

#[then("the result contains at least one space")]
fn then_contains_space(world: &mut TranscriberWorld) {
    let r = world.result.as_ref().unwrap();
    let text = r.as_ref().expect("expected Some(string) for space check");
    assert!(
        text.contains(' '),
        "transcription should contain at least one space, got: [{text}]"
    );
}

#[then("the result is not the sentinel value \"xyzzy\"")]
fn then_not_sentinel(world: &mut TranscriberWorld) {
    let r = world.result.as_ref().unwrap();
    assert_ne!(r, &Some("xyzzy".to_string()), "transcription should not be the sentinel value");
}

#[then("the result does not contain JSON artifacts")]
fn then_no_json_artifacts(world: &mut TranscriberWorld) {
    let r = world.result.as_ref().unwrap();
    if let Some(text) = r {
        assert!(
            !text.contains('"') && !text.contains('{') && !text.contains('}'),
            "transcription contains JSON artifacts: {:?}",
            text
        );
    }
}

fn main() {
    futures::executor::block_on(TranscriberWorld::run(
        "features/transcriber.feature",
    ));
}
