import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[2] / "src"))

from pytest_bdd import given, when, then, parsers
from detector import parse_board, render_board


@given(parsers.parse('the screenshot "{path}" is loaded'), target_fixture="image_path")
def screenshot_loaded(path):
    return Path(path)


@when("the board is parsed", target_fixture="board")
def board_parsed(image_path):
    return parse_board(image_path)


@then(parsers.parse("the rendered board should equal\n{expected}"))
def rendered_board_equals(board, expected):
    full = render_board(board)
    # render_board prepends a header + blank line; take only the grid rows
    # Grid rows start with a cell symbol (■·⚑* or digit); header lines contain ':'
    grid_lines = [l for l in full.splitlines() if l and ":" not in l and l[0] in "■·⚑*123456789"]
    expected_lines = [line.strip() for line in expected.strip().splitlines()]
    assert grid_lines == expected_lines, (
        f"\nExpected:\n{chr(10).join(expected_lines)}\n\nActual:\n{chr(10).join(grid_lines)}"
    )
