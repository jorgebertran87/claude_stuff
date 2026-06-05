Feature: Controlling services with docker compose
  As the service controller
  I want to drive each service's docker compose project
  So that aliases backed by compose can be started, stopped, and inspected

  Scenario: Starting a service runs docker compose start in its directory
    Given a compose controller
    When I start the service in "/srv/web"
    Then docker ran "compose -f /srv/web/docker-compose.yml start"
    And the control call succeeds

  Scenario: Stopping a service runs docker compose stop in its directory
    Given a compose controller
    When I stop the service in "/srv/web"
    Then docker ran "compose -f /srv/web/docker-compose.yml stop"
    And the control call succeeds

  Scenario: Restarting a service runs docker compose restart in its directory
    Given a compose controller
    When I restart the service in "/srv/web"
    Then docker ran "compose -f /srv/web/docker-compose.yml restart"
    And the control call succeeds

  Scenario: Querying a running service reports running
    Given a compose controller
    And docker compose ps reports a running container
    When I query the status of the service in "/srv/web"
    Then docker ran "compose -f /srv/web/docker-compose.yml ps --format json"
    And the reported status is "running"

  Scenario: Querying a stopped service reports stopped
    Given a compose controller
    And docker compose ps reports an exited container
    When I query the status of the service in "/srv/web"
    Then the reported status is "stopped"

  Scenario: A failing docker compose command surfaces as an error
    Given a compose controller
    And docker compose commands fail
    When I start the service in "/srv/web"
    Then the control call fails
