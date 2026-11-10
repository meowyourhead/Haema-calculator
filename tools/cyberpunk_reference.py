"""Extract the Cyberpunk 2077 reference screenshots from the saved Game UI Database pages.

The ``references/cyberpunk`` folder holds two 7–8 MB MHTML archives (the general UI
page and the HUD page).  This script writes their screenshots as numbered files plus
contact sheets so the visual language can be inspected without loading the archives
into an agent's context.

Usage:
    python tools/cyberpunk_reference.py [--out shots/cyberpunk-reference]

Then open ``page1-sheet-1.png`` … etc.  Each cell is labelled with its source page and
the screenshot's index so a design note can cite the exact reference.
"""

from __future__ import annotations

import argparse
import base64
import io
import pathlib
import re

from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCES = sorted((ROOT / "references" / "cyberpunk").glob("*.mhtml"))

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


def contact_sheets(items: list[tuple[int, bytes, tuple[int, int]]], out: pathlib.Path, tag: str) -> list[pathlib.Path]:
    sheets: list[pathlib.Path] = []
    for page in range((len(items) + PER_SHEET - 1) // PER_SHEET):
        chunk = items[page * PER_SHEET : (page + 1) * PER_SHEET]
        rows = (len(chunk) + COLS - 1) // COLS
        sheet = Image.new("RGB", (COLS * TILE_W, rows * (TILE_H + 22)), (16, 8, 24))
        draw = ImageDraw.Draw(sheet)
        for i, (idx, raw, size) in enumerate(chunk):
            image = Image.open(io.BytesIO(raw)).convert("RGB")
            image.thumbnail((TILE_W, TILE_H))
            x = (i % COLS) * TILE_W
            y = (i // COLS) * (TILE_H + 22)
            sheet.paste(image, (x + (TILE_W - image.width) // 2, y))
            draw.text((x + 4, y + TILE_H + 4), f"{tag} {idx:03d}  {size[0]}x{size[1]}", fill=(235, 225, 255))
        path = out / f"{tag}-sheet-{page + 1}.png"
        sheet.save(path)
        sheets.append(path)
    return sheets


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="shots/cyberpunk-reference")
    args = parser.parse_args()

    if not SOURCES:
        raise SystemExit("no Cyberpunk MHTML files found in 'references/cyberpunk'")
    out = ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)

    for page, source in enumerate(SOURCES, start=1):
        items = extract(source.read_bytes())
        if not items:
            raise SystemExit(f"no screenshots found in {source.name}")
        tag = "hud" if "hud" in source.name.lower() else "ui"
        for idx, raw, _ in items:
            (out / f"{tag}-{idx:03d}.jpg").write_bytes(raw)
        sheets = contact_sheets(items, out, tag)
        print(f"{source.name}: extracted {len(items)} screenshots")
        for sheet in sheets:
            print(f"  {sheet.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
