Feature: Web search tool
  As a user
  I want the assistant to search the web using a self-hosted SearXNG instance
  So that I get current information without depending on external scraping
  that breaks when sites deploy bot detection

  Scenario: Web search returns formatted results
    Given the SearXNG API returns search results for "rust programming language"
    When the web_search tool executes with query "rust programming language"
    Then the result contains "Rust Programming Language"
    And the result contains a URL starting with "http"

  Scenario: Web search handles empty results
    Given the SearXNG API returns no results for "xyznonexistent123"
    When the web_search tool executes with query "xyznonexistent123"
    Then the result is not empty
    And the result does not contain an error message

  Scenario: Web search handles HTTP errors gracefully
    Given the SearXNG API returns HTTP 500
    When the web_search tool executes with query "test"
    Then the result contains an error message
    And no panic occurs
