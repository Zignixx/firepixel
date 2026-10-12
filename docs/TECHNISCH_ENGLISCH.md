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
| 17 | CLI-Befehl | `firepixel-cli sprache` | `language` | `apps\firepixel-cli\src\main.rs`, `src\sprache.rs` | alter Name als versteckter Alias |
| 18 | CLI-Befehl | `sprache pruefen` | `language check` | ebd. | versteckter Alias |
| 19 | CLI-Befehl | `sprache vorlage` | `language template` | ebd. | versteckter Alias |
| 20 | CLI-Befehl | `sprache liste` | `language list` | ebd. | versteckter Alias |
| 21 | CLI-Option | `--eingebaut` | `--builtin` | ebd. | versteckter Alias |
| 22 | CLI-Option | `--kurz` | `--short` | ebd. | versteckter Alias |
| 23 | CLI-Option | `--streng` | `--strict` | ebd. | versteckter Alias |
| 24 | CLI-Option | `--ueberschreiben` | `--overwrite` | ebd. | versteckter Alias |
| 25 | CLI-Option | `--von` | `--from` | ebd. | versteckter Alias |
| 26 | Fluent-IDs | `cli-sprache`, `cli-sprache-{pruefen,vorlage,liste}`, `…-arg-{pfad,eingebaut,kurz,streng,zielordner,ueberschreiben,von}` (12) | `cli-language`, `cli-language-{check,template,list}`, `…-arg-{path,builtin,short,strict,target-dir,overwrite,from}` | `crates\fp-i18n\locales\{de,en}\kommandozeile.ftl` (aus den Argument-IDs abgeleitet) | – |
| 27 | GUI-Option | `firepixel --design=dunkel` | `--design=dark` | `apps\firepixel\src\main.rs`, `oberflaeche.ftl` (`ui-cli-help`) | `dunkel` weiter gültig |
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
| `ANLEITUNG.md`, `ANLEITUNG.<sprache>.md` (Hilfe ▸ Hilfe sucht sie neben der exe bzw. in `docs\`) | `crates\fp-engine\src\commands\misc.rs` `help_names` | Dateinamen der Handbücher im Repo; Umbenennung (z. B. `MANUAL.md`) nur zusammen mit den Dokumenten und mit den alten Namen als Rückfall – TE-2. Erlaubnisliste. |
| Rust-Bezeichner `Preset::Schwarzweiss`, `Preset::Foto` | `crates\fp-trace\src\lib.rs` | serde-Namen sind englisch; Bezeichner sind Quelltextnamen (TE-2). Erlaubnisliste. |
| Felder `label_de`, `menu_label_de`, `path_de`, `status_de`, `shortcuts_de` | `crates\fp-engine\src\catalog.rs`, `data\commands.json` | englische Schlüssel mit Sprachcode `de` (Texte des deutschen Originals) |
| Eingabe-Aliasse `links`/`rechts`/`mitte` (Maustasten), `nach-links` … (Tastennamen), `hell`/`dunkel`, `ja` | `fp-ui\src\bridge\inject.rs`, `fp-i18n\src\format.rs`, `fp-ui\src\bridge.rs`, `fp-engine\src\prefs.rs` | nur zusätzlich angenommene Eingaben; ausgegeben und dokumentiert werden die englischen Namen |
| JSF-Namen (`fw`, `dom`, `appJsCommandsDir` …) | `crates\fp-script` | vom Original-API vorgegeben |
| deutsche Beschreibungen in `crates\fp-engine\data\prefs.json` (`"effect"`), Befehlsbeschreibungen | `fp-engine` | Texte, keine Schlüssel |

## D. Für TE-2 (nur aufgelistet, nicht umbenannt)

**`.ftl`-Dateinamen** (je `locales\de` und `locales\en`, 16 Bereiche; Vorschlag):
`allgemein` → `common`, `bedienfelder` → `panels`, `befehlstexte` → `command_texts`,
`dateidialog` → `file_dialog`, `dialoge` → `dialogs`, `filter` (bleibt), `inspektor` → `inspector`,
`kommandozeile` → `cli`, `mcp` (bleibt), `meldungen` → `messages`, `menue` → `menu`,
`nachzeichnen` → `trace`, `oberflaeche` → `ui`, `skript` → `script`, `werkzeuge` → `tools`,
`willkommen` → `welcome`. Dabei beachten: `build.rs` von fp-i18n, die Bereichstabelle in
`crates\fp-i18n\README.md`, `AGENTEN_REGELN` §7 (`menue.ftl`, `nachzeichnen.ftl`), Prüfungen mit
Dateinamen (`fp_i18n::check`, `language template` kopiert die Dateinamen in neue Sprachpakete), und
dass vorhandene Sprachpakete der Nutzer unter `Languages\<code>\` die alten Dateinamen tragen
(Laden ist namensunabhängig; Prüfung/Bereichszuordnung ggf. mit Rückfall).

**Quelltext-Dateien mit deutschen Namen:**
`apps\firepixel-cli\src\sprache.rs` (→ `language.rs`; Funktionen `pruefen`/`vorlage`/`liste`),
`crates\fp-ui\src\inspector\texte.rs`, `crates\fp-ai\examples\ki_bilder.rs`.

**Testdateien mit deutschen Namen:** `apps\firepixel-cli\tests\sprache.rs`,
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

**Test-Umgebungsvariablen** (nur in `tests\`): `FP_QA_NUR`, `FP_QA_AUS`, `FP_QA_NEU`, `FP_QA_MODUS`
(Werte `basis`/`aktuell`), `FP_DESIGN_BILD` (Werte `vorher`/`nachher`), `FP_SPRACHEN_DEBUG`.

**Rust-Bezeichner** mit deutschen Namen (z. B. `Preset::Schwarzweiss`, `Theme::label_de`).
