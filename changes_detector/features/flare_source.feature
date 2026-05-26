Feature: FlareSolverr source
  As the change detector
  I want to fetch page content via FlareSolverr
  So that I can monitor Cloudflare-protected pages

  Scenario: Content mode returns the outer HTML of the matched element
    Given a mock FlareSolverr returning a page with element "div.content" containing "Hello"
    And a FlareSolverSource in content mode targeting selector "div.content"
    When I fetch from the source
    Then the fetch succeeds
    And the result contains "Hello"

  Scenario: Existence mode returns present when the element exists
    Given a mock FlareSolverr returning a page with element "div.content" containing "Hello"
    And a FlareSolverSource in existence mode targeting selector "div.content"
    When I fetch from the source
    Then the fetch succeeds
    And the result is "present"

  Scenario: Existence mode returns absent when the element is missing
    Given a mock FlareSolverr returning a page with element "div.content" containing "Hello"
    And a FlareSolverSource in existence mode targeting selector "div.missing"
    When I fetch from the source
    Then the fetch succeeds
    And the result is "absent"

  Scenario: A 500 response from FlareSolverr causes a fetch error
    Given a mock FlareSolverr that returns HTTP 500
    And a FlareSolverSource in content mode targeting selector "div.content"
    When I fetch from the source
    Then the fetch fails
