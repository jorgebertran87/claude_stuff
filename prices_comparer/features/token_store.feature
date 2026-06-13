Feature: Token store
  As the prices comparer
  I want the Glovo token kept in a shared file
  So that the bot and the traffic capturer can hand it over without a restart

  Scenario: A fresh store has no token
    Given a fresh token store
    Then the store has no token

  Scenario: A saved token can be read back
    Given a fresh token store
    When the token "abc123" is saved
    Then the current token is "abc123"

  Scenario: Saving a token replaces the previous one
    Given a token store holding "old-token"
    When the token "new-token" is saved
    Then the current token is "new-token"

  Scenario: A token written by one instance is seen by another on the same file
    Given a token store holding "shared-token"
    When another store opens the same file
    Then the current token is "shared-token"

  Scenario: A blank token counts as no token
    Given a fresh token store
    When the token "   " is saved
    Then the store has no token
