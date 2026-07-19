@transition
Feature: Battle Transition
  As a player
  I want flash and glow effects when entering battle
  So that battles feel dramatic and Pokémon-like

  Scenario: Screen flashes before battle scene opens
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    Then a flash overlay appears before the battle starts

  Scenario: Interacted NPC glows before battle
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    Then the interacted NPC shows a glow effect
