Feature: DI container wiring

  Scenario: build_voice_service assembles all components without panicking
    When build_voice_service is called with wake word "claudito" and language "es-ES"
    Then the container assembled without panicking

  Scenario: build_telegram_bot assembles all components without panicking
    When build_telegram_bot is called with an empty token
    Then the container assembled without panicking
