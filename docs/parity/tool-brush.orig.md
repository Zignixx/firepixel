# 01 `tool-brush` – Pinsel: Aufnahme des Originals (PAR-O-01-02, 13.10.2026 ≈ 04:25–05:10; Nachmessung PAR-O-03-04 ≈ 06:10–07:00)

Testdateien T01–T04 (`re\_testdateien\par\`, siehe `TESTFILES.md`), Arbeitskopien `par\work\p34_t0*.fw.png`. Fenster
1600 × 1000, Zoom 100 %, Leinwand 800 × 600 bei Bildschirm 297,183 (T03 600 × 500 bei 397,233). Referenzbilder:
`re\_testdateien\par\ref\tool-brush\` (+ `INDEX.md`), Pixeldateien und Hashes: `re\_testdateien\par\px\tool-brush\`
(`pixels.csv` = Ersatzmessung, `real_strokes.txt` = echte Züge, `r*.png`). Messart: **W** Fenstertext/Steuerelemente,
**J** JSF, **E** Export + Pixel, **B** Bild, **M** echter Mauszug (SetCursorPos/mouse_event, 30 Schritte à 15 ms).

## Messumgebung

Der frühere Leinwandfehler („interner Fehler“, FV-15) lag am Öffnen mit `fw.openDocument(url, false, true)`
(`re\25-leinwand-fehler.md`). Nachmessung mit `fw.openDocument(url)`: alle Leinwandaktionen gehen. Pinselvorgabe per JSF
(`setBrushNColor` ohne Auswahl, dann Werkzeug neu aktiviert), Standardzug **60,150 → 500,150** auf T02. Hinweis: das
Aktivieren des Pinsels per JSF wählt auf T02 meist B1 mit aus (Setup meldet `sel=1`).

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `inspector_idle` | Inspektor „Pinsel“ (Symbol + Titel links). Feldfolge Zeile 1: Pinsel-Symbol, **Strichfarbe** (Farbfeld), **Größe** (Feld + ▼ Popup-Schieber), **Strichart** (Liste, zeigt nur den Artnamen, z. B. „Weich abger.“), dann Spalte rechts: Deckkraft-Symbol, **Deckkraft** (Feld 100 + ▼), **Mischmodus** (Liste „Normal“). Zeile 2: „Rand:“ + Kantenvorschau (kleines Feld) + **Rand** (Feld + ▼); rechts Kontrollkästchen **„Transparenz beibehalten“**. Zeile 3: „Textur:“ + Texturliste („Ader“) + **Menge** (Feld 0 + ▼). Zeile 4: Knopf **„Pinselstrich bearbeiten“** (ohne „…“) + 2 Symbolknöpfe (Pinsel speichern, Pinsel löschen). Steuerelemente: Größe Edit 429–458 × 894–915; Strichart-ComboBox 477–575; Rand Edit 531–560 × 920–941; Textur-ComboBox 431–527 × 944–965; Menge Edit 531–560; Knopf 401–523 × 970–990; Symbolknöpfe 525–549 / 551–575; Deckkraft Edit 598–627; Mischmodus 646–768; Kontrollkästchen 583–736 × 920–936 | W, B `01` |
| 02 | `stroke_live` | **Strich wird während des Ziehens als fertiger Strich gezeichnet** (keine Kreis-Umrisse, kein Rahmen): Bild bei Halbzeit des Zugs zeigt Größe 10 deckend #C04040 von x 56 bis 285 (Größe 1: 60–280), Rest noch unberührt | M, B `02`, `02_…-d1` |
| 03 | `stroke_result` | Echter Zug, 1 px „Präzise abger.“: 441 px, 60,150–501,151, #C04040 – **pixelgleich** mit der Ersatzmessung (sha `69d6e5565fd13c78`). Größe 10: 56,146–506,156 (4 480 px, 10 Zeilen 146–155) – gegenüber der Ersatz-Vektorlinie (55,145–505,155) um **+1/+1 versetzt** (910 px verschieden) | M, E |
| 04 | `target_empty` | T01, nichts gewählt: **neues Bitmap-Objekt** (Name leer), Grenzen eng um den Strich 56,146–506,156; danach gewählt | M, J |
| 05 | `target_bitmap` | T02, B1 gewählt: **kein neues Objekt**, Strich in B1 (Grenzen 40,40–520,360 bleiben). Malen über den Rand von B1 hinaus **vergrößert die Bitmap** (Zug bis 300,400 → B1 40,40–520,406) | M, J |
| 06 | `target_vector` | T04, R1 (Vektor) gewählt: **neues Bitmap-Objekt** oben in Ebene 1, R1 unverändert, Auswahl wechselt auf das neue Bitmap. Strich nur 1 px breit (60,85–140,86), obwohl Vorgabe Größe 10: bei gewähltem Vektor zeigt der Inspektor dessen Strich, gemalt wird mit der gespeicherten Werkzeugvorgabe (Annahme). Pinsel ist **kein** Vektorwerkzeug (L36) | M, J, B `06` |
| 07 | `size` | Ersatz, Präzise abger.: Größe 1 → 60,150–501,151 (441 px); 10 → 55,145–505,155 (echt: 56,146–506,156); 50 → 35,125–525,175 (23 976 px). Strich ragt um Größe/2 über die Endpunkte hinaus (runde Enden). Popup „Größe“: **senkrechter Schieber** 24 × 222 px, öffnet nach oben über dem Feld | E, B `07_size_open` |
| 08 | `edge` | Ersatz, 20 px, geglättet, Rand 0/50/100 (`softness`): Querschnitt x = 200 über Verlauf #B49B5C: Rand 0 → 10 Reihen hart #C04040; Rand 50 → 1 Mischreihe je Seite (#B87C52 / #BD5848); Rand 100 → 2–3 Mischreihen (#B78755, #BC5E49, … #BE4B44, #B9714F). Ausdehnung bleibt 50,140–509,160. Popup „Rand“: senkrechter Schieber 24 px breit | E, B |
| 09 | `opacity` | Deckkraft 50 % (Inspektorfeld), Größe 20, **ein** Zug mit Selbstkreuzung (60,150→300,150→300,250→200,250→200,100): Kreuzung 200,150 = **#BA6D4E** = gleicher Wert wie einfacher Strich über #B49B5C → **keine Verstärkung im selben Zug**. Popup „Deckkraft“: senkrechter Schieber 24 px | M, E, B `09` |
| 10 | `blend` | Ersatz über B1 (20 px): Durchschnitt → bei 200/300/420: #BA6D4E / #602020 / #607020 (identisch mit Deckkraft 50); Multiplizieren → #882717 / #000000 / #002800; Negativ multiplizieren → #ECB485 / #C04040 / #C0B840. Liste vollständig in Einheit 02 Nr. 08 | E |
| 11 | `preserve_alpha` | T03, BM gewählt, Größe 20, Zug 300,250→520,250 (Start im deckenden Teil). **An:** Farbe nur auf deckenden Pixeln (rot x 291–400), transparenter Rand bleibt Alpha 0, BM-Grenzen bleiben. **Aus:** Strich bis x 531, BM wächst auf 200,100–531,400. Start im transparenten Bereich (100,250): an → nichts gemalt, Auswahl aufgehoben; aus → neues Bitmap 91,241–511,261 | M, E, B `11` (2) |
| 12 | `shift_line` | Umschalt + Ziehen: Winkel rastet in **45°-Schritten**, Länge = größere Komponente. 60,150→300,260 (24,6°) → 45°-Linie 56,146–306,396; 60,150→300,400 → 56,146–316,406. Ohne vorher gewählte Bitmap kam einmal „Ihre Anforderung konnte nicht ausgeführt werden.“ und nur ein Abdruck am Start (nicht wiederholt geprüft) | M, E, B `12` (2) |
| 13 | `click_dot` | Einzelklick, Größe 20, bei 150,300: **ein runder Abdruck** 141,291–161,311 (316 px) | M, E, B `13` |
| 14 | `esc` | Esc **während** des Ziehens bricht nicht ab (Strich vollständig, pixelgleich). Esc **nach** dem Zug: Werkzeug → **Zeiger**, neuer Verlaufsschritt „Bitmapmodus beenden“, B1 bleibt gewählt | M, J, B `14` |
| 15 | `cursor` | Bild nicht möglich: der Systemzeiger erscheint in dieser Remote-Umgebung nicht im Bildschirmfoto (auch nicht per GetCursorInfo/DrawIconEx). Gemessen: Größe 1 → Zeiger mit Hotspot 10,9; Größe 20 → eigener, je Größe erzeugter Zeiger (andere Kennung, Hotspot 0,0); Voreinstellung `UseBrushSizeCursors = true`. Feststelltaste: keine Änderung messbar | J, W |
| 16 | `selection_clip` | Auswahlrahmen 100,100–250,200 auf B1, dann Zug Größe 20: Farbe nur in **100,141–250,161** (3 000 px) → an der Auswahl beschnitten | M, E, B `16` |
| 17 | `undo` | **Ein Zug = ein Verlaufsschritt „Pinsel“** (Menü Bearbeiten: „Rückgängig: Pinsel“, „Wiederholen nicht möglich“ grau). Ein Strg+Z entfernt den ganzen Strich (Export = Ausgangsbild, 0 px verändert) | M, W, E |
| 18 | `no_frame` | Kein Auswahlrahmen/keine Markierung um B1 während des Ziehens (B1 gewählt; Zeilen direkt über/unter B1 bleiben weiß) | M, B `18` |

## Weitere Beobachtungen

- Strichart-Wahl **behält die aktuelle Größe** (alle 54 Arten geprüft; gilt im Objekt-Inspektor).
- Werkzeugeinstellung folgt den Dokument-Vorgaben (`dom.setBrushNColor` ohne Auswahl), wenn das Werkzeug danach neu
  aktiviert wird.
- Pinsel speichern (Symbolknopf links) → in der alten Umgebung „interner Fehler“ (Bild `..\tool-brush-stroke-options\15_…`),
  nicht nachgemessen.
- Echte Züge mit gerader Größe liegen um 1 px nach rechts/unten gegenüber der JSF-Vektorlinie gleicher Koordinaten; die
  Sollwerte für Firepixel sind die **echten** Züge (`real_strokes.txt`).

## Offen

15 (Bild des Mauszeigers) – in dieser Umgebung nicht aufnehmbar. Alles andere gemessen.
