# FIX-N33: Batch-Studio „+ Dateien…“/„Ordner…“ ohne Wirkung

Stand 13.10.2026, Slot `fp-s2`, Commit `7ab5fb3` (Code und Test).

## Ursache

`batch::show` nimmt das Studio für die Dauer des Zeichnens aus dem App-Zustand (`st.batch.take()`) und legt es danach zurück.
`files::pick_files` und `files::pick_folder` holten das Studio aber über `st.batch.as_ref()`. Beim Klick war es dort nicht vorhanden,
darum kehrten beide Funktionen still zurück: Es gab keinen Dialog und keine Meldung, weder nativ noch in der App.
Betroffen waren:

- „+ Dateien…“ und „Ordner…“ in der Knopfzeile,
- dieselben Knöpfe in der leeren Ablagefläche,
- „Ordner…“ und „Ordner mit Unterordnern…“ im ▾-Menü.

Strg+O war nicht betroffen, weil es außerhalb von `show` läuft.

Die anderen Vermutungen haben sich nicht bestätigt:

- Der Dialog lag nicht hinter einem Fenster.
- Das Studio zählt nicht als modaler Dialog.
- Die Inbox wurde abgeholt.

Das Studio ist kein eigenes Fenster, sondern ein Dokumentreiter im Hauptfenster. Deshalb ist das Hauptfenster der richtige Besitzer des Dialogs.

## Änderung

- `crates\fp-ui\src\batch\files.rs`: `pick_files(st, s: &Studio)` und `pick_folder(st, s: &Studio, recursive)` bekommen das Studio jetzt als Parameter. Alle sechs Aufrufstellen sind angepasst.
- `crates\fp-ui\src\batch\mod.rs`: Strg+O nimmt das Studio kurz heraus und reicht es weiter.
- Die übrigen Datei-Dialoge des Studios waren schon richtig, weil sie das Studio bereits als `s` übergeben:
  - Rezept öffnen und speichern unter (`banner.rs`)
  - Zielordner (`output_bar.rs`)
  - Bericht speichern (`report_ui.rs`)
  - ffmpeg-Pfad (`settings.rs`)
  - Parameterpfad eines Schritts (`recipe.rs`)

  „Bericht öffnen“ nutzt die Shell und keinen Dialog.

## Prüfung

- Regressionstest `batch_ui::n33_dateien_und_ordner_knoepfe_fuellen_die_liste` in `crates\fp-ui\tests\batch_ui.rs`, aufgerufen mit `cargo test -p fp-ui --test all batch_ui::`. Er klickt per Maus-Ereignis:
  - „Dateien…“ und „Ordner…“ in der Knopfzeile,
  - dieselben Knöpfe in der leeren Ablagefläche,
  - Strg+O.

  Er beantwortet den Dialog mit `FilePickOverride` und erwartet zwei Dateien in der Liste. Ohne den Fix scheitert er gleich am ersten Knopf (0 statt 2). Mit dem Fix sind alle 16 `batch_ui`-Tests grün.
- `cargo check --workspace --all-targets` läuft ohne Warnungen durch.
- Im echten Fenster habe ich den Slot-Build `target-s2\debug\firepixel.exe` mit eigenem Profil geprüft. Das Fenster lag rechts neben dem Original (x ≥ 1600), das Original wurde nicht angefasst. Bedient habe ich über die Brücke mit `ui_find`/`ui_input`. Das Skript liegt unter `re\_testdateien\fixn33\n33.ps1`, die Protokolle unter `log-internal.txt`/`log-native.txt` und die Bilder unter `re\_testdateien\fixn33\bilder\`.
  - **Dialog in der App** (Brücke, Normalfall): Ein Klick auf „Dateien…“ in der leeren Fläche öffnet den Dialog „Dateien für die Batch-Verarbeitung“. Nach `ui_file_dialog choose` mit zwei PNG-Dateien stehen beide in der Liste. Danach öffnet „Ordner…“ in der Knopfzeile den Ordner-Dialog. Nach `choose` mit dem Ordner ist `urlaub_03.png` in der Liste.
  - **Windows-Dialog** (`FP_FILE_DIALOG=native`, derselbe Weg wie beim normalen Start ohne `--mcp`):
    - Nach 0,3 s gibt es ein sichtbares Fenster der Klasse `#32770` mit dem Titel „Dateien für die Batch-Verarbeitung“. Sein Besitzer ist das Hauptfenster, und in der Z-Ordnung liegt es direkt über dem Hauptfenster.
    - Dateinamen per `WM_SETTEXT` + IDOK: Die Liste zeigt `urlaub_01`/`urlaub_02`.
    - „Ordner…“ öffnet „Ordner hinzufügen“, ebenfalls mit dem Hauptfenster als Besitzer. Danach steht `urlaub_03.png` in der Liste (Bild `native_5_liste_nach_ordner.png`).
  - Ein Bildschirmfoto des ganzen Desktops war nicht möglich (`CopyFromScreen`: „Handle ungültig“, die Sitzung ist ohne aktiven Desktop). Den Windows-Dialog habe ich deshalb mit `PrintWindow` aufgenommen (`native_2_dialog_fenster.png`, `native_4_ordnerdialog_fenster.png`).

## Für die Nachprüfung (NACHPRUEFUNG)

- **Ablauf:** Testversion normal starten (ohne `--mcp`) und *Datei ▸ Batch-Verarbeitung…* öffnen. Dann nacheinander klicken: „+ Dateien…“ oben, „+ Dateien…“ in der leeren Fläche, „Ordner…“ oben, „Ordner…“ in der leeren Fläche, ▾ ▸ „Ordner mit Unterordnern…“ und Strg+O.
- **Erwartung:** Jedes Mal erscheint sofort der Windows-Dialog über Firepixel. Die gewählten Dateien bzw. der Inhalt des gewählten Ordners stehen danach in der Liste.
- Zusätzlich prüfen, ob Rezepte ▸ „Öffnen…“/„Speichern unter…“ und Ziel ▸ „Ordner…“ einen Dialog zeigen.

## PARITAET

Keine Zeile betroffen, denn das Batch-Studio ist eine eigene Funktion.

## Neue Befunde

Keine.
