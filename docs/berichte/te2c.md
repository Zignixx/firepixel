# TE2-C – Technisches Englisch Teil 2, Teil C (13.10.2026)

Bereich: `apps\firepixel-cli\src`, `apps\firepixel\src`, `crates\fp-mcp\src`, `crates\fp-script\src`
(Regel: `docs\AGENTEN_REGELN.md` §1c, Liste: `docs\TECHNISCH_ENGLISCH.md` D).

## Umgesetzt

1. **Quelldatei umbenannt:** `apps\firepixel-cli\src\sprache.rs` → `language.rs` (`git mv`, `mod language;`
   in `main.rs`). Funktionen `pruefen`/`vorlage`/`liste` → `check`/`template`/`list`, deutsche Parameter
   (`pfad`, `eingebaut`, `kurz`, `streng`, `ziel`, `ueberschreiben`, `von`) → `path`, `builtin`, `short`,
   `strict`, `target_dir`, `overwrite`, `from`. Das Modul `fp_i18n::check` heißt dort jetzt `i18n_check`
   (Namenskonflikt mit der Funktion `check`). CLI-Befehle, Optionen und versteckte deutsche Aliasse
   unverändert. Weitere deutsch benannte Quelldateien gibt es im Bereich nicht.
2. **Kommentare mit alten .ftl-Namen:** `kommandozeile.ftl` → `cli.ftl` (firepixel-cli `main.rs`,
   `language.rs`), `befehlstexte.ftl` → `command_texts.ftl` (firepixel-cli `main.rs`), `oberflaeche.ftl`
   → `ui.ftl` (`apps\firepixel\src\main.rs`), `bedienfelder.ftl` → `panels.ftl`
   (`fp-script\src\autoshape\shapes.rs`), `skript.ftl` → `script.ftl` (`fp-script\src\bindings.rs`).
   In `fp-mcp\src` kamen nur `mcp.ftl`-Verweise vor (bleibt). Funktionale Dateinamen-Strings: keine im Bereich.
3. **Deutsche Rust-Bezeichner:** alle Typen, Module, Konstanten, Funktionen und Felder des Bereichs
   per Skript durchgesehen; außer den unter 1 genannten keine deutschen Namen in Laufzeitcode. Übrig
   sind nur Testfunktionen (bleiben laut Auftrag): `hilfe_vollstaendig_in_de_und_en`,
   `parameterbeschreibungen_in_de_und_en` (fp-mcp). JSF-API-Namen und Skriptnamen der eingebauten
   Befehle unverändert (TECHNISCH_ENGLISCH C).
4. **Mit angepasst (Aufrufer/Verweise):** `crates\fp-ui\tests\i18n_ausnahmen.txt` (Ausnahme
   `text sprache.rs | language-name = {code}\n` → `text language.rs | …`, sonst meldet der Scanner
   `i18n_keine_festen_texte` die Zeile), Kommentar in `apps\firepixel-cli\Cargo.toml`.

Baut auf TE2-A (86be8d0, Rückfall legacy_area_file_name in template) auf. Commit: siehe `git log --grep "TE2-C"`.

## Prüfung

`cargo check --workspace --all-targets` grün. `cargo test -p fp-neutral -p firepixel-cli -p fp-mcp`:
alles grün (fp-neutral inkl. `technical_english`, firepixel-cli `cli`/`language_english`/`sprache`, fp-mcp
alle) bis auf einen **schon vorher roten** Unit-Test, siehe Neue Befunde. Der i18n-Scanner (fp-ui) wurde
nicht gebaut (breiter fp-ui-Lauf, §7a) – Nachprüfung.

## PARITAET / Befunde

- PARITAET: keine Zeilen.
- `docs\TECHNISCH_ENGLISCH.md` D: Eintrag `apps\firepixel-cli\src\sprache.rs` erledigt; A17 „Fundstelle
  `src\sprache.rs`“ heißt jetzt `src\language.rs`.

## Nachprüfung

- `firepixel-cli language check --builtin`, `language template xx <ordner>`, `language list` und die
  alten Aliasse `sprache pruefen|vorlage|liste` verhalten sich wie vorher (reine Umbenennung).
- `cargo test -p fp-ui --test all i18n_keine_festen_texte::` grün (Ausnahme auf `language.rs` umgestellt).

## Neue Befunde

- `firepixel-cli` Unit-Test `tests::parameterbeschreibungen_in_de_und_en` (`apps\firepixel-cli\src\main.rs`)
  rot, unabhängig von TE2-C: 19 Parameterbeschreibungen/Hinweise ohne Schlüssel in `command_texts.ftl`
  (de/en), u. a. `file.open` („als „Unbenannt“ öffnen …“, „weitere Dateien [Pfad, …] …“),
  `edit.copyHtmlCode` („Ordner für die Bilder …“), `element.setPattern`, `element.setTexture` – Texte
  stammen aus fp-engine (u. a. NZ-4 `08187d5`). Zuständig: Besitzer von `command_texts.ftl`.
