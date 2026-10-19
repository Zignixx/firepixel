# Open findings

Last updated: 2026-10-19

This file lists the findings that are still **open** in Firepixel: known bugs, deviations from the reference application, postponed features and accepted limitations. Closed findings are not kept here; their history lives in the commit log.

## How findings are numbered

Every finding keeps the ID it was given when it was first recorded, so it can be traced through commits, tests and specifications:

- `BA-nnn` – item from the consolidated backlog of open findings (one BA number can cover several related entries).
- `TW-nn` – deviation found in a side-by-side comparison run against the reference application.
- `Nnn` – finding reported directly by the user (for example `N11`).
- Other tags such as `F24`, `U-1`, `L-3`, `K-7` or `E6` name the work package or comparison step that found the issue. Where a finding has both a tag and a BA/TW number, both are given.

Findings are grouped by their state:

- **[F] Fixable** – a bug or deviation that can be fixed in the current architecture (often after one more measurement in the reference application).
- **[V] Deferred** – a larger feature or expansion that was postponed on purpose (batch extensions, PDF/PSD export details, ...).
- **[N] Not feasible / informational** – limits of the platform or of the project rules (for example `unsafe_code = "forbid"`), performance notes for machines without a GPU, and items that only need a regenerated report.

## How to add a finding

1. Open a GitHub issue with the **"Parity gap"** template (Firepixel behaves differently from the reference application) or the **"Bug report"** template (anything else that is wrong). Describe the steps, the expected result and the current result.
2. A maintainer reviews the issue, gives it a new ID (next free `BA-nnn` or `TW-nn`, or the tag of the work package that will handle it) and adds it to the matching section below with a link to the issue.
3. When the fix is merged and verified, the entry is removed from this file.

---

## [F] Fixable now (6)

Recounted after F48, runner R-22 and release v0.2.3 (19 Oct 2026); 3 rows open, 8 waiting for confirmation in run 25. The earlier remnants (sprite
padding default, Shift-constrain for L-shape/arrow line, import rubber band, New Style defaults, text runs, SVG
layers, symbol origin, simple brush runner, TW-101 import edge, TW-102/103 export format choice, TW-104 symbol
properties header, TW-100 library drop, TW-105/106 runner effects, TW-107 start colours, TW-109 Alt rectangle) are
closed; their history is in the commit log and the archive (see [ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)). Run 15 closed
TW-97, TW-110, TW-112, TW-114 and TW-116; run 16 (preview `2026-10-18_02-04`, package F31, runner R-11) closed
TW-113/Q-52, TW-118/Q-53, TW-119, TW-120 and TW-121.

The wave after run 16 closed the rest:

- **TW-122 (Q-54):** in the reference application's own shortcut set Ctrl+Shift+B is Arrange ▸ Send Backward;
  Firepixel's bindings were right, but the runner profile used the "Web Standard" set (Ctrl+Shift+B = Break Apart).
  The parity runner now writes the Firepixel shortcut set into its fresh profile. Arrange commands are silent no-ops
  (no history step, document stays clean) when nothing can move, and faux bold/italic/underline on pointer-selected
  texts toggle across the whole selection.
- **TW-123:** the Optimize colour-table context menu is sized to its content (about 270 px); the Frames panel menu
  and the swatch drop-down were fixed the same way. Menus app-wide gained a 33 px arrow column; to be checked in the
  next full run.
- **TW-115 and TW-124 (runner R-12):** 9 px swatch pitch, objects serialised over the bridge, panel pop-up menus
  read instead of the panel, plain `for` loops in read-back scripts, drag end points allowed outside the view,
  remeasure or `unsupported` for the guide double-click and the export-area preview, inverted shot regions normalised.
- **Questions answered:** Q-49 (the colour pop-up HEX field now parses every keystroke like the reference),
  Q-51 (the reference does not switch the inspector tab back by itself, so Firepixel is right; runner step 04b
  stays and TW-111 closes as "no deviation"), Q-55 (the text-tool click behaviour was already right; the runner
  clicked a different point, step `xs-shortcuts/15` fixed), Q-56 (Modify-menu enable rules implemented and pinned by
  tests; Arrange is grey in bitmap edit mode).

Full run 17 (18 Oct 2026, preview `2026-10-18_04-57` from `7e3d2d12`, runner R-12) confirmed **TW-115** (swatch
probe and read-back #efffbf on both sides), **TW-123** (Optimize colour-table menu 282 vs. 270 px; panel menus within
5 px of the reference, none wider because of the 33 px arrow column), **TW-124** (object read-back, `for`-loop
read-backs, guide drag onto the ruler, `unsupported` instead of errors), **TW-111 / Q-51** and the byte-identical
animated GIF. It did not confirm **TW-122**: under the shortcut set the runner now writes, the text style shortcuts
and Ctrl+Shift+D behave differently from the reference machine (continued as TW-126). **Q-55** is continued as
TW-130 / Q-57. **Q-49** cannot be proven by the runner (the reference's pop-up closes without applying the text the
runner sets); it moves to the human click-test. Tags in the table: *RE spec* = write a behaviour description from
the reference first, *direct fix* = fix from the measured values, *runner* = parity runner change (R-13).

Follow-up run 18 (18 Oct 2026, preview `2026-10-18_10-59` from `4e677046`, packages F33/F34, runner R-13/R-13b with
the "Web Standard" set in Firepixel's profile, the reference machine's set) closed **TW-125** (Common Library menu
183 vs. 179 px), **TW-126** (Ctrl+Shift+D clones on both sides: `menu-edit/02`, `dialogs-d31-d40/06` and
`menu-modify-a/11` equal), **TW-127** (option start states set and read back; `xs-preferences/15` equal), **TW-128**
(both slices report JPEG and GIF) and **TW-129** (Feather / Save Bitmap Selection dialogs, the "add swatches" file
dialog, the library drop and Snap to Pixel with a selection all measured on the reference side). What is still
different is listed below as TW-131 to TW-135; the reference-side remainder of TW-129 (plain arrow keys) moved to
TW-132. Tags in the table now refer to runner R-14.

The wave after run 18 (18 Oct 2026) closed most of them; run 19 of the affected units is to confirm:

- **TW-131:** the URL panel menu has the reference's nine entries in its order, with its separators and enable
  rules (edit and delete grey without a selected URL): add used URLs (slices, hotspots, button instances and button
  symbols of the open documents), clear unused URLs, add / edit / delete URL, new URL library, import and export
  (`.txt` one URL per line, `.htm` one link per URL), Help. The default library stays in `URLs.txt`, further
  libraries are HTML files in `URL Libraries`. Limits: these panel actions are UI-side only (not reachable over
  CLI, MCP or JSF), and library edits have no undo.
- **TW-133 (in part, RE spec 48):** *Select Inverse* only inverts a pixel selection and otherwise shows a warning
  ("do not show again" possible) without touching the object selection; the Break Apart warning was already right
  (the runner had closed the reference's dialogs). *Text ▸ Size ▸ Larger / Smaller* step by exactly 1 pt (clamped
  at 1 pt), on selected texts and in the open editor. Stretched alignment leaves point (auto-width) text alone.
  Still open: "justified" on point text (see the table).
- **TW-134:** the preference *Delete objects when cropping* (`CropDocCropsImages`, Preferences ▸ Edit, default on)
  was stored but never read. The crop tool, *Crop Document* and *Canvas Size* now honour it: on deletes objects
  entirely outside the new canvas on all layers and states and clips crossing bitmaps (crossing vectors stay); off
  deletes and clips nothing. Pinned by tests for all three paths.
- **TW-135:** a pointer double-click on a text opens the editor with the caret at the click point; the late caret
  report a few frames later is removed, so a character typed at once is no longer lost or misplaced.
- **TW-132 (runner R-14):** read-back scripts on the reference side record open dialogs (values and a dialog shot)
  before they run, every dialog the runner cancels is logged, the history read-back is guarded and retried, Delete,
  Ctrl+D and the arrow keys are checked for effect, the text tool is reset at the start of a unit and Firepixel's
  menu read-back takes the enabled state from the accessibility tree. TW-130 / Q-57 stays open until run 19 reads
  back all text lines on the reference side.

The wave after run 19 (18 Oct 2026, on `main` up to `054bdc05`; confirmed by run 20) closed:

- **TW-136:** not a deviation. The reference's labels end in "..." as well, so they already matched; the 17 px came
  from the shot including the ≡ button row above and the drop shadow below. The menu frame is 323 × 231 px vs. the
  reference's 321 × 231 px; a kittest pins the size within 5 px of the reference.
- **TW-137 (RE spec):** Copy and Cut are History rows of their own ("Kopie" / "Copy", "Ausschneiden" / "Cut"), like
  Select All (always, also when everything is selected already) and Deselect (only when the selection changes). Copy
  is an undoable no-op and does not mark the document as changed; undo and redo of a step that leaves the document
  untouched no longer mark it either. A step without a name reads "Befehlsskript" / "Command script". The history
  limit `MaxUndos` was already 20 steps per document and drops exactly the oldest step (now pinned by tests).
- **TW-138 (RE spec):** text attributes set in the inspector while the text editor is open change only the marked
  text (or the caret) and never the text tool defaults, also not on commit or tool switch; with text objects selected
  by the pointer, or with nothing selected, the values become the defaults. Defaults are app-wide (`Text_*`
  preferences). The *Text* menu remnant stays open (table).
- **TW-139:** script read-back (element list, `textChars`, `pixelRect`) shows the live editor text while editing: a
  new text from the first click, edited text at once. Limits: MCP `get_document` and `render` still show the
  committed state, and the live element is not part of the selection.
- **TW-141 (package F40):** every main-menu row has an accessibility node with its enabled state, so the parity
  runner reads grey entries in Firepixel (Modify ▸ Snap to Pixel without a selection: `enabled: false`).

Follow-up run 19 (18 Oct 2026, preview `2026-10-18_13-04` from `a3510515`, packages F34–F36, runner R-14) closed
**TW-130 / Q-57** (the typed "Vr" lands in line 2 at the click point in both programs: "ZeileVr zwei"), **TW-131**
(nine URL menu entries in the reference's order), **TW-125** (States menu 210 vs. 207 px), **TW-135** (Firepixel
inserts at the double-click point, "ZeilQe eins", as the reference did in run 18; in run 19 the reference's
double-click opened no editor) and **TW-132** (dialogs recorded before the read-backs, history read-back, Ctrl+D and
the text read-backs work on the reference side; the rest continues as TW-140 and TW-141). TW-133 is confirmed except
for the stretch shortcut. TW-134 was not part of run 19 (A4-13 and the next `xs-preferences` run). New: TW-136 to
TW-141. Tags: *RE spec* = behaviour description from the reference first, *direct fix* = fix from the measured
values, *runner* = parity runner change (R-15).

Follow-up run 20 (18 Oct 2026, preview `2026-10-18_15-28` from `054bdc05`, packages F37–F40, runner R-15) closed
**TW-137** (`menu-edit/22`: both Histories list Clone, Copy, Paste), **TW-138** (`tool-text/14, 16`: the new text
keeps 12 pt black after attributes were set on a partial selection; `14` now 0.54 %), **TW-139** (`tool-text/07`
shows "Hallo Welt" and `xs-shortcuts/15_in_text_tx2` "ZeileVr zwei" while the editor is open) and **TW-141**
(`xs-canvas/15_snap_pixels_disabled`: `enabled: false` in both programs). **TW-134** stays unconfirmed: in
`xs-preferences/06` Firepixel crops to 280 × 220 and removes L1, ST and PG, but the reference did not apply the crop
(page 800 × 600, all seven paths). TW-140 stays open: runner R-15 did not change the reference-side keys. New:
TW-142 to TW-144. Tags refer to runner R-16.

The wave after run 20 (packages F42–F44 on `main` up to `65db4012`) fixed TW-142 (panel pop-up rows expose enabled
and toggled state), TW-143 (live bounds, `d1e79ed5`), TW-133 (stretched alignment stores and loads as left; the
script setter follows the command path) and changed the TW-138 rule after an RE spec (every text attribute command
writes its `Text_*` default, also on a partial selection; new text ignores `Text_FillColor`); TW-144 got tests that
pin the separators. Follow-up run 21 (18 Oct 2026, preview `2026-10-18_22-05` from `65db4012`, runner R-17 / R-17b /
R-18) closed **TW-133** (`xs-shortcuts/06`: all seven keys equal; Ctrl+Alt+Shift+S leaves TX1 "left" in both
programs), **TW-142** (`panel-library-styles/16`: nine entries with equal labels, `ellipsis` and `enabled`, Edit and
Delete URL `enabled: false` in both) and **TW-144** (with CR/LF kept, both read-backs separate the paragraphs of TX2
with `\r`; the reference's `\n` in run 20 was the read-back). Not confirmed: **TW-143** (see the table) and
**TW-134** (the reference's page stays 800 × 600 after R-17's `crop` action, so the crop option is still not
compared; Firepixel crops to 280 × 220 and keeps P1, E1, R2, R1). TW-140 is refined with the mechanism the new key
lines show. New: TW-145. Tags refer to runner R-19.

Package F45 (`a4c2bf2b`: element-list read-backs return the live text element while editing, TW-143) and runner R-19
(`7bb38c49`: reference-side substitutes for plain Delete, arrow and Escape keys, text selection by mouse drag, crop
commit by double-click, text colour reset at unit start) went into follow-up run 22 (18/19 Oct 2026, preview
`2026-10-18_23-25` from `a4c2bf2b`). It closed **TW-140** (Edit ▸ Clear deletes R1 in `menu-edit/09, 22` and both
Histories list Clone, Copy, Paste, Clear; `moveSelectionBy` nudges by 1 px with values equal to Firepixel's arrow keys
in `xs-shortcuts/10` and `panel-pages-states-history/14–21`; the Pointer tool click closes the text editor without a
stray "c"; Escape no longer cycles tools in `xs-preferences`; the history-limit read-back item moves to TW-147),
**TW-143** (`xs-shortcuts/15_in_text_tx2`: TX2 reads 50,150–120,194 in both programs while "Vr" is typed;
`tool-text/06`: Firepixel reads 114 live, the reference's double-click opened no editor in this run) and **TW-134**
(`xs-preferences/06`: the double-click commits the reference's crop, the page is 280 × 220 and P1, E1, R2, R1 remain
in both programs). The unit-start reset leaves the reference's text tool at ArialMT 12 pt regular #000000 in all seven
units. TW-145 is still not compared. New: TW-146 and TW-147. Tags refer to runner R-20.

Package F46 (`349fd4d0`, `cd86ce75`, after RE spec 51: Escape keeps the selection; one History row per arrow-key
nudge) and runner R-20 (`b1d34247`: editor polling and keyed typing with a commit check, drag selection verified by
highlight pixels, Pointer tool from the flyout, `orig.escape_deselects=false`, real Shift+arrows in
`panel-pages-states-history/21`, `history_not_comparable` flags, a paste check in `menu-edit/03`, `MaxUndos`) went into
follow-up run 23 (19 Oct 2026, preview `2026-10-19_01-32` from `cd86ce75`, 5 units). It closed **TW-147**: (a)
`xs-shortcuts/07` takes the Pointer tool from the flyout when the slot holds Select Behind, no runner error (13 in run
22); (b) `xs-shortcuts/21`: R1 stays selected after the Escape chain in both programs (1 / 1); (c) the nudge rows are
now compared with real Shift+arrows and continue as TW-148; (d) `menu-edit/03`: R1 at 50,50–150,120 on both sides
after Ctrl+X, Ctrl+V; (e) `panel-pages-states-history/22`: `MaxUndos` reads 20 in both programs. TW-146 is reduced to
a rest (14 of 17 reference texts are now created as typed). TW-145 is still not compared. New: TW-148 to TW-150. Tags
refer to runner R-21.

Runner R-21 (`b1916aa6`) reads the History row labels on both sides (`history_rows`: the reference's rows one by one,
Firepixel's from the bridge history), accepts the reference's text editor only with a caret before typing, resets
Firepixel's text tool at unit start to the same font, size, style and colour as the reference (ArialMT 12 pt regular
#000000), raises the slow limit per substitute (90 s + 2 s each) and writes foreign-window titles and log lines
without the console code-page error. TW-146 rest, TW-149 and TW-150 are fixed in the runner and wait for
confirmation in run 24; TW-148 now has the row labels in the log and is decided after run 24.

The tester feedback batch of 19 Oct 2026 by E-Man (pen tool and Select Behind) became TW-151 (FB-01) and TW-152
(FB-02), both after RE spec 52 and fixed on `main` (`c3a9e264`, `44d7c5f5`). The hover rule of Select Behind was
measured in the reference application with the new parity unit `xs-select-behind` (`c7515986`, 9 steps). Both rows
were kept, marked closed, for the human click-test (VERIFICATION A4-15 to A4-17) and run 24.

Full run 24 (19 Oct 2026, 06:09–11:10, all 49 units, preview `2026-10-19_06-06` from `44d7c5f5`, runner R-21; see
[PARITY.md](PARITY.md)) closed **TW-149** (Firepixel's text tool starts at Arial 12 pt regular black in 46 of 49
units, like the reference; the reset failed with a bridge error in three units without new text), **TW-150**
(`xs-shortcuts/08_tools` `ok` in 92.8 s under the new limit; the run-start window list has no code-page error) and
**TW-152** (FB-02: the hover rule of Select Behind is equal in `xs-select-behind`: the same 1680 px #E00000 frame on
B1 in `02` and `04`, no frame in `03` and `07`, the click cycle B1 → FR; the remaining differences continue as
TW-157). It decided **TW-148**: the reference merges consecutive key nudges into one History row, so F46's one row per
press is wrong (row now a fix item). TW-151 (FB-01) shows no regression in the pen-related units (`tool-path-reshape`,
`inspector-stroke`) but is not measured by the runner; it stays until the click-test (A4-15). TW-146 rest and TW-145
stay open. New: TW-153 to TW-157. Tags: *RE spec* = behaviour description from the reference first, *direct fix* =
fix from the measured values, *runner* = parity runner change (R-22).

Package F48 (`0419534c`, `ace7873c`) and runner R-22 (`72c695dd`) went onto `main` after run 24; release v0.2.3 was
published the same day. F48 fixes **TW-148** (consecutive key nudges of an unchanged selection merge into one Move
row with the summed offset; this corrects F46 per the run-24 measurement) and **TW-157** (crisp pixel-aligned hover
line, no frame right after a Select Behind click until the pointer moves). R-22 fixes the runner items **TW-153**,
**TW-155** and **TW-156**, closes **TW-154** as not comparable (no menu entry and no key in the reference) and adds
retries for **TW-146 rest**. All of them wait for confirmation in run 25 (the R-22 units). The count of 11 rows now
includes eight fixed or closed rows waiting for confirmation (TW-146 rest, TW-148, TW-151, TW-153 to TW-157); three
are open: TW-138 rest, TW-145 and U-1 rest.

Follow-up run 25 (19 Oct 2026, 12:19–14:39, 20 units, preview `2026-10-19_12-16` from `a225a01c`, runner R-22; see
[PARITY.md](PARITY.md)) confirmed and closed **TW-148** (`panel-pages-states-history/21`: the reference lists
`moveSelectionBy({x:20, y:10})` and `moveSelectionBy({x:10, y:0})` between the runner's select scripts, Firepixel
Move, Deselect, Move: one row per selection in both), **TW-157** (`xs-select-behind/08`: 680 px of #E00000 in
398,198–602,302 in both programs; `05`, `06`: no frame in either right after the click), **TW-153**
(`tool-autoshapes` 26 / 27 canvas pairs, fill #ffffff and stroke #000000 on both sides), **TW-154** (`menu-text/05`
and `menu-view/20` `unsupported` on the reference side, Firepixel opens both dialogs, shots present), **TW-155**
(`inspector-stroke/01` without a key and without runner error; `menu-edit/01, 22` read the reference's History with one
document: script row plus Clone, Copy, Paste, Delete against Firepixel's Clone, Copy, Paste, Clear; the
`tool-path-reshape/04` rest moves to TW-159), **TW-156** (`toolbox/13` starts and ends in Standard mode on both
sides; the mode cycle itself shows TW-158) and the TW-149 leftover (Firepixel's text reset succeeds in all 20 units,
including `panel-layers`, `panel-layers-menu` and `panel-pages-states-history`). It decided **TW-145**: the
reference's size field again misses the selected "123" in `tool-text/11`, but its Text ▸ Size 72 sets the text tool to
72 pt and its next texts in `13`, `14` and `16` are 72 pt like Firepixel's (`14` 0.18 %, `16` 0.27 %; run 24: 1.07 %
and 3.74 %), so the F43 rule gives the same next-text size; closed. **TW-146 rest** stays open, reduced: 16 of 17 new
texts are typed and none is discarded; only `07` loses its first character. New: TW-158 (*RE spec*) and TW-159
(*runner*). The 6 rows: TW-138 rest, TW-146 rest, TW-151 (click-test), TW-158, TW-159 and U-1 rest.

| ID | Area | What is open (expected vs. current) |
|---|---|---|
| TW-138 rest | Text tool defaults, Text menu (*RE spec*, rest) | The inspector follows the reference since the wave after run 19 (see above). Not covered by the behaviour description: *Text* menu commands (font, size, style, alignment) applied to pointer-selected text do not yet store their values as text tool defaults the way the inspector does. Measure in the reference whether they do, then align. |
| TW-146 rest | Reference-side text entry in `tool-text` (*runner*, R-23) | Run 25 (R-22): 16 of 17 new texts are created as typed and none is discarded at the commit (run 24: 12 of 16, three discarded); `13` finds the colour pop-up and `17` no longer opens the export-format combo. Left: the second text of `07` loses its first character "H", the click retry finds no editor and the following double-click opens none; in `11` the size field and Text ▸ Size do not reach the drag-selected "123" (the size goes to the text tool instead, which decided TW-145), and in `13` the drag over "123" leaves no highlight. Next: wait for the editor after the click retry and select "123" by double-click or Shift+Home before the size edit. |
| TW-151 (FB-01) | Pen tool: click on an existing anchor while drawing (tester feedback, *RE spec* 52 §1) | **Closed** (`c3a9e264`, confirm by click-test A4-15; run 24: no regression in `tool-path-reshape` and `inspector-stroke`, the anchor modes themselves are not measured by the runner). Clicking the last anchor of the path being drawn retracted nothing; in the reference it retracts the anchor's outgoing handle and the next segment leaves the point straight. Now: pressing on the last anchor sets its outgoing handle onto the anchor and keeps the incoming one; the path stays in progress. A drag past 2 screen px pulls the point again as a symmetric curve point (Alt keeps the previous incoming handle, Ctrl moves the point, Shift snaps to 45°). While drawing, an interior curve point becomes a corner, an interior corner point is deleted, a click on a segment inserts a point, the endpoint of another open path joins it; without a path in progress a click on an open path's endpoint continues it. Cursors: pen with a caret (convert) and pen with a ring and stub (join). The hit radius follows the *Snap distance* preference. Known limits: a join across elements appends the other path instead of merging the elements; no drag after a join, a close or an interior press; a double-click still ends the path. |
| TW-158 | View ▸ Screen mode: full screen (*RE spec*) | Run 25 `toolbox/13_view_modes_full_normal`: after the second F the reference shows only the document on a black background, without menu bar, document tabs, panels and status bar; Firepixel's full screen mode keeps the menu bar, the document tab and the status bar on its grey pasteboard (the first F, full screen with menus, matches: menus and panels in both, the reference's panels as icons). Write the behaviour of the three screen modes from the reference first (which bars and panels each mode hides, pasteboard colour, how Tab and Escape act, how the next F returns to Standard), then align. |
| TW-159 | Runner rests after R-22 (*runner*, R-23) | (a) `toolbox/13`: the reference's screen-mode read-back reads only the main window style and reports "standard" in all three modes, so the mode cycle is compared by the shots only; (b) `xs-shortcuts/10_nudge_point_result`: R-22 sent a real Right with the Subselection tool (log: "the arrow is not a nudge"), which selected the Text tool on the reference side, so the step is `unsupported` although R-22's rule substitutes arrows for that tool (−1 measured step); (c) `tool-path-reshape/04_freeform_size_keys`: the real Up switches the reference's tool (Freeform → Pen → Vector Path → Redraw Path), so the Freeform size keys are not compared; use the size field or mark the step unsupported; (d) `menu-text`: the reference's unit-start text reset left the size at 24 pt (taken over from `inspector-text`). |
| U-1 rest | File ▸ Import, PDF | SVG files now import with their layers; **PDF files still arrive as one layer** because the pure-Rust PDF reader exposes no optional content groups (layers). Library limit; needs either an upstream feature or an own OCG reader. |

## [V] Deferred / expansion items (5)

Wave 3 (18 Oct 2026) closed BA-086, BA-100 (PDF selectable text), BA-102 (PSD layers per object with text,
vector and effects), BA-105 and BA-106; BA-108, BA-109 and BA-110 were shortened to what is still open.
Wave 4 (18 Oct 2026) closed the rest of the expansion work:

- **BA-103 HTML Setup:** "Set Defaults" stores the dialog's values as document-independent defaults (used by New
  Document and by the HTML export for pages without settings of their own); a new option writes the page styles
  to an external `styles.css`; a new neutral "Generic HTML5" style; button symbols keep their navigation-bar
  markup and script in every style (tests export a nav-bar document).
- **BA-104 (CMD-1) Commands menu:** `commands.screenCapture` (via the operating system's own screenshot tool),
  `commands.symbolScript` (generates a JSF script for the selected graphic symbol) and `commands.sharedPalette`
  (one adaptive palette over all open documents, filled into the Swatches panel; `.act` file via the `path`
  parameter for CLI/MCP) now work. Limits are listed under [N].
- **BA-107 complete:** the "ffmpeg for videos" row in Preferences (same setting as the Batch Studio).
- **BA-037 AI hover preview:** optional, preference "Show selection preview while hovering" (off by default);
  only with a warm model cache; a click on the same pixel reuses the result.
- **BA-039 brush preview in stacking order:** brush, pencil, eraser and rubber stamp preview at the bitmap's
  layer position with its opacity, blend mode, effects and masks. Limits are listed under [N].

| ID | Area | What is deferred |
|---|---|---|
| BA-101 | PDF/PSD export | No JPEG 2000 encoder: the PDF compression "JPEG 2000 (JPEG)" writes DCT/JPEG on purpose and says so loudly (option label, the PDF's producer entry, both manuals). PSD export: the model has no stroke effect, so nothing maps to a PSD stroke. |
| BA-083 (BATCH-4) | Batch tests | Golden hashes of the resampler depend on `f64::sin` (Lanczos) and may need regenerating on other operating systems. Open until CI runs on another OS. |
| BA-108 (BATCH-5) | Batch metadata | Writing metadata boxes for AVIF/HEIF/JXL is missing (reading works since wave 3; TIFF reads and writes EXIF/GPS/IPTC). |
| BA-109 (BATCH-6) | Batch steps | All steps of the batch spec exist except `ai.upscale` and `ai.faceBlur` (need models). `format.multiPage` only combines; splitting multi-page sources stays with `document.pages`/`anim.toFrames`. Runs started in the Batch Studio UI are not checked by the MCP caller gate (the UI keeps the opt-ins of `script.external`/`script.code`). Minor: `fp_raster::Mask` has no `is_empty()`. |
| BA-110 (BATCH-8) | Batch GIF optimizer | Measured with the new opt-in modes `lzw: flexible` and `perceptualPalette` (off by default; figures in [BATCH.md](BATCH.md) §5.6): screen −54.3 %, cartoon −37.9 % (both modes −38.8 %) vs. the export at SSIM ≥ 0.99; the camera pan reaches SSIM 0.99 in no mode (best 0.986). Not done: lossy LZW with an SSIM bound, cancel inside one encode. |

## [N] Not feasible / informational (20)

| ID | Area | Note |
|---|---|---|
| BA-037 | AI hover preview | Tested with a stub decoder only; not yet tried with real model weights. Works only with a warm cache (model loaded, embedding cached); it never loads a model itself. |
| BA-039 | Brush live preview | Bitmaps with live effects are slower (whole scratch raster, 48 px effect margin); strokes that create a new bitmap or paint into a bitmap inside a symbol use the old overlay above all objects; below 100 % zoom the composite tiles are sampled NEAREST. Frame time on 2000 × 1500 rose from 2.4 to 4.3 ms on average. |
| BA-104 (CMD-1) | Commands menu | Screen capture uses the system's own capture tool, so there is no interactive drag rectangle (the area is entered as numbers before the countdown). The symbol script skips bitmaps, nested instances and compound shapes and writes gradients as solid fills. |
| TW-117 | Comparison runner, colour pop-up | Unchanged in runs 15 and 17. Canvas shots differ by capture method: Firepixel's shot contains the open colour pop-up and the selection handles, the original's does not (6.3 % of pixels above 32 levels); the fill under the hovered swatch is equal. Information only. |
| BA-097 | Performance (no GPU) | The first drag of large objects takes up to ~1–2 s with software rendering: the drag preview texture (`canvas\ghost.rs::drag_sprite`) holds the whole selection (up to ~6000 × 1500 px) and uploads at ~30 MB/s; later steps take ~100–200 ms. Idea: limit the preview to the visible area plus a margin. |
| BA-098 | Performance (no GPU) | The first frame after zoom/scroll with a large canvas texture takes up to 1.4 s (`canvas\mod.rs` renders the visible area plus half a view of margin on each side). Edits only upload the changed rectangle; zoom/scroll still upload everything. |
| BA-021 | English UI texts | The English labels of D39 ("Sample radius:") and the English history names of the Select commands were not measured in the original (only the German UI of the original is installed). |
| BA-099 | Performance | With 3200 selected objects, Inspector and Layers panel together still cost ≈ 45 ms per frame (dev profile); the Layers panel builds all rows, including invisible ones. |
| BA-114 (FARB-1b A1/A2) | Color picker | Deliberate deviations: own color dialog instead of the system color picker (`ChooseColor` only via `unsafe`); the eyedropper outside the app window shows the working color. All other points of this entry are done. |
| BA-099 | Performance | Command enabling (`command_state`) costs up to 5 ms per command with 3200 selected objects, mainly `modify.alterPath.*Fill`, `modify.transform.removeTransformations`, `text.orientation.*`. Only noticeable with an open menu. |
| BA-099 | Performance | After a move, `edit_at` drops the render cache of every element; the next render re-rasterizes all objects (costs background work and the first drag step). |
| BA-116 | Text rendering | A CFF test font reaches only ≈ 52–72 % exactly matching ink pixels (different font build on Windows, different CFF hinter than the original). Rotated/skewed text and faux italic stay unhinted; anti-aliasing "Strong" is deliberately unhinted. |
| BA-092 | Test images | Image series and paired images that contain text may change slightly because of hinting; they were not regenerated. Regenerate in the next image run. |
| BA-112 | Native file dialogs | Native file dialogs appear at the top-left corner of the main window (Windows places the Common Item Dialog there even with an owner); the original centres them. Moving them needs Win32 `SetWindowPos`, i.e. `unsafe` or a new dependency – both rejected. |
| BA-113 | Native Open/Import dialog | `rfd` cannot preselect a filter, so "All readable files" comes first instead of 13th. The in-app dialog and `ui_file_dialog` use the original order via `default_filter`. |
| BA-111 | Graphics backend | If DX12 initializes but the window surface fails, there is no fallback to Vulkan/GL in the same start (workaround: `FP_WGPU_ALL_BACKENDS=1` or `WGPU_BACKEND`). Watch on machines with a GPU. |
| BA-087 | Generated reports | Automatically generated test reports and the release package notes still show old German folder and `.ftl` names; the release folders next to the exe are `Languages\` and `Models\`. Regenerated by the next full test run. |
| BA-114 (F5) | Eyedropper | The eyedropper outside the window still returns the working color instead of the screen color. Reading the screen outside the window needs `unsafe` or an extra dependency (`unsafe_code = "forbid"`). |
| TW-58 | PNG file size | PNG8 is ≈ 5 % and PNG32 ≈ 0.6–1.3 % larger than the original's output. Row filters are identical; the difference comes from the deflate implementation (miniz instead of zlib). Decision: no zlib C backend; kept as information. |
| T-3 / T-5 | English UI texts | English step names and messages (`msg-step-add-states`, `msg-step-export-options` "Export Options", `ui-script-*` and other new EN texts) were not measured against the original, because only its German UI is installed (same as BA-021). |
