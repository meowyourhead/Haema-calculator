"""Generate the application icon from assets/icon/fox.jpg.

The source JPEG is a rounded-square tile on a white background with a soft drop shadow.
This script:
  * cuts the tile out along its dark-red outline, so no white or grey fringe survives
    (anti-aliased edge, transparent outside);
  * adds a subtle diagonal gradient to the red background only (lighter, warmer top-left
    to deeper bottom-right), leaving the fox untouched;
  * writes assets/icon/fox.ico (multi-size, exe resource) and assets/icon/fox-128.rgba
    (raw 128x128 RGBA, window / taskbar icon), plus assets/icon/fox.png for previewing.

Run:  python -I tools/make_icon.py
"""

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
ICON_DIR = ROOT / "assets" / "icon"

# How far (px, at 1024) to pull the edge inside the outline's outer boundary.
INSET = 3


def tile_mask(rgb: Image.Image) -> Image.Image:
    """Anti-aliased 'L' mask of the rounded tile."""
    a = np.asarray(rgb).astype(np.int16)
    # Tile pixels: strongly red (outline + background + fox). Everything outside is
    # white/grey (background and drop shadow) or the pale pink rim highlight.
    reddish = (a[..., 0] - a[..., 1]) > 60
    seed = Image.fromarray(np.where(reddish, 255, 0).astype(np.uint8), "L").copy()
    # Flood the outside from a corner; whatever is not reached is inside the tile.
    ImageDraw.floodfill(seed, (0, 0), 128)
    inside = np.asarray(seed) != 128
    m = Image.fromarray(np.where(inside, 255, 0).astype(np.uint8), "L").copy()
    # Keep only the tile itself: the connected region containing the centre. This drops
    # stray pockets in the shadow that the outer flood could not reach.
    ImageDraw.floodfill(m, (m.width // 2, m.height // 2), 128)
    m = Image.fromarray(np.where(np.asarray(m) == 128, 255, 0).astype(np.uint8), "L").copy()
    # Erode past the pale rim, then smooth the JPEG-noisy contour into a clean edge.
    m = m.filter(ImageFilter.MinFilter(2 * INSET + 1))
    m = m.filter(ImageFilter.GaussianBlur(4))
    # Sharpen the blurred step back into a ~1.5px anti-aliased ramp around the 50% line.
    f = np.asarray(m).astype(np.float32) / 255.0
    f = np.clip((f - 0.5) * 3.0 + 0.5, 0.0, 1.0)
    return Image.fromarray((f * 255).round().astype(np.uint8), "L").copy()


def background_gradient(rgb: Image.Image) -> Image.Image:
    a = np.asarray(rgb).astype(np.float32)
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    # Soft weight for "this is the flat red background": saturated mid red, little green.
    w = (
        np.clip((45 - g) / 25, 0, 1)
        * np.clip((50 - b) / 25, 0, 1)
        * np.clip((r - 95) / 35, 0, 1)
    )
    w = np.asarray(
        Image.fromarray((w * 255).astype(np.uint8), "L").copy().filter(ImageFilter.GaussianBlur(1.2))
    ).astype(np.float32) / 255.0

    h, wd = r.shape
    yy, xx = np.mgrid[0:h, 0:wd].astype(np.float32)
    # Diagonal position 0 (top-left) .. 1 (bottom-right).
    t = np.clip((xx / wd * 0.45 + yy / h * 0.55), 0, 1)
    t = t * t * (3 - 2 * t)  # smoothstep for a gentle falloff
    # Brightness: +24% at top-left down to -20% at bottom-right.
    gain = 1.24 + (0.80 - 1.24) * t
    # A touch of warmth (towards scarlet) in the lit corner.
    warm = (1 - t) ** 2
    out = a.copy()
    out[..., 0] = r * gain + 6 * warm
    out[..., 1] = g * gain + 10 * warm
    out[..., 2] = b * gain + 4 * warm
    blended = a + (out - a) * w[..., None]
    return Image.fromarray(np.clip(blended, 0, 255).round().astype(np.uint8), "RGB")


def resize_rgba(img: Image.Image, size: int) -> Image.Image:
    # Resample in premultiplied space so transparent pixels never bleed colour into edges.
    return img.convert("RGBa").resize((size, size), Image.LANCZOS).convert("RGBA")


def main() -> None:
    src = Image.open(ICON_DIR / "fox.jpg").convert("RGB")
    mask = tile_mask(src)
    rgb = background_gradient(src)

    # Fill transparent pixels with the edge red so non-premultiplied consumers don't fringe.
    edge_red = Image.new("RGB", src.size, (120, 8, 10))
    solid = np.asarray(mask) > 0
    rgb = Image.composite(rgb, edge_red, Image.fromarray(np.where(solid, 255, 0).astype(np.uint8)))
    rgba = rgb.copy()
    rgba.putalpha(mask)

    # Trim to the tile and re-centre on a square canvas with a small margin.
    bbox = mask.getbbox()
    tile = rgba.crop(bbox)
    side = max(tile.size)
    margin = round(side * 0.02)
    canvas = Image.new("RGBA", (side + 2 * margin,) * 2, (120, 8, 10, 0))
    canvas.paste(tile, (margin + (side - tile.width) // 2, margin + (side - tile.height) // 2))

    resize_rgba(canvas, 512).save(ICON_DIR / "fox.png")
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    big = resize_rgba(canvas, 256)
    big.save(ICON_DIR / "fox.ico", sizes=[(s, s) for s in sizes],
             append_images=[resize_rgba(canvas, s) for s in sizes if s != 256])
    (ICON_DIR / "fox-128.rgba").write_bytes(resize_rgba(canvas, 128).tobytes())
    print("bbox", bbox, "->", canvas.size)


if __name__ == "__main__":
    main()
