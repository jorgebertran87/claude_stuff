Feature: Controlling services from a Telegram command
  As the operator of my server
  I want to control services with per-action Telegram commands
  So that I can manage them from my phone without a shell

  Scenario: Starting a service replies with confirmation
    Given a service bot mapping alias "web" to a stopped service
    And a command "/start web" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "started"
    And the service behind "web" is running

  Scenario: Stopping a service replies with confirmation
    Given a service bot mapping alias "web" to a running service
    And a command "/stop web" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "stopped"
    And the service behind "web" is stopped

  Scenario: Restarting a service replies with confirmation
    Given a service bot mapping alias "web" to a running service
    And a command "/restart web" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "restarted"
    And the service behind "web" is running

  Scenario: Querying status reports the current state
    Given a service bot mapping alias "web" to a running service
    And a command "/status web" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "running"

  Scenario: An unknown alias replies with an error
    Given a service bot mapping alias "web" to a running service
    And a command "/start ghost" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "unknown alias"

  Scenario: A command missing its alias replies with usage
    Given a service bot mapping alias "web" to a running service
    And a command "/start" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "Usage"

  Scenario: An unrecognized command is ignored
    Given a service bot mapping alias "web" to a running service
    And a command "good morning bot" from chat 1
    When the bot processes the updates
    Then no reply is posted

  Scenario: A message from an unauthorized chat is ignored
    Given a service bot mapping alias "web" to a running service that only allows chat 99
    And a command "/start web" from chat 1
    When the bot processes the updates
    Then no reply is posted

  Scenario: The update offset advances past processed updates
    Given a service bot mapping alias "web" to a running service
    And a command with id 42 "/status web" from chat 1
    When the bot processes the updates
    Then the offset is 43
