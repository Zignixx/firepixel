"""Corpus contours for the #[ignore] test `pcl_corpus` of fp-geom (flattening of the original vs. PCLv).

Like re\\scripts\\pfade\\test_pcl.py: all PTHv with PBLv and PCLv from the FW-PNGs of the installation copy
(..\\korpus or FP_CORPUS_DIR, read only). One JSON line per contour:
  {"f": file, "closed": bool, "nodes": [[ax,ay,ix,iy,ox,oy], ...], "counts": [...], "pcl": [[x,y], ...],
   "py": [[x,y], ...]}       (py = flatten.py result, only if it differs from pcl)

Usage:   "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_pcl_corpus.py
Output:  tools/golden/fp-geom/out/pcl_corpus.jsonl (large, not part of the test data in the crate).
"""
import glob
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "format"))
sys.path.insert(0, os.path.join(RE, "pfade"))
from fwpng import FwFile      # noqa: E402
import flatten as FL          # noqa: E402

ROOT = os.environ.get("FP_CORPUS_DIR") or os.path.normpath(os.path.join(RE, "..", "korpus"))
OUT = os.path.join(HERE, "out", "pcl_corpus.jsonl")
os.makedirs(os.path.dirname(OUT), exist_ok=True)


def norm(pts, closed):
    pts = list(pts)
    while closed and len(pts) > 1 and math.dist(pts[0], pts[-1]) < 1e-4:
        pts.pop()
    return pts


def same(a, b, tol=1e-3):
    return len(a) == len(b) and all(math.dist(p, q) <= tol for p, q in zip(a, b))


files = sorted(glob.glob(os.path.join(ROOT, "**", "*.png"), recursive=True))
n_files = n_cont = n_same = 0
with open(OUT, "w", encoding="utf-8") as fh:
    for path in files:
        try:
            f = FwFile(path, inflate_tiles=False)
            roots = f.doc_roots()
        except Exception:
            continue
        n_files += 1
        rel = os.path.relpath(path, ROOT)
        for r in roots:
            for _, n in r.walk():
                if not (n.type == "v" and n.key == "PTHv"):
                    continue
                pbl, pcl = n.child("PBLv"), n.child("PCLv")
                if pbl is None or pcl is None:
                    continue
                pbps, ppls = pbl.children("PBPv"), pcl.children("PPLv")
                if len(pbps) != len(ppls):
                    continue
                for pbp, ppl in zip(pbps, ppls):
                    closed = bool(pbp.get("ISC"))
                    nodes, counts = [], []
                    for p in pbp.children("PBTv"):
                        x, y = p.get("XLC"), p.get("YLC")
                        nodes.append(((x, y), (p.get("XPC", x), p.get("YPC", y)), (p.get("XSC", x), p.get("YSC", y))))
                        bz = p.child("BZLv")
                        counts.append(bz.get("BZC", 1) if bz is not None else 1)
                    if len(nodes) < 2:
                        continue
                    ref = [(q.get("XLC"), q.get("YLC")) for q in ppl.children("PPTv")]
                    got = FL.flatten_contour(nodes, closed, counts)
                    rec = {"f": rel, "closed": closed,
                           "nodes": [[a[0], a[1], i[0], i[1], o[0], o[1]] for a, i, o in nodes],
                           "counts": counts, "pcl": [list(q) for q in ref]}
                    ok = same(norm(got, closed), norm(ref, closed))
                    n_cont += 1
                    n_same += ok
                    if [tuple(map(float, q)) for q in got] != [tuple(map(float, q)) for q in ref]:
                        rec["py"] = [list(q) for q in got]
                    fh.write(json.dumps(rec, separators=(",", ":")) + "\n")
print(json.dumps({"files": n_files, "contours": n_cont, "python_equals_pcl": n_same, "output": OUT}))
