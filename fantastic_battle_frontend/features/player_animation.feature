@animation
Feature: Player Animation
  As a player
  I want my character to animate while walking and face the right direction
  So that movement feels polished and Pokémon-like

  Scenario: Player shows correct idle frame after moving in a direction
    Given the map has loaded
    When the player moves east
    Then the player shows a east-facing idle frame
    And the player is not moving

  Scenario: Player changes frame during movement
    Given the map has loaded
    When the player begins moving east
    Then the player changes to at least 2 different frames during the move

  Scenario: Player returns to idle frame after movement completes
    Given the map has loaded
    When the player moves east
    Then the player returns to an idle frame after movement completes
