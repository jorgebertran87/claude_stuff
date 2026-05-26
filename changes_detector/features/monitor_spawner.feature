Feature: Monitor spawner task management
  As the changes detector service
  I want to manage running monitor tasks
  So that I can pause, resume, and remove monitors dynamically

  Scenario: Pausing an active task aborts it and returns true
    Given a spawner with a task named "news"
    When I pause "news"
    Then the operation returned true

  Scenario: Pausing a non-existent task returns false
    Given an empty spawner
    When I pause "news"
    Then the operation returned false

  Scenario: Removing an active task aborts it and returns true
    Given a spawner with a task named "news"
    When I remove "news"
    Then the operation returned true

  Scenario: Removing a non-existent task returns false
    Given an empty spawner
    When I remove "news"
    Then the operation returned false

  Scenario: Listing aliases returns a sorted list
    Given a spawner with tasks "beta, alpha, gamma"
    When I list aliases
    Then the aliases are "alpha, beta, gamma"

  Scenario: Listing aliases on an empty spawner returns an empty list
    Given an empty spawner
    When I list aliases
    Then the aliases are empty
