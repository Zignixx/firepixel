# 10 `tool-shapes` – Vergleich Original ↔ Firepixel (PAR-F-08-10, 13.10.2026)

Firepixel: Debug-Build `main` 2906afc (Slot fp-s3), eigene Instanz, `--mcp`, Fenster 1600 × 1000, Design „normal“, Zoom 100 %;
**echtes Fenster, Brücke** (Ziehen 30 Schritte, Tasten während des Zugs als eigene Ereignisse, Inspektorfelder per `ui_find`/Klick/Tippen,
Zustände per `run_script`, `history`, `export`). T01 je Messung frisch, Füllung #808080 / Strich #C04040. Bilder
`docs\parity\img\tool-shapes\`, Vergleich `re\_testdateien\par\cmp\tool-shapes\`.

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | rect_draw | 100,100→250,200 → Rechteck-Grundform, Rundung 0, Füllung + Strich 1 px, gewählt, Werkzeug bleibt | Grundform 100,100–250,200, Rundung 0, gewählt, Werkzeug bleibt ✅; **kein Strich** (→ 12) | ✅ | Folge 12 |
| 02 | rect_shift | Quadrat, Kante = größere Komponente (300,100–400,200) | gleich | ✅ | – |
| 03 | rect_alt | Alt von der Mitte 100,320–200,380; Umschalt+Alt 400,300–500,400 | gleich | ✅ | – |
| 04 | rect_click | Einzelklick erzeugt nichts | gleich | ✅ | – |
| 05 | rect_arrows | ↑ während des Zugs erhöht Rundung **solange gehalten** (1,5 s → 0,97), kurze Tipper ≈ 0–1 %; ↓ verringert; ← → ohne Wirkung; Esc bricht nicht ab | Tipper 0 ✅, ↓ ✅, Esc ✅; **gehalten 1,5 s → 0** (keine Wiederholung); **← verschiebt** das Rechteck (298,300 statt 300,300) | 🔧 behoben (a5ed533/a0c725f), Gegenprüfung offen | PAR 10/05 |
| 06 | rect_corner_handles | Zeiger: Eckpunkt 100,100→130,100 = Kante verschieben (130,100–300,250), Rundung bleibt, Schritt **„Rechteck modifizieren“**; Teilauswahl: Eckpunkt ziehen **verschiebt das ganze Rechteck** (Matrix −20,0, „Verschieben“) | Zeiger: Ergebnis ✅, Schritt **„Transformieren“**; Teilauswahl: Rechteck wird **zum Pfad**, nur Knoten 100,100 → 80,100, Schritt „Pfad nachzeichnen“ | 🔧 behoben (af8c0b5), Gegenprüfung offen | PAR 10/06 |
| 07 | rect_inspector | Werkzeug: Kopf „Rechteck“, B/H/X/Y leer, Füllung, Strich, Deckkraft, rechts „Kombinieren“ + 5 Pfadknöpfe grau; gewählt: Name **„Rechteck“**, Rundung 0 + %/px, Filter, „Kein Stil“ | Werkzeug: gleich, aber **ohne „Kombinieren“/Pfadknöpfe**; gewählt: Rundung/Filter/Stil ✅, **Namensfeld leer**, Strich „Ohne“ | 🔧 behoben (a0c725f, teilweise), Gegenprüfung offen | PAR 10/07 |
| 08 | ellipse | Pfad **„Ellipse“**, 4 Kurvenknoten 100,450 [100,477.61 \| 100,422.39] …; Umschalt Kreis 300,400–380,480 | Knoten **identisch**, Kreis ✅, Schritt „Ellipse“ ✅; Name **leer** | 🔧 behoben (a5ed533), Gegenprüfung offen | PAR 10/08 |
| 09 | polygon_inspector | Form Stern/Polygon, Seiten 5, Winkel 38, Autom. ☑; **Autom. setzt Winkel: 6 Seiten → 57**; Seitenfeld nimmt 0/2/400 an; Winkel „38“→3, „99“→9 | Felder und Vorgaben ✅; **6 Seiten → Winkel bleibt 38**; Seiten wird auf **3…360 begrenzt**; Winkel nimmt 20/38/99 vollständig | 🔧 behoben (a0c725f), Gegenprüfung offen | PAR 10/09, Frage 2 |
| 10 | polygon_draw | von der Mitte, eine Ecke in Zugrichtung: 6 Seiten rechts → 260,150 (140,98–260,202), unten → 400,210, 45° → **650,200** | rechts und unten **identisch**; 45° → 650,**201.96** (Radius 72,1 statt 70,7) | 🔧 behoben (a5ed533), Gegenprüfung offen | PAR 10/10 |
| 11 | star_draw | Stern 5, Autom.: 10 Knoten, außen 70, innen 26,7, Spitze 270,150; Name „Polygon“ | Knoten **identisch** (221.63,165.72 …), Schritt „Polygon“ ✅; Name leer (→ 08) | ✅ | – |
| 12 | fill_stroke | Füllung #808080 + **Strich #C04040 „Hard Rounded“ 1 px** aus den Vorgaben (Export 21 Farben) | Füllung ✅, **kein Strich** bei Rechteck/Ellipse/Polygon (Werkzeug-Inspektor zeigt „1-Pixel hart“ 1); Kante 175,100 #808080 statt #C04040 | 🔧 behoben (a5ed533), Gegenprüfung offen | PAR 10/12 |
| 13 | snap_start | Start 203,103 rastet an Ecke 200,100 | gleich (200,100–300,180) | ✅ | – |
| 14 | cursor | gleiches Fadenkreuz | nicht aufgenommen (Brücke ohne Mauszeiger) | – | offen |
| 15 | info_live | Inspektor **während des Zugs live** „Rechteck“ B 75 H 50 X 100 Y 100 | Felder **leer** während des Zugs (Statuszeile zeigt 150 × 100) | 🔧 behoben (a5ed533/a0c725f), Gegenprüfung offen | PAR 10/15 |
| 16 | undo | „Rechteck“, „Ellipse“, „Polygon“, „Rechteck modifizieren“ | erste drei ✅; Eckpunkt mit Zeiger „Transformieren“ (→ 06) | 🔧 behoben (af8c0b5), Gegenprüfung offen | Folge 06 |

**Zählung:** ✅ 6 · ❌ 9 · 🎨 0 · offen 1 (Mauszeiger).

## Hinweise / Annahmen / Fragen

- 05: Halten als `key press` + 1,5 s + `release` über die Brücke (ohne Betriebssystem-Tastenwiederholung); Firepixel reagiert nur auf
  Tastendrücke. Ob echte Wiederholung über winit wirkt, bleibt für PAR-R mit echter Tastatur offen.
- 09: Felder ohne Auswahl bedient; mit gewähltem Polygon zeigt der Inspektor das Objekt, getippte Ziffern gingen dann als Kürzel an die
  Leinwand (→ „Hintere auswählen“, wie im Original bei „100“).
- **Frage 2:** Original nimmt im Seitenfeld 0/2/400 als Text an und zeigt im Winkelfeld nach zwei Ziffern nur die erste – Firepixel begrenzt
  sinnvoll (3–360) und nimmt Winkel vollständig. Als 🎨 belassen oder angleichen? (Bewertung hier nur wegen Autom.-Winkel ❌.)
- 12: Ursache vermutlich Strichvorgabe der Form-Werkzeuge (Linie/Stift bekommen den Strich); Export Δ 0,26 % nur an den Kanten.
- PAR-X-TOOLS-3 (13.10.): **Frage 3 (Widerspruch der Aufnahmen):** 07 verlangt im Werkzeug-Inspektor Rechteck „Kombinieren“ + 5 Pfadknöpfe (grau), `inspector-fill`/21 (PAR 18/21, schon behoben) dagegen **keine** Pfadoperationen – nicht geändert, nur der Name „Rechteck“ im Feld der gewählten Grundform ist umgesetzt. 05: Rundung wächst mit 0,65/s gehaltener Taste (1,5 s → ≈ 0,97), Tipper je 0,004; die Oberfläche schickt ↑/↓ je Bild, solange die Taste beim Aufziehen gehalten ist.
