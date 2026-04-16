#!/usr/bin/env python3
"""CLI entry point: parse a minesweeper screenshot and print the board."""
import argparse
import sys
from pathlib import Path

from detector import parse_board, render_board


def main():
    parser = argparse.ArgumentParser(description="Parse a Minesweeper screenshot")
    parser.add_argument("image", type=Path, help="Path to the screenshot")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    board = parse_board(args.image)

    if args.json:
        import json
        data = {
            "rows": board.rows,
            "cols": board.cols,
            "header": {
                "mine_count": board.header.mine_count if board.header else None,
                "timer": board.header.timer if board.header else None,
                "has_hint": board.header.has_hint if board.header else None,
            },
            "cells": [
                [{"row": c.row, "col": c.col, "state": c.state.value,
                  "x": c.x, "y": c.y, "w": c.width, "h": c.height}
                 for c in row]
                for row in board.cells
            ],
        }
        print(json.dumps(data, indent=2))
    else:
        print(render_board(board))


if __name__ == "__main__":
    main()
