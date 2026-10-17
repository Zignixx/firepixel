"""Golden-Daten fuer die Pfadoperationen von fp-vector (Referenz re\\pfade\\: boolops, compound, offset,
simplify, shapes).

Ausgaben:
  crates/fp-vector/tests/data/pathops_golden.json   synthetische Faelle (normale Tests)
  tools/golden/fp-vector/out/compound_corpus.json    21 verschiedene Compound Shapes des Korpus
                                                     (wie re\\scripts\\pfade\\test_compound.py) plus die 4
                                                     veralteten iPad-Sketch-Faelle mit zurueckgeschobenem
                                                     Operanden (test_stale_shift.py) und die T16-Laufzeitdateien
Kontur-Format: {"closed": bool, "nodes": [[ax, ay, ix, iy, ox, oy], ...]}

Aufruf:  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_pathops.py
"""
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "format"))
sys.path.insert(0, os.path.join(RE, "pfade"))
from fwpng import FwFile           # noqa: E402
import bezier as BZ                # noqa: E402
import boolops as BO               # noqa: E402
import compound as CP              # noqa: E402
import offset as OF                # noqa: E402
import shapes as SH                # noqa: E402
import simplify as SI              # noqa: E402

OUT_GOLDEN = os.path.join(FP, "crates", "fp-vector", "tests", "data", "pathops_golden.json")
OUT_CORPUS = os.path.join(HERE, "out", "compound_corpus.json")
os.makedirs(os.path.dirname(OUT_GOLDEN), exist_ok=True)
os.makedirs(os.path.dirname(OUT_CORPUS), exist_ok=True)


def cj(c):
    return {"closed": bool(c.closed), "nodes": [[a[0], a[1], i[0], i[1], o[0], o[1]] for a, i, o in c.nodes]}


def cjs(cs):
    return [cj(c) for c in cs]


def poly(pts, closed=True):
    return BZ.Contour([(p, p, p) for p in pts], closed)


def sq(x0, y0, x1, y1):
    return poly([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])


# ------------------------------------------------------------------ synthetische Faelle
bool_cases = []


def add_bool(name, a, eo_a, b, eo_b):
    A, B = BO.Region(a, eo_a), BO.Region(b, eo_b)
    rec = {"name": name, "a": cjs(a), "eo_a": eo_a, "b": cjs(b), "eo_b": eo_b, "res": {}}
    for op in ("union", "intersect", "subtract", "xor"):
        rec["res"][op] = cjs(BO.boolean(A, B, op))
    bool_cases.append(rec)


add_bool("squares", [sq(0, 0, 10, 10)], True, [sq(5, 5, 15, 15)], True)
add_bool("shared_edge", [sq(0, 0, 10, 10)], True, [sq(10, 0, 20, 10)], True)
add_bool("nested_eo", [sq(0, 0, 10, 10), sq(3, 3, 7, 7)], True, [sq(5, -2, 12, 12)], True)
add_bool("nested_nz", [sq(0, 0, 10, 10), sq(3, 3, 7, 7)], False, [sq(5, -2, 12, 12)], True)
add_bool("lens", [SH.ellipse(0, 0, 10, 10)], True, [SH.ellipse(12, 0, 10, 10)], True)
add_bool("ellipse_rect", [SH.ellipse(100, 70.275, 20, 20)], True, [sq(80.275, 70, 140, 120)], True)
add_bool("star_rect", [SH.star(50, 50, 40, 15, 5)], True, [sq(30, 30, 90, 70)], True)
add_bool("touch_hole", [sq(50, 50, 150, 150)], True, [poly([(100, 50), (130, 80), (100, 110), (70, 80)])], True)
add_bool("curve_line", [BZ.Contour([((0, 0), (0, 0), (20, -10)), ((40, 0), (60, 10), (40, 0)), ((40, 20), (40, 20), (40, 20))], True)], True,
         [sq(10, -5, 30, 25)], True)
add_bool("rounded_frac", [SH.rounded_rect(0.3, 0.7, 30.6, 20.2, 5)], True, [SH.ellipse(28.1, 18.9, 7.3, 4.1)], True)
add_bool("open_contour", [poly([(0, 0), (20, 0), (20, 20)], closed=False)], True, [sq(5, 5, 25, 15)], True)

expand_cases = []
for cap in (0, 1, 2):
    c = poly([(40, 100), (160, 100)], closed=False)
    expand_cases.append({"name": "line_cap%d" % cap, "src": cjs([c]), "width": 10, "join": 2, "miter": 10, "cap": cap,
                         "res": cjs(OF.expand_stroke([c], 10, OF.MITER, 10, cap))})
for join in (0, 1, 2):
    c = sq(40, 130, 160, 170)
    expand_cases.append({"name": "rect_join%d" % join, "src": cjs([c]), "width": 10, "join": join, "miter": 10, "cap": 0,
                         "res": cjs(OF.expand_stroke([c], 10, join, 10, 0))})
c = SH.ellipse(50, 50, 30, 30)
expand_cases.append({"name": "circle", "src": cjs([c]), "width": 6, "join": 1, "miter": 4, "cap": 0, "res": cjs(OF.expand_stroke([c], 6, 1, 4, 0))})
c = poly([(0, 0), (30, 20), (60, 0)], closed=False)
expand_cases.append({"name": "zigzag_round", "src": cjs([c]), "width": 8, "join": 1, "miter": 4, "cap": 2,
                     "res": cjs(OF.expand_stroke([c], 8, 1, 4, 2))})

inset_cases = []
for name, cs, d, join, exp in (("square_shrink", [sq(0, 0, 40, 40)], 5, 2, False), ("square_grow_round", [sq(0, 0, 40, 40)], 5, 1, True),
                               ("circle_shrink", [SH.ellipse(50, 50, 30, 30)], 4, 2, False), ("vanish", [sq(0, 0, 20, 20)], 15, 2, False)):
    inset_cases.append({"name": name, "src": cjs(cs), "d": d, "join": join, "miter": 4, "expand": exp,
                        "res": cjs(OF.inset_path(cs, d, join, 4, exp))})

simplify_cases = []
circ = poly([(50 + 30 * math.cos(i * 2 * math.pi / 72), 50 + 30 * math.sin(i * 2 * math.pi / 72)) for i in range(72)])
for amt in (1, 2, 5):
    simplify_cases.append({"name": "circle72_%d" % amt, "src": cjs([circ]), "amount": amt, "res": cjs(SI.simplify([circ], amt))})
simplify_cases.append({"name": "rect", "src": cjs([sq(0, 0, 10, 5)]), "amount": 10, "res": cjs(SI.simplify([sq(0, 0, 10, 5)], 10))})
ell = SH.ellipse(0, 0, 40, 20)
ell8 = BZ.insert_node(BZ.insert_node(BZ.insert_node(BZ.insert_node(ell, 0, 0.5), 2, 0.5), 4, 0.5), 6, 0.5)
simplify_cases.append({"name": "ellipse8", "src": cjs([ell8]), "amount": 8, "res": cjs(SI.simplify([ell8], 8))})
sine = poly([(i * 2.5, 10 * math.sin(i * 0.3) + (0.4 if i % 3 == 0 else -0.3)) for i in range(40)], closed=False)
simplify_cases.append({"name": "sine40", "src": cjs([sine]), "amount": 2, "res": cjs(SI.simplify([sine], 2))})

shape_cases = {"kappa": SH.KAPPA, "ellipse": cj(SH.ellipse(10, 20, 30, 15)), "rounded": cj(SH.rounded_rect(1, 2, 41, 22, 6)),
               # polygon_tool(star=True) ist in der Referenz nicht aufrufbar (Parameter `star` verdeckt die Funktion)
               "star_auto": cj(SH.star(0, 0, math.dist((0, 0), (10, 5)), SH.star_inner_radius_automatic(math.dist((0, 0), (10, 5)), 5), 5,
                                       math.degrees(math.atan2(5, 10)))),
               "polygon7": cj(SH.polygon_tool((3, 4), (3, -6), 7))}

json.dump({"boolean": bool_cases, "expand": expand_cases, "inset": inset_cases, "simplify": simplify_cases, "shapes": shape_cases},
          open(OUT_GOLDEN, "w"), separators=(",", ":"))

# ------------------------------------------------------------------ Compound-Korpus
files = sorted({o["file"] for o in json.load(open(os.path.join(RE, "pfade", "_scratch", "csg_scan.json")))})
stale = {tuple(r["bbox"]): r for r in json.load(open(os.path.join(RE, "pfade", "_scratch", "stale_shift.json"))) if r.get("diff_vorher")}
cases = []
seen = set()


def anchors(cs):
    return [nd[0] for c in cs for nd in c.nodes]


for path in files:
    f = FwFile(path, inflate_tiles=False)
    for r in f.doc_roots():
        for p, n in r.walk():
            if n.type != "v" or n.key != "CSGv":
                continue
            res = n.child("PTHv")
            elm = n.child("ELMv").children()
            sig = repr(anchors(BZ.from_pth(res))) + repr([anchors(BZ.from_pth(k)) for k in elm])
            if sig in seen:
                continue
            seen.add(sig)
            ops = [{"contours": cjs(BZ.from_pth(k)), "eof": bool(k.get("EOF", True)), "pop": (k.get("POP") or 0) & 0xFFFFFFFF, "key": k.key}
                   for k in elm]
            py = CP.evaluate(n)
            b = BZ.bounds(BZ.from_pth(res))
            rec = {"file": os.path.basename(path), "operands": ops, "stored": cjs(BZ.from_pth(res)), "stored_eof": bool(res.get("EOF", True)),
                   "python": cjs(py), "bbox": [round(v, 2) for v in b]}
            st = stale.get(tuple(round(v, 2) for v in b))
            if st:
                rec["stale_shift"] = {"operand_from_bottom": st["operand_index_von_unten"], "delta": st["versatz"]}
            cases.append(rec)

# T16-Laufzeitdateien
D16 = os.path.join(RE, "_testdateien", "16")


def pths(fn):
    f = FwFile(os.path.join(D16, fn), inflate_tiles=False)
    return [n for _, n in f.doc_roots()[0].walk() if n.key == "PTHv"]


t16 = {}
vor, nach = pths("T16_expand_attr_vor.fw.png"), pths("T16_expand_attr_nach.fw.png")
t16["expand"] = [{"src": cjs(BZ.from_pth(v)), "fw": cjs(BZ.from_pth(nn))} for v, nn in zip(vor, nach)]
vor, nach = pths("T16_union_attr_vor.fw.png"), pths("T16_union_attr_nach.fw.png")
t16["union"] = {"a": cjs(BZ.from_pth(vor[0])), "b": cjs(BZ.from_pth(vor[1])), "fw": cjs(BZ.from_pth(nach[0]))}
vor, nach = pths("T16_touch_hole_vor.fw.png"), pths("T16_touch_hole_nach.fw.png")
t16["touch_hole"] = {"punch": cjs(BZ.from_pth(vor[0])), "target": cjs(BZ.from_pth(vor[1])), "fw": [cjs(BZ.from_pth(nn)) for nn in nach]}
for name in ("a", "b", "c", "gui_b", "gui_c"):
    fn = "T16_join_ends_%s.fw.png" % name
    if os.path.exists(os.path.join(D16, fn)):
        t16["join_" + name] = [cjs(BZ.from_pth(nn)) for nn in pths(fn)]
t16["join_vor"] = [cjs(BZ.from_pth(nn)) for nn in pths("T16_join_ends_vor.fw.png")]

json.dump({"compound": cases, "t16": t16}, open(OUT_CORPUS, "w"), separators=(",", ":"))
print(json.dumps({"bool": len(bool_cases), "expand": len(expand_cases), "inset": len(inset_cases), "simplify": len(simplify_cases),
                  "compound": len(cases), "stale": sum(1 for c in cases if "stale_shift" in c)}))
