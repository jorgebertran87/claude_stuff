Feature: Speech processing infrastructure integration
  As the system
  I want to denoise audio and cancel echoes using spectral subtraction
  So that voice input is cleaner before transcription

  Scenario: Denoise preserves sample count
    Given an audio signal of 1024 samples
    When denoise is applied with prop_decrease 0.5
    Then the output has 1024 samples

  Scenario: Cancel echo with identical signals produces near-silence
    Given an audio signal of 1024 samples
    When cancel_echo is applied with the same signal as reference at prop_decrease 1.0
    Then the output RMS is less than 1

  Scenario: Cancel echo with empty input returns empty output
    Given an empty audio signal
    When cancel_echo is applied with an empty reference at prop_decrease 0.5
    Then the output is empty

  Scenario: Denoise reduces amplitude
    Given an audio signal of 1024 samples
    When denoise is applied with prop_decrease 0.8
    Then the output RMS is less than or equal to the input RMS
