"""App icon + tray icon from the kitten mascot. Run after prepare_mascots.py:

    python3 packaging/make_icon.py        # needs Pillow; outputs are committed
"""
import struct
from PIL import Image, ImageDraw

kitten = Image.open("assets/mascot/kitten.png").convert("RGBA")


def squircle(size, top=(255, 226, 238), bottom=(255, 190, 215)):
    """Soft pink rounded square with a gentle vertical gradient."""
    bg = Image.new("RGBA", (size, size))
    px = bg.load()
    for y in range(size):
        t = y / (size - 1)
        c = tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,)
        for x in range(size):
            px[x, y] = c
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius=round(size * 0.225), fill=255)
    bg.putalpha(mask)
    return bg


def icon(size):
    base = squircle(size)
    k = int(size * 0.72)
    cat = kitten.resize((k, round(k * kitten.height / kitten.width)), Image.NEAREST)
    base.alpha_composite(cat, ((size - cat.width) // 2, (size - cat.height) // 2 + size // 40))
    return base


icon(512).save("assets/icon.png")
icon(256).save("assets/icon.ico", sizes=[(256, 256), (128, 128), (64, 64), (48, 48), (32, 32), (16, 16)])

# Tray / menu bar: the kitten alone, 44px, as raw RGBA with an 8-byte (w, h) header.
t = kitten.resize((44, round(44 * kitten.height / kitten.width)), Image.LANCZOS)
canvas = Image.new("RGBA", (44, 44), (0, 0, 0, 0))
canvas.alpha_composite(t, (0, (44 - t.height) // 2))
open("assets/tray.rgba", "wb").write(struct.pack("<II", 44, 44) + canvas.tobytes())
print("assets/icon.png, assets/icon.ico, assets/tray.rgba")
