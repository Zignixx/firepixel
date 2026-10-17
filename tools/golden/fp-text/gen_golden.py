"""Small golden data for the regular fp-text tests (crates/fp-text/tests/data/*.json).

  golden_luts.json    tone curves: text/glaettung.custom_lut for N in {4, 8, 16} and many (sharpness, strength)
  golden_fonts.json   metrics of the corpus fonts: upm, asc (layout.ascent), desc, advances, kerning pairs, ink
  golden_layout.json  point text layout (layout.layout_point_text): synthetic cases + sample of the corpus,
                      frame and glyphs (gid, x, y)
  golden_raster.json  point sampling (raster.sample_counts) for small texts, N in {1, 4, 8, 16}

The case names and sample texts in synthetic() are data (stored in golden_layout.json) and stay as they are.

Usage: python.exe -I -B gen_golden.py   (needs out/layout_corpus.jsonl from gen_layout_corpus.py)
"""
import json
import os
import random
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402,F401
from common import OUT  # noqa: E402

import fonts  # noqa: E402
import glaettung  # noqa: E402
import layout  # noqa: E402
import raster  # noqa: E402

DATA = os.path.normpath(os.path.join(common.HERE, "..", "..", "..", "crates", "fp-text", "tests", "data"))

FONTS = ["MyriadPro-Regular", "MyriadPro-Bold", "MyriadPro-Semibold", "ArialMT", "Arial-BoldMT", "Arial-Black",
         "TimesNewRomanPSMT", "Verdana", "CourierNewPSMT", "Georgia", "Tahoma"]
SAMPLE = "Hamburg Tennessee AVTo Wa yj fäß€1"
PAIRS = ["AV", "To", "Te", "Wa", "LT", "Yo", "av", "r.", "T,", "Pa", "ey", "ff", "VA", "LY"]


def luts():
    vals = [0, 1, 63, 64, 127, 128, 140, 192, 230, 254, 255]
    out = []
    for n in (4, 8, 16):
        for s in vals:
            for w in vals:
                out.append({"n": n, "s": s, "w": w, "lut": glaettung.custom_lut(n, s, w)})
    return out


def font_metrics():
    out = []
    for name in FONTS:
        f = fonts.lookup(name)
        if f is None:
            continue
        glyphs = []
        for ch in SAMPLE:
            g = f.glyph(ch)
            ink = layout.ink_x(f, g)
            glyphs.append({"ch": ch, "gid": g, "adv": f.advance(g), "ink": list(ink) if ink else None})
        kern = []
        for p in PAIRS:
            g1, g2 = f.glyph(p[0]), f.glyph(p[1])
            kern.append({"pair": p, "kern": f.kerning(g1, g2)})
        out.append({"ps": name, "upm": f.units_per_em, "asc": layout.ascent(f), "desc": layout.descent(f),
                    "family": f.family, "glyphs": glyphs, "kern": kern})
    return out


def run(face, size, text, **kw):
    r = {"FONs": face, "PTSf": size, "BOLb": False, "ITLb": False, "UNDb": False, "HSCf": 1.0, "RKNf": 0.0,
         "BLSf": 0.0, "LEDf": 1.0, "LDMi": 0, "JSTi": 0, "FCLi": 4278190080, "PINf": 0.0, "PSBf": 0.0, "PSAf": 0.0,
         "TRNs": text}
    r.update(kw)
    return r


def rec(runs, x=100.0, y=200.0, j=0, atk=True, story=None, name=""):
    a = {"TFTi": 0, "LFAf": x, "TOAf": y, "RIAf": 0.0, "BOAf": 0.0, "JSTi": j, "ATKb": atk,
         "PTSf": runs[0].get("PTSf", 12.0), "CFTs": runs[0].get("FONs")}
    for r in runs:
        r["JSTi"] = j
    out = {"name": name, "attrs": a, "runs": runs}
    if story is not None:
        out["story_kern"] = story
    return out


def synthetic():
    M, A, T = "MyriadPro-Regular", "ArialMT", "TimesNewRomanPSMT"
    cases = [
        rec([run(M, 20.0, "Hallo Welt")], name="einfach"),
        rec([run(A, 13.0, "Tennessee AV To")], j=1, name="zentriert"),
        rec([run(A, 13.0, "Tennessee AV To")], j=2, name="rechts"),
        rec([run(M, 18.0, "Laufweite", RKNf=0.5)], j=1, name="Laufweite positiv zentriert"),
        rec([run(M, 18.0, "Laufweite", RKNf=-0.3)], j=2, name="Laufweite negativ rechts"),
        rec([run(A, 24.0, "Fett jf", BOLb=True)], name="Faux-Fett"),
        rec([run(A, 30.0, "Hamburg", HSCf=0.75)], name="HSCf"),
        rec([run(M, 16.0, "Zeile1\rZeile2\x03Zeile3\rZ4", LEDf=1.5, PSAf=4.0, PSBf=6.0)], name="mehrzeilig"),
        rec([run(M, 16.0, "exakt\rZeile", LEDf=30.0, LDMi=1)], name="Zeilenabstand exakt"),
        rec([run(M, 20.0, "Basis"), run(M, 12.0, "hoch", BLSf=5.0), run(M, 20.0, " tief", BLSf=-3.0)], name="Grundlinienversatz"),
        rec([run(M, 20.0, "T"), run(M, 20.0, "ennessee")], name="Kerning ueber Laufgrenze"),
        rec([run(M, 20.0, "AV"), run(A, 20.0, "AV"), run(M, 14.0, "To")], name="gemischte Schriften"),
        rec([run(A, 20.0, "AVATAR Tower")], atk=False, name="Kerning aus"),
        rec([run(A, 20.0, "AVATAR Tower")], atk=False, story=[[3, 1], [9, 0]], name="Story-Kerning"),
        rec([run(T, 22.0, "Times jfy\r\rleer davor")], j=1, name="Leerzeile"),
        rec([run(M, 20.0, "\rbeginnt leer\r")], name="Umbruch am Anfang und Ende"),
        rec([run(M, 7.5, "klein 7,5 px", RKNf=0.25)], x=10.5, y=20.25, name="Bruchteil-Anker"),
        rec([run(A, 186.7, "Gross")], name="gross"),
    ]
    return cases


def layout_cases():
    out = []
    for c in synthetic():
        res = layout.layout_point_text(c)
        if res is None:
            continue
        c["py_frame"] = [res["frame"][k] for k in ("LFT", "TOP", "RIT", "BOT")]
        c["py_glyphs"] = [[g["gid"], g["x"], g["y"]] for ln in res["lines"] for g in ln["glyphs"]]
        out.append(c)
    recs = [json.loads(line) for line in open(os.path.join(OUT, "layout_corpus.jsonl"), encoding="utf-8")]
    recs = [r for r in recs if r.get("py_frame")]
    random.seed(11102026)
    for r in random.sample(recs, min(250, len(recs))):
        out.append({"name": r["file"], "attrs": r["attrs"], "runs": r["runs"], "story_kern": r["story_kern"],
                    "py_frame": r["py_frame"], "py_glyphs": r["py_glyphs"]})
    return out


def raster_cases():
    out = []
    texts = [("MyriadPro-Regular", 20.0, "Hg", 4), ("ArialMT", 13.0, "Tennessee", 4), ("ArialMT", 40.0, "Hg", 8),
             ("ArialMT", 40.0, "Hg", 16), ("TimesNewRomanPSMT", 17.0, "@ffi", 1), ("MyriadPro-Bold", 11.0, "a.b,c", 4),
             ("Verdana", 15.5, "Wolke 7", 8)]
    for i, (face, size, text, n) in enumerate(texts):
        c = rec([run(face, size, text, RKNf=0.13 if i % 2 else 0.0)], x=10.0 + 0.5 * (i % 3), y=40.0, j=i % 3)
        res = layout.layout_point_text(c)
        if res is None:
            continue
        fr = res["frame"]
        x0, y0 = int(fr["LFT"]) - 1, int(fr["TOP"]) - 1
        w, h = int(fr["RIT"] - fr["LFT"]) + 2, int(fr["BOT"] - fr["TOP"]) + 2
        lines = []
        for ln in res["lines"]:
            for g in ln["glyphs"]:
                f = g["font"]
                k = g["size"] / f.units_per_em
                lines += raster.flatten(f.outline(g["gid"]), k, k, g["x"] - x0, g["y"] - y0, tol=0.05)
        cnt = raster.sample_counts(lines, w, h, n)
        c["raster"] = {"x": x0, "y": y0, "w": w, "h": h, "n": n, "counts": [int(v) for v in cnt.flatten()]}
        out.append(c)
    return out


def main():
    os.makedirs(DATA, exist_ok=True)
    for name, data in (("golden_luts.json", luts()), ("golden_fonts.json", font_metrics()),
                       ("golden_layout.json", layout_cases()), ("golden_raster.json", raster_cases())):
        with open(os.path.join(DATA, name), "w", encoding="utf-8") as fo:
            json.dump(data, fo, ensure_ascii=False, separators=(",", ":"))
        print(name, len(data))


if __name__ == "__main__":
    main()
