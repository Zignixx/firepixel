# Bericht VT-6 – zweites Nachzeichen-Verfahren „Glatte Kurven“ (Potrace-Verfahren), Nutzerbefund N29

Slot `fp-s5` / `target-s5`. Commit: siehe `git log --grep "VT-6"`.

## Umgesetzt

- **Neues Modul `crates\fp-trace\src\potrace.rs`**: eigenständige Umsetzung nach dem Fachartikel
  P. Selinger, „Potrace: a polygon-based tracing algorithm“ (2003). Kein Code und keine Bibliothek von
  Potrace (GPL), keine neue Abhängigkeit.
  - Binarisierung: Schwarzweiß über `threshold`. Farbe: eigene k-Means-Farbreduktion auf `colors`,
    dann je Farbe eine Maske, **gestapelt** (größte Fläche zuunterst, jede Maske = ihre Farbe und alle
    darüber), deshalb ohne Haarrisse. Transparenz wie beim bisherigen Verfahren.
  - Pfadzerlegung mit Abbiegeregel (`turnPolicy`: minority/majority/black/white/left/right), XOR-Zerlegung
    (Löcher werden eigene Konturen, gerade/ungerade), Flecken unterdrücken (`turdSize`).
  - Optimales Polygon: Geradheit nach der Definition des Artikels (Gerade im Abstand ≤ ½ an allen Punkten,
    ≤ 3 Richtungen). Erst eine schnelle Kegelprüfung, dann eine genaue Prüfung mit binärer Suche.
    Danach kürzeste Polygonfolge mit kleinster Strafe; zweiter Lauf ab einer Polygonecke, damit am
    Startpunkt keine künstliche Ecke entsteht.
  - Eckpunkte verfeinern: Ausgleichsgeraden, Minimum der Abstandsquadrate im Quadrat ±½.
  - Glätten/Ecken (`alphaMax`, α = 4/3·(1 − e/D), begrenzt auf 0,55…1).
  - Kurvenoptimierung (`optCurve`, `optTolerance`): Bögen gleicher Krümmungsrichtung mit weniger als 179°
    Drehung werden zusammengefasst, mit flächengleichen Henkeln und Toleranzprüfung (DP: erst wenigste
    Bögen, dann kleinste Abweichung).
- `fp-trace`: `Engine { Clusters (Vorgabe), Potrace }`, `TraceOptions.engine` und `.potrace`
  (`PotraceOptions`, `TurnPolicy`, `ALPHA_MAX_LIMIT`). Alte JSON ohne diese Felder ergeben das bisherige
  Verfahren.
- **Befehl `modify.traceBitmap`** (`fp-engine\src\commands\trace.rs`): `engine` (`clusters`|`potrace`,
  Vorgabe `clusters`), `turdSize`, `turnPolicy`, `alphaMax`, `optCurve`, `optTolerance` und das Objekt
  `potrace`. Ein Aufruf bleibt ein Verlaufsschritt.
- **Dialog** (`fp-ui\src\dialogs\trace.rs`): Gestaltung N20 bleibt. Über den Vorgaben steht eine Zeile
  **„Verfahren: Farbflächen | Glatte Kurven“** (Erklärung als Tooltip), mit Live-Vorschau und den
  Kennzahlen zum Vergleich.
  - Eigene Hauptregler bei „Glatte Kurven“: Flecken (px²), Ecken (alphaMax), „Kurven zusammenfassen“ mit
    Toleranz.
  - „Erweitert“: Abbiegeregel (6 Knöpfe in 2 Reihen) statt „Aufräumen“; nur die gemeinsamen Optionen.
  - Die Wahl wird **sofort gemerkt**, auch bei Abbrechen (`TraceDialogEngine`).
  - Ein Wechsel der Vorgabe oder „Auf Vorgabe zurücksetzen“ behält das Verfahren.
- Texte de+en: `trace.ftl` (17 Schlüssel `trace-dlg-engine`, `trace-engine-*`, `trace-potrace-*`,
  `trace-turn-*`, `trace-unit-px2`).
  - **Ausnahme:** 7 `cmdtext-`-Schlüssel der neuen Parameterbeschreibungen in `command_texts.ftl`
    (de+en, Bereich UE). Sie sind nötig, weil der CLI-Test `parameterbeschreibungen_in_de_und_en` jede
    Beschreibung verlangt.
- Doku: `docs\AGENTS.md` und `AGENTS.en.md`, Abschnitt „Bitmap nachzeichnen“ (Parameter, Ablauf
  „Vergleichen“). Kommentar in `crates\fp-trace\Cargo.toml`.

## Tests (alle grün)

- `cargo test -p fp-trace --test potrace` (11, neu):
  - Golden-Tests: Quadrat ergibt genau 4 Ecken auf den Pixelkanten. Kreis r 40 ergibt 3–8 glatte Knoten,
    Radius ±0,75 px. Buchstabe T ergibt 8 Ecken mit 100 % Übereinstimmung. Buchstabe O ergibt 2 glatte
    Konturen, das Loch bleibt frei.
  - Flecken und Löcher, Abbiegeregel, alphaMax 0 und 4/3, Farbebenen gestapelt (> 99 %), Transparenz.
  - Wiederholung (50× Werte ändern, Endwert ergibt dasselbe Ergebnis), JSON-Rückwärtskompatibilität,
    Abbruch und Fortschritt, Vergleich.
- `fp-engine`:
  - Unit-Tests `engine_parameters` und Dialog-Rundlauf mit Konturverfahren.
  - `trace_bitmap::trace_with_second_engine_history_and_repeat` (§4b): ein Verlaufsschritt,
    Rückgängig/Wiederholen exakt, 5× nachzeichnen/zurücknehmen ergibt dasselbe Dokument, die Folge
    A, B, Rückgängig, C, und ein Fehler ohne Änderung.
- `fp-ui --test all dialogs_trace_engine_vt6::` (3, neu):
  - Umschalten startet genau eine Rechnung, eigene Regler, Wahl sofort gemerkt.
  - Abbrechen ändert nichts. OK übernimmt die Vorschau als einen Verlaufsschritt.
  - 20× hin und her ergibt dasselbe Ergebnis. Kein Überlauf in de und en.
  - Außerdem grün: alle `trace`-Module (dialogs_trace, trace_size_fv12 „Erweitert ohne Bildlauf“).
- `cargo check --workspace --all-targets`, `fp-i18n`, Scanner `i18n_keine_festen_texte`, `fp-neutral`:
  grün. Clippy in den eigenen Dateien ohne Warnungen.
- **Nicht von VT-6:** Der CLI-Test `parameterbeschreibungen_in_de_und_en` meldet 19 fehlende
  Übersetzungen fremder Befehle (`file.open`, `edit.copyHtmlCode`, `element.setPattern/Texture` …).
  Keine davon gehört zu `modify.traceBitmap`.

## Vergleich an einer Strichzeichnung (512 × 512, Linien, Ringe, Welle; Debug-Profil, ein Thread)

| Verfahren | Zeit | Pfade | Konturen | Knoten | Übereinstimmung |
|---|---|---|---|---|---|
| Farbflächen (Vorgabe Schwarzweiß) | ≈ 9–10 ms | 5 | 10 | 278 | 99,30 % |
| Glatte Kurven | ≈ 44–49 ms | 1 | 10 | **134** | **99,67 %** |
| Glatte Kurven ohne Kurvenoptimierung | ≈ 43 ms | 1 | 10 | 233 | 99,66 % |

Das neue Verfahren braucht etwa halb so viele Knoten und trifft genauer; es ist aber etwa 5× langsamer.

## Annahmen

- UI-Name „Glatte Kurven“ statt „Konturen“: „Konturen“ ist schon der Name einer Vorschau-Ansicht.
  Im Tooltip und in der Doku heißt es „Potrace-Verfahren“.
- Das Konturverfahren erzeugt **einen Pfad je Farbe bzw. Maske** (mit allen Konturen, gerade/ungerade),
  nicht einen Pfad je Fläche. Beim Schwarzweiß-Bild entsteht also ein schwarzer Pfad.
- Ohne `colors` (0) wird bei Farbe auf höchstens 256 Farben reduziert.
- Die Vorgaben (Logo/Illustration …) setzen beim Konturverfahren nur Farbmodus, Farben und Schwelle.

## PARITAET / Nachprüfung

- PARITAET (Bitmap nachzeichnen, N29) → „umgesetzt, Nachprüfung ausstehend“.
- NACHPRUEFUNG (echtes Fenster, Brücke): *Modifizieren ▸ Bitmap nachzeichnen…* öffnen, „Glatte Kurven“
  wählen.
  - Erwartet: Die Vorschau rechnet neu, die Regler wechseln, die Kennzahlen zeigen weniger Punkte.
  - Danach Abbrechen und neu öffnen: „Glatte Kurven“ ist vorgewählt. Dann OK: eine Gruppe auf der neuen
    Ebene, ein Verlaufsschritt.
  - Bildserie: Dialog mit „Glatte Kurven“ und offenem „Erweitert“ in 3 Designs × de/en.

## Neue Befunde

- (für UE) 19 fehlende `cmdtext-`-Übersetzungen fremder Befehle (siehe oben). Der CLI-Test
  `parameterbeschreibungen_in_de_und_en` ist rot.
