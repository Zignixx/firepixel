"""Golden data for fp-export from the Python references re/export (fw_palette, fw_rgba, fw_gif) and
re/verhalten (fwbehavior). Output: crates/fp-export/tests/data/golden.json (small, in the repo).

Cases: synthetic images (gradient, noise with alpha, few colors) x palette kinds (adaptive, web adaptive,
grid, web 216, grayscale, Mac, Windows, exact, custom), dither (off, diffusion 100/50, 2x2),
transparency (none, index alpha, RGBA), locked colors, lossy GIF, TRNs.
Usage: python -I gen_golden.py
"""
import json
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.abspath(os.path.join(FP, "..", "re"))
sys.path[:0] = [os.path.join(RE, "export"), os.path.join(RE, "verhalten")]
import expv  # noqa: E402
import fw_gif  # noqa: E402
import fw_palette as P  # noqa: E402
import fw_rgba  # noqa: E402

rng = random.Random(20261011)
W, H = 24, 16


def img_gradient():
    return [0xFF000000 | ((x * 255 // (W - 1)) << 16) | ((y * 255 // (H - 1)) << 8) | ((x * y) % 256) for y in range(H) for x in range(W)]


def img_noise_alpha():
    out = []
    for y in range(H):
        for x in range(W):
            a = rng.choice([0, 0, 40, 128, 200, 255, 255, 255])
            out.append((a << 24) | (rng.randrange(256) << 16) | (rng.randrange(256) << 8) | rng.randrange(256))
    return out


def img_few():
    cols = [0xFFFF0000, 0xFF00FF00, 0xFF0000FF, 0xFFFFFFFF, 0xFF333333, 0xFFCC9966]
    return [cols[(x // 4 + y // 3) % len(cols)] for y in range(H) for x in range(W)]


IMAGES = {"gradient": img_gradient(), "noise_alpha": img_noise_alpha(), "few": img_few()}
CASES = []


def run(name, px, **over):
    o = dict(expv.DEFAULTS)
    o.update(exportFormat=0, palette=[])
    o.update(over)
    matte = over.pop("_matte", 0xFFFFFFFF) if "_matte" in over else 0xFFFFFFFF
    o.pop("_matte", None)
    canvas = 0xFFFFFFFF
    q = P.build(px, o, matte=matte, canvas=canvas, width=W)
    if o["paletteTransparency"] == 3 and o["exportFormat"] == 2:
        idx = fw_rgba.remap_rgba(q.pal, px, W, H, o["ditherMode"] == 1, o["ditherPercent"], o["paletteMode"] == 1 and o["webSnapAdaptive"], o["webSnapTolerance"])
    elif o["ditherMode"] == 1:
        idx = q.remap_diffusion(px, W, H, o["ditherPercent"], o["paletteMode"] == 1 and o["webSnapAdaptive"], o["webSnapTolerance"])
    elif o["ditherMode"] == 2:
        idx = q.remap_2x2(px, W, H)
    else:
        idx = q.remap_none(px, W, H, o["optimized"], 7 if o["optimized"] else 0, 0)
    CASES.append({"name": name, "image": name.split("/")[0], "opts": {k: v for k, v in o.items() if k != "frames"},
                  "palette": list(q.pal), "indices": list(idx), "tmode": q.tmode})


for iname, px in IMAGES.items():
    run(iname + "/adaptive256", px)
    run(iname + "/adaptive16_d100", px, numEntriesRequested=16, ditherMode=1, ditherPercent=100)
    run(iname + "/adaptive32_d50", px, numEntriesRequested=32, ditherMode=1, ditherPercent=50)
    run(iname + "/websnap64", px, numEntriesRequested=64, webSnapAdaptive=True)
    run(iname + "/adaptive8_2x2", px, numEntriesRequested=8, ditherMode=2)
    run(iname + "/grid4", px, paletteMode=2, numGridEntries=4)
    run(iname + "/web216_d100", px, paletteMode=7, ditherMode=1, ditherPercent=100)
    run(iname + "/gray8", px, paletteMode=3, numEntriesRequested=8)
    run(iname + "/mac", px, paletteMode=4)
    run(iname + "/win_d100", px, paletteMode=5, ditherMode=1, ditherPercent=100)
    run(iname + "/exact", px, paletteMode=6)
    run(iname + "/locked", px, numEntriesRequested=12, palette=[{"color": 0xFF102030, "locked": True}, {"color": 0xFFFFFF00, "locked": True}])
    run(iname + "/custom", px, paletteMode=0, palette=[{"color": c, "locked": False} for c in (0xFF000000, 0xFF808080, 0xFFFFFFFF, 0xFFFF0000)])
    run(iname + "/indexalpha", px, paletteTransparency=2, numEntriesRequested=64)
    run(iname + "/rgba_png", px, exportFormat=2, paletteTransparency=3, numEntriesRequested=128)
    run(iname + "/rgba_png_d100", px, exportFormat=2, paletteTransparency=3, numEntriesRequested=64, ditherMode=1, ditherPercent=100)
    run(iname + "/unopt", px, optimized=False, numEntriesRequested=20)

# lossy GIF on an indexed image
c0 = next(c for c in CASES if c["name"] == "gradient/adaptive16_d100")
pal_rgb = [((c >> 16) & 255, (c >> 8) & 255, c & 255) for c in c0["palette"]]
lossy = {str(l): list(fw_gif.lossy_simulate(bytes(c0["indices"]), W, H, pal_rgb, l)) for l in (10, 40, 80)}
lzw = {"indices": c0["indices"], "data": list(fw_gif.lzw_encode(bytes(c0["indices"]), 4))}

dst = os.path.join(FP, "crates", "fp-export", "tests", "data")
os.makedirs(dst, exist_ok=True)
json.dump({"w": W, "h": H, "images": IMAGES, "cases": CASES, "lossy": {"palette": [list(p) for p in pal_rgb], "indices": c0["indices"], "out": lossy}, "lzw": lzw},
          open(os.path.join(dst, "golden.json"), "w"), separators=(",", ":"))
print("cases:", len(CASES), "->", os.path.join(dst, "golden.json"))
