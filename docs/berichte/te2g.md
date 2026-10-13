# TE2-G – Abschluss TE-2/DOC-3 (13.10.2026)

Slot `fp-s1` / `target-s1`. Commit: siehe `git log --grep "TE2-G"`.

## Umgesetzt

1. **Restsuche alte Namen** (`git grep` nach allen alten `.ftl`-, Quelldatei-, Beispiel-, Handbuch-,
   Bezeichner- und Variablennamen). Umgestellt (nur Kommentare/Doku, kein Verhalten):
   - `crates\fp-mcp\tests\sprache.rs` (`menue.ftl` → `menu.ftl`), `crates\fp-trace\Cargo.toml`
     (`nachzeichnen.ftl` → `trace.ftl`), `crates\fp-ui\tests\i18n_ausnahmen.txt` (Kommentar
     `inspector\texte.rs`/`texte::keys_of_lists_exist` → `texts`), `crates\fp-ui\tests\i18n_qa_glossar.rs`
     (`menu.ftl`), `crates\fp-ui\tests\panels_i18n.rs` (`panels.ftl`).
   - `crates\fp-ui\tests\design_bilder.rs` und `docs\DESIGN.md`: Aufruf `--test design_bilder` gibt es seit
     INF-1 nicht mehr → `--test all design_bilder::` (bzw. `design_willkommen::`); DESIGN nennt
     `FP_DESIGN_IMAGE=before|after` und `design_before_*`/`design_after_*` (alte Namen als Rückfall/Bestand).
   - `STATUS.md`: aktuelle Angaben auf neue Namen (Handbuch `MANUAL.md`, `--example ai_images`,
     `inspector\texts.rs`, `.ftl`-Dateien je Bereich mit „bis TE-2 …“, CLI `language check|template|list`
     mit `language.rs`, `FP_DESIGN_IMAGE=after`), TE-1-Punkt „Offen (TE-2)“ als erledigt, Gesamtübersicht:
     TE-2 und DOC-3 fertig.
   - `docs\TESTLEITFADEN_2026-10-13.md`: nichts zu ändern (nennt die Umbenennung schon richtig, `FP_QA_ONLY`).
2. **`docs\TECHNISCH_ENGLISCH.md`:** E21 `FP_QA_OUT` statt `FP_QA_OFF` (mit Begründung), E15 Funktionen
   `check`/`template`/`list` und Import `fp_i18n::check as i18n_check`, E16 Modul `inspector::texts`,
   E20–E25 genaue Fundstellen, Werte `current`/`baseline` und Ordner `…\i18n-qa\current\`/`baseline\`
   (Rückfall `aktuell\`/`basis\`), `before`/`after` und Bilder `design_before_*`/`design_after_*`;
   Nachtrag aus te2a–te2e (Rückfall `LEGACY_AREA_NAMES`/`area_file_name`/`legacy_area_file_name`,
   unveränderte `.ftl`, `MANUAL_STEM`/`LEGACY_MANUAL_STEM`, englische Unit-Testnamen, `curve_edit.rs`,
   was noch deutsch ist). Abschnitt A: Fundstellen A17/A26/A27 auf `language.rs`, `cli.ftl`, `ui.ftl`.
   Abschnitt D: jede Gruppe mit Stand (☑ erledigt bzw. „bleibt“).
3. **`docs\BEFUNDE.md`** (Abschnitt TE): 3 TE-1-Befunde für TE-2 abgehakt (mit Commits), Nachträge an
   „automatisch erzeugte Berichte“ und `dlg-md-*` (in TE-2 nicht erledigt, Datei jetzt `dialogs.ftl`, → FIX),
   **3 neue Befunde:** (a) roter firepixel-cli-Unit-Test `parameterbeschreibungen_in_de_und_en` (19 Texte ohne
   Schlüssel in `command_texts.ftl`, Welle 5 – nicht behoben), (b) deutsche QA-Bereichs-IDs/Ordner in
   `i18n_qa_*`, (c) 107 Bilder `design_vorher_*`/`design_nachher_*`, `design_willkommen.rs` schreibt fest
   `design_nachher_*`.
4. **`docs\PLAN.md`:** §7 neu gezählt (Stand nach DOC-3/TE-2): PARITAET_A 119/0/1/6, PARITAET_B Menüs
   276/0/0/30 ℹ️/44 ⏳, Dialoge 3/0/1/0/46 ⏳, Bedienfeldmenüs 1/1 ❔/8 ⏳, BEFUNDE 347 erledigt · 86 offen,
   NACHPRUEFUNG 172 (1 ✅ · 171 ⏳), Nutzerbefunde 18 ✅ · 5 🔧 · 1 ❌ teilweise · 7 ❌ (N25–N31), Umfang
   20 Crates + 2 Programme, ≈ 222.000 Zeilen Rust (609 Dateien), 192 Testdateien, 292 Commits. TE-2 und DOC-3
   in §4/§4c/§4f/§5 als ☑ mit Commits.

## Bewusst geblieben

- **Rückfall-Code und -Tests:** `fp_i18n::check::LEGACY_AREA_NAMES` und Kommentare dazu
  (`crates\fp-i18n\src\check.rs`), `crates\fp-i18n\tests\legacy_area_names.rs`,
  `apps\firepixel-cli\src\language.rs` (Kommentar zu `menue.ftl`), `apps\firepixel-cli\tests\language_english.rs`
  (`menue.ftl`/`allgemein.ftl`), `crates\fp-engine\src\commands\misc.rs` (`LEGACY_MANUAL_STEM = "ANLEITUNG"`,
  Tests `manual_legacy_names`, `help_command_returns_english_manual`), `crates\fp-neutral\tests\technical_english.rs`
  (Wortliste des Wächters), Rückfall-Lesestellen `FP_QA_NUR/AUS/NEU/MODUS`, `FP_DESIGN_BILD`, `FP_SPRACHEN_DEBUG`
  und Werte `aktuell`/`basis`/`vorher`/`nachher` in `i18n_qa_common\mod.rs`, `design_bilder.rs`,
  `dialogs_des1.rs`, `gesamt_sprachen.rs`, `i18n_qa_de.rs` (Kommentare nennen die alten Namen als „alt“).
- **Keine alten Namen, nur gleichlautende deutsche Wörter bzw. fremde Dateien:** „allgemein“, „Dialoge“,
  „Dateidialoge“ in Kommentaren/Texten; `re\bedienfelder\dialoge.json`, `re\11-bedienfelder-dialoge.md`,
  `re\18-export-dialoge.md` (Spezifikationen außerhalb des Repos); Fluent-ID `cmdtext-design-der-oberflaeche`
  (aus dem deutschen Text abgeleitete ID); `"Warndialoge zurücksetzen.jsf"` (eingebauter Befehl, Abschnitt C);
  temporäre Testordner `fp-dateidialog-…`, `fp-ai-dialoge-…`, `fp-i18n-dialoge-…`; Bildnamen `inspektor_*`,
  `N21_nachher_inspektor_*` der Testläufe.
- **Testdatei- und Testfunktionsnamen** (Nutzerentscheidung), z. B. `crates\fp-mcp\tests\sprache.rs`,
  `antworten_englisch_ohne_oberflaeche_umzuschalten`, `gesamt_oberflaeche`, `bedienfelder_bei_minimaler_dockbreite`.
- **QA-Bereichs-IDs, Ordner `basislayout`, Cachedatei `dialoge.json`, Bilder `design_vorher_*`/`design_nachher_*`**
  → neue Befunde (b)/(c), eigene Entscheidung nötig.
- **Historische Dokumente:** `docs\berichte\*`, `docs\progress-2026-10-13.md`, `docs\preview-notes-2026-10-13.md`,
  Erledigt-Einträge und ältere Agentenabschnitte in `BEFUNDE.md`, `PLAN.md` (§2, Pakettabellen mit
  `allgemein.ftl`/`ANLEITUNG.en.md`), `STATUS.md` („Erledigt (Commit …)“-Zeilen, TE-1-Abschnitt),
  `docs\TESTLEITFADEN_2026-10-12.md`, `ARCHITEKTUR.md` (nur `re\`-Verweise), `DESIGN.md` §12/§13 (Bestand
  `design_vorher_*`, Links auf vorhandene Bilder), Abschnitte A–D in `TECHNISCH_ENGLISCH.md` (Bestandsaufnahme).
- **Automatisch erzeugt:** `docs\I18N_PRUEFUNG.md`, `docs\funktionstest\*_auto.md` (alte `ANLEITUNG*`- und
  `.ftl`-Namen bis zum nächsten Lauf, GT-1).

## Geprüft

Siehe Abschnitt „Prüfung“ unten.

## Für PARITAET / Nachprüfung

- keine Zeilen; keine Nachprüfung im Fenster nötig (nur Kommentare und Doku).

## Prüfung

`cargo check --workspace --all-targets` grün; `cargo test -p fp-neutral -p fp-i18n -p fp-mcp` grün
(fp-neutral nach den letzten Doku-Änderungen erneut grün); `cargo test -p fp-ui --test all` Module
`panels_i18n::`, `i18n_qa_glossar::`, `design_bilder::`, `i18n_keine_festen_texte::`, `dialogs_des1::`,
`gesamt_sprachen::` 12/12 grün (4 ignoriert, Bild-/QA-Läufe), `FP_NO_SHELL_OPEN=1`.
