Feature: Claude CLI integration
  As the system
  I want to invoke the real claude CLI and parse its JSON output
  So that orders are answered and token usage is logged without any stub

  Scenario: A simple order returns a non-empty result
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "qué hora es"
    Then the returned string is non-empty

  Scenario: The token log file is created on first use
    Given the claude CLI is available and authenticated
    And no token log file exists yet
    When ClaudeCodeHandler handles "di hola"
    Then the token log file exists on disk

  Scenario: The token log file records the order and token counts
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "di hola"
    Then the token log contains the text "di hola"
    And the token log contains "input:"
    And the token log contains "output:"
    And the token log contains "total:"
    And the token log contains "cost:"

  Scenario: Two consecutive orders append two separate lines to the log
    Given the claude CLI is available and authenticated
    When ClaudeCodeHandler handles "primera orden"
    And ClaudeCodeHandler handles "segunda orden"
    Then the token log file has exactly 2 lines

  # ── Pure-function scenarios (no real claude CLI needed) ────────────────────────

  Scenario Outline: detect_intent routes orders to the correct intent
    When detect_intent is called with "<order>"
    Then the detected intent is "<intent>"

    Examples:
      | order                       | intent  |
      | quiero coger el autobús     | bus     |
      | en qué parada bajo          | bus     |
      | la linea 5 sale tarde       | bus     |
      | escucha musica hoy          | music   |
      | canción preciosa            | music   |
      | mi playlist de jazz         | music   |
      | reproduce ahora mismo       | music   |
      | lluvia intensa esperada     | weather |
      | temperatura muy alta        | weather |
      | hace calor en agosto        | weather |
      | hace frio esta tarde        | weather |
      | el clima es bueno           | weather |
      | el sol brilla fuerte        | weather |
      | what is the weather today   | weather |
      | hola qué tal                | search  |

  Scenario: strip_frontmatter removes YAML front matter
    When strip_frontmatter is called with "---\nkey: val\n---\nbody text"
    Then the stripped text is "body text"

  Scenario: strip_frontmatter passes through content with no front matter
    When strip_frontmatter is called with "plain content"
    Then the stripped text is "plain content"

  Scenario: extract_u64 parses an integer field from JSON
    When extract_u64 parses key "input_tokens" with value 42 from json
    Then the u64 result is 42

  Scenario: extract_str parses an unquoted value from JSON
    When extract_str parses key "flag" with unquoted value "false" from json
    Then the string result is "false"

  Scenario: load_skill returns the content of an existing skill file
    Given a skill file "ci-test-skill" with content "Test skill content"
    When load_skill is called for "ci-test-skill"
    Then the skill content equals "Test skill content"

  Scenario: load_prompt always includes the language rule
    When load_prompt is called for "hola mundo"
    Then the prompt contains "idioma"

