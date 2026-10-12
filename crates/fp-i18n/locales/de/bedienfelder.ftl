# Bedienfelder (crates\fp-ui\src\panels\*), Präfix panel-. Zuständig: I18N-BEDIENFELDER.
# Bedienfelder sind schmal: Texte knapp halten; lange Texte werden mit „…“ gekürzt.

## Reitertitel (Fenster-Menü, Dock)
panel-title-optimize = Optimieren
panel-title-history = Verlauf
panel-title-align = Ausrichten
panel-title-pages = Seiten
panel-title-states = Status
panel-title-layers = Ebenen
panel-title-styles = Stile
panel-title-color-palette = Farbpalette
panel-title-swatches = Farbfelder
panel-title-special-characters = Sonderzeichen
panel-title-image-editing = Bildbearbeitung
panel-title-path = Pfad
panel-title-auto-shapes = Automatische Formen
panel-title-document-library = Dokumentbibliothek
panel-title-common-library = Allgemeine Bibliothek
panel-title-info = Info
panel-title-color-mixer = Farbmischer
panel-title-url = URL
panel-title-behaviors = Verhalten
panel-title-find-replace = Suchen und ersetzen
panel-title-symbol-properties = Symboleigenschaften
panel-title-auto-shape-properties = Eigenschaften für automatische Form
panel-title-css-properties = CSS-Eigenschaften

## Allgemein
# Anzeigename eines Status ohne eigenen Namen
panel-state-default-name = Status { $n }

## Ausrichten
panel-align-relative-canvas = Relativ zur Leinwand
panel-align-relative-object = Relativ zum Objekt
panel-align-to-canvas = Zur Leinwand ausrichten
panel-align-align = Ausrichten:
panel-align-left = Links ausrichten
panel-align-hcenter = Horizontal zentrieren
panel-align-right = Rechts ausrichten
panel-align-top = Oben ausrichten
panel-align-vcenter = Vertikal zentrieren
panel-align-bottom = Unten ausrichten
panel-align-distribute = Proportionieren:
panel-align-dist-top = Oberkanten verteilen
panel-align-dist-vcenter = Vertikale Mitten verteilen
panel-align-dist-bottom = Unterkanten verteilen
panel-align-dist-left = Linke Kanten verteilen
panel-align-dist-hcenter = Horizontale Mitten verteilen
panel-align-dist-right = Rechte Kanten verteilen
panel-align-size = Größe:
panel-align-match-width = Breite angleichen
panel-align-match-height = Höhe angleichen
panel-align-match-both = Breite und Höhe angleichen
panel-align-space = Abstand:
panel-align-space-v = Vertikal gleichmäßig verteilen
panel-align-space-h = Horizontal gleichmäßig verteilen
panel-align-space-even = Gleichm.

## Verlauf
panel-history-copy-steps = Schritte kopieren
panel-history-save-command = Als Befehl speichern
panel-history-replay = Wiederholen
panel-history-replay-label = Schritte wiederholen
panel-history-copied = Schritte in die Zwischenablage kopiert

## Seiten
panel-pages-count = { $n ->
    [one] { $n } Seite
   *[other] { $n } Seiten
}
panel-pages-new = Neue / doppelte Seite
panel-pages-delete = Seite löschen
panel-pages-invalid-name = Ungültiges Zeichen im Seitennamen: \ / : * ? " < > |

## Status
panel-states-no-loop = Keine Schleife
panel-states-forever = Endlos
panel-states-delay-title = Statusverzögerung
panel-states-delay-label = Statusverzögerung:
panel-states-delay-unit = /100 Sek.
panel-states-include-export = Beim Export einschließen
panel-states-onion = Zwiebelschichteneffekt
panel-states-loop = GIF-Animationsschleife
panel-states-distribute = Auf Status verteilen
panel-states-new = Neu / Status duplizieren
panel-states-delete = Status löschen

## Ebenen
panel-layers-opacity = Deckkraft
panel-layers-web-layer = Webebene
panel-layers-kind-bitmap = Bitmap
panel-layers-kind-rectangle = Rechteck
panel-layers-kind-path = Pfad
panel-layers-kind-text = Text
panel-layers-kind-group = Gruppe
panel-layers-kind-compound = Zusammengesetzte Form
panel-layers-kind-symbol = Symbol
panel-layers-kind-slice = Segment
panel-layers-kind-hotspot = Hotspot
panel-layers-kind-object = Objekt
panel-layers-new-sublayer = Neue Unterebene
panel-layers-new-layer = Neu / Ebene duplizieren
panel-layers-new-bitmap = Neues Bitmap-Bild
panel-layers-add-mask = Maske hinzufügen
panel-layers-delete = Auswahl löschen

## Stile
panel-styles-source-document = Aktuelles Dokument
panel-styles-item = Stil { $name }
panel-styles-empty = Keine Stile im Dokument
panel-styles-new = Neuer Stil
panel-styles-delete = Stil löschen

## Farbfelder
panel-swatches-set-cubes = Farbwürfel
panel-swatches-set-continuous = Kontinuierlicher Farbton
panel-swatches-set-windows = Windows-System
panel-swatches-set-mac = Macintosh-System
panel-swatches-set-grayscale = Graustufen
panel-swatches-set-custom = Benutzerdefiniert
panel-swatches-menu = Farbfelder
panel-swatches-add = Farbfelder hinzufügen…
panel-swatches-replace = Farbfelder ersetzen…
panel-swatches-save = Farbfelder speichern…
panel-swatches-clear = Farbfelder löschen
panel-swatches-sort = Nach Farbe sortieren
panel-swatches-add-fill = Aktuelle Füllfarbe hinzufügen
panel-swatches-grid = Farbfelder-Raster

## Farbmischer
panel-mixer-defaults = Standardfarben
panel-mixer-swap = Farben austauschen
panel-mixer-model-hex = Hexadezimal
panel-mixer-model-grayscale = Graustufen

## Farbpalette
panel-palette-complementary = Komplementär
panel-palette-triad = Triade
panel-palette-analogous = Analog
panel-palette-square = Quadrat
panel-palette-add = Zu Farbfeldern hinzufügen

## Dokumentbibliothek
panel-doclib-kind-graphic = Grafik
panel-doclib-kind-button = Schaltfläche
panel-doclib-kind-animation = Animation
panel-doclib-kind-component = Komponente
panel-doclib-kind-symbol = Symbol
panel-doclib-count = { $n ->
    [one] { $n } Element
   *[other] { $n } Elemente
}
panel-doclib-kind = Art
panel-doclib-place = In Leinwandmitte einfügen
panel-doclib-new-symbol = Neues Symbol (aus der Auswahl)
panel-doclib-delete = Löschen
panel-doclib-delete-title = Symbol löschen
panel-doclib-delete-text = Das Symbol „{ $name }“ und alle Instanzen löschen?

## Allgemeine Bibliothek
panel-commonlib-own = Eigene Bibliothek
panel-commonlib-original = Ressourcenordner des Originals
panel-commonlib-empty = Keine Bibliotheksordner
panel-commonlib-insert = In das Dokument einfügen

## Automatische Formen
panel-autoshapes-unavailable = Die Laufzeit der automatischen Formen ist nicht verfügbar.
panel-autoshapes-insert = In die Leinwandmitte einfügen
panel-asp-none = Keine automatische Form ausgewählt.
panel-asp-default-title = Automatische Form
panel-asp-no-params = Diese Form hat keine einstellbaren Eigenschaften.
# Parameter der Formen (Beschriftung aus fp-script)
panel-asp-p-rotation = Drehung
panel-asp-p-apex = Scheitelpunkt
panel-asp-p-apex-arc = Scheitelbogen
panel-asp-p-arc = Bogen
panel-asp-p-tip-length = Länge der Spitze
panel-asp-p-tip-width = Breite der Spitze
panel-asp-p-count = Anzahl
panel-asp-p-clockwise = Im Uhrzeigersinn
panel-asp-p-closed = Geschlossen
panel-asp-p-corner-size = Eckgröße
panel-asp-p-corner-tl = Ecke oben links
panel-asp-p-corner-tr = Ecke oben rechts
panel-asp-p-corner-br = Ecke unten rechts
panel-asp-p-corner-bl = Ecke unten links
panel-asp-p-bend1 = Rundung Knick 1
panel-asp-p-bend2 = Rundung Knick 2
panel-asp-p-corner-type = Eckenart
panel-asp-p-curvature = Krümmung
panel-asp-p-curve = Kurve
panel-asp-p-curve-radius = Kurvenradius
panel-asp-p-tooth-depth = Zahntiefe
panel-asp-p-subdivisions = Unterteilungen
panel-asp-p-offset-x = Versatz X
panel-asp-p-offset-y = Versatz Y
panel-asp-p-format = Format
panel-asp-p-focal = Brennweite
panel-asp-p-frame-width = Rahmenbreite
panel-asp-p-gap = Spalt
panel-asp-p-height = Höhe
panel-asp-p-start = Anfang
panel-asp-p-end = Ende
panel-asp-p-shaft-thickness = Schaftdicke
panel-asp-p-hole = Loch
panel-asp-p-hollow = Hohl (0–1)
panel-asp-p-hour = Stunde
panel-asp-p-hue = Farbton
panel-asp-p-inner-cutout = Innenausschnitt
panel-asp-p-inner-radius-tube = Innerer Radius (Röhre)
panel-asp-p-length = Länge
panel-asp-p-link-corners = Ecken gemeinsam
panel-asp-p-minute = Minute
panel-asp-p-direction = Richtung
panel-asp-p-month = Monat (0–11)
panel-asp-p-points = Zacken
panel-asp-p-frame-offset = Rahmenversatz
panel-asp-p-overhang = Überhang
panel-asp-p-size = Größe
panel-asp-p-radius = Radius
panel-asp-p-radius1 = Radius 1
panel-asp-p-radius2 = Radius 2
panel-asp-p-inner-radius = Innerer Radius
panel-asp-p-rotation-x = Drehung X
panel-asp-p-rotation-y = Drehung Y
panel-asp-p-roundness = Rundung
panel-asp-p-roundness1 = Rundung 1
panel-asp-p-roundness2 = Rundung 2
panel-asp-p-width = Breite
panel-asp-p-seed = Zufallswert
panel-asp-p-symbol = Symbol (-1 = Auswahl)
panel-asp-p-sides = Seiten
panel-asp-p-tip-size = Spitzengröße
panel-asp-p-style = Stil (0 einfarbig, 1 zweifarbig)
panel-asp-p-teeth = Zähne
panel-asp-p-text = Text
panel-asp-p-thickness = Dicke
panel-asp-p-turns = Windungen
panel-asp-p-pointer-x = Zeiger X
panel-asp-p-pointer-y = Zeiger Y
panel-asp-p-unit = Einheit
panel-asp-p-shaft-length = Schaftlänge
panel-asp-p-wavelength = Wellenlänge
panel-asp-p-year = Jahr
panel-asp-p-color = Farbe
# Auswahlwerte (gespeichert wird der deutsche Wert, angezeigt die Übersetzung)
panel-asp-o-straight = gerade
panel-asp-o-round = rund
panel-asp-o-inverted = nach innen
panel-asp-o-hvh = waagerecht – senkrecht – waagerecht
panel-asp-o-vhv = senkrecht – waagerecht – senkrecht
panel-asp-o-none = Ohne
panel-asp-o-head1 = Spitze 1
panel-asp-o-head2 = Spitze 2
panel-asp-o-bar = Querstrich
panel-asp-o-box = Kästchen

## Optimieren
panel-opt-no-document = Kein Dokument geöffnet.
panel-opt-no-canvas = keine Leinwand
panel-opt-settings-error = Exporteinstellungen: { $error }
panel-opt-preset-gif-web216 = GIF Web 216
panel-opt-preset-gif-websnap256 = GIF WebSnap 256
panel-opt-preset-gif-websnap128 = GIF WebSnap 128
panel-opt-preset-gif-adaptive256 = GIF Adaptiv 256
panel-opt-preset-jpeg-better = JPEG – Höhere Qualität
panel-opt-preset-jpeg-smaller = JPEG – Kleinere Datei
panel-opt-preset-anim-gif-websnap128 = Animiertes GIF WebSnap 128
panel-opt-preset-png8 = PNG8
panel-opt-preset-png24 = PNG24
panel-opt-preset-png32 = PNG32
panel-opt-preset-webp80 = WebP 80
panel-opt-preset-webp-lossless = WebP verlustfrei
panel-opt-preset-avif60 = AVIF 60
panel-opt-preset-apng = APNG
panel-opt-preset-anim-webp = Animiertes WebP
panel-opt-format-anim-gif = Animiertes GIF
panel-opt-format-webp-lossless = WebP verlustfrei
panel-opt-format-anim-webp = Animiertes WebP
panel-opt-palette-adaptive = Adaptiv
panel-opt-palette-web-adaptive = Web-Adaptiv
panel-opt-palette-web216 = Web 216
panel-opt-palette-exact = Exakt
panel-opt-palette-mac = Macintosh
panel-opt-palette-windows = Windows
panel-opt-palette-grayscale = Graustufen
panel-opt-palette-bw = Schwarzweiß
panel-opt-palette-uniform = Einheitlich
panel-opt-palette-custom = Benutzerdefiniert
panel-opt-custom = Benutzerdefiniert
panel-opt-transp-none = Ohne Transparenz
panel-opt-transp-index = Indextransparenz
panel-opt-transp-alpha = Alphatransparenz
panel-opt-matte = Hintergr.:
panel-opt-colors = Farben:
panel-opt-loss = Verlust:
panel-opt-dither = Rastern:
panel-opt-quality = Qualität:
panel-opt-selective-quality = Selektive Qualität:
panel-opt-smoothing = Glättung:
panel-opt-speed = Geschwindigkeit:
panel-opt-interlaced = Zeilensprung
panel-opt-progressive = Progressiv
panel-opt-sort-by = Sortieren nach:
panel-opt-sort-none = Ohne
panel-opt-sort-luminance = Leuchtkraft
panel-opt-sort-popularity = Häufigkeit
panel-opt-color-count = { $n ->
    [one] { $n } Farbe
   *[other] { $n } Farben
}
panel-opt-add-transparent = Farbe zu Transparenz hinzufügen
panel-opt-remove-transparent = Farbe aus Transparenz entfernen
panel-opt-select-transparent = Farbe für Transparenz wählen (ausgewählte Farbe als einzige transparente)
panel-opt-select-transparent-label = Transparenz wählen
panel-opt-edit-color = Farbe bearbeiten
panel-opt-snap-web = Websicher machen
panel-opt-lock-color = Farbe sperren
panel-opt-add-color = Farbe hinzufügen
panel-opt-delete-color = Farbe löschen

## Pfad
panel-path-combine = Pfade kombinieren:
panel-path-join = Verbinden
panel-path-split = Teilen
panel-path-union = Vereinigen
panel-path-intersect = Überschneiden
panel-path-punch = Ausstanzen
panel-path-crop = Zuschneiden
panel-path-alter = Pfad ändern:
panel-path-simplify = Vereinfachen...
panel-path-expand-stroke = Strich erweitern...
panel-path-inset = Pfad einlassen...
panel-path-hard-fill = Harte Füllung
panel-path-anti-alias-fill = Geglättete Füllung
panel-path-feather-fill = Weiche Füllung
panel-path-edit-points = Punkte bearbeiten:
panel-path-snap-pixel = An Pixeln ausrichten
panel-path-to-marquee = Pfad in Auswahlrahmen
panel-path-compound = Zusammengesetzte Form:
panel-path-combine-compound = Kombinieren

## Info
panel-info-width = B:
panel-info-height = H:

## Sonderzeichen
panel-special-copied = „{ $ch }“ in die Zwischenablage kopiert (kein Text ausgewählt)

## Bildbearbeitung
panel-imgedit-tools = Bitmap-Werkzeuge:
panel-imgedit-adjust = Anpassungen:
panel-imgedit-auto-levels = Auto-Tonwert
panel-imgedit-brightness = Helligkeit/Kontrast...
panel-imgedit-curves = Kurven...
panel-imgedit-levels = Tonwert...
panel-imgedit-hue-sat = Farbton/Sättigung...
panel-imgedit-invert = Umkehren
panel-imgedit-filters = Filter:
panel-imgedit-gaussian = Gaußscher Weichzeichner...
panel-imgedit-unsharp = Unscharf maskieren...
panel-imgedit-noise = Rauschen...
panel-imgedit-numeric = Numerisch transformieren...

## URL
panel-url-add = Aktuelle URL zur Bibliothek hinzufügen
panel-url-delete = URL aus der Bibliothek löschen

## Verhalten
panel-beh-simple-rollover = Einfaches Rollover
panel-beh-swap-image = Bild austauschen
panel-beh-swap-restore = Bildaustausch wiederherstellen
panel-beh-nav-image = Navigationsleistenbild festlegen
panel-beh-nav-over = Navigationsleiste - Over
panel-beh-nav-down = Navigationsleiste - Down
panel-beh-nav-restore = Navigationsleiste wiederherstellen
panel-beh-status-text = Statusleistentext festlegen
panel-beh-popup = Popupmenü anzeigen
panel-beh-popup-menu = Popupmenü anzeigen…
panel-beh-sound = Sound abspielen
panel-beh-generic = Verhalten
panel-beh-add = Verhalten hinzufügen
panel-beh-remove = Verhalten löschen
panel-beh-events = Ereignisse
panel-beh-actions = Aktionen
panel-beh-select-hint = Hotspot oder Segment auswählen

## Suchen und ersetzen
panel-find-scope-selection = Auswahl suchen
panel-find-scope-page = Seite suchen
panel-find-kind-text = Text suchen
panel-find-kind-font = Schrift suchen
panel-find-kind-color = Farbe suchen
panel-find-kind-url = URL suchen
panel-find-kind-nonweb = Nicht-Web216 suchen
panel-find-find = Suchen:
panel-find-change = Ändern:
panel-find-nonweb-hint = Sucht Füll- und Strichfarben außerhalb der Web-216-Palette; „Ersetzen“ rastet sie auf die nächste websichere Farbe ein.
panel-find-whole-word = Ganzes Wort
panel-find-match-case = Groß/Klein
panel-find-next = Suchen
panel-find-replace = Ersetzen
panel-find-replace-all = Alle ersetzen
panel-find-hits = { $n } Treffer

## Symboleigenschaften
panel-symprops-none = Kein Rich Symbol ausgewählt
panel-symprops-name = Name
panel-symprops-value = Wert
panel-symprops-note = Hinweis: Die Darstellung der Rich Symbols folgt den Werten erst mit fp-script.

## CSS-Eigenschaften
panel-css-select = Objekt auswählen
panel-css-copy-all = Alle kopieren
panel-css-copied = CSS in die Zwischenablage kopiert
