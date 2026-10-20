# Open findings

Last updated: 2026-10-20

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

## [F] Fixable now (11)

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

After run 25 (on `main` up to `439a3ec5`): runner R-23 (`5f498cd7`) fixes **TW-159** (reference screen mode read
from the tools column's screen-mode buttons and the window against the monitor; "Teilauswahl" as a Subselection
name with a script nudge of the selected points; a tool switch by a real key makes the step `unsupported` with the
reason; the text size reset is read back per attempt) and adds waits and retries for **TW-146 rest**. Package F49
(`8a9cf670`, `439a3ec5`) fixes **TW-158** after the behaviour description of the three screen modes and closes the
measured part of **TW-138 rest** (the inspector never writes the baseline shift default; font changes store the
PostScript name and the recent font list; a style applied to text writes the text defaults). The second batch of
tester feedback (macOS) adds **TW-160** to **TW-162** (`b9f085f4`, `0352e7c9`, `53e08484`), fixed but only
type-checked for macOS; the Mac hands-on checks are VERIFICATION A4-19 to A4-21. The 9 rows: open are U-1 rest and
the runner item TW-146 rest; TW-138 rest is closed except the unmeasured R3 note; TW-158 and TW-159 wait for
run 26; TW-151 and TW-160 to TW-162 wait for the click-test.

Follow-up run 26 (19 Oct 2026, 16:11–16:55, 5 units, preview `2026-10-19_16-07` from `37c0ee81`, runner R-23; see
[PARITY.md](PARITY.md)) confirmed and closed **TW-158** (`toolbox/13_view_modes_full_normal`: after the second F both
programs show only the canvas on black without menu bar, tabs, panels and status bar, 1.97 % against 77.1 % in run 25;
all 49 units are now equal except for design) and most of **TW-159**: (b) `xs-shortcuts/10_nudge_point_result` is `ok`
on both sides and the first node reads 201,210 in both; (d) the reference's unit-start reset in `menu-text` reads 72 pt
before and 12 pt after; (a) Firepixel's screen-mode read-back is exact and the reference's recognises both full
screen modes from the window. The entry part of **TW-146 rest** is confirmed (all 15 new texts and three edits in
`tool-text` arrive as typed, none is discarded, the double-click in `07` finds the editor); its selection part stays
open. Reopened as **TW-159 rest**: (c) the Freeform size keys keep the tool but the step records no size. New (both
*runner*): **TW-163** (the reference's screen-mode button reads "full screen with menus" in Standard, six `runner error`
lines) and **TW-164** (the reference shows the Path Scrubber head in `tool-path-reshape/12_inspector_heads_freeform`).
`tool-text` and `menu-text` show no deviation with F49's text preference rules. The 10 rows: open are U-1 rest and the
runner items TW-146 rest, TW-159 rest, TW-163 and TW-164; TW-138 rest is closed except the unmeasured R3 note; TW-151
and TW-160 to TW-162 wait for the click-test.

Release v0.2.4 (19 Oct 2026, pre-release) ships F49 (screen modes, TW-158; text preference rules, TW-138 rest) and the
macOS fixes TW-160 to TW-162. Follow-up run 27 (19 Oct 2026, 20:49–21:18, 3 units, the same preview `2026-10-19_16-07`,
runner R-24 at `f7d13d61`; see [PARITY.md](PARITY.md)) confirmed and closed **TW-159 rest** (with the mouse held both
programs keep the Freeform tool and read size 72 after the keys; `tool-path-reshape/04` 0.46 %), **TW-164** (Freeform
through flyout entry 0 puts the Freeform head on the reference side, `12_inspector_heads_freeform` pixel-identical to
`01`) and **TW-163** (the reference's Standard mode is read from the window; `toolbox/13_view_modes_standard_normal`
reads Standard on both sides, one `runner error` instead of six). **TW-146 rest** stays open: none of R-24's three size
routes reaches the selected "123". New (both *runner*): **TW-165** (only the reference's text tool is reset after the
failed size step, so `tool-text/14, 16` compare 12 pt with 72 pt text) and **TW-166** (the Standard reset after full
screen finds neither the screen-mode button nor the menu bar). All units stay equal except for design. The 9 rows:
open are U-1 rest and the runner items TW-146 rest, TW-165 and TW-166; TW-138 rest is closed except the unmeasured R3
note; TW-151 and TW-160 to TW-162 wait for the click-test, including the macOS re-test by the tester.

Follow-up run 28 (19 Oct 2026, 21:57–22:21, 2 units, the same preview `2026-10-19_16-07`, runner R-25 at `74d6bfc8`;
see [PARITY.md](PARITY.md)) confirmed and closed **TW-166** (the reference leaves full screen by F, two F from full
screen with menus and one from full screen, each Standard reset reads Standard from the window, and `toolbox` has no
`runner error`). **TW-146 rest** stays open, refined: R-25's area test rejects the 24 changed pixels as the caret, but
none of the six selection tries reaches 60 % of the expected inverted area (ratio 0.12–0.14), so all size routes are
skipped. **TW-165** stays open, refined: the two-sided reset is armed only by a failed field or menu route, so with
every route skipped neither side is reset. New (*runner*): **TW-167** (the reference's panel dock started collapsed
to icons and its 110 px width is taken as the reference; after the screen modes it is expanded and not restored).
Quotas and canvas pairs are unchanged; all units stay equal except for design. The 9 rows: open are U-1 rest and the
runner items TW-146 rest, TW-165 and TW-167; TW-138 rest is closed except the unmeasured R3 note; TW-151 and TW-160 to
TW-162 wait for the click-test, including the macOS re-test by the tester.

**Moved to the click-test (20 Oct 2026): TW-146 rest and TW-165.** After runs 26 to 28 the runner cannot create a
text selection inside the reference application's text editor with the mouse: glyph-box drags (from the start and
from the end, 20 or 40 moves, with and without a first click) and word double-clicks only move the caret, 24 changed
pixels instead of the expected highlight of about 172 px (ratio 0.12–0.14 in `tool-text/07, 11, 13`), so no size or
colour route ever reaches the reference's selection. The two measurements that depend on it, the size and colour
applied to a partial text selection (TW-146 rest) and the two-sided text tool reset derived from that step (TW-165),
are therefore runner measurement gaps, not product deviations. Firepixel follows the code-derived rule (RE specs 49,
50 and 53): an edit on a selection changes only the selected characters, and the next new text takes the size but not
the colour. The hands-on check is [VERIFICATION.md](VERIFICATION.md) A1-19. The 7 rows: open are U-1 rest and the
runner item TW-167; TW-138 rest is closed except the unmeasured R3 note; TW-151 and TW-160 to TW-162 wait for the
click-test, including the macOS re-test by the tester.

**Tester feedback, third report (20 Oct 2026): TW-168 / FB-06.** With the Subselection tool a selected vector object
showed the bounding box like the Pointer tool; the reference shows the path itself. Fixed directly on `main`
(`0b71e6ca`); the runner never captures the reference's selection markings, so the comparison is the hands-on check
[VERIFICATION.md](VERIFICATION.md) A4-23. The 8 rows: open are U-1 rest and the runner item TW-167; TW-138 rest is
closed except the unmeasured R3 note; TW-151, TW-160 to TW-162 and TW-168 wait for the click-test, including the
macOS re-test by the tester.

**Tester feedback, batch 3 (20 Oct 2026): TW-169 to TW-172 / FB-07 to FB-11.** Five reports by E-Man on the Pen
undo steps, Redo, the inspector opacity and stroke width sliders and the Subselection snapping, measured in the
reference as RE spec 55 and fixed in F51a (`22f62a54`), F51b (`0932f67c`) and F51c (`901f0a56`). The runner does not
reach these interactions, so the comparison is the hands-on checks [VERIFICATION.md](VERIFICATION.md) A4-24 to A4-27.
The two deliberate deviations of TW-171 (live canvas preview, no step for an unchanged value) are listed in `[N]`. The
12 rows: open are U-1 rest and the runner item TW-167; TW-138 rest is closed except the unmeasured R3 note; TW-151,
TW-160 to TW-162 and TW-168 to TW-172 wait for the click-test, including the macOS re-test by the tester.

Follow-up run 29 (20 Oct 2026, 06:17–06:43, 3 units, 88 steps, 54 image pairs, preview `2026-10-20_06-08` from `main` at `901f0a56`, runner R-26; see [PARITY.md](PARITY.md)) confirmed and closed
**TW-167**: the reference started `toolbox` with its right panel dock collapsed to icons again (105 px window, set up
on purpose before the run); R-26's unit-start layout check measured it, clicked the dock's expand button once and read
271 px back. After full screen with menus the reference left the dock collapsed again (105 px), and the restore after
the screen-mode reset expanded it (`restored: ['panel dock expanded (105 -> 271 px)']`); the reference width
`dock_ref` is now 276 px (view edge), read only from an expanded dock. `00_overview_normal` is back to 25.5 % (run 28:
64.3 %) and `13_view_modes_standard_normal` to 25.9 % (run 28: 25.6 %), with the dock expanded on both sides.
`tool-path-reshape` (23 / 23) and `xs-select-behind` (9 / 9) read every pair as before. No `runner error`; quotas
and canvas pairs unchanged; all units stay equal except for design. The 11 rows: open is U-1 rest; TW-138 rest is
closed except the unmeasured R3 note; TW-151, TW-160 to TW-162 and TW-168 to TW-172 wait for the click-test,
including the macOS re-test by the tester.

| ID | Area | What is open (expected vs. current) |
|---|---|---|
| TW-138 rest | Text tool defaults, Text menu (*RE spec*, rest) | **Closed** (F49, `439a3ec5`; R3 F52; run 26: `tool-text` and `menu-text` without deviation, quotas and canvas pairs unchanged). The Properties panel (also routed to the open editor) no longer writes the baseline shift default; only the Text tool's own defaults command does (R1). Every font change (Text menu, panel, editor) stores the PostScript name next to the family and puts the family at the front of the recent font list, deduplicated and limited by the preference (R2). Applying a style from the Styles panel to text writes bold/italic/underline, the font, leading and anti-aliasing as text defaults, not the size (R4). R3 measured (RE spec 56, F52): the panel writes indent, spacing before/after, leading (factor, or px with `Text_LeadingMode` 1; switching % to px converts to round(factor · size)), horizontal scale and tracking (`Text_RangeKerning` = field / 100, was the field value) in all three states (nothing selected, pointer selection, open editor) - kept. New rules: activating the Text tool (tool switch, T shortcut, toolbox, switch from another tool) resets indent, spacing, leading (`DefaultLeading`, % mode), tracking, horizontal scale and baseline shift unless `TextEditor_RememberALLSettings` is on; size, style, font and alignment stay, so values set while the Text tool is active survive and values set with the Pointer are lost (R5; the text-tool-only opacity/blend reset is omitted, Firepixel has no such option). New text gets tracking 0 (R6; the reference's first-character kerning is not applied). Every panel field change is its own "Text Change" row, also with nothing selected; an open editor session on existing text ends as one "Text Change" row (R7). Left for the click-test A4-29: for a new text the reference writes two rows ("Text" and "Text Change"), Firepixel one; the leading conversion from px back to % is not measured. |
| TW-151 (FB-01) | Pen tool: click on an existing anchor while drawing (tester feedback, *RE spec* 52 §1) | **Closed** (`c3a9e264`, confirm by click-test A4-15; run 24: no regression in `tool-path-reshape` and `inspector-stroke`, the anchor modes themselves are not measured by the runner). Clicking the last anchor of the path being drawn retracted nothing; in the reference it retracts the anchor's outgoing handle and the next segment leaves the point straight. Now: pressing on the last anchor sets its outgoing handle onto the anchor and keeps the incoming one; the path stays in progress. A drag past 2 screen px pulls the point again as a symmetric curve point (Alt keeps the previous incoming handle, Ctrl moves the point, Shift snaps to 45°). While drawing, an interior curve point becomes a corner, an interior corner point is deleted, a click on a segment inserts a point, the endpoint of another open path joins it; without a path in progress a click on an open path's endpoint continues it. Cursors: pen with a caret (convert) and pen with a ring and stub (join). The hit radius follows the *Snap distance* preference. Known limits: a join across elements appends the other path instead of merging the elements; no drag after a join, a close or an interior press; a double-click still ends the path. |
| TW-160 (FB-04) | macOS: window frozen after minimise/restore (tester feedback, *direct fix*) | **Fixed, pending the Mac check** (`b9f085f4`, only type-checked for macOS; hands-on check A4-19). The minimised flag of the window was never refreshed after the app's own minimise, so the restored window was treated as invisible and stopped painting. Now the stale flag is compared with the real window state, cleared and a new frame requested. |
| TW-161 (FB-05) | macOS: Cmd+Q quit without the save prompt (tester feedback, *direct fix*) | **Fixed, pending the Mac check** (`0352e7c9`, only type-checked for macOS; hands-on check A4-20). The default app menu of the windowing library bound Cmd+Q to an immediate terminate, so unsaved changes were lost. Now the default menu is off and Cmd+Q goes through the Quit action with the save prompt. Since `e11f8d1b` the app has its own macOS app menu (About Firepixel, Services, Hide, Hide Others, Show All and Quit Firepixel with the save prompt); the macOS CI build with it is green. |
| TW-162 (FB-03) | macOS: native title bar (tester feedback, *direct fix*) | **Fixed, pending the Mac check** (`53e08484`, only type-checked for macOS; hands-on check A4-21). On macOS the window uses the system title bar with traffic lights; the app bar keeps the menus but draws no own window buttons, drag area or resize edges. Windows and Linux keep the own chrome. |
| TW-168 (FB-06) | Subselection tool: selection display on vector objects (tester feedback, *direct fix*) | **Fixed, pending the click-test** (`0b71e6ca`; hands-on comparison A4-23, the runner never captures the reference's selection markings). With the Subselection tool a selected path showed the bounding box with corner squares. Now vector objects (paths, rectangle primitives, autoshape children) show the path outline as a 1 px line through the anchors, a hollow 5 × 5 px square on each anchor (filled when selected; the *Show solid points* preference reverses this) and the control handles of every segment with a selected end, without a bounding box. Bitmaps, text and symbol instances keep the box; the Pointer and Select Behind tools are unchanged. Known limits: a whole autoshape or group selected as one element and compound shapes keep the box; the facing handles of the neighbouring points are shown but cannot be dragged. |
| TW-169 (FB-07) | Pen tool: History steps while drawing (tester feedback, *RE spec* 55 §1) | **Fixed, pending the click-test** (F51a `22f62a54`; tests `fb07_pen_history`; hands-on check A4-25). As in the reference, every placed point is now its own History step "Pen": the first point creates the path, further points append. Ctrl+Z while drawing removes only the last point and the rubber band continues from the new last point (curve handles included); undoing the first point removes the path and the Pen stays active. Finishing (double-click, Enter, Escape, tool change, closing) adds no step; the closing click is one step, in which a path started in this run is redrawn closed (its element id changes). Backspace while drawing removes the last point; a single click and then finishing leaves a one-point path. Vector Path ignores undo and redo while the mouse button is held; Line makes one step. Not measured in the reference: finishing with Enter or Escape (assumed: no step). |
| TW-170 (FB-08) | Edit ▸ Redo (tester feedback, *RE spec* 55 §2) | **Fixed, pending the click-test** (F51a `22f62a54`; tests `fb07_pen_history`; hands-on check A4-26). With nothing undone, Redo repeated the last command. Now Redo is redo only: `edit.redo` requires an undone step, Ctrl+Y and Ctrl+Shift+Z do nothing with an empty redo stack, and the Edit menu shows the disabled "Can't Redo", otherwise "Redo: ‹step›". The repeat-last-command fallback (`last_command_label`) is removed; the History panel's Replay button is unrelated and unchanged. No separate Repeat command on Ctrl+Shift+Y: the reference binds that key to Text ▸ Attach to Path. |
| TW-171 (FB-09, FB-10) | Inspector: opacity and stroke width sliders and fields (tester feedback, *RE spec* 55) | **Fixed, pending the click-test** (F51b `0932f67c`; tests `inspector_fb09`; hands-on check A4-24). Measured rule: while the thumb moves the field follows live, the reference's canvas does not; the value is applied on release (press on the arrow and drag) or when the click-opened pop-up closes (click beside it; Esc discards), one History step per apply ("Opacity"; stroke width: one `pi.stroke` step, now named "Stroke" / "Pinselstrich"). The number fields apply on Enter, Tab or a click into the document; a click on empty panel space keeps the focus and applies nothing. Enter in the click-opened pop-up applies (not measured; default). Deliberate deviations (see `[N]`): live canvas preview while dragging; an unchanged value makes no step. The reported "Enter does not apply in the field" could not be reproduced in tests and is re-checked in A4-24. Open: the Tab order from stroke width to the Edge field is unchecked; the field text lags the preview by one frame. |
| TW-172 (FB-11) | Subselection: point drags rounded to whole pixels (tester feedback, *RE spec* 55 §5) | **Fixed, pending the click-test** (F51c `901f0a56`; tests `fb11_subselection_snap`; hands-on check A4-27). Dragging an anchor or handle with the Subselection tool rounded to whole document pixels; the reference moves points by the screen delta divided by the zoom. Now point drags are free (no whole-pixel rounding, one "Move Point" step) and snap only to the grid, guides and smart guides when their switch is on; Pointer-tool point drags share this code path. Pointer object moves and Pen clicks stay whole-pixel. New Firepixel-own toggle View ▸ Snap to Pixels (`view.snapToPixels`, preference `SnapToPixelGrid`, on by default, no shortcut, after the Smart Guides submenu) controls that rounding; the reference has only the one-off Modify ▸ Snap to Pixel (Ctrl+K), which stays. Cursor: the new `Cursor::SubselectPoint` (hollow arrowhead) over a point or handle and while dragging, the plain black arrow otherwise, no segment hover marker. Open: cursor shape and menu placement not yet seen in a real window; smart-guide snapping of anchors to other objects unmeasured. |
| U-1 rest | File ▸ Import, PDF | **Fixed** (U-1b, `feat(import): PDF layers from optional content groups`; tests `crates/fp-io/tests/pdf_layers.rs`). PDF files (and AI ≥ 9) now import one layer per optional content group used on the page: names from the group's `/Name`, nested `/Order` entries as "Parent / Child" (label collections as "Label / Child"), groups in the default `/OFF` (or `/BaseState /OFF`) as hidden layers, content outside any group in "Layer 1"; layers stack in the order the page first paints them, so the look is kept; per page for multi-page files. The page content stream is split with the PDF library's parser (innermost `/OC … BDC` section, `/OC` of a `Do` XObject) and every layer is rendered in its own pass with the other content switched off. PDFs without groups import exactly as before (one layer). Limits: OCMDs (visibility expressions/policies) are not split, their content goes with the enclosing layer and is evaluated with all groups on; groups used only inside form XObjects stay with the layer of the `Do`; annotations go to "Layer 1"; encrypted files and pages with more than 64 groups fall back to one layer; `/Intent`, `/Usage` and `/AS` are ignored. |

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
| BA-083 (BATCH-4) | Batch tests | Golden hashes of the resampler depend on `f64::sin` (Lanczos) and may need regenerating on other operating systems. CI now runs the batch/export tests on macOS and Linux (`macos-preview.yml` / `linux-build.yml`, input `tests`, artifact `tests-macos` / `tests-linux`); first cross-OS run (20.10., Linux 39371722927, macOS 39371720556): all golden hashes equal on macOS and Linux (no regeneration needed). The failures were test assumptions: fp-batch tests built Windows paths (`C:\in\x.png`) as literals, which on Unix are one file name (now platform-native paths via a test helper); fp-script tests needed Myriad Pro metrics (now skip without the font) or any font at all on Linux (fp-text now indexes the Linux font folders, the Linux job installs fonts-liberation/fonts-dejavu-core). **Closed** by the second run (20 Oct 2026, Linux 39374765108 and macOS 39374760635 both green). |
| BA-108 (BATCH-5) | Batch metadata | **Closed** (BA-108b; tests `meta_b4`, `meta::container::ba108_tests`): EXIF and XMP are now written into AVIF/HEIF (`Exif`/`mime` items, `iinf`/`iloc`/`iref`/`ipma` and sequence chunk offsets updated) and JPEG XL (`Exif`/`xml ` boxes, bare codestreams wrapped); IPTC fields travel as XMP ([BATCH.md](BATCH.md) §3.7). Remaining limits: no ICC into AVIF/HEIF/JXL (hint stays); replaced HEIF payload bytes are blanked, not compacted; file-offset items pointing inside `meta` itself are not shifted; JPEG XL output has no batch encoder yet, so its writer runs only on JXL bytes from a registered encoder. |
| BA-109 (BATCH-6) | Batch steps | All steps of the batch spec exist except `ai.upscale` and `ai.faceBlur` (need models). `format.multiPage` only combines; splitting multi-page sources stays with `document.pages`/`anim.toFrames`. Runs started in the Batch Studio UI are not checked by the MCP caller gate (the UI keeps the opt-ins of `script.external`/`script.code`). Minor: `fp_raster::Mask` has no `is_empty()`. |
| BA-110 (BATCH-8) | Batch GIF optimizer | Measured with the new opt-in modes `lzw: flexible` and `perceptualPalette` (off by default; figures in [BATCH.md](BATCH.md) §5.6): screen −54.3 %, cartoon −37.9 % (both modes −38.8 %) vs. the export at SSIM ≥ 0.99; the camera pan reaches SSIM 0.99 in no mode (best 0.986). Done (BA-110 rest): lossy LZW with an SSIM bound (`lossyMinSsim`, off by default; cartoon −25.4 % at q 100 / −17.9 % at q 90 vs. `lzw: flexible` at SSIM ≥ 0.99, screen ±0 %, pan stays loss-free) and cancel inside one encode (per frame, search step and LZW row; partial output discarded), BATCH.md §5.6 points 8–9. |

## [N] Not feasible / informational (20)

| ID | Area | Note |
|---|---|---|
| BA-037 | AI hover preview | Tested with a stub decoder only; not yet tried with real model weights. Works only with a warm cache (model loaded, embedding cached); it never loads a model itself. |
| BA-039 | Brush live preview | Bitmaps with live effects are slower (whole scratch raster, 48 px effect margin); strokes that create a new bitmap or paint into a bitmap inside a symbol use the old overlay above all objects; below 100 % zoom the composite tiles are sampled NEAREST. Frame time on 2000 × 1500 rose from 2.4 to 4.3 ms on average. |
| BA-104 (CMD-1) | Commands menu | Screen capture uses the system's own capture tool, so there is no interactive drag rectangle (the area is entered as numbers before the countdown). The symbol script skips bitmaps, nested instances and compound shapes and writes gradients as solid fills. |
| TW-117 | Comparison runner, colour pop-up | Unchanged in runs 15 and 17. Canvas shots differ by capture method: Firepixel's shot contains the open colour pop-up and the selection handles, the original's does not (6.3 % of pixels above 32 levels); the fill under the hovered swatch is equal. Information only. |
| BA-097 | Performance (no GPU) | PERF-3: the drag preview (`canvas\ghost.rs::drag_sprites`) holds only the part in view plus half a view of margin (re-cut on scroll/zoom/far drags, same pixels), and mixing thousands of small objects no longer pays a thread hand-off each. First drag step with 3200 selected objects (release, software rendering, `select_all_perf`): 5.4 s → 1.7 s (UI 3.8 s → 0.36 s), later steps 470 → 300 ms; corpus iPad.png 1.0 s → 0.28 s. Residue: when the whole selection is in view (zoomed out), its preview still uploads at full size (~1.3 s for 6000 × 1500 px). |
| BA-098 | Performance (no GPU) | PERF-3: the canvas texture is now 512-px tiles on a page-anchored device grid (`Renderer::render_device`): scrolling uploads only new tiles, an edit only changed rectangles, after a zoom the visible tiles come first and the margin follows. 6000 × 1600 page with 3200 objects (release, software rendering): worst frame after zoom 1.0–2.5 s → 0.44–0.77 s, after scrolling one view 1.2–2.4 s → 0.31–0.58 s, settled after 2.1–5.9 s → 1.1 s. Residue: uploading the visible tiles of a 1800 × 1000 view still takes a few hundred ms without a GPU. |
| BA-021 | English UI texts | The English labels of D39 ("Sample radius:") and the English history names of the Select commands were not measured in the original (only the German UI of the original is installed). |
| BA-099 | Performance | PERF-3: Layers panel rows outside the visible list only reserve their place. With 3200 selected objects a frame with panels takes ≈ 112 ms instead of ≈ 281 ms (release, this server; panels hidden ≈ 9 ms). Residue: ≈ 100 ms per frame remain in the panels/inspector with such a selection. |
| BA-114 (FARB-1b A1/A2) | Color picker | Deliberate deviations: own color dialog instead of the system color picker (`ChooseColor` only via `unsafe`); the eyedropper outside the app window shows the working color. All other points of this entry are done. |
| BA-099 | Performance | Done (PERF-3): the selection checks of the menu state stop at the first deciding object without collecting/sorting the selection (`docops::visit_on_page`, `any_selected`); with 3200 selected objects the named commands take 0.2–0.4 ms instead of 4–12 ms, all 641 commands 46 ms instead of 252 ms (slowest 2.9 ms). |
| BA-099 | Performance | Done (PERF-3): moving keeps the held render cache (`docops::translate_at`); re-rasterizing small objects no longer fans out to threads per path (`fp-vector`), so the moved objects are still re-rasterized but the render after a move of 3200 objects takes ≈ 0.26 s instead of ≈ 3.7 s and an arrow-key frame ≈ 350 ms instead of ≈ 820 ms. |
| BA-116 | Text rendering | A CFF test font reaches only ≈ 52–72 % exactly matching ink pixels (different font build on Windows, different CFF hinter than the original). Rotated/skewed text and faux italic stay unhinted; anti-aliasing "Strong" is deliberately unhinted. |
| BA-112 | Native file dialogs | Native file dialogs appear at the top-left corner of the main window (Windows places the Common Item Dialog there even with an owner); the original centres them. Moving them needs Win32 `SetWindowPos`, i.e. `unsafe` or a new dependency – both rejected. |
| BA-113 | Native Open/Import dialog | `rfd` cannot preselect a filter, so "All readable files" comes first instead of 13th. The in-app dialog and `ui_file_dialog` use the original order via `default_filter`. |
| BA-111 | Graphics backend | If DX12 initializes but the window surface fails, there is no fallback to Vulkan/GL in the same start (workaround: `FP_WGPU_ALL_BACKENDS=1` or `WGPU_BACKEND`). Watch on machines with a GPU. |
| BA-114 (F5) | Eyedropper | The eyedropper outside the window still returns the working color instead of the screen color. Reading the screen outside the window needs `unsafe` or an extra dependency (`unsafe_code = "forbid"`). |
| TW-58 | PNG file size | PNG8 is ≈ 5 % and PNG32 ≈ 0.6–1.3 % larger than the original's output. Row filters are identical; the difference comes from the deflate implementation (miniz instead of zlib). Decision: no zlib C backend; kept as information. |
| TW-171 (FB-09, FB-10) | Inspector slider live preview | Deliberate enhancement beyond the reference (RE spec 55): while the opacity or stroke width thumb moves, the canvas shows the value live through a render-only override (`AppState::inspector_preview`, `fp-ui/src/inspector/preview.rs`, `RenderKey.preview`); the reference updates only the field. No History step and no dirty flag; the single real apply happens on release or when the pop-up closes. |
| TW-171 (FB-09, FB-10) | Inspector: unchanged value | Deliberate deviation (RE spec 55): applying an unchanged opacity or stroke width (also one clamped back to the old value) makes no History step; the reference records a step even then. |
| T-3 / T-5 | English UI texts | English step names and messages (`msg-step-add-states`, `msg-step-export-options` "Export Options", `ui-script-*` and other new EN texts) were not measured against the original, because only its German UI is installed (same as BA-021). |
