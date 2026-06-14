Feature: DeepSeek product selector
  As the prices comparer
  I want DeepSeek to pick the store product that best matches what I bought
  So that the comparison uses the right item, not just the cheapest near-match

  Scenario: DeepSeek picks the best matching candidate
    Given a mock DeepSeek API that selects candidate 1
    And a DeepSeek selector pointed at the mock
    When I select for "Leche entera desnatada 1L" among "Leche entera", "Leche desnatada"
    Then the chosen candidate is "Leche desnatada"

  Scenario: A reply that is not a valid index selects nothing
    Given a mock DeepSeek API that replies "I think the first one"
    And a DeepSeek selector pointed at the mock
    When I select for "Leche entera 1L" among "Leche entera", "Leche desnatada"
    Then nothing is selected

  Scenario: An out-of-range index selects nothing
    Given a mock DeepSeek API that selects candidate 9
    And a DeepSeek selector pointed at the mock
    When I select for "Leche entera 1L" among "Leche entera", "Leche desnatada"
    Then nothing is selected

  Scenario: An HTTP error selects nothing
    Given a mock DeepSeek API that returns HTTP 500
    And a DeepSeek selector pointed at the mock
    When I select for "Leche entera 1L" among "Leche entera", "Leche desnatada"
    Then nothing is selected
