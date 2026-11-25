"""Turns the source pixel-art cats in assets/mascot/source into clean transparent PNGs
(snapped to their pixel grid, integer-upscaled so they stay crisp):

    python3 packaging/prepare_mascots.py

kitten  = main mascot + app icon      sleepy = paused
yawn    = nothing-to-show-yet         tuxedo = no search results
"""
import os
from collections import deque
from PIL import Image

SRC, OUT = "assets/mascot/source", "assets/mascot"
JOBS = {"kitten": "kitten.jpg", "sleepy": "sleepy.png", "yawn": "yawn.webp", "tuxedo": "tuxedo.jpg"}
TARGET_H = 240


def remove_background(im, tol=48):
    """Flood-fill the near-white background from the border into transparency."""
    im = im.convert("RGBA")
    w, h = im.size
    px = im.load()
    near_white = lambda p: p[0] > 255 - tol and p[1] > 255 - tol and p[2] > 255 - tol
    seen = [[False] * h for _ in range(w)]
    q = deque((x, y) for x in range(w) for y in (0, h - 1))
    q.extend((x, y) for y in range(h) for x in (0, w - 1))
    while q:
        x, y = q.popleft()
        if not (0 <= x < w and 0 <= y < h) or seen[x][y]:
            continue
        seen[x][y] = True
        if near_white(px[x, y]):
            px[x, y] = (255, 255, 255, 0)
            q.extend(((x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)))
    return im


def crop(im):
    return im.crop(im.getchannel("A").point(lambda a: 255 if a > 10 else 0).getbbox())


def best_block(im):
    """Largest square block size whose down/up-sample barely changes the picture."""
    w, h = im.size
    best, rgb = 1, im.convert("RGBA")
    for b in range(2, 80):
        if w // b < 8 or h // b < 8:
            break
        small = rgb.resize((max(1, round(w / b)), max(1, round(h / b))), Image.NEAREST)
        back = small.resize((w, h), Image.NEAREST)
        diff = sum(
            abs(a - c)
            for p, q in zip(rgb.resize((w // 4 or 1, h // 4 or 1)).getdata(), back.resize((w // 4 or 1, h // 4 or 1)).getdata())
            for a, c in zip(p, q)
        ) / (w // 4 * (h // 4) * 4 or 1)
        if diff < 9:
            best = b
    return best


def clean_jpeg_noise(im):
    """JPEG leaves stray faint pixels; make alpha binary."""
    a = im.getchannel("A").point(lambda v: 255 if v > 128 else 0)
    im.putalpha(a)
    return im


for name, file in JOBS.items():
    im = Image.open(f"{SRC}/{file}")
    im = clean_jpeg_noise(crop(remove_background(im)))
    b = best_block(im)
    # snap to the sprite's own pixel grid
    small = im.resize((max(1, round(im.width / b)), max(1, round(im.height / b))), Image.NEAREST)
    scale = max(1, TARGET_H // small.height)
    out = small.resize((small.width * scale, small.height * scale), Image.NEAREST)
    out.save(f"{OUT}/{name}.png")
    print(f"{name}: block {b}px -> {small.size} grid, x{scale} = {out.size}")
