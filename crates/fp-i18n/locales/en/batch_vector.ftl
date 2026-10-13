# Batch Studio: step "Trace → SVG/FW-PNG", built-in templates, MCP tool `batch`
# (package BATCH-9, docs\BATCH_MODERN.md §2.5, §3.13, §5.8).

## Step vector.trace
batch-step-vector-trace = Trace → SVG
    .description = Traces images into editable vector paths – as color regions or smooth curves. Save as SVG (paths) or FW-PNG (editable).
batch-param-vector-trace-preset = Preset
batch-param-vector-trace-engine = Method
batch-param-vector-trace-colorMode = Color mode
batch-param-vector-trace-colors = Maximum colors
batch-param-vector-trace-filterSpeckle = Remove speckles up to
batch-param-vector-trace-cornerThreshold = Corner angle
batch-param-vector-trace-mode = Outlines
batch-param-vector-trace-hierarchical = Shapes
batch-param-vector-trace-threshold = Threshold (black and white)
batch-param-vector-trace-ignoreTransparent = Skip transparent pixels
batch-param-vector-trace-pathPrecision = Decimal places
batch-param-vector-trace-refine = Clean up
batch-param-vector-trace-turdSize = Speckles (smooth curves)
batch-param-vector-trace-turnPolicy = Diagonally touching pixels
batch-param-vector-trace-alphaMax = Corners (smooth curves)
batch-param-vector-trace-optCurve = Merge curves
batch-param-vector-trace-optTolerance = Merge tolerance
batch-param-vector-trace-keepOriginal = Keep original as hidden layer
batch-param-vector-trace-transparentCanvas = Transparent canvas
batch-value-vector-trace-preset-logo = Logo
batch-value-vector-trace-preset-blackWhite = Black and White
batch-value-vector-trace-preset-illustration = Illustration
batch-value-vector-trace-preset-photo = Photo
batch-value-vector-trace-engine-clusters = Color regions
batch-value-vector-trace-engine-potrace = Smooth curves
batch-value-vector-trace-colorMode-color = Color
batch-value-vector-trace-colorMode-blackWhite = Black and White
batch-value-vector-trace-mode-spline = Curves
batch-value-vector-trace-mode-polygon = Straight segments
batch-value-vector-trace-hierarchical-stacked = Stacked
batch-value-vector-trace-hierarchical-cutout = Cutout
batch-value-vector-trace-turnPolicy-minority = Minority
batch-value-vector-trace-turnPolicy-majority = Majority
batch-value-vector-trace-turnPolicy-black = Black
batch-value-vector-trace-turnPolicy-white = White
batch-value-vector-trace-turnPolicy-left = Left
batch-value-vector-trace-turnPolicy-right = Right
batch-warn-trace-first-frame = Animation: only the first frame was traced.

## Built-in templates (menu "Recipes ▸ Templates")
batch-template-web-webp = Resize for the web + WebP
    .description = Longest side 1920 px, WebP at quality 82, metadata without GPS.
batch-template-email-photos = Photos for email
    .description = Longest side 1280 px, JPEG at quality 80, transparency on white, metadata without GPS.
batch-template-save-as-jpeg = Save as JPEG
    .description = Transparency on white, JPEG at quality 85.
batch-template-remove-background = Remove background → PNG
    .description = Cuts out the main subject with the local AI model and saves a transparent PNG.
batch-template-transparency-to-white = Transparency to white
    .description = Fills transparent areas with white; the format stays.
batch-template-logos-to-svg = Trace logos → SVG
    .description = Traces logos and icons into clean vector paths with few colors and saves them as SVG.
batch-template-small-gifs = Small GIFs
    .description = Longest side 480 px, GIF with the “balanced” optimizer.
batch-template-strip-metadata = Remove metadata
    .description = Removes EXIF, GPS, XMP and comments; format and pixels stay.
batch-template-rename-by-date = Rename photos by date
    .description = Name from the capture date (EXIF) and a counter, e.g. 2026-10-13_001.jpg.
batch-template-square-1080 = Square 1080
    .description = Crops to a 1080 × 1080 px square (center), JPEG at quality 88.
batch-template-thumbnails-256 = Thumbnails 256
    .description = Fits into 256 × 256 px, PNG, name with “_thumb”.

## Command line
batch-cli-arg-templates = List the built-in templates (name for --recipe)
batch-cli-templates-header = Built-in templates (firepixel-cli batch --recipe <name> …):

## MCP tool batch
mcp-tool-batch = Batch processing
    .description = Processes many images with a recipe like the Batch Studio (own sessions, never the open document). action: templates (built-in templates), steps (step catalog), validate, plan (dry run), preview (one file before/after as images), run (execute; async true returns a jobId), status (progress, wait_ms), cancel. recipe: recipe object, path to .fpbatch.json or a template name such as web-webp or logos-to-svg.
mcp-tool-batch-action = Action (default run)
mcp-tool-batch-recipe = Recipe object, JSON text, path or template name
mcp-tool-batch-files = Files (absolute paths)
mcp-tool-batch-folder = Folder with the files
mcp-tool-batch-recursive = Include subfolders
mcp-tool-batch-out = Output folder (overrides the recipe target)
mcp-tool-batch-conflict = Conflict rule (overrides the recipe)
mcp-tool-batch-dryRun = Only plan, write nothing
mcp-tool-batch-threads = Number of worker threads
mcp-tool-batch-async = Start in the background and return the jobId at once
mcp-tool-batch-jobId = Job for status and cancel
mcp-tool-batch-wait_ms = status: wait at most this long for the end
mcp-tool-batch-file = preview: file
mcp-tool-batch-maxSize = preview: longest side of the images in pixels
mcp-batch-err-job-id = jobId is missing (from run with async true).
mcp-batch-err-unknown-job = Unknown job { $id }.
mcp-batch-err-action = Unknown action “{ $action }”. Possible: { $actions }.
mcp-batch-err-file = preview needs file (path of an image file).
mcp-batch-hint-status = Progress: batch with action status and jobId { $id } (wait_ms up to 60000); cancel with action cancel.
mcp-res-batch = Batch processing
    .description = Templates and steps of the Batch Studio with parameters
mcp-res-batch-title = Batch processing: templates and steps
mcp-res-batch-intro = Tool batch: recipe is a recipe object, a path to .fpbatch.json or a template name. Start long runs with async true and poll them with status.
mcp-res-batch-templates = Built-in templates
mcp-res-batch-steps = Steps
