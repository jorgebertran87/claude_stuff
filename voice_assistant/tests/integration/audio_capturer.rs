use cucumber::{given, when, then, World};
use shaku::HasComponent;

use voice_assistant::container;
use voice_assistant::domain::ports::AudioCapturer;
use voice_assistant::infrastructure::shared::audio::{
    bytes_to_i16, cancel_echo, denoise, i16_to_bytes,
};

#[derive(World)]
pub struct AudioCapturerWorld {
    capturer: Option<std::sync::Arc<dyn AudioCapturer>>,
    input_samples: Vec<i16>,
    input_bytes: Vec<u8>,
    output_samples: Vec<i16>,
    output_bytes: Vec<u8>,
    input_rms: f64,
    sample_rate: u32,
}

impl std::fmt::Debug for AudioCapturerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioCapturerWorld")
            .field("input_samples_len", &self.input_samples.len())
            .field("input_bytes_len", &self.input_bytes.len())
            .field("output_samples_len", &self.output_samples.len())
            .field("output_bytes_len", &self.output_bytes.len())
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
}

impl Default for AudioCapturerWorld {
    fn default() -> Self {
        Self {
            capturer: None,
            input_samples: Vec::new(),
            input_bytes: Vec::new(),
            output_samples: Vec::new(),
            output_bytes: Vec::new(),
            input_rms: 0.0,
            sample_rate: 16000,
        }
    }
}

fn rms(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len() as f64).sqrt()
}

// ── Given steps ────────────────────────────────────────────────────────────────

#[given("the AudioCapturer is resolved from the DI container")]
fn given_capturer_resolved(world: &mut AudioCapturerWorld) {
    let module = container::test_module();
    world.capturer = Some(HasComponent::<dyn AudioCapturer>::resolve(&module));
}

#[given(regex = r"^raw audio bytes \[(.+)\]$")]
fn given_raw_bytes(world: &mut AudioCapturerWorld, csv: String) {
    world.input_bytes = csv
        .split(',')
        .map(|s| s.trim().parse::<u8>().unwrap())
        .collect();
}

#[given(regex = r"^an audio signal of (\d+) samples$")]
fn given_signal(world: &mut AudioCapturerWorld, n: usize) {
    world.input_samples = (0..n)
        .map(|i| ((i as f64 * 0.1).sin() * 10000.0) as i16)
        .collect();
    world.input_rms = rms(&world.input_samples);
}

#[given("an empty audio signal")]
fn given_empty(world: &mut AudioCapturerWorld) {
    world.input_samples = vec![];
}

// ── When steps ─────────────────────────────────────────────────────────────────

#[when("an echo reference is set")]
fn when_set_echo_ref(world: &mut AudioCapturerWorld) {
    let capturer = world.capturer.as_ref().unwrap();
    let ref_samples: Vec<i16> = (0..200).map(|i| ((i * 100) % 32000) as i16 - 16000).collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    capturer.set_echo_reference(Some((ref_bytes, 16000, 2)));
}

#[when("the echo reference is cleared")]
fn when_clear_echo_ref(world: &mut AudioCapturerWorld) {
    let capturer = world.capturer.as_ref().unwrap();
    capturer.set_echo_reference(None);
}

#[when("the capturer is muted")]
fn when_muted(world: &mut AudioCapturerWorld) {
    world.capturer.as_ref().unwrap().mute();
}

#[when("the capturer is unmuted")]
fn when_unmuted(world: &mut AudioCapturerWorld) {
    world.capturer.as_ref().unwrap().unmute();
}

#[when(regex = r"^the capturer calibrates for (.+) seconds$")]
fn when_calibrate(world: &mut AudioCapturerWorld, secs: f64) {
    world.capturer.as_ref().unwrap().calibrate(secs);
}

#[when("bytes_to_i16 and i16_to_bytes are applied in sequence")]
fn when_roundtrip(world: &mut AudioCapturerWorld) {
    let samples = bytes_to_i16(&world.input_bytes);
    world.output_bytes = i16_to_bytes(&samples);
}

#[when(regex = r"^denoise is applied with prop_decrease (.+)$")]
fn when_denoise(world: &mut AudioCapturerWorld, prop: f32) {
    world.output_samples = denoise(&world.input_samples, 16000, prop);
}

#[when(regex = r"^cancel_echo is applied with the same signal as reference at prop_decrease (.+)$")]
fn when_cancel_same(world: &mut AudioCapturerWorld, prop: f32) {
    let reference = world.input_samples.clone();
    world.output_samples = cancel_echo(&world.input_samples, &reference, prop);
}

#[when(regex = r"^cancel_echo is applied with an empty reference at prop_decrease (.+)$")]
fn when_cancel_empty(world: &mut AudioCapturerWorld, prop: f32) {
    world.output_samples = cancel_echo(&world.input_samples, &[], prop);
}

// ── Then steps ─────────────────────────────────────────────────────────────────

#[then("the capturer is available")]
fn then_capturer_available(world: &mut AudioCapturerWorld) {
    assert!(world.capturer.is_some(), "AudioCapturer should resolve from container");
}

#[then("no panic occurs")]
fn then_no_panic(_world: &mut AudioCapturerWorld) {
    // If we got here, no panic occurred in the When steps
}

#[then("the output bytes equal the input bytes")]
fn then_bytes_equal(world: &mut AudioCapturerWorld) {
    assert_eq!(world.output_bytes, world.input_bytes);
}

#[then(regex = r"^the output has (\d+) samples$")]
fn then_sample_count(world: &mut AudioCapturerWorld, expected: usize) {
    assert_eq!(world.output_samples.len(), expected);
}

#[then(regex = r"^the output RMS is less than (\d+)$")]
fn then_rms_below(world: &mut AudioCapturerWorld, threshold: f64) {
    let out_rms = rms(&world.output_samples);
    assert!(out_rms < threshold, "output RMS {out_rms} should be < {threshold}");
}

#[then("the output is empty")]
fn then_empty(world: &mut AudioCapturerWorld) {
    assert!(world.output_samples.is_empty());
}

#[then("the output RMS is less than or equal to the input RMS")]
fn then_rms_reduced(world: &mut AudioCapturerWorld) {
    let out_rms = rms(&world.output_samples);
    assert!(
        out_rms <= world.input_rms + 0.01,
        "output RMS {out_rms} should be <= input RMS {}",
        world.input_rms
    );
}

fn main() {
    futures::executor::block_on(AudioCapturerWorld::run(
        "features/audio_capturer.feature",
    ));
}
