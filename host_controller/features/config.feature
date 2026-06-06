Feature: Loading configuration
  As the operator of my host
  I want configuration read with safe defaults and strict validation
  So that the bot starts correctly and refuses to run when misconfigured

  Scenario: A complete configuration is loaded
    Given a config value "TELEGRAM_BOT_TOKEN" of "secret123"
    And a config value "TELEGRAM_ALLOWED_CHATS" of "1, 2"
    And a config value "SSH_HOST" of "example.host"
    And a config value "SSH_PORT" of "2222"
    And a config value "SSH_USER" of "bot"
    And a config value "SSH_KEY" of "/keys/id"
    And a config value "SSH_KNOWN_HOSTS" of "/keys/known"
    And a config value "COMMAND_TIMEOUT_SECS" of "15"
    When the configuration is loaded
    Then loading succeeds
    And the bot token is "secret123"
    And the allowed chats are "1,2"
    And the ssh target is "bot@example.host:2222"
    And the command timeout is 15 seconds

  Scenario: Optional values fall back to defaults
    Given a config value "TELEGRAM_BOT_TOKEN" of "secret123"
    When the configuration is loaded
    Then loading succeeds
    And the allowed chats are empty
    And the ssh target is "botuser@host.docker.internal:22"
    And the command timeout is 30 seconds

  Scenario: A missing bot token is rejected
    Given a config value "TELEGRAM_ALLOWED_CHATS" of "1"
    When the configuration is loaded
    Then loading fails

  Scenario: A blank allowlist loads as deny-all
    Given a config value "TELEGRAM_BOT_TOKEN" of "secret123"
    And a config value "TELEGRAM_ALLOWED_CHATS" of ""
    When the configuration is loaded
    Then loading succeeds
    And the allowed chats are empty

  Scenario: A non-numeric chat id is rejected
    Given a config value "TELEGRAM_BOT_TOKEN" of "secret123"
    And a config value "TELEGRAM_ALLOWED_CHATS" of "1, abc"
    When the configuration is loaded
    Then loading fails

  Scenario: A non-numeric command timeout is rejected
    Given a config value "TELEGRAM_BOT_TOKEN" of "secret123"
    And a config value "COMMAND_TIMEOUT_SECS" of "soon"
    When the configuration is loaded
    Then loading fails
