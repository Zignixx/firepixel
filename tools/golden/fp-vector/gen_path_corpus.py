"""Corpus for the #[ignore] tests of fp-vector (path rendering against the render caches).

Measured like re\\scripts\\vektor\\test_paths.py (invoked there as `test_paths.py 40000 all`):
all paths of root 0 with a render cache, without visible live effects (survey: fx empty), cache area
<= 40,000 pixels. In addition the textured fills from re\\scripts\\vektor\\test_tex.py (files 647, 676,
681, 683, all roots, fill with FTBi, no stroke), there with the real texture tiles.

One JSON line per case:
  {"set": "paths"|"tex", "file": index, "name": file, "cat": [fill, stroke],
   "win": [x0, y0, w, h], "pth": <PTHv as fwfmt.tree.to_json without cache IMGv>,
   "texture": {"w","h","data": base64} | null,   (resolved TXFv mask, 8 bits)
   "pattern": {"w","h","rgba": base64} | null,   (resolved FPTv pattern image)
   "cache": base64 RGBA (w*h*4),
   "py": base64 RGBA | null   (result of vektor/pathrender.py, only if it differs from the cache)}

Usage:   "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_path_corpus.py
Output:  tools/golden/fp-vector/out/path_corpus.jsonl (large, not in the crate).
"""
import base64
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "scripts", "vektor"))
sys.path.insert(0, os.path.join(RE, "vektor"))
sys.path.insert(0, os.path.join(RE, "format"))
import cases            # noqa: E402
import corpus           # noqa: E402
import pathrender as PR  # noqa: E402
from fwfmt import render, tree  # noqa: E402

OUT = os.path.join(HERE, "out", "path_corpus.jsonl")
os.makedirs(os.path.dirname(OUT), exist_ok=True)
MAXPX = 40000


def b64(b):
    return base64.b64encode(bytes(b)).decode("ascii")


def cat(r):
    f = r.get('fill')
    b = r.get('brush')
    fk = (f['cat'] + ('+tex' if f['tb'] else '') + ('+feather' if f['ef'] else '') + ('' if f['et'] else '+hard')) if f else 'nofill'
    bk = b['name'] if b else 'nostroke'
    return [fk, bk]


def texture_of(pth, tiles):
    pat = pth.child("PATv")
    txf = pat.child("TXFv") if pat is not None else None
    msk = txf.child("MSKv") if txf is not None else None
    if msk is None:
        return None
    tw, th, ts = msk.get("WPX"), msk.get("HPX"), msk.get("TSZ", 256)
    if not tw or not th:
        return None
    ents = msk.child("TILv").children() if msk.child("TILv") is not None else []
    cols = -(-tw // ts)
    out = bytearray(b"\xff" * (tw * th))
    for k, e in enumerate(ents):
        ty, tx = divmod(k, cols)
        if e.tag == "TID" and e.value in tiles:
            px = tiles[e.value].pixels
        elif e.tag == "TMC":
            px = bytes([e.value & 255]) * (ts * ts)
        else:
            continue
        for yy in range(min(ts, th - ty * ts)):
            for xx in range(min(ts, tw - tx * ts)):
                out[(ty * ts + yy) * tw + tx * ts + xx] = px[yy * ts + xx]
    return {"w": tw, "h": th, "data": b64(out)}


def pattern_of(pth, store):
    pat = pth.child("PATv")
    fpl = pat.child("FPLv") if pat is not None else None
    fpt = fpl.child("FPTv") if fpl is not None else None
    img = fpt.child("IMGv") if fpt is not None else None
    if img is None:
        return None
    bmp = render.bitmap_from_img(img, store)
    if bmp is None:
        return None
    _, _, w, h, px = bmp
    return {"w": w, "h": h, "rgba": b64(px)}


def strip_tree(nd):
    """PTHv without cache IMGv, without pattern image/texture tiles (resolved separately)."""
    j = tree.to_json([nd])[0]["PTHv"]
    j = [c for c in j if "IMGv" not in c]
    for c in j:
        if "PATv" in c:
            for a in c["PATv"]:
                for key in ("TXFv", "TXBv"):
                    if key in a:
                        a[key] = [x for x in a[key] if "MSKv" not in x]
                if "FPLv" in a:
                    for x in a["FPLv"]:
                        if "FPTv" in x:
                            x["FPTv"] = [y for y in x["FPTv"] if "IMGv" not in y]
    return j


def same(a, b):
    for k in range(0, len(a), 4):
        if a[k + 3] == 0 and b[k + 3] == 0:
            continue
        if a[k:k + 4] != b[k:k + 4]:
            return False
    return True


def emit(fh, setname, fi, name, catv, nd, bmp, f, use_tiles):
    x0, y0, w, h, px = bmp
    store = render.TileStore(f.tiles)
    PR.TILES = f.tiles if use_tiles else {}
    out, missing = PR.render_path(nd, int(x0), int(y0), w, h)
    rec = {"set": setname, "file": fi, "name": name, "cat": catv, "win": [int(x0), int(y0), w, h],
           "pth": strip_tree(nd),
           "texture": texture_of(nd, f.tiles) if use_tiles else None,
           "pattern": pattern_of(nd, store), "cache": b64(px),
           "py": None if same(out, px) else b64(out), "missing": missing}
    fh.write(json.dumps(rec, separators=(",", ":")) + "\n")
    return same(out, px)


def main():
    n = ex = 0
    paths = corpus.corpus()
    with open(OUT, "w", encoding="utf-8") as fh:
        for r, nd, bmp, f in cases.iter_cases(lambda r: not r['fx']):
            x0, y0, w, h, px = bmp
            if w * h > MAXPX:
                continue
            ok = emit(fh, "paths", r["file"], os.path.basename(paths[r["file"]]), cat(r), nd, bmp, f, False)
            n += 1
            ex += ok
        nt = ext = 0
        for fi in (647, 676, 681, 683):
            f = corpus.load(paths[fi])
            store = render.TileStore(f.tiles)
            for root in f.doc_roots():
                for _, nd in root.walk():
                    if nd.key != 'PTHv' or nd.child('IMGv') is None or render._has_visible_effects(nd):
                        continue
                    pat = nd.child('PATv')
                    fpl = pat.child('FPLv') if pat is not None else None
                    if fpl is None or not fpl.get('FTB') or pat.child('BPLv') is not None:
                        continue
                    bmp = render.bitmap_from_img(nd.child('IMGv'), store)
                    if bmp is None:
                        continue
                    ok = emit(fh, "tex", fi, os.path.basename(paths[fi]), ["tex", "nostroke"], nd, bmp, f, True)
                    nt += 1
                    ext += ok
    print(json.dumps({"paths": n, "python_exact": ex, "tex": nt, "tex_exact": ext, "output": OUT}))


if __name__ == "__main__":
    main()
