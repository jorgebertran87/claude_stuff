use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::tts::gtts_text_synthesizer::synthesize_text;
use voice_assistant::infrastructure::tts::speaker_utils::{alexa_spotify_title, build_alexa_command};
use voice_assistant::infrastructure::tts::engine::{tts_segment, AudioSegment};

#[derive(Debug, Default, World)]
pub struct SpeakerWorld {
    text: String,
    lang: String,
    segment: Option<AudioSegment>,
    synth_bytes: Vec<u8>,
    combined_len: usize,
    title_alone_len: usize,
    extracted_title: String,
    detected_lang: String,
    alexa_command: String,
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r#"^the text "([^"]+)" and the language code "([^"]+)"$"#)]
fn given_text_and_lang(world: &mut SpeakerWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the text "([^"]+)" and the unsupported language code "([^"]+)"$"#)]
fn given_text_unsupported(world: &mut SpeakerWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the text "([^"]+)"$"#)]
fn given_text(world: &mut SpeakerWorld, text: String) {
    world.text = text;
}

#[given(regex = r#"^the response '(.+)'$"#)]
fn given_response(world: &mut SpeakerWorld, response: String) {
    world.text = response;
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("the TTS segment is generated")]
fn when_tts_segment(world: &mut SpeakerWorld) {
    world.segment = tts_segment(&world.text, &world.lang).ok();
}

#[when("tts_segment calls piper_synthesize")]
fn when_real_tts(world: &mut SpeakerWorld) {
    world.segment = tts_segment(&world.text, &world.lang).ok();
}

#[when("synthesize_text is called")]
fn when_synthesize(world: &mut SpeakerWorld) {
    world.synth_bytes = synthesize_text(&world.text);
}

#[when("the full TTS pipeline processes the response")]
fn when_full_pipeline(world: &mut SpeakerWorld) {
    if let Some((title, lang)) = alexa_spotify_title(&world.text) {
        if let Ok(title_seg) = tts_segment(&title, &lang) {
            world.title_alone_len = title_seg.len();
        }
        let command = build_alexa_command(&title, &lang);
        if let Ok(full_seg) = tts_segment(&command, &lang) {
            world.combined_len = full_seg.len();
        }
    }
}

#[when(regex = r#"^alexa_spotify_title extracts the title and detects its language as "(.+)"$"#)]
fn when_extract_title(world: &mut SpeakerWorld, expected_lang: String) {
    let (title, lang) = alexa_spotify_title(&world.text)
        .expect("should extract alexa+spotify title");
    world.extracted_title = title;
    world.detected_lang = lang.clone();
    assert_eq!(lang, expected_lang, "detected language mismatch");
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the result is a non-empty audio segment")]
fn then_non_empty_segment(world: &mut SpeakerWorld) {
    let seg = world.segment.as_ref().expect("segment should exist");
    assert!(!seg.is_empty(), "segment should not be empty");
}

#[then("the pipeline recovers and produces a non-empty audio segment in English")]
fn then_fallback_english(world: &mut SpeakerWorld) {
    let seg = world.segment.as_ref().expect("segment should exist after fallback");
    assert!(!seg.is_empty(), "fallback segment should not be empty");
}

#[then("the result is non-empty bytes")]
fn then_non_empty_bytes(world: &mut SpeakerWorld) {
    assert!(!world.synth_bytes.is_empty(), "synthesized bytes should not be empty");
}

#[then("the bytes start with a valid MP3 header")]
fn then_mp3_header(world: &mut SpeakerWorld) {
    assert!(world.synth_bytes.len() >= 3, "too few bytes for MP3 header");
    let has_mp3_sync = world.synth_bytes[0] == 0xFF && (world.synth_bytes[1] & 0xE0) == 0xE0;
    let has_id3 = &world.synth_bytes[0..3] == b"ID3";
    assert!(
        has_mp3_sync || has_id3,
        "expected MP3 sync word or ID3 tag, got: {:02X} {:02X} {:02X}",
        world.synth_bytes[0],
        world.synth_bytes[1],
        world.synth_bytes[2],
    );
}

#[then("the combined audio is longer than the song title spoken alone")]
fn then_combined_longer(world: &mut SpeakerWorld) {
    assert!(
        world.combined_len > world.title_alone_len,
        "combined ({}) should be longer than title alone ({})",
        world.combined_len,
        world.title_alone_len
    );
}

#[then(regex = r#"^build_alexa_command produces "(.+)"$"#)]
fn then_alexa_command(world: &mut SpeakerWorld, expected: String) {
    let cmd = build_alexa_command(&world.extracted_title, &world.detected_lang);
    world.alexa_command = cmd.clone();
    assert_eq!(cmd, expected);
}

#[then("synthesize_alexa_spotify produces non-empty audio bytes for the unified command")]
fn then_synth_non_empty(world: &mut SpeakerWorld) {
    let seg = tts_segment(&world.alexa_command, &world.detected_lang)
        .expect("tts_segment should succeed");
    assert!(!seg.is_empty(), "synthesized audio should not be empty");
}

fn main() {
    futures::executor::block_on(
        SpeakerWorld::run("features/speaker_integration.feature"),
    );
}
