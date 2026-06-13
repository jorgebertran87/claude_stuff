Feature: Order normalization
  As a shopper comparing a Glovo order
  I want its store-brand product names cleaned up before the comparison
  So that the items actually match what the supermarkets sell

  Scenario: A Glovo order is compared using the cleaned product names
    Given a store "Mercadona" selling "leche" at 1.10
    And a Glovo order from "Dia" of "IFA ELIGES Leche Entera, 1L"
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply shows "Mercadona" with total 1.10

  Scenario: Quantities survive normalization
    Given a store "Mercadona" selling "leche" at 1.10
    And a Glovo order from "Dia" of "IFA ELIGES Leche Entera, 1L x3"
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply shows "Mercadona" with total 3.30

  Scenario: When normalization fails the raw items are still compared
    Given a store "Mercadona" selling "IFA ELIGES Leche Entera, 1L" at 1.10
    And a Glovo order from "Dia" of "IFA ELIGES Leche Entera, 1L"
    And the normalizer is unavailable
    When I message "/glovo"
    Then the reply shows "Mercadona" with total 1.10

  Scenario: A typed basket is compared as written, without normalization
    Given a store "Mercadona" selling "milk" at 1.10
    And the normalizer cleans "milk" to "something the store does not sell"
    When I message "milk"
    Then the reply shows "Mercadona" with total 1.10

  Scenario: The cleaned order lists each item with the price paid on Glovo
    Given a store "Mercadona" selling "leche" at 1.10
    And a Glovo order from "Dia" of "IFA ELIGES Leche Entera, 1L" priced 1.49
    And the normalizer cleans "IFA ELIGES Leche Entera, 1L" to "leche"
    When I message "/glovo"
    Then the reply lists "leche" priced 1.49
