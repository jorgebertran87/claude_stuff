use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::audio::{bytes_to_i16, i16_to_bytes, MicrophoneCapturer};
use voice_assistant::domain::ports::AudioCapturer;

#[derive(Debug, Default, World)]
pub struct AudioWorld {
    input_bytes: Vec<u8>,
    output_bytes: Vec<u8>,
    capturer: Option<MicrophoneCapturer>,
    sample_rate: u32,
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r"^raw audio bytes \[(.+)\]$")]
fn given_raw_bytes(world: &mut AudioWorld, csv: String) {
    world.input_bytes = csv.split(',')
        .map(|s| s.trim().parse::<u8>().unwrap())
        .collect();
}

#[given("a MicrophoneCapturer with no echo reference")]
fn given_capturer_no_ref(world: &mut AudioWorld) {
    world.capturer = Some(MicrophoneCapturer::new());
}

#[given(regex = r"^a MicrophoneCapturer with an echo reference at (\d+) Hz$")]
fn given_capturer_with_ref(world: &mut AudioWorld, ref_rate: u32) {
    let mut capturer = MicrophoneCapturer::new();
    // Generate a reference signal as raw bytes (sine-like pattern)
    let ref_samples: Vec<i16> = (0..200).map(|i| ((i * 100) % 32000) as i16 - 16000).collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    capturer.set_echo_reference(Some((ref_bytes, ref_rate, 2)));
    world.capturer = Some(capturer);
}

#[given(regex = r"^raw audio bytes of (\d+) samples at (\d+) Hz$")]
fn given_raw_samples(world: &mut AudioWorld, num_samples: usize, sample_rate: u32) {
    let samples: Vec<i16> = (0..num_samples).map(|i| ((i * 137) % 32000) as i16 - 16000).collect();
    world.input_bytes = i16_to_bytes(&samples);
    world.sample_rate = sample_rate;
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("bytes_to_i16 and i16_to_bytes are applied in sequence")]
fn when_roundtrip(world: &mut AudioWorld) {
    let samples = bytes_to_i16(&world.input_bytes);
    world.output_bytes = i16_to_bytes(&samples);
}

#[when("apply_echo_cancellation is called")]
fn when_echo_cancel(world: &mut AudioWorld) {
    let capturer = world.capturer.as_ref().unwrap();
    world.output_bytes = capturer.apply_echo_cancellation(
        &world.input_bytes,
        world.sample_rate,
        2,
    );
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the output bytes equal the input bytes")]
fn then_bytes_equal(world: &mut AudioWorld) {
    assert_eq!(world.output_bytes, world.input_bytes);
}

#[then("the output equals the raw input")]
fn then_passthrough(world: &mut AudioWorld) {
    assert_eq!(world.output_bytes, world.input_bytes);
}

#[then("the output length matches the input length")]
fn then_same_length(world: &mut AudioWorld) {
    assert_eq!(world.output_bytes.len(), world.input_bytes.len());
}

#[then("the output differs from the raw input")]
fn then_differs(world: &mut AudioWorld) {
    assert_ne!(world.output_bytes, world.input_bytes, "echo cancellation should modify the signal");
}

fn main() {
    futures::executor::block_on(
        AudioWorld::run("features/audio_integration.feature"),
    );
}
