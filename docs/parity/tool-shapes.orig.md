# 10 `tool-shapes` – Rechteck, Ellipse, Polygon: Aufnahme des Originals (PAR-O-08-10, 13.10.2026 ≈ 08:05–08:35)

Testdatei T01 (Arbeitskopie `re\_testdateien\par\work\p810_t01.fw.png`, je Messung frisch mit `fw.openDocument(url)`).
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei 297,183, Füllung #808080, Strich #C04040 1 px („Hard Rounded“). **Echte Züge**, Werte
danach per Skript (Objektart, `pixelRect`, `roundness`, `originalSides`, Knoten). Referenzbilder `re\_testdateien\par\ref\tool-shapes\`
(+ `INDEX.md`), Export `re\_testdateien\par\px\tool-shapes\pixels.csv`, Knoten `paths.json`. Art wie Einheit 08 (M/J/W/E/B).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `rect_draw` | Ziehen 100,100→250,200 → **Rechteck-Grundform** (eigene Objektart, nicht Pfad), sides 100,100–250,200, Rundung 0, Füllung #808080 Vollton + Strich 1 px; bleibt gewählt, Werkzeug bleibt. Vorschau beim Ziehen = Umriss | M, J, B `01` (2) |
| 02 | `rect_shift` | Umschalt = Quadrat, **Kante = größere Komponente** (300,100→400,160 → 300,100–400,200) | M, J |
| 03 | `rect_alt` | Alt = **von der Mitte** (150,350→200,380 → 100,320–200,380); Umschalt+Alt = Quadrat von der Mitte (450,350 → 400,300–500,400) | M, J |
| 04 | `rect_click` | Einzelklick ohne Ziehen erzeugt **kein** Objekt | M, J |
| 05 | `rect_arrows` | Während des Ziehens: **↑ erhöht die Eckrundung, solange gehalten** (1,5 s gehalten → 0,97 = 97 %; kurze Tipper ≈ 0–1 %), ↓ verringert; ← → ohne Wirkung. **Esc bricht das Aufziehen nicht ab** (Rechteck entsteht beim Loslassen) | M, J |
| 06 | `rect_corner_handles` | Gewähltes Rechteck mit Zeiger: 4 Eckpunkte, eigener Zeiger darauf (Hotspot 4,4). **Eckpunkt ziehen (100,100→130,100) = Kante verschieben** (130,100–300,250), Rundung bleibt 0, Schritt **„Rechteck modifizieren“**. Mit Teilauswahl: Ziehen am Eckpunkt **verschiebt das ganze Rechteck** (Matrix −20,0; „Verschieben“). Keine eigenen Rundungsgriffe gesehen | M, J, B `06` (3) |
| 07 | `rect_inspector` | **Werkzeug** Rechteck: Kopf „Rechteck“, B/H/X/Y **leer**, Füllung (Farbfeld, 4 Füllart-Knöpfe, Vollton gedrückt, Rand Anti-Alias + Wert grau, Textur Ader 0, „Transparent“ grau), Strich (1, Präzise abger., Rand 0, Textur Ader 0), Deckkraft 100, Normal, „Kombinieren“ + 5 Pfadknöpfe grau. **„Eckrundung des Rechtecks:“ 0 + ▼ vorhanden, aber unsichtbar** (W23 bestätigt). **Gewähltes** Rechteck: Name „Rechteck“, B/H/X/Y, 3 Kantenknöpfe, **„Rundung:“ 0 + ▼ + Liste % / px**, Filter-Liste, Stil „Kein Stil“ | W, B `07` (2) |
| 08 | `ellipse` | Ellipse 100,400→250,500 → **Pfad** „Ellipse“ mit 4 Kurvenknoten (Henkel 0,552 × Halbachse: 100,450 [100,477.61 \| 100,422.39] …), geschlossen, gefüllt. Umschalt = Kreis mit **größerer Komponente** (300,400→380,450 → 300,400–380,480) | M, J, B `08` |
| 09 | `polygon_inspector` | Kopf „Polygon“; rechts **„Form:“** Liste **Stern / Polygon** (Vorgabe Polygon), **„Seiten:“ 5** + ▼, **„Winkel:“ 38** + ▼, **„Autom.“ ☑**. Winkel und Autom. auch bei Form Polygon sichtbar und aktiv. Autom. ☑ setzt Winkel automatisch: 5 Seiten → 38, 6 Seiten → 57. Seiten-Feld nimmt 0/2/400 als Text an. Winkel-Feld manuell: „20“ übernommen, „38“ → 3, „99“ → 9 (zweite Ziffer verworfen); nach „100“ war das Werkzeug „Hintere auswählen“ (übrige Ziffern als Kürzel an die Leinwand) | W, M, B `09` (3) |
| 10 | `polygon_draw` | Ziehen **von der Mitte**; **ein Eckpunkt liegt in Zugrichtung** (Drehung folgt der Maus): 6 Seiten, 200,150→260,150 → Ecke 260,150, r = 60 (140,98–260,202); nach unten → Ecke 400,210; schräg 45° → Ecke 650,200. Ergebnis Pfad „Polygon“, geschlossen, gefüllt | M, J, B `10` (2) |
| 11 | `star_draw` | Form Stern (per Klick in die Liste), 5 Seiten, Autom. (38): **10 Knoten**, Außenradius = Zuglänge (70), Innenradius 26,7 (≈ 0,38 × außen), Spitze in Zugrichtung (270,150); Name ebenfalls „Polygon“. Winkel 20 manuell (Autom. aus) → Stern 10 Knoten, gleiche Außenmaße. Setzen der Liste nur per Fensternachricht wirkt **nicht** aufs Werkzeug | M, J, B `11` |
| 12 | `fill_stroke` | Alle Formen: Füllung Vollton #808080 + Strich #C04040 „Hard Rounded“ 1 px aus den Vorgaben, Deckkraft 100 | J, E |
| 13 | `snap_start` | Startpunkt rastet an Objektecken (intelligente Fanglinien, Vorgabe an): Start 203,103 neben Rechteck 100,100–200,200 → **200,100** | M, J |
| 14 | `cursor` | Rechteck, Ellipse, Polygon: **gleiches Fadenkreuz** (Hotspot 10,9; auch Linie) | B `14` (3) |
| 15 | `info_live` | Inspektor zeigt **während des Aufziehens live** „Rechteck“ B 75 H 50 X 100 Y 100 (Hälfte des Zugs) | W, B `15` |
| 16 | `undo` | „Rechteck“, „Ellipse“, „Polygon“ (auch Stern), „Rechteck modifizieren“ (Eckpunkt mit Zeiger) | M, W |

## Hinweise

- Export Rechteck + Ellipse + Fünfeck (`s_rect_ellipse_poly.png`): Ellipse und Polygon mit Anti-Alias-Füllung, Strich hart (21 Farben).
- Werkzeugeinstellungen des Polygons wurden am Ende auf die Vorgabe zurückgesetzt (Polygon, 5, 38, Autom. ☑) und geprüft (Fünfeck 5 Knoten).
- Bedienfelder waren nach einem Prüflauf (u. a. Tab-Taste) ausgeblendet; mit Tab wieder eingeblendet, Leinwandlage danach geprüft (297,183).
