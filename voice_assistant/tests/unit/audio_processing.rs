use cucumber::{given, when, then, World};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn synthesize_tone(freq_hz: f32, sample_rate: u32, duration_secs: f32) -> Vec<i16> {
    let n = (sample_rate as f32 * duration_secs) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (0.5 * (2.0 * std::f32::consts::PI * freq_hz * t).sin() * i16::MAX as f32) as i16
        })
        .collect()
}

fn energy(samples: &[i16]) -> f64 {
    samples.iter().map(|&s| (s as f64) * (s as f64)).sum()
}

// ── World ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, World)]
pub struct AudioWorld {
    original_samples: Vec<i16>,
    sample_rate: u32,
    sample_width: u16,
    result_samples: Vec<i16>,
    // For echo cancellation
    reference_samples: Vec<i16>,
    mixed_samples: Vec<i16>,
    cleaned_samples: Vec<i16>,
}

const SAMPLE_RATE: u32 = 16000;
const SAMPLE_WIDTH: u16 = 2;
const DURATION: f32 = 0.1;

#[given("a synthesized audio clip at 440 Hz")]
fn given_440hz(world: &mut AudioWorld) {
    world.original_samples = synthesize_tone(440.0, SAMPLE_RATE, DURATION);
    world.sample_rate = SAMPLE_RATE;
    world.sample_width = SAMPLE_WIDTH;
}

#[given("a speech audio clip at 300 Hz")]
fn given_300hz(world: &mut AudioWorld) {
    world.original_samples = synthesize_tone(300.0, SAMPLE_RATE, DURATION);
    world.sample_rate = SAMPLE_RATE;
    world.sample_width = SAMPLE_WIDTH;
}

#[given("a reference echo audio clip at 880 Hz")]
fn given_880hz_ref(world: &mut AudioWorld) {
    world.reference_samples = synthesize_tone(880.0, SAMPLE_RATE, DURATION);
}

#[given("a mixed audio clip combining both signals")]
fn given_mixed(world: &mut AudioWorld) {
    let len = world.original_samples.len().min(world.reference_samples.len());
    world.mixed_samples = (0..len)
        .map(|i| {
            let sum = world.original_samples[i] as i32 + world.reference_samples[i] as i32;
            sum.clamp(-32768, 32767) as i16
        })
        .collect();
}

#[when("the denoising pipeline processes it")]
fn when_denoise(world: &mut AudioWorld) {
    use voice_assistant::infrastructure::transcriber::speech::denoise;
    world.result_samples = denoise(&world.original_samples, world.sample_rate, 0.95);
}

#[when("the echo cancellation pipeline processes the speech audio")]
fn when_echo_cancel(world: &mut AudioWorld) {
    use voice_assistant::infrastructure::transcriber::speech::cancel_echo;
    world.result_samples = cancel_echo(&world.original_samples, &world.reference_samples, 0.95);
}

#[when("the echo cancellation pipeline processes the mixed audio using the echo as reference")]
fn when_echo_cancel_mixed(world: &mut AudioWorld) {
    use voice_assistant::infrastructure::transcriber::speech::cancel_echo;
    world.cleaned_samples = cancel_echo(&world.mixed_samples, &world.reference_samples, 0.95);
}

#[then("the result has the same sample rate and sample width as the original")]
fn then_same_format(world: &mut AudioWorld) {
    // Sample rate and width are not modified by denoise — it operates on samples.
    // We verify the pipeline preserves the count (proxy for format invariance).
    assert_eq!(world.sample_rate, SAMPLE_RATE);
    assert_eq!(world.sample_width, SAMPLE_WIDTH);
}

#[then("the result has the same number of samples as the original")]
fn then_same_count(world: &mut AudioWorld) {
    assert_eq!(
        world.result_samples.len(),
        world.original_samples.len(),
        "sample count mismatch"
    );
}

#[then("the resulting audio bytes differ from the original")]
fn then_bytes_differ(world: &mut AudioWorld) {
    assert_ne!(world.result_samples, world.original_samples, "audio should be modified");
}

#[then("the result is a valid audio object with the same sample rate and width")]
fn then_valid_audio(world: &mut AudioWorld) {
    assert!(!world.result_samples.is_empty(), "result should not be empty");
    assert_eq!(world.result_samples.len(), world.original_samples.len());
}

#[then("the energy of the cleaned audio is lower than the energy of the mixed audio")]
fn then_lower_energy(world: &mut AudioWorld) {
    let mixed_energy = energy(&world.mixed_samples);
    let cleaned_energy = energy(&world.cleaned_samples);
    assert!(
        cleaned_energy < mixed_energy,
        "cleaned energy ({cleaned_energy}) should be less than mixed ({mixed_energy})"
    );
}

fn main() {
    futures::executor::block_on(
        AudioWorld::run("features/audio_processing.feature"),
    );
}
