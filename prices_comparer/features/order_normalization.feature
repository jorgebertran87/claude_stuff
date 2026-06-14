Feature: Order normalization
  As a shopper comparing a Glovo order
  I want its store-brand product names cleaned up before the comparison
  So that the items actually match what the supermarkets sell

  Scenario: A Glovo order is compared using the cleaned product names
    Given a store "Mercadona" pricing "leche" at 0.96 per litre
    And a Glovo order of "IFA ELIGES Leche Entera, 1L"
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply shows "leche" at 0.96 per litre for "Mercadona"

  Scenario: When normalization fails the raw items are still compared
    Given a store "Mercadona" pricing "IFA ELIGES Leche Entera, 1L" at 0.96 per litre
    And a Glovo order of "IFA ELIGES Leche Entera, 1L"
    And the normalizer is unavailable
    When I message "/glovo"
    Then the reply shows "IFA ELIGES Leche Entera, 1L" at 0.96 per litre for "Mercadona"

  Scenario: A typed basket is compared as written, without normalization
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And the normalizer cleans "milk" to "something the store does not sell"
    When I message "milk"
    Then the reply shows "milk" at 0.96 per litre for "Mercadona"

  Scenario: The cleaned order shows the per-unit price paid on Glovo
    Given a store "Mercadona" pricing "leche" at 0.96 per litre
    And a Glovo order of "IFA ELIGES Leche Entera, 1L" priced 1.49
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply shows the Glovo price 1.49 per litre

  Scenario: A multipack with a unit size is priced by its total volume
    Given a store "Mercadona" pricing "mayonesa" at 2.00 per litre
    And a Glovo order of "IFA ELIGES Mayonesa Sobre 20ML, Pk-12" priced 2.40
    And the normalizer cleans "IFA ELIGES Mayonesa Sobre 20ML, Pk-12" to "mayonesa"
    When I message "/glovo"
    Then the reply shows the Glovo price 10.00 per litre

  Scenario: The Glovo price competes in the comparison and can be the cheapest
    Given a store "Mercadona" pricing "leche" at 0.96 per litre
    And a Glovo order of "IFA ELIGES Leche Entera, 1L" priced 0.80
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply marks "Glovo" as the cheapest

  Scenario: The Glovo item's measurement decides how the stores are compared
    Given a store "Mercadona" pricing "huevos" at 0.30 per each
    And a store "Dia" pricing "huevos" at 2.50 per litre
    And a Glovo order of "Huevos Frescos Pk-12" priced 4.20
    And the normalizer cleans "Huevos Frescos Pk-12" to "huevos"
    When I message "/glovo"
    Then the reply marks "Mercadona" as the cheapest
