# FIX-G3 – TW-02 (textRuns mit allen Eigenschaften) und TW-08 (toSource ohne Klammern)

Stand 13.10.2026. Slot `fp-s4`, Commit siehe unten.

## Umgesetzt

**TW-02** (`crates\fp-script\src\imp\elem.rs`, `doc.rs`)
- `textRuns.initialAttrs` liefert alle Eigenschaften des Originals. Laut den Logs `menu-text` (02, 03, 15, 19) und `dialogs-d21-d30/06` sind es **18** Schlüssel (der TWEAKS-Eintrag nennt 19, gezählt per Skript über alle `orig`-Werte):
  `alignment, antiAliasSharpness, antiAliasStrength, baselineShift, bold, face, fillColor, horizontalScale, italic, leading, leadingMode, overSample, paragraphIndent, paragraphSpacingAfter, paragraphSpacingBefore, rangeKerning, size, underline`.
- Der erste Lauf (`changedAttrs`) liefert wie im Original den vollständigen aufgelösten Satz mit 15 Schlüsseln (ohne die 3 Glättungswerte). Weitere Läufe liefern nur ihre gesetzten Werte.
- `face` ist der PostScript-Name (`MyriadPro-Regular`). Reihenfolge: `CFTs` des Kopfs, sonst der Name des ersten Laufs derselben Familie, sonst Familie ohne Leerzeichen + `-Regular`. Der Familienname bleibt im Modell (`TextAttrs.font`) und über `font`/Inspektor unverändert verfügbar.
- Vorgaben wie im Original: `leadingMode:"percentage"`, `horizontalScale:1`, `fillColor` aus dem Kopf bzw. der Textfüllung, Glättung aus `TA1i/TA2i/TOSi` (192/64/8).
- Setzen über `setTextRuns` und `element.textRuns = …`: neu werden `horizontalScale`, `leadingMode` (`"percentage"`/`"exact"` oder 0/1), `paragraphIndent`, `paragraphSpacingBefore/After` angenommen, dazu `antiAliasSharpness/antiAliasStrength/overSample` (→ `text.setProperties`).
- Fehler behoben: `leading` wurde beim Setzen mit 100 multipliziert, das Modell speichert aber den Faktor (`LEDf` 1.2). Jetzt 1:1.
- Das Ergebnis ist zeichengleich mit der Original-Ausgabe aus `menu-text/02_size_menu_result` (Test `text_runs_to_source_has_original_shape`).

**TW-08** (`crates\fp-script\src\imp\mod.rs`)
- `toSource()` von Objekten ohne umschließende Klammern: `{ x:36, y:36 }` statt `({ x:36, y:36 })`, wie die Wirtobjekte des Originals (`guides`, `gridSize`, `textRuns`). Arrays und Zeichenketten unverändert (Tests).

## Tests
- Neu: `crates\fp-script\tests\tw02_textruns.rs` (9 Tests: 18/15 Schlüssel, Vorgabewerte, neue Felder setzen/lesen, Rundlauf `setTextRuns(textRuns)`, `face` aus Familienname, toSource-Form, Objekte/Arrays/Strings).
- `cargo test -p fp-script -p fp-mcp` grün, `cargo check --workspace --all-targets` ohne Warnungen. Keine bestehende Golden-Erwartung war betroffen.

## Erledigte Befunde
- TWEAKS TW-02 „`textRuns.initialAttrs` mit 19 Eigenschaften …“ – erledigt (18 gemessen).
- TWEAKS TW-08 „`toSource()` von Objekten ohne Klammern …“ – erledigt.

## PARITAET
- Skript-API Text-Läufe (`textRuns`) und `toSource`: „umgesetzt, Nachprüfung ausstehend“.

## Nachprüfung
- Paarlauf `menu-text` 02/03 und `dialogs-d21-d30` 06 erneut: `textRuns.toSource()` muss bis auf Werte, die von anderen Befunden abhängen (Schriftgröße nach Menüschritt), zeichengleich mit `orig` sein.
- `xs-canvas` 09/10: `guides.toSource()` beginnt mit `{ ` statt `({ `.

## Neue Befunde (nicht umgesetzt, außerhalb des Auftrags)
1. **Reine JS-Arrays** gibt das Original ohne Innenabstand und mit Lücken für `undefined` aus: `[800, 600, , 2, 1]`, `[, ]`, `[[0, "L1", …]]`. Firepixel: `[ 800, 600, null, 2, 1 ]`. Wirt-Arrays (z. B. `hGuides`, `textRuns`) dagegen mit Abstand `[ 300 ]`, leer `[  ]` (zwei Leerzeichen; Firepixel `[]`). Für eine Unterscheidung müsste `toSource` Wirt- und JS-Arrays erkennen (fp-script `conv::js_to_json` verliert das).
2. **Reine JS-Objektliterale** hätten im Original (SpiderMonkey) weiterhin `({a:1})`; Firepixel kann sie von Wirtobjekten nicht unterscheiden und gibt jetzt immer `{ a:1 }` aus.
3. **Wagenrücklauf in Zeichenketten**: Original `"Zeile eins\u000DZeile zwei"` (`menu-text/15`), Firepixel `\r`.
