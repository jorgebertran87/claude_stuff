Feature: Change detector
  As the changes detector service
  I want to track content snapshots with line-level diffs
  So that I can notify only when monitored content actually changes

  Scenario: First fetch bootstraps the snapshot without triggering a notification
    Given a fresh change detector
    When I check "Hello, world!"
    Then the result is Bootstrapped

  Scenario: Same content after bootstrapping returns no change
    Given a change detector seeded with "Hello, world!"
    When I check "Hello, world!"
    Then the result is NoChange

  Scenario: Different content returns Changed with a diff
    Given a change detector seeded with "old line"
    When I check "new line"
    Then the result is Changed
    And the diff contains "- old line"
    And the diff contains "+ new line"

  Scenario: Each changed line in the diff is separated by exactly one newline
    Given a change detector seeded with "old line"
    When I check "new line"
    Then the result is Changed
    And the diff is exactly "- old line\n+ new line\n"

  Scenario: Second identical check after a change returns no change
    Given a change detector seeded with "version 1"
    When I check "version 2"
    And I check "version 2"
    Then the result is NoChange

  Scenario: State is persisted so a reloaded detector picks up the snapshot
    Given a change detector seeded with "persisted content"
    When I reload the detector from the same file
    And I check "persisted content"
    Then the result is NoChange
