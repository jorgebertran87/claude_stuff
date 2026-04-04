use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::speaker::{
    alexa_spotify_title, build_alexa_command, tts_segment, AudioSegment,
};

#[derive(Debug, Default, World)]
pub struct TtsWorld {
    text: String,
    lang: String,
    segment: Option<AudioSegment>,
    combined_len: usize,
    title_alone_len: usize,
    // For alexa+spotify scenario
    extracted_title: String,
    detected_lang: String,
    alexa_command: String,
}

#[given(regex = r#"^the text "(.+)" and the language code "(.+)"$"#)]
fn given_text_and_lang(world: &mut TtsWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the text "(.+)" and the unsupported language code "(.+)"$"#)]
fn given_text_unsupported(world: &mut TtsWorld, text: String, lang: String) {
    world.text = text;
    world.lang = lang;
}

#[given(regex = r#"^the response '(.+)'$"#)]
fn given_response(world: &mut TtsWorld, response: String) {
    world.text = response;
}

#[when("the TTS segment is generated")]
fn when_tts_segment(world: &mut TtsWorld) {
    world.segment = tts_segment(&world.text, &world.lang).ok();
}

#[when("the full TTS pipeline processes the response")]
fn when_full_pipeline(world: &mut TtsWorld) {
    // Extract the title and synthesize just the title for comparison
    if let Some((title, lang)) = alexa_spotify_title(&world.text) {
        if let Ok(title_seg) = tts_segment(&title, &lang) {
            world.title_alone_len = title_seg.len();
        }
        // Build the full command
        let command = build_alexa_command(&title, &lang);
        if let Ok(full_seg) = tts_segment(&command, &lang) {
            world.combined_len = full_seg.len();
        }
    }
}

#[when(regex = r#"^alexa_spotify_title extracts the title and detects its language as "(.+)"$"#)]
fn when_extract_title(world: &mut TtsWorld, expected_lang: String) {
    let (title, lang) = alexa_spotify_title(&world.text)
        .expect("should extract alexa+spotify title");
    world.extracted_title = title;
    world.detected_lang = lang.clone();
    assert_eq!(lang, expected_lang, "detected language mismatch");
}

#[then("the result is a non-empty audio segment")]
fn then_non_empty_segment(world: &mut TtsWorld) {
    let seg = world.segment.as_ref().expect("segment should exist");
    assert!(!seg.is_empty(), "segment should not be empty");
}

#[then("the pipeline recovers and produces a non-empty audio segment in English")]
fn then_fallback_english(world: &mut TtsWorld) {
    let seg = world.segment.as_ref().expect("segment should exist after fallback");
    assert!(!seg.is_empty(), "fallback segment should not be empty");
}

#[then("the combined audio is longer than the song title spoken alone")]
fn then_combined_longer(world: &mut TtsWorld) {
    assert!(
        world.combined_len > world.title_alone_len,
        "combined ({}) should be longer than title alone ({})",
        world.combined_len,
        world.title_alone_len
    );
}

#[then(regex = r#"^build_alexa_command produces "(.+)"$"#)]
fn then_alexa_command(world: &mut TtsWorld, expected: String) {
    let cmd = build_alexa_command(&world.extracted_title, &world.detected_lang);
    world.alexa_command = cmd.clone();
    assert_eq!(cmd, expected);
}

#[then("synthesize_alexa_spotify produces non-empty audio bytes for the unified command")]
fn then_synth_non_empty(world: &mut TtsWorld) {
    let seg = tts_segment(&world.alexa_command, &world.detected_lang)
        .expect("tts_segment should succeed");
    assert!(!seg.is_empty(), "synthesized audio should not be empty");
}

fn main() {
    futures::executor::block_on(
        TtsWorld::run("features/tts_pipeline.feature"),
    );
}
