"""Golden-Daten fuer fp-raster aus den Python-Referenzen (re\\filter\\fwfilters.py, re\\effekte\\fx.py,
re\\symbole\\laufzeit\\t2_9slice.py).

Erzeugt kleine Eingabe-/Ausgabepaare als PNG plus manifest.json unter crates\\fp-raster\\tests\\data\\golden\\.
Der Rust-Test tests/golden.rs liest das Manifest, wendet den Filter (JSON im serde-Format von
fp_raster::Filter) an und vergleicht pixelgenau (alle vier Kanaele).

Aufruf:
  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I gen_golden.py
"""
import json
import os
import sys

import numpy as np

RE = r"C:\Users\Administrator\Desktop\REA\re"
for sub in ("filter", "effekte", os.path.join("symbole", "laufzeit"), "format"):
    sys.path.insert(0, os.path.join(RE, sub))

import fwfilters as F  # noqa: E402
import fx  # noqa: E402
import t2_9slice as NS  # noqa: E402
from fwfmt import pngio  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "..", "..", "crates", "fp-raster", "tests", "data", "golden"))
os.makedirs(OUT, exist_ok=True)

manifest = []
_saved = {}


def save(name, arr):
    arr = np.ascontiguousarray(arr.astype(np.uint8))
    h, w = arr.shape[:2]
    fn = name + ".png"
    with open(os.path.join(OUT, fn), "wb") as f:
        f.write(pngio.encode_rgba_png(w, h, arr.tobytes()))
    return fn


def inp(name, arr):
    if name not in _saved:
        _saved[name] = save("in_" + name, arr)
    return _saved[name]


def mask_png(arr):
    a = np.asarray(arr, np.int64).clip(0, 255)
    return np.stack([a, a, a, np.full_like(a, 255)], axis=-1)


# ---------------------------------------------------------------- Eingaben
rng = np.random.default_rng(20261011)


def rnd_img(w, h):
    img = rng.integers(0, 256, (h, w, 4), dtype=np.int64)
    sel = rng.random((h, w))
    img[..., 3] = np.where(sel < 0.25, 0, np.where(sel < 0.5, 255, img[..., 3]))
    return img.astype(np.uint8)


def grad_img(w, h):
    y, x = np.mgrid[0:h, 0:w]
    img = np.zeros((h, w, 4), np.int64)
    img[..., 0] = (x * 255) // max(w - 1, 1)
    img[..., 1] = (y * 255) // max(h - 1, 1)
    img[..., 2] = ((x + y) * 7) % 256
    img[..., 3] = 255
    return img.astype(np.uint8)


def disc_img(w, h):
    y, x = np.mgrid[0:h, 0:w]
    cx, cy, r = w / 2.0, h / 2.0, min(w, h) / 3.0
    d = np.sqrt((x + 0.5 - cx) ** 2 + (y + 0.5 - cy) ** 2)
    img = rng.integers(0, 256, (h, w, 4), dtype=np.int64)  # Farbe transparenter Pixel zufaellig
    a = np.clip((r + 1.5 - d) / 3.0, 0, 1) * 255
    img[..., 3] = a.astype(np.int64)
    img[..., 0] = np.where(a > 0, 200, img[..., 0])
    img[..., 1] = np.where(a > 0, (x * 9) % 256, img[..., 1])
    img[..., 2] = np.where(a > 0, 40, img[..., 2])
    return img.astype(np.uint8)


IMGS = {"rnd": rnd_img(23, 17), "grad": grad_img(31, 19), "disc": disc_img(24, 20)}


def add(name, src, fjson, out, out_origin=(0, 0), in_origin=(0, 0)):
    manifest.append({
        "name": name, "kind": "filter", "input": inp(src, IMGS[src]), "input_origin": list(in_origin),
        "filter": fjson, "output": save("out_" + name, out), "output_origin": list(out_origin),
    })


# ---------------------------------------------------------------- Farbfilter
for s in IMGS:
    add("invert_" + s, s, {"filter": "invert"}, F.invert(IMGS[s]))
    add("auto_levels_" + s, s, {"filter": "auto_levels"}, F.auto_levels(IMGS[s]))
    add("find_edges_" + s, s, {"filter": "find_edges"}, F.find_edges(IMGS[s]))
    add("alpha_" + s, s, {"filter": "convert_to_alpha"}, F.convert_to_alpha(IMGS[s]))
    add("blur_" + s, s, {"filter": "blur"}, F.blur(IMGS[s]))
    add("blur_more_" + s, s, {"filter": "blur_more"}, F.blur_more(IMGS[s]))
    for a in (1, 4, 8):
        add("sharpen%d_%s" % (a, s), s, {"filter": "sharpen", "params": {"amount": a}}, F.sharpen_amount(IMGS[s], a))
    for a in (1, 10):
        add("simple_blur%d_%s" % (a, s), s, {"filter": "simple_blur", "params": {"amount": a}}, F.simple_blur(IMGS[s], a))

for i, (b, c) in enumerate([(0, 0), (20, 0), (-30, 40), (10, -50), (0, 100), (0, -100), (-100, 73), (5, 15), (-6, 15)]):
    for s in ("rnd", "grad", "disc"):
        add("bc%d_%s" % (i, s), s, {"filter": "brightness_contrast", "params": {"brightness": b, "contrast": c}},
            F.brightness_contrast(IMGS[s], b, c))

HUES = [(0, -100, 0, False), (0, -100, 51, False), (-154, -15, 0, False), (-37, -46, 0, False), (90, 30, -20, False),
        (180, 100, 100, False), (-180, 0, -100, False), (200, 50, -30, True), (30, 80, 0, True), (360, 100, 60, True)]
for i, (h, sa, l, co) in enumerate(HUES):
    for s in ("rnd", "grad"):
        add("hue%d_%s" % (i, s), s, {"filter": "hue_saturation",
                                     "params": {"hue": h, "saturation": sa, "lightness": l, "colorize": co}},
            F.hue_saturation(IMGS[s], h, sa, l, co))

CURVES = [
    {"rgb": [(0, 0), (64, 90), (190, 160), (255, 255)], "red": [(0, 0), (255, 255)],
     "green": [(0, 0), (255, 255)], "blue": [(0, 0), (255, 255)]},
    {"rgb": [(0, 0), (255, 255)], "red": [(0, 30), (128, 100), (255, 220)], "green": [(20, 5), (230, 250)],
     "blue": [(0, 255), (255, 0)]},
    {"rgb": [(10, 0), (40, 200), (100, 60), (200, 230), (245, 255)], "red": [(0, 0), (255, 255)],
     "green": [(0, 0), (128.5, 140.25), (255, 255)], "blue": [(0, 0), (100, 100), (90, 50), (255, 255)]},
]
for i, cv in enumerate(CURVES):
    pts = lambda v: [{"x": float(x), "y": float(y)} for x, y in v]
    fj = {"filter": "curves", "params": {"rgb_points": pts(cv["rgb"]), "red_points": pts(cv["red"]),
                                         "green_points": pts(cv["green"]), "blue_points": pts(cv["blue"])}}
    for s in ("rnd", "grad"):
        add("curves%d_%s" % (i, s), s, fj, F.curves(IMGS[s], cv["rgb"], cv["red"], cv["green"], cv["blue"]))

LEVELS = [
    {"rgb": (10, 240, 5, 250, 1.4), "red": (0, 255, 0, 255, 0.7), "green": (30, 200, 0, 255, 1.0),
     "blue": (0, 255, 255, 0, 1.0)},
    {"rgb": (0, 255, 0, 255, 1.0), "red": (100, 100, 0, 255, 1.0), "green": (200, 50, 0, 255, 2.5),
     "blue": (0, 255, 30, 220, 0.1)},
]
for i, lv in enumerate(LEVELS):
    ch = lambda v: {"source_low": v[0], "source_high": v[1], "dest_low": v[2], "dest_high": v[3], "gamma": v[4]}
    fj = {"filter": "levels", "params": {k: ch(v) for k, v in lv.items()}}
    for s in ("rnd", "grad"):
        add("levels%d_%s" % (i, s), s, fj, F.levels(IMGS[s], lv))

for i, (col, op) in enumerate([("#000000", 100), ("#3366cc", 37), ("#ff8800", 0), ("#10e0a0", 99), ("#ffffff", 1)]):
    c = F._parse_color(col)
    fj = {"filter": "color_fill", "params": {"color": {"r": c[0], "g": c[1], "b": c[2], "a": 255}, "opacity": op,
                                             "blend_mode": 0}}
    for s in ("rnd", "disc"):
        add("colorfill%d_%s" % (i, s), s, fj, F.color_fill(IMGS[s], col, op, 0))

# ---------------------------------------------------------------- Weichzeichner
for r in (0.5, 2.0, 3.5, 4.1):
    for s in ("rnd", "disc", "grad"):
        out, m = F.gaussian_blur(IMGS[s], r)
        add("gauss_d%s_%s" % (str(r).replace(".", "p"), s), s,
            {"filter": "gaussian_blur", "params": {"radius": r, "precision": "Double"}}, out, (-m, -m))
for r in (2.0, 3.5, 1.3):
    for s in ("rnd", "disc", "grad"):
        rr = min(r, 70.0)
        m = int(np.ceil(rr * 2.5))
        img = IMGS[s]
        big = np.zeros((img.shape[0] + 2 * m, img.shape[1] + 2 * m, 4), np.uint8)
        big[m:m + img.shape[0], m:m + img.shape[1]] = img
        out = F.gaussian_blur_raw_pc24(big, rr)
        add("gauss_s%s_%s" % (str(r).replace(".", "p"), s), s,
            {"filter": "gaussian_blur", "params": {"radius": r, "precision": "Single"}}, out, (-m, -m))
def gauss_over_200_measured(img):
    """r > 200 (FX-3, re\\23 Gruppe 3): gefuellt und gemittelt wird nur das Pixelrechteck (Alpha > 0)
    der Eingabe; durchsichtige Pixel darin zaehlen mit. Ganz durchsichtig: die ganze Eingabe."""
    out = img.copy()
    ys, xs = np.nonzero(img[..., 3])
    if xs.size:
        y0, y1, x0, x1 = ys.min(), ys.max() + 1, xs.min(), xs.max() + 1
    else:
        y0, y1, x0, x1 = 0, img.shape[0], 0, img.shape[1]
    reg = img[y0:y1, x0:x1].astype(np.int64)
    n = reg.shape[0] * reg.shape[1]
    col = [int(reg[..., ch].sum()) // n for ch in range(3)] if n else [255, 255, 255]
    out[y0:y1, x0:x1] = (min(col[0], 255), min(col[1], 255), min(col[2], 255), 255)
    return out


for s in ("rnd", "disc"):
    add("gauss_210_" + s, s, {"filter": "gaussian_blur", "params": {"radius": 210.0}}, gauss_over_200_measured(IMGS[s]))

for i, (r, a, t) in enumerate([(4.1, 50, 0), (1.5, 200, 10), (2.0, 500, 0), (0.7, 1, 3)]):
    for s in ("rnd", "grad", "disc"):
        add("unsharp%d_%s" % (i, s), s, {"filter": "unsharp_mask",
                                         "params": {"radius": r, "amount": a, "threshold": t, "precision": "Double"}},
            F.unsharp_mask(IMGS[s], r, a, t))

def motion_blur_measured(img, ang, d):
    """Wie F.motion_blur (Proben i = 0 .. abstand), aber der Winkel als float wie im Original:
    sin 90 Grad und -cos 180 Grad liegen knapp unter 1, trunc(i*sin) = i - 1 (FX-3, re\\23 Gruppe 3:
    90/10 -> +9 mit Alpha-Stufen 255*k/11, 0/1 -> 1 px nach links; PM-6 D47 90/20 -> +19)."""
    import math
    m = max(0, int(d))
    H, W = img.shape[:2]
    big = np.zeros((H + 2 * m, W + 2 * m, 4), np.uint8)
    big[m:m + H, m:m + W] = img
    H, W = big.shape[:2]
    a = int(ang)
    if a >= 360:
        a %= 360
    if a < 0:
        a = a + 360 + ((-a - 1) // 360) * 360
    th = float(np.float32(np.float32(a) / np.float32(360.0)) * np.float32(6.2831855))
    sn, cs = math.sin(th), math.cos(th)
    out = big.copy()
    if int(d) < 0:
        return out, m
    src = np.where((big[..., 3] == 0)[..., None], 0, big.astype(np.int64))
    acc = np.zeros((H, W, 4), np.int64)
    cnt = np.zeros((H, W), np.int64)
    alive = np.ones((H, W), bool)
    ys, xs = np.mgrid[0:H, 0:W]
    i = 0
    while i <= int(d):
        dx, dy = (int(i * cs), int(i * sn))
        sx, sy = xs + dx, ys - dy
        alive &= (sx >= 0) & (sx < W) & (sy >= 0) & (sy < H)
        acc += np.where(alive[..., None], src[np.clip(sy, 0, H - 1), np.clip(sx, 0, W - 1)], 0)
        cnt += alive
        i += 1
    ok = cnt > 0
    out[ok] = np.minimum(255, acc // np.maximum(cnt, 1)[..., None])[ok].astype(np.uint8)
    return out, m


for i, (ang, d) in enumerate([(90, 10), (0, 5), (45, 7), (200, 3), (-30, 6), (359, 4), (720, 2)]):
    for s in ("rnd", "disc"):
        out, m = motion_blur_measured(IMGS[s], ang, d)
        add("motion%d_%s" % (i, s), s, {"filter": "motion_blur", "params": {"angle": ang, "distance": d}}, out, (-m, -m))

for i, (a, q) in enumerate([(30, 20), (100, 100), (10, 50)]):
    for s in ("rnd", "disc", "grad"):
        out, (ox, oy) = F.radial_blur(IMGS[s], a, q)
        add("radial%d_%s" % (i, s), s, {"filter": "radial_blur", "params": {"amount": a, "quality": q}}, out, (-ox, -oy))

def zoom_blur_measured(img, a, q):
    """Wie F.zoom_blur, aber auf dem vergroesserten Puffer (06 4.13, 00b2c9e0: Rechteck + Menge, dann
    Quadrat mit Halbkante floor(sqrt(((H'+1)/2)^2 + ((B'+1)/2)^2) + 0,5) + Menge) und Mitte der
    Eingabegrenzen (FX-3, re\\23 Gruppe 3: 100/100 am Vektor -> sichtbar 32,21-207,158)."""
    import math
    H, W = img.shape[:2]
    l, t, r, b = -a, -a, W + a, H + a
    w, h = r - l, b - t
    half = int(math.floor(math.sqrt(((h + 1) // 2) ** 2 + ((w + 1) // 2) ** 2) + 0.5)) + a
    cx, cy = l + w // 2, t + h // 2
    L, T = cx - half, cy - half
    big = np.zeros((2 * half, 2 * half, 4), np.uint8)
    big[-T:-T + H, -L:-L + W] = img
    out = F.zoom_blur_raw(big, a, q, ((W + 1) // 2 - L, (H + 1) // 2 - T))
    return out, (L, T)


for i, (a, q) in enumerate([(30, 20), (100, 100), (60, 40)]):
    for s in ("rnd", "disc"):
        out, org = zoom_blur_measured(IMGS[s], a, q)
        add("zoom%d_%s" % (i, s), s, {"filter": "zoom_blur", "params": {"amount": a, "quality": q}}, out, org)

# ---------------------------------------------------------------- Rauschen
for i, (a, col, org) in enumerate([(50, False, (0, 0)), (100, True, (-13, 7)), (400, False, (5, -3)), (1, True, (300, 400))]):
    for s in ("rnd", "grad"):
        add("noise%d_%s" % (i, s), s, {"filter": "add_noise", "params": {"amount": a, "use_color": col}},
            F.add_noise(IMGS[s], a, col, origin=org), org, org)
IMGS["wide"] = np.full((9, 620, 4), 128, np.uint8)
IMGS["wide"][..., 3] = 255
for i, (a, col, org) in enumerate([(50, False, (-13, 7)), (70, True, (487, -2))]):
    add("noise_wide%d" % i, "wide", {"filter": "add_noise", "params": {"amount": a, "use_color": col}},
        F.add_noise(IMGS["wide"], a, col, origin=org), org, org)

# ---------------------------------------------------------------- Masken (05 §2.2, 07 T6, 06 T7)
M = np.zeros((50, 60), np.int64)
M[15:35, 20:40] = 255
yy, xx = np.mgrid[0:50, 0:60]
M2 = np.where((xx - 30.0) ** 2 + (yy - 25.0) ** 2 < 81, 255, 0) | np.where((xx > 44) & (xx < 54) & (yy > 30) & (yy < 40), 200, 0)
MASKS = {"rect": M, "blob": M2}
for mn, mk in MASKS.items():
    mi = inp("mask_" + mn, mask_png(mk))
    for r in (1, 3, 10):
        out, g = fx.tent_blur(mk, r, last_row_screen=True)
        manifest.append({"name": "feather%d_%s" % (r, mn), "kind": "mask", "op": "feather", "amount": r,
                         "input": mi, "output": save("out_feather%d_%s" % (r, mn), mask_png(out)),
                         "output_origin": [-g, -g]})
        out, g = fx.tent_blur(mk, r, last_row_screen=False)
        manifest.append({"name": "tent%d_%s" % (r, mn), "kind": "mask", "op": "tent", "amount": r,
                         "input": mi, "output": save("out_tent%d_%s" % (r, mn), mask_png(out)),
                         "output_origin": [-g, -g]})
    for mode, op in ((0, "expand"), (1, "contract"), (2, "border"), (3, "smooth")):
        for amt in (3, 6):
            out = fx.morph(mk, amt, mode, 1)
            manifest.append({"name": "%s%d_%s" % (op, amt, mn), "kind": "mask", "op": op, "amount": amt,
                             "input": mi, "output": save("out_%s%d_%s" % (op, amt, mn), mask_png(out)),
                             "output_origin": [0, 0], "compare": "input_bounds"})

# ---------------------------------------------------------------- 9-Slice bikubisch (13 T2)
src9 = rnd_img(20, 20)
src9[..., 3] = np.where(src9[..., 3] == 0, 30, src9[..., 3])
si = inp("nine", src9)
H9 = [[tuple(int(v) for v in src9[y, x]) for x in range(20)] for y in range(20)]
for (W, H) in ((60, 60), (50, 20), (20, 20), (13, 31)):
    out = np.zeros((H, W, 4), np.uint8)
    for y in range(H):
        sy = NS.map_ecken(y, H) + 0.5
        for x in range(W):
            sx = NS.map_ecken(x, W) + 0.5
            NS.ZONE = NS.zone(x, W) + NS.zone(y, H)
            out[y, x] = NS.sample(H9, 20, 20, sx, sy, "bicubic1")
    NS.ZONE = None
    manifest.append({"name": "nineslice_%dx%d" % (W, H), "kind": "nineslice", "insets": [5, 5, 5, 5],
                     "size": [W, H], "input": si, "output": save("out_nineslice_%dx%d" % (W, H), out),
                     "output_origin": [0, 0]})

with open(os.path.join(OUT, "manifest.json"), "w", encoding="utf-8") as f:
    json.dump(manifest, f, indent=0)
print("%d Faelle nach %s" % (len(manifest), OUT))
