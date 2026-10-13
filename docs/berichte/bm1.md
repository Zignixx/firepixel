# BM-1 – Bitmap-Grenzen wie im Original (Nutzerbefund N26, Stand 13.10.2026)

Grundlage: Messung `re\22-ansicht-bitmap-farbe.md` §1 (Regel 1.2, Vorschlag 1.4). Das Original wurde nicht benutzt. Slot s3.

## Umgesetzt (Commit siehe `git log --grep "^BM-1:"`)

1. **PNG öffnen und importieren beschneidet** das Bitmap-Objekt auf das Rechteck der Pixel mit **Alpha ≥ 2**; die Lage bleibt, die Leinwand behält die Bildgröße.
   - `fp-io` `build::document_from_raster_trimmed` (neu), benutzt von `import_format` für alle Einzelbilder (PNG, JPEG, BMP, TGA, TIFF, QOI, WBMP, HEIC sowie GIF/WebP/APNG/JXL/AVIF mit einem Einzelbild) und von der Zwischenablage (`CF_DIB`). Deckende Bilder (JPEG) bleiben unverändert.
   - Ganz transparentes Bild: beim Öffnen Bitmap 0×0 bei (0, 0), beim Importieren kein Objekt (`file.import` meldet dann wie bisher „keine Objekte“).
   - `file.import` mit `at`: bei Einzelbildern ist `at` die linke obere Ecke des **Bildes**, das beschnittene Objekt behält seinen Versatz (Zielpunkt (20,30) → [120,80,220,180] wie gemessen). Fremdes Bild aus der Systemzwischenablage (läuft über `file.import`) dadurch ebenso.
   - JSF `dom.importFile` (`fp-script` `imp\doc.rs`): beschneidet ebenso, ganz transparent → kein Objekt; bei einem Zielrechteck mit anderer Größe wird das ganze Bild hineinskaliert und der Versatz mitskaliert.
   - `fp-raster` `RgbaImage::alpha_bounds_min(min_alpha)` (neu, `alpha_bounds` = `alpha_bounds_min(1)`, unverändert).
2. **Auswahlrahmen um die sichtbaren Pixel:** `fp-engine` `docops::element_frame_bounds` (neu): Bitmap → sichtbare Pixel, leere Bitmap → Objektgrenzen, sonst `element_bounds`. Benutzt vom Rahmen und den Eckpunkten der Auswahl (`fp-ui` `canvas\mod.rs`) und der Hervorhebung beim Überfahren (`tools\pointer.rs` `outline_overlay`). X/Y/B/H (Inspektor, Info), `left/top/width/height`, `getSelectionBounds` und alle Transformationen bleiben bei den Objektgrenzen.
3. **JSF `pixelRect`** einer Bitmap = sichtbare Pixel (`docops::element_pixel_rect`, neu); leere Bitmap → (−2³¹, −2³¹, 2³¹, 2³¹) wie im Original. Andere Elemente unverändert.
4. **FW-PNG unverändert:** gespeicherte Grenzen werden beim Öffnen übernommen (auch Bitmaps über die ganze Leinwand aus älteren Firepixel-Dateien), Speichern schreibt die Objektgrenzen wie sie sind. Kein Eingriff in fp-fwpng.
- **Leistung:** `fp-model` `ImageData::visible_bounds` / `TileGrid::visible_bounds` (neu) mit `VISIBLE_ALPHA_MIN = 2`. Das sichtbare Rechteck wird **je Kachel** zwischengespeichert (Schlüssel Kachel-Adresse + genutzte Größe, mit `Weak` gegen Verwechslung; Kachelpixel werden nie an Ort und Stelle geändert, jede Pixeländerung erzeugt neue Kacheln, also wird automatisch neu gerechnet). Einfarbige Kacheln kosten nichts. Der Speicher wird bei 16 384 Einträgen bereinigt.

## Tests (alle grün)

- `crates\fp-engine\tests\bitmap_bounds_bm1.rs` (8): PNG mit Rand → beschnitten, Alpha-Schwelle 1/2/3/16/128, Rand nur links/rechts; leeres Bild → 0×0, pixelRect leer, Speichern/Öffnen; deckendes Bild bleibt ganz; Import mit/ohne Zielpunkt, Rückgängig/Wiederholen; leerer Import → kein Objekt; FW-PNG mit Bitmap über die ganze Leinwand behält [0,0,400,300], Rahmen/pixelRect [100,50,200,150], Rundlauf; beschnittene PNG → FW-PNG → Öffnen; Bleistift außerhalb → Rahmen wächst, Objektgrenzen bleiben, Rückgängig/Wiederholen, 50× Abfragen; Hervorhebung beim Überfahren.
- `crates\fp-script\tests\pixel_rect_bm1.rs` (4): `left/top/width/height` | `pixelRect` | `getSelectionBounds` für FW-PNG (0,0,400,300 | 100,50,200,150 | 0,0,400,300) und geöffnete PNG; leere Bitmap ±2³¹; `importFile` mit Zielpunkt (20,30) → 120,80,100,100, leer → kein Objekt.
- `crates\fp-ui\tests\bitmap_frame_bm1.rs` (1, kittest): `sel_frames` = [100,50,200,150] bei Objektgrenzen [0,0,400,300] (Alpha-1-Pixel zählt nicht), Auswahlgrenzen für X/Y/B/H weiter 0,0,400,300; Malen außerhalb → Rahmen wächst; Rückgängig → zurück; viele Bilder → gleich.
- `fp-model` Unit-Test `image::tests::visible_bounds_threshold_and_tiles`; `cargo test -p fp-io` grün; `cargo check --workspace --all-targets`; `cargo test -p fp-neutral`.

## PARITAET

- N26 / Bitmap-Grenzen (Öffnen PNG, Importieren, Auswahlrahmen, `pixelRect`) → „umgesetzt, Nachprüfung ausstehend“ (BM-1).

## NACHPRUEFUNG (Zeile für `docs\NACHPRUEFUNG.md`)

- BM-1 (N26) | echtes Fenster, Brücke | `re\_testdateien\re5a_t1_rand.png`, `re5a_t3_hoehe.png`, `re5a_t4_leer.png`, `re5a_t1_rand_fp.fw.png` (alte Firepixel-Datei) öffnen; je Datei Strg+A; t1 außerdem in ein 500×500-Dokument importieren | Rahmen mit Eckpunkten nur um das rote Quadrat bzw. den Balken (Paarbilder `re\_testdateien\par\ref\re5a\`); Eigenschaften X/Y/B/H: t1.png 100/50/100/100, t3 80/0/140/400, t1_fp.fw.png 0/0/400/300 (Rahmen trotzdem nur ums Quadrat); t4 leer: Bitmap 0×0, kein Absturz; Import: Objekt 120/80 bei Zielpunkt 20/30 bzw. mittig in der Ansicht mit dem Quadrat an seiner Stelle im Bild; Überfahren mit dem Zeiger hebt nur das Quadrat hervor; Bleistift außerhalb malen (Bitmap gewählt) → Rahmen umfasst Quadrat + Punkt, Strg+Z → wieder nur Quadrat.

## Annahmen und Befunde

- **A:** JPEG/GIF/WebP/BMP/TIFF usw. beschneiden nach derselben Regel (im Original nur PNG gemessen); deckende Formate ändern sich dadurch nicht.
- **A:** Einfügen fremder Pixel aus der Systemzwischenablage beschneidet ebenfalls (läuft wie ein Import). Nicht gemessen (re\22 §1.4 offen).
- **A:** Leerer Import zeigt weiter die Meldung „keine Objekte“ (`err-file-no-objects`); ob das Original still nichts tut, ist nicht gemessen.
- **A:** Mehrbild-Animationen (GIF/WebP/APNG mit mehreren Einzelbildern) und Ebenen-Formate (PSD, SVG, PDF, AI) bleiben unbeschnitten.
- Offen (nicht Teil des Auftrags, Befund für FW-PNG-Schreiber): Das Original verwirft leere Bitmaps (0×0) beim Speichern; Firepixel schreibt sie weiter (läuft fehlerfrei, Rundlauf getestet).
- Offen (re\22 §1.4): ob Radieren bis zum Rand die Objektgrenzen nachträglich verkleinert – unverändert (Objektgrenzen bleiben, nur der Rahmen folgt den Pixeln).
- Skalier-/Verzerr-Werkzeug und `Transformieren` arbeiten weiter an den Objektgrenzen (Griffe des Skalierwerkzeugs nicht geändert); im Original nicht gemessen.
