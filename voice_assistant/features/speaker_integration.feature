Feature: Speaker infrastructure integration
  As the system
  I want to convert text to spoken audio reliably via the Google TTS endpoint
  So that the assistant can respond to the user in any language

  Scenario: Generating speech for a supported language produces audio
    Given the text "hola mundo" and the language code "es"
    When the TTS segment is generated
    Then the result is a non-empty audio segment

  Scenario: Unsupported language code falls back to English
    Given the text "hello world" and the unsupported language code "xx"
    When the TTS segment is generated
    Then the pipeline recovers and produces a non-empty audio segment in English

  Scenario: A Spanish phrase returns non-empty MP3 bytes via HTTP
    Given the text "hola" and the language code "es"
    When tts_segment makes a real HTTP request
    Then the result is a non-empty audio segment

  Scenario: An English phrase returns non-empty MP3 bytes via HTTP
    Given the text "hello" and the language code "en"
    When tts_segment makes a real HTTP request
    Then the result is a non-empty audio segment

  Scenario: synthesize_text runs the full pipeline and returns MP3 bytes
    Given the text "buenos días, ¿cómo estás?"
    When synthesize_text is called
    Then the result is non-empty bytes
    And the bytes start with a valid MP3 header

  Scenario: synthesize_text with markdown returns non-empty bytes
    Given the text "**hola** mundo [link](http://example.com)"
    When synthesize_text is called
    Then the result is non-empty bytes

  Scenario: Alexa and Spotify command produces more audio than the title alone
    Given the response 'Alexa, pon "Shape of You" en Spotify'
    When the full TTS pipeline processes the response
    Then the combined audio is longer than the song title spoken alone

  Scenario: Full Alexa+Spotify command is synthesized as a single phrase in the title's detected language
    Given the response 'Alexa, pon "Shape of You" en Spotify'
    When alexa_spotify_title extracts the title and detects its language as "en"
    Then build_alexa_command produces "Alexa, play Shape of You on Spotify"
    And synthesize_alexa_spotify produces non-empty audio bytes for the unified command
