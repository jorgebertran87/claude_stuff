Feature: Audio processing pipeline
  As the system
  I want to clean microphone audio before transcription
  So that background noise and speaker echo do not interfere with recognition

  Scenario: Denoising preserves the audio format
    Given a synthesized audio clip at 440 Hz
    When the denoising pipeline processes it
    Then the result has the same sample rate and sample width as the original
    And the result has the same number of samples as the original

  Scenario: Denoising modifies the signal
    Given a synthesized audio clip at 440 Hz
    When the denoising pipeline processes it
    Then the resulting audio bytes differ from the original

  Scenario: Echo cancellation returns a valid audio object
    Given a speech audio clip at 300 Hz
    And a reference echo audio clip at 880 Hz
    When the echo cancellation pipeline processes the speech audio
    Then the result is a valid audio object with the same sample rate and width

  Scenario: Echo cancellation reduces the energy of the mixed signal
    Given a speech audio clip at 300 Hz
    And a reference echo audio clip at 880 Hz
    And a mixed audio clip combining both signals
    When the echo cancellation pipeline processes the mixed audio using the echo as reference
    Then the energy of the cleaned audio is lower than the energy of the mixed audio
