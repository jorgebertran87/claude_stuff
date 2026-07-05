Feature: Current date injection
  As a user of the DeepSeek chat API
  I want the model to know the current date
  So that it can answer time-sensitive questions accurately
  without me having to include the date in every system prompt

  Scenario: Date is prepended to messages in a simple chat call
    Given a mock DeepSeek API server
    When chat is called with a system message "You are helpful" and user message "Hello"
    Then the request includes a system message containing "Current date:"
    And the original system message "You are helpful" is also present

  Scenario: Date is prepended to messages in a tool-calling chat
    Given a mock DeepSeek API server
    When chat_with_tools is called with a system message "You are helpful"
    Then the request includes a system message containing "Current date:"
    And the original system message "You are helpful" is also present

  Scenario: Date format is ISO 8601 YYYY-MM-DD
    Given a mock DeepSeek API server
    When chat is called
    Then the date message matches the format "Current date: 20\d{2}-\d{2}-\d{2}\."
