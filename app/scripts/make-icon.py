from PIL import Image, ImageDraw, ImageFilter
import math

S = 1024
# Brand palette (skills/design.md — "midnight glass")
NAVY_EDGE   = (5, 15, 30)      # #050f1e
NAVY_MID    = (12, 37, 62)     # #0c253e
MINT        = (52, 198, 154)   # #34c69a
MINT_BRIGHT = (76, 216, 173)   # #4cd8ad
MINT_PALE   = (183, 243, 230)  # #b7f3e6

def bg():
    """Deep navy with an off-centre aurora lift — the app's own background, squared."""
    img = Image.new("RGB", (S, S), NAVY_EDGE)
    px = img.load()
    # radial: brighter up-left of centre, falling to the deep navy at the corners
    cx, cy, r = S * 0.40, S * 0.34, S * 0.95
    for y in range(S):
        for x in range(S):
            d = min(1.0, math.hypot(x - cx, y - cy) / r)
            t = d * d                      # ease so the centre stays open
            px[x, y] = (
                int(NAVY_MID[0] + (NAVY_EDGE[0] - NAVY_MID[0]) * t),
                int(NAVY_MID[1] + (NAVY_EDGE[1] - NAVY_MID[1]) * t),
                int(NAVY_MID[2] + (NAVY_EDGE[2] - NAVY_MID[2]) * t),
            )
    return img

def v_mask(width, inset_top, inset_bot, half):
    """The V itself, as a mask: one polyline, curved joint at the vertex."""
    m = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(m)
    cx = S // 2
    d.line(
        [(cx - half, inset_top), (cx, inset_bot), (cx + half, inset_top)],
        fill=255, width=width, joint="curve",
    )
    return m

def mint_gradient():
    """Bright at the top, settling into the brand mint — echoes the wordmark."""
    g = Image.new("RGB", (S, S))
    px = g.load()
    for y in range(S):
        t = y / (S - 1)
        px_row = (
            int(MINT_BRIGHT[0] + (MINT[0] - MINT_BRIGHT[0]) * t),
            int(MINT_BRIGHT[1] + (MINT[1] - MINT_BRIGHT[1]) * t),
            int(MINT_BRIGHT[2] + (MINT[2] - MINT_BRIGHT[2]) * t),
        )
        for x in range(S):
            px[x, y] = px_row
    return g

# Geometry: generous padding so platform masks (iOS squircle, Android circle)
# never clip the strokes.
STROKE   = 132
HALF     = 208
TOP      = 322
BOTTOM   = 726

img  = bg()
mask = v_mask(STROKE, TOP, BOTTOM, HALF)

# Glow: the mark sits in light, like the panels in the app.
glow = Image.new("RGB", (S, S), (0, 0, 0))
glow.paste(Image.new("RGB", (S, S), MINT), (0, 0), mask)
glow = glow.filter(ImageFilter.GaussianBlur(46))
img = Image.blend(img, Image.blend(img, glow, 0.55), 0.42)

# The V, filled with the mint gradient.
img.paste(mint_gradient(), (0, 0), mask)

# A soft top highlight on the mark, so it reads as lit rather than flat.
hi = Image.new("L", (S, S), 0)
ImageDraw.Draw(hi).rectangle([0, 0, S, TOP + 150], fill=70)
hi = hi.filter(ImageFilter.GaussianBlur(60))
hi = Image.composite(hi, Image.new("L", (S, S), 0), mask)
img.paste(Image.new("RGB", (S, S), MINT_PALE), (0, 0), hi)

img.save("source-square.png")

# Rounded variant for macOS, which shows the tile unmasked (~18% radius, Apple-ish).
rounded = Image.new("RGBA", (S, S), (0, 0, 0, 0))
corner = Image.new("L", (S, S), 0)
ImageDraw.Draw(corner).rounded_rectangle([0, 0, S - 1, S - 1], radius=int(S * 0.185), fill=255)
rounded.paste(img, (0, 0), corner)
rounded.save("source-rounded.png")
print("wrote source-square.png + source-rounded.png")
