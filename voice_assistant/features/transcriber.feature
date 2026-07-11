Feature: Transcriber port integration
  As the system
  I want to transcribe real audio files through the Transcriber port
  So that voice input is converted to text without any stub

  Scenario: A Spanish WAV file produces a non-empty transcription
    Given the Transcriber is resolved from the DI container
    And the audio file "test_speech_es.wav" at 16000 Hz mono 16-bit
    And the language is "es-ES"
    When the Transcriber transcribes the audio
    Then the result is a non-empty string

  Scenario: An English WAV file produces a non-empty transcription
    Given the Transcriber is resolved from the DI container
    And the audio file "test_speech_en.wav" at 16000 Hz mono 16-bit
    And the language is "en-US"
    When the Transcriber transcribes the audio
    Then the result is a non-empty string

  Scenario: An empty audio buffer returns no transcription
    Given the Transcriber is resolved from the DI container
    And an AudioCapture with zero bytes of audio at 16000 Hz
    And the language is "es-ES"
    When the Transcriber transcribes the audio
    Then the result is None

  Scenario: A WAV file containing only the WAV header returns no transcription
    Given the Transcriber is resolved from the DI container
    And an AudioCapture with only the 44-byte WAV header at 16000 Hz
    And the language is "es-ES"
    When the Transcriber transcribes the audio
    Then the result is None

  Scenario: Transcription output is clean natural language text
    Given the Transcriber is resolved from the DI container
    And the audio file "test_speech_es.wav" at 16000 Hz mono 16-bit
    And the language is "es-ES"
    When the Transcriber transcribes the audio
    Then the result is a non-empty string
    And the result contains at least one space
    And the result is not the sentinel value "xyzzy"
    And the result does not contain JSON artifacts
