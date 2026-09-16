#!/usr/bin/env python3
"""Generate the Play Store feature graphic (1024x500).

Committed as a script, not just a PNG, because the brand palette lives in
skills/design.md and will change again — it already replaced the prairie palette
once. A regenerable asset tracks the identity; an exported PNG drifts from it.

Content stays centred: Play crops the feature graphic and overlays the app icon
on it in some placements, so anything near an edge is at risk.

Run from the repo root:  python3 app/store/play/make-feature-graphic.py
"""

from PIL import Image, ImageDraw, ImageFont, ImageFilter

W, H = 1024, 500
OUT = "app/store/play/feature-graphic.png"

# skills/design.md, "Visual identity" — midnight glass.
NAVY_TOP, NAVY_BOT = (2, 19, 41), (12, 37, 62)      # #021329 -> #0c253e
MINT_BRIGHT = (76, 216, 173)                         # #4cd8ad, the wordmark
MINT_PALE = (183, 243, 230)                          # #b7f3e6, glows
SEA_TEAL = (71, 158, 150)                            # #479e96, rules
WARM_WHITE = (238, 246, 242)                         # #eef6f2
SOFT_INK = (143, 168, 181)                           # #8fa8b5

WORDMARK = "VERDURE"
TAGLINE = "Your personal AI. On your device."
SUBLINE = "Runs offline  ·  Nothing leaves your phone"


def _font(path: str, size: int):
    try:
        return ImageFont.truetype(path, size)
    except OSError:
        # Never fail the asset over a missing system font; the layout still holds.
        return ImageFont.load_default()


def build() -> Image.Image:
    img = Image.new("RGB", (W, H), NAVY_TOP)
    px = img.load()
    for y in range(H):
        t = y / (H - 1)
        row = tuple(int(NAVY_TOP[i] + (NAVY_BOT[i] - NAVY_TOP[i]) * t) for i in range(3))
        for x in range(W):
            px[x, y] = row

    # Blurred, so it reads as light rather than as an ellipse with an edge.
    glow = Image.new("L", (W, H), 0)
    ImageDraw.Draw(glow).ellipse(
        (W // 2 - 300, H // 2 - 225, W // 2 + 300, H // 2 + 225), fill=46
    )
    glow = glow.filter(ImageFilter.GaussianBlur(110)).point(lambda v: min(v, 34))
    img = Image.composite(Image.new("RGB", (W, H), MINT_PALE), img, glow)

    d = ImageDraw.Draw(img)
    serif = _font("/System/Library/Fonts/NewYork.ttf", 96)
    sans = _font("/System/Library/Fonts/Supplemental/Georgia.ttf", 30)
    sub = _font("/System/Library/Fonts/Supplemental/Georgia.ttf", 22)

    # The wordmark is wide-tracked serif; PIL has no letter-spacing, so draw it
    # glyph by glyph.
    widths = [d.textlength(c, font=serif) for c in WORDMARK]
    tracking = 14
    total = sum(widths) + tracking * (len(WORDMARK) - 1)
    x = W / 2 - total / 2
    for ch, w in zip(WORDMARK, widths):
        d.text((x, 150), ch, font=serif, fill=MINT_BRIGHT)
        x += w + tracking

    d.line((W / 2 - total / 2, 272, W / 2 + total / 2, 272), fill=SEA_TEAL, width=2)
    for text, font, fill, y in ((TAGLINE, sans, WARM_WHITE, 300), (SUBLINE, sub, SOFT_INK, 350)):
        d.text(((W - d.textlength(text, font=font)) / 2, y), text, font=font, fill=fill)
    return img


if __name__ == "__main__":
    image = build()
    assert image.size == (W, H), f"Play requires exactly {W}x{H}, got {image.size}"
    image.save(OUT, "PNG", optimize=True)
    print(f"wrote {OUT} {image.size}")
