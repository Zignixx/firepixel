"""Golden-Daten fuer fp-text: Korpus-Textrahmen mit der Python-Referenz (re/text/layout.py).

Liest (nur lesend) re/text/_scratch/texts.jsonl (scripts/text/extract_text.py) und
re/text/_scratch/story_attrs.jsonl (scripts/text/story_attrs.py), rechnet fuer jeden Text das Layout mit
layout.layout_point_text (Modus round_each, mit Tinte, Kerning-Art der Story) und schreibt je Text eine
JSON-Zeile nach out/layout_corpus.jsonl:
  file, attrs, runs, story_kern, category (Filter wie check_layout.py), py_frame, py_glyphs [[gid, x, y], ...]
Zusaetzlich die Zaehlung wie check_layout.py auf stdout (Ziel: alle vier Kanten 3.998/4.147).

Aufruf (aus beliebigem Ordner):
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
            # Neutralitaet: Texte mit dem Programm- oder Herstellernamen werden nicht gespeichert
            # (betrifft 2 Datensaetze der Kategorie "kein ATE-Punkttext", ohne Einfluss auf die Messung).
            if neutral.contains(json.dumps(r, ensure_ascii=False)):
                st["weggelassen (Neutralitaet)"] += 1
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
                st["alle4"] += all(ok)
                st["vertikal"] += ok[1] and ok[3]
                st["horizontal"] += ok[0] and ok[2]
                st["breite"] += (fr["RIT"] - fr["LFT"]) == (a["RITf"] - a["LFTf"])
            else:
                st["skip " + cat] += 1
            fo.write(json.dumps(out, ensure_ascii=False) + "\n")
    print(json.dumps(dict(st), indent=1, ensure_ascii=False))


if __name__ == "__main__":
    main()
