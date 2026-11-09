"""Extract the base64-embedded fonts from the standalone HTML and convert them to TTF
so they can be embedded in the native Rust build."""
import base64
import io
import os
import re
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
HTML = os.path.join(ROOT, "Haematology Calculator.html")
OUT = os.path.join(ROOT, "assets", "fonts")
RAW = os.path.join(OUT, "raw")
os.makedirs(RAW, exist_ok=True)

src = open(HTML, encoding="utf-8").read()
blocks = re.findall(r"@font-face\s*\{(.*?)\}", src, re.S)
print(f"found {len(blocks)} font-face blocks")

for i, block in enumerate(blocks):
    fam = re.search(r"font-family:\s*'([^']+)'", block)
    style = re.search(r"font-style:\s*([^;]+);", block)
    weight = re.search(r"font-weight:\s*([^;]+);", block)
    data = re.search(r"url\(data:font/(woff2?|ttf|otf);base64,([A-Za-z0-9+/=]+)\)", block)
    if not (fam and data):
        print(f"  [{i}] skipped (no match)")
        continue
    ext = data.group(1)
    raw = base64.b64decode(data.group(2))
    name = f"{fam.group(1).replace(' ', '')}-{style.group(1) if style else 'normal'}-{weight.group(1).replace(' ', '_') if weight else '400'}"
    raw_path = os.path.join(RAW, f"{name}.{ext}")
    open(raw_path, "wb").write(raw)
    font = TTFont(io.BytesIO(raw))
    # Which variable axes does it expose?
    axes = []
    if "fvar" in font:
        axes = [(a.axisTag, a.minValue, a.defaultValue, a.maxValue) for a in font["fvar"].axes]
    print(f"  [{i}] {fam.group(1)} style={style.group(1) if style else '?'} weight={weight.group(1) if weight else '?'} ext={ext} bytes={len(raw)} axes={axes}")

    # Choose instances to emit. Static fonts are emitted as-is.
    is_italic = style is not None and "italic" in style.group(1)
    suffix = "-italic" if is_italic else ""
    if axes:
        # clamp the weight axis into the available range and emit useful static weights
        wanted = [400, 500, 600, 700, 800]
        if fam.group(1) in ("Poppins",):
            wanted = [500, 600, 700]
        elif fam.group(1) == "Lora":
            wanted = [400, 700]
        elif fam.group(1) == "Lato":
            wanted = [400, 700]
        for w in wanted:
            try:
                inst = instantiateVariableFont(font, {"wght": w}, updateFontNames=True)
                inst.flavor = None  # save as a plain TTF/OTF, not WOFF
                out_path = os.path.join(OUT, f"{fam.group(1).replace(' ', '')}-{w}{suffix}.ttf")
                inst.save(out_path)
                print(f"      -> {os.path.basename(out_path)}")
            except Exception as e:  # noqa: BLE001
                print(f"      !! could not instantiate {w}: {e}")
    else:
        out_path = os.path.join(OUT, f"{fam.group(1).replace(' ', '')}-{weight.group(1).replace(' ', '_') if weight else '400'}{suffix}.ttf")
        font.flavor = None  # save as a plain TTF/OTF, not WOFF
        font.save(out_path)
        print(f"      -> {os.path.basename(out_path)}")
