# Batch-Studio: Animation und Video (BATCH-7, docs\BATCH_MODERN.md §3.8, §5.5).
# Präfixe: batch-anim-, batch-step-anim-*, batch-step-video-import, batch-param-anim-*,
# batch-param-video-import-*.

## Schritte
batch-step-video-import = Video lesen
    .description = Liest ein Video über ffmpeg als Bildfolge: Ausschnitt, Bilder je Sekunde und Größe.
batch-step-anim-trim = Kürzen
    .description = Schneidet Anfang und Ende ab oder entfernt einzelne Bilder.
batch-step-anim-speed = Geschwindigkeit
    .description = Spielt die Animation schneller oder langsamer ab.
batch-step-anim-loop = Schleife
    .description = Wie oft die Animation läuft, rückwärts oder vor und zurück.
batch-step-anim-dropFrames = Bilder ausdünnen
    .description = Behält nur jedes n-te Bild oder fasst fast gleiche Bilder zusammen.
batch-step-anim-frameDelay = Bilddauer
    .description = Setzt die Anzeigedauer aller Bilder oder verlängert das erste bzw. letzte Bild.
batch-step-anim-toFrames = Animation → Einzelbilder
    .description = Speichert jedes Bild der Animation als eigene Datei.
batch-step-anim-fromFrames = Einzelbilder → Animation
    .description = Fügt alle Bilder der Liste in ihrer Reihenfolge zu einer Animation zusammen.
batch-step-anim-spriteSheet = Sprite-Sheet
    .description = Legt die Bilder einer Animation in ein Raster oder zerlegt ein Raster in Bilder.

## Parameter
batch-param-video-import-start = Anfang
batch-param-video-import-end = Ende
batch-param-video-import-fps = Bilder je Sekunde
batch-param-video-import-maxFrames = Höchstens Bilder
batch-param-video-import-maxSize = Längste Seite
batch-param-video-import-decoder = Decoder
batch-value-video-import-decoder-auto = Automatisch
batch-value-video-import-decoder-builtin = Eingebaut
batch-value-video-import-decoder-ffmpeg = ffmpeg
batch-param-anim-trim-unit = Angabe in
batch-value-anim-trim-unit-time = Zeit
batch-value-anim-trim-unit-frame = Bildnummer
batch-param-anim-trim-start = Anfang
batch-param-anim-trim-end = Ende
batch-param-anim-trim-removeFrames = Bilder entfernen
batch-param-anim-speed-factor = Faktor
batch-param-anim-speed-fps = Feste Bilder je Sekunde
batch-param-anim-speed-minDelay = Kürzeste Bilddauer
batch-param-anim-loop-mode = Wiederholen
batch-value-anim-loop-mode-infinite = Endlos
batch-value-anim-loop-mode-once = Einmal abspielen
batch-value-anim-loop-mode-count = Anzahl
batch-param-anim-loop-count = Wiederholungen
batch-param-anim-loop-pingPong = Vor und zurück
batch-param-anim-loop-reverse = Rückwärts
batch-param-anim-dropFrames-keepEvery = Jedes n-te Bild behalten
batch-param-anim-dropFrames-targetFps = Ziel-Bilder je Sekunde
batch-param-anim-dropFrames-mergeSimilar = Ähnliche Bilder zusammenfassen
batch-param-anim-dropFrames-threshold = Ähnlich unter
batch-param-anim-frameDelay-mode = Dauer
batch-value-anim-frameDelay-mode-all = Alle gleich
batch-value-anim-frameDelay-mode-keep = Unverändert
batch-param-anim-frameDelay-delay = Dauer je Bild
batch-param-anim-frameDelay-first = Erstes Bild
batch-param-anim-frameDelay-last = Pause am Ende
batch-param-anim-toFrames-pattern = Dateiname
batch-param-anim-fromFrames-delay = Dauer je Bild
batch-param-anim-fromFrames-size = Größe
batch-value-anim-fromFrames-size-first = Wie das erste Bild
batch-value-anim-fromFrames-size-largest = Größtes Bild
batch-value-anim-fromFrames-size-fixed = Fest
batch-param-anim-fromFrames-width = Breite
batch-param-anim-fromFrames-height = Höhe
batch-param-anim-fromFrames-background = Hintergrund
batch-param-anim-fromFrames-loop = Wiederholen
batch-value-anim-fromFrames-loop-infinite = Endlos
batch-value-anim-fromFrames-loop-once = Einmal abspielen
batch-param-anim-spriteSheet-direction = Richtung
batch-value-anim-spriteSheet-direction-toSheet = Animation → Sheet
batch-value-anim-spriteSheet-direction-fromSheet = Sheet → Animation
batch-param-anim-spriteSheet-columns = Spalten
batch-param-anim-spriteSheet-rows = Zeilen
batch-param-anim-spriteSheet-spacing = Abstand
batch-param-anim-spriteSheet-padding = Rand
batch-param-anim-spriteSheet-count = Anzahl Bilder
batch-param-anim-spriteSheet-delay = Dauer je Bild

## Zusammenfassungen
batch-anim-end-of-clip = Ende
batch-anim-summary-video = { $start } bis { $end }, { $fps } Bilder/s
batch-anim-summary-trim-time = { $start } bis { $end }
batch-anim-summary-trim-frames = Bild { $start } bis { $end }
batch-anim-summary-speed = { $factor } ×
batch-anim-summary-speed-fps = { $fps } Bilder/s
batch-anim-summary-drop = jedes { $n }. Bild
batch-anim-summary-drop-fps = { $fps } Bilder/s
batch-anim-summary-delay = { $ms } ms je Bild
batch-anim-loop-infinite = Endlos
batch-anim-loop-once = Einmal
batch-anim-loop-count = { $count } Wiederholungen

## Hinweise und Fehler
batch-anim-err-empty = Danach bleibt kein Bild übrig.
batch-anim-err-sheet-grid = Das Raster passt nicht in das Bild (zu viele Spalten/Zeilen oder zu großer Rand).
batch-anim-err-ffmpeg-missing = ffmpeg nicht gefunden – Pfad in den Einstellungen wählen.
batch-anim-ffmpeg-found = ffmpeg { $version } gefunden: { $path }
batch-anim-err-ffmpeg-start = ffmpeg ließ sich nicht starten ({ $path }): { $error }
batch-anim-err-ffmpeg-timeout = ffmpeg antwortet nicht ({ $path }).
batch-anim-err-ffmpeg-not-ffmpeg = Die gewählte Datei ist kein ffmpeg: { $path }
batch-anim-err-ffmpeg-pipe = Die Verbindung zu ffmpeg ist abgebrochen.
batch-anim-err-video-unreadable = Video nicht lesbar: { $path } ({ $detail })
batch-anim-err-video-no-frames = Das Video liefert im gewählten Ausschnitt keine Bilder: { $path }
batch-anim-err-no-profile = Kein Einstellungsordner verfügbar.
batch-anim-warn-not-video = „Video lesen“ wirkt nur auf Videos.
batch-anim-warn-no-builtin = Ein eingebauter Video-Decoder ist noch nicht verfügbar, ffmpeg wird benutzt.
batch-anim-issue-time = Zeitangabe bei „{ $param }“ nicht lesbar (z. B. 1500, 2.5s oder 0:02.500).
batch-anim-issue-end-before-start = Das Ende liegt vor dem Anfang.
batch-anim-issue-ranges = Bildbereich nicht lesbar: „{ $part }“ (z. B. 1-3, 10, 20-).

## Zeitleiste der Vorschau
batch-anim-timeline-title = Zeitleiste
batch-anim-timeline-start = Anfang
batch-anim-timeline-end = Ende
batch-anim-timeline-kept = { $kept } von { $total } Bildern, { $duration }
batch-anim-timeline-frame = Bild { $n }: { $ms } ms
batch-anim-timeline-remove = Bild entfernen
batch-anim-timeline-restore = Bild behalten
batch-anim-timeline-reset = Zurücksetzen
batch-anim-timeline-hint = Griffe ziehen zum Kürzen, Bild anklicken zum Entfernen.
