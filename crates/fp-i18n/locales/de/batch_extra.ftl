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
