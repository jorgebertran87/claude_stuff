Feature: Dia source
  As the prices comparer
  I want to look up product prices on Dia's online shop through FlareSolverr
  So that Dia can take part in basket comparisons despite Cloudflare

  Scenario: A matched product returns its unit price
    Given a mock FlareSolverr where searching Dia for "milk" finds "Leche entera DIA" at 1.05
    And a Dia source pointed at the mock
    When I ask the price of "milk"
    Then the price is 1.05

  Scenario: The first match wins when several products match
    Given a mock FlareSolverr where searching Dia for "milk" finds "Leche entera DIA" at 1.05 and "Leche desnatada DIA" at 0.89
    And a Dia source pointed at the mock
    When I ask the price of "milk"
    Then the price is 1.05

  Scenario: A product with no matches is reported as not sold
    Given a mock FlareSolverr where searching Dia for "caviar" finds nothing
    And a Dia source pointed at the mock
    When I ask the price of "caviar"
    Then the product is reported as not sold

  Scenario: An HTTP error from FlareSolverr makes the lookup fail
    Given a mock FlareSolverr that returns HTTP 500
    And a Dia source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: A response without product data makes the lookup fail
    Given a mock FlareSolverr where searching Dia returns a page with no product data
    And a Dia source pointed at the mock
    When I ask the price of "milk"
    Then the lookup fails

  Scenario: The source identifies itself as Dia
    Given a mock FlareSolverr where searching Dia for "milk" finds "Leche entera DIA" at 1.05
    And a Dia source pointed at the mock
    Then the store name is "Dia"
