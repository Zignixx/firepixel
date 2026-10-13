# 01 `tool-brush` – Pinsel: Aufnahme des Originals (PAR-O-01-02, 13.10.2026 ≈ 04:25–05:10)

Testdateien T01/T02 (`re\_testdateien\par\`, siehe `TESTFILES.md`). Fenster 1600 × 1000, Zoom 100 %, Leinwand 800 × 600
bei Bildschirm 297,183. Referenzbilder: `re\_testdateien\par\ref\tool-brush\` (+ `INDEX.md`), Pixeldateien und Hashes:
`re\_testdateien\par\px\tool-brush\` (`pixels.csv`, `pi_controls_dump.txt`). Messart: **W** Fenstertext/Steuerelemente,
**J** JSF, **E** Export + Pixel, **B** Bild.

## Wichtig: Einschränkung der Messumgebung

Jeder **Leinwandklick** im Original (Pinsel, Buntstift, Zeiger; auch nach Neustart, frisch geöffnete Datei) meldet „Es ist
ein interner Fehler aufgetreten.“ und malt bzw. wählt nichts (bekannt als BEFUNDE FV-15). Damit sind **Strich live,
Zielobjekt, Umschalt, Klick, Esc, Auswahlrahmen, Mauszeiger, Verlauf** heute im Original **nicht messbar** (Bild
`02_stroke_live-internal-error_pressed.png` belegt nur den Fehler). Ersatz: Strichbild per JSF als Vektorlinie mit
demselben Pinselobjekt (gleiche Strich-Engine; laut Werkzeuganalyse `re\10-werkzeuge-befehle.md` ist der Pinsel ein Pfad-Werkzeug im Bitmap-Modus – Annahme: gleiche Pixel) auf T02 →
Export PNG. Werte unten mit „Ersatz“ gekennzeichnet. Inspektor, Popups und Dialoge sind normal bedienbar.

Werkzeugvorgabe beim Start (gespeicherter Stand, nicht Werksvorgabe): Strichfarbe #C04040, Größe 1, „Weich abger.“,
Rand 100, Textur „Ader“ 0, Deckkraft 100, Normal, „Transparenz beibehalten“ aus.

## Prüfpunkte

| Nr | Kürzel | Original | Art |
|---|---|---|---|
| 01 | `inspector_idle` | Inspektor „Pinsel“ (Symbol + Titel links). Feldfolge Zeile 1: Pinsel-Symbol, **Strichfarbe** (Farbfeld), **Größe** (Feld + ▼ Popup-Schieber), **Strichart** (Liste, zeigt nur den Artnamen, z. B. „Weich abger.“), dann Spalte rechts: Deckkraft-Symbol, **Deckkraft** (Feld 100 + ▼), **Mischmodus** (Liste „Normal“). Zeile 2: „Rand:“ + Kantenvorschau (kleines Feld) + **Rand** (Feld + ▼); rechts Kontrollkästchen **„Transparenz beibehalten“**. Zeile 3: „Textur:“ + Texturliste („Ader“) + **Menge** (Feld 0 + ▼). Zeile 4: Knopf **„Pinselstrich bearbeiten“** (ohne „…“) + 2 Symbolknöpfe (Pinsel speichern, Pinsel löschen). Steuerelemente: Größe Edit 429–458 × 894–915; Strichart-ComboBox 477–575; Rand Edit 531–560 × 920–941; Textur-ComboBox 431–527 × 944–965; Menge Edit 531–560; Knopf 401–523 × 970–990; Symbolknöpfe 525–549 / 551–575; Deckkraft Edit 598–627; Mischmodus 646–768; Kontrollkästchen 583–736 × 920–936 | W, B `01` |
| 02 | `stroke_live` | **nicht messbar** (Leinwandklick → interner Fehler). Nach Handbuch/Erwartung: Strich wird während des Ziehens gezeichnet. → offen, nachmessen sobald Leinwand geht | – |
| 03 | `stroke_result` | Ersatz: 1-px-Strich „Präzise abger.“ (bc_Basic/bn_Hard Rounded, nicht geglättet) waagrecht 60,150→500,150: geänderte Pixel 60,150–501,151 (441 px, genau 1 Zeile), Farbe exakt #C04040 | E (`07_size-1.png`) |
| 04 | `target_empty` | nicht messbar (Leinwand). Erwartung aus Objektmodell: neues Bitmap-Objekt | – |
| 05 | `target_bitmap` | nicht messbar | – |
| 06 | `target_vector` | nicht messbar. Hinweis: per JSF `addNewLine`/`addNewRectangle` bei aktivem **Pinsel** → „interner Fehler“, Objekt entsteht trotzdem (teils doppelt gelistet); mit Zeiger-Werkzeug fehlerfrei | J |
| 07 | `size` | Ersatz, Präzise abger.: Größe 1 → 60,150–501,151 (441 px); 10 → 55,145–505,155 (Breite 10, 4 480 px); 50 → 35,125–525,175 (50, 23 976 px). Strich ragt um Größe/2 über die Endpunkte hinaus (runde Enden). Popup „Größe“: **senkrechter Schieber** 24 × 222 px, öffnet nach oben über dem Feld | E, B `07_size_open` |
| 08 | `edge` | Ersatz, 20 px, geglättet, Rand 0/50/100 (`softness`): Querschnitt x = 200 über Verlauf #B49B5C: Rand 0 → 10 Reihen hart #C04040; Rand 50 → 1 Mischreihe je Seite (#B87C52 / #BD5848); Rand 100 → 2–3 Mischreihen (#B78755, #BC5E49, … #BE4B44, #B9714F). Ausdehnung bleibt 50,140–509,160. Popup „Rand“: senkrechter Schieber 24 px breit (Höhe ≥ 140, unten abgeschnitten aufgenommen) | E, B |
| 09 | `opacity` | Ersatz: Objektdeckkraft 50 → gespeichert 50,196 (= 128/255); Strich über #B49B5C → #BA6D4E. Überlappung im selben Zug nicht messbar. Popup „Deckkraft“: senkrechter Schieber 24 px | E, B `09_opacity_open` |
| 10 | `blend` | Ersatz über B1 (20 px): Durchschnitt → bei 200/300/420: #BA6D4E / #602020 / #607020 (identisch mit Deckkraft 50); Multiplizieren → #882717 / #000000 / #002800; Negativ multiplizieren → #ECB485 / #C04040 / #C0B840. Liste vollständig in Einheit 02 Nr. 08 | E |
| 11 | `preserve_alpha` | nicht messbar (Leinwand); Kontrollkästchen vorhanden, Vorgabe aus | W |
| 12 | `shift_line` | nicht messbar | – |
| 13 | `click_dot` | nicht messbar | – |
| 14 | `esc` | nicht messbar | – |
| 15 | `cursor` | Voreinstellungen: `UseBrushSizeCursors = true`, `UsePrecisionCursors = false` (Pinselzeiger zeigt Größe); Bild nicht möglich (Systemzeiger nicht im Bildschirmfoto, Leinwand gestört) | J |
| 16 | `selection_clip` | nicht messbar | – |
| 17 | `undo` | nicht messbar (kein Strich); `fw.historyPalette` liefert per JSF keine Schritte | J |
| 18 | `no_frame` | nicht messbar | – |

## Weitere Beobachtungen

- Strichart-Wahl **behält die aktuelle Größe** (Messung: nach Wahl jeder der 54 Arten bleibt `diameter` = vorher
  eingestellter Wert 1 bzw. 20; die Größe der Vorlage wird nicht übernommen). Gilt im Objekt-Inspektor (Pfad gewählt).
- Werkzeugeinstellung folgt den Dokument-Vorgaben (`dom.setBrushNColor` ohne Auswahl), wenn das Werkzeug danach neu
  aktiviert wird (Wechsel Zeiger → Pinsel zeigt dann z. B. „1-Pixel hart“, Größe 1, Rand 0).
- Pinsel speichern (Symbolknopf links) → in dieser Umgebung „interner Fehler“ (Bild `..\tool-brush-stroke-options\15_…`).

## Offen (nachmessen, sobald Leinwandklicks im Original wieder gehen)

02, 04, 05, 06, 09 (Überlappung), 11, 12, 13, 14, 15 (Bild), 16, 17, 18 – Ablauf und Bildnamen wie in `INVENTORY.md`.
