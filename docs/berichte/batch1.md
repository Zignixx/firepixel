# BATCH-1 – Kern `fp-batch` (moderner Batch-Modus)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` (SPEC-BATCH), Paket BATCH-1 aus §6.1.

## Umgesetzt (Commits)

1. **API-Stand** `BATCH-1: Kern fp-batch (API-Stand) …` – Crate, Befehle `batch.*`, CLI, Texte.
2. **Tests + Bericht** `BATCH-1: Tests (§4b-Logikfälle), Rezept-Bearbeitung, $1-Gruppenverweise, Bericht` – siehe `git log --grep BATCH-1`.

| Datei | Inhalt |
|---|---|
| `crates\fp-batch\src\recipe.rs` | Rezept-Modell (serde, englische Schlüssel, `format`/`version`, `#[serde(flatten)] extra` überall), Migration v0 → v1, neuere Version → Fehler, `validate()`, Bearbeiten (`add_step`, `remove_step`, `move_step`, `set_param` idempotent) |
| `src\step.rs` | Trait `Step`, `StepRegistry` (+ `global()`), `ParamDef`/`ParamType` (Formular-Schema, `beginner` = **A**), `Issue`/`Severity`, `StepContext` (Abbrechen, Warnungen, Zielformat), `catalog_json()`, Textschlüssel `step_key`/`param_key`, `parse_color` |
| `src\item.rs` | `ItemInfo` (Kopf lesen ohne Dekodieren: PNG/APNG/GIF/JPEG/BMP/WebP/QOI), `collect()` (Dateien + Ordner rekursiv, natürliche Sortierung, Doppelte, Unbekannte), `Item` = `Still` / `Animation` (Vollbild-Frames, ms) / `Document` / `FileOnly`, `load`/`decode_bytes`, `map_images`, `flatten_document`, EXIF-Haken `set_exif_probe` |
| `src\pipeline.rs` | `Pipeline` (aufgelöste Schritte, unbekannte übersprungen, „nur Dateischritte“ §5.3, Sammel-Schritt-Position), `preview()` mit Proxy (1024 / Animation 480 px, 150 Frames), Generationen-Abbruch, optional volle Kodierung für die Dateigröße |
| `src\naming.rs` | §4.4 vollständig: Baukasten (original/text/counter/date/size/folder/exif/format), Ersetzungen, Leerzeichen, Präfix/Suffix, Umlaute, Sonderzeichen, Schreibweise (8 Arten), Regex mit Platzhaltern `{n:3}` `{date:…}` `{exif-date:…}` `{w}` `{h}` `{folder}` `{ext}` `{name}`, ungültige Zeichen/reservierte Namen/Kürzen/leer/führender Punkt, Endung (Schreibweise, `jpeg`→`jpg`, `.fw.png`); `plan()`: Ausgabeordner (`batch-output`, `-2`…, Struktur), Konflikte deterministisch in Listenreihenfolge (rename/skip/overwrite/ask), Original ersetzen nur mit Sicherung, Pfade > 260 |
| `src\executor.rs` | `run()` (rayon-Pool Kerne − 1, Byte-Semaphor, Element größer als Budget läuft allein), `CancelToken` (+ Generation), `Progress`-Kanal, Rückfrage `ask` (seriell, „für alle“), Fehler je Element ohne Abbruch, Sammel-Schritte in Phase 2, `spawn()` → `JobHandle` für die Oberfläche, `plan()` |
| `src\output.rs` | `OutputFormat` (+ `same`-Auflösung mit PNG-Rückfall), Kodieren über `fp-export::doc_export::export_whole` (PNG 8/24/32, JPEG mit Hintergrund, WebP/verlustfrei/animiert, AVIF, GIF/animiert, APNG, TIFF, BMP, QOI, ICO, WBMP), FW-PNG über `fp_engine::commands::save_document`, `register_encoder()` für weitere Formate, Temp `.<name>.fptmp` + atomares Umbenennen, `Original Files` (inkrementell `name-1.ext`), `\\?\`-Pfade, Kopieren/Verschieben, Zeitstempel |
| `src\report.rs` | `Report`/`ItemReport`/`Totals` (`batch-report.json`, Schlüssel englisch), `summary_text()`, `exit_code()`, `batch-log.txt` in `local_dir()\Logs` |
| `src\host.rs` | `install_host(&mut Session)` / `install()`; Befehle `batch.run`, `batch.plan`, `batch.preview` (PNG Base64), `batch.steps`, `batch.validate` (Sprache je Aufruf `language`) |
| `src\steps\mod.rs`, `steps\files.rs` | Registrierung; Schritte `file.copyOnly` (copy/move), Endkarten `rename`, `output` (Schema für Katalog/Formular) |
| `crates\fp-engine\src\commands\batch.rs` | Haken `trait BatchHost` + `set_batch_host`/`batch_host` (prozessweit, kein Feld in `Session`), Befehle `batch.*` |
| `apps\firepixel-cli\src\batch.rs` | `batch --recipe … (--in [--recursive] | Dateien) [--out] [--conflict] [--dry-run] [--threads] [--report] [--json]`, `batch --steps [--json]`; Exit 0/1/2/3; Fortschritt auf stderr (NDJSON mit `--json`); `batch --actions` unverändert |
| `crates\fp-i18n\locales\{de,en}\batch.ftl` | alle Texte (Gruppen, Schritte, Fehler, Hinweise, Befehle `cmd-batch-*`, `cmdtext-*`, CLI-Hilfe `batch-cli-*`) |

Tests (alle grün): `crates\fp-batch\tests\recipe_naming.rs` (18), `tests\run.rs` (17),
`apps\firepixel-cli\tests\batch_recipe.rs` (4); `cargo test -p fp-i18n`, `fp-neutral`, Scanner
`i18n_keine_festen_texte::`, `cargo check --workspace --all-targets`, `clippy -p fp-batch` ohne Warnungen.
Logikfälle §4b/§5.10: Parameter 50× ändern / Schritt 50× hinzufügen-entfernen ohne Reste; Lauf
zweimal → `batch-output-2` bzw. `name (2)` bzw. übersprungen, Effekt nie doppelt (Pixelvergleich);
Vorschau zweimal = gleich, veraltete Generation bricht ab; Abbrechen mitten im Lauf → keine `.fptmp`,
fertige Dateien lesbar, Originale unverändert (Prüfsumme); kaputte Datei mitten in der Liste, 0-Byte,
unbekannte Dateien; gleiche Namen aus zwei Ordnern, `a.png`+`a.jpg`→`a (2).webp`; Unicode/Emoji,
schreibgeschützte Quelle, Pfad > 300 Zeichen; 10 000 Dateien planen < 2 s; Budget 1 Byte und 1 vs. 4
Threads → gleiche Ziele; Animation 3 Frames GIF→GIF, Animation→JPEG mit Warnung; Original ersetzen
mit (inkrementeller) Sicherung.

## Öffentliche Schnittstellen für BATCH-2 … 11

- **Neuer Schritt** (BATCH-4 ff.): Datei `crates\fp-batch\src\steps\<gruppe>.rs` mit
  `pub fn register(r: &mut StepRegistry)`; in `steps\mod.rs` je **eine** Zeile `pub mod <gruppe>;`
  und `<gruppe>::register(r);`. `impl Step`: `id`, `group`, `params()` (Schema mit `ParamDef::int(..)
  .beginner().unit("px")` …), `apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext)`
  (Parameter kommen mit Vorgaben aufgefüllt). Pixel-Schritte am einfachsten über
  `item.map_images(|img| …)` (Einzelbild + jedes Frame; Dokumente werden flach gerechnet).
  Abbrechen zwischen Zeilen/Frames: `ctx.check_cancel()?`. Hinweise: `ctx.warn(..)`.
  Optional `applies_to()`, `preview_ok()`, `file_only()`, `kind() = Collect` + `collect()`, `summary()`.
- **Texte**: Name `batch-step-<id mit - statt .>` (+ `.description`), Parameter
  `batch-param-<id>-<param>`, in eigener `batch_<paket>.ftl` (de+en). `step::step_label(id)`.
- **Oberfläche (BATCH-2)**: `Recipe` (serde) bearbeiten über `add_step/remove_step/move_step/set_param`;
  `recipe.validate(&reg) -> Vec<Issue>` (Karte ⚠, „Verarbeiten“ sperren bei `Severity::Error`);
  `fp_batch::collect(paths, recursive, allow_other, existing) -> Collected {items, duplicates, skipped}`
  (`allow_other = pipeline::is_file_only(&recipe, &reg)`); `executor::plan(..)` für Trockenlauf/Plan-
  Reiter (`PlanEntry {source, target, action, hints: Vec<NameHint>, conflict_with}`,
  `NameHint::message()`); `executor::spawn(recipe, items, RunOptions) -> JobHandle` (`events.try_recv()`
  je Frame, `cancel()`, `join()`); `RunOptions.ask` für „Jedes Mal fragen“; `pipeline::preview(..)` mit
  `PreviewOptions { cancel: CancelToken::for_generation(gen, n), encode_full, .. }` →
  `PreviewResult {before, after, after_item, before_size, after_size, bytes_before, bytes_after, warnings}`;
  `StepRegistry::global().catalog_json()` bzw. `.iter()` für die Galerie, `StepGroup::label()`.
  Rezeptordner `fp_batch::recipe_dir()` (`BatchRecipes`), Endung `recipe::RECIPE_EXTENSION`.
- **Umbenennen-UI (BATCH-3)**: `naming::build_name(&RenameSpec, &NameContext) -> NameResult`,
  `naming::compile_regex`, `normalize_group_refs` (`$1_x` → `${1}_x`), `split_extension`,
  `apply_case`, `transliterate`, `sanitize_stem`. `explain.rs` ist noch frei.
- **Metadaten (BATCH-5)**: `item::set_exif_probe(fn(&Path, &[u8]) -> Option<ExifSummary>)` füllt
  `{exif-date}`/Kamera-Bausteine (Aufrufer setzt `ItemInfo.exif` über `item::read_exif(path)` vor dem
  Plan); `Meta { exif, icc, xmp, extra }` am Element.
- **Animation/Video (BATCH-7)**: `Item::Animation { width, height, frames: Vec<Frame{image, delay_ms}>,
  loop_count, meta }` (Vollbilder); `ItemKind::Video` wird bei `VIDEO_EXTENSIONS` erkannt, `load()`
  dekodiert Videos noch nicht (Fehler „unbekanntes Format“) → BATCH-7 hängt sich in `item::load` ein
  (Befund: Haken für Video-Decoder nötig, z. B. `set_video_loader`, analog `set_exif_probe`).
- **GIF-Optimierer (BATCH-8)** / weitere Formate (BATCH-9: SVG, PSD, JXL, PDF):
  `output::register_encoder(OutputFormat::Gif, fn(&Item, &Map) -> Result<Vec<u8>>)` ersetzt den
  eingebauten Kodierer (Optionen = `recipe.output.options`).
- **MCP (BATCH-9)**: `fp_batch::install()` einmal je Prozess, dann `run_command batch.run/plan/preview/
  steps/validate`; Hilfen `host::recipe_from_param`, `items_from_params`, `apply_overrides`.

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-engine\src\commands\mod.rs`: `pub mod batch;` + `batch::register(r);` (2 Zeilen).
- `apps\firepixel-cli\src\main.rs`: `mod batch;` + `if let Some(code) = batch::intercept() { return code; }`
  (4 Zeilen; der clap-`Batch`-Zweig bleibt unverändert, `--recipe/--steps` werden vorher abgefangen;
  `intercept()` steckt zugleich `fp_batch::install()` ein, daher gehen `batch.*` auch in `run --cmd` und
  `firepixel-cli mcp`). `apps\firepixel-cli\Cargo.toml`: Abhängigkeit `fp-batch` (1 Zeile + Kommentar).
- `crates\fp-i18n\tests\legacy_area_names.rs`: feste Zahl 18 Bereichsdateien und Liste „nie umbenannter“
  Bereiche verallgemeinert (sonst bricht jede neue `.ftl`, auch die von BATCH-2 ff.). Bitte I18N-KERN prüfen.
- `Cargo.lock`: neues Paket `fp-batch`.
- **Neu (für I18N/HG):** `crates\fp-ui\tests\i18n_keine_festen_texte.rs` → `SCAN_DIRS` um
  `("crates/fp-batch/src", Mode::Msg)` ergänzen (nicht selbst geändert).
- **Neu (für EB):** `file.batchProcess` mit optionalem `recipe` → an `batch.run` weiterleiten (SPEC §5.1;
  `export.rs` nicht angefasst). Die App (`fp-ui`) und `fp-mcp` müssen beim Start `fp_batch::install()`
  aufrufen (BATCH-2 bzw. BATCH-9).
- Vorbestehend, nicht von BATCH-1: `cargo test -p firepixel-cli --bin firepixel-cli`
  → `parameterbeschreibungen_in_de_und_en` meldet 19 fehlende `cmdtext-*` (file.open, edit.copyHtmlCode,
  element.setPattern …); keiner davon aus `batch.*`.

## Neue Abhängigkeiten (alle schon im Baum, permissiv)

`regex` 1.13 (MIT/Apache-2.0), `walkdir` 2.5 (MIT/Unlicense), `chrono` 0.4.45 mit `clock`/`serde`
(MIT/Apache-2.0), `rayon` (Workspace). Keine GPL/AGPL/LGPL, kein `qcms`, nichts heruntergeladen außer
dem Crate-Register. Kein `unsafe`.

## Annahmen

- Bericht `batch-report.json` wird nicht geschrieben, wenn das Ziel „gleicher Ort“ ist (sonst landet er
  im Quellordner); nur Protokoll. Bei mehreren Ausgabeordnern (Struktur) im gemeinsamen Oberordner.
- Neuer Ordner neben den Quellen: vorhanden **und leer** → wiederverwendet; nicht leer → `batch-output-2` …
  (SPEC §2.6); im eigenen Ordner greift die Konfliktregel (`name (2)`).
- In-Lauf-Konflikt mit Regel „überschreiben“: das spätere Element bekommt `name (2)` (nie die Ausgabe
  eines früheren Elements überschreiben).
- `ask` ohne Rückfrage-Funktion (CLI, MCP) verhält sich wie `rename`; die CLI bietet `ask` nicht an.
- `$1_x` im Regex-Ersetzen bedeutet Gruppe 1 + `_x` (Normalisierung zu `${1}_x`).
- Datumsbausteine in Ortszeit (chrono); „heute“ = Planzeitpunkt.
- Speichergrenze Vorgabe 4 GB (freier RAM wird ohne zusätzliche Crate nicht gemessen); `output.memoryLimitMb`
  bzw. `RunOptions.memory_budget` überschreibt.
- `file.copyOnly` verträgt keine Bildschritte und kein anderes Zielformat (Validierungsfehler);
  „verschieben“ nur bei ausdrücklicher Wahl, in jedes Ziel.
- Animation → Standbildformat (JPEG, PNG …): erstes Bild mit Warnung; Animation → PNG ist kein APNG (dafür `apng`).

## Offene Punkte

- Größen-Baustein `{w}x{h}` nutzt die Quellgröße (Plan vor dem Lauf); „Größe nach den Schritten“ braucht
  eine Neuberechnung nach dem Lauf (Konflikte können sich dann ändern) – mit BATCH-3/4 entscheiden.
- TIFF-Option `compression` wird noch nicht an `fp-export` durchgereicht (fp-export wählt selbst); TGA,
  PSD, JPEG XL, PDF, SVG schreiben erst über `register_encoder` (spätere Pakete).
- Kein Strg+C-Abbruch in der CLI (Prozess endet sofort; Temp-Dateien heißen `.<name>.fptmp`).
- `async`-Läufe über MCP (`batch.status`/`batch.cancel`, P2) noch nicht.
- Sammel-Schritte: Ergebnisse heißen wie das erste Element (`name (2)` … für weitere); Feinschliff mit
  `anim.fromFrames`/Kontaktbogen.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- CLI im echten Ordner: `firepixel-cli batch --recipe web.fpbatch.json --in <Fotos> --recursive` →
  `batch-output` neben den Fotos mit Struktur, `batch-report.json`, Exit 0; zweiter Lauf → `batch-output-2`.
- Großer Lauf (≥ 1 000 Fotos, gemischte Formate) auf allen Kernen: Speicher bleibt unter der Grenze,
  Abbrechen über die Oberfläche (sobald BATCH-2 da ist) hinterlässt keine `.fptmp`.
- `batch-log.txt` unter `%LOCALAPPDATA%\Firepixel\Logs` wird angehängt (englisch).
