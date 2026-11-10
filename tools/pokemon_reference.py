"""Extract the Pokémon: Let's Go reference screenshots from the saved Game UI Database pages.

The ``references/pokemon`` folder contains three 9 MB MHTML archives of the same page.
This script reads the first one, writes the screenshots as numbered files and
builds contact sheets so the visual language can be inspected without loading
the archive into an agent's context.

Usage:
    python tools/pokemon_reference.py [--out shots/pokemon-reference]

Then open ``sheet-1.png`` … ``sheet-5.png``.  Screenshot 018 is the tall-grass
meadow used for the page backdrop; 031–039 cover the bag and menu panels.
"""

from __future__ import annotations

import argparse
import base64
import io
import pathlib
import re

from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCES = sorted((ROOT / "references" / "pokemon").glob("*.mhtml"))

TILE_W, TILE_H = 380, 214
COLS = 5
PER_SHEET = 25


def extract(data: bytes) -> list[tuple[int, bytes, tuple[int, int]]]:
    boundary = re.search(rb'boundary="([^"]+)"', data).group(1)
    found: list[tuple[int, bytes, tuple[int, int]]] = []
    for part in data.split(b"--" + boundary):
        head, sep, body = part.partition(b"\r\n\r\n")
        if not sep:
            head, sep, body = part.partition(b"\n\n")
        if not sep:
            continue
        headers = head.decode("utf-8", "replace")
        loc = re.search(r"Content-Location:\s*(\S+)", headers)
        if not loc or not re.search(r"\.(jpe?g|png|gif|webp)$", loc.group(1), re.I):
            continue
        try:
            raw = base64.b64decode(body.strip(), validate=False)
            image = Image.open(io.BytesIO(raw))
            found.append((len(found) + 1, raw, image.size))
        except Exception:
            continue
    return found


def contact_sheets(items: list[tuple[int, bytes, tuple[int, int]]], out: pathlib.Path) -> list[pathlib.Path]:
    sheets: list[pathlib.Path] = []
    for page in range((len(items) + PER_SHEET - 1) // PER_SHEET):
        chunk = items[page * PER_SHEET : (page + 1) * PER_SHEET]
        rows = (len(chunk) + COLS - 1) // COLS
        sheet = Image.new("RGB", (COLS * TILE_W, rows * (TILE_H + 22)), (24, 24, 26))
        draw = ImageDraw.Draw(sheet)
        for i, (idx, raw, size) in enumerate(chunk):
            image = Image.open(io.BytesIO(raw)).convert("RGB")
            image.thumbnail((TILE_W, TILE_H))
            x = (i % COLS) * TILE_W
            y = (i // COLS) * (TILE_H + 22)
            sheet.paste(image, (x + (TILE_W - image.width) // 2, y))
            draw.text((x + 4, y + TILE_H + 4), f"{idx:03d}  {size[0]}x{size[1]}", fill=(225, 225, 225))
        path = out / f"sheet-{page + 1}.png"
        sheet.save(path)
        sheets.append(path)
    return sheets


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="shots/pokemon-reference")
    args = parser.parse_args()

    if not SOURCES:
        raise SystemExit("no Pokémon MHTML files found in 'references/pokemon'")
    out = ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)

    items = extract(SOURCES[0].read_bytes())
    if not items:
        raise SystemExit("no screenshots found in the archive")

    for idx, raw, _ in items:
        (out / f"{idx:03d}.jpg").write_bytes(raw)
    sheets = contact_sheets(items, out)

    print(f"extracted {len(items)} images to {out}")
    for sheet in sheets:
        print(f"  {sheet.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
