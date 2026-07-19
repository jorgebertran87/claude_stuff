@sound
Feature: Sound Effects
  As a player
  I want background music and sound effects
  So that the game feels polished and immersive

  Scenario: Background music starts after boot
    When the game starts
    Then background music begins playing

  Scenario: Footstep on movement
    Given the map has loaded
    When the player moves east
    Then a footstep sound is triggered

  Scenario: Battle start jingle
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    Then a battle start sound is triggered

  Scenario: Victory jingle
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "Zeus"
    Then a victory sound is triggered

  Scenario: Defeat tone
    Given the game loads with the API backend
    When the player moves east
    And the player presses space to interact
    And a battle question is displayed
    When the player answers "WrongAnswer"
    Then a defeat sound is triggered
