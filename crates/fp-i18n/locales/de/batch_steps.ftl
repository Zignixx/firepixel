# Batch-Studio: Schritte der Gruppen Geometrie, Farbe und Effekte, Erweitert (Paket BATCH-4,
# docs\BATCH_MODERN.md §3.3, §3.4, §3.13; Code crates\fp-batch\src\steps\{geometry,adjust,script}.rs).
# Schritte batch-step-<id>, Parameter batch-param-<id>-<param>, Auswahlwerte
# batch-value-<id>-<param>-<wert>, Zusammenfassungen batch-summary-<id>-…

## Geometrie
batch-step-resize = Größe ändern
    .description = Verkleinert oder vergrößert die Bilder auf eine feste Größe, in Prozent oder passend in einen Rahmen.
batch-param-resize-mode = Art
batch-param-resize-size = Seitenlänge
batch-param-resize-width = Breite
batch-param-resize-height = Höhe
batch-param-resize-percent = Prozent
batch-param-resize-onlyShrink = Nur verkleinern
batch-param-resize-filter = Neuberechnung
batch-param-resize-dpi = Auflösung
batch-value-resize-mode-percent = In Prozent
batch-value-resize-mode-width = Breite festlegen
batch-value-resize-mode-height = Höhe festlegen
batch-value-resize-mode-longEdge = Längste Seite
batch-value-resize-mode-shortEdge = Kürzeste Seite
batch-value-resize-mode-fit = In Rahmen einpassen
batch-value-resize-mode-fill = Rahmen füllen (zuschneiden)
batch-value-resize-mode-exact = Genau (verzerrt)
batch-value-resize-mode-none = Nicht ändern
batch-value-resize-filter-auto = Automatisch
batch-value-resize-filter-nearest = Pixelwiederholung
batch-value-resize-filter-bilinear = Bilinear
batch-value-resize-filter-bicubic = Bikubisch
batch-value-resize-filter-lanczos3 = Lanczos (beste Qualität)
batch-summary-resize-percent = Auf { $percent } %
batch-summary-resize-width = Breite { $width } px
batch-summary-resize-height = Höhe { $height } px
batch-summary-resize-longEdge = Längste Seite { $size } px
batch-summary-resize-shortEdge = Kürzeste Seite { $size } px
batch-summary-resize-fit = In { $width } × { $height } px einpassen
batch-summary-resize-fill = { $width } × { $height } px füllen
batch-summary-resize-exact = Genau { $width } × { $height } px
batch-summary-resize-none = Größe bleibt

batch-step-crop = Zuschneiden
    .description = Schneidet auf ein Seitenverhältnis, einen Bereich oder um feste Ränder zu.
batch-param-crop-mode = Art
batch-param-crop-ratio = Seitenverhältnis
batch-param-crop-ratioWidth = Verhältnis Breite
batch-param-crop-ratioHeight = Verhältnis Höhe
batch-param-crop-anchor = Ausrichtung
batch-param-crop-unit = Einheit
batch-param-crop-x = Links
batch-param-crop-y = Oben
batch-param-crop-width = Breite
batch-param-crop-height = Höhe
batch-param-crop-top = Oben
batch-param-crop-right = Rechts
batch-param-crop-bottom = Unten
batch-param-crop-left = Links
batch-value-crop-mode-aspect = Seitenverhältnis
batch-value-crop-mode-rect = Bereich
batch-value-crop-mode-edges = Ränder entfernen
batch-value-crop-ratio-free = Frei
batch-value-crop-unit-px = Pixel
batch-value-crop-unit-percent = Prozent
batch-summary-crop-aspect = Zuschneiden auf { $ratio }
batch-summary-crop-rect = Bereich ausschneiden
batch-summary-crop-edges = Ränder abschneiden
batch-warn-crop-empty = Der Zuschnitt liegt außerhalb des Bildes; das Bild bleibt unverändert.

batch-step-trim = Ränder abschneiden
    .description = Entfernt leere Ränder: transparente Bereiche oder die Farbe der Ecke oben links.
batch-param-trim-what = Was entfernen
batch-param-trim-tolerance = Toleranz
batch-param-trim-padding = Abstand lassen
batch-value-trim-what-transparent = Transparente Ränder
batch-value-trim-what-color = Farbe der Ecke
batch-value-trim-what-both = Beides
batch-summary-trim-transparent = Transparente Ränder entfernen
batch-summary-trim-color = Einfarbige Ränder entfernen
batch-summary-trim-both = Transparente und einfarbige Ränder entfernen
batch-warn-trim-empty = Das Bild hat keinen Inhalt außer Rand; es bleibt unverändert.

batch-step-rotate = Drehen/Spiegeln
    .description = Dreht oder spiegelt die Bilder; richtet Fotos auf Wunsch nach der Kamera-Ausrichtung (EXIF) auf.
batch-param-rotate-angle = Winkel
batch-param-rotate-flip = Spiegeln
batch-param-rotate-autoExif = Nach Kamera-Ausrichtung aufrichten
batch-param-rotate-background = Hintergrund
batch-param-rotate-filter = Neuberechnung
batch-value-rotate-flip-none = Nicht spiegeln
batch-value-rotate-flip-horizontal = Waagerecht
batch-value-rotate-flip-vertical = Senkrecht
batch-value-rotate-flip-both = Beides
batch-value-rotate-filter-bicubic = Bikubisch
batch-value-rotate-filter-bilinear = Bilinear
batch-value-rotate-filter-nearest = Pixelwiederholung
batch-summary-rotate = Um { $angle }° drehen
batch-summary-rotate-flip = Um { $angle }° drehen und spiegeln

batch-step-canvas = Leinwand/Rand
    .description = Fügt einen Rand hinzu oder bringt die Leinwand auf eine Größe, ein Seitenverhältnis oder ein Quadrat.
batch-param-canvas-mode = Art
batch-param-canvas-border = Rand
batch-param-canvas-top = Oben
batch-param-canvas-right = Rechts
batch-param-canvas-bottom = Unten
batch-param-canvas-left = Links
batch-param-canvas-width = Breite
batch-param-canvas-height = Höhe
batch-param-canvas-ratio = Seitenverhältnis
batch-param-canvas-ratioWidth = Verhältnis Breite
batch-param-canvas-ratioHeight = Verhältnis Höhe
batch-param-canvas-color = Farbe
batch-param-canvas-anchor = Bild ausrichten
batch-value-canvas-mode-addBorder = Rand hinzufügen
batch-value-canvas-mode-toSize = Auf Größe
batch-value-canvas-mode-toAspect = Auf Seitenverhältnis
batch-value-canvas-mode-square = Quadratisch
batch-value-canvas-ratio-free = Frei
batch-summary-canvas-addBorder = Rand { $border } px
batch-summary-canvas-toSize = Leinwand { $width } × { $height } px
batch-summary-canvas-toAspect = Leinwand auf Seitenverhältnis
batch-summary-canvas-square = Quadratische Leinwand

## Ausrichtung (Anker) für Zuschneiden und Leinwand
batch-value-crop-anchor-topLeft = Oben links
batch-value-crop-anchor-top = Oben
batch-value-crop-anchor-topRight = Oben rechts
batch-value-crop-anchor-left = Links
batch-value-crop-anchor-center = Mitte
batch-value-crop-anchor-right = Rechts
batch-value-crop-anchor-bottomLeft = Unten links
batch-value-crop-anchor-bottom = Unten
batch-value-crop-anchor-bottomRight = Unten rechts
batch-value-canvas-anchor-topLeft = Oben links
batch-value-canvas-anchor-top = Oben
batch-value-canvas-anchor-topRight = Oben rechts
batch-value-canvas-anchor-left = Links
batch-value-canvas-anchor-center = Mitte
batch-value-canvas-anchor-right = Rechts
batch-value-canvas-anchor-bottomLeft = Unten links
batch-value-canvas-anchor-bottom = Unten
batch-value-canvas-anchor-bottomRight = Unten rechts

batch-err-image-too-large = Das Bild wäre mit { $width } × { $height } Pixeln zu groß.

## Farbe und Effekte
batch-step-adjust-auto = Automatisch verbessern
    .description = Korrigiert Tonwerte, Kontrast und auf Wunsch den Weißabgleich automatisch.
batch-param-adjust-auto-levels = Tonwerte
batch-param-adjust-auto-contrast = Kontrast
batch-param-adjust-auto-whiteBalance = Weißabgleich

batch-step-adjust-basic = Helligkeit/Kontrast/Sättigung
    .description = Helligkeit, Kontrast, Sättigung, Farbton, Gamma und Farbtemperatur anpassen.
batch-param-adjust-basic-brightness = Helligkeit
batch-param-adjust-basic-contrast = Kontrast
batch-param-adjust-basic-saturation = Sättigung
batch-param-adjust-basic-hue = Farbton
batch-param-adjust-basic-gamma = Gamma
batch-param-adjust-basic-temperature = Farbtemperatur
batch-param-adjust-basic-tint = Tönung
batch-summary-adjust-basic = Helligkeit { $brightness }, Kontrast { $contrast }, Sättigung { $saturation }

batch-step-adjust-levels = Tonwertkorrektur
    .description = Schwarzpunkt, Gamma und Weißpunkt wie im Dialog „Tonwertkorrektur“.
batch-param-adjust-levels-inputLow = Tiefen
batch-param-adjust-levels-gamma = Mitteltöne
batch-param-adjust-levels-inputHigh = Lichter
batch-param-adjust-levels-outputLow = Ausgabe Tiefen
batch-param-adjust-levels-outputHigh = Ausgabe Lichter
batch-param-adjust-levels-channels = Alle Kanäle (JSON)

batch-step-adjust-curves = Gradationskurven
    .description = Ändert die Tonwerte über Kurvenpunkte wie im Dialog „Gradationskurven“.
batch-param-adjust-curves-points = Kurvenpunkte
batch-param-adjust-curves-channels = Alle Kanäle (JSON)
batch-issue-curve-points = Kurvenpunkte als Liste [[x, y], …] mit mindestens zwei Punkten angeben.
batch-issue-json-invalid = Ungültige Angabe: { $error }

batch-step-adjust-grayscale = Schwarzweiß/Sepia/Duoton
    .description = Wandelt die Bilder in Graustufen, Sepia oder zwei Farbtöne um.
batch-param-adjust-grayscale-style = Stil
batch-param-adjust-grayscale-dark = Dunkle Farbe
batch-param-adjust-grayscale-light = Helle Farbe
batch-value-adjust-grayscale-style-gray = Schwarzweiß
batch-value-adjust-grayscale-style-sepia = Sepia
batch-value-adjust-grayscale-style-duotone = Duoton

batch-step-adjust-invert = Umkehren
    .description = Kehrt die Farben um (Negativ).

batch-step-filter-sharpen = Schärfen
    .description = Schärft die Bilder mit „Unscharf maskieren“.
batch-param-filter-sharpen-amount = Stärke
batch-param-filter-sharpen-radius = Radius
batch-param-filter-sharpen-threshold = Schwellenwert
batch-summary-filter-sharpen = Schärfen { $amount } %

batch-step-filter-blur = Weichzeichnen
    .description = Zeichnet die Bilder weich: Gauß, Bewegung, radial oder Zoom.
batch-param-filter-blur-kind = Art
batch-param-filter-blur-radius = Radius
batch-param-filter-blur-angle = Winkel
batch-param-filter-blur-amount = Stärke
batch-param-filter-blur-quality = Qualität
batch-value-filter-blur-kind-gaussian = Gaußscher Weichzeichner
batch-value-filter-blur-kind-motion = Bewegungsunschärfe
batch-value-filter-blur-kind-radial = Radialer Weichzeichner
batch-value-filter-blur-kind-zoom = Zoom-Weichzeichner
batch-summary-filter-blur = { $kind }, { $radius } px

batch-step-filter-registry = Beliebiger Filter
    .description = Wendet einen Filter aus dem Menü „Filter“ mit seinen Einstellungen an.
batch-param-filter-registry-command = Filter
batch-param-filter-registry-params = Einstellungen (JSON)

batch-step-effect-live = Live-Effekt
    .description = Legt einen Firepixel-Effekt wie Schlagschatten oder Glühen auf das Bild und rechnet ihn ein.
batch-param-effect-live-effect = Effekt
batch-param-effect-live-params = Einstellungen (JSON)
batch-param-effect-live-expandCanvas = Leinwand erweitern
batch-value-effect-live-effect-dropShadow = Schlagschatten
batch-value-effect-live-effect-innerShadow = Innerer Schatten
batch-value-effect-live-effect-glow = Glühen
batch-value-effect-live-effect-innerGlow = Inneres Glühen
batch-value-effect-live-effect-bevel = Abschrägung innen
batch-value-effect-live-effect-outerBevel = Abschrägung außen
batch-value-effect-live-effect-emboss = Erhabene Prägung
batch-value-effect-live-effect-insetEmboss = Eingelassene Prägung
batch-value-effect-live-effect-solidShadow = Einfarbiger Schatten
batch-value-effect-live-effect-colorFill = Farbfüllung
batch-warn-effect-unsupported = Der Effekt „{ $effect }“ konnte nicht vollständig berechnet werden.

## Erweitert
batch-step-script-registry = Firepixel-Befehl
    .description = Führt einen oder mehrere Befehle von Firepixel mit ihren Parametern aus.
batch-param-script-registry-command = Befehl
batch-param-script-registry-params = Parameter (JSON)
batch-param-script-registry-commands = Weitere Befehle (JSON)
batch-param-script-registry-selectAll = Vorher alles auswählen
batch-summary-script-registry-many = { $count } Befehle

batch-step-script-command = Gespeicherter Befehl
    .description = Führt einen gespeicherten Befehl (Skript) aus dem Befehlsordner aus.
batch-param-script-command-name = Befehl
batch-param-script-command-path = Skriptdatei

batch-issue-script-no-command = Bitte einen Befehl angeben.
batch-issue-script-blocked = Der Befehl „{ $command }“ ist in der Batch-Verarbeitung nicht erlaubt.
batch-issue-script-unknown = Unbekannter Befehl „{ $command }“.
batch-issue-script-no-file = Bitte einen gespeicherten Befehl oder eine Skriptdatei wählen.
batch-issue-script-missing = Die Skriptdatei fehlt: { $path }
batch-issue-script-not-for-batch = Dieser Befehl ist als „nicht für Batch“ markiert.
