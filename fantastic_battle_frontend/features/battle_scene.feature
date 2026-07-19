@battle
Feature: Battle Scene
  As a player
  I want to answer questions in a battle scene
  So that I can engage in combat with NPCs

  Scenario: Question is displayed after interacting with an NPC
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    Then a battle question is displayed

  Scenario: Correct answer results in Victory
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "Zeus"
    Then the battle outcome "Victory" is displayed

  Scenario: Wrong answer results in Defeat
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "WrongAnswer"
    Then the battle outcome "Defeat" is displayed
