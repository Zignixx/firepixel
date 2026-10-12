# Properties inspector (crates\fp-ui\src\inspector\*). Owner: I18N-INSPEKTOR.
# The fields have fixed widths like the original (dialog units): keep translations short
# (guide: no longer than the English text). Longer texts are scaled down or shortened
# with "…" (full text in the tooltip).

## Tabs and inspector menu
pi-tab-properties = Properties
pi-tab-symbol-properties = Symbol Properties
pi-menu-full-height = Full Height
pi-menu-half-height = Half Height

## Common field labels
pi-edit = Edit...
pi-none = None
pi-no-entries = (no entries)
pi-edge = Edge:
pi-texture = Texture:
pi-shape = Shape:
pi-size = Size:
pi-tolerance = Tolerance:
pi-strength = Strength:
pi-pressure = Pressure:
pi-link = Link:
pi-alt = Alt:
pi-target = Target:
pi-opacity = Opacity:

## Document, tool, selection (quadrant 1/2)
pi-doc-title = Document
pi-doc-canvas = Canvas:
pi-doc-canvas-size = Canvas Size...
pi-doc-image-size = Image Size...
pi-doc-fit-canvas = Fit Canvas
pi-doc-state = State:
pi-multi-objects = { $n ->
    [one] { $n } object
   *[other] { $n } objects
}
pi-box-keep-proportions = Constrain proportions
pi-box-w = W:
pi-box-h = H:
pi-box-x = X:
pi-box-y = Y:
pi-box-limit = The value must be between 1 and { $max }; limited to { $value }.

## Element types (quadrant 1)
pi-type-bitmap = Bitmap
pi-type-rectangle = Rectangle
pi-type-path = Path
pi-type-text = Text
pi-type-group = Group ({ $n })
pi-type-compound = Compound Shape
pi-type-symbol = Symbol
pi-type-button = Button
pi-type-animation = Animation
pi-type-slice = Slice
pi-type-hotspot = Hotspot
pi-type-sublayer = Sub-layer
pi-type-object = Object

## Fill
pi-fill-none = No Fill
pi-fill-solid = Solid
pi-fill-gradient = Gradient
pi-fill-pattern = Pattern
pi-pattern-other = Other...
pi-pattern-pick-title = Choose Pattern
pi-filter-images = Images
pi-edge-hard = Hard
pi-edge-antialias = Anti-Alias
pi-edge-feather-short = Feather
pi-edge-feather = Feather
pi-texture-transparent = Transparent
pi-gradient-dither = Gradient dither

## Gradient shapes (gradient editor, fill categories)
pi-grad-linear = Linear
pi-grad-radial = Radial
pi-grad-cone = Cone
pi-grad-satin = Satin
pi-grad-starburst = Starburst
pi-grad-folds = Folds
pi-grad-elliptical = Elliptical
pi-grad-rectangle = Rectangle
pi-grad-bars = Bars
pi-grad-ripples = Ripples
pi-grad-waves = Waves
pi-grad-reverse = Reverse
pi-grad-color = Color:
pi-grad-delete = Delete

## Stroke
pi-stroke-inside = Align stroke inside
pi-stroke-center = Center stroke on path
pi-stroke-outside = Align stroke outside
pi-stroke-edit = Edit Stroke
pi-stroke-save = Save stroke
pi-stroke-delete = Delete stroke
pi-stroke-save-name = Name:
pi-stroke-default-name = Stroke { $n }
pi-stroke-cat-none = None
pi-stroke-saved = Saved

## Stroke categories and presets
pi-brush-cat-pencil = Pencil
pi-brush-cat-basic = Basic
pi-brush-cat-air-brush = Air Brush
pi-brush-cat-calligraphy = Calligraphy
pi-brush-cat-charcoal = Charcoal
pi-brush-cat-crayon = Crayon
pi-brush-cat-dashed = Dashed
pi-brush-cat-felt-tip = Felt Tip
pi-brush-cat-oil = Oil
pi-brush-cat-watercolor = Watercolor
pi-brush-pencil-1-pixel-hard = 1-Pixel Hard
pi-brush-pencil-1-pixel-soft = 1-Pixel Soft
pi-brush-pencil-colored-pencil = Colored Pencil
pi-brush-pencil-graphite = Graphite
pi-brush-basic-hard-line = Hard Line
pi-brush-basic-soft-line = Soft Line
pi-brush-basic-hard-rounded = Hard Rounded
pi-brush-basic-soft-rounded = Soft Rounded
pi-brush-air-brush-basic = Basic
pi-brush-air-brush-textured = Textured
pi-brush-calligraphy-bamboo = Bamboo
pi-brush-calligraphy-quill = Quill
pi-brush-charcoal-soft = Soft
pi-brush-charcoal-pastel = Pastel
pi-brush-crayon-basic = Basic
pi-brush-dashed-basic-dash = Basic Dash
pi-brush-dashed-dotted = Dotted
pi-brush-dashed-double-dash = Double Dash
pi-brush-felt-tip-light-marker = Light Marker
pi-brush-felt-tip-dark-marker = Dark Marker
pi-brush-oil-bristle = Bristle
pi-brush-watercolor-light = Light
pi-brush-watercolor-thick = Thick

## Live filters, style, path operations
pi-filters = Filters:
pi-fx-add = Add Live Filters or choose a preset
pi-fx-delete = Delete the selected Live Filter
pi-roundness = Roundness:
pi-pathop-normal = Normal
pi-pathop-union = Add/Union
pi-pathop-punch = Subtract/Punch
pi-pathop-intersect = Intersect
pi-pathop-crop = Crop
pi-combine = Combine
pi-style-none = No Style
pi-style-new = New Style
pi-style-redefine = Redefine Style
pi-style-clear = Clear Overrides
pi-style-unlink = Break Link to Style
pi-style-delete = Delete Style

## Animation, button, slice, hotspot
pi-anim-states = States:
pi-anim-to = to
pi-anim-scaling = Scaling:
pi-anim-rotation = Rotation:
pi-anim-cw = Rotate clockwise
pi-anim-ccw = Rotate counterclockwise
pi-btn-text = Text:
pi-btn-show-down = Show Down state on load
pi-slice-type = Type:
pi-slice-foreground = Foreground Image
pi-slice-background = Background Image
pi-slice-hpos = Horizontal position:
pi-slice-vpos = Vertical position:
pi-shape-rectangle = Rectangle
pi-shape-ellipse = Ellipse
pi-shape-polygon = Polygon

## Text
pi-text-bold = Faux Bold
pi-text-italic = Faux Italic
pi-text-underline = Underline
pi-text-align-left = Left align
pi-text-align-center = Center
pi-text-align-right = Right align
pi-text-align-justify = Justify
pi-text-align-stretch = Stretch alignment
pi-aa-none = No Anti-Alias
pi-aa-crisp = Crisp Anti-Alias
pi-aa-strong = Strong Anti-Alias
pi-aa-smooth = Smooth Anti-Alias
pi-aa-custom = Custom Anti-Alias...
pi-text-offset = Text offset:
pi-text-autokern = Auto kern
pi-mask-editing = Editing bitmap mask

## Swatches popup
pi-color-pipette = Eyedropper
pi-color-system = System color picker
pi-color-transparent = Transparent

## Tool options
pi-rect-roundness = Rectangle roundness:
pi-poly-star = Star
pi-poly-polygon = Polygon
pi-poly-sides = Sides:
pi-poly-angle = Angle:
pi-poly-auto = Auto
pi-path-precision = Precision:
pi-keep-transparency = Preserve transparency
pi-tip-round = Round tip
pi-tip-square = Square tip
pi-tip-round-brush = Round brush tip
pi-tip-square-brush = Square brush tip
pi-eyedrop-sample = Sample:
pi-eyedrop-1 = 1 Pixel
pi-eyedrop-3 = 3x3 Average
pi-eyedrop-5 = 5x5 Average
pi-color-format = Color format:
pi-live-marquee = Live marquee
pi-marquee-style = Style:
pi-marquee-normal = Normal
pi-marquee-fixed-ratio = Fixed Ratio
pi-marquee-fixed-size = Fixed Size
pi-marquee-w = W
pi-marquee-h = H
pi-fill-selection = Fill selection
pi-stamp-aligned = Source aligned
pi-whole-doc = Entire document
pi-antialias = Anti-Alias
pi-auto-erase = Auto erase
pi-intensity = Intensity:
pi-range = Range:
pi-range-shadows = Shadows
pi-range-midtones = Midtones
pi-range-highlights = Highlights
pi-exposure = Exposure:
pi-smudge-color = Smudge color
pi-check-pressure = Pressure
pi-check-size = Size
pi-check-strength = Strength
pi-check-preview = Preview
pi-check-velocity = Velocity
pi-rate = Rate:
pi-replace-from = From:
pi-replace-swatch = Swatch
pi-replace-image = Image
pi-replace-to = to:
pi-colorize = Colorize
pi-mask = Mask:
pi-mask-alpha = Alpha channel
pi-mask-gray = Grayscale

## AI tools
pi-ai-model = Model:
pi-ai-quality = Quality:
pi-ai-source = Source:
pi-ai-source-auto = Bitmap, else page
pi-ai-source-page = Visible page
pi-ai-sam-tiny = SAM 2.1 Tiny (fast)
pi-ai-sam-small = SAM 2.1 Small (precise)
pi-ai-subject-fast = Fast (BiRefNet lite)
pi-ai-subject-high = High (BiRefNet, slow)
pi-ai-matting-fast = Fast (Matting lite)
pi-ai-matting-high = High (HR matting, slow)
pi-ai-model-missing = Model missing – offered on first use
pi-ai-models = AI Models…
pi-ai-object-hint1 = Click: select object · Drag a box: object inside the box
pi-ai-object-hint2 = Shift+click: add · Alt+click: subtract · Esc: new object
pi-ai-subject-hint = Click: main subject as soft pixel selection · Shift: add · Alt: subtract
pi-ai-subject-run = Select Subject
pi-ai-radius = Radius:
pi-ai-radius-auto = (0 = automatic)
pi-ai-pixels = pixels
pi-ai-output = Output:
pi-ai-output-selection = Soft pixel selection
pi-ai-output-mask = Bitmap mask
pi-ai-refine-run = Refine Edge Now
pi-ai-refine-hint = Click inside the selection: refine edge
pi-ai-need-selection = Create a pixel selection first
