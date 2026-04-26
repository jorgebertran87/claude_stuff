import numpy as np
import cv2
from cell_classifier import _to_gray


def find_grid_bounds(img: np.ndarray, y_start: int) -> tuple[int, int, int, int]:
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


def find_cell_y_offset(grid_img: np.ndarray, cell_h: int) -> int:
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


def estimate_grid_size(grid_img: np.ndarray) -> tuple[int, int]:
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
