# FIX-B – rote Tests aus GT-1, Funktionsberichte, CT-1b (13.10.2026)

Slot `fp-s1` / `target-s1`, je Punkt ein Commit (Präfix `FIX-B:`), per `git cherry-pick` nach `main`
(`git log --grep "^FIX-B:"`). Arbeit nach Neustart der Sitzung (≈ 12:42) fortgesetzt.

## Umgesetzt

1. **Kodierung** – `bitmap.rs` hatte PAR-X schon repariert (`fcf0f86`). Repo-weit noch doppelt
   kodiert und zeilenweise repariert: `crates\fp-mcp\Cargo.toml` (Beschreibung),
   `crates\fp-trace\src\refine\shapes.rs` (Kommentare), `crates\fp-ui\tests\dialogs_des1.rs`,
   `crates\fp-ui\tests\select_all_perf.rs` (Kommentare/Ausgaben). Treffer in `docs\` sind Zitate.
   **Wächter** `crates\fp-neutral\tests\encoding.rs`: durchsucht `crates\`, `apps\`, `tools\`
   (rs, toml, ftl, jsf, js, py, ps1, json, md, txt …) und die Workspace-`Cargo.toml` nach
   `Ã`+Folgebyte, `Â`+Folgebyte, `â€…`, `â†…`, mitten im Text stehendem BOM; meldet Datei:Zeile.
   Muster nur als `\u{…}`, damit die Datei sich nicht selbst findet; Selbsttest mit
   „Füllen“, „…“, „–“, „°“, „Größe“, „Übersicht“, „„Text““.
2. **Leere Bitmaps** – `crates\fp-engine\tests\all_commands.rs` `file_roundtrip`: erwartet wird das
   Dokument **ohne** 0×0-Bitmaps (sie entfallen beim Speichern wie im Original, FIX-A `78d91bc`),
   zusätzlich: keine leere Bitmap in der Datei. `edit.insert.emptyBitmap`, `panel.layers.newBitmap`
   grün.
3. **Dialoge** (`dialogs_i18n` grün, 4/4):
   - *Numerisch transformieren*: Felder wieder bei 30 DLU, nur bei „Drehen“ rückt das Winkelfeld
     hinter „Winkel:“ → 321 px wie die Referenz (vorher 331, weil „Winkel:“ alle Modi verschob).
   - Bitmap-Auswahl speichern/wiederherstellen/löschen: längere Beschriftungen (qps) schieben die
     Felder nach rechts und verbreitern Dialog, Gruppen und Optionsfelder (Deutsch unverändert).
   - Voreinstellungen: alle Kästchen auf „Allgemein“ in die Seitenbreite (en „Developer options
     (pseudo-language in menu)“).
   - Referenz „Formularbreite“ des Gaußschen Weichzeichners 340 → 406 (Filteranordnung seit DLG-5
     `5f06b0c`; war hinter dem ersten Fehler verdeckt).
4. **Flackernde Tests** – `paint_live_pinsel1::large_bitmap_renders_only_affected_areas`: Bereich
   bleibt harte Prüfung, Zeit als Median je Bewegung, bester von bis zu 3 Läufen (< 40 ms).
   `preview_sync_bv5::doc_offsets`: zeichnet weiter, bis jedes Feld sein aktuelles Exportbild zeigt
   (höchstens ≈ 20 s), statt nach 2 Bildern abzubrechen. Beide grün.
5. **Funktionsberichte**
   - `edit.insert.newButton`/`newSymbol` (−1 Schritte): **Testfehler** – nach OK öffnet sich wie im
     Original (PM-5 D14) die Symbolbearbeitung als eigenes Dokument; `gesamt_logik` misst jetzt am
     Ausgangsdokument (Symbolbearbeitung schließen, zurück) → 1 Schritt, Rückgängig exakt.
   - `view.grid.editGrid`: legt wie im Original (PM-5 D18) keinen Schritt an; **Programmfehler** war,
     dass Strg+Z über einen früheren Schritt das Raster zurücksetzte (Raster liegt im Dokument-
     Schnappschuss). `Session::apply_history_snapshot` lässt das Raster jetzt beim Springen im
     Verlauf stehen (gleiche Seitenzahl). `gesamt_logik` kennt `NO_STEP_BY_DESIGN` (0 Schritte,
     Rückgängig lässt das Raster stehen). Logik Dialoge 66/66, Setzbefehle 31/31.
   - Ziehen „5 px zu wenig“: **kein Fehler** – die Oberkante 65 rastet mit den intelligenten
     Hilfslinien (Vorgabe ein, FV-14 Kante an Kante, Fang 5 px) an der Kante 60 des gelben
     Rechtecks ein. `gesamt_alltag` prüft jetzt beides: ohne Einrasten genau (40, 25), mit Einrasten
     (40, 20). Alltag 28/28.
   - Gesamttest: `file.updateHtml` nimmt mit Testvorgaben zuerst eine Datei, die zu einem eigenen
     Filter passt (nicht „Alle Dateien“) → `test.htm` (`file_dialog.rs`, nur Testvorgabe-Zweig);
     `file.batchProcess`: Probe kennt den Batch-Reiter (`st.batch`). Hauptmenü 348/349 (übrig nur
     🔶 KI-Steuerkanal, manuell).
6. **CT-1b**
   - `prompt()`: Neues `fp_engine::script_prompt`. `commands.runScript`/`commands.run` mit
     `prompts` (Liste Text/null) beantworten die Fragen der Reihe nach; fehlt eine Antwort, bricht
     der Lauf ab (Dokument und Verlauf unverändert) und das Ergebnis meldet
     `promptNeeded {text, default}`. Die Oberfläche (`actions.rs`) setzt bei Skripten aus Menü/Kürzel
     `prompts: []`, zeigt `dialogs\script_prompt.rs` (Titel „Firepixel“, Frage, Feld mit Vorgabe,
     OK/Abbrechen, Enter/Esc) und startet das Skript mit einer Antwort mehr erneut; Abbrechen = `null`.
     Ohne `prompts` (CLI, MCP, Batch, Brücke `run_command`) wie bisher der Vorgabewert.
     Am Ende ein Verlaufsschritt. Betrifft „Größe ausgewählter Objekte ändern“, „Drehen und
     Ausblenden“, „Breite festlegen“, „ALT-Text festlegen“ (im Gesamttest jetzt „Dialog“).
   - Trennlinien in ≡-Menüs: `menu.rs` `paint_separator` legt über die ganze Zeile des Trenners
     einen Klick-Fänger (ohne Wirkung, ohne Fokus), damit egui den Klick nicht dem nächsten Eintrag
     im Fangradius gibt. **Hinweis:** Im Offscreen-Test (`fixb_ct1b::separator_clicks_do_nothing`,
     echtes Popup wie `docking.rs`, 125 %, Klicks wie die Brücke) ließ sich der Fehler auch **ohne**
     den Fänger nicht nachstellen; der Test sichert das Verhalten ab, die Ursache im echten Fenster
     ist nicht belegt → Nachprüfung.
   - Vorschau im Browser / „Demo – aktuelles Dokument“: Ordner je Prozess eindeutig
     (`fp_engine::commands::export::preview_dir` → `%TEMP%\Firepixel\Preview\<pid>-doc<n>`);
     nach jedem `file.previewInBrowser.preview*` (auch aus Skripten) Statuszeile
     „Vorschau erstellt: <Ordner>“ / „Preview created: …“ (`ui-preview-created`, `ui.ftl` de/en).
   - Verlaufsnamen über die Brücke: `fp_i18n::with_ui_language` (Sprache vor allen offenen
     `with_language`) und `fp_i18n::step_label_in_ui_language` (Text aus `msg-step-*` zurück in die
     Oberflächensprache). `Session::execute` benennt Schritte damit; „Move“ → „Verschieben“.

Tests (alle grün, gezielt): `fp-neutral` (inkl. `encoding`), `fp-i18n` (inkl.
`with_ui_language_inside_with_language`), `fp-engine` `script_prompt` (Einheit), `all_commands::`,
`paint_live_pinsel1::`, `undo`; `fp-script` `prompt_fixb` (2); `fp-ui` `dialogs_i18n` (4),
`fixb_ct1b::` (5), `gesamt_logik::`, `gesamt_alltag::`, `gesamt_menu::`, `preview_sync_bv5::`,
`dialogs_dlg7::`, `kv3::`, `menu_fixes_fv13::`, `panel_menus_mn3::`, `menu_scroll_mn6::`;
`cargo check --workspace --all-targets`; i18n-Scanner.

## Befunde erledigt (BEFUNDE.md)

- (PAR-X-INSP-1) „`bitmap.rs`: Beschriftungen doppelt kodiert …“ – erledigt (PAR-X `fcf0f86`, Wächter FIX-B)
- (CT-1b) „Befehle ▸ Demo – aktuelles Dokument: Vorschau entsteht, aber keine Statusmeldung …“
- (CT-1b) „`prompt()` in Befehlsskripten zeigt keine Eingabe …“
- (CT-1b) „Verlauf-Bedienfeld zeigt über die Brücke ausgeführte Schritte englisch …“
- (CT-1b) „Bedienfeld-≡-Menüs: Trenner sind nicht wirkungslos …“ – Fänger eingebaut, bleibt offen
  bis zur Nachprüfung im echten Fenster
- GT-1 „Für FIX“ 1–7 (keine eigenen BEFUNDE-Zeilen)

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- JSF `prompt()` in Befehlsskripten mit Eingabedialog
- Raster bearbeiten: Strg+Z lässt das Raster stehen (PM-5 D18)

## NACHPRUEFUNG

- FIX-B-6a | echtes Fenster, Brücke | Rechteck wählen, Befehle ▸ Größe ausgewählter Objekte ändern | Dialog „Firepixel“ mit „Neue Größe in Prozent“ und Vorgabe; 25 + Enter → Rechteck auf 25 %, ein Verlaufsschritt; erneut, Esc → nichts geändert, kein Schritt. Ebenso Drehen und Ausblenden, Breite festlegen, ALT-Text festlegen; in en englische Frage.
- FIX-B-6b | echtes Fenster, Brücke | Ebenen-≡ öffnen, auf jede Trennlinie, 1 px darunter und 3 px darüber klicken (auch rechts am Rand der Zeile) | nichts wird ausgeführt, Menü bleibt offen; Klick mitten in den Eintrag darunter führt ihn aus. Bei Fehlschlag: Klickkoordinaten und Bild in den Befund (Ursache offscreen nicht nachstellbar).
- FIX-B-6c | echtes Fenster, Brücke | Befehle ▸ Demo – aktuelles Dokument; zweite Instanz parallel dasselbe | Statuszeile „Vorschau erstellt: …\Preview\<pid>-doc1“, je Instanz eigener Ordner.
- FIX-B-6d | Brücke | über MCP `run_command` `edit.nudge.right`, `select.none`, `element.addEffect` in deutscher Oberfläche | Verlauf-Bedienfeld zeigt „Verschieben“, „Auswahl aufheben“, „Filter hinzufügen“ (nicht englisch).
- FIX-B-5 | echtes Fenster | Raster bearbeiten (Abstand ändern, OK), dann Strg+Z | Raster bleibt, der vorherige Schritt wird zurückgenommen.

## Annahmen

- **A:** Neu angelegte leere Bitmaps (Einfügen ▸ Leere Bitmap, Ebenen ▸ Neues Bild) entfallen beim
  Speichern wie geöffnete leere PNGs (gemessen nur für geöffnete, re\22 §1); keine eigene Messung.
- **A:** Strg+Z und Wiederholen sowie Klick im Verlauf lassen das Raster stehen (D18 sagt Strg+Z);
  Seiten hinzufügen/löschen (andere Seitenzahl) stellen das Raster des Schnappschusses her.
- **A:** `prompt()` aus der Oberfläche läuft das Skript bis zur Frage, verwirft das Ergebnis und
  startet es nach der Antwort neu (Skripte sind dafür deterministisch genug). Fängt ein Skript den
  Abbruch selbst mit `try/catch` ab, läuft es ohne Antwort weiter (bei den mitgelieferten nicht).
- **A:** Titel des Eingabedialogs „Firepixel“ wie die `alert()`-Meldungen (MN-4); im Original nicht gemessen.
- Verlaufsnamen mit Platzhaltern (z. B. „Wiederholen: …“), die in einer anderen Sprache entstehen,
  bleiben in dieser Sprache (nur `msg-step-*` ohne Argumente werden zurückübersetzt).

## Neue Befunde

- keine (CT-1b N2, Markenname im Live-Filter-Menü, war nicht Teil des Auftrags und bleibt offen)
