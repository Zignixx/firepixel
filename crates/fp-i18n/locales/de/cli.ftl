# Kommandozeile firepixel-cli: Hilfe (`cli-<befehl>`, `.examples`, `cli-<befehl>-arg-<option>`, `.value`), Meldungen, Sprachprüfung – UE-3
cli-root = Firepixel – Kommandozeile (FW-PNG bearbeiten, rendern, Skripte, MCP-Server für KI-Agenten)
    .examples = 
        Beispiele:
          firepixel-cli commands --filter align
          firepixel-cli info entwurf.fw.png --json
          firepixel-cli run entwurf.fw.png --cmd select.all --cmd modify.align.left --out ausgerichtet.fw.png
          firepixel-cli run --new 400x300 --cmd draw.rectangle --params '{"{"}"rect":[20,20,200,120],"fillColor":"#3366ff"{"}"}' --out neu.fw.png
          firepixel-cli render entwurf.fw.png --zoom 0.5 --out vorschau.png
          firepixel-cli batch --actions aktionen.json --in eingang --out ausgang
          firepixel-cli script makro.jsf entwurf.fw.png --out ergebnis.fw.png
          firepixel-cli mcp                      (MCP über stdio, z. B. für Claude Code)
          firepixel-cli mcp --http --port 47800  (MCP über HTTP auf 127.0.0.1 mit Token)
          firepixel-cli mcp --bridge             (stdio-MCP zur laufenden Oberfläche, firepixel.exe --mcp)

        Befehlsnamen: <menü>.<eintrag> in camelCase (file.save, modify.transform.flipHorizontal, filter.blur.gaussian)
        sowie draw.*, element.*, layer.*, page.*, state.*, select.*. Parameter: firepixel-cli commands --filter <name> --json.
cli-run = Befehle auf eine Datei anwenden und speichern
    .examples = 
        Beispiele:
          firepixel-cli run in.fw.png --cmd select.all --cmd modify.group --out out.fw.png
          firepixel-cli run in.fw.png --cmd filter.blur.gaussian --params '{"{"}"radius":2{"}"}' --out out.fw.png
          firepixel-cli run --new 200x100 --cmd draw.text --params '{"{"}"text":"Hallo","anchor":[10,60],"size":32{"}"}' --out text.png
          firepixel-cli run in.fw.png --steps schritte.json --out out.fw.png

        --params gehört zum vorangehenden --cmd. --out mit Endung .png (ohne .fw) schreibt ein flaches PNG,
        sonst eine FW-PNG. --steps nimmt eine JSON-Liste [{"{"}"cmd": …, "params": {"{"}…{"}"}{"}"}, …] (Datei oder Text).
cli-run-arg-file = Eingabedatei (.fw.png oder .png); entfällt mit --new
    .value = FILE
cli-run-arg-new = Neues Dokument BREITExHÖHE[:#farbe] statt einer Datei, z. B. 800x600 oder 64x64:#00000000
cli-run-arg-cmds = Befehlsname (mehrfach möglich)
cli-run-arg-params = JSON-Parameter für das vorangehende --cmd
cli-run-arg-steps = Schrittliste als JSON-Datei oder JSON-Text
    .value = JSON|DATEI
cli-run-arg-group = Alle Befehle als ein Rückgängig-Schritt
cli-run-arg-out = Zieldatei (Vorgabe: Eingabedatei überschreiben nur mit --in-place)
    .value = DATEI
cli-run-arg-in-place = Eingabedatei überschreiben
cli-batch = Dieselben Aktionen auf alle Dateien eines Ordners anwenden
    .examples = 
        Beispiel aktionen.json:
          {"["} {"{"}"cmd": "select.all"{"}"},
            {"{"}"cmd": "modify.canvas.imageSize", "params": {"{"}"percent": 50{"}"}{"}"},
            {"{"}"cmd": "filter.sharpen.sharpen"{"}"} ]

          firepixel-cli batch --actions aktionen.json --in eingang --out ausgang
          firepixel-cli batch --actions '[{"{"}"cmd":"modify.transform.flipHorizontal"{"}"}]' --in a --out b --format png
cli-batch-arg-actions = Aktionen als JSON-Datei oder JSON-Text
    .value = JSON|DATEI
cli-batch-arg-input = Eingangsordner
    .value = ORDNER
cli-batch-arg-output = Ausgangsordner (wird angelegt)
    .value = ORDNER
cli-batch-arg-format = Ausgabeformat: fwpng (Vorgabe, editierbar), png, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng
cli-batch-arg-filter = Nur Dateien, deren Name diesen Text enthält
cli-render = Seite, Ausschnitt oder Element als PNG rendern (offscreen)
    .examples = 
        Beispiele:
          firepixel-cli render datei.fw.png --out seite.png
          firepixel-cli render datei.fw.png --page 1 --state 2 --zoom 2 --out s2.png
          firepixel-cli render datei.fw.png --rect 0,0,100,100 --zoom 4 --out ausschnitt.png
          firepixel-cli render datei.fw.png --element 17 --out element.png
cli-render-arg-rect = Ausschnitt l,t,r,b in Dokumentpixeln
cli-render-arg-element = Nur dieses Element (ID aus `info`)
cli-info = Dokumentbaum anzeigen (Seiten, Status, Ebenen, Elemente mit IDs)
    .examples = 
        Beispiele:
          firepixel-cli info datei.fw.png
          firepixel-cli info datei.fw.png --json --depth 1
          firepixel-cli info datei.fw.png --json --all-pages --types text
cli-info-arg-types = Nur diese Elementtypen (kommagetrennt: text,path,rectangle,bitmap,group,instance …)
cli-info-arg-custom-data = customData ausgeben
cli-commands = Befehlskatalog anzeigen
    .examples = 
        Beispiele:
          firepixel-cli commands
          firepixel-cli commands --category modify.align
          firepixel-cli commands --filter weich --json
cli-commands-arg-filter = Suchtext in Name, Beschriftung, Statustext, Menü
cli-commands-arg-category = Namenspräfix (draw, modify, filter …)
cli-commands-arg-all = Auch Platzhalter ohne Wirkung
cli-convert = Datei umwandeln (Format aus der Endung)
    .examples = 
        Beispiele:
          firepixel-cli convert entwurf.fw.png entwurf.png     (flaches PNG32)
          firepixel-cli convert entwurf.fw.png entwurf.jpg     (JPEG über die Optimierung wie im Original)
          firepixel-cli convert ebenen.psd ebenen.fw.png       (PSD mit Ebenen als FW-PNG)
          firepixel-cli convert foto.png foto.webp
        Lesen: .fw.png .png .psd .gif .jpg .bmp .tif .tga .webp .jxl .qoi .ico .svg .ai .eps
        Schreiben: .fw.png .png .gif .jpg .webp .avif .bmp .tif .qoi .ico .apng
cli-script = JSF-JavaScript (.jsf) ausführen
    .examples = 
        Beispiele:
          firepixel-cli script makro.jsf
          firepixel-cli script makro.jsf entwurf.fw.png --out ergebnis.fw.png
          firepixel-cli script --code "fw.createFirepixelDocument({"{"}x:100,y:100{"}"},{"{"}pixelsPerUnit:72,units:'inch'{"}"},'#ffffff').width"
cli-script-arg-script = Skriptdatei
    .value = SCRIPT
cli-script-arg-file = Dokument, das vorher geöffnet wird
    .value = FILE
cli-script-arg-code = Skript als Text statt Datei
cli-script-arg-out = Aktives Dokument danach speichern
    .value = DATEI
cli-script-arg-arg = Wert der globalen Variable FP_ARG
cli-mcp = MCP-Server für KI-Agenten (stdio oder HTTP auf 127.0.0.1 mit Token)
    .examples = 
        Beispiele – Einrichtung in Claude Code (stdio):
          claude mcp add firepixel -- firepixel-cli mcp
        HTTP (Token steht danach in %LOCALAPPDATA%\Firepixel\mcp-http.json):
          firepixel-cli mcp --http --port 47800
          claude mcp add --transport http firepixel http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token>"
        Brücke zur laufenden Oberfläche (firepixel.exe --mcp bzw. FIREPIXEL_MCP=1; Token in %LOCALAPPDATA%\Firepixel\mcp-bridge.json):
          claude mcp add firepixel-gui -- firepixel-cli mcp --bridge
        Doku: docs/AGENTS.md bzw. MCP-Ressource firepixel://docs/agents.md
cli-mcp-arg-http = Streamable HTTP statt stdio
cli-mcp-arg-bridge = stdio-Weiterleitung zur laufenden Firepixel-Oberfläche (Brückenmodus)
cli-mcp-arg-port = Port auf 127.0.0.1 (0 = frei wählen)
cli-mcp-arg-token = Festes Token (mindestens 16 Zeichen; Vorgabe: zufällig)
cli-mcp-arg-token-file = Datei für Adresse und Token (Vorgabe im Benutzerprofil)
cli-language = Sprachdateien prüfen, Vorlage für eine neue Sprache anlegen, Sprachen auflisten
cli-language-check = Sprachdateien prüfen: fehlende/verwaiste Schlüssel, Platzhalter, Syntax, Zugriffstasten
    .examples = 
        Beispiele:
          firepixel-cli language check %APPDATA%\Firepixel\Languages         (alle Sprachen im Ordner)
          firepixel-cli language check Languages\fr                            (eine Sprache, Ordner mit *.ftl)
          firepixel-cli language check fr.ftl                                 (eine Sprache in einer Datei)
          firepixel-cli language check Languages\fr\menu.ftl                   (nur dieser Bereich)
          firepixel-cli language check --builtin                              (die eingebauten Sprachen)

        Maßstab ist das eingebaute Deutsch. Fehler (Exit-Code 1): Syntax, doppelte Schlüssel, Platzhalter,
        die das Programm nicht kennt. Warnungen: fehlende/verwaiste Schlüssel, fehlende Platzhalter oder
        Zugriffstasten (&), doppelte Zugriffstasten in einem Menü bzw. Dialog.
cli-language-check-arg-path = Ordner oder .ftl-Datei
    .value = PFAD
cli-language-check-arg-builtin = Die eingebauten Sprachen prüfen
cli-language-check-arg-short = Fehlende Schlüssel nicht einzeln auflisten (nur zählen)
cli-language-check-arg-strict = Warnungen als Fehler werten (Exit-Code 1)
cli-language-template = Vorlage für eine neue Sprache anlegen (Kopie der englischen Dateien)
    .examples = 
        Beispiel:
          firepixel-cli language template fr %APPDATA%\Firepixel\Languages
        legt %APPDATA%\Firepixel\Languages\fr\*.ftl an (englische Texte als Ausgangspunkt).
        Danach rechts vom = übersetzen, language-name setzen, prüfen und Firepixel neu starten
        (oder Ansicht ▸ Sprache ▸ Sprachordner öffnen...).
cli-language-template-arg-code = Sprachcode (z. B. fr, es, pt-BR, ja)
    .value = CODE
cli-language-template-arg-target-dir = Zielordner (darin entsteht der Unterordner <code>)
    .value = ZIELORDNER
cli-language-template-arg-overwrite = Vorhandene Dateien überschreiben
cli-language-template-arg-from = Ausgangssprache der Vorlage (Vorgabe: en)
    .value = VON
cli-language-list = Verfügbare Sprachen (eingebaut und aus den Sprachordnern) mit Vollständigkeit
cli-root-arg-json = Ausgabe als JSON (maschinenlesbar)
cli-root-arg-lang = Sprache der Ausgaben (z. B. de, en; Vorgabe: FIREPIXEL_LANG bzw. Systemsprache)
    .value = CODE
cli-render-arg-out = Zieldatei (PNG)
    .value = DATEI
cli-render-arg-file = Eingabedatei
    .value = FILE
cli-render-arg-page = Seitenindex (Vorgabe: aktuelle Seite)
cli-render-arg-state = Statusindex
cli-render-arg-zoom = Vergrößerung (1 = 100 %)
cli-info-arg-file = Eingabedatei
    .value = FILE
cli-info-arg-depth = Tiefe des Baums (0 = nur Ebenen)
cli-info-arg-all-pages = Alle Seiten ausführlich
cli-info-arg-page = Ausführlich gezeigte Seite
cli-convert-arg-input = Eingabedatei
    .value = INPUT
cli-convert-arg-output = Ausgabedatei (Format aus der Endung)
    .value = OUTPUT
cli-help = Hilfe anzeigen
cli-version = Version anzeigen
cli-usage = Aufruf:
cli-heading-arguments = Argumente
cli-heading-options = Optionen
cli-heading-commands = Befehle
cli-value-command = BEFEHL
cli-error = Fehler: { $message }
cli-err-unknown-argument = unbekannte Option „{ $arg }“
cli-err-unknown-subcommand = unbekannter Befehl „{ $value }“
cli-err-invalid-value = ungültiger Wert „{ $value }“ für { $arg }
cli-err-missing-argument = Pflichtangabe fehlt: { $arg }
cli-err-missing-subcommand = Befehl fehlt
cli-err-conflict = { $arg } lässt sich nicht mit { $other } kombinieren
cli-err-number-of-values = falsche Anzahl Werte für { $arg }
cli-err-invalid-call = ungültiger Aufruf
cli-err-help-hint = Hilfe: firepixel-cli --help bzw. firepixel-cli <befehl> --help
cli-err-json = ungültiges JSON ({ $error }): { $text }
cli-err-new-size = --new erwartet BREITExHÖHE, nicht „{ $spec }“
cli-err-width = Breite „{ $value }“
cli-err-height = Höhe „{ $value }“
cli-err-export-formats = { $error } (Formate: .fw.png, { $formats })
cli-err-input-or-new = Eingabedatei oder --new angeben
cli-err-params-without-cmd = --params ohne vorangehendes --cmd
cli-err-json-list = { $option }: JSON-Liste erwartet
cli-err-step-without-cmd = Schritt ohne "cmd"
cli-msg-run-done = { $n } Befehl(e) ausgeführt{ $saved }
cli-msg-saved = , gespeichert: { $path }
cli-err-action-without-cmd = jede Aktion braucht "cmd"
cli-err-format = --format: fwpng oder { $formats }, nicht „{ $value }“
cli-msg-batch-done = { $ok } von { $n } Dateien verarbeitet → { $dir }
cli-err-rect = --rect l,t,r,b erwartet, nicht „{ $value }“
cli-err-rect-four = --rect braucht vier Zahlen
cli-msg-tree-doc = { $title } – { $n } Seite(n)
cli-msg-quoted = „{ $text }“
cli-msg-tree-page = Seite { $index } „{ $name }“ { $width }×{ $height } px, { $states } Status, Leinwand { $background }
cli-msg-tree-layer = Ebene #{ $id } „{ $name }“ ({ $kind }{ $hidden })
cli-msg-tree-hidden = , ausgeblendet
cli-msg-commands-count = { $n } Befehle (* = Pflichtparameter)
cli-err-script-or-code = Skriptdatei oder --code angeben
cli-err-no-document-to-save = kein Dokument zum Speichern offen
cli-msg-script-result = Ergebnis: { $result }
cli-msg-script-not-implemented = nicht implementiert: { $list }
cli-err-script = Skriptfehler: { $error }
cli-msg-mcp-http-running = Firepixel-MCP (HTTP) läuft auf { $url }
cli-msg-mcp-token = Token: { $token }
cli-msg-mcp-header = Header: Authorization: Bearer { $token }
cli-msg-mcp-token-file = Token-Datei: { $path }
cli-msg-mcp-stop = Beenden mit Strg+C.
cli-msg-mcp-bridge = Firepixel-MCP: Weiterleitung an die laufende Oberfläche ({ $address })
cli-lang-kind-syntax = Syntaxfehler
cli-lang-kind-doppelt = Doppelte Schlüssel
cli-lang-kind-platzhalter-unbekannt = Unbekannte Platzhalter (das Programm übergibt sie nicht)
cli-lang-kind-platzhalter-fehlt = Fehlende Platzhalter (Wert wird nicht angezeigt)
cli-lang-kind-zugriffstaste-konflikt = Zugriffstasten-Konflikte (gleiche &&-Taste im selben Menü/Dialog)
cli-lang-kind-zugriffstaste-fehlt = Zugriffstaste (&&) fehlt
cli-lang-kind-fehlt = Fehlende Schlüssel (Rückfall auf Englisch)
cli-lang-kind-verwaist = Verwaiste Schlüssel (gibt es im Programm nicht)
cli-lang-no-name = ohne language-name
cli-lang-summary = Sprache { $code } ({ $name }): { $keys } Schlüssel, { NUMBER($percent, minimumFractionDigits: 1, maximumFractionDigits: 1) } % übersetzt, { $missing } fehlen – { $files } Datei(en)
cli-lang-result = Ergebnis: { $errors } Fehler, { $warnings } Warnungen
cli-lang-errors-warnings = { $errors } Fehler, { $warnings } Warnungen
cli-lang-path-missing = Pfad angeben (Ordner oder .ftl-Datei) oder --builtin
cli-lang-bad-code = „{ $code }“ ist kein gültiger Sprachcode (Beispiele: fr, es, pt-BR, zh-TW)
cli-lang-no-builtin = keine eingebaute Sprache „{ $code }“
cli-lang-template-header = 
    # Firepixel – Sprache „{ $code }“ (Vorlage aus „{ $from }“, Datei { $file }).
    # Rechts vom = übersetzen; Schlüssel links, Platzhalter {"{"} $name {"}"} und Auswahl-Varianten [one]/[other] beibehalten.
    # & markiert die Zugriffstaste (Alt+Buchstabe), && ist ein wörtliches &. Prüfen: firepixel-cli language check <ordner>
cli-lang-template-skipped = , { $n } vorhandene übersprungen (--overwrite)
cli-lang-template-done = 
    Vorlage für „{ $code }“ in { $dir }: { $n } Datei(en) geschrieben{ $skipped }.
    Nächste Schritte: language-name in common.ftl setzen, übersetzen, dann
      firepixel-cli language check "{ $dir }"
cli-lang-col-code = Code
cli-lang-col-name = Name
cli-lang-col-origin = Herkunft
cli-lang-col-translated = übersetzt
cli-lang-builtin = eingebaut
cli-lang-folder = Sprachordner
cli-lang-pseudo = Testsprache
cli-lang-folders = Sprachordner:
cli-lang-folder-missing = (nicht vorhanden)
cli-check-error = Fehler
cli-check-warning = Warnung
cli-check-syntax = { $kind } – „{ $text }“
cli-check-duplicate = schon in { $file }:{ $line }
cli-check-source-text = Ausgangstext: „{ $text }“
cli-check-allowed = { $vars } (erlaubt: { $allowed })
cli-check-access-missing = Ausgangstext „{ $text }“ hat eine Zugriffstaste (&)
cli-check-access-conflict = Taste „{ $key }“ mehrfach in Gruppe { $group }: { $list }
cli-check-none = keine
cli-check-not-utf8 = { $path }: keine UTF-8-Datei (bitte als UTF-8 speichern)
cli-check-no-path = { $path }: weder Datei noch Ordner
cli-check-no-ftl = { $path }: keine .ftl-Dateien gefunden
