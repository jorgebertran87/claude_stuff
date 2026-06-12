Feature: Order reply
  As a shopper who buys through Glovo
  I want to ask the bot to compare one of my past orders
  So that I see what the basket would cost elsewhere without retyping it

  Scenario: The last order is compared using its store as where I bought
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a store "Dia" selling "milk" at 1.05 and "bread" at 1.30
    And a Glovo order from "Dia" of "milk x2, bread"
    When I message "/glovo"
    Then the reply shows "Mercadona" with total 3.10
    And the reply shows "Dia" with total 3.40
    And the reply shows "Dia" as where I bought, with total 3.40
    And the reply says I could have saved 0.30 buying at "Mercadona"

  Scenario: A specific order is compared by its id
    Given a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a Glovo order "1001" of "milk"
    And a Glovo order "1002" of "bread"
    When I message "/glovo 1002"
    Then the reply shows "Mercadona" with total 0.90

  Scenario: The paid total is shown next to today's prices
    Given a store "Mercadona" selling "milk" at 1.10
    And a Glovo order from "Mercadona" of "milk" paid 1.80
    When I message "/glovo"
    Then the reply says I paid 1.80 on Glovo

  Scenario: An order from a store that is not compared still shows all totals
    Given a store "Mercadona" selling "milk" at 1.10
    And a store "Dia" selling "milk" at 1.05
    And a Glovo order from "Burger King" of "milk"
    When I message "/glovo"
    Then the reply shows "Mercadona" with total 1.10
    And the reply says "Burger King" is not a compared store

  Scenario: No matching order gets a clear answer
    Given a store "Mercadona" selling "milk" at 1.10
    And an empty Glovo order history
    When I message "/glovo"
    Then the reply says no Glovo order was found

  Scenario: Glovo being unreachable gets a clear answer
    Given a store "Mercadona" selling "milk" at 1.10
    And a Glovo source that fails to respond
    When I message "/glovo"
    Then the reply says Glovo could not be reached

  Scenario: A typed basket still works as before
    Given a store "Mercadona" selling "milk" at 1.10
    And an empty Glovo order history
    When I message "milk"
    Then the reply shows "Mercadona" with total 1.10
