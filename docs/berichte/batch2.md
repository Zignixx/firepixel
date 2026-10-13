# BATCH-2 – UI-Gerüst „Batch-Studio“ (Reiter im Hauptfenster)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §2 (SPEC-BATCH), Paket BATCH-2 aus §6.1.
Gestaltung wie der Startbildschirm (DESIGN §12.1): Banner mit Pixelfeld und Logo, Karten mit
weichem Schatten (Normal), heller als die Fläche (Dunkel), Leuchtrand in Gruppenfarbe (OLED).

## Umgesetzt (Commit `BATCH-2: …`, siehe `git log --grep BATCH-2`)

| Datei | Inhalt |
|---|---|
| `crates\fp-ui\src\batch\mod.rs` | `Studio` (Zustand in `AppState::batch`), `open`/`open_with_files`/`close`/`is_front`, Reiter „Batch“ links im Dokument-Reiterstreifen (`doc_tab`, ×, Mittelklick, Ablegen auf den Reiter), Aufteilung `Layout` (3 Spalten ≥ ~950 px Inhalt, darunter 2 Spalten mit Reiter Rezept/Vorschau, unter 720 px 1 Spalte mit Reitern), ziehbare Spaltentrenner, `frame_input` (Ablegen, Tasten vor den Kürzeln des Dokuments), eigener Rezept-Verlauf (Gesten-Bündelung), Datei-Dialog-Antworten über einen Eingang (`Inbox`), `register_templates` |
| `batch\kit.rs` | Karten, Banner, Knöpfe (Haupt/Normal/Ghost), Segment-Knopf, Schalter, Griff, Pfeil, ⋯, gestrichelter Rahmen, Gruppenfarben und -symbole, Zustandssymbole |
| `batch\banner.rs` | Banner (Titel, „n Dateien · Größe · Rezept (ungespeichert •)“), Umschalter Einfach/Profi (gemerkt in `BatchMode`), Menü „Rezepte“ (Vorlagen, Meine Rezepte aus `%APPDATA%\Firepixel\BatchRecipes`, Öffnen, Speichern, Speichern unter, Leeres Rezept), Menü ⋯ (Unterordner, Rückgängig/Wiederholen im Rezept, Schließen) |
| `batch\files.rs` | Karte ① Dateien: Dateien…/Ordner…/▾ (Ordner mit Unterordnern, offene Dokumente, „Unterordner einbeziehen“), Filter (Suche; Profi: `/regex`), Typ, Sortierung (Hinzugefügt/Name natürlich/Datum/Größe/Pixel), virtuelle Liste (44 px, nur sichtbare Zeilen), Kästchen = verarbeiten, Miniatur 36 px mit Schachbrett, Abzeichen Animation/Video, Maße · Größe · Format · Unterordner, Zustand ✓/⚠/✕/läuft mit Tooltip, Fuß mit Summe, Entfernen, Leeren; leerer Zustand mit großer Ablagefläche; Hinweis „n Dateien waren schon in der Liste“ |
| `batch\thumbs.rs` | Miniaturen im Hintergrund (Warteschlange, 2 Threads, Schlüssel Pfad + Änderungszeit, alle Formate über `fp-batch`) |
| `batch\recipe.rs` | Karte ② Rezept: Schrittkarten (56 px: Griff ⠿, Symbolchip, Titel, Zusammenfassung, Schalter, ⋯ Duplizieren/Nach oben/Nach unten/Entfernen, Pfeil), Akkordeon, Ziehen mit Einfügelinie, ⚠ bei Prüffehlern, unbekannter Schritt bleibt erhalten; „+ Schritt hinzufügen“ → Schritt-Galerie als Überlagerung (Suche, Gruppen, Kacheln 54 px, ↑/↓/Eingabe/Esc; Anfänger: nur Schritte mit `beginner()`-Parametern); feste Endkarten „Dateiname“ (Schnellformular bis BATCH-3: Davor, Dahinter, Schreibweise, Leerzeichen; Profi: Regex) mit Beispiel „alt → neu“ und „Speichern als“ (Format + passende Formatoptionen) |
| `batch\params.rs` | generisches Formular aus `Step::params()` (Kästchen, Schieber mit Zahl und Einheit, Auswahl, Mehrfachauswahl, Farbe, Text, Pfad mit Datei-Dialog, Zeit, JSON), rote Kante + Meldung bei ungültigen Werten, „Mehr Optionen“, automatische Zusammenfassung inkl. „+ n Profi-Einstellungen“, Anmeldung eigener Kartenformulare |
| `batch\preview.rs` | Karte ③ Vorschau: Reiter Vorher/Nachher/Vergleich (Teiler ziehbar)/Plan/Fehler (n); Zoom (Einpassen/100/200 %, Mausrad um die Maus, Doppelklick passt ein), Verschieben (Umschalt+Ziehen im Vergleich), Hintergrund Schachbrett/Weiß/Schwarz; Rechnung im Hintergrund, 150 ms entprellt, Generationszähler; danach volle Kodierung für die echte Dateigröße „Nachher“ (vorher „≈ wird ermittelt“); Angaben „Vorher B×H · Größe“, „Nachher B×H · Größe (−n %)“, Rechenzeit, Hinweise |
| `batch\output_bar.rs` | Ausgabe-Leiste (64 px ab 1200 px Breite, sonst zweizeilig 96 px): Ziel (Neuer Ordner „batch-output“ – Vorgabe –, Eigener Ordner…, Gleicher Ort → Sicherung an), Ordner…, Bei Konflikt (Neuen Namen/Überspringen/Überschreiben/Jedes Mal fragen), Profi: Ordnerstruktur, gleicher Ort: Sicherungen behalten; Trockenlauf (F5), Start (Strg+Eingabe, gesperrt mit Grund im Tooltip), Fortschritt „n von m · noch m:ss“, Abbrechen; Rückfrage-Karte bei „Jedes Mal fragen“ (mit „Für alle weiteren“) |
| `batch\report_ui.rs` | Ende-Karte (✓/⚠/✕-Zähler, Summe vorher → nachher, Dauer, Ordner; Ordner öffnen – nur ohne `FP_NO_SHELL_OPEN` –, Bericht speichern…, Fehlgeschlagene erneut, Neues Rezept, Schließen), Plan-Tabelle Quelle → Ziel mit Hinweisen, Fehlerliste mit Kopieren |
| `crates\fp-ui\tests\batch_ui.rs` | 15 Tests + Bilder (siehe unten) |
| `crates\fp-i18n\locales\{de,en}\batch.ftl` | Abschnitt „Oberfläche: Batch-Studio“ (`batch-ui-*`), Beschriftungen der Endkarte „Speichern als“ (`batch-param-output-*`) und von „Nur kopieren“ (`batch-param-file-copyOnly-action-*`) |

Bilder (1440 × 900, sechs Dateien, Schritte „Größe ändern“ (BATCH-4) und „Metadaten“ (BATCH-5), Endkarte „Speichern als“ offen,
Vergleich vorher/nachher): `docs\screenshots\batch2_{normal,dunkel,oled}_{de,en}.png`.
Erzeugen: `cargo test -p fp-ui --test all batch_ui::batch2_bilder -- --ignored`.

## Erledigte offene Punkte aus `batch1.md`

- `fp_batch::install()` in der App (`FirepixelApp::with_session`, damit auch Tests und die MCP-Brücke
  `batch.*` haben). Headless-MCP über `firepixel-cli mcp` war schon über `batch::intercept()` versorgt;
  `fp-mcp` selbst bleibt BATCH-9.
- `("crates/fp-batch/src", Mode::Msg)` in `SCAN_DIRS` des Scanners – grün.
- *Datei ▸ Batch-Verarbeitung…* (`file.batchProcess` ohne Parameter) öffnet das Studio statt des alten
  Assistenten (Formular `batch_form`/`batch_params` entfernt). Mit Parametern läuft der Befehl ohne
  Oberfläche: mit `recipe` → `batch.run` (`outDir` wird zu `out`), sonst wie bisher.

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-ui\src\lib.rs`: `pub mod batch;`, `fp_batch::install()` in `with_session`,
  `batch::frame_input(&ctx, st)` vor `files::handle_dropped_files`, `let studio = batch::is_front(st)` und
  `&& !studio` an Werkzeugen, Dock und Inspektor (Studio vorn → ausgeblendet; Arbeitsbereich unverändert).
- `crates\fp-ui\src\app_state.rs`: Feld `pub batch: Option<Box<crate::batch::Studio>>` (+ `batch: None`).
- `crates\fp-ui\src\canvas\mod.rs`: in `show_document_area` 4 Zeilen (Studio statt Leinwand), in
  `doc_tabs` 2 Zeilen (Reiter „Batch“ links, kein Dokumentreiter aktiv, solange das Studio vorn ist).
- `crates\fp-ui\src\dialogs\menu_dialogs.rs`: Zweig `file.batchProcess` → `crate::batch::open(st)`;
  `batch_form`/`batch_params` entfernt. Die Texte `dlg-md-batch*`/`dlg-md-files-selected` in
  `dialogs.ftl` sind damit evtl. unbenutzt (nicht angefasst, I18N-KERN prüfen).
- `crates\fp-engine\src\commands\export.rs` (EB): `file.batchProcess` – `files`/`outDir` jetzt optional,
  neu `recipe` → `batch.run` über `batch_host()`. Ohne `recipe` unverändert (fehlt `outDir`, meldet
  `p.str` den Fehler wie vorher die Pflichtprüfung).
- `crates\fp-ui\Cargo.toml`: `fp-batch = { path = "../fp-batch" }`; `Cargo.lock` entsprechend.
- `crates\fp-ui\tests\i18n_keine_festen_texte.rs`: `SCAN_DIRS` + fp-batch.
- `crates\fp-ui\tests\clipping_audit.rs`: Ansicht `View::Batch` („Batch-Studio“: Studio mit drei
  Dateien, Schritt, offener Endkarte; Prüfbereich ganze Fläche unter den Leisten), `setup` schließt das
  Studio in allen anderen Ansichten (`st.batch = None`).

## Schnittstellen für BATCH-3 … 11

- **Eigene Karte eines Schritts** (statt des generischen Formulars):
  ```rust
  fp_ui::batch::params::register_card_body("resize", my_body); // fn(&mut egui::Ui, &mut CardCtx) -> bool
  ```
  `CardCtx { step_id, index: Option<usize>, params: &mut Value, defs: &[ParamDef], mode: Mode,
  show_more: &mut bool, files: &[ItemInfo], recipe: &Recipe, enabled: bool, pick_path: Option<String> }`.
  Das Formular ändert `ctx.params` (JSON-Objekt) und liefert `true` bei Änderung; das Studio schreibt
  das in einem Rückgängig-Schritt je Geste ins Rezept. `ctx.pick_path = Some(param)` öffnet den
  Datei-Dialog für einen Pfad-Parameter. Bausteine: `params::generic_body` (Rest wie gewohnt),
  `params::param_label`, `value_label` (sucht `batch-param-<id>-<param>-<wert>`, `batch-value-<id>-<param>-<wert>` (BATCH-4/5), `batch-choice-<id>-<param>-<wert>` (BATCH-6), `batch-value-<wert>`, sonst den Wert; Formate als Eigenname),
  `kit::*`. Anmelden einmal je Prozess, z. B. aus `pub mod <datei>;` in `batch\mod.rs` + einer
  `register()`-Zeile in `open()` – oder aus der eigenen Initialisierung.
- **Endkarten**: dieselbe Anmeldung mit `"rename"` (BATCH-3, `rename_ui.rs`; `params` = `RenameSpec` als
  JSON, `files` = sichtbare Dateien in Verarbeitungsreihenfolge für die Live-Tabelle) bzw. `"output"`
  (Format + Optionen; BATCH-8 z. B. GIF-Optimierer-Regler). Ohne Anmeldung: Schnellformular bzw.
  generisches Formular.
- **Zusammenfassung** der zugeklappten Karte: `Step::summary(&params)` (Parameter mit Vorgaben); leer →
  automatische Zusammenfassung aus den `beginner()`-Parametern.
- **Galerie**: ein Schritt erscheint im Anfänger-Modus, wenn mindestens ein Parameter `.beginner()` ist
  (oder er keine Parameter hat); Name `batch-step-<id>`, Erklärung `batch-step-<id>.description`,
  Symbol/Farbe nach `StepGroup` (`kit::group_icon`, `kit::group_color`).
- **Vorlagen** (BATCH-9): `fp_ui::batch::register_templates(fn() -> Vec<Template { title, description, recipe }>)`;
  erscheinen oben im Menü „Rezepte“.
- **Zeitleiste** (BATCH-7, `timeline.rs`): `preview::Shown` hat Vorher/Nachher-Texturen und Maße; die
  Animation (Frames) liefert `fp_batch::PreviewResult::after_item` – noch nicht in `Shown` übernommen
  (BATCH-7 ergänzt dort `frames` und zeichnet unter `image_view`).
- Zustand für Tests/Brücke: `batch::studio(&st)`, `studio_mut`, `Studio::{add_paths, edit, undo, redo,
  end_gesture, visible, to_process, issues, set_recipe}`, `output_bar::{start, dry_run, blocked}`,
  `recipe::add_step`, `thumbnails_pending`.

## Tests (alle grün)

`cargo test -p fp-ui --test all batch_ui::` (15 + 1 Bildtest): Menü öffnet das Studio als Reiter, kein
zweites Studio, neues Dokument kommt nach vorn, Reiter holt das Studio zurück, Strg+Z wirkt auf das
Rezept und nicht auf das Dokument, × schließt; `file.batchProcess` mit `recipe` → Bericht; Ablegen
(Dateien, Ordner rekursiv, Textdatei übersprungen, Doppelte nicht, Hinweis), Entf; leerer Zustand,
Start gesperrt; §4b: Schritt 50× hinzufügen/entfernen ohne Reste, Parameter 50× ändern = ein
Verlaufsschritt mit Endwert, gleicher Wert = keine Änderung, Folge A, B, ↶, C, ↷; Galerie per Klick,
Esc lässt das Rezept unverändert, Profi-Suche nach ID + Eingabe, Schalter = ein Schritt, Strg+Z;
Alt+↑ verschiebt (offene Karte folgt), Einfügeposition beim Ziehen; Moduswechsel verliert keine Werte
(„+ n Profi-Einstellungen“); Vorschau zeigt nach schnellem Auswahlwechsel die zuletzt gewählte Datei
(nie die alte), echte Dateigröße nachher, Rezeptänderung ersetzt; Trockenlauf schreibt nichts, Lauf über
Strg+Eingabe → `batch-output`, zweiter Lauf → `batch-output-2`, Originale bleiben; Rezept speichern
(`.fpbatch.json`) und laden (neuer Verlauf); nichts abgeschnitten in 3 Designs × de/en × 1024×768 /
1920×1080 × 100/150 % (auch mit offener Galerie) und schmal (700/900 px).

Dazu: `clipping_audit` mit `FP_RAND_VIEWS=Batch FP_RAND_FULL=1` (24 Varianten): **0 Funde**,
18 Hinweise (Bildlauf), 16 mit „…“ gekürzt. `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`,
Scanner `i18n_keine_festen_texte::` (mit fp-batch), `cargo test -p fp-neutral`, `clippy -p fp-ui --tests`
ohne Warnungen in den eigenen Dateien.

## Annahmen

- Kästchen in der Dateiliste = „verarbeiten“ (aktiviert); verarbeitet werden sichtbare **und**
  aktivierte Dateien in Listenreihenfolge (bestimmt die Nummerierung).
- Andere Dateiarten (z. B. `.txt`) nimmt die Liste nur an, wenn das Rezept ausdrücklich einen
  Dateischritt hat (z. B. „Nur kopieren“) und sonst nichts – ein leeres Rezept zählt nicht.
- Modus heißt in der Oberfläche „Einfach | Profi“ (EN „Simple | Pro“); Vorgabe Einfach, gemerkt in
  der Voreinstellung `BatchMode`, „Unterordner einbeziehen“ in `BatchRecursive`.
- Studio vorn: Werkzeugbedienfeld, Dock und Inspektor ausgeblendet (Symbolleiste bleibt). Ein neu
  geöffnetes/aktiviertes Dokument holt die Dokumente nach vorn; das Studio bleibt als Reiter offen.
- Tasten bei vorn liegendem Studio werden verbraucht, damit sie nicht auf das verdeckte Dokument
  wirken: Strg+Z/Y, Entf/Rücktaste, Pfeile, Strg+A (alle aktivieren), F5, Strg+Eingabe, Strg+O.
- Schließen des Studios (× oder Menü) während eines Laufs bricht den Lauf sauber ab (ohne Rückfrage).
- Ziel „Gleicher Ort“ schaltet die Sicherung ein (SPEC §2.7); BATCH-1 erzwingt sie ohnehin im Plan.
- Rezepte ohne Namen bekommen beim Speichern den Dateinamen als Namen.

## Offen (nicht in BATCH-2 bzw. für BATCH-11)

- Rückfrage „Batch läuft noch – abbrechen und schließen?“ beim Schließen des **Fensters** (braucht
  einen Haken in `actions.rs`/Quit); heute bricht das Beenden den Lauf über `Drop` ab.
- `firepixel.exe --batch [dateien…]` (apps\firepixel, HG) und Startseiten-Eintrag (P2); Haken dafür:
  `fp_ui::batch::open_with_files(st, &paths)`.
- Filter nach Größe/Pixel/Transparenz und Glob (Profi), Sortieren per Ziehen (P2).
- Hinweis „JPEG füllt Transparenz mit Weiß“ beim Zielformat JPEG; Zeitleiste/Abspielen (BATCH-7).
- Leertaste/Eingabe in der Dateiliste (P2), „Als Vorlage speichern“/„Hilfe“ im Kartenmenü.
- Fortschritt je Datei (`ItemProgress`) wird noch nicht angezeigt (nur fertig/gesamt).

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes Fenster (Brücke): *Datei ▸ Batch-Verarbeitung…* → Reiter „Batch“; Dateien aus dem Explorer auf
  das Studio ziehen (auch Ordner) → Liste mit Miniaturen; „Schritt hinzufügen“ → Galerie; Endkarte
  „Speichern als“ WebP 82 → Vorschau Vergleich, Größe nachher; „6 Dateien verarbeiten“ → Fortschritt,
  Ende-Karte, `batch-output` neben den Quellen; zweiter Lauf → `batch-output-2`. Erwartet: alles flüssig,
  nichts abgeschnitten, Strg+Z wirkt nur auf das Rezept.
- Großer Lauf (≥ 1 000 Fotos) mit Abbrechen über die Leiste: keine `.fptmp`, Liste bleibt bedienbar
  (virtuelle Liste), Miniaturen laden nach und nach.
- Gesamttests/Bildserien, die alle Menübefehle auslösen: `file.batchProcess` öffnet jetzt das Studio
  (vorn) statt eines Dialogs – falls ein Gesamttest danach die Leinwand erwartet, Studio schließen
  (`st.batch = None`) bzw. Befehl ausnehmen.
- `clipping_audit` voll (alle Ansichten): Einträge „Dialog file.batchProcess“ der Erlaubnisliste sind
  jetzt „nicht mehr gefunden“ (Dialog gibt es nicht mehr) – RAND streicht sie.

## Neue Abhängigkeiten

Keine neuen Crates (fp-ui hängt jetzt von `fp-batch` ab; alles schon im Baum, permissiv).
