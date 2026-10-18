# Meldungen: Fehler (err-), Status-/Infomeldungen und Verlaufsschritte (msg-), KI-Auswahl (ai-).
# Zuständig: I18N-ENGINE. Deutsch ist die Ausgangssprache; Englisch nach GLOSSAR.md.

## Fehlerarten von fp-engine (EngineError, Text = Display)
err-unknown-command = unbekannter Befehl „{ $name }“
err-not-implemented = Befehl „{ $name }“ ist noch nicht umgesetzt
err-unavailable = Befehl „{ $name }“ ist nicht verfügbar: { $reason }
err-invalid-params = ungültige Parameter für „{ $name }“: { $message }
err-no-document = kein Dokument geöffnet
err-selection = keine passende Auswahl: { $detail }
err-element-not-found = Element { $id } nicht gefunden
err-not-found = { $what } nicht gefunden
err-io = Datei: { $detail }
err-format = FW-PNG: { $detail }
err-backend-missing = für diese Operation fehlt { $what }
err-failed = { $detail }

## Voraussetzungen von Befehlen (Begründung bei „nicht verfügbar“)
err-req-document = kein Dokument geöffnet
err-req-selection = keine Auswahl
err-req-min-selected = mindestens { $n } Objekte auswählen
err-req-all-of = Auswahl enthält nicht nur Objekte der Art „{ $kind }“
err-req-any-of = Auswahl enthält kein Objekt der Art „{ $kind }“
err-req-clipboard = Zwischenablage leer
err-req-has-path = Dokument wurde noch nicht gespeichert
err-req-multiple-pages = nur eine Seite
err-req-multiple-frames = nur ein Status

## Arten von Objekten in den Begründungen ($kind)
obj-kind-bitmap = Bitmap
obj-kind-slice = Segment
obj-kind-hotspot = Hotspot
obj-kind-web-object = Hotspot oder Segment
obj-kind-rect-primitive = Rechteck
obj-kind-text = Text
obj-kind-path-attrs = Objekt mit Strich und Füllung
obj-kind-path = Pfad
obj-kind-compound-shape = zusammengesetzte Form
obj-kind-group = Gruppe
obj-kind-mask-group = Maskengruppe
obj-kind-instance = Symbolinstanz
obj-kind-graphic-instance = Grafikinstanz
obj-kind-button-instance = Schaltflächeninstanz
obj-kind-animation-instance = Animationsinstanz

## Parameterprüfung ($key = Parametername)
err-param-missing = Pflichtparameter „{ $key }“ fehlt
err-param-invalid = Parameter „{ $key }“ hat einen ungültigen Wert
err-param-bad-number = { $key }: ungültige Zahl
err-param-not-number = { $key }: keine Zahl
err-param-number-expected = { $key }: Zahl erwartet
err-param-absent = { $key } fehlt
err-param-negative = { $key }: negativ
err-param-bool-expected = { $key }: true/false erwartet
err-param-text-expected = { $key }: Text erwartet
err-param-hex-color-expected = { $key }: Farbe #rrggbb erwartet
err-param-color-expected = { $key }: Farbe erwartet
err-param-point-expected = { $key }: Punkt {"{"}x, y{"}"} erwartet
err-param-rect-expected = { $key }: Rechteck erwartet
err-param-ints-expected = { $key }: ganze Zahlen erwartet
err-param-id-expected = { $key }: ID erwartet
err-param-ids-expected = { $key }: Liste von IDs erwartet
err-param-points-expected = { $key }: Punktliste erwartet
err-param-detail = { $key }: { $detail }

## Nicht gefunden – mit Namen
err-nf-page-n = Seite { $n }
err-nf-tool = Werkzeug „{ $name }“
err-nf-shortcut-set = Tastenkürzel-Set „{ $name }“
err-nf-extension = Erweiterung „{ $name }“
err-nf-saved-command = gespeicherter Befehl „{ $name }“
err-nf-symbol-named = Symbol „{ $name }“
err-nf-symbol-id = Symbol { $id }
err-nf-style = Stil „{ $name }“
err-nf-brush = Pinselstrich „{ $name }“
err-nf-preset = Vorgabe „{ $name }“
err-nf-folder = Ordner { $path }
err-nf-html-file = HTML-Datei { $path }
err-update-html-no-table = { $path } ist keine HTML-Seite (weder </body> noch </html> gefunden). Die Datei bleibt unverändert.

## Weitere Fehler mit Werten
err-unsaved-discard = „{ $name }“ hat ungespeicherte Änderungen (discard: true zum Verwerfen)
err-unsaved = „{ $name }“ hat ungespeicherte Änderungen
err-invalid-name = ungültiger Name „{ $name }“
err-unknown-model = unbekanntes KI-Modell „{ $name }“
err-model-wrong-task = „{ $name }“ ist kein Modell für diese Aufgabe
err-unknown-blend = unbekannter Mischmodus { $name }
err-unknown-effect = unbekannter Effekt { $name }
err-unknown-preset = unbekannte Vorgabe „{ $name }“
err-unknown-format = unbekanntes Format „{ $name }“
err-unknown-palette = unbekannte Palette „{ $name }“
err-unknown-export-kind = unbekannte Exportart „{ $name }“
err-unknown-shortcut = unbekanntes Kürzel „{ $name }“
err-builtin-set-readonly = mitgelieferte Sets sind schreibgeschützt: zuerst mit „duplicate“ ein eigenes Set anlegen
err-unknown-tool = unbekanntes Werkzeug „{ $name }“
err-unknown-shape = unbekannte Form { $name }
err-unknown-stroke-preset = unbekannte Strichvorgabe { $name }
err-invalid-key = ungültiger Schlüssel { $name }
err-prefs = Voreinstellungen: { $detail }
err-program-start = Programm konnte nicht gestartet werden: { $detail }
err-scan-unavailable = Bilderfassung nicht verfügbar: { $detail }
err-dictionary = Wörterbuch { $path }: { $detail }
err-brush-detail = Pinsel: { $detail }
err-page-missing = Seite { $n } fehlt
err-invalid-set-name = „{ $name }“ ist kein zulässiger Name für ein eigenes Set
err-set-not-found = Set „{ $name }“ nicht gefunden
err-style-library = Stilbibliothek: { $detail }
err-shortcut-menu-needs-modifier = Menükurzbefehle müssen Modifizierungstasten enthalten.
err-shortcut-tool-plain = Werkzeugkurzbefehle müssen aus Buchstaben oder Zahlen ohne Modifizierungstasten bestehen.
err-shortcut-builtin-no-delete = mitgelieferte Sets können nicht gelöscht werden
err-renderer-unavailable = Renderer nicht verfügbar

## Import (fp-io), Export (fp-export)
err-import-unknown-format = unbekanntes Dateiformat
err-import-unsupported = { $detail }
err-import-corrupt = Datei beschädigt: { $detail }
err-import-too-large = Bild zu groß: { $w }×{ $h }
err-import-decode = Dekodierfehler ({ $codec }): { $detail }
err-import-fwpng = FW-PNG: { $detail }
err-import-io = E/A-Fehler: { $detail }
err-export-bad-size = ungültige Bildgröße { $w }×{ $h } für dieses Format
err-export-unsupported = Format wird nicht unterstützt: { $detail }
err-export-encode = Kodierfehler: { $detail }
err-export-render = Rendern fehlgeschlagen: { $detail }
err-export-io = E/A-Fehler: { $detail }

## Skripte (fp-script)
err-script = Skript: { $detail }
err-script-failed = Skriptfehler
err-script-doc-closed = Dokument ist geschlossen
err-script-doc-n-closed = Dokument { $id } ist geschlossen

## KI-Auswahl (fp-ai)
ai-err-model-missing = KI-Modell „{ $name }“ fehlt ({ $file }). Bitte über „KI-Modelle verwalten“ laden oder die Datei in den Modellordner legen: { $folder }
ai-err-weights = KI-Modell ungültig: { $detail }
ai-err-download = Download fehlgeschlagen: { $detail }
ai-err-download-disabled = Download ist gesperrt (Test- oder Stapelbetrieb)
ai-err-checksum = Prüfsumme der Datei { $file } stimmt nicht (erwartet { $expected }, erhalten { $got })
ai-err-cancelled = abgebrochen
ai-err-input = { $detail }
ai-err-io = Datei: { $detail }

## KI-Modelle (fp-ai models.rs; Modellnamen bleiben, Zusätze übersetzt)
ai-model-sam2-tiny = SAM 2.1 Hiera Tiny
ai-model-sam2-small = SAM 2.1 Hiera Small
ai-model-birefnet-lite = BiRefNet lite (allgemein)
ai-model-birefnet = BiRefNet (allgemein, hohe Qualität)
ai-model-birefnet-lite-matting = BiRefNet lite Matting
ai-model-birefnet-hr-matting = BiRefNet HR Matting (hohe Qualität)
ai-model-sam2-tiny-note = Meta (facebookresearch/sam2): Code und Gewichte Apache-2.0, kommerzielle Nutzung erlaubt; trainiert auf SA-1B/SA-V
ai-model-sam2-small-note = Meta (facebookresearch/sam2): Code und Gewichte Apache-2.0, kommerzielle Nutzung erlaubt; trainiert auf SA-1B/SA-V
ai-model-birefnet-lite-note = ZhengPeng7/BiRefNet, Swin-T; Gewichte MIT. Trainingsdaten laut Autor u. a. DIS5K, DUTS, HRSOD, UHRSD, HRS10K, P3M-10k, humans (P3M-10k ist nur für nichtkommerzielle Forschung freigegeben)
ai-model-birefnet-note = ZhengPeng7/BiRefNet, Swin-L (langsamer); Gewichte MIT. Trainingsdaten wie BiRefNet lite (u. a. P3M-10k, nur nichtkommerzielle Forschung)
ai-model-birefnet-lite-matting-note = ZhengPeng7/BiRefNet, Swin-T, für Haare und Fell; Gewichte MIT. Trainingsdaten laut Autor P3M-10k, humans, AM-2k, Distinctions-646, HIM2K (teils nur nichtkommerzielle Forschung)
ai-model-birefnet-hr-matting-note = ZhengPeng7/BiRefNet, Swin-L, für Haare und Fell (langsamer); Gewichte MIT. Trainingsdaten wie Matting lite (teils nur nichtkommerzielle Forschung)

## Hinweise und Rückgaben von Befehlen
msg-wizard-animated-gif = mehrere Status: animiertes GIF
msg-wizard-png32-edit = Weiterbearbeitung: verlustfreies PNG32
msg-wizard-png8 = wenige Farben (Grafik): PNG8 mit exakter Farbzahl
msg-wizard-png32-alpha = viele Farben mit Transparenz: PNG32
msg-wizard-jpeg = viele Farben (Foto): JPEG Qualität 80
msg-spelling-no-dictionary = kein Wörterbuch gefunden: <sprache>.aff/.dic über „Rechtschreibprüfung einrichten“ angeben
msg-no-usage-data = Firepixel erfasst und sendet keine Nutzungsdaten.
msg-panel-alias = Bedienfeldmenü: wie „{ $target }“

## Standardnamen neuer Ebenen, Seiten und Status
msg-default-layer = Ebene { $n }
msg-default-web-layer = Webebene
msg-default-page = Seite { $n }
msg-default-state = Status { $n }
msg-default-background = Hintergrund
msg-default-copy = { $name } Kopie
msg-default-copy-n = { $name } Kopie { $n }
msg-default-sublayer = Unterebene
msg-default-document = Unbenannt-{ $n }
msg-default-document-nameless = Dokument
msg-tool-roundness = Rundung { $px } px

## Import-Details (fp-io, $detail in err-import-corrupt/-unsupported/-decode)
err-import-act-not-image = ACT-Paletten (*.act, 256 Farben + Anzahl) sind keine Bilder: im Optimieren-Bedienfeld „Palette laden“ bzw. in den Farbfeldern verwenden.
err-import-apng-no-frame = APNG ohne Einzelbild
err-import-av1-aborted = AV1-Dekoder abgebrochen (beschädigte Daten)
err-import-avif-item-missing = AVIF: Item fehlt
err-import-avif-no-image = AVIF ohne Bild
err-import-avif-primary-missing = AVIF: primäres Item fehlt
err-import-avif-tile-not-av1 = AVIF: Rasterkachel ist kein AV1-Bild
err-import-avif-tiles = AVIF: Kachelzahl passt nicht zum Raster
err-import-bad-bit-depth = ungültige Bittiefe
err-import-bbox-empty = Begrenzungsrahmen mit Breite oder Höhe ≤ 0
err-import-decoder-hangs = Dekoder hängt
err-import-eps-no-bbox = AI/EPS ohne %%BoundingBox
err-import-field-width = Feldbreite
err-import-gif-ext-cut = GIF-Erweiterung abgeschnitten
err-import-gif-no-image = GIF ohne Bild
err-import-gif-palette-cut = GIF-Palette abgeschnitten
err-import-gif-subblock-cut = GIF-Unterblock abgeschnitten
err-import-heif-construction = HEIF: Item-Konstruktion über Verweise wird nicht unterstützt
err-import-hevc-aborted = HEVC-Dekoder abgebrochen (beschädigte Daten)
err-import-ico-dib = ICO-DIB
err-import-ico-header = ICO-Kopf
err-import-ico-no-image = ICO ohne Bild
err-import-image-desc-cut = Bildbeschreibung abgeschnitten
err-import-jpeg-no-header = JPEG ohne Kopf
err-import-jpeg-short = JPEG-Daten zu kurz
err-import-length = Länge
err-import-lzw-size-missing = LZW-Codegröße fehlt
err-import-no-image-decoded = kein Bild dekodiert
err-import-not-gif = kein GIF
err-import-not-qoi = kein QOI
err-import-offset = Versatz
err-import-pdf-aborted = PDF-Interpreter abgebrochen (beschädigte Daten)
err-import-pdf-encrypted = verschlüsselte PDF-Dateien werden nicht unterstützt
err-import-pdf-no-page = PDF ohne Seite
err-import-pdf-unreadable = PDF nicht lesbar
err-import-plane-short = Ebene zu kurz
err-import-psd-no-composite = PSD ohne Composite
err-import-sample-offset = Sample-Versatz
err-import-signature-missing = Signatur fehlt
err-import-tiff-no-colormap = TIFF-Palette ohne ColorMap
err-import-tiff-sample-format = TIFF-Sampleformat wird nicht unterstützt
err-import-tiff-short = TIFF-Daten zu kurz
err-import-tile-no-image = Kachel ohne Bild
err-import-unknown-header-size = unbekannte Kopfgröße
err-import-webp-no-frame = WebP ohne Einzelbild
err-import-eps-no-ai-data = EPS ohne AI-Daten kann nicht gelesen werden: Das Original rastert sie mit seinem PostScript-Modul (MPS.dll), das in Firepixel nicht enthalten ist. Bitte als AI-8-kompatibles EPS, SVG oder Pixelbild speichern.
err-import-avif-kind = AVIF: Bildart „{ $kind }“ wird nicht unterstützt
err-import-bmp-compression = BMP-Kompression { $comp } mit { $bpp } Bit wird nicht unterstützt
err-import-clipboard-format = Zwischenablageformat „{ $name }“ wird nicht unterstützt
err-import-box-length = Box „{ $name }“ mit falscher Länge
err-import-image-size = Bildgröße { $w }×{ $h }
err-import-pixels-short = Pixeldaten zu kurz ({ $have } < { $need })
err-import-tga-type = TGA-Typ { $kind } wird nicht unterstützt
err-import-tiff-color-type = TIFF-Farbtyp { $kind } wird nicht unterstützt
err-import-wbmp-type = WBMP-Typ { $kind }

## FW-PNG (fp-fwpng) und Pixelpuffer (fp-raster)
err-fwpng-not-png = keine PNG-Signatur
err-fwpng-truncated-chunk = PNG-Chunk bei Offset { $offset } abgeschnitten
err-fwpng-missing-ihdr = PNG ohne IHDR
err-fwpng-bad-chunk = ungültiger Chunk { $kind }: { $detail }
err-fwpng-tree = Tag-Baum: { $detail } (Offset { $offset })
err-fwpng-atxt = aTXT: { $detail } (Offset { $offset })
err-fwpng-document = Dokument: { $detail }
err-fwpng-png-decode = PNG-Dekodierung: { $detail }
err-fwpng-png-encode = PNG-Kodierung: { $detail }
err-fwpng-rgba-size = RGBA-Puffer passt nicht zu { $width } × { $height }: { $bytes } Byte
err-fwpng-io = E/A: { $detail }
err-raster-too-large = Pixelpuffer zu groß: { $w }×{ $h }
err-raster-size-mismatch = Datenlänge { $got } passt nicht zur erwarteten Länge { $expected }

## Namen von Verlaufsschritten (Verlauf-Bedienfeld, „Rückgängig …“)
msg-step-add-popup = Pop-up-Menü hinzufügen
msg-step-clear-overrides = Abweichungen löschen
msg-step-duplicate-layer = Ebene duplizieren
msg-step-duplicate-state = Status duplizieren
msg-step-edit-popup = Pop-up-Menü bearbeiten
msg-step-edit-symbol = Symbol bearbeiten
msg-step-edit-text = Text bearbeiten
msg-step-export-settings = Exporteinstellungen
msg-step-export-wizard = Exportassistent
msg-step-html-setup = HTML-Einrichtung
msg-step-import = Importieren
msg-step-move = Verschieben
msg-step-move-copy = Kopie verschieben
msg-step-duplicate = Duplizieren
msg-step-move-point = Punkt verschieben
msg-step-mask-link = Maske verknüpfen
msg-step-mask-unlink = Verknüpfung der Maske aufheben
msg-step-transform = Transformieren
msg-step-pen = Stift
msg-step-add-states = Status hinzufügen
msg-step-export-options = Exportoptionen
msg-step-brush = Pinsel
msg-step-modify-rect = Rechteck modifizieren
msg-step-new-layer = Neue Ebene
msg-step-refine-edge = Kante verfeinern
msg-step-filter-image = Bild filtern
msg-step-expand-paths = Pfade erweitern
msg-step-inset-paths = Pfade einlassen
msg-step-feather-selection = Weiche Auswahlkante
msg-step-expand-selection = Auswahl erweitern
msg-step-contract-selection = Auswahl verkleinern
msg-step-border-selection = Rand auswählen
msg-step-smooth-selection = Auswahl glätten
msg-step-stamp = Stempel
msg-step-replace-color = Farbe ersetzen
msg-step-red-eye = Rote Augen entfernen
msg-step-edit-bitmap = Bitmap bearbeiten
msg-step-select-pixels = Pixel auswählen
msg-step-fill = Füllen
msg-step-edit-fill = Füllung bearbeiten
msg-step-fill-color = Füllfarbe
msg-step-stroke-color = Strichfarbe
msg-step-repeat-filter = Filter wiederholen
msg-step-spelling = Rechtschreibung
msg-step-file-info = Datei-Info

## Statushinweise der Werkzeuge
msg-tool-knife-needs-paths = Messer: zuerst Pfade auswählen.
msg-tool-needs-path = Zuerst einen Pfad auswählen.
msg-tool-refine-edge-needs-pixels = Kante verfeinern: zuerst eine Pixelauswahl erstellen (z. B. mit „Motiv auswählen“ oder „Objektauswahl“).
msg-tool-scrubber-needs-path = Pfadausbesserung: zuerst einen Pfad auswählen.
msg-tool-stamp-needs-source = Stempel: zuerst mit Alt+Klick die Quelle festlegen.

## Nicht gefunden ($what in err-not-found)
err-nf-active-layer = aktive Ebene
err-nf-contour = Kontur
err-nf-document = Dokument
err-nf-edit-view = Bearbeitungsansicht
err-nf-effect = Effekt
err-nf-guide = Hilfslinie
err-nf-jpeg-mask = JPEG-Maske
err-nf-layer = Ebene
err-nf-mask = Maske
err-nf-page = Seite
err-nf-point = Punkt
err-nf-recent-file = zuletzt geöffnete Datei
err-nf-saved-selection = gespeicherte Auswahl
err-nf-segment = Segment
err-nf-shared-layer = geteilte Ebene
err-nf-state = Status
err-nf-swatch = Farbfeld
err-nf-symbol = Symbol

## Fehlende Bausteine ($what in err-backend-missing)
err-backend-autoshape-runtime = die Laufzeit der automatischen Formen (fp-script)
err-backend-pathops = Pfadoperationen (fp-vector)
err-backend-renderer = Renderer (fp-render)
err-backend-renderer-page = Renderer (fp-render) für die Seitenansicht
err-backend-renderer-vector = Renderer (fp-render) für Vektorobjekte
err-backend-script-runtime = die Skript-Laufzeit (fp-script)
err-backend-text-layout = die Textauslegung
err-backend-text-renderer = ein Renderer für die Textauslegung

## Voraussetzungen und Verlauf
err-req-can-redo = nichts zu wiederholen
err-req-can-undo = nichts rückgängig zu machen
err-req-pixel-selection = keine Pixelauswahl
err-req-state = im aktuellen Zustand nicht möglich

## Auswahl ($detail in err-selection)
err-sel-bitmap-locked = die Bitmap ist gesperrt
err-sel-clipboard-no-object = Zwischenablage enthält kein Objekt
err-sel-empty = leer
err-sel-empty-pixels = leere Pixelauswahl
err-sel-group-no-mask = die Gruppe hat keine Maske
err-sel-instances = Instanzen
err-sel-mask-needs-bitmap = Maskenausgabe braucht eine Bitmap als Quelle (Bitmap auswählen)
err-sel-no-attributes = keine Attribute in der Zwischenablage
err-sel-no-bitmap = keine Bitmap
err-sel-no-bitmap-at-point = keine Bitmap ausgewählt bzw. unter dem Punkt
err-sel-no-bitmap-selected = keine Bitmap ausgewählt
err-sel-no-bitmap-under-stroke = keine Bitmap unter dem Strich
err-sel-no-bitmap-under-wand = keine Bitmap unter dem Zauberstab
err-sel-no-bounds = Auswahl ohne Grenzen
err-sel-no-bounds-short = keine Grenzen
err-sel-no-crop-area = kein Zuschneidebereich
err-sel-no-instance = keine Symbolinstanz ausgewählt
err-sel-no-mask-group = keine Maskengruppe ausgewählt
err-sel-no-object = kein Objekt
err-sel-no-object-below = kein Objekt darunter
err-sel-no-objects = keine Objekte ausgewählt
err-sel-no-objects-with-bounds = keine Objekte mit Grenzen
err-sel-no-path = kein Pfad
err-sel-no-paths = keine Pfade
err-sel-no-paths-selected = keine Pfade ausgewählt
err-sel-no-popup = ausgewähltes Objekt hat kein Pop-up-Menü
err-sel-no-single-open-contour = Pfad hat keine einzelne offene Kontur
err-sel-no-stroked-object = kein Objekt mit Strich ausgewählt
err-sel-no-styled-object = kein Objekt mit Stil ausgewählt
err-sel-no-subject = kein Motiv gefunden
err-sel-no-text = kein Text
err-sel-no-text-on-path = kein Text auf Pfad ausgewählt
err-sel-no-text-selected = kein Text ausgewählt
err-sel-no-web-object = kein Slice/Hotspot
err-sel-nothing = nichts ausgewählt
err-sel-nothing-to-copy = nichts zu kopieren
err-sel-nothing-to-filter = nichts zum Filtern ausgewählt
err-sel-page-empty = Seite ist leer
err-sel-point-outside-bitmap = Punkt außerhalb der Bitmap
err-sel-same-symbol = Instanzen desselben Symbols auswählen
err-sel-text-and-path = einen Text und einen Pfad auswählen
err-sel-two-instances = genau zwei Instanzen auswählen
err-sel-two-objects = mindestens zwei Objekte auswählen
err-sel-two-paths = mindestens zwei Pfade auswählen
err-sel-two-paths-needed = mindestens zwei Pfade nötig

## Parameter ($message in err-invalid-params)
err-param-action-ext = action: list/install/remove
err-param-ai-prompt = Klickpunkt (point/points) oder Rahmen (box) fehlt
err-param-ai-value = unbekannter Wert „{ $value }“
err-sel-ai-nothing-to-retune = keine KI-Auswahl zum Nachstellen (die Auswahl wurde inzwischen geändert)
err-param-anchor-or-rect = anchor oder rect angeben
err-param-colors = colors: #rrggbb erwartet
err-param-contours = contours: Liste erwartet
err-param-crop = crop: Rechteck
err-param-data = data: Objekt erwartet
err-param-dither = dither: Zahl, true/false oder "2x2"
err-param-effect-or-index = effect oder index angeben
err-param-effects = effects: Liste erwartet
err-param-empty-rect = leerer Rahmen
err-param-four-points = vier Punkte erwartet
err-param-items = items: Liste von {"{"}text, url, target, submenu{"}"}
err-param-items-empty = items: mindestens ein Eintrag
err-param-matrix = Matrix mit 6 oder 9 Werten erwartet
err-param-matte = matte: Farbe
err-param-menu = menu: Objekt erwartet
err-param-name-missing = Name fehlt
err-param-no-contours = keine Konturen
err-param-no-points = keine Punkte
err-param-no-style = kein Stil angegeben
err-param-node = Knoten: {"{"}anchor, in, out{"}"} oder [x, y]
err-param-nodes-missing = nodes fehlt
err-param-not-object = Parameter müssen ein JSON-Objekt sein
err-param-output = output: "selection" oder "mask"
err-param-paper = paper: A3/A4/A5/Letter/Legal oder [b, h]
err-param-params = params: Objekt erwartet
err-param-path-needed = Dokument hat noch keinen Dateinamen: Parameter „path“ angeben
err-param-path-or-code = path oder code angeben
err-param-points-list = points: Liste erwartet
err-param-points-numbers = points: Zahlen erwartet
err-param-points-triples = points: [x, y, label] erwartet
err-param-radius-zero = Radius 0
err-param-replace = replace: Liste
err-param-settings = settings: Objekt erwartet
err-param-settings-missing = settings fehlt
err-param-three-points = mindestens drei Punkte
err-param-transparency = transparency: none/index/alpha/rgba
err-param-two-points = mindestens zwei Punkte
err-param-value = Wert
err-param-values = values: Objekt erwartet

## Sonstige Fehler
err-bitmap-unreadable = Bitmap konnte nicht gelesen werden
err-bitmap-unreadable-short = Bitmap
err-compound-insert-failed = Compound Shape konnte nicht eingefügt werden
err-cut-misses-path = Die Schnittlinie trifft keinen ausgewählten Pfad.
err-degenerate-quad = entartetes Viereck
err-distribute-failed = Verteilen fehlgeschlagen
err-element-insert-failed = Element konnte nicht eingefügt werden
err-empty-area = leerer Bereich
err-file-no-objects = die Datei enthält keine Objekte
err-file-no-symbols = die Datei enthält keine Symbole
err-format-no-palette = das Exportformat hat keine Palette
err-guides-locked = Hilfslinien sind gesperrt
err-image-insert-failed = Bild konnte nicht eingefügt werden
err-internal = intern
err-layer-no-delete = diese Ebene kann nicht gelöscht werden
err-no-filename = kein Dateiname
err-no-filter-yet = noch kein Filter angewandt
err-no-html = kein HTML erzeugt
err-no-master-page = keine Masterseite
err-no-new-object = kein neues Objekt
err-no-page = keine Seite
err-no-scanner = kein Scanner ausgewählt
err-no-shared-layer = keine freigegebene Ebene aktiv
err-no-size-optimize = für dieses Format gibt es keine Größenoptimierung (nur GIF/PNG 8 und JPEG)
err-no-steps = keine Schritte ausgewählt
err-no-symbol-edit = keine Symbol-Bearbeitung
err-no-symbols = keine Symbole
err-no-valid-page = keine gültige Seite
err-not-library-symbol = kein Symbol aus einer Bibliotheksdatei
err-object-move-failed = Objekt konnte nicht verschoben werden
err-only-own-layers = nur eigene Ebenen können dupliziert werden
err-parent-list = Elternliste
err-paste-failed = Einfügen fehlgeschlagen
err-scan-cancelled = Scannen abgebrochen
err-scan-shell-disabled = Bilderfassung: Starten von Fremdprogrammen ist abgeschaltet (FP_NO_SHELL_OPEN)
err-scan-windows-only = Bilderfassung gibt es nur unter Windows
err-steps-not-replayable = die ausgewählten Schritte sind nicht wiederholbar
err-stroke = Strich
err-stroke-misses-path = Der Strich berührt keinen ausgewählten Pfad.
err-view-empty = Ansicht leer
err-web-layer-no-duplicate = die Web-Ebene kann nicht dupliziert werden
err-web-layer-no-share = die Web-Ebene kann nicht freigegeben werden
err-sublayer-no-share = eine Unterebene kann nicht für Seiten freigegeben werden

# >>> UE-3 Farben und Schriften
# Fehler aus fp-color (Farbfelder, CMYK) und fp-text (Schriftdateien), UE-3
err-color-cmyk-short = CMYK-Tabelle zu kurz: { $n } Byte, erwartet { $expected }
err-color-swatch-short = Datei zu kurz ({ $n } Byte)
err-color-swatch-signature = keine { $format }-Datei (Signatur fehlt)
err-color-swatch-truncated = { $format }-Datei abgeschnitten
err-color-swatch-no-palette = keine Palette enthalten
err-text-font-io = Schriftdatei { $path } nicht lesbar: { $error }
err-text-bad-font = ungültige Schrift: { $detail }
# <<< UE-3 Farben und Schriften

# >>> UE-3 Import-Hinweise
# Hinweise beim Import (fp-io, Feld warnings im Befehlsergebnis), UE-3
msg-import-warn-psd-adjustment = Einstellungsebene „{ $name }“ ({ $kind }) wird nicht angewendet
msg-import-warn-psd-text = Text „{ $name }“: Absatz-/Zeichenformate nur vereinfacht übernommen (A)
msg-import-warn-psd-lab = Lab-Farbmodus: Werte werden ohne Umrechnung übernommen
msg-import-warn-psd-cmyk = CMYK-PSD: naive Umrechnung (das Original rechnet über die PSL-Bibliothek mit Profil, 12 §7a T7)
msg-import-warn-svg-pattern = SVG-Muster werden nicht übernommen
msg-import-warn-svg-gradient-stroke = SVG-Verlaufsstriche werden durch ihre erste Farbe ersetzt (A)
msg-import-warn-svg-gradient-repeat = SVG-Verläufe mit Wiederholung werden ohne Wiederholung übernommen
msg-import-warn-svg-gradient-focus = SVG-Verläufe mit verschobenem Brennpunkt werden zentriert übernommen (A)
msg-import-warn-svg-image = eingebettete SVG-Bilder dieses Typs werden nicht übernommen
msg-import-warn-svg-clip = SVG-Schnittpfade, Masken und Filter werden nicht übernommen
msg-import-warn-svg-text = SVG-Text auf Pfad wird als einfacher Text übernommen
msg-import-warn-ai-text = AI-Text auf Pfad wird als einfacher Text übernommen
msg-import-warn-ai-raster = platzierte Bilder/Raster werden nicht übernommen
msg-import-warn-eps-preview = EPS ohne AI-Daten: nur die eingebettete TIFF-Vorschau wurde übernommen (kein PostScript-Interpreter)
# <<< UE-3 Import-Hinweise

# >>> UE-3 Detailtexte
# Detailtexte in KI-, Export- und Importfehlern (fp-ai, fp-export, fp-io), UE-3
ai-err-d-birefnet-size = BiRefNet braucht ein Quadrat mit Kantenlänge Vielfaches von 32 (erhalten { $size })
ai-err-d-download-size = Größe { $done } statt { $size } Byte
ai-err-d-thread = Hintergrund-Thread: { $error }
ai-err-d-job-cancelled = Hintergrund-Auftrag abgebrochen
ai-err-d-result-taken = Ergebnis bereits abgeholt
ai-err-d-empty-region = Bildausschnitt ist leer
ai-err-d-no-sam = kein SAM-Modell
ai-err-d-point-or-box = Objektauswahl braucht einen Klickpunkt oder einen Rahmen
ai-err-d-no-birefnet = kein BiRefNet-Modell
ai-err-d-no-pixel-selection = keine Pixelauswahl
ai-err-d-sam-size = unbekannte SAM-2-Größe ({ $n } Blöcke)
ai-err-d-sam-input = SAM 2 braucht ein Bild mit 1024 × 1024
ai-err-d-no-point-no-box = kein Punkt und kein Rahmen
ai-err-d-header = Kopf: { $error }
ai-err-d-data-outside = Daten von { $name } außerhalb der Datei
ai-err-d-dtype = Datentyp { $dtype } von { $name } nicht unterstützt
ai-err-d-size-mismatch = Größe von { $name } passt nicht
ai-err-d-weight-missing = Gewicht { $name } fehlt
ai-err-d-not-matrix = { $name } ist keine Matrix
ai-err-d-not-square = { $name } ist keine quadratische Faltung
err-export-d-no-visible-states = keine sichtbaren Status
err-export-d-page-missing = Seite { $page } fehlt
err-export-d-no-slices = Es wurden keine gültigen Segmente zum Exportieren gefunden.
err-export-d-ico-count = ICO braucht 1…255 Bilder
err-export-d-webp-no-frame = animiertes WebP ohne Einzelbild
err-export-d-webp-config = WebP-Konfiguration
err-export-d-webp-anim = animiertes WebP: { $error }
err-export-d-apng-no-frame = APNG ohne Einzelbild
err-export-d-apng-first-frame = erstes APNG-Einzelbild muss die Leinwand abdecken
err-export-d-pdf-no-page = PDF ohne Seite
err-export-d-psd-background = Hintergrund
err-import-d-av1 = AV1-Dekoder: { $error }
# <<< UE-3 Detailtexte

# >>> UE-3 Kurzhilfe
# Eingebaute Kurzhilfe (Hilfe-Menü ohne MANUAL.md), UE-3
msg-help-title = Firepixel – Kurzhilfe
msg-help-intro = Die ausführliche Anleitung liegt als `MANUAL.de.md` neben dem Programm bzw. im Ordner `docs`.
msg-help-topic-start = Erste Schritte
msg-help-topic-tools = Werkzeuge
msg-help-topic-menus = Menüs
msg-help-topic-panels = Bedienfelder
msg-help-topic-export = Exportieren
msg-help-topic-automation = Automatisierung (Skripte, Kommandozeile, MCP)
msg-help-topic-support = Fehlerbehebung
msg-help-body-start = Datei ▸ Neu legt ein Dokument an, Datei ▸ Öffnen liest FW-PNG und gängige Bildformate.
msg-help-body-tools = Links das Werkzeugbedienfeld; Tastenkürzel stehen im QuickInfo.
msg-help-body-menus = Alle Befehle sind auch über die Kommandozeile `firepixel-cli` und MCP erreichbar.
msg-help-body-panels = Fenster-Menü blendet Bedienfelder ein und aus.
msg-help-body-export = Datei ▸ Exportieren bzw. Bildvorschau; die Einstellungen kommen aus dem Bedienfeld Optimieren.
msg-help-body-automation = Befehle ▸ Skript ausführen startet `.jsf`-Dateien.
msg-help-body-support = Voreinstellungen liegen in `%APPDATA%\Firepixel\preferences.json`; Löschen setzt sie zurück.
# <<< UE-3 Kurzhilfe

# >>> UE-3 HEIF
# HEIF-Container (fp-io isobmff.rs), UE-3
err-import-heif-container = HEIF-Container: { $detail }
err-import-d-box-short = Box zu kurz
err-import-d-item-no-location = Item ohne Ort
err-import-d-item-outside = Item-Daten außerhalb der Datei
err-import-d-sample-outside = Sample außerhalb der Datei
# <<< UE-3 HEIF

# >>> UE-3 Dateiteile
# Strukturteile in Fehlermeldungen der Dekoder (fp-io BMP/PSD/TGA/WBMP, fp-ai Gewichte), UE-3
err-import-part-header = Kopf
err-import-part-header-truncated = Kopf abgeschnitten
err-import-part-pixels-missing = Pixeldaten fehlen
err-import-part-pixels-truncated = Pixeldaten abgeschnitten
err-import-part-color-mode-data = Farbmodusdaten
err-import-part-resources = Ressourcen
err-import-part-layer-section = Ebenenbereich
err-import-part-layer-count = Ebenenzahl
err-import-part-layer = Ebene
err-import-part-channel = Kanal
err-import-part-mask = Maske
err-import-part-blending-ranges = Mischbereiche
err-import-part-name = Name
err-import-part-channel-data = Kanaldaten
err-import-part-color-table-truncated = Farbtabelle abgeschnitten
err-import-part-rle-truncated = RLE abgeschnitten
err-import-part-width = Breite
err-import-part-height = Höhe
err-import-part-file-too-short = Datei zu kurz
err-import-part-header-not-object = Kopf ist kein Objekt
# <<< UE-3 Dateiteile

# >>> FV-5 Warnungen ohne Ziel
# Warnungen bei immer aktiven Menübefehlen ohne passendes Ziel (Wortlaut des Originals, BEFUNDE EB Z01–Z18), FV-5
msg-warn-combine-paths = Zum Öffnen, Vereinigen, Überschneiden und Zuschneiden müssen zwei oder mehr Pfade ausgewählt werden.
msg-warn-alter-path = Zum Vereinfachen, Erweitern und Einlassen ist mindestens ein Pfad erforderlich.
msg-warn-tween-instances = Wählen Sie zwei oder mehr Instanzen desselben Bibliothekssymbols aus, bevor Sie den Tween-Befehl verwenden.
msg-warn-break-apart = Um die Verknüpfung zwischen einem Bibliothekssymbol und den Instanzen zu entfernen, müssen Sie zuerst ein Symbol bzw. eine Instanz auswählen.
msg-warn-insert-hotspot = Wählen Sie mindestens ein Objekt aus, bevor Sie den Befehl zum Einfügen von Hotspots verwenden.
msg-warn-insert-slice = Wählen Sie mindestens ein Objekt aus, bevor Sie den Befehl zum Einfügen eines Segments verwenden.
msg-warn-select-similar = Nehmen Sie mit dem Auswahlrahmen, dem Lasso oder dem Zauberstab eine Pixelauswahl vor, bevor Sie den Befehl "Ähnliches auswählen" verwenden.
msg-warn-feather = Bevor Sie den Befehl "Weiche Kante" verwenden, nehmen Sie mit dem Auswahlrahmen, dem Lasso oder dem Zauberstab eine Pixelauswahl vor.
msg-warn-select-inverse = Nehmen Sie mit dem Auswahlrahmen, dem Lasso oder dem Zauberstab eine Pixelauswahl vor, bevor Sie den Befehl "Auswahl umkehren" verwenden.
msg-warn-popup-hotspot-or-slice = Verhalten werden über das Verhalten-Bedienfeld auf Hotspot- oder Segmentobjekte angewendet. Hotspot oder Segment erstellen?
msg-warn-insert-behavior = Wählen Sie ein Segment oder einen Hotspot aus, bevor Sie den Befehl „Verhalten einfügen“ verwenden.
msg-warn-choice-hotspot = &Hotspot
msg-warn-choice-slice = &Segment
msg-warn-choice-cancel = &Abbrechen
msg-warn-dont-show = Nicht me&hr anzeigen
# Filter auf Vektorobjekt (TW-21): Rückfrage mit OK/Abbrechen/Nicht mehr anzeigen
msg-warn-filter-vectors = Dieser Vorgang konvertiert Vektoren in Bitmaps.
# Freiform/Umformen/Pfadausbesserung auf einer Grundform (TW-80): Meldung mit OK/Nicht mehr anzeigen
msg-warn-path-tools-primitive = Die Pfadausbesserungs-Werkzeuge können nur für Pfade verwendet werden. Wählen Sie einen Pfad aus, wenn Sie die Pfadausbesserungs-Werkzeuge verwenden möchten.
# Auswahl animieren über den letzten Status hinaus (Q-20): Rückfrage mit OK/Abbrechen/Nicht mehr anzeigen
msg-confirm-animate-add-states = Die Animation dieses Symbols reicht über den letzten Status im Dokument hinaus. Automatisch neue Status hinzufügen?
msg-insert-web-multi-slice = Es sind mehrere Elemente ausgewählt. Ein Segment oder mehrere Segmente erstellen?
msg-insert-web-multi-hotspot = Es sind mehrere Elemente ausgewählt. Einen Hotspot oder mehrere Hotspots erstellen?
btn-insert-web-single = Einzeln
btn-insert-web-multiple = Mehrere
msg-step-jpeg-mask-load = JPEG-Maske laden
msg-step-jpeg-mask-delete = JPEG-Maske löschen
# <<< FV-5 Warnungen ohne Ziel
# Wächter des Testprofils (fp_i18n::profile::check_write, NZ-1)
err-test-profile-write = Ein Testlauf darf nicht ins echte Nutzerprofil schreiben: { $path } (fp_i18n::profile::use_test_profile() oder FP_TEST_PROFILE verwenden, AGENTEN_REGELN §4a)

# T-9: Ebene statusübergreifend freigeben (re 40 §2b)
dlg-layer-share-confirm = Durch die Freigabe von Ebene "{ $name }" werden alle Objekte dieser Ebene in anderen Status als dem aktuellen gelöscht. Möchten Sie fortfahren?
dlg-layer-unshare-ask = Möchten Sie freigegebene Daten aus "{ $name }" in den aktuellen Status oder in alle Status kopieren?
dlg-layer-unshare-current = Aktuell
dlg-layer-unshare-all = Alle
msg-step-layer-options = Ebenenoptionen

# BA-104 Commands menu
err-symbol-script-no-instance = Wählen Sie zuerst eine Instanz eines Grafiksymbols aus.
err-symbol-script-not-graphic = Symbolskripte gibt es nur für Grafiksymbole (nicht für Schaltflächen oder Animationen).
err-screen-capture = Der Bildschirm konnte nicht aufgenommen werden: { $detail }
err-screen-capture-status = Das Aufnahmeprogramm endete mit { $status }.
err-screen-capture-no-tool = Kein Aufnahmeprogramm gefunden (grim, import oder gnome-screenshot).
