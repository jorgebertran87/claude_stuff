Feature: Running commands on the host over SSH
  As the host controller
  I want to run each command on the parent host through ssh
  So that a containerized bot can drive the host as a non-root user

  Background:
    Given an ssh executor for user "botuser" on host "host.docker.internal" port 22 with key "/secrets/id_ed25519" and known hosts "/secrets/known_hosts"

  Scenario: A command is sent to the host over a verified, non-interactive ssh session
    When I run "ls -la"
    Then ssh ran "-i /secrets/id_ed25519 -p 22 -o BatchMode=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=/secrets/known_hosts botuser@host.docker.internal ls -la"
    And the execution succeeds

  Scenario: The command's output and exit code are returned
    Given the host returns exit code 0 with output "hello"
    When I run "echo hello"
    Then the returned output is "hello"
    And the returned exit code is 0

  Scenario: A non-zero exit status is reported, not treated as an error
    Given the host returns exit code 2 with output "nope"
    When I run "false"
    Then the returned exit code is 2
    And the execution succeeds

  Scenario: An SSH transport failure surfaces as an error
    Given the ssh transport fails
    When I run "ls"
    Then the execution fails
