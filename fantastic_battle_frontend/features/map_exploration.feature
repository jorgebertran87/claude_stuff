@map
Feature: Map Exploration
  As a player
  I want to explore a tile-based map with my character
  So that I can walk around, avoid walls, and find NPCs

  Scenario: Full game world exploration
    Given the map has loaded
    Then the game canvas is visible
    And the map is 5 tiles wide and 5 tiles tall
    And the player is at grid position (0, 0)
    And the player is facing south
    And an NPC named Sphinx is at grid position (2, 0)
    And the NPC Sphinx is facing south
    When the player moves east
    Then the player is at grid position (1, 0)
    And the player is facing east
    When the player moves south
    Then the player is at grid position (1, 1)
    And the player is facing south
    When the player moves north
    Then the player is at grid position (1, 0)
    And the player is facing north
    When the player moves west
    Then the player is at grid position (0, 0)
    And the player is facing west
    When the player moves south
    Then the player stays at grid position (0, 0)
