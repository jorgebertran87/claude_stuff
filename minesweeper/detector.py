import numpy as np
import cv2
from pathlib import Path
from models import Board, Cell, CellState, Header


# Color thresholds in BGR
_NUMBER_COLORS = {
    CellState.NUMBER_1: {"low": np.array([100, 0, 0]),   "high": np.array([255, 80, 80])},   # blue
    CellState.NUMBER_2: {"low": np.array([0, 80, 0]),    "high": np.array([80, 200, 80])},   # green
    CellState.NUMBER_3: {"low": np.array([0, 0, 150]),   "high": np.array([80, 80, 255])},   # red
    CellState.NUMBER_4: {"low": np.array([80, 0, 0]),    "high": np.array([160, 60, 100])},  # dark navy
}

_FLAG_RED_LOW  = np.array([0, 0, 150])
_FLAG_RED_HIGH = np.array([80, 80, 255])


def _count_pixels(roi: np.ndarray, low: np.ndarray, high: np.ndarray) -> int:
    mask = cv2.inRange(roi, low, high)
    return int(np.count_nonzero(mask))


def _is_unrevealed(roi: np.ndarray) -> bool:
    """Unrevealed cells have strong 3-D raised borders: dark bottom/right, bright top/left."""
    h, w = roi.shape[:2]
    border = max(4, h // 10)
    top_strip    = roi[:border, border:-border]
    bottom_strip = roi[-border:, border:-border]
    top_mean    = float(np.mean(top_strip))
    bottom_mean = float(np.mean(bottom_strip))
    return (top_mean - bottom_mean) > 20


def _has_flag(inner: np.ndarray) -> bool:
    """A flag has red in the top half AND a dark stand in the bottom centre."""
    ih, iw = inner.shape[:2]
    red_top = _count_pixels(inner[:ih // 2, :], _FLAG_RED_LOW, _FLAG_RED_HIGH)
    if red_top < 30:
        return False
    # Black stand: very dark pixels in the bottom-centre strip
    stand = inner[ih * 2 // 3:, iw // 3: 2 * iw // 3]
    dark_mask = cv2.inRange(stand, np.array([0, 0, 0]), np.array([60, 60, 60]))
    return int(np.count_nonzero(dark_mask)) > 10


def _classify_cell(roi: np.ndarray) -> CellState:
    h, w = roi.shape[:2]
    pad = max(5, h // 8)
    inner = roi[pad:-pad, pad:-pad]

    if _has_flag(inner):
        return CellState.FLAG

    # Numbers: look for coloured pixels inside the inner region
    best_state, best_count = CellState.EMPTY, 0
    for state, rng in _NUMBER_COLORS.items():
        count = _count_pixels(inner, rng["low"], rng["high"])
        if count > best_count:
            best_count = count
            best_state = state

    if best_count > 100:
        return best_state

    # Unrevealed vs empty
    gray = cv2.cvtColor(roi, cv2.COLOR_BGR2GRAY)
    if _is_unrevealed(gray):
        return CellState.UNREVEALED

    return CellState.EMPTY


def _detect_header(img: np.ndarray) -> tuple[Header, int]:
    """Returns (Header, y_offset) where y_offset is where the grid starts."""
    h, w = img.shape[:2]
    # The toolbar is the top ~12% of the image
    toolbar_h = int(h * 0.12)

    # Mine counter: left LED display (red digits on black background)
    left_panel = img[:toolbar_h, :w // 5]
    mine_count = _read_led_display(left_panel)

    # Timer: right LED display
    right_panel = img[:toolbar_h, w * 4 // 5:]
    timer = _read_led_display_str(right_panel)

    # Hint lightbulb: yellow region in center-top
    center = img[:toolbar_h, w // 3: 2 * w // 3]
    yellow_mask = cv2.inRange(center,
                              np.array([0, 150, 150]),
                              np.array([100, 255, 255]))
    has_hint = int(np.count_nonzero(yellow_mask)) > 500

    # Grid starts just below the toolbar + a thin separator row
    grid_start = toolbar_h + max(4, int(h * 0.01))

    return Header(mine_count=mine_count, timer=timer, has_hint=has_hint), grid_start


def _read_led_display(panel: np.ndarray) -> int:
    """Rough digit count from a red-on-black LED panel."""
    red_mask = cv2.inRange(panel,
                           np.array([0, 0, 150]),
                           np.array([80, 80, 255]))
    pixels = int(np.count_nonzero(red_mask))
    # Very rough mapping: each digit ~400 red pixels at typical sizes
    return max(0, round(pixels / 400))


def _read_led_display_str(panel: np.ndarray) -> str:
    """Return a placeholder timer string (full OCR out of scope here)."""
    return "??"


def _find_grid_bounds(img: np.ndarray, y_start: int) -> tuple[int, int, int, int]:
    """Return (x0, y0, x1, y1) of the cell grid below the header."""
    gray = cv2.cvtColor(img[y_start:], cv2.COLOR_BGR2GRAY)
    # Cells have uniform light-gray background — find the bounding box of that region
    _, thresh = cv2.threshold(gray, 160, 255, cv2.THRESH_BINARY)
    contours, _ = cv2.findContours(thresh, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        h, w = img.shape[:2]
        return 0, y_start, w, h
    largest = max(contours, key=cv2.contourArea)
    gx, gy, gw, gh = cv2.boundingRect(largest)
    return gx, y_start + gy, gx + gw, y_start + gy + gh


def _find_cell_period(profile: np.ndarray, length: int, min_cell_px: int = 60) -> int:
    """Find the dominant cell period via FFT, ignoring frequencies above min_cell_px."""
    spectrum = np.abs(np.fft.rfft(profile - profile.mean()))
    freqs = np.fft.rfftfreq(len(profile))

    # Mask out frequencies that would imply cells smaller than min_cell_px
    max_freq = 1.0 / min_cell_px
    spectrum[freqs > max_freq] = 0
    spectrum[0] = 0  # ignore DC

    dominant_freq = freqs[np.argmax(spectrum)]
    if dominant_freq < 1e-9:
        return length  # fallback: single cell
    period = int(round(1.0 / dominant_freq))
    return max(min_cell_px, min(period, length))


def _estimate_grid_size(grid_img: np.ndarray) -> tuple[int, int]:
    """Estimate rows and columns via FFT of edge profiles."""
    gray = cv2.cvtColor(grid_img, cv2.COLOR_BGR2GRAY)
    edges = cv2.Canny(gray, 30, 100)

    h_profile = np.sum(edges, axis=1).astype(float)  # row sums → vertical period
    v_profile = np.sum(edges, axis=0).astype(float)  # col sums → horizontal period

    row_period = _find_cell_period(h_profile, grid_img.shape[0])
    col_period = _find_cell_period(v_profile, grid_img.shape[1])

    rows = max(1, round(grid_img.shape[0] / row_period))
    cols = max(1, round(grid_img.shape[1] / col_period))
    return rows, cols


def parse_board(image_path: str | Path) -> Board:
    img = cv2.imread(str(image_path))
    if img is None:
        raise FileNotFoundError(f"Cannot load image: {image_path}")

    header, grid_y = _detect_header(img)
    x0, y0, x1, y1 = _find_grid_bounds(img, grid_y)
    grid_img = img[y0:y1, x0:x1]

    rows, cols = _estimate_grid_size(grid_img)
    cell_h = grid_img.shape[0] // rows
    cell_w = grid_img.shape[1] // cols

    board = Board(rows=rows, cols=cols, header=header)
    board.cells = []

    for r in range(rows):
        row_cells = []
        for c in range(cols):
            cx, cy = c * cell_w, r * cell_h
            roi = grid_img[cy: cy + cell_h, cx: cx + cell_w]
            state = _classify_cell(roi)
            row_cells.append(Cell(
                row=r, col=c, state=state,
                x=x0 + cx, y=y0 + cy,
                width=cell_w, height=cell_h,
            ))
        board.cells.append(row_cells)

    return board


def render_board(board: Board) -> str:
    symbols = {
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
    lines = []
    if board.header:
        h = board.header
        lines.append(f"Mines: {h.mine_count}  Timer: {h.timer}  Hint: {'on' if h.has_hint else 'off'}")
        lines.append("")
    for row in board.cells:
        lines.append(" ".join(symbols.get(c.state, "?") for c in row))
    return "\n".join(lines)
