Feature: Glovo token refresh
  As the prices comparer
  I want to refresh the short-lived Glovo access token automatically
  So that /glovo keeps working without re-capturing a token every 20 minutes

  Scenario: A successful refresh stores the new access token
    Given a stored refresh token "refresh-1" and device urn "glv:device:abc"
    And a mock Glovo auth that issues access token "access-2" and refresh token "refresh-2"
    And a refresher pointed at the mock
    When I refresh
    Then the access token in the token store is "access-2"

  Scenario: The rotated refresh token replaces the stored one
    Given a stored refresh token "refresh-1" and device urn "glv:device:abc"
    And a mock Glovo auth that issues access token "access-2" and refresh token "refresh-2"
    And a refresher pointed at the mock
    When I refresh
    Then the stored refresh token is "refresh-2"

  Scenario: The next refresh sends the rotated token
    Given a stored refresh token "refresh-1" and device urn "glv:device:abc"
    And a mock Glovo auth that issues access token "access-2" and refresh token "refresh-2"
    And a refresher pointed at the mock
    When I refresh
    And I refresh
    Then the last refresh request sent the token "refresh-2"

  Scenario: A rejected refresh token is reported
    Given a stored refresh token "expired-token" and device urn "glv:device:abc"
    And a mock Glovo auth that rejects the refresh token
    And a refresher pointed at the mock
    When I refresh
    Then the refresh reports the token was rejected

  Scenario: With no refresh token configured the refresh is a no-op
    Given no refresh token is configured
    And a refresher pointed at the mock
    When I refresh
    Then no refresh request was made
    And the token store is still empty
