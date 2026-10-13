# BATCH-6 – Hintergrund und Überlagerung (Batch-Studio)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §3.5/§3.6, Paket BATCH-6 aus §6.1.
Kern-API aus BATCH-1 (`docs\berichte\batch1.md`). Commit: `BATCH-6: …` (siehe `git log --grep BATCH-6`).

## Umgesetzt

| Schritt | Inhalt |
|---|---|
| `background.remove` (P1) | KI-Freistellen mit BiRefNet über `fp-ai` (`pipeline::subject_raw` + `subject_from_raw`, wie `select.aiSubject`): `model` birefnetLite/birefnet, `edge` hard/antiAlias/feather + `feather` px, `tolerance` 0…100, `expand` −100…100, `smooth` 0…100, `refineEdge` (BiRefNet_lite-matting über `pipeline::refine_edge`; fehlt das Matting-Modell → Warnung, Schritt läuft ohne), `decontaminate` (Kantenfarben mit dem geschätzten Hintergrund entmischen), `output` transparent / mask (Graubild) / alpha (Farben bleiben, Maske ersetzt Alpha). Netzausgabe zwischengespeichert (Cache von `fp-ai`), Feineinstellungen rechnen ohne neues Netz. Abbrechen wird an `fp_ai::Cancel` weitergereicht (Überwachungs-Thread). **Modell fehlt → Fehler „KI-Modell fehlt: … „KI-Modelle verwalten…“ … Ordner“, nie Download**; `validate()` meldet es vorab als Warnung `aiModelMissing`. |
| `background.removeColor` (P1) | Farbe von Ecke (4 Ecken) oder eigene Farbe, `tolerance` 0…255 (größte Kanaldifferenz), `contiguous` (Flutfüllung von den Rändern, eingeschlossene Flächen bleiben), `feather` 0…10 px (nur nach innen), `decontaminate`: Motivfarbe aus dem Inneren in den Saum fortgesetzt, Motivanteil per Projektion (C−K)·(F−K)/|F−K|² → kein heller Rand, geglättete Kanten bleiben glatt. |
| `background.flatten` (P1) | „Transparenz → Farbe“: `kind` color/gradient (linear mit Winkel, radial)/image (Datei oder eingebettet, cover/contain/stretch/tile, über `color`)/blur (unscharfe Kopie des Bildes); `onlyIfTransparent`; **Kantenbehandlung** `edge` blend / unmix (Farbsaum mit `matte`-Farbe entmischen) / hard (`threshold`); `linearLight` (Mischung in linearem Licht). |
| `background.replace` (P2) | Entfernen (`method` ai oder color) + Füllen in einem Schritt, alle Parameter von remove/removeColor/flatten; „unscharfe Kopie“ nutzt das Bild vor dem Entfernen. |
| `background.shadow` (P2) | `style` drop (Abstand/Winkel/Weichheit/Deckkraft/Farbe wie Schlagschatten, Vorgabe 315°) bzw. below (flacher Bodenschatten unter dem Motiv, für Produktbilder); `expandCanvas` lässt die Leinwand genau um den Schatten wachsen. |
| `overlay.text` (P1) | Text/Wasserzeichen über `fp-text` (Systemschriften, Satz, Glättung): Platzhalter `{name}` `{ext}` `{folder}` `{n}` `{n:3}` `{total}` `{date}` `{time}` `{year}` `{exif-date}` (Rückfall Dateidatum, dann heute) `{w}` `{h}`, `(c)`→©, mehrzeilig; `position` 9 Lagen / tiled (Ziegelmuster, `spacing` %) / free (`x`/`y` %); `size` % der Bildbreite; `font`, `bold`, `italic`, `color`, `opacity`, `margin` % der kürzeren Seite, `rotation`, `shadow`, `outline` (+Farbe/Breite), `blend` (10 Mischmodi). Ohne Schrift → Warnung, Bild unverändert. |
| `overlay.image` (P1) | Logo/Bild aus Datei (`image`) oder **im Rezept eingebettet** (`imageData`, Base64 bzw. Data-URL), einmal dekodiert und zwischengespeichert; `scale` % der Bildbreite, `opacity`, `margin`, `rotation`, `position` inkl. Kachelung, `blend`. |
| `overlay.border` (P1) | `width` in px oder % der kürzeren Seite (`unit`), `color`, `radius` 0…50 % (abgerundet, außen transparent, kantengeglättet), `placement` outer (Bild wächst) / inner. |

Gemeinsam: alle Schritte wirken auf Einzelbild, jedes Frame einer Animation und flach gerechnete
Dokumente (`ItemKind::PIXELS`), prüfen Abbrechen je Frame, setzen `meta.has_transparency`.
**Vorschau**: Größen in % wirken im Proxy automatisch richtig; Pixelangaben (Rahmenbreite px,
Schattenabstand/-weichheit, weiche Kante, Unschärfe) werden im Vorschau-Proxy mit dem Maßstab
Proxy/Quelle (`ctx.info.width/height`) mitverkleinert – die Vorschau sieht aus wie das Ergebnis.

Dateien: `crates\fp-batch\src\steps\background.rs` (Schritte + gemeinsame Pixelhilfen: vormultipliziertes
Fließkommabild, Gauß-Näherung aus drei Kastenfiltern, Skalieren mit Flächenmittel, Drehen, 10 Mischmodi,
Base64, Bild-Parameter mit Zwischenspeicher), `steps\overlay.rs`, `crates\fp-i18n\locales\{de,en}\batch_bg.ftl`.

## Tests (alle grün)

- `crates\fp-batch\tests\steps_background.rs` (14): Pixelwerte für Farbe entfernen (zusammenhängend/
  überall, Toleranz, eigene Farbe, weiche Kante + Entmischen), Füllen (exakt Weiß, onlyIfTransparent,
  Entmischen gegen weißen Saum auf Schwarz, harte Kante, Verlauf linear/radial, eingebettetes Bild,
  unscharfe Kopie, lineares Licht), Ersetzen mit Farbe, Maskenausgaben + Entmischen, Schatten
  (Leinwandgröße, Richtung, Bodenschatten), Lauf über `executor::run` + **Vorschau = Ergebnis**.
- `tests\steps_overlay.rs` (10): Platzhalter, 9 Lagen + frei + Kachelung (deckt alles, Abstand wirkt),
  Logo (Größe, Lage, 50 % Deckkraft, Multiplizieren, Drehung), Kachel-Logo auf allen Frames gleich,
  Text (Ecke/Abstand/Höhe, 60 % Deckkraft), Kontur/Schatten/Drehung/Kachelung, `{n:3}` je Element
  verschieden und wiederholbar, Rahmen außen/innen/%/abgerundet, Vorschau-Maßstab,
  Katalog + Texte (Schritt, Parameter, jeder Auswahlwert) in de und en.
- `tests\steps_background_ai.rs` (eigenes Programm, Modellsuche auf leeren Ordner umgestellt):
  „KI-Modell fehlt“ (de/en, Modellname, Bild unverändert) für remove/birefnet/replace, Validierung
  als Warnung nur beim KI-Schritt, Lauf: Element „Fehler“, Auftrag läuft, keine Ausgabe, keine
  `.fptmp`, Original unverändert. `#[ignore]`-Test mit echtem BiRefNet_lite (hier lokal vorhanden,
  **gelaufen: grün**, 3,8 s; zweiter Durchgang mit `expand`/`hard` nutzt den Zwischenspeicher).
- `tests\golden_bg.rs` + `tools\golden\batch\{bg-flatten-unmix,bg-remove-color,bg-replace-gradient,
  bg-shadow,overlay-border-round,overlay-image-tiled}\{in,expected}\` + `recipe.json` (Logo eingebettet):
  Lauf über `executor::run`, Vergleich max. Kanalabweichung 2. Neu erzeugen mit `FP_GOLDEN_UPDATE=1`.
- Logikfälle §4b: Parameter 50× ändern → ein Schritt, Endwert gilt; Füllen/Farbe entfernen zweimal = einmal
  (kein doppelter Effekt); Rahmenbreite 50× → einmal mit Endwert; Animation (jedes Frame), Abbrechen →
  `Cancelled`; 1-Pixel-Bild, voll transparentes Bild, fehlendes Bild → Fehler statt Absturz.
- Sichtprüfung der Ergebnisse (Text unten rechts mit Schatten, gekacheltes gedrehtes Wasserzeichen,
  Schlag-/Bodenschatten, abgerundeter Rahmen, Ersetzen mit radialem Verlauf ohne weißen Saum) an
  vergrößerten Testbildern: in Ordnung.
- `cargo check --workspace --all-targets`, `cargo clippy -p fp-batch --all-targets` ohne Warnungen,
  `cargo test -p fp-i18n`, `cargo test -p fp-neutral`, Scanner `i18n_keine_festen_texte::` grün.

## Einhängungen / Befunde

- `crates\fp-batch\src\steps\mod.rs`: `pub mod background;`, `pub mod overlay;` und je eine
  `register`-Zeile.
- `crates\fp-batch\Cargo.toml`: `fp-ai` (Pfad), `fp-text`, `fp-color` (alle eigene Crates, Apache-2.0;
  keine neue Fremd-Crate). `Cargo.lock`: 3 Zeilen bei `fp-batch`.
- **Texte in `batch_bg.ftl`** (wie SPEC §6.1 und die Schnittstellen-Notiz in batch1.md), nicht in
  `batch.ftl` – so stören sich die parallelen Pakete nicht. Präfixe `batch-step-background-*`,
  `batch-step-overlay-*`, `batch-param-…`, `batch-choice-…`, `batch-bg-*`, `batch-overlay-*`.
- **Neu (für BATCH-2, Formular):** Beschriftungen der Auswahlwerte liegen unter
  `batch-choice-<id mit ->-<param>-<wert>` (z. B. `batch-choice-overlay-text-position-bottomRight`),
  Tooltip des Textfelds unter `batch-param-overlay-text-text.tooltip` (Platzhalterliste). Bitte im
  generischen Formular nutzen (Rückfall: der technische Wert); ggf. als Konvention in `step.rs` aufnehmen.
- **Neu (für BATCH-1/Kern):** `StepContext` kennt den Vorschau-Maßstab nicht; BATCH-6 schätzt ihn aus
  Proxygröße/`info`-Größe. Steht vor dem Schritt ein Größenschritt (`resize`), ist die Schätzung ungenau.
  Vorschlag: Feld `preview_scale: f64` in `StepContext`, von `pipeline::preview` gesetzt.
- **Neu (für I18N/HG):** Scanner `crates\fp-ui\tests\i18n_keine_festen_texte.rs` prüft `crates/fp-batch/src`
  noch nicht (schon von BATCH-1 gemeldet); BATCH-6 hat keine festen Nutzertexte.
- `fp_raster::Mask` hat kein `is_empty()` (nur Hinweis).

## Annahmen

- Größe des Text-Wasserzeichens = Schriftgröße in px = `size` % der Bildbreite (SPEC „% der Bildbreite“).
- Abstand zum Rand (`margin`) in % der **kürzeren** Bildseite (gleicher Abstand waagerecht und senkrecht).
- Kachelabstand (`spacing`) in % der kleineren Stempelseite; Reihen versetzt (Ziegelmuster), Muster mittig.
- Schlagschatten-Vorgaben 315°, 10 px, Weichheit 8, 50 %; `opacity` 0…100.
- `background.remove` `output: alpha` = „keep + mask as alpha“ der SPEC (Farben unverändert).
- Mischmodi für Überlagerungen: eigene Teilmenge (normal, multiply, screen, overlay, softLight,
  hardLight, darken, lighten, difference, exclusion) nach W3C-Formeln statt der 46 Modi des Registers.
- Text-Wasserzeichen hat keinen Golden-Fall (Systemschriften je Rechner verschieden), dafür Logiktests.
- KI-Schritte rechnen bei Animationen je Frame (teuer, aber korrekt); eine gemeinsame Maske für alle
  Frames wäre P2.

## Für die Nachprüfung (`docs\NACHPRUEFUNG.md`)

- Studio (sobald BATCH-2 da ist): Rezept „Hintergrund entfernen“ auf 5 Produktfotos → PNG transparent,
  Haare/Kanten sauber, kein heller Saum; Feineinstellung (Toleranz, Ausdehnen) ändern → Vorschau
  aktualisiert sich sofort (kein neues Netz), Ergebnis = Vorschau.
- Ohne Modell (Modellordner leer): Karte zeigt ⚠ „KI-Modell fehlt … KI-Modelle verwalten…“, Lauf
  meldet je Datei „Fehler“, kein Download-Versuch.
- „Transparenz füllen“ (Weiß) + JPEG auf freigestellte Logos mit weißem Saum: mit „Farbsaum entmischen“
  auf dunklem Grund kein heller Rand.
- Wasserzeichen „© {year} {name}“ unten rechts (60 %) und gekachelt −30° auf Hoch- und Querformat,
  de/en; Logo aus eingebettetem Rezept (Rezept speichern/laden mit „Bilddateien einbetten“).
- Rahmen 4 % weiß, Radius 10 % → PNG mit transparenten Ecken; mit JPEG-Ziel → Ecken in Hintergrundfarbe.
- CLI: `firepixel-cli batch --recipe tools\golden\batch\overlay-image-tiled\recipe.json --in <Fotos>`.

## Für PARITAET (umgesetzt, Nachprüfung ausstehend)

SPEC-BATCH §3.5 `background.remove`, `removeColor`, `flatten`, `replace`, `shadow`; §3.6 `overlay.text`,
`overlay.image`, `overlay.border`. Offen aus §3.5/§3.6: `background.subjectCenter`, `background.alphaCleanup`,
`overlay.roundCorners` (mit `overlay.border` Breite 0 + Radius schon möglich), `overlay.vignette`,
`overlay.contactSheet`.
