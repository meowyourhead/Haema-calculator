"""Bake the Genshin theme's blurred page backdrop.

The source is the ``Inventory / No items`` screenshot in
``references/genshin/Game UI Database - Genshin Impact (Console).mhtml``: a deliberately blurred
in-game landscape with a cream/navy UI veil.  We crop away the HUD, paint over
the "No items" caption, blur the remainder and store a tiny raw RGBA texture
that is compiled into the executable (no runtime image decoder needed).

Usage:
    python tools/make_genshin_backdrop.py

Output:
    assets/genshin-backdrop.rgba   (192 x 128, RGBA8, ~96 KB)
"""

from __future__ import annotations

import base64
import pathlib
import re

from PIL import Image, ImageEnhance, ImageFilter

ROOT = pathlib.Path(__file__).resolve().parent.parent
MHTML = ROOT / "references" / "genshin" / "Game UI Database - Genshin Impact (Console).mhtml"
OUT = ROOT / "assets" / "genshin-backdrop.rgba"
PREVIEW = ROOT / "shots" / "genshin-backdrop-preview.png"

# Source part inside the saved Game UI Database page.
SOURCE_HINT = "Genshin-Impact01032021-021304-82132_thumb.jpg"


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
    # Drop the top navigation and the bottom currency row.
    crop = source.convert("RGB").crop((0, 46, 500, 238))

    # The only text inside the crop is "No items"; replace it with a nearby
    # patch of the same grass so it disappears into the blur.
    patch = crop.crop((330, 55, 490, 145)).resize((180, 90), Image.LANCZOS)
    crop.paste(patch, (160, 55))

    # Downscale first (this is the main blur), then soften the 8-bit steps.
    small = crop.resize((192, 128), Image.LANCZOS)
    small = small.filter(ImageFilter.GaussianBlur(4))
    small = ImageEnhance.Color(small).enhance(1.08)

    # One last light pass so the texture is smooth when stretched full-screen.
    return small.filter(ImageFilter.GaussianBlur(1.2)).convert("RGBA")


def main() -> None:
    if not MHTML.exists():
        raise SystemExit(f"reference archive not found: {MHTML}")
    images = mhtml_images(MHTML.read_bytes())
    raw = images.get(SOURCE_HINT)
    if raw is None:
        raise SystemExit(f"source screenshot not found in the archive: {SOURCE_HINT}")

    import io

    baked = bake(Image.open(io.BytesIO(raw)))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(baked.tobytes())

    PREVIEW.parent.mkdir(parents=True, exist_ok=True)
    baked.resize((768, 512), Image.BICUBIC).save(PREVIEW)

    print(f"wrote {OUT.relative_to(ROOT)} ({OUT.stat().st_size:,} bytes)")
    print(f"preview {PREVIEW.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
