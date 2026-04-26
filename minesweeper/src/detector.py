"""Input adapters: load an image from a file path or raw bytes and parse its board."""
import cv2
import numpy as np
from pathlib import Path

from board_parser import parse_board_bgr
from renderer import render_board  # re-exported for callers that import from here
from models import Board

__all__ = ["parse_board", "parse_board_bytes", "render_board"]


def parse_board(image_path: str | Path) -> Board:
    bgr = cv2.imread(str(image_path))
    if bgr is None:
        raise FileNotFoundError(f"Cannot load image: {image_path}")
    return parse_board_bgr(bgr)


def parse_board_bytes(data: bytes) -> Board:
    buf = np.frombuffer(data, np.uint8)
    bgr = cv2.imdecode(buf, cv2.IMREAD_COLOR)
    if bgr is None:
        raise ValueError("Cannot decode image data")
    return parse_board_bgr(bgr)
