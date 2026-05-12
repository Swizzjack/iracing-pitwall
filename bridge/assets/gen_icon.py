#!/usr/bin/env python3
"""Generate assets/icon.ico — dark background (#1a1a1a) with yellow 'IR' text (#facc15).
Run from the bridge/assets/ directory or any directory; .ico is written next to this file."""

import io, struct, subprocess, sys, os

# Install Pillow silently if missing
try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    subprocess.check_call([sys.executable, "-m", "pip", "install", "--user", "--quiet", "Pillow"])
    from PIL import Image, ImageDraw, ImageFont

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(SCRIPT_DIR, "icon.ico")

BG    = (26, 26, 26)    # #1a1a1a
FG    = (250, 204, 21)  # #facc15
SIZES = [16, 24, 32, 48, 64, 256]

FONT_CANDIDATES = [
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/usr/share/fonts/liberation-sans/LiberationSans-Bold.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Bold.ttf",
]

def get_font(size):
    for path in FONT_CANDIDATES:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                pass
    # Pillow 10.0+ ships a built-in TrueType that respects the size parameter.
    return ImageFont.load_default(size=size)

def make_image(size):
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # Rounded rect background — radius ~15% of size
    r = max(2, round(size * 0.15))
    draw.rounded_rectangle([(0, 0), (size - 1, size - 1)], radius=r, fill=BG)

    # Font at 70% of icon height; stroke_width fakes bold for systems without a Bold TTF.
    font_size = max(6, round(size * 0.70))
    stroke = max(0, round(size * 0.04))
    font = get_font(font_size)

    text = "IR"
    bbox = draw.textbbox((0, 0), text, font=font, stroke_width=stroke)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    tx = (size - tw) / 2 - bbox[0]
    ty = (size - th) / 2 - bbox[1]
    draw.text((tx, ty), text, fill=FG, font=font, stroke_width=stroke, stroke_fill=FG)

    return img

def save_ico(path, sizes):
    """Assemble a multi-resolution ICO manually so each size is rendered natively."""
    # Render each size and encode to PNG bytes
    png_blobs = []
    for s in sizes:
        buf = io.BytesIO()
        make_image(s).save(buf, format="PNG")
        png_blobs.append(buf.getvalue())

    N = len(sizes)
    # ICO directory starts after 6-byte header + N×16-byte entries
    data_offset = 6 + N * 16

    # ICONDIR header
    ico = struct.pack("<HHH", 0, 1, N)

    # ICONDIRENTRY for each image
    for s, data in zip(sizes, png_blobs):
        bw = 0 if s == 256 else s   # 0 signals 256 per ICO spec
        bh = 0 if s == 256 else s
        ico += struct.pack("<BBBBHHII", bw, bh, 0, 0, 1, 32, len(data), data_offset)
        data_offset += len(data)

    # Image data
    for data in png_blobs:
        ico += data

    with open(path, "wb") as f:
        f.write(ico)

save_ico(OUT, SIZES)
print(f"Generated {OUT}  ({os.path.getsize(OUT):,} bytes,  sizes: {SIZES})")
