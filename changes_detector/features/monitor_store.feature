Feature: Monitor store persistence
  As the changes detector service
  I want to persist monitor configurations to disk as JSON
  So that all monitors survive container restarts

  Scenario: A new store starts with no monitors
    Given a monitor store in a temporary directory
    Then the store has 0 monitors

  Scenario: Adding a monitor increases the count
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    Then the store has 1 monitors

  Scenario: An added monitor can be found by alias
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    Then the store contains monitor "news-site"

  Scenario: An added monitor survives a reload from disk
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    And I reload the store from disk
    Then the store has 1 monitors
    And the store contains monitor "news-site"

  Scenario: Removing a monitor decreases the count
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    And I remove monitor "news-site"
    Then the store has 0 monitors

  Scenario: Pausing a monitor sets its paused flag
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    And I pause monitor "news-site"
    Then monitor "news-site" is paused

  Scenario: Paused flag survives a reload from disk
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    And I pause monitor "news-site"
    And I reload the store from disk
    Then monitor "news-site" is paused

  Scenario: Resuming a paused monitor clears the paused flag
    Given a monitor store in a temporary directory
    When I add monitor "news-site" watching "https://example.com" selector "h1" every 60 seconds
    And I pause monitor "news-site"
    And I resume monitor "news-site"
    Then monitor "news-site" is not paused
