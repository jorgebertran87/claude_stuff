Feature: Search prompt does not instruct web search
  As the system
  I want the search skill prompt to match DeepSeek's actual capabilities
  So that the model answers from its training knowledge instead of hallucinating tool calls

  The search skill is the fallback for any order that doesn't match bus, music, or
  weather intents. It was originally a Claude Code slash command that relied on
  `allowed-tools: WebSearch` and `$ARGUMENTS` substitution, neither of which
  exist in the DeepSeek backend. The prompt must describe what the model CAN do
  (answer from knowledge), not what it CAN'T (search Google).

  Scenario: Search prompt does not mention web search
    Given an order containing "quién fue Cervantes"
    When the system prompt is assembled
    Then the prompt does not contain "Google"
    And the prompt does not contain "search query"
    And the prompt does not contain "$ARGUMENTS"

  Scenario: Search prompt tells the model to answer from knowledge
    Given an order containing "qué es una referencia catastral"
    When the system prompt is assembled
    Then the prompt does not contain "Busca en"
    And the prompt contains "conocimiento" or "knowledge" or "training data"
