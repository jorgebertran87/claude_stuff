Feature: Question Asker
  As the battle system
  I want to obtain questions for NPCs
  So that each battle has themed content

  Scenario: Asking for a question returns the NPC's question
    When the game asks for a question from "Sphinx" for theme "Greek mythology"
    Then the question text is "Who rules Mount Olympus?"
    And the correct answer is "Zeus"

  Scenario: Different NPCs return different questions
    When the game asks for a question from "Sphinx" for theme "Greek mythology"
    And the game asks for a question from "Minotaur" for theme "Greek mythology"
    Then the questions are different
