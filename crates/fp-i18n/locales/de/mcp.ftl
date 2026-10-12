# Antworten des MCP-Servers an KI-Agenten (fp-mcp) – Agent AU
msg-mcp-legend = params: * = Pflicht; available = im aktuellen Zustand ausführbar
err-mcp-param-missing = Parameter { $param } fehlt
err-mcp-unknown-command = unbekannter Befehl „{ $name }“; ähnlich: { $similar }
err-mcp-unknown-tool = unbekanntes Werkzeug „{ $name }“
msg-mcp-categories-hint = list_commands mit {"{"}"category":"draw"{"}"} oder {"{"}"filter":"…"{"}"} aufrufen; Parameter mit describe_command

# >>> UE-3 Werkzeuge
# Werkzeuge des MCP-Servers: Titel, Beschreibung (.description) und Parameter (mcp-tool-<werkzeug>-<parameter>), UE-3
mcp-doc-arg = Dokument-ID aus get_document/documents; Vorgabe: aktives Dokument. Wird gesetzt, wird es zum aktiven Dokument.
mcp-tool-list_commands = Befehle auflisten
    .description = Listet Befehle des Firepixel-Befehlsregisters (gleiche Befehle wie Menüs, Werkzeuge und Bedienfelder der Oberfläche). Namen folgen dem Schema <menü>.<eintrag>[.<untereintrag>] in camelCase, z. B. file.open, edit.duplicate, modify.align.left, modify.transform.flipHorizontal, filter.blur.gaussian; dazu Werkzeug-/Bedienfeldbefehle wie draw.rectangle, draw.text, element.setFill, layer.add, select.set. Ohne category und filter kommt eine Übersicht der Kategorien mit Anzahl. Beispiele: {"{"}"category":"draw"{"}"} · {"{"}"filter":"align"{"}"} · {"{"}"category":"modify.transform","available_only":true{"}"}. Danach describe_command für Parameter aufrufen.
mcp-tool-list_commands-category = Namenspräfix (file, edit, view, select, modify, text, commands, filter, window, help, draw, element, layer, page, state, path, bitmap, color, tool, history, web, guide, transform) oder deutscher Menüname (Ändern, Filter …)
mcp-tool-list_commands-filter = Suchtext in Name, Beschriftung, Statustext und JS-Namen (Groß-/Kleinschreibung egal)
mcp-tool-list_commands-include_unimplemented = auch Menü-Platzhalter ohne Wirkung zeigen
mcp-tool-list_commands-available_only = nur Befehle, die im aktuellen Zustand ausführbar sind (Auswahl, Dokument …)
mcp-tool-describe_command = Befehl beschreiben
    .description = Vollständige Beschreibung eines Befehls: deutsche Beschriftung, Menüpfad, JSON-Schema der Parameter (Typen, Pflicht, Vorgaben, Wertebereiche), Verfügbarkeits-Prädikate (z. B. „mindestens 2 Objekte ausgewählt“) mit aktuellem Erfüllt-Status, Tastenkürzel im aktiven Set, JS-Entsprechungen und ein Aufrufbeispiel. Akzeptiert auch JS-Namen (addNewRectangle) und meldet bei Tippfehlern ähnliche Namen. Beispiel: {"{"}"name":"modify.align.left"{"}"}
mcp-tool-describe_command-name = Befehlsname, z. B. draw.rectangle
mcp-tool-run_command = Befehl ausführen
    .description = Führt einen Befehl aus (wie Menüklick/Werkzeug in der Oberfläche) und legt einen Rückgängig-Schritt an. Liefert den Rückgabewert (z. B. {"{"}"id": 17{"}"} für neue Objekte), die neue Auswahl mit Grenzen, Verlaufszähler und Dokumentgeneration. Die meisten Befehle wirken auf die aktuelle Auswahl – vorher select verwenden oder `ids` übergeben, wo der Befehl das anbietet. Koordinaten sind Dokumentpixel; Rechtecke [links, oben, rechts, unten] oder {"{"}left, top, right, bottom{"}"}; Farben "#rrggbb" oder "#rrggbbaa". Mit `steps` laufen mehrere Befehle als EIN Rückgängig-Schritt (bei Fehler wird alles zurückgenommen). Beispiele: {"{"}"name":"draw.rectangle","params":{"{"}"rect":[20,20,220,120],"fillColor":"#3366ff","strokeWidth":0{"}"}{"}"} · {"{"}"name":"draw.text","params":{"{"}"text":"Hallo","anchor":[30,80],"size":24,"color":"#000000"{"}"}{"}"} · {"{"}"name":"modify.align.centerHorizontal"{"}"} · {"{"}"name":"filter.blur.gaussian","params":{"{"}"radius":3{"}"}{"}"} · {"{"}"steps":[{"{"}"name":"select.all"{"}"},{"{"}"name":"modify.group"{"}"}]{"}"}
mcp-tool-run_command-name = Befehlsname (list_commands/describe_command)
mcp-tool-run_command-params = Parameter nach dem Schema aus describe_command
mcp-tool-run_command-steps = mehrere Befehle als ein Rückgängig-Schritt
mcp-tool-run_command-label = Verlaufsbeschriftung für steps
mcp-tool-get_document = Dokumentbaum lesen
    .description = Liest den Dokumentbaum als JSON: Seiten (Größe, Leinwandfarbe, Status), Ebenen in Original-Reihenfolge unten→oben (Web-Ebene zuletzt) mit IDs, Elemente oben→unten mit stabiler ID, Typ (rectangle, path, text, bitmap, group, maskGroup, autoShape, compoundShape, instance, slice, hotspot), Name, Grenzen {"{"}left, top, right, bottom, width, height{"}"}, Deckkraft (%), Mischmodus, Füllung/Strich, Text/Schrift, Effekte, Symbol; dazu Auswahl, aktive Ebene, Verlaufszähler und die Liste offener Dokumente. Große Dokumente mit depth, types, name, selected_only oder page eingrenzen. Beispiele: {"{"}{"}"} · {"{"}"depth":1{"}"} · {"{"}"types":["text"]{"}"} · {"{"}"selected_only":true{"}"} · {"{"}"all_pages":true,"depth":0{"}"}
mcp-tool-get_document-page = ausführlich gezeigte Seite (Vorgabe: aktuelle)
mcp-tool-get_document-state = Status (Frame), dessen Inhalt gezeigt wird
mcp-tool-get_document-depth = 0 = nur Ebenen, 1 = Elemente der Ebenen, 2+ = auch Gruppeninhalt
mcp-tool-get_document-types = nur diese Elementtypen
mcp-tool-get_document-name = nur Elemente, deren Name den Text enthält
mcp-tool-select = Auswählen
    .description = Setzt die Objektauswahl, auf die die meisten Befehle wirken. Genau eine Art angeben: ids (Element-IDs aus get_document), all, none, point (oberstes Objekt unter dem Punkt) oder rect (Objekte vollständig im Rechteck). mode: replace (Vorgabe), add, subtract, intersect, toggle. Liefert die neue Auswahl mit Typ, Name und Grenzen. Beispiele: {"{"}"ids":[12,15]{"}"} · {"{"}"all":true{"}"} · {"{"}"point":[50,40]{"}"} · {"{"}"rect":[0,0,200,100],"mode":"add"{"}"} · {"{"}"none":true{"}"}
mcp-tool-select-point = {"["}x, y] oder {"{"}x, y{"}"}
mcp-tool-select-rect = {"["}l, t, r, b] oder {"{"}left, top, right, bottom{"}"}
mcp-tool-select-deep = bei point in Gruppen absteigen
mcp-tool-render = Rendern (Bild)
    .description = Rendert offscreen (ohne Fenster) eine Seite, einen Ausschnitt oder ein einzelnes Element als PNG-Bild und gibt es als Bildinhalt zurück – zum Prüfen des Ergebnisses nach Änderungen. zoom 2 = 200 % (Pixel scharf), 0.5 = verkleinerte Vorschau. Große Ausgaben werden automatisch verkleinert (max_pixels). Beispiele: {"{"}{"}"} · {"{"}"zoom":0.5{"}"} · {"{"}"rect":[0,0,100,100],"zoom":4{"}"} · {"{"}"element":17{"}"} · {"{"}"page":1,"state":2{"}"}
mcp-tool-render-rect = Ausschnitt [l, t, r, b] in Dokumentpixeln
mcp-tool-render-element = nur dieses Element (transparenter Grund)
mcp-tool-open = Datei öffnen
    .description = Öffnet eine Datei als neues aktives Dokument: FW-PNG (.fw.png bzw. PNG mit Original-Daten, editierbar), PNG, PSD (Ebenen), GIF, JPEG, BMP, TIFF, TGA, WebP, JPEG XL, QOI, ICO, SVG, AI/EPS. Fremde Formate werden als unbenanntes Dokument geöffnet (Speichern mit save {"{"}path{"}"}). Pfade absolut angeben. Liefert die Dokument-ID und eine Kurzfassung. Beispiel: {"{"}"path":"C:\\Bilder\\entwurf.fw.png"{"}"}
mcp-tool-save = Speichern
    .description = Speichert das Dokument als FW-PNG (editierbar, mit flachem Vorschaubild). Ohne path unter dem bisherigen Namen; mit path wie „Speichern unter“. Für flache Bilder export verwenden. Beispiel: {"{"}"path":"C:\\out\\ergebnis.fw.png"{"}"}
mcp-tool-export = Exportieren
    .description = Exportiert ein flaches Bild, ohne den Dokumentnamen zu ändern. Format aus der Endung oder format: png (PNG32; auch Ausschnitt rect, Element, zoom), gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng (ganze Seite über die Original-Optimierung; quality/colors optional) oder fwpng (editierbare Kopie). Beispiele: {"{"}"path":"C:\\out\\seite.png"{"}"} · {"{"}"path":"C:\\out\\logo.png","element":17,"zoom":2{"}"} · {"{"}"path":"C:\\out\\foto.jpg","quality":85{"}"} · {"{"}"path":"C:\\out\\icon.gif","colors":32{"}"}
mcp-tool-export-format = png, fwpng, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng (Vorgabe: Endung)
mcp-tool-export-quality = JPEG/WebP/AVIF-Qualität
mcp-tool-export-colors = Farbzahl für GIF/PNG8
mcp-tool-export-rect = Ausschnitt [l, t, r, b]
mcp-tool-undo = Rückgängig
    .description = Macht die letzten Schritte im aktiven Dokument rückgängig (Vorgabe 1). Beispiel: {"{"}"steps":3{"}"}
mcp-tool-redo = Wiederholen
    .description = Stellt rückgängig gemachte Schritte wieder her (Vorgabe 1). Beispiel: {"{"}{"}"}
mcp-tool-history = Verlauf
    .description = Zeigt den Verlauf des aktiven Dokuments (Beschriftungen, Position, Befehle mit Parametern zum Wiederabspielen) oder springt mit goto zu einer Position (Zahl ausgeführter Schritte). Beispiele: {"{"}{"}"} · {"{"}"goto":2{"}"}
mcp-tool-new_document = Neues Dokument
    .description = Legt ein neues leeres Dokument an und aktiviert es. Beispiele: {"{"}"width":800,"height":600{"}"} · {"{"}"width":64,"height":64,"background":"#00000000"{"}"} (transparent)
mcp-tool-new_document-background = #rrggbb oder #rrggbbaa (Vorgabe weiß)
mcp-tool-new_document-resolution = Pixel/Zoll (Vorgabe 72)
mcp-tool-tool_event = Werkzeug bedienen
    .description = Bedient ein Werkzeug wie ein Mensch mit der Maus: wählt optional das Werkzeug (tool: pointer, rectangle, ellipse, polygon, line, pen, text, brush, pencil, eraser, marquee, ovalMarquee, lasso, magicWand, paintBucket, gradient, eyedropper, scale, skew, distort, crop, slice, hotspotRect, hand, zoom …) und schickt Zeigerereignisse in Dokumentkoordinaten an den Werkzeug-Automaten. drag ist eine Abkürzung für down → move(s) → up. Modifikatoren wie im Original: shift (Quadrat/45°), alt (von der Mitte/Kopie), space (beim Aufziehen verschieben). Ausgeführte Befehle landen im Verlauf. Beispiele: {"{"}"tool":"rectangle","drag":{"{"}"from":[10,10],"to":[110,60]{"}"}{"}"} · {"{"}"tool":"pen","events":[{"{"}"type":"down","x":10,"y":10{"}"},{"{"}"type":"up","x":10,"y":10{"}"},{"{"}"type":"down","x":90,"y":10{"}"},{"{"}"type":"up","x":90,"y":10{"}"},{"{"}"type":"double","x":90,"y":80{"}"}]{"}"} · {"{"}"tool":"text","events":[{"{"}"type":"down","x":20,"y":40{"}"},{"{"}"type":"up","x":20,"y":40{"}"},{"{"}"type":"text","text":"Hallo"{"}"},{"{"}"type":"key","key":"escape"{"}"}]{"}"}
mcp-tool-tool_event-tool = Werkzeugname (tool.rectangle, rectangle, Rechteck)
mcp-tool-documents = Offene Dokumente
    .description = Listet offene Dokumente (ID, Titel, Pfad, ungespeichert?) und aktiviert oder schließt optional eines. Beispiele: {"{"}{"}"} · {"{"}"activate":2{"}"} · {"{"}"close":3,"discard":true{"}"}
mcp-tool-run_script = JSF-Skript ausführen
    .description = Führt JSF-JavaScript (.jsf, API fw/dom wie das Original) in Firepixel aus – Code direkt oder eine Datei. Liefert Ergebnis, alert()-Texte und Fehler. Nicht umgesetzte API-Member werfen eine Ausnahme „nicht implementiert: <Name>“. Beispiel: {"{"}"code":"var d = fw.getDocumentDOM(); d.addNewRectangle({"{"}left:0,top:0,right:50,bottom:50{"}"}, 0); d.width"{"}"}
mcp-tool-screenshot = Bildschirmfoto der Oberfläche
    .description = Nur im Brückenmodus (laufende Firepixel-Oberfläche): Bildschirmfoto des Hauptfensters als PNG – zeigt Menüs, Bedienfelder, Inspektor, Leinwand mit Auswahlrahmen und offene Dialoge so, wie der Nutzer sie sieht. Für das reine Dokumentbild headless stattdessen render verwenden. Beispiele: {"{"}{"}"} · {"{"}"rect":[0,0,800,600]{"}"} (Ausschnitt in Fensterpunkten) · {"{"}"max_size":1200{"}"}
mcp-tool-screenshot-rect = Ausschnitt [l, t, r, b] in Fensterpunkten
mcp-tool-screenshot-max_size = größere Kante höchstens so viele Pixel (verkleinert)
mcp-tool-ui_state = Zustand der Oberfläche
    .description = Nur im Brückenmodus: Zustand der laufenden Oberfläche – aktives Werkzeug, Farben, Zoom und Bildlauf je Dokument, Fenstergröße, sichtbare Bedienfelder, Design, offene Dialoge, Statuszeile, Lage der Leinwand im Fenster (für screenshot-Koordinaten). Beispiel: {"{"}{"}"}
mcp-tool-ui_action = Aktion der Oberfläche
    .description = Nur im Brückenmodus: löst eine Aktion in der Oberfläche aus wie ein Mensch (inklusive Dialogen, die sich dann im Fenster öffnen): {"{"}"run":"modify.canvas.imageSize"{"}"} (Menübefehl über die Oberfläche, Dialogbefehle öffnen ihren Dialog), {"{"}"zoom":2{"}"} (200 %), {"{"}"fit":true{"}"} (einpassen), {"{"}"panel":"window.layers"{"}"} (Bedienfeld zeigen), {"{"}"tool":"tool.pen"{"}"}, {"{"}"theme":"dunkel"{"}"}, {"{"}"close_dialogs":true{"}"}, {"{"}"status":"Text in der Statuszeile"{"}"}. Für Dokumentänderungen ohne Dialog besser run_command verwenden.
mcp-tool-ui_input = Eingaben einspeisen (Maus, Tastatur, Dateien)
    .description = Nur im Brückenmodus: speist rohe Eingabe-Ereignisse in die laufende Oberfläche ein – auf demselben Weg wie echte Maus/Tastatur (egui-Eingabe vor jedem Frame, genau wie egui-winit sie unter Windows liefert: Strg+C/X/V werden zu Copy/Cut/Paste, Strg+V ohne Text in der Ablage liefert nur das Loslassen, Text nur ohne Strg). Koordinaten in Fensterpunkten (wie ui_find und screenshot-rect). Jeder Schritt läuft in eigenen Frames; die Antwort kommt, wenn alles verarbeitet ist. Ereignisse (`events`, nacheinander): {"{"}"type":"move","pos":[x,y]{"}"} · {"{"}"type":"click","pos":[x,y],"button":"left|right|middle","count":1|2,"modifiers":["ctrl","shift","alt"]{"}"} · {"{"}"type":"down"|"up","button":"left","pos":[x,y]{"}"} · {"{"}"type":"drag","from":[x,y],"to":[x,y],"steps":12,"ms":16,"modifiers":[]{"}"} · {"{"}"type":"wheel","delta":[0,1],"unit":"line|point","pos":[x,y],"modifiers":["ctrl"]{"}"} (positiv = nach oben/vergrößern) · {"{"}"type":"key","key":"C","modifiers":["ctrl"],"action":"tap|press|release"{"}"} (Namen wie egui: A–Z, 0–9, F1, Enter, Escape, Tab, Delete, ArrowLeft …) · {"{"}"type":"modifiers","modifiers":["shift"]{"}"} (gehaltene Umschalttasten setzen, [] löst) · {"{"}"type":"text","text":"abc"{"}"} · {"{"}"type":"type","text":"Hallo"{"}"} (je Zeichen Taste+Text) · {"{"}"type":"copy"|"cut"|"paste","text":"…"{"}"} · {"{"}"type":"drop_files","paths":["C:/x.png"],"pos":[x,y]{"}"} · {"{"}"type":"hover_files","paths":[…]{"}"} · {"{"}"type":"wait","ms":200{"}"} · {"{"}"type":"frames","n":2{"}"} · {"{"}"type":"screenshot","rect":[l,t,r,b]{"}"} · {"{"}"type":"pointer_gone"{"}"} · {"{"}"type":"focus","focused":true{"}"}. Optionen: "screenshot": true oder [l,t,r,b] (Bild nach dem letzten Schritt), "screenshot_each": true (Bild nach jedem Schritt, höchstens 24), "settle_frames": 2, "max_size". Antwort: ausgeführte Schritte, von egui gemeldete Widget-Ereignisse (angeklickt, Fokus), Zeigerposition, offene Dialoge, Statuszeile. Beispiel Strg+C/Strg+V: {"{"}"events":[{"{"}"type":"click","pos":[600,400]{"}"},{"{"}"type":"key","key":"C","modifiers":["ctrl"]{"}"},{"{"}"type":"key","key":"V","modifiers":["ctrl"]{"}"}],"screenshot":true{"}"}
mcp-tool-ui_input-events = Schritte nacheinander (siehe Beschreibung)
mcp-tool-ui_input-screenshot = true oder [l, t, r, b]: Bildschirmfoto nach dem letzten Schritt
mcp-tool-ui_find = Widgets der Oberfläche finden
    .description = Nur im Brückenmodus: sucht sichtbare Bedienelemente und Beschriftungen im aktuellen Fensterbild – Menüeinträge, Knöpfe, Reiter, Bedienfeld-Zeilen, Dialogtitel – über die Barrierefreiheits-Infos von egui (Rolle, Name) und die gezeichneten Texte. Liefert je Treffer Text, Rolle, Rechteck und Mittelpunkt in Fensterpunkten (direkt für ui_input nutzbar), oberste Treffer zuerst. Groß-/Kleinschreibung egal, Teiltreffer erlaubt (exact: true für genau). Beispiele: {"{"}"text":"Datei"{"}"} · {"{"}"text":"OK","role":"button","exact":true{"}"} · {"{"}"role":"menu_item"{"}"} · {"{"}"text":"Ebene 1"{"}"} · {"{"}"region":[0,0,400,60]{"}"}
mcp-tool-ui_find-role = z. B. button, menu_item, check_box, text_input, label, window, tab, slider
mcp-tool-ui_file_dialog = Datei-Dialog der laufenden App
    .description = Nur im Brückenmodus: zeigt, ob in der laufenden App ein Datei-Dialog offen ist (Öffnen, Speichern unter, Importieren, Exportieren, Ordner …) – Art, Titel, Filter, Startordner, vorgeschlagener Name, letzte Ereignisse – und beantwortet ihn. Solange ein Dialog offen ist, zeichnet die App weiter, Leinwand und Menüs sind wie bei einem modalen Dialog gesperrt. Bei aktivem KI-Steuerkanal zeigt die App einen eigenen Dialog statt des nativen (FP_DATEIDIALOG=nativ erzwingt den nativen). Beispiele: {"{"}"action":"status"{"}"} · {"{"}"action":"cancel"{"}"} · {"{"}"action":"choose","path":"C:\\out\\banner.fw.png"{"}"} · {"{"}"action":"choose","paths":["C:\\in\\a.png","C:\\in\\b.png"]{"}"}
mcp-tool-ui_file_dialog-path = Datei oder Ordner für choose
mcp-tool-ui_file_dialog-paths = mehrere Dateien (Öffnen-Dialoge mit Mehrfachauswahl)
# <<< UE-3 Werkzeuge

# >>> UE-3 Meldungen
# Meldungen, Hinweise und Ressourcen des MCP-Servers (UE-3)
err-mcp-hint-unknown-command = {" "}– Namen mit list_commands suchen
err-mcp-hint-invalid-params = {" "}– Schema mit describe_command prüfen
err-mcp-hint-unavailable = {" "}– Voraussetzung herstellen (z. B. select, new_document)
err-mcp-hint-no-document = {" "}– zuerst new_document oder open
err-mcp-doc-not-open-hint = Dokument { $id } ist nicht offen – documents listet die offenen Dokumente
err-mcp-doc-not-open = Dokument { $id } ist nicht offen
err-mcp-step-needs-name = jeder Schritt braucht name
err-mcp-name-or-steps = name oder steps angeben
err-mcp-no-document-hint = kein Dokument offen – new_document oder open
err-mcp-no-document = kein Dokument offen
err-mcp-select-kind = ids, all, none, point oder rect angeben
err-mcp-nothing-to-undo = nichts rückgängig zu machen
err-mcp-nothing-to-redo = nichts zu wiederholen
msg-mcp-history-hint = steps[0..position] sind ausgeführt; history {"{"}"goto": n{"}"} springt; Befehle lassen sich mit run_command {"{"}steps:[…]{"}"} erneut abspielen
err-mcp-rect-expected = rect: [l, t, r, b] oder {"{"}left, top, right, bottom{"}"} erwartet
err-mcp-drag-from-to = drag braucht from und to als [x, y]
msg-mcp-tool-nothing-yet = noch nichts übernommen – Vorgang ggf. mit up/double/key enter abschließen
err-mcp-unsaved = „{ $title }“ hat ungespeicherte Änderungen – save oder discard: true
err-mcp-scripts-off = Skripte sind in diesem Server abgeschaltet
err-mcp-code-or-path = code oder path angeben
msg-mcp-agent-steps = Agent-Schritte
err-mcp-open-failed = { $error } (lesbar: { $formats })
err-mcp-export-failed = { $error } (Formate: fwpng, { $formats })
err-mcp-folder = Ordner { $path }: { $error }
err-mcp-unknown-key = unbekannte Taste „{ $key }“
err-mcp-unknown-event = unbekannter Ereignistyp „{ $kind }“
err-mcp-unknown-canvas-tool = unbekanntes Werkzeug „{ $name }“; verfügbar: { $names }
err-mcp-event = Ereignis { $index }: { $error }
msg-mcp-truncated = nur die ersten { $n } Elemente; mit depth/types/name/page einschränken oder maxElements erhöhen
err-mcp-image-empty = leeres Bild
err-mcp-image-size = Bilddaten passen nicht zur Größe
err-mcp-only-http = nur http://-Adressen: { $url }
err-mcp-port-missing = Port fehlt in { $url }
err-mcp-only-local = nur lokale Adressen erlaubt: { $host }
err-mcp-bad-port = ungültiger Port in { $url }
err-mcp-token-short = Token fehlt oder ist zu kurz
err-mcp-token-file-unreadable = Token-Datei { $path } nicht lesbar ({ $error }) – läuft Firepixel mit eingeschaltetem KI-Steuerkanal (firepixel.exe --mcp bzw. FIREPIXEL_MCP=1)?
err-mcp-token-file = Token-Datei: { $error }
err-mcp-token-file-field = Token-Datei ohne { $field }
err-mcp-no-connection = keine Verbindung zu Firepixel auf 127.0.0.1:{ $port } ({ $error }) – läuft die Oberfläche noch?
err-mcp-bad-reply = ungültige Antwort: { $error }
err-mcp-element-not-on-page = Element { $id } nicht auf Seite { $page }
err-mcp-element-invisible = Element { $id } hat kein sichtbares Aussehen
err-mcp-page-no-canvas = Seite ohne Leinwand
err-mcp-empty-rect = leerer Ausschnitt
err-mcp-random = Zufallsquelle: { $error }
err-mcp-token-min = Token muss mindestens 16 Zeichen haben
err-mcp-http-thread = HTTP-Thread beendet
err-mcp-host-gone = die Sitzung ist nicht mehr erreichbar
err-mcp-io = E/A-Fehler: { $error }
err-mcp-protocol = MCP: { $error }
err-mcp-config = ungültige Einstellung: { $error }
err-mcp-no-ui = nur im Brückenmodus mit laufender Oberfläche verfügbar (Firepixel mit --mcp bzw. FIREPIXEL_MCP=1 starten); headless stattdessen render verwenden
err-mcp-timeout = die Oberfläche hat nicht rechtzeitig geantwortet (Fenster minimiert oder blockiert?)
err-mcp-unknown-resource = unbekannte Ressource { $uri }
msg-mcp-instructions = Firepixel ist ein Grafikprogramm für Bildschirm- und Webgrafik (Vektor + Bitmap, Seiten, Status, Ebenen, Symbole, FW-PNG). Ablauf: new_document oder open → get_document (IDs, Grenzen) → select → run_command (Befehle aus list_commands/describe_command) → render (Bild prüfen) → save/export. Alle Änderungen sind rückgängig machbar (undo/history). Befehlsnamen: <menü>.<eintrag> in camelCase (modify.align.left, filter.blur.gaussian) sowie draw.*, element.*, layer.*, page.*, state.*, select.*. Koordinaten in Dokumentpixeln, Rechtecke [l, t, r, b], Farben #rrggbb[aa]. Ausführliche Doku: Ressourcen firepixel://docs/agents.md, commands.md, document-model.md, examples.md.
msg-mcp-instructions-bridge = Brückenmodus: Du steuerst die laufende Oberfläche, der Nutzer sieht jede Änderung live. screenshot zeigt das ganze Fenster, ui_state den Oberflächenzustand, ui_action löst Oberflächenaktionen aus, ui_find liefert Positionen von Menüeinträgen/Knöpfen/Bedienfeld-Zeilen, ui_input speist echte Maus-/Tastatur-/Datei-Ereignisse ein (wie ein Mensch).
msg-mcp-instructions-headless = Headless-Sitzung (screenshot/ui_state/ui_action/ui_input/ui_find nur im Brückenmodus).
msg-mcp-cat-title = Firepixel-Befehlskatalog
msg-mcp-cat-intro = { $n } Befehle, davon { $implemented } mit Wirkung. Aufruf über MCP `run_command` ({"{"}"name": …, "params": {"{"}…{"}"}{"}"}), CLI `firepixel-cli run --cmd …` oder JSF. `*` = Pflichtparameter. Platzhalter ohne Wirkung sind mit _(nicht umgesetzt)_ markiert.
msg-mcp-cat-unimplemented = nicht umgesetzt
msg-mcp-cat-needs = braucht: { $list }
mcp-res-agents = Anleitung für KI-Agenten
    .description = Einrichtung, typische Abläufe, Befehlsnamen-Schema, Beispiele
mcp-res-commands = Befehlskatalog
    .description = Alle Befehle nach Kategorie mit Parametern, Pflichtangaben und Verfügbarkeit
mcp-res-document-model = Dokumentmodell
    .description = Seiten, Status, Ebenen, Elemente, IDs, Koordinaten, Farben
mcp-res-examples = Beispiele
    .description = Fertige Werkzeugaufrufe für häufige Aufgaben
mcp-res-jsf = JSF-Kompatibilität
    .description = Welche Teile der JSF-JavaScript-API umgesetzt sind
# <<< UE-3 Meldungen

# >>> UE-3 Brücke
# Antworten der laufenden Oberfläche an KI-Agenten (Brückenmodus, fp-ui bridge.rs/inject.rs), UE-3
err-mcp-ui-channel-closed = Steuerkanal beendet
err-mcp-ui-no-screenshot = kein Bildschirmfoto erhalten (Fenster minimiert?)
msg-mcp-ui-done-dialogs = Dialoge geschlossen
err-mcp-ui-unknown-command = unbekannter Befehl „{ $name }“ – list_commands
msg-mcp-ui-done-command = Befehl { $name }
err-mcp-ui-zoom-range = zoom zwischen 0.01 und 64 (1 = 100 %)
msg-mcp-ui-done-fit = eingepasst
msg-mcp-ui-done-panel = Bedienfeld { $name }
err-mcp-ui-unknown-panel = unbekanntes Bedienfeld „{ $name }“ (z. B. window.layers, window.history)
msg-mcp-ui-done-tool = Werkzeug { $name }
err-mcp-ui-unknown-theme = unbekanntes Design „{ $name }“ (normal, dunkel, oled)
msg-mcp-ui-done-theme = Design { $name }
msg-mcp-ui-done-status = Statuszeile
err-mcp-ui-no-action = keine Aktion angegeben (run, zoom, fit, panel, tool, theme, close_dialogs, status)
err-mcp-ui-modifiers = modifiers: Liste wie ["ctrl","shift"]
err-mcp-ui-unknown-modifier = unbekannte Umschalttaste „{ $name }“ (ctrl, shift, alt)
err-mcp-ui-unknown-button = unbekannte Maustaste „{ $name }“ (left, right, middle)
err-mcp-ui-key-missing = key fehlt (z. B. "C", "Enter", "Escape", "F5")
err-mcp-ui-unknown-key = unbekannte Taste „{ $key }“ (Namen wie egui: A, 5, F1, Enter, Escape, Tab, Delete, ArrowLeft, PageUp …)
err-mcp-ui-paths-empty = paths ist leer
err-mcp-ui-events-missing = events fehlt: Liste von Schritten, z. B. [{"{"}"type":"click","pos":[100,200]{"}"}]
err-mcp-ui-screenshot-arg = screenshot: true oder [l, t, r, b]
err-mcp-ui-step-type = Schritt { $n }: type fehlt
err-mcp-ui-step = Schritt { $n } ({ $kind }): { $error }
msg-mcp-ui-pointer = (Zeiger)
msg-mcp-ui-drag = drag { $from } → { $to } ({ $n } Schritte)
err-mcp-ui-delta = delta fehlt ([x, y] oder Zahl; positiv = nach oben)
err-mcp-ui-bad-value = { $name } „{ $value }“ ({ $list })
msg-mcp-ui-drop = drop_files { $n } Datei(en) bei { $pos }
err-mcp-ui-too-many-frames = zu viele Frames (> { $max }); Folge aufteilen
err-mcp-ui-unknown-type = unbekannter Typ „{ $kind }“ ({ $list })
# <<< UE-3 Brücke
