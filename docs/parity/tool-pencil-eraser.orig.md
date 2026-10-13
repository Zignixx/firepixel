# 03 `tool-pencil-eraser` – Buntstift, Radiergummi: Aufnahme des Originals (PAR-O-03-04, 13.10.2026 ≈ 06:40–07:20)

Testdateien T01, T02, T04 (Arbeitskopien `re\_testdateien\par\work\p34_t0*.fw.png`, je Messung frisch geöffnet mit
`fw.openDocument(url)`). Fenster 1600 × 1000, Zoom 100 %, Leinwand bei Bildschirm 297,183. Werkzeug per JSF
(`fw.activeTool = "Bleistift"` / `"Radiergummi"`), Strichfarbe #C04040, Füllfarbe #808080, B1 gewählt (sofern nicht anders
genannt). Echte Mauszüge (30 Schritte à 15 ms), Standardzug **60,150 → 500,150**, Diagonale 60,220 → 260,330.
Referenzbilder `re\_testdateien\par\ref\tool-pencil-eraser\` (+ `INDEX.md`), Exporte und Hashes
`re\_testdateien\par\px\tool-pencil-eraser\` (`pixels.csv`). Messart: **W** Steuerelemente, **J** JSF, **E** Export + Pixel,
**B** Bild, **M** echter Mauszug. Inspektorwerte = gespeicherter Stand beim Start (nicht zurückgesetzt).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `pencil_inspector` | Werkzeug heißt im Werkzeugobjekt „Bleistift“, die **Inspektor-Überschrift zeigt „Pinsel“**. Zeile 1: Werkzeugsymbol, **Farbfeld** (Strichfarbe, 216–237 × 894–915), Kontrollkästchen **„Anti-Alias“** (258,897) und darunter **„Autom. löschen“** (258,917), daneben **„Transparenz beibehalten“** (399,897), rechts Deckkraft-Symbol + **Deckkraft** 100 (Feld 598–627 + ▼) und **Mischmodus** „Normal“ (Liste 646–768, 55 Einträge laut CB_GETCOUNT, gleiche Liste wie Pinsel). Kein Größenfeld. Vorgabe: alle Kästchen aus | W, B `01` |
| 02 | `pencil_stroke` | Ohne Anti-Alias: waagrecht 441 px genau in Zeile 150 (x 60–500), Farbe exakt #C04040 (= Pinsel 1 px); Diagonale 201 px, 1 px breite Treppe (Zeile 280: x 169–170). Auf T01 (leer): neues Bitmap-Objekt 60,150–501,151 | M, E, J, B `02` |
| 03 | `pencil_aa` | Anti-Alias an: Diagonale 503 px, je Zeile bis 5 Mischpixel (Zeile 280: x 167–171 = #D39B7E #C96E60 #C76056 #CD806C #D4A183); Waagrechte 440 px (x 60–499) | M, E, B `03` |
| 04 | `pencil_autoerase` | Autom. löschen an: Zug, der **auf einem Pixel der Strichfarbe beginnt** (200,150 auf der zuvor gezogenen Linie), malt in **Füllfarbe #808080** (200,150–200,190); Zug, der woanders beginnt (220,160), malt #C04040 | M, E, B `04` |
| 05 | `pencil_live` | Strich erscheint live beim Ziehen (Halbzeit: x 60–279 rot), kein Rahmen | M, B `05` |
| 06 | `pencil_cursor` | Bild nicht möglich (Systemzeiger erscheint in dieser Remote-Umgebung nicht im Bildschirmfoto) – offen | – |
| 07 | `eraser_inspector` | Überschrift „Radiergummi“. **„Größe:“** 13 (Feld 280–312 × 894–915 + ▼), **„Form:“** zwei Bildknöpfe rund (400–424) / eckig (424–448), je 24 × 20, gedrückter Knopf = aktive Form (BM_GETCHECK liefert für beide 0, Zustand nur im Bild), **„Rand:“** Kantenvorschau (255–278) + Feld 2 + ▼ (280–312 × 918–939), rechts **Deckkraft** 100 + ▼. Kein Mischmodus, kein Anti-Alias | W, B `07` (2) |
| 08 | `eraser_round` | Rund, 13, Rand 2: 5 767 px verändert, 53,143–507,157 (14 Zeilen), Mitte voll transparent (weiße Leinwand sichtbar), Randzeilen teilweise (200,143 = #F1ECE0 über #B49D5C) | M, E, B `08` |
| 09 | `eraser_square` | Eckig, 13: 5 369 px, 54,144–507,157 (13 Zeilen), harte Kanten | M, E, B `09` |
| 10 | `eraser_edge` | Rand 0 (rund): 5 337 px, 54,144–507,157, hart. Rand 50: 5 767 px, 53,143–507,157, weiche Randzeilen (200,144 = #F6F3EB, 200,143 = #D3C59F) | M, E |
| 11 | `eraser_opacity` | Deckkraft 50: Mitte halb gelöscht (#B49B5C → #D9CDAD über weißer Leinwand ≈ 50 %), 5 767 px | M, E, B `11` |
| 12 | `eraser_vector` | T04, R1 (Vektor) gewählt: **keine Wirkung, keine Meldung**, Export pixelgleich mit unverändertem T04; Auswahl danach leer | M, E, J, B `12` |
| 13 | `eraser_selection` | Auswahlrahmen 100,100–250,200 auf B1, dann Zug: nur 100,143–250,157 gelöscht (2 100 px) | M, E, B `13` |
| 14 | `eraser_live` | Löschen sichtbar während des Ziehens (Halbzeit: Spur weiß bis ≈ x 280) | M, B `14` |
| 15 | `eraser_cursor` | Bild nicht möglich (wie 06) – offen | – |
| 16 | `undo` | Je Zug **ein** Verlaufsschritt; Namen im Menü Bearbeiten: Radiergummi → „Rückgängig: Radiergummi“, Buntstift → **„Rückgängig: Pinsel“**. Ein Strg+Z stellt jeweils das Ausgangsbild her (0 px verändert). Esc nach dem Zug → Schritt „Bitmapmodus beenden“, Werkzeug → Zeiger | M, W, E |

## Hinweise

- Im Original folgten auf Exporte nach Buntstift-Zügen im nächsten JSF-Lauf Zugriffsverletzungen (−1073741819) beim
  Schließen/Öffnen; Exportdateien wurden trotzdem vollständig geschrieben (Hashes in `pixels.csv`), das Programm lief weiter.
- Mischmodus-Liste: CB_GETCOUNT = 55 (Pinsel-Inspektor ebenso); Inhalt siehe `tool-brush-stroke-options.orig.md` Nr. 08.
