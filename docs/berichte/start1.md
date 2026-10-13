# START-1: Programmstart schneller (Nutzerbefund N30)

Stand 13.10.2026, Agent START-1, Slot `fp-s6`.

## Messverfahren

- Neue Umgebungsvariable **`FP_STARTUP_TRACE`** (`crates\fp-i18n\src\startup.rs`): `=1` schreibt auf
  stderr, `=<Pfad>` hängt an eine Datei an. Je Phase eine Zeile
  `startup <ms seit main> +<ms> epoch=<Unix-ms> <phase>`; bei gemessenen Phasen (`time`) ist `+` die
  eigene Dauer, sonst die Zeit seit der vorigen Marke. Die Unix-Zeit erlaubt, „exe laden“
  (Prozessstart bis `main`) von außen zu messen. Ohne die Variable kostet eine Marke nur einen
  `OnceLock`-Zugriff.
- Marken: `main`, `eframe::run_native`, Fenster + Grafik bereit (`App::new`), Schriften,
  Voreinstellungen, Sprache (Fluent), Befehlsordner, Arbeitsbereich, Sitzung + Befehlsregister,
  App-Zustand, Frames 1–12 (Beginn/Ende), Fenster sichtbar außerhalb, **Fenster an seinem Platz
  (= sichtbar für den Nutzer)**, **erster Frame danach (= bedienbar)**, Vorschaubilder
  „Zuletzt geöffnet“ fertig, Vorlade-Threads.
- Release-exe (`lto = "thin"`, 67 MB), Windows Server 2025 ohne Grafikkarte (wgpu nimmt DX12;
  Vulkan/GL finden hier keinen Adapter). Eigene Instanz je Lauf mit eigenem Profil (`APPDATA`,
  `LOCALAPPDATA`, `FP_TEST_PROFILE` im Scratchpad, `FP_NO_SHELL_OPEN=1`), Fenster 900 × 650 unten
  rechts (vorbereitete `app.ron`), nicht maximiert, ohne Fokus, per `WM_CLOSE` beendet.
- **kalt** = Erststart: neues Profil (Befehlsordner wird angelegt, Systemsprache) und frisch kopierte
  exe je Lauf. Den Datei-Cache von Windows leeren ging ohne Zusatzwerkzeug nicht; die Systemschriften
  lagen also im Cache. **warm** = gleiches Profil (Sprache de) mit 4 zuletzt geöffneten PNG-Dateien.
- Je Fall 2 × 5 Läufe, vorher/nachher abwechselnd; in der Tabelle steht der Median aus 10 Läufen.

## Ergebnis (Median, ms)

| Phase | kalt vorher | kalt nachher | warm vorher | warm nachher |
|---|---:|---:|---:|---:|
| exe laden (Prozessstart → `main`) | 610 | 536 | 31 | 32 |
| Fenster + Grafik-Init (winit, wgpu) | 406 | 425 | 131 | 131 |
| Schriften der Oberfläche (≈ 50 MB, inkl. Fluent-Parsen) | 33 | 0 ¹ | 24 | 0 ¹ |
| Voreinstellungen | 1 | 1 | 0 | 0 |
| Sprache (Fluent, Sprachordner) | 1 | 1 | 1 | 1 |
| Befehlsordner (eigene Befehle prüfen/anlegen) | 42 | 0 ² | 6 | 0 ² |
| Sitzung + Befehlsregister (inkl. Index der Systemschriften) | 31 | 3 ¹ | 27 | 2 ¹ |
| App-Zustand (Renderer, Skript-Host) | 0 | 0 | 0 | 0 |
| **`App::new` gesamt** | **117** | **7** | **67** | **8** |
| Frame 1 (Oberfläche aufbauen) | 13 | 11 | 13 | 12 |
| Vorschaubilder „Zuletzt geöffnet“ fertig (seit Start) | – | – | 298 | 244 |
| **erstes Bild gezeichnet** (seit Prozessstart) | 1245 | 1086 | 361 | 314 |
| **Fenster sichtbar** (an seinem Platz) | 1974 | 1597 | 1081 | 848 |
| **bedienbar** (erster Frame danach) | 2020 | 1648 | 1119 | 896 |

¹ im Hintergrund vorgeladen, parallel zu Fenster + Grafik-Init; fertig nach (Median, seit `main`)
UI-Schriften 20 ms, Schriftindex 29–34 ms, Sprachdateien 13–15 ms, also lange vor `App::new`
(≥ 130 ms): keine Wartezeit mehr.
² im Hintergrund-Thread, fertig nach 143 ms (warm) bzw. 477 ms (kalt) seit `main`.

**Gesamt: sichtbar warm −233 ms (−22 %), kalt −377 ms (−19 %); bedienbar warm −223 ms, kalt −372 ms.**
Die Streuung der kalten Läufe ist groß (exe laden 470–1030 ms, Grafik-Init 260–650 ms), die
Unterschiede dort stammen zum Teil aus dem Rauschen; warm streut es um ±30 ms.

## Was geändert ist

1. **Vorladen beim Start** (`crates\fp-ui\src\lib.rs`, `fn prewarm`): Drei Hintergrund-Threads laden
   direkt in `run()` die UI-Schriftdateien (`theme::prewarm_fonts`), den Index der Systemschriften
   (`fp_text::FontDb::shared`, gebraucht für die Vorgabeschrift des Textwerkzeugs) und die
   eingebauten Sprachdateien (`fp_i18n::languages`). Alles liegt in prozessweiten `OnceLock`s;
   `App::new` bekommt dasselbe Ergebnis wie vorher, nur ohne zu warten.
2. **Befehlsordner im Hintergrund** (`lib.rs`): `install_builtin_commands` läuft nach der
   Sprachwahl in einem eigenen Thread (Sprache ist da schon gesetzt, Namen wie bisher in der
   Sprache der Oberfläche). Das Befehle-Menü liest den Ordner erst beim Öffnen. Geht der Thread
   nicht zu starten, läuft es wie bisher direkt.
3. **Kürzere Beruhigungszeit außerhalb des Bildschirms** (`start_window.rs`, N09): `SETTLE`
   0,7 → 0,5 s (weiter mindestens 4 Frames). Die gemessenen Störungen (weißes erstes Bild ≈ 0,1 s,
   vergrößerter Ausschnitt bis ≈ 0,45 s) stammen aus dem Debug-Build mit ≈ 250 ms je Frame; im
   Release-Build sind es ≈ 60 ms. 0,5 s deckt die alte Messung weiter ab. Das ist der größte
   Einzelgewinn (≈ 200 ms) – **muss in der Nachprüfung bestätigt werden** (siehe unten).
4. **Testinstanzen stehlen keinen Fokus mehr** (`start_window.rs`): Beim Platzieren ruft Firepixel
   `focus_window()`. winit erzwingt den Vordergrund dann mit einem **simulierten Alt-Tastendruck
   (`SendInput`)**, wenn das Fenster nicht schon vorn ist – der Alt landet im gerade aktiven fremden
   Fenster (vermutlich die Ursache des Vorfalls in AGENTEN_REGELN §4d). In Testläufen
   (`fp_i18n::profile::is_test_run()`: `FP_NO_SHELL_OPEN`, `FP_TEST_PROFILE`, cargo-Tests) entfällt
   der Aufruf jetzt; für den Nutzer bleibt alles wie bisher. Nötig, damit die Messinstanzen §4e
   einhalten.
5. **Startzeit-Messung** `FP_STARTUP_TRACE` (siehe oben), Marken in `apps\firepixel\src\main.rs`,
   `lib.rs`, `start_window.rs`, `welcome\thumbs.rs`.

Hinweis: Die gemessene „nachher“-exe hatte in `startup::time` noch die globale `+`-Spalte (bei
parallelen Threads nicht die eigene Dauer); die Fußnoten oben nutzen deshalb die Endzeitpunkte.
Der Commit enthält die korrigierte Fassung (eigene Dauer je Phase), sonst gleicher Code.

## Geprüfte Punkte aus dem Auftrag, ohne Änderung

- **Sprachdateien nur für die aktive Sprache:** Eingebaut sind nur `de` (Ausgangssprache) und `en`
  (Rückfallsprache); beide werden für die Rückfallkette gebraucht. Das Parsen dauert ≈ 12 ms und
  läuft jetzt im Hintergrund – kein weiterer Gewinn möglich.
- **Vorschaubilder „Zuletzt geöffnet“:** waren schon asynchron (ein Thread je Datei,
  `welcome\thumbs.rs`); fertig nach ≈ 250 ms, also bevor das Fenster sichtbar ist.
- **KI-Modellsuche:** läuft beim Start nicht (erst im KI-Inspektor bzw. Modell-Dialog).
- **Texturen/Muster-Listen, Bibliotheken, Arbeitsbereich-Layouts:** werden beim Start nicht gelesen
  (`App::new` gesamt jetzt 7–8 ms, Frame 1 ≈ 12 ms).
- **Einstellungen** (Voreinstellungen, `app.ron`): < 1 ms.

## Was übrig bleibt (nicht im Code dieses Auftrags lösbar)

- **Fenster + Grafik-Init** 130 ms warm, ≈ 420 ms kalt. Davon ≈ 35 ms für das Abtasten von Vulkan
  und OpenGL, die hier keinen Adapter haben (gemessen: `WGPU_BACKEND=dx12` 124 ms statt 159 ms).
  Die Backends einzuschränken würde Rechner treffen, auf denen Vulkan/GL gebraucht wird → Befund.
- **Erstes Zeichnen** (Frame 1 Ende → Frame 2) ≈ 70–120 ms: Schrift-Atlas, Pipelines, Software-Raster.
- **exe laden kalt** 540–610 ms bei einer frisch geschriebenen 67-MB-exe (vermutlich Virenscanner
  beim ersten Start einer neuen Datei; warm 31 ms).
- **Beruhigungszeit** 0,5 s (N09-Schutz gegen Flackern) ist weiter der größte Posten.

## Tests

- `cargo check --workspace --all-targets` grün.
- `cargo test -p fp-neutral` grün.
- `cargo test -p fp-i18n` grün (neu: `startup::tests::disabled_without_env`).
- `cargo test -p fp-ui --lib start_window` grün (u. a. `settle_covers_measured_glitches`: 0,5 s
  liegt im erlaubten Bereich 0,5–1,0 s).
- Startmessung 2 × 5 Läufe je Fall in der echten Release-exe (eigene Instanzen, eigenes Profil,
  sichtbar 900 × 650 unten rechts, ohne Fokus, danach geschlossen).

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- **N30/N09 Start ohne Flackern mit 0,5 s:** Release-exe mit echtem Profil, maximierter Start
  (Vorgabe) und nicht maximiert, Software-Grafik; den Start mit schnellen Bildschirmfotos (≥ 20/s)
  aufnehmen. Erwartet: das Fenster erscheint genau einmal, fertig gezeichnet, ohne weißes Bild und
  ohne vergrößerten Ausschnitt; Taskleistenknopf einmal. Falls doch: `SETTLE` wieder auf 0,7 s.
- **N30 Startzeit beim Nutzer:** `FP_STARTUP_TRACE=C:\temp\start.log` setzen, Firepixel aus dem
  Explorer starten. Erwartet: „window placed“ ≈ 0,85 s (warm) nach Programmstart.
- **Fokus beim echten Start:** Doppelklick auf die exe → Firepixel ist nach dem Erscheinen vorn und
  hat den Tastaturfokus (unverändert).
- **Befehle-Menü beim Erststart:** neues Profil, Firepixel starten, sofort *Befehle* öffnen →
  die eigenen Befehle sind da (Hintergrund-Thread fertig nach ≈ 0,5 s ab Start).

## Neue Befunde

- (von START-1, für HG) winit `focus_window()` sendet unter Windows einen simulierten Alt-Druck an
  das aktive fremde Fenster, wenn Firepixel nicht vorn ist (`start_window::after_frame`). Für
  Testläufe abgestellt; für den Nutzer prüfen, ob der Aufruf beim normalen Start überhaupt nötig
  ist (beim Start aus dem Explorer ist das Fenster schon vorn, dann sendet winit nichts).
- (von START-1, für HG) wgpu tastet beim Start Vulkan, DX12 und OpenGL ab (≈ 35 ms auf Rechnern
  ohne Vulkan/GL). Möglich: unter Windows zuerst nur DX12 versuchen und nur bei Fehlschlag die
  anderen Backends.
