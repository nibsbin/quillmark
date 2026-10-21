"""Writes the Typst backend's fallback faces: Google Fonts' variable Roboto,
upright and italic, keeping the weight axis and pinning width to 100, and Noto
Sans Math cut to the blocks an equation commonly draws from. The hinting goes,
which PDF output and Typst's rasterizer never execute.

    uv run --with fonttools scripts/fallback-fonts.py
"""

import io
import json
import pathlib
import urllib.request

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

FACES = {
    "Roboto.ttf": "Roboto-VariableFont_wdth,wght.ttf",
    "Roboto-Italic.ttf": "Roboto-Italic-VariableFont_wdth,wght.ttf",
}
OUT = pathlib.Path(__file__).resolve().parent.parent / "crates/backends/typst/src/fonts"


def fetch(url):
    with urllib.request.urlopen(url) as response:
        return response.read()


def load(data):
    return TTFont(io.BytesIO(data), lazy=False, recalcTimestamp=False)


def save(font):
    out = io.BytesIO()
    font.save(out)
    return out.getvalue()


def options():
    return subset.Options(
        hinting=False,
        layout_features=["*"],
        name_IDs=["*"],
        name_languages=["*"],
        notdef_outline=True,
        legacy_kern=True,
    )


def trim(data):
    # Reloaded between steps: subsetting the instancer's own font object fails.
    font = load(save(instancer.instantiateVariableFont(load(data), {"wdth": 100})))
    subsetter = subset.Subsetter(options())
    subsetter.populate(glyphs=font.getGlyphOrder())
    subsetter.subset(font)
    return save(font)


text = fetch("https://fonts.google.com/download/list?family=Roboto").decode()
manifest = json.loads(text[text.index("{") :])["manifest"]

for file in manifest["files"]:
    if file["filename"] == "OFL.txt":
        (OUT / "OFL.txt").write_text(file["contents"].replace("\r\n", "\n"))

urls = {ref["filename"]: ref["url"] for ref in manifest["fileRefs"]}
for name in OUT.glob("*.ttf"):
    name.unlink()
for name, source in FACES.items():
    (OUT / name).write_bytes(trim(fetch(urls[source])))

# Latin, combining marks, Greek, punctuation, letterlike symbols, arrows,
# operators, the delimiters and big operators past them, and the math
# alphanumerics.
MATH_BLOCKS = [
    (0x20, 0x7E), (0xA0, 0xFF), (0x131, 0x131), (0x237, 0x237), (0x300, 0x36F),
    (0x391, 0x3C9), (0x3D0, 0x3F6), (0x2000, 0x206F), (0x20D0, 0x20FF),
    (0x2100, 0x214F), (0x2190, 0x21FF), (0x2200, 0x22FF), (0x2308, 0x230B),
    (0x231C, 0x2323), (0x23B4, 0x23B5), (0x23DC, 0x23E1), (0x25A1, 0x25A1),
    (0x25B3, 0x25B3), (0x25B7, 0x25B7), (0x25BD, 0x25BD), (0x25C1, 0x25C1),
    (0x25EF, 0x25EF), (0x2660, 0x266F), (0x2713, 0x2713), (0x27C2, 0x27C2),
    (0x27E6, 0x27EF), (0x27F5, 0x27FC), (0x2A00, 0x2A06), (0x2A0C, 0x2A11),
    (0x2A3F, 0x2A3F), (0x2A7D, 0x2A7E), (0x2AAF, 0x2AB0), (0x1D400, 0x1D7FF),
]
NOTO = "https://raw.githubusercontent.com/notofonts"
math = load(fetch(f"{NOTO}/notofonts.github.io/main/fonts/NotoSansMath/unhinted/otf/NotoSansMath-Regular.otf"))
subsetter = subset.Subsetter(options())
subsetter.populate(
    unicodes=[u for u in math.getBestCmap() if any(lo <= u <= hi for lo, hi in MATH_BLOCKS)]
)
subsetter.subset(math)
(OUT / "NotoSansMath-Regular.otf").write_bytes(save(math))
(OUT / "NotoSansMath-OFL.txt").write_text(fetch(f"{NOTO}/math/main/OFL.txt").decode().replace("\r\n", "\n"))
