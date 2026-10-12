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
More languages are plain `.ftl` text files in `%APPDATA%\Firepixel\Sprachen\` (*View ▸ Language ▸
Open Language Folder…*); a guide for translators is in `crates\fp-i18n\README.md`.

**Basics:**

- *File ▸ New* (Ctrl+N) creates a document (size, resolution, canvas color).
- *File ▸ Open* (Ctrl+O) reads FW-PNG and common image formats: PNG, GIF, JPEG, BMP, TIFF, TGA,
  WBMP, PSD, SVG, WebP, APNG, AVIF, HEIC, JPEG XL, QOI, ICO.
- *File ▸ Save* (Ctrl+S) writes FW-PNG; all layers, pages, states, symbols and Live Filters stay
  editable.
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

## Menus

- **File:** New, Open, Open Recent, Save, Import, Export, Image Preview, HTML Setup, Batch Process,
  Print.
- **Edit:** Undo, Cut/Copy/Paste. Copied objects are also available to other programs as an
  image, and an image from another program is pasted as a bitmap. Also Duplicate, Clone, Insert,
  Preferences and Keyboard Shortcuts.
- **View:** Magnification, Fit Selection/Fit All, Rulers, Grid, Guides, Smart Guides, Theme, Language.
- **Select, Modify, Text, Filters:** act on the selection. If a command is grayed out, running it
  via its shortcut shows the reason in the status bar.
- **Commands:** Run Script (`.jsf`), saved commands from the commands folder
  (`%APPDATA%\Firepixel\Commands`, subfolders become submenus) and the AI control channel.
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

## Exporting

In the **Optimize** panel you set the export format:

- GIF, animated GIF, JPEG, PNG 8/24/32,
- WebP (also lossless and animated), AVIF, APNG,
- TIFF, BMP, WBMP, QOI, ICO.

The tabs **Preview**, **2-Up** and **4-Up** above the document show the result before exporting,
each with file size and download time. 4-Up shows three settings side by side for comparison.

*File ▸ Export* writes images, slices including HTML/CSS, and all pages.

## Automation (Scripts, Command Line, MCP)

- **Scripts:** *Commands ▸ Run Script* runs JSF scripts (object `fw`). Each script is one undo
  step.
- **Command line:** `firepixel-cli` can inspect documents (`info`), apply commands (`run`,
  `batch`), render images (`render`), convert (`convert`), run scripts (`script`) and check
  translations (`sprache pruefen`). `firepixel-cli <command> --help` shows examples.
- **AI control (MCP):** `firepixel-cli mcp` (stdio), or in the running application
  *Commands ▸ AI Control Channel (MCP)*, or start with `--mcp`. The server listens on 127.0.0.1
  only and requires a token. Address and token are in `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`.
  Details are in `docs\AGENTS.md`.

## Troubleshooting

- **Preferences** are stored in `%APPDATA%\Firepixel\preferences.json`. Deleting the file resets
  them.
- If the workspace is in a mess: *Window ▸ Workspace Layouts ▸ Default Workspace*.
- Warnings suppressed with "Don't show again" come back via *Commands ▸ Reset Warning Dialogs*.
- Resources of the original are optional. Textures, patterns, libraries and the CMYK table of an
  existing installation are used by Firepixel if the folder is entered in *Preferences*. None of
  them is shipped.
