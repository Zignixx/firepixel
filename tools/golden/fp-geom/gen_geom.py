"""Golden-Testvektoren fuer fp-geom aus den Python-Referenzen (nur lesen) in re\\pfade und re\\vektor.

Aufruf (aus beliebigem Verzeichnis):
  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_geom.py
(-B: keine .pyc-Dateien in den nur lesbaren Referenzordnern anlegen)

Schreibt crates/fp-geom/tests/data/geom_golden.json. Zufallswerte mit festem Startwert, damit die Datei
reproduzierbar ist. Gleitkommawerte werden mit repr() geschrieben (exakte Rundreise nach f64).
"""
import json
import os
import random
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "pfade"))
sys.path.insert(0, os.path.join(RE, "vektor"))

import bezier as BZ          # noqa: E402
import flatten as FL         # noqa: E402
import fwraster as FR        # noqa: E402
import offset as OF          # noqa: E402

OUT = os.path.join(FP, "crates", "fp-geom", "tests", "data", "geom_golden.json")
rng = random.Random(20261011)


def f32(x):
    return struct.unpack("<f", struct.pack("<f", x))[0]


def rnd_pt(scale=200.0, q32=False):
    p = (rng.uniform(-scale, scale), rng.uniform(-scale, scale))
    return (f32(p[0]), f32(p[1])) if q32 else p


def rnd_seg(q32=False):
    kind = rng.random()
    p0 = rnd_pt(q32=q32)
    p3 = rnd_pt(q32=q32)
    if kind < 0.12:          # Gerade (Henkel auf den Ankern)
        return (p0, p0, p3, p3)
    if kind < 0.22:          # ein Henkel eingezogen
        return (p0, p0, rnd_pt(q32=q32), p3)
    if kind < 0.30:          # kleines Segment
        s = rng.uniform(0.05, 3.0)
        b = rnd_pt(q32=q32)
        pts = [(b[0] + rng.uniform(-s, s), b[1] + rng.uniform(-s, s)) for _ in range(4)]
        if q32:
            pts = [(f32(x), f32(y)) for x, y in pts]
        return tuple(pts)
    return (p0, rnd_pt(q32=q32), rnd_pt(q32=q32), p3)


def seg_list(s):
    return [list(p) for p in s]


# ------------------------------------------------------------------ Bezier-Grundlagen
TS = [0.0, 0.1, 0.33, 0.5, 0.77, 1.0]
bez = []
for _ in range(250):
    s = rnd_seg()
    p = rnd_pt()
    tsplit = rng.uniform(0.05, 0.95)
    a, b = BZ.split(s, tsplit)
    t0, t1 = sorted((rng.uniform(0, 1), rng.uniform(0, 1)))
    near = BZ.nearest_on_seg(s, p)
    bez.append({
        "seg": seg_list(s),
        "point": [list(BZ.point(s, t)) for t in TS],
        "deriv": [list(BZ.deriv(s, t)) for t in TS],
        "deriv2": [list(BZ.deriv2(s, t)) for t in TS],
        "tangent": [list(BZ.tangent(s, t)) for t in TS],
        "tsplit": tsplit, "split": [seg_list(a), seg_list(b)],
        "t0": t0, "t1": t1, "sub": seg_list(BZ.sub_segment(s, t0, t1)),
        "extrema": BZ.seg_extrema_t(s),
        "bounds": list(BZ.seg_bounds(s)),
        "hull": list(BZ.hull_bounds(s)),
        "length": BZ.seg_length(s),
        "p": list(p), "near_t": near[0], "near_d": near[1], "near_pt": list(near[2]),
    })

# ------------------------------------------------------------------ Original-Abflachung (PCLv)
flat = []
for i in range(400):
    s = rnd_seg(q32=True)
    n = [1, 1, 1, 64, 3, 2][i % 6]
    pts = FL.flatten_segment(s[0], s[1], s[2], s[3], n)
    flat.append({"seg": seg_list(s), "n": n, "pts": [list(q) for q in pts]})
# ohne Eigenheit (quirk=False) als Gegenprobe
flat_noquirk = []
for i in range(60):
    s = rnd_seg(q32=True)
    pts = FL.flatten_segment(s[0], s[1], s[2], s[3], 1, quirk=False)
    flat_noquirk.append({"seg": seg_list(s), "n": 1, "pts": [list(q) for q in pts]})

contours = []
for i in range(40):
    k = rng.randint(2, 7)
    nodes = []
    for _ in range(k):
        a = rnd_pt(q32=True)
        if rng.random() < 0.3:
            nodes.append((a, a, a))
        else:
            nodes.append((a, rnd_pt(q32=True), rnd_pt(q32=True)))
    closed = rng.random() < 0.6
    counts = [rng.choice([1, 1, 1, 64]) for _ in range(k)]
    poly = FL.flatten_contour(nodes, closed, counts)
    contours.append({"nodes": [[list(a), list(b), list(c)] for a, b, c in nodes], "closed": closed,
                     "counts": counts, "pts": [list(q) for q in poly]})

# ------------------------------------------------------------------ alte Rasterizer-Abflachung (07)
chord = []
for i in range(120):
    s = rnd_seg()
    pts = FR.flatten_adaptive(s[0], s[1], s[2], s[3])
    chord.append({"seg": seg_list(s), "pts": [list(q) for q in pts]})

# ------------------------------------------------------------------ Offset
offs = []
for i in range(120):
    s = rnd_seg()
    if BZ.is_line(s) is False and all(abs(p[0] - s[0][0]) < 1e-12 and abs(p[1] - s[0][1]) < 1e-12 for p in s):
        continue
    d = rng.choice([1.5, -1.5, 7.0, -7.0, 0.4, 20.0])
    res = OF.offset_segment(s, d)
    offs.append({"seg": seg_list(s), "d": d, "out": [seg_list(o) for o in res]})

# ------------------------------------------------------------------ Flaeche / Windungszahl
polys = []
for i in range(30):
    k = rng.randint(3, 9)
    pts = [rnd_pt(50, q32=True) for _ in range(k)]
    probes = [rnd_pt(60) for _ in range(5)]
    polys.append({"pts": [list(p) for p in pts], "area": BZ.signed_area(pts),
                  "probes": [list(p) for p in probes],
                  "winding": [BZ.winding([(pts, True)], p) for p in probes]})

data = {"_quelle": "re/pfade/bezier.py, flatten.py, offset.py; re/vektor/fwraster.py",
        "bezier": bez, "flatten": flat, "flatten_noquirk": flat_noquirk, "flatten_contour": contours,
        "chord": chord, "offset": offs, "polygons": polys}
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT, "w", encoding="utf-8") as fh:
    json.dump(data, fh, separators=(",", ":"))
print("geschrieben:", OUT, os.path.getsize(OUT), "Byte;",
      {k: len(v) for k, v in data.items() if isinstance(v, list)})
