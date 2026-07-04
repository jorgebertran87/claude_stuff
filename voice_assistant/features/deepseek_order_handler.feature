Feature: DeepSeek-backed order handler
  As a user
  I want the assistant to answer my orders using the DeepSeek chat API
  So that I don't depend on the Claude CLI for text orders while keeping
  Claude for skills (bus) and image analysis

  Scenario: An order is answered by DeepSeek
    Given a DeepSeek backend that replies "Son las tres de la tarde"
    When the handler handles "qué hora es"
    Then the return value is "Son las tres de la tarde"

  Scenario: Token usage is logged after each order
    Given a DeepSeek backend that replies "ok" with input_tokens=50 and output_tokens=200
    When the handler handles "hola"
    Then the token log file exists
    And the log line contains "hola"
    And the log line contains "input: 50"
    And the log line contains "output: 200"
    And the log line contains "total: 250"

  Scenario: Cache tokens are always zero in the log
    Given a DeepSeek backend that replies "ok" with input_tokens=10 and output_tokens=20
    When the handler handles "cualquier cosa"
    Then the log line contains "cache_read: 0"
    And the log line contains "cache_creation: 0"

  Scenario: Session id is None after every call (DeepSeek is stateless)
    Given a backend that always returns session_id None
    When the handler handles "primera orden"
    Then the return value is "ok"
    And the log line contains "cache_read: 0"

  Scenario: DeepSeek API returns an HTTP error
    Given a DeepSeek backend that returns HTTP 500
    When the handler handles "hola"
    Then the return value is an error message

  Scenario: DeepSeek API returns malformed JSON
    Given a DeepSeek backend that returns malformed JSON
    When the handler handles "hola"
    Then the return value is an error message

  Scenario: Handler with tools lets the model answer directly when no tools are needed
    Given a tool-backed DeepSeek backend that replies "Son las tres de la tarde"
    When the handler handles "qué hora es"
    Then the return value is "Son las tres de la tarde"

  Scenario: Handler with tools lets the model use web search and return an answer
    Given a tool-backed DeepSeek backend that replies with a tool call then "The capital is Paris"
    When the handler handles "cuál es la capital de Francia"
    Then the return value is "The capital is Paris"
