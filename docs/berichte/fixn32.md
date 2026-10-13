# FIX-N32 – Blauer Auswahlrahmen doppelt (Nutzerbefund N32)

Stand 13.10.2026, Slot `fp-s4`.

## Ursache

BM-1 (N26) hat nur den **gezeichneten** Auswahlrahmen und die Hervorhebung beim Überfahren auf die sichtbaren
Pixel umgestellt (`docops::element_frame_bounds`). Der Zeiger und das Transformieren-Werkzeug rechneten weiter
mit den **gespeicherten Objektgrenzen** (`docops::ids_bounds` → `element_bounds`):

| Zeichenpfad | Quelle vorher | Bild |
|---|---|---|
| Auswahlrahmen mit Eckgriffen (`canvas/mod.rs`, `show_edges`) | `element_frame_bounds` (sichtbare Pixel) | enges Rechteck mit Griffen |
| Verschiebe-Vorschau `Overlay::Ghost` (Zeiger, `PState::Moving`) und das Nachleuchten `ghost::Linger` nach dem Loslassen | `PointerTool::selection_bounds` = `ids_bounds` (Objektgrenzen) | **großes Rechteck ohne Griffe**, ragt über die Bildkante |
| Trefferprüfung der Griffe, Skalier-Vorschau (`Overlay::Rect` + `Handles`) | `selection_bounds` = Objektgrenzen | Griffe unsichtbar an den Ecken der Objektgrenzen |
| Transformieren-Werkzeuge (`TransformTool::current`) | `ids_bounds` | Transformationsrahmen um die Objektgrenzen |

Bei einer Bitmap, deren gespeicherte Grenzen größer sind als die sichtbaren Pixel (FW-PNG aus älteren
Firepixel-Versionen bzw. Dateien wie die Hand mit Schwert), erschienen beim Anklicken mit kleinster Mausbewegung
(Ziehschwelle) bzw. beim Verschieben und kurz nach dem Loslassen zwei Rechtecke. Zoom und Lage außerhalb der
Leinwand ändern nichts an der Ursache (beide Rechtecke in Dokumentkoordinaten, gleiche Abbildung).

## Umsetzung

- `crates\fp-engine\src\docops.rs`: neu `ids_frame_bounds` (Vereinigung von `element_frame_bounds`).
- `crates\fp-engine\src\tools\pointer.rs`: `PointerTool::selection_bounds` und `TransformTool::current`
  nutzen `ids_frame_bounds`. Damit: Ghost/Nachleuchten, Griff-Trefferprüfung, Skalier-Vorschau, Fangen beim
  Verschieben und Transformationsrahmen deckungsgleich mit dem gezeichneten Rahmen. Die Transformation selbst
  ist eine Matrix (Rechteck → Viereck) und wirkt unverändert auf die ganze Bitmap.
- Unverändert (bewusst, wie BM-1): Inspektor X/Y/B/H, `getSelectionBounds`, Info-Bedienfeld nutzen weiter die
  Objektgrenzen (`ids_bounds`).
- Hervorhebung beim Überfahren war schon richtig (BM-1).

## Tests

- `crates\fp-engine\tests\bitmap_bounds_bm1.rs` (+3, alle 12 grün; ohne die Änderung schlagen die 3 neuen fehl):
  - `n32_move_preview_uses_visible_pixels`: Overlay beim Ziehen enthält genau ein Rechteck = sichtbare Pixel,
    auch weit über die Leinwand hinaus (350…450 bei Leinwand 400); nach dem Ablegen erneut.
  - `n32_handles_sit_on_visible_frame`: Eckgriff des sichtbaren Rahmens skaliert von dort aus; Ziehen an der
    alten Ecke der Objektgrenzen skaliert nicht.
  - `n32_transform_box_uses_visible_pixels`: Skalieren-Werkzeug zeigt den Rahmen um die sichtbaren Pixel.
- `cargo test -p fp-ui --test all bitmap_frame_bm1::` grün; `cargo test -p fp-ui --lib canvas::` 45/46 – der
  eine Fehlschlag `canvas::tests::open_dialog_blocks_canvas_and_export_area_opens_dialog` (Exportbereich,
  `tests.rs:1269`) besteht genauso **ohne** diese Änderung (vorbestehend, nicht N32).
- `cargo check --workspace --all-targets` ohne Warnungen; `cargo test -p fp-neutral` grün.

## Echtes Fenster, Brücke

Slot-Build (debug), eigenes Profil, Fenster 1000×720 unten rechts. Datei `re\_testdateien\re5a_t1_rand_fp.fw.png`
(Objekt 0,0,400,300; sichtbar nur das rote Quadrat 100,50,200,150):
- `docs\screenshots\fixn32_ziehen_100.png`: 100 %, Quadrat beim Ziehen halb über die Leinwand hinaus → ein Rahmen
  mit Eckgriffen um das Quadrat, kein zweites Rechteck.
- `docs\screenshots\fixn32_danach_100.png`: nach dem Loslassen ein Rahmen.
- `docs\screenshots\fixn32_ziehen_150.png`: 150 %, beim Ziehen ein Rahmen.

## Für PARITAET / NACHPRUEFUNG

- N32 → 🔧 (umgesetzt, Nachprüfung offen). NACHPRUEFUNG: Hand-mit-Schwert-Datei des Nutzers bzw.
  `re5a_t1_rand_fp.fw.png` öffnen, Bitmap anklicken, verschieben (auch teilweise über die Leinwand), bei 100 % und
  150 %, außerdem Skalieren-Werkzeug (Q) → jeweils genau ein blauer Rahmen um die sichtbaren Pixel, Griffe an
  dessen Ecken.

## Neue Befunde

- Vorbestehend rot: `fp-ui` `canvas::tests::open_dialog_blocks_canvas_and_export_area_opens_dialog` (Export-Dialog
  übernimmt den Exportbereich nicht, `area = None`).
