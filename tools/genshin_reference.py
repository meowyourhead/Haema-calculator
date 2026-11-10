"""Extract the Genshin UI reference screenshots from the saved Game UI Database page.

The ``.mhtml`` archive is 10 MB of base64 images and must never be read raw into
an agent context.  This script writes the images plus numbered contact sheets to
a scratch folder so they can be inspected with an image viewer.

Usage:
    python tools/genshin_reference.py [--out shots/genshin-reference]

Then open the produced ``sheet-*.png`` files.  Each cell is labelled with the
source screenshot timestamp so a design note can cite the exact reference.
"""

from __future__ import annotations

import argparse
import base64
import pathlib
import re

from PIL import Image, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parent.parent
MHTML = ROOT / "references" / "genshin" / "Game UI Database - Genshin Impact (Console).mhtml"

TILE_W, TILE_H = 380, 214
COLS = 5
PER_SHEET = 25


def extract(data: bytes) -> list[tuple[str, bytes]]:
    boundary = re.search(rb'boundary="([^"]+)"', data).group(1)
    found: list[tuple[str, bytes]] = []
    for part in data.split(b"--" + boundary):
        head, sep, body = part.partition(b"\r\n\r\n")
        if not sep:
            head, sep, body = part.partition(b"\n\n")
        if not sep:
            continue
        headers = head.decode("utf-8", "replace")
        loc = re.search(r"Content-Location:\s*(\S+)", headers)
        if not loc:
            continue
        name = loc.group(1).split("/")[-1].split("?")[0]
        if not re.search(r"Genshin.*\.(jpe?g|png|webp)$", name, re.I):
            continue
        try:
            found.append((name, base64.b64decode(body.strip(), validate=False)))
        except Exception:
            continue
    return sorted(found)


def contact_sheets(items: list[tuple[str, bytes]], out: pathlib.Path) -> list[pathlib.Path]:
    sheets: list[pathlib.Path] = []
    for page in range((len(items) + PER_SHEET - 1) // PER_SHEET):
        chunk = items[page * PER_SHEET : (page + 1) * PER_SHEET]
        rows = (len(chunk) + COLS - 1) // COLS
        sheet = Image.new("RGB", (COLS * TILE_W, rows * (TILE_H + 22)), (24, 24, 26))
        draw = ImageDraw.Draw(sheet)
        for i, (name, raw) in enumerate(chunk):
            import io

            image = Image.open(io.BytesIO(raw)).convert("RGB")
            image.thumbnail((TILE_W, TILE_H))
            x = (i % COLS) * TILE_W
            y = (i // COLS) * (TILE_H + 22)
            sheet.paste(image, (x + (TILE_W - image.width) // 2, y))
            label = name.replace("Genshin-Impact01032021-", "").replace("_thumb", "").replace(".jpg", "")
            draw.text((x + 4, y + TILE_H + 4), label, fill=(220, 220, 220))
        path = out / f"sheet-{page + 1}.png"
        sheet.save(path)
        sheets.append(path)
    return sheets


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default="shots/genshin-reference")
    args = parser.parse_args()

    out = ROOT / args.out
    out.mkdir(parents=True, exist_ok=True)

    items = extract(MHTML.read_bytes())
    if not items:
        raise SystemExit("no Genshin screenshots found in the archive")

    for name, raw in items:
        (out / name).write_bytes(raw)
    sheets = contact_sheets(items, out)

    print(f"extracted {len(items)} screenshots to {out}")
    for sheet in sheets:
        print(f"  {sheet.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
