Feature: Google TTS HTTP integration
  As the system
  I want to fetch real speech audio from the Google TTS endpoint
  So that the assistant can speak in the user's language without any stub

  Scenario: A Spanish phrase returns non-empty MP3 bytes
    Given the text "hola" and the language code "es"
    When tts_segment makes a real HTTP request
    Then the response is a non-empty AudioSegment

  Scenario: An English phrase returns non-empty MP3 bytes
    Given the text "hello" and the language code "en"
    When tts_segment makes a real HTTP request
    Then the response is a non-empty AudioSegment

  Scenario: An unsupported language code falls back to English and still returns audio
    Given the text "hello world" and the unsupported language code "xx"
    When tts_segment makes a real HTTP request
    Then the response is a non-empty AudioSegment

  Scenario: synthesize_text runs the full pipeline and returns MP3 bytes
    Given the text "buenos días, ¿cómo estás?"
    When synthesize_text is called
    Then the result is non-empty bytes
    And the bytes start with a valid MP3 header

  Scenario: synthesize_text with markdown returns non-empty bytes
    Given the text "**hola** mundo [link](http://example.com)"
    When synthesize_text is called
    Then the result is non-empty bytes
