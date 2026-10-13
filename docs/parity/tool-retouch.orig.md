# 04 `tool-retouch` – Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen: Original (PAR-O-03-04, 13.10.2026 ≈ 07:00–07:30)

Test T02 (Arbeitskopie `re\_testdateien\par\work\p34_t02.fw.png`, je Messung frisch geöffnet), **B1 gewählt**. Fenster
1600 × 1000, Zoom 100 %, Leinwand bei 297,183. Werkzeug per JSF (`fw.activeTool = "Weichzeichner"` usw.), Werte im Inspektor
per Maus/Tastatur. Standardzug (echte Maus, 30 Schritte à 15 ms) **50,120 → 510,120** quer über B1: Verlauf (x 40–280,
weich) und harte Streifen schwarz/weiß/rot/grün/blau/gelb (je 40 px ab x 280). Exporte + Hashes
`re\_testdateien\par\px\tool-retouch\` (`pixels.csv`: geänderte Pixel, Grenzen, mittlere/größte Abweichung, Pixel in Zeile
120 bei x 150/300/318/322/340/358/362/380; Ausgangswerte #85828A #000000 #000000 #FFFFFF #FFFFFF #FFFFFF #FF0000 #FF0000).
Bilder `re\_testdateien\par\ref\tool-retouch\` (+ `INDEX.md`). Inspektorwerte = gespeicherter Stand beim Start.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `blur_inspector` | Überschrift „Weichzeichner“. **„Größe:“** 13 (Feld 280–312 × 894–915 + ▼), **„Form:“** rund/eckig (Bildknöpfe 423–446 / 447–470), **„Rand:“** Vorschau + 100 (+ ▼), **„Intensität:“** 50 (Feld 423–455 × 918–939 + ▼). Scharfzeichner identisch (Intensität 50) | W, B `01`, `04` |
| 02 | `blur_result` | Vorgabe: nur an harten Kanten verändert – 222 px, 278,113–482,127, mittlere Abw. 9,5, max. 29; weicher Verlauf praktisch unverändert | M, E, B `02` |
| 03 | `blur_intensity` | Intensität 10 → 156 px, max. 6; Intensität 100 → 1 722 px, max. 53 (deutlich stärker) | M, E |
| 04 | `sharpen_result` | Scharfzeichner Vorgabe: 40 px, 279,113–440,127, max. 12 | M, E, B `04` |
| 05 | `dodge_inspector` | Überschrift „Abwedler“: Größe 13, Form rund/eckig, Rand 100, **„Bereich:“** Liste **Schatten · Mitteltonfarben · Lichter** (Vorgabe Mitteltonfarben; 423–527 × 918–939), oben rechts **„Belichtung:“** 50 (Feld 597–629 + ▼). Nachbelichter identisch | W, B `05` (2), `07` |
| 06 | `dodge_ranges` | Belichtung 50. **Schatten:** 6 043 px, max. 63 – Schwarz → #3F3F3F, Rot → #FF3F3F, Verlauf #85828A → #A3A1A7. **Mitteltonfarben:** 3 867 px, max. 32 – Verlauf → #A09DA4, Schwarz/Weiß/Rot unverändert. **Lichter:** 3 861 px, max. 65 – Verlauf → #B3AFBA, Schwarz unverändert | M, E |
| 07 | `burn_ranges` | **Schatten:** 3 861 px – Verlauf → #5B5762. **Mitteltonfarben:** 3 861 px, max. 32 – Verlauf → #65626B. **Lichter:** 6 046 px, max. 64 – Weiß → #BFBFBF, Rot → #BF0000, Verlauf → #646268 | M, E |
| 08 | `smudge_inspector` | Überschrift „Verwischen“: Größe **22**, Form rund/eckig (426–449 / 450–473), Rand **50**, **„Druck:“** 50, Kontrollkästchen **„Farbe verwischen“** (576,896) mit Farbfeld (549–572 × 894–915) und **„Gesamtes Dokument“** (576,920) – nicht „Alle Ebenen verwenden“. Beide Kästchen aus | W, B `08` |
| 09 | `smudge_result` | Vorgabe: 6 391 px, 40,109–506,131, mittlere Abw. 36,6, max. 254 – Farben werden in Zugrichtung mitgezogen (Zeile 120: x 300 #000000 → #241B01, x 322 #FFFFFF → #040404, x 340 → #C9C9C9, x 380 #FF0000 → #FF2424) | M, E, B `09` |
| 10 | `smudge_color` | „Farbe verwischen“ an: 8 336 px, Startfarbe des Farbfelds wird mit eingemischt (Zeile 120 x 150: #85828A → #76828A); Farbwert des Felds per Steuerelement nicht auslesbar | M, E |
| 11 | `smudge_all_layers` | „Gesamtes Dokument“ an: 3 130 px, 40,109–521,131, mittlere Abw. 74,0 – anderes Ergebnis als Vorgabe (nimmt auch die weiße Leinwand außerhalb von B1 auf) | M, E |
| 12 | `repeat_stroke` | Weichzeichner zweimal derselbe Zug: 274 px, max. 49 (einmal: 222 px, max. 29) → zweiter Zug verstärkt | M, E |
| 13 | `shape_square` | Weichzeichner eckig: 178 px, 279,114–481,127 (statt 278,113–482,127 rund) | M, E, B `13` |
| 14 | `edge` | Weichzeichner Rand 0: 235 px, 278,114–482,127, max. 29 (Rand 100: 222 px) | M, E |
| 15 | `vector_target` | T04, R1 (Vektor) gewählt, Weichzeichner-Zug: **keine Wirkung, keine Meldung**, Export pixelgleich mit unverändertem T04, Auswahl danach leer | M, E, J, B `15` |
| 16 | `live` | Wirkung schon während des Ziehens sichtbar (Halbzeit: Weichzeichner verändert x 274–287 an der Kante bei 280, Verwischen x 269–291) | M, B `16` (2) |
| 17 | `cursor` | Bild nicht möglich (Systemzeiger erscheint in dieser Remote-Umgebung nicht im Bildschirmfoto) – offen | – |
| 18 | `undo` | Ein Zug = ein Schritt, Name = Werkzeug: „Rückgängig: Weichzeichner“, „…: Scharfzeichner“, „…: Abwedler“, „…: Nachbelichter“, „…: Verwischen“; ein Strg+Z → Ausgangsbild (0 px, Weichzeichner gemessen) | M, W, E |

## Hinweise

- „Bereich“ per Fensternachricht (CB_SETCURSEL + CBN_SELCHANGE) ändert nur die Anzeige, nicht das Werkzeug (drei gleiche
  Ergebnisse); per Maus/Tastatur gewählt wirkt es (Werte oben). Für Firepixel: Anzeige und Wirkung müssen zusammenhängen.
- Werkzeuge stehen nicht im JSF-Objekt `Tools`, lassen sich aber per `fw.activeTool = "<deutscher Name>"` wählen.
