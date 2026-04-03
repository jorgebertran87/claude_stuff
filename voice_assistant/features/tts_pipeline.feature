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

  Scenario: Full Alexa+Spotify command is synthesized as a single phrase in the title's detected language
    Given the response 'Alexa, pon "Shape of You" en Spotify'
    When alexa_spotify_title extracts the title and detects its language as "en"
    Then build_alexa_command produces "Alexa, play Shape of You on Spotify"
    And synthesize_alexa_spotify produces non-empty audio bytes for the unified command
