# Firepixel für KI-Agenten

Firepixel ist ein natives Grafikprogramm im Stil des Originals: Vektor- und Bitmapbearbeitung,
Seiten, Status (Frames), Ebenen, Symbole, Live-Effekte, automatische Formen, FW-PNG (`.fw.png`).
Agenten steuern es über **dasselbe Befehlsregister** wie die Oberfläche – über MCP, die
Kommandozeile oder JSF-JavaScript. Jede Änderung landet im Verlauf und ist rückgängig machbar.

Drei Wege zum MCP-Server:

| Weg | Wann | Sitzung |
|---|---|---|
| `firepixel-cli mcp` (stdio) | Automatisierung ohne Fenster | eigene, leere Sitzung je Start |
| `firepixel-cli mcp --http` | mehrere Clients, HTTP | eigene Sitzung je Serverprozess |
| **Brücke zur laufenden App** (`firepixel.exe --mcp` bzw. Menü *Befehle ▸ KI-Steuerkanal (MCP)*) | mit dem Nutzer zusammen arbeiten | **die Dokumente der Oberfläche**, Änderungen sofort sichtbar |

## 1. MCP einrichten

### stdio (headless)

```bash
claude mcp add firepixel -- C:\Pfad\zu\firepixel-cli.exe mcp
```

oder in `.mcp.json` (Projekt) bzw. `~/.claude.json`:

```json
{
  "mcpServers": {
    "firepixel": { "type": "stdio", "command": "C:\\Pfad\\zu\\firepixel-cli.exe", "args": ["mcp"] }
  }
}
```

### Streamable HTTP (headless, nur 127.0.0.1, mit Token)

```bash
firepixel-cli mcp --http --port 47800
```

URL und Token stehen auf stderr und in `%LOCALAPPDATA%\Firepixel\mcp-http.json`
(`{"url": "http://127.0.0.1:47800/mcp", "port": 47800, "token": "…", "header": "Authorization: Bearer …", "pid": …, "mode": "headless"}`).
Die Datei wird beim Beenden gelöscht.

```bash
claude mcp add --transport http firepixel http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token>"
```

### Brücke zur laufenden Oberfläche

1. Firepixel mit KI-Steuerkanal starten: `firepixel.exe --mcp` (fester Port: `--mcp=47811`), oder in
   der laufenden App **Befehle ▸ KI-Steuerkanal (MCP)** einschalten, oder vor dem Start die
   Umgebungsvariable `FIREPIXEL_MCP=1` setzen (Port: `FIREPIXEL_MCP_PORT`). Die Statuszeile zeigt
   „KI-Steuerkanal aktiv: http://127.0.0.1:47800/mcp“. Ist der Port belegt, nimmt Firepixel einen freien.
2. Adresse und Token stehen in **`%LOCALAPPDATA%\Firepixel\mcp-bridge.json`** (gleicher Aufbau wie oben,
   `"mode": "bridge"`; getrennt von `mcp-http.json`, damit ein headless-Server die Brücke nicht verdeckt).
   Beim Schließen der App oder Ausschalten des Kanals wird die Datei gelöscht; das Token ist bei jedem
   Start neu (256 Bit).
3. Verbinden – entweder direkt per HTTP mit dem Token aus der Datei:

   ```bash
   claude mcp add --transport http firepixel-gui http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token aus mcp-bridge.json>"
   ```

   oder **ohne Token-Kopieren** über die stdio-Weiterleitung der CLI, die Adresse und Token selbst liest:

   ```bash
   claude mcp add firepixel-gui -- C:\Pfad\zu\firepixel-cli.exe mcp --bridge
   ```

   (`--token-file <datei>` für eine andere Token-Datei.) Läuft die App nicht, meldet die Weiterleitung
   „Token-Datei … nicht lesbar – läuft Firepixel mit eingeschaltetem KI-Steuerkanal?“.

In der Brücke laufen alle Aufträge im UI-Thread auf derselben Sitzung wie Menüs und Werkzeuge: der
Nutzer sieht jede Änderung, alles steht im Verlauf-Bedienfeld. Zusätzlich gibt es die
Oberflächen-Werkzeuge `screenshot`, `ui_state`, `ui_action`, `ui_find` und `ui_input` (siehe 2 und 5a).
Mit `ui_find` + `ui_input` lässt sich die App vollständig wie von einem Menschen bedienen – auch wenn
Bildschirm und Eingabe der Windows-Sitzung (z. B. RDP ohne Konsole) von außen nicht erreichbar sind.

Sicherheit: Der Server bindet nur an `127.0.0.1`, verwirft Verbindungen von anderen Adressen,
verlangt bei jeder Anfrage das Token (`Authorization: Bearer …` oder `X-Firepixel-Token: …`,
Vergleich in konstanter Zeit), prüft den `Host`-Header (Schutz vor DNS-Rebinding) und führt keine
Shell-Befehle aus. Dateizugriffe gibt es nur über Öffnen, Speichern, Exportieren und JSF-`Files`.

Sprache: Antworten, Befehlsbeschriftungen (`label`, `description` in `list_commands`/`describe_command`)
und Fehlermeldungen kommen standardmäßig auf **Englisch** – unabhängig von der Sprache der Oberfläche,
die dabei nicht umschaltet. Jedes Werkzeug nimmt optional `"language": "de"` (oder einen anderen
Sprachcode); die Vorgabe des Servers lässt sich mit der Umgebungsvariablen `FIREPIXEL_MCP_LANG` ändern.
Befehlsnamen (`file.open` …) und Parameter sind sprachunabhängig. Auch die Ressourcen (diese Anleitung,
Befehlskatalog, Dokumentmodell, Beispiele, JSF-Liste) kommen in der Antwortsprache: Deutsch für `de`,
sonst Englisch (englische Fassung dieser Datei: `docs\AGENTS.en.md`).

Hinweis: Anfragen werden nebenläufig angenommen. Abhängige Aufrufe (z. B. `render` nach
`new_document`) erst nach der Antwort des vorigen senden – MCP-Clients wie Claude tun das von selbst.

## 2. Werkzeuge

| Werkzeug | Zweck | Beispiel |
|---|---|---|
| `list_commands` | Befehle suchen (ohne Argumente: Kategorien) | `{"category": "modify.align"}` · `{"filter": "weich"}` |
| `describe_command` | Schema, Pflichtparameter, Voraussetzungen, Kürzel, Beispiel | `{"name": "draw.rectangle"}` |
| `run_command` | Befehl ausführen (oder `steps` als **ein** Rückgängig-Schritt) | `{"name": "modify.group"}` |
| `get_document` | Dokumentbaum: Seiten, Status, Ebenen, Elemente mit IDs/Grenzen/Attributen | `{"depth": 1}` · `{"types": ["text"]}` |
| `select` | Auswahl setzen (IDs, alles, nichts, Punkt, Rechteck) | `{"ids": [12, 15]}` · `{"all": true}` · `{"point": [20, 20]}` |
| `render` | Seite/Ausschnitt/Element als PNG-Bild (offscreen, ohne Oberfläche) | `{"zoom": 0.5}` · `{"element": 12, "zoom": 2}` |
| `open` / `save` / `export` | Dateien (FW-PNG, PNG, JPEG, GIF, WebP, AVIF, PSD-Import …) | `{"path": "C:\\out\\a.png"}` |
| `undo` / `redo` / `history` | Verlauf | `{"steps": 2}` · `{"goto": 3}` |
| `new_document` | Neues Dokument | `{"width": 800, "height": 600, "background": "#ffffff"}` |
| `documents` | Offene Dokumente, wechseln, schließen | `{"activate": 2}` · `{"close": 3, "discard": true}` |
| `tool_event` | Werkzeug wie ein Mensch bedienen (Zeigerereignisse) | `{"tool": "ellipse", "drag": {"from": [10,10], "to": [90,60]}}` |
| `run_script` | JSF-JavaScript ausführen | `{"code": "fw.getDocumentDOM().width"}` |
| `screenshot` | **nur Brücke:** Bildschirmfoto des App-Fensters (Menüs, Bedienfelder, Dialoge) | `{}` · `{"rect": [0, 0, 800, 600]}` · `{"max_size": 1200}` |
| `ui_state` | **nur Brücke:** Werkzeug, Farben, Zoom/Bildlauf je Dokument, Fenstergröße, sichtbare Bedienfelder (`visible_panels`: nur wirklich gezeichnete; dazu `panels_hidden` und `batch_front`, wenn das Batch-Studio vorn ist), Dialog offen?, Statuszeile | `{}` |
| `ui_action` | **nur Brücke:** wie ein Klick in der Oberfläche – Menübefehl (öffnet ggf. seinen Dialog), Zoom, Einpassen, Bedienfeld, Werkzeug, Design, Dialoge schließen | `{"run": "modify.canvas.imageSize"}` · `{"zoom": 2}` · `{"fit": true}` · `{"panel": "window.layers"}` · `{"tool": "tool.pen"}` · `{"close_dialogs": true}` |
| `ui_find` | **nur Brücke:** sichtbare Bedienelemente/Beschriftungen finden (Menüeinträge, Knöpfe, Reiter, Bedienfeld-Zeilen, Dialog-Titelleisten) – Text, Rolle, Rechteck, Mittelpunkt in Fensterpunkten, oberste zuerst | `{"text": "Datei", "exact": true}` · `{"text": "OK", "role": "button"}` · `{"text": "Titelleiste"}` · `{"region": [0, 0, 400, 60]}` |
| `ui_input` | **nur Brücke:** rohe Maus-/Tastatur-/Datei-Ereignisse einspeisen, genau wie echte Eingaben (siehe 5a) | `{"events": [{"type": "click", "pos": [640, 420]}, {"type": "key", "key": "C", "modifiers": ["ctrl"]}], "screenshot": true}` |

| `ui_file_dialog` | **nur Brücke:** offenen Datei-Dialog (Öffnen, Speichern unter, Importieren, Ordner …) abfragen und beantworten (siehe 5b) | `{}` · `{"action": "choose", "path": "C:\\out\\a.fw.png"}` · `{"action": "choose", "paths": ["a.png", "b.png"]}` · `{"action": "cancel"}` |
| `batch` | **Batch-Verarbeitung:** viele Dateien mit einem Rezept (Vorlage, Datei oder Objekt) – Vorlagen, Schritte, prüfen, planen, Vorschau, ausführen (auch im Hintergrund mit Fortschritt), abbrechen (siehe 5c) | `{"action": "templates"}` · `{"recipe": "logos-to-svg", "folder": "C:\\logos", "out": "C:\\svg"}` · `{"action": "status", "jobId": 1, "wait_ms": 5000}` |

Headless liefern `screenshot`/`ui_state`/`ui_action`/`ui_find`/`ui_input`/`ui_file_dialog` eine Fehlermeldung mit
Hinweis auf die Brücke (dort `render` und `tool_event` benutzen).

Ressourcen (Markdown): `firepixel://docs/agents.md` (diese Datei), `firepixel://docs/commands.md`
(vollständiger Befehlskatalog mit Parametern), `firepixel://docs/document-model.md`,
`firepixel://docs/examples.md`, `firepixel://docs/jsf.md` (Umfang der JSF-API),
`firepixel://docs/batch.md` (Vorlagen und Schritte der Batch-Verarbeitung mit Parametern).

## 3. Befehlsnamen-Schema

* Menübefehle: `<menü>.<eintrag>[.<untereintrag>]` in camelCase nach dem englischen Menü des Originals:
  `file.open`, `file.saveAs`, `edit.duplicate`, `edit.undo`, `view.zoomIn`, `select.all`, `select.inverse`,
  `modify.group`, `modify.ungroup`, `modify.arrange.bringToFront`, `modify.align.left`,
  `modify.align.distributeWidths`, `modify.transform.flipHorizontal`, `modify.transform.numericTransform`,
  `modify.combinePaths.union`, `modify.alterPath.expandStroke`, `modify.canvas.imageSize`,
  `modify.symbol.convertToSymbol`, `modify.mask.groupAsMask`, `text.font`, `filter.blur.gaussian`,
  `filter.adjustColor.hueSaturation`, `filter.sharpen.unsharpMask`, `window.layers` …
* Werkzeug- und Bedienfeldbefehle (mit Parametern, ideal für Agenten):
  * Zeichnen: `draw.rectangle {rect, roundness?, fillColor?, strokeColor?, strokeWidth?, name?}`,
    `draw.ellipse {rect}`, `draw.line {from, to}`, `draw.polygon {center, corner, sides, star?}`,
    `draw.path {contours: [{closed, nodes: [{anchor, in, out}]}]}`, `draw.text {text, anchor | rect, font?, size?, color?}`
  * Attribute: `element.setFill {color | kind: "none", ids?}`, `element.setStroke {color?, width?, placement?}`,
    `element.setOpacity {opacity}`, `element.setBlendMode {mode}`, `element.setName {name}`,
    `element.setVisible`, `element.setLocked`, `element.setBounds {x?, y?, width?, height?}`,
    `element.addEffect {effect: "dropShadow"|"innerShadow"|"bevel"|"glow"|… , params?}`, `element.setGradient {from, to, colors?}`
    (`bevel` mit `params.BevelType` 0 Abgeflachte Kante innen, 1 Abgeflachte Kante außen, 2 Relief erhaben, 3 Relief vertieft;
    Gruppe „Abgeflachte Kante und Relief“)
  * Texturen und Muster (Namen wie in den Inspektorlisten, Dateien aus den Textur- bzw. Musterordnern):
    `element.setTexture {name | image, target?: "fill"|"stroke", amount?, ids?}` (ohne `name` und `image`: Textur entfernen),
    `element.setPattern {name | image, ids?}`, `pi.fill {pattern}` bzw. `pi.fill {texture, textureAmount?}` an der Auswahl
    (ohne Auswahl: Vorgabe des Werkzeugs); unbekannter Name → Fehlermeldung
  * Bitmap: `bitmap.floodFill {point, tolerance?, color?, fillSelection?}` (Farbeimer),
    `bitmap.gradient {from, to, colors?, fillSelection?, tolerance?}` (Verlauf-Werkzeug; `fillSelection`
    Vorgabe `true` = Pixelauswahl bzw. ganze Bitmap füllen, `false` = zusammenhängender Farbbereich am
    Anfangspunkt nach `tolerance` 0…255, Vorgabe 32), `bitmap.paint`, `bitmap.erase`
  * Auswahl: `select.set {ids, mode?}`, `select.inRect {rect}`, `select.pointAt {point}`
  * Struktur: `layer.add {name?}`, `layer.setVisible {layer, visible}`, `layer.setActive {layer}`,
    `page.add`, `page.setCurrent {index}`, `state.add {count?}`, `state.setCurrent {index}`
  * Bewegen/Transformieren: `edit.move {dx, dy}`, `transform.matrix {matrix}`, `path.setContours {id, contours}`,
    `modify.transform.numericTransform {mode: scale|resize|rotate, scaleX?, scaleY?, width?, height?, angle?, constrain?, scaleAttributes?}`
    (`scaleAttributes`, Vorgabe `true`: Strichbreite skaliert mit, wie „Attribute skalieren“ im Dialog),
    `modify.mask.revealAll` / `modify.mask.hideAll` (leere Maske über dem gewählten Objekt)
  * Text und Auswahl: `text.detachFromPath` (Text wird wieder Rahmen fester Breite = Breite vor dem Anfügen,
    mittig über den Pfadgrenzen; der Pfad kommt oben in die Ebene, Auswahl danach [Pfad, Text]),
    `select.similar {tolerance?}` (gleiche Füllung, gleicher Strich, gleiche Deckkraft und gleicher Mischmodus),
    `edit.pasteInside` (danach ist **nichts** ausgewählt, die ID steht im Ergebnis `{"id": …}`),
    `modify.canvas.trimCanvas` (Inhaltsgrenzen geschnitten mit der Leinwand)
  * Web und Vorlagen: `edit.copyHtmlCode {style?, dir?, baseName?, xhtml?}` (`baseName` und `xhtml` gelten nur
    für diesen Export, das Dokument bleibt unverändert; in der Oberfläche führt ein Assistent mit drei Seiten
    dorthin), `file.newFromTemplate` (übernimmt die Exportvorgabe der Vorlage; mitgeliefert sind zehn Vorlagen
    in `%APPDATA%\Firepixel\Templates`, Ordner `Document Presets`, `Grids`, `Mobile`, `Web`, `Wireframes`)
  * Werkzeug wählen: `tool.select {tool: "tool.pen"}`; Farben: `color.set`, `color.swap`, `color.defaults`
  * **Automatische Formen** (41 Formen, eigene Umsetzung; Liste: `autoShape.list`):
    `autoShape.insert {shape, at | rect}` (deutscher oder englischer Name, z. B. `"Stern"`/`"Star"`,
    `"Abgerundetes Rechteck"`, `"Pfeil"`, `"Torte"`, `"Spirale"`, `"Zahnrad"`, `"Kalender"`),
    `autoShape.dragControlPoint {id, index, to, from?, points?, shift?, alt?, ctrl?}` (Steuerpunkt ziehen;
    `from` = `to` ist ein Klick, z. B. Eckenart wechseln), `autoShape.describe {id}` (Parameter mit
    Werten und Steuerpunkten), `autoShape.setParams {id, params: {schlüssel: wert}}`, `autoShape.redraw {id}`.
  * Skripte: `commands.runScript {path | code}`, `commands.run {name}` (gespeicherter Befehl),
    `commands.manageSavedCommands {action: list|save|rename|delete, name?, code?}`.
* Platzhalter, die noch keine Wirkung haben, meldet `describe_command` mit `"implemented": false`.

## 4. Konventionen

* Koordinaten in Dokumentpixeln, Ursprung oben links der Leinwand.
* Rechtecke `[links, oben, rechts, unten]` (auch `{left, top, right, bottom}` oder `{x, y, width, height}`),
  Punkte `[x, y]`, Farben `"#rrggbb"` oder `"#rrggbbaa"`, Deckkraft in Prozent.
* Ebenen in `get_document` stehen **unten → oben** (wie `dom.layers` im Original; die Web-Ebene ist die
  letzte), Elemente einer Ebene **oben → unten**. Automatische Formen haben den Typ `autoShape`.
* Element-IDs bleiben während der Sitzung stabil. Neue Objekte liefern `{"id": …}` im Ergebnis von
  `run_command` und sind danach ausgewählt.
* Die meisten Menübefehle wirken auf die Auswahl. Ist ein Befehl nicht verfügbar, nennt die
  Fehlermeldung den Grund (z. B. „mindestens 2 Objekte auswählen“).
* Wiederholte Aufrufe sind sicher: dieselben Parameter erneut setzen ändert nichts doppelt
  (z. B. `autoShape.setParams` 50× → eine Form, ein Pfad).
* Neue Dokumente heißen „Unbenannt-1.fw.png“ (englische Oberfläche „Untitled-1.fw.png“). In der Oberfläche
  schlägt *Datei ▸ Neu* nach Strg+C die Größe der internen Zwischenablage vor.
* Filter aus Menü oder Kürzel fragen in der Oberfläche vor dem Umwandeln von Vektoren in Bitmaps nach
  („Dieser Vorgang konvertiert Vektoren in Bitmaps.“, „Nicht mehr anzeigen“ = Voreinstellung
  `DisableConfirmFilterVectors`). `run_command` und Skripte mit Parametern fragen nicht.
* Esc ohne laufende Aktion (kein Popup, Menü, Werkzeugzug oder KI-Lauf) hebt in der Oberfläche die
  Objektauswahl auf.
* *Auswahl animieren* fragt in der Oberfläche, wenn die Animation über den letzten Status hinausreicht
  („Automatisch neue Status hinzufügen?“, „Nicht mehr anzeigen“ = Voreinstellung `DisableConfirmAnimateStates`).
  `run_command` fragt nicht: `modify.animation.animateSelection` hat den Parameter `addStates` (Vorgabe
  `true`; `false` legt die Animation ohne neue Status an). Das neue Symbol heißt „Symbol“, die Instanz bleibt
  ohne Namen (ebenso nach `modify.symbol.convertToSymbol`).
* `select.next` / `select.previous` (Strg+Umschalt+Rechts/Links) gehen durch den Auswahlverlauf des Dokuments
  (jede neue, nicht leere Objektauswahl nach einem Befehl ist ein Eintrag, höchstens 100; am Ende keine Wirkung,
  gelöschte Objekte werden übersprungen), nicht zyklisch durch die Objekte.
* Einfügeplatz: `edit.paste` legt kopierte Objekte direkt über das oberste gewählte Objekt (bei Unterauswahl in
  einer Gruppe über die Gruppe), ohne Auswahl oben in die aktive Ebene. `modify.mask.groupAsMask` setzt die
  Gruppe an den Platz des untersten gewählten Objekts; Vereinigen/Überschneiden/Verbinden legen das Ergebnis an
  den Platz des oberen, Öffnen/Zuschneiden an den des unteren, Teilen ganz oben in die Liste.
* `select.convertMarqueeToPath` übernimmt Füllung und Strich aus den aktuellen Werkzeugvorgaben (wie eine neu
  gezeichnete Form). `modify.symbol.tweenInstances` hat die Vorgabe 10 Schritte.
* `modify.symbol.editSymbol` öffnet eine Symbolansicht so groß wie die Seite, Titel „<Dokument> (Symbol)“; sie
  zählt in `documents` weiterhin als eigenes Dokument. In der Oberfläche führt der Knopf „Zurück“ der
  Pfadleiste (`modify.symbol.exitEdit`) zurück.
* Kürzelsätze heißen in der Oberfläche neutral („Bildbearbeitung“, „Vektorzeichnung“, „Klassische
  Vektor-Tasten“). `edit.keyboardShortcuts {"set": …}` und `CurrentShortcutSet` nehmen weiter die alten internen
  Kennungen; gespeicherte Einstellungen und Skripte laufen unverändert.
* **Aktive Unterebene:** `layer.setActive` nimmt auch die ID einer Unterebene; neue Objekte landen dann in ihr.
  `layer.move` zählt die Position in der Elternebene (0 = unten), `layer.duplicate` legt die Kopie direkt
  darüber in dieselbe Elternebene (Ergebnis `{"layer": id}`, die Kopie wird aktiv), `layer.add` setzt die neue
  Ebene über die Ebene der obersten Stufe, `layer.delete` aktiviert danach die Ebene der obersten Stufe.
  „Für Seiten freigeben“ geht für eine Unterebene nicht (Menü grau, per Befehl eine klare Fehlermeldung).
  Im Ebenen-Bedienfeld macht ein Klick auf die Unterebenenzeile sie aktiv.
* Verlaufs-/Mustergriffe: `element.setFillHandles` ist ein Schritt („Verlaufsgriffe verschieben“); in der
  Oberfläche ziehen Zeiger und Hintere auswählen die Griffe. `modify.combinePaths.join` liefert immer ein Ergebnis ohne Namen.
  `modify.flattenSelection` beschneidet die Bitmap auf die sichtbaren Pixel (Alpha > 0).
* Neue Dokumente und neue Seiten haben die Exportvorgabe **PNG 32 Bit**; Dateien ohne Exporteinstellung lesen
  weiter GIF Adaptiv 256. PNG-Export und FW-PNG schreiben `pHYs` = `floor(trunc(dpi) / 0,0254)` (72 dpi → 2834).
* `view.setZoom` hat den optionalen Parameter `center` (Dokumentpunkt in die Fenstermitte).
  `panel.library.newSymbol` nimmt `nineSlice`. Der HTML-Export macht aus Schaltflächen-Instanzen eigene Segmente
  (aktiver Bereich aus der Web-Ebene des Symbols, sonst Instanzgrenzen; nur Status 1 mit Link).
  SVG-Import behält die Stapelfolge von Texten.

## 5. Typische Abläufe

**Neues Motiv bauen und prüfen**

1. `new_document {"width": 600, "height": 300, "background": "#ffffff"}`
2. `run_command {"name": "draw.rectangle", "params": {"rect": [0, 0, 600, 300], "fillColor": "#0f172a", "strokeWidth": 0, "name": "Hintergrund"}}`
3. `run_command {"name": "draw.text", "params": {"text": "Firepixel", "anchor": [40, 170], "size": 64, "color": "#f8fafc"}}`
4. `run_command {"name": "element.addEffect", "params": {"effect": "dropShadow"}}` (wirkt auf die Auswahl = der Text)
5. `render {"zoom": 0.5}` → Bild ansehen, ggf. korrigieren, `undo`
6. `save {"path": "C:\\out\\banner.fw.png"}`, `export {"path": "C:\\out\\banner.png"}`

**Vorhandene Datei bearbeiten**

1. `open {"path": "C:\\in\\entwurf.fw.png"}`
2. `get_document {"depth": 2}` → IDs und Grenzen
3. `select {"ids": [21, 22, 23]}` → `run_command {"name": "modify.align.top"}`
4. `render {"rect": [0, 0, 300, 200], "zoom": 2}` → Detail prüfen
5. `save {}`

**Mehrere Befehle als ein Schritt**

`run_command {"steps": [{"name": "select.all"}, {"name": "modify.group"}, {"name": "modify.transform.rotate90DegreeCw"}], "label": "Drehen"}`

**Automatische Form einfügen und anpassen**

1. `run_command {"name": "autoShape.insert", "params": {"shape": "Stern", "at": [100, 80]}}` → `{"id": 7}`
2. `run_command {"name": "autoShape.describe", "params": {"id": 7}}` → Parameter (`nodes`, `radius0` …) und Steuerpunkte
3. `run_command {"name": "autoShape.setParams", "params": {"id": 7, "params": {"nodes": 8}}}`
4. `run_command {"name": "autoShape.dragControlPoint", "params": {"id": 7, "index": 0, "to": [200, 40]}}`

**Wie ein Mensch zeichnen** (Werkzeug-Automaten, inklusive Original-Modifikatoren)

* Quadrat: `tool_event {"tool": "rectangle", "drag": {"from": [20, 20], "to": [120, 90], "shift": true}}`
* Pfad mit dem Stift: `tool_event {"tool": "pen", "events": [{"type":"down","x":10,"y":10},{"type":"up","x":10,"y":10},{"type":"down","x":120,"y":10},{"type":"move","x":160,"y":60},{"type":"up","x":160,"y":60},{"type":"key","key":"enter"}]}`
* Text tippen: `tool_event {"tool": "text", "events": [{"type":"down","x":30,"y":60},{"type":"up","x":30,"y":60},{"type":"text","text":"Hallo"},{"type":"key","key":"escape"}]}`

**Mit dem Nutzer in der laufenden App** (Brücke)

1. `ui_state {}` → welches Dokument ist offen, welches Werkzeug, Zoom
2. `get_document {"depth": 2}` → IDs; `run_command …` → der Nutzer sieht die Änderung sofort
3. `ui_action {"fit": true}` → Dokument einpassen; `screenshot {}` → so sieht der Nutzer es gerade
4. `ui_action {"run": "filter.blur.gaussian"}` öffnet den Dialog in der App (Nutzer bestätigt);
   ohne Dialog direkt: `run_command {"name": "filter.blur.gaussian", "params": {"radius": 3}}`

**JSF-Skripte wiederverwenden**

`run_script {"path": "C:\\skripte\\makro.jsf"}` – die API (`fw`, `fw.getDocumentDOM()`, `Files` …)
entspricht dem Original (550/565 Methoden, 546/721 Properties; die übrigen sind begründet ausgelassen,
siehe `firepixel://docs/jsf.md`). Zusätzlich `fp.run(name, params)` für jeden Befehl des Registers.
Wie im Original gilt:
* `dom.layers` (und `Frame.layers`) listet die Ebenen **flach**: jede Ebene der obersten Stufe, direkt danach
  ihre Unterebenen (unten → oben, rekursiv). `dom.topLayers` enthält nur die oberste Stufe.
  `frames[n].elements` enthält keine Unterebenen, `elemsandsublayers` schon.
* `pixelRect` von Text ist die gerenderte Tinte. Ohne gezeichnete Glyphe kommt das leere Rechteck
  (±2147483648). Bei Elementen mit Vektormaske ist es der Maskenbereich.
* `dom.guides.locked` liest die wirksame Sperre (*Ansicht ▸ Hilfslinien ▸ Hilfslinien sperren*).
* Alle Ebenen**nummern** zählen flach wie `dom.layers`: Nummer `i` in `currentLayerNum`, `setLayerVisible(i)`,
  `setLayerLocked`, `deleteLayer`, `getParentLayerNum`, `moveSelectionToLayer` … meint `dom.layers[i]`. Eine Unterebene
  kann die aktive Ebene sein: `currentLayerNum = <Unterebene>` aktiviert sie und liest sie zurück;
  `addNewSubLayer(index, name)` legt die Unterebene unter `index` an (verschachtelt), aktiviert sie und gibt den
  Namen zurück; `addNewLayer(name, shared)` gibt den Namen zurück. `duplicateLayer`/`reorderLayer` mit einer
  Unterebene bleiben in deren Elternebene.
* Weitere Skript-Details wie im Original: `addNewHotspot("slice", …)` legt ein Segment an, Form `"oval"` einen
  Kreis-Hotspot; `addNewText(rect, true)` nimmt die Vorgaben des Textwerkzeugs. `antiAliased`/`antiAliasMode`
  lesen und setzen die Textglättung (`"none"` = harte Kante). `fw.setActiveViewScale(scale, center)` setzt Zoom
  und Mittelpunkt. `toSource` gibt `null` in einem Array als Lücke aus (`[1, ]`). `setFillColor`/`setBrushColor`
  ohne Auswahl setzen die Farben des Werkzeugbedienfelds. `filterSelection`/`filterSelectionByName` wandeln
  Vektoren ohne Rückfrage in ein Bitmap um. `convertToAnimSymbol(name, status, …)` legt per Skript keine neuen
  Status an. `importFile` in ein Zielrechteck rundet die Lage. Text am Pfad: `rawLeft/rawTop/rawWidth` = Grenzen
  des Pfads.

**KI-Auswahl** (Objekt, Motiv, Kante – lokale Modelle, keine Netzverbindung beim Rechnen)

Die KI-Befehle erzeugen **Pixelauswahlen** (weiche Kanten möglich) auf Bitmaps oder auf der
sichtbaren Seite. Sie brauchen Modelldateien; fehlt eins, meldet der Befehl „KI-Modell … fehlt“ mit
dem Zielordner. Herunterladen geht nur in der Oberfläche nach Zustimmung des Nutzers
(„Befehle ▸ KI-Modelle verwalten…“, Details `docs\MODELLE.md`) – Agenten laden nichts selbst.

| Befehl | Zweck | Beispiel |
|---|---|---|
| `ai.models` | Modelle mit Aufgabe, Größe, Lizenz, `available`, Pfad; Suchordner und Download-Ziel | `run_command {"name": "ai.models"}` |
| `select.aiObject` | Objekt unter Punkt(en) oder im Rahmen (SAM 2.1). Parameter: `point` oder `points: [[x, y, 1\|0], …]` (1 = Objekt, 0 = Hintergrund), `box`, `mode` (`replace`/`add`/`subtract`/`intersect`), `session` + `refine: true` (dieselbe Auswahl verfeinern), `source: auto\|bitmap\|page`, `region`, `view`, `model: sam2Tiny\|sam2Small`, Feineinstellungen (siehe unten). Ergebnis `{session, iou, objectScore, bounds, source, tuning}` | `{"name": "select.aiObject", "params": {"point": [220, 140]}}` |
| `select.aiSubject` | Hauptmotiv (BiRefNet) als weiche Auswahl. `mode`, `source`, `point` (Bitmap unter dem Punkt), `region`, `model: birefnetLite\|birefnet`, Feineinstellungen (siehe unten). Ergebnis `{session, bounds, source, tuning}` | `{"name": "select.aiSubject", "params": {"source": "page"}}` |
| `select.aiRefineEdge` | Kante der vorhandenen Pixelauswahl verfeinern (Haare, Fell, echte Halbtransparenz). `radius` (0 = automatisch), `output: selection\|mask`, `source`, `model: birefnetLiteMatting\|birefnetHrMatting` – braucht eine Pixelauswahl | `{"name": "select.aiRefineEdge", "params": {"output": "mask"}}` |
| `ai.setModelFolder` | eigenen Modellordner festlegen (leer = Standard) | `{"name": "ai.setModelFolder", "params": {"path": "D:\\modelle"}}` |
| `ai.clearCache` | KI-Zwischenspeicher und geladene Modelle freigeben | `{"name": "ai.clearCache"}` |
| `ai.manageModels` | ohne Oberfläche wie `ai.models`; in der App öffnet es den Verwaltungsdialog | `ui_action {"run": "ai.manageModels"}` |

Feineinstellungen von `select.aiObject` und `select.aiSubject` (ohne Parameter gelten die Werte aus
dem Inspektor des Werkzeugs, sonst die Vorgabe; Begründung in `docs\MODELLE.md`):

| Parameter | Werte | Wirkung |
|---|---|---|
| `tolerance` | 0…100, Vorgabe 50 | 50 = Entscheidung des Modells; höher wählt mehr aus. SAM: Schwelle der Masken-Logits (±10), BiRefNet: Tonwerte der weichen Maske |
| `edge` | `hard` \| `antiAlias` (Vorgabe) \| `feather` | Kante wie beim Zauberstab; `feather` mit `feather: <px>` (ohne `edge` macht `feather` > 0 wie bisher eine weiche Kante) |
| `expand` | −100…100 px | ausdehnen (> 0) bzw. zusammenziehen (< 0); der weiche Rand bleibt erhalten |
| `smooth` | 0…100 px | glätten: rundet Ecken, entfernt Krümel und kleine Löcher |
| `variant` | `auto` \| `small` \| `medium` \| `large` | nur `select.aiObject`: Wahl unter den Masken des SAM-Decoders (`auto` = Wahl des Modells, sonst die Mehrfachmasken nach Fläche) |
| `retune` | `true` | die letzte Auswahl der Aufgabe (bzw. `session`) mit neuen Werten aus der zwischengespeicherten Modell-Ausgabe neu bilden – kein Modell-Laufen; nur solange die Auswahl noch genau dieses Ergebnis ist, sonst Fehler |

Beispiel: `run_command {"name": "select.aiSubject", "params": {}}` →
`run_command {"name": "select.aiSubject", "params": {"retune": true, "tolerance": 70, "expand": 2, "edge": "feather", "feather": 3}}`.
`select.aiRefineEdge` hat nur `radius` (Breite des Kantenbereichs); ein anderer Radius braucht einen
neuen Matting-Lauf.

Ablauf „Objekt freistellen“:

1. `run_command {"name": "ai.models"}` → Modelle `available`?
2. `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` → `{"session": 3, "iou": 0.94, …}`
3. Nachbessern: `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}`
4. Haare/Fell: `run_command {"name": "select.aiRefineEdge", "params": {"output": "selection"}}`
5. `render {"zoom": 1}` prüfen; dann z. B. `run_command {"name": "edit.copy"}` oder eine Maske anlegen.

Die Werkzeuge `tool.aiObject`, `tool.aiSubject`, `tool.aiRefineEdge` stellen in `tool_event` nur eine
Anfrage an die Oberfläche (headless ohne Wirkung) – Agenten nehmen die Befehle oben.

**Bitmap nachzeichnen** (Pixelbild → bearbeitbare Vektorpfade, lokal gerechnet)

`modify.traceBitmap` zeichnet jede ausgewählte Bitmap nach und legt das Ergebnis als Gruppe je Bitmap
auf eine neue Ebene („Nachgezeichnet“); die neuen Gruppen sind danach ausgewählt. Ein Aufruf ist
**ein** Verlaufsschritt. In der Oberfläche: *Modifizieren ▸ Bitmap nachzeichnen…* (Dialog mit Vorschau).

* `preset`: `logo` (Vorgabe; wenige flache Farben, mit Aufräumen), `blackWhite`, `illustration`, `photo`.
  Alle weiteren Optionen überschreiben einzelne Werte der Vorgabe: `colorMode` (`color`/`blackWhite`),
  `colors` (0 = keine Farbreduktion), `filterSpeckle`, `cornerThreshold`, `mode` (`spline`/`polygon`),
  `hierarchical` (`stacked`/`cutout`), `threshold` (Schwarzweiß), `ignoreTransparent`, `maxPixels`,
  `downscale` … – vollständig mit Bereichen: `describe_command {"name": "modify.traceBitmap"}`.
* `refine`: Aufräumen (Logo-Nachbearbeitung) – `true`/`false` oder ein Objekt über der Vorgabe, z. B.
  `{"snapColors": true, "colorTolerance": 12, "straighten": true, "shapes": true, "reduceNodes": true}`.
* `engine`: Verfahren – `clusters` (Vorgabe; Farbflächen, mit Aufräumen) oder `potrace` (Konturen nach
  dem Potrace-Verfahren, eigenständig umgesetzt: glatte Kurven mit wenigen Knoten, ideal für
  Strichzeichnungen, Scans und Schrift; Farbbilder als gestapelte Farbebenen, eine je Farbe). Das
  Konturverfahren nutzt von den übrigen Optionen nur `preset`, `colorMode`, `colors`, `threshold`,
  `ignoreTransparent`, `alphaThreshold`, `maxPixels`, `downscale`, `pathPrecision`,
  `refine.snapColors` und `refine.colorTolerance` und hat eigene Werte:
  `turdSize` (Flecken bis zu dieser Fläche in Pixeln entfernen, Vorgabe 2), `turnPolicy`
  (`minority` Vorgabe, `majority`, `black`, `white`, `left`, `right`: welche diagonal berührenden Pixel
  verbunden werden), `alphaMax` (Eckenschwelle 0…4/3, Vorgabe 1; 0 = Polygon, 4/3 = keine Ecken),
  `optCurve` (Bögen zusammenfassen, Vorgabe `true`), `optTolerance` (0…5 Pixel, Vorgabe 0,2) – einzeln
  oder als Objekt `potrace: {"turdSize": 4, "alphaMax": 0.8}`.
  **Farbeinrasten** („Glatte Kurven“ im Dialog = `engine: "potrace"`): Mit `refine.snapColors` (Vorgabe
  bei `logo`) nimmt auch das Konturverfahren die eingerastete, kantenbewusste Palette des
  Flächenverfahrens (Toleranz `refine.colorTolerance`). Mischtöne der Kantenglättung bekommen so keine
  eigene Farbebene (Logo mit zwei Farben → zwei Ebenen, wenige Knoten). Ohne Einrasten
  (`illustration`, `photo` oder `refine: {"snapColors": false}`) bleibt die Farbreduktion per k-Means.
  Vergleichen: `run_command {"name": "modify.traceBitmap", "params": {"engine": "potrace"}}` →
  `render` → `undo` → dasselbe mit `"engine": "clusters"`; `nodes` im Ergebnis zeigt die Knotenzahl.
* `keepOriginal` (Vorgabe `true`; `false` löscht die Bitmap), `layerName`.
* Ergebnis: `{ids, layer, paths, nodes, downscaled, keptOriginal}`.

Ablauf: `select {"ids": [5]}` → `run_command {"name": "modify.traceBitmap", "params": {"preset": "logo"}}`
→ `render {"zoom": 1}` vergleichen → bei Bedarf `undo` und mit anderen Werten wiederholen.

**Warnungen ohne Ziel**

Menübefehle, die ohne passendes Ziel nichts tun können (z. B. *Vereinfachen…* ohne Pfad), melden das
wie im Original mit einem Warntext; über MCP kommt er als Fehlermeldung, ohne Verlaufsschritt.

## 5a. Die laufende App bedienen: `ui_find` und `ui_input`

`ui_input` speist Ereignisse über ein egui-Plugin vor jedem Frame in die rohe Eingabe ein – derselbe
Weg wie echte Maus und Tastatur; Menüs, Kontextmenüs, Dialoge, Bedienfelder, Leinwand und Kürzel
reagieren also genau wie beim Nutzer. Die Übersetzung entspricht egui-winit unter Windows:

* Strg+C / Strg+X / Strg+V (auch Strg+Einfg, Umschalt+Entf, Umschalt+Einfg) werden beim Drücken zu
  `Copy`/`Cut`/`Paste`. `Paste` kommt nur, wenn die Systemzwischenablage **Text** enthält; sonst kommt
  nur das Loslassen der Taste an (Firepixel fügt dann seinen internen Inhalt ein) – wie in echt.
* Text entsteht nur ohne Strg; Umschalttasten kommen als `ModifiersChanged` und stehen an jedem
  Maus-/Rad-/Tastenereignis; das Mausrad mit Strg zoomt, wie bei einer echten Maus.
* Klicks brauchen eine bekannte Zeigerposition (`pos` angeben oder vorher `move`).
* Abgelegte Dateien: erst „Schweben“, dann Ablegen, danach die Zeigerposition (wie OLE-Drag&Drop) –
  über der Leinwand wird importiert, sonst geöffnet.

Koordinaten sind **Fensterpunkte** (wie in `ui_find` und `screenshot {"rect"}`). Die Lage der Seite
im Fenster steht in `ui_state` → `documents[].page_on_screen` `[l, t, r, b]` (dazu `canvas_area`);
Dokumentpunkt → Fenster: `l + (r − l) · x / Breite`, `t + (b − t) · y / Höhe`.

Schritte (`events`, nacheinander, jeder in eigenen Frames):

| Typ | Felder | Wirkung |
|---|---|---|
| `move` | `pos` | Zeiger bewegen |
| `click` / `double_click` / `right_click` | `pos?`, `button?` (`left`/`right`/`middle`), `count?`, `modifiers?` | Klick(s) |
| `down` / `up` | `button?`, `pos?` | Taste drücken/loslassen (für eigene Abläufe) |
| `drag` | `from`, `to`, `steps?` (12), `ms?` (16 je Schritt), `hold_ms?`, `button?`, `modifiers?` | Ziehen über mehrere Frames |
| `wheel` | `delta` (`[x, y]` oder Zahl, positiv = nach oben), `unit?` (`line`/`point`/`page`), `pos?`, `modifiers?`, `repeat?` | Mausrad |
| `key` | `key` (egui-Namen: `A`–`Z`, `0`–`9`, `F1`, `Enter`, `Escape`, `Tab`, `Delete`, `Backspace`, `ArrowLeft`, `PageUp` …), `modifiers?`, `action?` (`tap`/`press`/`release`), `repeat?` | Taste |
| `modifiers` | `modifiers` (`["shift"]`, `[]` löst) | Umschalttasten **halten** (über mehrere Schritte) |
| `text` / `type` | `text` | Text direkt (wie IME) / je Zeichen Taste + Text |
| `copy` / `cut` / `paste` | `text?` (für `paste`) | Zwischenablage-Ereignis direkt |
| `drop_files` / `hover_files` / `hover_end` | `paths`, `pos` / – | Dateien ablegen / darüber schweben (gilt in jedem Frame, bis `drop_files`, `hover_end` oder `pointer_gone`) / Schweben beenden |
| `wait` / `frames` | `ms` / `n` | warten (Animationen, Doppelklick-Pausen, Renderer) |
| `screenshot` | `rect?` | Bildschirmfoto an dieser Stelle |
| `pointer_gone` / `focus` | – / `focused` | Zeiger verlässt das Fenster / Fensterfokus |
| `zoom` | `factor` | Zwei-Finger-Zoom |

Optionen: `"screenshot": true` oder `[l, t, r, b]` (Bild nach dem letzten Schritt), `"screenshot_each": true`
(Bild nach jedem Schritt, höchstens 24), `"settle_frames": 2` (Frames nach dem letzten Schritt, bevor die
Antwort kommt), `"max_size"`. Die Antwort nennt die ausgeführten Schritte, die von egui gemeldeten
Widget-Ereignisse (`clicked`, `focus_gained` mit Rolle/Beschriftung), kopierten Text, Zeigerposition,
gehaltene Umschalttasten, `dialog_open` und die Statuszeile.

`ui_find` durchsucht für einen Frame den Barrierefreiheits-Baum von egui (Rolle + Name, z. B. `button`,
`check_box`, `text_input`, Dialog-Titelleisten als „Titelleiste <Titel>“) und alle gezeichneten Texte
(Menüs, Bedienfeld-Zeilen, Beschriftungen). Treffer sind nach Genauigkeit und Zeichenreihenfolge sortiert
(was oben liegt, zuerst) – `center` direkt als `pos` für `ui_input` verwenden. Gesucht wird in der
aktuellen Sprache der Oberfläche.

**Beispiele**

* Objekt kopieren und einfügen:
  `ui_input {"events": [{"type": "click", "pos": [640, 420]}, {"type": "key", "key": "C", "modifiers": ["ctrl"]}, {"type": "key", "key": "V", "modifiers": ["ctrl"]}], "screenshot": true}`
* Menü öffnen und Eintrag wählen: `ui_find {"text": "Datei", "exact": true}` → `ui_input {"events": [{"type": "click", "pos": <center>}]}`
  → `ui_find {"text": "Öffnen"}` → klicken (oder `{"type": "key", "key": "Escape"}` zum Schließen).
* Dialog verschieben: `ui_find {"text": "Titelleiste"}` → `ui_input {"events": [{"type": "drag", "from": [x, y], "to": [x - 160, y + 90]}]}`;
  Knöpfe im Dialog: `ui_find {"text": "OK", "role": "button"}`.
* Zoomen wie mit der Maus: `ui_input {"events": [{"type": "wheel", "delta": [0, 1], "pos": [640, 420], "modifiers": ["ctrl"]}]}`.
* Text tippen: `ui_action {"tool": "tool.text"}` → `ui_input {"events": [{"type": "click", "pos": [300, 300]}, {"type": "type", "text": "Hallo"}, {"type": "key", "key": "Escape"}]}`.
* Datei auf die Leinwand ziehen: `ui_input {"events": [{"type": "drop_files", "paths": ["C:\\bilder\\logo.png"], "pos": [500, 380]}, {"type": "wait", "ms": 400}]}`.
* Mit gehaltener Umschalttaste mehrere Objekte wählen: `{"type": "modifiers", "modifiers": ["shift"]}`, Klicks, `{"type": "modifiers", "modifiers": []}`.

Für Dokumentänderungen ohne Oberfläche sind `run_command`/`tool_event` schneller und genauer;
`ui_input` ist für das Prüfen und Bedienen der Oberfläche selbst gedacht.

## 5b. Datei-Dialoge in der laufenden App

Datei-Dialoge blockieren die App nicht mehr: Während einer offen ist, zeichnet Firepixel weiter und
die Brücke antwortet. Leinwand, Menüs und Kürzel sind wie bei einem modalen Dialog gesperrt
(`ui_state` → `dialog_open`, `file_dialog`).

* **KI-Steuerkanal aktiv** (Brücke läuft): Statt des Windows-Dialogs erscheint ein Dialog in der App mit
  Pfadfeld. `ui_file_dialog {}` zeigt Art (`open`, `open_many`, `save`, `folder`), Titel, Filter,
  Startordner, vorgeschlagenen Namen und die letzten Ereignisse. Beantworten:
  `{"action": "choose", "path": "…"}` (mehrere bei `open_many`: `"paths": […]`) oder `{"action": "cancel"}`.
  Der Nutzer kann denselben Dialog auch selbst ausfüllen.
  Zusatzkästchen (nur im Dialog in der App; `ui_file_dialog {}` zeigt ihren Stand): `"append_fw_png"`
  („.fw.png anhängen“, Speichern unter mit Typ FW-PNG), `"open_untitled"` („Als „Unbenannt“ öffnen“) und
  `"open_as_animation"` („Als Animation öffnen“, wirkt ab zwei Dateien). Sie gelten zusammen mit `choose`,
  z. B. `{"action": "choose", "paths": ["a.png", "b.png"], "open_as_animation": true}`, oder einzeln mit
  `{"action": "set", "open_untitled": true}` (Dialog bleibt offen).
* **Ohne Brücke** bzw. mit `FP_FILE_DIALOG=native`: der Windows-Dialog läuft in einem eigenen Thread, mit
  dem Hauptfenster als Besitzer (zentriert darüber, modal, kein eigener Taskleisten-Knopf).
  `ui_file_dialog` kann ihn auch dann beantworten, das Windows-Fenster bleibt aber sichtbar, bis es
  geschlossen wird (`native_window_still_open`); sein Ergebnis wird verworfen.
* **Testmodus** (Umgebungsvariable vor dem Start): `FP_FILE_DIALOG=cancel` bricht jeden Dialog sofort
  ab. `FP_FILE_DIALOG=path:C:\ordner` beantwortet ihn sofort: Speichern mit dem vorgeschlagenen Namen
  im Ordner, Öffnen mit der ersten passenden Datei (Mehrfachauswahl: alle passenden), Ordner mit dem
  Ordner selbst. Jedes Ereignis steht im Log und in `ui_file_dialog` → `last_events`. Der alte Name
  `FP_DATEIDIALOG` (Werte `abbrechen`, `pfad:`, `nativ`) wird weiter gelesen.

Ablauf „Speichern unter“ über die Oberfläche:
`ui_action {"run": "file.saveAs"}` → `ui_file_dialog {}` → `ui_file_dialog {"action": "choose", "path": "C:\\out\\bild.fw.png"}`
→ `ui_state {}` (Titel ohne Stern). `ui_action` antwortet sofort (`dialog_open: true`), der Dialog wartet
in der App, bis `ui_file_dialog` ihn beantwortet; genauso mit `"file.open"`, `"file.import"` usw.
Ohne Oberfläche geht es direkt: `save {"path": …}`.
Dateityp „Reduziertes PNG“ (aktuelle Seite flach mit den Optimierungseinstellungen des Dokuments,
Format PNG, ohne private Chunks): `ui_file_dialog {"action": "choose", "path": "C:\\out\\flach.png", "filter": 4}`
(Index in `filters`), dazu `"save_as_copy": true`, damit das Dokument Namen und Art behält; ohne
Oberfläche `run_command {"name": "file.saveAs", "params": {"path": "C:\\out\\flach.png", "flattened": true, "copy": false}}`.
Ohne Kopie gehört das Dokument danach zur PNG-Datei, und `file.save` schreibt wieder flach.

## 5c. Batch-Verarbeitung: viele Dateien mit einem Rezept

Das Werkzeug `batch` rechnet wie das **Batch-Studio** (*Datei ▸ Batch-Verarbeitung…*): ein **Rezept**
(Schritte + Dateiname + „Speichern als“ + Ziel) auf viele Dateien, parallel, mit Konfliktregeln und
Bericht `batch-report.json` (dazu `batch-report.html` mit Vorschaubildern vorher/nachher, Rezept
`output.reportHtml`, Vorgabe an). Jede Datei läuft in einer eigenen headless Sitzung; die offenen Dokumente
(auch in der Brücke) bleiben unberührt, Originale werden nie verändert (Vorgabe-Ziel: neuer Ordner
`batch-output` neben den Quellen).

`recipe` ist ein Rezept-Objekt (Aufbau wie `*.fpbatch.json`, Schlüssel englisch), JSON-Text, ein Pfad zu
einer Rezeptdatei oder der **Name einer eingebauten Vorlage** (eine vorhandene Datei gleichen Namens hat
Vorrang). Dateien: `files` (absolute Pfade) und/oder `folder` (+ `recursive`, Vorgabe `true`).
Überschreiben: `out` (Ausgabeordner), `conflict` (`rename` | `skip` | `overwrite`), `dryRun`, `threads`.

| `action` | Wirkung | Ergebnis |
|---|---|---|
| `templates` | eingebaute Vorlagen | `templates[]` mit `id`, `title`, `description`, `steps`, `format`, `recipe` |
| `steps` | Katalog aller Schritte mit Parameter-Schema (Typ, Bereich, Vorgabe, Werte) | `steps[]` |
| `validate` | Rezept prüfen | `{ok, issues[]}` (Fehler/Warnungen je Schritt, übersetzt) |
| `plan` | Trockenlauf: Quelle → Ziel, Konflikte, ohne zu schreiben | Plan |
| `preview` | eine Datei (`file`, `maxSize`) vorher/nachher | Größen, Bytes, Hinweise + **zwei PNG-Bilder** |
| `run` (Vorgabe) | ausführen, blockierend bis zum Bericht | Bericht (`totals`, `items[]`, `summary`, `ok`) |
| `run` + `"async": true` | im Hintergrund starten | `{jobId}` sofort |
| `status` | Fortschritt (`jobId`, optional `wait_ms` ≤ 60000: so lange auf das Ende warten) | `state` (`running`/`finished`/`cancelled`), `total`, `processed`, `done`, `failed`, `skipped`, `percent`, `etaMs`, `errors[]`, am Ende `report` |
| `cancel` | Auftrag abbrechen; fertige Dateien bleiben, keine Temporärdateien | wie `status` |

**Eingebaute Vorlagen** (`batch {"action": "templates"}`; im Studio unter *Rezepte ▸ Vorlagen*):

| Name | Inhalt |
|---|---|
| `web-webp` | längste Seite 1920 px, WebP 82, Metadaten ohne GPS |
| `email-photos` | längste Seite 1280 px, Transparenz auf Weiß, JPEG 80, ohne GPS |
| `save-as-jpeg` | Transparenz auf Weiß, JPEG 85 |
| `remove-background` | Hauptmotiv freistellen (lokales KI-Modell, siehe „KI-Auswahl“) → PNG 32 Bit |
| `transparency-to-white` | Transparenz auf Weiß, Format bleibt |
| `logos-to-svg` | `vector.trace` (Vorgabe Logo, Farbflächen) → SVG |
| `small-gifs` | längste Seite 480 px, GIF mit Optimierer „ausgewogen“ |
| `video-to-gif` | Video mit 12 Bildern/s einlesen, längste Seite 480 px, GIF mit Optimierer „ausgewogen“ (braucht ffmpeg) |
| `strip-metadata` | alle Metadaten entfernen, Format bleibt |
| `rename-by-date` | Name `JJJJ-MM-TT_001` aus dem Aufnahmedatum (EXIF), Datei wird nur kopiert |
| `square-1080` | Quadrat 1080 × 1080 (Mitte), JPEG 88 |
| `thumbnails-256` | in 256 × 256 einpassen, PNG, Name mit `_thumb` |

**Nachzeichnen im Batch** (`vector.trace`, Gruppe „Erweitert“): derselbe Befehl wie
`modify.traceBitmap` (siehe „Bitmap nachzeichnen“) auf jedes Bild. Parameter: `preset`
(`logo`/`blackWhite`/`illustration`/`photo`), `engine` (`clusters` Farbflächen | `potrace` glatte Kurven)
und – nur wenn gesetzt, sonst gilt die Vorgabe – `colorMode`, `colors`, `filterSpeckle`,
`cornerThreshold`, `mode`, `hierarchical`, `threshold`, `ignoreTransparent`, `pathPrecision`, `refine`,
`turdSize`, `turnPolicy`, `alphaMax`, `optCurve`, `optTolerance`; dazu `keepOriginal` (Original-Bitmap
als **ausgeblendete** Ebene behalten, Vorgabe `false`) und `transparentCanvas` (Vorgabe `true`).
Die Ausgabe wählt die Endkarte „Speichern als“ (`output.format`): `svg` (nur Pfade, Löcher
gerade/ungerade), `fwpng` (bearbeitbar: Ebene mit der Gruppe, ggf. ausgeblendete Original-Ebene) oder
ein Pixelformat (flach gerechnet). Beides auf einmal: `output.options.alsoFwpng: true` („Zusätzlich als
FW-PNG“, alle Formate außer FW-PNG) legt neben jede Ausgabe eine bearbeitbare FW-PNG gleichen Namens
(`logo.svg` + `logo.fw.png`, Konflikte nach `conflict`); der Bericht nennt sie je Datei in
`items[].extraTargets`.
Animationen: nur das erste Bild (Warnung im Bericht).

**Abläufe**

* Logos nachzeichnen: `batch {"action": "preview", "recipe": "logos-to-svg", "file": "C:\\logos\\a.png"}`
  → Bilder prüfen → `batch {"recipe": "logos-to-svg", "folder": "C:\\logos", "out": "C:\\svg"}`.
  Anderes Verfahren: `recipe` als Objekt, z. B.
  `{"steps": [{"id": "vector.trace", "params": {"preset": "blackWhite", "engine": "potrace"}}], "output": {"format": "svg"}}`.
* Viele Fotos im Hintergrund: `batch {"recipe": "web-webp", "folder": "D:\\Fotos", "async": true}` → `{jobId: 1}`
  → `batch {"action": "status", "jobId": 1, "wait_ms": 10000}` wiederholen, bis `state` ≠ `running`
  → `report.summary`; bei Bedarf `batch {"action": "cancel", "jobId": 1}`.
* Eigenes Rezept: `batch {"action": "steps"}` → Rezept bauen → `batch {"action": "validate", "recipe": {…}}`
  → `batch {"action": "plan", …}` → `batch {…}`.

Dasselbe über `run_command`: `batch.run`, `batch.plan`, `batch.preview`, `batch.steps`, `batch.validate`
(blockierend; `recipe` nimmt auch Vorlagennamen). In der Oberfläche öffnet
`firepixel.exe --batch <rezept|vorlage> [dateien|ordner …]` das Studio mit Rezept und Dateien.

### Video über ffmpeg

Firepixel liefert **keinen** Video-Decoder mit und lädt nichts herunter. Videos (`.mp4`, `.m4v`, `.mov`,
`.webm`, `.mkv`, `.avi`) liest der Batch über ein vom Nutzer installiertes **ffmpeg** als eigenen Prozess
(Argumente als Liste, keine Shell, kein Konsolenfenster, Bilder als rohes RGBA über eine Pipe). Bilder und
Animationen brauchen kein ffmpeg.

**Suchreihenfolge** (die erste gültige Datei gilt; ein gesetzter, aber ungültiger Pfad wird übersprungen):

1. Einstellung **„ffmpeg-Datei…“** in den Batch-Einstellungen (Abschnitt „Videos (ffmpeg)“), gespeichert
   in `<Profil>\video.json`, Schlüssel `ffmpegPath`
2. Umgebungsvariable **`FIREPIXEL_FFMPEG`** (Pfad zu `ffmpeg.exe`)
3. Suchpfad `PATH`
4. Ordner der laufenden exe (`ffmpeg.exe` neben `firepixel.exe` bzw. `firepixel-cli.exe`)

Fehlt ffmpeg, scheitert nur das Laden der Videodatei mit der Meldung „ffmpeg nicht gefunden – Pfad in den
Einstellungen wählen.“ (Bericht: Datei fehlgeschlagen, die übrigen laufen weiter); im Studio zeigt die
Dateiliste „⚠“ mit Klick in die Einstellungen. Gelesen wird mit dem Schritt `video.import` (`start`, `end`
in ms oder als `"1.5s"`/`"0:01:30"`, `fps` 0,1–60, Vorgabe 12, `maxFrames` 1–3000, Vorgabe 600, `maxSize`
längste Seite); alle Frames zusammen höchstens 1 GiB (größere Videos werden verkleinert, solange `maxSize`
nicht fest vorgegeben ist). Beispiel: `batch {"recipe": "video-to-gif", "files": ["C:\\clips\\a.mp4"]}`.

## 6. Befehle-Menü und gespeicherte Befehle

Das Menü **Befehle** zeigt den Befehlsordner (`%APPDATA%\Firepixel\Commands`, Unterordner =
Untermenüs). Beim ersten Start legt Firepixel dort eigene Befehle an (Batch-Befehle, Kreativ,
Dokument, Auswahl, Text, Web, „Größe ausgewählter Objekte ändern“, „Warndialoge zurücksetzen“).
„Skript ausführen…“ startet eine `.jsf`-Datei als **einen** Verlaufsschritt. Aus dem Verlauf-Bedienfeld
gespeicherte Schritte (Menü „Als Befehl speichern“) stehen als `fp.run(…)`-Zeilen in einer `.jsf`-Datei.
Agenten erreichen dasselbe mit `commands.runScript`, `commands.run` und `commands.manageSavedCommands`.

## 7. Kommandozeile

```
firepixel-cli commands [--filter TEXT] [--category CAT] [--all] [--json]
firepixel-cli info <datei> [--json] [--depth N] [--all-pages] [--types text,path]
firepixel-cli run <datei>|--new 800x600[:#farbe] --cmd <name> [--params <json>|@datei] … [--steps <json>] --out <datei>|--in-place
firepixel-cli batch --actions <json|datei> --in <ordner> --out <ordner> [--format png|fwpng|gif|jpg|webp|…]
firepixel-cli batch --recipe <rezept.fpbatch.json|vorlage> (--in <ordner> [--recursive] | <dateien…>) [--out <ordner>] [--conflict rename|skip|overwrite] [--dry-run] [--threads N] [--report bericht.json] [--json]
firepixel-cli batch --templates [--json]   (eingebaute Vorlagen, Namen für --recipe)
firepixel-cli batch --steps [--json]       (Katalog der Schritte)
firepixel-cli render <datei> [--page N] [--state N] [--zoom Z] [--rect l,t,r,b] [--element ID] --out x.png
firepixel-cli convert <ein> <aus>
firepixel-cli script <x.jsf> [datei] [--code JS] [--out datei] [--arg WERT]
firepixel-cli mcp [--http [--port N] [--token T] [--token-file DATEI]] | --bridge [--token-file DATEI]
firepixel-cli language check|template|list …   (alte Namen: sprache pruefen|vorlage|liste)
```

`--json` liefert überall maschinenlesbare Ausgaben; Exit-Code 0 = Erfolg, 1 = Fehler, 2 = falscher Aufruf.
`firepixel-cli <befehl> --help` zeigt Beispiele; `--lang de|en` (oder `FIREPIXEL_LANG`) wählt die
Sprache von Hilfe und Ausgaben. Parameter mit Leerzeichen lassen sich unter
PowerShell am einfachsten als Datei übergeben: `--params @p.json`.
