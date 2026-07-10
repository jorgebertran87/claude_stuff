use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::transcriber::speech::{cancel_echo, denoise};

#[derive(Debug, Default, World)]
pub struct SpeechProcessingWorld {
    input_samples: Vec<i16>,
    reference_samples: Vec<i16>,
    output_samples: Vec<i16>,
    input_rms: f64,
}

fn rms(samples: &[i16]) -> f64 {
    if samples.is_empty() { return 0.0; }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len() as f64).sqrt()
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given(regex = r"^an audio signal of (\d+) samples$")]
fn given_signal(world: &mut SpeechProcessingWorld, n: usize) {
    // Generate a deterministic signal with meaningful amplitude
    world.input_samples = (0..n).map(|i| {
        ((i as f64 * 0.1).sin() * 10000.0) as i16
    }).collect();
    world.input_rms = rms(&world.input_samples);
}

#[given("an empty audio signal")]
fn given_empty(world: &mut SpeechProcessingWorld) {
    world.input_samples = vec![];
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when(regex = r"^denoise is applied with prop_decrease (.+)$")]
fn when_denoise(world: &mut SpeechProcessingWorld, prop: f32) {
    world.output_samples = denoise(&world.input_samples, 16000, prop);
}

#[when(regex = r"^cancel_echo is applied with the same signal as reference at prop_decrease (.+)$")]
fn when_cancel_same(world: &mut SpeechProcessingWorld, prop: f32) {
    world.reference_samples = world.input_samples.clone();
    world.output_samples = cancel_echo(&world.input_samples, &world.reference_samples, prop);
}

#[when(regex = r"^cancel_echo is applied with an empty reference at prop_decrease (.+)$")]
fn when_cancel_empty(world: &mut SpeechProcessingWorld, prop: f32) {
    world.output_samples = cancel_echo(&world.input_samples, &[], prop);
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then(regex = r"^the output has (\d+) samples$")]
fn then_sample_count(world: &mut SpeechProcessingWorld, expected: usize) {
    assert_eq!(world.output_samples.len(), expected);
}

#[then(regex = r"^the output RMS is less than (\d+)$")]
fn then_rms_below(world: &mut SpeechProcessingWorld, threshold: f64) {
    let out_rms = rms(&world.output_samples);
    assert!(out_rms < threshold, "output RMS {out_rms} should be < {threshold}");
}

#[then("the output is empty")]
fn then_empty(world: &mut SpeechProcessingWorld) {
    assert!(world.output_samples.is_empty());
}

#[then("the output RMS is less than or equal to the input RMS")]
fn then_rms_reduced(world: &mut SpeechProcessingWorld) {
    let out_rms = rms(&world.output_samples);
    assert!(
        out_rms <= world.input_rms + 0.01,
        "output RMS {out_rms} should be <= input RMS {}",
        world.input_rms
    );
}

fn main() {
    futures::executor::block_on(
        SpeechProcessingWorld::run("features/speech_integration.feature"),
    );
}
