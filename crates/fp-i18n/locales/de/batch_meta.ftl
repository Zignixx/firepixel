# Batch-Studio: Metadaten und Optimieren (BATCH-5, docs\BATCH_MODERN.md §3.7, §3.12).
# Präfixe: batch-meta-, batch-step-metadata-*, batch-step-optimize-*, batch-param-metadata-*,
# batch-param-optimize-*, batch-param-output-compression.

## Schritte
batch-step-metadata-edit = Metadaten
    .description = Metadaten behalten oder entfernen, z. B. GPS-Position, Kamera oder Vorschaubild.
batch-step-metadata-set = Metadaten setzen
    .description = Urheber, Copyright, Beschreibung, Stichwörter oder Aufnahmedatum eintragen.
batch-step-metadata-orientation = Ausrichtung anwenden
    .description = Dreht das Bild nach der Kamera-Ausrichtung und setzt die Angabe zurück.
batch-step-optimize-png = PNG verlustfrei optimieren
    .description = Macht PNG-Dateien kleiner, ohne ein Pixel zu verändern.
batch-step-optimize-quantize = Farben reduzieren
    .description = Verringert die Zahl der Farben (z. B. für kleine PNG8- oder GIF-Dateien).
batch-step-optimize-targetSize = Zielgröße
    .description = Wählt die beste Qualität, bei der die Datei höchstens so groß wird.

## Parameter
batch-param-metadata-edit-keep = Behalten
batch-param-metadata-edit-remove = Entfernen
batch-param-metadata-edit-tags = Einzelne EXIF-Tags entfernen
batch-param-metadata-set-artist = Urheber
batch-param-metadata-set-copyright = Copyright
batch-param-metadata-set-description = Beschreibung
batch-param-metadata-set-title = Titel
batch-param-metadata-set-keywords = Stichwörter
batch-param-metadata-set-rating = Bewertung
batch-param-metadata-set-dateTaken = Aufnahmedatum
batch-param-metadata-set-gps = GPS-Position
batch-param-optimize-png-level = Stufe
batch-param-optimize-png-strip = Metadaten entfernen
batch-param-optimize-png-alphaClean = Unsichtbare Farben bereinigen
batch-param-optimize-quantize-colors = Farben
batch-param-optimize-quantize-dither = Dithering
batch-param-optimize-quantize-ditherAmount = Dithering-Stärke
batch-param-optimize-targetSize-maxKB = Höchstens
batch-param-optimize-targetSize-strategy = Verfahren
batch-param-optimize-targetSize-minQuality = Mindestqualität
batch-param-optimize-targetSize-minScale = Kleinste Größe
batch-param-output-compression = Kompression

## Kategorien
batch-meta-cat-gps = GPS-Position
batch-meta-cat-camera = Kamera
batch-meta-cat-software = Software
batch-meta-cat-thumbnails = Vorschaubild
batch-meta-cat-xmp = XMP
batch-meta-cat-iptc = IPTC
batch-meta-cat-comments = Kommentare
batch-meta-cat-icc = Farbprofil
batch-meta-what-comment = Kommentar
batch-meta-dither-none = ohne Dithering
batch-meta-dither-ordered = geordnetes Dithering
batch-meta-dither-floydSteinberg = Floyd-Steinberg

## Zusammenfassungen
batch-meta-summary-remove-all = Alle Metadaten entfernen
batch-meta-summary-keep-all = Alle Metadaten behalten
batch-meta-summary-remove = { $list } entfernen, Rest behalten
batch-meta-summary-orientation = Nach Kamera-Ausrichtung drehen
batch-meta-summary-png = Verlustfrei, Stufe { $level }
batch-meta-summary-quantize = { $colors } Farben, { $dither }
batch-meta-summary-target = Höchstens { $kb } KB

## Hinweise und Fehler
batch-meta-warn-format-drops = { $format } speichert keine Metadaten ({ $what }); sie fehlen in der neuen Datei.
batch-meta-warn-png-only = PNG-Optimierung wirkt nur bei „Speichern als PNG“ (gewählt: { $format }).
batch-meta-warn-target-format = Zielgröße: { $format } hat keine Qualitätseinstellung; nur „Qualität + Größe“ kann die Datei verkleinern.
batch-meta-warn-target-missed = Zielgröße { $target } KB nicht erreicht, kleinste Datei: { $size } KB.
batch-meta-info-target-resized = Für die Zielgröße auf { $percent } % verkleinert.
batch-meta-info-target-lossy = Für die Zielgröße verlustbehaftet statt verlustfrei gespeichert.
batch-meta-warn-no-date = Aufnahmedatum nicht geändert: Die Datei hat kein Datum zum Verschieben.
batch-meta-issue-tags = Ungültige EXIF-Tags (Zahl, „0x0112“ oder Name wie „Make“).
batch-meta-issue-date = Ungültiges Datum (z. B. „2026-10-13 14:30“ oder „+2h“, „-1d 30m“).
batch-meta-err-size = { $format } erlaubt höchstens { $max } × { $max } Pixel.

## Auswahlwerte (BATCH-11a, Schema batch-value-<id>-<param>-<wert>)
batch-value-metadata-edit-keep-all = Alle behalten, außer Auswahl
batch-value-metadata-edit-keep-none = Alle entfernen
batch-value-metadata-edit-keep-chosen = Nur Auswahl entfernen
batch-value-metadata-edit-remove-gps = GPS-Position
batch-value-metadata-edit-remove-camera = Kameradaten
batch-value-metadata-edit-remove-software = Software
batch-value-metadata-edit-remove-thumbnails = Vorschaubilder
batch-value-metadata-edit-remove-xmp = XMP
batch-value-metadata-edit-remove-iptc = IPTC
batch-value-metadata-edit-remove-comments = Kommentare
batch-value-metadata-edit-remove-icc = Farbprofil (ICC)
batch-value-metadata-set-gps-keep = Behalten
batch-value-metadata-set-gps-remove = Entfernen
batch-value-optimize-png-strip-none = Nichts entfernen
batch-value-optimize-png-strip-safe = Sicher (Farbprofil bleibt)
batch-value-optimize-png-strip-all = Alles entfernen
batch-value-optimize-quantize-dither-none = Ohne
batch-value-optimize-quantize-dither-ordered = Geordnet
batch-value-optimize-quantize-dither-floydSteinberg = Floyd-Steinberg
batch-value-optimize-targetSize-strategy-quality = Nur Qualität
batch-value-optimize-targetSize-strategy-qualityResize = Qualität, dann Größe
batch-value-output-compression-default = Standard
batch-value-output-compression-fast = Schnell
batch-value-output-compression-max = Maximal
batch-value-output-compression-none = Ohne
batch-value-output-compression-lzw = LZW
batch-value-output-compression-deflate = Deflate
batch-value-output-compression-packbits = PackBits
