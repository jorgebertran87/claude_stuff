#!/usr/bin/env python3
"""CLI entry point: parse a minesweeper screenshot and print the board."""
import argparse
from pathlib import Path

from detector import parse_board, render_board


def main():
    parser = argparse.ArgumentParser(description="Parse a Minesweeper screenshot")
    parser.add_argument("image", type=Path, help="Path to the screenshot")
    args = parser.parse_args()

    print(render_board(parse_board(args.image)))


if __name__ == "__main__":
    main()
