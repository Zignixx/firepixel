"""Testdateien re/_testdateien/09/*.fw.png -> out/testfiles.jsonl (alle Textobjekte der ersten Seite).

Je Text: Datei, Blatt-Tags (attrs), Laeufe, TAPv (Modus, Versatz, Pfadkonturen), PATv-Kante (FETi), MTXv,
Render-Cache (x, y, w, h, RGBA base64) und die Rahmenvorhersage von re/text/layout.py fuer Punkttexte.
Aufruf: python.exe -I -B gen_testfiles.py
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402,F401  (setzt sys.path)
from common import OUT, RE, b64, leaves, path_contours, runs_of
from fwpng import FwFile, iter_files
from fwfmt import render
import layout


def main():
    os.makedirs(OUT, exist_ok=True)
    d = os.path.join(RE, "_testdateien", "09")
    n = 0
    with open(os.path.join(OUT, "testfiles.jsonl"), "w", encoding="utf-8") as fo:
        for f in iter_files([d]):
            fw = FwFile(f)
            if "mkTS" not in fw.trees and "mkBS" not in fw.trees:
                continue
            store = render.TileStore(fw.tiles)
            roots = fw.doc_roots()
            if not roots:
                continue
            for _p, node in roots[0].walk():
                if node.key != "TXTv":
                    continue
                rec = {"file": os.path.basename(f), "attrs": leaves(node), "runs": runs_of(node)}
                tap = node.child("TAPv")
                if tap is not None:
                    rec["tap"] = {"TAMi": tap.get("TAMi", 0), "CHOf": tap.get("CHOf", 0.0),
                                  "contours": path_contours(tap.child("PTHv"))}
                pat = node.child("PATv")
                fpl = pat.child("FPLv") if pat is not None else None
                rec["fill_edge"] = fpl.get("FETi") if fpl is not None else None
                m = node.child("MTXv")
                if m is not None:
                    rec["mtx"] = leaves(m)
                img = node.child("IMGv")
                if img is not None:
                    bm = render.bitmap_from_img(img, store)
                    if bm is not None:
                        x, y, w, h, px = bm
                        rec["cache"] = {"x": x, "y": y, "w": w, "h": h, "rgba": b64(px)}
                a = rec["attrs"]
                if a.get("TFTi") == 0:
                    try:
                        res = layout.layout_point_text(rec)
                    except Exception:  # noqa: BLE001
                        res = None
                    if res:
                        fr = res["frame"]
                        rec["py_frame"] = [fr["LFT"], fr["TOP"], fr["RIT"], fr["BOT"]]
                fo.write(json.dumps(rec, ensure_ascii=False) + "\n")
                n += 1
    print("Texte:", n)


if __name__ == "__main__":
    main()
