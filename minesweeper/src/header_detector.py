import numpy as np
from cell_classifier import _to_gray
from models import Header

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
        sw = max(1, pw // 4)
        th = max(1, int(ph * 0.15))
        mh = max(1, int(ph * 0.12))

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


def detect_header(img: np.ndarray, bgr: np.ndarray | None = None) -> tuple[Header, int]:
    """Return (Header, grid_y) where grid_y is the pixel row where the cell grid starts."""
    h, w = img.shape[:2]
    toolbar_h = int(h * 0.12)

    led_src = bgr if bgr is not None else img
    left_panel = led_src[:toolbar_h, :w // 5]
    mine_count = _read_led_display(left_panel)

    # Locate the actual grid start by detecting the outer-border bevel highlight
    # (bright, uniform row just above the first cell row), followed by its shadow
    # (dark row), then the start of the first cell's interior.
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
