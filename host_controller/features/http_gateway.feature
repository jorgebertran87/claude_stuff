Feature: Talking to the Telegram Bot API over HTTP
  As the host controller
  I want a real gateway to the Telegram Bot API
  So that the bot can long-poll for messages and post replies

  Scenario: Fetching returns the messages from the API
    Given a mock Telegram API returning a message "hello" with update id 7 from chat 1
    And an http gateway for that API
    When I fetch updates from offset 0
    Then one update is returned
    And the update has id 7, chat 1, and text "hello"

  Scenario: Fetching long-polls at the requested offset
    Given a mock Telegram API that only answers a long-poll at offset 5
    And an http gateway for that API
    When I fetch updates from offset 5
    Then one update is returned

  Scenario: An update without a text message is skipped
    Given a mock Telegram API returning an update with no message
    And an http gateway for that API
    When I fetch updates from offset 0
    Then no updates are returned

  Scenario: A failing API response yields no updates
    Given a mock Telegram API that returns HTTP 500
    And an http gateway for that API
    When I fetch updates from offset 0
    Then no updates are returned

  Scenario: Posting a message sends it to the chat
    Given a mock Telegram API accepting messages
    And an http gateway for that API
    When I post "hello world" to chat 1
    Then the API received a message to chat 1 containing "hello world"
