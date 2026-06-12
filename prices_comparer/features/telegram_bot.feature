Feature: Telegram bot
  As a shopper
  I want to talk to the prices comparer through Telegram
  So that I can check my basket from my phone

  Scenario: A basket message from the configured chat gets a comparison reply
    Given a mock Telegram API delivering "milk, bread" from the configured chat
    And a store "Mercadona" selling "milk" at 1.10 and "bread" at 0.90
    And a Telegram bot connected to the mock
    When the bot processes one round of updates
    Then a reply mentioning "Mercadona" was sent to the configured chat

  Scenario: A message from a different chat is ignored
    Given a mock Telegram API delivering "milk" from an unknown chat
    And a store "Mercadona" selling "milk" at 1.10
    And a Telegram bot connected to the mock
    When the bot processes one round of updates
    Then no reply was sent

  Scenario: Processed updates are acknowledged so they are not handled twice
    Given a mock Telegram API delivering "milk" from the configured chat
    And a store "Mercadona" selling "milk" at 1.10
    And a Telegram bot connected to the mock
    When the bot processes one round of updates
    Then the next poll asks only for updates after the processed one
