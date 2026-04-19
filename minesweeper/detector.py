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

_FLAG_RED_LOW  = np.array([0,   0, 150])
_FLAG_RED_HIGH = np.array([80, 80, 255])
_FLAG_BLUE_LOW  = np.array([150,  0,  0])   # BGR: high B, low G/R (this app's flag style)
_FLAG_BLUE_HIGH = np.array([255, 90, 90])


def _count_pixels(roi: np.ndarray, low: np.ndarray, high: np.ndarray) -> int:
    mask = cv2.inRange(roi, low, high)
    return int(np.count_nonzero(mask))


def _is_unrevealed(roi: np.ndarray) -> bool:
    """Four-path bevel check.
    Path 1 – partial fragment (row-0 crop, h << w): use the original loose check;
      the top-of-cell bevel is only partially visible so the diff is moderate.
    Path 2 – peak-vs-mid bevel: max row-mean in the top half > mid by ≥40.
      Integer-division cell heights cause the first few pixels of deeper ROIs to
      drift into the previous cell's interior (top ≈ mid ≈ 192) rather than the
      bevel highlight.  Taking the max row-mean across the full top half finds the
      bevel peak wherever it falls (drift ≤ h//2 ≈ 27 px).  Revealed cells are
      flat, so their peak ≈ mid.  Lighting-bleed cells have peak-mid ≈ 17.
    Path 3 – washed-out bevel + flag: some flags in compressed photos lose the
      bevel but their flag image concentrates very dark pixels in the mid strip
      (mid < 75).  The abs(top-bot) < 55 guard prevents NUMBER_4 cells whose
      dark-navy digit also darkens mid from using this path spuriously – they
      are recovered via number detection in _classify_cell.
    """
    h, w = roi.shape[:2]
    e = max(3, h // 15)
    bw = max(3, w // 4)
    mid = float(np.mean(roi[h//3:2*h//3, bw:w - bw]))
    bot = float(np.mean(roi[-e:,         bw:w - bw]))
    top = float(np.mean(roi[:e,          bw:w - bw]))
    if h < w * 0.7:                              # partial row-0 fragment
        return top > bot + 25
    peak = float(roi[:h // 2, bw:w - bw].mean(axis=1).max())
    if peak > mid + 40:
        return True
    if mid < 75 and abs(top - bot) < 55:          # dark-flag interior
        return True
    # Path 4 – flat-block bevel: large uniform unrevealed regions have very subtle
    # peak-vs-mid contrast but retain a few bright (>240) highlight pixels from the
    # cell border.  Revealed cells (empty or numbers) never exceed ~210 in this game.
    bright_count = int(np.count_nonzero(roi[:, bw:w - bw] > 240))
    return bright_count >= 100


def _has_flag(inner: np.ndarray) -> bool:
    """Flag has coloured content concentrated in the top half of the cell interior.
    Number digits (e.g. '1' in blue) span the full height; flags are top-heavy."""
    ih, iw = inner.shape[:2]
    if ih < 2 or iw < 2:
        return False
    top_half = inner[:ih // 2, :]
    if top_half.ndim == 3:
        for low, high in [(_FLAG_BLUE_LOW, _FLAG_BLUE_HIGH), (_FLAG_RED_LOW, _FLAG_RED_HIGH)]:
            mask = cv2.inRange(inner, low, high)
            total = int(np.count_nonzero(mask))
            top_count = int(np.count_nonzero(mask[:ih // 2]))
            if top_count >= 20:
                ratio = top_count / total
                if ratio > 0.56:
                    return True
                # Flag triangle is compact; number digits have much larger coverage.
                # Accept a centered flag (ratio ≈ 0.5) if the colored area is small.
                if total < 350 and ratio >= 0.45:
                    return True
        return False
    return int(np.count_nonzero(top_half < 100)) >= 20


def _best_number_match(inner: np.ndarray) -> CellState | None:
    """Return the number CellState whose color range has the most pixels, or None if < 100."""
    best_state, best_count = None, 0
    for state, rng in _NUMBER_COLORS.items():
        count = _count_pixels(inner, rng["low"], rng["high"])
        if count > best_count:
            best_count, best_state = count, state
    return best_state if best_count > 100 else None


def _classify_cell(roi: np.ndarray) -> CellState:
    h, w = roi.shape[:2]
    pad = max(1, h // 8)
    gray = roi if roi.ndim == 2 else cv2.cvtColor(roi, cv2.COLOR_BGR2GRAY)
    inner = roi[pad:-pad, pad:-pad]
    if inner.size == 0:
        inner = roi

    if _is_unrevealed(gray):
        if _has_flag(inner):
            return CellState.FLAG
        if roi.ndim == 3:
            match = _best_number_match(inner)
            if match is not None:
                return match
        return CellState.UNREVEALED

    if roi.ndim == 3:
        best_state = _best_number_match(inner)
        if best_state is not None:
            # NUMBER_3 and NUMBER_1 share colors with flags.  In photos, some flags
            # lose bevel contrast entirely but keep the flag image.  Distinguish by
            # checking if the colored pixels are concentrated in the upper half
            # (flag shape) vs spread evenly (digit).
            if best_state in (CellState.NUMBER_3, CellState.NUMBER_1):
                ih, iw = inner.shape[:2]
                top_half = inner[:ih // 2, :]
                for low, high in [(_FLAG_RED_LOW, _FLAG_RED_HIGH),
                                   (_FLAG_BLUE_LOW, _FLAG_BLUE_HIGH)]:
                    total = _count_pixels(inner, low, high)
                    if total >= 40:
                        top_count = _count_pixels(top_half, low, high)
                        if top_count / total > 0.65:
                            return CellState.FLAG
            return best_state
        return CellState.EMPTY

    return CellState.EMPTY


def _detect_header(img: np.ndarray) -> tuple[Header, int]:
    """Returns (Header, y_offset) where y_offset is where the grid starts."""
    h, w = img.shape[:2]
    toolbar_h = int(h * 0.12)

    left_panel = img[:toolbar_h, :w // 5]
    mine_count = _read_led_display(left_panel)

    right_panel = img[:toolbar_h, w * 4 // 5:]
    timer = _read_led_display_str(right_panel)

    center = img[:toolbar_h, w // 3: 2 * w // 3]
    if center.ndim == 3:
        yellow_mask = cv2.inRange(center,
                                  np.array([0, 150, 150]),
                                  np.array([100, 255, 255]))
        has_hint = int(np.count_nonzero(yellow_mask)) > 500
    else:
        # Grayscale: yellow (~200) and red (~76) both map to mid-bright intensities
        bright_mask = cv2.inRange(center, np.array([180]), np.array([230]))
        has_hint = int(np.count_nonzero(bright_mask)) > 500

    # Locate the actual grid start by detecting the outer-border bevel highlight
    # (bright, uniform row just above the first cell row), followed by its shadow
    # (dark row), then the start of the first cell's interior.
    # Scan from halfway through the estimated toolbar to skip the status bar.
    grid_start = toolbar_h + max(4, int(h * 0.01))  # fallback
    cx0, cx1 = w // 5, 4 * w // 5
    y0_scan = max(toolbar_h // 2, 50)
    y1_scan = min(int(h * 0.25), h)
    strip = img[y0_scan:y1_scan, cx0:cx1].astype(np.float32)
    row_means = strip.mean(axis=1)
    row_stds = strip.std(axis=1)
    bevel_done = shadow_seen = False
    for i, (row_mean, row_std) in enumerate(zip(row_means, row_stds)):
        if not bevel_done and row_mean > 220 and row_std < 15:
            bevel_done = True
        elif bevel_done and not shadow_seen and row_mean < 160:
            shadow_seen = True
        elif shadow_seen and row_mean > 160:
            grid_start = y0_scan + i
            break

    return Header(mine_count=mine_count, timer=timer, has_hint=has_hint), grid_start


def _read_led_display(panel: np.ndarray) -> int:
    if panel.ndim == 3:
        mask = cv2.inRange(panel, np.array([0, 0, 150]), np.array([80, 80, 255]))
    else:
        mask = cv2.inRange(panel, np.array([60]), np.array([120]))
    pixels = int(np.count_nonzero(mask))
    return max(0, round(pixels / 400))


def _read_led_display_str(panel: np.ndarray) -> str:
    return "??"


def _find_grid_bounds(img: np.ndarray, y_start: int) -> tuple[int, int, int, int]:
    """Return (x0, y0, x1, y1) of the cell grid below the header."""
    region = img[y_start:]
    gray = region if region.ndim == 2 else cv2.cvtColor(region, cv2.COLOR_BGR2GRAY)
    _, thresh = cv2.threshold(gray, 160, 255, cv2.THRESH_BINARY)
    contours, _ = cv2.findContours(thresh, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    if not contours:
        h, w = img.shape[:2]
        return 0, y_start, w, h
    largest = max(contours, key=cv2.contourArea)
    gx, gy, gw, gh = cv2.boundingRect(largest)
    return gx, y_start + gy, gx + gw, y_start + gy + gh


def _find_cell_y_offset(grid_img: np.ndarray, cell_h: int) -> int:
    """Detect the y offset where actual cells start within grid_img.

    The top of grid_img may contain a thin strip from the header area before the
    first cell's bright top-highlight.  Find the first bright row (>220) that
    follows a darker region (<150) within the first cell_h rows.
    """
    gray = (grid_img if grid_img.ndim == 2 else cv2.cvtColor(grid_img, cv2.COLOR_BGR2GRAY)).astype(float)
    cx = min(35, grid_img.shape[1] // 5)
    strip = gray[:cell_h, cx:cx + 15].mean(axis=1)

    seen_dark = False
    for y in range(len(strip)):
        if strip[y] < 150:
            seen_dark = True
        elif seen_dark and strip[y] > 220:
            return y
    return 0


def _best_cell_count(profile: np.ndarray, total: int,
                      min_cells: int = 5, max_cells: int = 50) -> int:
    """Score each candidate cell count by gradient energy concentration at boundaries."""
    total_energy = float(np.sum(profile))
    if total_energy < 1:
        return 1

    win = max(2, total // 500)
    best_n, best_score = 1, -1.0

    for n in range(min_cells, max_cells + 1):
        period = total / n
        energy = 0.0
        covered = 0
        for k in range(1, n):
            pos = int(round(k * period))
            lo = max(0, pos - win)
            hi = min(len(profile), pos + win + 1)
            energy += float(np.sum(profile[lo:hi]))
            covered += hi - lo

        expected = total_energy * (covered / total)
        score = energy / expected if expected > 0 else 0.0

        if score > best_score:
            best_score = score
            best_n = n

    return best_n


def _estimate_grid_size(grid_img: np.ndarray) -> tuple[int, int]:
    gray = (grid_img if grid_img.ndim == 2 else cv2.cvtColor(grid_img, cv2.COLOR_BGR2GRAY)).astype(np.float32)
    gh, gw = grid_img.shape[:2]

    h_profile = np.abs(cv2.Sobel(gray, cv2.CV_64F, 0, 1, ksize=3)).sum(axis=1)
    v_profile = np.abs(cv2.Sobel(gray, cv2.CV_64F, 1, 0, ksize=3)).sum(axis=0)

    rows = _best_cell_count(h_profile, gh)
    cols = _best_cell_count(v_profile, gw)

    if cols > 1:
        sq_rows = max(1, round(gh / (gw / cols)))
        if rows == 1 or abs(rows - sq_rows) > max(2, round(sq_rows * 0.15)):
            rows = sq_rows
    if rows > 1:
        sq_cols = max(1, round(gw / (gh / rows)))
        if cols == 1 or abs(cols - sq_cols) > max(2, round(sq_cols * 0.15)):
            cols = sq_cols

    return rows, cols


def parse_board(image_path: str | Path) -> Board:
    bgr = cv2.imread(str(image_path))
    if bgr is None:
        raise FileNotFoundError(f"Cannot load image: {image_path}")
    img = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)

    header, grid_y = _detect_header(img)
    x0, y0, x1, y1 = _find_grid_bounds(img, grid_y)
    grid_img = img[y0:y1, x0:x1]
    bgr_grid = bgr[y0:y1, x0:x1]

    rows, cols = _estimate_grid_size(grid_img)
    cell_h = grid_img.shape[0] // rows
    cell_w = grid_img.shape[1] // cols

    # Detect sub-pixel phase: the grid_img may start a few pixels before the first
    # cell's top bevel, so actual cells begin at y_offset rather than y=0.
    y_offset = _find_cell_y_offset(grid_img, cell_h)
    gh = grid_img.shape[0]
    # Actual cell height for full rows (rows 1..rows-1); row 0 is a partial fragment
    actual_cell_h = (gh - y_offset) // (rows - 1) if (rows > 1 and y_offset > 0) else cell_h

    board = Board(rows=rows, cols=cols, header=header)
    board.cells = []

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
            state = _classify_cell(roi)
            row_cells.append(Cell(
                row=r, col=c, state=state,
                x=x0 + cx, y=y0 + cy,
                width=cell_w, height=h,
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
