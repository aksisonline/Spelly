"""Draws the Spelly cat (pink, round, two ears) to assets/icon.png and assets/icon.ico.
Pure stdlib, so there is no design file to lose: run `python3 packaging/make_icon.py`."""
import struct, zlib, os

SS = 3                               # supersampling
PINK, DARK, BG = (244, 114, 182), (131, 24, 67), (0, 0, 0)

def tri(x, y, a, b, c):
    def s(p, q, r): return (p[0]-r[0])*(q[1]-r[1]) - (q[0]-r[0])*(p[1]-r[1])
    d1, d2, d3 = s((x, y), a, b), s((x, y), b, c), s((x, y), c, a)
    return not ((d1 < 0 or d2 < 0 or d3 < 0) and (d1 > 0 or d2 > 0 or d3 > 0))

def shade(x, y):                     # x, y on the 44 grid -> (r, g, b, covered)
    for ex in (16, 28):
        if abs(x-ex) <= 1.3 and abs(y-23) <= 2.6: return DARK, 1
    if abs(x-22) <= 1.3 and abs(y-29) <= 1.0: return DARK, 1
    face = ((x-22)/17)**2 + ((y-25)/14)**2 <= 1
    ears = tri(x, y, (6, 22), (7, 4), (21, 13)) or tri(x, y, (38, 22), (37, 4), (23, 13))
    return (PINK, 1) if face or ears else (BG, 0)

def render(N):
    S = N / 44.0                      # the design lives on a 44x44 grid
    rows = []
    for py in range(N):
        row = bytearray()
        for px in range(N):
            r = g = b = cov = 0
            for sy in range(SS):
                for sx in range(SS):
                    (cr, cg, cb), c = shade((px+(sx+.5)/SS)/S, (py+(sy+.5)/SS)/S)
                    r += cr*c; g += cg*c; b += cb*c; cov += c
            n = SS*SS
            a = cov*255//n
            row += bytes((r//cov, g//cov, b//cov, a)) if cov else b"\0\0\0\0"
        rows.append(bytes(row))
    return rows

def png(rows, N):
    def chunk(t, d): return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t+d))
    raw = b"".join(b"\0"+r for r in rows)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", N, N, 8, 6, 0, 0, 0)) \
        + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")

data = png(render(512), 512)
small = png(render(256), 256)
os.makedirs("assets", exist_ok=True)
open("assets/icon.png", "wb").write(data)
# ICO with one embedded 256px-class PNG entry (0 = 256)
ico = struct.pack("<HHH", 0, 1, 1) + struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(small), 22) + small
open("assets/icon.ico", "wb").write(ico)
print("wrote assets/icon.png, assets/icon.ico")
