# TE2-F – Technisches Englisch Teil 2, Doku (13.10.2026)

## Umgesetzt

- `docs\ANLEITUNG.md` → `docs\MANUAL.md`, `docs\ANLEITUNG.en.md` → `docs\MANUAL.en.md` (`git mv`,
  Verlauf bleibt); gegenseitige Verweise in beiden Handbüchern angepasst.
- `docs\AGENTEN_REGELN.md`: §1 Zuständigkeit HG (`docs\MANUAL.md`, `docs\MANUAL.en.md`),
  §1b `allgemein.ftl` → `common.ftl`, §7 VT-Zeile `menue.ftl`/`nachzeichnen.ftl` → `menu.ftl`/`trace.ftl`.
- `crates\fp-i18n\GLOSSAR.md`: `allgemein.ftl` → `common.ftl` (2×).
- `docs\TECHNISCH_ENGLISCH.md`: neuer Abschnitt **E. Teil 2 (TE-2, 13.10.2026)** mit Tabelle alt → neu
  (14 ftl-Dateien, 3 Quell-/Beispieldateien, 2 Handbücher, 6 Test-Umgebungsvariablen, 2 Bezeichner)
  und Rückfällen; Abschnitt D als erledigt markiert (Testdatei- und Testfunktionsnamen bleiben laut
  Nutzerentscheidung); die beiden Zeilen in C (`ANLEITUNG*`, `Preset::Schwarzweiss/Foto`) als erledigt.

## Geprüft, ohne Änderung

- `docs\AGENTS.md`, `docs\AGENTS.en.md`, `docs\MODELLE.md`, `crates\fp-mcp\docs\{de,en}`: keine alten
  ftl-, Quelldatei-, Handbuch- oder Variablennamen. Ein Root-`README.md` und weitere
  `crates\*\README.md` (außer `fp-i18n`, macht TE2-A) gibt es nicht.
- Skripte für Testversion/Release: im Repo gibt es keine, die `ANLEITUNG` nennen; `REA\setup\*.ps1`
  und `REA\vorschau-build` (Quellkopie) ebenfalls nicht. Wer eine Testversion zusammenstellt und das
  Handbuch neben die exe legt, nimmt künftig `docs\MANUAL*.md`.
- `cargo test -p fp-neutral`: grün (alle 5 Testziele).
- Grep `ANLEITUNG` danach: nur noch Code und Texte anderer Agenten bzw. bewusste/historische Stellen:
  - `crates\fp-engine\src\commands\misc.rs` (`help_names`, Kommentare, Unit-Test) und die
    Erlaubnisliste in `crates\fp-neutral\tests\technical_english.rs` – **TE2-E** (MANUAL zuerst,
    `ANLEITUNG*` als Rückfall). Der Unit-Test `find_help`/echter `docs`-Ordner ist durch
    `is_file()` abgesichert und bleibt auch vor TE2-E grün.
  - `msg-help-intro` und Kommentar in `locales\{de,en}\meldungen.ftl` (künftig `messages.ftl`) nennt
    noch `ANLEITUNG.md` – Oberflächentext; **offen für TE2-A/TE2-E** (Befund unten).
  - historische Berichte `docs\berichte\*.md`, `docs\progress-2026-10-13.md`,
    `docs\preview-notes-2026-10-13.md`, automatisch erzeugte `docs\I18N_PRUEFUNG.md`,
    `docs\funktionstest\*_auto.md` (zeigen den alten Pfad bis zur Neuerzeugung);
    `BEFUNDE.md`, `PLAN.md`, `DESIGN.md`, `STATUS.md` gehören DOC-3.

## Commits

- `TE2-F: Handbuch ANLEITUNG*.md -> MANUAL*.md, Doku auf neue ftl-/Datei-/Variablennamen, TECHNISCH_ENGLISCH Abschnitt E`

## Für PARITAET

- keine Zeilen.

## Nachprüfung

- Hilfe ▸ Firepixel-Hilfe in der Testversion (DE und EN): öffnet `MANUAL.md` bzw. `MANUAL.en.md`
  (nach TE2-E); mit nur einer alten `ANLEITUNG.md` neben der exe wird diese geöffnet.

## Neue Befunde

- `msg-help-intro` (de/en) nennt noch „`ANLEITUNG.md` neben dem Programm“ – auf `MANUAL.md`
  (`MANUAL.en.md`) umstellen, sobald TE2-E die Suche umgestellt hat (falls nicht schon dort erledigt).
- `docs\funktionstest\*_auto.md` und `docs\I18N_PRUEFUNG.md` bei der nächsten Erzeugung neu schreiben
  (alte Pfade `docs\ANLEITUNG*.md`, alte ftl-Namen).
- Abschnitt E nennt die Rückfälle so, wie die Aufträge von TE2-A/E sie vorsehen; Abgleich mit deren
  Berichten `docs\berichte\te2a.md` … `te2e.md` (lagen beim Abschluss von TE2-F noch nicht vor)
  durch den Doku-Agenten.
