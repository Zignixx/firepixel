# 01 `tool-brush` – Vergleich Original ↔ Firepixel (PAR-F-01-02, 13.10.2026)

Firepixel: Debug-Build `main` a694b48 (enthält PINSEL-1), Slot fp-s4, eigene Instanz mit eigenem Profil, `--mcp`,
1600 × 1000, Design „normal“, Zoom 100 %; echtes Fenster, Brücke (`ui_input` für alle Züge und Felder). Arbeitskopien von
T01–T04, Farben per `color.set` (Strich #C04040, Füllung #808080). Züge wie im Original: 30 Schritte à 15 ms, Standardzug
60,150 → 500,150; Export PNG über MCP `export`, Hash = sha256 der RGBA-Pixel (16 Zeichen, gleiches Verfahren wie
`real_strokes.txt`, an `r03`/`r02` nachgerechnet). Grundbild T02 hashgleich mit `00_base_none.png` (bf663a48136ea54c).
Pinselvorgaben teils per `pi.tool.brush` (Art/Größe), Rand/Deckkraft/Mischmodus/Textur/Menge per Klick und Eingabe im Inspektor.
Bilder `docs\parity\img\tool-brush\`, Vergleich `re\_testdateien\par\cmp\tool-brush\` (16 Paare; Leinwandbilder 0,1–1 %
Abweichung, Inspektor/Popups gestaltungsbedingt hoch). Rohdaten: Scratchpad `parf0102\out\`.

| Nr | Prüfpunkt | Original | Firepixel | Bew. | Befund |
|---|---|---|---|---|---|
| 01 | inspector_idle | Zeile 1 Farbe · Größe ▼ · Strichart · Deckkraft ▼ · Mischmodus; Zeile 2 Rand + Vorschau ▼, „Transparenz beibehalten“; Zeile 3 Textur + Menge ▼; Knopf „Pinselstrich bearbeiten“ + 2 aktive Symbolknöpfe | gleiche Felder, Reihenfolge und Lage (± 3 px); Knopf **„Pinselstrich bearbeit…“ abgeschnitten**; „Pinselstrich speichern“ **grau**; Feld **Rand zeigt nach Strichart-Wahl den alten Wert** (100 bei „Präzise abger.“, gemalt wird hart); Werte aus `pi.tool.brush` erscheinen gar nicht im Inspektor | 🔧 behoben (2cfa2d9), Gegenprüfung offen | PAR 01/01 |
| 02 | stroke_live | Halbzeit: fertiger deckender Strich, kein Kreis, kein Rahmen | gleich (Halbzeitbild: Strich bis ≈ x 285, Kreis-Zeiger nur am Mauszeiger) | ✅ | – |
| 03 | stroke_result | 1 px: 60,150–501,151 (441 px, sha 69d6e556…); 10 px: 56,146–506,156 (4 480 px, sha db9c8103…) | 1 px: 60,150–**500**,151 (440 px, a119ea32…); 10 px: **55,145–504**,155 (4 470 px, 64584d23…); 20 px: 50,140–509,160 (Original echt 51,141–511,161); 50 px: 35,125–524,175 (23 926 statt 23 976) → **Endpunkt fehlt 1 px** (`trimEnd`), **echte Züge nicht um +1/+1 versetzt** | 🔧 behoben (67db4dd), Gegenprüfung offen | PAR 01/03 |
| 04 | target_empty | T01: neues Bitmap, Name leer, Grenzen eng, danach gewählt | neues Bitmap (Name leer), Grenzen 55,145–504,155, danach gewählt (Lage siehe 03) | ✅ | – |
| 05 | target_bitmap | B1 gewählt: kein neues Objekt; Zug über den Rand → B1 wächst (bis 406) | kein neues Objekt; Zug 200,300 → 300,400 → B1 40,40–520,404 (Lage siehe 03) | ✅ | – |
| 06 | target_vector | R1 gewählt: neues Bitmap oben, R1 unverändert, Auswahl → Bitmap; Strich **1 px** (Inspektor zeigt Strich des Vektors) | neues Bitmap oben, R1 unverändert, Auswahl → Bitmap ✅; Strich **10 px** (Werkzeugvorgabe) 55,80–144,90, Inspektor zeigt 1 / „1-Pixel hart“ | 🔧 behoben (67db4dd), Gegenprüfung offen | PAR 01/06 |
| 07 | size | 1/10/50: Breite = Größe, runde Enden; Popup senkrechter Schieber 24 × 222 nach oben | Breite = Größe, runde Enden (Abweichung 1 px siehe 03); Popup senkrechter Schieber ≈ 24 × 221 nach oben | ✅ | (→ 03) |
| 08 | edge | Größe 20, Rand 0/50/100: Querschnitt x 200 hart / 1 / 2–3 Mischreihen, Ausdehnung 50,140–509,160 | über Inspektorfeld: Rand 0 **hashgleich** (38682c08…), Rand 50/100 Querschnitt gleich bis ± 1 je Kanal, gleiche Ausdehnung | ✅ | – |
| 09 | opacity | 50 %, Selbstkreuzung: 200,150 = #BA6D4E (keine Verstärkung); Popup senkrecht 24 px | 200,150 = #BA6E4E (± 1), keine Verstärkung; Popup senkrecht 24 px (öffnet nach oben, Original nach unten, abgeschnitten) | ✅ | (→ 03) |
| 10 | blend | Durchschnitt #BA6D4E/#602020/#607020 · Multiplizieren #882717/#000000/#002800 · Negativ multiplizieren #ECB485/#C04040/#C0B840 | über Inspektorliste gewählt: **alle 9 Messwerte gleich** | ✅ | – |
| 11 | preserve_alpha | T03, Start im deckenden Teil: an → nur deckende Pixel, Grenzen bleiben; aus → BM wächst bis 531. Start 100,250 (außerhalb BM): an → **nichts gemalt, Auswahl aufgehoben**; aus → **neues Bitmap** 91,241–511,261 | Start innen: an/aus gleich (290–400 bzw. BM 200,100–529,400) ✅. Start außerhalb: an → **Strich in BM gemalt** (4 000 px), aus → **BM wächst** auf 90,100–509,400, kein neues Objekt | 🔧 behoben (67db4dd/2cfa2d9), Gegenprüfung offen | PAR 01/11 |
| 12 | shift_line | Umschalt: 45°-Raster, Länge = größere Komponente (60,150→300,260 ⇒ 56,146–306,396; →300,400 ⇒ 56,146–316,406) | **kein Einrasten**: gerade Linie zum Mauspunkt (55,145–304,265 bzw. 55,145–304,404) | 🔧 behoben (67db4dd), Gegenprüfung offen | PAR 01/12 |
| 13 | click_dot | Klick Größe 20 bei 150,300: runder Abdruck 141,291–161,311, 316 px | runder Abdruck 316 px, 140,290–160,310 (Lage siehe 03) | ✅ | (→ 03) |
| 14 | esc | Esc **während** des Zugs bricht nicht ab; Esc danach → Zeiger + Schritt „Bitmapmodus beenden“, B1 bleibt gewählt | Esc während des Zugs **verwirft den Strich** (Export = Grundbild); danach → Zeiger, B1 bleibt gewählt, aber **kein Verlaufsschritt** „Bitmapmodus beenden“ | 🔧 behoben (67db4dd), Gegenprüfung offen | PAR 01/14 |
| 15 | cursor | Größe 1 → Zeiger mit Hotspot; Größe 20 → eigener Größenkreis (`UseBrushSizeCursors`) | Größe 1 → Fadenkreuz; Größe 20 → Kreis in Spitzengröße + Fadenkreuz (nur Text/Bild Firepixel, Original ohne Bild) | ✅ | – |
| 16 | selection_clip | Auswahlrahmen 100,100–250,200: nur 100,141–250,161 (3 000 px) | 3 000 px in 100,140–250,160 (Lage siehe 03) | ✅ | (→ 03) |
| 17 | undo | ein Zug = ein Schritt „Pinsel“, „Rückgängig: Pinsel“, ein Strg+Z = Ausgangsbild | Schritt „Pinsel“, Menü „Rückgängig: Pinsel“, Strg+Z → hashgleich mit Grundbild | ✅ | – |
| 18 | no_frame | kein Rahmen um B1 während des Zugs | kein Rahmen (Bild mitten im Zug) | ✅ | – |

**Zählung:** ✅ 12 · ❌ 6 · 🎨 0.

## Annahmen / Hinweise

- 03: Sollwerte sind die **echten** Züge des Originals (`real_strokes.txt`). Größe 20 in Firepixel = Ersatzmessung des
  Originals (hashgleich bei Rand 0); der echte Zug des Originals liegt dort um +1/+1 (u02_06: 51,141–511,161). Ein Befund
  für alle Lageabweichungen (04, 05, 07, 09, 13, 16 nur deshalb nicht pixelgleich).
- 06: Im Original ist „malt mit 1 px“ als Annahme notiert (Inspektor zeigt den Vektorstrich). Niedrige Priorität; vor dem
  Beheben im Original gegenprüfen, ob wirklich der Strich des gewählten Vektors benutzt wird.
- 14: Firepixel verwirft den Zug bei Esc bewusst (PINSEL-1-Bericht) – widerspricht dem Original.
- 01: „Pinselstrich speichern“ grau – Wirkung im Original nicht messbar (interner Fehler), siehe 02/15.
- Verlaufsschritte von MCP-`select` heißen „Deselect“ (englisch) – nicht Teil dieser Einheit, nur beobachtet.
