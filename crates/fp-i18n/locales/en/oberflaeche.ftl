# User interface shell: application bar, workspaces, document tabs, view bar, start page, footer,
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
ui-appbar-more-menus = More menus

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

## Preview (2-Up/4-Up)
ui-preview-original = Original
ui-preview-caption = { $label }  { $size }  { $secs } sec @ 56 kbps
ui-preview-colors = { $n } colors
ui-preview-failed = Preview not possible: { $error }

## Start page (no document open)
ui-start-recent = Open a Recent Item
ui-start-open = Open...
ui-start-create-new = Create New
ui-start-new-document = Firepixel Document (FW-PNG)

## Footer: selection info, magnification, state controls
ui-sel-bitmap-mode = Bitmap mode
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
ui-filter-all-readable = All readable formats
ui-filter-all-files = All Files (*.*)
ui-filter-html = HTML files
ui-filter-scripts = Scripts (*.jsf, *.js)
ui-filter-programs = Programs (*.exe)
ui-file-open-title = Open
ui-file-import-title = Import
ui-file-update-html-title = Update HTML
ui-file-run-script-title = Run Script
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
ui-revert-confirm = Revert to the last saved version of "{ $name }"?
ui-not-implemented = “{ $label }” is not implemented yet.
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
    Firepixel – graphics and web design application

    Usage: firepixel [options] [file …]

    Options:
      --design=normal|dark|oled     theme for this session only
      --mcp[=PORT]                  start the AI control channel (MCP over HTTP, 127.0.0.1 only, with token)
      --help, -h                    show this help

    Files are opened at startup.
