@e2e
Feature: Full Game Loop
  As a player
  I want to experience a complete game cycle
  So that I can explore, fight, and return to the map

  Scenario: Complete a battle with a correct answer and return to the map
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "Zeus"
    And the battle outcome "Victory" is displayed
    Then the player returns to the map at grid position (1, 0)

  Scenario: Complete a battle with a wrong answer and return to the map
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "WrongAnswer"
    And the battle outcome "Defeat" is displayed
    Then the player returns to the map at grid position (1, 0)
