# TE2-D – Technisches Englisch Teil 2, Teil D (13.10.2026)

Bereich: `src\` und `examples\` von fp-ai, fp-trace, fp-color, fp-model, fp-raster, fp-export, fp-io,
fp-fwpng, fp-geom, fp-vector, fp-text, fp-render, fp-effects (Regel `docs\AGENTEN_REGELN.md` §1c,
Liste `docs\TECHNISCH_ENGLISCH.md` C/D).

## Umgesetzt (Commit „TE2-D: …“)

- `fp_trace::Preset::Schwarzweiss` → `Preset::BlackWhite`, `Preset::Foto` → `Preset::Photo`.
  serde-Namen (`blackWhite`, `photo`) und Aliasse (`schwarzweiss`, `bw`, `foto`) sowie
  `Preset::from_name` unverändert. Aufrufer angepasst: `crates\fp-ui\src\dialogs\trace.rs`,
  `crates\fp-ui\tests\dialogs_trace.rs`, `crates\fp-trace\tests\{trace,refine}.rs`.
  Die beiden Einträge der Erlaubnisliste in `crates\fp-neutral\tests\technical_english.rs` entfernt.
- `crates\fp-ai\examples\ki_bilder.rs` → `ai_images.rs` (Beispiel wird automatisch erkannt, kein
  Cargo-Eintrag; Aufrufzeile im Dateikopf angepasst). Weitere deutsch benannte Quell-/Beispieldateien
  gibt es im Bereich nicht.
- Kommentare mit altem .ftl-Namen: `meldungen.ftl` → `messages.ftl` in `fp-ai\src\{error,models}.rs`,
  `fp-color\src\swatches.rs`, `fp-raster\src\lib.rs`, `fp-export\src\lib.rs`, `fp-io\src\lib.rs`,
  `fp-fwpng\src\error.rs`. Keine anderen alten Bereichsnamen im Bereich.
- Bezeichner-Suche (Skript über alle Bezeichner, zerlegt in Wortteile, gegen deutsche Merkmale und
  ~200 deutsche Wörter): außer den Preset-Varianten keine deutschen Typen, Varianten, Module oder
  öffentlichen Funktionen. Nebenbei lokale Variablen `unten`/`oben` → `lower`/`upper` in
  `fp-raster\src\filters\curve_edit.rs`.

## Prüfung

`cargo check --workspace --all-targets`, `cargo test -p fp-neutral`, `-p fp-trace`, `-p fp-raster`
grün; fp-ui `dialogs_trace` nur übersetzt (Umbenennung rein mechanisch).

## Für die Doku (Doku-Agent)

- `docs\TECHNISCH_ENGLISCH.md` C: Zeile „Rust-Bezeichner `Preset::Schwarzweiss`, `Preset::Foto`“
  ist erledigt; D: `crates\fp-ai\examples\ki_bilder.rs` erledigt.
- `STATUS.md` (Zeile zur KI-Geschwindigkeit) nennt noch `--example ki_bilder` → `ai_images`.

## Nachprüfung / neue Befunde

Keine Nachprüfung im Fenster nötig (keine sichtbare Änderung). Keine neuen Befunde.
