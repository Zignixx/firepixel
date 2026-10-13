# FIX-D – Befunde aus dem Klicktest CT-2 (13.10.2026)

Slot `fp-s4`, Build `target-s4`. Grundlage: `docs\funktionstest\ct2_2026-10-13.md`, BEFUNDE „von CT-2“.
Wichtig vorab: CT-2 lief mit der Vorschau-exe von **07:22**. Mehrere Änderungen danach (29b532b
Farbe ersetzen, f1f12f1 Texturliste, 2cfa2d9 Strichart-Menü) lagen also noch nicht in der geprüften
exe. Jeder Punkt wurde deshalb auf `main` mit einem Test nachgestellt, bevor etwas geändert wurde.

## Umgesetzt (Commits auf main)

| Punkt | Commit | Was |
|---|---|---|
| W8 Farbe ersetzen beim Ziehen | c972c07 | Auf main schon richtig (29b532b): der ganze Strich wird blau. Neu: Regressionstests mit Werkzeugzug (fp-engine) und mit echten Mausereignissen (fp-ui). |
| Textur per Listenname („Ader“) | a31daf6 | `fp_engine::resources`: `texture_file`/`pattern_file` finden auch den übersetzten Listennamen (`pi-texture-name-*`/`pi-pattern-name-*`, aktuelle und alle geladenen Sprachen). Im Dokument steht der Dateiname („Vein“). Das Füllwerkzeug auf einem Objekt trägt Textur, Menge und „transparent“ aus dem Inspektor mit. `pi.fill`/`pi.stroke` melden einen unbekannten Texturnamen (`err-not-found`), sobald es Texturordner gibt. `tool.shape` lässt eine unbekannte Werkzeugvorgabe weg, statt am Zeichnen zu scheitern. Der Test `tools_fv16` erwartet jetzt den Fehler. |
| Untermenü der Stricharten über dem Hauptmenü | bbcab92 | Ursache: Die Menüeinträge füllen die Menübreite. Ohne Messdurchlauf nimmt egui 600 px (`default_area_size`), so wurden Haupt- und Untermenü 600 px breit. `menu.rs`: `show_nodes_max` begrenzt die Breite auf den breitesten Eintrag, `popup_menu` setzt sie als Vorgabe. Das Untermenü steht rechts daneben, ohne Platz rechts links daneben, nie darüber. |
| Texturliste schneidet Namen ab | ba52eaf | Auf main schon richtig (f1f12f1). Regressionstest de/en im 900 × 650-Fenster. |
| Esc in der KI-Fortschrittskarte | 58db5e8 | `input.rs`: Läuft eine KI-Rechnung, geht Esc nicht ans Werkzeug. Damit kein `modify.exitBitmapMode` mehr, Pixelauswahl und Bitmapmodus bleiben, wie beim Knopf „Abbrechen“. |
| Farbe der Werkzeugleiste auf die Auswahl | 3901427 | `tools_panel.rs`: Mit gewählten Vektoren/Texten setzt die Farbe auch deren Füllung bzw. Strich (ohne Strich wird einer angelegt). „Ohne“ entfernt Füllung bzw. Strich. Je ein Verlaufsschritt. Ohne Auswahl bzw. mit Bitmap nur die Werkzeugfarbe. |
| Deckkraft-Schieber läuft aus dem Fenster | 1d7dcf6 | `color_picker::slider_rect`: Der Schieber liegt wie bisher unter dem Knopf. Reicht das Fenster nicht, wird er nach oben ins Fenster geschoben, mit voller Höhe. |
| Kleinigkeiten: MCP-Verlaufsnamen | ec98f1c | `fp-mcp history`: Schritte aus der deutschen Oberfläche heißen in der englischen Antwort englisch (Befehlsname bzw. `msg-step-*`/`tool-*`/`cmd-*`). Eigene Namen bleiben, im Dokument bleibt die Sprache der Oberfläche. |
| Kleinigkeiten: Größenfeld, Live-Filter-Symbol | 9dd09c9 | „Pinselstrich bearbeiten“ (Werkzeugvorgabe) → OK setzt die Größe auch im Inspektor (30 statt 35), Abbrechen lässt sie. Live-Filter-Zeile mit Namen. Das ⓘ (seit c419943 statt Zahnrad) ist ein Knopf „Live-Filter bearbeiten“/„Edit Live Filter“ mit Tooltip (`pi-fx-edit`, de+en). |
| BEFUNDE | ea88e06 | 9 CT-2-Zeilen abgehakt. |

## Nicht geändert

- **Kolorieren → Farbton 300:** kein Fehler. FX-4 rechnet den Farbton beim Umschalten auf den Kreis um (−60 → 300). CT-1a hat das bestätigt. CT-2 begann bei Farbton 0, und 0 bleibt 0. Dass der Farbton unabhängig vom Ausgangswert auf 300 springt, steht in keiner Messung. Deshalb habe ich nichts erfunden. Offen ist nur eine Messung im Original (INVENTORY 28/04 `hue_colorize`, 61/01 `d41_fields`, noch nicht aufgenommen).
- **„Neues Rechteck ohne Strich“:** gehört laut Auftrag zu PAR-X-TOOLS-3.
- **KI-Variante „Groß“ + Glätten 5 → Grenze −1:** steht nur in der Sammelliste von CT-2, nicht in BEFUNDE und nicht im Auftrag. Nicht bearbeitet.

## Annahmen

- Das Untermenü geht wie unter Windows rechts auf, ohne Platz links. Im 900 × 650-Fenster mit dem Inspektor unten ist rechts kein Platz, es geht also links auf. Darüber liegt es nie.
- Farbe der Werkzeugleiste mit Auswahl: Füllung ohne Füllart-Wechsel (`pi.fill {color}`). Das Füllwerkzeug behält „einfarbig“ (PAR 07/03).
- Unbekannter Texturname meldet nur dann einen Fehler, wenn es auf dem Rechner Texturdateien gibt. Ohne Ressourcen bleibt es wie bisher beim Namen.

## Tests

- fp-engine `fixd_ct2::` (2), `tools_fv16::`/`parx_tools2::`: grün
- fp-ui `fixd_ct2::` (8): Farbe ersetzen mit Maus, Strichart-Untermenü 900 × 650 (dreimal öffnen und wählen), Texturliste de/en, Esc in der KI-Karte (schlägt ohne die Änderung fehl), Werkzeugleisten-Farbe auf Auswahl/Rückgängig/ohne Auswahl, Deckkraft-Schieber, Größenfeld nach dem Dialog (schlägt ohne die Änderung fehl), Live-Filter-ⓘ
- fp-ui `color_picker`, `effect`, `live`, `menu` (einzeln nacheinander): grün, außer `i18n_menue` (siehe neue Befunde)
- fp-mcp alle Tests grün (neu `history_labels_in_response_language`)
- fp-i18n, `i18n_keine_festen_texte`, fp-neutral: grün; `cargo check --workspace --all-targets`: sauber

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- Farbwähler-Popup der Werkzeugleiste wirkt auf die Auswahl (FARB-1b)
- Deckkraft-Klappschieber im Fenster (FARB-1b 6)
- Strichart-Menü: Untermenü neben dem Hauptmenü (PINSEL-1)

## Nachprüfung (echtes Fenster, Brücke, 900 × 650)

1. Pinsel → Strichart-Liste → über „Einfach“ fahren: Das Menü ist ≈ 230 px breit, das Untermenü steht daneben und verdeckt keine Kategorie.
2. Rechteck-Werkzeug, Textur „Ader“ aus der Liste, Menge 50, Rechteck aufziehen: Textur ist sichtbar. Füllwerkzeug mit Ader 50 auf ein Rechteck: Textur ist sichtbar. `element.setTexture {name: "Ader"}` → ok. `pi.fill {texture: "gibtsnicht"}` → Fehler.
3. Rechteck gewählt, Werkzeugleiste Füllung HEX 123456 + Enter: Rechteck #123456, ein Schritt. Strich ff00ff: Strich da. Strg+Z zweimal: wie vorher.
4. Farb-Popup unten im Fenster, Deckkraft-Klappknopf: Der Schieber steht ganz im Fenster, 0 und 100 sind erreichbar.
5. Pixelauswahl, KI-Objektauswahl, Esc bei „Bild analysieren“: „abgebrochen“, die Auswahl bleibt.
6. MCP englisch `history` nach Schritten in der deutschen Oberfläche: englische Namen („Rectangle“, „Fill“ …).
7. „Pinselstrich bearbeiten“ (ohne Auswahl), Größe 30, OK: Das Inspektorfeld zeigt 30.
8. Farbe ersetzen grün → blau ziehen (neue exe): der ganze Strich blau.

## Neue Befunde

- `i18n_menue::every_command_and_tool_has_texts_in_de_and_en` ist rot. Es fehlen `cmd-select-liveEdge`, `cmd-select-moveMarquee` und `cmd-select-movePixels` (de+en); neue Befehle von PAR-X (Bitmap-Auswahl, cdceac5).
- `menu_fixes_fv13::*`/`menu_mn4::*` scheitern im gemeinsamen Testprogramm nur parallel (gemeinsamer Befehle-Ordner im Testprofil). Einzeln und mit `--test-threads=1` grün.
- Hinweis: Mein Edit an `crates\fp-engine\tests\tools_fv16.rs` hat dort gemischte Zeilenenden vereinheitlicht (3 Zeilen inhaltsgleich, a31daf6).
