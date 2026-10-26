# Panels (crates\fp-ui\src\panels\*), prefix panel-. Owner: I18N-BEDIENFELDER.
# Panels are narrow: keep texts short; long texts are shortened with “…”.

## Tab titles (Window menu, dock)
panel-title-optimize = Optimize
panel-title-history = History
panel-title-align = Align
panel-title-pages = Pages
panel-title-states = States
panel-title-layers = Layers
panel-title-styles = Styles
panel-title-color-palette = Color Palette
panel-title-swatches = Swatches
panel-title-special-characters = Special Characters
panel-title-image-editing = Image Editing
panel-title-path = Path
panel-title-auto-shapes = Auto Shapes
panel-title-document-library = Document Library
panel-title-common-library = Common Library
panel-title-info = Info
panel-title-color-mixer = Color Mixer
panel-title-url = URL
panel-title-behaviors = Behaviors
panel-title-find-replace = Find
panel-title-symbol-properties = Symbol Properties
panel-title-auto-shape-properties = Auto Shape Properties
panel-title-css-properties = CSS Properties

## General
# Display name of a state without its own name
panel-state-default-name = State { $n }

## Align
panel-align-relative-canvas = Relative to canvas
panel-align-relative-object = Relative to object
panel-align-to-canvas = Align to canvas
panel-align-align = Align:
panel-align-left = Align left edge
panel-align-hcenter = Align horizontal center
panel-align-right = Align right edge
panel-align-top = Align top edge
panel-align-vcenter = Align vertical center
panel-align-bottom = Align bottom edge
panel-align-distribute = Distribute:
panel-align-dist-top = Distribute top edges
panel-align-dist-vcenter = Distribute vertical centers
panel-align-dist-bottom = Distribute bottom edges
panel-align-dist-left = Distribute left edges
panel-align-dist-hcenter = Distribute horizontal centers
panel-align-dist-right = Distribute right edges
panel-align-size = Match size:
panel-align-match-width = Match width
panel-align-match-height = Match height
panel-align-match-both = Match width and height
panel-align-space = Space:
panel-align-space-v = Space evenly vertically
panel-align-space-h = Space evenly horizontally
panel-align-space-even = Evenly

## History
panel-history-copy-steps = Copy Steps
panel-history-save-command = Save as Command
panel-history-replay = Replay
panel-history-replay-label = Replay Steps
panel-history-copied = Steps copied to the clipboard
panel-history-attached-tip = This step has no undo step of its own; it is undone together with the step before it and cannot be replayed on its own.
panel-history-not-replayable-tip = This step cannot be replayed or saved as a command.

## Pages
panel-pages-count = { $n ->
    [one] { $n } page
   *[other] { $n } pages
}
panel-pages-new = New/Duplicate Page
panel-pages-delete = Delete Page
panel-pages-invalid-name = Invalid character in page name: \ / : * ? " < > |

## States
panel-states-no-loop = No Looping
panel-states-forever = Forever
panel-states-delay-title = State Delay
panel-states-delay-label = State delay:
panel-states-delay-unit = /100 sec
panel-states-include-export = Include when exporting
panel-states-onion = Onion Skinning
panel-states-loop = GIF Animation Looping
panel-states-distribute = Distribute to States
panel-states-new = New/Duplicate State
panel-states-delete = Delete State

## Layers
panel-layers-opacity = Opacity
panel-layers-web-layer = Web Layer
panel-layers-kind-bitmap = Bitmap
panel-layers-kind-bitmap-mask = Bitmap mask
panel-layers-kind-rectangle = Rectangle
panel-layers-kind-path = Path
panel-layers-kind-text = Text
panel-layers-kind-group = Group
panel-layers-kind-compound = Compound Shape
panel-layers-kind-symbol = Symbol
panel-layers-kind-slice = Slice
panel-layers-kind-hotspot = Hotspot
panel-layers-kind-object = Object
panel-layers-new-sublayer = New Sub-layer
panel-layers-new-layer = New/Duplicate Layer
panel-layers-new-bitmap = New Bitmap Image
panel-layers-add-mask = Add Mask
panel-layers-delete = Delete Selection

## Styles
panel-styles-source-document = Current Document
panel-styles-item = Style { $name }
panel-styles-empty = No styles in document
panel-styles-new = New Style
panel-styles-delete = Delete Style

## Swatches
panel-swatches-set-cubes = Color Cubes
panel-swatches-set-continuous = Continuous Tone
panel-swatches-set-windows = Windows System
panel-swatches-set-mac = Macintosh System
panel-swatches-set-grayscale = Grayscale
panel-swatches-set-custom = Custom
panel-swatches-menu = Swatches
panel-swatches-add = Add Swatches
panel-swatches-replace = Replace Swatches
panel-swatches-save = Save Swatches
panel-swatches-clear = Clear Swatches
panel-swatches-sort = Sort by Color
panel-swatches-add-fill = Add Current Fill Color
panel-swatches-grid = Swatches grid

## Color Mixer
panel-mixer-defaults = Default Colors
panel-mixer-swap = Swap Colors
panel-mixer-model-hex = Hexadecimal
panel-mixer-model-grayscale = Grayscale

## Color Palette
panel-palette-complementary = Complementary
panel-palette-triad = Triad
panel-palette-analogous = Analogous
panel-palette-square = Square
panel-palette-add = Add to Swatches

## Document Library
panel-doclib-kind-graphic = Graphic
panel-doclib-kind-button = Button
panel-doclib-kind-animation = Animation
panel-doclib-kind-component = Component
panel-doclib-kind-symbol = Symbol
panel-doclib-count = { $n ->
    [one] { $n } item
   *[other] { $n } items
}
panel-doclib-kind = Kind
panel-doclib-place = Insert at canvas center
panel-doclib-new-symbol = New Symbol (from selection)
panel-doclib-delete = Delete
panel-doclib-delete-title = Delete Symbol
panel-doclib-delete-text = Delete the symbol “{ $name }” and all its instances?

## Common Library
panel-commonlib-own = My Library
panel-commonlib-original = Standard library
panel-commonlib-empty = No library folders
panel-commonlib-insert = Insert into document

## Auto Shapes
panel-autoshapes-unavailable = The auto shape runtime is not available.
panel-autoshapes-insert = Insert at canvas center
panel-asp-none = No auto shape selected.
panel-asp-default-title = Auto Shape
panel-asp-no-params = This shape has no adjustable properties.
# Shape parameters (labels from fp-script)
panel-asp-p-rotation = Rotation
panel-asp-p-apex = Apex
panel-asp-p-apex-arc = Apex arc
panel-asp-p-arc = Arc
panel-asp-p-tip-length = Tip length
panel-asp-p-tip-width = Tip width
panel-asp-p-count = Count
panel-asp-p-clockwise = Clockwise
panel-asp-p-closed = Closed
panel-asp-p-corner-size = Corner size
panel-asp-p-corner-tl = Top left corner
panel-asp-p-corner-tr = Top right corner
panel-asp-p-corner-br = Bottom right corner
panel-asp-p-corner-bl = Bottom left corner
panel-asp-p-bend1 = Bend 1 roundness
panel-asp-p-bend2 = Bend 2 roundness
panel-asp-p-corner-type = Corner type
panel-asp-p-curvature = Curvature
panel-asp-p-curve = Curve
panel-asp-p-curve-radius = Curve radius
panel-asp-p-tooth-depth = Tooth depth
panel-asp-p-subdivisions = Subdivisions
panel-asp-p-offset-x = Offset X
panel-asp-p-offset-y = Offset Y
panel-asp-p-format = Format
panel-asp-p-focal = Focal length
panel-asp-p-frame-width = Frame width
panel-asp-p-gap = Gap
panel-asp-p-height = Height
panel-asp-p-start = Start
panel-asp-p-end = End
panel-asp-p-shaft-thickness = Shaft thickness
panel-asp-p-hole = Hole
panel-asp-p-hollow = Hollow (0–1)
panel-asp-p-hour = Hour
panel-asp-p-hue = Hue
panel-asp-p-inner-cutout = Inner cutout
panel-asp-p-inner-radius-tube = Inner radius (tube)
panel-asp-p-length = Length
panel-asp-p-link-corners = Link corners
panel-asp-p-minute = Minute
panel-asp-p-direction = Direction
panel-asp-p-month = Month (0–11)
panel-asp-p-points = Points
panel-asp-p-frame-offset = Frame offset
panel-asp-p-overhang = Overhang
panel-asp-p-size = Size
panel-asp-p-radius = Radius
panel-asp-p-radius1 = Radius 1
panel-asp-p-radius2 = Radius 2
panel-asp-p-inner-radius = Inner radius
panel-asp-p-rotation-x = Rotation X
panel-asp-p-rotation-y = Rotation Y
panel-asp-p-roundness = Roundness
panel-asp-p-roundness1 = Roundness 1
panel-asp-p-roundness2 = Roundness 2
panel-asp-p-width = Width
panel-asp-p-seed = Random seed
panel-asp-p-symbol = Symbol (-1 = selection)
panel-asp-p-sides = Sides
panel-asp-p-tip-size = Tip size
panel-asp-p-style = Style (0 one color, 1 two colors)
panel-asp-p-teeth = Teeth
panel-asp-p-text = Text
panel-asp-p-thickness = Thickness
panel-asp-p-turns = Turns
panel-asp-p-pointer-x = Pointer X
panel-asp-p-pointer-y = Pointer Y
panel-asp-p-unit = Unit
panel-asp-p-shaft-length = Shaft length
panel-asp-p-wavelength = Wavelength
panel-asp-p-year = Year
panel-asp-p-color = Color
# Choice values (the German value is stored, the translation is shown)
panel-asp-o-straight = straight
panel-asp-o-round = round
panel-asp-o-inverted = inverted
panel-asp-o-hvh = horizontal – vertical – horizontal
panel-asp-o-vhv = vertical – horizontal – vertical
panel-asp-o-none = None
panel-asp-o-head1 = Arrowhead 1
panel-asp-o-head2 = Arrowhead 2
panel-asp-o-bar = Crossbar
panel-asp-o-box = Box

## Optimize
panel-opt-no-document = No document open.
panel-opt-no-canvas = no canvas
panel-opt-settings-error = Export settings: { $error }
panel-opt-preset-gif-web216 = GIF Web 216
panel-opt-preset-gif-websnap256 = GIF WebSnap 256
panel-opt-preset-gif-websnap128 = GIF WebSnap 128
panel-opt-preset-gif-adaptive256 = GIF Adaptive 256
panel-opt-preset-jpeg-better = JPEG - Better Quality
panel-opt-preset-jpeg-smaller = JPEG - Smaller File
panel-opt-preset-anim-gif-websnap128 = Animated GIF WebSnap 128
panel-opt-preset-png8 = PNG8
panel-opt-preset-png24 = PNG24
panel-opt-preset-png32 = PNG32
panel-opt-preset-webp80 = WebP 80
panel-opt-preset-webp-lossless = WebP Lossless
panel-opt-preset-avif60 = AVIF 60
panel-opt-preset-apng = APNG
panel-opt-preset-anim-webp = Animated WebP
panel-opt-format-anim-gif = Animated GIF
panel-opt-format-webp-lossless = WebP Lossless
panel-opt-format-anim-webp = Animated WebP
panel-opt-palette-adaptive = Adaptive
panel-opt-palette-web-adaptive = Web Adaptive
panel-opt-palette-web216 = Web 216
panel-opt-palette-exact = Exact
panel-opt-palette-mac = Macintosh
panel-opt-palette-windows = Windows
panel-opt-palette-grayscale = Grayscale
panel-opt-palette-bw = Black & White
panel-opt-palette-uniform = Uniform
panel-opt-palette-custom = Custom
panel-opt-custom = Custom
panel-opt-transp-none = No Transparency
panel-opt-transp-index = Index Transparency
panel-opt-transp-alpha = Alpha Transparency
panel-opt-matte = Matte:
panel-opt-colors = Colors:
panel-opt-loss = Loss:
panel-opt-dither = Dither:
panel-opt-quality = Quality:
panel-opt-selective-quality = Selective quality:
panel-opt-smoothing = Smoothing:
panel-opt-speed = Speed:
panel-opt-interlaced = Interlaced
panel-opt-progressive = Progressive
panel-opt-sort-by = Sort by:
panel-opt-sort-none = None
panel-opt-sort-luminance = Luminance
panel-opt-sort-popularity = Popularity
panel-opt-color-count = { $n ->
    [one] { $n } color
   *[other] { $n } colors
}
panel-opt-add-transparent = Add color to transparency
panel-opt-remove-transparent = Remove color from transparency
panel-opt-select-transparent = Select transparent color (selected color becomes the only transparent one)
panel-opt-select-transparent-label = Select Transparency
panel-opt-edit-color = Edit color
panel-opt-snap-web = Snap to web safe
panel-opt-lock-color = Lock color
panel-opt-add-color = Add color
panel-opt-delete-color = Delete color

## Path
panel-path-combine = Combine Paths:
panel-path-join = Join
panel-path-split = Split
panel-path-union = Union
panel-path-intersect = Intersect
panel-path-punch = Punch
panel-path-crop = Crop
panel-path-alter = Alter Path:
panel-path-simplify = Simplify...
panel-path-expand-stroke = Expand Stroke...
panel-path-inset = Inset Path...
panel-path-hard-fill = Hard Fill
panel-path-anti-alias-fill = Anti-Alias Fill
panel-path-feather-fill = Feather Fill
panel-path-edit-points = Edit Points:
panel-path-snap-pixel = Snap to Pixel
panel-path-to-marquee = Convert Path to Marquee
panel-path-compound = Compound Shape:
panel-path-combine-compound = Combine

## Info
panel-info-width = W:
panel-info-height = H:

## Special Characters
panel-special-copied = “{ $ch }” copied to the clipboard (no text selected)

## Image Editing
panel-imgedit-tools = Bitmap Tools:
panel-imgedit-adjust = Adjustments:
panel-imgedit-auto-levels = Auto Levels
panel-imgedit-brightness = Brightness/Contrast...
panel-imgedit-curves = Curves...
panel-imgedit-levels = Levels...
panel-imgedit-hue-sat = Hue/Saturation...
panel-imgedit-invert = Invert
panel-imgedit-filters = Filters:
panel-imgedit-gaussian = Gaussian Blur...
panel-imgedit-unsharp = Unsharp Mask...
panel-imgedit-noise = Add Noise...
panel-imgedit-numeric = Numeric Transform...

## URL
panel-url-add = Add current URL to library
panel-url-delete = Delete URL from library

## Behaviors
panel-beh-simple-rollover = Simple Rollover
panel-beh-swap-image = Swap Image
panel-beh-swap-restore = Swap Image Restore
panel-beh-nav-image = Set Nav Bar Image
panel-beh-nav-over = Nav Bar Over
panel-beh-nav-down = Nav Bar Down
panel-beh-nav-restore = Nav Bar Restore
panel-beh-status-text = Set Text of Status Bar
panel-beh-popup = Show Pop-up Menu
panel-beh-popup-menu = Show Pop-up Menu…
panel-beh-sound = Play Sound
panel-beh-generic = Behavior
panel-beh-add = Add behavior
panel-beh-remove = Remove behavior
panel-beh-events = Events
panel-beh-actions = Actions
panel-beh-select-hint = Select a hotspot or slice

## Find
panel-find-scope-selection = Search Selection
panel-find-scope-page = Search Page
panel-find-kind-text = Find Text
panel-find-kind-font = Find Font
panel-find-kind-color = Find Color
panel-find-kind-url = Find URL
panel-find-kind-nonweb = Find Non-Web216
panel-find-find = Find:
panel-find-change = Change:
panel-find-nonweb-hint = Finds fill and stroke colors outside the Web 216 palette; “Replace” snaps them to the nearest web-safe color.
panel-find-whole-word = Whole Word
panel-find-match-case = Match Case
panel-find-next = Find
panel-find-replace = Replace
panel-find-replace-all = Replace All
panel-find-hits = { $n ->
    [one] { $n } match
   *[other] { $n } matches
}

## Symbol Properties
panel-symprops-none = No rich symbol selected
panel-symprops-name = Name
panel-symprops-value = Value
panel-symprops-note = Note: rich symbols only follow these values once fp-script draws them.

## CSS Properties
panel-css-select = Select an object
panel-css-copy-all = Copy All
panel-css-copied = CSS copied to the clipboard
