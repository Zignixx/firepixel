# Batch Studio, polish part 2 (BATCH-11b, docs\BATCH_MODERN.md §2.3, §2.8, §3.2, §3.10):
# per-format fields of the "Save as" card, extra FW-PNG, HTML report, ffmpeg hint in list and card,
# size/pixel filter, template "Video → small GIF".

## End card "Save as"
batch-param-output-alsoFwpng = Also as FW-PNG
    .tooltip = Writes an editable FW-PNG with the same name next to each file (e.g. SVG and FW-PNG from "Trace" in one run).
batch-param-output-gif-background = Background
    .tooltip = Fill color for semi-transparent pixels when the GIF stores no transparency.
batch-ui-gif-section = GIF optimizer
batch-ui-gif-reached = Reached: { $size } (target { $target })
batch-ui-gif-missed = Target size not reachable: smallest file { $size } (target { $target })
batch-ui-gif-size-pending = Calculating size …
batch-ui-gif-size = Result for the preview file: { $size }
batch-ui-trace-pixel-hint = "Trace" creates paths. Choose SVG or FW-PNG here to keep vectors; a pixel format flattens the result.
batch-ui-jpeg-white-hint = JPEG stores no transparency: transparent areas are filled with the "Background" color (white by default).
batch-warn-gif-target-missed = Target size { $target } not reachable, smallest file { $reached }.
batch-warn-extra-exists = { $name } already exists, not overwritten (rule "Skip").

## ffmpeg hint in file list and "Read video" card
batch-ui-ffmpeg-missing-row = ffmpeg missing – video cannot be read
batch-ui-ffmpeg-missing-card = Videos need ffmpeg. Firepixel does not ship ffmpeg and downloads nothing: choose an existing ffmpeg file.
batch-ui-ffmpeg-choose = ffmpeg file…
    .tooltip = Opens the batch settings, section "Videos (ffmpeg)".

## Filter by size and pixels (Pro)
batch-ui-filter-size = File size (KB)
batch-ui-filter-width = Width (px)
batch-ui-filter-height = Height (px)
batch-ui-filter-alpha = Transparency
batch-ui-filter-alpha-any = Any
batch-ui-filter-alpha-with = With transparency
batch-ui-filter-alpha-without = Without transparency
batch-ui-filter-glob = Name pattern
    .tooltip = Wildcards * and ?, separate several patterns with ; (e.g. *.png; logo_*). Case does not matter.
batch-ui-filter-recipe-hint = These conditions are kept in the recipe as the step "Process only some".
batch-ui-filter-min = from
batch-ui-filter-max = to
batch-ui-filter-reset = Reset filter
batch-ui-filter-glob-hint = Wildcards * and ? (e.g. *.png)

## Summary card
batch-ui-done-open-report = View report
    .tooltip = Opens batch-report.html with thumbnails in the browser.

## Template "Video → small GIF"
batch-template-video-to-gif = Video → small GIF
    .description = Reads the video at 12 frames/s, longest side 480 px, GIF with the optimizer preset balanced (needs ffmpeg).

## HTML report batch-report.html
batch-html-title = Batch report
batch-html-recipe = Recipe “{ $name }”
batch-html-when = { $start } to { $end }
batch-html-dry-run = Dry run – nothing written
batch-html-cancelled-run = Cancelled
batch-html-tile-done = done
batch-html-tile-skipped = skipped
batch-html-tile-failed = errors
batch-html-tile-cancelled = not processed
batch-html-tile-size = Size before → after ({ $change })
batch-html-tile-size-in = Size of the sources
batch-html-tile-duration = Duration
batch-html-seconds = { $s } s
batch-html-errors = Errors ({ $n })
batch-html-col-file = File
batch-html-col-step = Step
batch-html-col-message = Message
batch-html-files = Files ({ $n })
batch-html-before = Before
batch-html-after = After
batch-html-no-preview = no preview
batch-html-ms = { $ms } ms
batch-html-also = Also: { $name }
batch-html-footer = Created with Firepixel · { $time }
batch-html-status-done = Done
batch-html-status-planned = Planned
batch-html-status-skipped = Skipped
batch-html-status-failed = Error
batch-html-status-cancelled = Not processed

## Command line: firepixel-cli batch --help
batch-cli-help-templates = Built-in templates (name for --recipe):
batch-cli-help-legacy = Older form without a recipe: firepixel-cli batch --actions <json|file> --in <folder> --out <folder> [--format png|fwpng|…] (help: firepixel-cli batch --actions x --help)
