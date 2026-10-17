# Beispiele (MCP-Werkzeugaufrufe)

## Neues Banner mit Text, rendern, exportieren

1. `new_document {"width": 600, "height": 200, "background": "#ffffff"}`
2. `run_command {"name": "draw.rectangle", "params": {"rect": [0, 0, 600, 200], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Hintergrund"}}`
3. `run_command {"name": "draw.text", "params": {"text": "Sommerangebot", "anchor": [40, 120], "size": 48, "color": "#ffffff"}}`
4. `render {"zoom": 0.5}` – Ergebnis ansehen
5. `export {"path": "C:\\out\\banner.png"}` und `save {"path": "C:\\out\\banner.fw.png"}`

## Objekte ausrichten

1. `get_document {"depth": 1}` – IDs suchen
2. `select {"ids": [12, 15, 19]}`
3. `run_command {"name": "modify.align.left"}` dann `run_command {"name": "modify.align.distributeHeights"}`

## Schlagschatten und Weichzeichner

- `run_command {"name": "element.addEffect", "params": {"effect": "dropShadow", "params": {"ShadowDistance": "7", "ShadowColor": "#000000"}}}`
- `run_command {"name": "filter.blur.gaussian", "params": {"radius": 2.5}}` (auf Bitmaps destruktiv, auf Vektoren als Live-Effekt)

## Zeichnen wie ein Mensch

- `tool_event {"tool": "ellipse", "drag": {"from": [50, 50], "to": [150, 120], "shift": true}}` – Kreis
- `tool_event {"tool": "pen", "events": [{"type":"down","x":10,"y":10},{"type":"up","x":10,"y":10},{"type":"down","x":100,"y":10},{"type":"move","x":140,"y":60},{"type":"up","x":140,"y":60},{"type":"key","key":"enter"}]}` – Pfad mit Kurve

## Mehrere Schritte als ein Rückgängig-Schritt

`run_command {"steps": [{"name": "select.all"}, {"name": "modify.group"}, {"name": "modify.transform.flipHorizontal"}], "label": "Spiegeln"}`

## Ebenen

- `run_command {"name": "layer.add", "params": {"name": "Hintergrund"}}` → neue aktive Ebene
- `run_command {"name": "layer.setVisible", "params": {"layer": 7, "visible": false}}`

## KI-Auswahl (Modelle lokal, siehe `ai.models`)

- `run_command {"name": "ai.models"}` – welche Modelle da sind (`available`), Ordner, Download-Ziel
- `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` – Objekt unter dem Punkt als Pixelauswahl; Ergebnis `{session, iou, objectScore, bounds, source}`
- `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}` – dieselbe Auswahl verfeinern (Label 0 = Hintergrund, 1 = Objekt)
- `run_command {"name": "select.aiObject", "params": {"box": [180, 90, 320, 220], "model": "sam2Small"}}` – Objekt im Rahmen
- `run_command {"name": "select.aiSubject", "params": {"source": "page"}}` – Hauptmotiv der sichtbaren Seite
- `run_command {"name": "select.aiRefineEdge", "params": {"output": "mask"}}` – Kante (Haare, Fell) verfeinern, Ergebnis als Bitmapmaske (vorher Pixelauswahl nötig)

## JSF-JavaScript

`run_script {"code": "var d = fw.getDocumentDOM(); d.addNewRectangle({left:0, top:0, right:100, bottom:50}, 0); d.setFillColor('#ff8800'); fw.selection.length"}`
