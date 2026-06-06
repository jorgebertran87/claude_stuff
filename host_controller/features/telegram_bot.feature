Feature: Driving the host from Telegram
  As the operator of my host
  I want the bot to run my messages as commands and reply with the result
  So that I can control the host from my phone, safely and within limits

  Scenario: An authorized command runs on the host and replies with the output
    Given a bot that allows chat 1
    And the host returns the output "hi"
    And a command "echo hi" from chat 1
    When the bot processes the updates
    Then the host ran "echo hi"
    And a reply to chat 1 contains "hi"

  Scenario: A message from an unauthorized chat is ignored
    Given a bot that allows chat 1
    And a command "rm -rf /" from chat 99
    When the bot processes the updates
    Then no reply is posted
    And no command is run on the host

  Scenario: The help command replies with usage and runs nothing
    Given a bot that allows chat 1
    And a command "/help" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "command"
    And no command is run on the host

  Scenario: A blank message is ignored
    Given a bot that allows chat 1
    And a command "   " from chat 1
    When the bot processes the updates
    Then no reply is posted

  Scenario: A command that runs too long replies with a timeout message
    Given a bot whose commands time out quickly
    And the host does not respond in time
    And a command "sleep 999" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "timed out"

  Scenario: An unreachable host replies with an error
    Given a bot that allows chat 1
    And the host is unreachable
    And a command "ls" from chat 1
    When the bot processes the updates
    Then a reply to chat 1 contains "error"

  Scenario: The update offset advances past processed updates
    Given a bot that allows chat 1
    And a command with id 42 "ls" from chat 1
    When the bot processes the updates
    Then the offset is 43
