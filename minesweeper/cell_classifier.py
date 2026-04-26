import numpy as np
import cv2
from models import CellState

_NUMBER_COLORS = {
    CellState.NUMBER_1: (np.array([100, 0, 0]),  np.array([255, 80, 80])),   # blue
    CellState.NUMBER_2: (np.array([0, 80, 0]),   np.array([80, 200, 80])),   # green
    CellState.NUMBER_3: (np.array([0, 0, 150]),  np.array([80, 80, 255])),   # red
    CellState.NUMBER_4: (np.array([80, 0, 0]),   np.array([160, 60, 100])),  # dark navy
    CellState.NUMBER_5: (np.array([0, 0, 80]),   np.array([100, 100, 160])), # maroon
    CellState.NUMBER_6: (np.array([81, 61, 205]), np.array([170, 150, 255])), # bright coral/pink (B/G max < 192 to exclude gray bg)
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
      are recovered via number detection in classify_cell.
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
    # cell border.  Scale threshold by cell area so revealed cells in large-cell
    # images (bright_count~100-156) don't alias as unrevealed (bright_count~572-780).
    bright_count = int(np.count_nonzero(roi[:, bw:w - bw] > 240))
    return bright_count >= max(100, h * (w - 2 * bw) // 30)


def _has_flag(inner: np.ndarray) -> bool:
    """Flag has coloured content concentrated in the top half of the cell interior.
    Number digits (e.g. '1' in blue) span the full height; flags are top-heavy."""
    ih, iw = inner.shape[:2]
    if ih < 2 or iw < 2:
        return False
    if inner.ndim == 3:
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
    return int(np.count_nonzero(inner[:ih // 2] < 100)) >= 20


def _best_number_match(inner: np.ndarray) -> CellState | None:
    """Return the number CellState whose color range has the most pixels, or None if < 100."""
    best_state, best_count = None, 0
    for state, (low, high) in _NUMBER_COLORS.items():
        count = _count_pixels(inner, low, high)
        if count > best_count:
            best_count, best_state = count, state
    return best_state if best_count > 100 else None


def classify_cell(roi: np.ndarray) -> CellState:
    h, w = roi.shape[:2]
    pad = max(1, h // 8)
    gray = _to_gray(roi)
    inner = roi[pad:-pad, pad:-pad]
    if inner.size == 0:
        inner = roi

    if _is_unrevealed(gray):
        if roi.ndim == 3:
            # NUMBER_4's dark navy aliases as FLAG_BLUE; digit coverage >10% wins.
            if _count_pixels(inner, *_NUMBER_COLORS[CellState.NUMBER_4]) > max(200, inner.shape[0] * inner.shape[1] // 10):
                return CellState.NUMBER_4
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
                ih = inner.shape[0]
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
