# Batch-Studio: Formular der Schrittkarten, Batch-Einstellungen (ffmpeg), Rückfrage beim Schließen
# (BATCH-11a, docs\BATCH_MODERN.md §2.4, §2.8, §5.5). Auswahlwerte stehen je Paket unter
# batch-value-<id>-<param>-<wert>.

## Formular
batch-form-auto = Automatisch
batch-form-transparent = transparent
batch-form-on = An
batch-form-off = Aus
batch-form-reset = Auf Vorgabe zurücksetzen
batch-form-reset-auto = Auf „Automatisch“ zurücksetzen
batch-form-more-count = { $label } ({ $n })
batch-form-time-hint = z. B. 1500 oder 0:01,5
batch-form-choose = Wählen...
batch-form-no-file = Keine Datei gewählt
batch-form-json-invalid = Kein gültiges JSON – die Änderung wird erst mit gültigem Inhalt übernommen.
batch-form-range = { $label }: bitte eine Zahl von { $min } bis { $max } eingeben.
batch-form-color =
    .tooltip = Farbe wählen (Raster, HEX/RGB, Pipette)

## Batch-Einstellungen
batch-settings-open = Einstellungen...
batch-settings-title = Batch-Einstellungen
batch-settings-section-general = Allgemein
batch-settings-section-video = Videos (ffmpeg)
batch-settings-mode = Modus
batch-settings-ffmpeg = ffmpeg-Datei
batch-settings-ffmpeg-choose = ffmpeg-Datei...
batch-settings-ffmpeg-auto = Automatisch suchen
batch-settings-ffmpeg-recheck = Erneut prüfen
batch-settings-ffmpeg-checking = Wird geprüft ...
batch-settings-ffmpeg-ok = ffmpeg { $version }
batch-settings-ffmpeg-missing = ffmpeg nicht gefunden
batch-settings-ffmpeg-by-override = vom Programm festgelegt
batch-settings-ffmpeg-by-setting = in den Einstellungen gewählt
batch-settings-ffmpeg-by-environment = aus FIREPIXEL_FFMPEG
batch-settings-ffmpeg-by-path = im Systempfad (PATH) gefunden
batch-settings-ffmpeg-by-nextToExe = neben Firepixel gefunden
batch-settings-ffmpeg-hint = Firepixel liefert ffmpeg nicht mit und lädt nichts herunter. Für Videos ffmpeg selbst installieren und hier die Datei ffmpeg.exe wählen – oder den Ordner in den Systempfad aufnehmen. Bilder und Animationen brauchen kein ffmpeg.
batch-settings-ffmpeg-saved = ffmpeg-Datei gespeichert.
batch-settings-ffmpeg-cleared = ffmpeg wird wieder automatisch gesucht.
batch-settings-ffmpeg-filter = Programme
batch-settings-close = Schließen

## Rückfrage beim Schließen während eines Laufs
batch-quit-title = Batch läuft noch
batch-quit-text = Batch läuft noch – abbrechen und schließen?
batch-quit-detail = { $done } von { $total } Dateien sind fertig. Fertige Dateien bleiben erhalten, die übrigen werden nicht verarbeitet.
batch-quit-confirm = Abbrechen und schließen
batch-quit-keep = Weiterlaufen lassen
