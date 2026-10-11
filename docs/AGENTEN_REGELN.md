# Phase 5: Regeln für die 6 parallelen Agenten (ab 11.10.2026)

Auftrag des Nutzers: **„Mach weiter, was fehlt. Jede Funktion in Firepixel muss auch klappen und funktionieren.“**
Dazu ein schönes, passendes **eigenes App-Icon**.

Vorher lesen: `..\UEBERGABE.md`, `ARCHITEKTUR.md` (§1 Regeln gelten weiter: Clean Room, keine Assets des Originals, `re\` nur lesen), den eigenen Abschnitt in `STATUS.md`.

## 1. Wer schreibt wo (strikt)

| Agent | Kürzel | darf schreiben |
|---|---|---|
| Leinwand-Werkzeuge | **LW** | `crates\fp-ui\src\canvas\*`, `crates\fp-ui\src\input.rs`, `crates\fp-engine\src\tools\*`, `crates\fp-engine\src\commands\tooling.rs`, `crates\fp-engine\tests\canvas_tools.rs` |
| Inspektor/Bedienfelder/Dialoge | **IBD** | `crates\fp-ui\src\inspector\*`, `panels\*`, `dialogs\*`, `ui_commands.rs`, `crates\fp-ui\tests\inspector_*.rs`, `panels_*.rs`, `dialogs_*.rs`; neu in fp-engine: `src\commands\panel.rs` (für `pi.*`-Befehle und Bedienfeldmenü-Befehle mit `fw_id`) |
| Engine-Befehle + Export-Anbindung | **EB** | `crates\fp-engine\src\` außer `tools\*`, `commands\tooling.rs`, `commands\panel.rs`; `crates\fp-engine\data\*`; `crates\fp-engine\tests\` außer `canvas_tools.rs`; `crates\fp-export\src\render.rs` (+ neue Dateien in fp-export nur für die Renderer-Anbindung) |
| Automatisierung | **AU** | `crates\fp-script`, `crates\fp-mcp`, `apps\firepixel-cli`, `docs\AGENTS.md`; in fp-ui nur die neue Datei `src\bridge.rs` |
| Rendertreue + Formate | **RF** | `crates\fp-vector`, `fp-text`, `fp-effects`, `fp-render`, `fp-io`, `fp-fwpng`, `fp-raster`, `fp-color`, `fp-model`, `fp-geom`; `crates\fp-export` außer `src\render.rs` |
| Hauptfenster, Icon, Gesamttest, Release | **HG** | `crates\fp-ui\src\` Shell-Dateien: `lib.rs`, `app_state.rs`, `actions.rs`, `menu.rs`, `appbar.rs`, `toolbar.rs`, `tools_panel.rs`, `docking.rs`, `workspace.rs`, `files.rs`, `theme.rs`, `icons.rs`, `widgets.rs`, `crates\fp-ui\data\*`, `crates\fp-ui\tests\layout.rs`, `apps\firepixel\*`, `assets\` (neu), `docs\FUNKTIONSTEST.md`, `docs\ANLEITUNG.md`, Workspace-`Cargo.toml` (nur `[workspace.dependencies]`/Profile, Bescheid in BEFUNDE) |

- Braucht ein Agent eine Änderung in fremden Dateien (z. B. ein Haken in `app_state.rs`, ein neuer Befehl in fp-engine, eine öffentliche Funktion in fp-render), trägt er sie als Befund in `docs\BEFUNDE.md` ein (siehe §3). **Nicht selbst in fremden Dateien ändern.** Ausnahme: eine einzelne `pub mod …;`-/`mod`-Zeile bzw. eine Registrierungszeile, um die eigene neue Datei einzuhängen – dann im Befund vermerken.
- `pub`-APIs anderer Crates nur rückwärtskompatibel ändern.
- Neue Abhängigkeiten nur in das eigene Crate-`Cargo.toml`. Nur reine Rust-Crates oder statisch gelinkt (einzelne exe, keine DLL), kein `unsafe` im eigenen Code.

## 1a. Neutralität (Nutzerwunsch, verbindlich)

Nirgends unter `firepixel\` darf der **Name des Originals oder seines Herstellers** stehen (die beiden Wörter sind allen Agenten bekannt; die Prüfung `crates\fp-neutral` kennt sie nur als Hash). Das gilt für UI-Texte, Quellcode, Kommentare, Doku-Kommentare, Datendateien, Testnamen, Fehlermeldungen, Cargo-Metadaten und Dokumente, in jeder Schreibweise (klein, groß, in Bezeichnern, Pfaden und Dateinamen). Stattdessen:
- das Programm, das wir nachbauen → „das Original“ bzw. „Referenzprogramm“ (Kommentare/Doku), in der UI „Firepixel“
- Dateiformat → „FW-PNG“; die Endung `.fw.png` bleibt
- die JSF-Objekte `fw`/`dom` bleiben (API-Kompatibilität)

Spezifikationen werden als `re\<Nr>-….md` zitiert, ohne Produktnamen. Die Prüfung `cargo test -p fp-neutral` (Crate `crates\fp-neutral`, meldet Datei:Zeile) muss grün bleiben. Agent **NT** setzt das durch und darf dafür in allen Dateien kleine gezielte Text-Edits machen.

Vom Nutzer genehmigte Ausnahme (11.10.): Treffer in der fertigen exe, die nur aus Fremd-Crates stammen (zune-jpeg, image, brotli – Formatkennungen), dürfen bleiben. Eigener Code, eigene Daten und die UI müssen sauber sein.

## 2. Bauen ohne sich gegenseitig zu stören

- **Eigener Build-Ordner je Agent** (seit dem Neustart: C: 499 GB, ~329 GB frei; 24 GB RAM, 24 Kerne): in jeder Shell vor cargo `$env:CARGO_TARGET_DIR = 'C:\Users\Administrator\Desktop\REA\firepixel\target-<kürzel>'` (z. B. `target-lw`). Dann blockiert niemand mehr die anderen. Kein `cargo clean` in fremden Ordnern. Ausnahme: HG baut die Release-exe am Ende im normalen `target\`.
- Immer `cargo … -j 4` (6 Agenten × 4 = 24 Kerne). Keine parallelen Korpus-Volltests; `--ignored`-Korpustests nur für das eigene Crate.
- „Blocking waiting for file lock“ ist normal: warten.
- **Code muss bei jedem Speichern kompilieren** (kleine, in sich geschlossene Schritte). Bricht der Build wegen der Datei eines anderen Agenten: 5–10 min warten und erneut versuchen, nicht in dessen Datei eingreifen; bleibt es > 20 min kaputt, Befund eintragen.
- Python immer mit vollem Pfad `C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I …`.
- Cargo-PATH: `$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')`

## 3. Befunde (`docs\BEFUNDE.md`)

Ein Eintrag je Zeile unter der Überschrift des **zuständigen** Agenten:
`- [ ] (von XY) Kurzbeschreibung – Datei/Befehl – erwartetes Verhalten`
Erledigt: `- [x] … – erledigt (Kürzel, Kurznotiz)`.
**Nach jedem größeren Schritt `docs\BEFUNDE.md` lesen** und eigene offene Einträge abarbeiten. Datei nur mit kleinen Edit-Operationen ändern (nie komplett überschreiben), weil mehrere gleichzeitig schreiben.

## 4. „Funktioniert“ heißt

1. Erreichbar wie im Original (Menü, Kürzel, Werkzeug, Bedienfeld, Inspektor, Kontextmenü).
2. Tut das Richtige am Dokument (Test prüft das Ergebnis, nicht nur „kein Fehler“), landet im Verlauf, Rückgängig/Wiederholen stellt exakt wieder her.
3. Keine Panik, keine „noch nicht umgesetzt“-Meldung. Wo etwas wirklich nicht machbar ist: begründet in STATUS.md und BEFUNDE.md.
4. Wo es eine Referenz gibt (Spezifikation, Python, das echte Original über `re\scripts\fwsteuerung\`), Ergebnis dagegen geprüft.

## 4b. Logikfehler gezielt testen (Nutzerbefund 11.10.)

Der Nutzer fand: Glühen-Effekt, Größe mehrmals geändert → 50 Glühen auf dem Element. „Ein Mal ausführen klappt“ reicht nicht. Jede interaktive Funktion braucht zusätzlich Tests für:
1. **Wiederholung/Idempotenz:** Wert 50× ändern, Schieber ziehen, Feld tippen (jede Taste ein Ereignis) → Ergebnis wie einmal mit dem Endwert; keine Duplikate (Effekte, Ebenen, Hilfslinien, Farbfelder, Stile, Symbole, Seiten …).
2. **Vorschau/Abbrechen:** Live-Vorschau ersetzt, statt zu addieren; Abbrechen stellt den Ausgangszustand exakt her; OK = genau ein Verlaufsschritt.
3. **Rückgängig/Wiederholen** nach jeder Aktion und in Folgen (A, B, Rückgängig, C, Wiederholen …).
4. **Auswahlwechsel mitten in der Bearbeitung** (Dialog/Inspektor wirkt auf das richtige Objekt, nicht auf das vorher gewählte).
5. **Mehrfachauswahl, leere Auswahl, gesperrte/ausgeblendete Objekte, Gruppen, Symbole, mehrere Seiten/Status.**
Solche Fehler gehören zuerst behoben, vor neuen Funktionen. HG prüft diese Fälle in der Funktionsliste mit.

## 4c. UI/UX auf Herz und Nieren (Nutzerwunsch 11.10.)

Alles muss bei jeder Größe stimmig aussehen. Jede Oberfläche (Hauptfenster, Bedienfelder, Inspektor, Dialoge, Menüs, Leinwand-Overlays) wird in egui_kittest geprüft:
- Fenstergrößen: 800×600, 1024×768, 1366×768, 1920×1080, 2560×1440, 3840×2160, sehr schmal/hoch; DPI-Skalierung 100/125/150/200 %.
- Bedienfelder und Docks schmal/breit ziehen, einklappen, lösen, schwebend sehr klein/groß, wieder andocken; Inspektor ein-/ausklappen; Dialoge (wo größenveränderlich) klein/groß.
- Alle 3 Designs bei jeder Größe.
- Lange Namen/Texte (Ebenen, Seiten, Dateien, Schriftnamen), viele Ebenen/Seiten/Status/Farbfelder (Bildlauf), leere Zustände.
- Prüfen: nichts überlappt, nichts wird abgeschnitten ohne Bildlauf/„…“, nichts springt oder flackert beim Ziehen, Mindestgrößen greifen, Abstände/Ausrichtung/Schriften/Farben/Hover-/Fokus-Zustände einheitlich. Automatisierbar: Rechtecke der Widgets gegen Überlappung und Fensterrand prüfen, Bildvergleich gegen Referenz-Screenshots.
**Lehre aus Nutzerbefunden (11.10. abends):** Trotz grüner Tests fand der Nutzer sofort, dass Strg+Mausrad direkt auf 6 %/6400 % springt und Dialoge sich nicht verschieben lassen. Deshalb: Tests müssen **realistische Eingaben** verwenden (echte Mausrad-Deltas Line/Point 120/Touchpad-Kleinschritte/Smooth-Scroll über mehrere Frames, Ziehen über viele Frames, Doppelklick-Timing, Tastenwiederholung) und prüfen, was ein Mensch **erwarten** würde (eine Rastung = eine Stufe; Fenster an der Titelleiste verschiebbar), nicht nur, dass der Code ohne Fehler läuft.
HG verantwortet die Gesamtprüfung und verteilt Befunde; jeder Agent prüft seine eigenen Oberflächenteile mit.

## 4d. Echte Eingaben nur ins richtige Fenster (Vorfall 11.10., 20:40)

Tastendrücke eines Agenten (Alt+Buchstabe, Esc) sind im Claude-Fenster des Nutzers gelandet, weil Windows das Zielfenster nicht nach vorn geholt hatte. Daher gilt: Vor **jeder einzelnen** echten Eingabe (Taste, Klick, Ziehen) per `GetForegroundWindow` prüfen, dass Prozess-ID **und** Fensterklasse die des Originals bzw. der eigenen Firepixel-Testinstanz sind; sonst nichts senden und abbrechen. Wo möglich `PostMessage` an das Zielfenster bzw. für Firepixel die Eingabe-Einspeisung der MCP-Brücke verwenden. Nie Prozesse beenden, ohne dass der Nutzer zustimmt.

## 4a. Keine fremden Programme/Fenster aus Tests

Am 11.10. hat ein Test ~80 „Pick an app“-Fenster beim Nutzer geöffnet (E-Mail-Befehl → `explorer.exe x.eml`). Darum:
- Befehle, die etwas extern öffnen (Browser, E-Mail, Explorer, Drucken, Hilfe, Datei-Zuordnungen, `start`, ShellExecute), öffnen **nur**, wenn die Umgebungsvariable `FP_NO_SHELL_OPEN` **nicht** gesetzt ist. EB baut die zentrale Prüfung in fp-engine ein (eine Funktion, die alle solchen Stellen benutzen).
- Jeder Testlauf (cargo test, kittest, CLI-/MCP-Tests, GUI-Sitzungen) setzt vorher `$env:FP_NO_SHELL_OPEN = '1'`. Tests prüfen nur, dass die Datei erzeugt wurde.
- Nie in einer Schleife Programme starten, ohne die Prozesse am Ende zu beenden.

## 5. Oberfläche testen

- Bevorzugt `egui_kittest` (offscreen, parallel ungefährlich).
- Echte Fenster (Firepixel oder Original) nur über die systemweite Sperre `..\re\scripts\fwsteuerung\gui-sitzung.ps1` bzw. `jsf-ausfuehren.ps1` – Bildschirm und Maus sind gemeinsam. Sitzungen kurz halten (< 5 min), danach Fenster schließen.
- Nie Dokumente des Nutzers im Original anfassen oder speichern.
- Screenshots nach `docs\screenshots\phase5_<kürzel>_*.png`.

## 6. Abschluss je Agent

`STATUS.md`-Abschnitt aktualisieren (Stand, Messzahlen, was geprüft ist, was offen bleibt mit Grund). Vorher: eigenes Crate/eigene Dateien `cargo build -j 3` und `cargo test -j 3 -p <crate>` grün, `cargo clippy -j 3 -p <crate>` ohne Warnungen in eigenen Dateien.
