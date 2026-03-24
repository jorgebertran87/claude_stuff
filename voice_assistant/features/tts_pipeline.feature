Feature: Text-to-speech audio generation
  As the system
  I want to convert text to spoken audio reliably
  So that the assistant can respond to the user in any language

  Scenario: Generating speech for a supported language produces audio
    Given the text "hola mundo" and the language code "es"
    When the TTS segment is generated
    Then the result is a non-empty audio segment

  Scenario: Unsupported language code falls back to English
    Given the text "hello world" and the unsupported language code "xx"
    When the TTS segment is generated
    Then the pipeline recovers and produces a non-empty audio segment in English

  Scenario: Alexa and Spotify command produces more audio than the title alone
    Given the response 'Alexa, pon "Shape of You" en Spotify'
    When the full TTS pipeline processes the response
    Then the combined audio is longer than the song title spoken alone

  Scenario: Command frame and song title are spoken with different voices
    Given the response 'Alexa, pon "Shape of You" en Spotify'
    When the command frame and the song title are each converted to audio
    Then the resulting audio bytes are different from each other
