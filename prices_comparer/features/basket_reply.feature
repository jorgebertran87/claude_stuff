Feature: Basket reply
  As a shopper messaging the bot
  I want my product list priced per unit at each store
  So that I can see where each item is cheapest

  Scenario: Each product shows each store's per-unit price
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Dia" pricing "milk" at 1.10 per litre
    When I message "milk"
    Then the reply shows "milk" at 0.96 per litre for "Mercadona"
    And the reply shows "milk" at 1.10 per litre for "Dia"
    And the reply marks "Mercadona" cheapest for "milk"

  Scenario: A store that does not sell a product shows a dash
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Lidl" that does not sell "milk"
    When I message "milk"
    Then the reply shows "Lidl" with no price

  Scenario: An empty message is answered with usage help
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    When I message ""
    Then the reply explains how to send a basket
