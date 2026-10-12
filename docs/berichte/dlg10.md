# DLG-10: Datei-Info (D10) und Rechtschreibprüfung einrichten (D24)

## Umgesetzt (Commit siehe `git log --grep "DLG-10"`)

**D10 Datei ▸ Datei-Info**
- Statt der Meldung öffnet sich ein eigenes Fenster „Datei-Info“ (`crates\fp-ui\src\dialogs\file_info.rs`, Client 560 × 470).
  - Gruppe „Beschreibung“: Dokumenttitel, Autor, Beschreibung (mehrzeilig), Schlüsselwörter, Copyright-Vermerk.
  - Gruppe „Datei“ (nur Anzeige): Pfad, Seiten, Symbole, Formatversion, Erstellt, Geändert, Anwendung (die letzten drei aus `xmp:CreateDate`/`ModifyDate`/`CreatorTool` eines vorhandenen Pakets).
  - Startfokus im Titelfeld, Esc/Abbrechen ändert nichts.
- Engine: `file.fileInfo` liegt jetzt in der neuen Datei `crates\fp-engine\src\commands\file_info.rs` (aus `file.rs` entfernt, Registrierungszeile in `commands\mod.rs`).
  - Neue optionale Parameter: `title`, `author`, `description`, `keywords` (Liste oder Text), `copyright`.
  - Antwort zusätzlich `metadata {…}`, die bisherigen Schlüssel bleiben.
- Gespeichert wird als XMP-Paket in `Document::file_meta.xmp_chunk` (iTXt, `re\02-fwpng-format.md` §1).
  - Ein vorhandenes Paket bleibt erhalten, ersetzt werden nur die fünf Dublin-Core-Felder.
  - Schreiben über fp-fwpng wie bisher. Rundreise Speichern/Öffnen ist getestet.
- Ändern ist **ein** Verlaufsschritt („Datei-Info“). OK ohne Änderung legt keinen Schritt an. 30× setzen ergibt keine doppelten Elemente.

**D24 Text ▸ Rechtschreibprüfung einrichten**
- Eigener Dialog `crates\fp-ui\src\dialogs\spelling_setup.rs` statt des Formulars:
  - Wörterbuchliste mit Kästchen (gefundene `.aff`/`.dic` plus aktive; Namen wie „Englisch (USA)“ über `dlg-spell-lang-*`)
  - Pfad zum persönlichen Wörterbuch mit Knopf „…“
  - „Persönliches Wörterbuch bearbeiten“ öffnet den Unterdialog 6519 (ein Wort je Zeile)
  - Optionen: Doppelte Wörter finden, Wörter mit Zahlen ignorieren, Internet- und Dateiadressen ignorieren, Wörter in GROSSBUCHSTABEN ignorieren. Vorgabe: alle an, wie im Original.
  - Startfokus OK, Esc bricht ab.
  - Zusätzlich, weil Firepixel keine Wörterbücher mitliefert: Feld „Wörterbuchordner (.aff/.dic)“ mit „…“.
- Engine `text.spellingSetup` (`commands\spelling.rs`):
  - Neue Parameter: `languages` (mehrere aktive Wörterbücher; ein Wort gilt als richtig, wenn eines es kennt), `personalDictionary` (Datei; beim Umstellen ziehen die vorhandenen Wörter in die neue Datei), `ignoreUrls`, `findDuplicates`.
  - Antwort zusätzlich `available`, `languages`, `personalDictionary`.
- `text.checkSpelling` meldet doppelte Wörter mit `duplicate: true`. Mit `replace[{…, duplicate: true}]` werden sie zusammengefasst („der der“ → „der“), als ein Verlaufsschritt.
- Der Prüfdialog zeigt dann „Doppeltes Wort:“, und Löschen/Ändern fassen zusammen.
- Entfernt: `spelling::setup_form`, `menu_dialogs` „file.fileInfo“-Zweig, Schlüssel `dlg-spell-setup-language`/`-personal`, `dlg-md-file-info`/`-text`. Die Texte von `dlg-spell-setup-folder`/`-ignore-upper`/`-ignore-numbers` sind neu.

**Texte** (de+en)
- `dialoge.ftl`: `dlg-fileinfo-*`, `dlg-spell-setup-*`, `dlg-spell-personal-*`, `dlg-spell-duplicate`, `dlg-spell-lang-*`
- `meldungen.ftl`: `msg-step-file-info`
- `befehlstexte.ftl`: 9 `cmdtext-*` für die neuen Parameter

## Geprüft
- `cargo test -p fp-ui --test all`:
  - `dialogs_dlg10::` (7 neu)
  - `dialogs_dlg1::`, `dialogs_move::`, `dialogs_open::`, `dialogs_text_layout::`
  - Scanner `i18n_keine_festen_texte::`
- `cargo test -p fp-engine --test all`: `phase5::`, `commands::`, `all_commands::`
- `cargo test -p fp-engine --lib commands::`: neue Unit-Tests für XMP, Doppelte und Adressen
- `cargo test -p fp-i18n`, `cargo test -p fp-neutral`, `cargo check --workspace --all-targets`: alles ohne Warnung grün.

## Befunde
- IBD-Befund „(von PM-5, PARITAET_B D07/D10/D11/D12/D13/D15/D24) Unverändert seit UX-B unvollständig …“: die Teile **Datei-Info (XMP-Fenster)** und **Rechtschreibprüfung einrichten (Wörterbuchliste, 4 Optionen)** sind erledigt (DLG-10). Offen bleiben D11, D12, D13 und D15.

## PARITAET_B §2 → „umgesetzt, Nachprüfung ausstehend“
- D10 Datei ▸ Datei-Info…
- D24 Text ▸ Rechtschreibprüfung einrichten…

## Nachprüfung (NACHPRUEFUNG)
- **D10:**
  - Ablauf: Datei ▸ Datei-Info…, Titel/Autor/Schlüsselwörter eintragen, OK.
  - Erwartet: Verlauf zeigt „Datei-Info“, Strg+Z leert die Felder wieder.
  - Dann speichern, schließen und neu öffnen: Die Felder stehen wieder im Fenster.
  - Datei mit vorhandenem XMP öffnen: Erstellt/Geändert/Anwendung werden angezeigt, nach dem Speichern sind die übrigen XMP-Inhalte unverändert.
  - Esc schließt ohne Änderung.
- **D24:**
  - Ablauf: Text ▸ Rechtschreibprüfung einrichten…
  - Erwartet: Startfokus OK, Liste mit Kästchen, Pfad + „…“, Knopf „Persönliches Wörterbuch bearbeiten“ (Unterdialog), 4 Optionen angehakt.
  - Text „der der WLET www.x.de“ prüfen: Nur „der“ wird als doppeltes Wort gemeldet. „Löschen“ fasst zusammen.
  - Optionen abschalten: WLET und die Adressteile werden gemeldet.
  - In en und qps auf Überlauf prüfen.

## Neue Befunde / Entscheidungen
- **XMP-Schlüssel (Nutzerentscheidung nötig):** Der PNG-übliche iTXt-Schlüssel für XMP und die Standard-Namensräume von XMP (x:xmpmeta, xmp:, xmpRights:) enthalten den Herstellernamen. Das verbietet §1a ohne Ausnahme.
  - Neue Pakete nutzen deshalb den Schlüssel `XMP` und nur Dublin Core (purl.org) ohne x:xmpmeta-Hülle.
  - Firepixel liest sie wieder ein (Erkennung am Paket). Fremdprogramme wie exiftool finden das XMP neuer Dateien aber nicht.
  - Vorhandene Pakete aus Originaldateien behalten ihren Schlüssel und ihre Namensräume.
  - Für volle Austauschbarkeit bräuchte es eine Ausnahme in `fp-neutral` für diese Formatkennung (Koordinator/Nutzer).
- Fenstergröße D10: Das Original misst 680 × 620 mit mehreren XMP-Registern (IPTC, Kamera …). Firepixel zeigt nur die Seite „Beschreibung“ in 560 × 470. Weitere Register sind ein möglicher Folgeauftrag.
- Das Feld „Wörterbuchordner“ in D24 gibt es im Original nicht (es liefert seine Wörterbücher mit). Es ist eine bewusste Ergänzung.
