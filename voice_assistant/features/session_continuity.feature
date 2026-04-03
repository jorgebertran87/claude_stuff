Feature: Conversation session continuity
  As a user
  I want the assistant to remember the conversation context across turns
  So that follow-up questions are answered coherently without repeating myself

  Scenario: Session id returned by the backend is stored after the first order
    Given a ClaudeCodeHandler with a backend that returns session_id "abc-123"
    When the handler handles "qué hora es"
    Then the stored session_id is "abc-123"

  Scenario: Stored session id is forwarded on the next order
    Given a ClaudeCodeHandler with a backend that returns session_id "abc-123"
    And the handler has already handled one order
    When the handler handles a second order
    Then the backend receives session_id "abc-123" on the second call

  Scenario: First order is always sent without a session id
    Given a ClaudeCodeHandler with no prior session
    When the handler handles "hola"
    Then the backend receives no session_id on that call

  Scenario: reset_session clears the stored session id
    Given a ClaudeCodeHandler with a backend that returns session_id "abc-123"
    And the handler has already handled one order
    When reset_session is called
    Then the stored session_id is None

  Scenario: Order after reset starts a new session without a session id
    Given a ClaudeCodeHandler with a backend that returns session_id "abc-123"
    And the handler has already handled one order
    And reset_session is called
    When the handler handles another order
    Then the backend receives no session_id on that call
