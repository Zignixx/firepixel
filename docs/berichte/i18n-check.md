# I18N-CHECK – alles in Deutsch und Englisch (13.10.2026, vor der nächsten Testversion)

Slot `fp-s3`, Ziel `target-s3`. Auftrag: Lücken in DE/EN finden und schließen (ftl, feste Texte,
Befehlsregister, CLI, Handbuch/AGENTS, echtes Fenster).

Commit: `aeef2aa` (alle Änderungen und dieser Bericht).

## Ergebnis je Bereich

| Bereich | Geprüft mit | Gefunden | Behoben |
|---|---|---|---|
| ftl de ↔ en | `fp-i18n` (vollständig, keine verwaisten Schlüssel, Platzhalter/Zugriffstasten), eigenes Skript: en-Werte mit Umlauten/deutschen Wörtern, en = de wortgleich | 6633/6633 Schlüssel, keine Lücke, kein verwaister Schlüssel, keine deutsche Kopie in en (einziger Treffer `batch-rn-transliterate` „ä → ae“ ist gewollt) | – |
| ftl en Feinheiten | `i18n_qa_glossar` (Hinweise) + Durchsicht | `batch-bg-err-model-missing` mit „Manage AI models…“ (falsche Schreibung des Menüs) und 8 en-Texte mit geraden statt typografischen Anführungszeichen (`batch-bg-*`, `batch-html-recipe`, `batch-rx/rn-group-named*`, `dlg-export-folder-missing`, `ui-revert-confirm`) | angeglichen („Manage AI Models...“, “…”) |
| Feste Texte im Code (UI, Meldungen, Statuszeile, Dialoge, Batch-Studio, Fehler, CLI/MCP-Ausgaben) | Scanner `i18n_keine_festen_texte` (scharf + Bericht `--ignored`): 0 Treffer; dazu Diff aller Commits seit 13.10. 00:00 (fp-ui, fp-engine, fp-batch, fp-export, fp-mcp, apps) auf neue Literale | keine neuen festen Nutzertexte; neue Texte von FIX-G1…G5, FIX-N32/N33, FIX-R1…R3 laufen über ftl (z. B. `dlg-numeric-scale-attributes`, `dlg-hs-external-css`, `menu-edit-undoImpossible`) | – |
| `batch-report.html` | Durchsicht `fp-batch/src/report_html.rs` | alle Texte über `batch-html-*` | – |
| Befehlsregister (Labels, Beschreibungen, Menüpfade, Parameterbeschreibungen; MCP `list_commands`/`describe_command`, CLI `commands`) | neuer Test `firepixel-cli` `tests::labels_and_descriptions_in_en` (jeder Befehl: en-Schlüssel für Label und ggf. Beschreibung vorhanden, kein deutscher Rest in Label/Beschreibung/Menüpfad) + bestehender `parameterbeschreibungen_in_de_und_en` | keine Lücke | Test bleibt als Wächter |
| CLI `--help` aller Unterbefehle, `--lang de` / `--lang en` | alle 13 Hilfen + `batch --actions x --help` in beiden Sprachen ausgegeben und durchsucht | **DE:** `batch --help` (Rezept-Modus) komplett mit clap-Gerüst auf Englisch („Usage: batch“, „Arguments:“, „Options:“, „Print help“, „[possible values: …]“, Wertnamen FILE/FOLDER/FILES); in allen Hilfen „[OPTIONS]“, „[default: …]“, „[possible values: …]“ englisch; Wertnamen `<FILE>`, `<SCRIPT>`, `<INPUT>`, `<OUTPUT>`, `<CATEGORY>`, `<DEPTH>`, `<PAGE>`, `<STATE>`, `<TYPES>`, `<ZOOM>`, `<ELEMENT>`, `<ARG>`, `<TOKEN_FILE>`, `<FILTER>` englisch bzw. Bezeichner; `language template --from` mit doppelter Vorgabe „(Vorgabe: en) [default: en]“ | `localize_help_text` (neue Schlüssel `cli-usage-options`, `cli-help-default`, `cli-help-possible-values`), `localize_scaffold` für `batch` (Hilfe-Option, Überschriften, Vorlage, Aufruf „firepixel-cli batch“), Wertnamen als `.value` bzw. `batch-cli-value-*` in de/en; Doppelangabe entfernt. Ergebnis: DE ohne englisches Gerüst, EN ohne deutsche Reste |
| Handbuch `MANUAL.md` / `MANUAL.en.md` | Abschnitte verglichen (gleich aufgebaut), Funktionen von heute gesucht | fehlten in beiden: Strich ab Werk „Präzise abger.“, Numerisch transformieren „Attribute skalieren“, Maske ▸ Alles einblenden/ausblenden, Unterebenen als Ordnerzeile + Doppelklick-Umbenennen, HTML-Einrichtung „CSS in externe Datei schreiben“, Batch-Studio („Dateien…“/„Ordner…“, Bericht `batch-report.html`) | in beiden Sprachen ergänzt (Beschriftungen aus den ftl-Dateien) |
| `AGENTS.md` / `AGENTS.en.md` | Abschnitte verglichen (gleich aufgebaut) | `modify.transform.numericTransform` mit `scaleAttributes` und `modify.mask.revealAll/hideAll` nicht erwähnt | in beiden ergänzt (§3) |
| Echtes Fenster, Englisch (Slot-Build, eigenes Profil, Brücke, Fenster 1600×1000 hinter dem Original) | Bilder `re\_testdateien\i18n\en_01…en_11`: Startseite, Hauptfenster, Modifizieren ▸ Maske, Numerisch transformieren, Bildgröße, Leinwandgröße, HTML-Einrichtung, Voreinstellungen, Batch-Studio, Datei-Menü, Maske + Unterebene im Ebenen-Bedienfeld | kein deutscher Rest, nichts abgeschnitten | – |
| Echtes Fenster, Deutsch | `de_01…de_04`: Modifizieren ▸ Maske, Numerisch transformieren, HTML-Einrichtung, Batch-Studio | in Ordnung | – |

## Nicht geändert (Hinweise, keine Lücke)

- `i18n_qa_glossar` meldet 335 Hinweise (meist Glossar-Heuristik, z. B. „Rand“ → „Edge“ bei „Margin“);
  Durchsicht ohne echten Fehler. Ellipsen-Hinweis `cmd-file-new` („&Neu“ / „&New...“) und
  `…`/`...` bei zwei Menüpunkten betreffen Menütexte nach dem Original, bewusst nicht angefasst.
- Wertnamen wie `<CODE>`, `<N>`, `<JSON>`, `<FORMAT>`, `<PORT>`, `<TOKEN>` bleiben in beiden Sprachen.
- CSS-Werte in der HTML-Einrichtung (`no-repeat`, `scroll`, `left`) sind Schlüsselwörter wie im Original.

## Prüfungen

- `cargo test -p fp-i18n -p fp-neutral -p firepixel-cli -p fp-batch` grün
- `cargo test -p fp-ui --test all i18n_` (Scanner 0), `clipping_audit::`, `--test dialogs_i18n` grün
- `cargo check --workspace --all-targets` ohne Warnungen

## Nachprüfung

- Testversion: `firepixel-cli --lang de batch --help` → „Aufruf: firepixel-cli batch [OPTIONEN] [DATEIEN]...“, „Optionen:“, „[mögliche Werte: …]“, „Hilfe anzeigen“; `--lang en` unverändert englisch.
- Hauptfenster in EN (Ansicht ▸ Sprache ▸ English): Startseite, Menüs, Dialoge wie oben ohne deutschen Text.
