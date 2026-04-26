import cv2
import numpy as np
from cell_classifier import classify_cell
from grid_analyzer import find_grid_bounds, find_cell_y_offset, estimate_grid_size
from header_detector import detect_header
from models import Board, Cell


def parse_board_bgr(bgr: np.ndarray) -> Board:
    img = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)

    header, grid_y = detect_header(img, bgr=bgr)
    x0, y0, x1, y1 = find_grid_bounds(img, grid_y)
    grid_img = img[y0:y1, x0:x1]
    bgr_grid = bgr[y0:y1, x0:x1]

    rows, cols = estimate_grid_size(grid_img)
    cell_h = grid_img.shape[0] // rows
    cell_w = grid_img.shape[1] // cols

    y_offset = find_cell_y_offset(grid_img, cell_h)
    gh = grid_img.shape[0]
    # Actual cell height for full rows (rows 1..rows-1); row 0 is a partial fragment
    actual_cell_h = (gh - y_offset) // (rows - 1) if (rows > 1 and y_offset > 0) else cell_h

    board = Board(rows=rows, cols=cols, header=header)

    for r in range(rows):
        row_cells = []
        for c in range(cols):
            cx = c * cell_w
            if y_offset > 0:
                if r == 0:
                    cy, h = 0, y_offset
                else:
                    cy = y_offset + (r - 1) * actual_cell_h
                    h = actual_cell_h
            else:
                cy, h = r * cell_h, cell_h
            cy = min(cy, max(0, gh - max(h, 1)))
            roi = bgr_grid[cy: cy + h, cx: cx + cell_w]
            state = classify_cell(roi)
            row_cells.append(Cell(
                row=r, col=c, state=state,
                x=x0 + cx, y=y0 + cy,
                width=cell_w, height=h,
            ))
        board.cells.append(row_cells)

    return board
