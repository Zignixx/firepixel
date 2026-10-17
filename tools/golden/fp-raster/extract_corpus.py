"""Korpusdaten fuer die #[ignore]-Tests von fp-raster (tests/corpus.rs).

1. Korpus-Paare aus re\\filter\\_scratch\\pairs (Eingabe/Cache je Bitmap bzw. Gruppe, 06 §9) -> PNG + pairs.json.
   Gleiche Inhalte (gleicher Hash-Suffix) werden nur einmal gespeichert; die Liste behaelt alle Faelle,
   damit die Zahlen denen von scripts\\filter\\test_pairs.py entsprechen.
2. Testdateien des Originals re\\_testdateien\\06\\*.fw.png (Laufzeittests 06 §11.2): je Bitmap-Element mit Namen
   die gespeicherten Pixel (Eingabe bzw. destruktives Ergebnis), Lage, Live-Filter, Seitenursprung -> fw06.json.
   Das Referenzbild (IDAT) liest der Rust-Test selbst aus der .fw.png.

Aufruf:
  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I extract_corpus.py
"""
import glob
import json
import os
import sys

import numpy as np

RE = r"C:\Users\Administrator\Desktop\REA\re"
sys.path.insert(0, os.path.join(RE, "format"))
import fwpng  # noqa: E402
from fwfmt import model, pngio, render  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "..", "..", "crates", "fp-raster", "tests", "data", "corpus"))
os.makedirs(OUT, exist_ok=True)


def save(fn, arr):
    arr = np.ascontiguousarray(np.asarray(arr, np.uint8))
    h, w = arr.shape[:2]
    p = os.path.join(OUT, fn)
    if not os.path.exists(p):
        with open(p, "wb") as f:
            f.write(pngio.encode_rgba_png(w, h, arr.tobytes()))
    return fn


def points(v):
    pts = []
    for gpl in v or []:
        for gpt in gpl.get("GPLv", []):
            d = {}
            for c in gpt.get("GPTv", []):
                d.update(c)
            pts.append([float(d.get("XLCf", 0.0)), float(d.get("YLCf", 0.0))])
    return pts


def norm_params(p):
    out = {}
    for k, v in p.items():
        if k.endswith("_points"):
            out[k] = points(v)
        elif isinstance(v, str):
            out[k] = v
        else:
            out[k] = str(v)
    return out


def norm_effects(effs):
    return [{"moa_id": e.get("moa_id") or "", "name": e.get("name") or "", "visible": bool(e.get("visible", True)),
             "params": norm_params(e.get("params", {}))} for e in effs]


# ---------------------------------------------------------------- 1. Paare
P = os.path.join(RE, "filter", "_scratch", "pairs")
idx = json.load(open(os.path.join(P, "index.json"), encoding="utf-8"))
pairs = []
for it in idx:
    if it.get("missing"):
        continue
    z = np.load(os.path.join(P, it["name"] + ".npz"))
    key = it["name"].rsplit("_", 1)[-1]
    pairs.append({"name": it["name"], "input": save("pair_in_%s.png" % key, z["inp"]),
                  "output": save("pair_out_%s.png" % key, z["out"]), "effects": norm_effects(it["effects"])})
json.dump(pairs, open(os.path.join(OUT, "pairs.json"), "w", encoding="utf-8"), indent=0)
print("Paare:", len(pairs))

# ---------------------------------------------------------------- 2. Testdateien 06
D = os.path.join(RE, "_testdateien", "06")
files = []
for fn in sorted(glob.glob(os.path.join(D, "*.fw.png"))):
    f = fwpng.FwFile(fn, inflate_tiles=True)
    st = render.TileStore(f.tiles)
    pg = f.doc_roots()[0]
    elems = []
    t7 = os.path.basename(fn).startswith("t7")
    for path, n in pg.walk():
        if n.key != "IMGv" or not (n.get("OBN") or (t7 and not elems)):
            continue
        b = render.bitmap_from_img(n, st)
        if b is None:
            continue
        x, y, w, h, px = b
        img = np.frombuffer(bytes(px), np.uint8).reshape(h, w, 4)
        effs = list(model.effects(n.child("EFLv"))) if n.child("EFLv") is not None else []
        base = os.path.basename(fn).replace(".fw.png", "")
        name = n.get("OBN") or "bitmap"
        elems.append({"name": name, "x": int(round(x)), "y": int(round(y)),
                      "pixels": save("fw06_%s_%s.png" % (base, name.replace(" ", "_")), img),
                      "effects": norm_effects(effs)})
    files.append({"file": os.path.basename(fn), "page_x": int(pg.get("XLC", 0)), "page_y": int(pg.get("YLC", 0)),
                  "elements": elems})
    print(os.path.basename(fn), [(e["name"], [x["name"] for x in e["effects"]]) for e in elems])
json.dump(files, open(os.path.join(OUT, "fw06.json"), "w", encoding="utf-8"), indent=0)
