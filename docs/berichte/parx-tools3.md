# PAR-X-TOOLS-3 – Einheiten 08–10 beheben (13.10.2026)

Slot `fp-s3`, Build `target-s3`. Grundlage: `docs\parity\tool-pointer-transform-crop.md`, `tool-line-pen.md`,
`tool-shapes.md` (34 ❌), Aufnahmen `*.orig.md`, `re\_testdateien\par\px\<einheit>\paths.json`.

## Commits (main)

| Commit | Inhalt |
|---|---|
| `af8c0b5` | Zeiger, Transformieren, Zuschneiden (fp-engine `tools\pointer.rs`, `commands\edit.rs`, `session.rs`, `tools\mod.rs`; `messages.ftl` de/en) |
| `a5ed533` | Linie, Stift, Vektorpfad, Pfad nachzeichnen, Formen (`tools\draw.rs`) |
| `cbcd693` | Tests `crates\fp-engine\tests\parx_tools3.rs` (31 Fälle), Sollwerte `tests\data\parx-tools3\expected.json` (Auszug aus `paths.json`); ältere Erwartungen in `canvas_tools.rs`/`tools.rs` an die Neumessung angepasst |
| `a0c725f` | Oberfläche: Bildvorschau aus dem Exportbereich, Inspektor live/halten, „Autom.“-Winkel, Name „Rechteck“, gehaltenes ↑/↓ (`canvas\mod.rs`, `input.rs`, `inspector\{mod,quads,tool_quads}.rs`, Test `crates\fp-ui\tests\parx_tools3_ui.rs`) |

## Umgesetzt (nach Messwert)

- **08/10, 08/16** Skalieren-Eckgriff: Projektion des Zeigers auf die Diagonale, gezogene Ecke auf ganzen Pixeln → R1 50,50–193,150, Alt 7,20–193,150, B1 40,40–603,415. Innen ziehen verschiebt (Drehpunkt nur ≤ 4 Bildschirmpixel greifbar).
- **08/15** Esc verwirft die Transformation (kein Schritt), Zeiger danach aktiv. **08/11/24** Schritt immer „Transformieren“ (auch Drehen/Neigen/Verzerren).
- **08/13** Neigen: Seitengriff bewegt nur die gegriffene Kante; Ecke senkrecht/waagrecht → Trapez (150,30/150,140; 30,50/170,50). **08/14** Verzerren-Seitengriff: beide Ecken +dx,+dy.
- **08/04** Zeiger am Eckpunkt/Knoten eines gewöhnlichen Pfads verschiebt das Objekt („Verschieben“). **08/03** Umschalt-Ziehen am gewählten Objekt verschiebt waagrecht/45° (Abwählen erst beim Loslassen ohne Zug); Alt-Ziehen „Duplizieren“. **08/06** Teilauswahl „Punkt verschieben“.
- **08/18/19** Zuschneiden-Werkzeug: Rahmen auf die Leinwand begrenzt, Objekte ganz außerhalb gelöscht, Bitmaps auf die Leinwand beschnitten, danach alle gewählt (`edit.cropDocument {tool: true}`; Menübefehl unverändert).
- **08/17/20** Inspektor B/H/X/Y live beim Aufziehen (`Session::tool_live_rect`); Zuschneiden/Exportbereich ohne Objektfang (250 × 150 statt 155). **08/20** Doppelklick öffnet die **Bildvorschau** (Register Datei, Exportbereich X/Y/B/H gesetzt).
- **08/21** Zeiger-Zug eines eben gewählten Objekts: Inspektor bleibt bis zum Loslassen beim bisherigen Layout („Dokument“).
- **09/01** Linie: Knoten auf Pixelmitten, Name „Linie“, Umschalt-45° mit der größeren Komponente (200.5,400.5).
- **09/05/18** Stift „Stift“ (auch Einfügen/Umwandeln/Entfernen), Vektorpfad „Pinsel“.
- **09/07** Esc/Eingabe: nächster Klick setzt den gewählten offenen Pfad am Endpunkt fort (auch 2 × Esc); Doppelklick/Abwählen → neuer Pfad. **09/08** Klick auf den Endpunkt eines nicht gewählten offenen Pfads setzt ihn fort (wird gewählt).
- **09/09** Alt-Ziehen auf dem soeben gesetzten Punkt ändert nur dessen Ausgangshenkel. **09/10** Klick auf Kurvenpunkt → Eckpunkt; Eckpunkt → entfernen. **09/11** kein Objektfang (Raster/Hilfslinien weiter).
- **09/06 (Nebenbefund)** Dokumentwechsel/-öffnen schließt den Werkzeugvorgang im alten Dokument ab; Schließen verwirft ihn.
- **09/12** Vektorpfad: Knoten auf Pixelmitten, Eckpunkte an Knicken ≥ 50°, sonst glatt. **09/14** Pfad nachzeichnen ersetzt nur den Abschnitt zwischen Zuganfang und -ende.
- **10/12** Rechteck/Ellipse/Polygon mit Strich 1 px (Strichfarbe). **10/08** Namen „Ellipse“/„Polygon“ (Rechteck ohne Namen; Feld zeigt „Rechteck“, 10/07). **10/10** Polygon ohne Objektfang (Ecke 650,200). **10/09** „Autom.“-Winkel 5 → 38, 6 → 57 (Verhältnis des regelmäßigen Sterns, abgeschnitten). **10/05** ↑/↓ beim Aufziehen: 0,65/s solange gehalten, Tipper 0,004; ← → verbraucht. **10/06** Rechteck-Eckpunkt mit Zeiger „Rechteck modifizieren“, mit Teilauswahl ganzes Rechteck verschieben. **10/15** Inspektor live beim Aufziehen.

Verlaufsnamen laufen über den neuen optionalen Befehlsparameter `step` (Endung eines `msg-step-…`-Schlüssels; neu: duplicate, move-point, transform, pen, brush, modify-rect, de+en).

## Befunde erledigt (BEFUNDE, Abschnitt PAR)

Alle 31 Einträge „(von PAR-F) tool-pointer-transform-crop/03 … /21“, „tool-line-pen/01 … /14“, „tool-shapes/05 … /15“ abgehakt; `tool-shapes/07` nur der Name (siehe Frage 3).
PARITAET-Zeilen: in `docs\parity\<einheit>.md` stehen 34 Zeilen auf „🔧 behoben (Commit), Gegenprüfung offen“.

## Prüfungen

`cargo test -p fp-engine --test all` (alle grün bis auf 2 ignorierte, davon 31 neu), `-p fp-engine --lib`, fp-ui `parx_tools3_ui::`, `gesamt_werkzeuge::`, `inspector_*::`, `canvas_fv14::`, `--lib parx_tools3`; `cargo check --workspace --all-targets`; `cargo test -p fp-i18n`, `-p fp-neutral`; clippy fp-engine ohne Warnung in den geänderten Dateien.

## Für die Nachprüfung (echtes Fenster, Brücke bzw. echte Tastatur)

| Punkt | Ablauf | Erwartet |
|---|---|---|
| 08/10 | T04, R1, Skalieren, Ecke 150,120 → 200,140, Enter; dann innen 80,70 → 120,80 | 50,50–193,150; Verschieben +40,+10 |
| 08/20 | Exportbereich 50,50 → 300,200, Doppelklick innen | Bildvorschau, Register Datei, Exportbereich ☑ 50/50/250/150 |
| 08/21 | Zeiger auf ungewähltem R1 drücken und ziehen | Inspektor zeigt bis zum Loslassen „Dokument“ |
| 09/07 | Stift 3 Klicks, Esc, 3 Klicks, Enter | ein Pfad mit 6 Knoten, Schritt „Stift“ |
| 10/05 | Rechteck aufziehen, ↑ 1,5 s **echt** gehalten | Rundung ≈ 0,97 |
| 10/15 | Rechteck aufziehen, mitten im Zug | Inspektor B/H/X/Y live |

## Annahmen und Fragen

- 08/10: Startpunkt des Original-Zugs „innen“ nicht aufgezeichnet; Drehpunkt deshalb nur direkt greifbar.
- 08/18: Text der Aufnahme sagt „danach alle Objekte gewählt“, im Datensatz `k18_crop_enter` sind sie ungewählt (bei Doppelklick/Griff gewählt) – umgesetzt: gewählt.
- 09/12: Original teilt eine Spitze in zwei Knoten (8 statt 7) – Abtast-Artefakt, nicht nachgebaut.
- **Frage 3:** 10/07 (Werkzeug-Inspektor Rechteck mit „Kombinieren“ + 5 grauen Pfadknöpfen) widerspricht `inspector-fill`/21 (PAR 18/21, ohne Pfadoperationen, schon behoben) – nicht geändert, Koordinator/Nutzer entscheiden.
- Frage 1 (Strichart-Vorgabe) und Frage 2 (Seitenfeld 3–360) wie vorgegeben nicht angefasst.
