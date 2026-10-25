# Werkzeuge und automatische Formen (Bereich I18N-MENUE)
#
# tool-<name>   Werkzeugnamen (Schlüssel aus dem stabilen Namen: tool.polygonLasso → tool-polygonLasso),
#               im Werkzeugbedienfeld, in Tooltips („Name (Kürzel)“), Flyouts und im Inspektor.
# tool-tip-…    weitere Tooltips des Werkzeugbedienfelds.
# tool-desc-…   Kurzbeschreibung eines Werkzeugs (zweite Zeile seines Tooltips).
# shape-…       automatische Formen (Schlüssel aus dem englischen Namen: „Rounded Rectangle“ → shape-rounded-rectangle).

## Werkzeuge

tool-pointer = Zeiger
tool-selectBehind = Hintere auswählen
tool-subselection = Teilauswahl
tool-maskElementSelect = Maskenelement-Auswahl
tool-marquee = Auswahlrahmen
tool-ovalMarquee = Ovaler Auswahlrahmen
tool-lasso = Lasso
tool-polygonLasso = Polygon-Lasso
tool-crop = Zuschneiden
tool-exportArea = Exportbereich
tool-magicWand = Zauberstab
tool-aiObject = Objektauswahl
tool-aiSubject = Motiv auswählen
tool-aiRefineEdge = Kante verfeinern
tool-line = Linie
tool-pen = Stift
tool-rectangle = Rechteck
tool-ellipse = Ellipse
tool-polygon = Polygon
tool-autoShape = Automatische Form
tool-text = Text
tool-pencil = Bleistift
tool-vectorPath = Vektorpfad
tool-redrawPath = Pfad nachzeichnen
tool-brush = Pinsel
tool-scale = Skalieren
tool-skew = Neigen
tool-distort = Verzerren
tool-nineSliceScale = 9-Slice-Skalierung
tool-freeform = Freiform
tool-reshapeArea = Bereich umformen
tool-pathScrubAdd = Pfadausbesserung - additiv
tool-pathScrubSubtract = Pfadausbesserung - subtraktiv
tool-eyedropper = Pipette
tool-paintBucket = Füllwerkzeug
tool-gradient = Verlauf
tool-eraser = Radiergummi
tool-blur = Weichzeichner
tool-sharpen = Scharfzeichner
tool-dodge = Abwedler
tool-burn = Nachbelichter
tool-smudge = Verwischen
tool-rubberStamp = Stempel
tool-replaceColor = Farbe ersetzen
tool-redEyeRemoval = Rote Augen entfernen
tool-knife = Messer
tool-hotspotRect = Hotspot-Rechteck
tool-hotspotCircle = Hotspot-Kreis
tool-hotspotPolygon = Hotspot-Polygon
tool-slice = Segment
tool-polygonSlice = Polygon-Segment
tool-hand = Hand
tool-zoom = Zoom

## Kurzbeschreibungen der Werkzeuge (zweite Zeile im Werkzeug-Tooltip, Firepixel-Erweiterung)

tool-desc-pointer = Wählt Objekte per Klick oder Rahmen aus und verschiebt sie.
tool-desc-selectBehind = Wählt bei jedem Klick das nächste darunterliegende Objekt aus.
tool-desc-subselection = Wählt Objekte in Gruppen, maskierte Inhalte und die Punkte eines Pfads aus.
tool-desc-maskElementSelect = Wählt die Maske eines maskierten Objekts aus, um sie einzeln zu bearbeiten.
tool-desc-marquee = Wählt einen rechteckigen Pixelbereich in einer Bitmap aus.
tool-desc-ovalMarquee = Wählt einen elliptischen Pixelbereich in einer Bitmap aus.
tool-desc-lasso = Wählt einen frei gezeichneten Pixelbereich aus.
tool-desc-polygonLasso = Wählt einen Pixelbereich mit geraden Kanten Punkt für Punkt aus.
tool-desc-crop = Schneidet das Dokument auf den aufgezogenen Bereich zu.
tool-desc-exportArea = Exportiert nur den aufgezogenen Bereich des Dokuments.
tool-desc-magicWand = Wählt zusammenhängende Pixel ähnlicher Farbe aus.
tool-desc-aiObject = Wählt mit einem KI-Modell das angeklickte Objekt aus.
tool-desc-aiSubject = Wählt mit einem KI-Modell das Hauptmotiv aus.
tool-desc-aiRefineEdge = Verbessert die Kante einer Auswahl, etwa an Haaren und Fell.
tool-desc-line = Zeichnet gerade Linien, mit Umschalt in 45°-Schritten.
tool-desc-pen = Zeichnet Vektorpfade Punkt für Punkt, auch mit Kurven.
tool-desc-rectangle = Zeichnet Rechtecke, mit Umschalt Quadrate.
tool-desc-ellipse = Zeichnet Ellipsen, mit Umschalt Kreise.
tool-desc-polygon = Zeichnet regelmäßige Vielecke und Sterne.
tool-desc-autoShape = Zeichnet die gewählte automatische Form mit eigenen Ziehpunkten.
tool-desc-text = Erstellt und bearbeitet Textblöcke.
tool-desc-pencil = Malt harte Linien von einem Pixel Breite in Bitmaps.
tool-desc-vectorPath = Zeichnet einen Vektorpfad freihändig.
tool-desc-redrawPath = Zeichnet einen Abschnitt des ausgewählten Pfads neu.
tool-desc-brush = Malt Pinselstriche in Bitmaps.
tool-desc-scale = Skaliert und dreht die ausgewählten Objekte.
tool-desc-skew = Neigt die ausgewählten Objekte.
tool-desc-distort = Verzerrt die ausgewählten Objekte an ihren Eckpunkten.
tool-desc-nineSliceScale = Skaliert Objekte, ohne Ecken und Ränder zu verzerren.
tool-desc-freeform = Biegt Pfade durch Ziehen oder Schieben.
tool-desc-reshapeArea = Formt Pfade innerhalb eines Kreises um, wie mit dem Finger.
tool-desc-pathScrubAdd = Verstärkt beim Überfahren den Druck des Pinselstrichs eines Pfads.
tool-desc-pathScrubSubtract = Verringert beim Überfahren den Druck des Pinselstrichs eines Pfads.
tool-desc-eyedropper = Nimmt eine Farbe aus dem Dokument auf.
tool-desc-paintBucket = Füllt Pixel ähnlicher Farbe oder ein Objekt mit der Füllfarbe.
tool-desc-gradient = Füllt mit einem Farbverlauf.
tool-desc-eraser = Löscht Pixel in Bitmaps.
tool-desc-blur = Zeichnet Bereiche einer Bitmap weich.
tool-desc-sharpen = Zeichnet Bereiche einer Bitmap schärfer.
tool-desc-dodge = Hellt Bereiche einer Bitmap auf.
tool-desc-burn = Dunkelt Bereiche einer Bitmap ab.
tool-desc-smudge = Verwischt Farben in einer Bitmap wie mit dem Finger.
tool-desc-rubberStamp = Kopiert Pixel von einer Quelle an eine andere Stelle, Alt+Klick setzt die Quelle.
tool-desc-replaceColor = Ersetzt beim Malen eine Farbe einer Bitmap durch eine andere.
tool-desc-redEyeRemoval = Entfernt rote Augen auf Fotos.
tool-desc-knife = Schneidet Pfade entlang der gezogenen Linie auf.
tool-desc-hotspotRect = Zeichnet einen rechteckigen Hotspot für Links.
tool-desc-hotspotCircle = Zeichnet einen kreisförmigen Hotspot für Links.
tool-desc-hotspotPolygon = Zeichnet einen Hotspot beliebiger Form Punkt für Punkt.
tool-desc-slice = Teilt das Dokument in rechteckige Segmente für den Web-Export.
tool-desc-polygonSlice = Zeichnet ein Segment beliebiger Form Punkt für Punkt.
tool-desc-hand = Verschiebt die Ansicht des Dokuments.
tool-desc-zoom = Vergrößert die Ansicht, mit Alt verkleinert es sie.

## Tooltips im Werkzeugbedienfeld

tool-tip-hideSlices = Segmente und Hotspots ausblenden (2)
tool-tip-showSlices = Segmente und Hotspots einblenden (2)
tool-tip-screenStandard = Standardbildschirmmodus
tool-tip-screenFullMenus = Vollbildmodus mit Menüs
tool-tip-screenFull = Vollbildmodus
tool-tip-strokeColor = Strichfarbe
tool-tip-fillColor = Füllfarbe
tool-tip-defaultColors = Standardfarben für Pinselstrich und Füllung anwenden (D)
tool-tip-noColor = Keine Pinselstriche oder Füllungen
tool-tip-swapColors = Pinselstrich-/Füllfarben austauschen (X)

## Automatische Formen (Rechteck-Flyout und Bedienfeld „Automatische Formen“)

shape-rounded-rectangle = Abgerundetes Rechteck
shape-beveled-rectangle = Abgeflachtes Rechteck
shape-chamfer-rectangle = Abgefastes Rechteck
shape-arrow = Pfeil
shape-arrow-line = Pfeillinie
shape-connector-line = Verbindungslinie
shape-doughnut = Donut
shape-pie = Torte
shape-l-shape = L-Form
shape-measure-tool = Messwerkzeug
shape-smart-polygon = Intelligentes Polygon
shape-spiral = Spirale
shape-star = Stern
shape-3d-box = 3D-Kasten
shape-annotation = Anmerkung
shape-calendar = Kalender
shape-clock = Uhr
shape-cog = Zahnrad
shape-crescent = Mondsichel
shape-cube = Würfel
shape-cylinder = Zylinder
shape-file-info = Dateiinfo
shape-flowchart = Flussdiagramm
shape-frame = Rahmen
shape-frame-2 = Rahmen 2
shape-frame-3 = Rahmen 3
shape-heart = Herz
shape-inspire = Anregung
shape-perspective = Perspektive
shape-pie-chart = Tortendiagramm
shape-pinwheel = Windrad
shape-posable-fig = Gliederpuppe
shape-revolve-path = Rotationspfad
shape-rounded-polygon = Abgerundetes Polygon
shape-ruler = Lineal
shape-save-time-stamp = Zeitstempel
shape-tabs = Registerkarten
shape-talking = Sprechblase
shape-thinking = Gedankenblase
shape-tube = Röhre
shape-wave = Welle

## Tooltips der Steuerpunkte automatischer Formen (Anzeige; im Dokument steht der deutsche Text)
autoshape-tip-spiral-toggle = Spirale öffnen/schließen
autoshape-tip-corner-toggle = Klicken, um zwischen den Ecken umzuschalten
