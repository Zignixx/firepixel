"""Golden-Testvektoren fuer fp-color aus den Python-Referenzen (nur lesen):
  re\\farbe\\farbmodelle.py, re\\farbe\\farbfelddateien.py, re\\vektor\\blend.py, re\\vektor\\stroke.py (overlap_alpha),
  re\\effekte\\fx.py (over, over_mmx, behind, over_exact, over_place; braucht numpy).

Aufruf:  "C:\\Users\\Administrator\\AppData\\Local\\Programs\\Python\\Python314\\python.exe" -I -B gen_color.py
Ausgabe: crates/fp-color/tests/data/color_golden.json

Grosse Wertebereiche werden als CRC-32 (zlib) ueber eine festgelegte Byte-Folge abgelegt; die Rust-Tests erzeugen
dieselbe Folge. Zufallsstichproben kommen aus einem 64-Bit-LCG, den die Rust-Tests identisch nachbauen:
  state = state * 6364136223846793005 + 1442695040888963407 (mod 2^64); next = state >> 33
"""
import json
import os
import struct
import sys
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))
FP = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
RE = os.path.normpath(os.path.join(FP, "..", "re"))
for sub in ("farbe", "vektor", "effekte"):
    sys.path.insert(0, os.path.join(RE, sub))

import numpy as np                 # noqa: E402
import farbmodelle as fm           # noqa: E402
import farbfelddateien as ff       # noqa: E402
import blend as BL                 # noqa: E402
import stroke as ST                # noqa: E402
import fx as FX                    # noqa: E402

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
import neutral                     # noqa: E402

# Die Referenzfunktionen in re\farbe\farbfelddateien.py tragen den Programmnamen des Originals; er kommt
# aus der Installationskopie (tools/golden/neutral.py), nicht aus diesem Quelltext.
_APP = neutral.names()[0].lower()
act_read_orig = getattr(ff, "act_read_" + _APP)
act_write_orig = getattr(ff, "act_write_" + _APP)
ase_write_orig = getattr(ff, "ase_write_" + _APP)
ase_to_orig_swatches = getattr(ff, "ase_to_%s_swatches" % _APP)

OUT = os.path.join(FP, "crates", "fp-color", "tests", "data", "color_golden.json")
M64 = (1 << 64) - 1


class Lcg:
    def __init__(self, seed):
        self.s = seed & M64

    def next(self):
        self.s = (self.s * 6364136223846793005 + 1442695040888963407) & M64
        return self.s >> 33

    def byte(self):
        return self.next() & 0xFF


class Crc:
    def __init__(self):
        self.v = 0

    def add(self, b):
        self.v = zlib.crc32(b, self.v)


out = {"_quelle": "re/farbe/farbmodelle.py, farbfelddateien.py; re/vektor/blend.py, stroke.py; re/effekte/fx.py"}

# ------------------------------------------------------------------ Farbmodelle: Raster 0..255 Schritt 5
G = list(range(0, 256, 5))
c = Crc()
for r in G:
    for g in G:
        for b in G:
            h8, s8, v8 = fm.rgb_to_hsb8(r, g, b)
            mh = fm.rgb_to_mixer_hsb(r, g, b)
            info = [int(x) for x in fm.info_panel_values(255, r, g, b, 3)[:3]]
            vals = [fm.hue_degrees(r, g, b), h8, s8, v8, mh[0], mh[1], mh[2], fm.gray_level(r, g, b),
                    fm.rgb_to_mixer_k(r, g, b), fm.rgb_to_gray_slider(r, g, b)] + info
            c.add(struct.pack("<13i", *vals))
out["models_grid_crc"] = c.v

c = Crc()
for h in range(-2, 363):
    for s in range(0, 101, 5):
        for b in range(0, 101, 5):
            c.add(bytes(fm.mixer_hsb_to_rgb(h, s, b)))
out["mixer_hsb_to_rgb_crc"] = c.v

c = Crc()
for h in range(-5, 366):
    for s in range(0, 256, 15):
        for v in range(0, 256, 15):
            c.add(struct.pack("<3i", *fm.hsb_deg_to_rgb(h, s, v)))
out["hsb_deg_to_rgb_crc"] = c.v

c = Crc()
for h in range(-3, 259):
    for s in range(0, 256, 15):
        for v in range(0, 256, 15):
            c.add(bytes(fm.hsb8_to_rgb(h, s, v)))
out["hsb8_to_rgb_crc"] = c.v

out["mixer_k_to_rgb"] = [[k / 2, list(fm.mixer_k_to_rgb(k / 2))] for k in range(-10, 211)]
out["gamma"] = [[lg, og, fm.gamma_preview_table(lg, og)] for lg, og in ((2.2, 1.8), (1.8, 2.2), (0.05, 1.8), (2.5, 1.0))]
out["to16_exakt"] = [[fm._to16_exakt(v, k) for k in range(4)] for v in range(256)]
c = Crc()
for v in range(65536):
    c.add(bytes([fm._to8_exakt(v)]))
out["to8_exakt_crc"] = c.v

rng = Lcg(14)
info = []
for _ in range(300):
    a, r, g, b = rng.byte(), rng.byte(), rng.byte(), rng.byte()
    info.append([[a, r, g, b], [fm.info_panel_values(a, r, g, b, m) for m in range(6)], list(fm.rgb_to_hex_text(r, g, b))])
out["info_panel"] = info

# ------------------------------------------------------------------ CMYK exakt (Tabelle aus der Installation)
tab = fm.load_cmyk_table(os.path.join(RE, "farbe", "_scratch", "cmyk_1001.bin"))
CG = list(range(0, 256, 15))
c = Crc()
for cc in CG:
    for m in CG:
        for y in CG:
            for k in CG:
                c.add(bytes(fm.cmyk_to_rgb_exakt(cc, m, y, k, tab)))
out["cmyk_grid_crc"] = c.v
rng = Lcg(16)
cm = []
for _ in range(400):
    v = [rng.byte() for _ in range(4)]
    cm.append([v, list(fm.cmyk_to_rgb_exakt(*v, tab))])
for v in ((255, 0, 0, 0), (0, 255, 0, 0), (0, 0, 255, 0), (0, 0, 0, 255), (255, 255, 255, 255), (0, 0, 0, 0),
          (0, 0, 0, 128), (128, 128, 0, 0)):
    cm.append([list(v), list(fm.cmyk_to_rgb_exakt(*v, tab))])
out["cmyk_samples"] = cm
out["cmyk_no_table"] = [[v, list(fm.cmyk_to_rgb_exakt(*v, None))] for v in ([10, 20, 30, 40], [255, 0, 128, 3])]

# ------------------------------------------------------------------ Mischmodi
modes = sorted(BL.MODES)
chan = {}
for mode in modes:
    c = Crc()
    for s in range(256):
        for d in range(256):
            res = BL.blend_rgb(mode, [s, d, s ^ d], [d, s, (s + d) & 255])
            c.add(bytes(res))
    chan[str(mode)] = c.v
out["blend_rgb_crc"] = chan


def pixel_samples(mode, n, seed):
    rng = Lcg(seed)
    crc = Crc()
    first = []
    for i in range(n):
        dst = [rng.byte() for _ in range(4)]
        src = [rng.byte() for _ in range(4)]
        sel = rng.next() % 8
        if sel == 0:
            dst[3] = 0
        elif sel == 1:
            dst[3] = 255
        sel = rng.next() % 8
        if sel == 0:
            src[3] = 0
        elif sel == 1:
            src[3] = 255
        op = rng.byte()
        if rng.next() % 3 == 0:
            op = 255
        m = rng.byte()
        if rng.next() % 2 == 0:
            m = 255
        buf = bytearray(dst)
        BL.blend_pixel(buf, 0, mode, src[0], src[1], src[2], src[3], op, m)
        crc.add(bytes(buf))
        if i < 8:
            first.append([dst, src, op, m, list(buf)])
    return crc.v, first


pix = {}
BL.MMX = True
for mode in modes:
    v, first = pixel_samples(mode, 4000, 1000 + mode)
    pix[str(mode)] = {"crc": v, "first": first}
BL.MMX = False
v, first = pixel_samples(0, 4000, 1000)
pix["0_scalar"] = {"crc": v, "first": first}
BL.MMX = True
out["blend_pixel"] = pix
out["blend_pixel_n"] = 4000

# fx: Bild-auf-Bild-Mischungen
rng = Lcg(5)
N = 20000
top = np.zeros((N, 4), np.int64)
bot = np.zeros((N, 4), np.int64)
for i in range(N):
    t = [rng.byte() for _ in range(4)]
    b = [rng.byte() for _ in range(4)]
    sel = rng.next() % 6
    if sel == 0:
        t[3] = 0
    elif sel == 1:
        t[3] = 255
    sel = rng.next() % 6
    if sel == 0:
        b[3] = 0
    elif sel == 1:
        b[3] = 255
    top[i] = t
    bot[i] = b
fxres = {}
for name, fn in (("over", FX.over), ("over_mmx", FX.over_mmx), ("behind", FX.behind), ("over_exact", FX.over_exact),
                 ("over_place", FX.over_place)):
    r = fn(top, bot)
    fxres[name] = {"crc": zlib.crc32(np.asarray(r, np.uint8).tobytes()), "first": np.asarray(r[:6]).tolist()}
out["fx_n"] = N
out["fx"] = fxres

out["opacity_byte"] = [BL.opacity_byte(p) for p in range(1001)]
out["interpolation_curve"] = list(BL._C)
ov = []
for n in (1, 2, 3, 5, 10, 20, 50, 100):
    ov.append([n, [ST.overlap_alpha(n, A) for A in range(0, 256, 5)]])
out["overlap_alpha"] = ov
out["stamp_promille"] = [(max(1, o) * 1000 // 255) * 255 // 1000 for o in range(256)]

# ------------------------------------------------------------------ Farbfelddateien
rng = Lcg(9)
pal_rand = [(rng.byte(), rng.byte(), rng.byte()) for _ in range(300)]
pal_gray = [(v, v, v) for v in range(255, -1, -1)]
pal_small = [(255, 0, 0), (0, 128, 255), (1, 2, 3), (254, 255, 0), (17, 17, 17)]
sw = {}
for name, pal in (("small", pal_small), ("gray", pal_gray), ("random", pal_rand)):
    act = act_write_orig(pal)
    ase = ase_write_orig(pal)
    sw[name] = {"act_len": len(act), "act_crc": zlib.crc32(act), "ase_len": len(ase), "ase_crc": zlib.crc32(ase),
                "ase_roundtrip": [list(x) for x in ase_to_orig_swatches(ff.ase_read(ase))] == [list(x) for x in pal]}
sw["small"]["act_hex"] = act_write_orig(pal_small).hex()
sw["small"]["ase_hex"] = ase_write_orig(pal_small).hex()
ps = bytes(range(48)) + bytes(768 - 48) + struct.pack(">HH", 16, 0xFFFF)
sw["photoshop_772"] = {"original": [list(x) for x in act_read_orig(ps)][-2:],
                       "original_len": len(act_read_orig(ps)), "photoshop_len": len(ff.act_read_photoshop(ps))}
# allgemeine ASE mit Gruppe, Lab, Gray, CMYK
def blk(t, body):
    return struct.pack(">HI", t, len(body)) + body
def nm(s):
    u = (s + "\0").encode("utf-16-be")
    return struct.pack(">H", len(u) // 2) + u
general = (b"ASEF" + struct.pack(">HHI", 1, 0, 6)
           + blk(0xC001, nm("Gruppe"))
           + blk(0x0001, nm("Rot") + b"RGB " + struct.pack(">fffH", 1.0, 0.0, 0.0, 2))
           + blk(0x0001, nm("C") + b"CMYK" + struct.pack(">ffffH", 1.0, 0.0, 0.0, 0.0, 0))
           + blk(0xC002, b"")
           + blk(0x0001, nm("L") + b"LAB " + struct.pack(">fffH", 0.5, 10.0, -20.0, 1))
           + blk(0x0001, nm("Grau") + b"Gray" + struct.pack(">fH", 0.25, 2)))
ents = ff.ase_read(general)
sw["general_ase_hex"] = general.hex()
sw["general_ase_read"] = [{"name": e["name"], "model": e["model"], "values": list(e["values"]), "type": e["type"],
                           "group": e["group"]} for e in ents]
sw["general_ase_original"] = [list(x) for x in ase_to_orig_swatches(ents, tab)]
sw["general_ase_original_no_table"] = [list(x) for x in ase_to_orig_swatches(ents, None)]
out["swatches"] = sw

os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT, "w", encoding="utf-8") as fh:
    json.dump(out, fh, separators=(",", ":"))
print("geschrieben:", OUT, os.path.getsize(OUT), "Byte")
