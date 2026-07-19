Feature: Battle API
  As a player
  I want to start and answer battles through the HTTP API
  So that I can engage in combat with NPCs

  Scenario: Interact with NPC starts a battle
    Given a game session exists
    When the client sends a POST request to that session's move endpoint with direction "East"
    And the client sends a POST request to that session's interact endpoint
    Then the response status is 200
    And the response body contains an NPC named "Sphinx"
    And the response body contains a battle with a question

  Scenario: Interact with no NPC does not start a battle
    Given a game session exists
    When the client sends a POST request to that session's interact endpoint
    Then the response status is 200
    And the response body contains no battle

  Scenario: Answer correctly results in Victory
    Given a game session exists
    And a battle has been started for that session
    When the client answers "Zeus"
    Then the response status is 200
    And the outcome is "Victory"

  Scenario: Answer incorrectly results in Defeat
    Given a game session exists
    And a battle has been started for that session
    When the client answers "WrongAnswer"
    Then the response status is 200
    And the outcome is "Defeat"

  Scenario: Answer non-existent battle returns 404
    Given a game session exists
    When the client sends a POST to answer a battle
    Then the response status is 404
