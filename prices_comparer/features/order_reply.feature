Feature: Order reply
  As a shopper who buys through Glovo
  I want a past order priced per unit at each supermarket
  So that I see where each item is cheaper, without retyping it

  Scenario: A Glovo order is compared per unit
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a Glovo order of "milk"
    When I message "/glovo"
    Then the reply shows "milk" at 0.96 per litre for "Mercadona"

  Scenario: The price paid on Glovo is shown
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a Glovo order of "milk" paid 1.20
    When I message "/glovo"
    Then the reply says I paid 1.20 on Glovo

  Scenario: Setting the Glovo token is acknowledged
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    When I message "/glovo_token fresh-token-123"
    Then the reply confirms the Glovo token was saved

  Scenario: Asking for an order with no token explains how to set it
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And Glovo has no token configured
    When I message "/glovo"
    Then the reply says Glovo is not configured

  Scenario: An expired token tells the user to capture a fresh one
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And the Glovo token has expired
    When I message "/glovo"
    Then the reply says the Glovo token has expired

  Scenario: Glovo being unreachable gets a clear answer
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And a Glovo source that fails to respond
    When I message "/glovo"
    Then the reply says Glovo could not be reached

  Scenario: No matching order gets a clear answer
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And an empty Glovo order history
    When I message "/glovo"
    Then the reply says no Glovo order was found

  Scenario: A typed basket still works as before
    Given a store "Mercadona" pricing "milk" at 0.96 per litre
    And an empty Glovo order history
    When I message "milk"
    Then the reply shows "milk" at 0.96 per litre for "Mercadona"
