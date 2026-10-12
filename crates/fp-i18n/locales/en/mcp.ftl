# Replies of the MCP server to AI agents (fp-mcp) – agent AU
msg-mcp-legend = params: * = required; available = can run in the current state
err-mcp-param-missing = Parameter { $param } is missing
err-mcp-unknown-command = Unknown command “{ $name }”; similar: { $similar }
err-mcp-unknown-tool = Unknown tool “{ $name }”
msg-mcp-categories-hint = call list_commands with {"{"}"category":"draw"{"}"} or {"{"}"filter":"…"{"}"}; parameters with describe_command

# >>> UE-3 Werkzeuge
# MCP server tools: title, description (.description) and parameters (mcp-tool-<tool>-<parameter>), UE-3
mcp-doc-arg = Document ID from get_document/documents; default: active document. If set, it becomes the active document.
mcp-tool-list_commands = List commands
    .description = Lists commands of the Firepixel command registry (the same commands as the menus, tools and panels of the user interface). Names follow the scheme <menu>.<item>[.<subitem>] in camelCase, e.g. file.open, edit.duplicate, modify.align.left, modify.transform.flipHorizontal, filter.blur.gaussian; plus tool/panel commands such as draw.rectangle, draw.text, element.setFill, layer.add, select.set. Without category and filter you get an overview of the categories with counts. Examples: {"{"}"category":"draw"{"}"} · {"{"}"filter":"align"{"}"} · {"{"}"category":"modify.transform","available_only":true{"}"}. Then call describe_command for the parameters.
mcp-tool-list_commands-category = Name prefix (file, edit, view, select, modify, text, commands, filter, window, help, draw, element, layer, page, state, path, bitmap, color, tool, history, web, guide, transform) or menu name in the UI language (Modify, Filters …)
mcp-tool-list_commands-filter = Search text in name, label, status text and JS names (case-insensitive)
mcp-tool-list_commands-include_unimplemented = also show menu placeholders without effect
mcp-tool-list_commands-available_only = only commands that can run in the current state (selection, document …)
mcp-tool-describe_command = Describe command
    .description = Full description of a command: label, menu path, JSON schema of the parameters (types, required, defaults, ranges), availability predicates (e.g. “at least 2 objects selected”) with their current status, keyboard shortcut in the active set, JS equivalents and a call example. Also accepts JS names (addNewRectangle) and suggests similar names for typos. Example: {"{"}"name":"modify.align.left"{"}"}
mcp-tool-describe_command-name = Command name, e.g. draw.rectangle
mcp-tool-run_command = Run command
    .description = Runs a command (like a menu click/tool in the user interface) and creates an undo step. Returns the return value (e.g. {"{"}"id": 17{"}"} for new objects), the new selection with bounds, history counter and document generation. Most commands act on the current selection – use select first or pass `ids` where the command offers it. Coordinates are document pixels; rectangles [left, top, right, bottom] or {"{"}left, top, right, bottom{"}"}; colors "#rrggbb" or "#rrggbbaa". With `steps` several commands run as ONE undo step (on error everything is rolled back). Examples: {"{"}"name":"draw.rectangle","params":{"{"}"rect":[20,20,220,120],"fillColor":"#3366ff","strokeWidth":0{"}"}{"}"} · {"{"}"name":"draw.text","params":{"{"}"text":"Hello","anchor":[30,80],"size":24,"color":"#000000"{"}"}{"}"} · {"{"}"name":"modify.align.centerHorizontal"{"}"} · {"{"}"name":"filter.blur.gaussian","params":{"{"}"radius":3{"}"}{"}"} · {"{"}"steps":[{"{"}"name":"select.all"{"}"},{"{"}"name":"modify.group"{"}"}]{"}"}
mcp-tool-run_command-name = Command name (list_commands/describe_command)
mcp-tool-run_command-params = Parameters following the schema from describe_command
mcp-tool-run_command-steps = several commands as one undo step
mcp-tool-run_command-label = History label for steps
mcp-tool-get_document = Read document tree
    .description = Reads the document tree as JSON: pages (size, canvas color, states), layers in original order bottom→top (web layer last) with IDs, elements top→bottom with stable ID, type (rectangle, path, text, bitmap, group, maskGroup, autoShape, compoundShape, instance, slice, hotspot), name, bounds {"{"}left, top, right, bottom, width, height{"}"}, opacity (%), blending mode, fill/stroke, text/font, effects, symbol; plus selection, active layer, history counter and the list of open documents. Narrow down large documents with depth, types, name, selected_only or page. Examples: {"{"}{"}"} · {"{"}"depth":1{"}"} · {"{"}"types":["text"]{"}"} · {"{"}"selected_only":true{"}"} · {"{"}"all_pages":true,"depth":0{"}"}
mcp-tool-get_document-page = page shown in detail (default: current)
mcp-tool-get_document-state = state (frame) whose content is shown
mcp-tool-get_document-depth = 0 = layers only, 1 = elements of the layers, 2+ = group content too
mcp-tool-get_document-types = only these element types
mcp-tool-get_document-name = only elements whose name contains the text
mcp-tool-select = Select
    .description = Sets the object selection most commands act on. Give exactly one kind: ids (element IDs from get_document), all, none, point (topmost object under the point) or rect (objects completely inside the rectangle). mode: replace (default), add, subtract, intersect, toggle. Returns the new selection with type, name and bounds. Examples: {"{"}"ids":[12,15]{"}"} · {"{"}"all":true{"}"} · {"{"}"point":[50,40]{"}"} · {"{"}"rect":[0,0,200,100],"mode":"add"{"}"} · {"{"}"none":true{"}"}
mcp-tool-select-point = {"["}x, y] or {"{"}x, y{"}"}
mcp-tool-select-rect = {"["}l, t, r, b] or {"{"}left, top, right, bottom{"}"}
mcp-tool-select-deep = with point, descend into groups
mcp-tool-render = Render (image)
    .description = Renders offscreen (without a window) a page, a region or a single element as a PNG image and returns it as image content – to check the result after changes. zoom 2 = 200 % (sharp pixels), 0.5 = reduced preview. Large outputs are reduced automatically (max_pixels). Examples: {"{"}{"}"} · {"{"}"zoom":0.5{"}"} · {"{"}"rect":[0,0,100,100],"zoom":4{"}"} · {"{"}"element":17{"}"} · {"{"}"page":1,"state":2{"}"}
mcp-tool-render-rect = Region [l, t, r, b] in document pixels
mcp-tool-render-element = only this element (transparent background)
mcp-tool-open = Open file
    .description = Opens a file as the new active document: FW-PNG (.fw.png or PNG with original data, editable), PNG, PSD (layers), GIF, JPEG, BMP, TIFF, TGA, WebP, JPEG XL, QOI, ICO, SVG, AI/EPS. Foreign formats open as an untitled document (save with save {"{"}path{"}"}). Give absolute paths. Returns the document ID and a summary. Example: {"{"}"path":"C:\\Images\\draft.fw.png"{"}"}
mcp-tool-save = Save
    .description = Saves the document as FW-PNG (editable, with a flat preview image). Without path under the current name; with path like “Save As”. For flat images use export. Example: {"{"}"path":"C:\\out\\result.fw.png"{"}"}
mcp-tool-export = Export
    .description = Exports a flat image without changing the document name. Format from the extension or format: png (PNG32; also region rect, element, zoom), gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng (whole page through the original optimization; quality/colors optional) or fwpng (editable copy). Examples: {"{"}"path":"C:\\out\\page.png"{"}"} · {"{"}"path":"C:\\out\\logo.png","element":17,"zoom":2{"}"} · {"{"}"path":"C:\\out\\photo.jpg","quality":85{"}"} · {"{"}"path":"C:\\out\\icon.gif","colors":32{"}"}
mcp-tool-export-format = png, fwpng, gif, jpg, webp, avif, bmp, tiff, qoi, ico, apng (default: extension)
mcp-tool-export-quality = JPEG/WebP/AVIF quality
mcp-tool-export-colors = Number of colors for GIF/PNG8
mcp-tool-export-rect = Region [l, t, r, b]
mcp-tool-undo = Undo
    .description = Undoes the last steps in the active document (default 1). Example: {"{"}"steps":3{"}"}
mcp-tool-redo = Redo
    .description = Redoes undone steps (default 1). Example: {"{"}{"}"}
mcp-tool-history = History
    .description = Shows the history of the active document (labels, position, commands with parameters for replaying) or jumps to a position with goto (number of executed steps). Examples: {"{"}{"}"} · {"{"}"goto":2{"}"}
mcp-tool-new_document = New document
    .description = Creates a new empty document and activates it. Examples: {"{"}"width":800,"height":600{"}"} · {"{"}"width":64,"height":64,"background":"#00000000"{"}"} (transparent)
mcp-tool-new_document-background = #rrggbb or #rrggbbaa (default white)
mcp-tool-new_document-resolution = Pixels/inch (default 72)
mcp-tool-tool_event = Operate tool
    .description = Operates a tool like a person with the mouse: optionally selects the tool (tool: pointer, rectangle, ellipse, polygon, line, pen, text, brush, pencil, eraser, marquee, ovalMarquee, lasso, magicWand, paintBucket, gradient, eyedropper, scale, skew, distort, crop, slice, hotspotRect, hand, zoom …) and sends pointer events in document coordinates to the tool state machine. drag is a shortcut for down → move(s) → up. Modifiers as in the original: shift (square/45°), alt (from the center/copy), space (move while dragging out). Executed commands go into the history. Examples: {"{"}"tool":"rectangle","drag":{"{"}"from":[10,10],"to":[110,60]{"}"}{"}"} · {"{"}"tool":"pen","events":[{"{"}"type":"down","x":10,"y":10{"}"},{"{"}"type":"up","x":10,"y":10{"}"},{"{"}"type":"down","x":90,"y":10{"}"},{"{"}"type":"up","x":90,"y":10{"}"},{"{"}"type":"double","x":90,"y":80{"}"}]{"}"} · {"{"}"tool":"text","events":[{"{"}"type":"down","x":20,"y":40{"}"},{"{"}"type":"up","x":20,"y":40{"}"},{"{"}"type":"text","text":"Hello"{"}"},{"{"}"type":"key","key":"escape"{"}"}]{"}"}
mcp-tool-tool_event-tool = Tool name (tool.rectangle, rectangle, Rectangle)
mcp-tool-documents = Open documents
    .description = Lists open documents (ID, title, path, unsaved?) and optionally activates or closes one. Examples: {"{"}{"}"} · {"{"}"activate":2{"}"} · {"{"}"close":3,"discard":true{"}"}
mcp-tool-run_script = Run JSF script
    .description = Runs JSF JavaScript (.jsf, fw/dom API like the original) in Firepixel – code directly or a file. Returns the result, alert() texts and errors. API members that are not implemented throw an exception “not implemented: <name>”. Example: {"{"}"code":"var d = fw.getDocumentDOM(); d.addNewRectangle({"{"}left:0,top:0,right:50,bottom:50{"}"}, 0); d.width"{"}"}
mcp-tool-screenshot = Screenshot of the user interface
    .description = Bridge mode only (running Firepixel user interface): screenshot of the main window as PNG – shows menus, panels, Properties, the canvas with selection frames and open dialogs exactly as the user sees them. For the pure document image use render headless instead. Examples: {"{"}{"}"} · {"{"}"rect":[0,0,800,600]{"}"} (region in window points) · {"{"}"max_size":1200{"}"}
mcp-tool-screenshot-rect = Region [l, t, r, b] in window points
mcp-tool-screenshot-max_size = longer edge at most this many pixels (reduced)
mcp-tool-ui_state = State of the user interface
    .description = Bridge mode only: state of the running user interface – active tool, colors, zoom and scroll position per document, window size, visible panels, theme, open dialogs, status bar, position of the canvas in the window (for screenshot coordinates). Example: {"{"}{"}"}
mcp-tool-ui_action = User interface action
    .description = Bridge mode only: triggers an action in the user interface like a person (including dialogs, which then open in the window): {"{"}"run":"modify.canvas.imageSize"{"}"} (menu command through the user interface, dialog commands open their dialog), {"{"}"zoom":2{"}"} (200 %), {"{"}"fit":true{"}"} (fit in window), {"{"}"panel":"window.layers"{"}"} (show panel), {"{"}"tool":"tool.pen"{"}"}, {"{"}"theme":"dunkel"{"}"}, {"{"}"close_dialogs":true{"}"}, {"{"}"status":"text in the status bar"{"}"}. For document changes without a dialog, run_command is better.
mcp-tool-ui_input = Inject input (mouse, keyboard, files)
    .description = Bridge mode only: injects raw input events into the running user interface – the same way as a real mouse/keyboard (egui input before each frame, exactly as egui-winit delivers it on Windows: Ctrl+C/X/V become Copy/Cut/Paste, Ctrl+V without text on the clipboard only delivers the release, text only without Ctrl). Coordinates in window points (like ui_find and screenshot rect). Each step runs in its own frames; the reply comes when everything has been processed. Events (`events`, in order): {"{"}"type":"move","pos":[x,y]{"}"} · {"{"}"type":"click","pos":[x,y],"button":"left|right|middle","count":1|2,"modifiers":["ctrl","shift","alt"]{"}"} · {"{"}"type":"down"|"up","button":"left","pos":[x,y]{"}"} · {"{"}"type":"drag","from":[x,y],"to":[x,y],"steps":12,"ms":16,"modifiers":[]{"}"} · {"{"}"type":"wheel","delta":[0,1],"unit":"line|point","pos":[x,y],"modifiers":["ctrl"]{"}"} (positive = up/zoom in) · {"{"}"type":"key","key":"C","modifiers":["ctrl"],"action":"tap|press|release"{"}"} (names as in egui: A–Z, 0–9, F1, Enter, Escape, Tab, Delete, ArrowLeft …) · {"{"}"type":"modifiers","modifiers":["shift"]{"}"} (set held modifier keys, [] releases) · {"{"}"type":"text","text":"abc"{"}"} · {"{"}"type":"type","text":"Hello"{"}"} (key+text per character) · {"{"}"type":"copy"|"cut"|"paste","text":"…"{"}"} · {"{"}"type":"drop_files","paths":["C:/x.png"],"pos":[x,y]{"}"} · {"{"}"type":"hover_files","paths":[…]{"}"} · {"{"}"type":"wait","ms":200{"}"} · {"{"}"type":"frames","n":2{"}"} · {"{"}"type":"screenshot","rect":[l,t,r,b]{"}"} · {"{"}"type":"pointer_gone"{"}"} · {"{"}"type":"focus","focused":true{"}"}. Options: "screenshot": true or [l,t,r,b] (image after the last step), "screenshot_each": true (image after each step, at most 24), "settle_frames": 2, "max_size". Reply: executed steps, widget events reported by egui (clicked, focus), pointer position, open dialogs, status bar. Example Ctrl+C/Ctrl+V: {"{"}"events":[{"{"}"type":"click","pos":[600,400]{"}"},{"{"}"type":"key","key":"C","modifiers":["ctrl"]{"}"},{"{"}"type":"key","key":"V","modifiers":["ctrl"]{"}"}],"screenshot":true{"}"}
mcp-tool-ui_input-events = Steps in order (see description)
mcp-tool-ui_input-screenshot = true or [l, t, r, b]: screenshot after the last step
mcp-tool-ui_find = Find user interface widgets
    .description = Bridge mode only: finds visible controls and labels in the current window image – menu items, buttons, tabs, panel rows, dialog titles – through egui's accessibility info (role, name) and the drawn texts. Returns per hit the text, role, rectangle and center in window points (directly usable for ui_input), topmost hits first. Case-insensitive, partial matches allowed (exact: true for exact). Examples: {"{"}"text":"File"{"}"} · {"{"}"text":"OK","role":"button","exact":true{"}"} · {"{"}"role":"menu_item"{"}"} · {"{"}"text":"Layer 1"{"}"} · {"{"}"region":[0,0,400,60]{"}"}
mcp-tool-ui_find-role = e.g. button, menu_item, check_box, text_input, label, window, tab, slider
mcp-tool-ui_file_dialog = File dialog of the running app
    .description = Bridge mode only: shows whether a file dialog (Open, Save As, Import, Export, folder …) is open in the running app – kind, title, filters, start folder, suggested name, recent events – and answers it. While a dialog is open the app keeps drawing, the canvas and menus are locked like with a modal dialog. With the AI control channel active the app shows its own dialog instead of the native one (FP_DATEIDIALOG=nativ forces the native one). Examples: {"{"}"action":"status"{"}"} · {"{"}"action":"cancel"{"}"} · {"{"}"action":"choose","path":"C:\\out\\banner.fw.png"{"}"} · {"{"}"action":"choose","paths":["C:\\in\\a.png","C:\\in\\b.png"]{"}"}
mcp-tool-ui_file_dialog-path = file or folder for choose
mcp-tool-ui_file_dialog-paths = several files (open dialogs with multiple selection)
# <<< UE-3 Werkzeuge

# >>> UE-3 Meldungen
# MCP server messages, hints and resources (UE-3)
err-mcp-hint-unknown-command = {" "}– look up names with list_commands
err-mcp-hint-invalid-params = {" "}– check the schema with describe_command
err-mcp-hint-unavailable = {" "}– establish the precondition (e.g. select, new_document)
err-mcp-hint-no-document = {" "}– first new_document or open
err-mcp-doc-not-open-hint = Document { $id } is not open – documents lists the open documents
err-mcp-doc-not-open = Document { $id } is not open
err-mcp-step-needs-name = every step needs name
err-mcp-name-or-steps = give name or steps
err-mcp-no-document-hint = no document open – new_document or open
err-mcp-no-document = no document open
err-mcp-select-kind = give ids, all, none, point or rect
err-mcp-nothing-to-undo = nothing to undo
err-mcp-nothing-to-redo = nothing to redo
msg-mcp-history-hint = steps[0..position] are executed; history {"{"}"goto": n{"}"} jumps; commands can be replayed with run_command {"{"}steps:[…]{"}"}
err-mcp-rect-expected = rect: [l, t, r, b] or {"{"}left, top, right, bottom{"}"} expected
err-mcp-drag-from-to = drag needs from and to as [x, y]
msg-mcp-tool-nothing-yet = nothing applied yet – finish the operation with up/double/key enter if needed
err-mcp-unsaved = “{ $title }” has unsaved changes – save or discard: true
err-mcp-scripts-off = Scripts are turned off in this server
err-mcp-code-or-path = give code or path
msg-mcp-agent-steps = Agent steps
err-mcp-open-failed = { $error } (readable: { $formats })
err-mcp-export-failed = { $error } (formats: fwpng, { $formats })
err-mcp-folder = Folder { $path }: { $error }
err-mcp-unknown-key = unknown key “{ $key }”
err-mcp-unknown-event = unknown event type “{ $kind }”
err-mcp-unknown-canvas-tool = unknown tool “{ $name }”; available: { $names }
err-mcp-event = Event { $index }: { $error }
msg-mcp-truncated = only the first { $n } elements; narrow down with depth/types/name/page or raise maxElements
err-mcp-image-empty = empty image
err-mcp-image-size = image data does not match the size
err-mcp-only-http = only http:// addresses: { $url }
err-mcp-port-missing = port missing in { $url }
err-mcp-only-local = only local addresses allowed: { $host }
err-mcp-bad-port = invalid port in { $url }
err-mcp-token-short = token missing or too short
err-mcp-token-file-unreadable = token file { $path } not readable ({ $error }) – is Firepixel running with the AI control channel switched on (firepixel.exe --mcp or FIREPIXEL_MCP=1)?
err-mcp-token-file = token file: { $error }
err-mcp-token-file-field = token file without { $field }
err-mcp-no-connection = no connection to Firepixel on 127.0.0.1:{ $port } ({ $error }) – is the user interface still running?
err-mcp-bad-reply = invalid reply: { $error }
err-mcp-element-not-on-page = element { $id } not on page { $page }
err-mcp-element-invisible = element { $id } has no visible appearance
err-mcp-page-no-canvas = page without canvas
err-mcp-empty-rect = empty region
err-mcp-random = random source: { $error }
err-mcp-token-min = the token must have at least 16 characters
err-mcp-http-thread = HTTP thread ended
err-mcp-host-gone = the session is no longer reachable
err-mcp-io = I/O error: { $error }
err-mcp-protocol = MCP: { $error }
err-mcp-config = invalid setting: { $error }
err-mcp-no-ui = only available in bridge mode with a running user interface (start Firepixel with --mcp or FIREPIXEL_MCP=1); headless, use render instead
err-mcp-timeout = the user interface did not answer in time (window minimized or blocked?)
err-mcp-unknown-resource = unknown resource { $uri }
msg-mcp-instructions = Firepixel is a graphics program for screen and web graphics (vector + bitmap, pages, states, layers, symbols, FW-PNG). Workflow: new_document or open → get_document (IDs, bounds) → select → run_command (commands from list_commands/describe_command) → render (check the image) → save/export. All changes can be undone (undo/history). Command names: <menu>.<item> in camelCase (modify.align.left, filter.blur.gaussian) plus draw.*, element.*, layer.*, page.*, state.*, select.*. Coordinates in document pixels, rectangles [l, t, r, b], colors #rrggbb[aa]. Detailed docs: resources firepixel://docs/agents.md, commands.md, document-model.md, examples.md.
msg-mcp-instructions-bridge = Bridge mode: you control the running user interface, the user sees every change live. screenshot shows the whole window, ui_state the state of the user interface, ui_action triggers user interface actions, ui_find returns positions of menu items/buttons/panel rows, ui_input injects real mouse/keyboard/file events (like a person).
msg-mcp-instructions-headless = Headless session (screenshot/ui_state/ui_action/ui_input/ui_find only in bridge mode).
msg-mcp-cat-title = Firepixel command catalog
msg-mcp-cat-intro = { $n } commands, { $implemented } of them with an effect. Call via MCP `run_command` ({"{"}"name": …, "params": {"{"}…{"}"}{"}"}), CLI `firepixel-cli run --cmd …` or JSF. `*` = required parameter. Placeholders without effect are marked _(not implemented)_.
msg-mcp-cat-unimplemented = not implemented
msg-mcp-cat-needs = needs: { $list }
mcp-res-agents = Guide for AI agents
    .description = Setup, typical workflows, command name scheme, examples
mcp-res-commands = Command catalog
    .description = All commands by category with parameters, required values and availability
mcp-res-document-model = Document model
    .description = Pages, states, layers, elements, IDs, coordinates, colors
mcp-res-examples = Examples
    .description = Ready-made tool calls for common tasks
mcp-res-jsf = JSF compatibility
    .description = Which parts of the JSF JavaScript API are implemented
# JSF compatibility list (resource jsf.md, fp_mcp::resources::jsf_markdown) – UE-4
mcp-res-jsf-title = JSF compatibility
mcp-res-jsf-intro = Implemented: **{ $m } of { $mt } methods**, **{ $p } of { $pt } properties** of the binding table of the original; deliberately left out: { $em } methods, { $ep } properties (reasons below).
mcp-res-jsf-rules = Type rules as in the original: missing arguments come from defaults, otherwise exception `-2147216337` (EBadParamCount); extra arguments are ignored; `"5"` counts as a number, numbers as text; `null` for boolean/int/float throws `-2147216343` (EBadParamType); assignments to read-only properties are ignored; errors are thrown as an HRESULT number. Members that are not implemented throw `Error("not implemented: Class.member")` (message in the reply language).
mcp-res-jsf-extras = In addition: `fp.run(name, params)` runs a command of the command registry (format of "Copy Steps" and saved commands), `fp.available(name)` checks whether it is available in the current state. Auto shapes: `dom.insertSmartShapeAt`, `dom.dragControlPoint`, `dom.redrawSmartShape`; the shape scripts run in a runtime of their own with `smartShape`, `Path`, `Contour`, `ContourNode`, `ControlPoint`.
mcp-res-jsf-class-omitted = **{ $class }**: left out – { $reason }
mcp-res-jsf-class-missing = **{ $class }**: not implemented yet ({ $methods } methods, { $props } properties)
mcp-res-jsf-class-counts = **{ $class }**: methods { $m }/{ $mt }, properties { $p }/{ $pt }
mcp-res-jsf-methods = Methods: { $list }
mcp-res-jsf-props = Properties: { $list }
mcp-res-jsf-member-omitted = left out: { $member } – { $reason }
mcp-res-jsf-why-firepixel-exportswf = Flash export (SWF) – obsolete format, no exporter in Firepixel
mcp-res-jsf-why-firepixel-exportfxg = FXG export (Flex) – obsolete format, no exporter in Firepixel
mcp-res-jsf-why-firepixel-exportmxmlandimages = MXML export (Flex) – obsolete format, no exporter in Firepixel
mcp-res-jsf-why-firepixel-exportdirectorasslices = Director export – obsolete format, no exporter in Firepixel
mcp-res-jsf-why-firepixel-exportdirectoraslayers = Director export – obsolete format, no exporter in Firepixel
mcp-res-jsf-why-firepixel-exportillustrator = Export as Illustrator 8 – no exporter in Firepixel (SVG/PDF via exportPDF or file.export)
mcp-res-jsf-why-style-clipcopysmallpreviewimage = Style preview image to the clipboard – Firepixel creates no style preview images
mcp-res-jsf-why-style-clipcopylargepreviewimage = Style preview image to the clipboard – Firepixel creates no style preview images
mcp-res-jsf-why-behaviorinfo = Object of the HTML export templates of the original; Firepixel exports HTML/behaviors natively (fp-export) and does not run these templates
mcp-res-jsf-why-behaviorinfo-menuitems-i = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-behaviorslist = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-imagemap = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-imagemaplist = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-sliceinfo = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-slices = like BehaviorInfo (HTML export templates)
mcp-res-jsf-why-exportdoc = like BehaviorInfo (HTML export templates)
# <<< UE-3 Meldungen

# >>> UE-3 Brücke
# Replies of the running user interface to AI agents (bridge mode, fp-ui bridge.rs/inject.rs), UE-3
err-mcp-ui-channel-closed = control channel closed
err-mcp-ui-no-screenshot = no screenshot received (window minimized?)
msg-mcp-ui-done-dialogs = dialogs closed
err-mcp-ui-unknown-command = unknown command “{ $name }” – list_commands
msg-mcp-ui-done-command = command { $name }
err-mcp-ui-zoom-range = zoom between 0.01 and 64 (1 = 100 %)
msg-mcp-ui-done-fit = fit in window
msg-mcp-ui-done-panel = panel { $name }
err-mcp-ui-unknown-panel = unknown panel “{ $name }” (e.g. window.layers, window.history)
msg-mcp-ui-done-tool = tool { $name }
err-mcp-ui-unknown-theme = unknown theme “{ $name }” (normal, dunkel, oled)
msg-mcp-ui-done-theme = theme { $name }
msg-mcp-ui-done-status = status bar
err-mcp-ui-no-action = no action given (run, zoom, fit, panel, tool, theme, close_dialogs, status)
err-mcp-ui-modifiers = modifiers: list like ["ctrl","shift"]
err-mcp-ui-unknown-modifier = unknown modifier key “{ $name }” (ctrl, shift, alt)
err-mcp-ui-unknown-button = unknown mouse button “{ $name }” (left, right, middle)
err-mcp-ui-key-missing = key missing (e.g. "C", "Enter", "Escape", "F5")
err-mcp-ui-unknown-key = unknown key “{ $key }” (names as in egui: A, 5, F1, Enter, Escape, Tab, Delete, ArrowLeft, PageUp …)
err-mcp-ui-paths-empty = paths is empty
err-mcp-ui-events-missing = events missing: list of steps, e.g. [{"{"}"type":"click","pos":[100,200]{"}"}]
err-mcp-ui-screenshot-arg = screenshot: true or [l, t, r, b]
err-mcp-ui-step-type = step { $n }: type missing
err-mcp-ui-step = step { $n } ({ $kind }): { $error }
msg-mcp-ui-pointer = (pointer)
msg-mcp-ui-drag = drag { $from } → { $to } ({ $n } steps)
err-mcp-ui-delta = delta missing ([x, y] or number; positive = up)
err-mcp-ui-bad-value = { $name } “{ $value }” ({ $list })
msg-mcp-ui-drop = drop_files { $n } file(s) at { $pos }
err-mcp-ui-too-many-frames = too many frames (> { $max }); split the sequence
err-mcp-ui-unknown-type = unknown type “{ $kind }” ({ $list })
# <<< UE-3 Brücke
