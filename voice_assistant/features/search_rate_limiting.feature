Feature: Search rate limiting
  As a system operator
  I want the web search tool to enforce a minimum delay between search requests
  So that SearXNG's upstream engines (Google, Brave) are not suspended for
  sending too many requests in a short period

  Background:
    Given the rate limiter enforces a minimum gap of 2 seconds between searches

  Scenario: First search proceeds immediately
    When the web_search tool executes with query "rust"
    Then the search completes without delay
    And the result is not a rate-limit error

  Scenario: Rapid successive searches are delayed
    Given a search was just performed with query "python"
    When the web_search tool executes with query "golang" immediately after
    Then the second search is delayed by at least 2 seconds
    And the result is not a rate-limit error

  Scenario: Searches after the interval proceed immediately
    Given a search was performed with query "rust" more than 2 seconds ago
    When the web_search tool executes with query "python"
    Then the search completes without delay
    And the result is not a rate-limit error

  Scenario: The minimum gap is configurable
    Given the rate limiter is configured with a gap of 5 seconds
    And a search was just performed with query "first"
    When the web_search tool executes with query "second" immediately after
    Then the second search is delayed by at least 5 seconds
