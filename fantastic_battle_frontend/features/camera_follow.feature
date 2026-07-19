@camera
Feature: Camera Follow
  As a player
  I want the camera to follow my character
  So that the game world feels alive and I can explore beyond the initial view

  Scenario: Camera is centered on player at start
    Given the map has loaded
    Then the camera is centered on the player's position

  Scenario: Camera follows player during movement
    Given the map has loaded
    When the player moves east
    Then the camera is centered on the player's position

  Scenario: Camera is clamped to map bounds
    Given the map has loaded
    Then the camera does not scroll beyond the map boundary
