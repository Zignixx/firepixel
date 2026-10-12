# Examples (MCP tool calls)

## New banner with text, render, export

1. `new_document {"width": 600, "height": 200, "background": "#ffffff"}`
2. `run_command {"name": "draw.rectangle", "params": {"rect": [0, 0, 600, 200], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Background"}}`
3. `run_command {"name": "draw.text", "params": {"text": "Summer sale", "anchor": [40, 120], "size": 48, "color": "#ffffff"}}`
4. `render {"zoom": 0.5}` – look at the result
5. `export {"path": "C:\\out\\banner.png"}` and `save {"path": "C:\\out\\banner.fw.png"}`

## Align objects

1. `get_document {"depth": 1}` – look up IDs
2. `select {"ids": [12, 15, 19]}`
3. `run_command {"name": "modify.align.left"}` then `run_command {"name": "modify.align.distributeHeights"}`

## Drop shadow and blur

- `run_command {"name": "element.addEffect", "params": {"effect": "dropShadow", "params": {"ShadowDistance": "7", "ShadowColor": "#000000"}}}`
- `run_command {"name": "filter.blur.gaussian", "params": {"radius": 2.5}}` (destructive on bitmaps, a live filter on vectors)

## Drawing like a person

- `tool_event {"tool": "ellipse", "drag": {"from": [50, 50], "to": [150, 120], "shift": true}}` – circle
- `tool_event {"tool": "pen", "events": [{"type":"down","x":10,"y":10},{"type":"up","x":10,"y":10},{"type":"down","x":100,"y":10},{"type":"move","x":140,"y":60},{"type":"up","x":140,"y":60},{"type":"key","key":"enter"}]}` – path with a curve

## Several steps as one undo step

`run_command {"steps": [{"name": "select.all"}, {"name": "modify.group"}, {"name": "modify.transform.flipHorizontal"}], "label": "Flip"}`

## Layers

- `run_command {"name": "layer.add", "params": {"name": "Background"}}` → new active layer
- `run_command {"name": "layer.setVisible", "params": {"layer": 7, "visible": false}}`

## AI selection (models are local, see `ai.models`)

- `run_command {"name": "ai.models"}` – which models are present (`available`), folder, download target
- `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` – object under the point as a pixel selection; result `{session, iou, objectScore, bounds, source}`
- `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}` – refine the same selection (label 0 = background, 1 = object)
- `run_command {"name": "select.aiObject", "params": {"box": [180, 90, 320, 220], "model": "sam2Small"}}` – object inside the box
- `run_command {"name": "select.aiSubject", "params": {"source": "page"}}` – main subject of the visible page
- `run_command {"name": "select.aiRefineEdge", "params": {"output": "mask"}}` – refine the edge (hair, fur), result as a bitmap mask (needs a pixel selection first)

## JSF JavaScript

`run_script {"code": "var d = fw.getDocumentDOM(); d.addNewRectangle({left:0, top:0, right:100, bottom:50}, 0); d.setFillColor('#ff8800'); fw.selection.length"}`
