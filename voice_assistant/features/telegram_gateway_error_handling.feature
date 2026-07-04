Feature: Telegram gateway error handling
  As the system
  I want the Telegram gateway to silently handle expected long-poll timeouts
  So that the logs are not cluttered with misleading error messages

  Scenario: Long-poll timeout is not logged as an error
    Given a UreqGateway
    When fetch_updates encounters a "timed out reading response" error from ureq
    Then no error is logged
    And an empty update list is returned

  Scenario: Genuine network errors are still logged
    Given a UreqGateway
    When fetch_updates encounters a "connection refused" error from ureq
    Then the error is logged to stderr
    And an empty update list is returned
