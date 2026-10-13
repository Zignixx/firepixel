# BATCH-7 – Animation + Video (ffmpeg) im Batch-Studio

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §3.8, §5.5, §5.10; Kern-API aus
`docs\berichte\batch1.md`, Oberfläche aus `docs\berichte\batch2.md`. Commit:
`BATCH-7: Animation und Video im Batch-Studio …` (siehe `git log --grep BATCH-7`).

## Umgesetzt

| Datei | Inhalt |
|---|---|
| `crates\fp-batch\src\steps\anim.rs` | Schritte `video.import`, `anim.trim` (Zeit oder Bildnummer, Bereiche `1-3, 10, 20-`), `anim.speed` (Faktor 0,1…10, feste fps, `minDelay` fasst zu kurze Frames zusammen), `anim.loop` (endlos/einmal/Anzahl, rückwärts, Pingpong ohne doppelte Endbilder), `anim.dropFrames` (jedes n-te, Ziel-fps, ähnliche zusammenfassen < x %), `anim.frameDelay` (alle gleich, erstes/letztes länger), `anim.toFrames` (Sammel-Schritt, Muster `{name}_{frame:4}`, `{n}`), `anim.fromFrames` (Sammel-Schritt, Größe erste/größte/fest, eingepasst und mittig, Hintergrund, Schleife), `anim.spriteSheet` (Animation → Sheet mit Atlas in `Meta::extra["anim.atlas"]`, Sheet → Frames N×M); Zeitleisten-Modell `Timeline` (Rasten auf Frame-Grenzen, Griffe, entfernte Frames, ↔ `anim.trim`-Parameter); Hilfen `quantize_delays`, `parse_frame_ranges`/`format_frame_ranges`, `frame_difference`, `trim_frames`, `speed_frames`, `loop_frames`, `drop_frames`, `frames_to_sheet`/`sheet_to_frames` |
| `crates\fp-batch\src\video\mod.rs` | `VideoParams` (Start/Ende ms-genau, fps 1…60, `maxFrames`, `maxSize`, Speichergrenze 1 GiB → automatisch verkleinert), Zeitangaben `1500`, `2.5s`, `m:ss.mmm`, `h:mm:ss.mmm`; `frame_delays` (Fehlerübertrag, Summe = Ausschnitt); `VideoInfo`; `trait VideoSource` (für BATCH-10); `decode`/`decode_with`; Haken `load` für `item::load` |
| `crates\fp-batch\src\video\ffmpeg.rs` | Suche: festgelegt (`set_override`) → Einstellung `<Profil>\video.json` `{"ffmpegPath": …}` (Datei oder Ordner) → `FIREPIXEL_FFMPEG` → `PATH` → neben der exe; `status()` (Pfad, Herkunft, Version, Klartext) für die Einstellungen; `version()` (`ffmpeg -version`), `probe()` (Kopfdaten aus `ffmpeg -i`, Drehung ±90° → Hochformat), `read_frames()` (`-ss … -i … -t … -an -sn -dn -vf fps=…,scale=W:H -frames:v N -f rawvideo -pix_fmt rgba pipe:1`, Pipe gestreamt, Lese-Thread mit kleinem Puffer); Argumente als Liste, keine Shell, `child_env`, stdin leer, **kein Konsolenfenster** (`CREATE_NO_WINDOW`); **Abbrechen** prüft alle 40 ms, auch wenn ffmpeg nichts liefert, und beendet den Prozess (`kill` + `wait`, Wächter `ChildGuard` auch bei Fehlern/Panik); Fehlermeldung „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen.“ |
| `crates\fp-batch\examples\fake_ffmpeg.rs` | Test-Attrappe (nur Tests; versteht `-version`, `-i` und den Frame-Aufruf, Modi `slow`/`stall`/`rotate90`/`FAKEFAIL`) |
| `crates\fp-ui\src\batch\timeline.rs` | Zeitleiste unter dem Vorschaubild: Frames als Abschnitte nach Dauer, Griffe Anfang/Ende (Druckpunkt entscheidet), Klick entfernt/holt Frame, außerhalb abgedunkelt, Zeiten, Zurücksetzen, Hinweise; `update()` rechnet die Dauern der Frames, die bei `anim.trim` ankommen (Rezept bis davor, Proxy 48 px, Hintergrund-Thread mit Generation), und gleicht das Modell mit dem Rezept ab (Rückgängig/Formular); `show_in_preview()` schreibt **einen** Schritt `anim.trim` (fehlt er: vorn eingefügt), eine Geste = ein Rückgängig-Schritt |
| `crates\fp-i18n\locales\{de,en}\batch_anim.ftl` | alle Texte (Schritte, Parameter, Auswahlwerte `batch-param-<id>-<param>-<wert>`, Zusammenfassungen, Fehler, Zeitleiste) |
| `crates\fp-batch\tests\anim_video.rs` | 30 Tests (s. u.) |
| `crates\fp-ui\tests\batch_timeline.rs` | 3 kittests (s. u.) |

**Zeiten bleiben stimmig**: Entfernte/zusammengefasste Frames geben ihre Dauer an das vorige behaltene
Frame (Gesamtdauer gleich); angeschnittene Frames beim Kürzen nach Zeit werden gekürzt (Summe = Ende −
Anfang); Rundung mit Fehlerübertrag, bei Ausgabe GIF auf 10 ms (Hundertstelsekunden), sonst 1 ms.
**0 Frames** ist ein Fehler des Elements („Danach bleibt kein Bild übrig.“), es entsteht keine Datei und
keine `.fptmp`; **1 Frame** bleibt eine Animation und lässt sich als GIF/APNG/WebP schreiben.
Ausgabe APNG/WebP animiert/GIF über die vorhandenen Kodierer (`output::encode`); den GIF-Optimierer von
BATCH-8 nutzt der Lauf automatisch, sobald er sich per `output::register_encoder(Gif, …)` einhängt.

## Tests (alle grün)

`cargo test -p fp-batch` (anim_video 30, lib 5, meta_optimize 14, recipe_naming 18, run 17),
`cargo test -p fp-ui --test all batch_timeline::` (3) und `batch_ui::` (15 + 1 ignoriert, unverändert grün),
`cargo test -p fp-i18n`, `cargo test -p fp-neutral`, Scanner `i18n_keine_festen_texte::`,
`cargo check --workspace --all-targets`, `cargo clippy -p fp-batch -p fp-ui` ohne Warnungen in eigenen Dateien.

- Kürzen nach Zeit (angeschnittene Frames, Summe = Fenster), nach Bildnummer + Bereiche, Textzeiten,
  Prüfung (Ende vor Anfang, ungültige Bereiche/Zeiten), 0 Frames (Fenster hinter dem Ende, alles entfernt),
  genau 1 Frame → GIF/APNG/WebP.
- Tempo: Faktor, Fehlerübertrag (33+34+33), feste fps, `minDelay` (10 × 10 ms → 5 × 20 ms), GIF-Raster
  10 ms mit gleicher Summe; **§4b**: Parameter 50× ändern = ein Schritt, gleiche Datei wie einmal mit
  Endwert (Prüfsumme).
- Schleife rückwärts/Pingpong/einmal/Anzahl, 1 Frame; Ausdünnen jedes n-te/Ziel-fps/ähnliche; Bilddauer.
- Animation → Einzelbilder (2 Quellen → `clip_0001.png` … `other_0002.png`, Pixel richtig), Einzelbilder
  → APNG/WebP/GIF (größte Größe, Dauern), Sprite-Sheet hin und zurück pixelgleich, Raster passt nicht → Fehler.
- Lauf GIF → kürzen + Tempo + Pingpong → GIF (Frames/Dauern), zweiter Lauf gleiche Bytes, Original
  unverändert; Vorschau zweimal gleich (ersetzt statt addiert).
- Zeitleiste: Rasten, Griff 50× ziehen = einmal, Ende nie vor Anfang, Parameter ↔ Modell ↔ Schritt.
- Video (Attrappe): Version, Kopfdaten, Drehung, kaputte Datei; Ausschnitt 0,5–1,5 s bei 10 fps → 10 ×
  100 ms, richtige Zeitpunkte; Größe/Höchstzahl; 1 Frame; 0 Frames → Meldung; **Abbrechen** während
  ffmpeg liefert (Prozess schreibt danach nichts mehr) und während ffmpeg schweigt (< 3 s); Video → GIF/
  APNG/WebP über den ganzen Lauf (Größe, 10 Frames, 1 s doppelt so schnell = 500 ms) und Vorschau;
  Einstellung im **Testprofil** (Datei und Ordner), entfernen; ohne ffmpeg: Element fehlgeschlagen mit
  „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen.“ (de/en), keine `.fptmp`.
- `real_ffmpeg_if_installed`: läuft nur mit installiertem ffmpeg (erzeugt ein 2-s-Testvideo mit `lavfi`
  und liest 10 Frames), sonst Meldung „übersprungen“. **Auf diesem Rechner ist kein ffmpeg installiert**
  (weder `PATH` noch `FIREPIXEL_FFMPEG`); es wurde nichts heruntergeladen. Die Attrappe deckt den Rest ab.
- kittests: Griffe über 30/12 Mausbewegungen, Klick entfernt/holt Frame, Zurücksetzen; schmal + Englisch
  + Dauern 0 + leer ohne Panik; Studio: GIF wählen → Zeitleiste erscheint, Ziehen fügt **einen**
  `anim.trim` vorn ein, Klick entfernt Bild 8, Rückgängig nimmt je Geste genau einen Schritt zurück und
  die Zeitleiste folgt.

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-batch\src\lib.rs`: `pub mod video;` · `steps\mod.rs`: `pub mod anim;` + `anim::register(r);`
- `crates\fp-batch\src\item.rs` (`load`): 3 Zeilen – Videos (`ItemKind::Video`) über `crate::video::load`
  (der in batch1.md offene „Lade-Haken für Videos“).
- `crates\fp-batch\src\executor.rs` (`run_collect`): 1 Zeile `steps::anim::tag_source` (Quellname vor
  Sammel-Schritten) und die Zielzeile über `steps::anim::collect_target` (Name aus dem Schritt, sonst wie
  bisher) – nötig für `anim.toFrames`.
- `crates\fp-ui\src\batch\mod.rs`: `pub mod timeline;` · `preview.rs`: Feld `timeline` in
  `PreviewState` (+ Vorgabe) und in `image_view` Platz für die Zeitleiste über den Angaben
  (`timeline::update` + `show_in_preview`), wie in batch2.md für BATCH-7 vorgesehen.
- **Für BATCH-2/11 (Voreinstellungen ▸ Batch)**: Zeile „ffmpeg-Datei…“ mit `fp_batch::video::ffmpeg::status()`
  (Text `message`, `is_ok()`), Knopf „wählen“ → `set_configured_path(Some(pfad))`, „automatisch“ →
  `set_configured_path(None)`; in der Dateiliste bei Videos ohne ffmpeg dieselbe Meldung und der Knopf.
- **Für BATCH-2/11 (Galerie)**: Bei Videos in der Liste die Karte `video.import` automatisch vorn einfügen
  (SPEC „automatisch bei Videos, Karte sichtbar“); der Lauf funktioniert auch ohne (Vorgaben 12 fps,
  ganzes Video bis 600 Frames).
- **Für BATCH-9 (MCP/Doku)**: Schritte im Katalog `batch.steps`; Abschnitt „Video über ffmpeg“ in
  `docs\AGENTS.md` (Suchreihenfolge, `FIREPIXEL_FFMPEG`, Fehlermeldung).
- **Für BATCH-10**: eingebaute Decoder setzen `video::VideoSource` um; `video.import` hat schon `decoder`
  auto/builtin/ffmpeg (builtin meldet heute „noch nicht verfügbar, ffmpeg wird benutzt“).

## Annahmen

- Videos werden beim Laden einmal mit den Vorgaben gelesen (12 fps, ganzes Video, ≤ 600 Frames, ≤ 1 GiB
  Speicher); `video.import` liest **neu**, wenn seine Parameter abweichen. Das kostet bei geänderten
  Werten einen zweiten ffmpeg-Lauf, braucht aber nur die eine Ladezeile im Kern. Besser später: Parameter
  an `item::load` durchreichen (Befund für den Kern).
- „Gestreamt“: ffmpeg liefert Frame für Frame über die Pipe (nie die ganze Datei im Speicher); die Frames
  selbst liegen als Animation im Speicher, begrenzt durch `maxFrames` und die Speichergrenze (Video wird
  sonst verkleinert). Echte Blockverarbeitung (32 Frames) bräuchte ein gestreamtes `Item` im Kern.
- `anim.loop` `count` = Zahl der Wiederholungen wie `Animation::loop_count` (0 = endlos, −1 = einmal);
  der Parameter `mode` (endlos/einmal/Anzahl) macht das für Anfänger eindeutig.
- `anim.speed` mit `minDelay` (Vorgabe 20 ms) fasst zu kurze Frames zusammen statt sie zu verlängern
  (Gesamtdauer bleibt; Browser zeigen < 20 ms sonst als 100 ms).
- `anim.toFrames`: Dateinamen kommen aus dem Muster (Endkarte „Dateiname“ wirkt dort nicht); gleiche
  Namen innerhalb eines Laufs bekommen ` (2)`. Bereits vorhandene Dateien im Ziel werden wie bei anderen
  Sammel-Ergebnissen behandelt (Konfliktregel nur für das erste Ergebnis, Kern-Verhalten).
- Sprite-Sheet-Atlas liegt in `Meta::extra["anim.atlas"]`; als eigene JSON-Datei neben dem Bild
  geschrieben wird er noch nicht (bräuchte eine Zusatzausgabe im Kern, P2).
- Zeitleiste rechnet die Frames, die bei `anim.trim` ankommen, im Hintergrund mit 48-px-Proxys über alle
  Frames (nicht nur die 150 der Vorschau), damit die Zeiten zu den Trim-Parametern passen.
- Kein Download, keine neue Crate (nur `std::process`, `regex` war schon da); Lizenz unverändert permissiv.

## Neue Abhängigkeiten

Keine.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes ffmpeg (vom Nutzer installiert): `cargo test -p fp-batch --test anim_video real_ffmpeg_if_installed -- --nocapture`
  → „echtes ffmpeg … “, 10 Frames. Dann im Studio ein MP4/MOV (Handy hochkant) ablegen, `video.import`
  0:01–0:03 bei 10 fps, Speichern als GIF → 20 Frames, 2 s, hochkant; **kein Konsolenfenster** blitzt auf.
- Abbrechen während ein langes Video gelesen wird → `ffmpeg.exe` verschwindet sofort aus dem Task-Manager,
  keine `.fptmp`.
- Ohne ffmpeg: Video in die Liste → Fehlerzeile „ffmpeg nicht gefunden – Pfad in den Einstellungen wählen.“
  (de/en), nach Wahl der Datei in den Einstellungen (sobald BATCH-2/11 die Zeile hat) läuft es.
- Zeitleiste in der Vorschau (echtes Fenster, Brücke): GIF wählen, Griffe ziehen, Bild anklicken, Strg+Z;
  3 Designs × de/en, schmale Vorschau-Karte (Zeitleiste verschwindet unter 3 × 74 px Höhe).
- GIF 30 fps (33 ms) → `anim.speed` Faktor 1 → Browser zeigt dieselbe Geschwindigkeit (Raster 10 ms,
  Summe gleich).

## Neue Befunde

- Kern: Parameter von `video.import` an `item::load` geben (doppeltes Lesen vermeiden); gestreamtes
  `Item::Animation` (Blöcke à 32 Frames) für sehr lange Videos.
- Kern: Zusatzdateien eines Elements (Atlas `.json` zum Sprite-Sheet).
- Oberfläche: ffmpeg-Zeile in den Voreinstellungen, Videozeile mit „ffmpeg-Datei wählen…“ (s. o.).
