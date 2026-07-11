Feature: AudioSpeaker port integration
  As the system
  I want to speak responses through the AudioSpeaker port
  So that the assistant can respond audibly to the user

  Background:
    Given the AudioSpeaker is resolved from the DI container

  Scenario: AudioSpeaker resolves from the DI container
    Then the speaker is available

  Scenario: Stop with no active playback does not panic
    When the speaker is stopped
    Then no panic occurs

  Scenario: Get echo reference returns None
    When the echo reference is requested
    Then the result is None

  Scenario: Beep does not panic
    When the speaker beeps
    Then no panic occurs

  Scenario: Speak processes text through TTS
    Given the language is "es-ES"
    When the speaker speaks "hola mundo"
    Then no panic occurs
