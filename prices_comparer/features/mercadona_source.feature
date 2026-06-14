Feature: Mercadona source
  As the prices comparer
  I want the per-unit price of products on Mercadona's online shop
  So that Mercadona can take part in per-unit basket comparisons

  Scenario: A matched product returns its per-unit price
    Given a mock Mercadona API where searching "leche" finds "Leche entera" at 0.96 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "leche"
    Then the per-unit price is 0.96 per litre

  Scenario: The first match wins when several products match
    Given a mock Mercadona API where searching "leche" finds "Leche entera" at 0.96 per litre and "Leche desnatada" at 0.89 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "leche"
    Then the per-unit price is 0.96 per litre

  Scenario: The cheapest match in the wanted measure wins
    Given a mock Mercadona API where searching "leche" finds "Leche entera" at 0.96 per litre and "Leche desnatada" at 0.89 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 0.89 per litre

  Scenario: A cheaper match in another measure is ignored
    Given a mock Mercadona API where searching "leche" finds "Leche entera" at 0.96 per litre and "Leche en polvo" at 0.50 per kilo
    And a Mercadona source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 0.96 per litre

  Scenario: A cola zero search always picks Coca-Cola, even if another brand is cheaper
    Given a mock Mercadona API where searching "cola zero" finds "Hacendado Cola Zero" at 0.50 per litre and "Coca-Cola Zero" at 1.20 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "cola zero" measured in litres
    Then the per-unit price is 1.20 per litre

  Scenario: A zero-priced listing is ignored
    Given a mock Mercadona API where searching "leche" finds "Leche gratis" at 0.00 per litre and "Leche entera" at 0.96 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "leche" measured in litres
    Then the per-unit price is 0.96 per litre

  Scenario: A cola query with extra words still resolves to Coca-Cola
    Given a mock Mercadona API where searching "refresco cola zero" finds "Refresco cola Hacendado Zero" at 0.40 per litre and "Coca-Cola Zero" at 1.20 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "refresco cola zero" measured in litres
    Then the per-unit price is 1.20 per litre

  Scenario: A chocolate query is not mistaken for cola
    Given a mock Mercadona API where searching "batido chocolate" finds "Batido de chocolate Hacendado" at 1.29 per litre
    And a Mercadona source pointed at the mock
    When I ask the price of "batido chocolate" measured in litres
    Then the per-unit price is 1.29 per litre

  Scenario: A cheaper but unrelated result is rejected
    Given a mock Mercadona API where searching "jamón serrano" finds "Hueso garrón" at 4.00 per kilo and "Jamón serrano reserva" at 10.00 per kilo
    And a Mercadona source pointed at the mock
    When I ask the price of "jamón serrano" measured in kilos
    Then the per-unit price is 10.00 per kilo

  Scenario: A singular product still matches a plural query
    Given a mock Mercadona API where searching "manzanas" finds "Manzana Golden" at 2.19 per kilo
    And a Mercadona source pointed at the mock
    When I ask the price of "manzanas" measured in kilos
    Then the per-unit price is 2.19 per kilo

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
