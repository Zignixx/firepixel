# BATCH-5 – Metadaten, Formate, Optimieren (Batch-Studio)

Stand 13.10.2026, Spezifikation `docs\BATCH_MODERN.md` §3.2, §3.7, §3.12, §5.10; Kern-API aus
`docs\berichte\batch1.md`. Commit: `BATCH-5: Metadaten (EXIF/XMP/IPTC/ICC behalten/entfernen/setzen,
Ausrichtung), TIFF-Kompression + TGA, PNG verlustfrei, Farben reduzieren, Zielgröße in KB …`
(siehe `git log --grep BATCH-5`).

## Umgesetzt

| Datei | Inhalt |
|---|---|
| `crates\fp-batch\src\meta\mod.rs` | Lesen beim Dekodieren (`read_into`: EXIF/ICC/XMP/IPTC/Kommentar aus JPEG, PNG, WebP → `Meta`), EXIF-Ausrichtung anwenden (Pixel, Tag + XMP `tiff:Orientation` = 1), `finish_encode` (Formatoptionen aus `Meta::extra`, eigener TIFF-Kodierer, PNG-Optimierung, Zielgröße, Metadaten schreiben, Hinweis bei Formaten ohne Metadaten), `info()` (Info-Tabelle vorher/nachher als JSON: Kamera, Datum, GPS ja/nein, Urheber, ICC-Beschreibung …), `install()` (EXIF-Leser für `{exif-date}`/Kamera-Bausteine über `item::set_exif_probe`, TGA-Kodierer) |
| `src\meta\exif.rs` | eigener EXIF/TIFF-Leser und -Schreiber: IFD0, Exif-, GPS-, Interop-IFD, IFD1 + Vorschaubild, LE/BE, Offsets neu; `summary()`, Tags entfernen, ASCII setzen, Ausrichtung an Ort und Stelle patchen (unveränderte Blöcke bleiben Byte für Byte erhalten), Tag-Namen (`Make`, `0x0112`, Zahl) |
| `src\meta\container.rs` | JPEG (APP1 EXIF/XMP, APP2 ICC in Teilen > 64 KB, APP13 IPTC, COM), PNG (`eXIf`, `iCCP`, `iTXt` XMP/Comment; `sRGB` weicht bei ICC), WebP (einfach → `VP8X` mit ICCP/EXIF/XMP, Alpha-Flag aus VP8L) – lesen und schreiben, idempotent |
| `src\meta\xmp.rs` | XMP ohne XML-Crate: Eigenschaften entfernen (Attribut/Element), Ausrichtung, `dc:creator/rights/description/title/subject`, `xmp:Rating` setzen bzw. neues Paket |
| `src\meta\encoders.rs` | TIFF mit `compression` none/lzw (Vorgabe)/deflate/packbits, Prädiktor 2, ICC- und XMP-Tag; eigener LZW-Kodierer (TIFF „early change“); TGA RLE/unkomprimiert, 24/32 Bit, oben links |
| `src\meta\pngopt.rs` | verlustfreie PNG-Optimierung (Palette 1/2/4/8 Bit mit kurzem `tRNS`, Grau 1–8 Bit, Grau+Alpha, Alpha weg, Alpha-Bereinigung, Filter 0–4/adaptiv, ab Stufe 4 zeilenweise nach Kompression, Deflate 9; nie größer) |
| `src\meta\quantize.rs` | Median-Cut RGBA + 2× k-means, Dithering none/ordered (Bayer 8×8)/Floyd-Steinberg (Serpentine) |
| `src\meta\target.rs` | Zielgröße: Bisektion über eine Leiter (JPEG/WebP/AVIF Qualität bis `minQuality`, GIF Farben 256→2, PNG verlustfrei → Farben 256→16), danach optional verkleinern bis `minScale`; ≤ 8 Kodierungen; Hinweis mit erreichter Größe |
| `src\steps\metadata.rs` | `metadata.edit` (keep all/none/chosen, remove gps/camera/software/thumbnails/xmp/iptc/comments/icc, `tags` einzelne EXIF-Tags), `metadata.set` (artist, copyright, description, title, keywords, rating, dateTaken fest oder `+2h`/`-1d 30m`, gps remove; Platzhalter `{name}` `{n}` `{date}` `{year}`), `metadata.orientation` |
| `src\steps\optimize.rs` | `optimize.png` (level 1–6, strip none/safe/all, alphaClean), `optimize.quantize` (colors, dither, ditherAmount; PNG → Palette-PNG, GIF ohne zweites Dithern), `optimize.targetSize` (maxKB, strategy quality/qualityResize, minQuality, minScale) |
| `crates\fp-i18n\locales\{de,en}\batch_meta.ftl` | alle Texte (Schritte, Parameter, Kategorien, Zusammenfassungen, Hinweise) |
| `crates\fp-batch\tests\meta_optimize.rs` | 14 Tests (s. u.) |
| `tools\golden\batch\metadata-no-gps\`, `tools\golden\batch\target-size-100kb\` | Golden-Fälle: `in\`, `recipe.json`, `expected.json` (`FP_BLESS=1` schreibt neu) |

Regel SPEC §3.7 umgesetzt: Ohne Schritt „Metadaten“ wird alles übernommen, soweit das Zielformat es
trägt (JPEG, PNG/APNG, WebP; TIFF nur ICC/XMP), die Ausrichtung wird angewendet und auf 1 gesetzt;
GIF/BMP/QOI/ICO/WBMP/TGA/AVIF → Hinweis „… speichert keine Metadaten (EXIF, ICC …)“. Vorschau mit
Dateigröße: `pipeline::preview(.., encode_full)` misst dieselbe Datei wie der Lauf (Test).

## Tests (alle grün)

`cargo test -p fp-batch` (meta_optimize 14, recipe_naming 18, run 17), `cargo clippy -p fp-batch
--all-targets` ohne Warnungen in fp-batch, `cargo check --workspace --all-targets`, `cargo test -p
fp-i18n`, `cargo test -p fp-neutral`, Scanner `fp-ui --test all i18n_keine_festen_texte::`.
Geprüft: EXIF-Rundlauf LE/BE, Zusammenfassung, Vorschaubild; alle 8 Ausrichtungen; Container-
Rundläufe (ICC 150 KB in JPEG-Teilen, zweimal schreiben = einmal, leer schreiben entfernt alles,
WebP VP8/VP8L dekodierbar); Konvertieren JPEG→JPEG/PNG/WebP behält Metadaten, Bild gedreht
(20×40, oben rot); GIF/BMP mit Hinweis; `metadata.edit` GPS (auch im XMP), Kategorien, einzelne Tags,
`none`, ungültige Tags; **§4b**: Schritt zweimal = einmal (gleiche Bytes), Parameter 50× ändern → ein
Schritt mit Endwert, `metadata.set` zweimal → genau ein `dc:creator`/`dc:subject` (Verschieben des
Datums wirkt bewusst je Anwendung), Vorschau zweimal gleich; TIFF alle Kompressionen + TGA pixelgenau
(auch Alpha, LZW mit Tabellen-Reset); PNG-Optimierung verlustfrei auf allen Stufen, Palette 1 Bit < ½,
Grau, nie größer, `strip safe` behält ICC; Quantisieren ≤ n Farben, exakt bei wenigen Farben,
Palette-PNG, GIF; Zielgröße JPEG/WebP/PNG erreicht, großzügig = ohne Schritt, unerreichbar → Hinweis,
mit Verkleinern erreicht; Golden „Metadaten ohne GPS“ und „Zielgröße 100 KB“ (Größe ± 5 %).

## Einhängungen in fremde Dateien (Befunde)

- `crates\fp-batch\src\item.rs` (`decode_bytes`): `let mut item` + 1 Zeile `crate::meta::read_into(data, &mut item);`.
- `crates\fp-batch\src\output.rs`: `encode` ruft `crate::meta::finish_encode(.., encode_plain)`; der
  bisherige Rumpf heißt `encode_plain` (öffentlich, unverändert).
- `crates\fp-batch\src\steps\files.rs`: Endkarte `output` + Parameter `compression`
  (default/fast/max/none/lzw/deflate/packbits).
- `crates\fp-batch\src\steps\mod.rs`, `lib.rs`: je Modulzeile + Registrierung.
- `crates\fp-batch\Cargo.toml`: `flate2`, `crc32fast`, `png` (alle schon im Baum, Workspace); `Cargo.lock`.
- **Für BATCH-4 (`rotate.autoExif`)**: Die EXIF-Ausrichtung ist nach dem Laden schon angewendet
  (`Meta::exif` hat Ausrichtung 1, `Meta::extra["meta.orientationApplied"]` = ursprünglicher Wert).
  `autoExif` darf nur nach `fp_batch::meta::orientation(meta)` drehen, **nicht** nach
  `ItemInfo.exif.orientation` (sonst doppelt gedreht).
- **Für BATCH-2 (UI)**: Info-Tabelle über `fp_batch::meta::info(&meta)` (vorher: `item::load`, nachher:
  `PreviewResult.after_item`); Werte-Beschriftungen der Auswahlen (`keep`, `strip`, `strategy`, `dither`)
  fehlen noch als eigene Schlüssel – Vorschlag `batch-param-<id>-<param>-<wert>` in `batch_meta.ftl`.
- **Für I18N/HG**: `crates/fp-batch/src` in `SCAN_DIRS` des Scanners aufnehmen (wie BATCH-1 gemeldet).

## Bibliotheken / Annahmen

- **Keine neue Crate.** `oxipng` 9 bindet `libdeflater` (C) fest ein → nicht reines Rust (Regel §1),
  daher eigene Optimierung; `quantette`, `kamadak-exif`, `img-parts` sind nicht im Baum (kein Netz-
  Download nötig) → eigene Umsetzung (Lizenz Apache-2.0 des Projekts, kein GPL).
- `keep: none` entfernt EXIF/XMP/IPTC/Kommentar, das **Farbprofil bleibt** (es ist Farbinformation,
  keine persönliche Angabe); entfernen nur mit `remove: ["icc"]`. `keep: chosen` = wie `all` mit der
  `remove`-Liste (die UI zeigt sie als Auswahl).
- `optimize.png` mit `strip: safe` (Vorgabe, wie oxipng) entfernt EXIF/XMP/IPTC/Kommentar, behält ICC;
  wirkt nur bei „Speichern als PNG“, sonst Hinweis.
- Beim Entfernen wird der EXIF-Block neu geschrieben; Herstellerdaten (MakerNote) mit absoluten
  Offsets können dabei unbrauchbar werden – bei „camera“ werden sie ohnehin entfernt.
- XMP in JPEG/PNG braucht die Pflicht-Kennung mit dem Herstellernamen des Standards; sie wird zur
  Laufzeit aus Bytes gebildet (kein Name im Quelltext, fp-neutral grün). Golden-Eingaben enthalten
  deshalb kein XMP (die Datei läge sonst im Repo).
- Zielgröße: Standard `maxKB` 200; Vorgabe-Startqualität = eingestellte Qualität der Endkarte;
  Verkleinern nur bei `qualityResize`; Animationen: alle Frames werden verkleinert, GIF über Farben
  (BATCH-8 hängt den eigenen GIF-Optimierer über `register_encoder` ein, die Leiter setzt dann
  `colors`). FW-PNG: Hinweis, keine Suche.
- `metadata.set` schreibt Texte als UTF-8 in ASCII-Tags (übliche Praxis, z. B. „©“).
- Datumsverschiebung ohne vorhandenes Datum: Hinweis, nichts geändert.

## Offen

- AVIF/JXL/HEIF: Metadaten lesen und schreiben (Box-Ebene) fehlt → Hinweis; TIFF-Quelle: Metadaten
  werden nicht gelesen; TIFF-Ausgabe ohne EXIF-IFD.
- WebP `alphaQuality`/`method`, AVIF `alphaQuality`: `fp-export::write_webp/write_avif` reichen sie
  nicht durch (Befund RF/fp-export). PSD-Ausgabe → BATCH-9.
- `metadata.set` GPS setzen (Koordinaten), IPTC schreiben, `metadata.export` (P3).
- `format.colorProfile` (Umrechnung nach sRGB) nicht Teil dieses Pakets.

## Nachprüfung (für `docs\NACHPRUEFUNG.md`)

- Echte Handyfotos (hochkant, mit GPS) → Rezept „Für das Web“: Ergebnis aufrecht, im Explorer
  (Eigenschaften ▸ Details) kein GPS, Kamera/Datum vorhanden; in einem Bildbetrachter mit
  Farbverwaltung gleiche Farben wie das Original (ICC übernommen).
- Zielgröße 100 KB über 20 gemischte Fotos: alle ≤ 100 KB, Bericht ohne Hinweis; mit 5 KB + „nur
  Qualität“ → ⚠ mit erreichter Größe.
- PNG-Screenshots mit `optimize.png`: Größe sinkt (Erwartung 10–40 %), Pixel gleich (Vergleich in der
  Bildvorschau).
- TIFF (LZW/Deflate) und TGA aus dem Batch in einem externen Programm öffnen.
