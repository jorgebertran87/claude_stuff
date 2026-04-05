Feature: Audio infrastructure integration
  As the system
  I want to convert audio formats and apply echo cancellation
  So that microphone input is clean and correctly formatted

  Scenario: bytes to i16 and back produces identical bytes
    Given raw audio bytes [0, 1, 255, 127]
    When bytes_to_i16 and i16_to_bytes are applied in sequence
    Then the output bytes equal the input bytes

  Scenario: Echo cancellation without reference returns raw bytes unchanged
    Given a MicrophoneCapturer with no echo reference
    And raw audio bytes of 200 samples at 16000 Hz
    When apply_echo_cancellation is called
    Then the output equals the raw input

  Scenario: Echo cancellation with same-rate reference produces different output
    Given a MicrophoneCapturer with an echo reference at 16000 Hz
    And raw audio bytes of 200 samples at 16000 Hz
    When apply_echo_cancellation is called
    Then the output length matches the input length
    And the output differs from the raw input

  Scenario: Echo cancellation with different-rate reference resamples and produces output
    Given a MicrophoneCapturer with an echo reference at 44100 Hz
    And raw audio bytes of 200 samples at 16000 Hz
    When apply_echo_cancellation is called
    Then the output length matches the input length
    And the output differs from the raw input

  Scenario: Echo cancellation with a same-content reference at different rate reduces amplitude
    Given a MicrophoneCapturer with an echo reference matching the mic signal at 44100 Hz
    And raw audio bytes of 512 samples at 16000 Hz
    When apply_echo_cancellation is called
    Then the output RMS is less than the input RMS

  Scenario: Same-rate self-cancellation achieves partial but not total suppression
    Given raw audio bytes of 160 samples at 16000 Hz
    And a MicrophoneCapturer using the current audio bytes as its own echo reference at 16000 Hz
    When apply_echo_cancellation is called
    Then the output RMS is between 2 and 10 percent of the input RMS

  Scenario: Echo cancellation with half-rate matching reference achieves strong amplitude reduction
    Given a MicrophoneCapturer with an echo reference at 8000 Hz matching 160 mic samples at 16000 Hz
    When apply_echo_cancellation is called
    Then the output RMS is between 2 and 20 percent of the input RMS

  Scenario: capture returns Some audio when recording writes sufficient data
    Given a fake rec that writes 100 bytes of audio to the capture file
    And a MicrophoneCapturer with no echo reference
    When capture is called
    Then the capture result contains audio sampled at 16000 Hz

  Scenario: capture returns None when the recording contains only a WAV header
    Given a fake rec that writes only the 44-byte WAV header to the capture file
    And a MicrophoneCapturer with no echo reference
    When capture is called
    Then the capture result is None

  Scenario: capture passes correct timing arguments to rec
    Given a fake rec that logs its arguments and writes 100 bytes of audio
    And a MicrophoneCapturer with no echo reference
    When capture is called with timeout_ms 3000 and pause_threshold_ms 2000
    Then rec was invoked with trim duration "3" and pause threshold "2.0"
