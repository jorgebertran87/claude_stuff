Feature: DeepSeek normalizer
  As the prices comparer
  I want a purchased order's store-brand line names cleaned up by DeepSeek
  So that its items match what the supermarkets sell, without the Claude CLI

  Scenario: Store-brand names are rewritten to generic product names
    Given a mock DeepSeek API that cleans the order to "leche, pan"
    And a DeepSeek normalizer pointed at the mock
    When I normalize an order of "IFA ELIGES Leche Entera, 1L" and "RUIPAN Pan Molde, 800G"
    Then the cleaned names are "leche" and "pan"

  Scenario: Each line keeps its quantity, price and size
    Given a mock DeepSeek API that cleans the order to "leche" keeping quantity 2 and price 1.49
    And a DeepSeek normalizer pointed at the mock
    When I normalize an order of "IFA ELIGES Leche Entera, 1L" with quantity 2 priced 1.49
    Then the cleaned line "leche" keeps quantity 2, price 1.49 and size 1 litre

  Scenario: An HTTP error makes normalization fail
    Given a mock DeepSeek API that returns HTTP 500
    And a DeepSeek normalizer pointed at the mock
    When I normalize an order of "IFA ELIGES Leche Entera, 1L"
    Then the normalization fails

  Scenario: A reply without a product list makes normalization fail
    Given a mock DeepSeek API that replies "sorry, I cannot help with that"
    And a DeepSeek normalizer pointed at the mock
    When I normalize an order of "IFA ELIGES Leche Entera, 1L"
    Then the normalization fails
