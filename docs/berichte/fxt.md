# FX-T – Text-Rasterung wie im Original (Teil von N28) und roter Test `all_commands`

Stand 13.10.2026, Slot `fp-s4`. Grundlage: `re\09-text.md` §6, `re\23-effekte.md` (Basisdatei
`re\_testdateien\par\fx\base\fx_text.fw.png`: „Fx Ag“, Arial-BoldMT 40 px, `#cc3300`, Rahmentext ab 40,60,
Glättung „glatt“), Befunde aus `fx1.md` (1-px-Rand, Massiver Schatten) und `fx2.md` (Neu-Rastern bis 159 Stufen).

## 1. Ursachen

Vergleich reines Textobjekt (ohne Effekt, Vorgabefall): gespeichertes Bild des Originals gegen
Firepixel nach einer Änderung (`element.setOpacity 100`, erzwingt Neu-Rastern).

| # | Ursache | Befund | Änderung |
|---|---|---|---|
| 1 | **Kein Hinting** | Das Original (Schriftmaschine des Systems) hintet die Glyphen in der N-fachen Auflösung der Überabtastung: Kanten (Versalhöhe, Querbalken, Stämme) liegen auf dem 1/4-px-Raster. Ungehintet weichen Kanten um 1–3 Abtastpunkte ab. Korpus (1.469 Render-Caches, `corpus_pixels`): Arial-BoldMT Tintenpixel exakt **54,6 % → 98,7 %**, ArialMT 41,4 → 95,3 %, Times 52,7 → 97,2 % mit vollem Hinting („Mono“) bei ppem = Größe·N; glattes/leichtes Hinting nur 64,7 % | `fp-text`: gehintete Umrisse über `skrifa` 0.46 (reines Rust, schon im Baum), TrueType-Interpreter bzw. PostScript-Hinter, Ziel „Mono“, ppem = Größe·N, Zwischenspeicher je Glyphe/Größe (begrenzt) |
| 2 | **Grundlinie nicht auf ganzen Pixeln** | Rahmentext: erste Grundlinie = Oberkante + lateinische Oberlänge = 60 + 28,63 = 88,63. Das Original setzt sie auf **89** (F-Oberkante 60,25 statt 60,0, Unterlänge reicht in Zeile 97 → pixelRect bis 98). Testdateien (Rahmentext/Text in Pfad): runden 94,1/95,7 %, abrunden gleich, aufrunden 89,1/86,8 %, ungerundet 91,5/91,4 % | gerundete Grundlinie beim Rastern gehinteter Glyphen; waagerechter Ursprung auf 1/N px (Korpus +0,1 %) |
| 3 | **Kein 1-px-Rand** | Objektbild des Originals = pixelRect + 1 px transparenter Rand (FX-1: Massiver Schatten exakt nur mit Rand) | `render::render(lay, None)` bzw. `coverage(lay, None)`: Rechteck der sichtbaren Pixel **+ 1 px**; mit explizitem Rechteck unverändert |
| 4 | Glättung „Glatt/Stark/Scharf“, Gamma | Tonkurven waren bereits exakt (Alpha-Stufen 0, 16 … 240, 255 wie im Original). „Stark“ hat im Original einen eigenen Pfad; gehintet etwas schlechter (91,2 % statt 91,9 %) → bleibt ungehintet | keine |
| 5 | Neu-Rastern nach Änderung | Unverändert zeigt Firepixel das gespeicherte Bild; nach jeder Änderung wird neu gerastert (richtig). Abweichung kam allein aus 1–3 | keine |

Was an der Schriftmaschine des Systems liegt (begründet, nicht weiter angleichbar):
- **Myriad Pro (CFF)**: Hinting bringt kaum etwas (Bold 72,1 → 72,4 %, Regular 48,4 → 52,0 % Tintenpixel). Die
  Korpusdateien stammen aus der Schriftmaschine des Originals mit deren eigener Myriad-Fassung (09 §8a T7: andere
  Vorschübe, z. B. „l“ 5,5 px → 5); die unter Windows installierte Fassung hat andere Konturen/Hints. Auch der
  PostScript-Hinter von `skrifa` (wie FreeType) ist nicht der des Originals.
- **Rest bei Arial/Times** (1–5 % der Tintenpixel, beim Testtext 7 Pixel mit 16 Stufen = 1 Abtastpunkt): Rundungsdetails
  des TrueType-Rasterers des Originals (Dropout, Rundung der Überabtastung).
- Gedrehter/gescherter/breitenskalierter Text und Faux-Kursiv werden ungehintet gerastert (Hinting nur für reine
  Skalierung + Verschiebung).

## 2. Vorher/nachher

**Reines Textobjekt** (`fx_text.fw.png`, neu gerastert gegen gespeichertes Bild, ganze Leinwand 240 × 180):

| | Alpha max | Pixel > 3 | pixelRect |
|---|---|---|---|
| vorher | 159 | 431 | 43,60–148,97 (1 Zeile zu kurz) |
| nachher | **16** | **7** | 43,60–148,98 (wie Original) |

**Ganze Kette** (Firepixel-CLI, JSF wie `run_group.tpl.jsf`: `openDocument` → `applyEffects` mit der Liste aus
`params.json` → `exportTo` PNG 32), Text-Fälle aller vier Gruppen gegen die Referenz-PNGs. A = Alpha max,
C = Farbe vormultipliziert max, n = Pixel mit Abweichung > 3:

| Fall | vorher A / C / n | nachher A / C / n |
|---|---|---|
| Schlagschatten vorgabe / klein / gross / aussparen | 131/127/980 · 192/128/431 · 150/120/431 · 67/0/840 | 16/13/7 · 29/13/7 · 16/13/7 · 7/0/5 |
| Innerer Schatten vorgabe / klein / gross / aussparen | 159/122/742 · 159/190/431 · 159/127/431 · 78/0/767 | 16/11/7 · 16/26/7 · 16/13/7 · 11/0/6 |
| Glühen vorgabe / klein / gross / halo | 69/49/801 · 255/255/537 · 140/108/1375 · 159/127/1187 | 8/6/7 · 0/16/7 · 14/11/7 · 16/13/7 |
| Inneres Glühen vorgabe / klein / gross | 159/59/431 · 159/159/500 · 159/127/431 | 16/8/7 · 16/16/7 · 16/13/7 |
| Massiver Schatten vorgabe / klein / gross | 192/154/370 · 159/127/439 · 192/154/665 | 16/13/6 · 16/13/8 · 16/13/8 |
| Abschrägung innen vorgabe / klein / gross / gedrückt / Rüsche | 159/126/862 · 159/127/449 · 159/123/896 · 159/94/823 · 159/123/838 | 16/15/7 · 16/13/7 · 16/13/7 · 16/13/7 · 16/13/7 |
| Abschrägung außen vorgabe / klein / gross | 255/233/2341 · 255/227/483 · 255/228/7804 | 0/21/22 · 149/122/3 · 0/15/16 |
| Prägung erhaben vorgabe / klein / gross | 61/50/1257 · 63/63/389 · 135/121/8770 | 11/7/20 · 10/3/3 · 15/13/10 |
| Prägung vertieft vorgabe / klein / gross | 57/61/1247 · 71/61/407 · 135/116/8823 | 9/11/20 · 8/10/3 · 15/17/10 |
| Weichzeichnen vorgabe / 3fach | 100/103/1119 · 105/122/1580 | 8/6/7 · 19/10/188 |
| Stärker weichz. vorgabe / 3fach | 100/111/1190 · 109/128/1727 | 6/4/7 · 28/18/227 |
| Gauß vorgabe / klein / gross / extrem210 | 8/7/800 · 116/92/963 · 2/2/0 · 255/74/38 | 2/2/0 · 10/9/8 · 2/2/0 · **255/69/4280** (s. Befund 1) |
| Bewegung / Radial / Zoom (je vorgabe, klein, gross) | 56–255, bis 7547 | 3–128, bis 6515 (Effektrechnung, FX-3) |
| Schärfen, Stark schärfen (vorgabe / 3fach) | 233/223/448 · 255/255/657 | 29/29/4 · 13/13/4 |
| Unscharf maskieren vorgabe / klein / gross | 201/161/403 · 159/127/431 · 255/204/205 | 49/39/93 · 17/14/7 · 175/140/52 |
| Gruppe 4 (Helligkeit/Kontrast, Farbton/Sättigung, Tonwert, Auto-Tonwert, Umkehren, Farbfüllung, In Alpha, Kanten finden; je Fall) | 110–159 Alpha, 431+ Pixel | **16**, 7 Pixel |
| Kurven klein / gross, Rauschen vorgabe / gross, Farbfüllung klein | 159/…/1360–1506 | 16/…/379–1464 (Farbabweichung aus der Effektrechnung, beim Vektor ebenso; FX-4) |

Alle Fälle, die am Vektor exakt sind, liegen beim Text jetzt bei höchstens 16 Stufen an 3–8 Pixeln (ein
Abtastpunkt des Hintings). Die übrigen großen Werte (Bewegung, Radial, Zoom, Kurven, Rauschen, Unscharf maskieren,
Weichzeichnen 3fach) zeigt der Vektor genauso – Effektrechnung, nicht Text (FX-3/FX-4).

**Korpus** (`cargo test -p fp-text --test corpus -- --ignored`): Glyphenpixel exakt 91,91 → **95,19 %**, Tintenpixel
53,5 → **71,9 %**, pixelgenaue Texte 1 → **73** von 1.469. Je Modus: glatt 90,3 → 94,6 %, scharf 94,9 → 96,3 %,
stark 91,9 → 91,9 %, benutzerdefiniert 87,9 → 96,7 %. Rahmen unverändert (4.002/4.147). Testdateien
(`testfiles`): Rahmentext 91,5 → 94,1 %, Text in Pfad 91,3 → 95,7 %, Glättung T9 95,5 → 98,7 %, normal 95,7 → 97,1 %.

## 3. Roter Test `cargo test -p fp-engine --test all all_commands::`

Ursache: Die Batch-Laufzeit (fp-batch) steckt sich in App/CLI/MCP über den Haken `BatchHost` ein, im Testprogramm
nicht → `batch.run/plan/preview/steps/validate` meldeten „fehlt die Batch-Verarbeitung“.
Behebung (ohne die Prüfung abzuschwächen): `fp-batch` als dev-dependency von fp-engine, `fp_batch::install()` in der
Testhilfe `fixture()`. Dazu echte Parameter (Quelldatei = gespeichertes Testdokument, Rezept PNG-Ausgabe) und
fachliche Prüfungen: `batch.run` schreibt genau eine PNG-Datei (`totals.done = 1`, `ok`), `batch.plan` liefert einen
Eintrag und schreibt nichts, `batch.preview` liefert Vorher/Nachher 400 × 300 als PNG. Ergebnis: `all_commands_work`
626/626, `repeated_commands_replace_instead_of_append` grün.

Nebenbei: `fp-render --test tiles` brach mit Überlauf in der Testhilfe ab (`i as u32 * 2_654_435_761`), jetzt
`wrapping_mul` (gleiche Farben wie im Release-Build).

## 4. Tests

- Neu `crates\fp-text\tests\fxt_raster.rs` (5 Tests, Referenz `tests\data\fxt\fx_text_original.png` = gespeichertes
  Textbild des Originals, ohne Metadaten): Golden Alpha/Farbe ≤ 16, ≤ 10 Pixel > 3; ungehintet bleibt die alte
  Abweichung (> 100); Objektbild 42,59–149,99 = pixelRect 43,60–148,98 + 1 px, `coverage` gleich; 50× neu setzen und
  rastern bitgleich (§4b); gedrehter Text ungehintet und vollständig mit Rand.
- Neu `crates\fp-render\tests\text_fxt.rs` (`--ignored`): Basisdatei ohne Render-Cache neu rendern gegen das flache
  Bild (43.193/43.200 gleich, max 16).
- `crates\fp-text\tests\golden.rs`: Raster-Golden der Python-Referenz mit `hinting: false` (Referenz ist ungehintet).
- `crates\fp-text\tests\corpus.rs`: zusätzlich Auswertung je Schrift.
- Grün: `cargo test -p fp-text` (alle), `-p fp-text --test corpus/testfiles/onpath_fit/legacy_fit -- --ignored`,
  `cargo test -p fp-render`, `-p fp-engine --test all all_commands::`, `cargo check --workspace --all-targets`,
  `cargo test -p fp-neutral`, clippy ohne Warnungen in fp-text/fp-render.

## 5. PARITAET / Nachprüfung

- PARITAET: Text-Darstellung (Rasterung, Glättung glatt/scharf/benutzerdefiniert, Grundlinie, Objektbild mit Rand)
  → „umgesetzt, Nachprüfung ausstehend“.
- NACHPRUEFUNG: In Firepixel (Brücke) `re\_testdateien\par\fx\base\fx_text.fw.png` öffnen, Text wählen, Deckkraft
  auf 100 setzen (erzwingt Neu-Rastern) → Bild unverändert zum Öffnen (max 16 Stufen an ≤ 10 Pixeln). Effekte aus
  `params.json` (z. B. Massiver Schatten vorgabe, Schlagschatten vorgabe) setzen, PNG 32 exportieren → gegen
  `par\fx\1-schatten-gluehen\*_text.png` ≤ 16 Stufen an ≤ 10 Pixeln. Ablauf als Skript: JSF wie
  `run_group.tpl.jsf` über `firepixel-cli script`.
- Sichtprüfung: Text in der Leinwand bei 100 % wirkt etwas schärfer (Hinting), bei Zoom unverändert glatt.

## 6. Neue Befunde

1. **(für FX-3) Gauß r > 200 („extrem210“)**: Das Original füllt nur das **pixelRect des Objekts** (43,60–148,98)
   deckend mit der Durchschnittsfarbe, der 1-px-Rand des Objektbilds bleibt transparent. Seit dem Rand füllt
   Firepixel 42,59–149,99 (4.280 Pixel). Abhilfe in der Effektrechnung: bei r > 200 auf das Alpha-Rechteck der
   Eingabe beschränken (am Vektor ohne Rand bisher zufällig gleich).
2. **(für FX-3) Weichzeichnen/Stärker weichzeichnen 3fach am Text**: Das Original wächst je Durchgang um 1 px
   (pixelRect 41,58–150,100 nach 3 Durchgängen), Firepixel bleibt auf dem Objektbild mit Rand (42,59–149,99).
3. Myriad Pro (CFF) bleibt bei ~52–72 % exakten Tintenpixeln (Schriftfassung/PostScript-Hinter, s. o.).
4. fp-ui-Bildvergleiche mit Text (kittest-Bildserien, Paarbilder) können sich durch das Hinting leicht ändern;
   nicht neu erzeugt (Bildserien gehören in die Nachprüfung).

## 7. Annahmen

- Das Original hintet mit voller Stärke („Mono“) in der Überabtastungs-Auflösung – abgeleitet aus dem Korpus
  (beste Übereinstimmung von vier Varianten), nicht aus dem Code.
- Grundlinie wird gerundet (nicht abgeschnitten): Testdateien runden = abrunden, `fx_text` (88,63 → 89) entscheidet für
  Runden.
- „Stark“ bleibt ungehintet (eigener Pfad im Original, gehintet geringfügig schlechter).
- Faux-Fett wird aus der gehinteten Kontur verbreitert (Testdateien 93,0 → 92,9 %, praktisch gleich).
- Freier Platz auf C: lag beim Start bei 7,5 GB (Regel: < 30 GB melden); während der Arbeit stieg er auf ~52 GB.
  Nur inkrementelle Builds im Slot, keine Release-Builds.
