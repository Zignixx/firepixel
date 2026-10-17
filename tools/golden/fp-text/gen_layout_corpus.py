"""Golden data for fp-text: corpus text frames with the Python reference (re/text/layout.py).

Reads (read only) re/text/_scratch/texts.jsonl (scripts/text/extract_text.py) and
re/text/_scratch/story_attrs.jsonl (scripts/text/story_attrs.py), computes the layout of every text with
layout.layout_point_text (mode round_each, with ink, kerning kind of the story) and writes one JSON line
per text to out/layout_corpus.jsonl:
  file, attrs, runs, story_kern, category (filter like check_layout.py), py_frame, py_glyphs [[gid, x, y], ...]
Also prints the count like check_layout.py on stdout (target: all four edges 3,998/4,147).
The category values are data (crates/fp-text/tests/corpus.rs matches "Tabulator") and stay as they are.

Usage (from any folder):
  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_layout_corpus.py
"""
import collections
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RE = r"C:\Users\Administrator\Desktop\REA\re"
sys.path.insert(0, os.path.join(RE, "text"))
import layout  # noqa: E402

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import neutral  # noqa: E402

OUT = os.path.join(HERE, "out")


def load_story_kern():
    p = os.path.join(RE, "text", "_scratch", "story_attrs.jsonl")
    out = {}
    for line in open(p, encoding="utf-8"):
        s = json.loads(line)
        if s["style"]:
            out[(s["file"], tuple(s["key"]))] = [[x["len"], x.get("11")] for x in s["style"]]
    return out


def category(r):
    a = r["attrs"]
    if a.get("TFTi") != 0 or a.get("RIAf") != 0 or a.get("BOAf") != 0:
        return "kein ATE-Punkttext"
    if any(a.get(k) is None or a[k] != int(a[k]) for k in ("LFTf", "TOPf", "RITf", "BOTf")):
        return "Rahmen nicht ganzzahlig (aeltere Datei)"
    text = "".join(x.get("TRNs", "") for x in r["runs"])
    if "\t" in text:
        return "Tabulator"
    if a.get("TOAf") - a.get("TOPf") <= 0:
        return "Schrift beim Speichern fehlend (asc=0)"
    return "ok"


def main():
    os.makedirs(OUT, exist_ok=True)
    sk_all = load_story_kern()
    recs = [json.loads(line) for line in open(os.path.join(RE, "text", "_scratch", "texts.jsonl"), encoding="utf-8")]
    st = collections.Counter()
    with open(os.path.join(OUT, "layout_corpus.jsonl"), "w", encoding="utf-8") as fo:
        for r in recs:
            # Neutrality: texts with the program or vendor name are not stored
            # (affects 2 records of the category "kein ATE-Punkttext", no influence on the measurement).
            if neutral.contains(json.dumps(r, ensure_ascii=False)):
                st["dropped (neutrality)"] += 1
                continue
            a = r["attrs"]
            cat = category(r)
            sk = sk_all.get((r["file"], (a.get("LFTf"), a.get("TOPf"), a.get("RNDi"))))
            rr = dict(r)
            if sk is not None:
                rr["story_kern"] = sk
            res = None
            if cat == "ok":
                try:
                    res = layout.layout_point_text(rr)
                except Exception as ex:  # noqa: BLE001
                    cat = "Fehler " + type(ex).__name__
                if res is None and cat == "ok":
                    cat = "Schrift nicht vorhanden"
            out = {"file": r["file"], "attrs": a, "runs": r["runs"], "story_kern": sk, "category": cat,
                   "tap": r.get("tap"), "mtx": r.get("mtx"), "pat": r.get("pat")}
            if res is not None:
                fr = res["frame"]
                out["py_frame"] = [fr["LFT"], fr["TOP"], fr["RIT"], fr["BOT"]]
                out["py_glyphs"] = [[g["gid"], g["x"], g["y"]] for ln in res["lines"] for g in ln["glyphs"]]
                ok = [fr["LFT"] == a["LFTf"], fr["TOP"] == a["TOPf"], fr["RIT"] == a["RITf"], fr["BOT"] == a["BOTf"]]
                st["n"] += 1
                st["all4"] += all(ok)
                st["vertical"] += ok[1] and ok[3]
                st["horizontal"] += ok[0] and ok[2]
                st["width"] += (fr["RIT"] - fr["LFT"]) == (a["RITf"] - a["LFTf"])
            else:
                st["skip " + cat] += 1
            fo.write(json.dumps(out, ensure_ascii=False) + "\n")
    print(json.dumps(dict(st), indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
