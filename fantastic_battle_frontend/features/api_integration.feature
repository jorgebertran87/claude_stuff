@api
Feature: API Integration
  As a player
  I want the game client to communicate with the backend
  So that the game world is server-authoritative

  Scenario: Game loads map and NPCs from the server
    Given the game loads with the API backend
    Then the map is 5 tiles wide and 5 tiles tall
    And the player is at grid position (0, 0)
    And the player is facing south
    And an NPC named Sphinx is at grid position (2, 0)

  Scenario: Server-validated movement succeeds
    Given the game loads with the API backend
    When the player moves east
    Then the player is at grid position (1, 0)
    And the player is facing east

  Scenario: Server-validated movement blocked by wall
    Given the game loads with the API backend
    When the player moves south
    Then the player stays at grid position (0, 0)
