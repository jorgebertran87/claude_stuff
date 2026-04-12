Feature: ClaudeCodeHandler token logging
  As a developer
  I want the handler to record token usage and cost after each order
  So that I can monitor API spend per request

  Scenario: Token log file is created after handling an order
    Given a ClaudeCodeHandler with a mocked query that returns a result message
    When the handler handles "pon música"
    Then the token log file exists

  Scenario: Token log contains the order and all token fields
    Given a ClaudeCodeHandler with a mocked query returning input_tokens=18, output_tokens=735, cache_read=38335, cache_creation=2610, cost=0.029965
    When the handler handles "mañana lloverá"
    Then the log line contains "mañana lloverá"
    And the log line contains "input: 18"
    And the log line contains "output: 735"
    And the log line contains "cache_read: 38335"
    And the log line contains "cache_creation: 2610"
    And the log line contains "total: 41698"
    And the log line contains "0.029965"

  Scenario: Token log appends one line per call
    Given a ClaudeCodeHandler with a mocked query
    When the handler handles "primera orden"
    And the handler handles "segunda orden"
    Then the log file has exactly 2 lines
    And line 1 contains "primera orden"
    And line 2 contains "segunda orden"

  Scenario: Handle returns the result from the message
    Given a ClaudeCodeHandler with a mocked query that returns result "respuesta esperada"
    When the handler handles "una orden"
    Then the return value is "respuesta esperada"

  Scenario: reset_session clears the stored session id
    Given a session-tracking backend
    When the handler handles "primera orden"
    And reset_session is called
    And the handler handles "segunda orden"
    Then the second call had no session id
