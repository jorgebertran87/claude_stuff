Feature: Parse photo_5985514867101731739_y

  Scenario: Full board matches expected output
    Given the screenshot "/images/photo_5985514867101731739_y.jpg" is loaded
    When the board is parsed
    Then the rendered board should equal
      """
      ■ ■ ■ ■ ■ ■ ■ ■ ■ ■ ■
      ■ ■ ■ ■ ■ ■ ■ ■ ■ ■ ■
      ■ ■ ■ ■ 4 3 2 ⚑ 2 2 ⚑
      ■ ■ 2 2 ⚑ 1 1 1 1 2 2
      ■ ■ 2 2 1 1 · · 1 2 ⚑
      1 2 ⚑ 1 · · · · 2 ⚑ 3
      · 1 1 1 · · · 1 3 ⚑ 2
      1 2 2 1 · 1 2 3 ⚑ 2 1
      1 ⚑ ⚑ 2 · 1 ⚑ ⚑ 2 2 1
      2 4 ⚑ 3 2 2 3 2 1 1 ⚑
      ⚑ 3 3 ⚑ 2 ⚑ 3 2 1 2 2
      2 ⚑ 2 1 2 2 ⚑ ⚑ 1 1 ⚑
      2 2 2 · · 2 4 4 2 1 1
      2 ⚑ 2 1 · 1 ⚑ ⚑ 2 1 1
      ⚑ 4 ⚑ 2 1 1 3 ⚑ 2 1 ⚑
      ⚑ 3 2 ⚑ 2 1 2 1 1 2 2
      1 1 2 2 4 ⚑ 2 · · 1 ⚑
      · · 2 ⚑ 4 ⚑ 2 · · 1 1
      · · 2 ⚑ 4 2 2 · · 1 1
      · · 1 1 3 ⚑ 3 1 2 2 ⚑
      · · · · 2 ⚑ 3 ⚑ 2 ⚑ 2
      """
