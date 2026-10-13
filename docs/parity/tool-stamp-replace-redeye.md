# 05 `tool-stamp-replace-redeye` – Vergleich Original ↔ Firepixel (PAR-F-05-07, 13.10.2026)

Firepixel: Debug-Build `main` 134d195 (Slot fp-s11, Stichproben auf 0c5dcf3 unverändert), eigene Instanz mit eigenem Profil und
`--mcp`, Fenster 1600 × 1000, Design „normal“, Zoom 100 %, Seite bei 297,181; echtes Fenster, Brücke (`ui_input` Ziehen 30 Schritte
à 15 ms, Alt-Klick mit gehaltener Taste, Felder per Klick + Strg+A + Tippen + Enter, `ui_find`, `history`, `export`). T02/T04 je
Messung frisch, B1 bzw. R1 gewählt. Ausgangsexport T02 pixelgleich mit dem Original (bf663a48136ea54c). Kennzahlen wie
`pixels.csv`; „Δ Orig“ = Pixel, die vom Original-Export abweichen. Bilder `docs\parity\img\tool-stamp-replace-redeye\`, Vergleich
`re\_testdateien\par\cmp\tool-stamp-replace-redeye\` (19 Paare; Zeigerbilder 13 und `02_…-alt_pressed` ohne Partner).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | stamp_inspector | Größe 16, Rand Vorschau + 50, „Quelle ausgerichtet“ ☑, „Gesamtes Dokument“ ☐, 100, Normal, keine Form-Knöpfe | gleich (Lage ± 5 px) | ✅ | – |
| 02 | stamp_source | Alt-Klick 120,268: Kreuz an der Quelle, blauer Kreis-Zeiger am Ziel | gleich; Kreuz etwas kleiner | ✅ | Hinweis |
| 03 | stamp_aligned | Versatz −210/+8; an: Zug 2 behält Versatz, aus: Zug 2 wieder ab Quelle (−210/−32); je 2 528 px, 322,252–399,309 | Versatz −210/+8 bzw. −210/−32 ✅; 2 532 / 2 534 px, **322,252–397,309** (Zugende 2 px kürzer); Δ Orig 1 214 px, max 136 | ❌ | PAR 05/03 |
| 04 | stamp_live | Kopie erscheint live beim Ziehen, Quellkreuz wandert mit | Kreuz wandert ✅; während des Zugs nur ein **blaues Vorschauband**, Kopie erst beim Loslassen | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/04 |
| 05 | stamp_no_source | 0 px, keine Meldung | 0 px (Hash gleich); Statuszeile „Stempel: zuerst mit Alt+Klick die Quelle festlegen.“ | 🎨 | Frage 1 |
| 06 | replace_inspector | Größe 13, Form rund/eckig, Toleranz 32, Von Farbfeld/Grafik + #00FF00, in #FF0000, Stärke 200, Kolorieren ☐ | gleich (Liste „Von“: Farbfeld, Grafik) | ✅ | – |
| 07 | replace_result | Rot → Blau, Zug 290 → 510: 520 px, 360,114–400,127 (13 Z.), Ergebnis #0000FE; Kolorieren exakt #0000FF; Grafik ab 370: 452 px ab x 364 | **560 px, 360,113–400,127 (14 Z.)**, Ergebnis **#A80057** (Mischung); Kolorieren **#7F002E**; Grafik 518 px ab x 363 | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/07 |
| 08 | replace_tolerance | Von #8E8881, Zug 60 → 270: Toleranz 0 → 20 px, 32 → 2 249, 100 → 2 867 | bei 0/32/100 **pixelgleich** 1 686 px (100–221); Befehl ohne Toleranzparameter, Stärke 200 → `strength` 100 | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/08 |
| 09 | replace_from_pick | Farbfeld-Popup (Raster, HEX, RGB, Deckkraft, Knöpfe); HEX + Enter setzt, Popup schließt | gleich; HEX zeigt wie im Original die Farbe unter dem Zeiger | ✅ | – |
| 10 | redeye_inspector | Kopf „Rote Augen / entfernen“, Toleranz 50, Stärke 50 | gleich | ✅ | – |
| 11 | redeye_result | **Rechteck** 100,250 → 225,287: 1 116 px in 107,255–214,282, Pupillen dunkel; Klick/waagrechter Zug: keine Wirkung | malt wie ein Pinsel (Größe 13) **entlang der Diagonale**: 347 px, 110,256–211,281, Augenmitte 120,268 unverändert #DC141E; Klick → 196 px, waagrechter Zug → 660 px; kein Rechteckrahmen | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/11 |
| 12 | redeye_tolerance | 10 = 50 (1 116 px); 100 → 1 170 px, #1F1F1F | 10 und 100 pixelgleich mit 50 (Toleranz ohne Wirkung) | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/11 |
| 13 | cursor | eigene Zeiger je Werkzeug | nicht aufgenommen (Brücke zeichnet keinen Systemzeiger) | – | offen |
| 14 | vector_target | T04/R1: keine Wirkung, keine Meldung, 7 Objekte, R1 Pfad | keine Wirkung (Export = T04-Ausgang), 7 Objekte, R1 Pfad, kein Schritt; Statuszeile „keine passende Auswahl: keine Bitmap unter dem Strich“ | 🎨 | Frage 1 |
| 15 | undo | „Stempel“, „Farbe ersetzen“, „Rote Augen entfernen“; wirkungsloser Rote-Augen-Zug → „Bitmap bearbeiten“ | ein Schritt je Zug ✅, Name bei allen **„Retusche“** | 🔧 behoben (29b532b), Gegenprüfung offen | PAR 05/15 |

**Zählung:** ✅ 5 · ❌ 7 · 🎨 2 · offen 1 (Mauszeiger).

## Fragen an den Nutzer

1. (05, 14) Firepixel meldet wirkungslose Züge in der Statuszeile (kein Dialog), das Original schweigt. Als hilfreiche Ergänzung
   behalten (🎨) oder wie das Original ohne Hinweis?

## Hinweise / Annahmen

- T04-Ausgangsexport weicht vom Original ab (668 px an E1/P1/PG-Strichkanten, max 87) – nicht Teil dieser Einheit, Vergleiche
  bei 14 daher gegen den eigenen T04-Export.
- 03: Gleiche Ursache vermutlich wie tool-retouch/13–14 (gemeinsame Spitzenrasterung, Zugende/weiche Kante).
- 15: gleiche Ursache wie PAR 04/18 (Befehl `bitmap.retouch` mit festem Namen).
- 07/08: Das Ersetzen mischt die Zielfarbe nur teilweise ein; Toleranz und Stärke kommen im Befehl nicht an (`params`: kein
  `tolerance`, `strength` 100).
- Annahme: „Gesamtes Dokument“ des Stempels nicht gemessen (Original auch nicht).
