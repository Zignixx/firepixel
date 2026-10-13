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
| Hauptfenster, Icon, Gesamttest, Release | **HG** | `crates\fp-ui\src\` Shell-Dateien: `lib.rs`, `app_state.rs`, `actions.rs`, `menu.rs`, `appbar.rs`, `toolbar.rs`, `tools_panel.rs`, `docking.rs`, `workspace.rs`, `files.rs`, `theme.rs`, `icons.rs`, `widgets.rs`, `crates\fp-ui\data\*`, `crates\fp-ui\tests\layout.rs`, `apps\firepixel\*`, `assets\` (neu), `docs\FUNKTIONSTEST.md`, `docs\MANUAL.md`, `docs\MANUAL.en.md`, Workspace-`Cargo.toml` (nur `[workspace.dependencies]`/Profile, Bescheid in BEFUNDE) |

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

## 1c. Technisches immer Englisch (Nutzerwunsch 12.10.2026, verbindlich)

Übersetzt wird nur, was der Nutzer in der Oberfläche liest. **Alles Technische ist immer Englisch**, unabhängig von der UI-Sprache:
- Ordner und Dateien, die die App anlegt oder liest: `%APPDATA%\Firepixel\…`, `%LOCALAPPDATA%\Firepixel\…`, neben der EXE, `~/.config/firepixel/…`. Beispiele: `Sprachen` → `Languages`, `Modelle` → `Models`.
- Namen und Schlüssel der Einstellungsdateien, Vorgabe-, Export- und Protokolldateien
- CLI-Befehle, -Optionen und JSON-Ausgabeschlüssel, MCP-Werkzeug- und Parameternamen, Umgebungsvariablen
- `.ftl`-Dateinamen, Fluent-IDs, Rust-Bezeichner, Datei- und Testnamen im Quelltext

Neue Namen nur noch englisch. Beim Umbenennen bestehender Pfade wird migriert: den alten deutschen Ordner als Rückfall lesen oder einmalig verschieben, damit Sprachpakete, KI-Modelle und Einstellungen des Nutzers erhalten bleiben. Kommentare und deutsche Doku sind davon nicht betroffen.
## 2. Bauen ohne sich gegenseitig zu stören

- **Eigener Build-Ordner je Agent** (seit dem Neustart: C: 499 GB, ~329 GB frei; 24 GB RAM, 24 Kerne): in jeder Shell vor cargo `$env:CARGO_TARGET_DIR = 'C:\Users\Administrator\Desktop\REA\firepixel\target-<kürzel>'` (z. B. `target-lw`). Dann blockiert niemand mehr die anderen. Kein `cargo clean` in fremden Ordnern. Ausnahme: HG baut die Release-exe am Ende im normalen `target\`.
- Immer `cargo … -j 4` (6 Agenten × 4 = 24 Kerne).
- **Platz (12.10.: C: lief voll, alle Agenten standen):** je Build-Ordner höchstens ein Profil bauen (debug für Tests; release nur wenn wirklich nötig, danach `target-<kürzel>\release` löschen); `$env:CARGO_INCREMENTAL = '0'` setzen; wenn du fertig bist, deinen eigenen `target-<kürzel>` löschen (nur deinen!). Vor großen Builds freien Platz prüfen (`(Get-PSDrive C).Free`), unter 30 GB dem Koordinator melden. Keine parallelen Korpus-Volltests; `--ignored`-Korpustests nur für das eigene Crate.
- **Build-Cache sccache: abgeschaltet** (12.10. ≈ 17:45). INF-1 hat 0 % Treffer gemessen (jeder Agent baut in einem anderen Worktree-Pfad), und die Rechteprüfung lehnt `RUSTC_WRAPPER` teilweise ab. **Kein `RUSTC_WRAPPER` mehr setzen.** Schneller wird es über rust-lld und das gebündelte Testprogramm (INF-1).
- **Linker rust-lld (seit `843de17`, INF-1):** `.cargo\config.toml` setzt für `x86_64-pc-windows-msvc` `linker = "rust-lld"` (lld-link aus der Rust-Toolchain, neben `+crt-static`). Nichts zu tun; die exe braucht weiter nur System-DLLs. Wer von einem älteren Stand kommt, übersetzt beim ersten Build einmal alles neu.
- **Ein Testprogramm je Crate (fp-ui, fp-engine; seit `b656ad5`, INF-1):** Alle `tests\*.rs` eines Crates sind Module **eines** Testprogramms `all` (`tests\all.rs`, Liste aus `crates\fp-testlist`). Das Crate wird einmal gelinkt statt einmal je Datei.
  - **Neue Testdatei:** einfach unter `tests\` anlegen, sie wird automatisch eingebunden. Keine Liste pflegen, nichts in `Cargo.toml` eintragen.
  - **Aufruf:** `cargo test -p fp-ui --test all` (alles), `cargo test -p fp-ui --test all inspector_fv10::` (eine frühere Datei), `cargo test -p fp-ui --test all inspector_fv10::name_des_tests` (ein Test). `--test <datei>` gibt es nicht mehr, außer für die Einzelprozess-Tests unten. Ignorierte Läufe wie bisher, z. B. `cargo test -p fp-ui --test all i18n_qa_de:: -- --ignored`. Gleiches für fp-engine (`cargo test -p fp-engine --test all undo::`).
  - **Hilfsmodule** (`tests\gesamt_common\`, `tests\i18n_qa_common\`, `tests\common\`) werden einmal gebaut: in der Testdatei `use crate::gesamt_common;` statt `mod gesamt_common;`. Ein `mod …;` baut eine zweite Kopie mit eigenen statischen Variablen und eigenen Temp-Ordnern; der Build warnt dann.
  - **Sprache je Thread:** Im Testprogramm `all` wirkt `fp_i18n::set_language` nur im eigenen Test-Thread; jeder Test beginnt mit Deutsch. Tests, die auf Englisch/qps umschalten, stören die anderen so nicht; ein Sprach-Mutex ist dafür nicht mehr nötig. Texte, die ein **Hintergrund-Thread** erzeugt, kommen dort in der Ausgangssprache.
  - **Eigener Prozess** (eigenes `[[test]]` mit `path` und Grund im `Cargo.toml`) nur, wenn ein Test **prozessweiten** Zustand umstellt, den die parallel laufenden Tests anderer Dateien sehen würden: Umgebungsvariablen, KI-Modellsuche/-Zwischenspeicher (`fp_ai`), Testmodus der Datei-Dialoge, die MCP-Brücke (`ui.mcpToggle`), Panik-Hooks, Kindprozesse mit festen Ports, Zeitmessungen. Sonst gehört der Test ins gemeinsame Programm. Heute eigener Prozess:
    - fp-ui: `ux_b_abgleich` (eigene wgpu-Renderer je Test; seit NZ-2 `1a56ce8` mit Mutex in `shell_off()` auch parallel grün), `ai_kittest_missing` und `ai_kittest_dialogs` (KI-Modellsuche umgestellt), `ai_kittest` und `ai_kittest_klickfolgen` (echte KI-Rechnung mit prozessweitem Zwischenspeicher), `ai_progress_ki2` (Zeitverhalten der KI-Karte), `dialogs_i18n` (öffnet jeden Dialog aller Befehle, hängt im gemeinsamen Programm), `file_dialogs_dlg3` (Testmodus der Datei-Dialoge)
    - fp-engine: `ai_missing` (KI-Modellsuche umgestellt)
  - Andere Crates (fp-render, fp-script, fp-text …) bleiben bei einem Programm je Datei: dort linkt jedes nur einen kleinen Teil des Baums. Dazunehmen: Feature in `crates\fp-testlist\Cargo.toml`, `autotests = false`, `[[test]] name = "all"` und `tests\all.rs` wie bei fp-engine.
- „Blocking waiting for file lock“ ist normal: warten.
- **Code muss bei jedem Speichern kompilieren** (kleine, in sich geschlossene Schritte). Bricht der Build wegen der Datei eines anderen Agenten: 5–10 min warten und erneut versuchen, nicht in dessen Datei eingreifen; bleibt es > 20 min kaputt, Befund eintragen.
- Python immer mit vollem Pfad `C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe -I …`.
- Cargo-PATH: `$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')`

## 1b. Phase 6: Mehrsprachigkeit (ab 12.10.2026)

Nutzerwunsch: Firepixel mehrsprachig – Deutsch (bisher) + Englisch, neue Sprachen später sehr einfach. Kern: `crates\fp-i18n` (Fluent; API `fp_i18n::t`, `fp_i18n::tr!`, `command_key`, `set_language`, `generation`; Regeln und Dateiaufteilung in `crates\fp-i18n\README.md`, Begriffe in `crates\fp-i18n\GLOSSAR.md`).
- **Kein fester Nutzertext mehr im Code.** Jeder sichtbare Text → Schlüssel in `locales\de\<bereich>.ftl` **und** `locales\en\<bereich>.ftl` (gleichzeitig; Englisch nach Glossar). Ausgenommen: Eigennamen, Formatnamen, Einheitenkürzel, Symbole.
- Bereichsdateien gehören je einem Agenten (Tabelle im README). Braucht ein Agent einen Text aus einem fremden Bereich: Schlüssel dort nicht selbst anlegen, sondern Befund – außer `btn-*`/`key-*`/`unit-*` aus `common.ftl`, die I18N-KERN pflegt (fehlende dort als Befund).
- Code-Dateien: Zuständigkeit wie in der Agentenliste der Phase 6 (Auftrag). Einzeilige Einhängungen in fremde Dateien nur mit Befund.
- Nach jeder umgestellten Datei: bauen, eigene Tests, `cargo test -p fp-i18n` (Vollständigkeit) und `cargo test -p fp-neutral`, dann committen (§2a).
- Die Oberfläche muss in Deutsch **unverändert** aussehen (gleicher Wortlaut wie bisher, bestehende Tests mit deutschen Texten bleiben grün), in Englisch und in der Testsprache `qps` darf nichts überlaufen (§4c).

## 2a. Versionsverwaltung (git, seit 12.10.2026)

`firepixel\` ist ein lokales git-Repository (Branch `main`, kein Remote, nichts hochladen). Git liegt unter `C:\Program Files\Git` (PATH wie in §2 nachladen).
- Nach jedem fertigen, baubaren Arbeitsschritt **nur die eigenen Dateien** committen: `git add <eigene Pfade>` und `git commit -m "<Kürzel>: <was>"` mit der Zeile `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` am Ende. Nie `git add -A` über fremde Bereiche, nie `git reset --hard`, `git checkout -- .`, `git clean`, Rebase oder Force – andere Agenten arbeiten gleichzeitig.
- **Gemeinsamer Index (Vorfall 12.10. ≈ 18:15):** Alle teilen sich einen git-Index, und vorgemerkte Dateien anderer würden sonst mitcommittet. Deshalb immer **mit Pfadangabe committen**: `git commit -m "…" -- <eigene Pfade>`. Kein blankes `git commit`, und nicht erst stagen und dann später committen.
- Meldet git `index.lock`, kurz warten und erneut versuchen.
- Versehentlich zerstörte Datei: `git log -- <datei>` und `git show <commit>:<pfad>` zum Wiederherstellen; im Bericht erwähnen.
- Build-Ordner und `tools\golden\*\out` sind per `.gitignore` ausgeschlossen.

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

Tastendrücke eines Agenten (Alt+Buchstabe, Esc) sind im Claude-Fenster des Nutzers gelandet, weil Windows das Zielfenster nicht nach vorn geholt hatte. Daher gilt: Vor **jeder einzelnen** echten Eingabe (Taste, Klick, Ziehen) per `GetForegroundWindow` prüfen, dass Prozess-ID **und** Fensterklasse die des Originals bzw. der eigenen Firepixel-Testinstanz sind; sonst nichts senden und abbrechen. Wo möglich `PostMessage` an das Zielfenster bzw. für Firepixel die Eingabe-Einspeisung der MCP-Brücke verwenden. Nie Prozesse beenden, ohne dass der Nutzer zustimmt. **Keine nativen Datei-Dialoge offen lassen** (Speichern unter, Öffnen, Importieren, Exportieren, HTML aktualisieren …): nur auslösen, wenn genau dieser Dialog geprüft wird, und im selben Schritt schließen; vor jeder Sitzung auf offene `#32770`-Fenster prüfen (Nutzerhinweis 12.10.). **Strg+Alt+Leertaste nie senden** (globale Taste der Claude-App). Vor jeder GUI-Sitzung das eigene Testdokument aktivieren (mehrere Agenten teilen sich das Original).

## 4e. Firepixel über die Brücke prüfen, Sperre nur fürs Original (Nutzerentscheidung 12.10. ≈ 17:00)

Damit die GUI-Sperre nicht alle ausbremst:
- **Firepixel-Seite ohne Sperre, parallel:** eigene Firepixel-Instanz mit eigenem APPDATA/LOCALAPPDATA und `--mcp` (siehe `re\_testdateien\ux\uxa\werkzeug\fpmcp.ps1`), bedient über die Brücke mit `ui_find`, `ui_input`, `screenshot` und `ui_state`. `ui_input` läuft über denselben Eingabeweg wie echte Maus und Tastatur (siehe `docs\AGENTS.md` §5a). Das gilt als **Prüfung im echten Fenster**. Ins Protokoll schreiben: „echtes Fenster, Brücke“.
- **Die Sperre `gui-sitzung.ps1` brauchen nur:**
  1. alles, was das **Original** bedient (JSF-Läufe, Klicks, Bildschirmfotos)
  2. Prüfungen mit **echter Systemmaus/-tastatur** (SetCursorPos/SendInput). Pflicht ist das nur, wo die Brücke das Verhalten nicht abbildet: Zeitverhalten und Verzögerung, Fensterverschieben bzw. -größe durch das Betriebssystem, Fokus und Taskleiste, Drag & Drop aus dem Explorer, Doppelklick-Zeiten, Mausrad-Hardware, IME, Startverhalten des Fensters.
  3. das abschließende Durchklicken (PLAN §5 Z), das bleibt mit echter Maus.
- **Die Brücke antwortet nur, solange das Fenster sichtbar ist** (BV-1, 12.10.): Ist das Fenster verdeckt, minimiert oder außerhalb des Bildschirms, zeichnet egui nicht. Deshalb:
  - Die Brücken-Instanz läuft **sichtbar, aber nicht aktiviert**, in einer freien Bildschirmecke, z. B. 900 × 650 unten rechts. Sie liegt nicht über dem Original und nicht maximiert im Vordergrund.
  - Sie nimmt nicht den Fokus, also kein SetForegroundWindow.
  - Laufen mehrere Brücken-Instanzen gleichzeitig, bekommt jede eine andere Ecke bzw. Lage.
  - Echte Eingaben anderer Agenten dürfen nie in eine Brücken-Instanz gehen (§4d).
- Instanzen am Ende immer beenden.

## 4a. Keine fremden Programme/Fenster aus Tests

Am 11.10. hat ein Test ~80 „Pick an app“-Fenster beim Nutzer geöffnet (E-Mail-Befehl → `explorer.exe x.eml`). Darum:
- Befehle, die etwas extern öffnen (Browser, E-Mail, Explorer, Drucken, Hilfe, Datei-Zuordnungen, `start`, ShellExecute), öffnen **nur**, wenn die Umgebungsvariable `FP_NO_SHELL_OPEN` **nicht** gesetzt ist. EB baut die zentrale Prüfung in fp-engine ein (eine Funktion, die alle solchen Stellen benutzen).
- Jeder Testlauf (cargo test, kittest, CLI-/MCP-Tests, GUI-Sitzungen) setzt vorher `$env:FP_NO_SHELL_OPEN = '1'`. Tests prüfen nur, dass die Datei erzeugt wurde.
- Nie in einer Schleife Programme starten, ohne die Prozesse am Ende zu beenden.
- **Tests nie ins echte Profil** (Wächter `fp_i18n::profile`, EB-3, 12.10.): Testläufe (cargo-Testbinaries und -Beispiele, `FP_TEST_PROFILE`, `FP_NO_SHELL_OPEN`) bekommen automatisch ein eigenes Testprofil `%TEMP%\firepixel-test-profiles\<pid>`; jeder Schreibzugriff unter dem echten `%APPDATA%\Firepixel` bzw. `%LOCALAPPDATA%\Firepixel` endet mit einer Panik (KI-Modelle bleiben lesbar). Profilpfade nur über `fp_i18n::profile::roaming_dir()`/`local_dir()`, gestartete Programme aus Tests mit `fp_i18n::profile::child_env(&mut cmd)`; geprüft von `cargo test -p fp-neutral --test no_real_profile` und `-p fp-engine --test no_real_profile`.

## 5. Oberfläche testen

- Bevorzugt `egui_kittest` (offscreen, parallel ungefährlich).
- Echte Fenster (Firepixel oder Original) nur über die systemweite Sperre `..\re\scripts\fwsteuerung\gui-sitzung.ps1` bzw. `jsf-ausfuehren.ps1` – Bildschirm und Maus sind gemeinsam. Sitzungen kurz halten (< 5 min), danach Fenster schließen.
- Nie Dokumente des Nutzers im Original anfassen oder speichern.
- Screenshots nach `docs\screenshots\phase5_<kürzel>_*.png`.

## 6. Abschluss je Agent

`STATUS.md`-Abschnitt aktualisieren (Stand, Messzahlen, was geprüft ist, was offen bleibt mit Grund). Vorher: eigenes Crate/eigene Dateien `cargo build -j 3` und `cargo test -j 3 -p <crate>` grün, `cargo clippy -j 3 -p <crate>` ohne Warnungen in eigenen Dateien.

## 7b. Schneller arbeiten (ab Welle 2, 12.10.2026 ≈ 22:30; gilt vor §2 und §6)

**1. Warme Bau-Slots statt eigener Worktree:** Es gibt 8 vorgebaute Slots, Worktree `C:\Users\Administrator\Desktop\REA\fp-s<N>` mit `target-s<N>` (N = 1…8). Der Koordinator nennt dir deinen Slot im Auftrag.
- **Zu Beginn:** `git -C ..\fp-s<N> status --short` muss leer sein, dann `git -C ..\fp-s<N> checkout -q --detach main`.
- **Bauen:** `CARGO_TARGET_DIR=…\target-s<N>`, `CARGO_INCREMENTAL=1`, `-j 3`. Der Slot ist vorgebaut, nur Geändertes wird neu übersetzt.
- **Arbeiten und committen:**
  - Du bearbeitest die Dateien **im Slot** und testest dort.
  - Committet wird auf `main` im Haupt-Arbeitsbaum. Dafür übernimmst du deine Änderungen per `git -C ..\fp-s<N> diff > patch` und `git apply` in den Hauptbaum, oder du committest im Slot und bringst den Commit per `git cherry-pick` nach main.
  - Immer mit Pfadangabe (§2a).
- **Am Ende:** Slot und `target-s<N>` **nicht löschen**, denn der nächste Agent nutzt sie warm weiter. Nur den Slot sauber hinterlassen: `git -C ..\fp-s<N> checkout -q --detach main` und keine offenen Änderungen.

**2. Keine Arbeit in gemeinsamen Doku-Dateien:**
- Du änderst **nicht** `PARITAET_*.md`, `PLAN.md`, `STATUS.md`, `NUTZERBEFUNDE.md`, `NACHPRUEFUNG.md` und `BEFUNDE.md`.
- Stattdessen schreibst du **eine neue Datei** `docs\berichte\<kürzel>.md` mit:
  - Was umgesetzt ist, mit Commits.
  - Welche Befunde erledigt sind (Wortlaut-Anfang).
  - Welche Zeilen in PARITAET auf „umgesetzt, Nachprüfung ausstehend“ gehören.
  - Was in die Nachprüfung kommt (Ablauf, erwartetes Ergebnis).
  - Neue Befunde.
- Ein Doku-Agent überträgt das am Ende der Welle gesammelt.

**3. Prüfen:** wie §7a, aber nur die eigenen Testmodule (`--test all <modul>::`), dazu `cargo check --workspace --all-targets`, und bei neuen Texten fp-i18n und der Scanner. `fp-neutral` nur, wenn Texte oder Bilder dazukommen.

## 7a. Prüfungen gesammelt am Ende (Nutzerwunsch 12.10.2026 ≈ 18:00, gilt vor §4, §4c und §4e)

Damit tagsüber mehr umgesetzt wird, laufen die teuren Prüfungen gesammelt am Ende, über Nacht.

**Ab jetzt macht jeder Umsetzungs-Agent nur noch:**
- seine Änderung
- **gezielte eigene Tests:** Unit-Test bzw. kittest für genau das, was er geändert hat, in seiner eigenen Testdatei; nur diese laufen lassen
- `cargo check --workspace --all-targets`
- bei Texten: `cargo test -p fp-i18n` und den i18n-Scanner
- `cargo test -p fp-neutral`
- Doku: BEFUNDE, PARITAET (Stand „umgesetzt, Nachprüfung ausstehend“), STATUS, PLAN

**Ans Ende verschoben (nicht mehr je Agent):**
- Prüfungen im echten Fenster bzw. über die Brücke
- Bildserien (3 Designs × de/en), Paarbilder
- breite Testläufe ganzer Crates bzw. aller fp-ui-Tests
- Release-Builds

Stattdessen trägt der Agent je Punkt eine Zeile in **`docs\NACHPRUEFUNG.md`** ein: was zu prüfen ist, der Ablauf und das erwartete Ergebnis.

**Nicht verschoben:**
- **Messungen im Original:** Sie sind keine Prüfung, sondern liefern die Vorgabe (PM-n, RE-n, und wenn ein Wert fehlt, um richtig zu bauen).
- Prüfagenten, deren Auftrag selbst die Prüfung ist (PM-4, KT-n).
- INF-1: Er muss beweisen, dass Bauen und Tests weiter gehen.

Den Gesamttest und die Nachprüfung macht am Ende PLAN §5 Z.

## 7. Phase 7 (ab 12.10.2026 mittags): kleine Aufträge, höchstens 4 Agenten auf Opus

Nutzervorgabe: höchstens **4 Agenten gleichzeitig**, alle Opus, **kleine Aufträge** (ein Feature-Schnitt oder eine Handvoll Befunde). Die Phase-5/6-Agenten laufen nicht mehr; ihre Bereiche aus §1/§1b übernehmen die Phase-7-Agenten **nur für die im Auftrag genannten Befunde/Dateien**. Alles andere wie bisher (§1a Neutralität, §2 eigener `target-<kürzel>`, `-j 4`, §2a git nur eigene Pfade, §3 Befunde, §4 „funktioniert“).

| Kürzel | Thema | darf schreiben |
|---|---|---|
| **VT-<n>** | Bitmap nachzeichnen (Vektorisierung) | neues Crate `crates\fp-trace`; in fp-engine die neue Datei `src\commands\trace.rs` + je eine Registrierungszeile; Schlüssel `cmd-modify-traceBitmap*`/`trace-*` in `locales\{de,en}\menu.ftl` bzw. neue `locales\{de,en}\trace.ftl`; ab VT-2 der neue Dialog `crates\fp-ui\src\dialogs\trace.rs` + Einhängezeile |
| **UE-<n>** | Übersetzung (Restbefunde Phase 6) | die im Auftrag genannten Befunde und deren Dateien |
| **FV-<n>** | Funktionsvergleich mit dem Original (Paritätsbefunde) | die im Auftrag genannten Befunde und deren Dateien |

Laufen zwei Phase-7-Agenten in derselben Datei, gilt: kleine, in sich geschlossene Edits, vor jedem Edit die Datei neu lesen, nie fremde Änderungen zurückdrehen. Jeder hakt seine Befunde in `BEFUNDE.md` mit „– erledigt (<Kürzel>, <commit>)“ ab.

Scratchpad: Der Scratchpad-Ordner ist für alle Agenten derselbe. Jeder legt Hilfsdateien (z. B. `env.ps1`, Testbilder) **nur in einem eigenen Unterordner `<kürzel>\`** ab, nie direkt im Scratchpad-Wurzelordner.
