from models import Board, CellState

_SYMBOLS: dict[CellState, str] = {
    CellState.UNREVEALED: "■",
    CellState.EMPTY:      "·",
    CellState.FLAG:       "⚑",
    CellState.MINE:       "*",
    CellState.NUMBER_1:   "1",
    CellState.NUMBER_2:   "2",
    CellState.NUMBER_3:   "3",
    CellState.NUMBER_4:   "4",
    CellState.NUMBER_5:   "5",
    CellState.NUMBER_6:   "6",
    CellState.NUMBER_7:   "7",
    CellState.NUMBER_8:   "8",
}


def render_board(board: Board) -> str:
    lines = []
    if board.header:
        lines.append(f"Mines: {board.header.mine_count}")
        lines.append("")
    for row in board.cells:
        lines.append(" ".join(_SYMBOLS.get(c.state, "?") for c in row))
    return "\n".join(lines)
