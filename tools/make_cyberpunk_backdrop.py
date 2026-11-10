"""Bake the Cyberpunk theme's blurred page backdrop.

The source is the neon-lit underground corridor screenshot (the ``ui`` reference
index 034) from the saved Game UI Database page.  We crop away most of the HUD,
blur the magenta/cyan scene and store a tiny raw RGBA texture that is compiled
into the executable (no runtime image decoder needed).

Usage:
    python tools/make_cyberpunk_backdrop.py

Output:
    assets/cyberpunk-backdrop.rgba   (192 x 108, RGBA8, ~83 KB)
"""

from __future__ import annotations

import base64
import io
import pathlib
import re

from PIL import Image, ImageEnhance, ImageFilter

ROOT = pathlib.Path(__file__).resolve().parent.parent
MHTML = ROOT / "references" / "cyberpunk" / "Game UI Database - Cyberpunk 2077.mhtml"
OUT = ROOT / "assets" / "cyberpunk-backdrop.rgba"
PREVIEW = ROOT / "shots" / "cyberpunk-backdrop-preview.png"

# Stable part name inside the saved Game UI Database page (ui reference 034).
SOURCE_HINT = "Cyberpunk-207712292020-014343-2349_thumb.jpg"


def mhtml_images(data: bytes) -> dict[str, bytes]:
    """Return {filename: decoded bytes} for every image part in the archive."""
    boundary = re.search(rb'boundary="([^"]+)"', data).group(1)
    images: dict[str, bytes] = {}
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
        name = loc.group(1).split("/")[-1].split("?")[0]
        try:
            images[name] = base64.b64decode(body.strip(), validate=False)
        except Exception:
            continue
    return images


def bake(source: Image.Image) -> Image.Image:
    # Crop inside the HUD frame; keep the pink doorway and the cyan arches.
    crop = source.convert("RGB").crop((34, 18, 466, 258))

    # Downscale first (this is the main blur), then soften the 8-bit steps.
    small = crop.resize((192, 108), Image.LANCZOS)
    small = small.filter(ImageFilter.GaussianBlur(4.0))
    small = ImageEnhance.Color(small).enhance(1.30)
    small = ImageEnhance.Contrast(small).enhance(1.12)
    small = ImageEnhance.Brightness(small).enhance(0.88)

    # A cool night-grade keeps the texture under the dark veil.
    r, g, b = small.split()
    r = r.point(lambda v: min(255, int(v * 0.92)))
    g = g.point(lambda v: int(v * 0.86))
    b = b.point(lambda v: min(255, int(v * 1.12)))
    small = Image.merge("RGB", (r, g, b))

    return small.filter(ImageFilter.GaussianBlur(1.1)).convert("RGBA")


def main() -> None:
    if not MHTML.exists():
        raise SystemExit(f"reference archive not found: {MHTML}")
    images = mhtml_images(MHTML.read_bytes())
    raw = images.get(SOURCE_HINT)
    if raw is None:
        raise SystemExit(f"source screenshot not found in the archive: {SOURCE_HINT}")

    baked = bake(Image.open(io.BytesIO(raw)))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(baked.tobytes())

    PREVIEW.parent.mkdir(parents=True, exist_ok=True)
    baked.resize((768, 432), Image.BICUBIC).save(PREVIEW)

    print(f"wrote {OUT.relative_to(ROOT)} ({OUT.stat().st_size:,} bytes)")
    print(f"preview {PREVIEW.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
