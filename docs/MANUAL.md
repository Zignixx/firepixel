# Firepixel – Quick Guide

Firepixel is a graphics and web design application for vector and bitmap editing with pages,
states (animation frames), layers, symbols and Live Filters. It reads and writes FW-PNG files
(`.fw.png`) and works like the well-known original: same menus, tools, panels, keyboard shortcuts
and mouse handling.

## Getting Started

**Installation:** none. Put `firepixel.exe` into any folder and start it. The file needs no
additional libraries. `firepixel-cli.exe` (command line and AI control) sits next to it but is not
needed for the user interface.

**Linux (x86_64, glibc 2.35 or later, e.g. Ubuntu 22.04 or Debian 12):** either make the AppImage
executable with `chmod +x Firepixel-x86_64.AppImage` and run it, or unpack the tar.gz and run
`./firepixel` in the folder `Firepixel-Linux-x64` (it also holds `firepixel-cli`, the manuals, a
`.desktop` file and icons). Firepixel runs on Wayland and X11 and needs a Vulkan or OpenGL driver plus
the libraries `libxkbcommon0`, `libxkbcommon-x11-0` (X11) and `libwayland-client0` (Wayland), which most
desktops already have. Open and save dialogs go through the XDG desktop portal, so
`xdg-desktop-portal` and a portal backend for your desktop (GTK, KDE, …) must be installed. The Linux
build is new and has not yet been tried on many desktops.

**Starting:**

- Double-click `firepixel.exe`, or drag files onto the exe or the open window.
- From the command prompt: `firepixel [--design=normal|dark|oled] [--mcp[=PORT]] [file …]`.

**Language:** *View ▸ Language* or *Edit ▸ Preferences ▸ General*. Firepixel ships with German
and English; on first start it uses the Windows language. The switch takes effect immediately.
More languages are plain `.ftl` text files in `%APPDATA%\Firepixel\Languages\` (*View ▸ Language ▸
Open Language Folder…*); a guide for translators is in `crates\fp-i18n\README.md`. This guide is also
available in German (`MANUAL.de.md`).

**Basics:**

- *File ▸ New* (Ctrl+N) creates a document (size, resolution, canvas color). New documents are named
  "Untitled-1.fw.png", "Untitled-2.fw.png" and so on. If you copied objects or pixels with Ctrl+C
  before, *New* suggests their size.
- *File ▸ Open* (Ctrl+O) reads FW-PNG and common image formats: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO. In PSD files, shape layers become paths
  and groups become sub-layers. Text from AI/EPS and SVG files comes in as editable text objects
  (AI area text in the same place as in the original); the file type in the dialog is "AI (*.ai;*.art)".
- *File ▸ Save* (Ctrl+S) writes FW-PNG; all layers, pages, states, symbols and Live Filters stay
  editable.
- The file dialogs (Open, Import, Save As, New from Template, Save as Template) start in the folder
  used last or in the document's folder; templates are stored in `%APPDATA%\Firepixel\Templates`.
  While a file dialog is open, Firepixel keeps redrawing.
- *File ▸ New from Template...*: the first time you use it, Firepixel creates ten templates of its own
  in five folders (`Document Presets`, `Grids`, `Mobile`, `Web`, `Wireframes`), e.g. Web Banner
  728 × 90 or Smartphone 390 × 844. Each template brings a fitting export setting (PNG 32 or JPEG).
  Templates you delete do not come back.
- *Edit ▸ Undo/Redo* (Ctrl+Z / Ctrl+Y); the History panel shows all steps. Copy and Cut get a row of
  their own ("Copy", "Cut"); Copy does not mark the document as changed.
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

Frequent shortcuts (set "Firepixel", switchable in *Edit ▸ Keyboard Shortcuts...*; further sets are called
"Image Editing", "Vector Drawing" and "Classic Vector Keys"):

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
that fits the tool and the selection. Esc with no action in progress deselects all objects (the first
Esc closes open popups and menus).

**Bitmap tools:** the **Brush** paints pixels: with no bitmap selected it creates a new one,
otherwise it paints into the selected one. **Eraser**, **Blur** and **Rubber Stamp** work with a
soft edge as in the original; Esc ends painting and switches to the Pointer. The **Gradient** tool
has its own colors (default black → white, a click on the swatch opens the gradient editor);
"Fill selection" and tolerance are separate from the Paint Bucket. **Replace Color** has its own
"From" and "To" colors (default green → red).
While you paint with Brush, Pencil, Eraser or Rubber Stamp, the preview appears at the bitmap's place
in the stacking order, with its opacity, blend mode, effects and masks. A stroke that creates a new
bitmap, or paints into a bitmap inside a symbol, is previewed on top until you release the mouse.

**Pen:** while you draw a path, a click on its last anchor retracts that point's outgoing handle, so the next
segment leaves it straight; dragging on it pulls the point out again as a symmetric curve point (Alt moves only the
outgoing handle, Ctrl moves the point, Shift snaps to 45°). A click on an interior curve point turns it into a
corner, a click on an interior corner point deletes it, a click on a segment inserts a point, and a click on the
end of another open path joins it; with no path in progress, a click on the end of an open path continues it. The
cursor shows the mode: a "^" for converting a point, a ring with a stub for joining. Points are hit within the
*Snap distance* set in the preferences. A double-click ends the path.

**Select Behind** (in the Pointer slot): resting the pointer over objects frames in red the object a click would
select – the next object below the selected one under the pointer, also while the pointer is over the selected
object, or the topmost object when nothing there is selected. The Pointer tool frames only the topmost object and
shows no frame over the selected one.

**Default stroke:** Line, Pen, Vector Path, Rectangle, Ellipse and Polygon draw with the stroke
*Basic ▸ Hard Rounded* (1 px), as in the original; a choice of your own in the inspector still applies.

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
- *Modify ▸ Transform ▸ Numeric Transform...:* with "Scale attributes" the stroke width grows with
  the scaling (rotating never changes it); the checkbox is remembered.
- *Modify ▸ Mask ▸ Reveal All / Hide All* puts an empty mask over the selected object. The Layers
  panel and the inspector still show the object itself (e.g. the bitmap) with the mask as a second
  thumbnail.
- *Select ▸ Select Inverse* inverts a pixel selection only; without one it shows a warning and leaves
  the selected objects alone.
- *Text ▸ Size ▸ Larger / Smaller* change the size by exactly 1 pt (24 → 25 → 24), on selected texts
  and in the open text editor; the smallest size is 1 pt. *Stretched Alignment* affects only
  fixed-width text blocks; point text (auto width) keeps its alignment.
- **Cropping:** the crop tool, *Edit ▸ Crop Document* and *Modify ▸ Canvas ▸ Canvas Size* follow the
  option *Edit ▸ Preferences ▸ Edit ▸ Delete objects when cropping* (on by default). On: objects
  entirely outside the new canvas are deleted on all layers and states, bitmaps crossing the edge are
  clipped, vector objects crossing it stay whole. Off: nothing is deleted or clipped, the objects only
  move with the canvas.
- *Select ▸ Select Similar* adds objects with the same fill, stroke, opacity and blending mode.
- *Text ▸ Detach from Path* turns the text back into a text block with its previous width, centered
  on the path. The path is on top afterwards. Text on a path shows only the first paragraph, as in
  the original.
- **Filters on vector objects:** Firepixel first asks "This operation converts vectors to bitmaps."
  (OK, Cancel, "Don't show again").
- *View ▸ Gamma 1.8* shows the canvas brighter, as on a monitor with Macintosh gamma. The document and
  the export stay unchanged.
- *Edit ▸ Paste* puts copied objects directly above the topmost selected object (in its layer; if an object
  inside a group is selected, above the group). Without a selection they go to the top of the active layer.
- *Modify ▸ Mask ▸ Group as Mask* puts the mask group in the place of the lowest selected object; *Group*
  stays in the place of the topmost one. Union, Intersect and Join put the result in the place of the upper
  object, Punch and Crop in the place of the lower one (*Combine Paths*).
- *Select ▸ Convert Marquee to Path* gives the new path fill and stroke like a newly drawn shape (colors
  from the Tools panel, fill and stroke category from the Property Inspector).
- *Select ▸ Next Selection / Previous Selection* (Ctrl+Shift+Right / Left) step through the document's
  selection history: every new object selection is an entry, nothing happens at the end.
- *Modify ▸ Symbol ▸ Edit Symbol* opens the symbol in the document's window, as large as the page and titled
  "<document> (Symbol)". A path bar below the view bar shows "Page 1 ▸ <symbol>"; "← Back" returns to the
  document. After *Convert to Symbol* the instance has no name of its own.
- *Modify ▸ Symbol ▸ Tween Instances...* suggests 10 steps the first time, afterwards the value last
  confirmed (until the program ends).
- *Modify ▸ Animation ▸ Animate Selection...* asks when the animation goes beyond the last state:
  "Automatically add new states?" (OK appends states, Cancel creates the animation without new states,
  "Don't show again" can be switched back on with *Reset Warning Dialogs*).
- **Commands:** Run Script (`.jsf`), saved commands from the commands folder
  (`%APPDATA%\Firepixel\Commands`, subfolders become submenus), Manage AI Models, Reset Warning
  Dialogs and the AI control channel. Built-in commands: **Screenshot** (after a countdown, the whole
  screen or an area entered as numbers, taken with the system's own capture tool, opens as a new
  document), **Create Symbol Script** (a JSF script that recreates the selected graphic symbol;
  bitmaps, nested instances and compound shapes are skipped, gradients become solid fills) and
  **Web ▸ Shared Palette** (one palette over all open documents, shown in the Swatches panel and
  optionally saved as `.act`).
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

- **Layers:** visibility (eye), lock, sub-layers; drag to reorder. Sub-layers are indented folder
  rows with an expander; double-click a row to rename it.
- **Pages / States:** pages and animation frames, master page, onion skinning. When an animation
  instance is selected, the canvas shows its motion path (green start, red end), and the selection
  frame encloses only the current frame.
- **Optimize:** export format and colors.
- **History:** step back, replay steps or save them as a command.
- **URL:** URLs for slices, hotspots and buttons. The ≡ menu has *Add Used URLs to Library* (from the
  slices, hotspots and buttons of all open documents), *Clear Unused URLs*, *Add URL...*, *Edit URL...*
  and *Delete URL* (both need a selected URL), *New URL Library...*, *Import URLs...* and *Export
  URLs...* (`.txt` with one URL per line or `.htm` with one link per URL) and Help. The default library
  is `URLs.txt` in `%APPDATA%\Firepixel`; further libraries are HTML files in its `URL Libraries`
  folder and can be chosen in the panel's library list. Changes to a library cannot be undone with
  Ctrl+Z, and these menu commands are not available to scripts, the command line or MCP.

The **Properties inspector** at the bottom shows, depending on the selection, fill, stroke,
opacity, blending mode, Live Filters (+/−) and text or symbol settings.

- **Live Filters (+):** the “+” menu groups the effects as in the original, e.g. *Bevel and Emboss* ▸
  *Inner Bevel*, *Outer Bevel*, *Inset Emboss*, *Raised Emboss*. The kind is part of the menu item;
  the dialog has no separate field for it. Double-click an entry in the filter list to open its settings.

- **Text tool defaults:** attributes set while the text editor is open change only the marked text;
  set with text selected by the pointer, or with nothing selected, they become the defaults for new texts.
- **Font search:** the opened font list (text inspector, Text tool, text editor) has a search field
  at the top. Typing filters (part of a word, case-insensitive), ↑/↓ select, Enter applies, Esc
  closes without a change.
- **Textures and patterns:** the lists show all textures or patterns from the resource folders;
  "Other..." below the list picks an image file of your own.
- **Color picker:** a click on a color box opens the popup with swatches, HEX field and eyedropper.
  While you move over the window, the eyedropper picks up every visible color, also over panels,
  menus and open dialogs. A click applies the color. Outside the Firepixel window the working color
  stays.

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
  Alt+click subtracts. With *Edit ▸ Preferences ▸ Edit ▸ Show selection preview while hovering* (off
  by default), resting the pointer shows the object's outline dashed; this only works once the model is
  loaded for the image, and a click on the same spot then selects at once.
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

A change adjusts the last selection at once, without running the model again. After
*Select ▸ Select Subject* the inspector also shows this fine tuning with the Pointer, Subselection,
Marquee, Lasso and Magic Wand, as long as the selection is unchanged. Even a very soft selection
(e.g. on a semi-transparent brush stroke) shows its marching ants.

## Exporting

In the **Optimize** panel you set the export format:

- GIF, animated GIF, JPEG, PNG 8/24/32,
- WebP (also lossless and animated), AVIF, APNG,
- TIFF, BMP, WBMP, QOI, ICO.

The tabs **Preview**, **2-Up** and **4-Up** above the document show the result before exporting,
each with file size and download time. 4-Up shows three settings side by side for comparison.

*File ▸ Export* writes images, slices including HTML/CSS, and all pages. In *File ▸ HTML Setup...*
(tab *General*), "Write CSS to external file" (on by default, works together with "Use CSS for
popups") writes the styles to a separate `.css` file next to the HTML file, included via `@import`.
Next to "Background image:", "Browse..." picks an image. If it is in the document's folder, the path
is entered relative to it. "Write page styles to an external file (styles.css)" moves the page styles
into `styles.css` next to the HTML; the style *Generic HTML5* writes an HTML5 page; buttons keep their
navigation-bar script in every style. *Set Defaults* stores the dialog's values for new documents and
for documents without HTML settings of their own.
In the PDF export options, the compression "JPEG 2000 (JPEG)" writes plain JPEG (DCT), because
Firepixel has no JPEG 2000 encoder; the PDF's producer entry notes this.

**PDF and PSD export options:**

- PDF, "Enable text selection": every visible text object is written as invisible text over the page
  image, so text can be selected, searched and copied in a PDF viewer. Only characters of the Western
  (WinAnsi) character set are included; others are left out of the text layer but stay visible in the
  image.
- PSD, "Convert to PSD layers": every object becomes its own PSD layer, grouped by document layer. With
  "Maintain editability" for Text, text objects become editable type layers; for Effects, drop shadow,
  inner shadow, glows, bevel/emboss and colour fill become layer effects; for Vector, paths with a plain
  fill become fill layers with a vector mask. The other choices render text, effects and vectors into
  the pixels.

**Web features:**

- *Edit ▸ Copy HTML Code...* is a wizard with three pages: HTML style, base file name for the slices,
  folder for the images. It has the buttons < Back, Next > and Finish. Afterwards the code is on the
  clipboard.
- *Modify ▸ Pop-up Menu ▸ Add Pop-up Menu...* opens the Pop-up Menu Editor. Next > and < Back lead
  through the four tabs, Done creates the menu. On the "Content" tab the entries are in a table
  (Text │ Link │ Target), the selected row can be edited in place; the buttons + / − / → / ← add, delete,
  indent and outdent entries.
- The slice guides divide the page as in the original: first horizontally above the topmost slice, then
  vertically along the slice edges, without cutting through a slice.
- Button instances count as slices on the canvas. They get the green slice overlay and the slice
  guides.

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
  step. `dom.layers` lists all layers flat, as in the original, each sub-layer right after its layer.
  `dom.topLayers` holds the top level only.
- **Batch Processing** (*File ▸ Batch Process...*): drag files in or add them with "Files..." or
  "Folder...", choose the steps and "Save as", look at the before/after preview and run. The
  target folder then holds the report `batch-report.html` with preview images. The "Plan" tab of the
  dry run estimates the size of each output file ("≈ 192 KB") and the total. If you cancel a run,
  the end card shows "Canceled: n done, m not processed". The size total then counts finished
  files only. All batch steps and their parameters are described in `docs\BATCH.md`; the card
  menu's "Help" opens the matching section.
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
  them. The row "ffmpeg for videos" (*Preferences ▸ Plug-ins*) is the same setting as in the Batch
  Studio; empty means search automatically.
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
