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
Befehlsnamen (`file.open` …) und Parameter sind sprachunabhängig.

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
| `ui_state` | **nur Brücke:** Werkzeug, Farben, Zoom/Bildlauf je Dokument, Fenstergröße, sichtbare Bedienfelder, Dialog offen?, Statuszeile | `{}` |
| `ui_action` | **nur Brücke:** wie ein Klick in der Oberfläche – Menübefehl (öffnet ggf. seinen Dialog), Zoom, Einpassen, Bedienfeld, Werkzeug, Design, Dialoge schließen | `{"run": "modify.canvas.imageSize"}` · `{"zoom": 2}` · `{"fit": true}` · `{"panel": "window.layers"}` · `{"tool": "tool.pen"}` · `{"close_dialogs": true}` |
| `ui_find` | **nur Brücke:** sichtbare Bedienelemente/Beschriftungen finden (Menüeinträge, Knöpfe, Reiter, Bedienfeld-Zeilen, Dialog-Titelleisten) – Text, Rolle, Rechteck, Mittelpunkt in Fensterpunkten, oberste zuerst | `{"text": "Datei", "exact": true}` · `{"text": "OK", "role": "button"}` · `{"text": "Titelleiste"}` · `{"region": [0, 0, 400, 60]}` |
| `ui_input` | **nur Brücke:** rohe Maus-/Tastatur-/Datei-Ereignisse einspeisen, genau wie echte Eingaben (siehe 5a) | `{"events": [{"type": "click", "pos": [640, 420]}, {"type": "key", "key": "C", "modifiers": ["ctrl"]}], "screenshot": true}` |

Headless liefern `screenshot`/`ui_state`/`ui_action`/`ui_find`/`ui_input` eine Fehlermeldung mit
Hinweis auf die Brücke (dort `render` und `tool_event` benutzen).

Ressourcen (Markdown): `firepixel://docs/agents.md` (diese Datei), `firepixel://docs/commands.md`
(vollständiger Befehlskatalog mit Parametern), `firepixel://docs/document-model.md`,
`firepixel://docs/examples.md`, `firepixel://docs/jsf.md` (Umfang der JSF-API).

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
  * Auswahl: `select.set {ids, mode?}`, `select.inRect {rect}`, `select.pointAt {point}`
  * Struktur: `layer.add {name?}`, `layer.setVisible {layer, visible}`, `layer.setActive {layer}`,
    `page.add`, `page.setCurrent {index}`, `state.add {count?}`, `state.setCurrent {index}`
  * Bewegen/Transformieren: `edit.move {dx, dy}`, `transform.matrix {matrix}`, `path.setContours {id, contours}`
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

**KI-Auswahl** (Objekt, Motiv, Kante – lokale Modelle, keine Netzverbindung beim Rechnen)

Die KI-Befehle erzeugen **Pixelauswahlen** (weiche Kanten möglich) auf Bitmaps oder auf der
sichtbaren Seite. Sie brauchen Modelldateien; fehlt eins, meldet der Befehl „KI-Modell … fehlt“ mit
dem Zielordner. Herunterladen geht nur in der Oberfläche nach Zustimmung des Nutzers
(„Befehle ▸ KI-Modelle verwalten…“, Details `docs\MODELLE.md`) – Agenten laden nichts selbst.

| Befehl | Zweck | Beispiel |
|---|---|---|
| `ai.models` | Modelle mit Aufgabe, Größe, Lizenz, `available`, Pfad; Suchordner und Download-Ziel | `run_command {"name": "ai.models"}` |
| `select.aiObject` | Objekt unter Punkt(en) oder im Rahmen (SAM 2.1). Parameter: `point` oder `points: [[x, y, 1\|0], …]` (1 = Objekt, 0 = Hintergrund), `box`, `mode` (`replace`/`add`/`subtract`/`intersect`), `session` + `refine: true` (dieselbe Auswahl verfeinern), `source: auto\|bitmap\|page`, `region`, `view`, `model: sam2Tiny\|sam2Small`, `feather`. Ergebnis `{session, iou, objectScore, bounds, source}` | `{"name": "select.aiObject", "params": {"point": [220, 140]}}` |
| `select.aiSubject` | Hauptmotiv (BiRefNet) als weiche Auswahl. `mode`, `source`, `point` (Bitmap unter dem Punkt), `region`, `model: birefnetLite\|birefnet`, `feather` | `{"name": "select.aiSubject", "params": {"source": "page"}}` |
| `select.aiRefineEdge` | Kante der vorhandenen Pixelauswahl verfeinern (Haare, Fell, echte Halbtransparenz). `radius` (0 = automatisch), `output: selection\|mask`, `source`, `model: birefnetLiteMatting\|birefnetHrMatting` – braucht eine Pixelauswahl | `{"name": "select.aiRefineEdge", "params": {"output": "mask"}}` |
| `ai.setModelFolder` | eigenen Modellordner festlegen (leer = Standard) | `{"name": "ai.setModelFolder", "params": {"path": "D:\\modelle"}}` |
| `ai.clearCache` | KI-Zwischenspeicher und geladene Modelle freigeben | `{"name": "ai.clearCache"}` |
| `ai.manageModels` | ohne Oberfläche wie `ai.models`; in der App öffnet es den Verwaltungsdialog | `ui_action {"run": "ai.manageModels"}` |

Ablauf „Objekt freistellen“:

1. `run_command {"name": "ai.models"}` → Modelle `available`?
2. `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` → `{"session": 3, "iou": 0.94, …}`
3. Nachbessern: `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}`
4. Haare/Fell: `run_command {"name": "select.aiRefineEdge", "params": {"output": "selection"}}`
5. `render {"zoom": 1}` prüfen; dann z. B. `run_command {"name": "edit.copy"}` oder eine Maske anlegen.

Die Werkzeuge `tool.aiObject`, `tool.aiSubject`, `tool.aiRefineEdge` stellen in `tool_event` nur eine
Anfrage an die Oberfläche (headless ohne Wirkung) – Agenten nehmen die Befehle oben.

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
| `drop_files` / `hover_files` | `paths`, `pos` | Dateien ablegen / darüber schweben |
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
(was oben liegt, zuerst) – `center` direkt als `pos` für `ui_input` verwenden.

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
firepixel-cli render <datei> [--page N] [--state N] [--zoom Z] [--rect l,t,r,b] [--element ID] --out x.png
firepixel-cli convert <ein> <aus>
firepixel-cli script <x.jsf> [datei] [--code JS] [--out datei] [--arg WERT]
firepixel-cli mcp [--http [--port N] [--token T] [--token-file DATEI]] | --bridge [--token-file DATEI]
```

`--json` liefert überall maschinenlesbare Ausgaben; Exit-Code 0 = Erfolg, 1 = Fehler, 2 = falscher Aufruf.
`firepixel-cli <befehl> --help` zeigt Beispiele. Parameter mit Leerzeichen lassen sich unter
PowerShell am einfachsten als Datei übergeben: `--params @p.json`.
