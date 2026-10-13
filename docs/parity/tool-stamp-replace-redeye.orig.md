# 05 `tool-stamp-replace-redeye` – Stempel, Farbe ersetzen, Rote Augen entfernen: Aufnahme des Originals (PAR-O-05-07, 13.10.2026 ≈ 06:45–07:40)

Testdateien T02, T04 (Arbeitskopien `re\_testdateien\par\work\p57_t0*.fw.png`, je Messung frisch mit `fw.openDocument(url)`).
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei Bildschirm 297,183. Werkzeug per Skript (`Tools.stamp`, `"Farbe ersetzen"`,
`"Rote Augen entfernen"`), Strich #C04040, Füllung #808080, B1 gewählt. Echte Mauszüge (30 Schritte à 15 ms). Referenzbilder
`re\_testdateien\par\ref\tool-stamp-replace-redeye\` (+ `INDEX.md`), Exporte/Hashes `re\_testdateien\par\px\tool-stamp-replace-redeye\pixels.csv`.
Messart: **W** Steuerelemente, **J** Skript, **E** Export + Pixel, **B** Bild, **M** echter Mauszug. Inspektorwerte = gespeicherter Stand beim Start.

B1-Inhalt (Dokumentkoordinaten): Verlauf 40–280 × 40–200, Streifen schwarz/weiß/rot/grün(0,160,0)/blau/gelb je 40 px ab x 280
(rot = x 360–400, #FF0000), Hautton 40–280 × 200–360 mit roten Augen r = 12 bei 120,268 und 200,268 (#DC141E), Schachbrett rechts unten.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `stamp_inspector` | Überschrift „Stempel“. **„Größe:“** 16 (Feld 280–312 + ▼), darunter **„Rand:“** Kantenvorschau (255–278) + 50 + ▼. Mitte: Kästchen **„Quelle ausgerichtet“** ☑ (390,896) und darunter **„Gesamtes Dokument“** ☐ (= alle Ebenen). Rechts Deckkraft-Symbol + **100** + ▼ und **Mischmodus** „Normal“ (55 Einträge, wie Pinsel). Keine Form-Knöpfe | W, B `01` |
| 02 | `stamp_source` | **Alt-Klick** setzt die Quelle (120,268). Danach: **Kreuz (+)** an der Quelle sichtbar, am Ziel ein **blauer Kreis** in Pinselgröße als Zeiger. Mit gehaltener Alt-Taste vor dem Klick bleibt der Stempel-Zeiger (Hotspot 11,9) | M, B `02` (2) |
| 03 | `stamp_aligned` | Zug 1 330,260 → 390,260 kopiert mit Versatz (−210, +8) aus der Quelle (Kreis-Pinsel 16, Rand 50: 322,252–399,269). **Ausgerichtet an:** Zug 2 (330,300 → 390,300) behält den Versatz (kopiert Zeile 308). **Aus:** Zug 2 beginnt wieder an der Quelle (Versatz −210, −32, also wieder das rote Auge). Je 2 528 px verändert | M, E, B `03` (2) |
| 04 | `stamp_live` | Kopie erscheint live beim Ziehen; das Quellkreuz wandert parallel mit | M, B `04` |
| 05 | `stamp_no_source` | Zug ohne vorher gesetzte Quelle: **keine Wirkung, keine Meldung** (0 px, Hash = unverändertes T02) | M, E, B `05` |
| 06 | `replace_inspector` | Überschrift „Farbe ersetzen“. Zeile 1: **„Größe:“** 13 + ▼, **„Form:“** zwei Bildknöpfe rund/eckig (424–447/447–470), **„Toleranz:“** 32 + ▼. Zeile 2: **„Von:“** Liste **Farbfeld / Grafik** (280–364) + Farbfeld (Vorgabe **#00FF00**), **„in:“** Farbfeld (Vorgabe **#FF0000**), **„Stärke:“** 200 + ▼, Kästchen **„Kolorieren“** ☐. Keine Liste „Farben ersetzen/Farbton/Farbe/Helligkeit“, kein Deckkraft-/Mischmodusfeld | W, B `06` (2) |
| 07 | `replace_result` | Von #FF0000 → in #0000FF, Zug 290,120 → 510,120: nur der rote Streifen ändert sich, 360,114–400,127 (13 Zeilen = Größe), **520 px**, Ergebnis **#0000FE** (nicht ganz #0000FF). Mit **Kolorieren** dieselben 520 px, Ergebnis exakt #0000FF. **Von: Grafik** nimmt die Farbe unter dem Startpunkt: Start auf Rot (370,120) → 452 px ab x 364 blau | M, E, B `07` |
| 08 | `replace_tolerance` | Von #8E8881 (Verlauf bei 160,120), Zug 60,120 → 270,120: Toleranz **0** → 20 px (nur 159–161, leicht verschoben, 160,120 → #7E7E8B), **32** → 2 249 px (74–247), **100** → 2 867 px (54–277). Ersetzt wird abgestuft, nicht voll in die Zielfarbe (mittlere Änderung 16 / 28 / 71) | M, E, B `08` (2) |
| 09 | `replace_from_pick` | Klick auf ein Farbfeld öffnet das **Farbfeld-Popup** (Farbfelder-Raster 382–635 × 644–825, **HEX**-Feld, **RGB** 3 Felder, Deckkraft 100 %, 3 Bildknöpfe); HEX eintippen + Enter setzt die Farbe, Popup schließt | M, W, B `09` |
| 10 | `redeye_inspector` | Überschrift **zweizeilig** „Rote Augen / entfernen“. **„Toleranz:“** 50 + ▼ (271–303), darunter **„Stärke:“** 50 + ▼. Sonst nichts (keine Größe) | W, B `10` |
| 11 | `redeye_result` | Werkzeug arbeitet mit **aufgezogenem Rechteck** (Rahmen + Kreuz während des Ziehens): Rechteck 100,250 → 225,287 über beide Augen → **1 116 px** in 107,255–214,282, Pupillen dunkel (Mitte #4E1C1E, Rand #7D191E). **Einfacher Klick** oder waagrechter Zug (Höhe 0) über die Augen: **keine Wirkung** | M, E, B `11` (2) |
| 12 | `redeye_tolerance` | Toleranz **10**: pixelgleich mit 50 (1 116 px). Toleranz **100**: 1 170 px, alle Augenpixel #1F1F1F | M, E, B `12` (2) |
| 13 | `cursor` | Eigene Zeiger je Werkzeug (eingezeichnet über GetCursorInfo); Stempel-Zeiger Hotspot 11,9 | B `13` (3) |
| 14 | `vector_target` | T04, R1 (Vektor) gewählt, Zug über R1 mit Stempel (nach Alt-Klick), Farbe ersetzen, Rote Augen: **keine Wirkung, keine Meldung**, Objekte unverändert (7, R1 bleibt Pfad), Export pixelgleich mit T04 | M, J, E, B `14` |
| 15 | `undo` | Je Zug ein Verlaufsschritt: **„Rückgängig: Stempel“**, **„Rückgängig: Farbe ersetzen“**, **„Rückgängig: Rote Augen entfernen“** (Rechteck); ein wirkungsloser waagrechter Rote-Augen-Zug legt trotzdem **„Bitmap bearbeiten“** an | M, W |

## Hinweise

- Nach einem Skriptlauf heißt der oberste Schritt „Befehlsskript“ – Menüs deshalb vor jedem Skript gelesen.
- Esc nach einem Zug beendet den Bitmapmodus, Werkzeug → Zeiger (wie Einheit 03).
- Annahme: „Gesamtes Dokument“ ist die Option „alle Ebenen verwenden“ (nicht gesondert gemessen, T02 hat nur eine Ebene).
