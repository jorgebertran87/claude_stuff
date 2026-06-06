Feature: Formatting command output for a Telegram reply
  As the host controller
  I want a command's output turned into one tidy reply
  So that I can read the result on my phone without it overflowing

  Scenario: Successful output is shown as-is
    Given command output with exit code 0 and stdout "hello world"
    When the output is formatted
    Then the reply is "hello world"

  Scenario: A failing command shows its exit code
    Given command output with exit code 1 and stdout "boom"
    When the output is formatted
    Then the reply contains "[exit 1]"
    And the reply contains "boom"

  Scenario: Standard error is included in the reply
    Given command output with exit code 0 and stderr "warning: deprecated"
    When the output is formatted
    Then the reply contains "warning: deprecated"

  Scenario: A command with no output still produces a reply
    Given command output with exit code 0 and no output
    When the output is formatted
    Then the reply is "(no output)"

  Scenario: Output longer than the Telegram limit is truncated with a marker
    Given command output with exit code 0 and stdout of 5000 characters
    When the output is formatted
    Then the reply is at most 4096 characters
    And the reply ends with "[truncated]"

  Scenario: The exit code survives truncation of long output
    Given command output with exit code 7 and stdout of 5000 characters
    When the output is formatted
    Then the reply contains "[exit 7]"
    And the reply is at most 4096 characters
