Feature: NPC Name Generator
  As the game world service
  I want to generate themed NPC names
  So that opponents match the player's chosen theme

  Scenario: Generating names for a theme returns the requested count
    When the game generates 3 NPC names for the theme "Greek mythology"
    Then 3 names are returned

  Scenario: Generated names are non-empty
    When the game generates 5 NPC names for the theme "Computer Science"
    Then all generated names are non-empty

  Scenario: Generating names for the same theme is deterministic
    When the game generates 5 NPC names for the theme "Greek mythology"
    And the game generates 5 NPC names for the theme "Greek mythology" again
    Then both calls return the same names
