# Firepixel for AI Agents

Firepixel is a native graphics application in the style of the original: vector and bitmap editing,
pages, states (frames), layers, symbols, Live Filters, auto shapes, FW-PNG (`.fw.png`).
Agents control it through **the same command registry** as the user interface – via MCP, the
command line or JSF JavaScript. Every change goes into the history and can be undone.

Three ways to the MCP server:

| Way | When | Session |
|---|---|---|
| `firepixel-cli mcp` (stdio) | automation without a window | own, empty session per start |
| `firepixel-cli mcp --http` | several clients, HTTP | own session per server process |
| **Bridge to the running app** (`firepixel.exe --mcp` or menu *Commands ▸ AI Control Channel (MCP)*) | working together with the user | **the documents of the user interface**, changes visible immediately |

## 1. Setting up MCP

### stdio (headless)

```bash
claude mcp add firepixel -- C:\path\to\firepixel-cli.exe mcp
```

or in `.mcp.json` (project) or `~/.claude.json`:

```json
{
  "mcpServers": {
    "firepixel": { "type": "stdio", "command": "C:\\path\\to\\firepixel-cli.exe", "args": ["mcp"] }
  }
}
```

### Streamable HTTP (headless, 127.0.0.1 only, with token)

```bash
firepixel-cli mcp --http --port 47800
```

URL and token are printed on stderr and stored in `%LOCALAPPDATA%\Firepixel\mcp-http.json`
(`{"url": "http://127.0.0.1:47800/mcp", "port": 47800, "token": "…", "header": "Authorization: Bearer …", "pid": …, "mode": "headless"}`).
The file is deleted when the server exits.

```bash
claude mcp add --transport http firepixel http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token>"
```

### Bridge to the running user interface

1. Start Firepixel with the AI control channel: `firepixel.exe --mcp` (fixed port: `--mcp=47811`), or
   switch on **Commands ▸ AI Control Channel (MCP)** in the running app, or set the environment
   variable `FIREPIXEL_MCP=1` before starting (port: `FIREPIXEL_MCP_PORT`). The status bar shows
   "AI control channel active: http://127.0.0.1:47800/mcp". If the port is taken, Firepixel picks a free one.
2. Address and token are in **`%LOCALAPPDATA%\Firepixel\mcp-bridge.json`** (same layout as above,
   `"mode": "bridge"`; separate from `mcp-http.json` so that a headless server does not hide the bridge).
   The file is deleted when the app closes or the channel is switched off; the token is new on every
   start (256 bits).
3. Connect – either directly via HTTP with the token from the file:

   ```bash
   claude mcp add --transport http firepixel-gui http://127.0.0.1:47800/mcp --header "Authorization: Bearer <token from mcp-bridge.json>"
   ```

   or **without copying the token** through the stdio relay of the CLI, which reads address and token itself:

   ```bash
   claude mcp add firepixel-gui -- C:\path\to\firepixel-cli.exe mcp --bridge
   ```

   (`--token-file <file>` for a different token file.) If the app is not running, the relay reports
   "token file … not readable – is Firepixel running with the AI control channel switched on?".

In the bridge all requests run in the UI thread on the same session as menus and tools: the user
sees every change, everything is listed in the History panel. In addition there are the
user-interface tools `screenshot`, `ui_state`, `ui_action`, `ui_find` and `ui_input` (see 2 and 5a).
With `ui_find` + `ui_input` the app can be operated completely like a human would – even when screen
and input of the Windows session (e.g. RDP without console) cannot be reached from outside.

Security: the server binds to `127.0.0.1` only, drops connections from other addresses, requires the
token with every request (`Authorization: Bearer …` or `X-Firepixel-Token: …`, compared in constant
time), checks the `Host` header (protection against DNS rebinding) and runs no shell commands. Files
are accessed only through Open, Save, Export and JSF `Files`.

Language: answers, command labels (`label`, `description` in `list_commands`/`describe_command`) and
error messages are in **English** by default – independent of the language of the user interface,
which does not switch. Every tool optionally takes `"language": "de"` (or another language code); the
server default can be changed with the environment variable `FIREPIXEL_MCP_LANG`. Command names
(`file.open` …) and parameters do not depend on the language. The resources (this guide, command
catalog, document model, examples, JSF list) also follow the answer language: German for `de`,
English otherwise (German version of this file: `docs\AGENTS.md`).

Note: requests are accepted concurrently. Send dependent calls (e.g. `render` after `new_document`)
only after the answer to the previous one – MCP clients like Claude do that by themselves.

## 2. Tools

| Tool | Purpose | Example |
|---|---|---|
| `list_commands` | search commands (without arguments: categories) | `{"category": "modify.align"}` · `{"filter": "feather"}` |
| `describe_command` | schema, required parameters, requirements, shortcuts, example | `{"name": "draw.rectangle"}` |
| `run_command` | run a command (or `steps` as **one** undo step) | `{"name": "modify.group"}` |
| `get_document` | document tree: pages, states, layers, elements with IDs/bounds/attributes | `{"depth": 1}` · `{"types": ["text"]}` |
| `select` | set the selection (IDs, all, none, point, rectangle) | `{"ids": [12, 15]}` · `{"all": true}` · `{"point": [20, 20]}` |
| `render` | page/area/element as a PNG image (offscreen, without user interface) | `{"zoom": 0.5}` · `{"element": 12, "zoom": 2}` |
| `open` / `save` / `export` | files (FW-PNG, PNG, JPEG, GIF, WebP, AVIF, PSD import …) | `{"path": "C:\\out\\a.png"}` |
| `undo` / `redo` / `history` | history | `{"steps": 2}` · `{"goto": 3}` |
| `new_document` | new document | `{"width": 800, "height": 600, "background": "#ffffff"}` |
| `documents` | open documents, switch, close | `{"activate": 2}` · `{"close": 3, "discard": true}` |
| `tool_event` | operate a tool like a human (pointer events) | `{"tool": "ellipse", "drag": {"from": [10,10], "to": [90,60]}}` |
| `run_script` | run JSF JavaScript | `{"code": "fw.getDocumentDOM().width"}` |
| `screenshot` | **bridge only:** screenshot of the app window (menus, panels, dialogs) | `{}` · `{"rect": [0, 0, 800, 600]}` · `{"max_size": 1200}` |
| `ui_state` | **bridge only:** tool, colors, zoom/scroll per document, window size, visible panels, dialog open?, status bar | `{}` |
| `ui_action` | **bridge only:** like a click in the user interface – menu command (opens its dialog, if any), zoom, fit, panel, tool, theme, close dialogs | `{"run": "modify.canvas.imageSize"}` · `{"zoom": 2}` · `{"fit": true}` · `{"panel": "window.layers"}` · `{"tool": "tool.pen"}` · `{"close_dialogs": true}` |
| `ui_find` | **bridge only:** find visible controls/labels (menu items, buttons, tabs, panel rows, dialog title bars) – text, role, rectangle, center in window points, topmost first | `{"text": "File", "exact": true}` · `{"text": "OK", "role": "button"}` · `{"text": "Title bar"}` · `{"region": [0, 0, 400, 60]}` |
| `ui_input` | **bridge only:** inject raw mouse/keyboard/file events, exactly like real input (see 5a) | `{"events": [{"type": "click", "pos": [640, 420]}, {"type": "key", "key": "C", "modifiers": ["ctrl"]}], "screenshot": true}` |
| `ui_file_dialog` | **bridge only:** query and answer an open file dialog (Open, Save As, Import, folder …) (see 5b) | `{}` · `{"action": "choose", "path": "C:\\out\\a.fw.png"}` · `{"action": "choose", "paths": ["a.png", "b.png"]}` · `{"action": "cancel"}` |

Headless, `screenshot`/`ui_state`/`ui_action`/`ui_find`/`ui_input`/`ui_file_dialog` return an error
message pointing to the bridge (use `render` and `tool_event` there).

Resources (Markdown): `firepixel://docs/agents.md` (this file), `firepixel://docs/commands.md`
(complete command catalog with parameters), `firepixel://docs/document-model.md`,
`firepixel://docs/examples.md`, `firepixel://docs/jsf.md` (scope of the JSF API).

## 3. Command name scheme

* Menu commands: `<menu>.<item>[.<subitem>]` in camelCase after the English menu of the original:
  `file.open`, `file.saveAs`, `edit.duplicate`, `edit.undo`, `view.zoomIn`, `select.all`, `select.inverse`,
  `modify.group`, `modify.ungroup`, `modify.arrange.bringToFront`, `modify.align.left`,
  `modify.align.distributeWidths`, `modify.transform.flipHorizontal`, `modify.transform.numericTransform`,
  `modify.combinePaths.union`, `modify.alterPath.expandStroke`, `modify.canvas.imageSize`,
  `modify.symbol.convertToSymbol`, `modify.mask.groupAsMask`, `text.font`, `filter.blur.gaussian`,
  `filter.adjustColor.hueSaturation`, `filter.sharpen.unsharpMask`, `window.layers` …
* Tool and panel commands (with parameters, ideal for agents):
  * Drawing: `draw.rectangle {rect, roundness?, fillColor?, strokeColor?, strokeWidth?, name?}`,
    `draw.ellipse {rect}`, `draw.line {from, to}`, `draw.polygon {center, corner, sides, star?}`,
    `draw.path {contours: [{closed, nodes: [{anchor, in, out}]}]}`, `draw.text {text, anchor | rect, font?, size?, color?}`
  * Attributes: `element.setFill {color | kind: "none", ids?}`, `element.setStroke {color?, width?, placement?}`,
    `element.setOpacity {opacity}`, `element.setBlendMode {mode}`, `element.setName {name}`,
    `element.setVisible`, `element.setLocked`, `element.setBounds {x?, y?, width?, height?}`,
    `element.addEffect {effect: "dropShadow"|"innerShadow"|"bevel"|"glow"|… , params?}`, `element.setGradient {from, to, colors?}`
  * Textures and patterns (names as in the inspector lists, files from the texture or pattern folders):
    `element.setTexture {name | image, target?: "fill"|"stroke", amount?, ids?}` (without `name` and `image`: remove the texture),
    `element.setPattern {name | image, ids?}`, `pi.fill {pattern}` or `pi.fill {texture, textureAmount?}` on the selection
    (no selection: the tool's default); an unknown name → error message
  * Bitmap: `bitmap.floodFill {point, tolerance?, color?, fillSelection?}` (Paint Bucket),
    `bitmap.gradient {from, to, colors?, fillSelection?, tolerance?}` (Gradient tool; `fillSelection`
    default `true` = fill the pixel selection or the whole bitmap, `false` = contiguous color area at the
    start point by `tolerance` 0…255, default 32), `bitmap.paint`, `bitmap.erase`
  * Selection: `select.set {ids, mode?}`, `select.inRect {rect}`, `select.pointAt {point}`
  * Structure: `layer.add {name?}`, `layer.setVisible {layer, visible}`, `layer.setActive {layer}`,
    `page.add`, `page.setCurrent {index}`, `state.add {count?}`, `state.setCurrent {index}`
  * Moving/transforming: `edit.move {dx, dy}`, `transform.matrix {matrix}`, `path.setContours {id, contours}`
  * Choosing a tool: `tool.select {tool: "tool.pen"}`; colors: `color.set`, `color.swap`, `color.defaults`
  * **Auto shapes** (41 shapes, own implementation; list: `autoShape.list`):
    `autoShape.insert {shape, at | rect}` (English or German name, e.g. `"Star"`/`"Stern"`,
    `"Rounded Rectangle"`, `"Arrow"`, `"Pie"`, `"Spiral"`, `"Cog"`, `"Calendar"`),
    `autoShape.dragControlPoint {id, index, to, from?, points?, shift?, alt?, ctrl?}` (drag a control point;
    `from` = `to` is a click, e.g. to change the corner type), `autoShape.describe {id}` (parameters with
    values and control points), `autoShape.setParams {id, params: {key: value}}`, `autoShape.redraw {id}`.
  * Scripts: `commands.runScript {path | code}`, `commands.run {name}` (saved command),
    `commands.manageSavedCommands {action: list|save|rename|delete, name?, code?}`.
* Placeholders that have no effect yet are reported by `describe_command` with `"implemented": false`.

## 4. Conventions

* Coordinates in document pixels, origin at the top left of the canvas.
* Rectangles `[left, top, right, bottom]` (also `{left, top, right, bottom}` or `{x, y, width, height}`),
  points `[x, y]`, colors `"#rrggbb"` or `"#rrggbbaa"`, opacity in percent.
* Layers in `get_document` are listed **bottom → top** (like `dom.layers` in the original; the Web Layer is
  the last one), elements of a layer **top → bottom**. Auto shapes have the type `autoShape`.
* Element IDs stay stable during the session. New objects return `{"id": …}` in the result of
  `run_command` and are selected afterwards.
* Most menu commands act on the selection. If a command is not available, the error message names
  the reason (e.g. "select at least two objects").
* Repeated calls are safe: setting the same parameters again changes nothing twice
  (e.g. `autoShape.setParams` 50× → one shape, one path).

## 5. Typical workflows

**Build and check a new design**

1. `new_document {"width": 600, "height": 300, "background": "#ffffff"}`
2. `run_command {"name": "draw.rectangle", "params": {"rect": [0, 0, 600, 300], "fillColor": "#0f172a", "strokeWidth": 0, "name": "Background"}}`
3. `run_command {"name": "draw.text", "params": {"text": "Firepixel", "anchor": [40, 170], "size": 64, "color": "#f8fafc"}}`
4. `run_command {"name": "element.addEffect", "params": {"effect": "dropShadow"}}` (acts on the selection = the text)
5. `render {"zoom": 0.5}` → look at the image, correct if necessary, `undo`
6. `save {"path": "C:\\out\\banner.fw.png"}`, `export {"path": "C:\\out\\banner.png"}`

**Edit an existing file**

1. `open {"path": "C:\\in\\draft.fw.png"}`
2. `get_document {"depth": 2}` → IDs and bounds
3. `select {"ids": [21, 22, 23]}` → `run_command {"name": "modify.align.top"}`
4. `render {"rect": [0, 0, 300, 200], "zoom": 2}` → check the detail
5. `save {}`

**Several commands as one step**

`run_command {"steps": [{"name": "select.all"}, {"name": "modify.group"}, {"name": "modify.transform.rotate90DegreeCw"}], "label": "Rotate"}`

**Insert and adjust an auto shape**

1. `run_command {"name": "autoShape.insert", "params": {"shape": "Star", "at": [100, 80]}}` → `{"id": 7}`
2. `run_command {"name": "autoShape.describe", "params": {"id": 7}}` → parameters (`nodes`, `radius0` …) and control points
3. `run_command {"name": "autoShape.setParams", "params": {"id": 7, "params": {"nodes": 8}}}`
4. `run_command {"name": "autoShape.dragControlPoint", "params": {"id": 7, "index": 0, "to": [200, 40]}}`

**Draw like a human** (tool state machines, including the modifiers of the original)

* Square: `tool_event {"tool": "rectangle", "drag": {"from": [20, 20], "to": [120, 90], "shift": true}}`
* Path with the pen: `tool_event {"tool": "pen", "events": [{"type":"down","x":10,"y":10},{"type":"up","x":10,"y":10},{"type":"down","x":120,"y":10},{"type":"move","x":160,"y":60},{"type":"up","x":160,"y":60},{"type":"key","key":"enter"}]}`
* Typing text: `tool_event {"tool": "text", "events": [{"type":"down","x":30,"y":60},{"type":"up","x":30,"y":60},{"type":"text","text":"Hello"},{"type":"key","key":"escape"}]}`

**Together with the user in the running app** (bridge)

1. `ui_state {}` → which document is open, which tool, zoom
2. `get_document {"depth": 2}` → IDs; `run_command …` → the user sees the change immediately
3. `ui_action {"fit": true}` → fit the document; `screenshot {}` → what the user sees right now
4. `ui_action {"run": "filter.blur.gaussian"}` opens the dialog in the app (the user confirms);
   directly without a dialog: `run_command {"name": "filter.blur.gaussian", "params": {"radius": 3}}`

**Reusing JSF scripts**

`run_script {"path": "C:\\scripts\\macro.jsf"}` – the API (`fw`, `fw.getDocumentDOM()`, `Files` …)
matches the original (550/565 methods, 546/721 properties; the rest is left out for documented reasons,
see `firepixel://docs/jsf.md`). In addition `fp.run(name, params)` for every command of the registry.

**AI selection** (object, subject, edge – local models, no network connection while computing)

The AI commands create **pixel selections** (soft edges possible) on bitmaps or on the visible page.
They need model files; if one is missing, the command reports "AI model … is missing" with the target
folder. Downloading is only possible in the user interface after the user agrees
("Commands ▸ Manage AI Models...", details in `docs\MODELLE.md`) – agents do not download anything themselves.

| Command | Purpose | Example |
|---|---|---|
| `ai.models` | models with task, size, license, `available`, path; search folders and download target | `run_command {"name": "ai.models"}` |
| `select.aiObject` | object under point(s) or inside a box (SAM 2.1). Parameters: `point` or `points: [[x, y, 1\|0], …]` (1 = object, 0 = background), `box`, `mode` (`replace`/`add`/`subtract`/`intersect`), `session` + `refine: true` (refine the same selection), `source: auto\|bitmap\|page`, `region`, `view`, `model: sam2Tiny\|sam2Small`, fine-tuning (see below). Result `{session, iou, objectScore, bounds, source, tuning}` | `{"name": "select.aiObject", "params": {"point": [220, 140]}}` |
| `select.aiSubject` | main subject (BiRefNet) as a soft selection. `mode`, `source`, `point` (bitmap under the point), `region`, `model: birefnetLite\|birefnet`, fine-tuning (see below). Result `{session, bounds, source, tuning}` | `{"name": "select.aiSubject", "params": {"source": "page"}}` |
| `select.aiRefineEdge` | refine the edge of the existing pixel selection (hair, fur, real semi-transparency). `radius` (0 = automatic), `output: selection\|mask`, `source`, `model: birefnetLiteMatting\|birefnetHrMatting` – needs a pixel selection | `{"name": "select.aiRefineEdge", "params": {"output": "mask"}}` |
| `ai.setModelFolder` | set your own model folder (empty = default) | `{"name": "ai.setModelFolder", "params": {"path": "D:\\models"}}` |
| `ai.clearCache` | release the AI cache and loaded models | `{"name": "ai.clearCache"}` |
| `ai.manageModels` | without user interface like `ai.models`; in the app it opens the management dialog | `ui_action {"run": "ai.manageModels"}` |

Fine-tuning of `select.aiObject` and `select.aiSubject` (without parameters the values from the
tool's Property inspector apply, otherwise the defaults; reasons in `docs\MODELLE.md`):

| Parameter | Values | Effect |
|---|---|---|
| `tolerance` | 0…100, default 50 | 50 = the model's own decision; higher selects more. SAM: threshold of the mask logits (±10), BiRefNet: tone levels of the soft mask |
| `edge` | `hard` \| `antiAlias` (default) \| `feather` | edge as with the Magic Wand; `feather` with `feather: <px>` (without `edge`, `feather` > 0 still gives a feathered edge) |
| `expand` | −100…100 px | expand (> 0) or contract (< 0); the soft edge is kept |
| `smooth` | 0…100 px | smooth: rounds corners, removes specks and small holes |
| `variant` | `auto` \| `small` \| `medium` \| `large` | `select.aiObject` only: choice among the masks of the SAM decoder (`auto` = the model's choice, otherwise the multimask outputs by area) |
| `retune` | `true` | rebuild the task's last selection (or `session`) with new values from the cached model output – no model run; only while the selection is still exactly that result, otherwise an error |

Example: `run_command {"name": "select.aiSubject", "params": {}}` →
`run_command {"name": "select.aiSubject", "params": {"retune": true, "tolerance": 70, "expand": 2, "edge": "feather", "feather": 3}}`.
`select.aiRefineEdge` only has `radius` (width of the edge band); a different radius needs a new
matting run.

Workflow "cut out an object":

1. `run_command {"name": "ai.models"}` → models `available`?
2. `run_command {"name": "select.aiObject", "params": {"point": [220, 140]}}` → `{"session": 3, "iou": 0.94, …}`
3. Touch up: `run_command {"name": "select.aiObject", "params": {"points": [[260, 150, 0]], "session": 3, "refine": true}}`
4. Hair/fur: `run_command {"name": "select.aiRefineEdge", "params": {"output": "selection"}}`
5. Check with `render {"zoom": 1}`; then e.g. `run_command {"name": "edit.copy"}` or create a mask.

The tools `tool.aiObject`, `tool.aiSubject`, `tool.aiRefineEdge` only send a request to the user
interface in `tool_event` (no effect headless) – agents use the commands above.

**Trace Bitmap** (pixel image → editable vector paths, computed locally)

`modify.traceBitmap` traces every selected bitmap and puts the result as one group per bitmap on a new
layer ("Traced"); the new groups are selected afterwards. One call is **one** history step. In the user
interface: *Modify ▸ Trace Bitmap...* (dialog with preview).

* `preset`: `logo` (default; few flat colors, with clean-up), `blackWhite`, `illustration`, `photo`.
  All other options override single values of the preset: `colorMode` (`color`/`blackWhite`),
  `colors` (0 = no color reduction), `filterSpeckle`, `cornerThreshold`, `mode` (`spline`/`polygon`),
  `hierarchical` (`stacked`/`cutout`), `threshold` (black and white), `ignoreTransparent`, `maxPixels`,
  `downscale` … – complete with ranges: `describe_command {"name": "modify.traceBitmap"}`.
* `refine`: clean-up (logo post-processing) – `true`/`false` or an object on top of the preset, e.g.
  `{"snapColors": true, "colorTolerance": 12, "straighten": true, "shapes": true, "reduceNodes": true}`.
* `engine`: method – `clusters` (default; color regions, with clean-up) or `potrace` (outlines using the
  Potrace method, implemented independently: smooth curves with few nodes, ideal for line art, scans
  and lettering; color images as stacked color layers, one per color). The outline method only uses
  `preset`, `colorMode`, `colors`, `threshold`, `ignoreTransparent`, `alphaThreshold`, `maxPixels`,
  `downscale`, `pathPrecision` from the other options and has its own values: `turdSize` (remove
  speckles up to this area in pixels, default 2), `turnPolicy` (`minority` default, `majority`, `black`,
  `white`, `left`, `right`: which diagonally touching pixels are joined), `alphaMax` (corner threshold
  0…4/3, default 1; 0 = polygon, 4/3 = no corners), `optCurve` (merge arcs, default `true`),
  `optTolerance` (0…5 pixels, default 0.2) – individually or as an object
  `potrace: {"turdSize": 4, "alphaMax": 0.8}`.
  To compare: `run_command {"name": "modify.traceBitmap", "params": {"engine": "potrace"}}` → `render`
  → `undo` → the same with `"engine": "clusters"`; `nodes` in the result shows the node count.
* `keepOriginal` (default `true`; `false` deletes the bitmap), `layerName`.
* Result: `{ids, layer, paths, nodes, downscaled, keptOriginal}`.

Workflow: `select {"ids": [5]}` → `run_command {"name": "modify.traceBitmap", "params": {"preset": "logo"}}`
→ compare with `render {"zoom": 1}` → if necessary `undo` and repeat with other values.

**Warnings without a target**

Menu commands that cannot do anything without a suitable target (e.g. *Simplify...* without a path)
report this with a warning text like the original; via MCP it arrives as an error message, without a
history step.

## 5a. Operating the running app: `ui_find` and `ui_input`

`ui_input` injects events into the raw input through an egui plugin before every frame – the same way
as a real mouse and keyboard; menus, context menus, dialogs, panels, canvas and shortcuts react exactly
as they do for the user. The translation matches egui-winit on Windows:

* Ctrl+C / Ctrl+X / Ctrl+V (also Ctrl+Ins, Shift+Del, Shift+Ins) become `Copy`/`Cut`/`Paste` when
  pressed. `Paste` only arrives if the system clipboard contains **text**; otherwise only the key
  release arrives (Firepixel then pastes its internal content) – as in reality.
* Text is only produced without Ctrl; modifier keys arrive as `ModifiersChanged` and are attached to
  every mouse/wheel/key event; the mouse wheel with Ctrl zooms, as with a real mouse.
* Clicks need a known pointer position (give `pos` or `move` first).
* Dropped files: first "hover", then drop, then the pointer position (like OLE drag and drop) – over the
  canvas they are imported, otherwise opened.

Coordinates are **window points** (as in `ui_find` and `screenshot {"rect"}`). The position of the page
in the window is in `ui_state` → `documents[].page_on_screen` `[l, t, r, b]` (plus `canvas_area`);
document point → window: `l + (r − l) · x / width`, `t + (b − t) · y / height`.

Steps (`events`, one after the other, each in its own frames):

| Type | Fields | Effect |
|---|---|---|
| `move` | `pos` | move the pointer |
| `click` / `double_click` / `right_click` | `pos?`, `button?` (`left`/`right`/`middle`), `count?`, `modifiers?` | click(s) |
| `down` / `up` | `button?`, `pos?` | press/release a button (for your own sequences) |
| `drag` | `from`, `to`, `steps?` (12), `ms?` (16 per step), `hold_ms?`, `button?`, `modifiers?` | drag over several frames |
| `wheel` | `delta` (`[x, y]` or a number, positive = up), `unit?` (`line`/`point`/`page`), `pos?`, `modifiers?`, `repeat?` | mouse wheel |
| `key` | `key` (egui names: `A`–`Z`, `0`–`9`, `F1`, `Enter`, `Escape`, `Tab`, `Delete`, `Backspace`, `ArrowLeft`, `PageUp` …), `modifiers?`, `action?` (`tap`/`press`/`release`), `repeat?` | key |
| `modifiers` | `modifiers` (`["shift"]`, `[]` releases) | **hold** modifier keys (across several steps) |
| `text` / `type` | `text` | text directly (like IME) / key + text per character |
| `copy` / `cut` / `paste` | `text?` (for `paste`) | clipboard event directly |
| `drop_files` / `hover_files` | `paths`, `pos` | drop files / hover over with files |
| `wait` / `frames` | `ms` / `n` | wait (animations, double-click pauses, renderer) |
| `screenshot` | `rect?` | screenshot at this point |
| `pointer_gone` / `focus` | – / `focused` | pointer leaves the window / window focus |
| `zoom` | `factor` | two-finger zoom |

Options: `"screenshot": true` or `[l, t, r, b]` (image after the last step), `"screenshot_each": true`
(image after every step, at most 24), `"settle_frames": 2` (frames after the last step before the answer
comes), `"max_size"`. The answer lists the executed steps, the widget events reported by egui
(`clicked`, `focus_gained` with role/label), copied text, pointer position, held modifier keys,
`dialog_open` and the status bar.

`ui_find` searches the accessibility tree of egui for one frame (role + name, e.g. `button`,
`check_box`, `text_input`, dialog title bars as "Title bar <title>") and all drawn texts (menus, panel
rows, labels). Matches are sorted by accuracy and drawing order (what lies on top comes first) – use
`center` directly as `pos` for `ui_input`. Texts are those of the current user interface language.

**Examples**

* Copy and paste an object:
  `ui_input {"events": [{"type": "click", "pos": [640, 420]}, {"type": "key", "key": "C", "modifiers": ["ctrl"]}, {"type": "key", "key": "V", "modifiers": ["ctrl"]}], "screenshot": true}`
* Open a menu and choose an item: `ui_find {"text": "File", "exact": true}` → `ui_input {"events": [{"type": "click", "pos": <center>}]}`
  → `ui_find {"text": "Open"}` → click (or `{"type": "key", "key": "Escape"}` to close).
* Move a dialog: `ui_find {"text": "Title bar"}` → `ui_input {"events": [{"type": "drag", "from": [x, y], "to": [x - 160, y + 90]}]}`;
  buttons in the dialog: `ui_find {"text": "OK", "role": "button"}`.
* Zoom like with the mouse: `ui_input {"events": [{"type": "wheel", "delta": [0, 1], "pos": [640, 420], "modifiers": ["ctrl"]}]}`.
* Type text: `ui_action {"tool": "tool.text"}` → `ui_input {"events": [{"type": "click", "pos": [300, 300]}, {"type": "type", "text": "Hello"}, {"type": "key", "key": "Escape"}]}`.
* Drag a file onto the canvas: `ui_input {"events": [{"type": "drop_files", "paths": ["C:\\images\\logo.png"], "pos": [500, 380]}, {"type": "wait", "ms": 400}]}`.
* Select several objects with Shift held: `{"type": "modifiers", "modifiers": ["shift"]}`, clicks, `{"type": "modifiers", "modifiers": []}`.

For document changes without the user interface, `run_command`/`tool_event` are faster and more
precise; `ui_input` is meant for checking and operating the user interface itself.

## 5b. File dialogs in the running app

File dialogs no longer block the app: while one is open, Firepixel keeps drawing and the bridge keeps
answering. Canvas, menus and shortcuts are locked as with a modal dialog
(`ui_state` → `dialog_open`, `file_dialog`).

* **AI control channel active** (bridge running): instead of the Windows dialog, a dialog inside the app
  with a path field appears. `ui_file_dialog {}` shows the kind (`open`, `open_many`, `save`, `folder`),
  title, filters, start folder, suggested name and the latest events. Answer it with
  `{"action": "choose", "path": "…"}` (several for `open_many`: `"paths": […]`) or `{"action": "cancel"}`.
  The user can also fill in the same dialog.
  Extra check boxes (only in the dialog inside the app; `ui_file_dialog {}` shows their state):
  `"append_fw_png"` ("Append .fw.png", Save As with type FW-PNG), `"open_untitled"` ("Open as Untitled")
  and `"open_as_animation"` ("Open as Animation", takes effect from two files on). They apply together with
  `choose`, e.g. `{"action": "choose", "paths": ["a.png", "b.png"], "open_as_animation": true}`, or on
  their own with `{"action": "set", "open_untitled": true}` (the dialog stays open).
* **Without the bridge** or with `FP_FILE_DIALOG=native`: the Windows dialog runs in its own thread, owned
  by the main window (centered above it, modal, no taskbar button of its own). `ui_file_dialog` can
  answer it then as well, but the Windows window stays visible until it is closed
  (`native_window_still_open`); its result is discarded.
* **Test mode** (environment variable before starting): `FP_FILE_DIALOG=cancel` cancels every dialog
  immediately. `FP_FILE_DIALOG=path:C:\folder` answers it immediately: Save with the suggested name in the
  folder, Open with the first matching file (multiple selection: all matching ones), folder with the
  folder itself. Every event is logged and listed in `ui_file_dialog` → `last_events`. The old name
  `FP_DATEIDIALOG` (values `abbrechen`, `pfad:`, `nativ`) is still read.

Workflow "Save As" through the user interface:
`ui_action {"run": "file.saveAs"}` → `ui_file_dialog {}` → `ui_file_dialog {"action": "choose", "path": "C:\\out\\image.fw.png"}`
→ `ui_state {}` (title without asterisk). Without the user interface it is direct: `save {"path": …}`.
File type "Flattened PNG" (current page flattened with the document's optimize settings, format PNG,
no private chunks): `ui_file_dialog {"action": "choose", "path": "C:\\out\\flat.png", "filter": 4}`
(index into `filters`), plus `"save_as_copy": true` to keep the document's name and type; without the
user interface `run_command {"name": "file.saveAs", "params": {"path": "C:\\out\\flat.png", "flattened": true, "copy": false}}`.
Without a copy the document belongs to the PNG file afterwards, and `file.save` writes it flattened again.

## 6. Commands menu and saved commands

The **Commands** menu shows the commands folder (`%APPDATA%\Firepixel\Commands`, subfolders =
submenus). On first start Firepixel puts its own commands there (batch commands, creative, document,
selection, text, web, resizing selected objects, resetting warning dialogs). "Run Script..." starts a
`.jsf` file as **one** history step. Steps saved from the History panel (menu "Save as Command") are
stored as `fp.run(…)` lines in a `.jsf` file. Agents achieve the same with `commands.runScript`,
`commands.run` and `commands.manageSavedCommands`.

## 7. Command line

```
firepixel-cli commands [--filter TEXT] [--category CAT] [--all] [--json]
firepixel-cli info <file> [--json] [--depth N] [--all-pages] [--types text,path]
firepixel-cli run <file>|--new 800x600[:#color] --cmd <name> [--params <json>|@file] … [--steps <json>] --out <file>|--in-place
firepixel-cli batch --actions <json|file> --in <folder> --out <folder> [--format png|fwpng|gif|jpg|webp|…]
firepixel-cli render <file> [--page N] [--state N] [--zoom Z] [--rect l,t,r,b] [--element ID] --out x.png
firepixel-cli convert <in> <out>
firepixel-cli script <x.jsf> [file] [--code JS] [--out file] [--arg VALUE]
firepixel-cli mcp [--http [--port N] [--token T] [--token-file FILE]] | --bridge [--token-file FILE]
firepixel-cli language check|template|list …   (old names: sprache pruefen|vorlage|liste)
```

`--json` gives machine-readable output everywhere; exit code 0 = success, 1 = error, 2 = wrong call.
`firepixel-cli <command> --help` shows examples; `--lang en|de` (or `FIREPIXEL_LANG`) chooses the
language of help and output. Parameters with spaces are easiest to pass as a file under PowerShell:
`--params @p.json`.
