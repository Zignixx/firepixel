# BATCH-11a – Feinschliff Teil 1 (Batch-Studio)

Stand 13.10.2026. Spezifikation `docs\BATCH_MODERN.md` §2.4, §2.8, §2.9, §2.10, §5.5; Gestaltung wie
der Startbildschirm (DESIGN §12.1). Commits: `git log --grep BATCH-11a` (4 Commits auf `main`).

## Umgesetzt

| Teil | Inhalt |
|---|---|
| **Parameter-Formular** (`crates\fp-ui\src\batch\params.rs`) | Generisches Formular aus dem Schema für alle Schritte von BATCH-4…9 und die Endkarte „Speichern als“: Zahlen als Schieber (runder Griff, gefüllte Spur; OLED in der Leitfarbe) + Zahlenfeld (Dezimalkomma je Sprache, Einheit gedämpft dahinter; Tippfehler bzw. außerhalb des Bereichs = rote Kante + Meldung „Größe: bitte eine Zahl von 1 bis 50 eingeben.“, der letzte gültige Wert bleibt); Auswahl als Segment-Knopf (bis 3 kurze Werte) oder Liste; Mehrfachauswahl als Chips mit Häkchen (Reihenfolge wie im Schema); Kästchen als Schalter; **Farbe mit dem Farbwähler-Popup des Originals** (FARB-1b: Raster, HEX/RGB, Pipette über Leinwand und Fenster, „Ohne“ = transparent bei Farben mit Vorgabe transparent) + HEX-Feld; Datei mit Feld + „Wählen…“; Zeit mit Beispiel-Hinweis; JSON (Profi) mit Prüfung. Werte ohne Vorgabe (z. B. Nachzeichnen) erscheinen als **„Automatisch“** (leeres Feld mit Hinweis, Listeneintrag, Segment Automatisch \| An \| Aus). **↺** am Zeilenende setzt einen geänderten Wert auf die Vorgabe bzw. „Automatisch“ zurück. Anfänger: nur `beginner()`-Felder, „Mehr Optionen (n)“. Barrierefreiheitsnamen je Feld (Schieber, Textfeld, Farbfeld, Segment, Chip „Entfernen: Kameradaten“, ↺ „Auf Vorgabe zurücksetzen: Größe“). |
| **Felder je Modus** (`fp-batch\src\step.rs`) | `ParamDef::when(param, werte)` + `is_visible` (Ketten, z. B. crop `ratioWidth` nur bei `mode` = aspect **und** `ratio` = free); Bedingungen an `resize`, `crop`, `canvas`, `trim`, `filter.blur`, `adjust.grayscale`, `background.removeColor/flatten/replace` (KI-Werte nur bei `method` = ai), `overlay.*` (x/y nur bei „frei“, Abstand nur bei Kacheln, Kontur nur mit Kontur), `metadata.edit`, `optimize.quantize/targetSize`, `anim.loop/dropFrames/frameDelay/fromFrames`, `vector.trace` (**Konturverfahren-Werte nur bei potrace**, Flächenwerte nur bei clusters – Befund BATCH-9). Versteckte Werte bleiben erhalten; die Zusammenfassung der zugeklappten Karte nennt nur passende Werte. Katalog (`batch.steps`, MCP, CLI) zeigt `visibleWhen` und `valueLabels`. |
| **Rückgängig im Rezept** | Ein Rückgängig-Schritt je Geste **und Parameter** (`recipe::changed_names`): Schieber 30 Bilder ziehen = 1 Schritt; Tippen in Feld A, dann Feld B (ohne Loslassen dazwischen) = 2 Schritte; Endkarte „Speichern als“ ebenso. |
| **Live-Vorschau** | Unverändert entprellt (150 ms, Generationszähler); geprüft: während eines Schieberzugs startet keine Rechnung, nach dem Loslassen genau eine. |
| **Ein Schema für Auswahlwerte** | `batch-value-<id>-<param>-<wert>` (`fp_batch::step::value_key`) für alle Pakete: 137 Schlüssel umbenannt (`batch-choice-*` aus batch_bg, Werte als `batch-param-…-<wert>` aus batch/batch_anim), alte Formen entfernt; fehlende Texte ergänzt (metadata keep/remove, metadata.set gps, optimize strip/dither/strategy, output compression). `value_label` sucht nur noch dieses Schema (Formate als Eigennamen, `filter.registry`-Befehle mit Menütext). Test: jeder Auswahlwert hat de+en-Text, keine alten Schlüssel, keine doppelten. |
| **Batch-Einstellungen** (`crates\fp-ui\src\batch\settings.rs`) | Banner ⋯ ▸ „Einstellungen…“: Überlagerung im Kartenstil mit „Allgemein“ (Modus, Unterordner) und „Videos (ffmpeg)“: Zustand im Hintergrund geprüft (`ffmpeg::status`: ✓ „ffmpeg 7.1“ + Herkunft + Pfad bzw. ⚠ „ffmpeg nicht gefunden“), Knöpfe **„ffmpeg-Datei…“** (`set_configured_path`, Datei-Dialog „Programme“), „Automatisch suchen“ (nur wenn eine Datei gewählt ist), „Erneut prüfen“; Hinweis bei fehlendem ffmpeg („liefert ffmpeg nicht mit und lädt nichts herunter …“). Esc/Schließen; Klicks neben die Karte erreichen das Studio nicht. Höhe nach Inhalt (Hinweis umbricht je Sprache). |
| **Vorschau-Maßstab** (Befund BATCH-4/6) | `StepContext::proxy_scale` (gesetzt von `pipeline::preview`) + `preview_scale()`; `geometry::preview_scale` und `background::preview_scale` nehmen ihn (Schätzung nur noch als Rückfall); `effect.live` verkleinert Abstand/Weichheit/Breite des Effekts auf dem Proxy (`scale_live_effect`). Test: Rahmen nach `resize` in der Vorschau 563 statt 538 px (= Ergebnis × Maßstab). |
| **Rückfrage beim Schließen** (Befund BATCH-2) | Fenster schließen (×, Alt+F4, *Datei ▸ Beenden*) während eines Laufs: Studio nach vorn, Karte „Batch läuft noch – abbrechen und schließen?“ mit „3 von 12 Dateien sind fertig …“, Knöpfe „Weiterlaufen lassen“ (auch Esc) und „Abbrechen und schließen“ (bricht ab, danach normaler Ablauf inkl. Rückfrage für ungespeicherte Dokumente). Dasselbe beim Schließen des **Studios** (× am Reiter, ⋯ ▸ Batch schließen). Die Konfliktfrage des Laufs wird währenddessen ausgeblendet. |

Texte: `crates\fp-i18n\locales\{de,en}\batch_form.ftl` (neu), Werte-Texte in `batch_meta.ftl`.
Bilder: `docs\screenshots\batch11a_form_{normal,dunkel,oled}_{de,en}.png` (Karte Text-Wasserzeichen offen)
und `batch11a_settings_{…}_{de,en}.png` (Einstellungen), 1440 × 900; erzeugen mit
`cargo test -p fp-ui --test all batch_form::batch11a_bilder -- --ignored`.

## Tests (alle grün)

- `cargo test -p fp-batch` (alle Programme), neu `value_labels` (8): Werte-Texte de/en, alte Schlüssel
  entfernt, Katalog mit `valueLabels`/`visibleWhen`, Bedingungen gültig (keine Selbstbezüge, Werte
  existieren), Felder je Modus, Kontext-Maßstab, Rahmen nach Größenschritt, Live-Effekt mitverkleinert
  (ohne die Korrektur schlägt der Rahmen-Test fehl: 538 statt ≈ 563). `anim_video`/`steps_overlay` auf
  das neue Schema umgestellt.
- `cargo test -p fp-ui --test all batch_` (42 + 3 ignoriert; neu `batch_form` 11): Felder je Modus,
  Schieber 30 Bilder = 1 Schritt + keine Vorschau während des Ziehens + genau eine danach, Zahlenfeld
  Zeichen für Zeichen (Strg+A im Feld wirkt nicht auf die Dateiliste), Fehler „12x“, Dezimalkomma
  „1600,0“, ↺ + Rückgängig, Geste je Parameter, Liste/Chips 50× ohne Duplikate (OLED), Automatisch +
  ↺ + Automatisch|An|Aus, Farbwähler-Zelle + HEX tippen, Einstellungen 3 Designs × de/en (1024×768)
  ohne Abschneiden, Klick daneben, Esc, Fenster- und Studio-Schließen während eines Laufs (Lauf wartet
  über die Konfliktregel „Jedes Mal fragen“), nichts abgeschnitten (4 Karten × 3 Designs × de/en ×
  1024/1920 × 100/150 %). `batch_ui`, `batch_rename`, `batch_timeline`, `batch_templates_b9` unverändert grün.
- `FP_RAND_VIEWS=Batch FP_RAND_FULL=1 clipping_audit::` (Studio jetzt mit offener Karte
  Text-Wasserzeichen im Profi-Modus): **0 Funde**, 12 Hinweise (Bildlauf), 6 mit „…“.
- `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, Scanner `i18n_keine_festen_texte::`,
  `cargo test -p fp-neutral`, `clippy -p fp-batch -p fp-ui --tests` ohne Warnungen in den eigenen Dateien.

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-ui\src\actions.rs` (HG): 3 Zeilen am Anfang von `Action::Quit` →
  `crate::batch::settings::intercept_quit(st)`.
- `crates\fp-ui\tests\clipping_audit.rs` (RAND): `batch_studio()` öffnet zusätzlich die Karte
  `overlay.text` im Profi-Modus statt der Endkarte „Speichern als“ (mehr Feldarten im Audit).
- `crates\fp-batch\src\steps\*.rs` (BATCH-4…9): nur `.when(…)` an Parametern, `ParamDef`-Literale über
  `ParamDef::new`, in `adjust.rs` `scale_live_effect`; `gifopt\` nicht angefasst.
- `crates\fp-batch\tests\anim_video.rs`, `steps_overlay.rs`: Werte-Schlüssel über `value_key`.

## Annahmen

- Einheitliches Schema ist `batch-value-…` (nicht `batch-param-…-<wert>`): trennt Feld- und Werte-Texte,
  88 der Schlüssel hatten es schon. Die GIF-Optimierer-Werte (`batch.ftl`, Block BATCH-8) sind mit
  umbenannt (nur Daten, kein Code von BATCH-8b betroffen).
- Segment-Knopf nur, wenn alle Werte nebeneinander passen; sonst Liste (z. B. metadata „Behalten“).
- „Automatisch“ = Parameter nicht im Rezept (Vorgabe des Schritts bzw. der Vorlage gilt); ↺ entfernt den
  Schlüssel aus dem Rezept.
- Farbfelder: „Ohne“ (transparent) im Farbwähler nur bei Farben mit Vorgabe `transparent` bzw. wenn die
  Farbe schon transparent ist.
- Die Einstellungen-Überlagerung zeigt Modus und Unterordner zusätzlich (wie im Banner); die
  ffmpeg-Prüfung startet `ffmpeg -version` nur im Hintergrund, nie beim Zeichnen.
- Schließen des Studios (×) während eines Laufs fragt jetzt ebenfalls (BATCH-2 hatte ohne Rückfrage
  abgebrochen). Nach „Abbrechen und schließen“ beim Fenster folgt ggf. die übliche Rückfrage für
  ungespeicherte Dokumente; bricht der Nutzer die ab, ist der Lauf trotzdem abgebrochen.
- Zahlenfeld: Werte außerhalb des Bereichs werden nicht übernommen (keine stille Begrenzung beim
  Tippen); der Schieber begrenzt immer.

## Für PARITAET/STATUS („umgesetzt, Nachprüfung ausstehend“)

Batch-Studio: Parameter-Formular aller Schritte, Felder je Modus, Batch-Einstellungen mit ffmpeg-Datei,
Rückfrage beim Schließen während eines Laufs, genaue Vorschau nach Größenschritten.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echtes Fenster (Brücke): Studio mit 5 Fotos, „Text/Wasserzeichen“ hinzufügen → Schieber „Größe“
  ziehen: Vorschau folgt nach dem Loslassen (kein Flackern), Strg+Z nimmt den ganzen Zug zurück; Feld
  tippen „6,5“ → 6,5 %; Farbe: Klick aufs Farbfeld → Farbwähler des Originals, Pipette über dem
  Vorschaubild nimmt die Farbe auf; ↺ setzt zurück. 3 Designs × de/en.
- `resize` Modus wechseln (Längste Seite → Breite → Einpassen): nur passende Felder; „Nachzeichnen“
  Verfahren „Glatte Kurven“: Konturwerte erscheinen, Flächenwerte verschwinden.
- ⋯ ▸ Einstellungen: ohne ffmpeg ⚠ + Hinweis; „ffmpeg-Datei…“ eine echte `ffmpeg.exe` wählen → ✓
  Version + „in den Einstellungen gewählt“; Video in die Liste → wird gelesen; „Automatisch suchen“
  → Einstellung gelöscht.
- Großer Lauf (≥ 200 Fotos) starten, dann **Alt+F4** bzw. × des Fensters (echte Systemeingabe, Sperre
  §4e): Rückfrage erscheint, „Weiterlaufen lassen“ → Lauf läuft weiter; erneut, „Abbrechen und
  schließen“ → Programm endet, keine `.fptmp` im Ziel, fertige Dateien bleiben. Ebenso × am Reiter „Batch“.
- Großes Foto (6000 px) mit `resize` 1200 + `overlay.border` 60 px bzw. `effect.live` Schlagschatten:
  Vorschau und Ergebnis (in einem Bildbetrachter verkleinert) zeigen gleich breite Rahmen/Schatten.

## Offen für BATCH-11b

- Endkarte „Speichern als“ bei GIF: Felder des GIF-Optimierers (`preset`, `quality`, `maxKB`) und
  „erreichte Größe“ bzw. ⚠ „Zielgröße nicht erreichbar“ (`Optimized.reached`) – nach BATCH-8b; dabei
  `output_defs` (feste Liste in `recipe.rs`) auf `ParamDef::when("format", …)` umstellen.
- Videozeile der Dateiliste: bei fehlendem ffmpeg Meldung + Knopf „ffmpeg-Datei…“ (Einstellungen hier
  wiederverwenden: `settings::open`); Karte `video.import` bei Videos automatisch vorn einfügen.
- Hinweise im Rezept: „SVG/FW-PNG in ‚Speichern als‘ wählen“ bei `vector.trace` mit Pixelformat;
  „JPEG füllt Transparenz mit Weiß“.
- Eigene Felder statt JSON: `filter.registry.params` aus `describe_command` des gewählten Befehls,
  `script.command.name` als Liste aus `steps::script::batch_commands()`, `effect.live.params` aus dem
  Effekt-Schema, `adjust.curves.points` als Kurven-Editor, `metadata.edit.tags`.
- Datei-Wahl bei Bild-Parametern (`overlay.image.image`, `background.*.image`) mit Bildfilter statt
  „Alle Dateien“.
- Dateiliste: Filter Größe/Pixel/Transparenz/Glob (Profi), Sortieren per Ziehen, Leertaste/Eingabe,
  Fortschritt je Datei (`ItemProgress`); Kartenmenü „Als Vorlage speichern“/„Hilfe“.
- Profi-Ausdrücke in Zahlenfeldern (`{width}/2`, P3).
- `anim.spriteSheet` `spacing`/`padding` werden in der Vorschau nicht mit dem Proxy-Maßstab gerechnet.
- `{w}x{h}` im Dateinamen nutzt weiter die Quellgröße (Kern, offen seit BATCH-1).

## Neue Befunde

- (für AU, Doku) `docs\AGENTS.md`/`AGENTS.en.md` §5c: Katalog `batch {"action": "steps"}` liefert jetzt
  je Parameter `visibleWhen` (`{param, values}`) und `valueLabels` – Satz ergänzen.
- (für I18N-KERN) Schlüsselschema der Batch-Werte steht in `fp_batch::step` (Doku-Kommentar); im
  README der Bereichsdateien ggf. `batch_form.ftl` (BATCH-11a) nachtragen.

## Neue Abhängigkeiten

Keine.
