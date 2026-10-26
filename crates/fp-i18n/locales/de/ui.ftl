# Oberfläche (Shell): Anwendungsleiste, Arbeitsbereiche, Dokumentreiter, Ansichtsleiste,
# Fußleiste, Docking, Datei-Dialoge, Statusmeldungen der App-Schicht. Zuständig: I18N-SHELL.

## Anwendungsleiste
ui-appbar-browse = Durchsuchen (Öffnen)
ui-appbar-hand-tool = Hand-Werkzeug (H)
ui-appbar-zoom-tool = Zoom-Werkzeug (Z)
ui-zoom = Vergrößerung
ui-window-minimize = Minimieren
ui-window-maximize = Maximieren
ui-window-restore = Wiederherstellen
ui-window-close = Schließen

## Arbeitsbereiche
ui-workspace-switcher = Einen Arbeitsbereich auswählen ({ $name })
ui-workspace-save-current = Aktuelles speichern...
ui-workspace-default = Standardarbeitsbereich
ui-workspace-expanded = Erweiterter Modus
ui-workspace-iconic = Symbolmodus
ui-workspace-iconic-names = Symbolmodus mit Bedienfeldnamen
ui-workspace-save-failed = Arbeitsbereich konnte nicht gespeichert werden: { $error }

## Designs und Werkzeuggruppen
ui-theme-normal = Normal
ui-theme-dark = Dunkel
ui-theme-oled = OLED
ui-toolgroup-select = Auswählen
ui-toolgroup-bitmap = Bitmap
ui-toolgroup-vector = Vektor
ui-toolgroup-web = Web
ui-toolgroup-colors = Farben
ui-toolgroup-view = Ansicht

## Dokumentreiter
ui-tab-merge-all = Alles hier zusammenführen
ui-tab-close = Schließen
ui-tab-close-all = Alle schließen
ui-tab-move-to-new-window = In neues Fenster verschieben
ui-tab-new-document = Neues Dokument
ui-tab-open-document = Dokument öffnen
ui-tab-more = Weitere Dokumente

## Ansichtsleiste
ui-view-original = Original
    .tooltip = Bearbeitbares Original
ui-view-preview = Vorschau
    .tooltip = Bildvorschau
ui-view-2up = 2-fach
    .tooltip = Zweifach-Vorschau
ui-view-4up = 4-fach
    .tooltip = Vierfach-Vorschau
ui-page-default-name = Seite { $n }
ui-page-nav = Seitenauswahl ändern
ui-symbol-edit-back = Zurück
    .tooltip = Symbolbearbeitung beenden und zum Dokument zurückkehren
# Ansicht ▸ QuickInfos: Marke mit den Dokumentkoordinaten am Mauszeiger über der Leinwand
ui-canvas-pointer-tag = x:{ $x } y:{ $y }
ui-canvas-size-tag = w:{ $w } h:{ $h }

## Vorschau (2-fach/4-fach)
ui-preview-original = Original
ui-preview-caption = { $label }  { $size }  { $secs } s bei 56 kbit/s
ui-preview-colors = { $n } Farben
ui-preview-failed = Vorschau nicht möglich: { $error }

## Fußleiste: Auswahlinfo, Vergrößerung, Status-Steuerung
ui-sel-bitmap-mode = Bitmapmodus
# FB-31 (Firepixel extension): pausing a running bitmap stroke with P
ui-stroke-pause-hint = P auf der Tastatur hält den aktuellen Pinselstrich an.
# FB-69 (Firepixel extension): Alt held during a running or paused stroke
ui-stroke-subtract = Abzugsmodus (Alt): Der Pinsel löscht nur, was dieser Strich gemalt hat. Alt loslassen malt wieder.
ui-stroke-subtract-paused = Strich angehalten, Abzugsmodus (Alt): Ziehen löscht nur, was dieser Strich gemalt hat. Alt loslassen malt wieder, Enter wendet den Strich an.
ui-stroke-paused = Strich angehalten. Maustaste loslassen oder Stift abheben ist jetzt sicher. Klick auf die Leinwand setzt den Strich fort, Enter wendet ihn an.
ui-sel-document = { $format } (Dokument)
ui-sel-objects = { $n } Objekte
ui-sel-object = Objekt
ui-sel-bitmap = Bitmap
ui-sel-rectangle = Rechteck
ui-sel-path = Pfad
ui-sel-group = Gruppieren
ui-sel-compound = Zusammengesetzte Form
ui-sel-text = Text
ui-sel-symbol = Symbol
ui-sel-slice = Segment
ui-zoom-fit-selection = Auswahl anpassen
ui-zoom-fit-all = Alles anpassen
ui-state-first = Erster Status
ui-state-play = Wiedergeben/Stoppen
ui-state-stop = Wiedergabe stoppen
ui-state-last = Letzter Status
ui-state-prev = Vorheriger Status
ui-state-next = Nächster Status
ui-exit-bitmap-mode = Bitmapmodus beenden
ui-export-area-set = Exportbereich festgelegt.

## Docking
ui-dock-panel-menu = Bedienfeldmenü
ui-dock-expand = Bedienfelder erweitern
ui-dock-collapse-icons = Auf Symbole minimieren

## Datei-Dialoge (Titel und Filter; Formatnamen bleiben)
ui-filter-all-readable = Alle lesbaren Dateien
ui-filter-fw-png = FW-PNG (*.png)
ui-filter-flattened-png = Reduziertes PNG (*.png)
ui-filter-all-files = Alle Dateien (*.*)
ui-filter-html = HTML-Dateien (*.htm)
ui-filter-scripts = Skripte (*.jsf, *.js)
ui-filter-programs = Programme (*.exe)
ui-file-open-title = Öffnen
ui-file-import-title = Importieren
ui-file-update-html-title = HTML-Datei suchen:
ui-file-new-from-template-title = Neu von Vorlage
ui-file-run-script-title = Skript ausführen
ui-import-place-hint = Klicken, um die Datei einzufügen, oder ein Rechteck aufziehen, um sie zu skalieren. Esc bricht ab.
ui-panel-collapse = Bedienfeld einklappen
ui-panel-expand = Bedienfeld ausklappen
ui-file-primary-browser-title = Primärbrowser festlegen
ui-file-secondary-browser-title = Sekundärbrowser festlegen
ui-file-save-as-title = Speichern unter
ui-file-save-template-title = Als Vorlage speichern

## Meldungen der App-Schicht (Öffnen, Speichern, Importieren, Zwischenablage, KI-Steuerkanal)
ui-file-saved-copy = Als Kopie gespeichert: { $path }
ui-file-save-failed =
    Die Datei konnte nicht gespeichert werden:
    { $error }
ui-file-open-failed =
    „{ $path }“ konnte nicht geöffnet werden:
    { $error }
ui-file-import-failed =
    Importieren nicht möglich:
    { $error }
ui-revert-confirm = Letzte gespeicherte Version von "{ $name }" wiederherstellen?
ui-not-implemented = „{ $label }“ ist noch nicht umgesetzt.
ui-script-failed = Skript konnte nicht ausgeführt werden.
ui-script-save-as = Skript speichern
ui-script-needs-document = Dieser Befehl ist nur möglich, wenn ein Dokument aktiv ist.
ui-script-error = Es ist ein Fehler aufgetreten.
ui-preview-created = Vorschau erstellt: { $path }
ui-color-copied = Farbe { $hex } kopiert
ui-warnings-reset = Warnmeldungen zurückgesetzt
ui-clip-no-document = kein Dokument geöffnet
ui-mcp-off = KI-Steuerkanal aus
ui-mcp-starting = KI-Steuerkanal wird gestartet (Adresse und Token in %LOCALAPPDATA%\Firepixel\mcp-bridge.json)
ui-mcp-stopped = KI-Steuerkanal beendet
ui-mcp-active = KI-Steuerkanal aktiv: { $url } (Token in { $file })
ui-mcp-failed = KI-Steuerkanal konnte nicht starten: { $error }

## Verlaufsschritte, die die Oberfläche selbst anlegt
ui-history-paste = Einfügen
ui-history-paste-objects = Objekte einfügen
ui-history-import = Importieren
ui-history-import-files = Dateien importieren

## Kommandozeile (firepixel.exe --help)
ui-cli-help =
    Firepixel Studio – Grafik- und Webdesign-Programm

    Aufruf: firepixel [Optionen] [Datei …]

    Optionen:
      --design=normal|dark|oled     Design nur für diese Sitzung
      --mcp[=PORT]                  KI-Steuerkanal starten (MCP über HTTP, nur 127.0.0.1, mit Token)
      --batch [REZEPT] [DATEI …]    Batch-Studio öffnen: REZEPT ist eine .fpbatch.json oder der Name
                                    einer Vorlage (z. B. web-webp, logos-to-svg, video-to-gif);
                                    die folgenden Dateien und Ordner kommen in die Liste
      --help, -h                    diese Hilfe anzeigen

    Dateien werden beim Start geöffnet.
    Beispiel: firepixel --batch web-webp C:\Fotos
    Ohne Fenster: firepixel-cli batch --recipe <rezept> … (firepixel-cli batch --help)

## Kontextmenü der Eingabefelder (Q-26)
fieldmenu-undo = Rückgängig
fieldmenu-cut = Ausschneiden
fieldmenu-copy = Kopieren
fieldmenu-paste = Einfügen
fieldmenu-delete = Löschen
fieldmenu-select-all = Alles auswählen

## Programmmenü unter macOS (app_menu.rs)
ui-macmenu-about = Über Firepixel Studio
ui-macmenu-services = Dienste
ui-macmenu-hide = Firepixel Studio ausblenden
ui-macmenu-hide-others = Andere ausblenden
ui-macmenu-show-all = Alle einblenden
ui-macmenu-quit = Firepixel Studio beenden
ui-split-close-pane = Diesen Bereich schließen (Fenster ▸ Zusätzliches Fenster schließen)
ui-split-pane-page = Seite dieses Bereichs; ein Klick in den Bereich wählt ihn für Werkzeuge und Bedienfelder
