use cucumber::{given, when, then, World};
use shaku::HasComponent;

use voice_assistant::container;
use voice_assistant::domain::ports::TextSynthesizer;

#[derive(World)]
pub struct TextSynthesizerWorld {
    synthesizer: Option<std::sync::Arc<dyn TextSynthesizer>>,
    output_bytes: Vec<u8>,
}

impl std::fmt::Debug for TextSynthesizerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextSynthesizerWorld")
            .field("output_len", &self.output_bytes.len())
            .finish()
    }
}

impl Default for TextSynthesizerWorld {
    fn default() -> Self {
        Self {
            synthesizer: None,
            output_bytes: Vec::new(),
        }
    }
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given("the TextSynthesizer is resolved from the DI container")]
fn given_synthesizer_resolved(world: &mut TextSynthesizerWorld) {
    let module = container::test_module();
    world.synthesizer = Some(HasComponent::<dyn TextSynthesizer>::resolve(&module));
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when(regex = r#"^synthesize_text is called with "(.*)"$"#)]
fn when_synthesize_text(world: &mut TextSynthesizerWorld, text: String) {
    let synth = world.synthesizer.as_ref().unwrap();
    world.output_bytes = synth.synthesize_text(&text);
}

#[when(regex = r#"^synthesize_alexa_spotify is called with '(.+)'$"#)]
fn when_synthesize_alexa_spotify(world: &mut TextSynthesizerWorld, text: String) {
    let synth = world.synthesizer.as_ref().unwrap();
    world.output_bytes = synth.synthesize_alexa_spotify(&text);
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the result is non-empty bytes")]
fn then_non_empty_bytes(world: &mut TextSynthesizerWorld) {
    assert!(!world.output_bytes.is_empty(), "synthesized bytes should not be empty");
}

#[then("the result is an empty byte vector")]
fn then_empty_bytes(world: &mut TextSynthesizerWorld) {
    assert!(world.output_bytes.is_empty(), "expected empty bytes, got {} bytes", world.output_bytes.len());
}

#[then("the bytes start with a valid MP3 header")]
fn then_mp3_header(world: &mut TextSynthesizerWorld) {
    assert!(world.output_bytes.len() >= 3, "too few bytes for MP3 header");
    let has_mp3_sync = world.output_bytes[0] == 0xFF && (world.output_bytes[1] & 0xE0) == 0xE0;
    let has_id3 = &world.output_bytes[0..3] == b"ID3";
    assert!(
        has_mp3_sync || has_id3,
        "expected MP3 sync word or ID3 tag, got: {:02X} {:02X} {:02X}",
        world.output_bytes[0],
        world.output_bytes[1],
        world.output_bytes[2],
    );
}

fn main() {
    futures::executor::block_on(TextSynthesizerWorld::run(
        "features/text_synthesizer.feature",
    ));
}
