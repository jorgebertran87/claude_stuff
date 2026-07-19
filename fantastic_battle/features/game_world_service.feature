Feature: Game World Service
  As a human player
  I want to explore a tile-based map, move around, and interact with NPCs
  So that I can encounter AI opponents to battle

  # ── Joining the game ─────────────────────────────────────────────────────

  Scenario: Joining a game with a theme places the player at the starting position
    When the human player joins the game with the theme "Greek mythology"
    Then the player is at position (5, 5)
    And the player is facing south
    And the session has the theme "Greek mythology"

  Scenario: Joining with a theme places themed NPCs on the map
    Given the NPC name generator provides the names Socrates, Aristotle, Plato for the theme "Greek mythology"
    When the human player joins the game with the theme "Greek mythology" and 3 questions
    Then the session has 3 NPCs
    And the NPCs are named Socrates, Aristotle, and Plato

  Scenario: Joining with a different theme places different NPCs
    Given the NPC name generator provides the names Ada Lovelace, Grace Hopper for the theme "Computer Science"
    When the human player joins the game with the theme "Computer Science" and 2 questions
    Then the NPCs are named Ada Lovelace and Grace Hopper

  # ── Movement ─────────────────────────────────────────────────────────────

  Scenario: Moving north decreases the y coordinate
    Given the human player has joined the game
    When the player moves north
    Then the player is at position (5, 4)
    And the player is facing north

  Scenario: Moving south increases the y coordinate
    Given the human player has joined the game
    When the player moves south
    Then the player is at position (5, 6)
    And the player is facing south

  Scenario: Moving east increases the x coordinate
    Given the human player has joined the game
    When the player moves east
    Then the player is at position (6, 5)
    And the player is facing east

  Scenario: Moving west decreases the x coordinate
    Given the human player has joined the game
    When the player moves west
    Then the player is at position (4, 5)
    And the player is facing west

  # ── Collision ────────────────────────────────────────────────────────────

  Scenario: Moving into a wall is rejected
    Given the human player has joined the game
    And there is a wall at position (5, 6)
    When the player moves south
    Then the move is rejected because the tile is not walkable
    And the player stays at position (5, 5)

  Scenario: Moving outside the map boundary is rejected
    Given the human player has joined the game on a 3 by 3 map
    When the player moves north
    Then the move is rejected because the position is out of bounds
    And the player stays at position (0, 0)

  # ── NPC Interaction ──────────────────────────────────────────────────────

  Scenario: Interacting with an NPC in front of the player returns the NPC
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 6)
    When the player interacts
    Then the interaction returns the NPC named "Sphinx"

  Scenario: Interacting with no NPC in front of the player returns nothing
    Given the human player has joined the game
    When the player interacts
    Then the interaction returns no NPC

  Scenario: Interacting ignores an NPC behind the player
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 4)
    When the player interacts
    Then the interaction returns no NPC

  # ── NPC Status ─────────────────────────────────────────────────────────────

  Scenario: NPCs start with Active status
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 6)
    Then the NPC "Sphinx" has status Active

  Scenario: Defeating an NPC with a correct answer marks it DefeatedCorrect
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 6)
    When the player defeats the NPC "Sphinx" with outcome Victory
    Then the NPC "Sphinx" has status DefeatedCorrect

  Scenario: Defeating an NPC with an incorrect answer marks it DefeatedIncorrect
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 6)
    When the player defeats the NPC "Sphinx" with outcome Defeat
    Then the NPC "Sphinx" has status DefeatedIncorrect

  Scenario: Interacting with a defeated NPC returns nothing
    Given the human player has joined the game
    And there is an NPC named "Sphinx" at position (5, 6) that has been defeated
    When the player interacts
    Then the interaction returns no NPC

  # ── Question Count ─────────────────────────────────────────────────────────

  Scenario: Joining with a question count generates exactly that many NPCs
    Given the NPC name generator provides the names Socrates, Aristotle, Plato, Homer, Herodotus for the theme "Greek mythology"
    When the human player joins the game with the theme "Greek mythology" and 3 questions
    Then the session has 3 NPCs
    And the NPCs are named Socrates, Aristotle, and Plato

  Scenario: Joining with more questions than spawn positions generates extra NPCs
    Given the NPC name generator provides the names A, B, C, D, E, F, G for the theme "Greek mythology"
    And the map has 3 NPC spawn positions
    When the human player joins the game with the theme "Greek mythology" and 7 questions
    Then the session has 7 NPCs
