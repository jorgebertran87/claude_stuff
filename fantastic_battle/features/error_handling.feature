Feature: Error Handling
  As a game client
  I want clear error responses for invalid requests
  So that I can handle failures gracefully without the server crashing

  Scenario: Move with missing direction field returns 400
    Given a game session exists
    When the client sends a POST request to "/api/sessions/{session_id}/move" with an empty body
    Then the server responds with status 400
    And the error message is "missing direction field"

  Scenario: Answer with missing answer field returns 400
    Given a game session exists
    When the client sends a POST request to "/api/sessions/{session_id}/battle/answer" with an empty body
    Then the server responds with status 400
    And the error message is "missing answer field"

  Scenario: Answer on already-over battle returns 409
    Given a game session exists with a finished battle
    When the client sends a POST request to "/api/sessions/{session_id}/battle/answer" with answer "Zeus"
    Then the server responds with status 409
    And the error message is "battle is already over"

  Scenario: Move on non-existent session returns 404
    When the client sends a POST request to "/api/sessions/nonexistent/move" with direction "East"
    Then the server responds with status 404
    And the error message is "session not found"

  Scenario: Interact on non-existent session returns 404
    When the client sends a POST request to "/api/sessions/nonexistent/interact"
    Then the server responds with status 404
    And the error message is "session not found"

  Scenario: Get battle on non-existent session returns 404
    When the client sends a GET request to "/api/sessions/nonexistent/battle"
    Then the server responds with status 404
    And the error message is "battle not found"

  Scenario: Answer on non-existent battle returns 404
    Given a game session exists with no battle
    When the client sends a POST request to "/api/sessions/{session_id}/battle/answer" with answer "Zeus"
    Then the server responds with status 404
    And the error message is "battle not found"
