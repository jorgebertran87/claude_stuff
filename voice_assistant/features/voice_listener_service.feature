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

  Scenario: Wake word is not detected for completely unrelated text
    Given the microphone captures "chocolate" for wake word detection
    And the microphone then captures "claudito" for wake word detection
    When the service waits for the wake word
    Then the service ignores the first utterance and keeps listening
    And the service detects the wake word on the second utterance

  Scenario: Fuzzy-matched wake word with an inline order extracts the trailing text
    Given the microphone captures "clauditto enciende la luz" for wake word detection
    When the service waits for the wake word
    Then the service detects the wake word via fuzzy matching
    And the inline order "enciende la luz" is returned

  # ── CLI Argument Parsing ────────────────────────────────────────────────────

  Scenario: --order with a value returns DirectOrder
    Given the command-line arguments "prog", "--order", "test order"
    When the arguments are parsed
    Then the result is DirectOrder with value "test order"

  Scenario: --order at the first position returns the correct value
    Given the command-line arguments "--order", "value"
    When the arguments are parsed
    Then the result is DirectOrder with value "value"

  Scenario: --order without a value is an error
    Given the command-line arguments "prog", "--order"
    When the arguments are parsed
    Then the parsing result is an error

  Scenario: --telegram as the only argument returns TelegramMode
    Given the command-line arguments "--telegram"
    When the arguments are parsed
    Then the parsing result is TelegramMode

  Scenario: --telegram with a program name returns TelegramMode
    Given the command-line arguments "prog", "--telegram"
    When the arguments are parsed
    Then the parsing result is TelegramMode

  Scenario: No flags returns ListenMode by default
    Given the command-line arguments "prog"
    When the arguments are parsed
    Then the parsing result is ListenMode

  Scenario: --order after another argument extracts the correct value
    Given the command-line arguments "prog", "other", "--order", "value"
    When the arguments are parsed
    Then the result is DirectOrder with value "value"

  # ── Wake Word Unit Tests ────────────────────────────────────────────────────

  Scenario: Exact match returns true
    Given a wake word "claudito"
    When the wake word is checked against "claudito"
    Then the wake word matches

  Scenario: Fuzzy match returns true
    Given a wake word "claudito"
    When the wake word is checked against "clauditto"
    Then the wake word matches

  Scenario: Completely unrelated text does not match
    Given a wake word "claudito"
    When the wake word is checked against "chocolate"
    Then the wake word does not match

  Scenario: Text without the wake word does not match
    Given a wake word "claudito"
    When the wake word is checked against "hola mundo"
    Then the wake word does not match

  Scenario: Numeric-only input does not match
    Given a wake word "claudito"
    When the wake word is checked against "123 456"
    Then the wake word does not match

  # ── Order Extraction Unit Tests ─────────────────────────────────────────────

  Scenario: Exact match extracts the trailing order text
    Given a wake word "claudito"
    When the order is extracted from "claudito pon música"
    Then the extracted order is "pon música"

  Scenario: Fuzzy match extracts the trailing order text
    Given a wake word "claudito"
    When the order is extracted from "clauditto enciende la luz"
    Then the extracted order is "enciende la luz"

  Scenario: No order is extracted when the wake word is not present
    Given a wake word "claudito"
    When the order is extracted from "hola mundo"
    Then no order is extracted

  Scenario: No order is extracted for unrelated words
    Given a wake word "claudito"
    When the order is extracted from "xyzzy algo"
    Then no order is extracted

  Scenario: No order is extracted when the wake word is alone
    Given a wake word "claudito"
    When the order is extracted from "claudito"
    Then no order is extracted

  Scenario: No order is extracted when the wake word is at the end
    Given a wake word "claudito"
    When the order is extracted from "hola claudito"
    Then no order is extracted

  # ── Language Prefix ─────────────────────────────────────────────────────────

  Scenario: Language prefix extracts the code before the first hyphen
    Given a language with code "es-ES"
    When the language prefix is requested
    Then the prefix is "es"

  Scenario: Language prefix for a code without hyphen is the full code
    Given a language with code "en"
    When the language prefix is requested
    Then the prefix is "en"

  Scenario: Language prefix for a complex code extracts the first part
    Given a language with code "zh-Hans-CN"
    When the language prefix is requested
    Then the prefix is "zh"
