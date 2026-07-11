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
