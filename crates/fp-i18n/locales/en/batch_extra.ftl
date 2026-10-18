# Batch Studio, BA-109 (docs\BATCH.md §3): additional steps in crates\fp-batch\src\steps\extra.rs.
# Steps batch-step-<id>, parameters batch-param-<id>-<param>, values batch-value-<id>-<param>-<value>.

## Center subject
batch-step-background-subjectCenter = Center subject
    .description = Finds the subject, crops to it and centers it with an even margin.
batch-param-background-subjectCenter-margin = Margin
batch-param-background-subjectCenter-square = Square
batch-param-background-subjectCenter-shadow = Shadow under the subject
batch-param-background-subjectCenter-detect = Find subject by
batch-param-background-subjectCenter-tolerance = Tolerance
batch-param-background-subjectCenter-fill = New area
batch-param-background-subjectCenter-color = Color
batch-value-background-subjectCenter-detect-auto = Automatic
batch-value-background-subjectCenter-detect-alpha = Transparency
batch-value-background-subjectCenter-detect-color = Differs from the corner color
batch-value-background-subjectCenter-fill-auto = Automatic (transparent or corner color)
batch-value-background-subjectCenter-fill-transparent = Transparent
batch-value-background-subjectCenter-fill-color = Color
batch-warn-subject-none = No subject found, the image stays unchanged.

## Clean up alpha
batch-step-background-alphaCleanup = Clean up alpha
    .description = Removes faint transparency, small specks and colored fringes at the edges.
batch-param-background-alphaCleanup-low = Fully transparent below
batch-param-background-alphaCleanup-high = Fully opaque above
batch-param-background-alphaCleanup-minIsland = Remove specks smaller than
batch-param-background-alphaCleanup-defringe = Defringe
batch-err-alpha-range = "Fully opaque above" must be greater than "Fully transparent below".

## Round corners
batch-step-overlay-roundCorners = Round corners
    .description = Rounds the corners of every image; outside the corners becomes transparent.
batch-param-overlay-roundCorners-radius = Radius
batch-param-overlay-roundCorners-unit = Unit
batch-value-overlay-roundCorners-unit-px = Pixels
batch-value-overlay-roundCorners-unit-percent = % of the shorter side

## Vignette
batch-step-overlay-vignette = Vignette
    .description = Darkens the edges of every image towards the corners.
batch-param-overlay-vignette-strength = Strength
batch-param-overlay-vignette-size = Untouched center
batch-param-overlay-vignette-color = Color

## Posterize / threshold / pixelate
batch-step-adjust-posterize = Posterize
    .description = Reduces every color channel to a few tonal steps.
batch-param-adjust-posterize-levels = Levels
batch-step-adjust-threshold = Threshold
    .description = Turns every pixel black or white depending on its brightness.
batch-param-adjust-threshold-threshold = Threshold
batch-step-filter-pixelate = Pixelate
    .description = Turns the image into a mosaic of square blocks.
batch-param-filter-pixelate-size = Block size

## Pad to multiple
batch-step-canvas-pad = Pad to multiple
    .description = Grows the canvas so that width and height are multiples of a number.
batch-param-canvas-pad-multiple = Multiple of
batch-param-canvas-pad-color = Color

## Still from animation
batch-step-anim-firstFrame = Still from animation
    .description = Keeps a single frame of an animation as a still image.
batch-param-anim-firstFrame-pick = Frame
batch-param-anim-firstFrame-number = Frame number
batch-param-anim-firstFrame-time = Time
batch-value-anim-firstFrame-pick-first = First
batch-value-anim-firstFrame-pick-middle = Middle
batch-value-anim-firstFrame-pick-last = Last
batch-value-anim-firstFrame-pick-number = Frame number
batch-value-anim-firstFrame-pick-time = At a time
batch-value-anim-firstFrame-pick-sharpest = Sharpest

## Sprite sheet atlas (BA-105)
batch-param-anim-spriteSheet-atlas = Atlas file
batch-value-anim-spriteSheet-atlas-json = JSON next to the sheet
batch-value-anim-spriteSheet-atlas-css = CSS next to the sheet
batch-value-anim-spriteSheet-atlas-none = None

## Dry run with the size token (BA-105)
batch-hint-size-after-processing = The name uses the image size, which is only known after processing; shown with the source size.

# B-3: file steps (crates\fp-batch\src\steps\fileops.rs)

## Sort into subfolders
batch-step-file-organize = Sort into subfolders
    .description = Puts the files into subfolders by date, type, size, orientation, camera or your own pattern.
batch-param-file-organize-by = Sort by
batch-param-file-organize-pattern = Folder pattern
    .tooltip = For example {"{"}date:YYYY{"}"}/{"{"}ext{"}"}. Placeholders: date, modified, ext, format, size, orientation, camera, width, height.
batch-param-file-organize-action = Files
batch-value-file-organize-by-date = Capture date (year/month)
batch-value-file-organize-by-type = File type
batch-value-file-organize-by-size = Size class
batch-value-file-organize-by-orientation = Orientation
batch-value-file-organize-by-camera = Camera
batch-value-file-organize-by-pattern = Own pattern
batch-value-file-organize-action-copy = Copy
batch-value-file-organize-action-move = Move
batch-err-organize-empty = Enter a folder pattern.
batch-err-organize-token = Unknown placeholder: { $tokens }

## Process only some
batch-step-file-filterSkip = Process only some
    .description = Processes only the files that meet the conditions; the others are skipped or copied unchanged.
batch-param-file-filterSkip-minWidth = Width at least
batch-param-file-filterSkip-maxWidth = Width at most
batch-param-file-filterSkip-minHeight = Height at least
batch-param-file-filterSkip-maxHeight = Height at most
batch-param-file-filterSkip-minKB = File size at least
batch-param-file-filterSkip-maxKB = File size at most
batch-param-file-filterSkip-transparency = Transparency
batch-param-file-filterSkip-animation = Animation
batch-param-file-filterSkip-name = File name
    .tooltip = Wildcards * and ?, several patterns separated by ; (for example *.png; logo*).
batch-param-file-filterSkip-invert = Reverse (process the others)
batch-param-file-filterSkip-action = Other files
batch-value-file-filterSkip-transparency-any = Any
batch-value-file-filterSkip-transparency-with = Only with transparency
batch-value-file-filterSkip-transparency-without = Only without transparency
batch-value-file-filterSkip-animation-any = Any
batch-value-file-filterSkip-animation-animated = Only animations
batch-value-file-filterSkip-animation-still = Only still images
batch-value-file-filterSkip-action-skip = Skip
batch-value-file-filterSkip-action-copy = Copy unchanged
batch-skip-filter-pixels = Size { $w } × { $h } px is outside the range.
batch-skip-filter-bytes = File size { $kb } KB is outside the range.
batch-skip-filter-opaque = Has no transparency.
batch-skip-filter-transparent = Has transparency.
batch-skip-filter-still = Is not an animation.
batch-skip-filter-animated = Is an animation.
batch-skip-filter-name = File name does not match.
batch-skip-filter-inverted = Meets the conditions (reversed).
batch-warn-filter-preview = Would be skipped: { $reason }
batch-warn-filter-copied = Copied unchanged.
batch-err-filter-range = The upper limit must not be smaller than the lower limit.
batch-err-filter-name = Invalid file name pattern: { $error }

## Detect duplicate images
batch-step-file-dedupe = Detect duplicate images
    .description = Finds identical or very similar images; the first one stays, the others are skipped or marked.
batch-param-file-dedupe-method = Compare
batch-param-file-dedupe-distance = Allowed difference
    .tooltip = 0 = nearly identical, 16 = loosely similar.
batch-param-file-dedupe-action = Duplicates
batch-value-file-dedupe-method-exact = Exactly the same pixels
batch-value-file-dedupe-method-similar = Similar (image fingerprint)
batch-value-file-dedupe-action-skip = Skip
batch-value-file-dedupe-action-mark = Only mark in the report
batch-skip-duplicate = Duplicate of { $name }.

## Set date
batch-step-file-timestamps = Set date
    .description = Sets the modification date of the written files.
batch-param-file-timestamps-mode = Date from
batch-param-file-timestamps-date = Date
    .tooltip = YYYY-MM-DD HH:MM
batch-value-file-timestamps-mode-source = Source file
batch-value-file-timestamps-mode-exif = Capture date (EXIF), otherwise source file
batch-value-file-timestamps-mode-fixed = Fixed date
batch-err-timestamp-date = Enter the date as YYYY-MM-DD HH:MM.

# B-3: document steps (crates\fp-batch\src\steps\document.rs)

## Find and replace
batch-step-document-findReplace = Find and replace
    .description = Replaces text, fonts, colors or URLs in documents; the document stays editable.
batch-param-document-findReplace-whatToFind = Find
batch-param-document-findReplace-find = Search for
batch-param-document-findReplace-replace = Replace with
batch-param-document-findReplace-matchCase = Match case
batch-param-document-findReplace-wholeWord = Whole word
batch-param-document-findReplace-regex = Regular expression
batch-value-document-findReplace-whatToFind-text = Text
batch-value-document-findReplace-whatToFind-font = Font
batch-value-document-findReplace-whatToFind-color = Color
batch-value-document-findReplace-whatToFind-url = URL
batch-value-document-findReplace-whatToFind-nonWeb216 = Non-web216 colors
batch-err-find-empty = Enter what to search for.
batch-err-find-color = Enter colors as #rrggbb.
batch-err-find-regex = Invalid search pattern: { $error }
batch-warn-find-none = Nothing found to replace.

## Show/hide layers
batch-step-document-layers = Show/hide layers
    .description = Shows or hides the layers whose name matches, before the document is saved or exported.
batch-param-document-layers-name = Layer name
    .tooltip = Wildcards * and ?, several names separated by ; (for example Background; Text*).
batch-param-document-layers-regex = Regular expression
batch-param-document-layers-mode = Matching layers
batch-value-document-layers-mode-only = Show only these
batch-value-document-layers-mode-show = Show
batch-value-document-layers-mode-hide = Hide
batch-err-layers-name = Enter a layer name.
batch-warn-layers-none = No layer name matches.

## Choose pages/states
batch-step-document-pages = Choose pages/states
    .description = Saves the chosen pages (and states) of every document as separate images.
batch-param-document-pages-pages = Pages
    .tooltip = Empty = all pages, otherwise for example 1-3, 5.
batch-param-document-pages-states = States
batch-param-document-pages-pattern = File name
    .tooltip = Placeholders: {"{"}name{"}"}, {"{"}page{"}"}, {"{"}pageName{"}"}, {"{"}state{"}"}.
batch-value-document-pages-states-first = First state
batch-value-document-pages-states-all = All states
batch-err-pages-none = No page to export.
batch-skip-not-document = Not a document with pages.

## Export slices
batch-step-document-slices = Export slices
    .description = Saves every slice of the documents as its own image (without HTML).
batch-param-document-slices-pattern = File name
    .tooltip = Placeholders: {"{"}name{"}"}, {"{"}slice{"}"} (slice name or number), {"{"}n{"}"}.
batch-skip-no-slices = The document has no slices.

# B-3: geometry, color and optimize steps (crates\fp-batch\src\steps\pixel.rs)

## Perspective
batch-step-perspective = Perspective
    .description = Straightens a skewed area: the four corners become the corners of the new image.
batch-param-perspective-tlX = Top left X
batch-param-perspective-tlY = Top left Y
batch-param-perspective-trX = Top right X
batch-param-perspective-trY = Top right Y
batch-param-perspective-brX = Bottom right X
batch-param-perspective-brY = Bottom right Y
batch-param-perspective-blX = Bottom left X
batch-param-perspective-blY = Bottom left Y
batch-param-perspective-size = Result size
batch-value-perspective-size-auto = From the corners
batch-value-perspective-size-keep = Keep image size

## Split into tiles
batch-step-tile-split = Split into tiles
    .description = Cuts every image into a grid of tiles, each saved as its own file.
batch-param-tile-split-mode = Split by
batch-param-tile-split-columns = Columns
batch-param-tile-split-rows = Rows
batch-param-tile-split-tileWidth = Tile width
batch-param-tile-split-tileHeight = Tile height
batch-param-tile-split-pattern = File name
    .tooltip = Placeholders: {"{"}name{"}"}, {"{"}row{"}"}, {"{"}col{"}"}.
batch-value-tile-split-mode-grid = Number of tiles
batch-value-tile-split-mode-size = Tile size

## Noise
batch-step-filter-noise = Add/remove noise
    .description = Adds film grain or removes speckles with a median filter.
batch-param-filter-noise-mode = Noise
batch-param-filter-noise-amount = Amount
batch-param-filter-noise-distribution = Distribution
batch-param-filter-noise-monochrome = Monochrome
batch-param-filter-noise-seed = Seed
batch-param-filter-noise-radius = Radius
batch-value-filter-noise-mode-add = Add
batch-value-filter-noise-mode-remove = Remove (median)
batch-value-filter-noise-distribution-uniform = Uniform
batch-value-filter-noise-distribution-gaussian = Gaussian

## Color LUT
batch-step-adjust-lut = Color LUT (.cube)
    .description = Applies a color look-up table from a .cube file.
batch-param-adjust-lut-file = LUT file
batch-param-adjust-lut-strength = Strength
batch-err-lut-file = Choose a .cube file.
batch-err-lut-line = A line of the LUT file does not hold three numbers.
batch-err-lut-size = The LUT file has no valid LUT_3D_SIZE (2 to 256).
batch-err-lut-1d = Only 3D LUT files are supported.
batch-err-lut-count = The number of LUT entries does not match LUT_3D_SIZE.

## Apply style
batch-step-effect-style = Apply style
    .description = Applies a saved style (fill, stroke, effects) by name to every image or document.
batch-param-effect-style-name = Style name
batch-param-effect-style-library = Style library
    .tooltip = Optional: style library (.json) or FW-PNG with styles; otherwise the styles of the document.
batch-err-style-name = Enter the name of a style.
batch-err-style-missing = Style “{ $name }” not found.

## Optimize JPEG
batch-step-optimize-jpeg = Optimize JPEG
    .description = Re-encodes JPEG files with the chosen quality, optionally down to a target size.
batch-param-optimize-jpeg-quality = Quality
batch-param-optimize-jpeg-maxKB = Target size (0 = off)
batch-param-optimize-jpeg-minQuality = Lowest quality
batch-param-optimize-jpeg-progressive = Progressive
batch-param-optimize-jpeg-subsampling = Color subsampling
batch-value-optimize-jpeg-subsampling-auto = Automatic
batch-value-optimize-jpeg-subsampling-444 = 4:4:4 (best)
batch-value-optimize-jpeg-subsampling-422 = 4:2:2
batch-value-optimize-jpeg-subsampling-420 = 4:2:0 (smallest)
batch-warn-jpeg-only = JPEG optimization only works with “Save as JPEG” (selected: { $format }).

## Web-safe colors
batch-step-optimize-webSafe = Web-safe colors
    .description = Snaps every color to the 216 web-safe colors.
batch-param-optimize-webSafe-dither = Dithering
batch-value-optimize-webSafe-dither-none = None
batch-value-optimize-webSafe-dither-ordered = Ordered

# B-3: animation steps (crates\fp-batch\src\steps\animfx.rs)

## Boomerang
batch-step-anim-boomerang = Boomerang
    .description = Plays the animation forward and then backward, without repeating the turning frames.
batch-param-anim-boomerang-hold = Pause at both ends

## Cross-fade loop
batch-step-anim-crossfade = Cross-fade loop
    .description = Fades the end of the animation into its start so the loop has no visible jump.
batch-param-anim-crossfade-frames = Fade frames
batch-param-anim-crossfade-mode = Fade
batch-param-anim-crossfade-delay = Duration per fade frame
    .tooltip = 0 = average frame duration.
batch-value-anim-crossfade-mode-overlap = Overlap (shorter animation)
batch-value-anim-crossfade-mode-append = Append frames
batch-err-crossfade-short = The animation has { $frames } frames; this fade needs at least { $need }.
batch-err-frame-size = The frames of the animation have different sizes.
