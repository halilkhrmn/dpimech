"""Builds DPIMech's icons from the pixel-art logo.

Usage: python tools/make_icon.py [source image]   (default: crates/gui/assets/logo-source.png)
Needs Pillow (pip install pillow). Writes into crates/gui/assets/:
  logo.png        512 px, transparent, used by the UI (window icon, header, About)
  dpimech.png     256 px, used for Windows notifications
  dpimech.ico     16–256 px, embedded into both executables
  tray-32.rgba    raw 32×32 RGBA for the tray icon (status dot is drawn on top at runtime)
"""

import os
import sys
from collections import deque

from PIL import Image

ROOT = os.path.join(os.path.dirname(__file__), "..")
ASSETS = os.path.join(ROOT, "crates", "gui", "assets")
WHITE_THRESHOLD = 40  # distance from pure white still counted as background (JPEG noise)


def remove_background(img):
    """Makes the white area connected to the image border transparent. White that is
    enclosed by the drawing (eye highlights) stays."""
    img = img.convert("RGBA")
    w, h = img.size
    px = img.load()

    def is_bg(x, y):
        r, g, b, _ = px[x, y]
        return 255 * 3 - (r + g + b) <= WHITE_THRESHOLD * 3

    seen = bytearray(w * h)
    queue = deque()
    for x in range(w):
        queue.extend(((x, 0), (x, h - 1)))
    for y in range(h):
        queue.extend(((0, y), (w - 1, y)))
    while queue:
        x, y = queue.popleft()
        i = y * w + x
        if seen[i] or not is_bg(x, y):
            continue
        seen[i] = 1
        px[x, y] = (0, 0, 0, 0)
        if x > 0:
            queue.append((x - 1, y))
        if x < w - 1:
            queue.append((x + 1, y))
        if y > 0:
            queue.append((x, y - 1))
        if y < h - 1:
            queue.append((x, y + 1))
    return img


def clear_enclosed_holes(img, below=0.6):
    """Background showing through the curled tentacles is enclosed by the drawing, so the
    border flood fill misses it. Near-white islands in the lower part of the drawing are
    holes; the eye highlights higher up stay white."""
    px = img.load()
    box = img.getbbox()
    if not box:
        return img
    limit = box[1] + (box[3] - box[1]) * below
    w, h = img.size
    for y in range(int(limit), h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if a and 255 * 3 - (r + g + b) <= WHITE_THRESHOLD * 3:
                px[x, y] = (0, 0, 0, 0)
    return img


def centered(img, margin=0.04):
    """Crops to the drawing and centres it on a transparent square."""
    box = img.getbbox()
    if not box:
        return img
    drawing = img.crop(box)
    w, h = drawing.size
    side = int(max(w, h) * (1 + 2 * margin))
    out = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    out.paste(drawing, ((side - w) // 2, (side - h) // 2))
    return out


def resized(img, size):
    # Large sizes keep the pixel-art look; tiny ones need smoothing to stay readable.
    method = Image.Resampling.NEAREST if size >= 128 else Image.Resampling.LANCZOS
    return img.resize((size, size), method)


def main():
    source = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ASSETS, "logo-source.png")
    os.makedirs(ASSETS, exist_ok=True)
    img = Image.open(source)
    if img.mode != "RGBA" or img.getextrema()[3][0] == 255:
        img = clear_enclosed_holes(remove_background(img))
    logo = centered(img)

    resized(logo, 512).save(os.path.join(ASSETS, "logo.png"), optimize=True)
    resized(logo, 256).save(os.path.join(ASSETS, "dpimech.png"), optimize=True)
    sizes = [16, 24, 32, 48, 64, 128, 256]
    resized(logo, 256).save(
        os.path.join(ASSETS, "dpimech.ico"),
        sizes=[(s, s) for s in sizes],
        append_images=[resized(logo, s) for s in sizes[:-1]],
    )
    with open(os.path.join(ASSETS, "tray-32.rgba"), "wb") as f:
        f.write(resized(logo, 32).tobytes())
    # Keep a lossless master so the JPEG is not needed again.
    master = os.path.join(ASSETS, "logo-source.png")
    if os.path.abspath(source) != os.path.abspath(master):
        resized(logo, 1024).save(master, optimize=True)
    print("icons written to", os.path.normpath(ASSETS))


if __name__ == "__main__":
    main()
