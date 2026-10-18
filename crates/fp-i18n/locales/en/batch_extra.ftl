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
