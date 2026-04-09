Feature: CLI direct order mode
  As a developer or power user
  I want to send a text order directly via the command line
  So that I can test responses without using the microphone

  Scenario: --order flag with a value selects direct order mode
    Given the CLI arguments are "--order pon música"
    When the arguments are parsed
    Then the mode is DirectOrder with text "pon música"

  Scenario: --order flag without a value returns an error
    Given the CLI arguments are "--order"
    When the arguments are parsed
    Then parsing fails with an error

  Scenario: --telegram flag selects Telegram mode
    Given the CLI arguments are "--telegram"
    When the arguments are parsed
    Then the mode is TelegramMode

  Scenario: No arguments selects listen mode
    Given no CLI arguments are provided
    When the arguments are parsed
    Then the mode is ListenMode

  Scenario: --both flag selects both mode
    Given the CLI arguments are "--both"
    When the arguments are parsed
    Then the mode is BothMode

  Scenario: --order takes precedence when combined with other flags
    Given the CLI arguments are "--order hola --telegram"
    When the arguments are parsed
    Then the mode is DirectOrder with text "hola"
