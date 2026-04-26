Feature: Parse photo_6012830459672333473_w

  Scenario: Full board matches expected output
    Given the screenshot "/images/photo_6012830459672333473_w.jpg" is loaded
    When the board is parsed
    Then the rendered board should equal
      """
      ■ ■ ■ 2 1 1 ⚑ 1 · · ·
      ■ ■ ■ ⚑ 1 1 2 2 2 2 2
      ■ ■ ■ 4 2 · 1 ⚑ 2 ⚑ ⚑
      ■ ■ ■ ⚑ 4 2 2 1 2 2 2
      ■ ■ 4 ⚑ ⚑ ⚑ 2 · · · ·
      1 2 ⚑ 5 6 ⚑ 2 · · · ·
      · 1 2 ⚑ ⚑ 2 1 1 1 1 ·
      1 1 2 2 2 2 1 3 ⚑ 2 ·
      3 ⚑ 2 1 2 3 ⚑ 3 ⚑ 3 1
      ⚑ ⚑ 4 3 ⚑ ⚑ 2 2 2 ⚑ 2
      2 3 ⚑ ⚑ 3 2 1 · 1 2 ⚑
      · 2 3 3 1 · · · · 1 1
      · 1 ⚑ 1 · 1 1 1 · · ·
      · 1 2 2 1 1 ⚑ 2 1 · ·
      · · 1 ⚑ 2 2 4 ⚑ 2 · ·
      1 1 1 1 2 ⚑ 3 ⚑ 3 1 ·
      ⚑ 3 2 2 2 2 2 2 ⚑ 1 ·
      2 ⚑ ⚑ 2 ⚑ 1 1 2 2 1 ·
      2 4 3 3 1 2 2 ⚑ 1 · ·
      ⚑ 2 ⚑ 1 · 1 ⚑ 2 1 · ·
      """
