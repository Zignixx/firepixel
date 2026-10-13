# BATCH-8b – GIF-Optimierer: „balanced“ ≥ 30 % kleiner bei gleicher SSIM

Stand 13.10.2026. Auftrag: Ziel aus `docs\BATCH_MODERN.md` §5.6 („balanced“ mindestens 30 % kleiner als
der bisherige Export bei gleicher SSIM) auf allen drei Testclips erreichen. Vorgänger: `batch8.md`
(−22 % / −13 % / ±0 %). Eigene Umsetzung, keine neue Abhängigkeit, nichts aus GPL/AGPL/LGPL.

## Ergebnis (balanced gegen den bisherigen Export, 320×240, 24 Frames)

| Clip | Export | balanced vorher | balanced jetzt | SSIM Export → jetzt |
|---|---:|---:|---:|---|
| Bildschirm | 9 879 B | 9 882 B (±0 %) | **4 510 B (−54,3 %)** | 1,0000 → 1,0000 (über alle 10-ms-Zeitpunkte 0,9988) |
| Kamerafahrt | 1 125 139 B | 882 569 B (−21,6 %) | **774 955 B (−31,1 %)** | 0,9627 → 0,9641 |
| Zeichentrick | 11 093 B | 9 642 B (−13,1 %) | **7 546 B (−32,0 %)** | 0,9962 → 0,9966 |

Das Ziel ist auf allen drei Clips erreicht. Bei gleicher SSIM gesucht (kleinste Qualität mit SSIM ≥
Export): Zeichentrick −33,1 % (Qualität 78), Kamerafahrt −31,1 % (Qualität 80).

## Umgesetzt (Commit `BATCH-8b: …`)

Alles in `crates\fp-batch\src\steps\gifopt\`:

| Verfahren | Datei | Wirkung (gemessen) |
|---|---|---|
| **Teilbilder bei getrennten Änderungen** („Bildschirmaufnahmen gezielt behandeln“): Änderungen eines Frames in 8×8-Zellen, Zusammenhang, Bereiche näher als 24 px bleiben zusammen (altes + neues Bild eines bewegten Objekts wird nie getrennt), Zusammenlegen nach geschätzten Kosten (Kopf + √(2·Fläche) LZW-Codes), höchstens 4 Teile. Die vorderen Teile stehen je 20 ms (kürzeste von Browsern beachtete Dauer) **vor** dem Bildwechsel, die Zeit kommt vom Vorframe; das letzte Teil zur Zeit des Frames → zu jedem Frame-Zeitpunkt ist das Bild exakt, Gesamtdauer gleich. Nur ohne Alpha, mit „nicht entsorgen“, Zuschnitt an, und wenn die Teile zusammen ≥ 10 % kleiner sind. Neue Option `splitFrames` (Vorgabe: an unter Qualität 95, „best“ teilt nie). | `frames.rs` (`change_regions`, `plan_in`), `encoder.rs` (`split_parts`) | Bildschirm −54 % (Grund: Ein fast leeres Rechteck über Zeiger, Eingabezeile und Balken kostet ≈ √(2·Fläche) Codes nur für die Transparenz) |
| **Inhaltsabhängige Voreinstellung**: Foto-Anteil aus verschiedenen Farben je Pixel des ersten Frames (≤ 2 % Grafik, ≥ 10 % Foto, < 256 Farben immer Grafik). Grafik: wenige Farben (balanced 44), wenig Verlust ((100−q)·2), Helligkeit in der Palette 1,5-fach gewichtet; Foto: mehr Farben (balanced 92, sonst Farbflecken), mehr Verlust ((100−q)·5), Gewicht 1 (farbtreu). Ausdrücklich gesetzte `colors`/`lossy` gelten weiter genau. | `mod.rs` (`quality_colors`, `quality_lossy`), `encoder.rs` (`photo_share`, `GifSettings::for_content`) | erst damit beide gleichzeitig ≥ 30 %: Zeichentrick braucht ≈ 44 Farben, die Kamerafahrt ≈ 92 (mit 44 Farben SSIM 0,9584 < 0,9627) |
| **Palette mit gewichteter Helligkeit** (Median-Cut + k-Means im Raum mit L·w, Farbsuche gleich) und Statistik ohne Sondergewichte (jedes Pixel jedes Frames zählt gleich statt „statisch 0,1, Kanten 2×“) | `palette.rs` (`build_weighted`, `Palette::weighted`), `color.rs` (`wdist2`), `encoder.rs` | Zeichentrick mit 48 Farben: SSIM 0,9952 → 0,9969 |
| **Wahrnehmungsgewichteter Verlust**: Toleranz des verlustbehafteten LZW mit 3-fach gewichteter Helligkeit (Farbton darf mehr abweichen als Helligkeit) | `lzw_lossy.rs` (`LOSSY_L_WEIGHT`) | Kamerafahrt, 56 Farben, Verlust 80: SSIM 0,9247 → 0,9682 (Größe 646 → 741 KB; bei gleicher SSIM insgesamt ≈ 2,5 % kleiner als mit weniger Farben) |
| **Verlust macht nie größer**: je Teilbild zusätzlich ohne Verlust kodiert, das kleinere zählt | `encoder.rs` | Flächen/Verläufe: vorher mit Verlust bis +1 % größer |
| **Totzone beim zeitlich stabilen Dithering**: kein Mischen, wenn die nächste Palettenfarbe < 0,01 (Oklab) neben der Quelle liegt, weicher Anstieg bis 0,03 → ruhige Flächen, lange LZW-Läufe; Dithering nur, wo Stufen sichtbar wären. Unter Qualität 50 weniger, ab 10 kein Dithering (sonst würde die Datei mit sinkender Qualität größer). | `dither.rs`, `mod.rs` | Zeichentrick, 48 Farben, `temporal`: 15 508 → 7 954 B bei SSIM 0,9947 → 0,9969 |
| **Farbreihenfolge in der Datei**: Transparenz auf Index 0, Farben nach Häufigkeit in den geänderten Bereichen; jedes Teilbild bekommt die kleinste LZW-Mindest-Codelänge, die seine Indizes braucht (kürzere Codes ohne eigene Farbtabelle) | `encoder.rs` (`FileOrder`) | Zeichentrick −2,3 %, Bildschirm −0,5 % |
| **Zielgröße schneller**: nächste Qualität aus ln(Größe) zwischen den einschließenden Kodierungen geschätzt (Regula falsi mit Illinois-Schritt), Ende bei ≥ 96 % des Ziels | `size.rs` | `maxKB` = 60 %: Kamerafahrt 8 → 6 Kodierungen, Zeichentrick 8 → 6 bei SSIM 0,9609 → 0,9889 (vorher Qualität 19, jetzt 49) |
| **Messgrößen**: Farbtreue in der Fläche (`metrics::color_error`: Oklab-Abstand der 8×8-Mittelwerte ×100 – sieht Farbflecken, die SSIM auf L nicht sieht) und SSIM über alle 10-ms-Zeitpunkte (Testhilfe `full_ssim`) | `metrics.rs`, `tests\gifopt_clips\mod.rs` | – |

Schon vorhanden und unverändert genutzt: Entfernen doppelter Frames / Zusammenfassen bei kaum Änderung
(Zeiten addiert), zeitliche Glättung, Differenz → Transparenz, Szenenwechsel → eigene Palette.

Dazu: `splitFrames` in der Endkarte `output.gif` (Schema), Texte `batch.ftl` de/en
(`batch-param-output-gif-splitFrames`), Golden-Werte `tools\golden\batch\gif\golden.json` neu,
Messung `tests\gifopt_measure.rs` mit Bytes, SSIM (12 Stichproben und alle 10 ms), Farbtreue,
Foto-Anteil/Farben/Verlust/Teilbilder.

## Messung vorher/nachher je Clip und Qualitätsstufe

Aufruf: `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture` (Debug-Build mit
opt-level 1, Zeiten nur relativ). „Farbe“ = Farbtreue in der Fläche (kleiner = besser).

| Clip | Stufe | vorher B (vs. Export) | vorher SSIM / PSNR | jetzt B (vs. Export) | jetzt SSIM / SSIM alle 10 ms / Farbe / PSNR |
|---|---|---:|---|---:|---|
| Bildschirm | Export | 9 879 | 1,0000 / ∞ | 9 879 | 1,0000 / 1,0000 / 0,000 / ∞ |
| Bildschirm | best | 9 882 (±0 %) | 1,0000 / ∞ | 9 882 (±0 %) | 1,0000 / 1,0000 / 0,000 / ∞ |
| Bildschirm | balanced | 9 882 (±0 %) | 1,0000 / ∞ | 4 510 (−54,3 %) | 1,0000 / 0,9988 / 0,000 / ∞ |
| Bildschirm | smallest | 9 882 (±0 %) | 1,0000 / ∞ | 4 510 (−54,3 %) | 1,0000 / 0,9988 / 0,000 / ∞ |
| Kamerafahrt | Export | 1 125 139 | 0,9627 / 38,44 | 1 125 139 | 0,9627 / 0,9627 / 0,166 / 38,44 |
| Kamerafahrt | best | 1 240 166 (+10 %) | 0,9832 / 37,44 | 1 186 429 (+5,4 %) | 0,9863 / 0,9863 / 0,165 / 38,19 |
| Kamerafahrt | balanced | 882 569 (−21,6 %) | 0,9632 / 35,06 | 774 955 (−31,1 %) | 0,9641 / 0,9642 / 0,332 / 33,17 |
| Kamerafahrt | smallest | 643 994 (−43 %) | 0,9003 / 31,65 | 608 766 (−45,9 %) | 0,9048 / 0,9058 / 0,562 / 30,13 |
| Kamerafahrt | maxKB = 60 % | 672 870, 8 Kodierungen | 0,9109 / 32,09 | 665 495, 6 Kodierungen | 0,9338 / 0,9345 / 0,469 / 31,28 |
| Zeichentrick | Export | 11 093 | 0,9962 / 49,64 | 11 093 | 0,9962 / 0,9962 / 0,101 / 49,64 |
| Zeichentrick | best | 15 053 (+35 %) | 0,9998 / 60,13 | 13 112 (+18,2 %) | 1,0000 / 1,0000 / 0,004 / 63,59 |
| Zeichentrick | balanced | 9 642 (−13,1 %) | 0,9973 / 49,28 | 7 546 (−32,0 %) | 0,9966 / 0,9966 / 0,079 / 47,90 |
| Zeichentrick | smallest | 8 192 (−26 %) | 0,9923 / 43,56 | 6 233 (−43,8 %) | 0,9908 / 0,9908 / 0,156 / 42,98 |
| Zeichentrick | maxKB = 60 % | 5 939, 8 Kodierungen | 0,9609 / 35,79 | 6 011, 6 Kodierungen | 0,9889 / 0,9889 / 0,216 / 40,66 |

(„vorher“ ohne genaue Bytezahl: aus den KB-Werten von `batch8.md` zurückgerechnet, ±50 B.)

## Bewertung, ehrlich

- **Bildschirm**: Die Frames sind verlustfrei; zu jedem Frame-Zeitpunkt ist das Bild Pixel für Pixel
  gleich. Teiländerungen (z. B. Fortschrittsbalken, Eingabezeile) erscheinen bis zu 40 ms **vor** dem
  Bildwechsel – daher SSIM über alle 10-ms-Zeitpunkte 0,9988 statt 1. Wer exakte Zeiten braucht:
  `splitFrames: false` oder „best“ (dann ±0 % wie bisher).
- **Kamerafahrt (Korn/Video)**: Gleiche SSIM, aber Farbtreue in der Fläche 0,33 gegen 0,17 beim Export
  und PSNR 33,2 statt 38,4 dB: Der Export hat 256 Farben, „balanced“ 91 Farben + Verlust im Korn.
  Sichtprüfung an Ausschnitten (3-fach vergrößert, Frame 5): Farbflächen bleiben erhalten, das Korn ist
  etwas gröber. Verworfen wurde eine Variante mit 48 Farben und gewichteter Helligkeit (−36 % bei
  gleicher SSIM), weil sie sichtbare Farbflecken hatte (Farbtreue 0,44) – SSIM auf L sieht das nicht.
- **Zeichentrick**: gleiche SSIM, Farbtreue besser als der Export (0,079 gegen 0,101), PSNR −1,7 dB
  (Himmelsverlauf in etwas breiteren Stufen, wie im Export ohne Dithering; Sichtprüfung: vergleichbar).
- Die Reserve über dem Ziel ist klein (Kamerafahrt −31,1 %, Zeichentrick −32,0 %). Golden-Werte
  prüfen Größe ± 5 % und SSIM.

## Verworfen (gemessen, nicht übernommen)

| Versuch | Ergebnis |
|---|---|
| Wörterbuch bei voller Tabelle weiterverwenden („deferred clear“), ohne bzw. mit Neubeginn bei sinkender Rate | Kamerafahrt +101 % bzw. +3,6 %, andere gleich |
| Früherer Neubeginn (3072 Codes) | kein Gewinn |
| Flexible Zerlegung (LZW-FP: kürzeren Eintrag ausgeben, wenn der folgende weiter reicht) | Bildschirm +55 %, Kamerafahrt +3 %, Zeichentrick −0,8 % (verdirbt den Aufbau langer Läufe) |
| Fehlerverteilung Floyd–Steinberg im verlustbehafteten LZW | Farbtreue besser, aber 3–7 % größer |
| Palette: mehr k-Means-Runden (16 statt 6) | ±0,5 %, Rauschen |
| Feste 48 Farben + Helligkeit 1,5 für alle Inhalte | Kamerafahrt −36 % bei gleicher SSIM, aber Farbflecken (siehe oben) |

## Tests (§4b)

- `cargo test -p fp-batch --lib gifopt` (17): neu `change_regions_split_merge_and_limit` (getrennt /
  nah beieinander / Höchstzahl / nichts geändert), `temporal_deadzone_keeps_fine_gradients_calm`,
  `weighted_palette_spends_more_levels_on_lightness`, `lossy_is_stricter_on_lightness`,
  `quality_mapping_is_monotonic_and_content_aware` (Zuordnung je Qualität, ausdrückliche Farben,
  Teilbilder nur unter „best“); `lossy_respects_threshold` auf eine Farbton-Palette umgestellt.
- `tests\gifopt.rs` (17): neu `split_frames_are_exact_at_frame_times` (zu jedem Frame-Zeitpunkt exakt,
  Gesamtdauer gleich, Teile ≥ 20 ms, ≥ 20 % kleiner, „best“ und `splitFrames: false` teilen nicht,
  bewegtes Objekt nicht getrennt, **Option 50× umschalten = einmal**), `content_class_and_code_size`
  (Grafik/Foto, kleinere Codelänge für Teilbilder, ausdrückliche Farben gelten),
  `size_falls_with_quality` (Voraussetzung der Zielgröße), Zielgröße mit ≤ 6 Kodierungen;
  `lossy_makes_smaller_and_keeps_quality`: Verlust bei Flächen „nie größer“ statt „kleiner“.
  Golden neu geschrieben (`FP_BLESS=1`).
- Alle Tests von `fp-batch` grün (auch BATCH-1…7, `anim_video`); `cargo clippy -p fp-batch
  --all-targets` ohne Warnungen in gifopt; `cargo check --workspace --all-targets`; `fp-i18n`,
  `fp-neutral`; Scanner `fp-ui --test all i18n_keine_festen_texte::` und `batch_ui::` grün.

## Annahmen

- „Gleiche SSIM“ wie in `batch8.md`: mittlere SSIM auf Oklab-L über 12 Zeitpunkte. Zusätzlich berichtet:
  SSIM über alle 10-ms-Zeitpunkte und Farbtreue, weil SSIM auf L Farbfehler und kurze Zwischenzustände
  nicht sieht.
- Teilbilder erscheinen vor dem Bildwechsel (nicht danach), damit jeder Frame-Zeitpunkt exakt ist.
  Untergrenze 20 ms je Teilbild, weil Browser kürzere Dauern als 100 ms abspielen.
- Foto-Anteil nur aus dem ersten Frame (schnell, eindeutig); Szenenwechsel ändern die Einstufung nicht.
- Ausdrücklich gesetzte `colors`/`lossy` haben Vorrang vor der inhaltsabhängigen Wahl (wie bisher
  werden sie nur unter der Ausgangsqualität bei der Zielgröße verschärft).
- **Vorfall**: Ein Hilfsskript schrieb einmal versehentlich mit relativem Pfad in den Hauptbaum
  (`firepixel\crates\fp-batch\src\steps\gifopt\encoder.rs`, Umgebungsvariablen-Versuch). Sofort bemerkt,
  Änderung in den Slot übernommen und die Datei im Hauptbaum mit `git checkout --` auf den Stand von
  `0b7b087` zurückgesetzt (nur diese eigene Datei, kein anderer Agent betroffen, nichts committet).

## Befunde / Einhängungen

- `docs\BATCH_MODERN.md` §3.10/§5.6 (Doku-Agent): Optionstabelle um `splitFrames` (bool, an unter
  Qualität 95) ergänzen; Voreinstellungen jetzt inhaltsabhängig (balanced: Grafik 44 Farben/Verlust 40,
  Foto 92 Farben/Verlust 100; smallest: 24/80 bzw. 60/200; best: 256/0); `lossy` vergleicht mit
  dreifach gewichteter Helligkeit.
- Für BATCH-2/11 (Oberfläche): `splitFrames` ist ein Profi-Feld der Endkarte `output.gif`;
  `Optimized.stats.photo` / `stats.lossy` / `global_colors` könnten als „Erkannt: Grafik/Foto · N
  Farben“ angezeigt werden.
- PARITAET: keine Zeilen (eigene Funktion ohne Gegenstück im Original).

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Bildschirmaufnahme (z. B. Rezept „Small GIFs“ auf eine echte Aufnahme mit Mauszeiger und Tippen):
  Ergebnis in Chrome, Firefox, Edge und der Windows-Fotos-App abspielen – Zeiger, Eingabe und Balken
  laufen flüssig, keine Doppelbilder, keine Lücken, Gesamtdauer gleich; mit `splitFrames: false`
  vergleichen (gleiches Bild, größere Datei).
- Echtes Video (Kamerafahrt mit Korn) und Zeichentrick mit „balanced“: neben dem bisherigen Export
  ansehen – keine Farbflecken, keine sichtbar stärkeren Stufen im Himmel; Größe ≥ 30 % kleiner.
- `maxKB`: Ergebnis liegt knapp unter dem Ziel, Anzeige der Kodierungen ≤ 8.
