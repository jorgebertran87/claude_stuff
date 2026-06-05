Feature: Controlling services by alias
  As the operator of my server
  I want to control services through stable aliases
  So that I never have to remember the underlying service names

  Scenario: Starting a stopped service by its alias
    Given a registry mapping alias "web" to service "nginx"
    And the service "nginx" is stopped
    When I start "web"
    Then the operation succeeds
    And the service "nginx" is running

  Scenario: Stopping a running service by its alias
    Given a registry mapping alias "web" to service "nginx"
    And the service "nginx" is running
    When I stop "web"
    Then the operation succeeds
    And the service "nginx" is stopped

  Scenario: Restarting a running service by its alias
    Given a registry mapping alias "web" to service "nginx"
    And the service "nginx" is running
    When I restart "web"
    Then the operation succeeds
    And the service "nginx" is running

  Scenario: Querying the status of a service by its alias
    Given a registry mapping alias "web" to service "nginx"
    And the service "nginx" is running
    When I query the status of "web"
    Then the reported status is "running"

  Scenario: Controlling an unknown alias fails
    Given a registry mapping alias "web" to service "nginx"
    When I start "ghost"
    Then the operation fails with an unknown alias error

  Scenario: A failing control backend surfaces the error
    Given a registry mapping alias "web" to service "nginx"
    And the control backend is unavailable
    When I start "web"
    Then the operation fails
