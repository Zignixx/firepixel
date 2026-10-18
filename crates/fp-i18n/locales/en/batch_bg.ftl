# Batch Studio, package BATCH-6 (docs\BATCH_MODERN.md §3.5/§3.6): groups "Background" and
# "Overlay" – crates\fp-batch\src\steps\background.rs and overlay.rs.
# Steps batch-step-<id>, parameters batch-param-<id>-<param>, choice values
# batch-value-<id>-<param>-<value> (dot of the id as -), own messages batch-bg-*/batch-overlay-*.

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
batch-value-background-remove-model-birefnetLite = Fast (BiRefNet lite)
batch-value-background-remove-model-birefnet = Accurate (BiRefNet)
batch-value-background-remove-edge-hard = Hard
batch-value-background-remove-edge-antiAlias = Anti-aliased
batch-value-background-remove-edge-feather = Feathered
batch-value-background-remove-output-transparent = Transparent background
batch-value-background-remove-output-mask = Mask only (black/white)
batch-value-background-remove-output-alpha = Mask as transparency, colors unchanged

## Remove color
batch-step-background-removeColor = Remove color
    .description = Removes a plain-colored area (e.g. a white background) without AI.
batch-param-background-removeColor-pick = Color from
batch-param-background-removeColor-color = Color
batch-param-background-removeColor-tolerance = Tolerance
batch-param-background-removeColor-contiguous = Only from the edges
batch-param-background-removeColor-feather = Feather
batch-param-background-removeColor-decontaminate = Decontaminate edge colors
batch-value-background-removeColor-pick-topLeft = Top left corner
batch-value-background-removeColor-pick-topRight = Top right corner
batch-value-background-removeColor-pick-bottomLeft = Bottom left corner
batch-value-background-removeColor-pick-bottomRight = Bottom right corner
batch-value-background-removeColor-pick-custom = Custom color

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
batch-value-background-flatten-kind-color = Color
batch-value-background-flatten-kind-gradient = Gradient
batch-value-background-flatten-kind-image = Image
batch-value-background-flatten-kind-blur = Blurred copy
batch-value-background-flatten-gradientShape-linear = Linear
batch-value-background-flatten-gradientShape-radial = Radial
batch-value-background-flatten-fit-cover = Fill
batch-value-background-flatten-fit-contain = Fit
batch-value-background-flatten-fit-stretch = Stretch
batch-value-background-flatten-fit-tile = Tile
batch-value-background-flatten-edge-blend = Blend normally
batch-value-background-flatten-edge-unmix = Remove color fringe
batch-value-background-flatten-edge-hard = Hard edge

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
batch-value-background-replace-method-ai = AI (detect subject)
batch-value-background-replace-method-color = Color
batch-value-background-replace-model-birefnetLite = Fast (BiRefNet lite)
batch-value-background-replace-model-birefnet = Accurate (BiRefNet)
batch-value-background-replace-edge-hard = Hard
batch-value-background-replace-edge-antiAlias = Anti-aliased
batch-value-background-replace-edge-feather = Feathered
batch-value-background-replace-pick-topLeft = Top left corner
batch-value-background-replace-pick-topRight = Top right corner
batch-value-background-replace-pick-bottomLeft = Bottom left corner
batch-value-background-replace-pick-bottomRight = Bottom right corner
batch-value-background-replace-pick-custom = Custom color
batch-value-background-replace-kind-color = Color
batch-value-background-replace-kind-gradient = Gradient
batch-value-background-replace-kind-image = Image
batch-value-background-replace-kind-blur = Blurred copy
batch-value-background-replace-gradientShape-linear = Linear
batch-value-background-replace-gradientShape-radial = Radial
batch-value-background-replace-fit-cover = Fill
batch-value-background-replace-fit-contain = Fit
batch-value-background-replace-fit-stretch = Stretch
batch-value-background-replace-fit-tile = Tile

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
batch-value-background-shadow-style-drop = Drop shadow
batch-value-background-shadow-style-below = Floor shadow

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
batch-value-overlay-text-position-topLeft = Top left
batch-value-overlay-image-position-topLeft = Top left
batch-value-overlay-text-position-top = Top center
batch-value-overlay-image-position-top = Top center
batch-value-overlay-text-position-topRight = Top right
batch-value-overlay-image-position-topRight = Top right
batch-value-overlay-text-position-left = Middle left
batch-value-overlay-image-position-left = Middle left
batch-value-overlay-text-position-center = Center
batch-value-overlay-image-position-center = Center
batch-value-overlay-text-position-right = Middle right
batch-value-overlay-image-position-right = Middle right
batch-value-overlay-text-position-bottomLeft = Bottom left
batch-value-overlay-image-position-bottomLeft = Bottom left
batch-value-overlay-text-position-bottom = Bottom center
batch-value-overlay-image-position-bottom = Bottom center
batch-value-overlay-text-position-bottomRight = Bottom right
batch-value-overlay-image-position-bottomRight = Bottom right
batch-value-overlay-text-position-tiled = Tiled
batch-value-overlay-image-position-tiled = Tiled
batch-value-overlay-text-position-free = Free
batch-value-overlay-image-position-free = Free
batch-value-overlay-text-blend-normal = Normal
batch-value-overlay-image-blend-normal = Normal
batch-value-overlay-text-blend-multiply = Multiply
batch-value-overlay-image-blend-multiply = Multiply
batch-value-overlay-text-blend-screen = Screen
batch-value-overlay-image-blend-screen = Screen
batch-value-overlay-text-blend-overlay = Overlay
batch-value-overlay-image-blend-overlay = Overlay
batch-value-overlay-text-blend-softLight = Soft light
batch-value-overlay-image-blend-softLight = Soft light
batch-value-overlay-text-blend-hardLight = Hard light
batch-value-overlay-image-blend-hardLight = Hard light
batch-value-overlay-text-blend-darken = Darken
batch-value-overlay-image-blend-darken = Darken
batch-value-overlay-text-blend-lighten = Lighten
batch-value-overlay-image-blend-lighten = Lighten
batch-value-overlay-text-blend-difference = Difference
batch-value-overlay-image-blend-difference = Difference
batch-value-overlay-text-blend-exclusion = Exclusion
batch-value-overlay-image-blend-exclusion = Exclusion

## Border
batch-step-overlay-border = Border
    .description = Draws a border around every image, optionally with rounded corners.
batch-param-overlay-border-width = Width
batch-param-overlay-border-unit = Unit
batch-param-overlay-border-color = Color
batch-param-overlay-border-radius = Corner radius
batch-param-overlay-border-placement = Placement
batch-value-overlay-border-unit-px = Pixels
batch-value-overlay-border-unit-percent = % of the shorter side
batch-value-overlay-border-placement-outer = Outside (image grows)
batch-value-overlay-border-placement-inner = Inside (image size stays)

## Summaries
batch-bg-summary-output-transparent = Transparent background
batch-bg-summary-output-mask = Mask only
batch-bg-summary-output-alpha = Mask as transparency
batch-bg-summary-kind-color = Fill with color
batch-bg-summary-kind-gradient = Fill with gradient
batch-bg-summary-kind-image = Fill with image
batch-bg-summary-kind-blur = Fill with blurred copy

## Messages
batch-bg-err-model-missing = AI model missing: “{ $model }”. Please load it via “Manage AI Models...” or put the file into the model folder: { $folder }
batch-bg-warn-matting-missing = “Refine edge” skipped: AI model “{ $model }” is missing.
batch-bg-err-no-image = Please choose an image.
batch-bg-err-image-read = The image “{ $path }” cannot be read: { $error }
batch-bg-err-image-data = The embedded image is damaged.
batch-overlay-warn-no-font = Text not set: no suitable font found.
