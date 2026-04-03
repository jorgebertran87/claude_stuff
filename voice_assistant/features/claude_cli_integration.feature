Feature: Claude CLI integration
  As the system
  I want to invoke the real claude CLI and parse its JSON output
  So that orders are answered and token usage is logged without any stub

  Scenario: A simple order returns a non-empty result
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "qué hora es"
    Then the returned string is non-empty

  Scenario: The response includes a session id
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "hola"
    Then the stored session_id is non-empty after the call

  Scenario: The token log file is created on first use
    Given the claude CLI is available and authenticated
    And no token log file exists yet
    When ClaudeCodeHandler handles "di hola"
    Then the token log file exists on disk

  Scenario: The token log file records the order and token counts
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "di hola"
    Then the token log contains the text "di hola"
    And the token log contains "input:"
    And the token log contains "output:"
    And the token log contains "total:"
    And the token log contains "cost:"

  Scenario: Two consecutive orders append two separate lines to the log
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "primera orden"
    And ClaudeCodeHandler handles "segunda orden"
    Then the token log file has exactly 2 lines
