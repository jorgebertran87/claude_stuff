Feature: Offline speech-to-text with Whisper.cpp
  As a user
  I want the assistant to transcribe speech locally using Whisper.cpp
  So that transcription works without internet, has better accuracy, and doesn't depend on unofficial Google APIs

  Scenario: Whisper transcribes a Spanish WAV file
    Given the audio file "test_speech_es.wav" at 16000 Hz mono 16-bit
    And the language is "es-ES"
    When WhisperTranscriber transcribes the audio
    Then the result is a non-empty string

  Scenario: Whisper transcribes an English WAV file
    Given the audio file "test_speech_en.wav" at 16000 Hz mono 16-bit
    And the language is "en-US"
    When WhisperTranscriber transcribes the audio
    Then the result is a non-empty string

  Scenario: Empty audio returns no transcription
    Given an AudioCapture with zero bytes of audio at 16000 Hz
    And the language is "es-ES"
    When WhisperTranscriber transcribes the audio
    Then the result is None

  Scenario: Whisper reads model path from WHISPER_MODEL env var
    Given WHISPER_MODEL is set to "/custom/path/model.bin"
    When WhisperTranscriber is constructed
    Then it uses the custom model path

  Scenario: Whisper falls back to default model path when env var is absent
    Given WHISPER_MODEL is not set
    When WhisperTranscriber is constructed
    Then it uses the default model path "/app/models/ggml-tiny.bin"
