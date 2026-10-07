"""Builds DPIMech's icons and logo images from the hand-drawn SVG logo.

Usage: python tools/make_icon.py
Needs CairoSVG and Pillow (pip install cairosvg pillow; CairoSVG needs the cairo library, which
Linux and macOS usually have). Sources in crates/gui/assets/:
  logo.svg              the mark: app icon everywhere
  logo-long.svg         mark + "PIMECH" in one line: the window's header
  logo-long-square.svg  mark above the name: README
Writes into crates/gui/assets/:
  logo.png        512 px mark on a transparent square (window icon, wizard, launcher)
  logo-long.png   one-line logo, 96 px high (sidebar, Easy mode top bar)
  dpimech.png     256 px mark (notifications, Linux menu icon, shortcut icons)
  dpimech.ico     16–256 px, embedded into both executables and the installer
  tray-32.rgba    raw 32×32 RGBA for the tray icon (status dot is drawn on top at runtime)
  icon-1024.png   1024 px mark for the macOS .icns (tools/build-macos-app.sh)
and site/logo.svg + site/favicon.ico for the website.
"""

import io
import os
import shutil

import cairosvg
from PIL import Image

ROOT = os.path.join(os.path.dirname(__file__), "..")
ASSETS = os.path.join(ROOT, "crates", "gui", "assets")
SITE = os.path.join(ROOT, "site")
MARGIN = 0.06  # room around the mark so it does not touch the icon's edges


def render(svg, width=None, height=None):
    data = cairosvg.svg2png(url=svg, output_width=width, output_height=height)
    return Image.open(io.BytesIO(data)).convert("RGBA")


def mark(size):
    """The mark drawn at this exact size (sharper than scaling one big image down),
    centred on a transparent square."""
    inner = round(size * (1 - 2 * MARGIN))
    drawing = render(os.path.join(ASSETS, "logo.svg"), height=inner)
    if drawing.width > inner:
        drawing = render(os.path.join(ASSETS, "logo.svg"), width=inner)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.paste(drawing, ((size - drawing.width) // 2, (size - drawing.height) // 2))
    return out


def main():
    mark(512).save(os.path.join(ASSETS, "logo.png"), optimize=True)
    mark(256).save(os.path.join(ASSETS, "dpimech.png"), optimize=True)
    mark(1024).save(os.path.join(ASSETS, "icon-1024.png"), optimize=True)
    render(os.path.join(ASSETS, "logo-long.svg"), height=96).save(
        os.path.join(ASSETS, "logo-long.png"), optimize=True
    )

    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = [mark(s) for s in sizes]
    images[-1].save(
        os.path.join(ASSETS, "dpimech.ico"),
        sizes=[(s, s) for s in sizes],
        append_images=images[:-1],
    )
    with open(os.path.join(ASSETS, "tray-32.rgba"), "wb") as f:
        f.write(mark(32).tobytes())

    shutil.copyfile(os.path.join(ASSETS, "logo.svg"), os.path.join(SITE, "logo.svg"))
    images[2].save(
        os.path.join(SITE, "favicon.ico"),
        sizes=[(16, 16), (32, 32), (48, 48)],
        append_images=[images[0], images[3]],
    )
    print("icons written to", os.path.normpath(ASSETS), "and", os.path.normpath(SITE))


if __name__ == "__main__":
    main()
