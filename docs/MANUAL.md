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

**macOS:** the window keeps the native title bar with the traffic-light buttons (close, minimise,
zoom); Firepixel's menu bar sits directly below it. Cmd+Q quits like File ▸ Exit and asks to save
changed documents first. The application menu in the macOS menu bar offers About Firepixel,
Services, Hide Firepixel, Hide Others, Show All and Quit Firepixel (Cmd+Q, also with the save prompt).

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
  PDF files (and AI files from version 9) bring their layers (optional content groups) as layers:
  hidden groups arrive as hidden layers, nested groups are named "Parent / Child", and content outside
  any group goes to "Layer 1".
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
  their own ("Copy", "Cut"); Copy does not mark the document as changed. Consecutive arrow-key nudges
  of the same selection form one Move row with the summed offset; selecting something else, another
  command or an undo starts a new row. Redo (Ctrl+Y, Ctrl+Shift+Z) first redoes an undone step (*Redo: ‹step›*).
  With nothing undone it repeats the last step as a new row (*Repeat: ‹step›*): after Clone another clone,
  after a move the same move again, after a pen point the same point again; a repeat can be repeated. Right
  after a redo, after a bitmap stroke (brush, pencil, eraser, retouch tools) and without steps the entry
  reads *Can't Redo* and does nothing.
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

**Tooltips:** a tool's tooltip shows its name and shortcut, e.g. "Pointer (V, 0)", and a second line
with a short description of what the tool does (the rows of a flyout show the description as their
tooltip). Toolbar buttons show only the command name. In the Property Inspector of the retouch tools
(Blur, Sharpen, Dodge, Burn, Smudge, Rubber Stamp) and the Eraser, the fields describe themselves:
"Brush size", "Brush preview", "Edge softness", "Round brush tip", "Pressure" and so on. All these tooltips
are switched with *View ▸ Tooltips*.

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

**Selecting with the Pointer and the Subselection:** dragging on empty canvas draws a marquee; every
object it touches is selected, and the marquee writes no History row. With the **Pointer**, Shift (also
Shift+Alt) **toggles** each object inside the marquee: unselected objects are added, selected ones are
deselected — so a Shift-marquee over selected objects removes them from the selection. Ctrl gives a
normal marquee. An Alt-marquee started on empty canvas selects like the Subselection: the children of a
group one by one, replacing the selection; Alt-dragging an object still drags a copy. With the
**Subselection**, Shift adds the objects or points inside the marquee (nothing is deselected) and Alt
gives a normal marquee. The Pointer shows the hollow Subselection arrow while Alt is held and the arrow
with "+" during any Alt drag (copy or Alt-marquee), like the Subselection's Alt-marquee; Shift shows no
mark. Firepixel extension: the Properties inspector of these two tools (nothing selected) has the option
**Marquee: Rectangle / Lasso** (preference `Firepixel_PointerLasso`, default Rectangle). With Lasso the
drag draws a free-form outline like the Lasso tool's; on release the Pointer selects the objects whose
centre lies inside it and the Subselection the anchor points inside it, with the same modifier rules.

**Bitmap tools:** the **Brush** paints pixels: with no bitmap selected it creates a new one,
otherwise it paints into the selected one. **Eraser**, **Blur** and **Rubber Stamp** work with a
soft edge as in the original; Esc ends painting and switches to the Pointer. After a Marquee, Lasso
or Magic Wand selection on a bitmap, Esc exits bitmap mode (History row "Exit Bitmap Mode"), removes the
pixel selection and switches to the Pointer; the bitmap stays selected. Esc while you are still
dragging a new marquee or lasso outline cancels the drag and also exits bitmap mode (an older pixel
selection is removed too) and switches to the Pointer; releasing the mouse then does nothing. Esc in
the middle of a Brush stroke switches to the Pointer at once and freezes the preview, but the whole
stroke is still painted when you release the mouse; the next Esc exits bitmap mode. With the
Marquee or Lasso and a pixel selection the arrow keys move only the marquee (1 px, Shift 10 px);
consecutive presses are one History row "Move". While a pixel selection exists, the Marquee, Lasso and
Magic Wand cursors show the combine mode of the next selection: Shift "+" (add), Alt "−" (subtract),
Shift+Alt "×" (intersect, in either order of pressing); without a pixel selection the cursor has no mark.
Inside the pixel selection the cursor becomes an arrow with a dashed rectangle — dragging moves the
marquee (also with the Lasso and the Magic Wand) — and with Ctrl an arrow with scissors (dragging moves
the pixels). The cursor follows a changed modifier with the next mouse move. The **Gradient** tool
has its own colors (default black → white, a click on the swatch opens the gradient editor);
"Fill selection" and tolerance are separate from the Paint Bucket. **Replace Color** has its own
"From" and "To" colors (default green → red).
While you paint with Brush, Pencil, Eraser or Rubber Stamp, the preview appears at the bitmap's place
in the stacking order, with its opacity, blend mode, effects and masks. A stroke that creates a new
bitmap, or paints into a bitmap inside a symbol, is previewed on top until you release the mouse.
**Blur**, **Sharpen**, **Dodge**, **Burn**, **Smudge**, **Rubber Stamp** and **Replace Color** also show
their effect live while you drag; releasing the mouse changes nothing more, and each stroke is one
history step named after the tool. Over an editable bitmap these tools hide the mouse pointer and show
only the outline of the brush tip (it disappears when the pointer leaves the canvas); where no bitmap
can be edited the pointer shows "not allowed". **Smudge** is never cut off at the bitmap edge, no
matter whether the brush centre was on the bitmap or beside it when you pressed (with the bitmap
selected, a stroke pressed beside it works like one pressed on it; with nothing selected a press beside
the bitmap does nothing): color carried past the edge fades out semi-transparent and the bitmap grows to
cover it, while dragging only transparency in grows nothing. With "Entire document" the bitmap grows to
the whole rectangle of the stroke (tip included), filled with the page, even for a stroke entirely
outside the bitmap. A stroke entirely outside without "Entire document" changes no pixel but is still
one history step. The Firepixel option **Confine to boundaries** in the Smudge inspector keeps the
smear inside the bitmap edge. **Blur** across the edge grows the bitmap by
up to 2 px.

**Pausing a stroke (Firepixel extension):** while you paint a stroke with Brush, Pencil, Eraser or one
of the retouch tools (Blur, Sharpen, Dodge, Burn, Smudge, Rubber Stamp, Replace Color), the status line
reads "Press P on the keyboard to pause the current brush stroke." Press **P** with the mouse button
still down (or the stylus on the tablet): the stroke is paused, and you can release the button or lift
the stylus, move freely (only the brush outline follows the pointer, nothing is painted) and zoom or
scroll. The next press on the canvas continues the **same** stroke from that point (the parts are not
joined by a line); **P** pauses again. Because it stays one stroke, a semi-transparent brush does not
get darker where the parts overlap, unlike two separate strokes (the eraser removes the overlap once,
Dodge/Burn/Blur/Sharpen work it once; Smudge and Rubber Stamp start afresh at each press). **Enter**
applies the paused stroke as one History row named after the tool. **Esc**, a tool change, a menu
command, Undo or Redo also apply the paused stroke first, so nothing is lost; Undo then takes the whole
stroke back. Without a running stroke, P keeps its toolbox shortcut. Scripts and MCP send the key
`pause` to `tool_event`; the commands `bitmap.paint`, `bitmap.erase` and `bitmap.retouch` take the
resumed parts as `breaks` (indices into `points`).

**Pen:** while you draw a path, a click on its last anchor retracts that point's outgoing handle, so the next
segment leaves it straight; dragging on it pulls the point out again as a symmetric curve point (Alt moves only the
outgoing handle, Ctrl moves the point, Shift snaps to 45°). A click on an interior curve point turns it into a
corner, a click on an interior corner point deletes it, a click on a segment inserts a point, and a click on the
end of another open path joins it; with no path in progress, a click on the end of an open path continues it. The
cursor shows the mode: an "x" with no path in progress (a click starts a new path), the plain pen while drawing, an
"o" over the first point (close), a "^" for converting a point, a "−" for deleting and a "+" for inserting one (the
rubber band is hidden then), a "/" over the end of an unselected open path (continue) and a ring with a stub for
joining. While you draw, the path shows its outline and a square on each anchor (the last placed one filled), no
bounding box and no fill until it is closed; after closing the first anchor is filled. Choosing the Pen with an
open path selected continues that path from its end point; a selected closed path shows its outline with hollow
anchors. Points are hit within the
*Snap distance* set in the preferences. A double-click or a change of tool ends the path; Enter
and Escape do nothing while a path is drawn (the next click adds to the same path). Every point is its own
History step "Pen": Ctrl+Z (or Backspace) while drawing removes only the last point and you keep drawing from the
point before; ending the path adds no step.

**Select Behind** (in the Pointer slot): resting the pointer over objects frames in red the object a click would
select – the next object below the selected one under the pointer, also while the pointer is over the selected
object, or the topmost object when nothing there is selected. The Pointer tool frames only the topmost object and
shows no frame over the selected one.

**Pointer:** a single selected path also shows its outline with filled squares on its anchors (a rectangle: its
four corners) instead of a bounding box; groups, bitmaps, text, symbols and several selected objects keep the box.

**Subselection:** a selected path, rectangle or autoshape part shows its outline, a small square on each point
(filled when the point is selected; *Show solid points* in the preferences reverses this) and the handles of the
selected points instead of a bounding box; bitmaps, text and symbols keep the box. Dragging a point or handle
with it is free (not rounded to whole pixels) and snaps only to the grid, guides and smart guides when they are on.

**Default stroke:** Line, Pen, Vector Path, Rectangle, Ellipse and Polygon draw with the stroke
*Basic ▸ Hard Rounded* (1 px), as in the original; a choice of your own in the inspector still applies.

## Menus

- **File:** New, Open, Open Recent, Save, Import, Export, Image Preview, HTML Setup, Batch Process,
  Print.
- **Edit:** Undo, Cut/Copy/Paste. Copied objects are also available to other programs as an
  image, and an image from another program is pasted as a bitmap. Also Duplicate, Clone, Insert,
  Preferences and Keyboard Shortcuts.
- **View:** Magnification, Fit Selection/Fit All, Rulers, Grid, Guides, Smart Guides, Theme, Language.
  *View ▸ Snap to Pixels* (Firepixel's own switch, off by default) rounds anchors and handles dragged with the
  Subselection tool to whole pixels; off, they move freely in steps of 1/zoom like in the original. Pointer moves
  and Pen points always land on whole pixels. Dragged anchors do not snap to smart guides. *Modify ▸ Snap to
  Pixel* (Ctrl+K) still snaps the selection once.
- **Select, Modify, Text, Filters:** act on the selection. If a command is grayed out, running it
  via its shortcut shows the reason in the status bar. Commands that can always be chosen but find
  no suitable target (e.g. *Simplify...* without a path) report this with a warning, like the
  original; "Don't show again" switches it off.
- *Modify ▸ Transform ▸ Numeric Transform...:* with "Scale attributes" the stroke width grows with
  the scaling (rotating never changes it) and the fill handles (gradient and pattern) are transformed
  with the object; without it the fill handles stay where they are in the document. The checkbox is
  remembered (preference `NumericXformXformsBFE`, also after Cancel). The Scale, Skew and Distort
  tools and Free Transform follow *Preferences ▸ General ▸ Scale strokes and effects*
  (`XformToolXformsBFE`, on by default) for the fill handles in the same way. A pure move always
  takes the fill handles along. In the Scale, Skew and Distort tools and Free Transform, Enter, a
  double-click or another tool applies all drags of the session as one History row "Transform";
  Esc cancels the session: the object returns to its state before the session, the Pointer is
  active and the row stays as a redo step (*Edit ▸ Redo: Transform* applies it again). Esc while
  the mouse is still held keeps the preview until you release, then ends the mode without
  applying anything. In scripts, `scaleSelection`, `rotateSelection`, `setQuadrangle`
  and `transformSelection` transform the fill handles (and, when scaling, the stroke width) only
  when the options string contains `transformAttributes`, as in the original
  (`"autoTrimImages transformAttributes"`); with `"autoTrimImages"` alone they stay in the document. For a text the
  numeric transform always scales the fill handles about the centre, and the transform tools always leave them,
  as in the original.
- **Scaling text changes its font size.** As in the original, a pure scale (no rotation, skew or
  reflection) of a text is turned into its attributes instead of being stored as a matrix: font size ×
  vertical factor (to 0.01 pt, at least 4 pt), horizontal scale × horizontal/vertical factor, exact
  leading, range kerning, indent and paragraph spacing along; baseline shift stays. The text is set
  again with the new size (sharp, pixel for pixel like a fresh text of that size); a small leftover
  matrix fits it onto the scaled frame, and none at all when the difference is below a fraction of a
  pixel (1.5 × of a 60 pt text gives a plain 90 pt text). This happens for *Modify ▸ Transform ▸
  Numeric Transform* scaling (every selected text, also inside groups) and for the Scale tool, Free
  Transform and *Modify ▸ Transform ▸ Scale* when exactly one text is selected. Rotating, reflecting,
  skewing, distorting, the Scale tool on a group or several objects, and a non-uniform scale of a
  rotated text keep the matrix. A fixed-width text keeps its frame width and wraps again; the
  leftover matrix then visibly stretches or squashes it, as in the original. *Remove Transformations*
  keeps the new size. In scripts, `scaleSelection` folds like the dialog; `setQuadrangle` takes its
  four points relative to a unit square (the square of side max(width, height) about the centre of the
  selection bounds, so a 100 × 60 rectangle is x 0…1, y 0.2…0.8) and folds a single text only with the
  option `rememberQuad`.
- **The transform frame of a text.** With exactly one text selected, the Scale, Skew and Distort
  tools, Free Transform and *Modify ▸ Transform ▸ Scale* show a frame 1.3 times the text's set box
  (15 % of its width added left and right, 15 % of its height above and below), turned with the text;
  for a point text the set box is its layout frame (a 60 pt "Ag" 74 × 72 at 100,100 gets the handles
  at 89/137/185 × 89/136/183), for a fixed-width text the box around the set lines. Paths, groups (also
  with a text inside) and several objects keep their bounds. The corners are shown on whole pixels, the
  frame line and handles one screen pixel left of and above the frame, as in the original. A corner
  drag scales by the pointer's position projected onto the frame's diagonal (+39/+38 at the corner of
  that "Ag" gives 83.74 pt); the next drag starts from the frame of the new state. *Remove
  Transformations* puts a text's left/top on the rounded left/top of the outline of its frame (derived from
  measurements, not every case is known).
- *Modify ▸ Mask ▸ Reveal All / Hide All* puts an empty mask over the selected object. The Layers
  panel and the inspector still show the object itself (e.g. the bitmap) with the mask as a second
  thumbnail. The History row is *Add Mask*.
- **Working with masks:** a *Subselection* click on a masked object selects its mask, and so does a
  *Pointer* click with Alt held (on a normal group, Pointer + Alt-click selects the clicked child;
  Alt-drag still duplicates); *Select Behind* never picks a mask. A click on the mask thumbnail in the
  Layers panel works like the context menu's *Edit Mask*: the mask is selected, its thumbnail gets a
  bright green frame, the row stays highlighted and shows a mask icon in its second column; a bitmap
  mask goes into bitmap editing. A click on the row's name selects the object again and frames its own
  thumbnail blue (a selected object's thumbnail is always framed blue). The anchors of a single-path mask move with the Subselection while the content stays.
  *Disable Mask* turns the mask off; the menu entry then reads *Enable Mask* and the mask thumbnail
  shows a red X. *Delete Mask* asks first: *Apply* (the default) bakes the mask into the object, which
  becomes one bitmap; *Discard* removes the mask and keeps the object; *Cancel* does nothing.
  *Ungroup* gives the mask object on top of the content, both selected.
  Firepixel extensions beyond the original: with a **group mask** selected, a further Subselection
  click (or a Pointer double-click) selects the child under the pointer, so its anchors can be edited
  one by one and the mask follows; the mask inspector has an **Invert** checkbox (vector and bitmap
  masks) that hides what lies inside the mask and keeps everything outside (the mask object's own
  opacity, or a grey value in grayscale mode, sets how strongly). Inverted masks are stored in a
  Firepixel-own tag of the FW-PNG file; other programs show the plain mask.
- **Mask modes.** *Group as Mask* always makes a grayscale mask with *Show fill and stroke* off; the masked
  element stays selected (History row *Group*). *Paste as Mask* gives a vector mask in *Path outline* mode
  and a bitmap mask in *Grayscale* mode and selects the mask; *Paste Inside* uses *Path outline* with
  *Show fill and stroke* on, so the mask's fill (or bitmap) shows under the content until you turn the
  checkbox off (both History rows *Paste*). In grayscale the mask value is the luma (Rec. 709) of the mask's
  rendered look times its alpha: white shows, black hides, red gives 21 %, green 72 %, blue 7 %; gradients and
  feathered edges ramp. With a mask selected (mask thumbnail in the Layers panel or *Edit Mask*) the inspector
  follows the kind of the mask object, not the mode: a path, text or group (also a group of bitmaps) shows
  **Vector mask** with *Path outline* / *Grayscale*, *Show fill and stroke* (usable in *Path outline* only) and
  all fill and stroke controls in both modes; a bitmap shows **Bitmap mask** with *Alpha channel* /
  *Grayscale*. Switching back is always possible; selecting the mask, the mode and the checkbox are each one
  History row *Modify Mask*. The colour wells show grey (the luma) while a mask is selected.
- **Painting on a bitmap mask** (brush, pencil, eraser) works live in both modes: white reveals, black hides,
  the masked element updates during the stroke, the brush outline is red, and each stroke is one *Brush*
  row. The painting tools show the mask's *Alpha channel* / *Grayscale* radios, which switch the mode.
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
- **Screen modes:** F (or the three screen-mode buttons in the View section of the Tools panel) cycles
  *Standard* → *Full Screen with Menus* → *Full Screen* → *Standard*; it needs an open document. Full screen
  with menus keeps the menu bar and the toolbar, floats the Tools panel at the left and shows the docked
  panels as an icon column at the right. Full screen shows only the canvas, centred on black, without
  bars, panels, tabs, rulers or scroll bars. Back with F or *Exit Full Screen Mode* in the canvas context
  menu; when the last document closes, Firepixel returns to Standard.
- *View ▸ Gamma 1.8* shows the canvas brighter, as on a monitor with Macintosh gamma. The document and
  the export stay unchanged.
- *View ▸ Tooltips* (on by default, remembered as the preference `Tooltips`) switches the tooltips of
  buttons, fields and panels everywhere (Tools panel, toolbar, Property Inspector, panels, dialogs). Off,
  no tooltip opens; the change applies at once.
- *View ▸ Coordinate Tag* (Firepixel extension, right below Tooltips; on by default, remembered as the
  preference `Firepixel_CoordinateTag`) switches the small grey tag just below the pointer over the canvas:
  with the Rectangle, Ellipse, Line, Polygon and Auto Shape tools the pointer position ("x:53 y:37") and,
  while a shape is drawn, its size ("w:80 h:50"); while objects are dragged with the Pointer their new
  top-left corner ("x:130 y:120"). The Pointer shows nothing on plain hover, Pen, Text and the other tools
  show no tag; the tag disappears on release. The two switches are independent of each other.
- **Swap Colors** (X or the button below the color boxes of the Tools panel): with nothing selected only
  the stroke and fill colors swap. With a selection the colors of the topmost selected object swap and
  are applied to every selected object as one History row "Stroke Color" ("Brush and Fill" when one side
  is none: then the none swaps too, and a new stroke gets the 1-Pixel Soft tip, 1 px). An existing
  stroke keeps its tip, width and placement; a text loses its fill and gets a 1 px stroke in its color; a
  bitmap keeps its pixels. Ctrl+Y repeats the swap, Ctrl+Z restores the objects and the colors: after
  Undo, Redo or a History jump the colors follow the selected object, and only with nothing selected (or
  only bitmaps) do the colors stored with the swap come back.
- *Edit ▸ Paste* puts copied objects directly above the topmost selected object (in its layer; if an object
  inside a group is selected, above the group). Without a selection they go to the top of the active layer.
- *Modify ▸ Mask ▸ Group as Mask* puts the mask group in the place of the lowest selected object; *Group*
  stays in the place of the topmost one. Union, Intersect and Join put the result in the place of the upper
  object, Punch and Crop in the place of the lower one (*Combine Paths*).
- *Select ▸ Convert Marquee to Path* gives the new path fill and stroke like a newly drawn shape (colors
  from the Tools panel, fill and stroke category from the Property Inspector).
- **New shapes take the last selected object's attributes:** selecting exactly one path or text (click,
  marquee, script) makes its fill, fill and stroke color, stroke tip, size and placement the current tool
  attributes; a path with the fill "none" gives the tool fill "none" with the path's own stored fill color,
  a text gives its color as the fill, no stroke and placement outside. Groups, auto shapes, several
  selected objects, bitmaps and web objects leave the tool attributes alone; while several objects or an
  auto shape are selected, the color boxes of the Tools panel only show the topmost object's colors. The
  attributes stay after deselecting, so the next rectangle, ellipse or line looks like the object selected
  last. Scripts see the change after they end. A stroke or fill "none" shows as a white box with a red
  diagonal.
- The fill "none" belongs to the document: a new or opened document starts with a solid fill in the
  current fill color, while the stroke (also "none"), its tip, size and placement carry over. After the
  program start the tool stroke is the 1-Pixel Soft tip in the size of the preference `BrushTipSize`
  (1 px when none is stored), centred.
- **Default Colors** (D or the button below the color boxes) sets only the colors (black stroke, white
  fill); tip, size and placement stay. A stroke "none" becomes the 1-Pixel Soft tip, 1 px, a fill "none"
  a solid fill.
- **Auto shapes** take the tool attributes in full: fill (also "none"), stroke color, tip, size and
  placement. Connector Line and Spiral never have a fill and take the tool stroke; only without a tool
  stroke do they draw their own black 1 px line. Arrow Line always draws black Hard Line 1 px without fill,
  Measure Tool a red Hard Line 1 px inside with a black length label, and the Measure Tool's shape is not
  selected afterwards. Only these two change the tool attributes (Arrow Line: black Hard Line 1 px,
  Measure Tool: red, both with fill "none"; the placement stays). Every shape is one History row "Shape";
  Undo removes the shape but leaves the tool attributes as they are.
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
and shows all panels. A panel whose content is taller than its space (e.g. the Path panel at its docked
height) shows a vertical scroll bar and scrolls with the mouse wheel.

Workspaces (switcher at the top right):

- Expanded Mode,
- Iconic Mode,
- Iconic Mode with Panel Names,
- your own layouts via *Save Current...*.

Your own layouts are stored as XML in `%APPDATA%\Firepixel\Workspace Layouts` and can be exchanged
with the layout format of the original.

Important panels:

- **Layers:** visibility (eye), lock, sub-layers; drag to reorder. Sub-layers are indented folder
  rows with an expander; double-click a row to rename it. The triangle of a layer or sub-layer
  expands or collapses it; this state belongs to the document (one History step, saved in the file),
  so every document and every reopened file shows its own layers as they were stored. A click on an object row selects only that
  object and sets the anchor; Shift+click selects all objects from the anchor to the clicked row in
  the document's order, top to bottom across layers and sub-layers (objects of hidden and collapsed
  layers count, objects of locked layers are left out), replacing the selection; Ctrl+click
  (Cmd+click on macOS) adds or removes one object and keeps the anchor; Ctrl+Shift+click acts like
  Shift+click. A click on a row that is part of a multi-selection keeps the selection. A click on a
  layer row selects the layer's own objects (not those of its sub-layers) and makes it the current
  layer; Shift+click on a layer row ends the range at the layer's own objects, Ctrl+click toggles each
  of them. The anchor counts while its object is still selected, even after a selection on the
  canvas; otherwise the bottom-most selected object is the anchor, and with nothing selected a
  Shift+click selects from the row to the bottom-most object. Clicks on rows of a locked layer (or of
  a sub-layer of one) change nothing. Selecting in the panel writes no History steps, except the plain
  click on a layer row, which writes one *Layer Change* step (Undo restores the current layer and the
  selection). Whenever the selection ends up as exactly one object (panel, canvas, marquee, Select All,
  script), that object's layer or sub-layer becomes the current layer; with two or more objects the
  current layer stays. The current layer is always the first unlocked layer from the chosen one
  upward (sub-layers share their parent's lock, the web layer included), wrapping around to the bottom
  layer: locking the chosen layer moves the current layer up, unlocking it brings it back. Each page
  remembers its chosen layer (the first visit after opening starts on the bottom-most unlocked layer, a
  new page on its first layer); after a state change in which the current layer is locked, the next
  unlocked layer above becomes the chosen one. A file opens on page 1 and state 1 with the bottom-most unlocked layer current
  and nothing selected; the panel scrolls to that layer's row. With nothing selected the current
  layer's row is highlighted with a bold name; with a selection the selected object rows are
  highlighted and bold, while the current layer and every layer holding a selected object are tinted
  light. The circle at the right of each layer row is filled where the layer holds a selected object
  of its own and grey on a layer without objects of its own (sub-layers do not count, the web layer
  alike). A click on an empty circle moves all selected objects as one block to the top of that layer
  or sub-layer (a sub-selected object leaves its group), keeps them selected and makes the layer
  current; Alt+click puts copies there instead and selects them. It writes one *Layer Change* step and
  does nothing with nothing selected, on a filled circle, on a locked layer, or between the web layer
  and normal objects. The web layer's lock is not kept when a file is opened. The New Layer button adds the layer above the current one on its level (beside a
  current sub-layer). A right-click on an object row selects that
  object first, so the context menu's *Flatten Selection*, *Add Mask* and *Merge Down* apply to it
  (*Merge Down* only when the object lies directly above a bitmap). A right-click on a layer row makes
  it the current layer and keeps the selection; on the web layer the share-to-pages entries are grey.
  *Add Mask* adds an empty, linked bitmap mask that shows everything and selects the mask.
  **Group contents (Firepixel extension):** a group row shows a small "›" right after its name; a click
  on it lists the group's objects indented below the row, with a thin line at their left (each with
  eye, lock, thumbnail and name; nested groups have their own "›", and the marker turns into "⌄"
  while open). A click on such a row selects that object exactly like a click with the Subselection
  tool (no History step, the inspector shows the object); Shift+click adds or removes it.
  Double-click renames it, the eye hides only that object. While an object inside a group is
  selected (from the panel or with the Subselection on the canvas), the group's row is tinted pale
  yellow, also while the group is collapsed; an open group shows the object's own row highlighted.
  A masked object additionally shows a circled arrow after its name: it lists the objects that make
  up the mask (the mask path, the objects of a group mask, or one row *Bitmap mask*). A click on
  such a row selects the mask for editing like a click on the mask thumbnail; on an object of a
  group mask it then selects that object (Shift+click adds or removes it). The open/closed state
  belongs to each object, lasts while Firepixel runs and is neither saved nor part of History. Rows
  inside a group cannot be dragged (reorder the group itself, or use *Modify ▸ Arrange*). The
  panel menu entry *Show Group Contents* (on by default) switches all of this off; the panel then
  looks and works like the original.
- **Pages / States:** pages and animation frames, master page, onion skinning. When an animation
  instance is selected, the canvas shows its motion path (green start, red end), and the selection
  frame encloses only the current frame.
  *New Page* (footer button, ≡ menu or a row's context menu) inserts the page directly after the
  current page and makes it current; it is named "Page N" with the smallest number not yet used.
  *Duplicate Page* (≡ menu, context menu, or dragging a row onto the New Page button; Alt+click on
  the button duplicates the current page) puts the copy directly after its source and makes it
  current; it is named "<name> copy", then "<name> copy 1", "copy 2" … (the copy of a copy counts on
  from the original name). Clicking or right-clicking a row makes that page current first, which is a
  History step "Change Page Selection" of its own.
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

- **Pop-up sliders (▾):** while you move the thumb, the canvas previews the value live (preview only) for
  opacity (also the Opacity slider in the Layers panel, which like its Blend mode list acts on the selected objects
  and is blank when they have different values), stroke size, stroke edge softness, fill edge feather, fill
  and stroke texture amount, rectangle roundness, the channel sliders of the Color Mixer (on the selection), and for
  selected text objects font size, tracking, leading, indent, space before and after and horizontal scale (with the
  text editor open only the field shows the value). Press on the arrow and drag: the value is applied on release; Esc
  discards it. Click the arrow: the slider stays open and the value is applied when you click beside it or press
  Enter; Esc discards it. That click beside the slider (on the canvas, another object, a panel or the menu bar)
  only closes it and applies the value to the objects it was opened for; it selects nothing and opens no menu
  (click again for that), as in the reference. Undo or Redo while the slider is open (shortcut, *Edit* menu or a click in the History
  panel) first discards the pending value and closes the slider like Esc, then undoes or redoes; a pending value
  only ever applies to the objects that were selected when the slider opened. A typed value is applied with Enter,
  Tab or a click into the document; a click on empty panel space applies nothing. Each apply is one History step;
  an unchanged value makes none. The slider opens
  flush left with its arrow, with the thumb of the current value under the point where you pressed; near the
  window edge it is moved just enough to stay inside (the first drag then sets the value under the pointer).
  The opacity sliders (inspector, tool options, Layers panel and the colour pop-up's opacity) move 1 % per 2 px
  and always give a whole per cent, like the reference.
- **Colour pop-up opacity:** when you change the opacity of a colour pop-up (its slider next to the "100" field, or
  a typed value) for the fill or stroke of the selection — in the inspector, in the Tools panel or in the Color
  Mixer — the canvas previews the colour with that opacity while the pop-up is open. Enter, or releasing after a
  press-and-drag on the opacity arrow, applies it (one History step); Esc closes the opacity slider and then the
  pop-up without any change. Undo or Redo closes the pop-up the same way without applying anything, then undoes or
  redoes.

- **Fill edge:** *Hard*, *Anti-Alias*, *Feather* and *Dither*. Feather and Dither use the width field next to the
  list (a new Feather or Dither edge starts at 10 px). *Dither* gives a retro pixel-art edge: the same transition as
  a feather of that width, but every pixel is either fully on or off, scattered by a fixed 8 × 8 dither pattern that
  is anchored to the document pixels (it does not shimmer when you scroll, zoom or move an object by whole pixels).
  Width 0 is a hard edge; the fill's own opacity still applies. Dither is a Firepixel extension; the reference's
  Hard, Anti-Alias and Feather edges stay as they are. In a `.fw.png` file a dither edge is stored as a feather of
  the same width plus a Firepixel marker, so other programs show a feathered edge. Text objects honour Feather and
  Dither too (the transition starts from the text's anti-aliased glyphs); the text inspector itself offers only
  the *Anti-aliasing level* list (its *No Anti-Alias* entry is the hard edge), so give a text a Feather or Dither
  edge from a script or the command `pi.fill {edge, feather}`.

- **Dither (fill density):** the per-cent field with the small checkerboard icon below the fill texture name turns
  the whole fill into an even pattern of fully opaque and empty pixels: 50 % is a checkerboard half tone of the
  fill colour, 25 % keeps every fourth pixel, 0 % switches it off and 100 % is the solid fill. It works for every
  fill category (solid, gradient, pattern) and uses the same fixed 8 × 8 pattern as the Dither edge, anchored to
  the document pixels. Combined with a Dither edge a pixel stays on only where both keep it, so the band fades out
  from the chosen density. A change is one *Fill* step in the History panel; changing the fill category keeps the
  density. Dither density is a Firepixel extension; a `.fw.png` file stores it only in a Firepixel marker (written
  when the value is not 0), other programs show the plain fill.

- **Gradient and pattern fills on text:** a text takes a gradient or pattern fill like a path (`pi.fill`,
  `setFill`); the glyphs show the fill exactly as a path with the same fill handles would. The handles lie in
  document coordinates: a new linear gradient runs from the top centre to the bottom centre of the text's bounds, a
  new pattern has its tile corner on the centre at 100 %. Moving the text (dragging, arrow keys, X/Y fields) takes
  the handles along; the Scale tool and Free Transform leave them where they are, even when you drag the text inside
  the transform frame (the text grows or moves, the fill does not); *Numeric Transform* and `scaleSelection` scale
  them about the centre, with or without "Scale attributes"; editing the text (more characters, another size) leaves
  them in place.

- **Fill handles of gradients and patterns:** with the Pointer, a selected object with a gradient or pattern fill
  shows its fill handles: the round centre handle and one or two square handles on lines from it. Dragging the
  **centre handle moves all three handles together** (the whole pattern or gradient shifts; tile size and angle
  stay), as measured in the reference. Dragging a square handle moves only that handle (it changes the tile width or
  height and the angle of a pattern, or the length and direction of a gradient). Dragging a **line** from the centre
  handle turns both arms rigidly about the centre handle (lengths stay; the dragged arm points at the pointer).
  **Shift** limits a drag to the dominant axis (horizontal or vertical). **Alt** on a handle clones the object
  like an Alt drag instead of moving the handle. The fill follows live while you drag. Over a handle the pointer
  is a hollow arrow, over a handle line an arrow with a rotate sign. Pressing on a handle never moves the object;
  drag the object at a spot away from the handles. Each handle drag is one History step.

- **Lock pattern to document (Firepixel extension):** normally a pattern or gradient moves with its object, as in
  the reference. Press the padlock button in the fill section of the Properties inspector (right of "Transparent",
  next to the gradient's dither button; shown when the selected object has a pattern or gradient fill) and the fill
  stays put in the document instead: moving the
  object (dragging, arrow keys, the X/Y fields, *Align*, `moveSelectionBy`/`moveSelectionTo`) and transforming it
  (Numeric Transform, the Scale, Skew and Distort tools, Free Transform, flips and rotations, with or without "Scale
  attributes") leave the fill handles where they are, so the object slides over a fixed pattern. Ticking or
  clearing it is one History step "Fill". Copies keep the setting: a duplicate, a clone, an Alt-drag copy or a pasted
  copy shows the pattern in the same place in the document as the original. You can still drag the fill handles
  themselves. The reference has no such option; in a `.fw.png` file the setting is kept only in the Firepixel marker
  of the fill (other programs show the object with the pattern moving along, files without it stay unchanged).
  Commands: `pi.fill {lockPattern: true}`; in scripts `setFill` with `lockToDocument: true` (a fill object read from
  `pathAttributes.fill` reports `lockToDocument: true` only when it is on).

- **Live Filters (+):** the “+” menu groups the effects as in the original, e.g. *Bevel and Emboss* ▸
  *Inner Bevel*, *Outer Bevel*, *Inset Emboss*, *Raised Emboss*. The kind is part of the menu item;
  the dialog has no separate field for it. Double-click an entry in the filter list to open its settings.

- **Screentone (Live Filter, Firepixel extension):** *Other* ▸ *Screentone* turns the object's pixels into a
  halftone screen like the printed tones of black and white illustrations and manga pages. Settings: *Pattern*
  (Dots, Lines, Cross-hatch), *Size* (cell pitch 2–32 px), *Angle* (0–180°, default the classic 45° screen),
  *Density from luminance* (on: dark areas get large dots, light areas small ones, white none; off: the constant
  *Density* 0–100 % everywhere), *Color* of the ink (default black) and *Keep object color* (ink with the object's
  own colour instead). Inked pixels are fully opaque, all others become transparent; the screen is anchored to the
  document grid. A `.fw.png` file stores the filter only as a Firepixel entry of the effect list, so the reference
  format keeps only its own effects (other programs show the object without the screen).

- **Text tool defaults:** attributes set while the text editor is open change only the marked text;
  set with text selected by the pointer, or with nothing selected, they become the defaults for new texts.
  A font change also puts the font at the top of the recent fonts; applying a style from the Styles panel
  to text makes its font, bold/italic/underline, leading and anti-aliasing the defaults (not the size). The
  baseline shift becomes a default only through the Text tool's own settings, not through the inspector.
  Choosing the Text tool (toolbox, T, or a switch from another tool) resets indent, spacing before and
  after, tracking and horizontal scale to their factory values, and the leading (as a percentage) and the
  baseline shift to the values of Preferences ▸ Text; font, size, style and alignment stay. So paragraph values set while the Text tool is
  active carry over to the next text, values set with the Pointer do not. The preference
  `TextEditor_RememberALLSettings` (off by default, no row in the Preferences dialog) keeps them. A new text starts with
  tracking 0, and every change in a text field of the inspector is its own "Text Change" step in the History.
- **Font search:** the opened font list (text inspector, Text tool, text editor) has a search field
  at the top. Typing filters (part of a word, case-insensitive), ↑/↓ select, Enter applies, Esc
  closes without a change.
- **Textures and patterns:** the lists show all textures or patterns from the resource folders;
  "Other..." below the list picks an image file of your own.
- **Color picker:** a click on a color box opens the popup with swatches, HEX field and eyedropper.
  While you move over the window, the eyedropper picks up every visible color, also over panels,
  menus and open dialogs. A click applies the color. Outside the Firepixel window the working color
  stays. HEX and RGB always show the base color; its opacity shows only in the opacity field and in
  the preview box (the reference shows a color mixed over white when it reopens a semi-transparent
  color; Firepixel deliberately does not). Press on the opacity arrow and drag: the opacity follows
  the pointer and is applied on release, which closes the popup; Esc during the drag cancels.

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
In the PDF export options, the compression "JPEG 2000" writes the page images as JPEG 2000 (JP2,
`/JPXDecode`). "Quality" sets the compression ratio, from about 10:1 (High) to about 40:1 (Low).
Transparency goes into a separate soft mask, as with JPEG. JPEG 2000 exists from PDF 1.5 on, so with
"PDF 1.3" or "PDF 1.4" the file is written as PDF 1.5.

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

## Third-party notices

The batch step *Blur faces* (`ai.faceBlur`) finds faces with a detector built into Firepixel. Its
classifier is the frontal face cascade `haarcascade_frontalface_default.xml` by Rainer Lienhart from
the OpenCV project, converted into a compact form and embedded in the program. It is distributed
under the following licence:

                            Intel License Agreement
                    For Open Source Computer Vision Library

     Copyright (C) 2000, Intel Corporation, all rights reserved.
     Third party copyrights are property of their respective owners.

     Redistribution and use in source and binary forms, with or without modification,
     are permitted provided that the following conditions are met:

       * Redistribution's of source code must retain the above copyright notice,
         this list of conditions and the following disclaimer.

       * Redistribution's in binary form must reproduce the above copyright notice,
         this list of conditions and the following disclaimer in the documentation
         and/or other materials provided with the distribution.

       * The name of Intel Corporation may not be used to endorse or promote products
         derived from this software without specific prior written permission.

     This software is provided by the copyright holders and contributors "as is" and
     any express or implied warranties, including, but not limited to, the implied
     warranties of merchantability and fitness for a particular purpose are disclaimed.
     In no event shall the Intel Corporation or contributors be liable for any direct,
     indirect, incidental, special, exemplary, or consequential damages
     (including, but not limited to, procurement of substitute goods or services;
     loss of use, data, or profits; or business interruption) however caused
     and on any theory of liability, whether in contract, strict liability,
     or tort (including negligence or otherwise) arising in any way out of
     the use of this software, even if advised of the possibility of such damage.
