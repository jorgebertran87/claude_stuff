Feature: Battle Repository
  As the battle system
  I want to persist and retrieve active battles
  So that battle state survives across HTTP requests

  Scenario: Saving and finding a battle by session ID
    Given a battle for session "abc" with question "Who rules Mount Olympus?"
    When the battle is saved
    Then the battle can be found by session id "abc"
    And the found battle has question "Who rules Mount Olympus?"

  Scenario: Answering a stored battle changes its outcome
    Given a battle for session "abc" with question "Test?" and answer "Correct"
    When the battle is saved
    And the human player answers "Correct"
    Then the battle outcome is "Victory"

  Scenario: Finding a non-existent battle returns nothing
    When looking for a battle by session id "nonexistent"
    Then no battle is found
