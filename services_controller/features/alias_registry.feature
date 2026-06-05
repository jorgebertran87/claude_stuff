Feature: Loading service aliases from a config file
  As the operator of my server
  I want to declare my aliases in a config file
  So that the controller knows which service each alias points to

  Scenario: Loading aliases makes them resolvable
    Given a config file mapping "web" to "nginx" and "db" to "postgres"
    When I load the registry from that file
    Then the registry resolves "web" to "nginx"
    And the registry resolves "db" to "postgres"

  Scenario: Resolving an alias that is not in the config file fails
    Given a config file mapping "web" to "nginx"
    When I load the registry from that file
    Then resolving "ghost" reports an unknown alias

  Scenario: A missing config file is rejected
    Given no config file exists at the given path
    When I load the registry from that file
    Then loading fails

  Scenario: A malformed config file is rejected
    Given a config file with invalid contents
    When I load the registry from that file
    Then loading fails
