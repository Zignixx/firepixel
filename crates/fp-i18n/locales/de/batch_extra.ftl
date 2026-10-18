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
