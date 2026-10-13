# TE2-A – `.ftl`-Bereichsdateien englisch benannt (13.10.2026)

## Umgesetzt
Commit `TE2-A: .ftl-Bereichsdateien englisch …` (siehe `git log --grep TE2-A`).

Umbenannt per `git mv` in `crates\fp-i18n\locales\de` und `\en` (je 14 Dateien, Inhalt unverändert
außer Dateiverweisen):

| alt | neu |
|---|---|
| `allgemein.ftl` | `common.ftl` |
| `bedienfelder.ftl` | `panels.ftl` |
| `befehlstexte.ftl` | `command_texts.ftl` |
| `dateidialog.ftl` | `file_dialog.ftl` |
| `dialoge.ftl` | `dialogs.ftl` |
| `inspektor.ftl` | `inspector.ftl` |
| `kommandozeile.ftl` | `cli.ftl` |
| `meldungen.ftl` | `messages.ftl` |
| `menue.ftl` | `menu.ftl` |
| `nachzeichnen.ftl` | `trace.ftl` |
| `oberflaeche.ftl` | `ui.ftl` |
| `skript.ftl` | `script.ftl` |
| `werkzeuge.ftl` | `tools.ftl` |
| `willkommen.ftl` | `welcome.ftl` |

Unverändert: `filter.ftl`, `mcp.ftl`, `bitmap_selection.ftl`, `image_preview.ftl`.

- `build.rs` war schon namensunabhängig (bettet jede `*.ftl` ein), keine Änderung nötig.
- Sichtbare Texte mit Dateinamen angepasst: `cli.ftl` de/en (Beispiel `language check Languages\fr\menu.ftl`,
  „language-name in common.ftl setzen“); Kommentarköpfe in `dialogs.ftl`, `image_preview.ftl`, `trace.ftl`.
- Tests mit Dateinamen: `crates\fp-i18n\tests\paths_english.rs`, `apps\firepixel-cli\tests\{language_english,sprache}.rs`,
  `crates\fp-ui\tests\{i18n_sprachwahl,i18n_qa_glossar}.rs`. Weitere funktionale Verweise gab es nicht
  (`fp-engine\tests\registry.rs` und `gen_data.py` meinen `re\werkzeuge\menuebefehle.json` außerhalb des Repos,
  fp-trace-Tests nur `expect("nachzeichnen")`).
- `crates\fp-i18n\README.md` (Bereichstabelle, Beispiele, Absatz zum Rückfall, fehlende Zeile `bitmap_selection.ftl`)
  und `GLOSSAR.md` auf die neuen Namen.

## Rückfall für Sprachpakete der Nutzer
- Laden (`load_external_dir`) liest jede `*.ftl` – alte Namen laden wie bisher.
- `fp_i18n::check::LEGACY_AREA_NAMES` (alt → neu, ohne `.ftl`), `area_file_name` (`menue.ftl` → `menu.ftl`,
  Groß-/Kleinschreibung egal), `legacy_area_file_name` (umgekehrt). `check::load_path` erkennt einen
  Sprachordner auch nur an alten Namen und prüft eine einzelne alte Bereichsdatei gegen ihren neuen Bereich.
- `firepixel-cli language template` schreibt die neuen Namen; liegt der Bereich schon unter altem Namen vor,
  zählt er als vorhanden (übersprungen), `--overwrite` schreibt den neuen Namen und entfernt die alte Datei
  (sonst doppelte Schlüssel).
- Tests: `crates\fp-i18n\tests\legacy_area_names.rs` (3), `apps\firepixel-cli\tests\language_english.rs`
  `template_and_check_with_old_area_names`.

## Geprüft
`cargo test -p fp-i18n` grün, `cargo check --workspace --all-targets` grün, `cargo test -p fp-neutral` grün
(auch `technical_english`), `firepixel-cli --test language_english --test sprache` grün,
`fp-ui --test all` Module `i18n_sprachwahl::`, `i18n_keine_festen_texte::` (Scanner), `i18n_qa_glossar::` (mit `--ignored`) grün.

## Nachprüfung
- Ein vorhandenes Sprachpaket mit alten Dateinamen unter `%APPDATA%\Firepixel\Languages\<code>\` in der
  fertigen exe: Sprache erscheint in Ansicht ▸ Sprache, `firepixel-cli language check <ordner>` meldet
  denselben Anteil wie vorher.

## Neue Befunde
- `crates\fp-ui\tests\i18n_qa_bericht.rs`/`i18n_qa_common\mod.rs`: Bereichs-IDs der QA-Läufe
  (`bedienfelder`, `inspektor`, `dialoge` → Ausgabeordner/Cachedatei `dialoge.json`) sind deutsche technische
  Namen, aber keine `.ftl`-Verweise – nicht umbenannt (Testinfrastruktur, eigene Entscheidung nötig).
- `docs\I18N_PRUEFUNG.md` (erzeugt von `i18n_qa_glossar`) nennt beim nächsten Lauf `menu.ftl`; nicht mitcommittet (Doku TE2-F).
