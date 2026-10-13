# FIX-G2 – HTML-Einrichtung, Rauschen, Rechtschreibung, Exportvorgabe (13.10.2026)

Auftrag aus `docs\parity\TWEAKS_2026-10-13.md`: TW-03, TW-06, TW-09, TW-13 und Q-07 (Nutzerentscheid 21:20).
Slot `fp-s3`. Commit: siehe `git log --grep FIX-G2`.

## Umgesetzt

| ID | Änderung | Dateien |
|---|---|---|
| TW-03 | HTML-Einrichtung, Register „Allgemein“: neues Kontrollkästchen „CSS in externe Datei schreiben“ / „Write CSS to external file“. Vorgabe an, grau solange „CSS für Popupmenüs verwenden“ aus ist (Häkchen bleibt stehen, re\18 §6.2). Gespeichert im Dokument als `external_css` (`ECSi`). **Wirkung im HTML-Export:** Bei CSS-Pop-up-Menüs stehen die Menüstile in `<Basisname>.css` neben der HTML-Datei, im Kopf nur `<style type="text/css" media="screen">@import url("…");</style>`, wie in der Original-Vorlage (`WRITE_CSS` → `WRITE_EXTERNAL_CSS`). Ist ein Dateiname im Dokument gespeichert (`externalCSSFilename`), gilt dieser. Ohne Pop-up-Menüs oder mit JS-Menüs entsteht keine CSS-Datei. „HTML-Code kopieren“ legt die CSS-Datei mit den Bildern ab. | `fp-ui` dialogs/html_setup.rs, `fp-export` html.rs + doc_export.rs (`external_css_file`), ftl DE/EN `dlg-hs-external-css` |
| TW-06 | Register „Allgemein“ zweispaltig wie das Original: links HTML-Stil, Kommentare und Kleinbuchstaben; rechts Erweiterung, CSS für Popupmenüs und externe CSS-Datei. Dokumenteigenschaften: Wiederholen links, Anhang rechts in derselben Zeile. **Anhang-Vorgabe „scroll“:** Die Zuordnung von `CBAi` war vertauscht. Laut re\08 §1.5 gilt 0 = fixed, 1 = scroll, und die Testdateien des Originals haben 1. Korrigiert im Dialog, in `body_css` (Export) und in `HtmlSettings::cs6_default` (jetzt 1). | html_setup.rs, doc_export.rs, fp-model export.rs |
| TW-09 | Rauschen im Menüdialog öffnet mit 50. Ursache: Die gemerkten Filterwerte (DLG-5) wurden in `preferences.json` gespeichert. Im Laufprofil stand noch `AddNoiseAmount: 20` aus einem früheren Lauf. Das Original legt diese Werte nicht in seiner Voreinstellungsdatei ab (`re\_testdateien\ux\o_prefs_werte.txt` hat keinen solchen Schlüssel) und startet nach einem Neustart wieder mit den Vorgaben. Jetzt gilt: Die Werte bleiben in der Sitzung app-weit gemerkt, `Preferences::to_json`/`save` lässt die Schlüssel in `prefs::SESSION_ONLY_KEYS` weg (Gauß, Unscharf maskieren, Helligkeit/Kontrast, Bewegung, Farbton/Sättigung, Rauschen, Radial, Zoom). Die „Vorschau“-Schalter werden weiter gespeichert. | `fp-engine` prefs.rs |
| TW-13 | Rechtschreibung einrichten: „Pfad zum persönlichen Wörterbuch“ ist vorbelegt mit `%APPDATA%\Firepixel\Dictionaries\Personal.dic`. Das ist der Profilordner neben `preferences.json`; `Dictionaries` wird schon nach Wörterbüchern durchsucht. Erst OK übernimmt den Pfad. Dabei zieht die bisherige Wortliste aus den Voreinstellungen in die Datei um (vorhandene Logik). | `fp-engine` commands/spelling.rs (`default_personal_dictionary`) |
| Q-07 | Exportdialog: Ein Dokument ohne Segmente startet mit „Nur Bilder“, damit HTML „Ohne“ (grau), solange im Dokument noch keine Art gewählt wurde (`setByUser` = `not_yet_set`). Danach gilt die zuletzt exportierte Art des Dokuments. Gemerkt wird sie jetzt auch, wenn die Seite noch keine Exporteinstellungen hatte (vorher ging die Wahl dann verloren). Dokumente mit Segmenten starten wie bisher mit „HTML und Bilder“. | `fp-ui` dialogs/export.rs |

## Tests

- `fp-export` (lib, alle 52 grün): neu `html::tests::external_css_for_popup_menus`, `doc_export::tests::external_css_file_in_html_export` (Export-Test der CSS-Datei: `test.css` + `@import`, aus = keine Datei) und `doc_export::tests::background_attachment_codes`.
- `fp-engine` (lib): neu `prefs::tests::filter_memory_is_session_only`.
- `fp-ui --test all fixg2::` (5 neu, grün): HTML-Einrichtung (Vorgaben, Speichern), Anordnung in DE/EN mit Grau-Regel (kittest), Rauschen 50 nach Neustart, Pfad des Wörterbuchs vorbelegt, Exportvorgabe ohne/mit Segmenten und letzte Wahl.
- Weiter grün: `export_e1/e2/e3`, `fixc_dialogs`, `dialogs_dlg1`, `dialogs_dlg1b`, `dialogs_dlg5`, `dialogs_dlg10`, `form_form1`, `image_preview_p4`, `html_alt_fixe`, `nz4`; `fp-model`, `fp-fwpng`, `fp-i18n`, `fp-neutral`; i18n-Scanner (dialogs) 0.
- `cargo check --workspace --all-targets` ohne Warnungen. Clippy meldet in den geänderten Zeilen nichts Neues.
- Bildschirmfotos über die MCP-Brücke des Slot-Builds mit eigenem Profil: `re\_testdateien\par\fixg2\`
  - `01_html_setup_general.png`: zweispaltig, „scroll“, ☑ externe CSS
  - `02_export_no_slices.png`: Nur Bilder, HTML „Ohne“
  - `03_spelling_setup.png`: Pfad vorbelegt
  - `04_add_noise.png`: Menge 50
  - `05_html_setup_general_en.png` und `06_spelling_setup_en.png`: englisch
  - `c_*.png`: Ausschnitte

## Erledigte Befunde (Wortlaut-Anfang)

- TW-03 „HTML-Einrichtung, Register Allgemein: ☑ „CSS in externe Datei schreiben““
- TW-06 „HTML-Einrichtung: Anhang „scroll“, zweispaltige Anordnung“
- TW-09 „Rauschen hinzufügen: Menge 50“
- TW-13 „Rechtschreibung einrichten: Pfad zum persönlichen Wörterbuch vorbelegt“
- Q-07 „Vorgabe für ein Dokument ohne Segmente auf „Nur Bilder“ stellen?“ (Nutzerentscheid umgesetzt)

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- D13 HTML-Einrichtung (Register Allgemein: externe CSS, Anordnung, Anhang)
- D45 Rauschen hinzufügen (Vorgabe 50 nach Neustart)
- D24 Rechtschreibung einrichten (Pfad vorbelegt)
- D07 Exportieren (Vorgabe ohne Segmente)

## Für die Nachprüfung

1. **HTML-Einrichtung:** Datei ▸ HTML-Einrichtung… öffnen. Erwartet: zwei Spalten wie im Original-Bild `dialogs-d11-d20/orig/04_d13_tabs_normal.png`, Anhang „scroll“, ☑ „CSS in externe Datei schreiben“. Dann „CSS für Popupmenüs“ abwählen: Das zweite Kästchen wird grau und bleibt angehakt.
2. **Export mit externer CSS:** Segment mit Pop-up-Menü anlegen und als HTML und Bilder exportieren. Erwartet: `<Name>.css` neben `<Name>.htm`, im Kopf `@import url("<Name>.css")`, das Menü funktioniert im Browser. Mit abgewählter Option: keine CSS-Datei, die Stile stehen im Kopf.
3. **Rauschen:** Rauschen mit 20 bestätigen und erneut öffnen: 20. Programm neu starten und erneut öffnen: 50.
4. **Rechtschreibung:** Text ▸ Rechtschreibprüfung einrichten… Erwartet: Pfad `…\Firepixel\Dictionaries\Personal.dic`. Ein Wort hinzufügen und OK. Erwartet: Die Datei existiert und enthält das Wort.
5. **Export ohne Segmente:** `par_vector` öffnen, Datei ▸ Exportieren… Erwartet: „Nur Bilder“, HTML „Ohne“. „HTML und Bilder“ wählen und exportieren, dann den Dialog erneut öffnen: „HTML und Bilder“.

## Hinweise / neue Befunde

- Die Dokumentvorgabe `background_attachment` ist jetzt 1. Dokumente, die Firepixel vorher neu angelegt und gespeichert hat, tragen noch 0 und gelten damit als „fixed“. Das ist selten, weil die Option nur mit Hintergrundbild wirkt. Keine Umstellung beim Laden.
- Der Dialog „HTML-Einrichtung“ behält seine feste Höhe (430 px, für die Register Tabelle und Dokumentspezifisch). Unter „Allgemein“ bleibt dadurch unten etwas mehr Luft als im Original.
- „Durchsuchen…“ neben „Hintergrundbild“ fehlt weiter (nicht im Auftrag).
- Offen bleibt, ob das Original die letzte Exportart auch dokumentübergreifend für neue Dokumente merkt. Umgesetzt ist die Wahl je Dokument, wie im Datenmodell des Originals (`exportFilesStyle` + `setByUser`).

## Nachtrag: roter Test `canvas::tests::open_dialog_blocks_canvas_and_export_area_opens_dialog`

Der Test hat nichts mit Q-07 zu tun. Er war veraltet: Seit PAR-X (`a0c725f`, PAR 08/20) öffnet Enter bzw. ein Doppelklick im Exportbereich wie im Original die **Bildvorschau**, Register „Datei“, mit gesetztem Exportbereich, und nicht mehr den Export-Dialog. Deshalb war `dialogs.export.area` leer. Der Test prüft jetzt die Bildvorschau: Register 1 und Zuschnitt `[20, 20, 120, 90]` in allen Ansichten. Danach ist er grün.
