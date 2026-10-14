# G-B5 – Paket B5 aus der Mess-Sitzung B-2b

Grundlage: `docs\berichte\b2b.md` (BA-017, BA-123, BA-124, Nebenbefund N1). Slot `fp-s4`.

## Umgesetzt (Commit `0e8c6ae`)

| Punkt | Änderung | Codestelle | Test |
|---|---|---|---|
| BA-017 | `rawHeight` (und bei senkrechter Linie `rawWidth`) eines Texts am Pfad nach den Messwerten: offene Linie stückweise linear über der Pfadhöhe durch die Messpunkte bei 14 pt (0 → 5, 20 → 9, 40 → 12, 80 → 19, 160 → 31, darüber Steigung 0,15), skaliert mit Schriftgröße/14; senkrechte Linie (Breite < 1 px) 10 × 65 (skaliert); geschlossene Form Pfadhöhe/2 + 1; Rechteck-Grundform 5 (B-2) | `crates\fp-script\src\imp\elem.rs` `text_raw_rect`, `path_raw_size` | `gb5_messwerte::ba017_raw_height_of_text_on_path_matches_measurements` (10 Fälle: Linie 14/28 pt, Höhe 20/40/80/160, Höhe 40 bei 28 pt, senkrecht, Ellipse 100 × 240, Kreis 120) |
| BA-123 | `reorderLayer(<Unterebene>, <Ebene der obersten Stufe>, …)`: die Unterebene verlässt ihre Elternebene und wird Ebene der obersten Stufe direkt unter dem Ziel (flach vor dem Ziel), in beiden Richtungen; sie wird aktiv (`currentLayerNum` = ihr Index). Unterebene → Unterebene unverändert (BA-075) | `crates\fp-script\src\imp\doc2.rs` `move_sublayer`, `reorderLayer` | `gb5_messwerte::ba123_sublayer_onto_top_level_becomes_top_level_below_target` (die drei Beispiele des Originals) |
| BA-124 | Einheit Pixel/cm bleibt im Dokument: `createFirepixelDocument(…, {units: "cm"})` rechnet nicht mehr in dpi um (`file.new` neuer Parameter `perCm`); `resolution` 28,35 / 50 / 59,06, `resolutionUnits` `cm`, auch nach Speichern/Öffnen. `pHYs` = `trunc(px/cm) · 100` (2800 / 5000 / 5900) in FW-PNG (`make_phys`) und im PNG-Export (`ExportContext::resolution_per_cm`, `PngOptions::per_cm`); dpi unverändert (BA-035) | `crates\fp-fwpng\src\png.rs`, `crates\fp-export\src\{pipeline,png_out,flat_png,doc_export}.rs`, `crates\fp-engine\src\commands\file.rs`, `crates\fp-script\src\imp\fw.rs` | `gb5_messwerte::ba124_resolution_per_cm_kept_and_phys_truncated`, `fp-export tests\gb5_phys_cm.rs` |
| N1 | `dom.setExportOptions({...})` setzt die Exporteinstellungen der Seite (JSF-Schlüssel `exportFormat`, `colorMode`, `paletteMode`, `numEntriesRequested`/`numPaletteEntries`, `webSnapAdaptive`, `webSnapTolerance`, `jpegQuality`, `lossyGifAmount`, `interlacedGIF`; Formatwechsel ohne `colorMode`: GIF indiziert, JPEG 24 Bit) über `pi.export.settings` – ein Verlaufsschritt wie im Original | `crates\fp-script\src\imp\doc.rs` `setExportOptions`, `export_settings_from_json` | `gb5_messwerte::n1_set_export_options_changes_document_export_settings` |

Neuer Befehlstext (de/en) in `command_texts.ftl`: `cmdtext-aufloesung-in-pixel-cm-einheit-bleibt-erhalten`.

Prüfungen: `gb5_messwerte` 4/4, `b7_messwerte`, `b5_skript`, `f6_flat_layers`, `f8_flat_layer_numbers`, fp-export `gb5_phys_cm` + `golden`, fp-fwpng lib, fp-i18n – grün; `cargo check --workspace --all-targets` ohne Warnungen.

## Erledigte Befunde

- BEFUNDE: „(von F6/M1, klein, unsicher) **Rahmen „roh“ beim Anfügen an einen Pfad:** …“ (BA-017), „(von B-5, Messung) `reorderLayer` einer Unterebene …“ (BA-123), „(von B-7, Messung) pHYs bei Einheit Pixel/cm …“ (BA-124) – abgehakt.
- BEFUNDE_ABBAU: BA-017, BA-123, BA-124 und N1 mit „Ergebnis G-B5: erledigt“.

## PARITAET

BA-017, BA-123, BA-124, N1 → „umgesetzt, Nachprüfung ausstehend“.

## Nachprüfung (für NACHPRUEFUNG.md)

- BA-017: Text „Hallo Welt“ 14 pt an Linie 350 × 80 anfügen → `rawHeight` 19; an Ellipse 100 × 240 → 100 × 121; an senkrechte Linie → 10 × 65 (wie Original).
- BA-123: Ebenen E1, A (A1, A2), B; `reorderLayer(A1, B, false)` → flach `E1, A, A2^A, A1, B`, A1 aktiv; im Ebenen-Bedienfeld erscheint A1 als eigene Ebene unter B.
- BA-124: `createFirepixelDocument({x:20,y:10}, {pixelsPerUnit:28.35, units:"cm"})` → Dokumenteigenschaften zeigen 28,35 Pixel/cm; gespeicherte Datei und PNG-Export `pHYs` 2800.
- N1: `setExportOptions({exportFormat:"GIF", paletteMode:"web216"})` → Dokument-Inspektor zeigt „GIF Web 216“, ein Verlaufsschritt.

## Offene Punkte / neue Befunde

- BA-017: B-2 maß für eine schräge Linie der Höhe 40 `rawHeight` 44, B-2b dagegen 12 (Linie 350 breit). Der B-2-Wert stammt vermutlich von einer anderen Linienlänge/Text; umgesetzt ist die B-2b-Tabelle. Zwischenwerte und andere Schriften sind angenähert (Regel des Originals unbekannt).
- BA-123: `reorderLayer` einer Unterebene auf eine Ebene der obersten Stufe mit `makeCopy = true` bzw. auf die Web-Ebene ist nicht gemessen; es gilt dieselbe Regel.
- N1: Liegen in einer alten Datei Exporteinstellungen nur als `legacy_export` vor, bleiben diese vor den neu gesetzten sichtbar (Getter liest `legacy_export` zuerst) – klein, unbestätigt.
- i18n-Scanner (fp-ui) nicht gelaufen (nur neuer Befehlstext in `command_texts.ftl`, keine UI-Texte); fällt in den Nachtlauf.
