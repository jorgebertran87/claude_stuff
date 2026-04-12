Feature: Telegram bot infrastructure integration
  As the system
  I want to route Telegram commands through the bot
  So that users can interact with the assistant via Telegram

  Scenario: /list command returns help text
    Given a TelegramBot with a fake gateway
    And an update with text "/list" from chat 1
    When run_once processes the updates
    Then the gateway posted a message to chat 1 containing "/reset"

  Scenario: /reset command resets the session
    Given a TelegramBot with a fake gateway
    And a handler exists for chat 1
    And an update with text "/reset" from chat 1
    When run_once processes the updates
    Then the gateway posted a message to chat 1 containing "Sesión reiniciada"

  Scenario: /voice_mode toggles voice mode on and off
    Given a TelegramBot with a fake gateway
    And an update with text "/voice_mode" from chat 1
    When run_once processes the updates
    Then the gateway posted a message to chat 1 containing "activado"
    When run_once processes another "/voice_mode" from chat 1
    Then the gateway posted a message to chat 1 containing "desactivado"

  Scenario: Regular message is forwarded to the handler
    Given a TelegramBot with a fake gateway
    And an update with text "hola mundo" from chat 1
    When run_once processes the updates
    Then the handler received "hola mundo"
    And the gateway posted a message to chat 1

  Scenario: Unauthorized chat is ignored
    Given a TelegramBot with a fake gateway allowing only chat 99
    And an update with text "hola" from chat 1
    When run_once processes the updates
    Then the gateway posted no messages

  Scenario: Offset advances after processing updates
    Given a TelegramBot with a fake gateway
    And an update with id 42 and text "test" from chat 1
    When run_once processes the updates
    Then the offset is 43

  Scenario: Photo update with failed download posts error message
    Given a TelegramBot with a fake gateway
    And a photo update from chat 1 with no downloadable bytes
    When run_once processes the updates
    Then the gateway posted a message to chat 1 containing "descargar"

  Scenario: Photo update with downloadable bytes is routed through Claude and a response is posted
    Given a TelegramBot with a fake gateway
    And a photo update from chat 1 with downloadable bytes
    When run_once processes the updates
    Then the gateway posted a message to chat 1
