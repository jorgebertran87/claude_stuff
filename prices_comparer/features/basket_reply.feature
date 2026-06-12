Feature: Basket reply
  As a shopper messaging the bot
  I want to send my product list, optionally with the store where I bought it
  So that I get the totals everywhere and know if I overpaid

  Scenario: A basket message is answered with every store's total and the cheapest
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Dia" selling "milk" at 1.05 and "bread" at 1.00
    When I message "milk, bread"
    Then the reply shows "Mercadona" with total 2.00
    And the reply shows "Dia" with total 2.05
    And the reply marks "Mercadona" as the cheapest

  Scenario: Naming the store where I bought shows what I could have saved
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Dia" selling "milk" at 1.05 and "bread" at 1.00
    When I message "milk, bread @ Dia"
    Then the reply shows "Dia" as where I bought, with total 2.05
    And the reply says I could have saved 0.05 buying at "Mercadona"

  Scenario: Buying at the cheapest store is acknowledged
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Dia" selling "milk" at 1.05 and "bread" at 1.00
    When I message "milk, bread @ Mercadona"
    Then the reply says I bought at the cheapest store

  Scenario: A store missing a product is shown as incomplete in the reply
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Lidl" selling "milk" at 0.99
    When I message "milk, bread"
    Then the reply shows "Lidl" as incomplete, missing "bread"

  Scenario: An unreachable store is shown as unavailable in the reply
    Given a store "Mercadona" selling "milk" at 1.10
    And a store "Dia" that fails to respond
    When I message "milk"
    Then the reply shows "Dia" as unavailable

  Scenario: Buying at a store that does not sell everything cannot be compared
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Lidl" selling "milk" at 0.99
    When I message "milk, bread @ Lidl"
    Then the reply says the bought total could not be compared

  Scenario: Naming an unknown store explains which stores are known
    Given a store "Mercadona" selling "milk" at 1.10
    And a store "Dia" selling "milk" at 1.05
    When I message "milk @ Eroski"
    Then the reply says "Eroski" is not a known store
    And the reply lists "Mercadona" and "Dia" as the known stores

  Scenario: An empty message is answered with usage help
    Given a store "Mercadona" selling "milk" at 1.10
    When I message ""
    Then the reply explains how to send a basket
