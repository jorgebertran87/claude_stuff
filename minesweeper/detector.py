import numpy as np
import cv2
from pathlib import Path
from models import Board, Cell, CellState, Header


_NUMBER_COLORS = {
    CellState.NUMBER_1: (np.array([100, 0, 0]),  np.array([255, 80, 80])),   # blue
    CellState.NUMBER_2: (np.array([0, 80, 0]),   np.array([80, 200, 80])),   # green
    CellState.NUMBER_3: (np.array([0, 0, 150]),  np.array([80, 80, 255])),   # red
    CellState.NUMBER_4: (np.array([80, 0, 0]),   np.array([160, 60, 100])),  # dark navy
}

_FLAG_RED_LOW  = np.array([0,   0, 150])
_FLAG_RED_HIGH = np.array([80, 80, 255])
_FLAG_BLUE_LOW  = np.array([150,  0,  0])   # BGR: high B, low G/R (this app's flag style)
_FLAG_BLUE_HIGH = np.array([255, 90, 90])


def _to_gray(img: np.ndarray) -> np.ndarray:
    return img if img.ndim == 2 else cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)


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
                # Flag triangle is compact; accept a centered flag (ratio ≈ 0.5) if area is small.
                if ratio > 0.56 or (total < 350 and ratio >= 0.45):
                    return True
        return False
    return int(np.count_nonzero(top_half < 100)) >= 20


def _best_number_match(inner: np.ndarray) -> CellState | None:
    """Return the number CellState whose color range has the most pixels, or None if < 100."""
    best_state, best_count = None, 0
    for state, (low, high) in _NUMBER_COLORS.items():
        count = _count_pixels(inner, low, high)
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


def _detect_header(img: np.ndarray, bgr: np.ndarray | None = None) -> tuple[Header, int]:
    """Returns (Header, y_offset) where y_offset is where the grid starts."""
    h, w = img.shape[:2]
    toolbar_h = int(h * 0.12)

    led_src = bgr if bgr is not None else img
    left_panel = led_src[:toolbar_h, :w // 5]
    mine_count = _read_led_display(left_panel)

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

    return Header(mine_count=mine_count), grid_start


_SEGS: dict[tuple[int, ...], int] = {
    (1, 1, 1, 0, 1, 1, 1): 0, (0, 0, 1, 0, 0, 1, 0): 1,
    (1, 0, 1, 1, 1, 0, 1): 2, (1, 0, 1, 1, 0, 1, 1): 3,
    (0, 1, 1, 1, 0, 1, 0): 4, (1, 1, 0, 1, 0, 1, 1): 5,
    (1, 1, 0, 1, 1, 1, 1): 6, (1, 0, 1, 0, 0, 1, 0): 7,
    (1, 1, 1, 1, 1, 1, 1): 8, (1, 1, 1, 1, 0, 1, 1): 9,
}


def _read_led_display(panel: np.ndarray) -> int:
    """Decode a 3-digit 7-segment LED mine counter from a BGR or grayscale panel."""
    if panel.ndim == 3:
        r = panel[:, :, 2].astype(int)
        g = panel[:, :, 1].astype(int)
        b = panel[:, :, 0].astype(int)
        red = (r > 120) & (g < 80) & (b < 80)
    else:
        # Red segments appear ~60–80 in grayscale after BGR→GRAY conversion
        red = (panel > 50) & (panel < 95)

    # Trim to rows containing lit segments (ignores content outside this panel's columns).
    row_any = red.any(axis=1)
    if not row_any.any():
        return 0
    ys = np.where(row_any)[0]
    y0, y1 = int(ys[0]), int(ys[-1]) + 1
    red = red[y0:y1, :]
    ph = red.shape[0]

    col_prof = red.sum(axis=0)
    active = (col_prof > 0).astype(np.int8)
    changes = np.diff(active, prepend=0, append=0)
    starts = np.where(changes == 1)[0]
    ends = np.where(changes == -1)[0] - 1
    groups = list(zip(starts.tolist(), ends.tolist()))
    if not groups:
        return 0

    # Use the widest group as canonical digit-cell width so that narrow digits (e.g. "1")
    # are still placed at the correct position.
    digit_w = max(x1 - x0 + 1 for x0, x1 in groups)
    gap = max(2, digit_w // 8)
    x_start = groups[0][0]
    step = digit_w + gap
    cells = [(x_start + k * step, x_start + k * step + digit_w - 1) for k in range(3)]

    result = 0
    for x0, x1 in cells:
        x0 = max(0, x0)
        x1 = min(red.shape[1] - 1, x1)
        if x0 > x1:
            result *= 10
            continue
        d = red[:, x0:x1 + 1].astype(float)
        pw = d.shape[1]
        sw = max(1, pw // 4)   # columns for vertical-segment corner regions
        th = max(1, int(ph * 0.15))  # rows for top/bottom horizontal segments
        mh = max(1, int(ph * 0.12))  # half-height of middle-segment strip

        top = d[:th, :].sum()
        bot = d[-th:, :].sum()
        mid = d[ph // 2 - mh // 2: ph // 2 + mh // 2 + 1, :].sum()
        ul  = d[:ph // 2, :sw].sum()
        ur  = d[:ph // 2, -sw:].sum()
        ll  = d[ph // 2:, :sw].sum()
        lr  = d[ph // 2:, -sw:].sum()

        ref = max(top, bot, ul, ur, ll, lr, 1.0)
        t = ref * 0.4
        segs = (
            int(top > t),
            int(ul  > t / 2),
            int(ur  > t / 2),
            int(mid > t * 0.7),
            int(ll  > t / 2),
            int(lr  > t / 2),
            int(bot > t),
        )
        result = result * 10 + _SEGS.get(segs, 0)

    return result


def _find_grid_bounds(img: np.ndarray, y_start: int) -> tuple[int, int, int, int]:
    """Return (x0, y0, x1, y1) of the cell grid below the header."""
    region = img[y_start:]
    gray = _to_gray(region)
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
    gray = _to_gray(grid_img).astype(float)
    cx = min(35, grid_img.shape[1] // 5)
    strip = gray[:cell_h, cx:cx + 15].mean(axis=1)

    seen_dark = False
    for y in range(len(strip)):
        if strip[y] < 150:
            seen_dark = True
        elif seen_dark and strip[y] > 220:
            return y
    return 0


def _autocorr_period(profile: np.ndarray, total: int) -> int:
    """Return the lag with peak autocorrelation in the search range."""
    p_lo = max(30, total // 50)
    p_hi = min(total // 3, max(100, total // 15))
    if p_hi <= p_lo:
        return total
    pn = (profile - profile.mean()).astype(np.float64)
    best_val, best_lag = -np.inf, p_lo
    for lag in range(p_lo, p_hi + 1):
        val = float(np.dot(pn[:total - lag], pn[lag:]))
        if val > best_val:
            best_val, best_lag = val, lag
    return best_lag


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
    gray = _to_gray(grid_img).astype(np.float32)
    gh, gw = grid_img.shape[:2]

    h_profile = np.abs(cv2.Sobel(gray, cv2.CV_64F, 0, 1, ksize=3)).sum(axis=1)
    v_profile = np.abs(cv2.Sobel(gray, cv2.CV_64F, 1, 0, ksize=3)).sum(axis=0)

    rows = _best_cell_count(h_profile, gh)
    cols = _best_cell_count(v_profile, gw)

    if cols > 1:
        auto_rows = max(1, round(gh / _autocorr_period(h_profile, gh)))
        if rows == 1 or abs(rows - auto_rows) > 1:
            rows = auto_rows
        sq_rows = max(1, round(gh / (gw / cols)))
        if rows == 1 or abs(rows - sq_rows) > max(2, round(sq_rows * 0.15)):
            rows = sq_rows
    if rows > 1:
        sq_cols = max(1, round(gw / (gh / rows)))
        if cols == 1 or abs(cols - sq_cols) > max(2, round(sq_cols * 0.15)):
            cols = sq_cols

    return rows, cols


def _parse_board_bgr(bgr: np.ndarray) -> Board:
    img = cv2.cvtColor(bgr, cv2.COLOR_BGR2GRAY)

    header, grid_y = _detect_header(img, bgr=bgr)
    x0, y0, x1, y1 = _find_grid_bounds(img, grid_y)
    grid_img = img[y0:y1, x0:x1]
    bgr_grid = bgr[y0:y1, x0:x1]

    rows, cols = _estimate_grid_size(grid_img)
    cell_h = grid_img.shape[0] // rows
    cell_w = grid_img.shape[1] // cols

    y_offset = _find_cell_y_offset(grid_img, cell_h)
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
            state = _classify_cell(roi)
            row_cells.append(Cell(
                row=r, col=c, state=state,
                x=x0 + cx, y=y0 + cy,
                width=cell_w, height=h,
            ))
        board.cells.append(row_cells)

    return board


def parse_board(image_path: str | Path) -> Board:
    bgr = cv2.imread(str(image_path))
    if bgr is None:
        raise FileNotFoundError(f"Cannot load image: {image_path}")
    return _parse_board_bgr(bgr)


def parse_board_bytes(data: bytes) -> Board:
    buf = np.frombuffer(data, np.uint8)
    bgr = cv2.imdecode(buf, cv2.IMREAD_COLOR)
    if bgr is None:
        raise ValueError("Cannot decode image data")
    return _parse_board_bgr(bgr)


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
