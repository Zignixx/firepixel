# Technisches immer Englisch – Bestandsaufnahme und Umbenennung (TE-1, 12.10.2026)

Regel: `docs\AGENTEN_REGELN.md` §1c. Übersetzt wird nur Oberflächentext; alles Technische (Ordner,
Dateien, Schlüssel, CLI, MCP, Umgebungsvariablen) ist englisch. Diese Liste erfasst die
**Laufzeit-Oberfläche nach außen** in `crates\` und `apps\` (Stand Commit von TE-1).

Wächter: `cargo test -p fp-neutral --test technical_english` sucht in den Laufzeitquellen
(`crates\*\src`, `apps\*\src`, ohne Testmodule) nach deutschen Wörtern und Umlauten in
Pfad-Konstruktionen (`.join("…")`, `temp_dir("…")`, `PathBuf::from` …), Dateinamen-Literalen
(`*.json`, `*.md`, `*.png` …), serde-/clap-Namen und Umgebungsvariablen. Alte Namen, die nur noch
für die Migration gelesen werden, stehen in Konstanten `LEGACY_*` oder als serde-`alias`.
Bewusste Ausnahmen stehen mit Begründung in der Erlaubnisliste des Tests (siehe Abschnitt C).

## Migration (wie sie wirkt)

`fp_i18n::paths` (neu, in `crates\fp-i18n\src\paths.rs`), benutzt von allen Ordnern unten:

1. Neuer Ordner existiert → er wird benutzt, der alte bleibt unberührt (keine Migration). Lesende
   Stellen (Sprachen, KI-Modelle) schauen zusätzlich in einen übrig gebliebenen alten Ordner.
2. Nur der alte existiert → er wird beim ersten Zugriff nach dem Start **einmal umbenannt**
   (`std::fs::rename`, gleicher Elternordner, nichts wird kopiert oder gelöscht).
3. Umbenennen geht nicht (kein Schreibrecht neben der exe, gesperrte Datei …) → der alte Ordner
   wird weiter benutzt (lesen und schreiben). Der Versuch wird in derselben Sitzung nicht wiederholt.
4. Keiner existiert → neuer Name.

Eingeschaltet wird das Umbenennen nur von den Programmen selbst (`firepixel.exe`, `firepixel-cli.exe`
rufen `fp_i18n::paths::enable_migration()`), und auch dort **nicht** in Test- und Entwicklungsläufen
(`FP_NO_SHELL_OPEN`, `FP_NO_MIGRATE` oder `CARGO_MANIFEST_DIR` gesetzt – `cargo run`/`cargo test`
setzen Letzteres auch für Kindprozesse). `FP_MIGRATE=1` erzwingt es (nur mit umgebogenem
`APPDATA`/`LOCALAPPDATA` in Tests benutzt). Ohne Schalter gilt Fall 3: der alte Ordner wird gelesen.
Damit migrieren Tests und Agenten-Läufe die echten Ordner des Nutzers nie; das passiert erst, wenn
der Nutzer die nächste Version startet.

## A. Umbenannt (alt → neu) – 31 Namen

| # | Art | alt | neu | Fundstelle | Migration |
|---|---|---|---|---|---|
| 1 | Ordner | `%APPDATA%\Firepixel\Sprachen` (bzw. `$XDG_CONFIG_HOME/firepixel/Sprachen`, `~/.config/firepixel/Sprachen`) | `…\Languages` | `crates\fp-i18n\src\lib.rs` `user_language_dir`, `language_dir_in` | einmal umbenennen, sonst Rückfall; alter Ordner neben neuem wird mitgelesen |
| 2 | Ordner | `Sprachen\` neben der exe | `Languages\` | `crates\fp-i18n\src\lib.rs` `exe_language_dir` | wie 1 (ohne Schreibrecht: Rückfall) |
| 3 | Ordner | `%LOCALAPPDATA%\Firepixel\Modelle` | `…\Models` | `crates\fp-ai\src\models.rs` `local_folder`, `folder_in` | wie 1; alter Ordner neben neuem wird mit durchsucht |
| 4 | Ordner | `Modelle\` neben der exe | `Models\` | `crates\fp-ai\src\models.rs` `exe_folder` | wie 3 |
| 5 | Ordner | Rückfall-Downloadordner `Modelle` (relativ) | `Models` | `crates\fp-ai\src\models.rs` `download_folder` | – (nur wenn kein Ordner bestimmbar) |
| 6 | Ordner | `%APPDATA%\Firepixel\Allgemeine Bibliothek` | `…\Common Library` | `crates\fp-ui\src\panels\library.rs` `common_library_dir_in` | wie 1 |
| 7 | Datei | `%APPDATA%\Firepixel\Commands\.firepixel-befehle` (Markierung „eingebaute Befehle installiert“) | `.firepixel-commands` | `crates\fp-script\src\builtin_commands.rs` `install_builtin_commands` | Datei einmal umbenannt, sonst alte gelesen (gelöschte Befehle kommen nicht wieder) |
| 8 | Ordner (temp.) | `%TEMP%\Firepixel\Vorschau` | `Preview` | `crates\fp-engine\src\commands\export.rs` (Vorschau im Browser) | keine (temporär) |
| 9 | Ordner (temp.) | `…\Vorschau\dok<N>` | `…\Preview\doc<N>` | ebd. | keine |
| 10 | Ordner (temp.) | `%TEMP%\Firepixel\EMail` | `Email` | ebd. (Datei ▸ Senden per E-Mail) | keine |
| 11 | Ordner (temp.) | `%TEMP%\Firepixel\Druck` | `Print` | ebd. (Drucken ohne `outDir`) | keine |
| 12 | Datei | `<name>_druck_<n>.png` | `<name>_print_<n>.png` | ebd. | keine |
| 13 | Ordner (temp.) | `%TEMP%\Firepixel\Hilfe` | `Help` | `crates\fp-engine\src\commands\misc.rs` `help_file` | keine |
| 14 | Datei (temp.) | `KURZHILFE.md` | `QUICK_HELP.md` | ebd. | keine |
| 15 | Datei (temp.) | `%TEMP%\Firepixel\zwischenablage-<pid>.png` | `clipboard-<pid>.png` | `crates\fp-ui\src\os_clipboard.rs` | keine (wird sofort gelöscht) |
| 16 | JSON-Schlüssel | `befehl_id` (Bedienfeldtabelle) | `command_id` | `crates\fp-engine\data\commands.json`, `src\catalog.rs`, Erzeuger `tools\golden\fp-engine\gen_data.py` | serde `alias = "befehl_id"` |
| 17 | CLI-Befehl | `firepixel-cli sprache` | `language` | `apps\firepixel-cli\src\main.rs`, `src\language.rs` (bis TE-2 `sprache.rs`) | alter Name als versteckter Alias |
| 18 | CLI-Befehl | `sprache pruefen` | `language check` | ebd. | versteckter Alias |
| 19 | CLI-Befehl | `sprache vorlage` | `language template` | ebd. | versteckter Alias |
| 20 | CLI-Befehl | `sprache liste` | `language list` | ebd. | versteckter Alias |
| 21 | CLI-Option | `--eingebaut` | `--builtin` | ebd. | versteckter Alias |
| 22 | CLI-Option | `--kurz` | `--short` | ebd. | versteckter Alias |
| 23 | CLI-Option | `--streng` | `--strict` | ebd. | versteckter Alias |
| 24 | CLI-Option | `--ueberschreiben` | `--overwrite` | ebd. | versteckter Alias |
| 25 | CLI-Option | `--von` | `--from` | ebd. | versteckter Alias |
| 26 | Fluent-IDs | `cli-sprache`, `cli-sprache-{pruefen,vorlage,liste}`, `…-arg-{pfad,eingebaut,kurz,streng,zielordner,ueberschreiben,von}` (12) | `cli-language`, `cli-language-{check,template,list}`, `…-arg-{path,builtin,short,strict,target-dir,overwrite,from}` | `crates\fp-i18n\locales\{de,en}\cli.ftl` (bis TE-2 `kommandozeile.ftl`; aus den Argument-IDs abgeleitet) | – |
| 27 | GUI-Option | `firepixel --design=dunkel` | `--design=dark` | `apps\firepixel\src\main.rs`, `ui.ftl` (bis TE-2 `oberflaeche.ftl`; `ui-cli-help`) | `dunkel` weiter gültig |
| 28 | MCP-Wert | `ui_action {"theme": "dunkel"}` (Schema-enum) | `"dark"` | `crates\fp-mcp\src\tools.rs`, `crates\fp-ui\src\bridge.rs`, `mcp.ftl` | `dunkel`/`hell` weiter angenommen |
| 29 | MCP-Ausgabe | `ui_state` → `"theme": "dunkel"` | `"dark"` | `crates\fp-ui\src\bridge.rs` `theme_name` | – |
| 30 | MCP-Beispiel | `describe_command` Beispielpfad `C:\pfad\datei.fw.png` | `C:\path\file.fw.png` | `crates\fp-mcp\src\tools.rs` | – |
| 31 | Umgebungsvariable | `FP_DATEIDIALOG` mit `abbrechen` / `pfad:<ordner>` / `nativ` (auch `ui_state.file_dialog.last_events[].by`) | `FP_FILE_DIALOG` mit `cancel` / `path:<folder>` / `native` | `crates\fp-ui\src\file_dialog.rs`, `docs\AGENTS(.en).md`, `mcp.ftl` | alter Name und alte Werte weiter gelesen |

Dazu englische Rückfall-Wertnamen der CLI-Hilfe im Quelltext (`DATEI`/`ORDNER` → `FILE`/`FOLDER`;
angezeigt werden weiter die übersetzten Werte aus `kommandozeile.ftl`).

Doku angepasst: `docs\AGENTS.md`, `docs\AGENTS.en.md`, `docs\ANLEITUNG.md`, `docs\ANLEITUNG.en.md`,
`docs\MODELLE.md`, `crates\fp-i18n\README.md`, Texte in `kommandozeile.ftl`, `dialoge.ftl`
(`dlg-ai-folder-hint`), `oberflaeche.ftl`, `mcp.ftl`. Die MCP-Ressourcen (`crates\fp-mcp\docs`,
`jsf.md` wird erzeugt) enthielten keine deutschen technischen Namen. Automatisch erzeugte
Prüfberichte (`docs\I18N_PRUEFUNG.md`, `docs\funktionstest\*_auto.md`) zeigen die alten Pfade, bis
sie neu erzeugt werden.

Tests: `crates\fp-i18n\tests\paths_english.rs`, `crates\fp-ai\tests\models_path.rs`,
`crates\fp-engine\tests\catalog_english.rs`, `apps\firepixel-cli\tests\language_english.rs`,
Wächter `crates\fp-neutral\tests\technical_english.rs`.

## B. Geprüft und schon englisch (bleibt)

- `%APPDATA%\Firepixel\preferences.json`, `data\app.ron` (eframe), `Commands`, `Extensions`,
  `Shortcuts\<set>.json`, `Workspace Layouts`, `URLs.txt`, `Dictionaries`; `%TEMP%\Firepixel\Scan\scan.png`;
  MCP `mcp-http.json`, `mcp-bridge.json` (`%LOCALAPPDATA%\Firepixel` bzw. XDG_STATE_HOME).
- Alle Schlüssel in `preferences.json` (Originalnamen wie `CurrentShortcutSet`, `AiModelFolder`,
  `Language`, `Firepixel_*`) und im eframe-Speicher (`firepixel-ui`, `window`, `fp-ui-theme`).
- serde-Namen aller Typen (geprüft per Skript über alle `Serialize`/`Deserialize`-Typen) außer
  `befehl_id` (A16). `fp-trace` `Preset`: serde-Namen schon `blackWhite`/`photo` (Aliasse `schwarzweiss`, `foto`).
- CLI `run`, `batch`, `render`, `info`, `commands`, `convert`, `script`, `mcp` mit allen Optionen und
  JSON-Ausgabeschlüsseln; MCP: alle 22 Werkzeugnamen, Parameter, JSON-Felder, Ressourcen-URIs.
- Befehlsnamen und -parameter des Registers (`file.export`, `outDir`, `backup` …).
- Umgebungsvariablen der Laufzeit: `FP_NO_SHELL_OPEN`, `FP_COMMANDS_DIR`, `FP_EXTENSIONS_DIR`,
  `FP_SHORTCUTS_DIR`, `FP_HELP`, `FP_DICT_DIR`, `FP_FONT_DIRS`, `FP_NO_DOWNLOAD`, `FP_NO_LAUNCH`,
  `FIREPIXEL_LANG`, `FIREPIXEL_MCP`, `FIREPIXEL_MCP_LANG`, `FP_CORPUS_DIR` …
- Keine Protokoll- oder Absturzdateien im eigenen Code (nur `log`-Ausgabe).

## C. Bleibt bewusst deutsch bzw. unverändert, weil …

| Name | Fundstelle | Grund |
|---|---|---|
| Skriptnamen der eingebauten Befehle (`Text\Großbuchstaben.jsf`, `Dokument\…`) | `crates\fp-script\src\builtin_commands.rs` | Datei- und Ordnernamen im `Commands`-Ordner erscheinen im Menü „Befehle“ (wie im Original) und folgen der UI-Sprache beim ersten Start – Oberflächentext. Erlaubnisliste des Wächters. |
| `Commands`, `Auto Shapes`, `Auto Shape Tools`, `Settings`, `Templates`, `Language`, `Original Files`, `Configuration\{Textures,Patterns,Libraries}` | `fp-script\src\imp\fw.rs`, `imp\files.rs`, `fp-engine\src\commands\export.rs`, `fp-ui\src\inspector\*` | englisch und bilden bewusst die Ordnerstruktur des Originals nach |
| ~~`ANLEITUNG.md`, `ANLEITUNG.<sprache>.md`~~ | `crates\fp-engine\src\commands\misc.rs` `help_names` | **erledigt in TE-2** (→ `MANUAL.md`, `MANUAL.<sprache>.md`; alte Namen nur noch als Rückfall, siehe E). |
| ~~Rust-Bezeichner `Preset::Schwarzweiss`, `Preset::Foto`~~ | `crates\fp-trace\src\lib.rs` | **erledigt in TE-2** (→ `Preset::BlackWhite`, `Preset::Photo`, siehe E). |
| Felder `label_de`, `menu_label_de`, `path_de`, `status_de`, `shortcuts_de` | `crates\fp-engine\src\catalog.rs`, `data\commands.json` | englische Schlüssel mit Sprachcode `de` (Texte des deutschen Originals) |
| Eingabe-Aliasse `links`/`rechts`/`mitte` (Maustasten), `nach-links` … (Tastennamen), `hell`/`dunkel`, `ja` | `fp-ui\src\bridge\inject.rs`, `fp-i18n\src\format.rs`, `fp-ui\src\bridge.rs`, `fp-engine\src\prefs.rs` | nur zusätzlich angenommene Eingaben; ausgegeben und dokumentiert werden die englischen Namen |
| JSF-Namen (`fw`, `dom`, `appJsCommandsDir` …) | `crates\fp-script` | vom Original-API vorgegeben |
| deutsche Beschreibungen in `crates\fp-engine\data\prefs.json` (`"effect"`), Befehlsbeschreibungen | `fp-engine` | Texte, keine Schlüssel |

## D. Für TE-2 (Liste von TE-1) – erledigt

**Erledigt in TE-2 (13.10.2026), Ergebnis in Abschnitt E.** Ausnahme laut Nutzerentscheidung: Testdateien, Testdaten-Dateien und Testfunktionsnamen mit deutschen Namen **bleiben** (sie erscheinen nirgends nach außen). Die Liste unten bleibt als Bestandsaufnahme stehen.

**`.ftl`-Dateinamen** (je `locales\de` und `locales\en`, 16 Bereiche; Vorschlag) – ☑ erledigt (TE2-A, E1–E14):
`allgemein` → `common`, `bedienfelder` → `panels`, `befehlstexte` → `command_texts`,
`dateidialog` → `file_dialog`, `dialoge` → `dialogs`, `filter` (bleibt), `inspektor` → `inspector`,
`kommandozeile` → `cli`, `mcp` (bleibt), `meldungen` → `messages`, `menue` → `menu`,
`nachzeichnen` → `trace`, `oberflaeche` → `ui`, `skript` → `script`, `werkzeuge` → `tools`,
`willkommen` → `welcome`. Dabei beachten: `build.rs` von fp-i18n, die Bereichstabelle in
`crates\fp-i18n\README.md`, `AGENTEN_REGELN` §7 (`menue.ftl`, `nachzeichnen.ftl`), Prüfungen mit
Dateinamen (`fp_i18n::check`, `language template` kopiert die Dateinamen in neue Sprachpakete), und
dass vorhandene Sprachpakete der Nutzer unter `Languages\<code>\` die alten Dateinamen tragen
(Laden ist namensunabhängig; Prüfung/Bereichszuordnung ggf. mit Rückfall).

**Quelltext-Dateien mit deutschen Namen** – ☑ erledigt (TE2-B/C/D, E15–E17):
`apps\firepixel-cli\src\sprache.rs` (→ `language.rs`; Funktionen `pruefen`/`vorlage`/`liste`),
`crates\fp-ui\src\inspector\texte.rs`, `crates\fp-ai\examples\ki_bilder.rs`.

**Testdateien mit deutschen Namen** – bleiben (Nutzerentscheidung 13.10.; Verweise in Kommentaren auf die neuen `.ftl`-/Quelldateinamen umgestellt, TE2-G): `apps\firepixel-cli\tests\sprache.rs`,
`crates\fp-ai\tests\referenz.rs` (+ `testdata\referenz.json`), `crates\fp-engine\tests\{eb1_paritaet,
paritaet_menue, stichproben, text_vermessung, warnungen}.rs` (+ `tests\data\paritaet_menue.tsv`,
`stichproben.jsf`), `crates\fp-i18n\tests\leistung.rs`, `crates\fp-mcp\tests\sprache.rs`,
`crates\fp-script\tests\{befehle, befehlsskripte, fwsteuerung}.rs`, `crates\fp-ui\tests\{bitmapwerkzeuge_fv11,
design_bilder, design_willkommen, dialogs_warnung, gesamt_alltag, gesamt_dateien, gesamt_groessen,
gesamt_kuerzel, gesamt_logik, gesamt_menu, gesamt_oberflaeche, gesamt_sprachen, gesamt_werkzeuge,
i18n_keine_festen_texte, i18n_menue, i18n_qa_bericht, i18n_qa_glossar, i18n_sprachwahl,
inspector_paritaet, tippen_messung, ue3_befunde, ux_a_abgleich, ux_b_abgleich, ux_b_paritaet}.rs`,
`crates\fp-ui\tests\i18n_ausnahmen.txt`, `crates\fp-ui\tests\gesamt_common\`. Dazu viele deutsche
Testfunktionsnamen (z. B. `vorlage_pruefen_liste`, `projekt_ist_neutral`) in fast allen Crates.

**Test-Umgebungsvariablen** (nur in `tests\`) – ☑ erledigt (TE2-E, E20–E25): `FP_QA_NUR`, `FP_QA_AUS`, `FP_QA_NEU`, `FP_QA_MODUS`
(Werte `basis`/`aktuell`), `FP_DESIGN_BILD` (Werte `vorher`/`nachher`), `FP_SPRACHEN_DEBUG`.

**Rust-Bezeichner** mit deutschen Namen (z. B. `Preset::Schwarzweiss`, `Theme::label_de`) – ☑ erledigt
(TE2-B…E, E26/E27 und Nachtrag in E); `label_de` u. ä. bleiben (Abschnitt C).

## E. Teil 2 (TE-2, 13.10.2026)

Umgesetzt von TE2-A … TE2-F (Berichte `docs\berichte\te2a.md` … `te2f.md`). Regel wie oben: alte
Namen, die Nutzer oder Skripte noch haben können, werden weiter gelesen (Rückfall), ausgegeben und
dokumentiert werden nur die neuen.

| # | Art | alt | neu | Fundstelle | Rückfall |
|---|---|---|---|---|---|
| 1 | `.ftl`-Datei | `allgemein.ftl` | `common.ftl` | `crates\fp-i18n\locales\{de,en}\` | Laden ist namensunabhängig; Sprachpakete der Nutzer unter `Languages\<code>\` mit alten Dateinamen werden weiter geladen und geprüft (TE2-A) |
| 2 | `.ftl`-Datei | `bedienfelder.ftl` | `panels.ftl` | ebd. | wie 1 |
| 3 | `.ftl`-Datei | `befehlstexte.ftl` | `command_texts.ftl` | ebd. | wie 1 |
| 4 | `.ftl`-Datei | `dateidialog.ftl` | `file_dialog.ftl` | ebd. | wie 1 |
| 5 | `.ftl`-Datei | `dialoge.ftl` | `dialogs.ftl` | ebd. | wie 1 |
| 6 | `.ftl`-Datei | `inspektor.ftl` | `inspector.ftl` | ebd. | wie 1 |
| 7 | `.ftl`-Datei | `kommandozeile.ftl` | `cli.ftl` | ebd. | wie 1 |
| 8 | `.ftl`-Datei | `meldungen.ftl` | `messages.ftl` | ebd. | wie 1 |
| 9 | `.ftl`-Datei | `menue.ftl` | `menu.ftl` | ebd. | wie 1 |
| 10 | `.ftl`-Datei | `nachzeichnen.ftl` | `trace.ftl` | ebd. | wie 1 |
| 11 | `.ftl`-Datei | `oberflaeche.ftl` | `ui.ftl` | ebd. | wie 1 |
| 12 | `.ftl`-Datei | `skript.ftl` | `script.ftl` | ebd. | wie 1 |
| 13 | `.ftl`-Datei | `werkzeuge.ftl` | `tools.ftl` | ebd. | wie 1 |
| 14 | `.ftl`-Datei | `willkommen.ftl` | `welcome.ftl` | ebd. | wie 1 |
| – | `.ftl`-Datei | `filter.ftl`, `mcp.ftl` | bleiben | ebd. | – |
| 15 | Quelldatei | `apps\firepixel-cli\src\sprache.rs` | `language.rs` (Funktionen `pruefen`/`vorlage`/`liste` → `check`/`template`/`list`, Parameter englisch; das Modul `fp_i18n::check` heißt dort `i18n_check` wegen der Funktion `check`) | `apps\firepixel-cli` (TE2-C) | – (Quelltext; CLI-Befehle und versteckte Aliasse unverändert) |
| 16 | Quelldatei | `crates\fp-ui\src\inspector\texte.rs` (Modul `inspector::texte`) | `texts.rs` (`inspector::texts`) | `crates\fp-ui` (TE2-B) | – |
| 17 | Beispiel | `crates\fp-ai\examples\ki_bilder.rs` (`cargo run --example ki_bilder`) | `ai_images.rs` (`--example ai_images`) | `crates\fp-ai` (TE2-D) | – |
| 18 | Handbuch | `docs\ANLEITUNG.md` | `docs\MANUAL.md` | `docs\` (TE2-F) | Hilfe ▸ Hilfe sucht zuerst `MANUAL*`, dann `ANLEITUNG*` (alte Testversionen, eigene Kopien neben der exe) – `crates\fp-engine\src\commands\misc.rs` `help_names` (TE2-E) |
| 19 | Handbuch | `docs\ANLEITUNG.en.md` (bzw. `ANLEITUNG.<sprache>.md`) | `docs\MANUAL.en.md` (`MANUAL.<sprache>.md`) | ebd. | wie 18 |
| 20 | Test-Umgebungsvariable | `FP_QA_NUR` | `FP_QA_ONLY` | `crates\fp-ui\tests\i18n_qa_common\mod.rs` (`only_filter`), `design_bilder.rs`, `dialogs_des1.rs` (TE2-E) | alter Name weiter gelesen (Helfer `i18n_qa_common::env_var(neu, alt)`) |
| 21 | Test-Umgebungsvariable | `FP_QA_AUS` (Ausgabeordner) | `FP_QA_OUT` (nicht `OFF`: „AUS“ stand für *Ausgabe*; passt zu `FP_UXB_OUT`, `FP_RENDER_OUT`) | `i18n_qa_common::out_root` | wie 20 |
| 22 | Test-Umgebungsvariable | `FP_QA_NEU` | `FP_QA_NEW` | `i18n_qa_common::discover_dialogs` | wie 20 |
| 23 | Test-Umgebungsvariable | `FP_QA_MODUS` mit `aktuell`/`basis` | `FP_QA_MODE` mit `current`/`baseline`; Ergebnisordner `…\i18n-qa\current\` bzw. `baseline\` (vorher `aktuell\`/`basis\`) | `i18n_qa_common::mode`, `mode_dir`; `i18n_qa_de`, `i18n_qa_bericht`, `i18n_qa_glossar` | alter Name und alte Werte weiter gelesen; `mode_dir()` liest die alten Ordner, solange es die neuen nicht gibt (vorhandene Basis bleibt nutzbar) |
| 24 | Test-Umgebungsvariable | `FP_DESIGN_BILD` mit `vorher`/`nachher` | `FP_DESIGN_IMAGE` mit `before`/`after`; neue Bilder `docs\screenshots\design_before_*.png`/`design_after_*.png` | `crates\fp-ui\tests\design_bilder.rs` | alter Name und alte Werte weiter gelesen (Bilder dann ebenfalls `design_before_*`/`design_after_*`); die 107 vorhandenen Bilder `design_vorher_*`/`design_nachher_*` behalten ihre Namen, `design_willkommen.rs` schreibt noch `design_nachher_*` (Befund) |
| 25 | Test-Umgebungsvariable | `FP_SPRACHEN_DEBUG` | `FP_LANGUAGES_DEBUG` | `crates\fp-ui\tests\gesamt_sprachen.rs` | wie 20 |
| 26 | Rust-Bezeichner | `fp_trace::Preset::Schwarzweiss` | `Preset::BlackWhite` | `crates\fp-trace\src\lib.rs` und Aufrufer (TE2-D) | serde-Namen unverändert `blackWhite`/`photo`, Aliasse `schwarzweiss`/`foto` bleiben |
| 27 | Rust-Bezeichner | `fp_trace::Preset::Foto` | `Preset::Photo` | ebd. | wie 26 |

Weitere Bezeichner, die TE2-B … TE2-E nebenbei umbenannt haben, stehen in deren Berichten
(`docs\berichte\te2a.md` … `te2e.md`); Nachtrag siehe unten.

**Nachtrag (Abgleich mit den Berichten, TE2-G):**

- Rückfall für Sprachpakete (TE2-A): `fp_i18n::check::LEGACY_AREA_NAMES` (alt → neu),
  `area_file_name` (`menue.ftl` → `menu.ftl`, Groß-/Kleinschreibung egal), `legacy_area_file_name`;
  `check::load_path` erkennt einen Sprachordner auch nur an alten Namen. `firepixel-cli language template`
  schreibt die neuen Namen, ein Bereich unter altem Namen zählt als vorhanden; `--overwrite` schreibt den
  neuen Namen und entfernt die alte Datei. Tests `crates\fp-i18n\tests\legacy_area_names.rs`,
  `apps\firepixel-cli\tests\language_english.rs` `template_and_check_with_old_area_names`.
- Unverändert geblieben (TE2-A): `filter.ftl`, `mcp.ftl`, `bitmap_selection.ftl`, `image_preview.ftl`.
- Handbuch (TE2-E): Konstanten `MANUAL_STEM`, `LEGACY_MANUAL_STEM`; Reihenfolge Englisch
  `MANUAL.en.md`, `ANLEITUNG.en.md`, `MANUAL.md`, `ANLEITUNG.md` (Sprachfassung vor Name). `msg-help-intro`
  in `messages.ftl` nennt `MANUAL.md` bzw. `MANUAL.en.md`. Erlaubnisliste des Wächters ohne
  `ANLEITUNG` und ohne `Preset::Schwarzweiss/Foto` (TE2-D/E).
- Unit-Testnamen im Quelltext englisch: `fp-ui` `file_dialog.rs` (`test_mode_path_answers_by_kind`,
  `multiple_paths_only_for_multi_select`), `bridge\inject.rs` (`ctrl_c_becomes_copy_like_egui_winit`,
  `click_needs_pointer_position_and_carries_modifiers`, `files_and_text`, `structure_and_errors`,
  `main_window_driven_by_input`) (TE2-B); `fp-engine` `misc.rs` (`manual_per_language`,
  `manual_legacy_names`, `help_command_returns_english_manual`) (TE2-E).
- Lokale Variablen `unten`/`oben` → `lower`/`upper` in `fp-raster\src\filters\curve_edit.rs` (TE2-D).
- `.ftl`-Namen in Kommentaren aller Crates und Tests auf die neuen Namen (TE2-A…E; Reste in
  Testkommentaren, `fp-trace\Cargo.toml`, `i18n_ausnahmen.txt` von TE2-G).
- **Noch deutsch (Befunde in `docs\BEFUNDE.md`, Abschnitt TE):** QA-Bereichs-IDs und -Ordner der
  `i18n_qa_*`-Tests (`menues`, `bedienfelder`, `inspektor`, `dialoge`, `kontextmenues`, `hauptfenster`,
  `basislayout`, Cachedatei `dialoge.json`, Befundarten, Bildpräfix `phase6_basis_de_`) und die 107
  Bilder `design_vorher_*`/`design_nachher_*`.

**Bleibt (Nutzerentscheidung 13.10.2026):** Testdateien (`tests\*.rs` mit deutschen Namen),
Testdaten-Dateien (`*.tsv`, `*.json`, `*.jsf` unter `tests\`) und Testfunktionsnamen. Ebenso
`fp-trace`-Werte und Eingabe-Aliasse aus Abschnitt C.

**Doku angepasst (TE2-F):** `docs\ANLEITUNG*.md` per `git mv` zu `docs\MANUAL*.md` (gegenseitige
Verweise), `docs\AGENTEN_REGELN.md` (§1 Zuständigkeit, §1b `common.ftl`, §7 `menu.ftl`/`trace.ftl`),
`crates\fp-i18n\GLOSSAR.md` (`common.ftl`). `docs\AGENTS.md`, `AGENTS.en.md`, `MODELLE.md` und die
MCP-Ressourcen `crates\fp-mcp\docs\{de,en}` enthielten keine der alten Namen; ein Root-`README.md`
und weitere `crates\*\README.md` außer `fp-i18n` gibt es nicht; Skripte für Testversion/Release
im Repo verweisen nicht auf die Handbuchnamen. Historische Berichte (`docs\berichte\*.md`),
Fortschritts-/Testversionsnotizen und automatisch erzeugte Prüfberichte (`docs\I18N_PRUEFUNG.md`,
`docs\funktionstest\*_auto.md`) zeigen die alten Namen, bis sie neu erzeugt bzw. abgelöst werden.
