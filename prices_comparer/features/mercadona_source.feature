Feature: Mercadona source
  As the prices comparer
  I want the per-unit price of products on Mercadona's online shop
  So that Mercadona can take part in per-unit basket comparisons

  Scenario: A matched product returns its per-unit price
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 0.96 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the per-unit price is 0.96 per litre

  Scenario: The first match wins when several products match
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 0.96 per litre and "Leche desnatada" at 0.89 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the per-unit price is 0.96 per litre

  Scenario: The cheapest match in the wanted measure wins
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 0.96 per litre and "Leche desnatada" at 0.89 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "milk" measured in litres
    Then the per-unit price is 0.89 per litre

  Scenario: A cheaper match in another measure is ignored
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 0.96 per litre and "Leche en polvo" at 0.50 per kilo
    And a Mercadona source pointed at the mock
    When I ask the price of "milk" measured in litres
    Then the per-unit price is 0.96 per litre

  Scenario: A cola zero search always picks Coca-Cola, even if another brand is cheaper
    Given a mock Mercadona API where searching "cola zero" finds "Hacendado Cola Zero" at 0.50 per litre and "Coca-Cola Zero" at 1.20 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "cola zero" measured in litres
    Then the per-unit price is 1.20 per litre

  Scenario: A product with no matches is reported as not sold
    Given a mock Mercadona API where searching "caviar" finds nothing
    And a Mercadona source pointed at the mock
    When I ask the price of "caviar"
    Then the product is reported as not sold

  Scenario: An HTTP error makes the lookup fail
    Given a mock Mercadona API that returns HTTP 500
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: A malformed response makes the lookup fail
    Given a mock Mercadona API that returns invalid JSON
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: The source identifies itself as Mercadona
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 0.96 per litre
    And a Mercadona source pointed at the mock
    Then the store name is "Mercadona"
