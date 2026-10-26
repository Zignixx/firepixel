# User interface shell: application bar, workspaces, document tabs, view bar, footer,
# docking, file dialogs, status messages of the app layer. Owner: I18N-SHELL.

## Application bar
ui-appbar-browse = Browse (Open)
ui-appbar-hand-tool = Hand tool (H)
ui-appbar-zoom-tool = Zoom tool (Z)
ui-zoom = Magnification
ui-window-minimize = Minimize
ui-window-maximize = Maximize
ui-window-restore = Restore Down
ui-window-close = Close

## Workspaces
ui-workspace-switcher = Choose a workspace ({ $name })
ui-workspace-save-current = Save Current...
ui-workspace-default = Default Workspace
ui-workspace-expanded = Expanded Mode
ui-workspace-iconic = Iconic Mode
ui-workspace-iconic-names = Iconic Mode with Panel Names
ui-workspace-save-failed = The workspace could not be saved: { $error }

## Themes and tool groups
ui-theme-normal = Normal
ui-theme-dark = Dark
ui-theme-oled = OLED
ui-toolgroup-select = Select
ui-toolgroup-bitmap = Bitmap
ui-toolgroup-vector = Vector
ui-toolgroup-web = Web
ui-toolgroup-colors = Colors
ui-toolgroup-view = View

## Document tabs
ui-tab-merge-all = Consolidate All to Here
ui-tab-close = Close
ui-tab-close-all = Close All
ui-tab-move-to-new-window = Move to New Window
ui-tab-new-document = New Document
ui-tab-open-document = Open Document
ui-tab-more = More Documents

## View bar
ui-view-original = Original
    .tooltip = Editable original
ui-view-preview = Preview
    .tooltip = Image preview
ui-view-2up = 2-Up
    .tooltip = 2-Up preview
ui-view-4up = 4-Up
    .tooltip = 4-Up preview
ui-page-default-name = Page { $n }
ui-page-nav = Change page
ui-symbol-edit-back = Back
    .tooltip = Finish editing the symbol and return to the document
# View ▸ Tooltips: tag with the document coordinates at the pointer over the canvas
ui-canvas-pointer-tag = x:{ $x } y:{ $y }
ui-canvas-size-tag = w:{ $w } h:{ $h }

## Preview (2-Up/4-Up)
ui-preview-original = Original
ui-preview-caption = { $label }  { $size }  { $secs } sec @ 56 kbps
ui-preview-colors = { $n } colors
ui-preview-failed = Preview not possible: { $error }

## Footer: selection info, magnification, state controls
ui-sel-bitmap-mode = Bitmap mode
# FB-31 (Firepixel extension): pausing a running bitmap stroke with P
ui-stroke-pause-hint = Press P on the keyboard to pause the current brush stroke.
# FB-69 (Firepixel extension): Alt held during a running or paused stroke
ui-stroke-subtract = Subtraction mode (Alt): the brush erases only what this stroke has painted. Release Alt to paint again.
ui-stroke-subtract-paused = Stroke paused, subtraction mode (Alt): drag to erase only what this stroke has painted. Release Alt to paint again, hit Enter to apply the stroke.
ui-stroke-paused = Stroke paused. It is safe to let go of mouse button or lift stylus. Click anywhere on the screen to resume current stroke. Hit Enter to apply paused stroke.
ui-sel-document = { $format } (Document)
ui-sel-objects = { $n } objects
ui-sel-object = Object
ui-sel-bitmap = Bitmap
ui-sel-rectangle = Rectangle
ui-sel-path = Path
ui-sel-group = Group
ui-sel-compound = Compound Shape
ui-sel-text = Text
ui-sel-symbol = Symbol
ui-sel-slice = Slice
ui-zoom-fit-selection = Fit Selection
ui-zoom-fit-all = Fit All
ui-state-first = First State
ui-state-play = Play/Stop
ui-state-stop = Stop Playback
ui-state-last = Last State
ui-state-prev = Previous State
ui-state-next = Next State
ui-exit-bitmap-mode = Exit Bitmap Mode
ui-export-area-set = Export area set.

## Docking
ui-dock-panel-menu = Panel menu
ui-dock-expand = Expand Panels
ui-dock-collapse-icons = Collapse to Icons

## File dialogs (titles and filters; format names stay)
ui-filter-all-readable = All readable files
ui-filter-fw-png = FW-PNG (*.png)
ui-filter-flattened-png = Flattened PNG (*.png)
ui-filter-all-files = All Files (*.*)
ui-filter-html = HTML files (*.htm)
ui-filter-scripts = Scripts (*.jsf, *.js)
ui-filter-programs = Programs (*.exe)
ui-file-open-title = Open
ui-file-import-title = Import
ui-file-update-html-title = Locate HTML File:
ui-file-new-from-template-title = New from Template
ui-file-run-script-title = Run Script
ui-import-place-hint = Click to place the file, or drag a rectangle to scale it. Esc cancels.
ui-panel-collapse = Collapse panel
ui-panel-expand = Expand panel
ui-file-primary-browser-title = Set Primary Browser
ui-file-secondary-browser-title = Set Secondary Browser
ui-file-save-as-title = Save As
ui-file-save-template-title = Save as Template

## App layer messages (open, save, import, clipboard, AI control channel)
ui-file-saved-copy = Saved as copy: { $path }
ui-file-save-failed =
    The file could not be saved:
    { $error }
ui-file-open-failed =
    “{ $path }” could not be opened:
    { $error }
ui-file-import-failed =
    Import not possible:
    { $error }
ui-revert-confirm = Revert to the last saved version of “{ $name }”?
ui-not-implemented = “{ $label }” is not implemented yet.
ui-script-failed = Could not run the script.
ui-script-save-as = Save Script
ui-script-needs-document = This command is only possible when a document is active.
ui-script-error = An error occurred.
ui-preview-created = Preview created: { $path }
ui-color-copied = Color { $hex } copied
ui-warnings-reset = Warning dialogs reset
ui-clip-no-document = no document open
ui-mcp-off = AI control channel off
ui-mcp-starting = Starting the AI control channel (address and token in %LOCALAPPDATA%\Firepixel\mcp-bridge.json)
ui-mcp-stopped = AI control channel stopped
ui-mcp-active = AI control channel active: { $url } (token in { $file })
ui-mcp-failed = The AI control channel could not start: { $error }

## History steps created by the user interface itself
ui-history-paste = Paste
ui-history-paste-objects = Paste Objects
ui-history-import = Import
ui-history-import-files = Import Files

## Command line (firepixel.exe --help)
ui-cli-help =
    Firepixel Studio – graphics and web design application

    Usage: firepixel [options] [file …]

    Options:
      --design=normal|dark|oled     theme for this session only
      --mcp[=PORT]                  start the AI control channel (MCP over HTTP, 127.0.0.1 only, with token)
      --batch [RECIPE] [FILE …]     open the Batch Studio: RECIPE is a .fpbatch.json or the name of
                                    a template (e.g. web-webp, logos-to-svg, video-to-gif);
                                    the following files and folders are added to the list
      --help, -h                    show this help

    Files are opened at startup.
    Example: firepixel --batch web-webp C:\Photos
    Without a window: firepixel-cli batch --recipe <recipe> … (firepixel-cli batch --help)

## Context menu of input fields (Q-26)
fieldmenu-undo = Undo
fieldmenu-cut = Cut
fieldmenu-copy = Copy
fieldmenu-paste = Paste
fieldmenu-delete = Delete
fieldmenu-select-all = Select All

## macOS application menu (app_menu.rs)
ui-macmenu-about = About Firepixel Studio
ui-macmenu-services = Services
ui-macmenu-hide = Hide Firepixel Studio
ui-macmenu-hide-others = Hide Others
ui-macmenu-show-all = Show All
ui-macmenu-quit = Quit Firepixel Studio
ui-split-close-pane = Close this pane (Window ▸ Close Duplicate Window)
ui-split-pane-page = Page of this pane; a click into the pane makes it the one tools and panels act on
