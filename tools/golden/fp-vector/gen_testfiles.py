"""Run-time test files from re\\_testdateien\\07 (T2 natural brushes, T5 color alpha, T6 gradient shapes/feathered
edge/contour gradient, T7 BFBi=2) for the #[ignore] test `testfiles` of fp-vector.

Per file: canvas, the paths in drawing order (like vektor/page.py: layers bottom->top, ELMv reversed) with
opacity/blend mode, the flat IDAT (original result) and the result of the Python reference (page.py,
background composited last, like scripts/vektor/eval_page.py).

Usage:   "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_testfiles.py
Output:  tools/golden/fp-vector/out/testfiles.json
"""
import base64
import glob
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "scripts", "vektor"))
sys.path.insert(0, os.path.join(RE, "vektor"))
sys.path.insert(0, os.path.join(RE, "format"))
import fwpng                     # noqa: E402
import page as PG                # noqa: E402
import blend as BL               # noqa: E402
from fwfmt import render, pngio, tree  # noqa: E402

D07 = os.path.join(RE, "_testdateien", "07")
OUT = os.path.join(HERE, "out", "testfiles.json")
os.makedirs(os.path.dirname(OUT), exist_ok=True)
NAMES = sorted(glob.glob(os.path.join(D07, "t2_*.fw.png")) + glob.glob(os.path.join(D07, "t5_*.fw.png")) +
               glob.glob(os.path.join(D07, "t6_*.fw.png")) + glob.glob(os.path.join(D07, "t7_*.fw.png")))


def b64(b):
    return base64.b64encode(bytes(b)).decode("ascii")


def strip(nd):
    j = tree.to_json([nd])[0]["PTHv"]
    j = [c for c in j if "IMGv" not in c]
    for c in j:
        if "PATv" in c:
            for a in c["PATv"]:
                for key in ("TXFv", "TXBv"):
                    if key in a:
                        a[key] = [x for x in a[key] if "MSKv" not in x]
    return j


def mask_of(pth, key, tiles):
    """Texture mask (TXFv = fill, TXBv = stroke) from the tiles of the file; missing tiles = 255."""
    pat = pth.child("PATv")
    tx = pat.child(key) if pat is not None else None
    msk = tx.child("MSKv") if tx is not None else None
    if msk is None:
        return None
    tw, th, ts = msk.get("WPX"), msk.get("HPX"), msk.get("TSZ", 256)
    if not tw or not th:
        return None
    ents = msk.child("TILv").children() if msk.child("TILv") is not None else []
    cols = -(-tw // ts)
    out = bytearray(b"\xff" * (tw * th))
    for k, e in enumerate(ents):
        ty, tx_ = divmod(k, cols)
        if e.tag == "TID" and e.value in tiles:
            px = tiles[e.value].pixels
        elif e.tag == "TMC":
            px = bytes([e.value & 255]) * (ts * ts)
        else:
            continue
        for yy in range(min(ts, th - ty * ts)):
            for xx in range(min(ts, tw - tx_ * ts)):
                out[(ty * ts + yy) * tw + tx_ * ts + xx] = px[yy * ts + xx]
    return {"w": tw, "h": th, "data": b64(out)}


class Capture(PG.VectorPage):
    def __init__(self, *a, **k):
        super().__init__(*a, **k)
        self.captured = []
        self.other = 0

    def _vector(self, e, cv):
        tiles = self.store.tiles
        self.captured.append({"pth": strip(e), "opa": e.get("OPA"), "bld": e.get("BLD"), "fx": bool(render._has_visible_effects(e)),
                              "texture": mask_of(e, "TXFv", tiles), "stroke_texture": mask_of(e, "TXBv", tiles)})
        return super()._vector(e, cv)

    def _element(self, e, cv):
        if e.key not in ("PTHv",) and e.get("VIS") is not False:
            self.other += 1
        return super()._element(e, cv)


out = []
for path in NAMES:
    f = fwpng.FwFile(path, inflate_tiles=True)
    pg = f.doc_roots()[0]
    store = render.TileStore(f.tiles)
    r = Capture(pg, store, use_caches=True)
    cv = r.render(with_background=False)
    bgc = pg.get("BGC", 0)
    a = (bgc >> 24) & 255
    px = cv.px
    if a:
        o2 = bytearray(len(px))
        for k in range(0, len(px), 4):
            o2[k:k + 4] = bytes(((bgc >> 16) & 255, (bgc >> 8) & 255, bgc & 255, a))
            if px[k + 3]:
                BL.over_mmx(o2, k, px[k], px[k + 1], px[k + 2], px[k + 3])
        px = o2
    W, H, img = pngio.decode_image(f.chunks)
    out.append({"name": os.path.basename(path), "w": int(pg.get("WID")), "h": int(pg.get("HIT")), "xlc": pg.get("XLC", 0.0),
                "ylc": pg.get("YLC", 0.0), "bgc": bgc & 0xFFFFFFFF, "elements": r.captured, "other": r.other,
                "missing": sorted(set(r.missing)), "idat": b64(img), "idat_w": W, "idat_h": H, "python": b64(px)})
    print(os.path.basename(path), len(r.captured), "paths", r.other, "other", sorted(set(r.missing)))
json.dump(out, open(OUT, "w"), separators=(",", ":"))
print(json.dumps({"files": len(out), "output": OUT}))
