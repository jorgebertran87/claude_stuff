use cucumber::{given, when, then, World};

use voice_assistant::infrastructure::shared::audio::{bytes_to_i16, i16_to_bytes};
use voice_assistant::infrastructure::audio_capturer::MicrophoneCapturer;
use voice_assistant::domain::ports::AudioCapturer;

#[derive(Default, World)]
pub struct AudioWorld {
    input_bytes: Vec<u8>,
    output_bytes: Vec<u8>,
    capturer: Option<MicrophoneCapturer>,
    sample_rate: u32,
}

impl std::fmt::Debug for AudioWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioWorld")
            .field("input_bytes_len", &self.input_bytes.len())
            .field("output_bytes_len", &self.output_bytes.len())
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
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
    let capturer = MicrophoneCapturer::new();
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

// The reference contains the same signal content as the mic but at a different sample rate.
// After correct resampling, the spectral subtraction should reduce the output amplitude.
#[given(regex = r"^a MicrophoneCapturer with an echo reference matching the mic signal at (\d+) Hz$")]
fn given_capturer_with_matching_ref(world: &mut AudioWorld, ref_rate: u32) {
    let capturer = MicrophoneCapturer::new();
    // Use the same deterministic formula as given_raw_samples (512 samples at 16000 Hz).
    // We store it at ref_rate so resample() will convert it back when applied.
    let ref_samples: Vec<i16> = (0..512).map(|i| ((i * 137) % 32000) as i16 - 16000).collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    capturer.set_echo_reference(Some((ref_bytes, ref_rate, 2)));
    world.capturer = Some(capturer);
}

#[given(regex = r"^a MicrophoneCapturer using the current audio bytes as its own echo reference at (\d+) Hz$")]
fn given_capturer_with_self_as_ref(world: &mut AudioWorld, ref_rate: u32) {
    let capturer = MicrophoneCapturer::new();
    capturer.set_echo_reference(Some((world.input_bytes.clone(), ref_rate, 2)));
    world.capturer = Some(capturer);
}

#[given(regex = r"^a MicrophoneCapturer with an echo reference at 8000 Hz matching (\d+) mic samples at 16000 Hz$")]
fn given_capturer_with_halfrate_matching_ref(world: &mut AudioWorld, num_samples: usize) {
    // Mic signal: (i * 137) % 32000 - 16000  (no wrap for num_samples <= 233)
    let mic: Vec<i16> = (0..num_samples)
        .map(|i| ((i * 137) % 32000) as i16 - 16000)
        .collect();
    // Reference: every other mic sample, stored at 8000 Hz (half rate, same duration)
    let ref_samples: Vec<i16> = (0..num_samples / 2)
        .map(|i| ((2 * i * 137) % 32000) as i16 - 16000)
        .collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    let capturer = MicrophoneCapturer::new();
    capturer.set_echo_reference(Some((ref_bytes, 8000, 2)));
    world.input_bytes = i16_to_bytes(&mic);
    world.sample_rate = 16000;
    world.capturer = Some(capturer);
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

#[then("the output RMS is less than the input RMS")]
fn then_rms_reduced(world: &mut AudioWorld) {
    let rms = |bytes: &[u8]| -> f64 {
        let samples = bytes_to_i16(bytes);
        if samples.is_empty() { return 0.0; }
        let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / samples.len() as f64).sqrt()
    };
    let in_rms = rms(&world.input_bytes);
    let out_rms = rms(&world.output_bytes);
    assert!(
        out_rms < in_rms,
        "output RMS {out_rms:.1} should be less than input RMS {in_rms:.1}",
    );
}

#[then(regex = r"^the output RMS is between (\d+) and (\d+) percent of the input RMS$")]
fn then_rms_between_pct(world: &mut AudioWorld, low_pct: u64, high_pct: u64) {
    let rms = |bytes: &[u8]| -> f64 {
        let samples = bytes_to_i16(bytes);
        if samples.is_empty() { return 0.0; }
        let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / samples.len() as f64).sqrt()
    };
    let in_rms  = rms(&world.input_bytes);
    let out_rms = rms(&world.output_bytes);
    let lo = in_rms * low_pct  as f64 / 100.0;
    let hi = in_rms * high_pct as f64 / 100.0;
    assert!(
        out_rms >= lo && out_rms <= hi,
        "output RMS {out_rms:.1} should be between {low_pct}% ({lo:.1}) \
         and {high_pct}% ({hi:.1}) of input RMS {in_rms:.1}",
    );
}

#[then(regex = r"^the output RMS is less than (\d+) percent of the input RMS$")]
fn then_rms_less_than_pct(world: &mut AudioWorld, pct: u64) {
    let rms = |bytes: &[u8]| -> f64 {
        let samples = bytes_to_i16(bytes);
        if samples.is_empty() { return 0.0; }
        let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / samples.len() as f64).sqrt()
    };
    let in_rms  = rms(&world.input_bytes);
    let out_rms = rms(&world.output_bytes);
    let threshold = in_rms * pct as f64 / 100.0;
    assert!(
        out_rms < threshold,
        "output RMS {out_rms:.1} should be less than {pct}% ({threshold:.1}) \
         of input RMS {in_rms:.1}",
    );
}

fn main() {
    futures::executor::block_on(
        AudioWorld::run("features/audio_integration.feature"),
    );
}
