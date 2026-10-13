# FX-4 – Effekte Gruppe 4: Farbe anpassen, Rauschen, Sonstige (N28)

Stand 13.10.2026. Grundlage: `re\23-effekte.md` Gruppe 4, Referenzen `re\_testdateien\par\fx\4-farbe-rauschen-sonstige\`
(23 Fälle × 3 Objekte), Dialogbilder `re\_testdateien\par\ref\fx-4-farbe-rauschen-sonstige\INDEX.md`.

## Verfahren

- **Ende-zu-Ende** wie RE-FX: Basisdatei öffnen → `dom.applyEffects` mit der Effektliste aus `params.json` (Kurven aus
  `run_group.tpl.jsf` nachgebildet) → `exportTo` PNG 32 ganze Leinwand → pixelweise Vergleich (Farbe dort, wo beide
  Alpha ≥ 16 haben; Alpha getrennt). Lauf über `firepixel-cli script` (ohne Oberfläche).
- **Golden** (`crates\fp-effects\tests\fx4_parity.rs`, Daten `tests\data\fx4\`, 18 Fälle × 3 Objekte = 54):
  Eingabe = Objektpuffer des Originals aus dem Export ohne Wirkung, Erwartung = Ausschnitt des Original-Exports. Je Fall
  Live-Effekt **und** derselbe Filter (`fp_raster::Filter`, Menü/Batch). Ziel ≤ 3 Stufen; Rauschen zusätzlich Streuung.

## Ergebnis (größte Farbabweichung Cmax / Pixel > 3 Stufen, Ende-zu-Ende Vektor + Bitmap; Text siehe unten)

| Effekt | Fälle | vorher | nachher | Ursache / Änderung |
|---|---|---|---|---|
| Helligkeit/Kontrast | vorgabe, klein, gross | 1 / 0 (klein Vektor) | 1 / 0 | Rest 1 Stufe bei Vektor klein (Mittelwert, s. offen) |
| Farbton/Sättigung inkl. Kolorieren | 4 | 0 | 0 | – |
| Kurven | vorgabe, klein, gross | **217 / 5139** (klein 17) | 0 | Punktlisten gingen als Text verloren: `applyEffects` (fp-script) und `element.addEffect` (fp-engine) machten aus `[{x,y},…]` einen String → Kurve = Identität. Jetzt `GPLv`-Punktliste |
| Tonwertkorrektur | 3 | 0 | 0 | – |
| Auto-Tonwert (einfarbig → schwarz) | vorgabe | 0 | 0 | Eigenheit bestätigt, eigener Test |
| Umkehren | vorgabe | 0 | 0 | – |
| Farbfüllung | vorgabe, klein (20 %), gross (Modus 1) | **147 / 2018** (Bitmap), 30 / 32 (Vektor) | 0 | Live-Effekt mischte halbtransparente Pixel mit `div255(a)`; das Original gewichtet `DIV[a][a−div255(A·a)+A]` wie der Filter (06 §5). fp-effects nutzt jetzt die Filterrechnung |
| Rauschen | vorgabe, klein, gross (farbig) | 0 (Streuung gleich) | 0 | deterministisch, Startwert aus Pufferlage |
| In Alphakanal konv. | vorgabe | 0 | 0 | – |
| Ränder suchen | vorgabe | **255 / 52** (Vektor), 4 / 1 (Bitmap) | 0 | Farbe durchsichtiger Pixel im Objektpuffer: Original **Vektor weiß**, **Bitmap gespeicherte Farben**, Text schwarz (Simulation: nur so exakt, gilt auch für Weichzeichnen/Schärfen am Vektor). `fp-render` `compose.rs` setzte alles auf schwarz |

Golden (54 Fälle, inkl. Text mit Originalpuffer): alle **exakt** (max 0), Filterweg = Live-Weg.

**Text** Ende-zu-Ende: weicht schon **ohne** Effekt ab (Alpha bis 159 an 431 px, `helligkeit-kontrast_vorgabe_text`) →
Textdarstellung, nicht Gruppe 4. Zusätzlich gemessen (aus den Rausch-Startwerten gelöst und per Simulation bestätigt):
der Objektpuffer des Originals für den Text ist **23,46–167,112** (144 × 66 = pixelRect 43,60–148,98 erweitert um
−20/−14/+19/+14), Firepixel nimmt den Rahmen 38,58–202,106. Folge: Rauschen am Text liegt anders (Streuung gleich),
und Weichzeichnen/Schärfen am Textrand können abweichen. Mit dem Originalpuffer rechnen alle Text-Fälle exakt.

## Dialoge (re\23 Vorgaben/Grenzen)

- Rauschen als Live-Effekt ohne gespeicherten Schlüssel zeigte „Farbe“ **an**; Vorgabe ist **aus** → behoben (`effects.rs`).
- Farbton/Sättigung: mit „Kolorieren“ Farbton 0…360 und Sättigung 0…100 (vorher immer −180…180/−100…100) – Filter-
  und Live-Dialog (`form.rs` `sync_colorize_ranges`, auch beim Laden gemerkter Werte). Annahme: beim Umschalten wird
  der Farbton auf den Kreis umgerechnet (−30 ↔ 330), die Sättigung geklemmt.
- Kurven: fehlende Punktliste lässt Firepixel nicht scheitern (fehlend = Identität; Original: `exportTo` false).
- Übrige Grenzen (Helligkeit/Kontrast ±100, Tonwert 0–255/γ 0,1–10, Rauschen 1–400, Farbfüllung 0–100 %) stimmten.

## Commits

- `FX-4: Gruppe 4 …` (dieser Commit; Hash im Log unter „FX-4“).

Geänderte Dateien: `crates\fp-effects\src\color_fill.rs`, `crates\fp-render\src\compose.rs` (Transparenzfarbe des
Effektpuffers – **betrifft auch FX-3**: Weichzeichnen/Schärfen am Vektor mit Rundung werden dadurch exakt, laut
Simulation gegen `3-weichzeichnen-schaerfen`), `crates\fp-engine\src\commands\draw.rs` (`element.addEffect` Punktlisten),
`crates\fp-engine\src\commands\panel.rs` (`effect_param_value` jetzt `pub(crate)`, gemeinsam genutzt),
`crates\fp-script\src\imp\doc.rs` (`applyEffects` reicht Listen durch), `crates\fp-ui\src\dialogs\form.rs`,
`crates\fp-ui\src\dialogs\effects.rs`, `crates\fp-effects\tests\golden.rs` (Python-Referenz-Fälle der Farbfüllung an
halbtransparenten Pixeln übersprungen: die Referenz rechnet `div255`, das Original nicht).

## Tests

- `cargo test -p fp-effects` (neu `fx4_parity`: 54 Golden-Fälle + Kurven ohne Liste + Auto-Tonwert schwarz + Farbfüllung
  Live = Filter) – grün.
- `cargo test -p fp-engine --test all fx4_effects::` – Punktlisten, §4b: 50× Wert ändern → ein Effekt, Rückgängig/
  Wiederholen, Folge A/B/Rückgängig/C, Auswahlwechsel, Mehrfach- und leere Auswahl – grün.
- `cargo test -p fp-ui --lib fx4_` – Rauschen-Vorgabe, Kolorieren-Bereiche 25× hin und her – grün.
- `cargo test -p fp-script --test api`, `cargo test -p fp-neutral`, `cargo check --workspace --all-targets` – grün.
- Hinweis: `cargo test -p fp-render --test tiles` bricht mit „attempt to multiply with overflow“ in der Testdatei selbst
  (`tiles.rs:19`, `i as u32 * 2_654_435_761`) – unabhängig von FX-4.

## PARITAET → „umgesetzt, Nachprüfung ausstehend“

Effekte/Filter Gruppe 4 (Helligkeit/Kontrast, Farbton/Sättigung, Kurven, Tonwert, Auto-Tonwert, Umkehren,
Farbfüllung, Rauschen, In Alpha, Ränder suchen) als Live-Effekt und Filter; Dialoge Rauschen (Vorgabe Farbe aus) und
Farbton/Sättigung (Kolorieren-Bereiche).

## Nachprüfung (NACHPRUEFUNG)

- Brücke: Objekt mit Kurven-Effekt per Eigenschaften-(i) öffnen, Punkt ziehen, OK → Leinwand ändert sich, ein Verlaufsschritt.
- Brücke: Filter ▸ Farbe anpassen ▸ Farbton/Sättigung, „Kolorieren“ an → Farbton-Feld nimmt 300 an, Schieber reicht bis 360; aus → −60.
- Brücke: Effekt „Rauschen hinzufügen“ einfügen, (i) → „Farbe“ nicht angehakt, Menge 50.
- Ende-zu-Ende-Lauf (Skript im Scratchpad `fx4\sweep.ps1`): alle Vektor/Bitmap-Fälle max ≤ 1.

## Neue Befunde

- (FX-4 → Text/Render) Objektpuffer für Live-Effekte an Text: Original 23,46–167,112 für `fx_text.fw.png`
  (pixelRect + ca. 20/14 px), Firepixel nimmt den Textrahmen 38,58–202,106; außerdem weicht die Textdarstellung selbst
  ab (Alpha bis 159, 431 px ohne Effekt). Regel für den Rand aus nur einem Messpunkt nicht ableitbar → weitere Messung nötig.
- (FX-4) Helligkeit/Kontrast „klein“ (10/10) am einfarbigen Vektor: 1 Stufe in G/B; passt nur mit Mittelwert 109 statt
  113 – Rechenweg des Mittelwerts am Vektor offen (Bitmap exakt).
- (FX-4 → FX-3) Durch die Transparenzfarbe Vektor = weiß sollten Weichzeichnen/Schärfen-Fälle am Vektor exakt werden;
  bitte im FX-3-Lauf gegenprüfen.
