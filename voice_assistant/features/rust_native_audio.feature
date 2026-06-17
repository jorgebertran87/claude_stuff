Feature: Rust-native audio capture and playback via cpal + rodio
  As a developer
  I want audio capture and playback to use pure-Rust libraries (cpal, rodio)
  So that we eliminate fragile CLI subprocess dependencies (sox rec, ffplay)
  and get programmatic control over audio I/O

  Scenario: Audio capture produces non-empty WAV bytes from a real device
    Given a working audio input device
    When the capturer records with a 3-second phrase limit
    Then it returns an AudioCapture with non-empty raw bytes
    And the sample rate is 16000 Hz
    And the sample width is 2 (16-bit)

  Scenario: Silence produces no capture when no voice is detected
    Given a working audio input device with no audible signal
    When the capturer records with a short phrase limit
    Then it returns None

  Scenario: Audio playback plays MP3 bytes through default output
    Given valid MP3 audio bytes
    When the player plays the bytes
    Then playback completes without error

  Scenario: Playback can be stopped mid-stream
    Given valid MP3 audio bytes of long duration
    When playback starts and stop is called immediately
    Then playback terminates without error

  Scenario: Beep generates a short audible tone
    When the speaker beeps
    Then a tone is played through the default output device
