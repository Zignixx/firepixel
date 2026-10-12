# Meldungen: Fehler (err-), Status-/Infomeldungen und Verlaufsschritte (msg-), KI-Auswahl (ai-).
# Zuständig: I18N-ENGINE. Deutsch ist die Ausgangssprache; Englisch nach GLOSSAR.md.

## Fehlerarten von fp-engine (EngineError, Text = Display)
err-unknown-command = unknown command “{ $name }”
err-not-implemented = command “{ $name }” is not implemented yet
err-unavailable = command “{ $name }” is not available: { $reason }
err-invalid-params = invalid parameters for “{ $name }”: { $message }
err-no-document = no document open
err-selection = no suitable selection: { $detail }
err-element-not-found = element { $id } not found
err-not-found = { $what } not found
err-io = File: { $detail }
err-format = FW-PNG: { $detail }
err-backend-missing = this operation needs { $what }
err-failed = { $detail }

## Voraussetzungen von Befehlen (Begründung bei „nicht verfügbar“)
err-req-document = no document open
err-req-selection = no selection
err-req-min-selected = select at least { $n } objects
err-req-all-of = selection does not contain only { $kind }
err-req-any-of = selection contains no { $kind }
err-req-clipboard = clipboard empty
err-req-has-path = document has not been saved yet
err-req-multiple-pages = only one page
err-req-multiple-frames = only one state

## Parameterprüfung ($key = Parametername)
err-param-missing = required parameter “{ $key }” is missing
err-param-invalid = parameter “{ $key }” has an invalid value
err-param-bad-number = { $key }: invalid number
err-param-not-number = { $key }: not a number
err-param-number-expected = { $key }: number expected
err-param-absent = { $key } missing
err-param-negative = { $key }: negative
err-param-bool-expected = { $key }: true/false expected
err-param-text-expected = { $key }: text expected
err-param-hex-color-expected = { $key }: color #rrggbb expected
err-param-color-expected = { $key }: color expected
err-param-point-expected = { $key }: point {"{"}x, y{"}"} expected
err-param-rect-expected = { $key }: rectangle expected
err-param-ints-expected = { $key }: integers expected
err-param-id-expected = { $key }: ID expected
err-param-ids-expected = { $key }: list of IDs expected
err-param-points-expected = { $key }: list of points expected
err-param-detail = { $key }: { $detail }

## Nicht gefunden – mit Namen
err-nf-page-n = Page { $n }
err-nf-tool = Tool “{ $name }”
err-nf-shortcut-set = Keyboard shortcut set “{ $name }”
err-nf-extension = Extension “{ $name }”
err-nf-saved-command = Saved command “{ $name }”
err-nf-symbol-named = Symbol “{ $name }”
err-nf-symbol-id = Symbol { $id }
err-nf-style = Style “{ $name }”
err-nf-brush = Brush stroke “{ $name }”
err-nf-preset = Preset “{ $name }”
err-nf-folder = Folder { $path }
err-nf-html-file = HTML file { $path }

## Weitere Fehler mit Werten
err-unsaved-discard = “{ $name }” has unsaved changes (discard: true to discard)
err-unsaved = “{ $name }” has unsaved changes
err-invalid-name = invalid name “{ $name }”
err-unknown-model = unknown AI model “{ $name }”
err-model-wrong-task = “{ $name }” is not a model for this task
err-unknown-blend = unknown blending mode { $name }
err-unknown-effect = unknown effect { $name }
err-unknown-preset = unknown preset “{ $name }”
err-unknown-format = unknown format “{ $name }”
err-unknown-palette = unknown palette “{ $name }”
err-unknown-export-kind = unknown export type “{ $name }”
err-unknown-shortcut = unknown shortcut “{ $name }”
err-builtin-set-readonly = built-in sets are read-only: first create your own set with “duplicate”
err-unknown-tool = unknown tool “{ $name }”
err-unknown-shape = unknown shape { $name }
err-unknown-stroke-preset = unknown stroke preset { $name }
err-invalid-key = invalid key { $name }
err-prefs = Preferences: { $detail }
err-program-start = program could not be started: { $detail }
err-scan-unavailable = image acquisition not available: { $detail }
err-dictionary = Dictionary { $path }: { $detail }
err-brush-detail = Brush: { $detail }
err-page-missing = page { $n } is missing
err-invalid-set-name = “{ $name }” is not a valid name for a custom set
err-set-not-found = set “{ $name }” not found
err-style-library = Style library: { $detail }
err-shortcut-menu-needs-modifier = Menu shortcuts must include modifier keys.
err-shortcut-tool-plain = Tool shortcuts must consist of letters or digits without modifier keys.
err-shortcut-builtin-no-delete = built-in sets cannot be deleted
err-renderer-unavailable = renderer not available

## Import (fp-io), Export (fp-export)
err-import-unknown-format = unknown file format
err-import-unsupported = { $detail }
err-import-corrupt = file damaged: { $detail }
err-import-too-large = image too large: { $w }×{ $h }
err-import-decode = decoding error ({ $codec }): { $detail }
err-import-fwpng = FW-PNG: { $detail }
err-import-io = I/O error: { $detail }
err-export-bad-size = invalid image size { $w }×{ $h } for this format
err-export-unsupported = format not supported: { $detail }
err-export-encode = encoding error: { $detail }
err-export-render = rendering failed: { $detail }
err-export-io = I/O error: { $detail }

## Skripte (fp-script)
err-script = Script: { $detail }
err-script-failed = script error
err-script-doc-closed = document is closed
err-script-doc-n-closed = document { $id } is closed

## KI-Auswahl (fp-ai)
ai-err-model-missing = AI model “{ $name }” is missing ({ $file }). Please load it via “Manage Models” or put the file into the model folder: { $folder }
ai-err-weights = invalid AI model: { $detail }
ai-err-download = download failed: { $detail }
ai-err-download-disabled = download is disabled (test or batch mode)
ai-err-checksum = checksum of file { $file } does not match (expected { $expected }, got { $got })
ai-err-cancelled = cancelled
ai-err-input = { $detail }
ai-err-io = File: { $detail }

## Hinweise und Rückgaben von Befehlen
msg-wizard-animated-gif = several states: animated GIF
msg-wizard-png32-edit = further editing: lossless PNG32
msg-wizard-png8 = few colors (graphic): PNG8 with exact color count
msg-wizard-png32-alpha = many colors with transparency: PNG32
msg-wizard-jpeg = many colors (photo): JPEG quality 80
msg-spelling-no-dictionary = no dictionary found: specify <language>.aff/.dic via “Spelling Setup”
msg-no-usage-data = Firepixel neither collects nor sends usage data.

## Standardnamen neuer Ebenen, Seiten und Status
msg-default-layer = Layer { $n }
msg-default-web-layer = Web Layer
msg-default-page = Page { $n }
msg-default-state = State { $n }
msg-default-background = Background
msg-default-copy = { $name } copy
msg-default-copy-n = { $name } copy { $n }
msg-default-sublayer = Sub-layer
msg-tool-roundness = Roundness { $px } px

## Namen von Verlaufsschritten (Verlauf-Bedienfeld, „Rückgängig …“)
msg-step-add-popup = Add Pop-up Menu
msg-step-clear-overrides = Clear Overrides
msg-step-duplicate-layer = Duplicate Layer
msg-step-duplicate-state = Duplicate State
msg-step-edit-popup = Edit Pop-up Menu
msg-step-edit-symbol = Edit Symbol
msg-step-edit-text = Edit Text
msg-step-export-settings = Export Settings
msg-step-export-wizard = Export Wizard
msg-step-html-setup = HTML Setup
msg-step-import = Import
msg-step-move = Move
msg-step-move-copy = Move Copy
msg-step-new-layer = New Layer
msg-step-refine-edge = Refine Edge
msg-step-repeat-filter = Repeat Filter
msg-step-spelling = Spelling

## Statushinweise der Werkzeuge
msg-tool-knife-needs-paths = Knife: select paths first.
msg-tool-needs-path = Select a path first.
msg-tool-refine-edge-needs-pixels = Refine Edge: first create a pixel selection (e.g. with “Select Subject” or “Object Selection”).
msg-tool-scrubber-needs-path = Path Scrubber: select a path first.
msg-tool-stamp-needs-source = Rubber Stamp: first Alt+click to set the source.
msg-tool-symbol-edit-pending = Edit Symbol: symbol editing mode is coming.

## Nicht gefunden ($what in err-not-found)
err-nf-active-layer = Active layer
err-nf-contour = Contour
err-nf-document = Document
err-nf-edit-view = Editing view
err-nf-effect = Effect
err-nf-guide = Guide
err-nf-jpeg-mask = JPEG mask
err-nf-layer = Layer
err-nf-mask = Mask
err-nf-page = Page
err-nf-point = Point
err-nf-recent-file = Recent file
err-nf-saved-selection = Saved selection
err-nf-segment = Segment
err-nf-shared-layer = Shared layer
err-nf-state = State
err-nf-swatch = Swatch
err-nf-symbol = Symbol

## Fehlende Bausteine ($what in err-backend-missing)
err-backend-autoshape-runtime = the Auto Shape runtime (fp-script)
err-backend-pathops = path operations (fp-vector)
err-backend-renderer = renderer (fp-render)
err-backend-renderer-page = renderer (fp-render) for the page view
err-backend-renderer-vector = renderer (fp-render) for vector objects
err-backend-script-runtime = the script runtime (fp-script)
err-backend-text-layout = text layout
err-backend-text-renderer = a renderer for text layout

## Voraussetzungen und Verlauf
err-req-can-redo = nothing to redo
err-req-can-undo = nothing to undo
err-req-pixel-selection = no pixel selection
err-req-state = not possible in the current state

## Auswahl ($detail in err-selection)
err-sel-bitmap-locked = the bitmap is locked
err-sel-clipboard-no-object = clipboard contains no object
err-sel-empty = empty
err-sel-empty-pixels = empty pixel selection
err-sel-group-no-mask = the group has no mask
err-sel-instances = instances
err-sel-mask-needs-bitmap = mask output needs a bitmap as source (select a bitmap)
err-sel-no-attributes = no attributes on the clipboard
err-sel-no-bitmap = no bitmap
err-sel-no-bitmap-at-point = no bitmap selected or under the point
err-sel-no-bitmap-selected = no bitmap selected
err-sel-no-bitmap-under-stroke = no bitmap under the stroke
err-sel-no-bitmap-under-wand = no bitmap under the Magic Wand
err-sel-no-bounds = selection has no bounds
err-sel-no-bounds-short = no bounds
err-sel-no-crop-area = no crop area
err-sel-no-instance = no symbol instance selected
err-sel-no-mask-group = no mask group selected
err-sel-no-object = no object
err-sel-no-object-below = no object below
err-sel-no-objects = no objects selected
err-sel-no-objects-with-bounds = no objects with bounds
err-sel-no-path = no path
err-sel-no-paths = no paths
err-sel-no-paths-selected = no paths selected
err-sel-no-popup = selected object has no pop-up menu
err-sel-no-single-open-contour = path has no single open contour
err-sel-no-stroked-object = no object with a stroke selected
err-sel-no-styled-object = no object with a style selected
err-sel-no-subject = no subject found
err-sel-no-text = no text
err-sel-no-text-on-path = no text on a path selected
err-sel-no-text-selected = no text selected
err-sel-no-web-object = no slice/hotspot
err-sel-nothing = nothing selected
err-sel-nothing-to-copy = nothing to copy
err-sel-nothing-to-filter = nothing selected to filter
err-sel-page-empty = page is empty
err-sel-point-outside-bitmap = point outside the bitmap
err-sel-same-symbol = select instances of the same symbol
err-sel-text-and-path = select a text and a path
err-sel-two-instances = select exactly two instances
err-sel-two-objects = select at least two objects
err-sel-two-paths = select at least two paths
err-sel-two-paths-needed = at least two paths needed

## Parameter ($message in err-invalid-params)
err-param-action-ext = action: list/install/remove
err-param-ai-prompt = click point (point/points) or box (box) missing
err-param-anchor-or-rect = specify anchor or rect
err-param-colors = colors: #rrggbb expected
err-param-contours = contours: list expected
err-param-crop = crop: rectangle
err-param-data = data: object expected
err-param-dither = dither: number, true/false or "2x2"
err-param-effect-or-index = specify effect or index
err-param-effects = effects: list expected
err-param-empty-rect = empty rectangle
err-param-four-points = four points expected
err-param-items = items: list of {"{"}text, url, target, submenu{"}"}
err-param-items-empty = items: at least one entry
err-param-matrix = matrix with 6 or 9 values expected
err-param-matte = matte: color
err-param-menu = menu: object expected
err-param-name-missing = name missing
err-param-no-contours = no contours
err-param-no-points = no points
err-param-no-style = no style specified
err-param-node = node: {"{"}anchor, in, out{"}"} or [x, y]
err-param-nodes-missing = nodes missing
err-param-not-object = parameters must be a JSON object
err-param-output = output: "selection" or "mask"
err-param-paper = paper: A3/A4/A5/Letter/Legal or [w, h]
err-param-params = params: object expected
err-param-path-needed = document has no file name yet: specify parameter “path”
err-param-path-or-code = specify path or code
err-param-points-list = points: list expected
err-param-points-numbers = points: numbers expected
err-param-points-triples = points: [x, y, label] expected
err-param-radius-zero = radius 0
err-param-replace = replace: list
err-param-settings = settings: object expected
err-param-settings-missing = settings missing
err-param-three-points = at least three points
err-param-transparency = transparency: none/index/alpha/rgba
err-param-two-points = at least two points
err-param-value = value
err-param-values = values: object expected

## Sonstige Fehler
err-bitmap-unreadable = bitmap could not be read
err-bitmap-unreadable-short = bitmap
err-compound-insert-failed = compound shape could not be inserted
err-cut-misses-path = The cut line does not hit any selected path.
err-degenerate-quad = degenerate quadrilateral
err-distribute-failed = distribute failed
err-element-insert-failed = element could not be inserted
err-empty-area = empty area
err-file-no-objects = the file contains no objects
err-file-no-symbols = the file contains no symbols
err-format-no-palette = the export format has no palette
err-guides-locked = guides are locked
err-image-insert-failed = image could not be inserted
err-internal = internal
err-layer-no-delete = this layer cannot be deleted
err-no-filename = no file name
err-no-filter-yet = no filter applied yet
err-no-html = no HTML generated
err-no-master-page = no master page
err-no-new-object = no new object
err-no-page = no page
err-no-scanner = no scanner selected
err-no-shared-layer = no shared layer active
err-no-size-optimize = this format has no size optimization (only GIF/PNG 8 and JPEG)
err-no-steps = no steps selected
err-no-symbol-edit = no symbol editing
err-no-symbols = no symbols
err-no-valid-page = no valid page
err-not-library-symbol = not a symbol from a library file
err-object-move-failed = object could not be moved
err-only-own-layers = only own layers can be duplicated
err-parent-list = parent list
err-paste-failed = paste failed
err-scan-cancelled = scanning cancelled
err-scan-shell-disabled = image acquisition: starting external programs is disabled (FP_NO_SHELL_OPEN)
err-scan-windows-only = image acquisition is only available on Windows
err-steps-not-replayable = the selected steps cannot be replayed
err-stroke = stroke
err-stroke-misses-path = The stroke does not touch any selected path.
err-view-empty = view is empty
err-web-layer-no-duplicate = the Web Layer cannot be duplicated
err-web-layer-no-share = the Web Layer cannot be shared
