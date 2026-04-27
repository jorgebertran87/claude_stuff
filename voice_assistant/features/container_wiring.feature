Feature: DI container wiring

  Scenario: build_telegram_bot assembles all components without panicking
    When build_telegram_bot is called with an empty token
    Then the container assembled without panicking
