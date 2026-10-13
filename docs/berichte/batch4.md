# BATCH-4 – Geometrie, Farbe/Effekte, Erweitert (Batch-Studio)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §3.3, §3.4, §3.13, Paket BATCH-4 aus §6.1.
Aufgesetzt auf BATCH-1 (Kern) und BATCH-5 (`f289d79`, Metadaten).

## Umgesetzt (Commit `BATCH-4: …`, siehe `git log --grep BATCH-4`)

| Datei | Inhalt |
|---|---|
| `crates\fp-batch\src\steps\geometry.rs` | `resize` (percent/width/height/longEdge/shortEdge/fit/fill/exact/none, `onlyShrink`, Filter nearest/bilinear/bicubic/lanczos3/auto, `dpi`), `crop` (aspect 1:1…9:16/frei, rect/edges in px oder %, 9 Anker), `trim` (transparent/Eckfarbe/beides, Toleranz, Polster), `rotate` (90/180/270 verlustfrei, freier Winkel bikubisch/bilinear mit Hintergrund, Spiegeln, `autoExif`), `canvas` (addBorder je Seite, toSize, toAspect, square, Farbe, Anker). Eigener Resampler (getrennt, vormultipliziertes Alpha, Kern beim Verkleinern verbreitert, zeilenweise parallel, ohne Zwischenpuffer). Gemeinsame Hilfen (Vorschau-Maßstab, Größenprüfung, Anker, Mischen). |
| `src\steps\adjust.rs` | `adjust.auto` (Weißabgleich Grauwelt, Auto-Tonwert über `levels_math` mit **einem** Histogramm über alle Frames, Auto-Kontrast 0,5 %), `adjust.basic` (Helligkeit/Kontrast und Farbton/Sättigung über die Register-Filter von fp-raster, dazu Gamma, Temperatur, Tönung), `adjust.levels`, `adjust.curves` (P2, auf Wunsch mit erledigt: RGB-Regler/Punkte oder alle Kanäle als JSON), `adjust.grayscale` (grau/Sepia/Duoton), `adjust.invert`, `filter.sharpen` (Unscharf maskieren), `filter.blur` (Gauß/Bewegung/radial/Zoom), `filter.registry` (jeder `filter.*`-Befehl des Registers mit Parametern), `effect.live` (10 Effekte über `fp-effects`, Leinwand erweitern an/aus). |
| `src\steps\script.rs` | `script.registry` (ein oder mehrere Register-Befehle als ein Schritt, `selectAll`), `script.command` (gespeicherter `.jsf` aus dem Befehlsordner bzw. Pfad, über `commands.runScript` mit JSF-Laufzeit); eigene headless Sitzung je Element (gemeinsames Register, einmal je Prozess gebaut); Dokumente bleiben Dokumente; gesperrt: `batch.`, `file.`, `commands.`, `app.`, `help.`, `window.`; Ausschluss-Markierungen aus `re\21-batch.md` §4.2; `batch_commands()` (Liste für die Galerie). |
| `crates\fp-i18n\locales\{de,en}\batch_steps.ftl` | Alle Texte: Schrittnamen + Beschreibung, alle Parameter, Auswahlwerte, Zusammenfassungen, Hinweise/Fehler. |
| Tests `crates\fp-batch\tests\steps_geometry.rs` (26), `steps_adjust.rs` (15), `steps_script.rs` (6), Hilfen `tests\batch4_common\` | alle grün. |

Prüfungen: `cargo test -p fp-batch` (alle 9 Testprogramme grün, auch BATCH-1/-5), `cargo clippy -p fp-batch --all-targets`
ohne Warnungen in fp-batch, `cargo check --workspace --all-targets`, `cargo test -p fp-i18n`, `cargo test -p fp-neutral`,
Scanner `fp-ui --test all i18n_keine_festen_texte::` grün.

### Testfälle (Golden + §4b)
- Golden-Hashes (FNV über Maße+Pixel): Resampling Lanczos/Bikubisch/Bilinear/Nächster (verkleinern/vergrößern),
  Farbe (auto, basic, Sepia, Schärfen, Gauß, Schlagschatten). Größen aller Modi aller Geometrie-Schritte.
- Gleichheit mit den Register-Filtern (Helligkeit/Kontrast, Farbton/Sättigung, Unscharf maskieren, Umkehren über `filter.registry`).
- Vorgaben = **bitgleich** unverändert; einfarbig bleibt einfarbig; kein dunkler Saum an transparenten Kanten.
- 0×0 durch alle Geometrie-Schritte; riesige Ziele (32768², Rand 10000, 10000 %) → Fehler „zu groß“, Element unverändert, keine Panik.
- Animation: alle Frames gleiche Größe/Ausschnitt (trim: Vereinigung; effect.live: gemeinsame Grenzen), Verzögerungen bleiben.
- Wiederholung: Parameter 50× ändern → ein Schritt, Ergebnis = Endwert; Effekt-Vorschau 50× geändert und zurück = gleich (kein doppelter Effekt); viermal 90° = Ausgangsbild.
- Vorschau: Pixelparameter werden mit dem Proxy-Maßstab gerechnet (2048×1024 → Vorschau 900×500, hochgerechnet 1800×1000 = volles Ergebnis).
- Determinismus 1 vs. 8 Threads (Resampling, freie Drehung, Farbe, Effekte).
- EXIF: alle 8 Ausrichtungen, nie doppelt gedreht (s. u.).
- Echter Lauf (Executor): PNG + GIF-Animation → drehen, verkleinern, Rand; Original unverändert.

## Abstimmung mit BATCH-5 (Hinweis des Koordinators)
BATCH-5 wendet die EXIF-Ausrichtung schon beim Laden an (`meta::read_into`, Merker `meta.orientationApplied`, EXIF danach 1).
`rotate.autoExif` ruft deshalb nur noch `crate::meta::apply_orientation` auf (gleiche Rechnung wie `metadata.orientation`):
nach dem Laden ohne Wirkung, nie doppelt; die Datei wird nicht erneut gelesen (sonst Doppeldrehung, wenn `metadata.edit` die
EXIF-Daten vorher entfernt hat). `geometry::EXIF_APPLIED_KEY` = `meta::EXTRA_ORIENTATION_APPLIED`.

## Annahmen
- Texte in eigener Datei `batch_steps.ftl` (wie SPEC §6.1 und BATCH-1-Bericht), nicht in `batch.ftl` – vermeidet Konflikte mit parallelen Paketen.
- Neue Schlüsselform für Auswahlwerte: `batch-value-<id>-<param>-<wert>` (für das generische Formular von BATCH-2/11); Zusammenfassungen `batch-summary-<id>-…`.
- `resize`: Vorgabe-Modus `longEdge` 1600 px, `onlyShrink` an; `auto` = Lanczos3, bei Pixelgrafik (≤ 512 px, ≤ 256 Farben) nächster Nachbar, einmal je Element entschieden. `fill` mit „nur verkleinern“ schneidet auf das Rahmen-Seitenverhältnis zu, ohne zu vergrößern.
- `crop` rect/edges komplett außerhalb → Warnung, Bild unverändert; teilweise außerhalb → beschnitten.
- `trim` mit Polster wächst bei Bedarf über das Bild hinaus (transparent bzw. Eckfarbe).
- `canvas`: Hintergrundfarbe liegt hinter dem ganzen Bild (halbtransparente Pixel werden gemischt), wie die Leinwandfarbe im Dokument.
- `rotate`: positive Winkel = im Uhrzeigersinn; Vorgabe 90°.
- `adjust.basic`: Temperatur/Tönung je bis ±40 Tonwertstufen (Rot/Blau bzw. Grün); Gamma per Tabelle.
- `effect.live`: Parameter im Dateiformat des Effekts (`{"ShadowBlur": "4"}`), Vorgaben der Effekt-Engine; Bevel-Familie setzt `BevelType` aus dem Namen. Direkt über `fp-effects` (dieselbe Rechnung, die `element.addEffect` + Rendern ergibt), ohne Sitzung.
- `filter.registry`/`script.*`: Einzelbilder/Frames werden als Dokument mit einer Bitmap gerechnet und danach die Leinwand flach gerechnet; seit BM-1 fallen dabei Pixel mit Alpha 1 weg (Grenzen ab Alpha 2).
- `script.command`: fehlende Datei = Warnung bei der Prüfung (Rezept kann von anderem Rechner stammen), Fehler beim Ausführen.

## Neue Abhängigkeiten (eigene Crates, Apache-2.0)
`fp-effects`, `fp-script` in `crates\fp-batch\Cargo.toml` (2 Zeilen + Kommentar). `Cargo.lock` entsprechend.

## Einhängungen in fremde Dateien (Befunde)
- `crates\fp-batch\src\steps\mod.rs`: je 3 Zeilen `pub mod …; // BATCH-4` und `…::register(r); // BATCH-4`.

## Neue Befunde
- (für BATCH-1/Kern) `StepContext` sollte den Vorschau-Maßstab tragen (`preview_scale`). Bis dahin bestimmt BATCH-4 ihn aus `ctx.info` und dem Proxy und merkt ihn in `meta.extra["previewScale"]`; Schritte anderer Pakete, die die Größe vor dem ersten BATCH-4-Schritt ändern, verfälschen den Wert.
- (für BATCH-1/Kern) `{w}`/`{h}` im Namen nutzen die Quellgröße; nach `resize`/`crop` weicht die Ausgabegröße ab (offener Punkt aus batch1.md, jetzt relevant).
- (für BATCH-2/11) Parameter-UI: Formular aus `params()`; Auswahltexte über `batch-value-<id>-<param>-<wert>`; Felder je `mode` ein-/ausblenden (z. B. `resize.size` nur bei longEdge/shortEdge, `crop.x/y/width/height` nur bei rect); `filter.registry.params` als Formular aus `describe_command` des gewählten Befehls; `script.command.name` als Auswahl aus `steps::script::batch_commands()`.
- (für I18N/HG) `crates\fp-ui\tests\i18n_keine_festen_texte.rs` → `SCAN_DIRS` um `crates/fp-batch/src` ergänzen (schon von BATCH-1 gemeldet).
- `effect.live` rechnet Effekt-Pixelwerte (Abstand, Weichheit) in der Vorschau nicht mit dem Proxy-Maßstab um → Schatten wirkt in der Vorschau großer Bilder relativ größer.
- Golden-Hashes des Resamplers hängen an `f64::sin` (Lanczos); auf anderen Betriebssystemen (CI später) ggf. neu erzeugen.

## Zeilen für PARITAET („umgesetzt, Nachprüfung ausstehend“)
Batch-Studio-Schritte `resize`, `crop` (ohne smart), `trim`, `rotate` (+EXIF), `canvas`, `adjust.auto/basic/levels/curves/grayscale/invert`, `filter.sharpen/blur/registry`, `effect.live`, `script.registry`, `script.command`.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)
- CLI: `firepixel-cli batch --recipe web.json --in <Fotos>` mit `resize longEdge 1600` + `filter.sharpen` + JPEG → Ausgaben ≤ 1600 px, scharf, Hochformat-Handyfotos stehen richtig (EXIF), nichts doppelt gedreht.
- `effect.live` dropShadow auf transparente PNG-Logos → Schatten sichtbar, Leinwand gewachsen; mit „Leinwand erweitern“ aus gleiche Größe.
- `script.command` mit einem gespeicherten Befehl aus `%APPDATA%\Firepixel\Commands` (z. B. „90° drehen“) und einem mit `//--not for batch--` (muss abgelehnt werden).
- Vorschau im Studio (sobald BATCH-2 da ist): Rand/Größe in der Vorschau wie im Ergebnis; GIF-Animation verkleinern behält Zeiten.
