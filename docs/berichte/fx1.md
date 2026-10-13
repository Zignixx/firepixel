# FX-1 – Schatten und Glühen gleich dem Original (N28, Gruppe 1)

Stand 13.10.2026, Slot `fp-s4`. Grundlage: `re\23-effekte.md` Gruppe 1, Referenzen `re\_testdateien\par\fx\1-schatten-gluehen\`
(54 PNG, `params.json`), Menü-Vorgaben `EFDv` (`re\effekte\_scratch\efdv.json`), Dialogbilder `par\ref\fx-1-schatten-gluehen\INDEX.md`.

## Verfahren

- **Bibliothek** (`crates\fp-effects\tests\fx1_parity.rs`, `sweep_fx1`, `--ignored`): Objektbild = zusammengesetztes Bild der
  Basisdatei, auf das Alpha-Rechteck beschnitten; Effektliste aus `params.json`; Ergebnis auf die Leinwand 240 × 180;
  Vergleich mit der Referenz, Alpha und vormultiplizierte Farbe getrennt (max, Mittel, Pixel > 3).
- **Ganze Kette** (Firepixel-CLI, JSF wie `run_group.tpl.jsf`: `openDocument` → `applyEffects` → `exportTo` PNG 32):
  Vektor und Bitmap wie die Bibliothek (alle Fälle max ≤ 1), Text siehe „Offene Befunde“.

## Ursachen und Änderungen

| # | Ursache | Änderung |
|---|---|---|
| 1 | **Glühen-Dialog schrieb die Deckkraft ins Alpha der Farbe**, das Glühen liest aber `BevelContrast` (Farb-Alpha wird ignoriert) → Deckkraft-Regler ohne Wirkung, immer 75 %. Dasselbe beim Inneren Glühen. | Dialog schreibt `BevelContrast` = Deckkraft, Farbe als `#rrggbb`; beim Bearbeiten zeigt er `BevelContrast` an |
| 2 | **Menü-Vorgaben falsch**: Glühen Breite 4/Weichheit 4/Füllfarbe statt „Normal“ 5/65 %/12/`#df0000`; fehlende Schlüssel (`EdgeThreshold` 0 statt 1 → Glühen begann im weichen Rand, `BevelContrast` 75); Inneres Glühen Füllfarbe statt Schwarz; Schlagschatten `0xa6` statt `0xa2` (64 %), Innerer Schatten `0xa6` statt `0xa5` | Vorgaben im Dialog nach `EFDv`; `element.addEffect` per Name (`glow`, `dropShadow`, `innerShadow`, `innerGlow`) legt die vollständige Menü-Vorgabe an, übergebene Parameter überschreiben sie |
| 3 | `element.addEffect {"effect": "glow"}` ohne `BevelType` ergab **Abschrägung innen** | durch #2 behoben (BevelType 4) |
| 4 | **Glühen mit Versatz > 0**: Band begann bei Abstand ≥ Versatz, im Original bei Abstand > Versatz (1 px weiter außen) | `bevel.rs::glow`: `dd > g0` |
| 5 | Inneres Glühen: Breite < 3 wird im Original als 2 gespeichert | Dialog speichert 2 (Renderer tat es schon) |

Abstand/Winkel-Rundung (Abstand 1 bei 45° → kein Versatz), Weichzeichnungskern, Aussparen und Farbe waren bereits exakt.
Gemeinsame Hilfsfunktionen (`util.rs`, Dreiecksfilter) **nicht** geändert. `bevel.rs` (auch FX-2) nur in `glow()` geändert.

## Abweichung vorher/nachher (Bibliothek, max Alpha / max Farbe vormult., Stufen)

| Fall | Vektor vorher → nachher | Text vorher → nachher | Bitmap vorher → nachher |
|---|---|---|---|
| Schlagschatten vorgabe/klein/gross/aussparen | ≤ 1 → ≤ 1 | ≤ 1 → ≤ 1 | ≤ 1 → ≤ 1 |
| Innerer Schatten vorgabe/klein/gross/aussparen | ≤ 1 → ≤ 1 | ≤ 1 → ≤ 1 | ≤ 1 → ≤ 1 |
| Glühen vorgabe | 0 → 0 | 1 → 1 | 1 → 1 |
| Glühen klein | 0 → 0 | 0 → 0 | 0 → 0 |
| Glühen gross | 8 (7492 px > 3) → **0** | 7 (3270 px) → **0** | 4 (198 px) → **0** |
| Glühen halo | 26 (3076 px) → **0** | 26 (1970 px) → **0** | 26 (1037 px) → **0** |
| Inneres Glühen vorgabe/klein/gross | 0 → 0 | 0 → 0 | 0 → 0 |
| Massiver Schatten (Zusatz, gleicher Ordner) | 0 → 0 | 192/255 (27/55 px) → unverändert, s. u. | 0 → 0 |

Vor allem wirkte sich im Programm Ursache 1/2 aus: Glühen über das Menü hatte andere Breite, Weichheit, Farbe, Deckkraft
(75 % statt der eingestellten) und `EdgeThreshold` – das ist der Befund N28.

## Tests

- `crates\fp-effects\tests\fx1_parity.rs::golden_fx1`: 18 Golden-Fälle (je Effekt Vorgabe + Extremfälle: Schlagschatten
  vorgabe/klein(45°, Abstand 1)/gross/aussparen, Innerer Schatten vorgabe/gross/aussparen, Glühen vorgabe ×2/klein/gross/halo ×2,
  Inneres Glühen vorgabe/klein(Breite 1 → 2)/gross, Massiver Schatten vorgabe/gross), erlaubt max 1. Referenzen in
  `tests\data\fx1\` (Ergebnisbilder ohne Oberfläche, ohne Metadaten neu kodiert; Basisobjekte als PNG).
- `crates\fp-effects\tests\golden.rs`: Python-Referenz-Fälle `glow_*_st<n>` (Versatz > 0) übersprungen, Begründung im Code (Original ≠ Python-Referenz A).
- `crates\fp-ui\tests\fx1_effect_dialogs.rs` (4 Tests): Menü-Vorgaben wie Original; Deckkraft 50× ändern → ein Effekt,
  `BevelContrast` = letzter Wert, ein Verlaufsschritt, Bearbeiten zeigt den Wert (§4b); Inneres Glühen Breite 0/1/2 → 2;
  `element.addEffect` per Name mit Vorgabe.
- Grün: `cargo test -p fp-effects`, `fp-ui --test all fx1_effect_dialogs::`, `dialogs_effects::`, `--lib dialogs::effects`,
  `fp-engine --test all phase5::`, `cargo check --workspace --all-targets`, `cargo test -p fp-neutral`.
- `fp-engine --test all all_commands::` scheitert nur an `batch.*` („fehlt die Batch-Verarbeitung (fp-batch)“) – unabhängig von FX-1.

## PARITAET / Nachprüfung

- PARITAET: Schlagschatten, Innerer Schatten, Glühen, Inneres Glühen (Ergebnis, Dialog-Vorgaben, Deckkraft) → „umgesetzt, Nachprüfung ausstehend“.
- NACHPRUEFUNG: In Firepixel (Brücke) Rechteck wählen, Effekt „+“ → Schatten und Glühen → Glühen, OK → Dialog zeigt Breite 5,
  Deckkraft 65 %, Weichheit 12, Versatz 0, Rot `#df0000`; Deckkraft auf 30 % ziehen → Glühen sichtbar schwächer (vorher ohne Wirkung).
  `re\_testdateien\par\fx\base\fx_vector.fw.png` öffnen, Effekte aus `params.json` setzen, PNG exportieren → pixelgleich zur Referenz (max 1).

## Offene Befunde (nicht FX-1-Code)

1. **Text-Rasterung** (fp-text/fp-render): Bei Text weicht schon das Objekt selbst ab (z. B. Schlagschatten „klein“, der den
   Text unverändert lässt: Alpha max 192, 431 Pixel > 3); alle Text-Fälle der ganzen Kette erben das. Außerdem liefert JSF
   `pixelRect` des Texts 38,58–202,106 statt 43,60–148,98 und für alle Objekte ohne Effektausdehnung.
2. **Massiver Schatten + Text** (1 Zeile/Spalte fehlt): Das Original rechnet mit einem Textbild, das 1 px transparenten Rand
   hat; mit diesem Rand ist die Bibliothek für alle Text-Fälle exakt (geprüft mit `FX1_PAD_TEXT=1`). Die Begrenzung
   `@00b286c0` bezieht sich auf das Objektbild. Abhilfe in fp-render (Text-Ebene mit 1 px Rand an die Effektkette geben)
   oder fp-text – nicht geändert, da fremder Bereich und Massiver Schatten nicht im Auftrag.
3. JSF-Vorgabe-Befehl in `fp-script\src\builtin_commands.rs` setzt Schlagschatten `#000000a5` (Menü-Vorgabe ist `a2`) – prüfen, ob gewollt.

## Annahmen

- Menü „Glühen“ fügt die Vorgabe „Normal“ aus `EFDv` ein (Farbe `#df0000`, nicht die Füllfarbe), wie in re\23 angenommen.
- Feldgrenze „Versatz“ (Glühen) ist in den Dialogressourcen nicht angegeben; 0–100 beibehalten.
- Für vorhandene Glühen-Effekte ohne `BevelContrast` zeigt der Dialog 75 % (Leser-Vorgabe).

## Commits

Siehe `git log --grep "FX-1:"` (Commit-Liste in der Abschlussmeldung).
