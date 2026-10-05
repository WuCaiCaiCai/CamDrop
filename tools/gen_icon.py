"""One-off generator for assets/icon.png and assets/icon.ico.

Draws the CamDrop mark (lake-blue rounded tile, white down-arrow into a tray)
from the same geometry as assets/icon.svg, supersampled 4x. Run once:

    python tools/gen_icon.py

Uses only the standard library, so no Pillow or SVG tooling is required.
"""

import os
import struct
import zlib

SIZE = 256
SS = 4
N = SIZE * SS
S = 256.0

TOP = (70, 191, 230)
BOTTOM = (30, 130, 168)
WHITE = (255, 255, 255, 255)
OUT_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "assets")


def in_rect(px, py, x0, y0, x1, y1):
    return x0 <= px <= x1 and y0 <= py <= y1


def in_round_rect(px, py, x0, y0, x1, y1, r):
    if px < x0 or px > x1 or py < y0 or py > y1:
        return False
    cx = min(max(px, x0 + r), x1 - r)
    cy = min(max(py, y0 + r), y1 - r)
    dx = px - cx
    dy = py - cy
    return dx * dx + dy * dy <= r * r


def sign(p1, p2, p3):
    return (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])


def in_triangle(px, py, a, b, c):
    d1 = sign((px, py), a, b)
    d2 = sign((px, py), b, c)
    d3 = sign((px, py), c, a)
    neg = d1 < 0 or d2 < 0 or d3 < 0
    pos = d1 > 0 or d2 > 0 or d3 > 0
    return not (neg and pos)


def sample(u, v):
    stem = in_rect(u, v, 115 / S, 56 / S, 141 / S, 130 / S)
    head = in_triangle(u, v, (79 / S, 128 / S), (177 / S, 128 / S), (128 / S, 172 / S))
    tray = (
        in_rect(u, v, 48.5 / S, 150 / S, 63.5 / S, 192 / S)
        or in_rect(u, v, 192.5 / S, 150 / S, 207.5 / S, 192 / S)
        or in_rect(u, v, 48.5 / S, 184.5 / S, 207.5 / S, 199.5 / S)
    )
    if stem or head or tray:
        return WHITE
    if in_round_rect(u, v, 16 / S, 16 / S, 240 / S, 240 / S, 54 / S):
        return (
            round(TOP[0] + (BOTTOM[0] - TOP[0]) * v),
            round(TOP[1] + (BOTTOM[1] - TOP[1]) * v),
            round(TOP[2] + (BOTTOM[2] - TOP[2]) * v),
            255,
        )
    return (0, 0, 0, 0)


def render_raw():
    out = bytearray()
    for y in range(SIZE):
        out.append(0)  # PNG filter type: none
        for x in range(SIZE):
            acc = [0, 0, 0, 0]
            for sy in range(SS):
                for sx in range(SS):
                    u = (x * SS + sx + 0.5) / N
                    v = (y * SS + sy + 0.5) / N
                    c = sample(u, v)
                    for i in range(4):
                        acc[i] += c[i]
            count = SS * SS
            for i in range(4):
                out.append(round(acc[i] / count))
    return bytes(out)


def chunk(tag, data):
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def build_png(raw):
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 9))
    png += chunk(b"IEND", b"")
    return png


def build_ico(png):
    header = struct.pack("<HHH", 0, 1, 1)
    # width/height 0 means 256; offset = 6 (header) + 16 (entry)
    entry = struct.pack("<BBBBHHII", 0, 0, 0, 0, 1, 32, len(png), 22)
    return header + entry + png


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    png = build_png(render_raw())
    with open(os.path.join(OUT_DIR, "icon.png"), "wb") as f:
        f.write(png)
    with open(os.path.join(OUT_DIR, "icon.ico"), "wb") as f:
        f.write(build_ico(png))
    print("wrote assets/icon.png and assets/icon.ico")


if __name__ == "__main__":
    main()
