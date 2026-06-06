Feature: Interpreting an incoming message
  As the operator of my host
  I want plain messages treated as commands while a couple of words stay reserved
  So that I can type commands naturally and /start and /help never reach the host

  Scenario: A plain message is taken as a command to run
    Given the message "ls -la"
    When the message is interpreted
    Then the result is a command to run "ls -la"

  Scenario: Surrounding whitespace is trimmed from a command
    Given the message "   df -h   "
    When the message is interpreted
    Then the result is a command to run "df -h"

  Scenario: The /start command is reserved for help
    Given the message "/start"
    When the message is interpreted
    Then the result is a help request

  Scenario: The /help command is reserved for help
    Given the message "/help"
    When the message is interpreted
    Then the result is a help request

  Scenario: A blank message is ignored
    Given the message "    "
    When the message is interpreted
    Then the result is ignored

  Scenario: An unknown slash command is still run as a command
    Given the message "/status"
    When the message is interpreted
    Then the result is a command to run "/status"
