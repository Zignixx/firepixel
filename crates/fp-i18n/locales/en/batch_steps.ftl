# Batch Studio: steps of the groups Geometry, Color and effects, Advanced (package BATCH-4,
# docs\BATCH_MODERN.md §3.3, §3.4, §3.13; code crates\fp-batch\src\steps\{geometry,adjust,script}.rs).
# Steps batch-step-<id>, parameters batch-param-<id>-<param>, choice values
# batch-value-<id>-<param>-<value>, summaries batch-summary-<id>-…

## Geometry
batch-step-resize = Resize
    .description = Shrinks or enlarges the images to a fixed size, by percent or to fit a frame.
batch-param-resize-mode = Mode
batch-param-resize-size = Side length
batch-param-resize-width = Width
batch-param-resize-height = Height
batch-param-resize-percent = Percent
batch-param-resize-onlyShrink = Only shrink
batch-param-resize-filter = Resampling
batch-param-resize-dpi = Resolution
batch-value-resize-mode-percent = By percent
batch-value-resize-mode-width = Set width
batch-value-resize-mode-height = Set height
batch-value-resize-mode-longEdge = Long edge
batch-value-resize-mode-shortEdge = Short edge
batch-value-resize-mode-fit = Fit in frame
batch-value-resize-mode-fill = Fill frame (crop)
batch-value-resize-mode-exact = Exact (distort)
batch-value-resize-mode-none = Don't change
batch-value-resize-filter-auto = Automatic
batch-value-resize-filter-nearest = Nearest neighbor
batch-value-resize-filter-bilinear = Bilinear
batch-value-resize-filter-bicubic = Bicubic
batch-value-resize-filter-lanczos3 = Lanczos (best quality)
batch-summary-resize-percent = To { $percent }%
batch-summary-resize-width = Width { $width } px
batch-summary-resize-height = Height { $height } px
batch-summary-resize-longEdge = Long edge { $size } px
batch-summary-resize-shortEdge = Short edge { $size } px
batch-summary-resize-fit = Fit in { $width } × { $height } px
batch-summary-resize-fill = Fill { $width } × { $height } px
batch-summary-resize-exact = Exactly { $width } × { $height } px
batch-summary-resize-none = Size unchanged

batch-step-crop = Crop
    .description = Crops to an aspect ratio, an area or by fixed edges.
batch-param-crop-mode = Mode
batch-param-crop-ratio = Aspect ratio
batch-param-crop-ratioWidth = Ratio width
batch-param-crop-ratioHeight = Ratio height
batch-param-crop-anchor = Alignment
batch-param-crop-unit = Unit
batch-param-crop-x = Left
batch-param-crop-y = Top
batch-param-crop-width = Width
batch-param-crop-height = Height
batch-param-crop-top = Top
batch-param-crop-right = Right
batch-param-crop-bottom = Bottom
batch-param-crop-left = Left
batch-value-crop-mode-aspect = Aspect ratio
batch-value-crop-mode-rect = Area
batch-value-crop-mode-edges = Remove edges
batch-value-crop-ratio-free = Free
batch-value-crop-unit-px = Pixels
batch-value-crop-unit-percent = Percent
batch-summary-crop-aspect = Crop to { $ratio }
batch-summary-crop-rect = Cut out an area
batch-summary-crop-edges = Cut off edges
batch-warn-crop-empty = The crop lies outside the image; the image stays unchanged.

batch-step-trim = Trim
    .description = Removes empty borders: transparent areas or the color of the top left corner.
batch-param-trim-what = What to remove
batch-param-trim-tolerance = Tolerance
batch-param-trim-padding = Keep padding
batch-value-trim-what-transparent = Transparent borders
batch-value-trim-what-color = Corner color
batch-value-trim-what-both = Both
batch-summary-trim-transparent = Remove transparent borders
batch-summary-trim-color = Remove solid borders
batch-summary-trim-both = Remove transparent and solid borders
batch-warn-trim-empty = The image has nothing but border; it stays unchanged.

batch-step-rotate = Rotate/Flip
    .description = Rotates or flips the images; optionally straightens photos by the camera orientation (EXIF).
batch-param-rotate-angle = Angle
batch-param-rotate-flip = Flip
batch-param-rotate-autoExif = Straighten by camera orientation
batch-param-rotate-background = Background
batch-param-rotate-filter = Resampling
batch-value-rotate-flip-none = Don't flip
batch-value-rotate-flip-horizontal = Horizontal
batch-value-rotate-flip-vertical = Vertical
batch-value-rotate-flip-both = Both
batch-value-rotate-filter-bicubic = Bicubic
batch-value-rotate-filter-bilinear = Bilinear
batch-value-rotate-filter-nearest = Nearest neighbor
batch-summary-rotate = Rotate by { $angle }°
batch-summary-rotate-flip = Rotate by { $angle }° and flip

batch-step-canvas = Canvas/Border
    .description = Adds a border or brings the canvas to a size, an aspect ratio or a square.
batch-param-canvas-mode = Mode
batch-param-canvas-border = Border
batch-param-canvas-top = Top
batch-param-canvas-right = Right
batch-param-canvas-bottom = Bottom
batch-param-canvas-left = Left
batch-param-canvas-width = Width
batch-param-canvas-height = Height
batch-param-canvas-ratio = Aspect ratio
batch-param-canvas-ratioWidth = Ratio width
batch-param-canvas-ratioHeight = Ratio height
batch-param-canvas-color = Color
batch-param-canvas-anchor = Image alignment
batch-value-canvas-mode-addBorder = Add border
batch-value-canvas-mode-toSize = To size
batch-value-canvas-mode-toAspect = To aspect ratio
batch-value-canvas-mode-square = Square
batch-value-canvas-ratio-free = Free
batch-summary-canvas-addBorder = Border { $border } px
batch-summary-canvas-toSize = Canvas { $width } × { $height } px
batch-summary-canvas-toAspect = Canvas to aspect ratio
batch-summary-canvas-square = Square canvas

## Alignment (anchor) for crop and canvas
batch-value-crop-anchor-topLeft = Top left
batch-value-crop-anchor-top = Top
batch-value-crop-anchor-topRight = Top right
batch-value-crop-anchor-left = Left
batch-value-crop-anchor-center = Center
batch-value-crop-anchor-right = Right
batch-value-crop-anchor-bottomLeft = Bottom left
batch-value-crop-anchor-bottom = Bottom
batch-value-crop-anchor-bottomRight = Bottom right
batch-value-canvas-anchor-topLeft = Top left
batch-value-canvas-anchor-top = Top
batch-value-canvas-anchor-topRight = Top right
batch-value-canvas-anchor-left = Left
batch-value-canvas-anchor-center = Center
batch-value-canvas-anchor-right = Right
batch-value-canvas-anchor-bottomLeft = Bottom left
batch-value-canvas-anchor-bottom = Bottom
batch-value-canvas-anchor-bottomRight = Bottom right

batch-err-image-too-large = The image would be too large at { $width } × { $height } pixels.

## Color and effects
batch-step-adjust-auto = Auto enhance
    .description = Automatically corrects levels, contrast and, optionally, white balance.
batch-param-adjust-auto-levels = Levels
batch-param-adjust-auto-contrast = Contrast
batch-param-adjust-auto-whiteBalance = White balance

batch-step-adjust-basic = Brightness/Contrast/Saturation
    .description = Adjusts brightness, contrast, saturation, hue, gamma and color temperature.
batch-param-adjust-basic-brightness = Brightness
batch-param-adjust-basic-contrast = Contrast
batch-param-adjust-basic-saturation = Saturation
batch-param-adjust-basic-hue = Hue
batch-param-adjust-basic-gamma = Gamma
batch-param-adjust-basic-temperature = Color temperature
batch-param-adjust-basic-tint = Tint
batch-summary-adjust-basic = Brightness { $brightness }, contrast { $contrast }, saturation { $saturation }

batch-step-adjust-levels = Levels
    .description = Black point, gamma and white point as in the Levels dialog.
batch-param-adjust-levels-inputLow = Shadows
batch-param-adjust-levels-gamma = Midtones
batch-param-adjust-levels-inputHigh = Highlights
batch-param-adjust-levels-outputLow = Output shadows
batch-param-adjust-levels-outputHigh = Output highlights
batch-param-adjust-levels-channels = All channels (JSON)

batch-step-adjust-curves = Curves
    .description = Changes the tones with curve points as in the Curves dialog.
batch-param-adjust-curves-points = Curve points
batch-param-adjust-curves-channels = All channels (JSON)
batch-issue-curve-points = Give the curve points as a list [[x, y], …] with at least two points.
batch-issue-json-invalid = Invalid value: { $error }

batch-step-adjust-grayscale = Black and white/Sepia/Duotone
    .description = Converts the images to grayscale, sepia or two tones.
batch-param-adjust-grayscale-style = Style
batch-param-adjust-grayscale-dark = Dark color
batch-param-adjust-grayscale-light = Light color
batch-value-adjust-grayscale-style-gray = Black and white
batch-value-adjust-grayscale-style-sepia = Sepia
batch-value-adjust-grayscale-style-duotone = Duotone

batch-step-adjust-invert = Invert
    .description = Inverts the colors (negative).

batch-step-filter-sharpen = Sharpen
    .description = Sharpens the images with Unsharp Mask.
batch-param-filter-sharpen-amount = Amount
batch-param-filter-sharpen-radius = Radius
batch-param-filter-sharpen-threshold = Threshold
batch-summary-filter-sharpen = Sharpen { $amount }%

batch-step-filter-blur = Blur
    .description = Blurs the images: Gaussian, motion, radial or zoom.
batch-param-filter-blur-kind = Type
batch-param-filter-blur-radius = Radius
batch-param-filter-blur-angle = Angle
batch-param-filter-blur-amount = Amount
batch-param-filter-blur-quality = Quality
batch-value-filter-blur-kind-gaussian = Gaussian Blur
batch-value-filter-blur-kind-motion = Motion Blur
batch-value-filter-blur-kind-radial = Radial Blur
batch-value-filter-blur-kind-zoom = Zoom Blur
batch-summary-filter-blur = { $kind }, { $radius } px

batch-step-filter-registry = Any filter
    .description = Applies a filter from the Filters menu with its settings.
batch-param-filter-registry-command = Filter
batch-param-filter-registry-params = Settings (JSON)

batch-step-effect-live = Live effect
    .description = Applies a Firepixel effect such as a drop shadow or glow to the image and flattens it.
batch-param-effect-live-effect = Effect
batch-param-effect-live-params = Settings (JSON)
batch-param-effect-live-expandCanvas = Expand canvas
batch-value-effect-live-effect-dropShadow = Drop Shadow
batch-value-effect-live-effect-innerShadow = Inner Shadow
batch-value-effect-live-effect-glow = Glow
batch-value-effect-live-effect-innerGlow = Inner Glow
batch-value-effect-live-effect-bevel = Inner Bevel
batch-value-effect-live-effect-outerBevel = Outer Bevel
batch-value-effect-live-effect-emboss = Raised Emboss
batch-value-effect-live-effect-insetEmboss = Inset Emboss
batch-value-effect-live-effect-solidShadow = Solid Shadow
batch-value-effect-live-effect-colorFill = Color Fill
batch-warn-effect-unsupported = The effect “{ $effect }” could not be fully rendered.

## Advanced
batch-step-script-registry = Firepixel command
    .description = Runs one or more Firepixel commands with their parameters.
batch-param-script-registry-command = Command
batch-param-script-registry-params = Parameters (JSON)
batch-param-script-registry-commands = More commands (JSON)
batch-param-script-registry-selectAll = Select all first
batch-summary-script-registry-many = { $count } commands

batch-step-script-command = Saved command
    .description = Runs a saved command (script) from the Commands folder.
batch-param-script-command-name = Command
batch-param-script-command-path = Script file

batch-issue-script-no-command = Please enter a command.
batch-issue-script-blocked = The command “{ $command }” is not allowed in batch processing.
batch-issue-script-unknown = Unknown command “{ $command }”.
batch-issue-script-no-file = Please choose a saved command or a script file.
batch-issue-script-missing = The script file is missing: { $path }
batch-issue-script-not-for-batch = This command is marked “not for batch”.
