Feature: AudioCapturer port integration
  As the system
  I want to capture and process audio through the AudioCapturer port
  So that microphone input is clean and correctly formatted

  Background:
    Given the AudioCapturer is resolved from the DI container

  # ── Port-level scenarios (through container, no hardware needed) ────────────

  Scenario: AudioCapturer resolves from the DI container
    Then the capturer is available

  Scenario: Setting and clearing echo reference
    When an echo reference is set
    And the echo reference is cleared
    Then no panic occurs

  Scenario: Mute and unmute
    When the capturer is muted
    And the capturer is unmuted
    Then no panic occurs

  Scenario: Calibrate
    When the capturer calibrates for 1.0 seconds
    Then no panic occurs

  # ── Signal processing utilities (used internally by the capturer) ───────────

  Scenario: bytes to i16 and back produces identical bytes
    Given raw audio bytes [0, 1, 255, 127]
    When bytes_to_i16 and i16_to_bytes are applied in sequence
    Then the output bytes equal the input bytes

  Scenario: Denoise preserves sample count
    Given an audio signal of 1024 samples
    When denoise is applied with prop_decrease 0.5
    Then the output has 1024 samples

  Scenario: Denoise reduces amplitude
    Given an audio signal of 1024 samples
    When denoise is applied with prop_decrease 0.8
    Then the output RMS is less than or equal to the input RMS

  Scenario: Cancel echo with identical signals produces near-silence
    Given an audio signal of 1024 samples
    When cancel_echo is applied with the same signal as reference at prop_decrease 1.0
    Then the output RMS is less than 1

  Scenario: Cancel echo with empty input returns empty output
    Given an empty audio signal
    When cancel_echo is applied with an empty reference at prop_decrease 0.5
    Then the output is empty

  Scenario: Cancel echo reduces amplitude
    Given an audio signal of 1024 samples
    When cancel_echo is applied with the same signal as reference at prop_decrease 0.5
    Then the output RMS is less than or equal to the input RMS

  # ── Echo cancellation (through the AudioCapturer trait) ─────────────────────

  Scenario: Echo cancellation without reference passes through
    Given the AudioCapturer is resolved from the DI container
    And raw audio bytes [0, 1, 2, 3, 4, 5]
    When echo cancellation is applied at 16000 Hz
    Then the output bytes equal the input bytes

  Scenario: Echo cancellation with reference modifies signal
    Given the AudioCapturer is resolved from the DI container
    And an echo reference of 200 samples at 16000 Hz
    And an audio signal of 200 samples
    When echo cancellation is applied at 16000 Hz to the signal bytes
    Then the output bytes are not empty
    And the output bytes differ from the input bytes

  Scenario: Echo cancellation resamples when reference rate differs
    Given the AudioCapturer is resolved from the DI container
    And an echo reference of 200 samples at 8000 Hz
    And an audio signal of 200 samples
    When echo cancellation is applied at 16000 Hz to the signal bytes
    And echo cancellation is applied at 16000 Hz with reference at 16000 Hz stored as alternative
    Then the output bytes differ from the alternative output bytes

  # ── WAV encoding (shared audio utility used by the capturer) ────────────────

  Scenario: WAV encoding produces a complete header
    Given audio samples [1, -2, 3, -4]
    When the samples are encoded as WAV at 16000 Hz
    Then the WAV output is 52 bytes long
    And the WAV output starts with the RIFF and WAVE magics
    And the WAV header declares RIFF size 44, byte rate 32000 and data size 8
    And the WAV data section contains the samples

  Scenario: WAV encoding of no samples yields only the header
    Given an empty audio signal
    When the samples are encoded as WAV at 16000 Hz
    Then the WAV output is 44 bytes long
    And the WAV header declares RIFF size 36, byte rate 32000 and data size 0

  # ── Speech accumulation (voice-activity detection used by the capturer) ─────
  # Amplitudes are chosen as exact powers of two (0.5 = 16384/32768) so RMS
  # boundary comparisons against the thresholds are exact.

  Scenario: Quiet audio before voice onset is discarded
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.0625 arrives at 0 ms
    Then the accumulator keeps listening
    When the accumulation finishes
    Then no speech is produced

  Scenario: Voice onset starts accumulation
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 0 ms
    Then the accumulator keeps listening
    When the accumulation finishes
    Then the accumulated speech has 100 samples

  Scenario: A chunk at exactly the voice threshold does not start accumulation
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.25 arrives at 0 ms
    And the accumulation finishes
    Then no speech is produced

  Scenario: Sustained silence after speech stops the capture
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 0 ms
    And a chunk of 100 samples with amplitude 0.0 arrives at 50 ms
    Then the accumulator keeps listening
    When a chunk of 100 samples with amplitude 0.0 arrives at 100 ms
    Then the accumulator stops
    When the accumulation finishes
    Then the accumulated speech has 300 samples

  Scenario: A chunk at exactly the silence threshold keeps the capture alive
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 0 ms
    And a chunk of 100 samples with amplitude 0.125 arrives at 50 ms
    And a chunk of 100 samples with amplitude 0.125 arrives at 100 ms
    And a chunk of 100 samples with amplitude 0.125 arrives at 150 ms
    Then the accumulator keeps listening

  Scenario: Voice resuming resets the silence counter
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 0 ms
    And a chunk of 100 samples with amplitude 0.0 arrives at 50 ms
    Then the accumulator keeps listening
    When a chunk of 100 samples with amplitude 0.5 arrives at 100 ms
    And a chunk of 100 samples with amplitude 0.0 arrives at 150 ms
    Then the accumulator keeps listening
    When a chunk of 100 samples with amplitude 0.0 arrives at 200 ms
    Then the accumulator stops

  Scenario: Timeouts during speech count toward the pause
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 0 ms
    And a timeout elapses at 50 ms
    Then the accumulator keeps listening
    When a timeout elapses at 100 ms
    Then the accumulator stops
    When the accumulation finishes
    Then the accumulated speech has 100 samples

  Scenario: Timeout without any voice produces no speech
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a timeout elapses at 500 ms
    Then the accumulator keeps listening
    When a timeout elapses at 1000 ms
    Then the accumulator stops
    When the accumulation finishes
    Then no speech is produced

  Scenario: Capture stops at the maximum duration
    Given a speech accumulator with voice threshold 0.25, silence threshold 0.125, pause 100 ms, timeout 1000 ms and max duration 3000 ms
    When a chunk of 100 samples with amplitude 0.5 arrives at 2950 ms
    Then the accumulator keeps listening
    When a chunk of 100 samples with amplitude 0.5 arrives at 3000 ms
    Then the accumulator stops
    When a chunk of 100 samples with amplitude 0.5 arrives at 3050 ms
    Then the accumulator stops
    When the accumulation finishes
    Then the accumulated speech has 100 samples

  # ── Capture from virtual microphone ─────────────────────────────────────────
  # NOTE: The capture scenario requires snd-aloop on the host but is currently
  # disabled due to ALSA/cpal config compatibility. Exactly 6 mutants remain
  # hardware-bound and unkillable without real audio flowing through cpal:
  # 5x `record_loop -> Some(vec![...])/None` whole-function replacements and
  # 1x `capture -> None`. All other record_loop/encode_wav logic is covered by
  # the speech-accumulation and WAV-encoding scenarios above.
  #
  # Scenario: Capture audio from virtual microphone
  #   Given the AudioCapturer is resolved from the DI container
  #   And a 440 Hz test tone is playing
  #   When the capturer records for up to 3 seconds
  #   Then a non-empty audio capture is produced
  #   And the audio capture has sample rate 16000
