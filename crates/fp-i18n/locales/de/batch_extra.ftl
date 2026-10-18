# Batch-Studio, BA-109 (docs\BATCH.md §3): weitere Schritte in crates\fp-batch\src\steps\extra.rs.
# Schritte batch-step-<id>, Parameter batch-param-<id>-<param>, Werte batch-value-<id>-<param>-<wert>.

## Motiv zentrieren
batch-step-background-subjectCenter = Motiv zentrieren
    .description = Findet das Motiv, schneidet darauf zu und zentriert es mit gleichmäßigem Rand.
batch-param-background-subjectCenter-margin = Rand
batch-param-background-subjectCenter-square = Quadratisch
batch-param-background-subjectCenter-shadow = Schatten unter dem Motiv
batch-param-background-subjectCenter-detect = Motiv erkennen über
batch-param-background-subjectCenter-tolerance = Toleranz
batch-param-background-subjectCenter-fill = Neue Fläche
batch-param-background-subjectCenter-color = Farbe
batch-value-background-subjectCenter-detect-auto = Automatisch
batch-value-background-subjectCenter-detect-alpha = Transparenz
batch-value-background-subjectCenter-detect-color = Abweichung von der Eckfarbe
batch-value-background-subjectCenter-fill-auto = Automatisch (transparent oder Eckfarbe)
batch-value-background-subjectCenter-fill-transparent = Transparent
batch-value-background-subjectCenter-fill-color = Farbe
batch-warn-subject-none = Kein Motiv gefunden, das Bild bleibt unverändert.

## Alpha bereinigen
batch-step-background-alphaCleanup = Alpha bereinigen
    .description = Entfernt schwache Transparenz, kleine Flecken und farbige Säume an den Kanten.
batch-param-background-alphaCleanup-low = Voll transparent unter
batch-param-background-alphaCleanup-high = Voll deckend über
batch-param-background-alphaCleanup-minIsland = Flecken entfernen kleiner als
batch-param-background-alphaCleanup-defringe = Saum entfernen
batch-err-alpha-range = „Voll deckend über“ muss größer sein als „Voll transparent unter“.

## Ecken abrunden
batch-step-overlay-roundCorners = Ecken abrunden
    .description = Rundet die Ecken jedes Bildes ab; außerhalb der Ecken wird es transparent.
batch-param-overlay-roundCorners-radius = Radius
batch-param-overlay-roundCorners-unit = Einheit
batch-value-overlay-roundCorners-unit-px = Pixel
batch-value-overlay-roundCorners-unit-percent = % der kürzeren Seite

## Vignette
batch-step-overlay-vignette = Vignette
    .description = Dunkelt die Ränder jedes Bildes zu den Ecken hin ab.
batch-param-overlay-vignette-strength = Stärke
batch-param-overlay-vignette-size = Unveränderte Mitte
batch-param-overlay-vignette-color = Farbe

## Tontrennung / Schwellenwert / Verpixeln
batch-step-adjust-posterize = Tontrennung
    .description = Reduziert jeden Farbkanal auf wenige Tonstufen.
batch-param-adjust-posterize-levels = Stufen
batch-step-adjust-threshold = Schwellenwert
    .description = Macht jedes Pixel je nach Helligkeit schwarz oder weiß.
batch-param-adjust-threshold-threshold = Schwellenwert
batch-step-filter-pixelate = Verpixeln
    .description = Macht aus dem Bild ein Mosaik aus quadratischen Blöcken.
batch-param-filter-pixelate-size = Blockgröße

## Auf Vielfaches auffüllen
batch-step-canvas-pad = Auf Vielfaches auffüllen
    .description = Vergrößert die Leinwand, bis Breite und Höhe Vielfache einer Zahl sind.
batch-param-canvas-pad-multiple = Vielfaches von
batch-param-canvas-pad-color = Farbe

## Standbild aus Animation
batch-step-anim-firstFrame = Standbild aus Animation
    .description = Behält ein einzelnes Bild einer Animation als Standbild.
batch-param-anim-firstFrame-pick = Bild
batch-param-anim-firstFrame-number = Bildnummer
batch-param-anim-firstFrame-time = Zeitpunkt
batch-value-anim-firstFrame-pick-first = Erstes
batch-value-anim-firstFrame-pick-middle = Mittleres
batch-value-anim-firstFrame-pick-last = Letztes
batch-value-anim-firstFrame-pick-number = Bildnummer
batch-value-anim-firstFrame-pick-time = Zu einem Zeitpunkt
batch-value-anim-firstFrame-pick-sharpest = Schärfstes

## Atlas des Sprite-Sheets (BA-105)
batch-param-anim-spriteSheet-atlas = Atlas-Datei
batch-value-anim-spriteSheet-atlas-json = JSON neben dem Sheet
batch-value-anim-spriteSheet-atlas-css = CSS neben dem Sheet
batch-value-anim-spriteSheet-atlas-none = Keine

## Probelauf mit Größe im Namen (BA-105)
batch-hint-size-after-processing = Der Name enthält die Bildgröße, die erst nach der Bearbeitung feststeht; angezeigt mit der Größe der Quelle.

# B-3: Dateischritte (crates\fp-batch\src\steps\fileops.rs)

## In Unterordner sortieren
batch-step-file-organize = In Unterordner sortieren
    .description = Legt die Dateien in Unterordner nach Datum, Dateityp, Größe, Ausrichtung, Kamera oder eigenem Muster.
batch-param-file-organize-by = Sortieren nach
batch-param-file-organize-pattern = Ordnermuster
    .tooltip = Zum Beispiel {"{"}date:YYYY{"}"}/{"{"}ext{"}"}. Platzhalter: date, modified, ext, format, size, orientation, camera, width, height.
batch-param-file-organize-action = Dateien
batch-value-file-organize-by-date = Aufnahmedatum (Jahr/Monat)
batch-value-file-organize-by-type = Dateityp
batch-value-file-organize-by-size = Größenklasse
batch-value-file-organize-by-orientation = Ausrichtung
batch-value-file-organize-by-camera = Kamera
batch-value-file-organize-by-pattern = Eigenes Muster
batch-value-file-organize-action-copy = Kopieren
batch-value-file-organize-action-move = Verschieben
batch-err-organize-empty = Bitte ein Ordnermuster eingeben.
batch-err-organize-token = Unbekannter Platzhalter: { $tokens }

## Nur einige verarbeiten
batch-step-file-filterSkip = Nur einige verarbeiten
    .description = Verarbeitet nur Dateien, die die Bedingungen erfüllen; die übrigen werden übersprungen oder unverändert kopiert.
batch-param-file-filterSkip-minWidth = Breite mindestens
batch-param-file-filterSkip-maxWidth = Breite höchstens
batch-param-file-filterSkip-minHeight = Höhe mindestens
batch-param-file-filterSkip-maxHeight = Höhe höchstens
batch-param-file-filterSkip-minKB = Dateigröße mindestens
batch-param-file-filterSkip-maxKB = Dateigröße höchstens
batch-param-file-filterSkip-transparency = Transparenz
batch-param-file-filterSkip-animation = Animation
batch-param-file-filterSkip-name = Dateiname
    .tooltip = Platzhalter * und ?, mehrere Muster mit ; trennen (zum Beispiel *.png; logo*).
batch-param-file-filterSkip-invert = Umkehren (die anderen verarbeiten)
batch-param-file-filterSkip-action = Übrige Dateien
batch-value-file-filterSkip-transparency-any = Egal
batch-value-file-filterSkip-transparency-with = Nur mit Transparenz
batch-value-file-filterSkip-transparency-without = Nur ohne Transparenz
batch-value-file-filterSkip-animation-any = Egal
batch-value-file-filterSkip-animation-animated = Nur Animationen
batch-value-file-filterSkip-animation-still = Nur Einzelbilder
batch-value-file-filterSkip-action-skip = Überspringen
batch-value-file-filterSkip-action-copy = Unverändert kopieren
batch-skip-filter-pixels = Größe { $w } × { $h } px liegt außerhalb des Bereichs.
batch-skip-filter-bytes = Dateigröße { $kb } KB liegt außerhalb des Bereichs.
batch-skip-filter-opaque = Hat keine Transparenz.
batch-skip-filter-transparent = Hat Transparenz.
batch-skip-filter-still = Ist keine Animation.
batch-skip-filter-animated = Ist eine Animation.
batch-skip-filter-name = Dateiname passt nicht.
batch-skip-filter-inverted = Erfüllt die Bedingungen (umgekehrt).
batch-warn-filter-preview = Würde übersprungen: { $reason }
batch-warn-filter-copied = Unverändert kopiert.
batch-err-filter-range = Die Obergrenze darf nicht kleiner als die Untergrenze sein.
batch-err-filter-name = Ungültiges Dateinamenmuster: { $error }

## Doppelte Bilder erkennen
batch-step-file-dedupe = Doppelte Bilder erkennen
    .description = Findet gleiche oder sehr ähnliche Bilder; das erste bleibt, die übrigen werden übersprungen oder markiert.
batch-param-file-dedupe-method = Vergleichen
batch-param-file-dedupe-distance = Erlaubter Unterschied
    .tooltip = 0 = fast gleich, 16 = grob ähnlich.
batch-param-file-dedupe-action = Doppelte
batch-value-file-dedupe-method-exact = Genau gleiche Pixel
batch-value-file-dedupe-method-similar = Ähnlich (Bild-Fingerabdruck)
batch-value-file-dedupe-action-skip = Überspringen
batch-value-file-dedupe-action-mark = Nur im Bericht markieren
batch-skip-duplicate = Doppelt zu { $name }.

## Datum setzen
batch-step-file-timestamps = Datum setzen
    .description = Setzt das Änderungsdatum der geschriebenen Dateien.
batch-param-file-timestamps-mode = Datum aus
batch-param-file-timestamps-date = Datum
    .tooltip = JJJJ-MM-TT HH:MM
batch-value-file-timestamps-mode-source = Quelldatei
batch-value-file-timestamps-mode-exif = Aufnahmedatum (EXIF), sonst Quelldatei
batch-value-file-timestamps-mode-fixed = Festes Datum
batch-err-timestamp-date = Bitte das Datum als JJJJ-MM-TT HH:MM eingeben.

# B-3: Dokumentschritte (crates\fp-batch\src\steps\document.rs)

## Suchen und ersetzen
batch-step-document-findReplace = Suchen und ersetzen
    .description = Ersetzt Text, Schriften, Farben oder URLs in Dokumenten; das Dokument bleibt bearbeitbar.
batch-param-document-findReplace-whatToFind = Suchen nach
batch-param-document-findReplace-find = Suchen
batch-param-document-findReplace-replace = Ersetzen durch
batch-param-document-findReplace-matchCase = Groß-/Kleinschreibung beachten
batch-param-document-findReplace-wholeWord = Ganzes Wort
batch-param-document-findReplace-regex = Regulärer Ausdruck
batch-value-document-findReplace-whatToFind-text = Text
batch-value-document-findReplace-whatToFind-font = Schrift
batch-value-document-findReplace-whatToFind-color = Farbe
batch-value-document-findReplace-whatToFind-url = URL
batch-value-document-findReplace-whatToFind-nonWeb216 = Nicht-Web216-Farben
batch-err-find-empty = Bitte den Suchbegriff eingeben.
batch-err-find-color = Bitte Farben als #rrggbb eingeben.
batch-err-find-regex = Ungültiges Suchmuster: { $error }
batch-warn-find-none = Nichts zum Ersetzen gefunden.

## Ebenen ein-/ausblenden
batch-step-document-layers = Ebenen ein-/ausblenden
    .description = Blendet die Ebenen mit passendem Namen ein oder aus, bevor das Dokument gespeichert oder exportiert wird.
batch-param-document-layers-name = Ebenenname
    .tooltip = Platzhalter * und ?, mehrere Namen mit ; trennen (zum Beispiel Hintergrund; Text*).
batch-param-document-layers-regex = Regulärer Ausdruck
batch-param-document-layers-mode = Passende Ebenen
batch-value-document-layers-mode-only = Nur diese einblenden
batch-value-document-layers-mode-show = Einblenden
batch-value-document-layers-mode-hide = Ausblenden
batch-err-layers-name = Bitte einen Ebenennamen eingeben.
batch-warn-layers-none = Kein Ebenenname passt.

## Seiten/Status wählen
batch-step-document-pages = Seiten/Status wählen
    .description = Speichert die gewählten Seiten (und Status) jedes Dokuments als eigene Bilder.
batch-param-document-pages-pages = Seiten
    .tooltip = Leer = alle Seiten, sonst zum Beispiel 1-3, 5.
batch-param-document-pages-states = Status
batch-param-document-pages-pattern = Dateiname
    .tooltip = Platzhalter: {"{"}name{"}"}, {"{"}page{"}"}, {"{"}pageName{"}"}, {"{"}state{"}"}.
batch-value-document-pages-states-first = Erster Status
batch-value-document-pages-states-all = Alle Status
batch-err-pages-none = Keine Seite zum Exportieren.
batch-skip-not-document = Kein Dokument mit Seiten.

## Slices exportieren
batch-step-document-slices = Slices exportieren
    .description = Speichert jeden Slice der Dokumente als eigenes Bild (ohne HTML).
batch-param-document-slices-pattern = Dateiname
    .tooltip = Platzhalter: {"{"}name{"}"}, {"{"}slice{"}"} (Slice-Name oder Nummer), {"{"}n{"}"}.
batch-skip-no-slices = Das Dokument hat keine Slices.

# B-3: Geometrie-, Farb- und Optimierungsschritte (crates\fp-batch\src\steps\pixel.rs)

## Perspektive
batch-step-perspective = Perspektive
    .description = Richtet einen verzerrten Bereich gerade: die vier Ecken werden zu den Ecken des neuen Bildes.
batch-param-perspective-tlX = Oben links X
batch-param-perspective-tlY = Oben links Y
batch-param-perspective-trX = Oben rechts X
batch-param-perspective-trY = Oben rechts Y
batch-param-perspective-brX = Unten rechts X
batch-param-perspective-brY = Unten rechts Y
batch-param-perspective-blX = Unten links X
batch-param-perspective-blY = Unten links Y
batch-param-perspective-size = Ergebnisgröße
batch-value-perspective-size-auto = Aus den Ecken
batch-value-perspective-size-keep = Bildgröße beibehalten

## In Kacheln teilen
batch-step-tile-split = In Kacheln teilen
    .description = Schneidet jedes Bild in ein Raster aus Kacheln, jede wird als eigene Datei gespeichert.
batch-param-tile-split-mode = Teilen nach
batch-param-tile-split-columns = Spalten
batch-param-tile-split-rows = Zeilen
batch-param-tile-split-tileWidth = Kachelbreite
batch-param-tile-split-tileHeight = Kachelhöhe
batch-param-tile-split-pattern = Dateiname
    .tooltip = Platzhalter: {"{"}name{"}"}, {"{"}row{"}"}, {"{"}col{"}"}.
batch-value-tile-split-mode-grid = Anzahl Kacheln
batch-value-tile-split-mode-size = Kachelgröße

## Rauschen
batch-step-filter-noise = Rauschen hinzufügen/entfernen
    .description = Fügt Filmkorn hinzu oder entfernt Sprenkel mit einem Medianfilter.
batch-param-filter-noise-mode = Rauschen
batch-param-filter-noise-amount = Stärke
batch-param-filter-noise-distribution = Verteilung
batch-param-filter-noise-monochrome = Einfarbig
batch-param-filter-noise-seed = Startwert
batch-param-filter-noise-radius = Radius
batch-value-filter-noise-mode-add = Hinzufügen
batch-value-filter-noise-mode-remove = Entfernen (Median)
batch-value-filter-noise-distribution-uniform = Gleichmäßig
batch-value-filter-noise-distribution-gaussian = Gaußsch

## Farb-LUT
batch-step-adjust-lut = Farb-LUT (.cube)
    .description = Wendet eine Farbtabelle aus einer .cube-Datei an.
batch-param-adjust-lut-file = LUT-Datei
batch-param-adjust-lut-strength = Stärke
batch-err-lut-file = Bitte eine .cube-Datei wählen.
batch-err-lut-line = Eine Zeile der LUT-Datei enthält nicht drei Zahlen.
batch-err-lut-size = Die LUT-Datei hat keine gültige LUT_3D_SIZE (2 bis 256).
batch-err-lut-1d = Nur 3D-LUT-Dateien werden unterstützt.
batch-err-lut-count = Die Zahl der LUT-Einträge passt nicht zu LUT_3D_SIZE.

## Stil anwenden
batch-step-effect-style = Stil anwenden
    .description = Wendet einen gespeicherten Stil (Füllung, Strich, Effekte) nach Namen auf jedes Bild oder Dokument an.
batch-param-effect-style-name = Stilname
batch-param-effect-style-library = Stilbibliothek
    .tooltip = Optional: Stilbibliothek (.json) oder FW-PNG mit Stilen; sonst die Stile des Dokuments.
batch-err-style-name = Bitte den Namen eines Stils eingeben.
batch-err-style-missing = Stil „{ $name }“ nicht gefunden.

## JPEG optimieren
batch-step-optimize-jpeg = JPEG optimieren
    .description = Kodiert JPEG-Dateien mit der gewählten Qualität neu, auf Wunsch bis zu einer Zielgröße.
batch-param-optimize-jpeg-quality = Qualität
batch-param-optimize-jpeg-maxKB = Zielgröße (0 = aus)
batch-param-optimize-jpeg-minQuality = Niedrigste Qualität
batch-param-optimize-jpeg-progressive = Progressiv
batch-param-optimize-jpeg-subsampling = Farbunterabtastung
batch-value-optimize-jpeg-subsampling-auto = Automatisch
batch-value-optimize-jpeg-subsampling-444 = 4:4:4 (beste)
batch-value-optimize-jpeg-subsampling-422 = 4:2:2
batch-value-optimize-jpeg-subsampling-420 = 4:2:0 (kleinste)
batch-warn-jpeg-only = JPEG-Optimierung wirkt nur mit „Speichern als JPEG“ (gewählt: { $format }).

## Websichere Farben
batch-step-optimize-webSafe = Websichere Farben
    .description = Rundet jede Farbe auf die 216 websicheren Farben.
batch-param-optimize-webSafe-dither = Dithering
batch-value-optimize-webSafe-dither-none = Keins
batch-value-optimize-webSafe-dither-ordered = Geordnet

# B-3: Animationsschritte (crates\fp-batch\src\steps\animfx.rs)

## Bumerang
batch-step-anim-boomerang = Bumerang
    .description = Spielt die Animation vorwärts und dann rückwärts, ohne die Wendebilder doppelt zu zeigen.
batch-param-anim-boomerang-hold = Pause an beiden Enden

## Überblendete Schleife
batch-step-anim-crossfade = Überblendete Schleife
    .description = Blendet das Ende der Animation in ihren Anfang über, damit die Schleife ohne sichtbaren Sprung läuft.
batch-param-anim-crossfade-frames = Überblendbilder
batch-param-anim-crossfade-mode = Überblenden
batch-param-anim-crossfade-delay = Dauer je Überblendbild
    .tooltip = 0 = mittlere Bilddauer.
batch-value-anim-crossfade-mode-overlap = Überlappend (kürzere Animation)
batch-value-anim-crossfade-mode-append = Bilder anhängen
batch-err-crossfade-short = Die Animation hat { $frames } Bilder; diese Überblendung braucht mindestens { $need }.
batch-err-frame-size = Die Bilder der Animation sind unterschiedlich groß.
