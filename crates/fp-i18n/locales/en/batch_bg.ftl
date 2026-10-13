# Batch Studio, package BATCH-6 (docs\BATCH_MODERN.md §3.5/§3.6): groups "Background" and
# "Overlay" – crates\fp-batch\src\steps\background.rs and overlay.rs.
# Steps batch-step-<id>, parameters batch-param-<id>-<param>, choice values
# batch-choice-<id>-<param>-<value> (dot of the id as -), own messages batch-bg-*/batch-overlay-*.

## Remove background (AI)
batch-step-background-remove = Remove background
    .description = Cuts out the main subject with AI and makes the background transparent.
batch-param-background-remove-model = AI model
batch-param-background-remove-edge = Edge
batch-param-background-remove-feather = Feather
batch-param-background-remove-tolerance = Tolerance
batch-param-background-remove-expand = Expand/contract
batch-param-background-remove-smooth = Smooth
batch-param-background-remove-refineEdge = Refine edge (hair)
batch-param-background-remove-decontaminate = Decontaminate edge colors
batch-param-background-remove-output = Result
batch-choice-background-remove-model-birefnetLite = Fast (BiRefNet lite)
batch-choice-background-remove-model-birefnet = Accurate (BiRefNet)
batch-choice-background-remove-edge-hard = Hard
batch-choice-background-remove-edge-antiAlias = Anti-aliased
batch-choice-background-remove-edge-feather = Feathered
batch-choice-background-remove-output-transparent = Transparent background
batch-choice-background-remove-output-mask = Mask only (black/white)
batch-choice-background-remove-output-alpha = Mask as transparency, colors unchanged

## Remove color
batch-step-background-removeColor = Remove color
    .description = Removes a plain-colored area (e.g. a white background) without AI.
batch-param-background-removeColor-pick = Color from
batch-param-background-removeColor-color = Color
batch-param-background-removeColor-tolerance = Tolerance
batch-param-background-removeColor-contiguous = Only from the edges
batch-param-background-removeColor-feather = Feather
batch-param-background-removeColor-decontaminate = Decontaminate edge colors
batch-choice-background-removeColor-pick-topLeft = Top left corner
batch-choice-background-removeColor-pick-topRight = Top right corner
batch-choice-background-removeColor-pick-bottomLeft = Bottom left corner
batch-choice-background-removeColor-pick-bottomRight = Bottom right corner
batch-choice-background-removeColor-pick-custom = Custom color

## Transparency → color
batch-step-background-flatten = Fill transparency
    .description = Fills transparent areas with a color, a gradient, an image or a blurred copy.
batch-param-background-flatten-kind = Fill
batch-param-background-flatten-color = Color
batch-param-background-flatten-color2 = Second color
batch-param-background-flatten-gradientShape = Gradient shape
batch-param-background-flatten-angle = Angle
batch-param-background-flatten-image = Image
batch-param-background-flatten-imageData = Embedded image
batch-param-background-flatten-fit = Fit
batch-param-background-flatten-blur = Blur
batch-param-background-flatten-onlyIfTransparent = Only images with transparency
batch-param-background-flatten-edge = Edges
batch-param-background-flatten-matte = Old background color
batch-param-background-flatten-threshold = Threshold
batch-param-background-flatten-linearLight = Blend in linear light
batch-choice-background-flatten-kind-color = Color
batch-choice-background-flatten-kind-gradient = Gradient
batch-choice-background-flatten-kind-image = Image
batch-choice-background-flatten-kind-blur = Blurred copy
batch-choice-background-flatten-gradientShape-linear = Linear
batch-choice-background-flatten-gradientShape-radial = Radial
batch-choice-background-flatten-fit-cover = Fill
batch-choice-background-flatten-fit-contain = Fit
batch-choice-background-flatten-fit-stretch = Stretch
batch-choice-background-flatten-fit-tile = Tile
batch-choice-background-flatten-edge-blend = Blend normally
batch-choice-background-flatten-edge-unmix = Remove color fringe
batch-choice-background-flatten-edge-hard = Hard edge

## Replace background
batch-step-background-replace = Replace background
    .description = Removes the background (AI or color) and puts a new fill behind the subject.
batch-param-background-replace-method = Cut out with
batch-param-background-replace-model = AI model
batch-param-background-replace-edge = Edge
batch-param-background-replace-feather = Feather
batch-param-background-replace-tolerance = Tolerance
batch-param-background-replace-expand = Expand/contract
batch-param-background-replace-smooth = Smooth
batch-param-background-replace-refineEdge = Refine edge (hair)
batch-param-background-replace-decontaminate = Decontaminate edge colors
batch-param-background-replace-pick = Color from
batch-param-background-replace-keyColor = Color to remove
batch-param-background-replace-keyTolerance = Color tolerance
batch-param-background-replace-contiguous = Only from the edges
batch-param-background-replace-kind = New background
batch-param-background-replace-color = Color
batch-param-background-replace-color2 = Second color
batch-param-background-replace-gradientShape = Gradient shape
batch-param-background-replace-angle = Angle
batch-param-background-replace-image = Image
batch-param-background-replace-imageData = Embedded image
batch-param-background-replace-fit = Fit
batch-param-background-replace-blur = Blur
batch-param-background-replace-linearLight = Blend in linear light
batch-choice-background-replace-method-ai = AI (detect subject)
batch-choice-background-replace-method-color = Color
batch-choice-background-replace-model-birefnetLite = Fast (BiRefNet lite)
batch-choice-background-replace-model-birefnet = Accurate (BiRefNet)
batch-choice-background-replace-edge-hard = Hard
batch-choice-background-replace-edge-antiAlias = Anti-aliased
batch-choice-background-replace-edge-feather = Feathered
batch-choice-background-replace-pick-topLeft = Top left corner
batch-choice-background-replace-pick-topRight = Top right corner
batch-choice-background-replace-pick-bottomLeft = Bottom left corner
batch-choice-background-replace-pick-bottomRight = Bottom right corner
batch-choice-background-replace-pick-custom = Custom color
batch-choice-background-replace-kind-color = Color
batch-choice-background-replace-kind-gradient = Gradient
batch-choice-background-replace-kind-image = Image
batch-choice-background-replace-kind-blur = Blurred copy
batch-choice-background-replace-gradientShape-linear = Linear
batch-choice-background-replace-gradientShape-radial = Radial
batch-choice-background-replace-fit-cover = Fill
batch-choice-background-replace-fit-contain = Fit
batch-choice-background-replace-fit-stretch = Stretch
batch-choice-background-replace-fit-tile = Tile

## Shadow
batch-step-background-shadow = Add shadow
    .description = Puts a soft shadow under the cut-out subject.
batch-param-background-shadow-style = Style
batch-param-background-shadow-color = Color
batch-param-background-shadow-opacity = Opacity
batch-param-background-shadow-distance = Distance
batch-param-background-shadow-angle = Angle
batch-param-background-shadow-blur = Softness
batch-param-background-shadow-expandCanvas = Expand canvas
batch-choice-background-shadow-style-drop = Drop shadow
batch-choice-background-shadow-style-below = Floor shadow

## Text/watermark
batch-step-overlay-text = Text/watermark
    .description = Writes text on every image, e.g. a copyright, the file name or the date.
batch-param-overlay-text-text = Text
    .tooltip = Placeholders: {"{"}name{"}"}, {"{"}date{"}"}, {"{"}exif-date{"}"}, {"{"}year{"}"}, {"{"}n{"}"}, {"{"}n:3{"}"}, {"{"}w{"}"}, {"{"}h{"}"}; (c) becomes ©.
batch-param-overlay-text-size = Size
batch-param-overlay-text-font = Font
batch-param-overlay-text-bold = Bold
batch-param-overlay-text-italic = Italic
batch-param-overlay-text-color = Color
batch-param-overlay-text-opacity = Opacity
batch-param-overlay-text-shadow = Shadow
batch-param-overlay-text-outline = Outline
batch-param-overlay-text-outlineColor = Outline color
batch-param-overlay-text-outlineWidth = Outline width
batch-param-overlay-text-position = Position
batch-param-overlay-text-x = Horizontal position
batch-param-overlay-text-y = Vertical position
batch-param-overlay-text-margin = Margin
batch-param-overlay-text-rotation = Rotation
batch-param-overlay-text-spacing = Tile spacing
batch-param-overlay-text-blend = Blend mode

## Logo/image
batch-step-overlay-image = Insert logo/image
    .description = Places a logo or image as a watermark on every image.
batch-param-overlay-image-image = Image
batch-param-overlay-image-imageData = Embedded image
batch-param-overlay-image-scale = Size
batch-param-overlay-image-opacity = Opacity
batch-param-overlay-image-position = Position
batch-param-overlay-image-x = Horizontal position
batch-param-overlay-image-y = Vertical position
batch-param-overlay-image-margin = Margin
batch-param-overlay-image-rotation = Rotation
batch-param-overlay-image-spacing = Tile spacing
batch-param-overlay-image-blend = Blend mode

## Positions and blend modes (per step, text and image)
batch-choice-overlay-text-position-topLeft = Top left
batch-choice-overlay-image-position-topLeft = Top left
batch-choice-overlay-text-position-top = Top center
batch-choice-overlay-image-position-top = Top center
batch-choice-overlay-text-position-topRight = Top right
batch-choice-overlay-image-position-topRight = Top right
batch-choice-overlay-text-position-left = Middle left
batch-choice-overlay-image-position-left = Middle left
batch-choice-overlay-text-position-center = Center
batch-choice-overlay-image-position-center = Center
batch-choice-overlay-text-position-right = Middle right
batch-choice-overlay-image-position-right = Middle right
batch-choice-overlay-text-position-bottomLeft = Bottom left
batch-choice-overlay-image-position-bottomLeft = Bottom left
batch-choice-overlay-text-position-bottom = Bottom center
batch-choice-overlay-image-position-bottom = Bottom center
batch-choice-overlay-text-position-bottomRight = Bottom right
batch-choice-overlay-image-position-bottomRight = Bottom right
batch-choice-overlay-text-position-tiled = Tiled
batch-choice-overlay-image-position-tiled = Tiled
batch-choice-overlay-text-position-free = Free
batch-choice-overlay-image-position-free = Free
batch-choice-overlay-text-blend-normal = Normal
batch-choice-overlay-image-blend-normal = Normal
batch-choice-overlay-text-blend-multiply = Multiply
batch-choice-overlay-image-blend-multiply = Multiply
batch-choice-overlay-text-blend-screen = Screen
batch-choice-overlay-image-blend-screen = Screen
batch-choice-overlay-text-blend-overlay = Overlay
batch-choice-overlay-image-blend-overlay = Overlay
batch-choice-overlay-text-blend-softLight = Soft light
batch-choice-overlay-image-blend-softLight = Soft light
batch-choice-overlay-text-blend-hardLight = Hard light
batch-choice-overlay-image-blend-hardLight = Hard light
batch-choice-overlay-text-blend-darken = Darken
batch-choice-overlay-image-blend-darken = Darken
batch-choice-overlay-text-blend-lighten = Lighten
batch-choice-overlay-image-blend-lighten = Lighten
batch-choice-overlay-text-blend-difference = Difference
batch-choice-overlay-image-blend-difference = Difference
batch-choice-overlay-text-blend-exclusion = Exclusion
batch-choice-overlay-image-blend-exclusion = Exclusion

## Border
batch-step-overlay-border = Border
    .description = Draws a border around every image, optionally with rounded corners.
batch-param-overlay-border-width = Width
batch-param-overlay-border-unit = Unit
batch-param-overlay-border-color = Color
batch-param-overlay-border-radius = Corner radius
batch-param-overlay-border-placement = Placement
batch-choice-overlay-border-unit-px = Pixels
batch-choice-overlay-border-unit-percent = % of the shorter side
batch-choice-overlay-border-placement-outer = Outside (image grows)
batch-choice-overlay-border-placement-inner = Inside (image size stays)

## Summaries
batch-bg-summary-output-transparent = Transparent background
batch-bg-summary-output-mask = Mask only
batch-bg-summary-output-alpha = Mask as transparency
batch-bg-summary-kind-color = Fill with color
batch-bg-summary-kind-gradient = Fill with gradient
batch-bg-summary-kind-image = Fill with image
batch-bg-summary-kind-blur = Fill with blurred copy

## Messages
batch-bg-err-model-missing = AI model missing: "{ $model }". Please load it via "Manage AI models…" or put the file into the models folder: { $folder }
batch-bg-warn-matting-missing = "Refine edge" skipped: AI model "{ $model }" is missing.
batch-bg-err-no-image = Please choose an image.
batch-bg-err-image-read = The image "{ $path }" cannot be read: { $error }
batch-bg-err-image-data = The embedded image is damaged.
batch-overlay-warn-no-font = Text not set: no suitable font found.
