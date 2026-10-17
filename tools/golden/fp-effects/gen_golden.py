"""Golden data for fp-effects from the Python references (re\\effekte\\fx.py, bevel.py, psfx.py).

Writes input/output pairs as PNG plus manifest.json under crates\\fp-effects\\tests\\data\\golden\\.
The Rust test tests/golden.rs reads the manifest, applies the effect list (EffectMoaID + parameter strings
as in the file) with fp_effects::apply_chain and compares pixel-exact (both transparent = equal).

Usage:
  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I gen_golden.py
"""
import json
import os
import sys

import numpy as np

RE = r"C:\Users\Administrator\Desktop\REA\re"
for sub in ("effekte", "format"):
    sys.path.insert(0, os.path.join(RE, sub))

import fx  # noqa: E402
import bevel  # noqa: E402
import psfx  # noqa: E402
from fwfmt import pngio  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "..", "..", "crates", "fp-effects", "tests", "data", "golden"))
os.makedirs(OUT, exist_ok=True)
for f in os.listdir(OUT):
    if f.endswith(".png") or f == "manifest.json":
        os.remove(os.path.join(OUT, f))

DROP = "{a7944db8-6ce2-11d1-8c76000502701850}"
INNER = "{5600f702-774c-11d3-baad0000861f4d01}"
BEVEL = "{7fe61102-6ce2-11d1-8c76000502701850}"
IGLOW = "{2ba87123-8220-11d3-baad0000861f4d01}"
SOLID = "{b90c950e-64df-11d8-aaf2000a9582f7d4}"
FILL = "{dd54adc0-a279-11d3-b92a000502f3fdbe}"
PS = "{8eeadf50-9efe-11da-a7460800200c9a66}"

manifest = []
_inputs = {}
rng = np.random.default_rng(20261011)


def save_png(name, arr):
    arr = np.ascontiguousarray(np.clip(arr, 0, 255).astype(np.uint8))
    h, w = arr.shape[:2]
    fn = name + ".png"
    with open(os.path.join(OUT, fn), "wb") as f:
        f.write(pngio.encode_rgba_png(w, h, arr.tobytes()))
    return fn


# ---------------------------------------------------------------- Inputs (x, y, rgba)
def rect(w, h, color, a=255):
    img = np.zeros((h, w, 4), np.int64)
    img[..., :3] = color
    img[..., 3] = a
    return img


def disc(w, h, color, r=None):
    y, x = np.mgrid[0:h, 0:w]
    cx, cy = w / 2.0, h / 2.0
    r = r or min(w, h) / 2.0 - 1
    d = np.sqrt((x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2)
    img = np.zeros((h, w, 4), np.int64)
    img[..., :3] = color
    img[..., 3] = (np.clip(r + 0.5 - d, 0, 1) * 255).astype(np.int64)
    return img


def ring(w, h, color):
    a = disc(w, h, color)
    b = disc(w, h, color, r=min(w, h) / 4.0)
    a[..., 3] = np.maximum(a[..., 3] - b[..., 3], 0)
    return a


def glyph():
    """Text-like shape: 'T' made of thin bars with soft edges, two colors."""
    img = np.zeros((14, 12, 4), np.int64)
    img[1:3, 1:11, :3] = (20, 40, 200)
    img[1:3, 1:11, 3] = 255
    img[3:13, 5:7, :3] = (200, 30, 30)
    img[3:13, 5:7, 3] = 255
    img[3:13, 4, :3] = (200, 30, 30)
    img[3:13, 4, 3] = 90
    img[3:13, 7, :3] = (200, 30, 30)
    img[3:13, 7, 3] = 160
    img[0, 1:11, :3] = (20, 40, 200)
    img[0, 1:11, 3] = 60
    return img


def noisy(w, h):
    img = rng.integers(0, 256, (h, w, 4), dtype=np.int64)
    m = disc(w, h, (0, 0, 0))[..., 3]
    img[..., 3] = (img[..., 3] * m) // 255
    return img


INPUTS = {
    "rect": (40, 40, rect(20, 20, (255, 0, 0))),
    "rect_a128": (40, 40, rect(20, 20, (255, 0, 0), 128)),
    "gray": (10, 10, rect(40, 30, (128, 128, 128))),
    "disc": (-5, 3, disc(24, 24, (30, 160, 60))),
    "ring": (7, -9, ring(30, 30, (240, 200, 10))),
    "glyph": (3, 4, glyph()),
    "noisy": (0, 0, noisy(18, 16)),
    "small": (5, 5, rect(3, 2, (0, 0, 255))),
}


def inp_name(key):
    if key not in _inputs:
        x, y, img = INPUTS[key]
        _inputs[key] = save_png("in_" + key, img)
    return _inputs[key]


def effect(moa, **params):
    return {"moa_id": moa, "visible": True, "params": {k: str(v) for k, v in params.items()}, "name": moa}


def case(name, key, effects, legacy=False):
    x, y, img = INPUTS[key]
    obj = (x, y, np.asarray(img, np.int64))
    if legacy:
        fx.set_legacy_cs5(True)
    try:
        res, skipped = fx.apply_live_effects(obj, effects)
    finally:
        if legacy:
            fx.set_legacy_cs5(False)
    assert not skipped, (name, skipped)
    rx, ry, rimg = res
    rimg = np.asarray(rimg, np.int64)
    if rimg.size == 0:
        rimg = np.zeros((1, 1, 4), np.int64)
    manifest.append({
        "name": name,
        "input": inp_name(key), "input_origin": [x, y],
        "output": save_png("out_%03d" % len(manifest), rimg), "output_origin": [int(rx), int(ry)],
        "effects": [{"moa_id": e["moa_id"], "visible": True, "params": e["params"]} for e in effects],
        "legacy_cs5": legacy,
    })


# ---------------------------------------------------------------- Cases
# Drop shadow
for key in ("rect", "rect_a128", "disc", "glyph", "ring", "noisy"):
    for blur, dist, ang in ((4, 0, 315), (4, 2, 315), (3, 3, 45), (3, 6, 90), (2, 3, 135), (3, 6, 225),
                            (3, 3, 270), (0, 1, 315), (1, 2, 450), (5, 7, 315), (6, 1, 90)):
        case("drop_%s_b%d_d%d_a%d" % (key, blur, dist, ang), key,
             [effect(DROP, ShadowColor="#000000a2", ShadowBlur=blur, ShadowDistance=dist, ShadowAngle=ang, ShadowType=0)])
for key in ("rect", "disc", "glyph"):
    case("drop_knock_" + key, key, [effect(DROP, ShadowColor="#3020ffff", ShadowBlur=4, ShadowDistance=3, ShadowAngle=315, ShadowType=1)])
    case("drop_cs5_" + key, key, [effect(DROP, ShadowColor="#00000040", ShadowBlur=4, ShadowDistance=2, ShadowAngle=315)], legacy=True)

# Inner shadow
for key in ("rect", "rect_a128", "disc", "glyph", "ring", "gray"):
    for blur, dist, ang in ((4, 7, 315), (2, 3, 315), (0, 1, 90), (1, 2, 200), (3, 4, 45)):
        case("inner_%s_b%d_d%d_a%d" % (key, blur, dist, ang), key,
             [effect(INNER, ShadowColor="#000000a5", ShadowBlur=blur, ShadowDistance=dist, ShadowAngle=ang, ShadowType=0)])
    case("inner_knock_" + key, key, [effect(INNER, ShadowColor="#ff0000c0", ShadowBlur=2, ShadowDistance=3, ShadowAngle=315, ShadowType=1)])

# Solid shadow
for key in ("rect", "disc", "glyph", "ring", "small"):
    for ang, dist, col in ((45, 12, "#ff0000"), (45, 12, "#ffffff00"), (30, 7, "#0000ff"), (90, 1, "#00ff00"),
                           (270, 1, "#123456"), (315, 5, "#ff00ff80"), (200, 9, "#ffffff00"), (450, 4, "#000000")):
        case("solid_%s_a%d_d%d_%s" % (key, ang, dist, col.strip("#")), key,
             [effect(SOLID, angle=ang, distance=dist, color=col, usecolor="true")])

# Inner glow
for key in ("rect", "gray", "disc", "ring", "glyph"):
    for width, soft, start, con, col in ((4, 4, 0, 65, "#000000"), (2, 7, 0, 35, "#330000"), (2, 0, 0, 100, "#ffffff"),
                                         (6, 2, 0, 80, "#00ff00"), (4, 3, 2, 65, "#0000ff"), (3, 0, 1, 100, "#ff8800")):
        case("iglow_%s_w%d_s%d_st%d_c%d" % (key, width, soft, start, con), key,
             [effect(IGLOW, OuterBevelColor=col, MaskSoftness=soft, GlowWidth=width, GlowStartDistance=start, BevelContrast=con)])

# Bevel / emboss
BEV = dict(BevelWidth=6, BevelContrast=75, EdgeThreshold=0, AngleSoftness=3, MaskSoftness=0, LightDistance=100,
           LightAngle=135, SlopeType=0, SlopeMultiplier=1, OuterBevelColor="#df0000", DownBlendColor="#0000003f",
           HitBlendColor="#ffffff3f", HiliteColor="#ffffff", ShadowColor="#000000", EmbossFaceColor="#00000000",
           ShowObject="false", ButtonState=0)


def bev(**kw):
    p = dict(BEV)
    p.update(kw)
    return effect(BEVEL, **p)


for key in ("gray", "disc", "ring", "glyph"):
    case("bevel_inner_" + key, key, [bev(BevelType=0)])
    case("bevel_outer_" + key, key, [bev(BevelType=1)])
    case("emboss_raised_" + key, key, [bev(BevelType=2, BevelWidth=2, MaskSoftness=2)])
    case("emboss_inset_" + key, key, [bev(BevelType=3, BevelWidth=2, MaskSoftness=2, EmbossFaceColor="#0000001e")])
for st in (1, 2, 3):
    case("bevel_state%d" % st, "gray", [bev(BevelType=0, ButtonState=st)])
    case("bevel_outer_state%d" % st, "disc", [bev(BevelType=1, ButtonState=st)])
for sl in range(1, 7):
    case("bevel_slope%d" % sl, "gray", [bev(BevelType=0, SlopeType=sl)])
    case("bevel_outer_slope%d" % sl, "gray", [bev(BevelType=1, SlopeType=sl)])
    case("bevel_slope%d_disc" % sl, "disc", [bev(BevelType=0, SlopeType=sl, BevelWidth=4, SlopeMultiplier=1.5)])
for wdt, soft, con, ang in ((1, 0, 75, 135), (3, 10, 60, 45), (10, 5, 90, 300), (4, 1, 100, 0)):
    case("bevel_w%d_s%d_c%d_a%d" % (wdt, soft, con, ang), "ring", [bev(BevelType=0, BevelWidth=wdt, AngleSoftness=soft, BevelContrast=con, LightAngle=ang)])
case("emboss_showobj", "disc", [bev(BevelType=2, BevelWidth=3, MaskSoftness=1, ShowObject="true")])
case("emboss_face", "gray", [bev(BevelType=3, BevelWidth=2, MaskSoftness=2, EmbossFaceColor="#4080c0ff")])

# Glow
for key in ("gray", "disc", "glyph", "rect_a128"):
    for wdt, soft, start, con, edge in ((3, 7, 0, 65, 1), (1, 9, 0, 20, 1), (5, 12, 0, 65, 1), (4, 5, 8, 60, 1), (2, 3, 2, 50, 0.5)):
        case("glow_%s_w%d_s%d_st%d" % (key, wdt, soft, start), key,
             [bev(BevelType=4, GlowWidth=wdt, MaskSoftness=soft, GlowStartDistance=start, BevelContrast=con, EdgeThreshold=edge,
                  AngleSoftness=25, OuterBevelColor="#999999")])

# Color fill
for key in ("rect_a128", "disc", "noisy"):
    for col, op in (("#ff0000", 100), ("#00ff80", 50), ("#123456", 33)):
        case("fill_%s_%s_%d" % (key, col.strip("#"), op), key, [effect(FILL, Color=col, Opacity=op, Blendmode=0)])

# Photoshop effects
SHADOW_TPL = "1&{op:f}&1&{ang:f}&0.000000&{dist:f}&{size:f}&{choke:f}&0&1&multiply&RGBColor;{r:f};{g:f};{b:f}&2$$0.000000;0.000000;1$255.000000;255.000000;1"
for key in ("gray", "disc", "glyph", "rect_a128"):
    for size, dist, ang, op, choke in ((0, 1, 120, 75, 0), (1, 2, 90, 75, 0), (2, 3, -90, 50, 0), (3, 3, 120, 75, 0),
                                       (5, 5, 45, 100, 0), (10, 4, 135, 60, 0), (6, 2, 120, 75, 30)):
        rec = SHADOW_TPL.format(op=op, ang=ang, dist=dist, size=size, choke=choke, r=10, g=20, b=211.996765)
        case("ps_drop_%s_s%d_d%d_a%d_c%d" % (key, size, dist, ang, choke), key, [effect(PS, dropShadow=rec)])
for key in ("disc", "rect_a128"):
    case("ps_fill_" + key, key, [effect(PS, solidFill="1&60.000000&normal&RGBColor;255.000000;0.000000;127.500000")])
    rec = SHADOW_TPL.format(op=75, ang=120, dist=3, size=5, choke=0, r=0, g=0, b=0)
    case("ps_fill_drop_" + key, key, [effect(PS, solidFill="1&100.000000&normal&RGBColor;0.000000;0.000000;255.000000", dropShadow=rec)])

# Chains
for key in ("gray", "disc"):
    case("chain_iglow_drop_" + key, key, [
        effect(IGLOW, OuterBevelColor="#ffff00", MaskSoftness=3, GlowWidth=3, GlowStartDistance=0, BevelContrast=70),
        effect(DROP, ShadowColor="#000000a2", ShadowBlur=4, ShadowDistance=3, ShadowAngle=315, ShadowType=0)])
    case("chain_bevel_inner_" + key, key, [
        bev(BevelType=0, BevelWidth=3),
        effect(INNER, ShadowColor="#000000a5", ShadowBlur=2, ShadowDistance=2, ShadowAngle=315, ShadowType=0)])
    case("chain_fill_glow_" + key, key, [
        effect(FILL, Color="#00ff00", Opacity=50, Blendmode=0),
        bev(BevelType=4, GlowWidth=3, MaskSoftness=4, GlowStartDistance=0, BevelContrast=65, EdgeThreshold=1, OuterBevelColor="#0000ff")])

with open(os.path.join(OUT, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=0)
print(len(manifest), "cases ->", OUT)
