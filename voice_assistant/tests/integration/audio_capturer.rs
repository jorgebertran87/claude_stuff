use std::process::{Command, Stdio};
use std::time::Duration;

use cucumber::{given, when, then, World};
use shaku::HasComponent;

use voice_assistant::container;
use voice_assistant::domain::model::AudioCapture;
use voice_assistant::domain::ports::AudioCapturer;
use voice_assistant::infrastructure::shared::audio::{
    bytes_to_i16, cancel_echo, denoise, encode_wav, i16_to_bytes,
};
use voice_assistant::infrastructure::shared::vad::{
    CaptureDecision, CaptureEvent, SpeechAccumulator,
};

#[derive(World)]
pub struct AudioCapturerWorld {
    capturer: Option<std::sync::Arc<dyn AudioCapturer>>,
    input_samples: Vec<i16>,
    input_bytes: Vec<u8>,
    output_samples: Vec<i16>,
    output_bytes: Vec<u8>,
    alternative_output_bytes: Vec<u8>,
    input_rms: f64,
    sample_rate: u32,
    capture_result: Option<AudioCapture>,
    accumulator: Option<SpeechAccumulator>,
    last_decision: Option<CaptureDecision>,
    speech: Option<Vec<i16>>,
    wav_bytes: Vec<u8>,
}

impl std::fmt::Debug for AudioCapturerWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioCapturerWorld")
            .field("input_samples_len", &self.input_samples.len())
            .field("input_bytes_len", &self.input_bytes.len())
            .field("output_samples_len", &self.output_samples.len())
            .field("output_bytes_len", &self.output_bytes.len())
            .field("alternative_output_bytes_len", &self.alternative_output_bytes.len())
            .field("sample_rate", &self.sample_rate)
            .field("capture_is_some", &self.capture_result.is_some())
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
            alternative_output_bytes: Vec::new(),
            input_rms: 0.0,
            sample_rate: 16000,
            capture_result: None,
            accumulator: None,
            last_decision: None,
            speech: None,
            wav_bytes: Vec::new(),
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

#[given(regex = r"^an echo reference of (\d+) samples at (\d+) Hz$")]
fn given_echo_ref(world: &mut AudioCapturerWorld, n: usize, rate: u32) {
    let capturer = world.capturer.as_ref().unwrap();
    let ref_samples: Vec<i16> = (0..n)
        .map(|i| ((i as f64 * 0.1).sin() * 20000.0) as i16)
        .collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    capturer.set_echo_reference(Some((ref_bytes, rate, 2)));
    world.sample_rate = rate;
}

#[given("a 440 Hz test tone is playing")]
fn given_test_tone(_world: &mut AudioCapturerWorld) {
    // Route ALSA default PCM → loopback capture via plug (format conversion).
    // Use fully-nested syntax to avoid ALSA config parsing issues.
    std::fs::write("/etc/asound.conf", "\
pcm.!default {\n    type plug\n    slave {\n        pcm {\n            type hw\n            card 1\n            device 0\n            subdevice 1\n        }\n    }\n}\n\
ctl.!default {\n    type hw\n    card 1\n}\n").ok();

    // Generate a 440 Hz sine wave test tone (loud enough to trigger VAD at 0.02).
    let gen = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "lavfi",
            "-i", "sine=frequency=440:duration=4",
            "-ar", "16000",
            "-ac", "1",
            "-sample_fmt", "s16",
            "/tmp/test_tone.wav",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !gen {
        panic!("ffmpeg failed to generate test tone");
    }
}

#[given(regex = r"^audio samples \[(.+)\]$")]
fn given_i16_samples(world: &mut AudioCapturerWorld, csv: String) {
    world.input_samples = csv
        .split(',')
        .map(|s| s.trim().parse::<i16>().unwrap())
        .collect();
}

#[given(regex = r"^a speech accumulator with voice threshold (.+), silence threshold (.+), pause (\d+) ms, timeout (\d+) ms and max duration (\d+) ms$")]
fn given_accumulator(
    world: &mut AudioCapturerWorld,
    vad_threshold: f64,
    silence_threshold: f64,
    pause_ms: u64,
    timeout_ms: u64,
    max_duration_ms: u64,
) {
    world.accumulator = Some(SpeechAccumulator::new(
        vad_threshold,
        silence_threshold,
        pause_ms,
        timeout_ms,
        max_duration_ms,
        50,
    ));
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

#[when(regex = r"^echo cancellation is applied at (\d+) Hz$")]
fn when_echo_cancel(world: &mut AudioCapturerWorld, rate: u32) {
    let capturer = world.capturer.as_ref().unwrap();
    world.output_bytes = capturer.apply_echo_cancellation(&world.input_bytes, rate, 2);
}

#[when(regex = r"^echo cancellation is applied at (\d+) Hz to the signal bytes$")]
fn when_echo_cancel_signal(world: &mut AudioCapturerWorld, rate: u32) {
    let capturer = world.capturer.as_ref().unwrap();
    let bytes = i16_to_bytes(&world.input_samples);
    world.input_bytes = bytes.clone();
    world.output_bytes = capturer.apply_echo_cancellation(&bytes, rate, 2);
}

#[when(regex = r"^echo cancellation is applied at (\d+) Hz with reference at (\d+) Hz stored as alternative$")]
fn when_echo_cancel_alt(world: &mut AudioCapturerWorld, mic_rate: u32, ref_rate: u32) {
    let capturer = world.capturer.as_ref().unwrap();
    // Set echo reference at the given rate
    let ref_samples: Vec<i16> = (0..200)
        .map(|i| ((i as f64 * 0.1).sin() * 20000.0) as i16)
        .collect();
    let ref_bytes = i16_to_bytes(&ref_samples);
    capturer.set_echo_reference(Some((ref_bytes, ref_rate, 2)));
    // Apply echo cancellation and store as alternative
    let bytes = i16_to_bytes(&world.input_samples);
    world.alternative_output_bytes = capturer.apply_echo_cancellation(&bytes, mic_rate, 2);
}

#[when("the capturer records for up to 3 seconds")]
fn when_capture(world: &mut AudioCapturerWorld) {
    let capturer = world.capturer.as_ref().unwrap().clone();

    // Start capture in a background thread so we can feed audio into the
    // loopback AFTER the capture stream is open.
    let handle = std::thread::spawn(move || {
        capturer.capture(Some(3000), Some(3000), Some(800))
    });

    // Give cpal time to open the capture stream
    std::thread::sleep(Duration::from_millis(300));

    // Now play the test tone — audio flows through the loopback while
    // the capture stream is already listening.
    let mut child = Command::new("ffmpeg")
        .args([
            "-i", "/tmp/test_tone.wav",
            "-f", "alsa",
            "hw:1,0,0",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("ffmpeg playback failed to start");

    world.capture_result = handle.join().expect("capture thread panicked");

    let _ = child.kill();
    let _ = child.wait();
}

#[when(regex = r"^the samples are encoded as WAV at (\d+) Hz$")]
fn when_encode_wav(world: &mut AudioCapturerWorld, rate: u32) {
    world.wav_bytes = encode_wav(&world.input_samples, rate);
}

#[when(regex = r"^a chunk of (\d+) samples with amplitude (.+) arrives at (\d+) ms$")]
fn when_chunk_arrives(world: &mut AudioCapturerWorld, n: usize, amplitude: f64, elapsed_ms: u64) {
    let value = (amplitude * 32768.0) as i16;
    let chunk = vec![value; n];
    let accumulator = world.accumulator.as_mut().unwrap();
    world.last_decision = Some(accumulator.on_event(CaptureEvent::Chunk(&chunk), elapsed_ms));
}

#[when(regex = r"^a timeout elapses at (\d+) ms$")]
fn when_timeout_elapses(world: &mut AudioCapturerWorld, elapsed_ms: u64) {
    let accumulator = world.accumulator.as_mut().unwrap();
    world.last_decision = Some(accumulator.on_event(CaptureEvent::Timeout, elapsed_ms));
}

#[when("the accumulation finishes")]
fn when_accumulation_finishes(world: &mut AudioCapturerWorld) {
    world.speech = world.accumulator.take().unwrap().finish();
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

#[then("the output bytes are not empty")]
fn then_bytes_not_empty(world: &mut AudioCapturerWorld) {
    assert!(!world.output_bytes.is_empty(), "output bytes should not be empty");
}

#[then("the output bytes differ from the input bytes")]
fn then_bytes_differ(world: &mut AudioCapturerWorld) {
    assert_ne!(world.output_bytes, world.input_bytes, "output bytes should differ from input");
}

#[then("the output bytes differ from the alternative output bytes")]
fn then_bytes_differ_alt(world: &mut AudioCapturerWorld) {
    assert_ne!(
        world.output_bytes, world.alternative_output_bytes,
        "output bytes should differ from alternative output bytes"
    );
}

#[then("a non-empty audio capture is produced")]
fn then_capture_produced(world: &mut AudioCapturerWorld) {
    let capture = world.capture_result.as_ref().expect("capture should be Some");
    assert!(!capture.raw.is_empty(), "capture raw data should not be empty");
    assert!(capture.raw.len() > 44, "capture should have more than just WAV header");
}

#[then(regex = r"^the audio capture has sample rate (\d+)$")]
fn then_capture_sample_rate(world: &mut AudioCapturerWorld, expected: u32) {
    let capture = world.capture_result.as_ref().expect("capture should be Some");
    assert_eq!(capture.sample_rate, expected, "sample rate mismatch");
}

#[then(regex = r"^the WAV output is (\d+) bytes long$")]
fn then_wav_len(world: &mut AudioCapturerWorld, expected: usize) {
    assert_eq!(world.wav_bytes.len(), expected, "WAV output length mismatch");
}

#[then("the WAV output starts with the RIFF and WAVE magics")]
fn then_wav_magics(world: &mut AudioCapturerWorld) {
    assert_eq!(&world.wav_bytes[0..4], b"RIFF", "missing RIFF magic");
    assert_eq!(&world.wav_bytes[8..12], b"WAVE", "missing WAVE magic");
}

#[then(regex = r"^the WAV header declares RIFF size (\d+), byte rate (\d+) and data size (\d+)$")]
fn then_wav_header_fields(
    world: &mut AudioCapturerWorld,
    riff_size: u32,
    byte_rate: u32,
    data_size: u32,
) {
    let field = |offset: usize| {
        u32::from_le_bytes(world.wav_bytes[offset..offset + 4].try_into().unwrap())
    };
    assert_eq!(field(4), riff_size, "RIFF size field mismatch");
    assert_eq!(field(28), byte_rate, "byte rate field mismatch");
    assert_eq!(field(40), data_size, "data size field mismatch");
}

#[then("the WAV data section contains the samples")]
fn then_wav_data_section(world: &mut AudioCapturerWorld) {
    let data = bytes_to_i16(&world.wav_bytes[44..]);
    assert_eq!(data, world.input_samples, "WAV data section mismatch");
}

#[then("the accumulator keeps listening")]
fn then_keeps_listening(world: &mut AudioCapturerWorld) {
    assert_eq!(world.last_decision, Some(CaptureDecision::Continue));
}

#[then("the accumulator stops")]
fn then_accumulator_stops(world: &mut AudioCapturerWorld) {
    assert_eq!(world.last_decision, Some(CaptureDecision::Stop));
}

#[then("no speech is produced")]
fn then_no_speech(world: &mut AudioCapturerWorld) {
    assert!(world.speech.is_none(), "expected no speech, got {:?} samples", world.speech.as_ref().map(Vec::len));
}

#[then(regex = r"^the accumulated speech has (\d+) samples$")]
fn then_speech_len(world: &mut AudioCapturerWorld, expected: usize) {
    let speech = world.speech.as_ref().expect("speech should be produced");
    assert_eq!(speech.len(), expected, "accumulated speech length mismatch");
}

fn main() {
    futures::executor::block_on(AudioCapturerWorld::run(
        "features/audio_capturer.feature",
    ));
}
