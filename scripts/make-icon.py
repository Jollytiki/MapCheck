#!/usr/bin/env python3
"""Draws MapCheck's app icon and packs it into an .icns.

The mark is a closed traverse — the thing the app exists to check — on the
cyan-to-emerald gradient the app's own logo uses, with a rose dot marking the
point of beginning. Written with the standard library only so the build needs
no image dependencies.

Usage: python3 scripts/make-icon.py [output.icns]
"""

import struct
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

# Straight from style.css.
CYAN = (0x06, 0xB6, 0xD4)
EMERALD = (0x10, 0xB9, 0x81)
INK = (0x0B, 0x0F, 0x19)
ROSE = (0xF4, 0x3F, 0x5E)

MASTER = 1024
SS = 2  # supersampling factor, for antialiased edges


def write_png(path: Path, size: int, pixels: bytearray) -> None:
    """Write RGBA pixels as a PNG."""
    raw = bytearray()
    stride = size * 4
    for y in range(size):
        raw.append(0)  # no per-row filter
        raw.extend(pixels[y * stride : (y + 1) * stride])

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def rounded_rect_rows(size: int, radius: float):
    """For each row, the x range covered by a rounded rectangle."""
    for y in range(size):
        if y < radius:
            dy = radius - y - 0.5
        elif y > size - radius:
            dy = y + 0.5 - (size - radius)
        else:
            dy = 0.0
        inset = radius - (radius * radius - dy * dy) ** 0.5 if dy > 0 else 0.0
        yield y, inset, size - inset


def draw() -> bytearray:
    """Render the icon at MASTER*SS, then box-filter down to MASTER."""
    big = MASTER * SS
    buf = bytearray(big * big * 4)

    # The traverse: a closed figure inset in the tile, in supersampled units.
    left, right = 0.28 * big, 0.72 * big
    top, bottom = 0.26 * big, 0.70 * big
    stroke = 0.042 * big
    half = stroke / 2

    # Each edge as an axis-aligned band, so the fill test stays cheap.
    edges = [
        (left - half, right + half, top - half, top + half),
        (left - half, right + half, bottom - half, bottom + half),
        (left - half, left + half, top - half, bottom + half),
        (right - half, right + half, top - half, bottom + half),
    ]
    # Point of beginning, bottom-left corner.
    pob = (left, bottom, 0.055 * big)

    radius = 0.225 * big
    for y, x0, x1 in rounded_rect_rows(big, radius):
        row = y * big * 4
        fy = y / big
        in_band = [e for e in edges if e[2] <= y <= e[3]]
        py = y + 0.5
        for x in range(int(x0), int(x1) + 1):
            # Diagonal gradient, matching the logo's 135deg.
            t = (x / big + fy) / 2
            r = int(CYAN[0] + (EMERALD[0] - CYAN[0]) * t)
            g = int(CYAN[1] + (EMERALD[1] - CYAN[1]) * t)
            b = int(CYAN[2] + (EMERALD[2] - CYAN[2]) * t)

            px = x + 0.5
            if (px - pob[0]) ** 2 + (py - pob[1]) ** 2 <= pob[2] ** 2:
                r, g, b = ROSE
            elif any(ex0 <= px <= ex1 for ex0, ex1, _, _ in in_band):
                r, g, b = INK

            at = row + x * 4
            buf[at] = r
            buf[at + 1] = g
            buf[at + 2] = b
            buf[at + 3] = 255

    # Box-filter down, which also antialiases the rounded corners.
    out = bytearray(MASTER * MASTER * 4)
    for y in range(MASTER):
        for x in range(MASTER):
            acc = [0, 0, 0, 0]
            for dy in range(SS):
                base = ((y * SS + dy) * big + x * SS) * 4
                for dx in range(SS):
                    at = base + dx * 4
                    acc[0] += buf[at]
                    acc[1] += buf[at + 1]
                    acc[2] += buf[at + 2]
                    acc[3] += buf[at + 3]
            at = (y * MASTER + x) * 4
            n = SS * SS
            out[at] = acc[0] // n
            out[at + 1] = acc[1] // n
            out[at + 2] = acc[2] // n
            out[at + 3] = acc[3] // n
    return out


def main() -> int:
    target = Path(sys.argv[1] if len(sys.argv) > 1 else "AppIcon.icns")

    print("Drawing the icon...")
    master = draw()

    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        source = Path(tmp) / "master.png"
        write_png(source, MASTER, master)

        # The sizes macOS asks for, each as its own file in the iconset.
        for size, name in [
            (16, "icon_16x16.png"),
            (32, "icon_16x16@2x.png"),
            (32, "icon_32x32.png"),
            (64, "icon_32x32@2x.png"),
            (128, "icon_128x128.png"),
            (256, "icon_128x128@2x.png"),
            (256, "icon_256x256.png"),
            (512, "icon_256x256@2x.png"),
            (512, "icon_512x512.png"),
            (1024, "icon_512x512@2x.png"),
        ]:
            subprocess.run(
                ["sips", "-z", str(size), str(size), str(source),
                 "--out", str(iconset / name)],
                check=True, capture_output=True,
            )

        target.parent.mkdir(parents=True, exist_ok=True)
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(target)],
            check=True,
        )

    print(f"Wrote {target} ({target.stat().st_size:,} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
