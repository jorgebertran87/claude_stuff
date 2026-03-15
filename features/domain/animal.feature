Feature: Animal Behavior
  As a pet owner
  I want to interact with my animal
  So that I can see it react correctly

  Scenario Outline: A hungry animal eats food
    Given I have a <animal> that is "hungry"
    When I feed the <animal> a <food>
    Then the <animal> should be <emotion>
    And its energy level should increase by <energy>

    Examples:

      | animal | food | emotion   | energy |
      | Dog    | bone | ok        | 2      |
      | Cat    | fish | satisfied | 5      |
      | Duck   | shit | sad       | -10    |

  Scenario Outline: Different animals make different sounds
    Given I have a <animal>
    When I listen to it
    Then it should make a "<sound>" sound

    Examples:

      | species | sound |
      | Dog     | Woof  |
      | Cat     | Meow  |
      | Duck    | Quack |