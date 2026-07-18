Feature: Server Health
  As a game client
  I want to check the server is running
  So that I know the backend is available

  Scenario: The server responds to a health check
    When the client sends a GET request to "/health"
    Then the server responds with status 200
    And the response body is "ok"
