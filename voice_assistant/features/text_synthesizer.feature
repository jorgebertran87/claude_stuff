Feature: TextSynthesizer port integration
  As the system
  I want to convert text to spoken audio through the TextSynthesizer port
  So that the assistant can respond to the user in any language

  Background:
    Given the TextSynthesizer is resolved from the DI container

  Scenario: Synthesizing Spanish text produces audio bytes
    When synthesize_text is called with "hola mundo"
    Then the result is non-empty bytes
    And the bytes start with a valid MP3 header

  Scenario: Synthesizing English text produces audio bytes
    When synthesize_text is called with "hello world"
    Then the result is non-empty bytes

  Scenario: Empty text returns empty bytes
    When synthesize_text is called with ""
    Then the result is an empty byte vector

  Scenario: Text with markdown is cleaned before synthesis
    When synthesize_text is called with "**hola** mundo [link](http://example.com)"
    Then the result is non-empty bytes

  Scenario: Alexa and Spotify text is detected and reformatted
    When synthesize_alexa_spotify is called with 'Alexa, pon "Shape of You" en Spotify'
    Then the result is non-empty bytes
    And the bytes start with a valid MP3 header
