Feature: Wake word detection
  As a user
  I want the assistant to activate only when I say the wake word
  So that it does not respond to unrelated conversation

  Scenario: User says only the wake word
    Given the microphone captures "claudito"
    When the service waits for the wake word
    Then it detects the wake word
    And it returns no inline order

  Scenario: User says the wake word followed by an order in the same utterance
    Given the microphone captures "claudito pon música"
    When the service waits for the wake word
    Then it detects the wake word
    And it returns "pon música" as the inline order

  Scenario: A capture returns nothing before the wake word is heard
    Given the microphone returns no audio on the first capture
    And the microphone captures "claudito" on the second capture
    When the service waits for the wake word
    Then it skips the empty capture and keeps listening
    And it detects the wake word

  Scenario: An utterance does not contain the wake word
    Given the microphone captures "hola mundo"
    And the microphone then captures "claudito"
    When the service waits for the wake word
    Then it ignores the first utterance
    And it detects the wake word on the second utterance

  Scenario: User pronounces the wake word with a slight typo
    Given the microphone captures "clauditto"
    When the service waits for the wake word
    Then it detects the wake word via fuzzy matching
    And it returns no inline order
