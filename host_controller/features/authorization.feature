Feature: Authorizing who can run commands
  As the operator of my host
  I want only specific Telegram chats to run commands
  So that a remote-command bot can never be driven by a stranger

  Scenario: A chat on the allowlist is authorized
    Given an allowlist of chats 1, 2
    When chat 1 is checked
    Then the chat is authorized

  Scenario: Every chat on the allowlist is authorized
    Given an allowlist of chats 1, 2
    When chat 2 is checked
    Then the chat is authorized

  Scenario: A chat not on the allowlist is denied
    Given an allowlist of chats 1, 2
    When chat 3 is checked
    Then the chat is denied

  Scenario: An empty allowlist denies every chat
    Given an empty allowlist
    When chat 1 is checked
    Then the chat is denied

  Scenario: A group chat is denied even if allowlisted
    Given an allowlist of chats 1, -1002003000
    When chat -1002003000 is checked
    Then the chat is denied

  Scenario: A chat id of zero is denied even if allowlisted
    Given an allowlist of chats 0
    When chat 0 is checked
    Then the chat is denied
