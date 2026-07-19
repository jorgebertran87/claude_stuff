Feature: Game World API
  As a game client
  I want to interact with the game world through HTTP endpoints
  So that I can explore the map, move my character, and encounter NPCs

  # ── Joining a game ────────────────────────────────────────────────────────

  Scenario: Creating a new game session returns session state
    When the client sends a POST request to "/api/sessions"
    Then the response status is 200
    And the response body contains a session id
    And the player position is (0, 0) facing "South"

  # ── Retrieving a session ──────────────────────────────────────────────────

  Scenario: Retrieving an existing session returns its state
    Given a game session exists
    When the client sends a GET request to that session
    Then the response status is 200
    And the player position is (0, 0) facing "South"

  Scenario: Retrieving a non-existent session returns 404
    When the client sends a GET request to "/api/sessions/nonexistent"
    Then the response status is 404

  # ── Movement ──────────────────────────────────────────────────────────────

  Scenario: Moving east succeeds on the test map
    Given a game session exists
    When the client sends a POST request to that session's move endpoint with direction "East"
    Then the response status is 200
    And the player position is (1, 0) facing "East"

  Scenario: Moving into a wall is rejected
    Given a game session exists
    When the client sends a POST request to that session's move endpoint with direction "South"
    Then the response status is 409
    And the error message is "cannot walk through walls"

  Scenario: Moving out of bounds is rejected
    Given a game session exists
    When the client sends a POST request to that session's move endpoint with direction "North"
    Then the response status is 409
    And the error message is "cannot move outside the map"

  # ── NPC Interaction ──────────────────────────────────────────────────────

  Scenario: Interacting with an NPC in front of the player returns the NPC
    Given a game session exists
    When the client sends a POST request to that session's move endpoint with direction "East"
    And the client sends a POST request to that session's interact endpoint
    Then the response status is 200
    And the response body contains an NPC named "Sphinx"

  Scenario: Interacting with no NPC in front of the player returns no NPC
    Given a game session exists
    When the client sends a POST request to that session's interact endpoint
    Then the response status is 200
    And the response body contains no NPC
