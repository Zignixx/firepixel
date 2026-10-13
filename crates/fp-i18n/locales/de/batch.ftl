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
      firepixel-cli batch --steps --json
    Exit-Code: 0 alles fertig, 1 mindestens ein Fehler, 2 Aufruf falsch, 3 abgebrochen.
batch-cli-arg-recipe = Rezept-Datei (.fpbatch.json) oder JSON-Text
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
