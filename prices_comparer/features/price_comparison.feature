Feature: Price comparison
  As a shopper
  I want the total price of my product list computed for each supermarket
  So that I can see which store is cheapest for my whole basket

  Scenario: Every store has every product so all totals are complete
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Dia" selling "milk" at 1.05 and "bread" at 1.00
    When I compare the basket "milk, bread"
    Then the total for "Mercadona" is 2.00
    And the total for "Dia" is 2.05
    And the cheapest store is "Mercadona"

  Scenario: Quantities multiply the unit price
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    When I compare the basket "milk x3, bread"
    Then the total for "Mercadona" is 4.20

  Scenario: A store missing a product reports an incomplete total
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Maskom" selling "milk" at 1.00
    When I compare the basket "milk, bread"
    Then the total for "Mercadona" is 2.00
    And the total for "Maskom" is incomplete, missing "bread"
    And the cheapest store is "Mercadona"

  Scenario: Incomplete stores never win the cheapest comparison
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Maskom" selling "milk" at 0.10
    When I compare the basket "milk, bread"
    Then the cheapest store is "Mercadona"

  Scenario: A store that fails to respond is reported as unavailable
    Given a store "Mercadona" selling "milk" at 1.10
    And a store "Carrefour" that fails to respond
    When I compare the basket "milk"
    Then the total for "Mercadona" is 1.10
    And the store "Carrefour" is reported as unavailable
    And the cheapest store is "Mercadona"

  Scenario: Comparing an empty basket is rejected
    Given a store "Mercadona" selling "milk" at 1.10
    When I compare the basket ""
    Then the comparison fails with an empty basket error

  Scenario: A product found in no store is reported as missing everywhere
    Given a store "Mercadona" selling "milk" at 1.10
    And a store "Dia" selling "milk" at 1.05
    When I compare the basket "milk, caviar"
    Then the product "caviar" is reported as missing in every store
    And the total for "Mercadona" is incomplete, missing "caviar"
