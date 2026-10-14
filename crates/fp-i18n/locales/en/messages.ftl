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
err-update-html-no-table = { $path } is not an HTML page (neither </body> nor </html> found). The file was not changed.

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
ai-err-model-missing = AI model “{ $name }” is missing ({ $file }). Please load it via “Manage AI Models” or put the file into the model folder: { $folder }
ai-err-weights = invalid AI model: { $detail }
ai-err-download = download failed: { $detail }
ai-err-download-disabled = download is disabled (test or batch mode)
ai-err-checksum = checksum of file { $file } does not match (expected { $expected }, got { $got })
ai-err-cancelled = cancelled
ai-err-input = { $detail }
ai-err-io = File: { $detail }

## AI models (fp-ai models.rs; model names stay, additions translated)
ai-model-sam2-tiny = SAM 2.1 Hiera Tiny
ai-model-sam2-small = SAM 2.1 Hiera Small
ai-model-birefnet-lite = BiRefNet lite (general)
ai-model-birefnet = BiRefNet (general, high quality)
ai-model-birefnet-lite-matting = BiRefNet lite Matting
ai-model-birefnet-hr-matting = BiRefNet HR Matting (high quality)
ai-model-sam2-tiny-note = Meta (facebookresearch/sam2): code and weights Apache-2.0, commercial use allowed; trained on SA-1B/SA-V
ai-model-sam2-small-note = Meta (facebookresearch/sam2): code and weights Apache-2.0, commercial use allowed; trained on SA-1B/SA-V
ai-model-birefnet-lite-note = ZhengPeng7/BiRefNet, Swin-T; weights MIT. Training data according to the author include DIS5K, DUTS, HRSOD, UHRSD, HRS10K, P3M-10k, humans (P3M-10k is licensed for non-commercial research only)
ai-model-birefnet-note = ZhengPeng7/BiRefNet, Swin-L (slower); weights MIT. Training data as for BiRefNet lite (including P3M-10k, non-commercial research only)
ai-model-birefnet-lite-matting-note = ZhengPeng7/BiRefNet, Swin-T, for hair and fur; weights MIT. Training data according to the author: P3M-10k, humans, AM-2k, Distinctions-646, HIM2K (partly non-commercial research only)
ai-model-birefnet-hr-matting-note = ZhengPeng7/BiRefNet, Swin-L, for hair and fur (slower); weights MIT. Training data as for Matting lite (partly non-commercial research only)

## Hinweise und Rückgaben von Befehlen
msg-wizard-animated-gif = several states: animated GIF
msg-wizard-png32-edit = further editing: lossless PNG32
msg-wizard-png8 = few colors (graphic): PNG8 with exact color count
msg-wizard-png32-alpha = many colors with transparency: PNG32
msg-wizard-jpeg = many colors (photo): JPEG quality 80
msg-spelling-no-dictionary = no dictionary found: specify <language>.aff/.dic via “Spelling Setup”
msg-no-usage-data = Firepixel neither collects nor sends usage data.
msg-panel-alias = Panel menu: same as “{ $target }”

## Standardnamen neuer Ebenen, Seiten und Status
msg-default-layer = Layer { $n }
msg-default-web-layer = Web Layer
msg-default-page = Page { $n }
msg-default-state = State { $n }
msg-default-background = Background
msg-default-copy = { $name } copy
msg-default-copy-n = { $name } copy { $n }
msg-default-sublayer = Sub-layer
msg-default-document = Untitled-{ $n }
msg-default-document-nameless = Document
msg-tool-roundness = Roundness { $px } px

## Import-Details (fp-io, $detail in err-import-corrupt/-unsupported/-decode)
err-import-act-not-image = Photoshop palettes (*.act) are not images: use them via “Load Palette” in the Optimize panel or in the Swatches panel.
err-import-apng-no-frame = APNG without frame
err-import-av1-aborted = AV1 decoder aborted (damaged data)
err-import-avif-item-missing = AVIF: item missing
err-import-avif-no-image = AVIF without image
err-import-avif-primary-missing = AVIF: primary item missing
err-import-avif-tile-not-av1 = AVIF: grid tile is not an AV1 image
err-import-avif-tiles = AVIF: tile count does not match the grid
err-import-bad-bit-depth = invalid bit depth
err-import-bbox-empty = bounding box with width or height ≤ 0
err-import-decoder-hangs = decoder hangs
err-import-eps-no-bbox = AI/EPS without %%BoundingBox
err-import-field-width = field width
err-import-gif-ext-cut = GIF extension truncated
err-import-gif-no-image = GIF without image
err-import-gif-palette-cut = GIF palette truncated
err-import-gif-subblock-cut = GIF sub-block truncated
err-import-heif-construction = HEIF: item construction by reference is not supported
err-import-hevc-aborted = HEVC decoder aborted (damaged data)
err-import-ico-dib = ICO DIB
err-import-ico-header = ICO header
err-import-ico-no-image = ICO without image
err-import-image-desc-cut = image descriptor truncated
err-import-jpeg-no-header = JPEG without header
err-import-jpeg-short = JPEG data too short
err-import-length = length
err-import-lzw-size-missing = LZW code size missing
err-import-no-image-decoded = no image decoded
err-import-not-gif = not a GIF
err-import-not-qoi = not a QOI
err-import-offset = offset
err-import-pdf-aborted = PDF interpreter aborted (damaged data)
err-import-pdf-encrypted = encrypted PDF files are not supported
err-import-pdf-no-page = PDF without page
err-import-pdf-unreadable = PDF not readable
err-import-plane-short = plane too short
err-import-psd-no-composite = PSD without composite
err-import-sample-offset = sample offset
err-import-signature-missing = signature missing
err-import-tiff-no-colormap = TIFF palette without ColorMap
err-import-tiff-sample-format = TIFF sample format is not supported
err-import-tiff-short = TIFF data too short
err-import-tile-no-image = tile without image
err-import-unknown-header-size = unknown header size
err-import-webp-no-frame = WebP without frame
err-import-eps-no-ai-data = EPS without Illustrator data cannot be read: the original program rasterizes it with its PostScript module (MPS.dll), which Firepixel does not include. Please save it as Illustrator 8 EPS, SVG or pixel image.
err-import-avif-kind = AVIF: image type “{ $kind }” is not supported
err-import-bmp-compression = BMP compression { $comp } with { $bpp } bits is not supported
err-import-clipboard-format = clipboard format “{ $name }” is not supported
err-import-box-length = box “{ $name }” with wrong length
err-import-image-size = image size { $w }×{ $h }
err-import-pixels-short = pixel data too short ({ $have } < { $need })
err-import-tga-type = TGA type { $kind } is not supported
err-import-tiff-color-type = TIFF color type { $kind } is not supported
err-import-wbmp-type = WBMP type { $kind }

## FW-PNG (fp-fwpng) und Pixelpuffer (fp-raster)
err-fwpng-not-png = no PNG signature
err-fwpng-truncated-chunk = PNG chunk truncated at offset { $offset }
err-fwpng-missing-ihdr = PNG without IHDR
err-fwpng-bad-chunk = invalid chunk { $kind }: { $detail }
err-fwpng-tree = tag tree: { $detail } (offset { $offset })
err-fwpng-atxt = aTXT: { $detail } (offset { $offset })
err-fwpng-document = Document: { $detail }
err-fwpng-png-decode = PNG decoding: { $detail }
err-fwpng-png-encode = PNG encoding: { $detail }
err-fwpng-io = I/O: { $detail }
err-raster-too-large = pixel buffer too large: { $w }×{ $h }
err-raster-size-mismatch = data length { $got } does not match the expected length { $expected }

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
msg-step-duplicate = Duplicate
msg-step-move-point = Move Point
msg-step-transform = Transform
msg-step-pen = Pen
msg-step-brush = Brush
msg-step-modify-rect = Modify Rectangle
msg-step-new-layer = New Layer
msg-step-refine-edge = Refine Edge
msg-step-filter-image = Filter Image
msg-step-feather-selection = Feather Selection
msg-step-expand-selection = Expand Selection
msg-step-contract-selection = Contract Selection
msg-step-border-selection = Border Selection
msg-step-smooth-selection = Smooth Selection
msg-step-stamp = Rubber Stamp
msg-step-replace-color = Replace Color
msg-step-red-eye = Red Eye Removal
msg-step-edit-bitmap = Edit Bitmap
msg-step-select-pixels = Select Pixels
msg-step-fill = Fill
msg-step-edit-fill = Edit Fill
msg-step-fill-color = Fill Color
msg-step-stroke-color = Stroke Color
msg-step-repeat-filter = Repeat Filter
msg-step-spelling = Spelling
msg-step-file-info = File Info

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
err-param-ai-value = unknown value “{ $value }”
err-sel-ai-nothing-to-retune = no AI selection to adjust (the selection has changed since)
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

# >>> UE-3 Farben und Schriften
# Errors from fp-color (swatches, CMYK) and fp-text (font files), UE-3
err-color-cmyk-short = CMYK table too short: { $n } bytes, expected { $expected }
err-color-swatch-short = file too short ({ $n } bytes)
err-color-swatch-signature = not a { $format } file (signature missing)
err-color-swatch-truncated = { $format } file truncated
err-color-swatch-no-palette = no palette included
err-text-font-io = font file { $path } not readable: { $error }
err-text-bad-font = invalid font: { $detail }
# <<< UE-3 Farben und Schriften

# >>> UE-3 Import-Hinweise
# Import notes (fp-io, field warnings in the command result), UE-3
msg-import-warn-psd-adjustment = adjustment layer “{ $name }” ({ $kind }) is not applied
msg-import-warn-psd-text = text “{ $name }”: paragraph/character formats only imported in simplified form (A)
msg-import-warn-psd-lab = Lab color mode: values are imported without conversion
msg-import-warn-psd-cmyk = CMYK PSD: naive conversion (the original converts through the PSL library with a profile, 12 §7a T7)
msg-import-warn-svg-pattern = SVG patterns are not imported
msg-import-warn-svg-gradient-stroke = SVG gradient strokes are replaced by their first color (A)
msg-import-warn-svg-gradient-repeat = repeating SVG gradients are imported without repetition
msg-import-warn-svg-gradient-focus = SVG gradients with a moved focal point are imported centered (A)
msg-import-warn-svg-image = embedded SVG images of this type are not imported
msg-import-warn-svg-clip = SVG clipping paths, masks and filters are not imported
msg-import-warn-svg-text = SVG text on a path is imported as plain text
msg-import-warn-ai-text = Illustrator text on a path is imported as plain text
msg-import-warn-ai-raster = placed images/rasters are not imported
msg-import-warn-eps-preview = EPS without Illustrator data: only the embedded TIFF preview was imported (no PostScript interpreter)
# <<< UE-3 Import-Hinweise

# >>> UE-3 Detailtexte
# Detail texts in AI, export and import errors (fp-ai, fp-export, fp-io), UE-3
ai-err-d-birefnet-size = BiRefNet needs a square with an edge length that is a multiple of 32 (got { $size })
ai-err-d-download-size = size { $done } instead of { $size } bytes
ai-err-d-thread = background thread: { $error }
ai-err-d-job-cancelled = background job cancelled
ai-err-d-result-taken = result already collected
ai-err-d-empty-region = image region is empty
ai-err-d-no-sam = no SAM model
ai-err-d-point-or-box = object selection needs a click point or a box
ai-err-d-no-birefnet = no BiRefNet model
ai-err-d-no-pixel-selection = no pixel selection
ai-err-d-sam-size = unknown SAM 2 size ({ $n } blocks)
ai-err-d-sam-input = SAM 2 needs an image of 1024 × 1024
ai-err-d-no-point-no-box = no point and no box
ai-err-d-header = header: { $error }
ai-err-d-data-outside = data of { $name } outside the file
ai-err-d-dtype = data type { $dtype } of { $name } not supported
ai-err-d-size-mismatch = size of { $name } does not match
ai-err-d-weight-missing = weight { $name } missing
ai-err-d-not-matrix = { $name } is not a matrix
ai-err-d-not-square = { $name } is not a square convolution
err-export-d-no-visible-states = no visible states
err-export-d-page-missing = page { $page } missing
err-export-d-no-slices = No valid slices were found to export.
err-export-d-ico-count = ICO needs 1…255 images
err-export-d-webp-no-frame = animated WebP without a frame
err-export-d-webp-config = WebP configuration
err-export-d-webp-anim = animated WebP: { $error }
err-export-d-apng-no-frame = APNG without a frame
err-export-d-apng-first-frame = the first APNG frame must cover the canvas
err-export-d-pdf-no-page = PDF without a page
err-export-d-psd-background = Background
err-import-d-av1 = AV1 decoder: { $error }
# <<< UE-3 Detailtexte

# >>> UE-3 Kurzhilfe
# Built-in quick help (Help menu without MANUAL.md), UE-3
msg-help-title = Firepixel – quick help
msg-help-intro = The detailed guide is `MANUAL.en.md` next to the program or in the folder `docs`.
msg-help-topic-start = Getting started
msg-help-topic-tools = Tools
msg-help-topic-menus = Menus
msg-help-topic-panels = Panels
msg-help-topic-export = Export
msg-help-topic-automation = Automation (scripts, command line, MCP)
msg-help-topic-support = Troubleshooting
msg-help-body-start = File ▸ New creates a document, File ▸ Open reads FW-PNG and common image formats.
msg-help-body-tools = On the left the Tools panel; keyboard shortcuts are shown in the tooltip.
msg-help-body-menus = All commands can also be reached through the command line `firepixel-cli` and MCP.
msg-help-body-panels = The Window menu shows and hides panels.
msg-help-body-export = File ▸ Export or Image Preview; the settings come from the Optimize panel.
msg-help-body-automation = Commands ▸ Run Script starts `.jsf` files.
msg-help-body-support = Preferences are stored in `%APPDATA%\Firepixel\preferences.json`; deleting the file resets them.
# <<< UE-3 Kurzhilfe

# >>> UE-3 HEIF
# HEIF container (fp-io isobmff.rs), UE-3
err-import-heif-container = HEIF container: { $detail }
err-import-d-box-short = box too short
err-import-d-item-no-location = item without location
err-import-d-item-outside = item data outside the file
err-import-d-sample-outside = sample outside the file
# <<< UE-3 HEIF

# >>> UE-3 Dateiteile
# File structure parts in decoder error messages (fp-io BMP/PSD/TGA/WBMP, fp-ai weights), UE-3
err-import-part-header = header
err-import-part-header-truncated = header truncated
err-import-part-pixels-missing = pixel data missing
err-import-part-pixels-truncated = pixel data truncated
err-import-part-color-mode-data = color mode data
err-import-part-resources = resources
err-import-part-layer-section = layer section
err-import-part-layer-count = layer count
err-import-part-layer = layer
err-import-part-channel = channel
err-import-part-mask = mask
err-import-part-blending-ranges = blending ranges
err-import-part-name = name
err-import-part-channel-data = channel data
err-import-part-color-table-truncated = color table truncated
err-import-part-rle-truncated = RLE truncated
err-import-part-width = width
err-import-part-height = height
err-import-part-file-too-short = file too short
err-import-part-header-not-object = header is not an object
# <<< UE-3 Dateiteile

# >>> FV-5 Warnungen ohne Ziel
# Warnings for always-enabled menu commands without a suitable target (BEFUNDE EB Z01–Z18), FV-5
msg-warn-combine-paths = Two or more paths must be selected to punch, union, intersect or crop.
msg-warn-alter-path = Simplify, Expand Stroke and Inset Path require at least one path.
msg-warn-tween-instances = Select two or more instances of the same library symbol before using the Tween command.
msg-warn-break-apart = To break the link between a library symbol and its instances, first select a symbol or an instance.
msg-warn-insert-hotspot = Select at least one object before using the command to insert hotspots.
msg-warn-insert-slice = Select at least one object before using the command to insert a slice.
msg-warn-select-similar = Make a pixel selection with the Marquee, Lasso or Magic Wand before using the "Select Similar" command.
msg-warn-feather = Before using the "Feather" command, make a pixel selection with the Marquee, Lasso or Magic Wand.
msg-warn-popup-hotspot-or-slice = Behaviors are applied to hotspot or slice objects through the Behaviors panel. Create a hotspot or a slice?
msg-warn-insert-behavior = Select a slice or a hotspot before using the "Add Behavior" command.
msg-warn-choice-hotspot = &Hotspot
msg-warn-choice-slice = S&lice
msg-warn-choice-cancel = &Cancel
msg-warn-dont-show = Don't s&how again
msg-step-jpeg-mask-load = Load JPEG Mask
msg-step-jpeg-mask-delete = Delete JPEG Mask
# <<< FV-5 Warnungen ohne Ziel
# Wächter des Testprofils (fp_i18n::profile::check_write, NZ-1)
err-test-profile-write = test run must not write to the real user profile: { $path } (use fp_i18n::profile::use_test_profile() or FP_TEST_PROFILE, AGENTEN_REGELN §4a)
