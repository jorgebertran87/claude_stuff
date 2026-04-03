use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::speaker::{synthesize_text, tts_segment};

#[derive(Debug, Default, World)]
pub struct TtsHttpWorld {
    text: String,
    lang: String,
    segment: Option<voice_assistant::infrastructure::speaker::AudioSegment>,
    synth_bytes: Vec<u8>,
}

#[given(regex = r#"^the text "(.+)" and the language code "(.+)"$"#)]
fn given_text_lang(world: &mut TtsHttpWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the text "(.+)" and the unsupported language code "(.+)"$"#)]
fn given_text_unsupported(world: &mut TtsHttpWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the text "(.+)"$"#)]
fn given_text(world: &mut TtsHttpWorld, text: String) {
    world.text = text;
}

#[when("tts_segment makes a real HTTP request")]
fn when_real_tts(world: &mut TtsHttpWorld) {
    world.segment = tts_segment(&world.text, &world.lang).ok();
}

#[when("synthesize_text is called")]
fn when_synthesize(world: &mut TtsHttpWorld) {
    world.synth_bytes = synthesize_text(&world.text);
}

#[then("the response is a non-empty AudioSegment")]
fn then_non_empty(world: &mut TtsHttpWorld) {
    let seg = world.segment.as_ref().expect("segment should exist");
    assert!(!seg.is_empty(), "segment should not be empty");
}

#[then("the result is non-empty bytes")]
fn then_non_empty_bytes(world: &mut TtsHttpWorld) {
    assert!(!world.synth_bytes.is_empty(), "synthesized bytes should not be empty");
}

#[then("the bytes start with a valid MP3 header")]
fn then_mp3_header(world: &mut TtsHttpWorld) {
    // MP3 frames start with sync bits 0xFF 0xFB (or 0xFF 0xE0..0xFF)
    // or ID3 tag starts with "ID3"
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

fn main() {
    futures::executor::block_on(
        TtsHttpWorld::run("features/tts_http_integration.feature"),
    );
}
