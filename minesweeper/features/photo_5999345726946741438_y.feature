Feature: Parse photo_5999345726946741438_y

  Scenario: Full board matches expected output
    Given the screenshot "/images/photo_5999345726946741438_y.jpg" is loaded
    When the board is parsed
    Then the rendered board should equal
      """
      · · 1 ⚑ 1 · 1 ⚑ 3 ⚑ ⚑
      · · 1 1 1 · 1 1 3 ⚑ 3
      1 1 2 2 2 1 · · 1 1 1
      1 ⚑ 2 ⚑ ⚑ 2 1 1 · · ·
      2 2 3 4 4 4 ⚑ 2 1 2 2
      ⚑ 3 2 ⚑ ⚑ 3 ⚑ 2 1 ⚑ ⚑
      ⚑ 3 ⚑ 3 3 3 2 2 2 3 2
      3 4 2 2 2 ⚑ 1 1 ⚑ 2 1
      ⚑ ⚑ 1 1 ⚑ 2 1 1 1 2 ⚑
      ⚑ 3 1 1 2 3 2 1 · 1 1
      1 2 1 2 3 ⚑ ⚑ 1 · · ·
      · 1 ⚑ 2 ⚑ ⚑ 3 1 · 1 1
      2 3 3 4 4 3 1 · · 1 ⚑
      ⚑ ⚑ 2 ⚑ ⚑ 2 1 · · 2 2
      3 4 3 3 4 ⚑ 3 2 1 2 ⚑
      ⚑ 2 ⚑ 2 3 ⚑ ⚑ 2 ⚑ 2 1
      2 4 3 3 ⚑ 3 2 2 1 1 ·
      1 ⚑ ⚑ 2 1 2 1 1 · · ·
      1 3 4 3 1 1 ⚑ 1 · · ·
      · 1 ⚑ ⚑ 1 1 1 1 · · ·
      """
