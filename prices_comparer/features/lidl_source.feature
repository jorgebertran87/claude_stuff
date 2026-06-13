Feature: Lidl source
  As the prices comparer
  I want the per-unit price of products on Lidl's online shop
  So that Lidl can take part in per-unit basket comparisons

  Scenario: A matched product returns its per-unit price
    Given a mock Lidl API where searching "milk" finds "Leche entera Milbona 1L" at 0.99
    And a Lidl source pointed at the mock
    When I ask the price of "milk"
    Then the per-unit price is 0.99 per litre

  Scenario: The first match wins when several products match
    Given a mock Lidl API where searching "milk" finds "Leche entera Milbona 1L" at 0.99 and "Leche desnatada Milbona 1L" at 0.95
    And a Lidl source pointed at the mock
    When I ask the price of "milk"
    Then the per-unit price is 0.99 per litre

  Scenario: A product with no matches is reported as not sold
    Given a mock Lidl API where searching "caviar" finds nothing
    And a Lidl source pointed at the mock
    When I ask the price of "caviar"
    Then the product is reported as not sold

  Scenario: A product without a recognisable size has no per-unit price
    Given a mock Lidl API where searching "milk" finds "Leche entera Milbona" at 0.99
    And a Lidl source pointed at the mock
    When I ask the price of "milk"
    Then the product is reported as not sold

  Scenario: An HTTP error makes the lookup fail
    Given a mock Lidl API that returns HTTP 500
    And a Lidl source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: A malformed response makes the lookup fail
    Given a mock Lidl API that returns invalid JSON
    And a Lidl source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: The source identifies itself as Lidl
    Given a mock Lidl API where searching "milk" finds "Leche entera Milbona 1L" at 0.99
    And a Lidl source pointed at the mock
    Then the store name is "Lidl"
