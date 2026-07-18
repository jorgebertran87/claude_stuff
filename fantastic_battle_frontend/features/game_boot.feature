Feature: Game Boot
  As a player
  I want the game to initialize and show a title screen
  So that I know the game has loaded successfully

  Scenario: The game canvas appears with the title text
    When the game starts
    Then the game canvas is visible
    And the title "Fantastic Battle" is displayed
