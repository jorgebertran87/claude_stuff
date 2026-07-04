Feature: DeepSeek tool calling
  As a developer
  I want the deepseek_client to support OpenAI-compatible tool calling
  So that the model can request and receive tool execution results

  Scenario: Model returns a text response without tool calls
    Given a DeepSeek API that returns a text response
    When chat_with_tools is called with a FakeToolHandler returning "search result"
    Then the response content is "Paris is the capital of France"
    And no tool calls were made

  Scenario: Model requests a single tool call and gets a final answer
    Given a DeepSeek API that returns a web_search tool call
    When chat_with_tools is called with a FakeToolHandler returning "search result"
    Then one tool call was made
    And the tool call was for "web_search"
    And the response is a final answer

  Scenario: Model requests multiple tool calls in parallel
    Given a DeepSeek API that returns two tool calls in one response
    When chat_with_tools is called with a FakeToolHandler returning "search result"
    Then two tool calls were made
    And the response is a final answer

  Scenario: Model requests tool calls across multiple rounds
    Given a DeepSeek API that returns a tool call then a text response
    When chat_with_tools is called with a FakeToolHandler returning "search result"
    Then one tool call was made
    And the response is a final answer

  Scenario: Tool execution error is returned to the model
    Given a DeepSeek API that returns a web_search tool call
    When chat_with_tools is called with an ErrorToolHandler
    Then the response is a final answer

  Scenario: Max tool rounds exceeded returns an error
    Given a DeepSeek API that always returns a tool call
    When chat_with_tools is called with a handler returning "ok"
    Then an error is returned
