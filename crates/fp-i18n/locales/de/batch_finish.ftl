# Batch-Studio, Feinschliff Teil 2 (BATCH-11b, docs\BATCH_MODERN.md §2.3, §2.8, §3.2, §3.10):
# Felder der Endkarte „Speichern als“ je Format, zusätzliche FW-PNG, HTML-Bericht, ffmpeg-Hinweis in
# Liste und Karte, Größen-/Pixelfilter, Vorlage „Video → kleines GIF“.

## Endkarte „Speichern als“
batch-param-output-alsoFwpng = Zusätzlich als FW-PNG
    .tooltip = Schreibt neben jede Datei eine bearbeitbare FW-PNG mit demselben Namen (z. B. SVG und FW-PNG aus „Nachzeichnen“ in einem Lauf).
batch-param-output-gif-background = Hintergrund
    .tooltip = Füllfarbe für halbtransparente Pixel, wenn das GIF keine Transparenz speichert.
batch-ui-gif-section = GIF-Optimierer
batch-ui-gif-reached = Erreicht: { $size } (Ziel { $target })
batch-ui-gif-missed = Zielgröße nicht erreichbar: kleinste Datei { $size } (Ziel { $target })
batch-ui-gif-size-pending = Größe wird berechnet …
batch-ui-gif-size = Ergebnis der Vorschaudatei: { $size }
batch-ui-trace-pixel-hint = „Nachzeichnen“ liefert Pfade. Für Vektoren hier SVG oder FW-PNG wählen; mit einem Pixelformat wird das Ergebnis flach gerechnet.
batch-ui-jpeg-white-hint = JPEG speichert keine Transparenz: Transparente Bereiche werden mit der Farbe „Hintergrund“ (Vorgabe Weiß) gefüllt.
batch-warn-gif-target-missed = Zielgröße { $target } nicht erreichbar, kleinste Datei { $reached }.
batch-warn-extra-exists = { $name } gibt es schon, nicht überschrieben (Regel „Überspringen“).

## ffmpeg-Hinweis in Dateiliste und Karte „Video lesen“
batch-ui-ffmpeg-missing-row = ffmpeg fehlt – Video kann nicht gelesen werden
batch-ui-ffmpeg-missing-card = Für Videos braucht Firepixel ffmpeg. Firepixel liefert ffmpeg nicht mit und lädt nichts herunter: Wählen Sie eine vorhandene ffmpeg-Datei.
batch-ui-ffmpeg-choose = ffmpeg-Datei…
    .tooltip = Öffnet die Batch-Einstellungen, Abschnitt „Videos (ffmpeg)“.

## Filter nach Größe und Pixeln (Profi)
batch-ui-filter-size = Dateigröße (MB)
batch-ui-filter-pixels = Längste Seite (px)
batch-ui-filter-min = ab
batch-ui-filter-max = bis
batch-ui-filter-reset = Filter zurücksetzen
batch-ui-filter-glob-hint = Platzhalter * und ? (z. B. *.png)

## Ende-Karte
batch-ui-done-open-report = Bericht ansehen
    .tooltip = Öffnet batch-report.html mit Vorschaubildern im Browser.

## Vorlage „Video → kleines GIF“
batch-template-video-to-gif = Video → kleines GIF
    .description = Liest das Video mit 12 Bildern/s, längste Seite 480 px, GIF mit dem Optimierer „ausgewogen“ (braucht ffmpeg).

## HTML-Bericht batch-report.html
batch-html-title = Batch-Bericht
batch-html-recipe = Rezept „{ $name }“
batch-html-when = { $start } bis { $end }
batch-html-dry-run = Trockenlauf – nichts geschrieben
batch-html-cancelled-run = Abgebrochen
batch-html-tile-done = fertig
batch-html-tile-skipped = übersprungen
batch-html-tile-failed = Fehler
batch-html-tile-cancelled = nicht verarbeitet
batch-html-tile-size = Größe vorher → nachher ({ $change })
batch-html-tile-size-in = Größe der Quellen
batch-html-tile-duration = Dauer
batch-html-seconds = { $s } s
batch-html-errors = Fehler ({ $n })
batch-html-col-file = Datei
batch-html-col-step = Schritt
batch-html-col-message = Meldung
batch-html-files = Dateien ({ $n })
batch-html-before = Vorher
batch-html-after = Nachher
batch-html-no-preview = keine Vorschau
batch-html-ms = { $ms } ms
batch-html-also = Zusätzlich: { $name }
batch-html-footer = Erstellt mit Firepixel · { $time }
batch-html-status-done = Fertig
batch-html-status-planned = Geplant
batch-html-status-skipped = Übersprungen
batch-html-status-failed = Fehler
batch-html-status-cancelled = Nicht verarbeitet

## Kommandozeile: firepixel-cli batch --help
batch-cli-help-templates = Eingebaute Vorlagen (Name für --recipe):
batch-cli-help-legacy = Ältere Form ohne Rezept: firepixel-cli batch --actions <json|datei> --in <ordner> --out <ordner> [--format png|fwpng|…] (Hilfe: firepixel-cli batch --actions x --help)
