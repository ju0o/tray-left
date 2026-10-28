#!/usr/bin/env python3
"""Build Jusage's original SVG and multi-size Windows ICO using Python's stdlib."""

from __future__ import annotations

import math
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SIZES = (16, 24, 32, 48, 64, 128, 256)
SCALE = 4
INK = (220, 227, 239)
SLATE = (17, 21, 34)
EDGE = (48, 61, 89)
TEAL = (98, 215, 200)

SVG = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256">
  <rect x="6" y="6" width="244" height="244" rx="54" fill="#303D59"/>
  <rect x="10" y="10" width="236" height="236" rx="50" fill="#111522"/>
  <path d="M76 80h98v68c0 22-16 38-38 38h-10c-21 0-37-16-37-37"
        fill="none" stroke="#DCE3EF" stroke-width="24"
        stroke-linecap="round" stroke-linejoin="round"/>
  <circle cx="89" cy="151" r="10" fill="#62D7C8"/>
</svg>
'''


def cubic(a: tuple[float, float], b: tuple[float, float],
          c: tuple[float, float], d: tuple[float, float], t: float) -> tuple[float, float]:
    u = 1.0 - t
    return (
        u**3 * a[0] + 3 * u**2 * t * b[0] + 3 * u * t**2 * c[0] + t**3 * d[0],
        u**3 * a[1] + 3 * u**2 * t * b[1] + 3 * u * t**2 * c[1] + t**3 * d[1],
    )


def put(canvas: bytearray, side: int, x: int, y: int, color: tuple[int, int, int]) -> None:
    start = (y * side + x) * 4
    canvas[start : start + 4] = bytes((*color, 255))


def rounded_rect(canvas: bytearray, side: int, box: tuple[float, float, float, float],
                 radius: float, color: tuple[int, int, int]) -> None:
    scale = side / 256
    x0, y0, x1, y1 = box
    left = max(0, math.floor(x0 * scale))
    top = max(0, math.floor(y0 * scale))
    right = min(side, math.ceil(x1 * scale))
    bottom = min(side, math.ceil(y1 * scale))
    for py in range(top, bottom):
        y = (py + 0.5) / scale
        cy = min(max(y, y0 + radius), y1 - radius)
        for px in range(left, right):
            x = (px + 0.5) / scale
            cx = min(max(x, x0 + radius), x1 - radius)
            if (x - cx) ** 2 + (y - cy) ** 2 <= radius**2:
                put(canvas, side, px, py, color)


def stroke(canvas: bytearray, side: int, a: tuple[float, float], b: tuple[float, float],
           width: float, color: tuple[int, int, int]) -> None:
    scale = side / 256
    radius = width / 2
    left = max(0, math.floor((min(a[0], b[0]) - radius) * scale))
    top = max(0, math.floor((min(a[1], b[1]) - radius) * scale))
    right = min(side, math.ceil((max(a[0], b[0]) + radius) * scale))
    bottom = min(side, math.ceil((max(a[1], b[1]) + radius) * scale))
    dx, dy = b[0] - a[0], b[1] - a[1]
    length2 = dx * dx + dy * dy
    for py in range(top, bottom):
        y = (py + 0.5) / scale
        for px in range(left, right):
            x = (px + 0.5) / scale
            t = 0.0 if length2 == 0 else min(1.0, max(0.0, ((x - a[0]) * dx + (y - a[1]) * dy) / length2))
            if (x - (a[0] + t * dx)) ** 2 + (y - (a[1] + t * dy)) ** 2 <= radius**2:
                put(canvas, side, px, py, color)


def disc(canvas: bytearray, side: int, center: tuple[float, float], radius: float,
         color: tuple[int, int, int]) -> None:
    stroke(canvas, side, center, center, radius * 2, color)


def downsample(canvas: bytearray, side: int, size: int) -> bytes:
    out = bytearray(size * size * 4)
    factor = side // size
    samples = factor * factor
    for y in range(size):
        for x in range(size):
            alpha = red = green = blue = 0
            for sy in range(y * factor, (y + 1) * factor):
                for sx in range(x * factor, (x + 1) * factor):
                    start = (sy * side + sx) * 4
                    r, g, b, a = canvas[start : start + 4]
                    alpha += a
                    red += r * a
                    green += g * a
                    blue += b * a
            at = (y * size + x) * 4
            out[at] = 0 if alpha == 0 else round(red / alpha)
            out[at + 1] = 0 if alpha == 0 else round(green / alpha)
            out[at + 2] = 0 if alpha == 0 else round(blue / alpha)
            out[at + 3] = round(alpha / samples)
    return bytes(out)


def render(size: int) -> bytes:
    side = size * SCALE
    canvas = bytearray(side * side * 4)
    rounded_rect(canvas, side, (6, 6, 250, 250), 54, EDGE)
    rounded_rect(canvas, side, (10, 10, 246, 246), 50, SLATE)

    points: list[tuple[float, float]] = [(76, 80), (174, 80), (174, 148)]
    curve_start = points[-1]
    for index in range(1, 33):
        points.append(cubic(curve_start, (174, 170), (158, 186), (136, 186), index / 32))
    points.append((126, 186))
    curve_start = points[-1]
    for index in range(1, 33):
        points.append(cubic(curve_start, (105, 186), (89, 170), (89, 149), index / 32))
    for start, end in zip(points, points[1:]):
        stroke(canvas, side, start, end, 24, INK)
    disc(canvas, side, (89, 151), 10, TEAL)
    return downsample(canvas, side, size)


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    body = kind + payload
    return struct.pack(">I", len(payload)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)


def png(size: int, rgba: bytes) -> bytes:
    rows = b"".join(b"\0" + rgba[y * size * 4 : (y + 1) * size * 4] for y in range(size))
    return (
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
        + png_chunk(b"IDAT", zlib.compress(rows, 9))
        + png_chunk(b"IEND", b"")
    )


def dib(size: int, rgba: bytes) -> bytes:
    xor = bytearray()
    mask_stride = ((size + 31) // 32) * 4
    mask = bytearray(mask_stride * size)
    for y in reversed(range(size)):
        for x in range(size):
            r, g, b, a = rgba[(y * size + x) * 4 : (y * size + x + 1) * 4]
            xor.extend((b, g, r, a))
            if a < 128:
                mask[(size - 1 - y) * mask_stride + x // 8] |= 1 << (7 - x % 8)
    header = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 32, 0, len(xor), 0, 0, 0, 0)
    return header + xor + mask


def ico(images: list[tuple[int, bytes]]) -> bytes:
    directory = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    offset = 6 + 16 * len(images)
    data = bytearray()
    for size, rgba in images:
        image = dib(size, rgba)
        directory.extend(struct.pack(
            "<BBBBHHII", 0 if size == 256 else size, 0 if size == 256 else size,
            0, 0, 1, 32, len(image), offset,
        ))
        data.extend(image)
        offset += len(image)
    return bytes(directory + data)


def validate_icon(blob: bytes, expected: tuple[int, ...]) -> None:
    reserved, kind, count = struct.unpack_from("<HHH", blob)
    if (reserved, kind, count) != (0, 1, len(expected)):
        raise ValueError("invalid ICO directory header")
    actual = []
    for index in range(count):
        width, height, _, _, planes, bits, length, offset = struct.unpack_from("<BBBBHHII", blob, 6 + 16 * index)
        size = 256 if width == 0 else width
        if height != (0 if size == 256 else size) or planes != 1 or bits != 32 or offset + length > len(blob):
            raise ValueError(f"invalid ICO entry at index {index}")
        header_size, dib_width, dib_height = struct.unpack_from("<Iii", blob, offset)
        if (header_size, dib_width, dib_height) != (40, size, size * 2):
            raise ValueError(f"invalid DIB dimensions at index {index}")
        mask_stride = ((size + 31) // 32) * 4
        expected_length = 40 + size * size * 4 + mask_stride * size
        if length != expected_length or offset < 6 + 16 * count:
            raise ValueError(f"invalid ICO image length or offset at index {index}")
        actual.append(size)
    if tuple(actual) != expected:
        raise ValueError(f"unexpected icon sizes: {actual}")


def main() -> None:
    assets = ROOT / "assets"
    images = [(size, render(size)) for size in SIZES]
    icon = ico(images)
    validate_icon(icon, SIZES)
    (assets / "jusage.svg").write_text(SVG, encoding="utf-8", newline="\n")
    (assets / "jusage.ico").write_bytes(icon)
    (assets / "jusage-icon-preview.png").write_bytes(png(256, render(256)))


if __name__ == "__main__":
    main()
