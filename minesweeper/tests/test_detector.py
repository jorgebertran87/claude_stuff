import pytest
import numpy as np
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent.parent))

from models import Board, Cell, CellState, Header
from detector import parse_board, render_board, _classify_cell, _detect_header

IMAGE_PATH = Path("/images/telegram_image_1776362994657440358.jpg")


@pytest.fixture(scope="module")
def board() -> Board:
    return parse_board(IMAGE_PATH)


# --- Header ---

class TestHeader:
    def test_header_is_parsed(self, board):
        assert board.header is not None

    def test_mine_count_is_positive(self, board):
        assert board.header.mine_count >= 0

    def test_hint_is_detected(self, board):
        # The lightbulb is lit (yellow) in the test image
        assert board.header.has_hint is True


# --- Grid shape ---

class TestGridShape:
    def test_board_has_rows(self, board):
        assert board.rows > 0

    def test_board_has_cols(self, board):
        assert board.cols > 0

    def test_cells_match_dimensions(self, board):
        assert len(board.cells) == board.rows
        for row in board.cells:
            assert len(row) == board.cols


# --- Cell types ---

class TestCellTypes:
    def test_board_has_unrevealed_cells(self, board):
        assert len(board.unrevealed_cells()) > 0

    def test_board_has_flags(self, board):
        assert len(board.flagged_cells()) > 0

    def test_board_has_number_cells(self, board):
        assert len(board.number_cells()) > 0

    def test_numbers_are_in_range(self, board):
        for cell in board.number_cells():
            assert 1 <= cell.number <= 8

    def test_cell_coordinates_are_positive(self, board):
        for row in board.cells:
            for cell in row:
                assert cell.x >= 0
                assert cell.y >= 0


# --- Cell access API ---

class TestBoardAccess:
    def test_get_valid_cell(self, board):
        cell = board.get(0, 0)
        assert isinstance(cell, Cell)

    def test_get_out_of_bounds_returns_none(self, board):
        assert board.get(-1, 0) is None
        assert board.get(board.rows, 0) is None
        assert board.get(0, board.cols) is None


# --- Render ---

class TestRender:
    def test_render_produces_string(self, board):
        output = render_board(board)
        assert isinstance(output, str)
        assert len(output) > 0

    def test_render_has_correct_row_count(self, board):
        output = render_board(board)
        # header adds 2 lines; remaining lines = rows
        lines = [l for l in output.split("\n") if l.strip()]
        assert len(lines) >= board.rows


# --- Unit: cell classifier ---

class TestCellClassifier:
    def _solid_bgr(self, b, g, r, size=60):
        return np.full((size, size, 3), [b, g, r], dtype=np.uint8)

    def test_blue_number_detected_as_1(self):
        roi = self._solid_bgr(200, 30, 30)
        assert _classify_cell(roi) == CellState.NUMBER_1

    def test_green_number_detected_as_2(self):
        roi = self._solid_bgr(30, 150, 30)
        assert _classify_cell(roi) == CellState.NUMBER_2

    def test_red_number_detected_as_3(self):
        roi = self._solid_bgr(30, 30, 220)
        assert _classify_cell(roi) == CellState.NUMBER_3
