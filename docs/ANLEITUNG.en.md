# Firepixel – Quick Guide

Firepixel is a graphics and web design application for vector and bitmap editing with pages,
states (animation frames), layers, symbols and Live Filters. It reads and writes FW-PNG files
(`.fw.png`) and works like the well-known original: same menus, tools, panels, keyboard shortcuts
and mouse handling.

## Getting Started

**Installation:** none. Put `firepixel.exe` into any folder and start it. The file needs no
additional libraries. `firepixel-cli.exe` (command line and AI control) sits next to it but is not
needed for the user interface.

**Starting:**

- Double-click `firepixel.exe`, or drag files onto the exe or the open window.
- From the command prompt: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [file …]`.

**Language:** *View ▸ Language* or *Edit ▸ Preferences ▸ General*. Firepixel ships with German
and English; on first start it uses the Windows language. The switch takes effect immediately.
More languages are plain `.ftl` text files in `%APPDATA%\Firepixel\Languages\` (*View ▸ Language ▸
Open Language Folder…*); a guide for translators is in `crates\fp-i18n\README.md`. This guide is also
available in German (`ANLEITUNG.md`).

**Basics:**

- *File ▸ New* (Ctrl+N) creates a document (size, resolution, canvas color).
- *File ▸ Open* (Ctrl+O) reads FW-PNG and common image formats: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO.
- *File ▸ Save* (Ctrl+S) writes FW-PNG; all layers, pages, states, symbols and Live Filters stay
  editable.
- The file dialogs (Open, Import, Save As, New from Template, Save as Template) start in the folder
  used last or in the document's folder; templates are stored in `%APPDATA%\Firepixel\Templates`.
  While a file dialog is open, Firepixel keeps redrawing.
- *Edit ▸ Undo/Redo* (Ctrl+Z / Ctrl+Y); the History panel shows all steps.
- At the top of the window is the application bar with logo, menus, quick buttons and the
  workspace switcher, below it the main toolbar, the tools on the left, the panels on the right
  and the Properties inspector at the bottom.

**Themes:** *View ▸ Theme* or *Edit ▸ Preferences ▸ User Interface*:

- **Normal** (light gray),
- **Dark**,
- **OLED** (pure black, colored tool groups, glowing selection).

## Tools

The Tools panel has the sections Select, Bitmap, Vector, Web, Colors and View. A small triangle
on a button means there are more tools: hold the mouse button down or click the triangle. Each
slot remembers the tool used last.

Frequent shortcuts (set "Firepixel", switchable in *Edit ▸ Keyboard Shortcuts*):

| Key | Tool | Key | Tool |
|---|---|---|---|
| V / 0 | Pointer | A / 1 | Subselection |
| M | Marquee | L | Lasso |
| W | Magic Wand | B | Brush |
| P | Pen | U | Rectangle / Ellipse / Polygon |
| T | Text | N | Line |
| K | Slice | J | Hotspot |
| I | Eyedropper | G | Paint Bucket / Gradient |
| Q | Scale | C | Crop |
| H / Space | Hand | Z | Zoom |

Shift while dragging constrains proportions and angles, Alt draws from the center. Double-click a
bitmap to enter bitmap mode; double-click text to edit it. A right-click opens the context menu
that fits the tool and the selection.

**Bitmap tools:** the **Brush** paints pixels: with no bitmap selected it creates a new one,
otherwise it paints into the selected one. **Eraser**, **Blur** and **Rubber Stamp** work with a
soft edge as in the original; Esc ends painting and switches to the Pointer. The **Gradient** tool
has its own colors (default black → white, a click on the swatch opens the gradient editor);
"Fill selection" and tolerance are separate from the Paint Bucket. **Replace Color** has its own
"From" and "To" colors (default green → red).

## Menus

- **File:** New, Open, Open Recent, Save, Import, Export, Image Preview, HTML Setup, Batch Process,
  Print.
- **Edit:** Undo, Cut/Copy/Paste. Copied objects are also available to other programs as an
  image, and an image from another program is pasted as a bitmap. Also Duplicate, Clone, Insert,
  Preferences and Keyboard Shortcuts.
- **View:** Magnification, Fit Selection/Fit All, Rulers, Grid, Guides, Smart Guides, Theme, Language.
- **Select, Modify, Text, Filters:** act on the selection. If a command is grayed out, running it
  via its shortcut shows the reason in the status bar. Commands that can always be chosen but find
  no suitable target (e.g. *Simplify...* without a path) report this with a warning, like the
  original; "Don't show again" switches it off.
- **Commands:** Run Script (`.jsf`), saved commands from the commands folder
  (`%APPDATA%\Firepixel\Commands`, subfolders become submenus), Manage AI Models, Reset Warning
  Dialogs and the AI control channel.
- **Window:** shows and hides panels. *Workspace Layouts* saves and loads arrangements; at the end
  is the list of open documents.

## Panels

The panels are docked on the right. Clicking the active tab collapses the group; dragging a tab
detaches it or moves it into another group. The ≡ menu of each panel holds its commands. Tab hides
and shows all panels.

Workspaces (switcher at the top right):

- Expanded Mode,
- Iconic Mode,
- Iconic Mode with Panel Names,
- your own layouts via *Save Current...*.

Your own layouts are stored as XML in `%APPDATA%\Firepixel\Workspace Layouts` and can be exchanged
with the layout format of the original.

Important panels:

- **Layers:** visibility (eye), lock, sub-layers; drag to reorder.
- **Pages / States:** pages and animation frames, master page, onion skinning.
- **Optimize:** export format and colors.
- **History:** step back, replay steps or save them as a command.

The **Properties inspector** at the bottom shows, depending on the selection, fill, stroke,
opacity, blending mode, Live Filters (+/−) and text or symbol settings.

- **Font search:** the opened font list (text inspector, Text tool, text editor) has a search field
  at the top. Typing filters (part of a word, case-insensitive), ↑/↓ select, Enter applies, Esc
  closes without a change.
- **Textures and patterns:** the lists show all textures or patterns from the resource folders;
  "Other..." below the list picks an image file of your own.

## Trace Bitmap

*Modify ▸ Trace Bitmap...* (also in the context menu of a bitmap) turns a pixel image into editable,
filled vector paths. In the dialog:

- **Preset:** Logo (few flat colors, crisp edges), Black and White (line art, scans, lettering),
  Illustration (many colors), Photo (soft shapes).
- **Settings:** colors or threshold, detail and smoothness, corners, remove speckles.
  **Clean up** (on for Logo) unifies colors, straightens lines, detects circles and rectangles and
  uses fewer nodes; the details are under *Advanced*.
- **Preview:** Compare, Original, Vector or Outlines, plus figures (paths, points, colors, match).

OK takes over the result of the preview as a group on a new layer "Traced", in one undo step; with
"Keep original" (default) the bitmap stays.

The window opens large (≈ 1050 × 830, at most 90 % of the main window) and remembers the size you
drag it to.

## AI Selection

Three tools in the slot of the Magic Wand (Bitmap section) create pixel selections with local AI
models, without a network connection while computing:

- **Object Selection:** click an object or drag a box; as with the Magic Wand, Shift+click adds and
  Alt+click subtracts.
- **Select Subject** (also *Select ▸ Select Subject*): selects the main subject with a soft edge.
- **Refine Edge** (also *Select ▸ Refine Edge...*): improves the edge of an existing selection for
  hair, fur and semi-transparency.

The models are not shipped. If one is missing, Firepixel offers the download; nothing is loaded
before you agree. Overview, licenses and model folder: *Commands ▸ Manage AI Models...* (details in
`docs\MODELLE.md`).

While a model is computing, a card at the bottom above the canvas shows the tool, the model, the
stages, a bar and the time; *Cancel* or Esc stops it. The tool's inspector offers the **fine
tuning** the models really support:

- *Tolerance* (more or less selection), *Edge* (Hard, Anti-Alias, Feather), *Expand* (> 0 larger,
  < 0 smaller), *Smooth*,
- Object Selection only: *Variant* (Automatic, Small, Medium, Large).

A change adjusts the last selection at once, without running the model again.

## Exporting

In the **Optimize** panel you set the export format:

- GIF, animated GIF, JPEG, PNG 8/24/32,
- WebP (also lossless and animated), AVIF, APNG,
- TIFF, BMP, WBMP, QOI, ICO.

The tabs **Preview**, **2-Up** and **4-Up** above the document show the result before exporting,
each with file size and download time. 4-Up shows three settings side by side for comparison.

*File ▸ Export* writes images, slices including HTML/CSS, and all pages.

**Image Preview** (*File ▸ Image Preview...*): with the buttons at the bottom (1, 2 or 4 preview
windows) you compare formats and qualities side by side. Each pane has its own setting; the pane you
click shows it on the left. Zoom and scrolling apply to all panes. The **Options** tab has:

- format, palette, colors, loss, dither, quality/smoothing (JPEG), matte;
- the **color table**: a click shows the color values, buttons for locked, web-safe and
  transparent, ≡ or a right-click opens the palette menu;
- three **eyedroppers** for transparency;
- **Optimize to Size** (button at the bottom right), which searches the setting for a target file
  size.

Choose saved settings from the list at the top; "+" saves the current one.

## Automation (Scripts, Command Line, MCP)

- **Scripts:** *Commands ▸ Run Script* runs JSF scripts (object `fw`). Each script is one undo
  step.
- **Command line:** `firepixel-cli` can inspect documents (`info`), apply commands (`run`,
  `batch`), render images (`render`), convert (`convert`), run scripts (`script`) and check
  translations (`language check`). `firepixel-cli <command> --help` shows examples; `--lang en|de`
  chooses the language of the output.
- **AI control (MCP):** `firepixel-cli mcp` (stdio), or in the running application
  *Commands ▸ AI Control Channel (MCP)*, or start with `--mcp`. The server listens on 127.0.0.1
  only and requires a token. Address and token are in `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`.
  AI agents get answers in English, in German on request. Details are in `docs\AGENTS.en.md`
  (German: `docs\AGENTS.md`).

## Troubleshooting

- **Preferences** are stored in `%APPDATA%\Firepixel\preferences.json`. Deleting the file resets
  them.
- **Folder names:** since 12.10.2026 the app's folders have English names: `Languages` (formerly
  `Sprachen`), `Models` (formerly `Modelle`, under `%LOCALAPPDATA%\Firepixel`), `Common Library`
  (formerly `Allgemeine Bibliothek`). On the first start of a new version Firepixel renames the old
  folders once; if that fails, it keeps reading them.
- If the workspace is in a mess: *Window ▸ Workspace Layouts ▸ Default Workspace*.
- Warnings suppressed with "Don't show again" come back via *Commands ▸ Reset Warning Dialogs*
  (or *Preferences ▸ Reset All Warning Dialogs*).
- "AI model … is missing": load it in *Commands ▸ Manage AI Models...* or put the file into the model
  folder shown there.
- Resources of the original are optional. Textures, patterns, libraries and the CMYK table of an
  existing installation are used by Firepixel if the folder is entered in *Preferences*. None of
  them is shipped.
