# Firepixel: Architektur und Arbeitsregeln

Firepixel ist ein natives Grafikprogramm in Rust, das sich bedient wie das Original und dessen Dateien (`.fw.png`) liest und schreibt. Grundlage sind die Spezifikationen unter `..\re\` (02–16) mit ihren geprüften Python-Referenzen. Ergebnis ist **eine einzelne `firepixel.exe`** (C-Laufzeit statisch gelinkt, Assets eingebettet) plus `firepixel-cli.exe`.

Vorbild für den Aufbau: photocraft (egui/eframe, viele kleine Crates in klaren Schichten, CPU-Referenz-Compositor, ein Befehlsregister für UI/CLI/MCP).

## 1. Regeln

1. **Clean Room:** Implementiert wird nach den Spezifikationen und Python-Referenzen in `..\re\`, nicht durch Übersetzen von dekompiliertem Code. Die Ghidra-Exporte nur lesen, wenn eine Spezifikation eine Lücke hat, und die Lücke dann in eigenen Worten im Crate-Dokument notieren.
2. **Keine Assets des Originals:** keine Icons, Bilder, SWF-Panels, Bibliotheken, Muster, Texturen oder Farbbücher aus `..\korpus\` (Installationskopie des Originals) in den Code oder in die exe. Alle Icons zeichnen wir selbst (Vektor-Icons im Code, siehe §6). Menü- und Dialogbeschriftungen sind funktionale Begriffe und dürfen gleich lauten.
3. `..\re\`, `..\korpus\` und `..\tools\` sind **nur lesbar**. Testkorpus: die 710 FW-PNGs in `..\korpus\` (Verzeichnisverbindung auf die Installationskopie; Tests und Skripte auch über `FP_CORPUS_DIR`) und die Testdateien in `..\re\_testdateien\`.
4. Jeder Agent schreibt nur in die ihm zugeteilten Crates bzw. Dateien. Gemeinsame Typen ändert man nur in Absprache über den Auftrag.
5. Sprache: Bezeichner im Code englisch, Doku-Kommentare und Projektdokumente deutsch, UI-Texte deutsch (Original-Begriffe aus `..\re\inventory\resources_de.json`).
6. Python für Vergleiche immer mit vollem Pfad: `C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I …`. Ein bloßes `python` zeigt auf einen hängenden Store-Alias.
7. **Der Workspace muss jederzeit ladbar bleiben.** Ein neues Crate wird in einem Schritt angelegt: `Cargo.toml` und eine baubare `src/lib.rs` (notfalls nur ein Doku-Kommentar) zusammen. Nie ein `Cargo.toml` ohne Quelltext liegen lassen, nie eine nicht parsebare `Cargo.toml` speichern. Wer nachgezogen kommt, prüft vor größeren Umbauten mit `cargo metadata --format-version 1 > $null`.
8. Rechenlast moderat halten: `cargo build -j 4`, keine parallelen Volltests über den ganzen Korpus.

## 2. Workspace und Schichten

Abhängigkeiten zeigen nur nach unten. Jedes Crate liegt unter `crates/`, die Programme unter `apps/`.

```
apps/firepixel        GUI (eframe)           ─┐
apps/firepixel-cli    run/batch/render/info/mcp ┤
                                              │
fp-ui        egui-Oberfläche, Themes, Dialoge, Bedienfelder, Werkzeug-Interaktion
fp-mcp       MCP-Server (rmcp): stdio + Streamable HTTP auf 127.0.0.1 mit Token
fp-script    JSF-Kompatibilität (boa_engine), fw/dom-API → Befehle
fp-engine    Befehlsregister, Undo/Redo, Auswahl, Werkzeug-Zustandsautomaten, Voreinstellungen, Sitzungen
fp-export    Export: Paletten, Quantisierer, Dithering, GIF/JPEG/PNG8/24/32, Animation, Slices, HTML/Verhalten
fp-io        Import und moderne Formate: PSD, GIF, BMP/TIFF/TGA, WebP, APNG, AVIF, JXL, QOI, ICO, SVG, Zwischenablage
fp-render    Dokument-Compositor (CPU-Referenz): Seiten, Status, Ebenen, Masken, Symbole/9-Slice, Mischmodi
fp-text      Textmodell, Layout, Rasterisierung (cosmic-text/swash/rustybuzz)
fp-vector    Pfad-Rasterizer, Füllungen/Verläufe/Muster, Striche/Pinsel, Pfadoperationen (Boolesche Ops)
fp-effects   Live-Effekte (Schatten, Glühen, Abschrägung, Prägung, Farbfüllung, PS-Effekte)
fp-raster    Pixelpuffer (Kacheln), Bitmap-Filter, Mischen
fp-fwpng     .fw.png-Leser/-Schreiber (Tag-Baum, mkTS/mkBT/mkBF/prVW, Rundreise)
fp-model     Dokumentmodell (reine Daten, serde)
fp-color     Farben, Farbmodelle, CMYK exakt, Mischmodus-Formeln, Farbfelddateien (ACT/ASE/ACO)
fp-geom      Punkte, Rechtecke, Matrizen, Bézier, Abflachung
```

### Zuordnung Spezifikation → Crate

| Spezifikation | Crate(s) | Python-Referenz (Orakel) |
|---|---|---|
| 02 FW-PNG | fp-fwpng | `re\format\fwpng.py` (Rundreise 1.420/1.420) |
| 03 JS-API | fp-script, fp-engine (Befehlsnamen) | `re\api\fw_api.json` |
| 04 Objektmodell | fp-model | `re\objektmodell\fw_model_entwurf.rs` (kompiliert, 710/710) |
| 05 Live-Effekte | fp-effects | `re\effekte\` |
| 06 Bitmap-Filter | fp-raster | `re\filter\fwfilters.py`, `fweffects.py` |
| 07 Vektor-Rendering | fp-vector, fp-color (Mischmodi) | `re\vektor\` (1.687/1.719 Pfade) |
| 08 Export | fp-export | `re\export\fw_palette.py`, `fw_rgba.py` |
| 09 Text | fp-text | `re\text\layout.py` (96,4 % Rahmen) |
| 10 Werkzeuge/Befehle | fp-engine, fp-ui | `re\werkzeuge\menuebefehle.json`, `ctool_slots.json` |
| 11 Bedienfelder/Dialoge | fp-ui | `re\bedienfelder\dialoge.json`, `re\ui-referenz\*.png` |
| 12 Import | fp-io | `re\import\psd.py`, `psd_map.py`, `gif.py` |
| 13 Symbole | fp-render | `re\symbole\symrender.py` (648/651 Seiten), `nineslice.py` |
| 14 Farbe/Voreinstellungen | fp-color, fp-engine | `re\farbe\farbmodelle.py`, `farbfelddateien.py` |
| 15 Verhalten/Pop-up | fp-export | `re\verhalten\fwbehavior.py` (300/300) |
| 16 Pfadoperationen | fp-vector, fp-geom | `re\pfade\compound.py`, `flatten.py` |

## 3. Kernentscheidungen

- **Dokumentmodell** (fp-model): reine Daten ohne UI- und Render-Abhängigkeit. IDs sind stabile `u64`-Handles statt Zeiger. Original-Felder, die in der Datei stehen, bleiben erhalten, auch unbekannte Tags (Rundreise). Ausgangspunkt ist der Entwurf `re\objektmodell\fw_model_entwurf.rs`.
- **Rendering:** zuerst ein **CPU-Referenz-Compositor** in fp-render, bitgenau zu den Spezifikationen, geprüft gegen das flache PNG der Korpusdateien. Die Leinwand in der UI zeigt dessen Ergebnis als Textur. Ein GPU-Pfad ist eine spätere Optimierung und muss gegen die CPU-Referenz getestet werden.
- **Pixelpuffer:** RGBA8, das Original rechnet intern mit vormultipliziertem bzw. nicht vormultipliziertem Alpha je nach Stelle (siehe Spezifikationen), Kacheln 128×128 wie `mkBT`. Copy-on-Write-Kacheln für günstiges Undo.
- **Befehle** (fp-engine): **ein Register** für alles. Jeder Menüpunkt, jedes Werkzeug, jeder Dialog und jede Bedienfeld-Aktion ist ein `Command` mit Namen (`modify.group`, `filter.blur.gaussian`, …), JSON-Parametern (serde), `apply(&mut Document) -> Result<UndoStep>`, Beschreibung und Kürzel. UI, CLI, MCP und JSF rufen ausschließlich dieses Register auf. Namen und Kürzel folgen `re\werkzeuge\menuebefehle.json` bzw. den Tastenkürzel-Sets.
- **Undo:** Verlauf mit Schritten wie im Verlauf-Bedienfeld, maximale Anzahl aus Voreinstellungen (`MaxUndos`), auch Agent-Befehle landen im Verlauf.
- **Fehler:** `thiserror` in Bibliotheken, kein `unwrap`/`expect` auf Nutzerdaten, beschädigte Dateien liefern einen Fehler statt einer Panik.
- **Nebenläufigkeit:** `rayon` für Filter und Rendering großer Flächen. Die UI blockiert nie, lange Operationen laufen mit Fortschritt im Hintergrund.

## 4. Oberfläche (fp-ui)

- **Detailgetreu wie das Original** (Vorlage: `re\ui-referenz\*.png`, `re\11-bedienfelder-dialoge.md`, `re\10-werkzeuge-befehle.md`):
  - Menüleiste mit allen Menüs und Einträgen, gleiche Reihenfolge, Trenner und Kürzel
  - Hauptsymbolleiste
  - links das Werkzeugbedienfeld mit den Gruppen Auswählen/Bitmap/Vektor/Web/Farben/Ansicht und Flyouts
  - Dokument-Reiter mit Original/Vorschau/2-fach/4-fach
  - unten der Eigenschaften-Inspektor, je nach Auswahl ein anderes Layout nach der Auswahlregel aus 11
  - rechts andockbare Bedienfeldgruppen: Optimieren/Verlauf/Ausrichten, Seiten/Status/Ebenen, Stile/Farbpalette/Farbfelder, Dokument- und Allgemeine Bibliothek …
  - Statuszeile mit Status-Steuerung, Größe und Zoom
- Gleiche Tastenkürzel, gleiche Mausbedienung (Ziehen, Umschalt/Alt-Modifikatoren, Doppelklick-Verhalten).
- **Themes:** drei umschaltbare Designs, gespeichert in den Voreinstellungen (Menü Ansicht und Voreinstellungen):
  - `Normal`: Standard, das Hellgrau des Originals
  - `Dunkel`
  - `OLED`: reines Schwarz #000000, bewusst **farbenfroh**: kräftige Akzentfarbe je Werkzeuggruppe, bunte Icons, farbige Reiter-Kennungen, leuchtende Auswahl und Hover, farbige Fokus-Ränder
  - Alle Farben kommen aus einer zentralen `Theme`-Struktur, kein fest eingebauter Farbwert in Widgets.
- Icons: eigene Vektor-Icons (im Code gezeichnet oder als eingebettetes eigenes SVG), in Größe und Lage wie die Originale, damit man sich sofort zurechtfindet.
- Schrift: Segoe UI vom System (zur Laufzeit geladen), Rückfall auf die egui-Standardschrift.

## 5. Formate (fp-io / fp-export)

- Original-Formate: `.fw.png` lesen und schreiben; Import von PSD, AI/EPS (soweit spezifiziert), GIF, JPEG, PNG, BMP, TIFF, TGA, WBMP; Export nach 08 (GIF, JPEG, PNG8/24/32, animiertes GIF, Slices + HTML/CSS, PSD).
- Zusätzlich: WebP (verlustbehaftet und verlustfrei, animiert) und APNG für Import und Export, auch im Optimieren-Bedienfeld; AVIF (Export mit ravif, Import), JPEG XL (Import mit jxl-oxide), HEIC/HEIF (Import, wenn statisch machbar), QOI, ICO (mehrere Größen), SVG-Import.
- Nur reine Rust-Crates oder statisch gelinkte C-Bibliotheken, keine DLLs neben der exe.

## 6. Automatisierung

- `firepixel-cli`:
  - `run <datei> --cmd <name> --params <json> --out <datei>`
  - `batch --actions <json> --in <ordner> --out <ordner>`
  - `render <datei> --page n --out x.png`
  - `info <datei>` (Dokumentbaum als JSON)
  - `commands` (Katalog)
  - `mcp` (stdio-MCP-Server, ohne Oberfläche oder als Brücke zur laufenden App)
- GUI: Steuerkanal nur auf 127.0.0.1 mit Token, Streamable-HTTP-MCP. Ein Agent kann damit Dokumentbaum lesen, Befehle ausführen, Werkzeug-Mausereignisse senden, Renderings und Screenshots als Bild holen und exportieren.
- MCP-Werkzeuge bewusst schlank: `list_commands`/`describe_command`, `run_command`, `get_document`, `select`, `render`, `export`, `open`/`save`, `undo`/`redo`. Dazu die eingebaute Doku als MCP-Ressource.
- JSF-Kompatibilität: `.jsf`-Skripte laufen über boa_engine. `fw`/`dom` bilden auf das Befehlsregister ab, Signaturen und Typregeln nach `re\03-js-api.md` inklusive §10.11.

## 7. Tests und Messung

- Jedes Crate hat Einheitstests und, wo es eine Referenz gibt, **Korpus- bzw. Golden-Tests**: Ausgabe der Rust-Implementierung gegen die Python-Referenz bzw. gegen das flache PNG der `.fw.png`. Ergebnis als Zahl, zum Beispiel „Pfade pixelgenau 1.687/1.719“.
- Korpustests sind mit `#[ignore]` markiert und laufen mit `cargo test -p <crate> -- --ignored`. So bleibt `cargo test` schnell.
- Stand und Zahlen jedes Crates trägt der zuständige Agent in `STATUS.md` ein, je Crate ein Abschnitt.
- Fenster-Screenshots von Firepixel nur über die Sperre (`..\re\scripts\fwsteuerung\gui-sitzung.ps1`), weil parallel das Original ferngesteuert wird. Für UI-Tests bevorzugt `egui_kittest` bzw. Offscreen-Rendering.

## 8. Phasen (höchstens 3 Agenten gleichzeitig)

| Phase | Agent 1 | Agent 2 | Agent 3 |
|---|---|---|---|
| 1 Fundament | fp-geom + fp-color | fp-model + fp-fwpng | fp-raster (Puffer, Filter) |
| 2 Darstellung | fp-vector (inkl. Pfadoperationen) | fp-text | fp-effects + fp-render |
| 3 Programmkern | fp-engine (Befehle, Undo, Werkzeuge, Voreinstellungen) | fp-io + fp-export | fp-ui Grundgerüst (Fenster, Menüs, Docking, Themes, Leinwand) |
| 4 Oberfläche und Automatisierung | fp-ui Inspektor + Bedienfelder + Dialoge | fp-ui Werkzeug-Interaktion auf der Leinwand | fp-script + fp-mcp + firepixel-cli |
| 5 Abschluss | Integration und Fehlerbehebung | Leistung (Kacheln, Caches, ggf. GPU) | Release-Build, Gesamttest, Doku |

Jede Phase endet mit `cargo build --release` und `cargo test` ohne Fehler und Warnungen.
