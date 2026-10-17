"""Erzeugt crates/fp-export/src/tables_data.rs aus re/export/_scratch/quant_tables.json.

Eingebettet werden nur die unregelmaessigen Zahlentabellen (Kanalreduktion 1..6 Bit, Windows-Halbtonpalette);
Web-Snap und Mac-Systempalette sind Formeln (hier gegen die Tabellen geprueft).
Aufruf: python -I gen_tables.py
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.abspath(os.path.join(FP, "..", "re"))
T = json.load(open(os.path.join(RE, "export", "_scratch", "quant_tables.json")))

# Formeln pruefen
assert T["websnap"] == [((v + 25) // 51) * 51 for v in range(256)], "Web-Snap-Formel"
mac = []
for r in range(5, -1, -1):
    for g in range(5, -1, -1):
        for b in range(5, -1, -1):
            if r or g or b:
                mac.append((r * 51 << 16) | (g * 51 << 8) | b * 51)
ramp = [0xEE, 0xDD, 0xBB, 0xAA, 0x88, 0x77, 0x55, 0x44, 0x22, 0x11]
mac += [v << 16 for v in ramp] + [v << 8 for v in ramp] + ramp + [(v << 16) | (v << 8) | v for v in ramp] + [0]
assert [int(h, 16) for h in T["mac"]] == mac, "Mac-Formel"

out = ["// Automatisch erzeugt von tools/golden/fp-export/gen_tables.py - nicht von Hand aendern.",
       "//! Unregelmaessige Zahlentabellen der Quantisierung (08 §2.3, §2.2).", "",
       "/// Kanalreduktion auf 1..6 Bit (Index = Bits - 1), gerundet wie das Original (08 §2.3 Schritt 3).",
       "pub(crate) static REDUCE: [[u8; 256]; 6] = ["]
for bits in range(1, 7):
    vals = T["reduce"][str(bits)]
    assert len(vals) == 256
    rows = [", ".join(str(v) for v in vals[i:i + 32]) for i in range(0, 256, 32)]
    out.append("    [" + ",\n     ".join(rows) + "],")
out.append("];")
out.append("")
out.append("/// Windows-Halbton-/Systempalette in der Reihenfolge des Originals (RGB, Schwarz zuerst).")
out.append("pub(crate) static WINDOWS_PALETTE: [u32; 256] = [")
w = [int(h, 16) for h in T["win"]]
for i in range(0, 256, 8):
    out.append("    " + ", ".join("0x%06x" % v for v in w[i:i + 8]) + ",")
out.append("];")
dst = os.path.join(FP, "crates", "fp-export", "src", "tables_data.rs")
open(dst, "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
print("geschrieben:", dst)
