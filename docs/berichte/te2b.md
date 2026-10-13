# TE2-B: Technisches Englisch Teil 2, Teil B (fp-ui src)

Bereich: nur `crates\fp-ui\src\` (Aufrufer in `crates\fp-ui\tests\` mit angepasst). Stand 13.10.2026.

## Commits

- `TE2-B: inspector/texte.rs -> texts.rs, .ftl-Namen in Kommentaren, englische Unit-Testnamen` (dieser Commit, enthält auch diesen Bericht)

## Umbenennungen alt → neu

**Datei/Modul** (einzige `.rs`-Datei unter `fp-ui\src` mit deutschem Namen):
- `crates\fp-ui\src\inspector\texte.rs` → `texts.rs`, Modul `crate::inspector::texte` → `crate::inspector::texts`
  (Aufrufer: `inspector\{mod, ai_quads, obj_quads, popups, quads, tool_quads}.rs`, `dialogs\effects.rs`,
  Tests `tests\inspector_fv9.rs`, `tests\nz1.rs`)

**.ftl-Namen in Kommentaren** (nur Kommentare, keine Strings): `dialoge`→`dialogs`, `allgemein`→`common`,
`inspektor`→`inspector`, `menue`→`menu`, `bedienfelder`→`panels`, `werkzeuge`→`tools`, `willkommen`→`welcome`
in `dialogs\preferences.rs`, `inspector\texts.rs`, `menu.rs`, `menu_kbd.rs`, `panels\{autoshapes, color, common, mod}.rs`,
`toolbar.rs`, `tools_panel.rs`, `welcome.rs`.

**Unit-Tests im Quelltext** (private `#[test]`-Funktionen in `src`, keine Aufrufer):
- `file_dialog.rs`: `testmodus_pfad_beantwortet_nach_art` → `test_mode_path_answers_by_kind`,
  `mehrere_pfade_nur_bei_mehrfachauswahl` → `multiple_paths_only_for_multi_select`
- `bridge\inject.rs`: `ctrl_c_wird_copy_wie_egui_winit` → `ctrl_c_becomes_copy_like_egui_winit`,
  `klick_braucht_zeigerposition_und_traegt_umschalttasten` → `click_needs_pointer_position_and_carries_modifiers`,
  `dateien_und_text` → `files_and_text`, `aufbau_und_fehler` → `structure_and_errors`,
  `hauptfenster_ueber_eingabe_bedienen` → `main_window_driven_by_input`

Typen, Enum-Varianten, Module und öffentliche Funktionen in `fp-ui\src`: nach Durchsicht aller Bezeichner
(Wortzerlegung ohne Kommentare/Strings) keine weiteren deutschen Namen.

## Geprüft

`cargo check --workspace --all-targets` grün; `cargo test -p fp-neutral` grün; `cargo test -p fp-ui --lib`
(inspector::texts, bridge::inject::tests, file_dialog::tests) 9/9; `cargo test -p fp-ui --test all inspector_fv9:: nz1::` 17/17.

## Offen (nicht in meinem Bereich bzw. bewusst gelassen)

- `Preset::Schwarzweiss`, `Preset::Foto` werden in `fp-ui\src\dialogs\trace.rs` benutzt, sind aber in `crates\fp-trace` definiert (anderes Crate).
- Deutsche Eingabe-Aliasse in `bridge\inject.rs` (`"eingabe"`, `"seite"`, Maustasten) und `file_dialog.rs` (`"pfad:"`, `FP_DATEIDIALOG`): bewusst als zusätzlich angenommene Eingaben (TECHNISCH_ENGLISCH.md, Abschnitt C).
- Kommentare in `crates\fp-ui\tests\i18n_ausnahmen.txt` (Zeilen 81, 92) nennen noch `inspector\texte.rs` bzw. `texte::keys_of_lists_exist` (nur Kommentar, Datei liegt in `tests\`).
- Doku-Verweise auf `inspector\texte.rs` in `STATUS.md`, `docs\BEFUNDE.md`, `docs\TECHNISCH_ENGLISCH.md` (Abschnitt D) für den Doku-Agenten.
- Deutsche Strings in Fehlermeldungen/Asserts (z. B. `workspace.rs` Layout-Parser) sind Texte, keine Bezeichner.
