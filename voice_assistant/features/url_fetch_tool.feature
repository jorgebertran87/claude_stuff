Feature: URL fetch tool
  As a user
  I want the assistant to fetch and read URLs
  So that I can ask about the content of web pages

  Scenario: Fetch a plain text URL returns its content
    Given a URL "https://example.com/data.txt" that returns plain text "hello world"
    When the url_fetch tool executes with url "https://example.com/data.txt"
    Then the result contains "hello world"

  Scenario: Fetch an HTML page strips tags and returns readable text
    Given a URL "https://example.com/page" that returns HTML with body text "Welcome to Example"
    When the url_fetch tool executes with url "https://example.com/page"
    Then the result contains "Welcome to Example"
    And the result does not contain "<html"

  Scenario: Fetch a URL that returns HTTP 404
    Given a URL "https://example.com/missing" that returns HTTP 404
    When the url_fetch tool executes with url "https://example.com/missing"
    Then the result contains an error message

  Scenario: Fetch a URL that is unreachable
    Given a URL "https://unreachable.local/broken" that is unreachable
    When the url_fetch tool executes with url "https://unreachable.local/broken"
    Then the result contains an error message

  Scenario: Fetch result is truncated if too large
    Given a URL "https://example.com/large" that returns 60000 characters of text
    When the url_fetch tool executes with url "https://example.com/large"
    Then the result is shorter than 55000 characters

  Scenario: Fetch a URL that responds within the timeout succeeds
    Given a URL "https://example.com/slow" that returns plain text "done" after a 2-second delay
    When the url_fetch tool executes with url "https://example.com/slow"
    Then the result contains "done"

  Scenario: Fetch with a custom-configured agent uses the injected timeouts
    Given a url_fetch tool configured with a 1-second connect timeout and a 1-second read timeout
    When the url_fetch tool executes with url "https://example.com/hanging"
    Then the result contains an error message
