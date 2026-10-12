# Image Preview (File ▸ Image Preview…, crates\fp-ui\src\dialogs\image_preview\, re\17-bildvorschau.md) – owner: BV-<n>
# Prefix image-preview-; the labels of the left column stay in dialoge.ftl (dlg-preview-*).

# Pane header
image-preview-preview = Preview
image-preview-preview-tip = Preview the current settings
image-preview-saved-settings = Saved settings:
image-preview-saved-settings-tip = Saved settings
image-preview-save-current-tip = Save current settings
image-preview-quality = Quality { $quality }
image-preview-quality-smoothing = Quality { $quality } ({ $smoothing })
image-preview-colors = { $n ->
    [one] { $n } color
   *[other] { $n } colors
}
image-preview-millions = Millions
image-preview-millions-alpha = Millions+Alpha
# Size and download time: numbers come preformatted (point, K = 1000 bytes)
image-preview-size-bytes = { $n } bytes
image-preview-size-line = { $size }  { $seconds } sec @56kbps
image-preview-busy = calculating …
image-preview-pane = Preview pane { $n }

# Toolbar
image-preview-tool-pointer = Pointer
image-preview-tool-export-area = Export Area
image-preview-tool-zoom = Zoom
image-preview-zoom-tip = Set magnification
image-preview-view-1 = 1 Preview Window
image-preview-view-2 = 2 Preview Windows
image-preview-view-4 = 4 Preview Windows
image-preview-first-frame = First frame
image-preview-play = Play/Stop
image-preview-last-frame = Last frame
image-preview-current-frame = Current frame
image-preview-prev-frame = Previous frame
image-preview-next-frame = Next frame

# "Preset Name" dialog ("+" in the pane)
image-preview-preset-name-title = Preset Name
image-preview-preset-name-label = Name:

# "Options" tab (re\17 §2, package P2)
image-preview-opt-format = Format:
image-preview-opt-palette = Palette:
image-preview-opt-quality = Quality:
image-preview-opt-loss = Loss:
image-preview-opt-dither = Dither:
image-preview-opt-smoothing = Smoothing:
image-preview-opt-smoothing-none = No smoothing
image-preview-opt-smoothing-max = 8 Maximum smoothing
image-preview-opt-palette-websnap = WebSnap Adaptive
image-preview-opt-matte = Matte:
image-preview-opt-progressive = Progressive browser display
image-preview-opt-sharpen = Sharpen color edges
image-preview-opt-remove-unused = Remove unused colors
image-preview-opt-interlaced = Interlaced browser display
image-preview-tip-format = Export file format
image-preview-tip-options = Options for this tab
image-preview-tip-palette = Indexed palette
image-preview-tip-quality = JPEG quality
image-preview-tip-loss = Lossy GIF compression
image-preview-tip-colors = Maximum number of colors
image-preview-tip-actual-colors = Actual number of colors
image-preview-tip-dither = Dither image
image-preview-tip-dither-amount = Dither amount
image-preview-tip-smoothing = Smoothing level
image-preview-tip-progressive = Progressive
image-preview-tip-sharpen = Do not compress sharp edges
image-preview-tip-edit-color = Edit color
image-preview-tip-transparent = Transparent
image-preview-tip-snap-web = Snap to web safe
image-preview-tip-lock = Lock color
image-preview-tip-add = Add color
image-preview-tip-delete = Delete color
image-preview-tip-pipette-set = Select transparency color
image-preview-tip-pipette-add = Add color to transparency
image-preview-tip-pipette-remove = Remove color from transparency
image-preview-tip-transparency = Select transparency type
image-preview-tip-matte = Transparency matte color
image-preview-tip-remove-unused = Optimize image for smaller files
image-preview-tip-interlaced = Interlaced image
image-preview-tip-optimize-size = Optimize to size wizard

# Color table, status line, palette menu (§2.5, §2.7)
image-preview-color-table = Color table
image-preview-color-status = R:{ $r } G:{ $g } B:{ $b } Hex:{ $hex } Index:{ $index }
image-preview-color-status-alpha = R:{ $r } G:{ $g } B:{ $b } A:{ $a } Hex:{ $hex } Index:{ $index }
image-preview-flag-locked = Locked
image-preview-flag-edited = Edited
image-preview-flag-websafe = Web Safe
image-preview-flag-transparent = Transparent
image-preview-menu-add-color = Add Color…
image-preview-menu-edit-color = Edit Color…
image-preview-menu-delete-color = Delete Color
image-preview-menu-replace-color = Replace Color…
image-preview-menu-snap-web = Snap To Web Safe
image-preview-menu-transparent = Transparent
image-preview-menu-lock = Lock Color
image-preview-menu-unlock-all = Unlock All Colors
image-preview-menu-sort-luminance = Sort by Luminance
image-preview-menu-sort-popularity = Sort by Popularity
image-preview-menu-unsorted = Unsorted
image-preview-menu-swatch-feedback = Show Swatch Feedback
image-preview-menu-remove-edit = Remove Edit
image-preview-menu-remove-all-edits = Remove All Edits
image-preview-menu-load-palette = Load Palette…
image-preview-menu-save-palette = Save Palette…
image-preview-palette-error = Palette: { $error }
image-preview-palette-empty = The file contains no colors.

# "Color" dialog (double-click, edit/add color)
image-preview-color-title = Color
image-preview-color-add-title = Add Color
image-preview-color-hex = Hex:

# "Optimize to Size" dialog (§2.8)
image-preview-size-title = Optimize to Size
image-preview-size-target = Target size:
image-preview-size-unit = k
image-preview-size-too-small = The resulting file would be too small. The original values were restored.

# "File" tab (§3)
image-preview-file-scale = Scale
image-preview-file-percent = %:
image-preview-file-width = W:
image-preview-file-height = H:
image-preview-file-x = X:
image-preview-file-y = Y:
image-preview-file-pixels = Pixels
image-preview-file-constrain = Constrain proportions
image-preview-file-export-area = Export area
image-preview-tip-scale-percent = Scale the image on export
image-preview-tip-scale-width = Scale to this width
image-preview-tip-scale-height = Scale to this height
image-preview-tip-constrain = Keep the image proportions
image-preview-tip-export-area = Export an area of the image
image-preview-tip-area-x = Start point X coordinate
image-preview-tip-area-y = Start point Y coordinate
image-preview-tip-area-width = Width of the export area
image-preview-tip-area-height = Height of the export area

# "Animation" tab (§4)
image-preview-disposal-unspecified = Unspecified
image-preview-disposal-none = None
image-preview-disposal-background = Restore to Background
image-preview-disposal-previous = Revert to Previous
image-preview-disposal-letter-unspecified = U
image-preview-disposal-letter-none = N
image-preview-disposal-letter-background = B
image-preview-disposal-letter-previous = P
image-preview-anim-delay-first = { $delay } /100 sec
image-preview-anim-runtime = Duration: { $seconds } sec
image-preview-anim-export-frame = Export state { $n }
image-preview-anim-play-once = Play once
image-preview-anim-loop = Loop
image-preview-anim-forever = Forever
image-preview-anim-crop-each = Crop each state
image-preview-anim-save-differences = Save differences between states
image-preview-tip-disposal = Disposal method
image-preview-tip-frame-delay = State delay
image-preview-tip-runtime = Total animation delay time
image-preview-tip-repeat = Number of repeats
image-preview-tip-crop-each = Crop to changed pixels
image-preview-tip-save-differences = Save changed pixels only
