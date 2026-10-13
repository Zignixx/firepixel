# 09 `tool-line-pen` – Vergleich Original ↔ Firepixel (PAR-F-08-10, 13.10.2026)

Firepixel: Debug-Build `main` 2906afc (Slot fp-s3), eigene Instanz, `--mcp`, Fenster 1600 × 1000, Design „normal“, Zoom 100 %;
**echtes Fenster, Brücke** (Klicks mit 450 ms Pause, Ziehen 30 Schritte, Knoten per `run_script`, `history`, `export`). T01/T04 je Messung
frisch, Füllung #808080 / Strich #C04040; Stift-Ergebnisse nach Werkzeugwechsel auf Zeiger gelesen (Firepixel legt den Pfad erst beim
Beenden an). Bilder `docs\parity\img\tool-line-pen\`, Vergleich `re\_testdateien\par\cmp\tool-line-pen\`.

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | line_draw | Pfad **„Linie“**, Knoten **auf Pixelmitten** 100.5,100.5 / 300.5,150.5; Umschalt waagrecht, 45° mit Länge = größere Komponente (100,300→200,390 → **200.5,400.5**), senkrecht; bleibt gewählt, Werkzeug bleibt | Name **leer**; Knoten **ganzzahlig** 100,100 / 300,150; 45°-Ende **195,395** (Länge = Mittel/Projektion); waagrecht/senkrecht, gewählt, Werkzeug, Schritt „Linie“ ✅ | ❌ | PAR 09/01 |
| 02 | line_inspector | Kopf „Linie“, nur Strichteil (Farbe, 1, Strichart, Rand 0, Textur Ader 0, „Pinselstrich bearbeiten“), Deckkraft 100, Normal | gleich aufgebaut (Strichart „1-Pixel hart“ statt „Präzise abger.“ – Vorgabe des Profils, s. 17) | ✅ | – |
| 03 | pen_corner | Klicks = Eckpunkte, Gummiband zur Maus | Eckpunkte ✅ (Bild `03_`) | ✅ | – |
| 04 | pen_curve | Drücken 200,300 → 250,330: 200,300 [150,270 \| 250,330] | identisch | ✅ | – |
| 05 | pen_close | Klick auf Start schließt, Füllung #808080 + Strich; Schritt **„Stift“** | geschlossen, gefüllt ✅; Schritt **„Pfad“** | ❌ | PAR 09/05 |
| 06 | pen_dblclick | Doppelklick setzt letzten Punkt und beendet; nächste Klicks neuer Pfad | gleich | ✅ | PAR 09/06 (Nebenbefund) |
| 07 | pen_esc_enter | Esc/Enter beenden sichtbar, **offener Pfad bleibt gewählt und wird beim nächsten Klick am Endpunkt fortgesetzt** (auch nach 2 × Esc); Strg+D → neuer Pfad | Esc, Enter und 2 × Esc beginnen jeweils einen **neuen Pfad**; Strg+D ✅ | ❌ | PAR 09/07 |
| 08 | pen_continue | P1 **nicht gewählt**: Klick auf Endpunkt 420,240, dann 480,280 → P1 bekommt 5. Knoten | **neuer Pfad** 420,240–480,280, P1 unverändert | ❌ | PAR 09/08 |
| 09 | pen_alt_handle | Alt-Ziehen neuer Punkt: nur Ausgangshenkel 200,450 [200,450 \| 250,480]; Alt-Ziehen auf dem soeben gesetzten Punkt ändert dessen Ausgangshenkel (200,445 \| 200,520) | neuer Punkt identisch ✅; auf dem soeben gesetzten Punkt entsteht ein **zweiter Knoten** 200,450 (4 statt 3 Knoten) | ❌ | PAR 09/09 |
| 10 | pen_add_delete | Klick auf Segment fügt Punkt ein (Henkel 235.18,169.88 \| 262.65,170.12); Klick auf inneren Kurvenpunkt **wandelt in Eckpunkt** (Henkel eingezogen); Schritt jeweils „Stift“ | Einfügen **identisch**; Klick auf Punkt **löscht** ihn (4 Knoten); Schritte „Punkt einfügen“ / „Punkt entfernen“ | ❌ | PAR 09/10 |
| 11 | pen_snap | **kein Fang**: 102,207 bleibt, 152,122 bleibt | rastet: 102,207 → **102,205** (E1-Mitte), 152,122 → **150,120** (R1-Ecke) | ❌ | PAR 09/11 |
| 12 | vector_path | Zickzack 7 Ecken → **8 Eckknoten** auf ,5, offen, ohne Füllung; Schritt **„Pinsel“** | **7 Kurvenknoten** (Ecken gerundet, Henkel ±16,67), offen ✅; Schritt **„Pfad“**; Δ Export 603 px | ❌ | PAR 09/12 |
| 13 | vector_path_brush | „Soft Rounded“ 12 px, Weichheit 50 → 5 Knoten, weicher Rand | nicht gemessen (Strichart über `pi.tool.brush` nicht setzbar, Name unbekannt) | – | offen |
| 14 | redraw_path | P1 von 290,200 über 330,260 bis 370,200: **nur Abschnitt ersetzt**, P1 hat 10 Knoten; Schritt „Pfad nachzeichnen“ | **ganzer P1 ersetzt** durch 3 Knoten 290,200 / 330,260 / 370,200; Schritt ✅ | ❌ | PAR 09/14 |
| 15 | points_display | Punkte beim Zeichnen / gewählter Pfad mit Stift (Bilder) | Bilder `15_*` vorhanden (Kennzahl in `summary.md`) | ✅ | Bild |
| 16 | cursor | 5 Stift-Zeiger, Linie Fadenkreuz | nicht aufgenommen (Brücke ohne Mauszeiger; Bilder `16_*` zeigen nur die Lage) | – | offen |
| 17 | stroke_defaults | Strich aus Werkzeug/Dokument, Füllung nur bei geschlossenen Stiftpfaden | Füllung nur geschlossen ✅; Strich „1-Pixel hart“ #C04040 | ✅ | Frage 1 |
| 18 | undo | „Linie“, „Stift“ (auch Einfügen/Umwandeln), „Pinsel“, „Pfad nachzeichnen“ | „Linie“ ✅, „Pfad nachzeichnen“ ✅; Stift „Pfad“, „Punkt einfügen“/„Punkt entfernen“, Vektorpfad „Pfad“ | ❌ | 05/10/12 |

**Zählung:** ✅ 6 · ❌ 10 · 🎨 0 · offen 2 (13 Strichart, 16 Mauszeiger).

## Hinweise / Annahmen / Fragen

- **Neuer Fehler beim Messen (06/07):** Ein unbeendeter Stiftpfad wandert in das **nächste geöffnete Dokument** (Punkte 400,100/500,100
  aus Dokument A erschienen nach Schließen/Öffnen als Anfang eines Pfads in Dokument B). Als Befund PAR 09/06 eingetragen.
- Firepixel legt den Stiftpfad erst beim Beenden ins Dokument; das Original führt ihn nach jedem Klick als Objekt (JSF sieht ihn sofort).
- **Frage 1:** Strichart-Vorgabe neuer Pfade im frischen Profil ist „1-Pixel hart“, im Original „Präzise abger.“ (Hard Rounded) –
  Vorgabe angleichen? (Bei Rechteck/Ellipse/Polygon fehlt der Strich ganz, siehe tool-shapes/12.)
- 12: Zug als Polylinie mit 10 Zwischenpunkten je Strecke (Original: echter Mauszug derselben Ecken).
