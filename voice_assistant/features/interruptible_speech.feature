Feature: Interruptible speech playback
  As a user
  I want to be able to interrupt the assistant while it is speaking
  So that I can give a new command without waiting for it to finish

  Scenario: Speech finishes without any interruption
    Given the assistant is speaking a response
    And no audio is captured during playback
    When the speech ends naturally
    Then the service reports it was not interrupted

  Scenario: User says the wake word while the assistant is speaking
    Given the assistant is speaking a long response
    And the microphone captures "claudito" during playback
    When the wake word is detected
    Then the service stops the speaker
    And the service reports it was interrupted

  Scenario: Speaker is stopped when the wake word interrupts playback
    Given the assistant is speaking a long response
    And the microphone captures "claudito" during playback
    When the wake word is detected
    Then the speaker receives a stop signal

  Scenario: Echo reference is cleared after speech ends normally
    Given the assistant speaks a response
    When the speech finishes
    Then the echo reference on the capturer is cleared

  Scenario: Echo reference is cleared even when speech is interrupted
    Given the assistant is speaking a long response
    And the microphone captures "claudito" during playback
    When the wake word interrupts the speech
    Then the echo reference on the capturer is cleared

  Scenario: Unrelated speech during playback does not trigger an interrupt
    Given the assistant is speaking a long response
    And the microphone first captures "hola mundo"
    And the microphone then captures "claudito"
    When the service processes both captures
    Then the assistant is eventually interrupted by the wake word
    And the speaker is stopped
