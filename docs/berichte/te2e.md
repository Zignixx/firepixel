# TE2-E: Technisches Englisch Teil 2, Teil E (13.10.2026)

Slot `fp-s5` / `target-s5`. Commit: siehe `git log --grep "TE2-E"`.

## Umgesetzt

### Bereich A – `crates\fp-engine\src`
- **Hilfe-Handbuch** (`commands\misc.rs`, `help_names`): sucht je Sprachfassung zuerst `MANUAL.<sprache>.md`
  bzw. `MANUAL.md`, direkt danach den alten Namen `ANLEITUNG.<sprache>.md` bzw. `ANLEITUNG.md` (Konstanten
  `MANUAL_STEM`, `LEGACY_MANUAL_STEM`). Reihenfolge Englisch: `MANUAL.en.md`, `ANLEITUNG.en.md`, `MANUAL.md`,
  `ANLEITUNG.md`; die Sprachfassung geht vor dem Namen. Tests: `manual_per_language` (neue Namen),
  `manual_legacy_names` (alte Namen, gemischte Ordner), `help_command_returns_english_manual` (Repo, beide Namen).
  Die deutschen Testnamen `anleitung_je_sprache`/`hilfe_befehl_liefert_englische_anleitung` sind umbenannt.
- Erlaubnisliste `crates\fp-neutral\tests\technical_english.rs`: Eintrag `misc.rs`/`ANLEITUNG` entfernt
  (der alte Name steht jetzt in einer `LEGACY_*`-Konstante; der Wächter verlangt das Entfernen veralteter Einträge).
- **Zusatz Koordinator:** `msg-help-intro` in `fp-i18n\locales\{de,en}\messages.ftl` (vor TE2-A `meldungen.ftl`) nennt `MANUAL.md` bzw.
  `MANUAL.en.md` (en ohne „(in German)“, da es jetzt `docs\MANUAL.en.md` gibt); Kommentar darüber ebenso.
- `.ftl`-Namen in Kommentaren: `meldungen.ftl` → `messages.ftl`, `befehlstexte.ftl` → `command_texts.ftl`
  (`command.rs`, `error.rs`, `tests\i18n_engine.rs`). Weitere Nennungen gab es in fp-engine nicht
  („nachzeichnen“ u. ä. sonst nur als deutsches Wort im Kommentar).
- Deutsch benannte Quelldateien in `fp-engine\src`: keine vorhanden. Deutsche Rust-Bezeichner: außer den zwei
  Testfunktionen oben keine gefunden (`label_de`, `display_de`, `category_de`, `name_de` … bleiben, Abschnitt C).

### Bereich B – Test-Umgebungsvariablen (alte Namen und Werte werden weiter gelesen)
| neu | alt | Fundstellen |
|---|---|---|
| `FP_QA_ONLY` | `FP_QA_NUR` | `i18n_qa_common` (`only_filter()`), `design_bilder`, `dialogs_des1` |
| `FP_QA_OUT` | `FP_QA_AUS` | `i18n_qa_common::out_root` |
| `FP_QA_NEW` | `FP_QA_NEU` | `i18n_qa_common::discover_dialogs` |
| `FP_QA_MODE` = `current`/`baseline` | `FP_QA_MODUS` = `aktuell`/`basis` | `i18n_qa_common::mode`, `i18n_qa_de`, `i18n_qa_bericht`, `i18n_qa_glossar` |
| `FP_DESIGN_IMAGE` = `before`/`after` | `FP_DESIGN_BILD` = `vorher`/`nachher` | `design_bilder` |
| `FP_LANGUAGES_DEBUG` | `FP_SPRACHEN_DEBUG` | `gesamt_sprachen` |

- **Abweichung vom Auftrag:** `FP_QA_AUS` heißt `FP_QA_OUT`, nicht `FP_QA_OFF`. „AUS“ steht hier für
  *Ausgabe* (Ausgabeordner), nicht für „aus“; `FP_QA_OFF` würde wie ein Abschalter gelesen. `FP_QA_OUT`
  passt zu `FP_UXB_OUT`, `FP_RENDER_OUT`, `FP_TRACE_OUT`.
- Neuer Helfer `i18n_qa_common::env_var(neu, alt)`; Ergebnisordner jetzt `…\i18n-qa\current\` bzw. `baseline\`,
  `mode_dir()` liest die alten Ordner `aktuell\`/`basis\`, solange es die neuen nicht gibt (vorhandene Basis bleibt nutzbar).
- `design_bilder` schreibt neue Bilder als `design_before_*.png`/`design_after_*.png` (auch bei alten Werten).
- Weitere deutsche Umgebungsvariablen: keine (Laufzeit, Tests, `tools\`). `FP_DATEIDIALOG` war schon durch TE-1
  mit Rückfall erledigt. In `tools\` und `docs\funktionstest\` gab es keine Verweise.

## Für TE2-F (Doku)
Neue Namen in `docs\` eintragen: `FP_QA_ONLY`, `FP_QA_OUT` (nicht OFF), `FP_QA_NEW`, `FP_QA_MODE=baseline|current`,
`FP_DESIGN_IMAGE=before|after`, `FP_LANGUAGES_DEBUG`; Fundstellen u. a. `docs\DESIGN.md` Z. 11,
`docs\I18N_PRUEFUNG.md` Z. 21/24/50, `docs\TECHNISCH_ENGLISCH.md` Abschnitt D, `STATUS.md` Z. 578.

## Offen / Hinweise (nicht umbenannt)
- 107 vorhandene Bilder `docs\screenshots\design_vorher_*`/`design_nachher_*` behalten ihre Namen; neue heißen
  `design_before_*`/`design_after_*`. `design_willkommen.rs` schreibt fest `design_nachher_startbildschirm*.png`.
- QA-Bereichsnamen (`hauptfenster`, `menues`, `bedienfelder` …), Befundarten (`rohschluessel`, `unmarkiert`, …),
  Ordner `basislayout` in `i18n_qa_bericht` und Bildpräfixe `phase6_basis_de_` sind deutsch geblieben (Ausgaben
  der Testläufe, Testdateien/-namen nicht Teil dieses Auftrags).

## Nachprüfung
- `i18n_qa_de -- --ignored` mit `FP_QA_ONLY=menues:1`: Ergebnis landet in `…\i18n-qa\current\`, Vergleich mit
  der vorhandenen Basis (`basis\`) läuft ohne „keine Basis“. Gleich mit `FP_QA_NUR`/`FP_QA_MODUS=aktuell`.
- Hilfe ▸ Hilfe in der App (de/en) öffnet `docs\MANUAL.md` bzw. `docs\MANUAL.en.md`; Kurzhilfe ohne Handbuch nennt `MANUAL.md`.

## Neue Befunde
- keine
