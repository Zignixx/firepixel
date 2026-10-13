# Batch-Studio: Schritt „Nachzeichnen → SVG/FW-PNG“, eingebaute Vorlagen, MCP-Werkzeug `batch`
# (Paket BATCH-9, docs\BATCH_MODERN.md §2.5, §3.13, §5.8; Code crates\fp-batch\src\steps\vector.rs,
# crates\fp-batch\src\templates.rs, crates\fp-mcp\src\batch.rs).

## Schritt vector.trace
batch-step-vector-trace = Nachzeichnen → SVG
    .description = Zeichnet Bilder als bearbeitbare Vektorpfade nach – mit Farbflächen oder glatten Kurven. Speichern als SVG (Pfade) oder FW-PNG (bearbeitbar).
batch-param-vector-trace-preset = Vorgabe
batch-param-vector-trace-engine = Verfahren
batch-param-vector-trace-colorMode = Farbmodus
batch-param-vector-trace-colors = Höchstzahl der Farben
batch-param-vector-trace-filterSpeckle = Flecken entfernen bis
batch-param-vector-trace-cornerThreshold = Eckenwinkel
batch-param-vector-trace-mode = Konturen
batch-param-vector-trace-hierarchical = Flächen
batch-param-vector-trace-threshold = Schwelle (Schwarzweiß)
batch-param-vector-trace-ignoreTransparent = Transparente Pixel auslassen
batch-param-vector-trace-pathPrecision = Nachkommastellen
batch-param-vector-trace-refine = Aufräumen
batch-param-vector-trace-turdSize = Flecken (glatte Kurven)
batch-param-vector-trace-turnPolicy = Diagonal berührende Pixel
batch-param-vector-trace-alphaMax = Ecken (glatte Kurven)
batch-param-vector-trace-optCurve = Kurven zusammenfassen
batch-param-vector-trace-optTolerance = Toleranz beim Zusammenfassen
batch-param-vector-trace-keepOriginal = Original als ausgeblendete Ebene behalten
batch-param-vector-trace-transparentCanvas = Transparente Leinwand
batch-value-vector-trace-preset-logo = Logo
batch-value-vector-trace-preset-blackWhite = Schwarzweiß
batch-value-vector-trace-preset-illustration = Illustration
batch-value-vector-trace-preset-photo = Foto
batch-value-vector-trace-engine-clusters = Farbflächen
batch-value-vector-trace-engine-potrace = Glatte Kurven
batch-value-vector-trace-colorMode-color = Farbe
batch-value-vector-trace-colorMode-blackWhite = Schwarzweiß
batch-value-vector-trace-mode-spline = Kurven
batch-value-vector-trace-mode-polygon = Gerade Segmente
batch-value-vector-trace-hierarchical-stacked = Stapeln
batch-value-vector-trace-hierarchical-cutout = Ausschneiden
batch-value-vector-trace-turnPolicy-minority = Minderheit
batch-value-vector-trace-turnPolicy-majority = Mehrheit
batch-value-vector-trace-turnPolicy-black = Schwarz
batch-value-vector-trace-turnPolicy-white = Weiß
batch-value-vector-trace-turnPolicy-left = Links
batch-value-vector-trace-turnPolicy-right = Rechts
batch-warn-trace-first-frame = Animation: nur das erste Bild wurde nachgezeichnet.

## Eingebaute Vorlagen (Menü „Rezepte ▸ Vorlagen“)
batch-template-web-webp = Für das Web verkleinern + WebP
    .description = Längste Seite 1920 px, WebP mit Qualität 82, Metadaten ohne GPS.
batch-template-email-photos = Fotos für E-Mail
    .description = Längste Seite 1280 px, JPEG mit Qualität 80, Transparenz auf Weiß, Metadaten ohne GPS.
batch-template-save-as-jpeg = Als JPEG speichern
    .description = Transparenz auf Weiß, JPEG mit Qualität 85.
batch-template-remove-background = Hintergrund entfernen → PNG
    .description = Stellt das Hauptmotiv mit dem lokalen KI-Modell frei und speichert ein transparentes PNG.
batch-template-transparency-to-white = Transparenz auf Weiß
    .description = Füllt transparente Bereiche mit Weiß, das Format bleibt.
batch-template-logos-to-svg = Logos nachzeichnen → SVG
    .description = Zeichnet Logos und Symbole als saubere Vektorpfade mit wenigen Farben nach und speichert sie als SVG.
batch-template-small-gifs = Kleine GIFs
    .description = Längste Seite 480 px, GIF mit dem Optimierer „ausgewogen“.
batch-template-strip-metadata = Metadaten entfernen
    .description = Entfernt EXIF, GPS, XMP und Kommentare; Format und Pixel bleiben.
batch-template-rename-by-date = Fotos nach Datum umbenennen
    .description = Name aus Aufnahmedatum (EXIF) und Zähler, z. B. 2026-10-13_001.jpg.
batch-template-square-1080 = Quadrat 1080
    .description = Schneidet auf ein Quadrat mit 1080 × 1080 px zu (Mitte), JPEG mit Qualität 88.
batch-template-thumbnails-256 = Vorschaubilder 256
    .description = Passt in 256 × 256 px ein, PNG, Name mit „_thumb“.

## Kommandozeile
batch-cli-arg-templates = Eingebaute Vorlagen auflisten (Name für --recipe)
batch-cli-templates-header = Eingebaute Vorlagen (firepixel-cli batch --recipe <name> …):

## MCP-Werkzeug batch
mcp-tool-batch = Batch-Verarbeitung
    .description = Verarbeitet viele Bilder mit einem Rezept wie das Batch-Studio (eigene Sitzungen, nie das offene Dokument). action: templates (eingebaute Vorlagen), steps (Schrittkatalog), validate, plan (Trockenlauf), preview (eine Datei vorher/nachher als Bilder), run (ausführen; async true liefert eine jobId), status (Fortschritt, wait_ms), cancel. recipe: Rezept-Objekt, Pfad zu .fpbatch.json oder Vorlagenname wie web-webp oder logos-to-svg.
mcp-tool-batch-action = Aktion (Vorgabe run)
mcp-tool-batch-recipe = Rezept-Objekt, JSON-Text, Pfad oder Vorlagenname
mcp-tool-batch-files = Dateien (absolute Pfade)
mcp-tool-batch-folder = Ordner mit den Dateien
mcp-tool-batch-recursive = Unterordner einbeziehen
mcp-tool-batch-out = Ausgabeordner (überschreibt das Ziel des Rezepts)
mcp-tool-batch-conflict = Konfliktregel (überschreibt das Rezept)
mcp-tool-batch-dryRun = Nur planen, nichts schreiben
mcp-tool-batch-threads = Zahl der Arbeits-Threads
mcp-tool-batch-async = Im Hintergrund starten und sofort die jobId liefern
mcp-tool-batch-jobId = Auftrag für status und cancel
mcp-tool-batch-wait_ms = status: höchstens so lange auf das Ende warten
mcp-tool-batch-file = preview: Datei
mcp-tool-batch-maxSize = preview: längste Seite der Bilder in Pixeln
mcp-batch-err-job-id = jobId fehlt (aus run mit async true).
mcp-batch-err-unknown-job = Unbekannter Auftrag { $id }.
mcp-batch-err-action = Unbekannte Aktion „{ $action }“. Möglich: { $actions }.
mcp-batch-err-file = preview braucht file (Pfad einer Bilddatei).
mcp-batch-hint-status = Fortschritt: batch mit action status und jobId { $id } (wait_ms bis 60000); abbrechen mit action cancel.
mcp-res-batch = Batch-Verarbeitung
    .description = Vorlagen und Schritte des Batch-Studios mit Parametern
mcp-res-batch-title = Batch-Verarbeitung: Vorlagen und Schritte
mcp-res-batch-intro = Werkzeug batch: recipe ist ein Rezept-Objekt, ein Pfad zu .fpbatch.json oder ein Vorlagenname. Lange Läufe mit async true starten und mit status abfragen.
mcp-res-batch-templates = Eingebaute Vorlagen
mcp-res-batch-steps = Schritte
