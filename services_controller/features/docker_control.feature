Feature: Controlling Docker containers
  As the service controller
  I want to drive containers through the Docker Engine API
  So that aliases backed by Docker can be started, stopped, and inspected

  Scenario: Starting a container issues a start request
    Given a mock Docker API that accepts control of container "nginx"
    And a Docker controller targeting that API
    When I start container "nginx"
    Then the control call succeeds

  Scenario: Stopping a container issues a stop request
    Given a mock Docker API that accepts control of container "nginx"
    And a Docker controller targeting that API
    When I stop container "nginx"
    Then the control call succeeds

  Scenario: Querying a running container reports running
    Given a mock Docker API reporting container "nginx" as running
    And a Docker controller targeting that API
    When I query the status of container "nginx"
    Then the control call succeeds
    And the reported status is "running"

  Scenario: Querying a stopped container reports stopped
    Given a mock Docker API reporting container "nginx" as stopped
    And a Docker controller targeting that API
    When I query the status of container "nginx"
    Then the control call succeeds
    And the reported status is "stopped"

  Scenario: A Docker API error surfaces as a failure
    Given a mock Docker API that fails control of container "nginx"
    And a Docker controller targeting that API
    When I start container "nginx"
    Then the control call fails
