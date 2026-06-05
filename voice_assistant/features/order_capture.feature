Feature: Order capture
  As a user
  I want the assistant to listen for my command after the wake word
  So that it can execute what I ask

  Scenario: User speaks a clear order on the first attempt
    Given the microphone captures a valid audio clip
    And the transcription returns "enciende la luz"
    When the service listens for an order
    Then it returns "enciende la luz"

  Scenario: All capture attempts time out
    Given the microphone returns no audio on every attempt
    When the service listens for an order
    Then it returns no order after exhausting all retries

  Scenario: First capture times out but the second succeeds
    Given the microphone returns no audio on the first attempt
    And the microphone captures a valid audio clip on the second attempt
    And the transcription returns "apaga la luz"
    When the service listens for an order
    Then it retries and returns "apaga la luz"

  Scenario: First transcription fails but the second succeeds
    Given the microphone captures a valid audio clip on both attempts
    And the first transcription returns nothing
    And the second transcription returns "qué hora es"
    When the service listens for an order
    Then it retries and returns "qué hora es"

  Scenario: The assistant beeps once before each listening attempt
    Given the microphone returns no audio on the first attempt
    And the microphone captures a valid audio clip on the second attempt
    And the transcription returns "hola"
    When the service listens for an order
    Then the speaker has beeped exactly 2 times
