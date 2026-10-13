# Moderner Batch-Modus (Batch-Studio, docs\BATCH_MODERN.md) – Kern crates\fp-batch, Befehle batch.*,
# firepixel-cli batch --recipe. Angelegt von BATCH-1; BATCH-2 ff. ergänzen hier bzw. in batch_*.ftl.
# Präfix batch-; Schritte batch-step-<id> (Punkt als -), Parameter batch-param-<id>-<param>.

## Gruppen
batch-group-files = Dateien
batch-group-format = Format
batch-group-geometry = Geometrie
batch-group-color = Farbe und Effekte
batch-group-background = Hintergrund
batch-group-overlay = Überlagerung
batch-group-metadata = Metadaten
batch-group-animation = Animation
batch-group-optimize = Optimieren
batch-group-document = Dokument
batch-group-advanced = Erweitert

## Schritte (BATCH-1)
batch-step-file-copyOnly = Nur kopieren/verschieben
    .description = Kopiert oder verschiebt die Dateien unverändert, ohne sie zu öffnen.
batch-step-rename = Dateiname
    .description = Legt fest, wie die neuen Dateien heißen.
batch-step-output = Speichern als
    .description = Format und Formatoptionen der neuen Dateien.
batch-param-file-copyOnly-action = Aktion
batch-param-rename-mode = Art
batch-param-output-format = Format
batch-param-output-quality = Qualität
batch-summary-file-copy = Dateien kopieren
batch-summary-file-move = Dateien verschieben
## GIF-Optimierer (BATCH-8, steps\gifopt)
batch-step-output-gif = GIF-Optionen
    .description = Kleine GIFs bei sehr guter Qualität: Palette, Dithering, Bilddifferenz, Verlust und Zielgröße.
batch-param-output-gif-preset = Voreinstellung
batch-param-output-gif-quality = Qualität
    .tooltip = Steuert Farben, Dithering-Stärke und Verlust. Die Voreinstellung setzt sie: Am kleinsten 60, Ausgewogen 80, Beste Qualität 100.
batch-param-output-gif-maxKB = Zielgröße
    .tooltip = Größte Dateigröße in KB; 0 = keine Zielgröße. Firepixel senkt dafür Qualität, Bildrate und zuletzt die Größe.
batch-param-output-gif-colors = Höchstens Farben
    .tooltip = Leer = automatisch nach Qualität und Inhalt (Grafik oder Foto).
batch-param-output-gif-palette = Palette
batch-param-output-gif-dither = Dithering
batch-param-output-gif-ditherAmount = Dithering-Stärke
batch-param-output-gif-lossy = Verlust
    .tooltip = Leer = automatisch nach Qualität und Inhalt.
batch-param-output-gif-diffTransparency = Unveränderte Pixel transparent
batch-param-output-gif-diffThreshold = Schwelle „unverändert“
batch-param-output-gif-cropChanged = Nur geänderten Bereich speichern
batch-param-output-gif-mergeSimilar = Fast gleiche Bilder zusammenlegen
batch-param-output-gif-disposal = Entsorgung
batch-param-output-gif-transparency = Transparenz
batch-param-output-gif-splitFrames = Getrennte Änderungen einzeln speichern
    .tooltip = Automatisch = an unter Qualität 95. Getrennte Änderungen eines Bildes werden als einzelne kleine Teilbilder gespeichert.
batch-value-output-gif-preset-smallest = Am kleinsten
batch-value-output-gif-preset-balanced = Ausgewogen
batch-value-output-gif-preset-best = Beste Qualität
batch-value-output-gif-palette-global = Global
batch-value-output-gif-palette-perFrame = Je Bild
batch-value-output-gif-palette-auto = Automatisch
batch-value-output-gif-dither-none = Kein
batch-value-output-gif-dither-ordered = Geordnet
batch-value-output-gif-dither-blueNoise = Blaues Rauschen
batch-value-output-gif-dither-floydSteinberg = Floyd-Steinberg
batch-value-output-gif-dither-temporal = Zeitlich stabil
batch-value-output-gif-disposal-auto = Automatisch
batch-value-output-gif-disposal-keep = Nicht entsorgen
batch-step-anim-stabilize = Ruhiger Hintergrund
    .description = Friert kleine Änderungen über die Zeit ein (weniger Rauschen, kleinere Dateien).
batch-param-anim-stabilize-threshold = Schwelle
batch-summary-anim-stabilize = Schwelle { $threshold }

## Fehler
batch-err-recipe-json = Das Rezept ist kein gültiges JSON: { $error }
batch-err-recipe-not-object = Das Rezept muss ein JSON-Objekt sein.
batch-err-recipe-format = Unbekanntes Rezeptformat „{ $format }“.
batch-err-recipe-newer = Das Rezept hat Version { $version }; diese Firepixel-Version kann bis Version { $supported } lesen.
batch-err-recipe-missing = Kein Rezept angegeben.
batch-err-no-files = Keine Dateien angegeben.
batch-err-conflict-value = Unbekannte Konfliktregel „{ $value }“ (rename, skip, overwrite, ask).
batch-err-regex = Ungültiger regulärer Ausdruck: { $error }
batch-err-format-unavailable = Das Format { $format } kann noch nicht geschrieben werden.
batch-err-unknown-format = Unbekanntes Dateiformat: { $path }
batch-err-empty-file = Die Datei ist leer.
batch-err-empty-page = Das Bild hat keine Pixel.
batch-err-no-frames = Die Animation hat keine Bilder.
batch-err-not-image = Keine Bilddatei: { $path }
batch-err-not-collect = Der Schritt { $id } ist kein Sammel-Schritt.
batch-err-worker-panic = Die Batch-Verarbeitung wurde unerwartet beendet.
err-backend-batch-runtime = die Batch-Verarbeitung (fp-batch)

## Prüfung des Rezepts
batch-issue-unknown-step = Unbekannter Schritt „{ $id }“ (wird übersprungen, Einstellungen bleiben erhalten).
batch-issue-final-step = „{ $id }“ ist eine feste Endkarte und gehört nicht in die Schrittliste.
batch-issue-missing-folder = Bitte einen Ausgabeordner wählen.
batch-issue-unknown-format = Unbekanntes Ausgabeformat „{ $format }“.
batch-issue-copy-only-combined = „Nur kopieren/verschieben“ geht nur ohne Bildschritte und mit „Speichern als: wie Quelle“.
batch-issue-param-type = Ungültiger Wert für „{ $param }“.
batch-issue-param-range = „{ $param }“ muss zwischen { $min } und { $max } liegen.
batch-issue-param-enum = „{ $param }“ muss einer dieser Werte sein: { $values }.
batch-issue-params-not-object = Die Parameter müssen ein JSON-Objekt sein.

## Hinweise zu Namen und Zielen
batch-hint-invalid-chars = Ungültige Zeichen ersetzt
batch-hint-reserved-name = Reservierter Name, „_“ angehängt
batch-hint-truncated = Name gekürzt
batch-hint-empty-name = Leerer Name, „_“ verwendet
batch-hint-leading-dot = Führender Punkt ersetzt
batch-hint-exif-date-missing = EXIF-Datum fehlt, Änderungsdatum verwendet
batch-hint-conflict = Konflikt: Name schon vergeben
batch-hint-overwrite = Vorhandene Datei wird überschrieben
batch-hint-replace-original = Original wird ersetzt (Sicherung in „Original Files“)
batch-hint-backup-forced = Sicherung eingeschaltet, weil das Original ersetzt wird
batch-hint-skipped = Übersprungen
batch-hint-long-path = Pfad länger als 260 Zeichen
batch-hint-format-fallback = Format der Quelle nicht schreibbar, PNG verwendet
batch-hint-same-fallback-png = Das Format { $format } kann nicht geschrieben werden; es wird PNG verwendet.

## Meldungen
batch-msg-cancelled = Abgebrochen.
batch-msg-summary = { $done } fertig, { $skipped } übersprungen, { $failed } Fehler · { $before } → { $after }
batch-msg-cancelled-summary = Abgebrochen: { $done } fertig, { $rest } nicht verarbeitet
batch-msg-dry-run-summary = Trockenlauf: { $planned } von { $total } Dateien würden geschrieben, { $skipped } übersprungen
batch-warn-first-frame-only = { $format } speichert keine Animation; nur das erste Bild wird geschrieben.
batch-warn-not-applicable = „{ $step }“ wirkt nicht auf diese Datei.
batch-warn-preview-skipped = „{ $step }“ wird in der Vorschau nicht gerechnet.
batch-warn-unchanged = Unverändert (gleicher Name am gleichen Ort).

## Befehlsregister (batch.*)
cmd-batch-run = Batch ausführen
    .description = Führt ein Batch-Rezept über eine Dateiliste aus und liefert den Bericht
cmd-batch-plan = Batch planen
    .description = Plant Quelle und Ziel jeder Datei mit Konflikten, ohne zu schreiben
cmd-batch-preview = Batch-Vorschau
    .description = Rechnet die Vorschau einer Datei vorher und nachher
cmd-batch-steps = Batch-Schritte
    .description = Katalog aller Batch-Schritte mit Parametern
cmd-batch-validate = Batch-Rezept prüfen
    .description = Prüft ein Batch-Rezept und meldet Fehler und Warnungen je Schritt
cmdtext-liste-der-dateien = Liste der Dateien
cmdtext-ordner-mit-den-dateien = Ordner mit den Dateien
cmdtext-unterordner-einbeziehen = Unterordner einbeziehen
cmdtext-rezept-objekt-oder-pfad-einer-rezept-datei = Rezept (Objekt) oder Pfad einer Rezept-Datei
cmdtext-nur-planen-nichts-schreiben = nur planen, nichts schreiben
cmdtext-zahl-der-arbeits-threads = Zahl der Arbeits-Threads
cmdtext-ausgabeordner-ueberschreibt-das-ziel-des-rezepts = Ausgabeordner (überschreibt das Ziel des Rezepts)
cmdtext-konfliktregel-ueberschreibt-das-rezept = Konfliktregel (überschreibt das Rezept)
cmdtext-datei = Datei
cmdtext-laengste-seite-der-vorschau-in-pixeln = längste Seite der Vorschau in Pixeln

## Kommandozeile: firepixel-cli batch --recipe / --steps
batch-cli-about = Ein Batch-Rezept auf Dateien und Ordner anwenden (moderner Batch-Modus)
batch-cli-examples =
    Beispiele:
      firepixel-cli batch --recipe web.fpbatch.json --in fotos --recursive
      firepixel-cli batch --recipe web.fpbatch.json a.png b.jpg --out ausgabe --dry-run
      firepixel-cli batch --recipe web-webp --in fotos --out web          (eingebaute Vorlage)
      firepixel-cli batch --recipe video-to-gif clip.mp4 --out gifs       (braucht ffmpeg)
      firepixel-cli batch --templates
      firepixel-cli batch --steps --json
    Nach dem Lauf liegen batch-report.json und batch-report.html (mit Vorschaubildern) im Ausgabeordner.
    Exit-Code: 0 alles fertig, 1 mindestens ein Fehler, 2 Aufruf falsch, 3 abgebrochen.
batch-cli-arg-recipe = Rezept-Datei (.fpbatch.json), JSON-Text oder Name einer eingebauten Vorlage (--templates)
batch-cli-arg-steps = Katalog aller Schritte zeigen
batch-cli-arg-in = Eingangsordner
batch-cli-arg-recursive = Unterordner einbeziehen
batch-cli-arg-files = Dateien oder Ordner
batch-cli-arg-out = Ausgabeordner (überschreibt das Ziel des Rezepts)
batch-cli-arg-conflict = Konfliktregel: rename, skip oder overwrite
batch-cli-arg-dry-run = Nur planen, nichts schreiben
batch-cli-arg-threads = Zahl der Arbeits-Threads
batch-cli-arg-report = Bericht (JSON) in diese Datei schreiben
batch-cli-arg-json = Maschinenlesbare Ausgabe (Fortschritt als NDJSON auf stderr)
batch-cli-arg-lang = Sprache der Ausgaben (de, en …)
batch-cli-err-args = Falscher Aufruf: { $error }
batch-cli-skipped-file = Übersprungen (kein unterstütztes Format): { $path }
batch-cli-progress-done = [{ $n }/{ $total }] { $source } → { $target }
batch-cli-progress-skipped = [{ $n }/{ $total }] { $source }: { $reason }
batch-cli-progress-failed = [{ $n }/{ $total }] { $source }: Fehler: { $error }
batch-cli-report-written = Bericht: { $path }

## Oberfläche: Batch-Studio (BATCH-2, crates\fp-ui\src\batch\)
batch-ui-tab = Batch
    .tooltip = Batch-Verarbeitung: viele Dateien mit einem Rezept bearbeiten
batch-ui-title = Batch-Verarbeitung
batch-ui-subtitle = { $n ->
        [one] 1 Datei
       *[other] { $n } Dateien
    } · { $size } · Rezept: „{ $recipe }“
batch-ui-unsaved = (ungespeichert •)
batch-ui-recipe-untitled = Neues Rezept
batch-ui-mode-beginner = Einfach
batch-ui-mode-pro = Profi
batch-ui-recipes = Rezepte
    .tooltip = Vorlagen, eigene Rezepte öffnen und speichern
batch-ui-menu-templates = Vorlagen
batch-ui-menu-mine = Meine Rezepte
batch-ui-menu-none = Noch keine gespeicherten Rezepte
batch-ui-menu-open = Rezept öffnen...
batch-ui-menu-save = Rezept speichern
batch-ui-menu-save-as = Rezept speichern unter...
batch-ui-menu-new = Leeres Rezept
batch-ui-more = Weitere Befehle
batch-ui-undo = Rückgängig im Rezept
batch-ui-redo = Wiederholen im Rezept
batch-ui-close = Batch schließen
batch-ui-recursive = Unterordner einbeziehen
batch-ui-filter-recipes = Batch-Rezepte
batch-ui-recipe-loaded = Rezept „{ $name }“ geladen
batch-ui-recipe-load-failed = Rezept lässt sich nicht laden: { $error }
batch-ui-recipe-saved = Rezept „{ $name }“ gespeichert
batch-ui-on = an
batch-ui-off = aus
# Karte Dateien
batch-ui-files-title = Dateien
batch-ui-files-count = { $n ->
        [one] 1 Datei
       *[other] { $n } Dateien
    }
batch-ui-files-count-filtered = { $shown } von { $total }
batch-ui-files-add = Dateien...
    .tooltip = Bilder, Animationen oder Videos hinzufügen (Strg+O)
batch-ui-files-add-title = Dateien für die Batch-Verarbeitung
batch-ui-files-add-folder = Ordner...
batch-ui-files-add-folder-rec = Ordner mit Unterordnern...
batch-ui-files-add-open = Offene Dokumente
batch-ui-files-folder-title = Ordner hinzufügen
batch-ui-files-more = Weitere Arten hinzuzufügen
batch-ui-files-no-open-docs = Kein offenes Dokument ist als Datei gespeichert.
batch-ui-files-remove = Entfernen
    .tooltip = Markierte Datei aus der Liste nehmen (Entf); die Datei selbst bleibt unverändert
batch-ui-files-clear = Leeren
batch-ui-files-empty = Bilder, Animationen oder Videos hierher ziehen
batch-ui-files-empty-hint = PNG, JPEG, GIF, WebP, AVIF, BMP, TIFF, QOI, FW-PNG, PSD und mehr – auch ganze Ordner
batch-ui-files-drop = Zum Hinzufügen loslassen
batch-ui-files-duplicates = { $n ->
        [one] 1 Datei war schon in der Liste
       *[other] { $n } Dateien waren schon in der Liste
    }
batch-ui-files-skipped = { $n } übersprungen (kein unterstütztes Format)
batch-ui-files-videos = { $n ->
        [one] 1 Video
       *[other] { $n } Videos
    }
batch-ui-file-enabled = „{ $name }“ verarbeiten
batch-ui-badge-frames = { $n } B.
batch-ui-filter-hint = Filtern...
batch-ui-filter-hint-pro = Filtern... (mit / davor: regulärer Ausdruck)
batch-ui-filter-supported = Alle unterstützten Dateien
batch-ui-filter-all = Alle Arten
batch-ui-filter-images = Bilder
batch-ui-filter-animations = Animationen
batch-ui-filter-videos = Videos
batch-ui-filter-documents = FW-PNG/Dokumente
batch-ui-filter-other = Andere
batch-ui-filter-failed = Nur Dateien mit Fehler
batch-ui-sort = Reihenfolge (bestimmt die Nummerierung)
batch-ui-sort-added = Hinzugefügt
batch-ui-sort-name = Name
batch-ui-sort-date = Datum
batch-ui-sort-size = Größe
batch-ui-sort-pixels = Pixel
# Karte Rezept
batch-ui-recipe-title = Rezept
batch-ui-recipe-steps = { $n ->
        [0] keine Schritte
        [one] 1 Schritt
       *[other] { $n } Schritte
    }
batch-ui-recipe-empty = Noch keine Schritte
batch-ui-recipe-empty-hint = Mit „Schritt hinzufügen“ eine Kette bauen – oder nur umbenennen und als anderes Format speichern.
batch-ui-add-step = Schritt hinzufügen
    .tooltip = Schritt aus dem Katalog wählen
batch-ui-step-enabled = Schritt ein- oder ausschalten
batch-ui-step-menu = Weitere Aktionen
batch-ui-step-drag = Ziehen zum Umsortieren (Alt+Pfeil hoch/runter)
batch-ui-step-duplicate = Duplizieren
batch-ui-step-up = Nach oben
batch-ui-step-down = Nach unten
batch-ui-step-remove = Entfernen
batch-ui-step-more = Mehr Optionen
batch-ui-step-less = Weniger Optionen
batch-ui-step-hidden-pro = { $n ->
        [one] + 1 Profi-Einstellung
       *[other] + { $n } Profi-Einstellungen
    }
batch-ui-step-unknown = Unbekannter Schritt „{ $id }“
batch-ui-step-unknown-hint = Aus einer neueren Version; wird übersprungen, Einstellungen bleiben erhalten
batch-ui-step-no-params = Dieser Schritt hat keine Einstellungen.
batch-ui-end-section = Ausgabe
batch-ui-end-fixed = Feste Endkarte: zeigt, was am Ende geschrieben wird
batch-ui-end-rename = Dateiname
batch-ui-end-output = Speichern als
batch-ui-rename-same = Name bleibt gleich: { $name }
batch-ui-rename-hint = Name der Ausgabedateien
batch-ui-rename-prefix = Davor
batch-ui-rename-suffix = Dahinter
batch-ui-rename-case = Schreibweise
batch-ui-rename-spaces = Leerzeichen → _
batch-ui-rename-find = Suchen (regulärer Ausdruck)
batch-ui-rename-replace = Ersetzen durch
batch-ui-rename-mode-builder = Baukasten
batch-ui-rename-mode-regex = Regulärer Ausdruck
batch-ui-case-keep = unverändert
batch-ui-case-lower = klein
batch-ui-case-upper = GROSS
batch-ui-case-sentence = Erster Buchstabe groß
batch-ui-case-title = Jedes Wort Groß
batch-ui-case-kebab = mit-bindestrich
batch-ui-case-snake = mit_unterstrich
batch-ui-case-camel = binnenMajuskel
batch-ui-output-same = Format wie Quelle
batch-ui-output-quality = Qualität { $q }
batch-ui-choose-file = Datei wählen...
batch-ui-gallery-title = Schritt hinzufügen
batch-ui-gallery-close = Katalog schließen (Esc)
batch-ui-gallery-search = Suchen...
batch-ui-gallery-empty = Kein passender Schritt
batch-ui-gallery-empty-hint = Weitere Schritte stehen im Profi-Modus zur Verfügung.
batch-ui-gallery-empty-hint-pro = Anderen Suchbegriff versuchen.
# Karte Vorschau
batch-ui-preview-title = Vorschau
batch-ui-preview-nav = { $i }/{ $n }
batch-ui-tab-before = Vorher
batch-ui-tab-after = Nachher
batch-ui-tab-compare = Vergleich
batch-ui-tab-plan = Plan
batch-ui-tab-errors = Fehler ({ $n })
batch-ui-preview-empty = Dateien hinzufügen, um die Vorschau zu sehen
batch-ui-preview-pick = Eine Datei in der Liste wählen
batch-ui-preview-computing = Vorschau wird gerechnet...
batch-ui-preview-error = Vorschau nicht möglich
batch-ui-preview-before-info = Vorher { $w }×{ $h } · { $size }
batch-ui-preview-after-info = Nachher { $w }×{ $h } · { $size }
batch-ui-preview-size-pending = wird ermittelt
batch-ui-preview-time = { $s } s
batch-ui-preview-prev = Vorige Datei
batch-ui-preview-next = Nächste Datei
batch-ui-preview-split = Teiler vorher/nachher
batch-ui-zoom = Zoom (Mausrad zoomt, Doppelklick passt ein)
batch-ui-zoom-fit = Einpassen
batch-ui-bg = Hintergrund hinter Transparenz
batch-ui-bg-checker = Schachbrett
batch-ui-bg-white = Weiß
batch-ui-bg-black = Schwarz
# Plan, Fehler, Ende
batch-ui-plan-empty = Noch kein Trockenlauf
batch-ui-plan-empty-hint = „Trockenlauf“ (F5) zeigt Ziele, Konflikte und Hinweise, ohne etwas zu schreiben.
batch-ui-plan-summary = { $n ->
        [one] 1 Datei
       *[other] { $n } Dateien
    } · { $conflicts ->
        [0] keine Konflikte
        [one] 1 Konflikt
       *[other] { $conflicts } Konflikte
    }
batch-ui-plan-source = Quelle
batch-ui-plan-target = Ziel
batch-ui-plan-note = Hinweis
batch-ui-plan-conflict-with = gleicher Name wie Datei { $n }
batch-ui-errors-copy = Kopieren
batch-ui-errors-file = Datei
batch-ui-errors-message = Meldung
batch-ui-done-title = Fertig
batch-ui-done-errors = Fertig mit Fehlern
batch-ui-done-cancelled = Abgebrochen
batch-ui-done-ok = { $n } fertig
batch-ui-done-skipped = { $n } übersprungen
batch-ui-done-failed = { $n } Fehler
batch-ui-done-counts = { $done } fertig · { $skipped } übersprungen · { $failed } Fehler
batch-ui-done-duration = Dauer { $s } s
batch-ui-done-open-folder = Ordner öffnen
batch-ui-done-save-report = Bericht speichern...
batch-ui-done-retry = Fehlgeschlagene erneut
batch-ui-done-new = Neues Rezept
batch-ui-done-close = Schließen
batch-ui-report-saved = Bericht gespeichert: { $name }
batch-ui-failed-notice = { $n ->
        [one] 1 Datei ließ sich nicht verarbeiten
       *[other] { $n } Dateien ließen sich nicht verarbeiten
    }
# Ausgabe-Leiste
batch-ui-target = Ziel
batch-ui-target-new = Neuer Ordner „{ $name }“
batch-ui-target-folder = Eigener Ordner...
batch-ui-target-same = Gleicher Ort wie das Original
batch-ui-target-choose = Ordner...
    .tooltip = Eigenen Ausgabeordner wählen
batch-ui-target-choose-title = Ausgabeordner wählen
batch-ui-conflict = Bei Konflikt
batch-ui-conflict-rename = Neuen Namen wählen
batch-ui-conflict-skip = Überspringen
batch-ui-conflict-overwrite = Überschreiben
batch-ui-conflict-ask = Jedes Mal fragen
batch-ui-keep-structure = Ordnerstruktur beibehalten
batch-ui-backup-keep = Sicherungen behalten
    .tooltip = Originale landen in „Original Files“; mit Häkchen werden ältere Sicherungen nicht überschrieben
batch-ui-dry-run = Trockenlauf
    .tooltip = Ziele, Konflikte und Hinweise zeigen, ohne zu schreiben (F5)
batch-ui-start = { $n ->
        [one] 1 Datei verarbeiten
       *[other] { $n } Dateien verarbeiten
    }
    .tooltip = Verarbeitung starten (Strg+Eingabe)
batch-ui-cancel = Abbrechen
batch-ui-cancelling = Wird abgebrochen...
batch-ui-running = Läuft...
batch-ui-progress = { $done } von { $total }
batch-ui-remaining = noch { $time }
batch-ui-blocked-empty = Keine Dateien in der Liste
batch-ui-blocked-filtered = Keine Datei aktiviert bzw. alle herausgefiltert
batch-ui-blocked-invalid = Das Rezept enthält Fehler:
batch-ui-ask-title = Datei gibt es schon
batch-ui-ask-text = „{ $name }“ ist im Ziel schon vorhanden. Was soll geschehen?
batch-ui-ask-all = Für alle weiteren übernehmen
# Beschriftungen der Endkarte „Speichern als“ und von „Nur kopieren“ (Formular im Studio)
batch-param-output-lossless = Verlustfrei
batch-param-output-bitDepth = Farbtiefe
batch-value-output-bitDepth-auto = automatisch
batch-value-output-bitDepth-8 = 8 Bit (Palette)
batch-value-output-bitDepth-24 = 24 Bit
batch-value-output-bitDepth-32 = 32 Bit (mit Transparenz)
batch-param-output-palette = Palette
batch-param-output-interlace = Zeilensprung
batch-param-output-progressive = Progressiv
batch-param-output-subsampling = Farb-Unterabtastung
batch-value-output-subsampling-auto = automatisch
batch-param-output-background = Hintergrund
batch-param-output-colors = Farben
batch-param-output-dither = Dithering
batch-value-output-dither-none = keins
batch-value-output-dither-ordered = geordnet
batch-value-output-dither-floydSteinberg = Floyd-Steinberg
batch-value-output-dither-blueNoise = Blaues Rauschen
batch-param-output-ditherAmount = Dithering-Stärke
batch-param-output-transparency = Transparenz
batch-param-output-speed = Geschwindigkeit
batch-value-file-copyOnly-action-copy = Kopieren
batch-value-file-copyOnly-action-move = Verschieben
