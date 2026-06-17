Feature: Offline text-to-speech with Piper TTS
  As a user
  I want the assistant to synthesize speech locally using Piper TTS
  So that it works without internet, has consistent quality, and doesn't depend on unofficial Google APIs

  Scenario: Piper synthesizes Spanish text to non-empty audio bytes
    Given the text "Hola, ¿cómo estás?" in language "es"
    When PiperSynthesizer synthesizes the text
    Then the returned bytes are non-empty valid MP3

  Scenario: Piper falls back to English voice when language model is missing
    Given the text "Hello world" in language "fr"
    When PiperSynthesizer synthesizes the text
    Then the synthesis succeeds using the English voice model

  Scenario: Piper handles empty text gracefully
    Given an empty string
    When PiperSynthesizer synthesizes the text
    Then it returns an empty byte vector

  Scenario: Piper splits long text into chunks
    Given text longer than 200 characters
    When PiperSynthesizer synthesizes the text
    Then the text is split into sentence-level chunks
    And each chunk is synthesized separately
