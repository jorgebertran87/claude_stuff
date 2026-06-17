Feature: Google Speech API key from environment
  As a developer
  I want the Google Speech API key to be read from the environment
  So that the key is not hardcoded in source code and can be rotated without rebuilding

  Scenario: Transcriber uses GOOGLE_SPEECH_API_KEY env var when set
    Given the environment variable GOOGLE_SPEECH_API_KEY is set to "test-key-123"
    When GoogleTranscriber is constructed
    Then the API key used for speech recognition requests is "test-key-123"

  Scenario: Transcriber falls back to built-in default when env var is absent
    Given the environment variable GOOGLE_SPEECH_API_KEY is not set
    When GoogleTranscriber is constructed
    Then the API key used for speech recognition requests is the built-in default

  Scenario: Empty GOOGLE_SPEECH_API_KEY falls back to built-in default
    Given the environment variable GOOGLE_SPEECH_API_KEY is set to ""
    When GoogleTranscriber is constructed
    Then the API key used for speech recognition requests is the built-in default
