# BATCH-3 – Umbenennen-UI (Endkarte „Dateiname“)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §4 (SPEC-BATCH), Paket BATCH-3 aus §6.1.
Nutzerwunsch: Umbenennen mit einfachem Regex **und einem Anfänger-Modus für Leute ohne
Regex-Kenntnisse**, Präfix, Suffix, alles klein/groß.

## Umgesetzt (Commit `BATCH-3: …`, siehe `git log --grep BATCH-3`)

| Datei | Inhalt |
|---|---|
| `crates\fp-ui\src\batch\rename_ui.rs` (neu) | Kartenformular `rename` (über `params::register_card_body`): **Baukasten** mit Bausteinen als Chips in Gruppenfarben (Originalname ganz/erste n/letzte n/ohne Nummer, Text, Nummer mit Start/Schritt/Stellen/je Ordner, Datum mit Quelle Änderung/EXIF/heute und Format-Vorlagen – Profi: freies Format –, Bildgröße, Ordnername, Kameradaten, Zielformat): anklicken = Einstellungen darunter, × entfernt, Ziehen mit Einfügelinie, Strg+←/→ verschiebt den Chip mit Fokus, „+ Baustein“-Menü mit Erklärungen; **Regex** (Profi, Umschalter „Baukasten \| Regulärer Ausdruck“): Suchen/Ersetzen in Code-Schrift, rote Kante + Meldung „Fehler an Stelle n: …“ (Bibliothekstext im Tooltip), **Bedeutung in Klartext** unter dem Muster und **„Neuer Name: …“** unter dem Ersetzen-Feld, Groß/klein ignorieren, alle Treffer, ganzer Name mit Endung, „Aus Baukasten übernehmen“, Platzhalter-Zeile, **Spickzettel** (8 Beispiele mit Beispielname und „Einfügen“); **Optionen** in beiden Arten: Text ersetzen (mehrere Zeilen, alle Vorkommen, Groß/klein), Leerzeichen (beibehalten/_/-/entfernen/eigenes), Präfix, Suffix, Schreibweise (Anfänger ohne kebab/snake/camel), „Mehr Optionen“: Umlaute, Sonderzeichen, Endung klein/unverändert/GROSS, jpeg → jpg; **Vorlagen ▾** (Fotos nach Aufnahmedatum `2026-10-13_001`, Durchnummerieren `Bild_001`, Datum voranstellen, Web-tauglich, Zurücksetzen); **Live-Tabelle alt → neu** (Alt \| → \| Neu \| Hinweis): zeichenweise Hervorhebung (entfernt rot durchgestrichen, eingefügt grün getönt), unveränderte Namen gedämpft mit „=“, ⚠ Konflikt (gleicher Name wie Zeile x bzw. existiert im Ziel, aufgelöst nach der Konfliktregel), ungültige Zeichen, reservierte Namen, gekürzt, leerer Name, führender Punkt; ⓘ EXIF-Datum fehlt; Tooltip je Zeile mit vollem Namen und allen Hinweisen; Zusammenfassung „n von m Namen ändern sich · k Konflikte“, Knopf „Nummer anhängen“ (Baukasten); ungültiges Muster → Fehlerblock, alte Namen gedämpft; ≤ 10 Zeilen sichtbar mit Bildlauf, höchstens 200 Zeilen (Rest „… und n weitere“); zwischengespeichert bis zur nächsten Änderung (Plan über ≤ 1 000 Dateien) |
| `crates\fp-batch\src\explain.rs` (neu) | `explain_pattern` (Erklärung aus dem Syntaxbaum von `regex-syntax`: Literale, `\d \w \s .` mit natürlichen Formen und Zählung `\d\d` → „2 Ziffern“, Klassen `[a-z0-9]` → „a–z, 0–9“, Unicode-Klassen, Wiederholungen `? * + {n} {n,} {n,m}` inkl. genügsam, Gruppen nummeriert/benannt, Alternativen, Anker „am Anfang/am Ende/der ganze Name“, Flags `(?i)`), `check_pattern` (Fehlerart → verständlicher Text mit Stelle in Zeichen, Rückverweise/Lookaround mit Hinweis auf `$1`), `explain_replacement` (`$1`, `${name}`, `$0`, `$$`, alle Platzhalter), `builder_to_regex` (Baukasten → gleichwertiges Muster `^(.*)$` + Platzhalter), `regex_is_replaceable`, `CHEAT_SHEET`, `apply_sample`, `char_diff` (LCS, lange Namen über gemeinsamen Anfang/Ende) |
| `crates\fp-batch\src\naming.rs` | Platzhalter erweitert (rückwärtskompatibel): `{n:Stellen:Start:Schritt}`, `{nf:…}` (je Ordner), `{name:first:N}`, `{name:last:N}`, `{name:trim}`, `{exif:model\|make\|lens\|iso\|aperture}`, `{format}`, `{{` = wörtliches `{`. **Ersetzungen gelten jetzt in beiden Arten** (vorher nur im Baukasten), damit der Wechsel Baukasten → Regex exakt gleichwertig ist |
| `crates\fp-i18n\locales\{de,en}\batch_rename.ftl` (neu) | alle Texte (`batch-rx-*` Erklärung/Fehler, `batch-rn-*` Karte/Tabelle/Spickzettel) |
| `crates\fp-batch\tests\rename_explain.rs` (neu) | 12 Tests |
| `crates\fp-ui\tests\batch_rename.rs` (neu) | 10 Tests + Bildtest |

Bilder 1440 × 1080, 3 Designs × de/en, Rezept-Spalte gerollt bis zur Live-Tabelle:
`docs\screenshots\batch3_baukasten_{normal,dunkel,oled}_{de,en}.png` (Anfänger, Chips, Einstellungen der
Nummer, Optionen, Tabelle) und `docs\screenshots\batch3_regex_{normal,dunkel,oled}_{de,en}.png` (Profi,
Muster `^IMG_(\d+)` mit Bedeutung, Ersetzen-Erklärung, Optionen, Tabelle mit Hervorhebung).
Erzeugen: `cargo test -p fp-ui --test all batch_rename::batch3_bilder -- --ignored`.

## Tests (alle grün)

- `cargo test -p fp-batch` (alle Testprogramme, inkl. `rename_explain` 12, `recipe_naming` 18, `run` 17).
  `rename_explain`: Erklärung des Spezifikationsbeispiels `^IMG_(\d+)` → „am Anfang „IMG_“, dann eine oder
  mehrere Ziffern (Gruppe 1)“ und 16 weitere Muster (de) bzw. Englisch; **leere Eingabe** (leeres Muster,
  `^$`, leeres Ersetzen); **ungültiges Regex** (8 Fälle mit Stelle und Klartext, unbekannte Unicode-Klasse,
  Lauf liefert Fehler statt Panik); Ersetzen-Erklärung; erweiterte Platzhalter; Ersetzungen im
  Regex-Modus; **Baukasten → Regex gleichwertig** (11 Baukästen × 6 Dateinamen mit Unicode/Emoji, `$`,
  `{`, `.fw.png`, führendem Punkt, EXIF – `build_name` identisch); **50× umschalten** ohne angehäufte
  Teile, eigenes Muster bleibt; Spickzettel (alle 8 gültig, verändern ihr Beispiel); `char_diff` mit
  Unicode und sehr langen Namen; **sehr lange Namen** (400 × „ä“ → gekürzt ≤ 255 mit Endung, 500 Gruppen,
  300 offene Klammern → Meldung statt Panik).
- `cargo test -p fp-ui --test all batch_rename::` (10 + Bildtest): Karte mit Baukasten/Optionen/Tabelle,
  Vorgabe „unverändert“ markiert; Baustein über „+ Baustein“ hinzufügen (gewählt, Einstellungen offen),
  × entfernen, Rückgängig; Verschieben (Logik, Einfügeposition, Strg+→); **Präfix 50 Tasten = ein
  Verlaufsschritt** mit Endwert, Rückgängig/Wiederholen; **Vorlage 50×** → drei Bausteine, keine
  Duplikate, ⓘ EXIF-Datum fehlt; **Umschalten übersetzt gleichwertig** (Tabelle vorher = nachher,
  Bedeutung sichtbar, 25× hin und her ohne Änderung, eigenes Muster geht nicht verloren);
  **ungültiges Regex**: „Fehler an Stelle 1: Klammer „(“ wird nicht geschlossen“, alte Namen bleiben,
  Verarbeiten gesperrt, Spickzettel-Einfügen macht es gültig, Anfänger-Hinweis „Zum Baukasten wechseln“;
  **Konflikte** (3 × „foto“ → 2 Konflikte, „gleicher Name wie Zeile 1“, `foto (2).png`, „Nummer
  anhängen“ → 0); leere Liste, keine Bausteine → `_`, ungültige Zeichen → `_`; **Unicode/Emoji/sehr lange
  Namen nichts abgeschnitten**: de/en/qps × 3 Designs × (1024×768 @100 %, 1920×1080 @150 %) × Anfänger/
  Profi, jeweils in 5 Bildlauf-Stellungen bis zur Tabelle (Profi mit offenem Spickzettel, falls sichtbar).
- `cargo test -p fp-ui --test all batch_ui::` (BATCH-2, 15) grün; `clipping_audit` mit
  `FP_RAND_VIEWS=Batch FP_RAND_FULL=1` grün (0 Funde).
- `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte::`,
  `cargo test -p fp-neutral`, `cargo clippy -p fp-batch -p fp-ui --tests` ohne Warnungen in eigenen Dateien.

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-ui\src\batch\mod.rs` (BATCH-2): `pub mod rename_ui;` und `rename_ui::register();` in
  `Studio::new` (2 Zeilen).
- `crates\fp-ui\src\batch\recipe.rs` (BATCH-2): die Zeile „alt → neu“ unter der Endkarte erscheint nur
  noch ohne eigenes Kartenformular (die Karte hat jetzt die Live-Tabelle; sonst doppelt). 3 Zeilen.
- `crates\fp-batch\Cargo.toml`: `regex-syntax = "0.8"` (MIT/Apache-2.0, war schon im Baum über `regex`);
  `crates\fp-batch\src\lib.rs`: `pub mod explain;`; `Cargo.lock` eine Zeile.
- `crates\fp-batch\src\naming.rs` (BATCH-1, Kern): Platzhalter erweitert, Ersetzungen in beiden Arten
  (BATCH-1-Tests unverändert grün).
- **Neu (für BATCH-2/11):** Der Studio-Tastengriff `consume(NONE, ArrowLeft/Right)` in `batch\mod.rs`
  (Ende von `keyboard`) verbraucht auch **Alt**+Pfeil (egui `matches_logically` ignoriert Alt). Darum
  verschieben die Chips mit **Strg**+←/→. Wenn Alt+←/→ gewünscht ist: dort `!i.modifiers.alt` prüfen.
- **Neu (für RAND/HG):** `clipping_audit` öffnet im Batch-Studio die Endkarte „Speichern als“; die Karte
  „Dateiname“ prüft `batch_rename::unicode_lange_namen_drei_designs_nichts_abgeschnitten` (inkl. qps).
  Optional eine zweite Batch-Ansicht mit offener Karte „Dateiname“ in `clipping_audit.rs` aufnehmen.

## Annahmen

- **Ersetzungen („Text ersetzen“) wirken auch im Regex-Modus** (nach dem Muster, vor Leerzeichen/Präfix/
  Suffix/Schreibweise, Reihenfolge §4.4 unverändert). Nur so ist „Umschalten übersetzt den Baukasten in
  ein gleichwertiges Muster“ exakt; alle übrigen Optionen gelten ohnehin in beiden Arten.
- Die Übersetzung nutzt `^(.*)$` (ganzer Name = Gruppe 1) und Platzhalter für Bausteine, die kein
  Regex-Teil sind (Nummer, Datum, Größe …). Wechsel zum Regex übersetzt nur, wenn das Muster leer oder
  noch eine unveränderte Übersetzung ist; eigene Muster bleiben, „Aus Baukasten übernehmen“ ersetzt sie
  bewusst. Zurück zum Baukasten zeigt die gespeicherten Bausteine (keine Regex → Baukasten-Rückübersetzung).
- Der Regex-Umschalter erscheint nur im Profi-Modus (SPEC §2.10). Ein Regex-Rezept im Anfänger-Modus
  zeigt einen Hinweis mit „Zum Baukasten wechseln“.
- Im Text-Baustein erscheinen Leerzeichen im Chip als `␣` (SPEC §4.1); Chip-Beschriftungen werden bei
  sehr schmaler Karte mit „…“ gekürzt (voller Text im Tooltip).
- Die Tabelle zählt Änderungen/Konflikte über die ersten 1 000 Dateien (Hinweis „gezählt: die ersten
  n“); das Studio übergibt der Karte ohnehin höchstens 200 sichtbare Dateien. Alle Ziele zeigt der
  Trockenlauf (Reiter „Plan“).
- Vorlagen der Karte ersetzen nur die Bausteine (Web-tauglich nur Optionen, Zurücksetzen alles);
  Rezept-Vorlagen im Banner (z. B. „Fotos nach Datum umbenennen“) sind BATCH-9.
- Beispielnamen im Spickzettel sind neutrale Dateinamen (`DSC_0042`, `scan 2026-10-13` …), nicht übersetzt.

## Für PARITAET/STATUS („umgesetzt, Nachprüfung ausstehend“)

- SPEC-BATCH §4.1 Baukasten, §4.2 Regex mit Klartext-Erklärung und Spickzettel, §4.3 Live-Tabelle,
  „Vorlagen nach Datum“ (Paket BATCH-3).

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes Fenster (Brücke): Batch-Studio mit 20 Fotos, Endkarte „Dateiname“ aufklappen → Chips ziehen
  (Einfügelinie, flüssig), „+ Baustein ▸ Nummer“, Start/Stellen tippen → Tabelle folgt sofort; Vorlage
  „Fotos nach Aufnahmedatum“ mit echten JPEGs mit EXIF (nach BATCH-5) → Datum aus EXIF, ⓘ bei Fotos ohne.
  Erwartet: nichts springt, ein Rückgängig je Eingabe.
- Profi: Umschalten auf „Regulärer Ausdruck“ → Muster `^(.*)$` + Platzhalter, gleiche Namen; `(abc`
  tippen → rote Kante, Meldung mit Stelle, „Verarbeiten“ gesperrt; Spickzettel „Datum umstellen“ einfügen.
- Lauf mit Umbenennen (Trockenlauf, dann Verarbeiten) → Dateien heißen wie in der Tabelle.
- Tastatur: Tab bis zu einem Chip, Strg+→/← verschiebt; Eingabe/Leertaste öffnet die Einstellungen.

## Offen

- Chips per Tastatur entfernen (Entf/Rücktaste nimmt das Studio für die Dateiliste ab) – heute über ×.
- „Größe nach den Schritten“ für den Baustein Bildgröße (Plan vor dem Lauf kennt nur die Quellgröße; offen
  aus `batch1.md`), Baustein-Hinweis „Größe der Quelldatei vor den Schritten“ steht in der Karte.
- Rückübersetzung Regex → Baukasten gibt es nicht (nicht allgemein möglich); der Baukasten bleibt erhalten.

## Neue Abhängigkeiten

`regex-syntax` 0.8 (MIT/Apache-2.0) – war schon über `regex` im Baum, nichts heruntergeladen. Kein `unsafe`.
