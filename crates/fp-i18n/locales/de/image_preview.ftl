# Bildvorschau (Datei ▸ Bildvorschau…, crates\fp-ui\src\dialogs\image_preview\, re\17-bildvorschau.md) – zuständig: BV-<n>
# Präfix image-preview-; die Beschriftungen der linken Spalte stehen weiter in dialoge.ftl (dlg-preview-*).

# Kopfzeile je Feld
image-preview-preview = Vorschau
image-preview-preview-tip = Vorschau der aktuellen Einstellungen
image-preview-saved-settings = Gespeicherte Einstellungen:
image-preview-saved-settings-tip = Gespeicherte Einstellungen
image-preview-save-current-tip = Aktuelle Einstellungen speichern
image-preview-quality = Qualität { $quality }
image-preview-quality-smoothing = Qualität { $quality } ({ $smoothing })
image-preview-colors = { $n ->
    [one] { $n } Farbe
   *[other] { $n } Farben
}
image-preview-millions = Millionen
image-preview-millions-alpha = Millionen+Alpha
# Größe und Ladezeit: Zahlen kommen fertig (Punkt, K = 1000 Byte)
image-preview-size-bytes = { $n } Byte
image-preview-size-line = { $size }  { $seconds } s bei 56 KBit/s
image-preview-busy = wird berechnet …
image-preview-pane = Vorschaufeld { $n }

# Werkzeugleiste
image-preview-tool-pointer = Zeiger
image-preview-tool-export-area = Exportbereich
image-preview-tool-zoom = Größer/kleiner zoomen
image-preview-zoom-tip = Vergrößerung festlegen
image-preview-view-1 = 1 Vorschaufenster
image-preview-view-2 = 2 Vorschaufenster
image-preview-view-4 = 4 Vorschaufenster
image-preview-first-frame = Gehe zu erstem Status
image-preview-play = Abspielen / Stoppen
image-preview-last-frame = Gehe zu letztem Status
image-preview-current-frame = Aktueller Status
image-preview-prev-frame = Vorheriger Status
image-preview-next-frame = Nächster Status

# Dialog „Vorgabenname“ („+“ im Feld)
image-preview-preset-name-title = Vorgabenname
image-preview-preset-name-label = Name:

# Registerkarte „Optionen“ (re\17 §2, Paket P2)
image-preview-opt-format = Format:
image-preview-opt-palette = Palette:
image-preview-opt-quality = Qualität:
image-preview-opt-loss = Verlust:
image-preview-opt-dither = Rastern:
image-preview-opt-smoothing = Glättung:
image-preview-opt-smoothing-none = Ohne Glätten
image-preview-opt-smoothing-max = 8 Maximale Glättung
image-preview-opt-palette-websnap = WebSnap-Adaptiv
image-preview-opt-matte = Hintergrund:
image-preview-opt-progressive = Progressive Browseranzeige
image-preview-opt-sharpen = Farbränder scharfzeichnen
image-preview-opt-remove-unused = Nicht verwendete Farben entfernen
image-preview-opt-interlaced = Browseranzeige mit Zeilensprungverfahren
image-preview-tip-format = Exportdateiformat
image-preview-tip-options = Optionen für diese Registerkarte
image-preview-tip-palette = Palettenindex
image-preview-tip-quality = JPEG-Qualität
image-preview-tip-loss = Verlustreiche GIF-Komprimierung
image-preview-tip-colors = Maximale Anzahl von Farben
image-preview-tip-actual-colors = Tatsächliche Anzahl von Farben
image-preview-tip-dither = Bild rastern
image-preview-tip-dither-amount = Rastermenge
image-preview-tip-smoothing = Glättungsstufe
image-preview-tip-progressive = Progressiv
image-preview-tip-sharpen = Präzise Ränder nicht komprimieren
image-preview-tip-edit-color = Farbe bearbeiten
image-preview-tip-transparent = Transparent
image-preview-tip-snap-web = Websicher machen
image-preview-tip-lock = Farbe sperren
image-preview-tip-add = Farbe hinzufügen
image-preview-tip-delete = Farbe löschen
image-preview-tip-pipette-set = Farbe für Transparenz wählen
image-preview-tip-pipette-add = Farbe zu Transparenz hinzufügen
image-preview-tip-pipette-remove = Farbe aus Transparenz entfernen
image-preview-tip-transparency = Transparenztyp wählen
image-preview-tip-matte = Transparenz-Hintergrundfarbe
image-preview-tip-remove-unused = Bild für kleinere Dateien optimieren
image-preview-tip-interlaced = Zeilensprung-Bild
image-preview-tip-optimize-size = Assistent für Dateigrößenoptimierung

# Farbtabelle, Statuszeile, Palettenmenü (§2.5, §2.7)
image-preview-color-table = Farbtabelle
image-preview-color-status = R:{ $r } G:{ $g } B:{ $b } Hex:{ $hex } Index:{ $index }
image-preview-color-status-alpha = R:{ $r } G:{ $g } B:{ $b } A:{ $a } Hex:{ $hex } Index:{ $index }
image-preview-flag-locked = Gesperrt
image-preview-flag-edited = Modifiziert
image-preview-flag-websafe = Websicher
image-preview-flag-transparent = Transparent
image-preview-menu-add-color = Farbe hinzufügen…
image-preview-menu-edit-color = Farbe bearbeiten…
image-preview-menu-delete-color = Farbe löschen
image-preview-menu-replace-color = Paletteneintrag ersetzen…
image-preview-menu-snap-web = Websicher machen
image-preview-menu-transparent = Transparent
image-preview-menu-lock = Farbe sperren
image-preview-menu-unlock-all = Alle Farben freigeben
image-preview-menu-sort-luminance = Nach Leuchtkraft sortieren
image-preview-menu-sort-popularity = Nach Häufigkeit sortieren
image-preview-menu-unsorted = Nicht sortiert
image-preview-menu-swatch-feedback = Farbfeld-Rückmeldung anzeigen
image-preview-menu-remove-edit = Bearbeitung entfernen
image-preview-menu-remove-all-edits = Alle Bearbeitungen entfernen
image-preview-menu-load-palette = Palette laden…
image-preview-menu-save-palette = Palette speichern…
image-preview-palette-error = Palette: { $error }
image-preview-palette-empty = Die Datei enthält keine Farben.

# Dialog „Farbe“ (Doppelklick, Farbe bearbeiten/hinzufügen)
image-preview-color-title = Farbe
image-preview-color-add-title = Farbe hinzufügen
image-preview-color-hex = Hex:

# Dialog „Für Dateigröße optimieren“ (§2.8)
image-preview-size-title = Für Dateigröße optimieren
image-preview-size-target = Zielgröße:
image-preview-size-unit = k
image-preview-size-too-small = Die resultierende Datei wäre zu klein. Die ursprünglichen Werte wurden eingegeben.

# Registerkarte „Datei“ (§3)
image-preview-file-scale = Skalieren
image-preview-file-percent = %:
image-preview-file-width = B:
image-preview-file-height = H:
image-preview-file-x = X:
image-preview-file-y = Y:
image-preview-file-pixels = Pixel
image-preview-file-constrain = Proportionen beibehalten
image-preview-file-export-area = Exportbereich
image-preview-tip-scale-percent = Bild beim Exportieren skalieren
image-preview-tip-scale-width = Auf diese Breite skalieren
image-preview-tip-scale-height = Auf diese Höhe skalieren
image-preview-tip-constrain = Bildgröße proportional festlegen
image-preview-tip-export-area = Einen Bildbereich exportieren
image-preview-tip-area-x = Anfangspunkt X-Koordinate
image-preview-tip-area-y = Anfangspunkt Y-Koordinate
image-preview-tip-area-width = Breite des Exportbereichs
image-preview-tip-area-height = Höhe des Exportbereichs

# Registerkarte „Animation“ (§4)
image-preview-disposal-unspecified = Nicht angegeben
image-preview-disposal-none = Ohne
image-preview-disposal-background = Hintergrund wiederherstellen
image-preview-disposal-previous = Vorheriges wiederherstellen
image-preview-disposal-letter-unspecified = N
image-preview-disposal-letter-none = O
image-preview-disposal-letter-background = H
image-preview-disposal-letter-previous = V
image-preview-anim-delay-first = { $delay } /100 Sek.
image-preview-anim-runtime = Laufzeit: { $seconds } Sek.
image-preview-anim-export-frame = Status { $n } exportieren
image-preview-anim-play-once = Einmal abspielen
image-preview-anim-loop = Schleife
image-preview-anim-forever = Endlos
image-preview-anim-crop-each = Jeden Status zuschneiden
image-preview-anim-save-differences = Unterschiede zwischen den Status speichern
image-preview-tip-disposal = Löschmethode
image-preview-tip-frame-delay = Statusverzögerung
image-preview-tip-runtime = Gesamte Animationsverzögerungszeit
image-preview-tip-repeat = Anzahl der Wiederholungen
image-preview-tip-crop-each = Auf geänderte Pixel zuschneiden
image-preview-tip-save-differences = Nur geänderte Pixel speichern
