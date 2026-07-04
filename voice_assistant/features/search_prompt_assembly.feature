Feature: Search prompt instructs the model about available tools
  As the system
  I want the search skill prompt to tell DeepSeek about its web tools
  So that the model knows when to use web_search and url_fetch

  The search skill is the fallback for any order that doesn't match bus, music, or
  weather intents. The model now has access to web_search (DuckDuckGo) and
  url_fetch tools. The prompt should mention these capabilities.

  Scenario: Search prompt mentions web_search tool
    Given an order containing "quién fue Cervantes"
    When the system prompt is assembled
    Then the prompt contains "web_search"

  Scenario: Search prompt instructs the model to be concise
    Given an order containing "qué es una referencia catastral"
    When the system prompt is assembled
    Then the prompt contains "conciso"
