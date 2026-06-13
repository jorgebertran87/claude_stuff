Feature: Price comparison
  As a shopper
  I want each product priced per unit at every supermarket
  So that I can see which store is cheapest per litre, kilo or unit

  Scenario: A product is priced per unit at each store
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Dia" pricing "milk" at 1.10 per litre
    When I compare the basket "milk"
    Then "milk" costs 0.96 per litre at "Mercadona"
    And "milk" costs 1.10 per litre at "Dia"

  Scenario: The cheapest store is marked for each product
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Dia" pricing "milk" at 1.10 per litre
    When I compare the basket "milk"
    Then the cheapest store for "milk" is "Mercadona"

  Scenario: A store that does not sell a product shows no price
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Lidl" that does not sell "milk"
    When I compare the basket "milk"
    Then "milk" has no price at "Lidl"

  Scenario: A store that fails to respond shows no price
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Carrefour" that fails to respond
    When I compare the basket "milk"
    Then "milk" costs 0.96 per litre at "Mercadona"
    And "milk" has no price at "Carrefour"

  Scenario: A price in a different unit cannot win on a lower number
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a store "Dia" pricing "milk" at 0.50 per kilo
    When I compare the basket "milk"
    Then the cheapest store for "milk" is "Mercadona"

  Scenario: Comparing an empty basket is rejected
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    When I compare the basket ""
    Then the comparison fails with an empty basket error
