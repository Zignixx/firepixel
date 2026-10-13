# Batch-Studio, Paket BATCH-6 (docs\BATCH_MODERN.md §3.5/§3.6): Gruppen „Hintergrund“ und
# „Überlagerung“ – crates\fp-batch\src\steps\background.rs und overlay.rs.
# Schritte batch-step-<id>, Parameter batch-param-<id>-<param>, Auswahlwerte
# batch-choice-<id>-<param>-<wert> (Punkt der ID als -), eigene Meldungen batch-bg-*/batch-overlay-*.

## Hintergrund entfernen (KI)
batch-step-background-remove = Hintergrund entfernen
    .description = Stellt das Hauptmotiv mit KI frei und macht den Hintergrund transparent.
batch-param-background-remove-model = KI-Modell
batch-param-background-remove-edge = Kante
batch-param-background-remove-feather = Weiche Kante
batch-param-background-remove-tolerance = Toleranz
batch-param-background-remove-expand = Ausdehnen/Zusammenziehen
batch-param-background-remove-smooth = Glätten
batch-param-background-remove-refineEdge = Kante verfeinern (Haare)
batch-param-background-remove-decontaminate = Kantenfarben entmischen
batch-param-background-remove-output = Ergebnis
batch-choice-background-remove-model-birefnetLite = Schnell (BiRefNet lite)
batch-choice-background-remove-model-birefnet = Genau (BiRefNet)
batch-choice-background-remove-edge-hard = Hart
batch-choice-background-remove-edge-antiAlias = Geglättet
batch-choice-background-remove-edge-feather = Weiche Kante
batch-choice-background-remove-output-transparent = Hintergrund transparent
batch-choice-background-remove-output-mask = Nur Maske (schwarz/weiß)
batch-choice-background-remove-output-alpha = Maske als Transparenz, Farben unverändert

## Farbe entfernen
batch-step-background-removeColor = Farbe entfernen
    .description = Entfernt eine einfarbige Fläche (z. B. weißen Hintergrund) ohne KI.
batch-param-background-removeColor-pick = Farbe von
batch-param-background-removeColor-color = Farbe
batch-param-background-removeColor-tolerance = Toleranz
batch-param-background-removeColor-contiguous = Nur von den Rändern aus
batch-param-background-removeColor-feather = Weiche Kante
batch-param-background-removeColor-decontaminate = Kantenfarben entmischen
batch-choice-background-removeColor-pick-topLeft = Ecke oben links
batch-choice-background-removeColor-pick-topRight = Ecke oben rechts
batch-choice-background-removeColor-pick-bottomLeft = Ecke unten links
batch-choice-background-removeColor-pick-bottomRight = Ecke unten rechts
batch-choice-background-removeColor-pick-custom = Eigene Farbe

## Transparenz → Farbe
batch-step-background-flatten = Transparenz füllen
    .description = Füllt transparente Bereiche mit einer Farbe, einem Verlauf, einem Bild oder einer unscharfen Kopie.
batch-param-background-flatten-kind = Füllung
batch-param-background-flatten-color = Farbe
batch-param-background-flatten-color2 = Zweite Farbe
batch-param-background-flatten-gradientShape = Verlaufsform
batch-param-background-flatten-angle = Winkel
batch-param-background-flatten-image = Bild
batch-param-background-flatten-imageData = Eingebettetes Bild
batch-param-background-flatten-fit = Einpassen
batch-param-background-flatten-blur = Unschärfe
batch-param-background-flatten-onlyIfTransparent = Nur Bilder mit Transparenz
batch-param-background-flatten-edge = Kanten
batch-param-background-flatten-matte = Alte Hintergrundfarbe
batch-param-background-flatten-threshold = Schwelle
batch-param-background-flatten-linearLight = In linearem Licht mischen
batch-choice-background-flatten-kind-color = Farbe
batch-choice-background-flatten-kind-gradient = Verlauf
batch-choice-background-flatten-kind-image = Bild
batch-choice-background-flatten-kind-blur = Unscharfe Kopie
batch-choice-background-flatten-gradientShape-linear = Linear
batch-choice-background-flatten-gradientShape-radial = Kreisförmig
batch-choice-background-flatten-fit-cover = Füllen
batch-choice-background-flatten-fit-contain = Einpassen
batch-choice-background-flatten-fit-stretch = Strecken
batch-choice-background-flatten-fit-tile = Kacheln
batch-choice-background-flatten-edge-blend = Normal mischen
batch-choice-background-flatten-edge-unmix = Farbsaum entmischen
batch-choice-background-flatten-edge-hard = Harte Kante

## Hintergrund ersetzen
batch-step-background-replace = Hintergrund ersetzen
    .description = Entfernt den Hintergrund (KI oder Farbe) und setzt eine neue Fläche dahinter.
batch-param-background-replace-method = Freistellen mit
batch-param-background-replace-model = KI-Modell
batch-param-background-replace-edge = Kante
batch-param-background-replace-feather = Weiche Kante
batch-param-background-replace-tolerance = Toleranz
batch-param-background-replace-expand = Ausdehnen/Zusammenziehen
batch-param-background-replace-smooth = Glätten
batch-param-background-replace-refineEdge = Kante verfeinern (Haare)
batch-param-background-replace-decontaminate = Kantenfarben entmischen
batch-param-background-replace-pick = Farbe von
batch-param-background-replace-keyColor = Zu entfernende Farbe
batch-param-background-replace-keyTolerance = Farbtoleranz
batch-param-background-replace-contiguous = Nur von den Rändern aus
batch-param-background-replace-kind = Neuer Hintergrund
batch-param-background-replace-color = Farbe
batch-param-background-replace-color2 = Zweite Farbe
batch-param-background-replace-gradientShape = Verlaufsform
batch-param-background-replace-angle = Winkel
batch-param-background-replace-image = Bild
batch-param-background-replace-imageData = Eingebettetes Bild
batch-param-background-replace-fit = Einpassen
batch-param-background-replace-blur = Unschärfe
batch-param-background-replace-linearLight = In linearem Licht mischen
batch-choice-background-replace-method-ai = KI (Motiv erkennen)
batch-choice-background-replace-method-color = Farbe
batch-choice-background-replace-model-birefnetLite = Schnell (BiRefNet lite)
batch-choice-background-replace-model-birefnet = Genau (BiRefNet)
batch-choice-background-replace-edge-hard = Hart
batch-choice-background-replace-edge-antiAlias = Geglättet
batch-choice-background-replace-edge-feather = Weiche Kante
batch-choice-background-replace-pick-topLeft = Ecke oben links
batch-choice-background-replace-pick-topRight = Ecke oben rechts
batch-choice-background-replace-pick-bottomLeft = Ecke unten links
batch-choice-background-replace-pick-bottomRight = Ecke unten rechts
batch-choice-background-replace-pick-custom = Eigene Farbe
batch-choice-background-replace-kind-color = Farbe
batch-choice-background-replace-kind-gradient = Verlauf
batch-choice-background-replace-kind-image = Bild
batch-choice-background-replace-kind-blur = Unscharfe Kopie
batch-choice-background-replace-gradientShape-linear = Linear
batch-choice-background-replace-gradientShape-radial = Kreisförmig
batch-choice-background-replace-fit-cover = Füllen
batch-choice-background-replace-fit-contain = Einpassen
batch-choice-background-replace-fit-stretch = Strecken
batch-choice-background-replace-fit-tile = Kacheln

## Schatten
batch-step-background-shadow = Schatten hinzufügen
    .description = Legt einen weichen Schatten unter das freigestellte Motiv.
batch-param-background-shadow-style = Art
batch-param-background-shadow-color = Farbe
batch-param-background-shadow-opacity = Deckkraft
batch-param-background-shadow-distance = Abstand
batch-param-background-shadow-angle = Winkel
batch-param-background-shadow-blur = Weichheit
batch-param-background-shadow-expandCanvas = Leinwand erweitern
batch-choice-background-shadow-style-drop = Schlagschatten
batch-choice-background-shadow-style-below = Bodenschatten

## Text/Wasserzeichen
batch-step-overlay-text = Text/Wasserzeichen
    .description = Schreibt Text auf jedes Bild, z. B. Copyright, Dateiname oder Datum.
batch-param-overlay-text-text = Text
    .tooltip = Platzhalter: {"{"}name{"}"}, {"{"}date{"}"}, {"{"}exif-date{"}"}, {"{"}year{"}"}, {"{"}n{"}"}, {"{"}n:3{"}"}, {"{"}w{"}"}, {"{"}h{"}"}; (c) wird zu ©.
batch-param-overlay-text-size = Größe
batch-param-overlay-text-font = Schrift
batch-param-overlay-text-bold = Fett
batch-param-overlay-text-italic = Kursiv
batch-param-overlay-text-color = Farbe
batch-param-overlay-text-opacity = Deckkraft
batch-param-overlay-text-shadow = Schatten
batch-param-overlay-text-outline = Kontur
batch-param-overlay-text-outlineColor = Konturfarbe
batch-param-overlay-text-outlineWidth = Konturbreite
batch-param-overlay-text-position = Lage
batch-param-overlay-text-x = Position waagerecht
batch-param-overlay-text-y = Position senkrecht
batch-param-overlay-text-margin = Abstand zum Rand
batch-param-overlay-text-rotation = Drehung
batch-param-overlay-text-spacing = Kachelabstand
batch-param-overlay-text-blend = Mischmodus

## Logo/Bild
batch-step-overlay-image = Logo/Bild einfügen
    .description = Setzt ein Logo oder Bild als Wasserzeichen auf jedes Bild.
batch-param-overlay-image-image = Bild
batch-param-overlay-image-imageData = Eingebettetes Bild
batch-param-overlay-image-scale = Größe
batch-param-overlay-image-opacity = Deckkraft
batch-param-overlay-image-position = Lage
batch-param-overlay-image-x = Position waagerecht
batch-param-overlay-image-y = Position senkrecht
batch-param-overlay-image-margin = Abstand zum Rand
batch-param-overlay-image-rotation = Drehung
batch-param-overlay-image-spacing = Kachelabstand
batch-param-overlay-image-blend = Mischmodus

## Lagen und Mischmodi (je Schritt Text und Bild)
batch-choice-overlay-text-position-topLeft = Oben links
batch-choice-overlay-image-position-topLeft = Oben links
batch-choice-overlay-text-position-top = Oben Mitte
batch-choice-overlay-image-position-top = Oben Mitte
batch-choice-overlay-text-position-topRight = Oben rechts
batch-choice-overlay-image-position-topRight = Oben rechts
batch-choice-overlay-text-position-left = Mitte links
batch-choice-overlay-image-position-left = Mitte links
batch-choice-overlay-text-position-center = Mitte
batch-choice-overlay-image-position-center = Mitte
batch-choice-overlay-text-position-right = Mitte rechts
batch-choice-overlay-image-position-right = Mitte rechts
batch-choice-overlay-text-position-bottomLeft = Unten links
batch-choice-overlay-image-position-bottomLeft = Unten links
batch-choice-overlay-text-position-bottom = Unten Mitte
batch-choice-overlay-image-position-bottom = Unten Mitte
batch-choice-overlay-text-position-bottomRight = Unten rechts
batch-choice-overlay-image-position-bottomRight = Unten rechts
batch-choice-overlay-text-position-tiled = Gekachelt
batch-choice-overlay-image-position-tiled = Gekachelt
batch-choice-overlay-text-position-free = Frei
batch-choice-overlay-image-position-free = Frei
batch-choice-overlay-text-blend-normal = Normal
batch-choice-overlay-image-blend-normal = Normal
batch-choice-overlay-text-blend-multiply = Multiplizieren
batch-choice-overlay-image-blend-multiply = Multiplizieren
batch-choice-overlay-text-blend-screen = Negativ multiplizieren
batch-choice-overlay-image-blend-screen = Negativ multiplizieren
batch-choice-overlay-text-blend-overlay = Ineinanderkopieren
batch-choice-overlay-image-blend-overlay = Ineinanderkopieren
batch-choice-overlay-text-blend-softLight = Weiches Licht
batch-choice-overlay-image-blend-softLight = Weiches Licht
batch-choice-overlay-text-blend-hardLight = Hartes Licht
batch-choice-overlay-image-blend-hardLight = Hartes Licht
batch-choice-overlay-text-blend-darken = Abdunkeln
batch-choice-overlay-image-blend-darken = Abdunkeln
batch-choice-overlay-text-blend-lighten = Aufhellen
batch-choice-overlay-image-blend-lighten = Aufhellen
batch-choice-overlay-text-blend-difference = Differenz
batch-choice-overlay-image-blend-difference = Differenz
batch-choice-overlay-text-blend-exclusion = Ausschluss
batch-choice-overlay-image-blend-exclusion = Ausschluss

## Rahmen
batch-step-overlay-border = Rahmen
    .description = Zeichnet einen Rahmen um jedes Bild, auf Wunsch mit abgerundeten Ecken.
batch-param-overlay-border-width = Breite
batch-param-overlay-border-unit = Einheit
batch-param-overlay-border-color = Farbe
batch-param-overlay-border-radius = Eckenradius
batch-param-overlay-border-placement = Lage
batch-choice-overlay-border-unit-px = Pixel
batch-choice-overlay-border-unit-percent = % der kürzeren Seite
batch-choice-overlay-border-placement-outer = Außen (Bild wird größer)
batch-choice-overlay-border-placement-inner = Innen (Bildgröße bleibt)

## Zusammenfassungen
batch-bg-summary-output-transparent = Hintergrund transparent
batch-bg-summary-output-mask = Nur Maske
batch-bg-summary-output-alpha = Maske als Transparenz
batch-bg-summary-kind-color = Mit Farbe füllen
batch-bg-summary-kind-gradient = Mit Verlauf füllen
batch-bg-summary-kind-image = Mit Bild füllen
batch-bg-summary-kind-blur = Mit unscharfer Kopie füllen

## Meldungen
batch-bg-err-model-missing = KI-Modell fehlt: „{ $model }“. Bitte über „KI-Modelle verwalten…“ laden oder die Datei in den Modellordner legen: { $folder }
batch-bg-warn-matting-missing = „Kante verfeinern“ übersprungen: KI-Modell „{ $model }“ fehlt.
batch-bg-err-no-image = Bitte ein Bild wählen.
batch-bg-err-image-read = Das Bild „{ $path }“ ist nicht lesbar: { $error }
batch-bg-err-image-data = Das eingebettete Bild ist beschädigt.
batch-overlay-warn-no-font = Text nicht gesetzt: keine passende Schrift gefunden.
