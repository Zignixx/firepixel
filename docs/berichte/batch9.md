# BATCH-9 – Vektor, Vorlagen, MCP (Batch-Studio)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §2.5, §3.13, §5.8, Paket BATCH-9 aus §6.1.
Slot `fp-s5` / `target-s5`. Commit: siehe `git log --grep "BATCH-9"`.

## Umgesetzt

| Datei | Inhalt |
|---|---|
| `crates\fp-batch\src\steps\vector.rs` | Schritt **`vector.trace` „Nachzeichnen → SVG“** (Gruppe Erweitert): ruft in einer eigenen headless Sitzung je Element **denselben Befehl** wie das Menü (`modify.traceBitmap`) – beide Verfahren `clusters` (Farbflächen) und `potrace` (glatte Kurven), Vorgaben `logo/blackWhite/illustration/photo`, Aufräumen. Parameter `preset`, `engine` (Anfänger) und Profi-Werte `colorMode, colors, filterSpeckle, cornerThreshold, mode, hierarchical, threshold, ignoreTransparent, pathPrecision, refine, turdSize, turnPolicy, alphaMax, optCurve, optTolerance` mit Vorgabe `null` = „wie die Vorgabe“ (werden nur weitergereicht, wenn gesetzt); `keepOriginal` (Original-Bitmap als **ausgeblendete** Ebene, Vorgabe aus), `transparentCanvas` (Vorgabe an). Ergebnis ist ein **Dokument** (bearbeitbar). Dokumente werden vorher flach gerechnet, Animationen: erstes Bild + Warnung. **SVG-Kodierer** der Endkarte „Speichern als“ (`register_encoder(Svg)` über `fp_export::vector_out::write_svg`): Pfade mit `fill-rule="evenodd"`, ausgeblendete Ebenen fehlen; Einzelbilder ohne Nachzeichnen → eingebettetes Bild. FW-PNG über den vorhandenen Weg; Pixelformate rechnen das Ergebnis flach. |
| `crates\fp-batch\data\recipes\*.fpbatch.json` (11) | Eingebaute Vorlagen als gewöhnliche Rezeptdateien (englische Schlüssel): `web-webp`, `email-photos`, `save-as-jpeg`, `remove-background`, `transparency-to-white`, `logos-to-svg`, `small-gifs`, `strip-metadata`, `rename-by-date`, `square-1080`, `thumbnails-256`. |
| `crates\fp-batch\src\templates.rs` | Vorlagen eingebettet (`include_str!`), Titel/Beschreibung übersetzt (`batch-template-<id>`), `all()`, `ids()`, `find()` (ID ohne Groß-/Kleinschreibung, `_`=`-`, mit/ohne `.fpbatch.json`, oder Titel de/en/aktuell), `recipe()`, `catalog_json()`. |
| `crates\fp-mcp\src\batch.rs` | **MCP-Werkzeug `batch`**: `action` = `templates`, `steps`, `validate`, `plan`, `preview` (Text + zwei PNG-Bilder vorher/nachher), `run` (blockierend; mit `async: true` sofort `{jobId}`), `status` (Fortschritt: `state`, `total`, `processed`, `done`, `failed`, `skipped`, `percent`, `etaMs`, `errors[]`, am Ende `report`; `wait_ms` ≤ 60 s), `cancel`. Aufträge laufen in `fp_batch::executor::spawn` (eigene Threads und Sitzungen, nie die Sitzung der Oberfläche/Brücke). **Ressource `firepixel://docs/batch.md`** (Vorlagen + Schrittkatalog in der Antwortsprache). |
| `crates\fp-i18n\locales\{de,en}\batch_vector.ftl` | Texte de+en: Schritt, Parameter, Auswahlwerte (`batch-value-vector-trace-*`), Warnung, 11 Vorlagen, CLI, MCP-Werkzeug/-Parameter/-Fehler, Ressource. |
| `docs\AGENTS.md`, `docs\AGENTS.en.md` | Werkzeug `batch` in der Tabelle §2, Ressource `batch.md`, neuer Abschnitt **5c „Batch-Verarbeitung“** (Aktionen, Vorlagen, Nachzeichnen im Batch, Abläufe, `--batch`), CLI-Zeilen `--recipe <vorlage>`, `--templates`, `--steps`. |
| Tests | `crates\fp-batch\tests\steps_vector.rs` (13), `crates\fp-mcp\tests\batch_tool.rs` (4), `apps\firepixel-cli\tests\batch_templates.rs` (2), `crates\fp-ui\tests\batch_templates_b9.rs` (2) – alle grün. |

### Einhängungen in fremde Dateien (Befunde, je klein)

- `crates\fp-batch\src\steps\mod.rs`: `pub mod vector; // BATCH-9` + `vector::register(r); // BATCH-9`.
- `crates\fp-batch\src\lib.rs`: `pub mod templates;`.
- `crates\fp-batch\src\host.rs` (BATCH-1): `recipe_from_param` nimmt bei einem Text, der keine vorhandene Datei ist, den Namen einer eingebauten Vorlage (damit gehen Vorlagennamen in `batch.run/plan/preview/validate`, `file.batchProcess`, MCP und JSF).
- `apps\firepixel-cli\src\batch.rs` (BATCH-1): `--recipe <vorlage>` (Datei hat Vorrang), neue Option `--templates [--json]` (+ Abfangen in `intercept`).
- `crates\fp-mcp`: `lib.rs` `pub mod batch;`, `tools.rs` je eine Zeile (Werkzeugliste `.chain([crate::batch::tool_def()])`, Verteiler `"batch" => …`), `resources.rs` (Liste + `read`), `Cargo.toml` Abhängigkeit `fp-batch` (eigenes Crate). `Cargo.lock` entsprechend.
- `crates\fp-ui\src\lib.rs`: eine Zeile nach `fp_batch::install()` – `batch::register_templates(…)` (Anschluss an BATCH-2: Vorlagen erscheinen im Menü „Rezepte ▸ Vorlagen“).
- `crates\fp-ui\src\batch\mod.rs` (BATCH-2): `request_launch`/`apply_launch`/`launch` und ein Aufruf am Anfang von `frame_input` – Start des Studios mit Rezept und Dateien.
- `apps\firepixel\src\main.rs` (HG): `--batch [rezept|vorlage] [dateien/ordner …]` bzw. `--batch=<rezept>` → `fp_ui::batch::request_launch` (offener Punkt aus `batch2.md`). Hilfetext `ui-cli-help` (ui.ftl, HG) **nicht** ergänzt → Befund.

## Prüfungen (alle grün)

`cargo test -p fp-batch` (alle 13 Testprogramme), `cargo test -p fp-mcp` (inkl. `sprache`: Werkzeug in de/en übersetzt, Anzahl Werkzeuge = Server-Liste), `cargo test -p firepixel-cli --test batch_templates --test batch_recipe`, `cargo test -p fp-ui --test all batch_templates_b9::`, `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte::`, `cargo test -p fp-neutral`, `clippy -p fp-batch -p fp-mcp -p firepixel --all-targets` ohne Warnungen in den eigenen Dateien.

Testfälle (§4b):
- Logo (Kreis + Quadrat) nachgezeichnet: genau eine sichtbare Ebene mit einer Gruppe, keine Bitmap, gerendert < 3 % abweichende Pixel; Leinwand transparent bzw. wie die Quelle.
- `keepOriginal`: ausgeblendete Bitmap-Ebene; Vorschau/SVG/flach identisch zu ohne (keine `<image>` im SVG).
- Beide Verfahren an Strichzeichnung (< 4 % Abweichung, unterschiedliche Ergebnisse, `evenodd` für Ringe); Farbe mit Konturverfahren und eigenen Werten.
- Vorgabe-abhängige Parameter werden nur gesetzt weitergereicht (Wahl `photo` ändert alle Werte).
- Validierung (Vorgabe, Verfahren, Bereiche, Typen).
- Wiederholung: Parameter 50× ändern → ein Schritt, Ergebnis = einmal mit Endwert; nochmals nachzeichnen → wieder genau eine Ebene/Gruppe; 1 vs. 8 Threads gleich.
- Animation → erstes Bild + eine Warnung; ganz transparentes Bild → Fehler, Element unverändert.
- Vorschau-Pipeline mit `logos-to-svg` (Format svg, echte Größe, Bild stimmt).
- Lauf (Executor): SVG und FW-PNG (bearbeitbar, mit ausgeblendeter Original-Ebene) aus zwei PNGs; Originale unverändert, keine `.fptmp`; zweiter Lauf → `logo 0 (2).svg`.
- Vorlagen: alle Dateien eingebunden, IDs eindeutig, keine Prüffehler, keine unbekannten Schritte oder Parameter (Tippfehler), Titel/Beschreibung de+en, Sprache ändert nur Texte; Suche nach ID/Titel; vorhandene Datei hat Vorrang; „Transparenz auf Weiß“ und „Web + WebP“ rechnen end-to-end.
- MCP: Liste/Übersetzung, Ressource de/en, templates/steps/validate/plan (schreibt nichts)/Fehlerfälle, preview mit zwei PNG-Bildern, run blockierend, async + status (`wait_ms`, wiederholbar ohne Doppelzählung), Trockenlauf im Hintergrund schreibt nichts, cancel ohne Temporärdateien; Sitzung bekommt nie ein Dokument.
- CLI: `--templates` (Text de/en, JSON), `--recipe logos-to-svg --in …` → SVGs, unbekannter Name → Exit ≠ 0.
- UI: Vorlagen nach dem App-Start im Menü (de/en), keine Doppelanmeldung bei zweiter App; `launch` mit Vorlagenname/Datei/unbekannt, gleiche Dateien nicht doppelt, Rezept ersetzt statt addiert.

## Annahmen

- „Ausgabe SVG und/oder FW-PNG“: das Format wählt die Endkarte „Speichern als“ (ein Format je Lauf). Beide auf einmal = zwei Läufe (MCP/CLI: `out` je Format). Mehrere Ausgabedateien je Element brauchen einen Haken im Executor (BATCH-1) → Befund.
- Ohne `keepOriginal` wird die Original-Bitmap gelöscht und ihre leere Ebene entfernt; mit `keepOriginal` bleibt sie **ausgeblendet** (im FW-PNG einblendbar), damit SVG/Vorschau nur die Pfade zeigen.
- Leinwand standardmäßig transparent (Logos); weiße Flächen der Quelle werden – wie im Menübefehl – als weiße Pfade nachgezeichnet. Weißen Grund entfernen: vorher `background.removeColor`.
- Vorschau rechnet auf dem Proxy (≤ 1024 px); pixelbezogene Werte (`filterSpeckle`, `turdSize`) wirken dort relativ stärker.
- Vorlage „Kleine GIFs“ nutzt nur `resize` + Optionen des GIF-Optimierers von BATCH-8 (`preset: balanced`, `quality: 80`, Rest nach Vorgabe) und kein `video.import`/`anim.*`, damit sie ohne ffmpeg gültig ist. „Video → kleines GIF“ als eigene Vorlage → Befund.
- Auf main (nach BATCH-7/8) neu aufgesetzt: Konflikte nur in `fp-batch\src\lib.rs` und `steps\mod.rs` (Registrierungszeilen, beide Seiten behalten); danach Workspace-Prüfung und alle obigen Tests erneut grün.
- `rename-by-date` ohne Bildschritte → Datei wird nur kopiert (SPEC §5.3).
- MCP `run` ohne `async` blockiert (in der Brücke den UI-Thread) – in der Doku wird für viele Dateien `async: true` empfohlen.
- `--batch`: erstes Argument nach `--batch`, das auf `.fpbatch.json` endet oder keine vorhandene Datei ist, gilt als Rezept/Vorlage; alle vorhandenen Pfade danach als Dateien/Ordner. Unbekannter Vorlagenname → Studio öffnet mit leerem Rezept.

## Zeilen für PARITAET („umgesetzt, Nachprüfung ausstehend“)

Batch-Studio: Schritt `vector.trace` (Nachzeichnen → SVG/FW-PNG, beide Verfahren), Ausgabeformat SVG, eingebaute Vorlagen (11) im Menü „Rezepte ▸ Vorlagen“, MCP-Werkzeug `batch` + Ressource `batch.md`, CLI `batch --recipe <vorlage>`/`--templates`, `firepixel.exe --batch`.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes Fenster (Brücke): *Datei ▸ Batch-Verarbeitung…* → *Rezepte ▸ Vorlagen* zeigt 11 Vorlagen (de/en, nichts abgeschnitten); „Logos nachzeichnen → SVG“ wählen, 3 Logos ablegen → Schrittkarte „Nachzeichnen → SVG“ mit Vorgabe/Verfahren, Vorschau zeigt die Pfade, Größe „Nachher“; Start → SVGs in `batch-output`, im Browser/Firepixel öffnen: Pfade, keine Bitmap. Verfahren „Glatte Kurven“ → weniger Knoten.
- `firepixel.exe --batch logos-to-svg C:\logos` → Studio vorn mit Rezept und Dateien; `firepixel.exe --batch C:\r\mein.fpbatch.json a.png` → Rezeptdatei geladen (Titel ohne •).
- MCP (Claude Code an `firepixel-cli mcp`): `batch {"action":"templates"}`, `batch {"action":"preview", …}` zeigt zwei Bilder, `batch {"recipe":"web-webp","folder":…,"async":true}` → `status` bis fertig; `cancel` während eines großen Laufs (≥ 500 Fotos): keine `.fptmp`.
- Bildserie: Vorlagen-Menü und Karte „Nachzeichnen → SVG“ offen (Profi) in 3 Designs × de/en.

## Neue Befunde

- (für BATCH-1) Mehrere Ausgaben je Element (z. B. SVG **und** FW-PNG in einem Lauf): Executor-Haken für Zusatzdateien.
- (für BATCH-1/11) HTML-Bericht `batch-report.html` (P2, im Paket BATCH-9 der Spezifikation genannt) nicht umgesetzt: braucht `report.rs`/`executor.rs`.
- (für HG) `ui-cli-help` (ui.ftl) um `--batch [rezept] [dateien …]` ergänzen; Startseiten-Eintrag „Batch“ (P2).
- (für BATCH-9-Folge, nach BATCH-7/8 auf main) Vorlage „Video → kleines GIF“ (`video.import` 12 fps + `resize` 480 + GIF-Optimierer „ausgewogen“).
- (für BATCH-11) Karte „Nachzeichnen → SVG“: Profi-Felder je `engine` ein-/ausblenden (Konturverfahren-Werte nur bei `potrace`), Hinweis „SVG/FW-PNG in ‚Speichern als‘ wählen“, wenn das Ausgabeformat ein Pixelformat ist.
- (für Koordinator) Platte C: lief während BATCH-9 auf **0 GB** voll (Build brach mit „nicht genug Speicherplatz“ ab). Ich habe `target-s5\debug\incremental` (≈ 12 GB, eigener Slot) gelöscht; danach 20–60 GB frei. Weiter mit `CARGO_INCREMENTAL=0` gebaut.

## Neue Abhängigkeiten

Keine Fremd-Crates. `fp-mcp` hängt jetzt von `fp-batch` ab (eigenes Crate, Apache-2.0).
