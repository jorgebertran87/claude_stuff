Feature: Voice Listener Service
  As a user
  I want the voice listener to orchestrate the voice interaction flow
  So that wake word detection, order capture, melody playback, meta-commands,
  and interruptible speech work together seamlessly

  # ── handle_with_melody ─────────────────────────────────────────────────────

  Scenario: Handle with melody forwards the order and starts the melody
    Given a VoiceListenerService with an order handler that returns "Son las 3 de la tarde"
    When the service handles the order "qué hora es" with melody
    Then the order handler receives "qué hora es"
    And the response is "Son las 3 de la tarde"
    And the melody is playing (stop signal is initially false)

  Scenario: Setting the stop signal terminates the melody thread
    Given a VoiceListenerService with an order handler that returns "ok"
    And the service is handling the order "enciende la luz" with melody
    When the stop signal is set
    Then the melody thread terminates within 2 seconds

  # ── handle_meta_commands ───────────────────────────────────────────────────

  Scenario: "Elimina la sesión" resets the session and returns a confirmation
    Given a VoiceListenerService with a tracking order handler
    When the service checks the meta-command "Elimina la sesión"
    Then reset_session is called on the order handler
    And the confirmation message is "Sesión eliminada."

  Scenario: Meta-command detection is case-insensitive
    Given a VoiceListenerService with a tracking order handler
    When the service checks the meta-command "ELIMINA LA SESIÓN"
    Then reset_session is called on the order handler

  Scenario: A normal order is not recognized as a meta-command
    Given a VoiceListenerService with a tracking order handler
    When the service checks the meta-command "qué hora es"
    Then reset_session is not called on the order handler
    And no confirmation message is returned

  Scenario: A phrase similar to the reset command is not a false positive
    Given a VoiceListenerService with a tracking order handler
    When the service checks the meta-command "elimina la cuenta"
    Then reset_session is not called on the order handler
    And no confirmation message is returned

  # ── listen_for_order ───────────────────────────────────────────────────────

  Scenario: Order is captured and transcribed on the first attempt
    Given the microphone captures a valid audio clip for order listening
    And the transcription of the order returns "enciende la luz"
    When the service listens for an order
    Then the service returns "enciende la luz"

  Scenario: Service retries when no audio is captured on the first attempt
    Given the microphone returns no audio on the first order attempt
    And the microphone captures a valid audio clip on the second order attempt
    And the transcription of the order returns "apaga la luz"
    When the service listens for an order
    Then the service retries and returns "apaga la luz"

  Scenario: Service retries when transcription returns nothing
    Given the microphone captures a valid audio clip on both order attempts
    And the first transcription of the order returns nothing
    And the second transcription of the order returns "qué hora es"
    When the service listens for an order
    Then the service retries and returns "qué hora es"

  Scenario: The speaker beeps once before each listening attempt
    Given the microphone returns no audio on the first order attempt
    And the microphone captures a valid audio clip on the second order attempt
    And the transcription of the order returns "hola"
    When the service listens for an order
    Then the speaker has beeped for order exactly 2 times

  # ── wait_for_wake_word ─────────────────────────────────────────────────────

  Scenario: Wake word is detected with an exact match
    Given the microphone captures "claudito" for wake word detection
    When the service waits for the wake word
    Then the service detects the wake word
    And no inline order is returned

  Scenario: Wake word with an inline order extracts the trailing text
    Given the microphone captures "claudito pon música" for wake word detection
    When the service waits for the wake word
    Then the service detects the wake word
    And the inline order "pon música" is returned

  Scenario: Wake word is detected via fuzzy matching
    Given the microphone captures "clauditto" for wake word detection
    When the service waits for the wake word
    Then the service detects the wake word via fuzzy matching
    And no inline order is returned

  Scenario: Non-wake-word utterances are ignored until the wake word is heard
    Given the microphone captures "hola mundo" for wake word detection
    And the microphone then captures "claudito" for wake word detection
    When the service waits for the wake word
    Then the service ignores the first utterance and keeps listening
    And the service detects the wake word on the second utterance
