"""Erzeugt das App-Icon von Firepixel (eigenes Design, ohne Anlehnung an das Original).

Motiv: eine Flamme aus Pixelquadraten, deren Spitze nach rechts oben in einzelne Funken-Pixel
zerfällt, auf einem nachtvioletten, abgerundeten Quadrat mit warmem Schein.

Quellen der Wahrheit sind die Pixelkarten unten:
  * KARTE_KLEIN (14 x 14) – pixelgenau für 16, 20 und 32 px (1 bzw. 2 px je Zelle)
  * KARTE_GROSS (20 x 20) – pixelgenau für 24 px (1 px je Zelle), als Vektor mit Fugen ab 40 px
Ausgabe (neben diesem Skript):
  firepixel.svg, png\\firepixel_<n>.png, firepixel.ico (16..256), firepixel_256.png
  und die Vorschau ..\\..\\docs\\screenshots\\phase5_icon_vorschau.png

Aufruf: python.exe -I assets\\icon\\icon_erzeugen.py   (Pillow + numpy)
"""
from __future__ import annotations

import io
import os
import struct

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HIER = os.path.dirname(os.path.abspath(__file__))
WURZEL = os.path.normpath(os.path.join(HIER, "..", ".."))

# --- Farben ---------------------------------------------------------------------------------
FARBEN = {
    "w": (255, 248, 206),  # Kern, fast weiß
    "y": (255, 214, 58),   # Gelb
    "o": (255, 138, 30),   # Orange
    "r": (242, 70, 42),    # Rot
    "R": (196, 28, 70),    # dunkles Karminrot (Schattenkante)
    # Funken (werden groß kleiner gezeichnet)
    "Y": (255, 222, 80),
    "O": (255, 150, 40),
    "M": (246, 64, 168),   # Magenta
    "V": (178, 108, 255),  # Violett
}
FUNKEN = set("YOMV")
HG_OBEN = (58, 20, 96)      # Hintergrund oben (Nachtviolett)
HG_UNTEN = (20, 6, 40)      # Hintergrund unten
RAND_HELL = (140, 80, 200)  # feine Innenkante

KARTE_KLEIN = [
    "..........M...",
    ".......Y......",
    ".........O..V.",
    "........r.....",
    ".......rr..M..",
    "......rror....",
    ".....rroor....",
    "....Rrooyor...",
    "...Rroyyyor...",
    "...Rroywyyor..",
    "..Rroywwwyor..",
    "..Rroywwwyor..",
    "...Rroyyyor...",
    "....RRrrrr....",
]

KARTE_GROSS = [
    "..............V.....",
    "...........M......Y.",
    ".............O......",
    ".........M.....M....",
    "...........Y........",
    "..........r..O......",
    ".........rr.........",
    "........rrr..r......",
    ".......rror.........",
    "......rroor..r......",
    ".....Rrooorr........",
    ".....Rroyoor........",
    "....Rrooyyoor.......",
    "....Rroyywyor.......",
    "...Rrroywwwyor......",
    "...Rroywwwwyor......",
    "...Rroywwwwyor......",
    "....Rroyyyyor.......",
    ".....Rrooooor.......",
    ".......RRrrr........",
]

GROESSEN = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]


def zellen(karte):
    for j, zeile in enumerate(karte):
        for i, ch in enumerate(zeile):
            if ch != ".":
                yield i, j, ch


def mitte_versatz(karte, n_zellen_x, n_zellen_y):
    """Schwerpunkt-Korrektur: die Karte ist schon zentriert gezeichnet, kein Versatz."""
    return 0, 0


# --- Hintergrund ----------------------------------------------------------------------------
def hintergrund(s: int, ss: int = 8) -> Image.Image:
    """Abgerundetes Quadrat mit Verlauf, Schein und Innenkante; `ss`-fach überabgetastet."""
    S = s * ss
    radius = 0.22 * S
    rand = 0.0 if s <= 24 else 0.03 * S  # kleine Größen nutzen die volle Fläche
    maske = Image.new("L", (S, S), 0)
    ImageDraw.Draw(maske).rounded_rectangle([rand, rand, S - 1 - rand, S - 1 - rand], radius=radius, fill=255)
    y = np.linspace(0, 1, S)[:, None]
    x = np.linspace(0, 1, S)[None, :]
    oben = np.array(HG_OBEN, float)
    unten = np.array(HG_UNTEN, float)
    t = np.clip(y * np.ones_like(x), 0, 1)
    rgb = oben[None, None, :] * (1 - t[..., None]) + unten[None, None, :] * t[..., None]
    # warmer Schein unter der Flamme
    cx, cy = 0.46, 0.68
    d = np.sqrt(((x - cx) / 0.42) ** 2 + ((y - cy) / 0.40) ** 2)
    schein = np.clip(1 - d, 0, 1) ** 1.6 * (0.55 if s >= 40 else 0.35)
    warm = np.array((255, 96, 40), float)
    rgb = rgb * (1 - schein[..., None]) + warm[None, None, :] * schein[..., None]
    bild = Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB").convert("RGBA")
    bild.putalpha(maske)
    if s >= 32:
        # feine helle Innenkante oben (Glanz)
        kante = Image.new("RGBA", (S, S), (0, 0, 0, 0))
        b = max(1, ss)
        ImageDraw.Draw(kante).rounded_rectangle(
            [rand + b / 2, rand + b / 2, S - 1 - rand - b / 2, S - 1 - rand - b / 2],
            radius=radius - b / 2, outline=RAND_HELL + (110,), width=b)
        verlauf = Image.fromarray((np.clip(1.2 - y * 1.6, 0, 1) * 255 * np.ones_like(x)).astype(np.uint8), "L")
        a = np.array(kante.getchannel("A"), float) * np.array(verlauf, float) / 255
        kante.putalpha(Image.fromarray(a.astype(np.uint8), "L"))
        bild = Image.alpha_composite(bild, kante)
    return bild.resize((s, s), Image.Resampling.BOX)


# --- Flamme ---------------------------------------------------------------------------------
def pixelgenau(s: int, karte, zelle: int) -> Image.Image:
    """Flamme ohne Kantenglättung: jede Zelle `zelle` x `zelle` Pixel, zentriert."""
    bild = hintergrund(s)
    n = len(karte)
    x0 = (s - n * zelle) // 2
    y0 = (s - n * zelle) // 2
    px = bild.load()
    for i, j, ch in zellen(karte):
        f = FARBEN[ch]
        for dy in range(zelle):
            for dx in range(zelle):
                px[x0 + i * zelle + dx, y0 + j * zelle + dy] = f + (255,)
    return bild


def gross(s: int) -> Image.Image:
    """Große Fassung: Zellen mit Fugen, Abrundung, Innenverlauf; Funken kleiner und leuchtend."""
    ss = 4
    S = s * ss
    bild = hintergrund(s).resize((S, S), Image.Resampling.NEAREST)
    n = len(KARTE_GROSS)
    feld = 0.80 * s
    zelle = feld / n
    # ganzzahlige Zellgrenzen in Zielpixeln (scharfe Kanten)
    x0 = (s - feld) / 2
    y0 = (s - feld) / 2 + 0.01 * s
    grenzen_x = [round(x0 + k * zelle) for k in range(n + 1)]
    grenzen_y = [round(y0 + k * zelle) for k in range(n + 1)]
    fuge = max(1, round(zelle * 0.12)) if s >= 96 else 0
    ebene = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    leuchten = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(ebene)
    dl = ImageDraw.Draw(leuchten)
    for i, j, ch in zellen(KARTE_GROSS):
        a, b = grenzen_x[i] * ss, grenzen_y[j] * ss
        c, e = grenzen_x[i + 1] * ss - fuge * ss, grenzen_y[j + 1] * ss - fuge * ss
        f = FARBEN[ch]
        if ch in FUNKEN:
            # Funke: kleiner, mittig in der Zelle
            m = (c - a) * 0.18
            a, b, c, e = a + m, b + m, c - m, e - m
            dl.rectangle([a - (c - a) * 0.6, b - (e - b) * 0.6, c + (c - a) * 0.6, e + (e - b) * 0.6], fill=f + (90,))
        r = (c - a) * 0.18 if s >= 96 else 0
        d.rounded_rectangle([a, b, c - 1, e - 1], radius=r, fill=f + (255,))
        if s >= 64 and ch not in FUNKEN:
            # Glanzkante oben links
            h = max(ss, (e - b) * 0.16)
            hell = tuple(min(255, int(v + (255 - v) * 0.45)) for v in f)
            d.rounded_rectangle([a, b, c - 1, b + h], radius=r, fill=hell + (255,))
    # weicher Schein der Funken
    from PIL import ImageFilter
    leuchten = leuchten.filter(ImageFilter.GaussianBlur(radius=zelle * ss * 0.6))
    bild = Image.alpha_composite(bild, leuchten)
    bild = Image.alpha_composite(bild, ebene)
    # außerhalb des Hintergrunds nichts zeichnen (Schein abschneiden)
    maske = hintergrund(s).getchannel("A").resize((S, S), Image.Resampling.BILINEAR)
    a = np.minimum(np.array(bild.getchannel("A")), np.array(maske))
    bild.putalpha(Image.fromarray(a, "L"))
    return bild.resize((s, s), Image.Resampling.BOX)


def icon(s: int) -> Image.Image:
    if s == 16:
        return pixelgenau(16, KARTE_KLEIN, 1)
    if s == 20:
        return pixelgenau(20, KARTE_KLEIN, 1)
    if s == 24:
        return pixelgenau(24, KARTE_GROSS, 1)
    if s == 32:
        return pixelgenau(32, KARTE_KLEIN, 2)
    return gross(s)


# --- SVG ------------------------------------------------------------------------------------
def svg() -> str:
    s = 256
    n = len(KARTE_GROSS)
    feld = 0.80 * s
    zelle = feld / n
    x0 = (s - feld) / 2
    y0 = (s - feld) / 2 + 0.01 * s
    fuge = zelle * 0.12
    hx = lambda c: "#%02x%02x%02x" % c
    t = [
        '<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="0 0 256 256">',
        "  <!-- Firepixel-Icon: Flamme aus Pixeln, die nach rechts oben in Funken zerfällt."
        " Eigenes Design; erzeugt von icon_erzeugen.py (Quelle: Pixelkarte KARTE_GROSS). -->",
        "  <defs>",
        f'    <linearGradient id="hg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{hx(HG_OBEN)}"/>'
        f'<stop offset="1" stop-color="{hx(HG_UNTEN)}"/></linearGradient>',
        '    <radialGradient id="schein" cx="0.46" cy="0.68" r="0.42" gradientTransform="translate(0.46 0.68) scale(1 0.95) translate(-0.46 -0.68)">'
        '<stop offset="0" stop-color="#ff6028" stop-opacity="0.55"/><stop offset="1" stop-color="#ff6028" stop-opacity="0"/></radialGradient>',
        '    <linearGradient id="kante" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#8c50c8" stop-opacity="0.45"/>'
        '<stop offset="0.7" stop-color="#8c50c8" stop-opacity="0"/></linearGradient>',
        '    <filter id="glimm" x="-1" y="-1" width="3" height="3"><feGaussianBlur stdDeviation="6"/></filter>',
        "  </defs>",
        f'  <rect x="7.7" y="7.7" width="240.6" height="240.6" rx="56.3" fill="url(#hg)"/>',
        f'  <rect x="7.7" y="7.7" width="240.6" height="240.6" rx="56.3" fill="url(#schein)"/>',
        f'  <rect x="8.2" y="8.2" width="239.6" height="239.6" rx="55.8" fill="none" stroke="url(#kante)" stroke-width="1"/>',
        '  <g filter="url(#glimm)" opacity="0.35">',
    ]
    for i, j, ch in zellen(KARTE_GROSS):
        if ch in FUNKEN:
            t.append(f'    <rect x="{x0 + i * zelle - zelle * 0.3:.2f}" y="{y0 + j * zelle - zelle * 0.3:.2f}" '
                     f'width="{zelle * 1.6:.2f}" height="{zelle * 1.6:.2f}" fill="{hx(FARBEN[ch])}"/>')
    t.append("  </g>")
    t.append('  <g shape-rendering="crispEdges">')
    for i, j, ch in zellen(KARTE_GROSS):
        x, y, w = x0 + i * zelle, y0 + j * zelle, zelle - fuge
        if ch in FUNKEN:
            m = w * 0.18
            x, y, w = x + m, y + m, w - 2 * m
        t.append(f'    <rect x="{x:.2f}" y="{y:.2f}" width="{w:.2f}" height="{w:.2f}" rx="{w * 0.18:.2f}" fill="{hx(FARBEN[ch])}"/>')
    t.append("  </g>")
    t.append("</svg>")
    return "\n".join(t) + "\n"


# --- ICO ------------------------------------------------------------------------------------
def ico(bilder: dict[int, Image.Image]) -> bytes:
    """Mehrstufige ICO mit PNG-Einträgen (Windows Vista und neuer)."""
    daten = []
    for s in sorted(bilder):
        puffer = io.BytesIO()
        bilder[s].save(puffer, "PNG", optimize=True)
        daten.append((s, puffer.getvalue()))
    kopf = struct.pack("<HHH", 0, 1, len(daten))
    versatz = 6 + 16 * len(daten)
    eintraege, nutzlast = b"", b""
    for s, png in daten:
        g = 0 if s >= 256 else s
        eintraege += struct.pack("<BBBBHHII", g, g, 0, 0, 1, 32, len(png), versatz + len(nutzlast))
        nutzlast += png
    return kopf + eintraege + nutzlast


# --- Vorschau -------------------------------------------------------------------------------
def vorschau(bilder: dict[int, Image.Image]) -> Image.Image:
    rand = 24
    breite = rand + sum(s + rand for s in GROESSEN)
    zeile = 256 + 2 * rand + 22
    bild = Image.new("RGBA", (breite, zeile * 2 + 40), (255, 255, 255, 255))
    d = ImageDraw.Draw(bild)
    try:
        schrift = ImageFont.truetype(os.path.join(os.environ.get("WINDIR", "C:\\Windows"), "Fonts", "segoeui.ttf"), 14)
    except OSError:
        schrift = ImageFont.load_default()
    d.text((rand, 8), "Firepixel – App-Icon in allen Größen (oben hell, unten dunkel; 16/20/24/32 pixelgenau)", fill=(40, 40, 40), font=schrift)
    for k, grund in enumerate([(240, 240, 240), (30, 30, 30)]):
        y = 40 + k * zeile
        d.rectangle([0, y, breite, y + zeile - 4], fill=grund + (255,))
        x = rand
        for s in GROESSEN:
            bild.alpha_composite(bilder[s], (x, y + rand + 256 - s))
            d.text((x, y + rand + 262), f"{s}", fill=(120, 120, 120) if k == 0 else (170, 170, 170), font=schrift)
            x += s + rand
    # 16 und 32 zusätzlich 8-fach vergrößert (Pixelraster sichtbar)
    lupe = Image.new("RGBA", (16 * 8 + 32 * 4 + 3 * rand + 20 * 6 + 24 * 5 + 2 * rand, 128 + 2 * rand), (240, 240, 240, 255))
    x = rand
    for s, f in [(16, 8), (20, 6), (24, 5), (32, 4)]:
        lupe.alpha_composite(bilder[s].resize((s * f, s * f), Image.Resampling.NEAREST), (x, rand + (128 - s * f) // 2))
        x += s * f + rand
    gesamt = Image.new("RGBA", (max(breite, lupe.width), bild.height + lupe.height), (255, 255, 255, 255))
    gesamt.alpha_composite(bild, (0, 0))
    gesamt.alpha_composite(lupe, (0, bild.height))
    return gesamt


def main() -> None:
    bilder = {s: icon(s) for s in GROESSEN}
    os.makedirs(os.path.join(HIER, "png"), exist_ok=True)
    for s, b in bilder.items():
        b.save(os.path.join(HIER, "png", f"firepixel_{s}.png"), optimize=True)
    with open(os.path.join(HIER, "firepixel.ico"), "wb") as f:
        f.write(ico(bilder))
    with open(os.path.join(HIER, "firepixel.svg"), "w", encoding="utf-8") as f:
        f.write(svg())
    ziel = os.path.join(WURZEL, "docs", "screenshots", "phase5_icon_vorschau.png")
    vorschau(bilder).convert("RGB").save(ziel, optimize=True)
    print("ok", ziel)


if __name__ == "__main__":
    main()
