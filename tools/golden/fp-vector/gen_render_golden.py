"""Golden data for the path rendering of fp-vector: synthetic paths (own geometry, no corpus data),
rendered with the Python reference re\\vektor\\pathrender.py.

Output: crates/fp-vector/tests/data/render_golden.json
  [{"name", "win": [x0, y0, w, h], "pth": <PTHv in fwfmt.tree.to_json form>, "rgba": base64}]

Usage:   "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_render_golden.py
"""
import base64
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
sys.path.insert(0, os.path.join(RE, "vektor"))
sys.path.insert(0, os.path.join(RE, "format"))
import pathrender as PR          # noqa: E402
from fwfmt import tree           # noqa: E402

OUT = os.path.join(FP, "crates", "fp-vector", "tests", "data", "render_golden.json")


def pcl(contours):
    """contours: [(points, closed)] -> PCLv list."""
    out = []
    for pts, closed in contours:
        ppl = [{"ISCb": bool(closed)}, {"PPCi": len(pts)}]
        for x, y in pts:
            ppl.append({"PPTv": [{"XLCf": float(x)}, {"YLCf": float(y)}, {"PRSf": 1.0}, {"VELf": 0.0}]})
        out.append({"PPLv": ppl})
    return {"PCLv": out}


def brush(**kw):
    b = dict(CATs="bc_Basic", BDIi=1, BANi=0, BASi=1000, BMMi=14, BMSi=10, BSEi=0, BSFi=1, BSHi=1, BCNi=1000, BEFi=0,
             BRTi=1, BFBi=1, BFRi=0, BNTi=1, BSPi=150, BTBi=0, BTEi=0, BTSi=0, BSMi=0, BCMi=0, BIAb=1, NDIi=0,
             DO1i=8, DO2i=1, DO3i=1, DF1i=2, DF2i=2, DF3i=2)
    b.update(kw)
    return {"BPLv": [{k: v} for k, v in b.items()]}


def stops(lst):
    return [{"FGIv": [{"FGPf": p}, {"FGCi": c}]} for p, c in lst]


def fill(sh=0, fet=1, fef=0, sx=None, grad=None, ostops=None, extra=None):
    f = [{"FSHi": sh}, {"FETi": fet}, {"FEFi": fef}, {"FSMi": 2}]
    if sx is not None:
        f.append({"FSXi": sx})
    if grad:
        fgy = [{"FG0v": stops(grad)}]
        if ostops:
            fgy.append({"FG1v": stops(ostops)})
        f.append({"FGLv": [{"FGYv": fgy}]})
    if extra:
        f += [{k: v} for k, v in extra.items()]
    return {"FPLv": f}


def path(contours, fill_=None, stroke=None, fcl=0xFF3366CC, bcl=0xFF202020, eof=True, agm=False, brp=1, ptt=0, fot=False,
         handles=((0, 0), (0, 0), (0, 0))):
    pat = [{"FCLi": fcl & 0xFFFFFFFF}, {"BCLi": bcl & 0xFFFFFFFF}]
    if fill_:
        pat.append(fill_)
    if stroke:
        pat.append(stroke)
    (psx, psy), (pex, pey), (pfx, pfy) = handles
    j = [{"PATv": pat}, {"BRPi": brp}, {"PTTi": ptt}, {"FOTb": fot}, {"PSXf": float(psx)}, {"PSYf": float(psy)},
         {"PEXf": float(pex)}, {"PEYf": float(pey)}, {"PFXf": float(pfx)}, {"PFYf": float(pfy)}, pcl(contours),
         {"EOFb": eof}, {"AGMb": agm}]
    return j


def circle(cx, cy, r, n=48):
    return [(cx + r * math.cos(2 * math.pi * i / n), cy + r * math.sin(2 * math.pi * i / n)) for i in range(n)]


cases = []


def add(name, j, win):
    nd = tree.from_json([{"PTHv": j}])[0]
    x0, y0, w, h = win
    rgba, missing = PR.render_path(nd, x0, y0, w, h)
    if missing:
        print("missing", name, missing)
    cases.append({"name": name, "win": list(win), "pth": j, "rgba": base64.b64encode(bytes(rgba)).decode("ascii")})


tri = [([(3.3, 2.7), (30.6, 8.2), (12.4, 33.9)], True)]
star = [([(20 + 16 * math.cos(math.pi / 2 + 4 * math.pi * i / 5), 20 - 16 * math.sin(math.pi / 2 + 4 * math.pi * i / 5)) for i in range(5)], True)]
sq_hole = [([(2.25, 2.25), (34.5, 2.25), (34.5, 34.75), (2.25, 34.75)], True), ([(10.1, 10.1), (25.3, 11.0), (24.6, 26.2), (11.2, 25.7)], True)]
W = (0, 0, 40, 40)
for nm, c in (("tri", tri), ("star", star), ("hole", sq_hole)):
    add(nm + "_xor", path(c, fill()), W)
    add(nm + "_winding", path(c, fill(), eof=False), W)
    add(nm + "_agm", path(c, fill(), agm=True), W)
    add(nm + "_hard", path(c, fill(fet=0)), W)
    add(nm + "_hard_agm", path(c, fill(fet=0), agm=True), W)
add("circle_feather3", path([(circle(20, 20, 11.3), True)], fill(fef=3)), W)
add("circle_feather7_winding", path([(circle(20, 19, 9.7), True)], fill(fef=7), eof=False), W)
add("color_alpha", path(tri, fill(), fcl=0x80FF0000), W)

g3 = [(0.0, 0xFFFF0000), (0.45, 0xFF00FF00), (1.0, 0xFF0000FF)]
go = [(0.0, 0xFF000000), (0.7, 0x40000000), (1.0, 0xC0000000)]
rect = [([(1, 1), (39, 1), (39, 33), (1, 33)], True)]
H = ((6.5, 8.25), (31.75, 20.5), (12.0, 29.0))
for sh in (2, 3, 4, 5, 7, 8, 9, 10, 12, 13, 14):
    add("grad_%d" % sh, path(rect, fill(sh=sh, grad=g3), handles=H), (0, 0, 40, 34))
    add("grad_%d_opacity" % sh, path(rect, fill(sh=sh, grad=g3, ostops=go), handles=H), (0, 0, 40, 34))
add("grad_linear_hard", path(tri, fill(sh=2, fet=0, grad=g3), handles=H), W)
add("grad_equal_stops", path(rect, fill(sh=2, grad=[(0.0, 0xFF101010), (0.5, 0xFF808080), (0.5, 0xFFF0F000), (1.0, 0xFF00FFFF)]), handles=H), (0, 0, 40, 34))
add("web_dither", path(rect, fill(sh=16, extra={"FD1i": 0xFFFF0000, "FD2i": 0xFF00FF00, "FD3i": 0xFF0000FF, "FD4i": 0xFFFFFFFF}),
                       handles=((3.5, 2.5), (10, 10), (0, 10))), (0, 0, 40, 34))

line = [([(4.3, 6.7), (35.2, 9.1), (20.6, 31.8)], False)]
closed = [([(6.5, 6.5), (33.25, 7.0), (31.0, 32.5), (8.0, 30.0)], True)]
curve = [([(4 + i * 0.8, 20 + 12 * math.sin(i * 0.2)) for i in range(40)], False)]
presets = {
    "pencil1_aa": brush(CATs="bc_Pencil", BDIi=1, BIAb=1),
    "pencil1_hard": brush(CATs="bc_Pencil", BDIi=1, BIAb=0, BFBi=0),
    "soft_rounded4": brush(BDIi=4, BSEi=500, BSPi=60),
    "hard_rounded3": brush(BDIi=3, BSEi=0, BSPi=15),
    "square5_hard": brush(BDIi=5, BSHi=0, BIAb=0, BFBi=0),
    "square4_aa_soft": brush(BDIi=4, BSHi=0, BSEi=300),
    "linear_soft6": brush(BDIi=6, BSEi=700, BSFi=0, BCNi=600),
    "dashed": brush(BDIi=2, NDIi=2, DO1i=5, DF1i=3, DO2i=2, DF2i=6),
    "dashed_hard": brush(BDIi=1, BIAb=0, NDIi=1, DO1i=4, DF1i=4),
    "bfb2": brush(BDIi=3, BFBi=2),
}
for nm, b in presets.items():
    add("stroke_line_" + nm, path(line, None, b), W)
    add("stroke_curve_" + nm, path(curve, None, b), W)
for brp in (0, 2):
    for nm in ("pencil1_aa", "pencil1_hard", "soft_rounded4", "square5_hard"):
        add("stroke_closed_brp%d_%s" % (brp, nm), path(closed, fill(), presets[nm], brp=brp), W)
add("stroke_ptt_inside", path(closed, None, presets["pencil1_aa"], ptt=1), W)
add("stroke_ptt_inside_hard", path(closed, None, presets["pencil1_hard"], ptt=1, agm=True), W)
add("stroke_open_brp0_is_center", path(line, None, presets["soft_rounded4"], brp=0), W)
add("fill_over_stroke", path(closed, fill(), presets["soft_rounded4"], fot=True), W)
add("stroke_color_alpha", path(closed, fill(), presets["soft_rounded4"], bcl=0x80FF00FF), W)
add("stroke_brp2_winding", path(sq_hole, fill(), presets["hard_rounded3"], brp=2, eof=False), W)

os.makedirs(os.path.dirname(OUT), exist_ok=True)
json.dump(cases, open(OUT, "w"), separators=(",", ":"))
print(json.dumps({"cases": len(cases), "output": OUT}))
