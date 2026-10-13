# FX-3 – Weichzeichnen und Schärfen wie das Original (13.10.2026)

Grundlage: Nutzerbefund N28 (Gruppe 3), Messreihe `re\23-effekte.md` Gruppe 3 (24 Fälle × Vektor/Text/Bitmap,
PNG 32 der ganzen Leinwand 240 × 180), `re\06-bitmap-filter.md` §3.2, §4.7–4.13, Dialogbilder
`re\_testdateien\par\ref\fx-3-weichzeichnen-schaerfen\INDEX.md`. Das Original wurde nicht bedient.

## Vergleichsverfahren

- **Isoliert** (`crates\fp-effects\tests\fx3.rs`): Eingabe = Objekt, wie das Original es ohne Wirkung exportiert
  (Fall „radial klein“ 1/1 = unverändert) bzw. das Bitmap-Quellbild; Effektliste aus `params.json`;
  `fp_effects::apply_chain`; auf die Leinwand gesetzt; pixelweise gegen die Referenz. Kennzahlen: max. Abweichung
  Alpha, max. Abweichung Farbe (gewichtet mit der kleineren Deckkraft), Pixel mit Abweichung > 3.
- **Ganzer Weg** (`crates\fp-script\tests\fx3_e2e.rs`, `#[ignore]`): derselbe Ablauf wie im Original per Skript
  (`fw.openDocument` der Basisdatei → `selectAll` → `dom.applyEffects` → `dom.exportTo` PNG 32), dann
  Pixelvergleich (Skript im Scratchpad).

## Ergebnis (schlechtester der drei Objekte; Alpha/Farbe in Stufen, Pixel > 3)

| Effekt (Fälle) | vorher isoliert | nachher isoliert | nachher ganzer Weg Vektor+Bitmap |
|---|---|---|---|
| Weichzeichnen (Vorgabe, 3fach) | 111 / 44 / 396 (Text), Bitmap 10/8/110 | 1 / 0 / 0 | 0 / 0 / 0 |
| Stärker weichzeichnen | 124 / 46 / 439 | 1 / 0 / 0 | 0 / 0 / 0 |
| Gauß 4,1 / 0,5 / 30 | 3 / 1 / 0 | 2 / 1 / 0 | 1 / 1 / 0 |
| Gauß 210 (> 200, Mittelfarbe) | 255 / 12 / 8000 (Bitmap) | 0 / 0 / 0 (Text siehe unten) | 0 / 0 / 0 |
| Bewegung 90/10, 0/1, 45/100 | 128 / 56 / 4151 | 1 / 0 / 0 | 1 / 0 / 0 |
| Radial 30/20, 1/1, 100/100 | 51 / 0 / 2727 (Text) | 3 / 1 / 0 | 2 / 1 / 0 |
| Zentriert (Zoom) 30/20, 1/1, 100/100 | 128 / 0 / 19214 | 1 / 0 / 0 | 1 / 0 / 0 |
| Schärfen / Stark schärfen (Vorgabe, 3fach) | 183 / 92 / 247 (Text) | 0 / 0 / 0 | 0 / 0 / 0 (vorher Bitmap 0/22) |
| Unscharf maskieren 4,1/50/0, 0,5/10/0, 20/500/10 | 164 / 5 / 115 | 5 / 5 / 11 | 0 / 5 / 0 |

Restabweichungen (begründet): Unscharf maskieren 20/500/10 am **Text** bis 5 Stufen an 11 Pixeln (Textpuffer des
Originals nur angenähert, siehe 4.); an der **Bitmap** 1 Pixel Farbe 5 bei Alpha 2–3 (unsichtbar). Radial 100/100
Bitmap 2 Stufen Alpha am Rand. Ganzer Weg **Text** (auf main mit FX-T 2b9d75d, Text-Rasterung mit Hinting und 1 px
Rand): Gauß 210 exakt (0/0/0, vorher 4.280 Pixel zu viel – Koordinator-Befund, erledigt durch 3.), Gauß/Bewegung/
Radial/Zoom groß ≤ 3, die übrigen Fälle 4–29 Stufen an ≤ 18 Pixeln = Rest der Text-Rasterung selbst (unverändertes
Objekt 16 Stufen an 7 Pixeln), Unscharf 20/500/10 verstärkt diesen Rest 5-fach (101 an 13 Pixeln). Kein Filterfehler.

## Ursachen und Änderungen

1. **Bewegungsunschärfe** (`fp-raster\src\filters\blur.rs`): Proben `i = 0 … abstand` (06 §4.11) mit Winkel als
   `float` (`θ = a/360·2π` in einfacher Genauigkeit). sin 90° und −cos 180° liegen dadurch knapp unter 1:
   `trunc(i·sin) = i − 1`, die Unschärfe reicht dort `abstand − 1` weit und das Pixel zählt doppelt (Alpha-Stufen
   255·k/11 bei 90°/10); bei 0° volle Weite (0°/1 → 1 px nach links, Alpha 127). Bestätigt damit auch PM-6 D47
   (90/20 → +19) und klärt die offene Frage aus DLG-6 (Winkel 0: 300,200,420,300 statt 301).
2. **Zentriertes Weichzeichnen** (`blur.rs`): Ausgabegrenzen nach `@00b2c9e0` (Rechteck + Menge, dann Quadrat mit
   Halbkante `⌊√(((H'+1)/2)² + ((B'+1)/2)²) + 0,5⌋ + Menge`), Mitte = Mitte der **Eingabe**grenzen
   (`ZoomBlurParams::apply`). Vorher wuchs das Ergebnis nicht (100/100: 19.214 Pixel falsch). Menübefehl 10/20 bleibt
   unverändert (PM-6 D50), weil dort keine Probe das Objekt erreicht.
3. **Gauß > 200** (`gauss.rs`): gefüllt und gemittelt wird das Pixelrechteck (Alpha > 0) im Rechenbereich,
   durchsichtige Pixel darin zählen mit (Text 43,60–148,98 statt Textpuffer; Bitmap 72,53–168,127).
4. **Effektpuffer** (`fp-render\src\compose.rs`, `effect_buffer`, nur bei Ketten mit Bitmap-Filter):
   Bitmap = Pixelgrenzen (Weichzeichnen wächst nicht, Mitte von Radial/Zoom, Mittelwert Gauß > 200); Text =
   Pixelgrenzen + 15 px (rechts 16) – angepasst an Radial (Mitte) und an die Randnormierung des inneren Gauß von
   Unscharf maskieren (Vorgabe exakt). **Annahme (A):** die Textpuffergröße des Originals für andere Schriften/Größen
   ist nicht gemessen. Die Farbe durchsichtiger Pixel (Vektor weiß, Bitmap gespeichert, Text schwarz) hat FX-4
   (9bbc5e2) parallel an derselben Stelle umgesetzt; FX-3 hatte denselben Befund (Bitmap-Schärfen 3fach 22 Stufen
   mit schwarzen Pixeln, Vektor-Weichzeichnen 31) und baut darauf auf.
5. **`Filter::apply`** (`filters\mod.rs`) reicht `apply` an den Einzelfilter weiter (nötig für die Zoom-Mitte).
6. **Dialoge** (`fp-ui\src\dialogs\effects.rs`, `catalog.rs`): Rückfallwert Radius Gauß/Unscharf maskieren 4,1 statt
   1,0 (Vorgabe laut re\23 „4.1“). Grenzen geprüft und unverändert richtig: Gauß 0,1–250, Bewegung 0–360/1–100,
   Radial/Zoom 1–100, Unscharf 1–500/0,1–250/0–255; Weichzeichnen/Schärfen ohne Dialog.

Gemeinsamer Kern mit FX-1/FX-2: nicht geändert (fp-effects-Quellen unberührt); `compose.rs` nur für Ketten mit
Filtern. Mitgelaufen: fp-effects `golden` (FX-1/2), `fx4_parity`, fp-raster alle Tests.

## Tests

- `crates\fp-effects\tests\fx3.rs`: `golden_fx3` (18 Fälle × 3 Objekte = 54 Bilder unter `tests\data\fx3`, je Effekt
  Vorgabe + Extremfall, Ziel ≤ 3 Stufen, zwei begründete Ausnahmen), `chain_is_replaced_not_accumulated` (50 Werte,
  Endwert = einmal gesetzt, unsichtbar = unverändert), `extreme_parameters` (Gauß 210/250, Bewegung 0°/1 und 90°/10,
  Radial/Zoom 1/1 ohne Wirkung, Zoom 100/100 Grenzen, Unscharf 500/250/255), `sweep_fx3` (#[ignore], alle 72).
- `crates\fp-script\tests\fx3_e2e.rs`: `filter_effect_repeat_undo_and_selection` (§4b: 50× applyEffects → 1 Eintrag
  mit Endwert, Rückgängig/Wiederholen, zweites Objekt bekommt nur seinen Effekt), `fx3_end_to_end_export` (#[ignore]).
- fp-raster: `gauss.rs` Test > 200 angepasst; `tests\blur_bounds_dlg6.rs` (90/20 letzte Zeile 255/21, Winkel 0 → 300);
  Golden neu erzeugt mit `tools\golden\fp-raster\gen_golden.py` (motion float-Winkel, zoom Puffer/Mitte,
  gauss_210 Pixelrechteck; 21 Ausgabebilder geändert, übrige byteweise gleich).
- `fp-ui\tests\dialogs_dlg6.rs`: Bewegung Winkel 0/10 nach zweimal 90/20 → 310 (volle Weite).
- Grün: `cargo check --workspace --all-targets`, `cargo test -p fp-neutral`, fp-raster, fp-effects (inkl. FX-1/2
  `golden`, FX-4 `fx4_parity`), fp-render, fp-engine `fx4_effects`, fp-ui `dialogs_dlg5/dlg6` (Modul `all`).

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

- Live-Effekte/Filter Gruppe 3 (Weichzeichnen, Stärker weichz., Gauß inkl. > 200, Bewegung, Radial, Zentriert,
  Scharfzeichnen, Stark scharfz., Unscharf maskieren) an Vektor und Bitmap; PARITAET_B D47 (Winkel 0 jetzt 300) und
  D50 (Zentriert live wächst).

## Nachprüfung

- `fx_vector.fw.png` / `fx_bitmap.fw.png` öffnen, je Fall die Effektliste aus `re\23` Gruppe 3 setzen, PNG 32 der
  ganzen Leinwand exportieren → ≤ 2 Stufen gegen `par\fx\3-weichzeichnen-schaerfen\` (Unscharf 20/500/10 Bitmap ≤ 5).
- Menü Filter ▸ Bewegungsunschärfe Winkel 0, Umfang 20 an B1 320,200,420,300 → 300,200,420,300; Winkel 90 → bis 319.
- Effekt „Zentriertes Weichzeichnen“ 100/100 an einem Rechteck: Ergebnis reicht deutlich über das Objekt hinaus.
- Dialog Gauß/Unscharf maskieren aus einem Effekt ohne gespeicherten Radius: Feld zeigt 4.1.

## Neue Befunde

- (an fp-text, Rest nach FX-T) Text ohne Effekt noch 16 Stufen an 7 Pixeln; Text-Filterfälle im ganzen Weg erben das.
- (A, an RE) Textpuffer des Originals (Größe um die Pixelgrenzen) messen, z. B. per Radial-Mitte und Unscharf
  maskieren an weiteren Schriftgrößen; Firepixel nimmt +15 px (rechts +16).
- (an Engine) `pixelRect` eines Elements mit Live-Filter meldet nach `applyEffects` die Grenzen ohne Effekt
  (Original: z. B. Gauß 4,1 am Vektor 64,44–176,136).
- (an Engine, A) 50 aufeinanderfolgende `applyEffects` aus einem Skript fasst Firepixel zu einem Verlaufsschritt
  zusammen (ein Rückgängig entfernt den Effekt ganz); im Original nicht gemessen.
