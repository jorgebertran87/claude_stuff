Feature: Glovo source
  As the prices comparer
  I want to read baskets from the user's Glovo order history
  So that a past order can be compared without retyping the list

  Scenario: Fetching the last order returns its products, store and paid total
    Given a mock Glovo API with an order from "Dia" of "milk x2, bread" paid 3.50
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then the basket has "milk" with quantity 2
    And the basket has "bread" with quantity 1
    And the basket was bought at "Dia"
    And the basket was paid 3.50

  Scenario: Per-item prices are captured from the order detail
    Given a mock Glovo API with an order from "Dia" of "milk" paid 1.50
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then the item "milk" is priced 1.50

  Scenario: Fetching by a store word returns the latest order from that store
    Given a mock Glovo API with order "1001" from "McDonald's" of "fries" paid 8.50 and order "1002" from "Supermercados El Jamón" of "milk" paid 1.20
    And a Glovo source pointed at the mock
    When I fetch the order matching "jamon"
    Then the basket has "milk" with quantity 1
    And the basket was bought at "Supermercados El Jamón"

  Scenario: A word matching no store is reported as no order found
    Given a mock Glovo API with an order from "Dia" of "milk" paid 1.20
    And a Glovo source pointed at the mock
    When I fetch the order matching "carrefour"
    Then no order is found

  Scenario: An empty order history is reported as no order found
    Given a mock Glovo API with no orders
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then no order is found

  Scenario: An HTTP error reports Glovo as unavailable
    Given a mock Glovo API that returns HTTP 500
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then the fetch reports Glovo is unavailable

  Scenario: A malformed response reports Glovo as unavailable
    Given a mock Glovo API that returns invalid JSON
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then the fetch reports Glovo is unavailable

  Scenario: With no token configured the fetch reports it is not configured
    Given a mock Glovo API with an order from "Dia" of "milk" paid 1.20
    And a Glovo source with no token
    When I fetch the last order
    Then the fetch reports the token is not configured

  Scenario: A rejected token reports it has expired
    Given a mock Glovo API that rejects the token as unauthorized
    And a Glovo source pointed at the mock
    When I fetch the last order
    Then the fetch reports the token has expired

  Scenario: The source identifies itself as Glovo
    Given a mock Glovo API with no orders
    And a Glovo source pointed at the mock
    Then the basket source name is "Glovo"
