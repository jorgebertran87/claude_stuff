@dialog
Feature: Dialog Box
  As a player
  I want a Pokémon-style dialog box with typewriter text
  So that NPC interactions feel immersive

  Scenario: Dialog box appears during NPC interaction
    Given the game loads with the API backend
    When the player moves east
    And the player triggers an NPC interaction
    Then a dialog box is visible on screen

  Scenario: Typewriter effect reveals text progressively
    Given the game loads with the API backend
    When the player moves east
    And the player triggers an NPC interaction
    Then the dialog box text grows character by character

  Scenario: Dialog is dismissible after text completes
    Given the game loads with the API backend
    When the player moves east
    And the player triggers an NPC interaction
    And the dialog text has finished appearing
    When the player presses space
    Then the dialog box is dismissed
