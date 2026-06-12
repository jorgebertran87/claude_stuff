Feature: Mercadona source
  As the prices comparer
  I want to look up product prices on Mercadona's online shop
  So that Mercadona can take part in basket comparisons

  Scenario: A matched product returns its unit price
    Given a mock Mercadona API where searching "milk" finds "Leche entera Hacendado" at 1.15
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the price is 1.15

  Scenario: The first match wins when several products match
    Given a mock Mercadona API where searching "milk" finds "Leche entera" at 1.15 and "Leche desnatada" at 0.99
    And a Mercadona source pointed at the mock
    When I ask the price of "milk"
    Then the price is 1.15

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
    Given a mock Mercadona API where searching "milk" finds "Leche entera Hacendado" at 1.15
    And a Mercadona source pointed at the mock
    Then the store name is "Mercadona"
