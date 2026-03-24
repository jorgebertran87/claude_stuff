Feature: Conversation flow and state transitions
  As a user
  I want the assistant to manage multi-turn conversations correctly
  So that follow-up questions feel natural and interruptions are handled gracefully

  Scenario: Melody thread is fully stopped before the response is spoken
    Given an order has been handled and a response is ready
    When the service speaks the response
    Then the melody thread is no longer alive after playback ends

  Scenario: Echo reference is always cleared after a response is spoken
    Given the assistant speaks a response
    When the speech finishes
    Then the echo reference stored in the capturer is None

  Scenario: A response ending with a question skips the wake word on the next turn
    Given the user says "claudito" followed by "qué hora es"
    And the assistant responds with "¿En qué ciudad?"
    When the assistant finishes speaking
    Then waiting_for_answer is set to True
    And the next order "en madrid" is captured without requiring the wake word again

  Scenario: An interruption during speech sets waiting_for_answer to True
    Given the assistant is speaking a long response
    And the user says the wake word "claudito" during playback
    When the speech is interrupted
    Then waiting_for_answer is set to True
    So that the next order is captured directly without requiring the wake word again
