Feature: Google Speech recognition integration
  As the system
  I want to transcribe real audio files using the Google Speech HTTP API
  So that voice input is converted to text without any stub

  Scenario: A valid WAV file produces a non-empty transcription
    Given the audio file "test.wav" at 16000 Hz mono 16-bit
    And the language is "es-ES"
    When GoogleTranscriber transcribes the audio
    Then the result is a non-empty string

  Scenario: Transcribing with language "en-US" returns a non-empty result
    Given the audio file "test.wav" at 16000 Hz mono 16-bit
    And the language is "en-US"
    When GoogleTranscriber transcribes the audio
    Then the result is a non-empty string

  Scenario: An empty audio buffer returns no transcription
    Given an AudioCapture with zero bytes of audio at 16000 Hz
    And the language is "es-ES"
    When GoogleTranscriber transcribes the audio
    Then the result is None

  Scenario: A WAV file containing only the WAV header returns no transcription
    Given an AudioCapture with only the 44-byte WAV header at 16000 Hz
    And the language is "es-ES"
    When GoogleTranscriber transcribes the audio
    Then the result is None
