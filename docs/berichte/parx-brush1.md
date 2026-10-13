# Bericht PAR-X-BRUSH-1 (13.10.2026) – Pinsel und Strichoptionen (Einheiten 01/02) beheben

Grundlage: `docs\parity\tool-brush.md` (6 ❌), `tool-brush-stroke-options.md` (10 ❌, davon 02/05 → PAR 18/12 bei
PAR-X-INSP-1), Aufnahmen `*.orig.md`, `re\24-pinsel.md`, `re\_testdateien\par\px\tool-brush*\` (`real_strokes.txt`,
`presets.csv`, `03_*.png`). Slot fp-s4, per cherry-pick auf main.

## Commits

1. `67db4dd` PAR-X: Pinsel nach echten Zügen des Originals (Engine) – `fp-engine\src\paint.rs`, `tools\bitmap.rs`,
   `commands\bitmap.rs`, `commands\modify.rs`, Golden-Tests `tests\parx_brush1.rs` + `tests\data\parx-brush1\`
   (Betreff mit unsichtbarem BOM-Zeichen am Anfang, Inhalt korrekt)
2. `2cfa2d9` PAR-X: Pinsel-Inspektor und Strichoptionen wie das Original (Oberfläche) – `obj_quads.rs`, `popups.rs`,
   `fields.rs`, `canvas\overlay.rs`, `dialogs\brush_edit.rs`, `panel.rs` (`pi.brush.save tool`), Texte de, kittests
3. dieser Bericht + Parität/BEFUNDE

## Behoben (Messwert → Ergebnis)

| Befund | Ursache / Umsetzung | Prüfung |
|---|---|---|
| 01/03 Lage | Pinsel ließ den Endpunkt weg (`trimEnd`), gerade Größen ohne +1/+1 → Endpunkt eingeschlossen, gerade Größe +1/+1 (nur Pinsel) | Hash **gleich** r03 (1 px), r02 (10 px), r13 (Klick 20 px) |
| 01/12 Umschalt | 45°-Raster, Länge = größere Komponente (`snap_brush_line`) | Hash **gleich** r12 a2/b |
| 01/14 Esc | Esc mitten im Zug bricht nicht ab (r14_esc_mid hashgleich); Esc danach → Zeiger + Schritt „Bitmapmodus beenden“ (`modify.exitBitmapMode {step}`) | `escape_mid_stroke_and_after` |
| 01/11 außerhalb | Start ≥ 32 px neben der gewählten Bitmap: „Transparenz beibehalten“ → nichts, Auswahl aufgehoben; sonst neues Bitmap 91,241–511,261 (`bitmap.paint newBitmap`) | T03 beide Fälle |
| 01/06 Vektor | gewählter Vektor: es wird mit dessen Strich gemalt, ohne Strich „1-Pixel hart“ (Annahme des Originals übernommen) | T04 R1 → 1-px-Strich |
| 01/01 Inspektor | Strichart-Wahl setzt Rand und Texturmenge der Art; Werte aus `pi.tool.brush` (Vorgabepinsel) erscheinen; Knopf 122 px wie das Original, Text ganz; Speichern/Löschen im Werkzeug-Inspektor aktiv | kittests, `inspector_i18n` (Deutsch unverändert) |
| 02/03 Stricharten | **Ursache: Zufalls-Startwert 0** – MINSTD bleibt bei 0 stehen (alle Werte 2³¹−1): Streuung in einer Reihe, alle Spitzen am selben Versatz, Zufallsfarben falsch. Jetzt Startwert je Zug und je Punkt (`RNDi`), Geschwindigkeit 0,125 (wie erzeugte Pfade), Vorgabetextur „Ader“ (Vein) wie die Liste des Originals | 54 Probestriche: feste Arten pixel-/grenzgleich, Zufallsarten über 8 Startwerte (Pixelzahl ± 15 %, Ausdehnung ± 30 %, Mittelfarbe ± 24); `tests\data\parx-brush1\cmp03.csv` |
| 02/13 Spitzenabstand | gleiche Ursache (Startwert 0) – Spitzen 3 / Abstand 50 jetzt gespreizt | Höhe ≥ 35 px |
| 02/01 Menü | „Ohne“ + Trenner, Menü aus den Menüknoten der Anwendung (≈ 220 px), Untermenü rechts daneben | kittest (Breite < 320, Untermenü rechts) |
| 02/08 Mischmodi | Auflösen, Überlagern, Luminanz, Farbton, Löschen (Liste wie Original, 46 Modi + 9 Trenner) | kittest gegen die Liste aus .orig.md |
| 02/10, 02/11 Dialog | Prozentfelder mit „%“ (`fields::number_unit`), Ein/Aus bei „Ohne“ 0, Winkel 0 → 360 | – (Nachprüfung) |
| 02/15 Speichern | `pi.brush.save {tool: true}` speichert die Werkzeugvorgabe; Knopf aktiv | kittest |
| 02/18 Neustart | Voreinstellungen `BrushToolStroke`, `BrushTipSize`, `BrushToolSoftEdge`, `BrushToolTexture`, `BrushToolTextureAmount` | kittest (neue Sitzung mit den Voreinstellungen) |
| 02/19 Kantenvorschau | Spitze in Pinselgröße (1 → Punkt), auf das Feld begrenzt | – (Nachprüfung) |

## Annahmen / Hinweise

- **Gestrichelt/Gepunktet:** Die Strichmuster sind jetzt positionsgleich mit dem Original. Die Bilder des Originals
  (`03_05-*.png`) enthalten zusätzlich eine 1-px-Linie 60–500 zwischen den Strichen (deshalb „endet bei 501“); sie
  stammt vermutlich aus der Ersatzmessung (Vektorlinie) und gehört nicht zum Pinsel – nicht nachgebaut.
- **Endpunkt:** FV-11 hatte im Original 300→400 = 300–399 gemessen, PAR-O (echte Züge, hashgleich) 60→500 = 60–500.
  Umgesetzt nach PAR-O; FV-11-Tests angepasst. Im Original gegenprüfen (siehe Nachprüfung).
- **Start außerhalb:** FV-11 (10 bzw. 19 px neben der Bitmap → sie wächst) und 01/11 (100 px → neues Bitmap) passen nur
  mit einer Abstandsgrenze zusammen; angenommen 32 px (`OUTSIDE_NEW_BITMAP`).
- **Vektor gewählt (01/06):** Original-Annahme „malt mit dem Inspektorstrich“ übernommen.
- Esc mitten im Zug und Endpunkt/Versatz nur beim Pinsel; Bleistift/Radiergummi unverändert (PAR-X-TOOLS-1).
- Malen versetzt die Sitzung nicht in den Bitmapmodus (sonst bliebe er nach „Auswahl aufheben“ hängen); der Schritt
  „Bitmapmodus beenden“ entsteht über `modify.editBitmap` + `exitBitmapMode {step: true}` beim Esc nach einem Zug.
- `all_commands_work`: 2 Fehler `edit.insert.emptyBitmap`/`panel.layers.newBitmap` (leere Bitmap beim Speichern) –
  schon vor diesen Änderungen, nicht Teil des Auftrags.

## Für NACHPRUEFUNG.md

| Was | Ablauf | Erwartet |
|---|---|---|
| Pinsel Lage | T02, Präzise abger. 10, Zug 60,150→500,150 | 56,146–506,156 (sha db9c8103…) |
| Umschalt | Umschalt + Ziehen 60,150→300,260 | 45°-Linie bis 306,396 |
| Esc | mitten im Zug Esc, loslassen; danach Esc | Strich ganz; dann Zeiger + Verlauf „Bitmapmodus beenden“ |
| außerhalb | T03 BM gewählt, Start 100,250, Größe 20, mit/ohne „Transparenz beibehalten“ | nichts + Auswahl weg / neues Bitmap |
| Stricharten | Punkte, Konfetti, Pelz, Garn, Harke, Spitzen 3/Abstand 50 ziehen | gestreut wie Original, Spuren gespreizt |
| Inspektor | Strichart wechseln; Knopf „Pinselstrich bearbeiten“; Speichern-Symbol; Neustart | Rand folgt der Art, Text ganz, Speichern aktiv, Vorgabe gemerkt |
| Strichart-Menü | Liste öffnen, Kategorie überfahren | „Ohne“, schmal, Untermenü rechts |
| Dialog | Pinselstrich bearbeiten → Optionen/Form | „100%“, Ein/Aus 0, Winkel 360 |
| Original gegenprüfen | Pinsel 1 px Zug 300,20→400,20 (FV-11-Messung) | 300–400 oder 300–399? |

## Neue Befunde

- (PAR-X-BRUSH-1) Original gegenprüfen: Endpunkt (FV-11 300–399 gegen PAR-O 60–500) und Abstandsgrenze „Start
  außerhalb → neues Bitmap“ (angenommen 32 px).
- (PAR-X-BRUSH-1) `command_texts.ftl`: neue Parametertexte (`bitmap.paint seed/newBitmap`, `exitBitmapMode step`,
  `pi.brush.save tool`) ohne englische Übersetzung (Rückfall Deutsch, wie andere Parametertexte).
