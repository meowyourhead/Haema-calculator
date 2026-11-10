"""Bake the Pokémon theme's blurred page backdrop.

The source is the tall-grass route screenshot (the 18th image part in the saved
Game UI Database page for Let's Go, Pikachu/Eevee).  We crop away the dialogue
box, paint over the "It's tiny!" caption, blur the meadow and store a tiny raw
RGBA texture that is compiled into the executable.

Usage:
    python tools/make_pokemon_backdrop.py

Output:
    assets/pokemon-backdrop.rgba   (192 x 108, RGBA8, ~83 KB)
"""

from __future__ import annotations

import base64
import io
import pathlib
import re

from PIL import Image, ImageEnhance, ImageFilter

ROOT = pathlib.Path(__file__).resolve().parent.parent
MHTML = sorted((ROOT / "references" / "pokemon").glob("*.mhtml"))[0]
OUT = ROOT / "assets" / "pokemon-backdrop.rgba"
PREVIEW = ROOT / "shots" / "pokemon-backdrop-preview.png"

SOURCE_INDEX = 18  # 1-based position of the tall-grass encounter screenshot


def image_parts(data: bytes) -> list[bytes]:
    boundary = re.search(rb'boundary="([^"]+)"', data).group(1)
    parts: list[bytes] = []
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
            Image.open(io.BytesIO(raw)).verify()
        except Exception:
            continue
        parts.append(raw)
    return parts


def bake(source: Image.Image) -> Image.Image:
    # Remove the pale dialogue box at the bottom and keep the sunny meadow.
    crop = source.convert("RGB").crop((0, 0, 500, 230))

    # The "It's tiny!" caption sits over the grass; replace it with a nearby
    # patch.  The blur makes the seam invisible.
    patch = crop.crop((340, 55, 475, 115)).resize((135, 60), Image.LANCZOS)
    crop.paste(patch, (185, 55))

    small = crop.resize((192, 108), Image.LANCZOS)
    small = small.filter(ImageFilter.GaussianBlur(4.2))
    small = ImageEnhance.Color(small).enhance(1.08)
    small = ImageEnhance.Brightness(small).enhance(1.06)
    small = small.filter(ImageFilter.GaussianBlur(1.0))
    return small.convert("RGBA")


def main() -> None:
    if not MHTML.exists():
        raise SystemExit(f"reference archive not found: {MHTML}")
    parts = image_parts(MHTML.read_bytes())
    if len(parts) < SOURCE_INDEX:
        raise SystemExit(f"expected at least {SOURCE_INDEX} image parts, found {len(parts)}")

    source = Image.open(io.BytesIO(parts[SOURCE_INDEX - 1]))
    if source.size != (500, 281):
        raise SystemExit(f"unexpected source size {source.size}; reference order may have changed")

    baked = bake(source)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(baked.tobytes())

    PREVIEW.parent.mkdir(parents=True, exist_ok=True)
    baked.resize((768, 432), Image.BICUBIC).save(PREVIEW)

    print(f"wrote {OUT.relative_to(ROOT)} ({OUT.stat().st_size:,} bytes)")
    print(f"preview {PREVIEW.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
