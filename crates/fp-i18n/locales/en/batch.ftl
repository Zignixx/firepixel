# Modern batch mode (Batch Studio, docs\BATCH_MODERN.md) – core crates\fp-batch, commands batch.*,
# firepixel-cli batch --recipe. Created by BATCH-1; BATCH-2 and later add keys here or in batch_*.ftl.
# Prefix batch-; steps batch-step-<id> (dot as -), parameters batch-param-<id>-<param>.

## Groups
batch-group-files = Files
batch-group-format = Format
batch-group-geometry = Geometry
batch-group-color = Color and Effects
batch-group-background = Background
batch-group-overlay = Overlay
batch-group-metadata = Metadata
batch-group-animation = Animation
batch-group-optimize = Optimize
batch-group-document = Document
batch-group-advanced = Advanced

## Steps (BATCH-1)
batch-step-file-copyOnly = Copy/Move Only
    .description = Copies or moves the files unchanged, without opening them.
batch-step-rename = File Name
    .description = Sets how the new files are named.
batch-step-output = Save As
    .description = Format and format options of the new files.
batch-param-file-copyOnly-action = Action
batch-param-rename-mode = Mode
batch-param-output-format = Format
batch-param-output-quality = Quality
batch-summary-file-copy = Copy files
batch-summary-file-move = Move files
## GIF optimizer (BATCH-8, steps\gifopt)
batch-step-output-gif = GIF Options
    .description = Small GIFs at very good quality: palette, dithering, frame differences, loss and target size.
batch-param-output-gif-preset = Preset
batch-param-output-gif-quality = Quality
batch-param-output-gif-maxKB = Target size
batch-param-output-gif-colors = Maximum colors
batch-param-output-gif-palette = Palette
batch-param-output-gif-dither = Dithering
batch-param-output-gif-ditherAmount = Dithering strength
batch-param-output-gif-lossy = Loss
batch-param-output-gif-diffTransparency = Unchanged pixels transparent
batch-param-output-gif-diffThreshold = “Unchanged” threshold
batch-param-output-gif-cropChanged = Save changed area only
batch-param-output-gif-mergeSimilar = Merge nearly identical frames
batch-param-output-gif-disposal = Disposal
batch-param-output-gif-transparency = Transparency
batch-param-output-gif-splitFrames = Save separate changes individually
batch-param-output-gif-preset-smallest = Smallest
batch-param-output-gif-preset-balanced = Balanced
batch-param-output-gif-preset-best = Best Quality
batch-param-output-gif-palette-global = Global
batch-param-output-gif-palette-perFrame = Per Frame
batch-param-output-gif-palette-auto = Automatic
batch-param-output-gif-dither-none = None
batch-param-output-gif-dither-ordered = Ordered
batch-param-output-gif-dither-blueNoise = Blue Noise
batch-param-output-gif-dither-floydSteinberg = Floyd–Steinberg
batch-param-output-gif-dither-temporal = Temporally Stable
batch-param-output-gif-disposal-auto = Automatic
batch-param-output-gif-disposal-keep = Do Not Dispose
batch-step-anim-stabilize = Steady Background
    .description = Freezes small changes over time (less noise, smaller files).
batch-param-anim-stabilize-threshold = Threshold
batch-summary-anim-stabilize = Threshold { $threshold }

## Errors
batch-err-recipe-json = The recipe is not valid JSON: { $error }
batch-err-recipe-not-object = The recipe must be a JSON object.
batch-err-recipe-format = Unknown recipe format “{ $format }”.
batch-err-recipe-newer = The recipe has version { $version }; this version of Firepixel reads up to version { $supported }.
batch-err-recipe-missing = No recipe given.
batch-err-no-files = No files given.
batch-err-conflict-value = Unknown conflict rule “{ $value }” (rename, skip, overwrite, ask).
batch-err-regex = Invalid regular expression: { $error }
batch-err-format-unavailable = The { $format } format cannot be written yet.
batch-err-unknown-format = Unknown file format: { $path }
batch-err-empty-file = The file is empty.
batch-err-empty-page = The image has no pixels.
batch-err-no-frames = The animation has no frames.
batch-err-not-image = Not an image file: { $path }
batch-err-not-collect = The step { $id } is not a collecting step.
batch-err-worker-panic = Batch processing stopped unexpectedly.
err-backend-batch-runtime = batch processing (fp-batch)

## Recipe check
batch-issue-unknown-step = Unknown step “{ $id }” (skipped, settings are kept).
batch-issue-final-step = “{ $id }” is a fixed end card and does not belong in the step list.
batch-issue-missing-folder = Please choose an output folder.
batch-issue-unknown-format = Unknown output format “{ $format }”.
batch-issue-copy-only-combined = “Copy/Move Only” works only without image steps and with “Save As: same as source”.
batch-issue-param-type = Invalid value for “{ $param }”.
batch-issue-param-range = “{ $param }” must be between { $min } and { $max }.
batch-issue-param-enum = “{ $param }” must be one of: { $values }.
batch-issue-params-not-object = The parameters must be a JSON object.

## Notes on names and targets
batch-hint-invalid-chars = Invalid characters replaced
batch-hint-reserved-name = Reserved name, “_” appended
batch-hint-truncated = Name shortened
batch-hint-empty-name = Empty name, “_” used
batch-hint-leading-dot = Leading dot replaced
batch-hint-exif-date-missing = EXIF date missing, modification date used
batch-hint-conflict = Conflict: name already taken
batch-hint-overwrite = Existing file will be overwritten
batch-hint-replace-original = Original will be replaced (backup in “Original Files”)
batch-hint-backup-forced = Backup turned on because the original is replaced
batch-hint-skipped = Skipped
batch-hint-long-path = Path longer than 260 characters
batch-hint-format-fallback = Source format cannot be written, PNG used
batch-hint-same-fallback-png = The { $format } format cannot be written; PNG is used instead.

## Messages
batch-msg-cancelled = Canceled.
batch-msg-summary = { $done } done, { $skipped } skipped, { $failed } failed · { $before } → { $after }
batch-msg-cancelled-summary = Canceled: { $done } done, { $rest } not processed
batch-msg-dry-run-summary = Dry run: { $planned } of { $total } files would be written, { $skipped } skipped
batch-warn-first-frame-only = { $format } does not store animation; only the first frame is written.
batch-warn-not-applicable = “{ $step }” does not apply to this file.
batch-warn-preview-skipped = “{ $step }” is not calculated in the preview.
batch-warn-unchanged = Unchanged (same name in the same place).

## Command registry (batch.*)
cmd-batch-run = Run Batch
    .description = Runs a batch recipe on a list of files and returns the report
cmd-batch-plan = Plan Batch
    .description = Plans source and target of every file with conflicts, without writing
cmd-batch-preview = Batch Preview
    .description = Calculates the before and after preview of a file
cmd-batch-steps = Batch Steps
    .description = Catalog of all batch steps with parameters
cmd-batch-validate = Check Batch Recipe
    .description = Checks a batch recipe and reports errors and warnings per step
cmdtext-liste-der-dateien = List of files
cmdtext-ordner-mit-den-dateien = Folder with the files
cmdtext-unterordner-einbeziehen = Include subfolders
cmdtext-rezept-objekt-oder-pfad-einer-rezept-datei = Recipe (object) or path of a recipe file
cmdtext-nur-planen-nichts-schreiben = only plan, write nothing
cmdtext-zahl-der-arbeits-threads = Number of worker threads
cmdtext-ausgabeordner-ueberschreibt-das-ziel-des-rezepts = Output folder (overrides the recipe target)
cmdtext-konfliktregel-ueberschreibt-das-rezept = Conflict rule (overrides the recipe)
cmdtext-datei = File
cmdtext-laengste-seite-der-vorschau-in-pixeln = longest side of the preview in pixels

## Command line: firepixel-cli batch --recipe / --steps
batch-cli-about = Apply a batch recipe to files and folders (modern batch mode)
batch-cli-examples =
    Examples:
      firepixel-cli batch --recipe web.fpbatch.json --in photos --recursive
      firepixel-cli batch --recipe web.fpbatch.json a.png b.jpg --out output --dry-run
      firepixel-cli batch --steps --json
    Exit code: 0 all done, 1 at least one error, 2 wrong call, 3 canceled.
batch-cli-arg-recipe = Recipe file (.fpbatch.json) or JSON text
batch-cli-arg-steps = Show the catalog of all steps
batch-cli-arg-in = Input folder
batch-cli-arg-recursive = Include subfolders
batch-cli-arg-files = Files or folders
batch-cli-arg-out = Output folder (overrides the recipe target)
batch-cli-arg-conflict = Conflict rule: rename, skip or overwrite
batch-cli-arg-dry-run = Only plan, write nothing
batch-cli-arg-threads = Number of worker threads
batch-cli-arg-report = Write the report (JSON) to this file
batch-cli-arg-json = Machine-readable output (progress as NDJSON on stderr)
batch-cli-arg-lang = Output language (de, en …)
batch-cli-err-args = Wrong call: { $error }
batch-cli-skipped-file = Skipped (unsupported format): { $path }
batch-cli-progress-done = [{ $n }/{ $total }] { $source } → { $target }
batch-cli-progress-skipped = [{ $n }/{ $total }] { $source }: { $reason }
batch-cli-progress-failed = [{ $n }/{ $total }] { $source }: Error: { $error }
batch-cli-report-written = Report: { $path }

## UI: Batch Studio (BATCH-2, crates\fp-ui\src\batch\)
batch-ui-tab = Batch
    .tooltip = Batch processing: process many files with one recipe
batch-ui-title = Batch Processing
batch-ui-subtitle = { $n ->
        [one] 1 file
       *[other] { $n } files
    } · { $size } · Recipe: “{ $recipe }”
batch-ui-unsaved = (unsaved •)
batch-ui-recipe-untitled = New recipe
batch-ui-mode-beginner = Simple
batch-ui-mode-pro = Pro
batch-ui-recipes = Recipes
    .tooltip = Templates; open and save your own recipes
batch-ui-menu-templates = Templates
batch-ui-menu-mine = My Recipes
batch-ui-menu-none = No saved recipes yet
batch-ui-menu-open = Open Recipe...
batch-ui-menu-save = Save Recipe
batch-ui-menu-save-as = Save Recipe As...
batch-ui-menu-new = Empty Recipe
batch-ui-more = More commands
batch-ui-undo = Undo in Recipe
batch-ui-redo = Redo in Recipe
batch-ui-close = Close Batch
batch-ui-recursive = Include subfolders
batch-ui-filter-recipes = Batch recipes
batch-ui-recipe-loaded = Recipe “{ $name }” loaded
batch-ui-recipe-load-failed = Cannot load recipe: { $error }
batch-ui-recipe-saved = Recipe “{ $name }” saved
batch-ui-on = on
batch-ui-off = off
# Files card
batch-ui-files-title = Files
batch-ui-files-count = { $n ->
        [one] 1 file
       *[other] { $n } files
    }
batch-ui-files-count-filtered = { $shown } of { $total }
batch-ui-files-add = Files...
    .tooltip = Add images, animations or videos (Ctrl+O)
batch-ui-files-add-title = Files for Batch Processing
batch-ui-files-add-folder = Folder...
batch-ui-files-add-folder-rec = Folder with Subfolders...
batch-ui-files-add-open = Open Documents
batch-ui-files-folder-title = Add Folder
batch-ui-files-more = More ways to add
batch-ui-files-no-open-docs = No open document is saved as a file.
batch-ui-files-remove = Remove
    .tooltip = Take the selected file off the list (Del); the file itself stays unchanged
batch-ui-files-clear = Clear
batch-ui-files-empty = Drag images, animations or videos here
batch-ui-files-empty-hint = PNG, JPEG, GIF, WebP, AVIF, BMP, TIFF, QOI, FW-PNG, PSD and more – whole folders too
batch-ui-files-drop = Release to add
batch-ui-files-duplicates = { $n ->
        [one] 1 file was already in the list
       *[other] { $n } files were already in the list
    }
batch-ui-files-skipped = { $n } skipped (unsupported format)
batch-ui-files-videos = { $n ->
        [one] 1 video
       *[other] { $n } videos
    }
batch-ui-file-enabled = Process “{ $name }”
batch-ui-badge-frames = { $n } fr.
batch-ui-filter-hint = Filter...
batch-ui-filter-hint-pro = Filter... (start with / for a regular expression)
batch-ui-filter-supported = All supported files
batch-ui-filter-all = All types
batch-ui-filter-images = Images
batch-ui-filter-animations = Animations
batch-ui-filter-videos = Videos
batch-ui-filter-documents = FW-PNG/documents
batch-ui-filter-other = Other
batch-ui-filter-failed = Only files with errors
batch-ui-sort = Order (sets the numbering)
batch-ui-sort-added = Added
batch-ui-sort-name = Name
batch-ui-sort-date = Date
batch-ui-sort-size = Size
batch-ui-sort-pixels = Pixels
# Recipe card
batch-ui-recipe-title = Recipe
batch-ui-recipe-steps = { $n ->
        [0] no steps
        [one] 1 step
       *[other] { $n } steps
    }
batch-ui-recipe-empty = No steps yet
batch-ui-recipe-empty-hint = Build a chain with “Add step” – or just rename and save in another format.
batch-ui-add-step = Add step
    .tooltip = Choose a step from the catalog
batch-ui-step-enabled = Turn step on or off
batch-ui-step-menu = More actions
batch-ui-step-drag = Drag to reorder (Alt+Up/Down)
batch-ui-step-duplicate = Duplicate
batch-ui-step-up = Move Up
batch-ui-step-down = Move Down
batch-ui-step-remove = Remove
batch-ui-step-more = More options
batch-ui-step-less = Fewer options
batch-ui-step-hidden-pro = { $n ->
        [one] + 1 pro setting
       *[other] + { $n } pro settings
    }
batch-ui-step-unknown = Unknown step “{ $id }”
batch-ui-step-unknown-hint = From a newer version; skipped, settings are kept
batch-ui-step-no-params = This step has no settings.
batch-ui-end-section = Output
batch-ui-end-fixed = Fixed final card: shows what is written at the end
batch-ui-end-rename = File name
batch-ui-end-output = Save as
batch-ui-rename-same = Name stays the same: { $name }
batch-ui-rename-hint = Name of the output files
batch-ui-rename-prefix = Before
batch-ui-rename-suffix = After
batch-ui-rename-case = Case
batch-ui-rename-spaces = Spaces → _
batch-ui-rename-find = Find (regular expression)
batch-ui-rename-replace = Replace with
batch-ui-rename-mode-builder = Builder
batch-ui-rename-mode-regex = Regular expression
batch-ui-case-keep = unchanged
batch-ui-case-lower = lower case
batch-ui-case-upper = UPPER CASE
batch-ui-case-sentence = Sentence case
batch-ui-case-title = Title Case
batch-ui-case-kebab = kebab-case
batch-ui-case-snake = snake_case
batch-ui-case-camel = camelCase
batch-ui-output-same = Same format as source
batch-ui-output-quality = Quality { $q }
batch-ui-choose-file = Choose File...
batch-ui-gallery-title = Add step
batch-ui-gallery-close = Close catalog (Esc)
batch-ui-gallery-search = Search...
batch-ui-gallery-empty = No matching step
batch-ui-gallery-empty-hint = More steps are available in Pro mode.
batch-ui-gallery-empty-hint-pro = Try another search term.
# Preview card
batch-ui-preview-title = Preview
batch-ui-preview-nav = { $i }/{ $n }
batch-ui-tab-before = Before
batch-ui-tab-after = After
batch-ui-tab-compare = Compare
batch-ui-tab-plan = Plan
batch-ui-tab-errors = Errors ({ $n })
batch-ui-preview-empty = Add files to see the preview
batch-ui-preview-pick = Choose a file in the list
batch-ui-preview-computing = Computing preview...
batch-ui-preview-error = Preview not possible
batch-ui-preview-before-info = Before { $w }×{ $h } · { $size }
batch-ui-preview-after-info = After { $w }×{ $h } · { $size }
batch-ui-preview-size-pending = calculating
batch-ui-preview-time = { $s } s
batch-ui-preview-prev = Previous file
batch-ui-preview-next = Next file
batch-ui-preview-split = Before/after divider
batch-ui-zoom = Zoom (mouse wheel zooms, double-click fits)
batch-ui-zoom-fit = Fit
batch-ui-bg = Background behind transparency
batch-ui-bg-checker = Checkerboard
batch-ui-bg-white = White
batch-ui-bg-black = Black
# Plan, errors, end
batch-ui-plan-empty = No dry run yet
batch-ui-plan-empty-hint = “Dry run” (F5) shows targets, conflicts and notes without writing anything.
batch-ui-plan-summary = { $n ->
        [one] 1 file
       *[other] { $n } files
    } · { $conflicts ->
        [0] no conflicts
        [one] 1 conflict
       *[other] { $conflicts } conflicts
    }
batch-ui-plan-source = Source
batch-ui-plan-target = Target
batch-ui-plan-note = Note
batch-ui-plan-conflict-with = same name as file { $n }
batch-ui-errors-copy = Copy
batch-ui-errors-file = File
batch-ui-errors-message = Message
batch-ui-done-title = Done
batch-ui-done-errors = Done with errors
batch-ui-done-cancelled = Cancelled
batch-ui-done-ok = { $n } done
batch-ui-done-skipped = { $n } skipped
batch-ui-done-failed = { $n } errors
batch-ui-done-counts = { $done } done · { $skipped } skipped · { $failed } errors
batch-ui-done-duration = Duration { $s } s
batch-ui-done-open-folder = Open Folder
batch-ui-done-save-report = Save Report...
batch-ui-done-retry = Retry Failed
batch-ui-done-new = New Recipe
batch-ui-done-close = Close
batch-ui-report-saved = Report saved: { $name }
batch-ui-failed-notice = { $n ->
        [one] 1 file could not be processed
       *[other] { $n } files could not be processed
    }
# Output bar
batch-ui-target = Target
batch-ui-target-new = New folder “{ $name }”
batch-ui-target-folder = Custom folder...
batch-ui-target-same = Same place as the original
batch-ui-target-choose = Folder...
    .tooltip = Choose a custom output folder
batch-ui-target-choose-title = Choose Output Folder
batch-ui-conflict = On conflict
batch-ui-conflict-rename = Choose a new name
batch-ui-conflict-skip = Skip
batch-ui-conflict-overwrite = Overwrite
batch-ui-conflict-ask = Ask every time
batch-ui-keep-structure = Keep folder structure
batch-ui-backup-keep = Keep backups
    .tooltip = Originals go to “Original Files”; when checked, older backups are not overwritten
batch-ui-dry-run = Dry Run
    .tooltip = Show targets, conflicts and notes without writing (F5)
batch-ui-start = { $n ->
        [one] Process 1 file
       *[other] Process { $n } files
    }
    .tooltip = Start processing (Ctrl+Enter)
batch-ui-cancel = Cancel
batch-ui-cancelling = Cancelling...
batch-ui-running = Running...
batch-ui-progress = { $done } of { $total }
batch-ui-remaining = { $time } left
batch-ui-blocked-empty = No files in the list
batch-ui-blocked-filtered = No file enabled, or all filtered out
batch-ui-blocked-invalid = The recipe contains errors:
batch-ui-ask-title = File already exists
batch-ui-ask-text = “{ $name }” already exists in the target. What should happen?
batch-ui-ask-all = Apply to all remaining
# Labels of the final card “Save as” and of “Copy only” (studio form)
batch-param-output-lossless = Lossless
batch-param-output-bitDepth = Color depth
batch-param-output-bitDepth-auto = automatic
batch-param-output-bitDepth-8 = 8 bit (palette)
batch-param-output-bitDepth-24 = 24 bit
batch-param-output-bitDepth-32 = 32 bit (with transparency)
batch-param-output-palette = Palette
batch-param-output-interlace = Interlaced
batch-param-output-progressive = Progressive
batch-param-output-subsampling = Chroma subsampling
batch-param-output-subsampling-auto = automatic
batch-param-output-background = Background
batch-param-output-colors = Colors
batch-param-output-dither = Dithering
batch-param-output-dither-none = none
batch-param-output-dither-ordered = ordered
batch-param-output-dither-floydSteinberg = Floyd-Steinberg
batch-param-output-dither-blueNoise = Blue noise
batch-param-output-ditherAmount = Dither amount
batch-param-output-transparency = Transparency
batch-param-output-speed = Speed
batch-param-file-copyOnly-action-copy = Copy
batch-param-file-copyOnly-action-move = Move
