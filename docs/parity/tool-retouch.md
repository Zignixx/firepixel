# 04 `tool-retouch` – Vergleich Original ↔ Firepixel (PAR-F-03-04, 13.10.2026)

Firepixel: Debug-Build `main` 236e35e (Slot fp-s2), eigene Instanz mit eigenem Profil und `--mcp`, Fenster 1600 × 1000,
Design „normal“, Zoom 100 %; echtes Fenster, Brücke (`ui_input` Ziehen 30 Schritte à 15 ms, Felder per Klick + Tippen + Enter,
`ui_find`, `history`, `export`). T02 je Messung frisch, **B1 gewählt**; Standardzug 50,120 → 510,120. Kennzahlen wie
`pixels.csv` (geänderte Pixel gegen T02, Grenzen, mittlere/größte Abweichung, Zeile 120 bei x 150/300/318/322/340/358/362/380).
Ausgangsexport T02 pixelgleich mit dem Original. Bilder `docs\parity\img\tool-retouch\`, Vergleich
`re\_testdateien\par\cmp\tool-retouch\` (14 Paare).

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | blur_inspector | „Weichzeichner“: Größe 13, Form rund/eckig, Rand **weiche Vorschau** + 100, Intensität 50; Scharfzeichner gleich | Felder, Werte, Lage gleich (± 3 px); **Rand-Vorschau bei 100 ein harter kleiner Punkt** (auch Abwedler/Nachbelichter) | ✅ gegengeprüft 13.10. (PAR-R-1: Rand 100 weich auslaufend, 50 halbweich, 0 harter Kreis; Vorschau etwas größer als im Original) | PAR 04/01 |
| 02 | blur_result | 222 px, 278,113–482,127, mittl. 9,5, max 29; Verlauf unverändert | 258 px, 278,113–482,127, mittl. 10,3, max 41; Verlauf unverändert; Δ Orig 15 px > 16 | ✅ | Hinweis |
| 03 | blur_intensity | 10 → 156 px, max 6; 100 → **1 722 px** (auch weicher Verlauf x 47…), max 53 | 10 → 168 px, max 10 ✅; 100 → **330 px**, nur Kanten (279…483), max 67 | ✅ gegengeprüft 13.10. (PAR-R-1: 10 → 156 px, max 4; 100 → 300 px, max 40; je 0 px > 16 gegen Original) | PAR 04/03 |
| 04 | sharpen_result | 40 px, 279,113–440,127, max 12 | **1 521 px**, 48,113–440,127 (auch Verlauf), max **95** | ✅ gegengeprüft 13.10. (PAR-R-1: 42 px, 279,113–440,127, max 12; 0 px > 16) | PAR 04/04 |
| 05 | dodge_inspector | „Abwedler“: 13, Form, Rand 100, Bereich Schatten · Mitteltonfarben · Lichter (Vorgabe Mitteltonfarben), Belichtung 50; Nachbelichter gleich | gleich; Liste öffnet nach oben mit denselben 3 Einträgen; Auswahl wirkt (Ergebnisse verschieden) | ✅ | – |
| 06 | dodge_ranges | Schatten: Schwarz → #3F3F3F, Rot → #FF3F3F, Verlauf → #A3A1A7. Mitteltöne: nur Verlauf → #A09DA4 (Schwarz/Weiß/Rot gleich). Lichter: nur Verlauf → #B3AFBA | Schatten: Schwarz → **#5B5B5B**, Rot → #FF3737, Verlauf → **#949299**. Mitteltöne: Verlauf → **#BDBCC0**, **Rot → #FF7272**. Lichter: Verlauf → **#9B989F** (dunkler als Mitteltöne), **Rot → #FF0E0E** | ✅ gegengeprüft 13.10. (PAR-R-1: Zeile 120 je Bereich gleich (Schatten #A3A1A7/#3F3F3F/#FF3F3F, Mitteltöne #A09DA4, Lichter #B3AFBA); 0 px > 16) | PAR 04/06 |
| 07 | burn_ranges | Schatten: nur Verlauf → #5B5762. Mitteltöne: nur Verlauf → #65626B. Lichter: Weiß → #BFBFBF, Rot → #BF0000, Verlauf → #646268 | Schatten: Verlauf → #726F76, **Rot → #BA0000**. Mitteltöne: Verlauf → **#49484C**, **Rot → #A30000**. Lichter: **Weiß → #ABABAB, Rot → #F30000**, Verlauf → #737178 | ✅ gegengeprüft 13.10. (PAR-R-1: Zeile 120 je Bereich gleich (#5B5762, #65626B, Lichter #646268/#BFBFBF/#BF0000); höchstens 1 px > 16) | PAR 04/06 |
| 08 | smudge_inspector | „Verwischen“: Größe 22, Form, Rand 50, Druck 50, Farbfeld + „Farbe verwischen“, „Gesamtes Dokument“; beide aus | gleich (Lage ± 3 px); Farbfeld zeigt gespeicherten Stand (Original grün, Firepixel grau – Profilwert, kein Befund) | ✅ | – |
| 09 | smudge_result | 6 391 px, 40,109–506,131, mittl. 36,6, max 254; Z. 120: x 150 unverändert, x 300 #241B01, x 322 #040404, x 340 #C9C9C9, x 380 #FF2424 | **8 310 px**, 42,109–510,132, mittl. 26,0; Z. 120: **x 150 #797996** (Verlauf verändert), x 300 #322A05, x 322 #090909, x 340 #CDCDCD, x 380 #FF3232 | ✅ gegengeprüft 13.10. (PAR-R-1: Verlauf unverändert (x 150 #85828A), Zeile 120 #282100/#030303/#D7D7D7/#FF2828; Rest 416 px > 16 (max 59) in der Übergangsform, wie im PAR-X-Bericht als Grenze genannt) | PAR 04/09 |
| 10 | smudge_color | „Farbe verwischen“ an: 8 336 px, Farbe des Felds eingemischt (x 150 → #76828A) | Kästchen schaltet, Ergebnis **pixelgleich mit 09** (Befehl `bitmap.retouch` ohne Farbparameter) | ✅ gegengeprüft 13.10. (PAR-R-1: Ergebnis anders als 09 (1 134 px > 16), Option wirkt; Farbwert des Originals unbekannt) | PAR 04/10 |
| 11 | smudge_all_layers | „Gesamtes Dokument“ an: 3 130 px, 40,109–521,131, nimmt weiße Leinwand auf | Kästchen schaltet, Ergebnis **pixelgleich mit 09** (ohne Wirkung) | ✅ gegengeprüft 13.10. (PAR-R-1: anders als 09 (Rand bei x 40); Original weicht von seinem 09 auch nur ≤ 16 ab; gegen Original 416 px > 16 wie 09) | PAR 04/10 |
| 12 | repeat_stroke | zweimal: 274 px, max 49 (einmal 222/29) – verstärkt | zweimal: 348 px, max 63 (einmal 258/41) – verstärkt | ✅ | – |
| 13 | shape_square | eckig: 178 px, **279,114–481,127** (kleiner als rund) | eckig: 276 px, **278,113–482,127** (größer als rund, 14 statt 13 Zeilen) | ✅ gegengeprüft 13.10. (PAR-R-1: eckig 156 px, 279,114–481,127 wie Original; 0 px > 16) | PAR 03/09 |
| 14 | edge | Rand 0: 235 px, 278,114–482,127, max 29 (Rand 100: 222) | Rand 0: **348 px**, 278,113–483,127, max **61** (Rand 100: 258) | ✅ gegengeprüft 13.10. (PAR-R-1: Rand 0: 156 px, 279,114–481,127, max 21; 0 px > 16) | PAR 04/14 |
| 15 | vector_target | R1 gewählt: keine Wirkung, keine Meldung, **Auswahl danach leer** | keine Wirkung, keine Meldung, kein Verlaufsschritt; **R1 bleibt gewählt** | ✅ gegengeprüft 13.10. (PAR-R-1: keine Wirkung, kein Schritt, keine Meldung, Auswahl danach leer) | PAR 04/15 |
| 16 | live | Halbzeit: Weichzeichner x 274–287, Verwischen x 269–291 | Halbzeit: Änderung bis x 287 (Weichzeichner) bzw. x 291 (Verwischen) | ✅ | – |
| 17 | cursor | nicht aufgenommen (offen) | nicht aufgenommen | – | offen |
| 18 | undo | ein Zug = ein Schritt, Name = Werkzeug („Weichzeichner“, „Scharfzeichner“, „Abwedler“, „Nachbelichter“, „Verwischen“); Strg+Z → Ausgangsbild | ein Schritt ✅, Strg+Z → Hash bf663a48136ea54c ✅; Name bei **allen fünf „Retusche“** | ✅ gegengeprüft 13.10. (PAR-R-1: Schritte „Weichzeichner“, „Scharfzeichner“, „Abwedler“, „Nachbelichter“, „Verwischen“) | PAR 04/18 |

**Zählung:** ✅ 17 · ❌ 0 · 🎨 0 · offen 1 (Mauszeiger) – nach Gegenprüfung PAR-R-1 13.10. (12 behobene Zeilen geprüft: 12 ✅, 0 ❌).

## Hinweise / Annahmen

- 02: Firepixel weicht an den harten Kanten etwas stärker (max 41 statt 29); als ✅ mit Hinweis gewertet, beim Beheben von
  04/03 und 04/14 (gleicher Weichzeichnerkern) mitprüfen.
- 06/07: Firepixel verändert in jedem Bereich auch gesättigte Farben (Rot) und die Bereiche sind nicht wie im Original
  getrennt (Mitteltöne lassen im Original Schwarz/Weiß/Rot unverändert, Lichter wirken bei Abwedeln nur auf den hellen Verlauf).
  Original-Ergebnisse: `re\_testdateien\par\px\tool-retouch\b06_dodge_k*.png`, `b07_burn_k*.png`.
- 10: Farbwert des Original-Farbfelds war nicht auslesbar (Bild zeigt grün); verglichen wird nur, ob die Option wirkt.
- 13 und Radiergummi „eckig“ (03/09) haben dieselbe Ursache vermutlich im gemeinsamen Spitzenraster (Größe 13 → 14 Zeilen,
  weiche Kante auch bei „eckig“) – Befund unter PAR 03/09 mitgeführt.
- Mauszeiger (17) wie im Original nicht aufnehmbar.
