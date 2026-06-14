Feature: Dia source
  As the prices comparer
  I want the per-unit price of products on Dia's online shop through FlareSolverr
  So that Dia can take part in per-unit basket comparisons despite Cloudflare

  Scenario: A matched product returns its per-unit price
    Given a mock FlareSolverr where searching Dia for "leche" finds "Leche entera 1L" at 1.05
    And a Dia source pointed at the mock
    When I ask the price of "leche"
    Then the per-unit price is 1.05 per litre

  Scenario: The first match wins when several products match
    Given a mock FlareSolverr where searching Dia for "leche" finds "Leche entera 1L" at 1.05 and "Leche desnatada 1L" at 0.89
    And a Dia source pointed at the mock
    When I ask the price of "leche"
    Then the per-unit price is 1.05 per litre

  Scenario: The cheapest match in the wanted measure wins
    Given a mock FlareSolverr where searching Dia for "leche" finds "Leche entera 1L" at 1.05 and "Leche desnatada 1L" at 0.89
    And a Dia source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 0.89 per litre

  Scenario: A cheaper match in another measure is ignored
    Given a mock FlareSolverr where searching Dia for "leche" finds "Leche entera 1L" at 1.05 and "Leche en polvo 1kg" at 0.50
    And a Dia source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 1.05 per litre

  Scenario: A cheaper but unrelated result is rejected
    Given a mock FlareSolverr where searching Dia for "leche" finds "Agua mineral 1L" at 0.30 and "Leche entera 1L" at 1.05
    And a Dia source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 1.05 per litre

  Scenario: A cola search always picks Coca-Cola, even if another brand is cheaper
    Given a mock FlareSolverr where searching Dia for "cola" finds "Hacendado Cola 2L" at 1.00 and "Coca-Cola 2L" at 3.00
    And a Dia source pointed at the mock
    When I ask the price of "cola" measured in litres
    Then the per-unit price is 1.50 per litre

  Scenario: A product with no matches is reported as not sold
    Given a mock FlareSolverr where searching Dia for "caviar" finds nothing
    And a Dia source pointed at the mock
    When I ask the price of "caviar"
    Then the product is reported as not sold

  Scenario: A product without a recognisable size has no per-unit price
    Given a mock FlareSolverr where searching Dia for "leche" finds "Leche entera DIA" at 1.05
    And a Dia source pointed at the mock
    When I ask the price of "leche"
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
    Given a mock FlareSolverr where searching Dia for "milk" finds "Leche entera 1L" at 1.05
    And a Dia source pointed at the mock
    Then the store name is "Dia"
