# 09 `tool-line-pen` – Linie, Stift, Vektorpfad, Pfad nachzeichnen: Aufnahme des Originals (PAR-O-08-10, 13.10.2026 ≈ 07:50–08:20)

Testdateien T01, T04 (P1) als Arbeitskopien `re\_testdateien\par\work\p810_t0*.fw.png`, je Messung frisch mit `fw.openDocument(url)`.
Fenster 1600 × 1000, Zoom 100 %, Leinwand bei 297,183, Füllung #808080, Strich #C04040 1 px („Hard Rounded“). **Echte Klicks und
Züge**, danach Knoten per Skript (`x,y[pred|succ]`). Referenzbilder `re\_testdateien\par\ref\tool-line-pen\` (+ `INDEX.md`), Exporte
`re\_testdateien\par\px\tool-line-pen\pixels.csv`, alle Knoten `paths.json`. Art wie Einheit 08 (M/J/W/E/B).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `line_draw` | Linie 100,100→300,150: Pfad „Linie“ mit 2 Knoten **auf Pixelmitten** (100.5,100.5 / 300.5,150.5), pixelRect 100,100–301,151. Umschalt rastet waagrecht / 45° (100,300→200,390 ergibt Ende 200.5,400.5: Länge = größere Komponente) / senkrecht. Neue Linie bleibt gewählt, Werkzeug bleibt. 1 px hart gerendert (nur 2 Farben). Inspektor danach „Pfad“ Linie: B 1 H 200 X **401** Y **101** (senkrechte Linie bei 400.5) | M, J, E, B `01` |
| 02 | `line_inspector` | Kopf „Linie“; nur Strichteil: Farbfeld, Größe 1 + ▼, „Präzise abger.“, Rand 0 + ▼, Textur „Ader“ 0, „Pinselstrich bearbeiten“ + 2 Knöpfe; Deckkraft 100, Mischmodus Normal (55). **Keine Füllung**, kein B/H/X/Y | W, B `02` |
| 03 | `pen_corner` | Klicks = Eckpunkte (Henkel auf dem Punkt). Zwischen den Klicks Gummiband-Vorschau zur Maus | M, J, B `03` |
| 04 | `pen_curve` | Drücken bei 200,300 und Ziehen nach 250,330 → Kurvenpunkt, **Ausgangshenkel = Mausposition, Eingangshenkel gespiegelt** (150,270 \| 250,330) | M, J, B `04` |
| 05 | `pen_close` | Über dem Startpunkt eigener Zeiger; Klick schließt → **geschlossener Pfad mit Füllung #808080** (Vollton) + Strich; ohne Klick auf den Start bleibt der Pfad offen **ohne Füllung**. Schritt „Stift“ | M, J, B `05` |
| 06 | `pen_dblclick` | Doppelklick setzt den letzten Punkt (einmal) und **beendet** den Pfad; nächste Klicks beginnen einen neuen Pfad | M, J |
| 07 | `pen_esc_enter` | Esc bzw. Enter beenden den Zug sichtbar, **aber der offene Pfad bleibt gewählt, und der nächste Stiftklick setzt ihn am Endpunkt fort** (auch nach 2 × Esc). Erst Auswahl aufheben (Strg+D) oder Doppelklick → neuer Pfad. Ein Skriptlauf mitten im Zeichnen ließ nur den ersten Punkt übrig | M, J |
| 08 | `pen_continue` | P1 **nicht gewählt**: Klick auf seinen Endpunkt 420,240 (eigener Zeiger) und dann 480,280 → P1 bekommt 5. Knoten (fortgesetzt, kein neuer Pfad) | M, J, B `16` |
| 09 | `pen_alt_handle` | Neuer Punkt mit **Alt-Ziehen**: nur **Ausgangshenkel** (200,450 [200,450 \| 250,480]). Alt-Ziehen auf dem soeben gesetzten Kurvenpunkt: Ausgangshenkel neu (200,520), Eingangshenkel bleibt fast (200,445) | M, J, B `09` |
| 10 | `pen_add_delete` | P1 gewählt, Klick **auf Segment** (249,170) → **Punkt eingefügt**, Kurvenform bleibt (Henkel neu berechnet). Klick auf **inneren Kurvenpunkt** 370,200 → **nicht gelöscht, sondern in Eckpunkt umgewandelt** (Henkel eingezogen). Schritt jeweils „Stift“. Eigene Zeiger + / − | M, J, B `16` |
| 11 | `pen_snap` | **Kein Fang** des Stifts an Objektmitte/-ecke: Klick 102,207 (E1-Mitte 100,205) bleibt 102,207; 152,122 (R1-Ecke) bleibt 152,122 | M, J |
| 12 | `vector_path` | Vektorpfad (Genauigkeit 5) Zickzack mit 7 Ecken → **8 Eckknoten** auf Pixelmitten (eine Spitze in 2 Knoten 295.5/305.5 aufgeteilt), offen, ohne Füllung; Schritt **„Pinsel“** | M, J, E, B `12` |
| 13 | `vector_path_brush` | Mit Strich „Soft Rounded“ 12 px, Weichheit 50: gleicher Zug → 5 Knoten mit Kurven, Rand weich (114 Farben, 95,245–407,307) | M, E, B `13` |
| 14 | `redraw_path` | Pfad nachzeichnen auf P1 von Knoten 290,200 über 330,260 bis 370,200: Abschnitt ersetzt, P1 hat danach **10 Knoten** (neue Punkte auf ,5, Doppelknoten an den Nahtstellen), Pfad bleibt offen; Schritt **„Pfad nachzeichnen“** | M, J, B `14` |
| 15 | `points_display` | Punktdarstellung beim Zeichnen und beim gewählten Pfad (Stift aktiv) nur als Referenzbild aufgenommen (Aussehen dort) | B `15` (2) |
| 16 | `cursor` | Laut Zeiger-Handle **5 verschiedene Stift-Zeiger** (normal, Schließen über Startpunkt, Fortsetzen über Endpunkt, + über Segment, − über Punkt; Hotspot 5,2 bzw. 6,2). Linie = Fadenkreuz (Hotspot 10,9, wie Rechteck/Ellipse/Polygon); Vektorpfad und Pfad nachzeichnen teilen einen eigenen Zeiger (10,9) | B `16` (6) |
| 17 | `stroke_defaults` | Neue Pfade: Strich aus Werkzeug/Dokument (#C04040, „Hard Rounded“ 1), Füllung **nur bei geschlossenen** Stiftpfaden (#808080 Vollton) | J |
| 18 | `undo` | „Linie“, „Stift“ (auch Punkt einfügen/umwandeln), „Pinsel“ (Vektorpfad), „Pfad nachzeichnen“ | M, W |

## Hinweise

- Inspektor Stift: links Füllung (Farbfeld + 4 Füllart-Knöpfe, Vollton gedrückt; Rand Anti-Alias, Wert grau; Textur Ader 0, „Transparent“ grau), Mitte Strich wie Linie, rechts Deckkraft/Mischmodus. Vektorpfad/Pfad nachzeichnen: zusätzlich **„Genauigkeit: 5“** + ▼ und rechts „Kombinieren“ (grau), keine Füllung.
- Beim Pfad nachzeichnen meldete sich vor dem Zug einmal „Skript konnte nicht ausgeführt werden.“ (vom Steuerskript, per Abbrechen geschlossen).
