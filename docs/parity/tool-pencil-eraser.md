# 03 `tool-pencil-eraser` – Vergleich Original ↔ Firepixel (PAR-F-03-04, 13.10.2026)

Firepixel: Debug-Build `main` 236e35e (Slot fp-s2), eigene Instanz mit eigenem Profil und `--mcp`, Fenster 1600 × 1000
(oben links, nicht aktiviert), Design „normal“, Zoom 100 %, Leinwand bei 297,181; echtes Fenster, Brücke (`ui_input` Ziehen
30 Schritte à 15 ms, `ui_find`, `screenshot`, `history`, `export`). Testdateien: frische Kopien von `par_empty|par_bitmap|par_vector.fw.png`
je Messung, B1 gewählt, Strich #C04040, Füllung #808080. Standardzug 60,150 → 500,150, Diagonale 60,220 → 260,330.
Ausgangsexport T02 **pixelgleich** mit dem Original (Hash bf663a48136ea54c). Kennzahlen wie `pixels.csv` (geänderte Pixel
gegen T02, Grenzen, mittlere/größte Abweichung); „Δ Orig“ = Pixel, die vom Export des Originals abweichen.
Bilder `docs\parity\img\tool-pencil-eraser\`, Vergleich `re\_testdateien\par\cmp\tool-pencil-eraser\` (14 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | pencil_inspector | Überschrift **„Pinsel“**; Farbfeld, Anti-Alias, Autom. löschen, Transparenz beibehalten, Deckkraft 100, Mischmodus Normal; alle aus | gleiche Felder, gleiche Lage (± 3 px), alle aus; Überschrift **„Bleistift“** | 🎨 | Frage 1 |
| 02 | pencil_stroke | T02: 642 px, 60,150–501,331, #C04040; T01: neues Bitmap 60,150–501,151 | T02: 642 px, gleiche Grenzen, max 192; 38 px Treppenversatz in der Diagonale (Zwischenpunkte der Brücke mit Nachkommastellen, Original ganzzahlige Mauspunkte); T01: Bitmap 60,150–501,151, Export pixelgleich | ✅ | – |
| 03 | pencil_aa | Waagrechte bleibt 1 px #C04040 (x 60–499); Diagonale je Zeile bis 5 Mischpixel (Z. 280: #D39B7E #C96E60 #C76056 #CD806C #D4A183); 943 px, max 192 | **Waagrechte auf 2 Zeilen verteilt** (y 149/150 je ≈ #A76862, keine volle Strichfarbe); Diagonale nur 4 Pixel/Zeile (#D09077 #C65C53 #C65C53 #D08C74); 1 388 px, max 111; Δ Orig 1 435 px | ✅ gegengeprüft 13.10. (PAR-R-1: Waagrechte 1 Zeile #C04040, Zeilen 149/151 unverändert, Waagrechte pixelgleich; Diagonale mit ganzzahligen Punkten 29 px > 16, mit Brückenpunkten 108 px > 16 (Nachkommastellen, vgl. 02)) | PAR 03/03 |
| 04 | pencil_autoerase | Zug ab Strichfarbe → #808080, sonst #C04040 | **pixelgleich** (Hash 03c21af80f72b5f6) | ✅ | – |
| 05 | pencil_live | Halbzeit: rot x 60–279 | Halbzeit: Änderung x 60–293 | ✅ | – |
| 06 | pencil_cursor | nicht aufgenommen (offen) | nicht aufgenommen | – | offen |
| 07 | eraser_inspector | „Radiergummi“: Größe 13, Form rund/eckig (gedrückter Knopf), Rand Vorschau + 2, Deckkraft 100; kein Mischmodus/Anti-Alias | gleich (Knöpfe „Runde/Quadratische Spitze“, gedrückt = aktiv), Lage ± 3 px | ✅ | – |
| 08 | eraser_round | 5 767 px, 53,143–507,157, Randzeile #F1ECE0 | 5 783 px, gleiche Grenzen, Randzeile #EEE9DB; Δ Orig max 11 | ✅ | – |
| 09 | eraser_square | 5 369 px, 54,144–507,157, **harte Kanten** (13 Zeilen) | 5 796 px, **53,143–507,157, weiche Randzeilen** (#FEFEFE, #D4C5A2), Enden weich | ✅ gegengeprüft 13.10. (PAR-R-1: Hash 0aae977f6db17805 = Original, pixelgleich) | PAR 03/09 |
| 10 | eraser_edge | Rand 0: 5 337 px, 54,144–507,157, hart. Rand 50: 5 767 px, 200,143 #D3C59F | Rand 0: **5 783 px, 53,143–507,157, weich wie Rand 2**; Rand 50: 5 796 px, Δ Orig 843 px > 16 | ✅ gegengeprüft 13.10. (PAR-R-1: Rand 0 Hash df60ada40f816670 = Original; Rand 50: 16 px > 16, max 37, Grenzen gleich) | PAR 03/10 |
| 11 | eraser_opacity | Deckkraft 50: Mitte **halb** gelöscht (#D9CDAD), max 127 | Mitte **voll gelöscht** (#FFFFFF, Tupfer summieren sich), Ränder halb; max 255 | ✅ gegengeprüft 13.10. (PAR-R-1: max. Abw. 5, Mitte halb gelöscht) | PAR 03/11 |
| 12 | eraser_vector | R1 gewählt: keine Wirkung, keine Meldung, Auswahl danach leer | gleich (Export = unverändertes T04, kein Verlaufsschritt, Auswahl leer) | ✅ | – |
| 13 | eraser_selection | Auswahlrahmen 100,100–250,200 + Zug: nur 100,143–250,157, 2 100 px | 2 100 px, gleiche Grenzen; Δ Orig max 9 | ✅ | – |
| 14 | eraser_live | Halbzeit Spur bis ≈ x 280 | Halbzeit bis x 287 | ✅ | – |
| 15 | eraser_cursor | nicht aufgenommen (offen) | nicht aufgenommen | – | offen |
| 16 | undo | je Zug ein Schritt „Radiergummi“ / Buntstift „Pinsel“; Strg+Z → Ausgangsbild; Esc → Schritt „Bitmapmodus beenden“, Werkzeug Zeiger | Schritte „Radiergummi“ / „Pinsel“, Menü „Rückgängig: Radiergummi“; Strg+Z → Hash bf663a48136ea54c (beide); Esc → Zeiger, **aber kein Schritt „Bitmapmodus beenden“** | ✅ gegengeprüft 13.10. (PAR-R-1: Esc → Zeiger + Schritt „Bitmapmodus beenden“ (Radiergummi und Buntstift), Strg+Z → bf663a48136ea54c) | PAR 03/16 |

**Zählung:** ✅ 13 · ❌ 0 · 🎨 1 · offen 2 (Mauszeiger, auch im Original nicht aufgenommen) – nach Gegenprüfung PAR-R-1 13.10. (5 behobene Zeilen geprüft: 5 ✅, 0 ❌).

## Fragen / Annahmen

1. 01: Das Original nennt den Buntstift im Inspektor „Pinsel“ (wie im Verlauf). Firepixel zeigt „Bleistift“, übernimmt aber
   „Pinsel“ im Verlauf. Frage an den Nutzer: Überschrift wie das Original („Pinsel“) oder lesbarer „Bleistift“ lassen?
2. 02: Die Brücke interpoliert Ziehpunkte mit Nachkommastellen (z. B. 66,67/223,67); das Original bekam ganzzahlige
   Bildschirmpunkte. Der Treppenversatz (38 px) wird daher nicht als Fehler gewertet.
3. 12: Strecke des Originals über R1 nicht notiert; hier 30,85 → 200,85 (quer durch R1).
4. Mischmodus-Liste (01): Inhalt/Anzahl gehört zu `tool-brush-stroke-options` Nr. 08 (gleiche Liste), dort verglichen.
5. 13: Ohne gewähltes Bitmap löscht Firepixel nach einem Auswahlrahmen nichts (kein Verlaufsschritt); im Original nicht
   gemessen (dort B1 gewählt) – nur notiert.
