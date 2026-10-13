# BATCH-11b – Feinschliff Teil 2 (Batch-Studio)

Stand 13.10.2026. Spezifikation `docs\BATCH_MODERN.md` §2.3, §2.5, §2.8, §3.2, §3.10; Gestaltung wie der
Startbildschirm (DESIGN §12.1). Commits: `git log --grep BATCH-11b` (4 Commits auf `main`).

## Umgesetzt

| Teil | Inhalt |
|---|---|
| **Endkarte „Speichern als“: eigene Felder je Format** (`fp-batch\src\steps\files.rs`, `fp-ui\src\batch\recipe.rs`) | Schema `output` mit `ParamDef::when("format", …)` statt fester Liste in der Oberfläche (`output_defs` entfernt). Gleichnamige Felder je Format mit der Vorgabe des Kodierers: `quality` JPEG 85 / WebP 82 / AVIF 60, `compression` PNG (default/fast/max), TIFF (default/none/lzw/deflate/packbits), TGA (default/none); `palette` nur bei PNG 8 Bit (Kette). Bei **GIF** hängt die Karte das Schema `output.gif` als Abschnitt „GIF-Optimierer“ an: Voreinstellung (Segment), Qualität, Zielgröße (KB), im Profi-Modus Farben, Palette, Dithering, Stärke, Verlust, Differenz-Transparenz, Schwelle, Zuschnitt, Zusammenlegen, Entsorgung, Transparenz, **Teilbilder (`splitFrames`)**, Hintergrund (nur ohne Transparenz). `colors`, `lossy`, `splitFrames` sind jetzt „Automatisch“ (ohne Vorgabe), weil der Kodierer sie nach Qualität und Inhalt wählt. **Voreinstellung wählen setzt die Qualität** (60/80/100) im selben Rückgängig-Schritt. Unter den Feldern: „Erreicht: 184 KB (Ziel 200 KB)“ bzw. ⚠ „Zielgröße nicht erreichbar: kleinste Datei … (Ziel …)“ aus der echten Kodierung der Vorschaudatei; ohne Zielgröße „Ergebnis der Vorschaudatei: …“. **Formatwechsel räumt auf**: Werte, die das neue Format so nicht annimmt (TIFF `lzw` → PNG), werden entfernt, gültige (Qualität) bleiben; Rückgängig stellt beides her. Hinweise in der Karte: „Nachzeichnen“ + Pixelformat, „JPEG füllt Transparenz mit ‚Hintergrund‘“. Kein JSON-Feld mehr in der Endkarte. |
| **SVG und FW-PNG in einem Lauf** (Befund BATCH-7/BATCH-9; `executor.rs`) | Option `alsoFwpng` („Zusätzlich als FW-PNG“, alle Formate außer FW-PNG): neben jede Ausgabe eine bearbeitbare FW-PNG gleichen Stamms (`logo.svg` + `logo.fw.png`), Temporärdatei + atomares Umbenennen, Abbrechen beachtet; Konflikt nach Regel (umbenennen `name (2).fw.png`, überspringen mit Hinweis, überschreiben). Auch im Sammel-Pfad (`anim.fromFrames`). Bericht: `items[].extraTargets`. |
| **HTML-Bericht** (`fp-batch\src\report_html.rs`, neu) | `batch-report.html` neben `batch-report.json` (Rezept `output.reportHtml`, Vorgabe an; JSON `reportHtml`). Eigenständige Datei ohne Skripte/Netz: Banner im Firepixel-Stil (Verlauf + Pixelraster), Kacheln (fertig, übersprungen, Fehler, Größe vorher → nachher mit %, Dauer), Fehlerliste (Datei, Schritt, Meldung), je Datei eine Karte mit **Vorschaubildern vorher/nachher** (112 px, als PNG eingebettet, Schachbrett hinter Transparenz), Maßen, Größen, Änderung in %, Hinweisen, Zusatzdateien, Verweis auf die Ausgabe. Hell/dunkel nach System. Texte in der Laufsprache (`batch-html-*`), alles HTML-maskiert. Bis 400 Dateien mit Bildern (parallel), Quellen > 64 MB und Videos ohne Bild. Ende-Karte: Knopf **„Bericht ansehen“** (nur ohne `FP_NO_SHELL_OPEN`). |
| **Zielgröße nicht erreichbar im Lauf** (`output.rs`) | GIF mit `maxKB` über dem Ziel → Warnung im Bericht und in der Vorschau. |
| **ffmpeg-Hinweis in Zeile und Karte** (`files.rs`, `recipe.rs`, `mod.rs`) | Fehlt ffmpeg (nur Dateisuche, zwischengespeichert 3 s; `Studio::ffmpeg_available`), zeigt die **Videozeile** „⚠ ffmpeg fehlt – Video kann nicht gelesen werden“ als Verweis (Rolle Link „ffmpeg-Datei…“) → öffnet die Batch-Einstellungen (Abschnitt ffmpeg); die Zeile wird dabei nicht ausgewählt. Die Karte **„Video lesen“** zeigt ⚠ im Kopf, die Meldung (Firepixel lädt nichts herunter) und den Knopf „ffmpeg-Datei…“. „ffmpeg-Datei…“ verwirft den Zwischenspeicher. |
| **Filter nach Größe/Pixeln** (Befund BATCH-2; `files.rs`) | Profi: in der Typ-Liste Felder „Dateigröße (MB) ab/bis“ und „Längste Seite (px) ab/bis“ (Dezimalkomma), „Filter zurücksetzen“, Liste mit „•“ bei aktivem Filter; Zähler „1 von 3 Dateien“, gefilterte Dateien werden nicht verarbeitet. Suchtext mit `*`/`?` = Platzhalter über den ganzen Namen (beide Modi, Tooltip). Reihenfolge-Zwischenspeicher kennt jetzt auch den Modus. |
| **Vorlage „Video → kleines GIF“** | `data\recipes\video-to-gif.fpbatch.json`: `video.import` 12 fps, `resize` längste Seite 480, GIF „ausgewogen“ (Qualität 80). Menü, CLI, MCP, Suche nach deutschem/englischem Titel. |
| **Hilfetexte** | `firepixel --help` (`ui-cli-help`): `--batch [REZEPT] [DATEI …]` mit Vorlagen und Beispiel. `firepixel-cli batch --help` (ohne `--actions`) zeigt jetzt die Hilfe des Rezept-Modus, die Liste der Vorlagen und die ältere Form; Beispiele mit Vorlage, Video → GIF, `--templates`, Hinweis auf den HTML-Bericht; `--recipe` nennt Vorlagennamen. |
| **Formular** (`params.rs`) | `generic_body` in Bausteine geteilt (`form_rows`, `label_column`, `more_toggle`, `set_param`/`unset_param`, `text_info` öffentlich) – die Endkarte nutzt zwei Schemata mit gemeinsamer Beschriftungsspalte und einem „Mehr Optionen (n)“. Zahlenfeld mit „Automatisch“ ist so breit, dass der Hinweis ganz passt (vorher „Automati…“). |
| **Spezifikation** | `BATCH_MODERN.md` §3.2 (Felder je Format, `alsoFwpng`, GIF verweist auf §3.10), §3.10 (Voreinstellung → Qualität, `colors`/`lossy`/`splitFrames` automatisch, `transparency`, `background`, `splitFrames`), §2.8 (HTML-Bericht umgesetzt). |

Texte: `crates\fp-i18n\locales\{de,en}\batch_finish.ftl` (neu), Tooltips der GIF-Felder in `batch.ftl`,
CLI-Beispiele in `batch.ftl`, `ui-cli-help` in `ui.ftl`.

Bilder (1440 × 900, `docs\screenshots\batch11b_{gif,video,done}_{normal,dunkel,oled}_{de,en}.png`):
`gif` = Profi, Endkarte GIF offen; `video` = Videozeile + Karte „Video lesen“ ohne ffmpeg; `done` =
Ende-Karte nach einem echten Lauf („Bericht ansehen“ im Test gesperrt, weil `FP_NO_SHELL_OPEN`).
Erzeugen: `cargo test -p fp-ui --test all batch_finish::batch11b_bilder -- --ignored`.
Gestaltungsdurchsicht: 18 Bilder angesehen – Karten, Abstände (16 px), Schriften und Leitfarben wie der
Startbildschirm, nichts abgeschnitten (bis auf vorhandene „…“-Kürzungen mit Tooltip); gefunden und
behoben: „Automati…“ im Zahlenfeld.

## Tests (alle grün)

- `cargo test -p fp-batch` (alle Programme), neu `tests\finish_b11b.rs` (7): Felder je Format (keine
  doppelten sichtbaren Namen in allen 18 Formaten, Vorgaben je Kodierer, Ketten), GIF-Felder ohne JSON
  mit Automatik und Texten de/en, Zielgröße nicht erreichbar → Warnung (ohne Ziel keine), SVG + FW-PNG
  in einem Lauf (Dokument mit Pfaden, `extraTargets`, zweiter Lauf `(2)`, „überspringen“ überschreibt
  nichts, keine `.fptmp`, ohne Haken/bei FW-PNG keine Zusatzdatei), Zusatzname, HTML-Bericht (Masken
  von `<`/`&`, ≥ 4 eingebettete Bilder, Maße 120×80 → 40×27, Fehlerliste, relativer Verweis,
  `reportHtml`, Wiederholung ersetzt in neuer Sprache, abschaltbar), Vorlage Video → GIF; Unit-Tests
  `report_html` (Base64, Maskieren, URL, Prozent).
- `cargo test -p fp-ui --test all batch_` (49 + 4 ignoriert; neu `batch_finish` 7): GIF-Endkarte ohne
  JSON (Anfänger/Profi), Voreinstellung per Klick → Qualität, 50× hin und her ohne doppelte Schlüssel,
  Rückgängig/Wiederholen gemeinsam; Zielgröße Zeichen für Zeichen getippt = 1 Schritt, ⚠ nicht
  erreichbar bzw. „Erreicht“; Formatwechsel über die Liste räumt auf + Rückgängig; Schalter
  „Zusätzlich als FW-PNG“ + Hinweise; ffmpeg-Verweis in der Zeile und Knopf in der Karte öffnen die
  Einstellungen (Esc schließt, Auswahl bleibt), mit ffmpeg kein Hinweis; Filter Platzhalter, Felder in
  der Typ-Liste getippt, Zähler, Anfänger ignoriert Profi-Filter, nur Gefiltertes wird verarbeitet.
  `batch_ui::trockenlauf_und_lauf_zweimal` war in einem Lauf unter Last einmal rot (2 statt 3
  Plan-Einträge), allein und im zweiten Gesamtlauf grün – siehe Befunde.
- `FP_RAND_VIEWS=Batch FP_RAND_FULL=1 clipping_audit::`: **0 Funde**, 12 Hinweise (Bildlauf), 6 mit „…“.
- `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte::`,
  `cargo test -p firepixel-cli --test batch_templates --test batch_recipe`.
- `cargo test -p fp-neutral`: rot durch eine **fremde** Datei `docs\funktionstest\ct1a_2026-10-13.md:60`
  (CT-1a, schon auf `main`); eigene Dateien und Bilder sauber (Bildtest prüft jedes PNG).

## Einhängungen in fremde Dateien

- `crates\fp-i18n\locales\{de,en}\ui.ftl` (HG): `ui-cli-help` um `--batch` ergänzt (Befund BATCH-9).
- `apps\firepixel-cli\src\batch.rs` (AU/BATCH-1): `batch --help` ohne `--actions` abgefangen.
- `crates\fp-batch\src\steps\gifopt\mod.rs` (BATCH-8): nur das Schema von `output.gif` (Automatik,
  `background`); Kodierer unverändert.

## Befunde erledigt (Wortlaut-Anfang)

- (von BATCH-7/BATCH-9, Kern) „Mehrere Ausgaben je Element (SVG **und** FW-PNG in einem Lauf …“ – SVG/FW-PNG
  erledigt (`alsoFwpng`); Atlas-`.json` zum Sprite-Sheet bleibt offen.
- (von BATCH-9, für BATCH-1/11) „HTML-Bericht `batch-report.html` (P2) nicht umgesetzt“ – erledigt.
- (von BATCH-9, für HG) „`ui-cli-help` (`ui.ftl`) um `--batch …` ergänzen“ – erledigt (Startseiten-Eintrag
  „Batch“ offen).
- (von BATCH-9) „Vorlage „Video → kleines GIF““ – erledigt.
- (von BATCH-2/7/8/9, für BATCH-11) „Oberfläche: Filter nach Größe/Pixel …“ – Größe/Pixel, Platzhalter,
  GIF-Felder (`preset`, `quality`, `maxKB`) mit erreichter Größe, ffmpeg in der Videozeile, Hinweis
  „SVG/FW-PNG wählen“ und „JPEG füllt Transparenz“ erledigt; offen: Transparenz-Filter, Regex/Glob als
  eigenes Feld, Sortieren per Ziehen, Leertaste/Eingabe, „Als Vorlage speichern“/„Hilfe“ im Kartenmenü,
  Fortschritt je Datei, Karte `video.import` automatisch einfügen.

## Für PARITAET/STATUS („umgesetzt, Nachprüfung ausstehend“)

Batch-Studio: Felder je Ausgabeformat inkl. GIF-Optimierer, Zielgröße mit Rückmeldung, SVG + FW-PNG in
einem Lauf, HTML-Bericht mit Vorschaubildern, ffmpeg-Hinweis in Liste und Karte, Größen-/Pixelfilter,
Vorlage „Video → kleines GIF“, Hilfetexte `--batch` / `firepixel-cli batch --help`.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes Fenster (Brücke): Studio mit 5 Fotos, „Speichern als“ → GIF: Voreinstellung „Am kleinsten“ →
  Qualität springt auf 60, Strg+Z nimmt beides zurück; Zielgröße 50 KB → nach der Vorschau „Erreicht …“
  bzw. ⚠ „Zielgröße nicht erreichbar …“; Profi: „Teilbilder“ Automatisch | An | Aus. 3 Designs × de/en.
- Format TIFF → Kompression LZW → Format PNG: Kompression „Standard“, Strg+Z → TIFF/LZW.
- Logos (Vorlage „Logos nachzeichnen → SVG“), „Zusätzlich als FW-PNG“ an, Lauf: je Logo `.svg` und
  `.fw.png`; FW-PNG in Firepixel öffnen → Ebene mit Pfadgruppe bearbeitbar; zweiter Lauf → `(2)`.
- Nach einem Lauf „Bericht ansehen“ (ohne `FP_NO_SHELL_OPEN`): Browser zeigt `batch-report.html` mit
  Vorschaubildern, Größen, Fehlerliste; hell/dunkel nach Windows-Einstellung; Verweise öffnen die Ausgabe.
  Ordner verschieben → Bilder bleiben (eingebettet).
- Ohne ffmpeg ein `.mp4` in die Liste: Zeile „⚠ ffmpeg fehlt …“, Klick öffnet die Einstellungen; Karte
  „Video lesen“ mit Knopf „ffmpeg-Datei…“; nach Wahl einer echten `ffmpeg.exe` verschwinden die Hinweise
  (spätestens nach 3 s).
- Profi: Typ-Liste → „Längste Seite ab 1000“ → nur große Bilder, Zähler „n von m“; Suchtext `*.png`.
- `firepixel.exe --help` und `firepixel-cli batch --help` (de/en) lesen; `firepixel-cli batch --recipe
  video-to-gif clip.mp4 --out gifs` mit ffmpeg.

## Annahmen

- „Alle übrigen Endkarten“ = die Formatoptionen aller übrigen Ausgabeformate in „Speichern als“ (die
  Endkarte „Dateiname“ hatte schon eigene Felder, BATCH-3). Die JSON-Felder einzelner Schritte
  (`filter.registry.params`, `script.*`, `adjust.curves.points`, `metadata.edit.tags`) bleiben offen.
- Pixelfilter = **längste Seite** in Pixeln (verständlicher als Megapixel); Dateien ohne bekannte Maße
  passen immer. Größen-/Pixelfilter nur im Profi-Modus (SPEC §2.10), Werte bleiben beim Wechsel erhalten.
- GIF-Voreinstellung setzt die Qualität ausdrücklich (statt sie nur intern zu bevorzugen), damit der
  Schieber den wirksamen Wert zeigt.
- `alsoFwpng` zählt nicht in `bytesOut`/Summen (die Hauptausgabe zählt); der Trockenlauf nennt die
  Zusatzdatei nicht.
- HTML-Bericht: Vorschaubilder höchstens 400 Dateien, Dauer = Wandzeit aus `started`/`finished`.
- ffmpeg-Zustand in Zeile/Karte = nur Dateisuche (kein Prozessstart beim Zeichnen); ob die Datei wirklich
  startet, zeigt weiter die Prüfung in den Einstellungen.

## Neue Befunde

- (für AU, Doku) `docs\AGENTS.md`/`AGENTS.en.md` §5c: „Beides auf einmal: zweimal laufen lassen“ ist
  überholt → `output.options.alsoFwpng: true`; Bericht nennt `batch-report.html` (`reportHtml`) und
  `items[].extraTargets`; Vorlage `video-to-gif` in die Tabelle.
- (für NT) `docs\funktionstest\ct1a_2026-10-13.md:60` enthält einen gesperrten Namen (`fp-neutral` rot).
- (für BATCH/RAND) `batch_ui::trockenlauf_und_lauf_zweimal` einmal unter Last mit 2 statt 3
  Plan-Einträgen – vermutlich Zeit-/Temp-Ordner-abhängig, beobachten.
- (offen, Batch) Transparenz-Filter, Glob/Regex als eigene Felder, Sortieren per Ziehen,
  Leertaste/Eingabe, Fortschritt je Datei, „Als Vorlage speichern“/„Hilfe“ im Kartenmenü, JSON-Felder der
  Schritte (s. Annahmen), Atlas-JSON zum Sprite-Sheet, Startseiten-Eintrag „Batch“.

## Neue Abhängigkeiten

Keine (Base64 und HTML-Maskierung eigen; `png`, `rayon`, `chrono` schon im Crate).
