Feature: Battle Service
  As a human player
  I want to choose a theme and battle AI-controlled players that question me on it
  So that I win each battle by answering correctly, or am defeated if I fail

  # ── Choosing the theme ──────────────────────────────────────────────────────

  Scenario: The human player chooses the theme for the game
    When the human player chooses the theme "Greek mythology"
    Then the chosen theme is "Greek mythology"

  Scenario: A blank theme is rejected
    When the human player chooses a blank theme
    Then the theme is rejected because a theme is required

  # ── AI players ──────────────────────────────────────────────────────────────

  Scenario: An AI-controlled player enters the game with a name
    When an AI player named "Sphinx" enters the game
    Then the game has an AI player named "Sphinx"

  Scenario: An AI player with a blank name is rejected
    When an AI player with a blank name tries to enter the game
    Then the player is rejected because a player needs a name

  # ── Battling ────────────────────────────────────────────────────────────────

  Scenario: An AI player opens the battle by asking a question about the theme
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    When the human player battles "Sphinx"
    Then the human player is asked "Who rules Mount Olympus?"

  Scenario: Each AI player asks its own question
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And an AI player named "Minotaur"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And "Minotaur" will ask "Who built the labyrinth?" with correct answer "Daedalus" for the theme "Greek mythology"
    When the human player battles "Sphinx"
    And the human player battles "Minotaur"
    Then "Sphinx" posed the question "Who rules Mount Olympus?"
    And "Minotaur" posed the question "Who built the labyrinth?"

  Scenario: A rematch against the same AI player poses the same question
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    When the human player battles "Sphinx"
    And the human player battles "Sphinx" again
    Then both battles pose the question "Who rules Mount Olympus?"

  # ── Battle outcome ──────────────────────────────────────────────────────────

  Scenario: Answering the question correctly wins the battle
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And the human player battles "Sphinx"
    When the human player answers "Zeus"
    Then the human player wins the battle

  Scenario: Answering the question incorrectly defeats the human player
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And the human player battles "Sphinx"
    When the human player answers "Poseidon"
    Then the human player is defeated

  Scenario: Answer matching ignores letter case
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And the human player battles "Sphinx"
    When the human player answers "zeus"
    Then the human player wins the battle

  Scenario: Answer matching ignores surrounding whitespace
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And the human player battles "Sphinx"
    When the human player answers "  Zeus  "
    Then the human player wins the battle

  Scenario: A finished battle cannot be answered again
    Given the human player has chosen the theme "Greek mythology"
    And an AI player named "Sphinx"
    And "Sphinx" will ask "Who rules Mount Olympus?" with correct answer "Zeus" for the theme "Greek mythology"
    And the human player battles "Sphinx"
    When the human player answers "Zeus"
    When the human player tries to answer again
    Then the answer is rejected because the battle is already over
    And the human player still wins the battle
