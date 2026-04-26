from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


class CellState(Enum):
    UNREVEALED = "unrevealed"
    EMPTY = "empty"
    FLAG = "flag"
    MINE = "mine"
    NUMBER_1 = 1
    NUMBER_2 = 2
    NUMBER_3 = 3
    NUMBER_4 = 4
    NUMBER_5 = 5
    NUMBER_6 = 6
    NUMBER_7 = 7
    NUMBER_8 = 8


@dataclass
class Cell:
    row: int
    col: int
    state: CellState
    x: int = 0
    y: int = 0
    width: int = 0
    height: int = 0

    @property
    def number(self) -> Optional[int]:
        if isinstance(self.state.value, int):
            return self.state.value
        return None


@dataclass
class Header:
    mine_count: int


@dataclass
class Board:
    rows: int
    cols: int
    cells: list[list[Cell]] = field(default_factory=list)
    header: Optional[Header] = None

    def get(self, row: int, col: int) -> Optional[Cell]:
        if 0 <= row < self.rows and 0 <= col < self.cols:
            return self.cells[row][col]
        return None

    def flagged_cells(self) -> list[Cell]:
        return [c for row in self.cells for c in row if c.state == CellState.FLAG]

    def unrevealed_cells(self) -> list[Cell]:
        return [c for row in self.cells for c in row if c.state == CellState.UNREVEALED]

    def number_cells(self) -> list[Cell]:
        return [c for row in self.cells for c in row if c.number is not None]
