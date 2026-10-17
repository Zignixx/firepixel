"""Pixel corpus for fp-text: render caches (IMGv) of the corpus point texts like scripts/text/render_check2.py.

Selection like text/check_render.candidates(all_aa=True): single-line ATE point text, one run, solid color,
without stroke/effects/matrix/faux/HSCf/BLSf, cache <= 300,000 px. Writes out/pixel_corpus.jsonl with per text:
file, attrs, runs, story_kern, cache (x, y, w, h, alpha base64) and the result of the Python reference
(exact pixels with point sampling N x N, tone curves from text/glaettung.py, "strong" empirical like
render_check2.LUT_STRONG). Overall rate on stdout (Python report: 91.84 % with rounded Bezier curve).

Usage: python.exe -I -B gen_pixel_corpus.py [max_texts] [--nopy]
"""
import collections
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402,F401
from common import CORPUS, OUT, b64  # noqa: E402

import numpy as np  # noqa: E402

import check_layout  # noqa: E402
import check_render  # noqa: E402
import glaettung  # noqa: E402
import layout  # noqa: E402
import raster  # noqa: E402
from fwpng import FwFile  # noqa: E402
from fwfmt import render  # noqa: E402

LUT_STRONG = check_render.LUT4[2]


def lut_of(a):
    taa = a.get("TAAi", 0)
    if taa == 2:
        return 4, LUT_STRONG
    if taa == 3:
        taa = 1
    return glaettung.lut_for(taa, a.get("TOSi", 8), a.get("TA1i", 192), a.get("TA2i", 64))


def main():
    args = sys.argv[1:]
    maxn = int(args[0]) if args and args[0].isdigit() else 3000
    nopy = "--nopy" in args
    os.makedirs(OUT, exist_ok=True)
    SK = check_layout.load_story_kern()
    C = check_render.candidates(True, None)
    byfile = collections.defaultdict(list)
    for r in C:
        byfile[r["file"]].append(r)
    st = collections.Counter()
    n = 0
    with open(os.path.join(OUT, "pixel_corpus.jsonl"), "w", encoding="utf-8") as fo:
        for fn, recs in sorted(byfile.items()):
            if n >= maxn:
                break
            fw = FwFile(os.path.join(CORPUS, fn))
            store = render.TileStore(fw.tiles)
            want = {(r["attrs"]["LFTf"], r["attrs"]["TOPf"], r["attrs"].get("RNDi")): r for r in recs}
            for _p, node in fw.doc_roots()[0].walk():
                if node.key != "TXTv":
                    continue
                r = want.pop((node.get("LFT"), node.get("TOP"), node.get("RND")), None)
                if r is None or node.child("IMGv") is None:
                    continue
                bm = render.bitmap_from_img(node.child("IMGv"), store)
                if bm is None or bm[2] * bm[3] > 300000:
                    continue
                x, y, w, h, px = bm
                a = r["attrs"]
                sk = SK.get((fn, (a.get("LFTf"), a.get("TOPf"), a.get("RNDi"))))
                rr = dict(r)
                if sk is not None:
                    rr["story_kern"] = sk
                lay = layout.layout_point_text(rr)
                if lay is None:
                    continue
                ref = np.frombuffer(bytes(px), dtype=np.uint8).reshape(h, w, 4)[:, :, 3]
                out = {"file": fn, "attrs": a, "runs": r["runs"], "story_kern": sk,
                       "cache": {"x": x, "y": y, "w": w, "h": h, "alpha": b64(ref.tobytes())}}
                if not nopy:
                    N, lut = lut_of(a)
                    lines = []
                    for ln in lay["lines"]:
                        for g in ln["glyphs"]:
                            f = g["font"]
                            k = g["size"] / f.units_per_em
                            lines += raster.flatten(f.outline(g["gid"]), k, k, g["x"] - x, g["y"] - y, tol=0.05)
                    cnt = raster.sample_counts(lines, w, h, N)
                    our = np.array(lut, dtype=np.int32)[np.minimum(cnt, len(lut) - 1)]
                    d = np.abs(our - ref.astype(np.int32))
                    ex = int((d == 0).sum())
                    out["py_exact"] = ex
                    st["exact"] += ex
                    st["px"] += int(d.size)
                    st["taa%d_exact" % a.get("TAAi", 0)] += ex
                    st["taa%d_px" % a.get("TAAi", 0)] += int(d.size)
                fo.write(json.dumps(out, ensure_ascii=False) + "\n")
                st["texts"] += 1
                n += 1
    if st["px"]:
        st["quote"] = round(st["exact"] / st["px"], 6)
    print(json.dumps(dict(st), indent=1))


if __name__ == "__main__":
    main()
