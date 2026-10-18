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
