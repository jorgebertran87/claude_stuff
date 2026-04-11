use cucumber::{given, when, then, World};

use voice_assistant::domain::model::{AudioCapture, Language};
use voice_assistant::domain::ports::Transcriber;
use voice_assistant::infrastructure::transcriber::GoogleTranscriber;

#[derive(Debug, Default, World)]
pub struct SpeechWorld {
    audio: Option<AudioCapture>,
    language: String,
    result: Option<Option<String>>,
}

#[given(regex = r#"^the audio file "([^"]+)" at (\d+) Hz mono 16-bit$"#)]
fn given_audio_file(world: &mut SpeechWorld, filename: String, sample_rate: u32) {
    let bytes = std::fs::read(format!("tests/{filename}"))
        .or_else(|_| std::fs::read(&filename))
        .unwrap_or_else(|_| panic!("Cannot read audio file: {filename}"));
    world.audio = Some(AudioCapture::new(bytes, sample_rate, 2));
}

#[given(regex = r#"^the language is "([^"]+)"$"#)]
fn given_language(world: &mut SpeechWorld, lang: String) {
    world.language = lang;
}

#[given(regex = r"^an AudioCapture with zero bytes of audio at (\d+) Hz$")]
fn given_zero_bytes(world: &mut SpeechWorld, sample_rate: u32) {
    world.audio = Some(AudioCapture::new(vec![], sample_rate, 2));
}

#[given(regex = r"^an AudioCapture with only the 44-byte WAV header at (\d+) Hz$")]
fn given_wav_header_only(world: &mut SpeechWorld, sample_rate: u32) {
    let mut header = vec![0u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    world.audio = Some(AudioCapture::new(header, sample_rate, 2));
}

#[when("GoogleTranscriber transcribes the audio")]
fn when_transcribe(world: &mut SpeechWorld) {
    let transcriber = GoogleTranscriber;
    let lang = Language::new(&world.language).unwrap();
    let audio = world.audio.as_ref().unwrap();
    world.result = Some(transcriber.transcribe(audio, &lang));
}

#[then("the result is a non-empty string")]
fn then_non_empty(world: &mut SpeechWorld) {
    let r = world.result.as_ref().unwrap();
    assert!(r.is_some(), "expected Some(string), got None");
    assert!(!r.as_ref().unwrap().is_empty(), "transcription should not be empty");
}

#[then("the result is None")]
fn then_none(world: &mut SpeechWorld) {
    let r = world.result.as_ref().unwrap();
    assert!(r.is_none(), "expected None, got {:?}", r);
}

fn main() {
    futures::executor::block_on(
        SpeechWorld::run("features/transcriber_integration.feature"),
    );
}
