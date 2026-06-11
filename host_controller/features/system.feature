Feature: System command runner

  The production runner spawns real processes. When the bot's timeout fires it
  abandons the run mid-flight — the spawned process must die with it instead of
  lingering on the machine.

  Scenario: Output and exit code are captured
    When the runner runs a command printing "hi" that exits with code 3
    Then the captured stdout is "hi"
    And the captured exit code is 3

  Scenario: An abandoned command does not leave its process running
    When the runner abandons a long-running command
    Then the spawned process is no longer running
