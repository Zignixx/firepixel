# BATCH-8 – GIF-Optimierer „kleine GIFs, sehr gute Qualität“

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §3.10, §5.6 (Paket BATCH-8 aus §6.1).
Eigene Umsetzung ohne GPL/AGPL/LGPL-Bausteine (kein gifski, gifsicle/giflossy, libimagequant,
zenquant, auch kein `quantette`): **keine neue Abhängigkeit**.

## Umgesetzt (Commit `BATCH-8: GIF-Optimierer …`)

Modul `crates\fp-batch\src\steps\gifopt\`:

| Datei | Inhalt |
|---|---|
| `mod.rs` | Einhängen als Kodierer für „Speichern als: GIF“ (`output::register_encoder`, Standbild und Animation, Dokumente flach), Optionen aus `recipe.output.options` (`GifOptions`: `preset` smallest/balanced/best → Qualität 60/80/100, `quality` 1…100, `maxKB`, `colors`, `palette`, `dither`, `ditherAmount`, `lossy`, `diffTransparency`, `diffThreshold`, `cropChanged`, `mergeSimilar`, `disposal` auto/keep, `transparency`, `background`), Qualität steuert Farben (2^(3+5q/100) → 128 bei 80), Verlust ((100−q)·2 → 40 bei 80), Dithering-Stärke, Glättung gemeinsam; Endkarte `output.gif` (nur Schema für das Formular), Schritt `anim.stabilize` („Ruhiger Hintergrund“, §3.9); öffentliche API `optimize()`, `encode_item()`, `encode_source()`, `Optimized {bytes, stats, quality, scale, frame_step, reached, encodes}` |
| `color.rs` | Oklab (Ottosson), sRGB-Tabelle, schneller Farb-Hasher |
| `palette.rs` | gewichtete Farbstatistik (statische Pixel 0,1, Kanten bis 2×; über 60 000 Farben gröbere Fächer 6/5 Bit), ≤ k Farben → exakt übernommen, sonst Median-Cut mit varianzminimierender Teilung (Wu-artig) + k-Means (Lloyd, 6 Runden) in Oklab |
| `dither.rs` | none / ordered (Bayer 8×8) / blueNoise / temporal / floydSteinberg (Serpentine, Oklab); geordnete Arten als Zwei-Farben-Mischung (Mittelwert stimmt); `temporal`: feste 64×64-Blue-Noise-Matrix (Void-and-Cluster nach Ulichney 1993, eigen erzeugt, Datei `bluenoise64.bin`, Test prüft Gleichheit mit dem Generator), Stärke an Kanten/Textur bis 85 % schwächer; statische Pixel übernehmen die Vorframe-Indizes |
| `frames.rs` | Vorverarbeitung: Alpha → 1 Bit bzw. Matte, Bildrate ausdünnen (Dauer addiert), fast gleiche Frames zusammenlegen, zeitliche Glättung nur wo die ganze 3×3-Umgebung ruhig ist (Rauschen auf stehendem Grund wird eingefroren, Kamerafahrten verschmieren nicht; gleichgerichtete kleine Änderungen = Blende, nicht geglättet); Planung je Frame gegen das sichtbare Bild: unverändert → Transparenz (oder exakt dieselbe Farbe, je nachdem, was den LZW-Code verlängert), Zuschnitt auf das Rechteck der Änderungen |
| `lzw_lossy.rs` | GIF-LZW (Trie, Neustart bei 4096) mit verlustbehafteter Verlängerung: Folge-Index wird akzeptiert, wenn seine Farbe ≤ Schwelle (lossy/200·0,09 Oklab) von der gewünschten liegt, Fehler halb nach rechts/unten weitergegeben; unveränderte Pixel nur transparent oder exakt gleich (kein Wandern), geänderte nie zurück auf die bisher sichtbare Farbe (eine langsame Blende bleibt erhalten); bei mehreren gleichwertigen Fortsetzungen entscheidet eine Vorausschau (≤ 48 Pixel) |
| `encoder.rs` | Paletten global / perFrame / auto (Szenenwechsel an der groben Farbverteilung, eigene Palette nur bei deutlich kleinerem Fehler; erste Szene bestimmt die globale), Wahl der Entsorgung je Frame nach kleinerer kodierter Größe (keep/background, Rechteck des Vorframes wird bei Bedarf vergrößert und mit denselben Pixeln neu kodiert), GIF89a-Schreiber (GCT/LCT, NETSCAPE-Schleife, Dauer mit Rundungsübertrag, min. 2 cs) |
| `size.rs` | Zielgröße: volle Kodierung bei der Ausgangsqualität, dann Qualität 10, Bisektion; reicht das nicht: halbe Bildrate, dann Größe (Faktor aus dem Größenverhältnis, mind. −10 %, bis 0,1); höchstens 8 Kodierungen; `reached` meldet, ob erreicht |
| `metrics.rs` | SSIM auf Oklab-L (8×8, Schritt 4), PSNR RGB |

Dazu: `crates\fp-batch\src\steps\mod.rs` (2 Zeilen), Texte `batch.ftl` de+en (Präfixe
`batch-step-output-gif`, `batch-param-output-gif-*` inkl. Auswahlwerte, `batch-step-anim-stabilize`,
`batch-param-anim-stabilize-threshold`, `batch-summary-anim-stabilize`), Golden-Werte
`tools\golden\batch\gif\golden.json`.

## Tests (alle grün)

- `cargo test -p fp-batch --lib gifopt` (12): Oklab-Rundreise, Palette exakt/Verlauf gleichmäßig/
  Vergröberung, Blue-Noise-Datei = Generator + Permutation + wenig Klumpen, Mischung hält den
  Mittelwert (alle Arten), LZW verlustfrei (70 000 Indizes, 256 Farben, Neustart), Transparenz-
  Gleichwertigkeit verkürzt, Verlust hält die Schwelle, SSIM/PSNR-Ordnung, Glättung/Zusammenlegen.
- `tests\gifopt.rs` (14, Rundreise über den GIF-Dekodierer von fp-io): verlustfreie Einstellungen
  Pixel für Pixel gleich (Pixelart, Freisteller mit Alpha, Bildschirm; je Palettenart), Differenz +
  Zuschnitt (> 2× kleiner, kleine Rechtecke), Alpha-Animation mit Entsorgung „Hintergrund“ exakt,
  statische Bereiche flimmern nicht (4 Dithering-Arten), Verlust kleiner bei SSIM > 0,8/0,9,
  Palette auto nur bei Szenenwechsel, Zielgröße erreicht/gemeldet (≤ 8 Kodierungen, Rauschen
  unerreichbar), Determinismus, **Werte 50× setzen = einmal**, Voreinstellungen ordnen sich nach Größe,
  Zusammenlegen hält die Dauer, langsame Blende bleibt (alle Voreinstellungen), Grenzfälle (1×1, 1 Frame, Frames anderer Größe, leere Animation →
  Fehler, Matte), Registrierung + Lauf über `executor::run` mit `anim.stabilize` + `maxKB`,
  `anim.stabilize` 50× = einmal, **Golden** (3 Clips × 3 Voreinstellungen: Größe ± 5 %, SSIM ≥ − 0,005;
  `FP_BLESS=1` schreibt neu).
- `tests\gifopt_measure.rs` (`#[ignore]`, eigener Prozess, damit der bisherige Export erreichbar ist):
  `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture`.
- Tests von BATCH-1/4/5/6/7 in fp-batch weiter grün (GIF läuft jetzt über den Optimierer; `anim_video` deckte das Verschlucken kleiner Blenden auf, behoben);
  `cargo check --workspace --all-targets`, `fp-i18n`, `fp-neutral`, `clippy -p fp-batch --all-targets`
  ohne Warnungen in eigenen Dateien. i18n-Scanner `fp-ui --test all i18n_keine_festen_texte::` und `batch_ui::` grün (18 Tests).

## Messung (3 selbst erzeugte Animationen, 320×240, 24 Frames)

Bezug „Export (bisher)“ = `fp-export` (Auto-Crop, Differenz-Transparenz, 256 Farben), SSIM/PSNR über
die Zeit gemittelt (Debug-Build mit opt-level 1, Zeiten nur relativ).

| Clip | Kodierer | KB | vs. Export | SSIM | PSNR dB |
|---|---|---:|---:|---:|---:|
| Bildschirm | Export (bisher) | 9,6 | ±0 % | 1,0000 | ∞ |
| Bildschirm | einfach (globale Palette, FS, ohne Differenz/Verlust) | 41,2 | +327 % | 1,0000 | ∞ |
| Bildschirm | best / balanced / smallest | 9,7 | +0 % | 1,0000 | ∞ |
| Bildschirm | maxKB = 60 % | 4,7 | −51 % | (Größe 0,59) | – |
| Kamerafahrt | Export (bisher) | 1098,8 | ±0 % | 0,9627 | 38,4 |
| Kamerafahrt | einfach | 1217,6 | +11 % | 0,9826 | 37,3 |
| Kamerafahrt | best | 1211,1 | +10 % | 0,9832 | 37,4 |
| Kamerafahrt | **balanced** | 861,9 | **−22 %** | 0,9632 | 35,1 |
| Kamerafahrt | smallest | 628,9 | −43 % | 0,9003 | 31,7 |
| Kamerafahrt | maxKB = 60 % | 657,1 | −40 % | 0,9109 | 32,1 |
| Kamerafahrt | gleiche SSIM wie Export (Qualität 80 = balanced) | 861,9 | **−22 %** | 0,9632 | 35,1 |
| Zeichentrick | Export (bisher) | 10,8 | ±0 % | 0,9962 | 49,6 |
| Zeichentrick | einfach | 147,1 | +1258 % | 0,9999 | 60,4 |
| Zeichentrick | best | 14,7 | +35 % | 0,9998 | 60,1 |
| Zeichentrick | **balanced** | 9,4 | **−13 %** | 0,9973 | 49,3 |
| Zeichentrick | smallest | 8,0 | −26 % | 0,9923 | 43,6 |
| Zeichentrick | maxKB = 60 % | 5,8 | −46 % | 0,9609 | 35,8 |
| Zeichentrick | gleiche SSIM wie Export (Qualität 78) | 9,6 | **−12 %** | 0,9963 | 48,2 |

Bewertung: Gegenüber einem einfachen Kodierer ist der Optimierer 4× (Bildschirm) bis 15×
(Zeichentrick) kleiner. Der bisherige Export ist schon ein guter Bezug (er hat Auto-Crop und
Differenz-Transparenz); bei **gleicher SSIM** ist der Optimierer 12–22 % kleiner, bei rein flachen
Bildschirminhalten gleich groß (verlustfrei, beide gleich viele Bytes je Frame). Das Spezifikationsziel
„balanced ≥ 30 % kleiner bei gleicher oder besserer SSIM“ ist damit **nicht erreicht** (balanced:
−13 % mit besserer SSIM beim Zeichentrick, −22 % mit gleicher SSIM bei der verrauschten Kamerafahrt).
„best“ ist bewusst größer als der Export (feinere Palette, PSNR +10 dB beim Zeichentrick).
Hinweis: SSIM auf L bewertet Banding kaum und bestraft Dithering-Rauschen; ohne Dithering lägen SSIM
und Größe bei den Verläufen etwas günstiger, das Bild hätte dann aber sichtbare Stufen.

## Annahmen

- Bericht als `docs\berichte\batch8.md` (Auftrag); SPEC §5.6 nennt `batch-gifopt.md` – gleiche Inhalte.
- Optionen stehen in `recipe.output.options`; die Endkarte „Speichern als“ (BATCH-1, `files.rs`) wurde
  nicht geändert. Das Schema der GIF-Optionen steht in der eigenen Endkarte `output.gif` (Art `Final`);
  die Oberfläche (BATCH-2/11) kann es bei Format GIF einblenden.
- Gesetzte Werte gelten als ausdrücklich; fehlende folgen `preset`/`quality`. Bei der Zielgröße werden
  auch ausdrücklich gesetzte Werte unter der Ausgangsqualität verschärft (Verlust +2 je Stufe).
- `diffThreshold` (×100 Oklab, Vorgabe 3) wird mit der Qualität skaliert (0,013 bei 100, 0,021 bei 80,
  0,03 bei 60), weil 0,03 bei „balanced“ Kamerafahrten sichtbar verschmiert.
- Zielgröße: jede Kodierung voll statt Schätzung auf jedem 4. Frame (ausgedünnte Frames haben größere
  Differenzen, die Schätzung wäre zu hoch); dafür höchstens 8 Kodierungen. Skalieren bikubisch
  (`fp_raster::resample`) statt Lanczos.
- Animiertes APNG/WebP: laut §6.1 Teil von BATCH-7 (Ausgabe) – hier nicht angefasst; die Bausteine
  (`frames::prepare`, Differenz-Rechteck) sind öffentlich und können dort genutzt werden.
- Testkorpus: 3 Mess-Clips (Bildschirm, Kamerafahrt, Zeichentrick) + Pixelart, Freisteller,
  Szenenwechsel werden im Test deterministisch erzeugt (`tests\gifopt_clips`), keine Bilddateien im
  Repository; `tools\golden\batch\gif\golden.json` hält die Golden-Werte.
- Letztes Frame einer Animation mit Transparenz: Entsorgung „Hintergrund“ (sauberer Schleifenbeginn).

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-batch\src\steps\mod.rs`: `pub mod gifopt;` + `gifopt::register(r);` (2 Zeilen).
- `crates\fp-i18n\locales\{de,en}\batch.ftl`: eigener Block „GIF-Optimierer (BATCH-8)“.
- Für BATCH-2/11: bei Format GIF die Felder der Endkarte `output.gif` zeigen (Anfänger: `preset`,
  `quality`, `maxKB`), „erreichte Größe“ bzw. ⚠ „Zielgröße nicht erreichbar“ aus `gifopt::optimize()`
  (`Optimized.reached`) anzeigen – der Kodierer-Haken von BATCH-1 gibt nur Bytes zurück, keine Warnung.
- Für BATCH-5 (`optimize.targetSize`): für GIF kann `gifopt::size::encode_to_target` direkt genutzt
  werden (Qualität → Bildrate → Größe).

## Offene Punkte

- Spezifikationsziel −30 % bei gleicher SSIM nicht erreicht (siehe Messung); Ansätze: optimale statt
  gierige LZW-Zerlegung, Palette nach Wahrnehmung gewichtet (Hauttöne), Messung mit echtem
  Videomaterial (Korpus aus 5 Clips laut §5.6 fehlt: Verlauf und Pixelart nur als Logikfälle).
- Laufzeit: die Kamerafahrt 320×240×24 braucht im Debug-Build ≈ 1,7 s je Kodierung (Zielgröße bis 8×);
  keine Parallelisierung je Frame (Palette/Dithering wären parallelisierbar).
- Kein Abbrechen innerhalb einer Kodierung (der Kodierer-Haken hat kein `CancelToken`).

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Batch-Studio, Rezept „Video → kleines GIF“ bzw. GIF → GIF mit `preset` smallest/balanced/best und
  `maxKB`: Ergebnis in Browser (Chrome/Firefox/Edge) und in Firepixel abspielen – keine Reste bei
  Alpha-Animationen, kein Flimmern im stehenden Hintergrund, Schleife korrekt, Dauer gleich.
- Messung wiederholen: `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture`
  (Release-Build für Zeiten).
