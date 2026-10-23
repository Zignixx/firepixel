# Manual verification

Last updated: 2026-10-23

This file is the human click-checklist for Firepixel. Most of Firepixel is checked automatically: unit and integration tests, headless UI tests with image series, and a scripted comparison runner that drives Firepixel and the reference application side by side through the automation bridge (`--mcp`) and compares the results. Some things cannot be covered that way and stay **manual**:

- **Embedded panel dialogs of the reference application, owner-drawn lists and menus.** The runner cannot read or click them reliably (File Info, Preview in Browser, Special Characters, the auto shape panel, ...), so the comparison has to be done by hand.
- **Real input.** Physical mouse drags, brush and pen strokes with a real mouse or tablet, real keyboard input (Alt tap, access keys, Ctrl+A, Ctrl+Tab, IME), focus changes and timing. The bridge injects events at a level where focus, hover and modifier state behave differently than with real hardware.
- **Window and OS integration.** Drag and drop between windows, program start and flicker, the taskbar, native file dialogs, Explorer and browser hand-off, clipboard content from other programs, window sizes and DPI scaling.
- **Visual judgement.** Themes (Normal, Dark, OLED), languages (de/en and the pseudo-locale `qps`), clipped text and paired screenshots that need a human eye.
- **External material.** Real phone photos, installed ffmpeg, AI models, a PDF viewer or a second program to open the output.

## How to use this list

- Work top-down: **Part A** holds the priority checks (what the runner cannot reach, the unclear units of comparison run 12, spot checks of the fixes from 2026-10-17). **Part B** is the full catalogue of pending real-window checks, grouped by topic.
- Every item has an ID, the steps and the expected result. Tick the checkbox (`- [x]`) when the item passes.
- In brackets after the ID: the work package or finding that introduced the change (for example `RT-3`, `TW-52`, `CT-C-06`), so the item can be traced to its commit, and what the check needs: *real mouse*, *real keyboard*, *real window* (manual GUI session), *bridge* (could be scripted through the automation bridge, but was skipped in the last runs), *original* (also needs the reference application), *images* (screenshot series in 3 themes × de/en).
- If an item fails, open a GitHub issue ("Bug report" or "Parity gap" template) and quote the ID; the maintainers then add it to `FINDINGS.md`.

Conventions: coordinates are document pixels; "R1", "E1", "B1", "P1", "PG" are the rectangle, ellipse, bitmap, path and polygon objects of the parity test documents; files such as `par_layers.fw.png` or `re5a_t1_rand.png` are parity test documents. In the reference application, always open a **copy** of a test file via its script API instead of creating a new document, and do not restart it yourself.

---

# Part A – Priority checks

## A1. What the runner cannot reach

### Embedded panel dialogs of the reference application

- [ ] **A1-01** (D10; needs: real window, original)
  **Steps:** Open File ▸ File Info in both programs, in de and en. In the original, note the tab names in order, the field labels with type, position and size, the buttons, the initial focus, Esc with a real key and resizing.
  **Expected:** Firepixel shows only the tab "Description", no overflow arrow, fields and footer complete, nothing clipped (en not yet checked). Differences in the original's layout go into a "Parity gap" issue (see also VER-9.10).
- [ ] **A1-02** (MN-4, BA-094; needs: real window, original)
  **Steps:** With an open document, use File ▸ Preview in Browser and Commands ▸ Demo – current document in both programs (shell open allowed, `FP_NO_SHELL_OPEN` not set).
  **Expected:** The default browser opens the preview of the document (all pages for the demo command); compare the generated page with the original's.
- [ ] **A1-03** (CF-C CT-C-11/19/21; needs: real window, original)
  **Steps:** Open Window ▸ Special Characters, Info, Color Mixer and URL in both programs; collapse and close the floating panels; insert a special character into a text object.
  **Expected:** Panels float staggered with a title bar arrow (collapse) and × (close); the character is inserted at the caret; look and behaviour match the original (see VER-4.28).
- [ ] **A1-04** (F23, TW-88/TW-90; needs: real window, original)
  **Steps:** Draw each auto shape (rounded rectangle, arrow, arrow line, L-shape, star, spiral ...) in both programs, open the Auto Shapes and Auto Shape Properties panels, change values; draw with and without Shift.
  **Expected:** Same properties and handle tooltips as the original; Shift squares the same shapes as in the original (see FINDINGS TW-88 rest, VER-1.31, VER-1.32).

### Real keyboard

- [ ] **A1-05** (CF-FP4 FP4-04; needs: real keyboard) – see VER-6.19: tap Alt with the Brush tool active; the menu bar becomes active, the access key opens File, the tool does not change.
- [ ] **A1-06** (NZ-3; needs: real keyboard) – see VER-3.19 and VER-3.20: access keys in Curves, Levels and Image Size, letters always underlined.
- [ ] **A1-07** (F21 TW-78; needs: real keyboard) – see VER-6.20: Ctrl+A in an Inspector field selects the field text only.
- [ ] **A1-08** (CF-B CT-B-03; needs: real keyboard) – see VER-6.17: Ctrl+Tab / Ctrl+Shift+Tab cycle through documents.
- [ ] **A1-09** (CF-C CT-C-06, N11; needs: real keyboard) – see VER-4.24: start page focus frame with Tab and arrow keys.
- [ ] **A1-10** (IME; needs: real keyboard)
  **Steps:** Switch Windows to a Japanese or Chinese IME, type into a text object, an Inspector field and the font search field; confirm and cancel the composition.
  **Expected:** The composition window appears at the caret, committed text lands in the field, Esc cancels only the composition, no shortcut is triggered while composing.

### Brush and pen with a real mouse or tablet

- [ ] **A1-11** (PINSEL-1; needs: real mouse) – see VER-1.06, VER-1.07 and VER-1.09: live brush stroke, large bitmap, Esc during a stroke.
- [ ] **A1-12** (F26; needs: real mouse, original)
  **Steps:** Paint the same test stroke with a simple brush (e.g. "1-Pixel Hard") and a natural brush in both programs, with direction sensitivity and "Size ← Random" set.
  **Expected:** Strokes match the original; known gap: the simple brush ignores direction, spacing and flow (FINDINGS F26).
- [ ] **A1-13** (tablet; needs: tablet)
  **Steps:** Paint with a pen tablet using pressure and speed sensitivities.
  **Expected:** Speed sensitivity works; pressure is not supported (FINDINGS BA-039) – the stroke must still be drawn without errors.
- [ ] **A1-14** (FV-14, RT-1; needs: real mouse) – Pen tool: click and drag Bézier points, Alt-drag handles (see VER-9.02), close a path; snapping of the Pen does not catch objects (VER-1.12).

### Drag and drop between windows

- [ ] **A1-15** (N07; needs: real mouse)
  **Steps:** Drag a PNG from Explorer onto the canvas of an open document; drag one onto the empty start page; drag several files at once.
  **Expected:** Onto a document: the image is inserted as an object at the drop point, no new document; onto the start page: the file opens; several files: each handled the same way.
- [ ] **A1-16** (RT-3 L4; needs: real window) – see VER-1.16: copy an image from a web browser and paste it into Firepixel.
- [ ] **A1-17** (BATCH-9; needs: real mouse)
  **Steps:** Drag files and a folder from Explorer into the Batch Studio file list.
  **Expected:** All supported files are added (folder recursively as configured), unsupported files are reported.
- [ ] **A1-18** (CF-C CT-C-02/03, F19 TW-69; needs: real mouse) – see VER-4.23 and VER-1.36: reorder states/pages and drag across eye icons.

### Text selection in the text editor

- [ ] **A1-19** (TW-146 rest, TW-165; RE specs 49, 50 and 53; needs: real mouse, reference application) – moved from
  A2 on 20 Oct 2026: in runs 26 to 28 the runner's drags and word double-clicks only moved the caret in the reference's
  text editor (24 px instead of about 172 px of highlight), so the selection edits were never measured there.
  **Steps:** In both programs create the text "Abc 123" (Arial 12 pt, black), double-click into it and select only
  "123" with the mouse. In the Properties panel set the size to 72, then a colour (for example #A0A0A0). Leave the
  editor, then create a new text with the Text tool.
  **Expected:** Only "123" changes to 72 pt and the new colour; "Abc " keeps 12 pt and black. The new text takes the
  size 72 pt but not the colour (it stays black). Firepixel matches the reference application; differences go into a
  "Parity gap" issue.

## A2. Open items from comparison runs 12–27

Comparison run 12 left seven units unclear (`?`). Runs 13 and 14 (17 Oct 2026) resolved or reclassified all of
them; they stay listed here, ticked, so the history of each check is clear. Run 15 (17/18 Oct 2026) resolved
TW-97, TW-110, TW-112, TW-114 and TW-116; run 16 (18 Oct 2026) resolved TW-113, TW-118, TW-119, TW-120 and TW-121.
Full run 17 (18 Oct 2026) confirmed TW-115 and TW-123 and the `xs-canvas` pair review; it did not confirm TW-122
(continued as TW-126) and added TW-125 to TW-130. Follow-up run 18 (18 Oct 2026, 10 units) resolved TW-125 (Common
Library menu), TW-126, TW-127 (`15`), TW-128 and TW-129 and added TW-131 to TW-135. The wave after run 18 fixed
TW-131, TW-134, TW-135 and most of TW-133 and changed the runner for TW-132 (R-14); those items are ticked as fixed
and are confirmed by run 19 or, failing that, by the matching A4 check. Run 19 (18 Oct 2026, 6 units) confirmed
TW-125 (States menu), TW-130 / Q-57, TW-131, TW-132 (rest moved to TW-140 / TW-141), TW-135 and TW-133 except for
the stretch shortcut, and added TW-136 to TW-141; TW-134 was not part of run 19. After run 19, TW-136 turned out to
be a capture artefact (struck below), and TW-137, TW-138, TW-139 and TW-141 were fixed. Run 20 (18 Oct 2026, 7 units)
confirmed TW-137, TW-138, TW-139 and TW-141; it could not confirm TW-134 (the reference did not apply the crop),
left TW-140 open (runner R-16) and added TW-142 to TW-144. Run 21 (18 Oct 2026, 7 units) confirmed TW-133, TW-142
and TW-144, did not confirm TW-143 and TW-134, pinned down the mechanism of TW-140 (runner R-19) and added TW-145. Run
22 (18/19 Oct 2026, 7 units, runner R-19) confirmed TW-134 and TW-143, closed TW-140 (the reference-side substitutes
work) and added TW-146 and TW-147 (runner R-20); TW-145 is still not compared. Run 23 (19 Oct 2026, 5 units,
runner R-20, package F46) closed TW-147, reduced TW-146 to a rest and added TW-148 to TW-150; TW-145 is still not
compared. Full run 24 (19 Oct 2026, all 49 units, runner R-21, packages F46 and F47) closed TW-149, TW-150 and TW-152,
decided TW-148 (the reference merges consecutive key nudges; now a Firepixel fix), left TW-145 and TW-146 rest open
and added TW-153 to TW-157. Run 25 (19 Oct 2026, 20 units, runner R-22, package F48) confirmed TW-148, TW-153 to
TW-157 and the TW-149 leftover, decided TW-145 (the next text is 72 pt in both programs), reduced TW-146 rest to one
lost text and added TW-158 and TW-159. Run 26 (19 Oct 2026, 5 units, runner R-23, package F49) confirmed TW-158 and
TW-159 (b, d), confirmed the entry part of TW-146 rest, kept its selection part and TW-159 (c) open as runner items and
added TW-163 and TW-164. Run 27 (19 Oct 2026, 3 units, runner R-24, the same preview as run 26) confirmed TW-159
rest, TW-163 and TW-164, kept TW-146 rest open (no size route reaches the selected "123") and added TW-165 and TW-166.
Run 28 (19 Oct 2026, 2 units, runner R-25, the same preview) confirmed TW-166, kept TW-146 rest and TW-165 open (no
selection try reaches the expected highlight, so no size route runs and no text tool is reset) and added TW-167.
On 20 Oct 2026 TW-146 rest and TW-165 were moved to the hands-on check A1-19: the runner cannot select text in the
reference's editor with the mouse, so they are measurement gaps, not deviations.
Run 29 (20 Oct 2026, 3 units, runner R-26, preview `2026-10-20_06-08`) confirmed TW-167.
Full run 30 (20/21 Oct 2026, 49 units, runner R-26, preview `2026-10-20_19-06`) added TW-173 (real deviation) and
TW-174 (runner) and the questions Q-58 and Q-59 ([FINDINGS.md](FINDINGS.md)).
Follow-up run 31 (21 Oct 2026, 8 units, runner R-27, preview `2026-10-21_06-27`) confirmed TW-173 and TW-189, closed
TW-188 as a runner matter and added TW-190 and TW-191 (runner) and the questions Q-60 and Q-61.
Follow-up run 32 (21 Oct 2026, 3 units, runner R-28, preview `2026-10-21_08-16`) confirmed TW-189 (fixed in F55) and
closed TW-190, TW-191 and Q-60 (runner matters).
On the evening of 21 Oct 2026 runner R-29 (`6945bf79`) fixed TW-174 (fixed reference panel layout per unit). The
product fixes of that evening (TW-186, TW-187, TW-192 to TW-194, Q-61) are click-test items A4-38 to A4-42.
Full run 33 (21/22 Oct 2026, 49 units, runner R-29 with real plain keys in every unit, preview `2026-10-21_21-47`)
confirmed TW-174, TW-193 and TW-194 and TW-192 for the colours, and added TW-195 (real deviation, the Arrow Line's
stroke in `tool-autoshapes`), the runner item TW-196 (the reference's state after a restart in the middle of a unit)
and the question Q-63 (tool stroke attributes carried between steps and units) ([FINDINGS.md](FINDINGS.md)); TW-176,
TW-177, TW-186 and TW-187 are not reached by any step.
In the night of 22 Oct 2026 runner R-30 (`924fbfc7`) fixed the runner item TW-196 (the unit-start reset is repeated
after a reference restart inside a unit, item below) and runner R-31 (`63f1e059`) sets the tool stroke on both sides
at unit start (Q-63, closed); the product fixes of F61 (`132b4941`, `84346fec`: TW-195 and the refined follow rules of
TW-192, RE spec 59) are click-test items A4-43 and A4-44.
Follow-up run 34 (22 Oct 2026, 8 units, runner R-31 with R-30, preview `2026-10-22_05-02`) confirmed TW-195, the
refined follow rules of TW-192, R-31 (the same tool stroke on both sides at unit start) and the runner item TW-196
(item below), and added the runner item TW-197 (the reference's Properties inspector collapsed, item below).
On 22 Oct 2026 tester feedback batch 5 (FB-17 to FB-23, TW-198 to TW-203) was measured in RE spec 60 (RE-28) and
fixed in F62 (`208d0564`), F63 (`bd397c68`), F64 (`8a681109`), F65 (`8c636395`, TW-202) and F66 (`08ef298d`);
the runner does not reach these interactions, so they are click-test items A4-45 to A4-49. FB-17 turned View ▸
Tooltips off by default (F64); TW-203 (a Dither fill edge) waited for the user's decision and was then built as an
extension (F67, A4-50).
Follow-up run 35 (22 Oct 2026, 6 units, runner R-31, preview `2026-10-22_14-27`) changed no step status and no
compared value: it shows F65 only in the Pointer display of a selected rectangle (outline + corner squares), F64 in
the slider pop-up placement and the default-off View ▸ Tooltips, and F66 in the width field's tooltip; the Pen, the
pen cursors, the live sliders, the patterns and the focus return are reached by no step and stay with A4-45 to A4-49.
The runner item TW-197 did not recur (item below).
On 23 Oct 2026 tester feedback batch 6 (FB-24 to FB-30, TW-204 to TW-209) was measured in RE spec 61 (RE-29) and
fixed in F69 (`c9c9a329`, TW-204), F70 (`4657c7cb`, TW-205, TW-206 and the remaining live sliders), F72 (`249f4569`,
TW-208) and F68 (`99114278`, TW-207; a real-window check over the bridge found 0 differing pixels inside a moved
pattern-filled shape, after a scripted move and after a pointer drag); the runner does not reach these interactions,
so they are click-test items A4-51 to A4-55. FB-27 (a blend mode preview while hovering the list) is declined
(TW-209). Follow-up run 36 over the touched units is to follow.
After v0.2.9 (23 Oct 2026) F73 (`2e0fe826`, TW-203 rest: text fills take the Feather and Dither edge kinds like paths,
through `pi.fill` or a script, since the text inspector has no Edge combo), F74 (`b585f6b8`, TW-207 rest: the Scale,
Skew, Distort and Free Transform tools honour the preference "Scale strokes and effects", `XformToolXformsBFE`, for
the fill handles; the Numeric Transform dialog keeps its own `NumericXformXformsBFE`) and F75 (`2150bbf3`, TW-207 rest: the
scripting option `transformAttributes`) followed; none of them was checked in a real window, so they are click-test
items A4-56 and A4-57.
Later on 23 Oct 2026 measurement RE-30 (RE spec 62) re-measured the Q-62 Escape and arrow rules with real keys and
measured gradient and pattern fills on text (Q-64). F76 (`3cf23641`: transform mode Escape leaves a redo step,
the Brush hands over to the Pointer at Escape and finishes the stroke on release, the Marquee and the Lasso end
bitmap mode at Escape while dragging, marquee nudges merge into one row) and F77 (`2b702176`: text gradient and
pattern fills drawn from the fill handles, numeric transform scales them) followed; neither was checked in a real
window, so they are click-test items A4-58 and A4-59.
After v0.2.10 F79 (`7337428e`, TW-215: the Layers panel shows each layer's stored disclosure and its triangle writes
it as one History step; the opacity sliders and the colour pop-up's alpha slider run 2 px per per cent) followed the
findings of runner R-32 and follow-up run 37; it was not checked in a real window, so it is click-test item A4-60.
Later on 23 Oct 2026 measurement RE-31 (RE spec 63) measured the TW-208 rest and TW-214, and F80 (`297a4613`, TW-214:
a fixed frame that becomes point text keeps its top; TW-213 closed as not a deviation), F81 (`6fede0b9`, TW-208 rest:
the Layers Shift range over the document order with locked layers left out, inert rows of locked layers, layer-row
Shift and Ctrl, the anchor rules) and F82 (`3523c4e3`, TW-214 rest: the fixed-frame height, the factory font of
`addNewText(r, false)`, the zero-width rect, the 1 px on the left) followed; none of them was checked in a real
window, so they are click-test items A4-61 and A4-62.
Later on 23 Oct 2026 measurement RE-32 (RE spec 64) measured the current layer (Q-65), and F83 (`5b47a56b`: the
bottom-most unlocked layer current on open, a single selected object makes its layer current, locking the current
layer moves it up, the New Layer button beside a sublayer, one "Layer Change" row for a layer-row click, the Layers
selection circles and tints) followed; it was not checked in a real window, so it is click-test item A4-63.
The open checks below come from runs 17 to 37, tester feedback batch 6, the fixes after v0.2.9, RE-30, F79, RE-31 and
RE-32.

- [x] **TW-100** (`panel-library-styles/03`) – resolved: run 13 measured equal positions at an integer drop point; the unit counts as equal and the half-pixel case is no longer tracked.
- [x] **TW-101** (`xs-file-formats/09`) – resolved in run 13: import by dragging lands on the same pixels 121,120–417,322 in both programs.
- [x] **TW-102** (`export-results/08`) – resolved in run 14: images-only, PNG 8, PNG 24 and PNG 32 exports are pixel-identical to the original's files of the same run.
- [x] **TW-103** (`export-results/07`) – resolved in run 14: the animated GIF has 2 frames with the same delay, loop, disposal and transparency index; only the frame-1 palette differs (TW-114, FINDINGS).
- [x] **TW-104** (`inspector-group-symbol/04`) – resolved in run 14: header row, column divider and rows are present; the remaining table look is TW-110 below.
- [x] **TW-105** (`panel-swatches-mixer/19`) – reclassified in run 14: the swatch probe hits index 5 on both sides (no hit-test offset); the colours differ because of the `.act` import bug **TW-113** (check below) and unequal start sets in the runner (TW-115).
- [x] **TW-106** (`panel-pages-states-history/16`) – reclassified in run 13: the replay offset came from script steps in the original's history (runner effect).
- [x] **TW-97 / Q-50** (`xs-shortcuts/13`) – resolved in run 15: pressing on R1's body, the Alt-drag duplicates in both programs (8 vs. 8 objects).
- [x] **TW-110** (`inspector-group-symbol/04`) – resolved in run 15: the Symbol Properties table has the original's look (light rows, grid, clipped first column).
- [x] **TW-112** (`color-popup/17`) – resolved in run 15: a drag from black to red in the pop-up gives #ff0000 in both programs.
- [x] **TW-114** (`export-results/07`) – resolved in run 15: the animated GIF export is byte-identical to the original's file of the same run.
- [x] **TW-116** (`tool-autoshapes/06, 13`) – resolved in run 15: the runner reaches the size handle and the spiral's `center` handle on both sides.
- [x] **Q-48** (`xs-canvas`) – resolved in run 15: the reference fits the large test document (zoom 0.42833) and the drag steps are measured; both programs leave a 15 px margin when fitting.
- [x] **TW-113 / Q-52** (`panel-swatches-mixer/06–08`) – resolved in run 16: after "replace swatches" with the 16-colour `.act` file and "sort by colour" both programs show the same first row (#ffff00, #f0dcb4, #00ffff, #00ff00, #ff8000, #4080c0, …, #ffffff, #808080) followed by black; "add" appends in both.
- [x] **TW-118 / Q-53** (`xs-shortcuts/13`) – resolved in run 16: a Shift-drag pressed on the selected R1 leaves it at 50,50–150,120 in both programs.
- [x] **TW-119** (`xs-canvas/02, 12`) – resolved in run 16: grid snap ends at 12,20–2412,1620 in both programs; the Ctrl+wheel point lies inside both views and the zoom-centre pair differs by 1.6 % (was 6.3 %).
- [x] **TW-120** (`inspector-group-symbol/11, 12`) – resolved in run 16: with two objects selected both programs show the topmost object's fill and stroke (black fill; red 1 px stroke).
- [x] **TW-121** (`inspector-group-symbol/05`) – resolved in run 16: the button instance shows the symbol name, a disabled text field, "PNG32" and "Add filter…" in both programs.
- [x] **TW-111 / Q-51** (`inspector-group-symbol/04–05`) – resolved without a change: the reference does not bring Properties back to the front by itself either; Firepixel is right and runner step 04b stays.
- [x] **TW-115** (`panel-swatches-mixer/00a, 19, 20`) – resolved in run 17: at the 9 px pitch the probe and the read-back give #efffbf in both programs.
- [x] **TW-122 / Q-54 → TW-126** (`xs-shortcuts/06`) – partly resolved in run 18: with both programs on "Web Standard", Ctrl+Shift+U and Ctrl+Shift+E are equal and Ctrl+Shift+D clones; the four keys that still differ are continued as **TW-133** below.
- [x] **TW-123** (`panel-optimize/13`, panel menus) – resolved in run 17: the Optimize colour-table menu is 282 vs. 270 px (frame included); Pages, History, Swatches and Layers menus are within 5 px of the reference; main menus stay narrower than the reference, none became wider through the 33 px arrow column.
- [x] **xs-canvas pairs** (`xs-canvas/03, 11, 13, 14, 16`) – reviewed in run 17: the fit pair differs by the fit zoom of different view sizes (0.43875 vs. 0.44875), the grid pair by the sub-pixel phase of one-pixel grid lines at 0.42833 (cells and lines line up); no rendering difference, the unit counts as equal.
- [x] **TW-125** (`panel-pages-states-history/07`) – resolved: the Common Library menu in run 18 (183 vs. 179 px), the States menu in run 19 (210 vs. 207 px; run 17: 231 px).
- [x] **TW-126** (`menu-edit/02`, `dialogs-d31-d40/06`, `menu-modify-a/11`) – resolved in run 18: Ctrl+Shift+D clones in both programs; twelve tweened instances on both sides; Firepixel's history shows the clone step.
- [x] **TW-128** (`panel-optimize/19`) – resolved in run 18: both programs report JPEG and GIF as the two slices' export formats.
- [x] **TW-130 / Q-57** (`xs-shortcuts/15_in_text_tx2`) – resolved in run 19: with all text objects read back, both programs end with "Zeile eins / ZeileVr zwei / Zeile drei" (the typed text lands at the click point in line 2). The reference adds a stray "c" after Escape (runner, TW-140) and shows the text live while Firepixel's read-back changes only on commit (TW-139).
- [x] **TW-131** (`panel-library-styles/16`) – fixed after run 18: the URL panel menu has the reference's nine entries (add used URLs to the library, clear unused URLs, add / edit / delete URL, new URL library, import, export, Help) with edit and delete grey without a selected URL. Confirmed in run 19 (nine entries in the reference's order); the "..." labels continue as TW-136, the enabled states are not read on Firepixel's side (TW-141); hands-on check A4-11.
- [x] **TW-133** (`xs-shortcuts/06`) – fixed in part after run 18 (RE spec 48) and completed after run 20 (stretched alignment stores and loads as left); confirmed in run 21: all seven keys equal, Ctrl+Alt+Shift+S leaves TX1 "left" in both programs. "Justified" on point text is pinned by tests only.
- [x] **TW-134** (`xs-preferences/06`) – fixed after run 18: the crop tool, Crop Document and Canvas Size honour *Delete objects when cropping* (`CropDocCropsImages`); on deletes objects entirely outside and clips crossing bitmaps, off changes nothing but the position. Not part of run 19. Run 20: Firepixel crops to 280 × 220 and removes L1, ST and PG (entirely outside), but the reference did not apply the crop (page 800 × 600, all seven paths; runner, TW-140), so the step compares nothing. Run 21: the same with R-17's `crop` action (reference page 800 × 600 after both tries, the second with keyboard focus on the view; TW-140); confirm after runner R-19 or by hands-on check A4-13. Run 22: confirmed – R-19's double-click commits the reference's crop, the page is
280 × 220 and P1, E1, R2, R1 remain in both programs (the option off stays with A4-13).
- [x] **TW-135** (`tool-text/06`) – fixed after run 18: a pointer double-click places the caret at the click point and a character typed at once lands there ("ZeilQe eins"). Run 19: Firepixel reads back "ZeilQe eins" after Escape, as the reference showed in run 18; the reference's double-click opened no editor in run 19 (runner, TW-140); hands-on check A4-12.
- [ ] **Q-49** (`color-popup/05`; needs: real keyboard)
  **Steps:** Select a rectangle, open the fill colour pop-up, type #3366CC into the hex field and press Enter; repeat with "zz12".
  **Expected:** The same result as the reference. The runner cannot prove it: the reference's pop-up closes without applying the text the runner sets; Firepixel gives #3366cc and, for "zz12", #000000.
- [x] **TW-127 / TW-129** (runner R-13) – resolved in run 18: `xs-preferences` sets and reads back its options (`15` equal; `06` continued as TW-134); the reference opens Feather / Save Bitmap Selection, the "add swatches" file dialog and receives the library drop; `xs-canvas/15` selects R1 first. The plain arrow keys in `xs-shortcuts/10` moved to TW-132.
- [x] **TW-132** (runner R-14) – runner changed after run 18: read-back scripts on the reference side record open dialogs first and every cancelled dialog is logged; the history read-back is guarded and retried (`menu-edit/01, 22`); Delete, Ctrl+D and the plain arrow keys are checked for effect and retried once (`menu-edit/09, 20`, `xs-shortcuts/10`); the text tool is left with Escape at the start of a unit (`tool-text`); Firepixel's menu read-back takes the enabled state from the accessibility tree (`xs-canvas/15`). Confirmed in run 19 for the dialogs, the history read-back, Ctrl+D and the text read-backs; Delete and the plain arrow keys select other tools on the reference side and a "c" arrives after Escape (continued as **TW-140** below); Firepixel's menu enabled state is still not read (continued as **TW-141** below).
- [x] ~~**TW-136** (`panel-library-styles/16`)~~ – not a deviation (capture artefact): the reference's labels end in "..." as well, and the 17 px came from the shot including the ≡ button row and the drop shadow; the menu frame is 323 × 231 vs. 321 × 231 px, pinned by a size test.
- [x] **TW-137** (`menu-edit/22`) – fixed after run 19 (RE spec), confirmed in run 20 (both Histories list Clone, Copy, Paste): Copy and Cut are History rows of their own ("Kopie" / "Copy", "Ausschneiden" / "Cut"), Select All always adds a row, Deselect only when the selection changes; Copy does not mark the document as changed.
  **Steps:** Select a rectangle; Clone (Ctrl+Shift+D), Copy (Ctrl+C), Paste (Ctrl+V); look at the History panel.
  **Expected:** As in the reference: Clone, Copy, Paste are listed.
- [x] **TW-138** (`tool-text/11, 13, 14, 16`) – fixed after run 19 (RE spec), confirmed in run 20 (the new text in `14` and `16` keeps 12 pt black; `14` now 0.54 %): attributes set while the editor is open never change the text tool defaults; with pointer-selected text or nothing selected they do. The *Text* menu remnant stays open in FINDINGS (TW-138 rest).
  **Steps:** Text tool: type a text, select three characters and set 72 pt in the inspector; press Escape; click elsewhere and type a new text.
  **Expected:** The new text keeps the tool's earlier size and colour as in the reference.
- [x] **TW-139** (`tool-text/06, 07`, `xs-shortcuts/15_in_text_tx2`) – fixed after run 19, confirmed in run 20 ("Hallo Welt" and "ZeileVr zwei" read back while the editor is open; the bounds remnant is TW-143), still to try with the real keyboard: script read-back shows the live editor text (limits: MCP `get_document` / `render` show the committed state; the live element is not in the selection).
  **Steps:** Text tool: click on an empty canvas and type "Hallo"; without pressing Escape run a script that lists the document's texts; look at the canvas.
  **Expected:** The new text object exists and shows "Hallo" in the read-back and on the canvas while the editor is open, as in the reference.
- [x] **TW-140** (runner R-19) – resolved in run 22: on the reference side Edit ▸ Clear deletes R1 (`menu-edit/09, 22`; both Histories list Clone, Copy, Paste, Clear), `moveSelectionBy` nudges by 1 px with values equal to Firepixel's arrow keys (`xs-shortcuts/10`, `panel-pages-states-history/14–21`), the Pointer tool click closes the text editor without a stray "c", Escape no longer cycles tools in `xs-preferences`, and the double-click commits the crop. Run 21 had shown the mechanism: the reference takes unmodified Delete, Right, Down and Escape as the tool letters E, T, N and C although the keys arrive correctly. Side effects of the substitutes continue as TW-147, text entry as TW-146.
  **Steps:** Runner R-19 on the reference side: Delete via Edit ▸ Clear or `deleteSelection(false)`, nudges via `moveSelectionBy`, leaving the text editor by clicking the Pointer tool, text selection by mouse drag, crop commit by double-click inside the crop box, text colour by script; Escape only for menus and dialogs. Hands-on: press Delete and the Right arrow with the real keyboard on a selected object in the reference, to see whether it behaves the same way.
  **Expected:** `menu-edit/09, 22` delete R1, `xs-shortcuts/10` and `panel-pages-states-history/14–21` nudge by 1 px, `xs-preferences/06` crops the reference's page to 280 × 220, no stray "c", no tool change (all met in run 22; `tool-text/11, 13` continue as TW-146).
- [x] **TW-141** (`xs-canvas/15_snap_pixels_disabled`) – fixed by package F40 and confirmed in run 20: Modify ▸ Snap to Pixel without a selection reads `enabled: false` in both programs. Panel pop-up menus continue as TW-142.
- [x] **TW-142** (`panel-library-styles/16`) – fixed after run 20 (panel pop-up rows expose enabled and toggled state; runner R-18 reads them) and confirmed in run 21: nine entries with equal labels, "..." flags and enabled states; Edit URL and Delete URL `enabled: false` in both programs.
- [x] **TW-143** (`xs-shortcuts/15_in_text_tx2`, `tool-text/06`) – fixed in `d1e79ed5` and package F45 (`a4c2bf2b`: the element list returns the live text element while editing), confirmed in run 22: while "Vr" is typed TX2 reads 50,150–120,194 in both programs; in `tool-text/06` Firepixel reads 114 live (the reference's double-click opened no editor in that run, TW-146).
  **Steps:** Double-click into line 2 of TX2 with the text tool, type "Vr" and read the text objects back through the layer element list before Escape.
  **Expected:** As in the reference: TX2's bounds grow to 50,150–120,194 while the editor is open.
- [x] **TW-144** (`xs-shortcuts/15_in_text_tx2`, `tool-text/06`) – resolved in run 21: with CR/LF kept in the read-backs (runner R-17b) both programs separate TX2's paragraphs with `\r`; the `\n` of run 20 was the reference-side read-back. Tests pin the separators.
- [x] **TW-145** (`tool-text/11, 13, 14, 16`) – decided in run 25: the reference's size field again misses the drag-selected "123" in `11`, but its Text ▸ Size 72 sets the text tool to 72 pt, and its next texts are 72 pt like Firepixel's (`13` text box 100,48–361,101; `14` 0.18 %, `16` 0.27 %; the next unit starts with the reference's tool at 72 pt). The F43 rule gives the same next-text size; earlier runs (21–24) compared nothing because the reference's text or selection failed.
- [x] **TW-146 rest** (`tool-text/07, 11, 13`; runner) – **moved to A1-19** (20 Oct 2026; runner measurement gap, not a deviation). Entry part confirmed in runs 26 to 28 (all reference texts and the edits in `06`, `08` and `10` arrive as typed on the first attempt). Left (run 28, R-25): the area test (at least 60 % of the expected inverted area) rejects the 24 changed pixels as the caret, and no try gives more: five drags and the word double-click over "123" read 24 of 171.7 px (ratio 0.14), so the size field, the keys and the Text Editor route are skipped (`text_size.method` null) and "Abc 123" stays 12 pt; `07` ("Welt", ratio 0.12) and `13` the same, and `13`'s grey does not reach "123". Runs 30 and 31: the entry part is no longer stable; the reference lost the first typed character in `tool-text/03, 04, 05, 20` in run 30 (virtual-key path) and in `02` and `09` in run 31 (scan-code path, R-27; a second try saved `12`), with the Subselection tool active after the first try, so the loss does not depend on the key path. Full run 33: 4 `runner error` lines, the "Welt" selection in `tool-text/07` and the first typed character lost in `08`, `12` and `15`.
  **Steps:** Runner: read the selection by script after the drag, capture the highlight while the view keeps focus, and gate the colour step of `13` like the size routes. Hands-on: in the reference click with the text tool, type "Abc 123", drag over "123" (or double-click it) and set 72 pt.
  **Expected:** The size and colour edits change only "123" on both sides.
- [x] **TW-147** (`xs-shortcuts/07, 21`, `panel-pages-states-history/16, 21, 22`, `menu-edit/03`; runner R-20) – resolved in run 23: the Pointer tool is taken from the flyout when the slot holds Select Behind (`xs-shortcuts/07`, no runner error), Escape keeps R1 selected in both programs (`xs-shortcuts/21`, 1 / 1, F46 after RE spec 51), the paste read-back is R1 at 50,50–150,120 on both sides (`menu-edit/03`) and `MaxUndos` reads 20 in both (`panel-pages-states-history/22`). The nudge rows (c) are now compared with real Shift+arrows and continue as TW-148.
- [x] **TW-148** (`panel-pages-states-history/21`; F48 `0419534c`) – confirmed in run 25: the reference lists the select script for R1, `moveSelectionBy({x:20, y:10})`, the select script for E1 and `moveSelectionBy({x:10, y:0})`; Firepixel lists Move, Deselect, Move (run 24: Move, Move, Move, Deselect, Move). One row per selection with the summed offset in both; the pause question stays with the hands-on check A4-18.
- [x] **TW-149** (runner R-21) – resolved in run 24: Firepixel's text tool is reset at unit start to Arial 12 pt regular black in 46 of 49 units, like the reference (bridge error in three units without new text).
- [x] **TW-150** (`xs-shortcuts/08`; runner R-21) – resolved in run 24: `08_tools` is `ok` in 92.8 s under the limit of 90 s plus 2 s per substitute; the run-start window list has no code-page error.
- [x] **TW-152** (FB-02, `xs-select-behind`) – confirmed in run 24: both programs draw the same 1680 px #E00000 frame on B1 in `02` and `04`, none in `03` and `07`, and the click cycle reads B1, then FR. The rest continues as TW-157 and A4-17.
- [x] **TW-153** (`tool-autoshapes`; runner R-22) – confirmed in run 25: the reference draws with fill #ffffff and stroke #000000 again, `tool-autoshapes` 26 / 27 canvas pairs within 1 % (run 24: 7 / 27; the one pair above, `05` at 1.01 %, was 0.97 % in run 17). One unit start (`menu-text`) still left the reference's text size at 24 pt (TW-159).
- [x] **TW-154** (`menu-text/05`, `menu-view/20`; runner R-22) – confirmed in run 25: the reference side is `unsupported` with the reason (no menu entry, no key in the "Web Standard" set), Firepixel opens the Text Editor and Edit Guides by command and both shots are present.
- [x] **TW-155** (`inspector-stroke/01`, `menu-edit/01, 22`; runner R-22) – confirmed in run 25: `inspector-stroke/01` hovers instead of using arrows (no runner error); with one document open the reference's History reads the script row plus Clone (`01`) and Clone, Copy, Paste, Delete (`22`), equal to Firepixel's rows. `tool-path-reshape/04` continues as TW-159 (the real Up switches the reference's tool).
- [x] **TW-156** (`toolbox/13`; runner R-22) – confirmed in run 25: every screen-mode step starts and ends in Standard mode on both sides and `13_view_modes_standard_normal` is `ok`. The compared F cycle shows a real difference in full screen mode (TW-158).
- [x] **TW-157** (`xs-select-behind/05, 06, 08`; F48 `ace7873c`) – confirmed in run 25: on FR with nothing selected both programs draw 680 px of #E00000 in 398,198–602,302 (run 24: Firepixel 100 px plus an anti-aliased line); right after a click neither shows a #E00000 frame (run 24: Firepixel 100 and 1680 px). The blue-and-red question stays with A4-17.
- [x] **TW-158** (`toolbox/13_view_modes_full_normal`; *RE spec*, F49 `8a9cf670`) – confirmed in run 26: after the second F both programs show only the canvas, centred on black, without menu bar, document tabs, panels and status bar (1.97 %, run 25: 77.1 %); after the first F both show menus and panels; Firepixel's read-back gives full_menus, full and standard. The hands-on check A4-22 (Tab, Escape, the third F) stays.
- [x] **TW-159** (`toolbox/13`, `xs-shortcuts/10_nudge_point`, `menu-text` unit start; runner R-23 `5f498cd7`) – confirmed in run 26 except (c): `xs-shortcuts/10_nudge_point_result` is `ok` on both sides and the first node reads 201,210 in both; the reference's text reset in `menu-text` reads 72 pt before and 12 pt after; both full screen modes are read on both sides (the reference's Standard read-back continues as TW-163).
- [x] **TW-159 rest** (`tool-path-reshape/04_freeform_size_keys`; runner R-24 `f7d13d61`) – confirmed in run 27: with the mouse held the reference's three Up and two ] keys keep the Freeform tool, the size read while the button is held is 72 in both programs, and the shot pair is 0.46 %.
  **Expected:** `04` reads the Freeform size after the keys on both sides (Properties panel shot or script value).
- [x] **TW-163** (`toolbox/13`; runner R-24 `f7d13d61`) – confirmed in run 27: the reference's Standard mode is read from the window against the monitor, the Standard resets from Standard and from full screen with menus succeed and `13_view_modes_standard_normal` reads Standard on both sides (one `runner error` instead of six; the rest continues as TW-166).
  **Expected:** The Standard read-back reads "standard" from the window against the monitor; no `runner error` line in `toolbox/13`.
- [x] **TW-164** (`tool-path-reshape/12_inspector_heads_freeform_normal`; runner R-24 `f7d13d61`) – confirmed in run 27: Freeform is chosen through flyout entry 0 and read back, and the reference's shot shows the Freeform head, pixel-identical to its `01` shot (9.7 %, run 26: 10.8 % with the Path Scrubber head).
  **Expected:** Both sides show the Freeform head (size, Pressure, Preview), as in `01`.
- [x] **TW-165** (`tool-text/12–16`; runner) – **moved to A1-19** (20 Oct 2026; derived from the selection step of TW-146 rest, a runner measurement gap, not a deviation). Run 28: R-25's two-sided reset is armed only by a failed field or menu route; with every size route skipped (TW-146 rest) neither text tool is reset, so Firepixel's texts in `14` and `16` read 72 pt (F43) against the reference's 12 pt, and `14_bounds` (1.10 %) and `16_overflow` (3.82 %) stay above 1 % (run 26: 0.19 % and 0.27 %).
  **Expected:** Both text tools are reset whenever the size does not reach the reference's selection, skipped routes included; `14` and `16` back within 1 %.
- [x] **TW-166** (`toolbox/13_view_modes_full_normal`; runner R-25 `74d6bfc8`) – confirmed in run 28: the reference leaves full screen by F (F, F from full screen with menus, one F from full screen), each Standard reset reads Standard from the window (1600 × 1000, not covering the monitor), and `toolbox` has no `runner error` (run 27: one). The panel dock part continues as TW-167.
  **Expected:** The runner leaves full screen by key before the button; no `runner error` line in `toolbox/13`.
- [x] **TW-167** (`toolbox/00, 13`; runner R-26) – confirmed in run 29: the reference started `toolbox` with its dock collapsed to icons (105 px); the unit-start layout check expanded it (one click on the dock's expand button, 271 px read back) and logged it, and after full screen with menus, which leaves the dock collapsed again, the restore expanded it a second time (`restored: ['panel dock expanded (105 -> 271 px)']`, `dock_ref` 276 from an expanded dock). `00_overview_normal` 25.5 % and `13_view_modes_standard_normal` 25.9 % (run 28: 64.3 % and 25.6 %), both with the dock expanded.
  **Expected:** The unit starts and ends with the expanded default dock on the reference side; the dock width after `13` equals the one at unit start.
- [x] **TW-173** (`menu-edit/02_redo_last_result`; fixed in F53a `d1bb1d87`) – confirmed in run 31: three R1 in both programs, the Edit menu reads the enabled "Wiederholen: Klonen" in both, and Ctrl+Y right after a redo still adds nothing (`xs-shortcuts/02, 17`). Run 30: select R1 in T04, Ctrl+Shift+D, Ctrl+Y: the reference ends with three R1 (every run since run 1), Firepixel with two since F51a (TW-170). Ctrl+Y right after a redo adds nothing in either program (`xs-shortcuts/02`).
  **Expected:** Ctrl+Y with an empty redo stack repeats a repeatable last command (here the Clone) as in the reference; the redo-only cases of TW-170 stay equal.
- [x] **TW-174** (`panel-layers/04`, `panel-swatches-mixer/19`; runner R-29 `6945bf79`) – confirmed in full run 33: `04` hides both layers and `19` reads #efffbf on both sides, and the panel pairs are back at the run 24/25 values (Layers 55.6–67.8 %, Pages and States 44.4–55.6 %); 20 of 21 flyouts opened at the wanted size and one was dragged back. After a reference restart in the middle of a unit the dock comes back collapsed until the next unit start (TW-196, runner). Fixed by R-29: the reference's right dock is set per unit (icon mode for the panel units, flyouts dragged back to the measured sizes); verified in a test: `04` hides both layers and `19` reads #efffbf on both sides. Run 30: the reference's docked panels had other sizes than in runs 24 and 25 (Layers panel shot 270 × 113 px, was 245 × 389), so its eye drag left two layers visible and swatch 5 read #ffffbf instead of #efffbf; Firepixel unchanged.
  **Expected:** The reference starts each panel unit with a fixed panel layout; both steps read the same rows as in run 25.
- [x] **TW-189** (`xs-shortcuts/22_enter_canvas_result`; fixed in F55 `f543d7b9`) – confirmed in run 32: Enter keeps the group selected in both programs (`[object Group]` on both sides). R-27 test run and run 31: with the group G of T06 selected, Enter keeps the group selected in the reference (`[object Group]`); Firepixel reads no selection.
  **Expected:** Enter on the canvas leaves the selection unchanged; both read `[object Group]`.
- [x] **TW-190** (canvas pairs of all units, `toolbox/02_flyouts_*`; runner R-28 `4e1a35af`) – closed by run 32: View ▸ Tooltips is off in Firepixel at each unit start, its canvas shots are pixel-identical to run 30 and its flyout shots carry no tooltip. Run 31: Firepixel's shots carry the View ▸ Tooltips coordinate tag (on by default from TW-184 until F64 `8a681109`, off by default since then, FB-17) and the flyout description tooltip; the reference's tag is switched off.
  **Expected:** View ▸ Tooltips in the same state on both sides at unit start; flyout shots without the description tooltip; Firepixel's canvas shots pixel-identical to run 30.
- [x] **TW-191** (`xs-shortcuts/02_edit_result`; runner R-28 `4e1a35af`) – closed by run 32: no re-selection on either side; the Delete removes nothing and both documents keep 9 elements. Run 31: the reference's key check re-selected R1 before the final Delete and deleted the three R1; Firepixel's Delete with nothing selected did nothing.
  **Expected:** the key is pressed as scripted on both sides; both documents keep 9 elements.
- [x] **TW-196** (`preview-options/06`, `toolbox/00`, `xs-canvas/01` to `22`, `xs-windows-tabs/03`; runner R-30 `924fbfc7`, with R-31 `63f1e059`) – confirmed in follow-up run 34: the one reference restart (a script access violation while `toolbox` reopened its test file before `05_letter_cycle_o_result`) was followed by four "orig restart in toolbox before 05_letter_cycle_o_result" lines, the reopened file and the unit-start state (`MaxUndos` 20, dock 105 → 271 px, default colours #808080 / #c04040 → white / black, text tool, tool stroke 2 → 1 px); the later `toolbox` window pairs read as in runs 32 and 33; `xs-canvas` and `xs-windows-tabs` had no restart and are back at their run 30/31 values (`xs-canvas/04, 05, 17, 21, 22` 26.0–40.0 %, `xs-windows-tabs/03` 26.4 %); `toolbox/00` (49.8 %, before the restart) is raised by TW-197. Fixed in the runner: after a reference restart inside a unit the runner repeats the unit-start reset (layout and dock mode, flyout sizes, `MaxUndos`, default colours, text tool reset, the R-31 tool stroke) and reopens the interrupted test file; log line "orig restart in <unit> before <step>: unit-start reset repeated"; `--dry-run` self check `R-30`. Run 33: the four restarts after a script access violation all happened while the test file was being opened; the steps were `ok` but shot in the stored state (dock collapsed to icons, tool colours #808080 / #c04040, `MaxUndos` 3): `toolbox/00_overview_normal` 64.4 %, the `xs-canvas` window pairs 42.9–65.1 %, `xs-windows-tabs/03_tab_overflow` 64.9 %.
  **Expected:** every restart inside a unit is followed by the log line and the unit-start state; the reference's dock is expanded, its tool colours are the default colours and `MaxUndos` reads 20 in the following steps; the window pairs are back at the run 31/32 values.
- [x] **TW-197** (`tool-autoshapes/07`, `tool-path-reshape/01, 12`, `tool-web/13`, the overview pairs, `toolbox/00` to `05`; runner R-32 `f05236b5`) – closed by follow-up run 37: in follow-up run 34 the reference's Properties inspector was collapsed to its tab row from the run start until the restart in `toolbox` (shots 1256 × 56 px instead of 1256 × 125, the document view 69 px taller); inspector pairs 60.2–65.5 % (run 33: 9.7–29.9 %), `toolbox/00` 49.8 % (run 32: 25.2 %), the reference's fit zoom in `toolbox` 47.1 % against Firepixel's 44.9 %, its page in `menu-modify-a` 34 px lower; no compared value and no canvas pair crossed 1 %. Run 35: not recurred; the inspector was expanded in all six units (shots 1256 × 125 px, page top 183), `tool-path-reshape/01, 12` 9.7–10.6 %, `toolbox/00` 25.9 %; still open until the runner sets the inspector itself. Fixed by runner R-32 (`f05236b5`) and confirmed by follow-up run 37: the inspector read 125 px at all nine unit starts and at the run end, logged; the restore path was verified in R-32's probe.
  **Expected:** the runner sets the reference's Properties inspector to its expanded height at unit start and after a restart and logs it; the inspector pairs are back at the run 33 values.
- [x] **Q-60** (`xs-preferences/20_tab_focus_focus`; runner R-28, F55 UI test `q60_pref_category_click`) – closed by run 32 as a runner matter: the runner's click had hit the page heading; with the list row the Down moves both lists to "Bearbeitung". Run 31: after a click on "Allgemein" the real Down moves the reference's category list to "Bearbeitung"; Firepixel stays on "Allgemein" without a focus frame.
  **Expected:** after a click on a category, Down selects the next category in both programs (check by UI test and by hand).

Questions answered after run 16: **Q-49** (the colour pop-up HEX field now parses every keystroke like the
reference), **Q-51** (see TW-111 above), **Q-55** (`xs-shortcuts/15`: Firepixel's text-tool click was already right;
the runner clicked a different point, the step now clicks document 62,62), **Q-56** (`xs-canvas/15`: Modify-menu
enable rules implemented and pinned by tests; Snap to Pixel is grey without a path object; Arrange is grey in bitmap
edit mode). Run 17 confirmed Q-51 and Q-56 (Firepixel `ok`; the reference greys Snap to Pixel without a selection),
continued Q-55 as TW-130 / Q-57 and left Q-49 to the real keyboard (see above). Grid snapping at low zoom stays in
VER-10.28.

## A3. Spot checks of the fixes from 2026-10-17

- [ ] **A3-01** (T-10; needs: real keyboard) – see VER-2.19: hex field in the color pop-up without focus does nothing; invalid hex "zz12" gives #000000.
- [ ] **A3-02** (T-10, Q-44; needs: real mouse)
  **Steps:** Layers panel: drag a layer by its name onto a locked layer; drag the web layer; drag near the top and bottom edge of a row.
  **Expected:** Dropping into the locked layer is rejected, the web layer cannot be moved, drop zones top/bottom are 6 px, dragging only starts from the name.
- [ ] **A3-03** (T-10, TW-98; needs: real mouse) – grid snapping: View ▸ Grid ▸ Snap to Grid, zoom ≈ 45 %, drag an object by ≈ 13/17 document px; tolerance `SnapDistance` 5 / zoom.
- [ ] **A3-04** (CF-FP4 FP4-01/02; needs: real mouse) – see VER-1.34: gradient preview line with the Fill tool, handle line with the Pointer.
- [ ] **A3-05** (CF-FP4 FP4-03, L-3; needs: real window) – see VER-1.35: double-click an instance edits the symbol in place.
- [ ] **A3-06** (CF-FP4; needs: real window) – see VER-4.33: mixer arrow slider and the "Current/All" prompt titled "Firepixel".
- [ ] **A3-07** (F24; needs: real window)
  **Steps:** Select a rectangle, Styles ▸ New Style; repeat with text in the selection; in de and en. Hover an auto shape handle.
  **Expected:** Name "Style" preselected and selected, text checkboxes off (on with text), thumbnail on the left, two checkbox columns, OK/Cancel bottom right, nothing clipped in **en** (de already checked). Hover: small grey box below left of the handle (TW-96).
- [ ] **A3-08** (F25; needs: real window) – see VER-2.20: opacity stored as 8-bit, field shows whole percent; also save and reopen an edited text and compare its run properties (FINDINGS F25).
- [ ] **A3-09** (F26; needs: real mouse) – see A1-12 and A1-05: brush runner and Alt-drag duplicate.
- [ ] **A3-10** (L-3; needs: real window)
  **Steps:** Convert a rectangle 0,0–50,50 to a symbol, edit it; edit it from the Library on a 400 × 300 document; open an older Firepixel file containing a symbol.
  **Expected:** Instance stays at 0,0–50,50, content at −25,−25, 50 × 50; from the Library 175,125–225,175; the older file shows the same position with the reference point in the centre (not yet checked).
- [ ] **A3-11** (D10; needs: real window) – see A1-01, especially the English UI.
- [ ] **A3-12** (U-1; needs: real window)
  **Steps:** File ▸ Import an SVG with groups and layers, a single-layer SVG, a PDF and a vector test file; use the "Vector File Options" (scale 200 %, page choice, "Convert layers to states").
  **Expected:** Import succeeds and scale works (see VER-3.32); known gaps: no layers from SVG/PDF, options without effect (FINDINGS U-1) – note anything else.

## A4. Wave-4 features and later fixes to try

- [ ] **A4-01** (BA-103; needs: real window)
  **Steps:** File ▸ HTML Setup, change a few values (e.g. extension, style), click *Set Defaults*, OK; File ▸ New, open HTML Setup again.
  **Expected:** A confirmation message appears; the new document shows the stored values; an older document with its own HTML settings keeps them.
- [ ] **A4-02** (BA-103; needs: real window, browser)
  **Steps:** Sliced document, HTML Setup: tick "Write page styles to an external file (styles.css)", export HTML and images; open the page.
  **Expected:** `styles.css` lies next to the HTML and is linked; the HTML has no `<style>` block; the page looks the same as without the option.
- [ ] **A4-03** (BA-103; needs: real window, browser)
  **Steps:** Document with a button symbol (up/over/down states), HTML style *Generic HTML5*; export and open in a browser; repeat with HTML and XHTML.
  **Expected:** HTML5 doctype and `<meta charset>`; hover and press swap the state images in every style.
- [ ] **A4-04** (BA-104; needs: real window)
  **Steps:** Commands ▸ Screenshot: once the whole screen, once an area entered as numbers, each with a countdown of 3 s.
  **Expected:** After the countdown a new untitled document opens with the screen content or the chosen area.
- [ ] **A4-05** (BA-104; needs: real window)
  **Steps:** Select an instance of a graphic symbol with paths, texts and a gradient fill; Commands ▸ Create Symbol Script; save the script; run it in a new document (Commands ▸ Run Script).
  **Expected:** A save dialog offers the script; running it creates a graphic symbol of the same name with the paths and texts; the gradient arrives as a solid fill (known limit).
- [ ] **A4-06** (BA-104; needs: real window)
  **Steps:** Open two documents with different colours; Commands ▸ Web ▸ Shared Palette, save as `.act`; load the `.act` in the Swatches panel.
  **Expected:** The Swatches panel shows one palette with colours from both documents; the `.act` file loads back with the same colours.
- [ ] **A4-07** (BA-107; needs: real window)
  **Steps:** Edit ▸ Preferences ▸ Plug-ins: set "ffmpeg for videos" to an ffmpeg file, OK; open the Batch Studio settings.
  **Expected:** The Batch Studio shows the same path; clearing it in either place clears both (empty = search automatically).
- [ ] **A4-08** (BA-037; needs: real mouse, AI model)
  **Steps:** Edit ▸ Preferences ▸ Edit: tick "Show selection preview while hovering". Object Selection tool on a photo; click once (model loads), then rest the pointer over another object; then click on that same spot.
  **Expected:** After a short rest a dashed outline of the object appears; moving the pointer removes it; the click selects at once with the same outline. With the option off, no outline appears.
- [ ] **A4-09** (BA-039; needs: real mouse)
  **Steps:** Bitmap with opacity 50 %, blend mode Multiply and a drop shadow, partly covered by a rectangle above it; paint across it with the Brush, then with the Eraser and the Rubber Stamp.
  **Expected:** While painting, the stroke is hidden under the rectangle and already shows the opacity, blend mode and shadow; on release nothing jumps.
- [ ] **A4-10** (BA-039; needs: real mouse)
  **Steps:** Repeat A4-09 at 50 % and 200 % zoom, on a 2000 × 1500 bitmap, and with a bitmap inside a symbol being edited.
  **Expected:** Painting stays smooth; at 50 % the preview may look slightly blocky until release; the bitmap inside a symbol is previewed on top until release (known limit).
- [ ] **A4-11** (TW-131; needs: real window)
  **Steps:** Document with two slices with URLs and one hotspot. URL panel ≡ menu without and with a selected URL; *Add Used URLs to Library*; *Add URL...*, *Edit URL...*, *Delete URL*; *New URL Library...*; *Export URLs...* as `.htm` and `.txt`; *Clear Unused URLs*; *Import URLs...* of both files.
  **Expected:** Nine entries in the reference's order, Edit and Delete grey without a selected URL; the used URLs appear in the list; the new library shows in the panel's library list and survives a restart; both exported files load back with the same URLs.
- [ ] **A4-12** (TW-135; needs: real mouse and keyboard)
  **Steps:** Pointer tool: double-click inside the word "Zeile" of a multi-line text and type "Q" at once, then a few more characters quickly; repeat at the start and end of a line.
  **Expected:** The editor opens with the caret at the click point; every typed character appears there in order ("ZeilQe eins"), none is lost and the caret does not jump.
- [ ] **A4-13** (TW-134; needs: real window)
  **Steps:** Edit ▸ Preferences ▸ Edit: switch *Delete objects when cropping* off. Vector test file with paths partly and fully outside 40,40–320,260 plus a bitmap crossing the edge; crop with the crop tool, undo, then Crop Document and Modify ▸ Canvas ▸ Canvas Size; switch the option on and repeat.
  **Expected:** Off: all objects stay, the bitmap is not clipped, everything only moves with the canvas origin. On: objects fully outside are deleted on all layers and states, the crossing bitmap is clipped, crossing vector paths stay whole.
- [ ] **A4-14** (Linux build; needs: real Linux desktop, X11 and Wayland)
  **Steps:** On Ubuntu 22.04 or later (or Debian 12): `chmod +x Firepixel-x86_64.AppImage` and start it; unpack the tar.gz and start `./firepixel`; open, save and export a file; start `./firepixel-cli --help`.
  **Expected:** The window opens under X11 and Wayland with the GPU renderer; open and save dialogs appear (needs `xdg-desktop-portal`); missing runtime libraries are reported clearly; no OpenSSL is needed.
- [ ] **A4-15** (TW-151 / FB-01, tester feedback; needs: real mouse)
  **Steps:** Pen tool: click, then drag out a curve point, then click once more on that last anchor (no drag) and click a new point; undo, press on the last anchor again and drag (also with Alt, Ctrl, Shift held). While drawing, click an interior curve point, an interior corner point and a segment; move over the endpoint of another open path and click it; end the path, then click an endpoint of an open path with no path in progress. Repeat at 200 % zoom and with a changed *Snap distance* preference.
  **Expected:** The click on the last anchor retracts only its outgoing handle and the path stays in progress: the next segment leaves the point straight, the incoming curve is unchanged. A drag pulls the point again as a symmetric curve point (Alt keeps the previous incoming handle, Ctrl moves the point, Shift snaps to 45°). The interior curve point becomes a corner, the interior corner point is deleted, the segment click inserts a point, the other path is joined, the open path is continued. The cursor shows the pen with "^" over the last anchor and interior curve points and the pen with a ring and stub ("o-") over another path's endpoint; the hit radius follows the snap distance. Known limits: a join across elements appends the other path; no drag after a join, a close or an interior press; a double-click still ends the path.
- [ ] **A4-16** (TW-152 / FB-02, tester feedback; needs: real mouse, original for comparison)
  **Steps:** A rectangle partly over a bitmap. Select the rectangle with the Pointer tool, switch to Select Behind (Pointer slot flyout); rest the pointer over the overlap, over the rectangle only and over the bitmap only; click twice on the overlap. Then Edit ▸ Deselect and rest the pointer over the overlap again, in the release or preview build too (not only in a debug build). Compare with the Pointer tool over the overlap.
  **Expected:** Over the overlap the bitmap behind the selected rectangle gets the red hover frame (#E00000, 1 px on the bounds, filled 5 × 5 px squares on the corners); over the rectangle only no frame; over the bitmap only the bitmap's frame; clicks cycle rectangle → bitmap → rectangle. With nothing selected the topmost object (the rectangle) gets the frame – one preview build showed no frame in this case, which the tests do not reproduce, so check it explicitly. The Pointer tool shows no frame over the selected rectangle.
- [ ] **A4-17** (TW-152 / FB-02 open item; needs: real window, original, screenshot)
  **Steps:** In the reference application and in Firepixel: select the rectangle of A4-16, Select Behind, rest the pointer over the overlap; take a screenshot with the system's screenshot tool (not the runner's capture).
  **Expected:** Note whether the reference draws its blue selection frame of the rectangle together with the red hover frame of the bitmap (the runner's shots never show its selection outline). Firepixel draws both; if the reference hides the selection outline while the hover frame is shown, report it as a parity gap quoting TW-152.
- [ ] **A4-18** (TW-148, package F48; needs: real keyboard, original for comparison)
  **Steps:** In both programs: select a rectangle with the Pointer tool and press Shift+Right, Shift+Right, Shift+Down; then plain Right twice; then click another object and press Right. Repeat on a fresh selection with a pause of several seconds between two presses, once with an Undo between two presses, and once with Alt+Right three times. Look at the History panel after each part.
  **Expected:** The three Shift presses and the following plain presses on the first object form one Move row with the summed offset (here 22,10); the press after the selection change starts a new row; an Undo between presses ends the merge. Alt+Right: the first press creates the copy (its own row), the two further presses move the copy and merge into that row. Note whether a long pause starts a new row in the reference (Firepixel has no pause timeout); if it does, report it quoting TW-148.
- [ ] **A4-19** (FB-04, tester feedback; needs: real Mac)
  **Steps:** Open a document; minimise the window with the yellow traffic light; restore it by clicking the Dock icon, then again by clicking the minimised window in the Dock. Repeat with `--mcp` and while a batch runs.
  **Expected:** After each restore the window repaints at once and reacts to clicks, keys and menus; no force-quit is needed.
- [ ] **A4-20** (FB-05, tester feedback; needs: real Mac)
  **Steps:** Change a document without saving; press Cmd+Q, also while a text field or the text tool is active; choose *Quit Firepixel* in the application menu (menu bar, Firepixel menu). Check the other items of that menu: About Firepixel, Services, Hide Firepixel (Cmd+H), Hide Others, Show All; also after switching the language. Finally use *Quit* from the Dock icon's context menu.
  **Expected:** Cmd+Q and *Quit Firepixel* show the unsaved-changes prompt (Cancel keeps the app open, No quits); About opens Firepixel's About dialog; Hide/Hide Others/Show All and Services work like in other Mac apps; the labels follow the UI language. Known limit: *Quit* from the Dock menu still quits at once without asking (it goes through `terminate:`, which has no hook without Objective-C calls); note it if it happens.
- [ ] **A4-21** (FB-03, tester feedback; needs: real Mac)
  **Steps:** Start Firepixel; look at the top of the window; use the three traffic lights, drag the title bar, double-click it, resize at the edges; enter and leave full screen.
  **Expected:** Native title bar with traffic lights and no own minimise/maximise/close buttons at the top right; Firepixel's menu bar sits directly below the title bar without an extra empty strip; moving, zoom and resizing work through the system frame. Quit while maximised and restart: the window opens maximised at the same place, the title bar not cut off at the top of the screen.
- [ ] **A4-22** (TW-158, package F49; needs: real window, original for comparison)
  **Steps:** Without a document press F; open a document and press F three times, also with the screen-mode buttons of the Tools panel; in each mode press Tab and Escape and right-click the canvas; in full screen use *Exit Full Screen Mode* from the canvas context menu; enter full screen again and close the last document.
  **Expected:** Without a document F does nothing. F cycles Standard → Full Screen with Menus (menu bar and toolbar, floating Tools panel at the left, panels as an icon column at the right, no document tabs) → Full Screen (only the canvas, centred on black with a thin grey top/left edge, no bars, panels, tabs, rulers or scroll bars) → Standard. Escape does not leave a mode; the context menu entry returns to Standard; closing the last document returns to Standard. Compare each mode with the reference and note any difference quoting TW-158.
- [ ] **A4-23** (TW-168 / FB-06, tester feedback; needs: real window, original for comparison)
  **Steps:** In both programs draw a pen path with curve points, a rectangle, an autoshape, a bitmap and a text. Click each with the Subselection tool; select single anchors and Shift-click a second one; drag a handle of a selected point; turn *Show solid points* on in the preferences and repeat. Select a group and a compound shape as a whole. Compare with the Pointer and Select Behind tools. Take screenshots with the system's screenshot tool (the runner never captures the reference's selection markings).
  **Expected:** On paths, rectangles and autoshape children no bounding box: a 1 px outline along the path, a hollow 5 × 5 px square on each anchor (filled when selected, reversed with *Show solid points*) and the handles of every segment with a selected end. Bitmaps, text and symbol instances keep the box; Pointer and Select Behind are unchanged. Known limits: a group or autoshape selected as one element and compound shapes keep the box; the facing handles of the neighbouring points are shown but cannot be dragged. Note every difference from the reference (colours, square size, which handles appear) quoting TW-168.
- [ ] **A4-24** (TW-171 / FB-09, FB-10, tester feedback; includes the re-check of "Enter in the opacity field"; needs: real window, original for comparison)
  **Steps:** Select a rectangle with a stroke. Opacity: press on the ▾ arrow and drag the thumb, release; click the arrow, move the thumb, click beside the pop-up; open it again, move the thumb, press Esc; open it again, move the thumb, press Enter. Type a value into the opacity field and press Enter (the tester's report), then Tab, then click into the document; type a value and click empty panel space. Apply the current value again. Repeat with the stroke width, and press Tab from the stroke width field. Watch the canvas, the field and the History panel each time.
  **Expected:** While the thumb moves the field follows and the canvas shows a live preview (Firepixel only; the reference leaves the canvas unchanged). Release, a click beside the pop-up and Enter apply exactly one step ("Opacity", or "Stroke" for the width); Esc discards without a step. In the field Enter, Tab and a click into the document apply; a click on empty panel space applies nothing and keeps the focus. An unchanged value makes no step (the reference records one). After Esc or applying, no preview remains and the document is dirty only after a real apply. Note where Tab goes from the stroke width (the Edge field expected) and any visible lag of the field text, quoting TW-171.
- [ ] **A4-25** (TW-169 / FB-07, tester feedback; needs: real window, original for comparison)
  **Steps:** In both programs draw a rectangle, then with the Pen click three points and watch the History panel; press Ctrl+Z twice while drawing and click again; draw a curve point by dragging, place another point and press Ctrl+Z; press Backspace while drawing; undo back to the first point and once more; close a path on its start point; finish paths with a double-click, Enter, Escape and a tool change; click once and finish. Draw with the Vector Path and press Ctrl+Z with the button held; draw a Line.
  **Expected:** Rectangle, then one "Pen" row per point. Ctrl+Z removes only the last point and the rubber band continues from the new last point (with the curve's handle); undoing the first point removes the path, the Pen stays active and the next Ctrl+Z undoes the rectangle. Backspace removes the last point. The closing click is one step and the path is closed and filled; finishing adds no row (Enter and Escape were not measured in the reference). One click and finishing leaves a one-point path. Vector Path ignores Ctrl+Z while the button is held; Line makes one row. Note differences quoting TW-169.
- [ ] **A4-26** (TW-170 / FB-08, tester feedback; needs: real window, original for comparison)
  **Steps:** Open a document, make one change and open the Edit menu; press Ctrl+Y and Ctrl+Shift+Z; undo the change and open the Edit menu again; press Ctrl+Y; press Ctrl+Shift+Y with a text and a path selected.
  **Expected:** With nothing undone the Edit menu shows the disabled "Can't Redo", and Ctrl+Y and Ctrl+Shift+Z do nothing (no repeat of the last command). After an undo it shows "Redo: ‹step›" and Ctrl+Y redoes it. Ctrl+Shift+Y is Text ▸ Attach to Path, as in the reference. The History panel's Replay button works as before. Note differences quoting TW-170. Run 30 found one case against this rule: after a Clone (Ctrl+Shift+D) with nothing undone, the reference's Ctrl+Y clones again (TW-173); also try Clone then Ctrl+Y and read the Edit menu label before it.
- [ ] **A4-27** (TW-172 / FB-11, tester feedback; needs: real window, original for comparison)
  **Steps:** In both programs draw a closed Pen triangle and zoom to 1600 %. With the Subselection tool hover over a point, a handle, a segment, the fill and empty space; drag an anchor and a handle by a fraction of a pixel and read the coordinates in the Info panel; repeat with the grid, guides and smart guides switched on. Move the whole object with the Pointer and place Pen points. Open the View menu and switch *Snap to Pixels* off, then repeat the Pointer move and the Pen points.
  **Expected:** Over a point or handle and while dragging the cursor is a hollow arrowhead, otherwise the plain black arrow, with no marker over segments. Point and handle drags are free (fractional positions, one "Move Point" row) and snap only to the grid, guides and smart guides when switched on. Pointer moves and Pen points are whole-pixel while *Snap to Pixels* is on (default) and fractional when it is off. *Snap to Pixels* sits after the Smart Guides submenu, has no shortcut and is Firepixel's own (the reference has only Modify ▸ Snap to Pixel, Ctrl+K). Note the cursor shape and whether anchors snap to other objects via smart guides, quoting TW-172.
- [ ] **A4-28** (U-1 rest, PDF layers; needs: real window, real layered PDFs, original for comparison)
  **Steps:** Import or open real PDF files with layers (optional content groups), not hand-made ones: an export from a layout or drawing program with nested layers, one with a layer hidden by default, a multi-page file and an AI file from version 9 or later. Look at the Layers panel and the canvas of each page; switch the layers on and off; compare with the same files opened in the reference application and in a PDF viewer with its layer list.
  **Expected:** One layer per group used on the page, named after the group (nested ones as "Parent / Child"), groups hidden by default arrive as hidden layers, content outside any group lands in "Layer 1"; the stacking order keeps the look of the page; a PDF without layers imports as one layer as before. Known limits: visibility expressions (OCMDs) are not split, groups used only inside form XObjects stay with the layer that draws the XObject, `/Intent`, `/Usage` and `/AS` are ignored, each layer is rendered once; files with more than 64 groups or encrypted files fall back to one layer. Note wrong names, wrong order, missing or duplicated content and slow imports, quoting U-1.
- [ ] **A4-29** (TW-138 R5 to R7, RE spec 56; needs: real window, original for comparison)
  **Steps:** In both programs: with the Text tool active, set indent, spacing before and after, leading (also switch it between % and px), tracking, horizontal scale and baseline shift in the Properties panel and create a text; then switch to the Pointer, set other values with nothing selected, switch back to the Text tool (toolbox, T, and from another tool) and create a text; repeat with the preference `TextEditor_RememberALLSettings` on. Watch the History panel while changing the fields with nothing selected, with a pointer-selected text and in an open editor session, and when a new text is created.
  **Expected:** Values set while the Text tool is active survive into the next text; values set with the Pointer are lost when the Text tool is activated (back to the factory values, leading at the default percentage), unless the preference is on; font, size, style and alignment always stay. A new text starts with tracking 0. Each field change with nothing selected or on a pointer-selected text is its own "Text Change" row; an editor session on existing text ends as one "Text Change" row. Known differences: for a new text the reference writes two rows ("Text" and "Text Change"), Firepixel one; the reset of the text tool's own opacity and blend mode is not done (Firepixel's text tool has no own opacity). Not measured yet: the leading conversion from px back to %. Note differences quoting TW-138.
- [ ] **A4-30** (BA-101, JPEG 2000 in the PDF export; needs: real window, external PDF readers)
  **Steps:** Export a document with photos (one with transparency, one grayscale) as PDF with the compression "JPEG 2000" at the quality levels High and Low, once with PDF 1.3 selected; open the files in at least two external PDF readers and import them back into Firepixel; compare with the same export using "JPEG".
  **Expected:** The readers show the images without errors or colour shifts, transparency is kept (SMask), the grayscale photo stays gray; the file written as PDF 1.3 reports version 1.5 (JPXDecode needs it, noted in the manual); the Low level is clearly smaller than High; the re-import shows the same pictures.
- [ ] **A4-31** (BA-109, `ai.upscale`; needs: real window, model download)
  **Steps:** Open "Manage AI Models..." and check the dialog: eight rows including the two Real-ESRGAN models with licence, size and state, nothing cut off at the bottom. Download x4plus. In the Batch Studio add the step "AI upscale" (scale 4, alpha network) and run it on a small photo with transparency and on a 1000 px photo; cancel the long one midway; compare the output with a bicubic ×4.
  **Expected:** The card shows scale, model and alpha with their defaults; without the model the hint with "Manage AI Models..." appears; the result is sharper than bicubic without colour shifts and with the transparency intact; progress moves per tile, cancel stops within a few seconds and leaves no output file; the report names the model.
- [ ] **A4-32** (BA-109, `ai.faceBlur`; needs: real window, photos with faces)
  **Steps:** In the Batch Studio add the step "Blur faces" and run it on a portrait, a group photo, a photo with a profile face and a photo without people; try shape rect, pixelate on, strength 100 and a region JSON `{x, y, width, height}` covering only half the picture.
  **Expected:** Frontal faces are unrecognisable (ellipse with soft edge, or rectangle, or mosaic); no hit on the photo without people and no error, the report says "0 faces"; a profile face may be missed (documented limit); with the region only faces inside it change; the JSON field rejects malformed input with a readable message.
- [ ] **A4-33** (TW-180 / FB-12, TW-175, tester feedback batch 4; needs: real window, original for comparison)
  **Steps:** Open a document with two paths and a rectangle, zoom 1600 %. View ▸ Snap to Pixels must be unchecked after a fresh start. Drag an anchor with the Subselection tool by a few screen pixels: the point moves in 1/16 px steps (Info panel). Check Snap to Pixels and drag again: the point lands on whole pixels. With Smart Guides on, drag an anchor past the edge and the centre of the rectangle and past an anchor of the other path.
  **Expected:** Default off; on → whole pixels, off → fractional; no smart-guide line and no snap while dragging an anchor (the Pointer still snaps with a dashed line); the Pointer's object moves stay whole-pixel in both states.
- [ ] **A4-34** (TW-181 / FB-14b, tester feedback batch 4; needs: real window, real mouse)
  **Steps:** Blur tool, size 22, over a bitmap: paint several long strokes with many direction changes; watch the canvas during and after each stroke; move the pointer off the canvas and back; move it over an area with no editable bitmap.
  **Expected:** One tip outline at the pointer during the stroke, nothing left behind after it; no system cursor over the editable bitmap (crosshair only with precise cursors); the outline disappears off the canvas; over nothing editable a "not allowed" cursor and no outline. Same for Sharpen, Dodge, Burn, Smudge.
- [ ] **A4-35** (TW-182 / FB-14a, tester feedback batch 4; needs: real window, real mouse, original for comparison)
  **Steps:** On a pixel-art bitmap paint slowly with Blur, Sharpen, Dodge, Burn and Smudge, holding the button; compare with the original.
  **Expected:** The effect appears under the brush while the button is held; releasing changes nothing visible; one History row per stroke named after the tool; speed acceptable on a 2000 px bitmap.
- [ ] **A4-36** (TW-183 / FB-15, tester feedback batch 4; needs: real window, real mouse, original for comparison)
  **Steps:** Bitmap smaller than the canvas, Smudge size 22: drag from inside the bitmap across its edge into the empty canvas; undo; repeat with "Entire document" on; then check "Confine to boundaries" and repeat; also start a stroke outside the bitmap and drag into it.
  **Expected:** The smear continues past the edge and fades out, the bitmap's bounds grow (Info panel), undo restores the old size; with "Entire document" the canvas colour is smeared into the bitmap; with "Confine to boundaries" the smear stops at the old edge; a stroke from outside drags transparency in. Compare the look with the original.
- [ ] **A4-37** (TW-184 / FB-13, tester feedback batch 4; needs: real window, original for comparison)
  **Steps:** Hover over tools in the toolbox and in a flyout, over toolbar buttons, over the Smudge inspector fields and checkboxes; toggle View ▸ Tooltips and move and drag over the canvas.
  **Expected:** Tool tooltips: "Name (shortcut)" plus a one-line description; toolbar: the name only; inspector: the descriptions as in the original ("Brush size", "Edge softness", …), none on labels and arrows; the "x:… y:…" tag follows the pointer while hovering and while dragging and is gone with the switch off or off the canvas. Since F57 (Q-61) the tag depends on the tool; check it with A4-40.
- [ ] **A4-38** (TW-186, RE spec 58 §1; needs: real window, original for comparison)
  **Steps:** In both programs draw a rectangle with a red fill and a 2 px blue stroke, select it and press X (then the toolbox swap button); press Ctrl+Y, Ctrl+Z and Ctrl+Shift+Z and watch the History panel and the toolbox colour wells. Repeat with nothing selected; with two rectangles of different colours selected; with a rectangle whose stroke is none and one whose fill is none; with a text; with a bitmap. Jump back in the History panel past the swap.
  **Expected:** With nothing selected only the toolbox wells swap and no row is written. With a selection the wells take the topmost selected object's colours, swap, and every selected object gets the swapped fill and stroke as one row "Stroke Color" (both sides coloured) or "Brush and Fill" (one side none; a new stroke is the 1-pixel anti-aliased tip, 1 px; an existing stroke keeps tip, width and placement). The second rectangle gets the same colours as the first. Ctrl+Y repeats the swap as a new row. Undo, redo and the History jump restore the objects and the wells. A text loses its fill and gets a 1 px stroke in its old colour; a bitmap gets the row and no pixel change. Note the English row names (derived) and differences, quoting TW-186.
- [ ] **A4-39** (TW-194, was TW-04, RE spec 58 §6; needs: real window, real keyboard, original for comparison)
  **Steps:** In both programs select a bitmap, take the Marquee and drag a selection on it; press Escape once and look at the History panel, the active tool, the selection handles and the marching ants; press Escape again. Repeat with the Lasso and the Magic Wand. After the first Escape press Delete.
  **Expected:** The first Escape writes one row "Exit bitmap mode", the pixel selection is gone, bitmap mode ends, the Pointer becomes active and the bitmap stays selected; the second Escape changes nothing. Delete then removes the whole bitmap (no pixel selection left). Same for all three tools. Note differences quoting TW-194.
- [ ] **A4-40** (Q-61, RE spec 58 §4; needs: real window, real mouse, original for comparison)
  **Steps:** Switch View ▸ Tooltips on in both programs. Hover over the canvas with the Pointer, Pen, Text, Rectangle, Ellipse, Line, Polygon and an Auto Shape tool; press the Pointer on an object without moving, then drag the object; draw a rectangle. Also try the Subselection, Scale, Crop and Marquee tools and a handle drag. Restart Firepixel and check that the switch kept its state.
  **Expected:** No tag on plain hover with the Pointer, Pen or Text and none on a press without movement; the shape tools show "x:… y:…" (pointer position) on hover at once; dragging an object shows its new top-left corner; drawing shows "w:… h:…"; the tag's top-left sits at the pointer +0/+14 in a light grey rounded box and is gone on release. The switch survives a restart (preference `Tooltips`); a fresh profile starts with it off (since F64 `8a681109`, tester feedback FB-17). Note what the reference shows for the Subselection, Scale, Crop and Marquee tools and handle drags (unmeasured; Firepixel shows none), quoting Q-61.
- [ ] **A4-41** (TW-192, was Q-58, RE spec 58 §2; needs: real window, original for comparison)
  **Steps:** In both programs set the tool stroke to 2 px centred blue and the fill to red, then run Commands ▸ Run Script with `addNewRectangle`, `addNewOval` and `addNewLine`; set the stroke to none and run `addNewLine` again. Select an object with a 4 px outside green stroke, deselect it and run `addNewRectangle`; run a script that selects an object and draws in the same script. Select an object with stroke none and look at the stroke well in the Tools panel.
  **Expected:** Rectangle and oval take fill, stroke colour, tip, size and placement exactly; the line takes the stroke and no fill, and with the tool stroke none still gets a 1 px stroke in the tool stroke colour. After selecting the green object the tool attributes follow it and stay after deselecting, so the next rectangle has the 4 px outside green stroke; inside one script the selection does not change the tool attributes. Note how the stroke well looks after following an object with stroke none, quoting TW-192.
- [ ] **A4-42** (TW-193, was Q-59, RE spec 58 §3; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs open a document with a vector, a group and a bitmap on one layer and a web layer; with nothing selected right-click each object row in the Layers panel and read the context menu; right-click a layer row with and without a selection; right-click the web layer. Run Add Mask from the object row menu and look at the Layers panel and the History; run Flatten Selection on a single vector with a thick stroke. Right-click an object that is part of a multi-selection.
  **Expected:** The right-click selects the object row (row highlight) before the menu opens; Flatten Selection and Add Mask are enabled for vector, group and bitmap; Merge Down only when the object lies directly above a bitmap. On a layer row the selection stays, the layer becomes current, Flatten and Add Mask are grey without a selection; Share Layer to Pages and to All Pages are grey on the web layer. Add Mask gives an empty linked mask that shows everything, one row "Add Mask", and the mask is selected afterwards; Flatten gives a bitmap whose bounds include the stroke. Note what the reference does with the multi-selection and with the panel footer's Add Mask button (Firepixel: Reveal All), quoting TW-193.
- [ ] **A4-43** (TW-195, RE spec 59 §1; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs set the tool stroke to red 4 px (for example `bn_1-Pixel Hard`, placement inside) and the fill to green. In a new document each, draw every auto shape from the Rectangle flyout (Arrow, Chamfer, Beveled and Rounded Rectangle, Doughnut, L-Shape, Smart Polygon, Star, Pie, Connector, Spiral) and read stroke, tip, size, placement and fill in the Properties panel and the toolbox wells; repeat with the tool stroke none and with the fill none. Then draw the Arrow Line and look at the shape and the toolbox; draw with the Measure tool and look at the shape, its label, the selection and the toolbox; press Ctrl+Z after each and look at the toolbox again. Read the History row of each shape. With the tool stroke none draw a Spiral and a Connector.
  **Expected:** The nine closed shapes take fill (also none), stroke colour, tip, size and placement of the tool; Connector and Spiral have no fill and the red 4 px stroke. Arrow Line: black `bn_Hard Line` 1 px without anti-aliasing, no fill, and the tool becomes black `bn_Hard Line` 1 px with fill none, placement kept. Measure: red `bn_Hard Line` 1 px inside with a black label ("… px"), the shape is not selected afterwards, the tool becomes red `bn_Hard Line` 1 px with fill none. All other shapes leave the tool unchanged. Ctrl+Z removes the shape and keeps the tool. Each shape writes one row "Shape" ("Form"). With the tool stroke none the Spiral gets a black `bn_Soft Line` 1 px and the Connector a black `bn_1-Pixel` 1 px without anti-aliasing. Note differences (also with a gradient or pattern tool fill, unmeasured; Firepixel gives a solid fill), quoting TW-195.
- [ ] **A4-44** (TW-192 refined, RE spec 59 §2 to §4; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs draw two paths with different fills and strokes, a group and a text. Select one path, then both paths (Shift-click), then the group and an auto shape: watch the toolbox wells, draw a rectangle during the multi-selection and read its attributes, deselect. Select a path with fill none and read the fill well and the tool fill colour; open a new document and look at the fill; switch back to the first document. Set the tool stroke to red 4 px inside and press the Default colours button; set the stroke to none and press it again; the same with fill none. Quit Firepixel (no stored `BrushTipSize`), start it again and read the tool stroke in a new document.
  **Expected:** Only a single path or text is followed (a text: fill = text colour, stroke none, placement outside). During the multi-selection and with the auto shape selected the tool does not change but the wells display the topmost object's colours (the group: no change); after deselecting the wells show the tool colours again, and the rectangle drawn during the multi-selection uses the tool attributes. Fill none: the fill well shows a white box with a red diagonal and the tool fill colour becomes the path's stored fill colour; the new document starts with a solid fill in that colour, the first document has its none back. Default colours: black / white, the 4 px inside stays; a stroke none becomes `bn_1-Pixel Anti-Aliased` 1 px, a fill none solid white. Fresh start: `bn_1-Pixel Anti-Aliased` 1 px centred (the reference's profile gave 2 px; the 1 px is the FIX-G5 decision of 13 Oct 2026). Note differences quoting TW-192.
- [ ] **A4-45** (TW-198 / FB-20, tester feedback batch 5; needs: real window, real mouse, a Mac without pattern files)
  **Steps:** On a Mac (and on Windows with no Patterns folder in the profile) draw a rectangle, select it and press the Pattern fill-kind button in the Property Inspector; open the pattern list and pick another pattern; then drag the rectangle with the Pointer, switch tools and draw a new shape. Run `pi.fill` from a script with an unknown pattern name.
  **Expected:** No error message; the list shows the eight built-in patterns (Checkerboard, Diagonal Stripes, Polka Dots, Grid, Bricks, Waves, Diamonds, Crosshatch; with pattern files present also those, "Cotton" first as the default); the pattern is applied and visible on the canvas; dragging and the tool switch work afterwards; the script call falls back to the default pattern and reports `patternFallback`. Note anything that still blocks the pointer, quoting TW-198.
- [ ] **A4-46** (TW-199 / FB-18, tester feedback batch 5; needs: real window, real mouse, Windows and macOS)
  **Steps:** With a tool active whose cursor is painted on the canvas (Pen, Brush, Rectangle), click into another application's window, then move back over Firepixel's canvas without clicking; reactivate Firepixel by clicking straight onto the canvas; repeat with the title bar and the taskbar / Dock as the way back; leave and re-enter the canvas over a panel. Repeat with the eyedropper of the colour pop-up.
  **Expected:** One cursor at all times: the system arrow while Firepixel is inactive, the tool's cursor once it is active again (also right after the click onto the canvas), no trailing copy and no flicker at the switch. Note what each OS shows, quoting TW-199.
- [ ] **A4-47** (TW-200 / FB-21, RE spec 60 §2, tester feedback batch 5; needs: real window, real mouse, original for comparison)
  **Steps:** Select a rectangle with Edge: Feather and drag the feather slider from its arrow; the same with the stroke edge softness and the texture amount; select a two-paragraph text and drag the font size, tracking, leading, indent, space before, space after and horizontal scale sliders. Press Esc during one drag; type a value into a field and press Enter. Dock the inspector at the window bottom and open each pop-up with a high and a low value; compare the place of the pop-up with the original's.
  **Expected:** The canvas follows each slider live; on release one History row per committed value ("Text Change" for text); Esc discards and leaves the object unchanged; Enter applies. The pop-up (24 × 222) opens with its left edge flush with the arrow button and the thumb of the current value under the pointer, also at the window bottom (moved into the window, not flipped above the button; only then may the first movement jump). The original updates only the field (deliberate difference, TW-171 `[N]`) and its pop-up may reach below the window. Note differences, quoting TW-200.
- [ ] **A4-48** (TW-201 / FB-22, RE spec 60 §3 and §4, tester feedback batch 5; needs: real window, original for comparison)
  **Steps:** Select a text with the Pointer and hover over every control of the text inspector (font, style, size, colour well, tracking, leading and its unit, bold / italic / underline, orientation, the alignments, indent, space before / after, the stroke well with stroke none, anti-aliasing, horizontal scale, use font kerning), the opacity panel, W / H / X / Y and the object name; also over the slider arrows, the field glyphs and the labels. Change leading, space before and space after on a one-line text, then on a text with two paragraphs. Repeat the hovering in English.
  **Expected:** The tips read as measured ("Schriftfamilie", "Schriftstil", "Größe", "Unterschneidung oder Laufweite", "Zeilenabstand", "Einheit für Zeilenabstand", "Absatzeinzug", "Leerraum vor Absatz", "Leerraum nach Absatz", "Horizontale Skalierung", "Anti-Aliasing-Stufe", "Unterschneidung von Schriftarten verwenden", "Auswahlbreite", "Auswahlhöhe", "X-Position der Auswahl", "Y-Position der Auswahl", "Objektnamen bearbeiten"; in English their translations); the colour wells show the hex value, the empty stroke well "#FFFFFF00"; no tip on the arrows, glyphs, labels and the No Style menu. On one line leading and spacing change nothing visible; with two paragraphs the leading acts from the second line and the spacings between the paragraphs, as in the original. Note differences, quoting TW-201.
- [ ] **A4-49** (TW-202 / FB-19, RE spec 60 §1, tester feedback batch 5; needs: real window, real mouse, original for comparison; fixed by F65 `8c636395`)
  **Steps:** In both programs draw a three-point path with the Pen and look at the canvas after each click; close it on the first point; switch to the Pointer, the Subselection and back to the Pen by clicking the toolbox; select an open path, a rectangle primitive, a text and a bitmap with each of these tools; repeat at 400 %. With the Pen hover over empty canvas without a path in progress, and while drawing over the first point, the last point, an inner corner point, a segment and the end point of another, unselected open path.
  **Expected:** No bounding box for a free path: while drawing the outline + 5 × 5 anchors (earlier ones hollow, the last one filled) + the rubber band, no fill until the path is closed, then the fill and the first anchor filled. A selected free path: Pointer outline + filled anchors, Pen and Subselection outline + hollow anchors; a rectangle primitive: outline + 4 corner squares (filled with the Pointer and the Pen, hollow with the Subselection); text and bitmap keep their box. Pen cursors × (new path), o, ^, −, +, / and the plain pen while drawing; the rubber band is hidden over a hit. F65 result: *to be filled in.* Note differences, quoting TW-202. (Follow-up run 35 saw only the Pointer display of a selected rectangle, as outline + four corner squares; no runner step uses the Pen.)
- [ ] **A4-50** (TW-203 / FB-23, Firepixel extension; needs: real window)
  **Steps:** Select a filled rectangle, set Edge to "Dither" with width 15 in the Property Inspector, zoom to 400 %; drag the width slider; set width 0; switch to Feather and back; save as FW-PNG, reopen, and open the same file in a reader that does not know the marker.
  **Expected:** The edge becomes a dithered transition of fully opaque or transparent pixels in a fixed Bayer pattern, no grey ramp; the slider previews live; width 0 looks like Hard; Feather and Dither keep the width; the reopened file still shows Dither, the other reader a Feather of the same width.
- [ ] **A4-51** (TW-204 / FB-26, RE spec 61 §5, tester feedback batch 6; needs: real window, original for comparison)
  **Steps:** In both programs create, in a 15 pt font, a one-line point text, a point text with three paragraphs (hard returns), a fixed-width text with one paragraph that wraps softly to three lines and a fixed-width text with three paragraphs. Select each with the Pointer, type 33 into the paragraph indent field of the text inspector and press Enter; then drag the indent slider from its arrow and release; set the indent back to 0. Watch the lines and the selection bounds and read the History.
  **Expected:** The first line of every paragraph moves right by the indent: every line of the three-paragraph texts, only the first line of the soft-wrapped paragraph, which rewraps (3 → 4 lines). On point text Firepixel keeps the object's left, top and bottom and grows the bounds on the right by the indent (deliberate deviation: the reference moves the left of an auto-width text by the indent and keeps the width; a one-line text kept its bounds). The slider previews live; one "Text Change" row per committed value; leading and spacing unchanged. Note differences, quoting TW-204.
- [ ] **A4-52** (TW-205 / FB-28, RE spec 61 §4, tester feedback batch 6; needs: real window, real mouse, original for comparison)
  **Steps:** Select a red path and drag the Opacity slider of the Layers panel from its arrow to 56, then release; press Esc during a second drag; click the arrow, move the thumb and click beside the pop-up. Open the Blend mode list, hover several entries, then pick Multiply. Select two objects with different opacities and blend modes, then click a layer row that holds them; deselect everything. Read the History.
  **Expected:** Firepixel previews the opacity on the canvas while dragging (the reference only counts the field; deliberate difference); release applies one History row; Esc leaves the object unchanged; the pop-up sits flush with its arrow with the thumb under the press point. Hovering the blend modes previews nothing (as in the reference, TW-209), a choice writes one row. With mixed values both fields are blank (not the first object's value); with no selection both are greyed out. Note differences, quoting TW-205.
- [ ] **A4-53** (TW-206 / FB-29, RE spec 61 §2, tester feedback batch 6; needs: real window, real mouse, original for comparison)
  **Steps:** Select a red (`#FF0000`) rectangle and open the fill colour pop-up of the inspector. Press on the opacity arrow next to the "100" field, drag to 50 and release; reopen, type 50 and press Enter; reopen, change the opacity and press Esc twice. Read HEX and RGB at alpha 50 and after reopening the semi-transparent colour. Repeat with the stroke colour pop-up of a blue stroke. Repeat from the Tools panel fill and stroke wells with a vector selected, from the Color Mixer swatches and its channel sliders, and for a text colour and text stroke. Drag the roundness slider of a rectangle. Read the History each time.
  **Expected:** The canvas follows the opacity while dragging or after typing (the reference updates nothing live; deliberate difference); releasing after a press-and-drag applies and closes the pop-up; Enter applies; Esc closes the slider, then the pop-up, without change. HEX stays `#FF0000` and RGB 255 0 0 at alpha 50, also after reopening (the reference shows a blended value such as `#FF7A7A` on reopening; deliberate difference). The fill becomes `#ff000080` with one row, the stroke `#0000ff80` with one row (the reference two). The Tools panel wells, the Color Mixer and the text colours preview the same way; the roundness slider previews live and writes one row on release. Note differences, quoting TW-206.
- [ ] **A4-54** (TW-207 / FB-24, FB-25, RE spec 61 §1, tester feedback batch 6; needs: real window, real mouse, original for comparison)
  **Steps:** Draw a triangle with a thick stroke, pick a pattern fill (for example Bricks) in the inspector and look at the three fill handles with the Pointer. Move the shape by a Pointer drag, by arrow keys and Shift+arrow and by the X and Y fields; duplicate it; undo. Drag fill handle 1 by an odd amount (for example 37/23). Scale the shape to 150 % with Numeric Transform with "Scale attributes" on and off, rotate it by 90°, and scale it with the Scale tool. Repeat the moves at 400 %.
  **Expected:** The pattern travels with the shape: after every move the pixels inside are the same as before (no joints jump to other columns or rows) and all three handles move by the same delta; dragging handle 1 by 37/23 moves the pattern by exactly 37/23 (no snapping to the tile). A new pattern's handle 1 sits at the centre of the bounds, handle 2 one tile width to the right, handle 3 one tile height up. With "Scale attributes" on the pattern scales and rotates with the shape; off, the handles stay in the document (tile size unchanged). Since F74 the Scale, Skew, Distort and Free Transform tools follow the tools' own flag (A4-57); known rest: the Pointer's own scale handles always transform the fill handles. Note differences, quoting TW-207.
- [ ] **A4-55** (TW-208 / FB-30, RE spec 61 §3, tester feedback batch 6; needs: real window, real mouse, Windows and macOS, original for comparison)
  **Steps:** Draw four paths on one layer and one path on a second layer; add a locked layer with an object. In the Layers panel click the bottom path row, then Shift+click the top path row; Shift+click a middle row; Ctrl+click single rows on and off (on the Mac Cmd+click, and also Ctrl+click); Ctrl+Shift+click a row; click a row that is part of the multi-selection; click a layer row, then Shift+click a path row on the other layer; select an object on the canvas, then Shift+click a row.
  **Expected:** A click selects only that row (the anchor); Shift+click selects the range from the anchor to the row and replaces the selection; Ctrl+click (Cmd on the Mac, and the plain Ctrl key there too) toggles the row and keeps the anchor; Ctrl+Shift+click acts like Shift+click; a click on a selected row keeps the multi-selection; a layer row selects all its objects and becomes the anchor; a range across layers includes the objects in between but not those of locked or shared layers; after a canvas selection the range starts at the topmost selected row (both superseded by F81, see A4-61: shared layers count, locked ones do not, and the anchor stays while its object is selected, else the bottom-most selected object). Note differences, quoting TW-208.
- [ ] **A4-56** (TW-203 rest, F73 `2e0fe826`, Firepixel extension; needs: real window)
  **Steps:** Type a text in a large bold font (for example 72 pt) and select it with the Pointer. Over the bridge or in Commands ▸ Run Script set the fill edge: `pi.fill {edge: 3, feather: 8}` (or `fw.getDocumentDOM().setFillEdgeMode("dither", 8)`), zoom to 400 %; then `setFillEdgeMode("feather", 8)`, then width 0 for both kinds; then pick an entry of the text inspector's anti-aliasing level list. Give the text two colours (select a word and recolour it) and repeat Dither 8. Undo each step.
  **Expected:** Dither 8: the glyph edges become a dithered transition of fully opaque or transparent pixels in a fixed Bayer pattern on document pixels (no grey ramp), reaching up to 8 px beyond the glyphs; Feather 8: a soft ramp of the same reach; dither width 0 looks like Hard, feather width 0 like Anti-Alias; the anti-aliasing list sets edge 1 (Anti-Alias) again, its "None" entry Hard. Both colours keep their own runs inside the edge. The text inspector shows no Edge combo (as in the reference). Undo restores the previous edge. Note differences, quoting TW-203.
- [ ] **A4-57** (TW-207 rest, F74 `b585f6b8` and F75 `2150bbf3`, RE spec 61 §1.5; needs: real window, real mouse, original for comparison)
  **Steps:** Draw a rectangle with a pattern fill (for example Bricks) and a second one with a linear gradient, and look at the fill handles with the Pointer. In Preferences ▸ General turn "Scale strokes and effects" off, scale each shape to about 150 % with the Scale tool by a corner and press Enter; repeat with the Skew, Distort and Free Transform tools (commit with Enter, a double-click and a tool switch); turn the preference on again and repeat the Scale tool. Open Modify ▸ Transform ▸ Numeric Transform, untick "Scale attributes", press Cancel, reopen the dialog; scale by 150 % with the box off, then on. In Commands ▸ Run Script run `fw.getDocumentDOM().scaleSelection(1.5, 1.5, "autoTrimImages")` and then the same with `"autoTrimImages transformAttributes"`. Undo each step and read the History.
  **Expected:** With the preference off the pattern keeps its tile size and phase in the document and the gradient handles stay where they were, while the shape scales, skews or distorts; on, pattern and gradient transform with the shape (handles scaled about the opposite corner). The Numeric Transform box stays unticked after Cancel and reopening (its own setting, independent of the preference) and decides the handles the same way. The script call without `transformAttributes` leaves the handles (and the stroke width) unchanged, with it they scale. Undo restores shape and handles. Note differences, quoting TW-207.
- [ ] **A4-58** (Q-62, F76 `3cf23641`, RE spec 62 §1; needs: real window, real mouse, real keyboard, original for comparison)
  **Steps:** (1) Select a rectangle, press Ctrl+T, drag a corner, drag a second corner and press Escape; look at the canvas, the active tool, the History panel and Edit ▸ Redo, then press Ctrl+Y. Repeat with the Scale tool; once press Escape while the mouse is still held after the first drag, keep moving and release. Repeat with Enter instead of Escape, then press Escape. (2) Select a bitmap, take the Brush, start a long stroke and press Escape halfway with the mouse held; keep dragging, release; press Escape again. (3) On a selected bitmap drag a Marquee selection, then start a second Marquee drag beside it and press Escape while holding the mouse; keep moving and release. Repeat on a fresh bitmap where the first Marquee press opens bitmap mode, and with the Lasso. (4) Drag a Marquee selection on a bitmap, press Right, Right, Shift+Down, press Delete to see where the marquee sits, undo, then press Ctrl+Z once more; repeat with the Lasso. (5) Select a shape, type 123 into the X field of the Property Inspector, press Escape twice, then Enter. Read the History each time.
  **Expected:** (1) Every drag applies at once; Escape returns the object to its start size, the Pointer is active, the History keeps one greyed "Transform" row and Edit ▸ Redo reads "Redo: Transform"; Ctrl+Y applies it again. Escape with the mouse held keeps the frozen preview until the release, then nothing is applied, the mode ends and no row is written. Enter applies one "Transform" row; Escape afterwards does nothing. (2) At the Escape the tool switches to the Pointer and the preview stops growing; the release still paints the whole stroke as one "Brush" row and bitmap mode stays; the next Escape writes "Exit Bitmap Mode" and the bitmap stays selected. (3) Escape drops the drag and ends bitmap mode (the older selection is gone too), the Pointer is active, the bitmap stays selected and the release does nothing; one "Exit Bitmap Mode" row with an earlier selection, two when the press itself opened bitmap mode (the reference also writes "Edit Bitmap" at the press, Firepixel not). (4) Only the marquee moves (1 px, 10 px with Shift), the pixels stay (the hole is at the moved marquee); the arrows make one "Move" row and one undo puts the marquee back. (5) Escape leaves 123 in the field with the caret and changes nothing; Enter applies it as one row. Note differences, quoting Q-62.
- [ ] **A4-59** (Q-64, F77 `2b702176`, RE spec 62 §2; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs type "Ag" with the Text tool, set Arial 60 pt, select the text with the Pointer and pick a linear gradient (black to white) in the fill colour pop-up of the Property Inspector; look at the glyphs and the three fill handles at 100 % and 400 %. Then pick a pattern (Weave or another). Move the text by a Pointer drag, by Right and Shift+Down and by the X and Y fields; scale it with the Scale tool by a corner and press Enter; with the Scale tool drag inside the frame and press Enter; scale it to 150 % with Modify ▸ Transform ▸ Numeric Transform, once with "Scale attributes" on and once off; type more characters at the end and change the size to 90. Undo each step.
  **Expected:** The gradient runs over the glyphs from the top to the bottom of the object bounds (handle 1 top centre, handle 2 bottom centre, handle 3 one text height to the right of handle 1), the pattern has its tile origin on handle 1 at the centre of the bounds; both are sharp at 400 % and keep the text's anti-aliased edge. Every move keeps the image inside the glyphs identical and takes the handles along. The Scale tool (corner drag or a drag inside the frame) leaves the handles in the document, so gradient and pattern do not stretch or travel with the text (the reference also changes the font size, Firepixel stores a matrix: TW-212). Numeric Transform scales the handles about the centre, with "Scale attributes" on and off alike. Typing and the size change leave the handles where they were. Undo restores text and handles. Note differences, quoting Q-64.
- [ ] **A4-60** (TW-215, F79 `7337428e`, TW-205; RE spec 60 §2.3, RE spec 61 §4.1, RE spec 42 §2; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs open a copy of `par_layers.fw.png` and collapse "Ebene 1" with its triangle; open another file (e.g. a copy of `par_vector.fw.png`) and look at its Layers panel; switch back, save the copy, close it and open it again. Click a layer's triangle, then press Ctrl+Z and Ctrl+Y and look at the History panel. Select an object, press the arrow of the Layers panel's opacity field, drag 41 px down and release; repeat with the opacity of the Property Inspector and with the alpha slider of the fill colour pop-up (press its arrow, drag 41 px down, release).
  **Expected:** The other file shows its layers as stored (expanded; no collapse carried over from the first file), and the copy reopens with "Ebene 1" collapsed as saved. A triangle click is one History step in Firepixel; Ctrl+Z restores the previous disclosure and Ctrl+Y redoes it (note whether the reference writes a row for the click itself: only its script call `setLayerDisclosure` is measured, one row each). The 41 px drag gives 80 in all three sliders, as in the reference (the field counts 2 px per per cent, 100 % at the top); the pop-up keeps its 24 × 222 px size. Note differences, quoting TW-215.
- [ ] **A4-61** (TW-208 rest, F81 `6fede0b9`; RE spec 63 §1; needs: real window, real mouse, Windows and macOS, original for comparison)
  **Steps:** In both programs open a copy of `par_layers.fw.png` (T07). Click L1obj on "Ebene 1", Shift+click the object row of the shared layer "Gemeinsam" inside the locked "Ebene 3", then Ctrl+click L1obj; read the selection after each click. Click, Shift+click and Ctrl+click the "Ebene 3" layer row and an object row inside it. In a second copy unlock "Ebene 3", keep "Ebene 2" hidden and collapse it, click L1obj and Shift+click the object of "Unterebene", so the range crosses the hidden, collapsed layer. Shift+click a layer row; Ctrl+click a layer row with some of its objects selected. Deselect the anchor object with Ctrl+click and Shift+click another row; press Ctrl+D and Shift+click a row. Look at the History panel after each step.
  **Expected:** On T07 the Shift+click on the locked layer's row changes nothing (1 selected object, L1obj) and the Ctrl+click empties the selection (0); clicks of any kind on rows of locked layers or of their sublayers change nothing (selection, anchor and current layer stay). The range across layers runs over the document order from top to bottom and does not stop at layer boundaries: objects of hidden and collapsed layers, of shared layers and of sublayers are included, objects of locked layers are not. A Shift+click on a layer row ends the range at the first or last of the layer's own objects (not its sublayers' objects); a Ctrl+click on a layer row toggles each own object (selected ones off, the others on). The anchor stays while its object is selected; otherwise the range starts at the bottom-most selected object, with nothing selected at the bottom-most object of the document (a Shift+click selects from that row to the end). No History rows. On the Mac Cmd+click toggles. Note differences, quoting TW-208.
- [ ] **A4-62** (TW-214, F80 `297a4613` and F82 `3523c4e3`; RE spec 63 §2; needs: real window, original for comparison)
  **Steps:** In both programs create a 600 × 400 document and in Commands ▸ Run Script run `fw.getDocumentDOM().addNewText({left: 100, top: 100, right: 300, bottom: 200}, true)`, give the new text the characters "Ag" in Arial 12 pt (`textRuns` with `initialAttrs.face` and `size`, as in the test `tw214_new_text_frame`) and read `left`, `top`, `width`, `height`, `autoExpand` and `pixelRect`; repeat with Arial 30 and 60 pt, with three lines, with Myriad Pro 60 pt and with Times New Roman 60 pt; repeat the first case with `false` as the second argument and read the font and size. Then set `autoExpand = true` on each text and read the values again. Run `addNewText({left: 100, top: 100, right: 100, bottom: 100}, true)` and give it "Ag". Finally click with the Text tool at (100, 100), type "Ag" and read the values. Look at the canvas at 100 %.
  **Expected:** The object is the rectangle plus 2 px on each side (left 98, top 98, width 204); its height is the first line plus 4: Arial 12 / 30 / 60 pt → 13 / 32 / 63 px (object 17 / 36 / 67), Myriad Pro 60 → 58, Times 60 → 61, each further line adds leading × size; the first baseline sits at 100 + round(top of "d" × size) (Arial +9 / +21 / +43). `true` takes the Text tool's font, `false` the factory font Myriad Pro 12 pt; both stay fixed width (`autoExpand` false). After `autoExpand = true` top and left stay (98, 98), the baseline becomes the frame top (100) + ceil(ascent × size), the height ceil(lines × leading × size) + 4, and the Arial ink moves 1 px right (`pixelRect` left 100 → 101), the Myriad and Times ink does not. The zero-width rect makes a fixed frame shown 10 px wide (object 14) without ink (the reference's height of 722 px is not reproduced). The Text tool click puts the baseline at 100, the object top at 100 − ceil(ascent × size) − 2 (87 at 12 pt) and the left at 97 (Arial). Known rests: the Myriad 60 pt ink is 1 px wider in Firepixel, and `initialAttrs.face` reads "Arial" against "ArialMT". Note differences, quoting TW-214.
- [ ] **A4-63** (Q-65, F83 `5b47a56b`; RE spec 64; needs: real window, real mouse, original for comparison)
  **Steps:** In both programs open a copy of `par_layers.fw.png` (T07) and look at the Layers panel (which row is highlighted, the circles at the right) and the History panel. Click the object row L1obj, then click an object on another layer on the canvas with the Pointer, then drag a marquee over one object; after each read the current layer (row look, or `fw.getDocumentDOM().currentLayerNum` in Commands ▸ Run Script) and the History. Shift+click a second object on another layer (two selected), then press Ctrl+D. Click the "Ebene 2" layer row, press Ctrl+Z and Ctrl+Y. In a second copy unlock "Ebene 3", click the object row inside "Unterebene", then press the New Layer button; draw a rectangle. Make an unlocked layer with an unlocked layer above it current and lock it with its lock icon; then lock the top layers so that only the web layer stays unlocked above.
  **Expected:** T07 opens with "Ebene 1" current and visible: its row dark blue (#a8c6ed) and bold, scrolled into view, nothing selected, no History row, all circles empty (the web layer's grey). One selected object makes its layer current whichever way it was selected (row, canvas, marquee), a sublayer or a hidden layer too, without a History row; with two selected objects the current layer stays unchanged, and Ctrl+D keeps it. With a selection the selected object rows are dark blue and bold, the current layer and every layer holding a selected object light blue (#d3e2f5), and the circle is filled on each layer holding a selected object (not on the current layer for its own sake). The layer-row click writes one "Layer Change" row; Ctrl+Z restores the previous current layer and selection, Ctrl+Y redoes it. With the sublayer current the New Layer button inserts a sublayer above it in the same parent and makes it current; the rectangle lands at the top of the current layer. Locking the current layer moves the current layer up to the next unlocked layer (the web layer counts), with none above to the bottom layer. Known: a click on another layer's circle while objects are selected moves them onto that layer in the reference, in Firepixel it acts as a layer-row click (TW-217). Note differences, quoting Q-65.
---

# Part B – Full catalogue

## 1. Canvas, tools and strokes (real mouse / tablet)

|---|---|---|---|---|
- [ ] **VER-1.01** (FV-16 (W25/W26); needs: bridge, images)  
  **Steps:** Gradient tool with no selection; change fill/stroke color in the Tools panel; click the color well, set a stop to red; switch to the Fill tool.  
  **Expected:** Inspector shows the bucket icon and **one** color well black → white, no None/Solid/Gradient/Pattern buttons, edge "Anti-alias" and texture usable; the well stays black → white after changing fill/stroke; after editing it shows red → …; the Fill tool shows the four buttons. Compare with a screenshot of the original.
- [ ] **VER-1.02** (AU-3; needs: bridge)  
  **Steps:** Select a rectangle, run `fw.getDocumentDOM().setFillNColorNTexture({category:'fc_Solid'}, '#ff0000', '<texture name>')` via Commands ▸ Run Script, then set texture amount 50 in the Inspector.  
  **Expected:** Inspector shows the texture, the fill is visibly textured.
- [ ] **VER-1.03** (AU-3; needs: bridge)  
  **Steps:** Rectangle tool, no selection, fill texture ▸ "Other…", pick a PNG outside the texture folders, amount 50, draw a rectangle.  
  **Expected:** Field shows the file name without extension or path; the new rectangle is textured; the Inspector shows the same texture.
- [ ] **VER-1.04** (PERF-1; needs: real window)  
  **Steps:** Release build: open the large wireframe test file (thousands of objects), zoom 400 %, Ctrl+A, press Right arrow 10×.  
  **Expected:** Each key press visible in the next frame (< ≈ 100 ms); Optimize and Pages panels update color table / thumbnail shortly after the move. A slow first drag is known (BA-097).
- [ ] **VER-1.05** (FIX-A; needs: real window)  
  **Steps:** Document 400 × 300, zoom 400 %, scroll with wheel and Hand tool far left/up and right/down; repeat in the 2-Up and 4-Up tabs.  
  **Expected:** Document corner stops exactly at the canvas edge (left/top) and at the scroll bars (right/bottom), no grey margin behind it, scroll thumb at the end.
- [ ] **VER-1.06** (PINSEL-1 (N31); needs: real mouse)  
  **Steps:** Brush, size 35, "Soft rounded", edge 37; drag slowly and quickly on an empty area.  
  **Expected:** Stroke appears smoothly under the pointer while dragging, no circle outlines; release without flicker; one history step "Brush".
- [ ] **VER-1.07** (PINSEL-1; needs: real mouse)  
  **Steps:** Open a 4000 × 3000 photo, select the bitmap, paint with size 35 across the image at 100 % and 400 %.  
  **Expected:** Smooth; the stroke stays identical after release.
- [ ] **VER-1.08** (PINSEL-1; needs: bridge)  
  **Steps:** On a semi-transparent bitmap: opacity 21, blend mode Average, "Preserve transparency" on, texture "Heart 01" amount 50, paint.  
  **Expected:** Stroke semi-transparent and blended, nothing outside opaque pixels, texture visible.
- [ ] **VER-1.09** (PINSEL-1 (BA-038); needs: real mouse)  
  **Steps:** Press Esc in the middle of a brush stroke; then paint, Ctrl+Z, Ctrl+Y.  
  **Expected:** The stroke stays, then the Pointer tool and a step "Exit bitmap mode" (like the original); undo/redo restore exactly.
- [ ] **VER-1.10** (PINSEL-1; needs: bridge)  
  **Steps:** Tool Inspector ▸ "Edit Brush Stroke" with a bitmap selected; switch tabs; Shape: size 30, square, OK; paint; reopen.  
  **Expected:** Dialog opens (no hint); stroke is 30 px and square; values are kept.
- [ ] **VER-1.11** (PINSEL-1; needs: bridge)  
  **Steps:** Open the stroke category menu.  
  **Expected:** 12 categories, 54 stroke types, "Stroke Options…" at the bottom; changing the type keeps the size.
- [ ] **VER-1.12** (RT-1 (L2); needs: bridge)  
  **Steps:** Doc 400 × 300 at 100 %, smart guides on; R1 100,100–210,160; drag a 50 × 30 rectangle until its left edge is 4 px right of 210.  
  **Expected:** It snaps flush to 210, magenta guide at x 210 (the Pen still does not snap to objects).
- [ ] **VER-1.13** (RT-1/RT-3 (L3); needs: bridge)  
  **Steps:** `re5a_t1_rand_fp.fw.png`, marquee 100,50–150,150, Delete; read properties; Ctrl+D, Esc; Ctrl+C, Ctrl+V. Second run: Eraser across the square, Esc.  
  **Expected:** In bitmap mode 0,0, 400 × 300; after Esc (Pointer) 150,50, 50 × 100; the copy is the same. Eraser run: 100,50, 100 × 100.
- [ ] **VER-1.14** (RT-1 (L8); needs: bridge)  
  **Steps:** Bitmap 0,0–400,300 with pixels at 100,50–200,150; Filters ▸ Adjust Color ▸ Invert.  
  **Expected:** Bounds 100,50, 100 × 100; one history step "Filter Image".
- [ ] **VER-1.15** (RT-1 (P3); needs: bridge)  
  **Steps:** Vector Path tool: draw a wave with precision 1, 5 and 10 (default).  
  **Expected:** Node count grows with precision; at 10 no fitting.
- [ ] **VER-1.16** (RT-3 (L4); needs: real window)  
  **Steps:** Copy `re5a_t1_rand.png` from a web browser (as PNG), new document 500 × 500, Ctrl+V.  
  **Expected:** Bitmap at 200,200, 100 × 100.
- [ ] **VER-1.17** (RT-3 (L5); needs: bridge)  
  **Steps:** `re5a_t1_rand_fp.fw.png`, Scale tool; drag outside the box; press Enter.  
  **Expected:** Handles at 0/400 × 0/300 (object bounds as in the original); the pointer frame stays at the visible pixels (no double frame, N32); dragging outside rotates around 200,150; cropped after Enter.
- [ ] **VER-1.18** (RT-3 (L7); needs: bridge)  
  **Steps:** Marquee 330,60–380,110, Select ▸ Border 5.  
  **Expected:** Symmetric border, soft at the bottom down to 116.
- [ ] **VER-1.19** (RT-3 (L9); needs: bridge)  
  **Steps:** Rectangle, Filters ▸ Blur ▸ Gaussian Blur, confirm prompt with OK, then Cancel in the dialog; repeat with OK/OK.  
  **Expected:** Cancel: object stays vector, no history step. OK/OK: one step, object is a bitmap.
- [ ] **VER-1.20** (RT-3 (D3); needs: bridge)  
  **Steps:** Text Arial 72 "Fx Ag" at 40,60, add the live effect Noise.  
  **Expected:** Noise area 11,36–257,148.
- [ ] **VER-1.21** (CF-B (CT-B-05); needs: bridge)  
  **Steps:** Parity bitmap document, B1, Red Eye Removal tool 50/50; drag a **rectangle** 100,250 → 225,287; repeat with strength 25/75; then a horizontal drag without height.  
  **Expected:** Pupil at 120,268 ≈ #4E1C1E, step "Remove Red Eye"; 25/75 lighter/darker; horizontal drag: no change, step "Edit Bitmap".
- [ ] **VER-1.22** (CF-B (CT-B-04); needs: real window)  
  **Steps:** Rulers on, vertical guide at x 200, double-click it; enter 240, OK; single click on the guide; rulers off and double-click.  
  **Expected:** Dialog "Move Guide" with 200; the guide moves in **one** step; single click: no step; without rulers: no dialog.
- [ ] **VER-1.23** (CF-B (CT-B-06); needs: bridge)  
  **Steps:** New document, Rectangle tool without selection, texture "Vein" (already shown), amount 50, draw.  
  **Expected:** `fillTexture` = Vein, fill textured.
- [ ] **VER-1.24** (CF-B (CT-B-08); needs: bridge)  
  **Steps:** Hover the tools.  
  **Expected:** Tooltips "Rectangle (U)", "Subselection (A, 1)", flyout "Slice (K)" – each shortcut shown once.
- [ ] **VER-1.25** (F18 (TW-55); needs: bridge)  
  **Steps:** Test document T10, View ▸ Grid ▸ Show Grid.  
  **Expected:** Cyan dotted grid like the original.
- [ ] **VER-1.26** (B-3 (BA-043); needs: real mouse, original)  
  **Steps:** Rectangle with gradient, Pointer; drag the end handle (square); for an ellipse/rectangle gradient drag the second-axis handle.  
  **Expected:** Gradient rotates/stretches live, exactly one history step, Ctrl+Z restores; second axis changes only the width. In the original also drag the round handle: does it move the whole gradient? (Firepixel: only the dragged handle.) Cosmetic: old handles stay visible during the drag.
- [ ] **VER-1.27** (B-3 (BA-045); needs: real window)  
  **Steps:** New document with transparent canvas, View ▸ Gamma; check 2-Up/4-Up; turn it off.  
  **Expected:** Checkerboard lighter, also in the 2-Up/4-Up tiles; back to normal when off.
- [ ] **VER-1.28** (B-3 (BA-005); needs: bridge)  
  **Steps:** Join a named open path alone; select a named rectangle alone and Join.  
  **Expected:** Path is closed, name in Layers empty; rectangle: no message, name empty.
- [ ] **VER-1.29** (B-3 (BA-013); needs: bridge, original)  
  **Steps:** Visible rectangle plus a selected rectangle with fill alpha 0, Convert to Bitmap.  
  **Expected:** Bitmap bounds cover only the visible rectangle. Open (original not measured): an object with opacity 0 is rasterized opaque in Firepixel but dropped in the original.
- [ ] **VER-1.30** (T-7; needs: bridge)  
  **Steps:** Modify ▸ Alter Path ▸ Inset Path on E1 inside 5 and 1; on P1 inside 5 and outside 5.  
  **Expected:** E1: 2 nodes; P1 inside 5: 2 contours (known: C1 with 3 instead of 2 nodes); outside 5: 1 contour with 8 nodes.
- [ ] **VER-1.31** (F23 (TW-88); needs: real window, original)  
  **Steps:** Draw a rounded rectangle and an arrow with Shift from 100,100 to 220,160; same without Shift; a star with/without Shift; watch the L-shape and arrow line.  
  **Expected:** 120 × 120 with Shift, 120 × 60 without; star identical. L-shape/arrow line: compare with the original (see FINDINGS TW-88).
- [ ] **VER-1.32** (F23 (TW-90); needs: real window)  
  **Steps:** Draw a spiral and hover its centre; hover the radius handle; hover a corner handle of a rounded rectangle.  
  **Expected:** Tooltip "Open/close spiral", none on the radius handle; corner: "Click to toggle between corners"; no "Click: " prefix.
- [ ] **VER-1.33** (K-7 (TW-89); needs: real window)  
  **Steps:** Import `par.png` with a drag 100,100 → 421,341; then import with a click (< 10 px drag).  
  **Expected:** Visible content x 121–417, y 120–323 (296 wide), nothing selected afterwards, one step; click: original size, top-left at the click point. The rubber band still shows the raw drag rectangle (known, FINDINGS TW-89).
- [ ] **VER-1.34** (CF-FP4 (FP4-01/02); needs: real mouse)  
  **Steps:** Rectangle with linear gradient, Gradient tool, drag the end handle with a real mouse and hold.  
  **Expected:** Line from start to pointer visible, fill updates only after release, one step; with the Pointer tool the handle line follows during the drag.
- [ ] **VER-1.35** (CF-FP4 (FP4-03); needs: real window)  
  **Steps:** Rectangle 0,0–50,50, F8, double-click the instance; also Modify ▸ Symbol ▸ Edit Symbol; then "Back".  
  **Expected:** Symbol editing shows the content at the instance position (not at the centre of the canvas); the instance stays 0,0–50,50.
- [ ] **VER-1.36** (F19 (TW-69); needs: real mouse)  
  **Steps:** `par_layers.fw.png`: drag across several layer eye icons (slow and fast), including sub-layers.  
  **Expected:** Every row passed over toggles like the first one.

## 2. Inspector and color picker

|---|---|---|---|---|
- [ ] **VER-2.01** (NZ-1 (W28); needs: bridge, images)  
  **Steps:** Select the tools Path Scrubber additive/subtractive, Red Eye Removal, Polygon Slice, Rectangle Hotspot, Reshape Area in de and en.  
  **Expected:** Inspector header wraps onto two lines, never inside a word, never past the Inspector edge; compare with the original.
- [ ] **VER-2.02** (NZ-2 (N24); needs: bridge)  
  **Steps:** Select text or the Text tool, open the font list in the Inspector (Normal, Dark, OLED); drag the thumb; use the wheel; type "ar". Repeat in Text ▸ Editor.  
  **Expected:** Visible scroll bar with thumb even without hover; dragging scrolls smoothly, list stays open, selection unchanged; typing shortens/hides the bar. Do not store screenshots with real font names.
- [ ] **VER-2.03** (AU-3; needs: bridge)  
  **Steps:** Create text, choose "Vertical, right to left" in the Text Inspector, read `fw.selection[0].orientation` via script; then `setTextOrientation("vertical left to right")`.  
  **Expected:** "vertical right to left"; the Inspector then shows "Vertical, left to right".
- [ ] **VER-2.04** (FARB-1a (N27a); needs: real mouse)  
  **Steps:** Rectangle, effect Glow, click its color, color-wheel button, drag to red in the big field, drag the hue bar, "Apply", OK; repeat with Esc. Spot-check Inspector fill and stroke.  
  **Expected:** Marker and preview follow and stay; color well and canvas preview red; OK = one history step; Esc restores the old color.
- [ ] **VER-2.05** (FARB-1b (2); needs: real mouse)  
  **Steps:** Open a color pop-up and move the pointer over canvas, panels and menu bar; click.  
  **Expected:** Eyedropper cursor; preview/HEX/RGB show the color under the pointer live (canvas exact, other areas after a short rest); click takes the color and closes; the canvas does not react to the click.
- [ ] **VER-2.06** (FARB-1b (4); needs: real window)  
  **Steps:** Glow effect, color well, click the canvas with the eyedropper; Esc in the pop-up.  
  **Expected:** Glow color = pixel, effect dialog stays open; Esc closes only the pop-up.
- [ ] **VER-2.07** (FARB-1b (5); needs: real window)  
  **Steps:** Dialog with a color well near the top window edge (low window).  
  **Expected:** Pop-up at y = 0, covering the color well.
- [ ] **VER-2.08** (FARB-1b (6); needs: real window)  
  **Steps:** Opacity drop-down button, drag the slider, Enter; Esc.  
  **Expected:** Slider 24 × 222 below the button, top = 100; resulting color has alpha; Esc first closes only the slider.
- [ ] **VER-2.09** (FARB-1b (7); needs: real window)  
  **Steps:** Copy buttons HEX/RGB.  
  **Expected:** Clipboard holds `#RRGGBB` or `rgba(r,g,b,1.00)`; pop-up closes.
- [ ] **VER-2.10** (FARB-1b (8); needs: real window)  
  **Steps:** Color-wheel button; Apply; Esc.  
  **Expected:** Color dialog next to the pop-up, pop-up stays open; Apply sets the color, Esc returns to the pop-up.
- [ ] **VER-2.11** (RT-3 (D5); needs: bridge)  
  **Steps:** Rectangle #ff000080, fill pop-up, swatch white.  
  **Expected:** Fill #ffffff at 100 %.
- [ ] **VER-2.12** (F17 (TW-52); needs: real mouse)  
  **Steps:** T04, select R1: (a) opacity arrow, drag the slider, Esc; (b) instead click the empty canvas; (c) press the arrow and drag while holding, release; (d) open a color pop-up, Esc, Pen, Esc, Esc.  
  **Expected:** (a) pop-up closed, R1 selected, opacity unchanged, no "*" in the title; (b) closed, R1 selected, value applied; (c) applied, closed; (d) first 1 selected, at the end 0 (TW-32).
- [ ] **VER-2.13** (F17 (TW-44/Q-27); needs: bridge)  
  **Steps:** T06, select a slice; type a name; clear it.  
  **Expected:** Name field = automatic export name, combo "PNG32" with list and edit icon; typed name sets the export name, empty = automatic again.
- [ ] **VER-2.14** (RT-2 (E2); needs: bridge)  
  **Steps:** New doc 300 × 300 (unsaved), slice 100,100–200,200; save as "test.fw.png".  
  **Expected:** Name field "_r2_c2", after saving "test_r2_c2"; `fw.selection[0].name` stays `null`.
- [ ] **VER-2.15** (F17 (Q-26); needs: real mouse)  
  **Steps:** Right-click in Inspector field W.  
  **Expected:** Edit menu Undo … Select All (localized), entries act on the field.
- [ ] **VER-2.16** (CF-B (CT-B-10); needs: real window)  
  **Steps:** Select a slice and a button, de and en.  
  **Expected:** "Edit..." fully readable and not touching "Target:"; the slice shows the edit icon.
- [ ] **VER-2.17** (CF-B (CT-B-11); needs: bridge)  
  **Steps:** Vector path, opacity 50 + Enter.  
  **Expected:** `fw.selection[0].opacity` and `get_document` return 50.
- [ ] **VER-2.18** (F19 (TW-76); needs: real window)  
  **Steps:** `par_symbol_web.fw.png`, select a slice.  
  **Expected:** Thumbnail with an area in web-object color, edit icon 15 × 10 as in the original.
- [ ] **VER-2.19** (T-10; needs: real keyboard)  
  **Steps:** Open the fill pop-up and without clicking type "#3366CC" + Enter; then click into HEX and type "zz12", Enter.  
  **Expected:** First: nothing happens (HEX field has no focus). Second: preview and RGB 0/0/0 while typing, fill #000000 after Enter.
- [ ] **VER-2.20** (F25; needs: real window, original)  
  **Steps:** Opacity field 30; `setOpacity(50.6)` via script.  
  **Expected:** Stored as 8-bit (30.19607843137255 / 50.588…), field shows whole percent. Compare in the real window; original not measured.

## 3. Dialogs

|---|---|---|---|---|
- [ ] **VER-3.01** (DLG-1; needs: real mouse)  
  **Steps:** Numeric Transform (resize, no constrain): type height 70 → click width; same in Expand Stroke miter limit, Brightness/Contrast (contrast −30 → click brightness), Hue/Saturation, Unsharp Mask radius, Animate Selection direction; also click a slider or empty dialog area; Cancel/Esc. Inspector: type W → click H, type H → click W, type W → click canvas.  
  **Expected:** A click elsewhere commits the typed value; OK applies it; Cancel/Esc discards.
- [ ] **VER-3.02** (KV-1 (D43); needs: real mouse)  
  **Steps:** Curves with a real mouse: click far from / near the curve, set points at 18 and 25 and try 29, drag < 3 units, drag a point out of the field or past its neighbour and back, click a ramp.  
  **Expected:** Far: nothing; near: new point, draggable at once; 29 rejected; drag starts after > 3 units; point deleted on release outside, restored when moved back before release; end points not deletable; preview after release; ramp click flips the display, same result.
- [ ] **VER-3.03** (KV-1 (D43); needs: bridge)  
  **Steps:** Curves: output "300" + Tab; Auto; Shadows eyedropper then click a canvas swatch; Esc in pick mode; preview off + Cancel; OK.  
  **Expected:** Point at 255; Auto sets channel curves from the 0.5 % limits, dialog stays open, no own step; per-channel curves (c,0)–(255,255); Esc ends only picking; Levels and Curves reopen without preview; OK = exactly one step, next call starts at identity.
- [ ] **VER-3.04** (KV-2 (D44); needs: real mouse)  
  **Steps:** Levels with a real mouse: click the track beside a handle; drag the midtone handle left; drag black toward white; cross the output handles; channel Red; Auto; Highlights eyedropper on B1; Esc while picking; preview off + Cancel; OK/Cancel; reopen.  
  **Expected:** Track click: nothing; gamma > 1, image lighter during the drag; midtone moves along, black stops before it; crossed outputs invert; per-channel fields/handles/histogram; Auto to 0.5 % limits, dialog stays; white point = picked color, eyedropper off afterwards; OK = one step, Cancel = none; reopen shows 0/1/255 and 0/255.
- [ ] **VER-3.05** (E1 (D07); needs: bridge)  
  **Steps:** Export "Images only" named "bild"; export again; Cancel the prompt; export once each as Layers/States/Pages/Sprites/PDF/PSD/SVG; Browse… for both folders; reopen the dialog.  
  **Expected:** Files `bild_r1_c1.<ext>`, `bild_r1_c1_s2.<ext>` …; second export asks "… already exists. Continue and replace files?"; Cancel keeps timestamps and returns to the dialog; type and folders remembered, no history step.
- [ ] **VER-3.06** (DLG-6 (D47/D49/D50); needs: bridge)  
  **Steps:** Opaque B1 320,200,420,300: Filters ▸ Blur ▸ Motion Blur 90/20, Enter, Ctrl+Z; Radial Blur 10/20; Zoom Blur 10/20.  
  **Expected:** 320,200,420,319, undo back to 320,200,420,300; radial 317,197,424,304; zoom: bounds unchanged.
- [ ] **VER-3.07** (KV-3; needs: bridge, images)  
  **Steps:** Restore Bitmap Selection: Ctrl+D, Restore… → "Invert" on, OK; Delete…; check en/qps.  
  **Expected:** Three operations greyed after Ctrl+D; frame around everything except the rectangle, history "Load Selection"; Delete removes the entry without prompt; no overflowing texts.
- [ ] **VER-3.08** (E2; needs: bridge)  
  **Steps:** File ▸ Export, type PDF, "Options…"; set None/ZIP; enable "Password to open" with a password; export and open in a PDF viewer.  
  **Expected:** Dialog "PDF Export Options", default PDF 1.6/JPEG/High; "Quality" greyed for None/ZIP; the viewer asks for the password.
- [ ] **VER-3.09** (E2; needs: bridge)  
  **Steps:** CSS sprites: options id, prefix "p-", layout Horizontal, preset JPEG; export; new export dialog in the same document; prefix "1 x".  
  **Expected:** Name field ends in ".jpg"; `name.jpg` side by side and `name.css` with `#p-…`; values are kept; invalid prefix shows "Some of the CSS selector names …" with OK.
- [ ] **VER-3.10** (E2; needs: bridge)  
  **Steps:** Layers to files: prefix document name, suffix # (01…), GIF; PSD with "Smaller PSD file"; HTML and Images ▸ Options…; Images only / SVG.  
  **Expected:** `<document>_<layer>_01.gif` …, title "Export Options - Layers to Files", lists without checkbox greyed; smaller PSD: four lists reduced and greyed, PSD has one layer; HTML and Images opens **HTML Setup**; Images only and SVG: "Options…" greyed.
- [ ] **VER-3.11** (E3; needs: bridge)  
  **Steps:** HTML Setup Table: canvas color off, cell color `#123456`, contents non-breaking space; document-specific "Include areas without slices" off; export.  
  **Expected:** Empty HTML cells carry `bgcolor="#123456"` and `&nbsp;`.
- [ ] **VER-3.12** (E3; needs: bridge)  
  **Steps:** File ▸ Export Wizard… on a document with 2 states: Next → states, single image file → target, The Web → analysis, Finish; Back on each page; target image editing; Cancel in Image Preview.  
  **Expected:** Ends in Image Preview 2-Up GIF │ JPEG; Back works on every page; image editing proposes TIFF; Cancel leaves export settings unchanged.
- [ ] **VER-3.13** (DES-1 (N11); needs: bridge, images)  
  **Steps:** In 3 themes open Image Size, Canvas Size, New Document, "Clear History" prompt, Unsaved Changes, Gaussian Blur.  
  **Expected:** Footer with etched line behind OK/Cancel, nothing covered; no footer for Gaussian Blur (buttons top right); Dark: title bar clearly darker; OK tinted, stronger on hover; Preferences deliberately without footer.
- [ ] **VER-3.14** (DES-1 + RAND-2b; needs: real window)  
  **Steps:** Dialog taller than the window (1024 × 768 at 150 %, e.g. Export).  
  **Expected:** Fixed button strip as footer (surface + etched line), buttons visible and usable.
- [ ] **VER-3.15** (DES-1; needs: real mouse)  
  **Steps:** Drag a dialog by its title bar.  
  **Expected:** Shadow/glow moves along, no leftovers on canvas or panels.
- [ ] **VER-3.16** (DLG-10; needs: bridge)  
  **Steps:** Check Spelling on "der der WLET www.x.de"; then switch options off.  
  **Expected:** Only "der" reported as a duplicate word, "Delete" merges; with options off WLET and the address parts are reported. Needs a dictionary in the profile.
- [ ] **VER-3.17** (FORM-1 (D35–D39); needs: real keyboard)  
  **Steps:** Pixel selection, Expand Marquee…, "500", Enter; Enter again; same with "0".  
  **Expected:** Field shows 100 selected, dialog stays open, no step; second Enter applies 100; "0" → 1.
- [ ] **VER-3.18** (FORM-1; needs: real mouse)  
  **Steps:** Open any form dialog and click the canvas or a panel beside it; Esc.  
  **Expected:** Dialog stays open (modal); Esc closes it.
- [ ] **VER-3.19** (NZ-3; needs: real keyboard)  
  **Steps:** Curves: click a point; Alt+E, Alt+A, Alt+U, Alt+V. Levels: drag a handle; Alt+I, Alt+U, Alt+A, Alt+V.  
  **Expected:** Input field focused and selected; the access keys work; the handle's field is focused and follows; letters always underlined.
- [ ] **VER-3.20** (NZ-3 (D19); needs: real keyboard)  
  **Steps:** Image Size: Alt+P and Alt+B.  
  **Expected:** Toggle the checkboxes; both letters underlined.
- [ ] **VER-3.21** (NZ-4; needs: bridge)  
  **Steps:** Edit ▸ Insert ▸ New Button…, OK.  
  **Expected:** Symbol editing of an empty button, no instance in the document.
- [ ] **VER-3.22** (RAND-2b; needs: real window)  
  **Steps:** Window 1024 × 768 at 150 %: open Preferences, Edit ▸ Keyboard Shortcuts, Commands ▸ Manage AI Models.  
  **Expected:** Dialog fully inside the window, content scrollable by wheel or scroll bar; OK/Cancel/Help/Close always at the bottom and working; Enter triggers the default button.
- [ ] **VER-3.23** (FX-1; needs: bridge)  
  **Steps:** Open `fx_vector.fw.png`, set the drop-shadow/glow effects of the parity set, export PNG 32.  
  **Expected:** Pixel-equal to the reference (max difference 1).
- [ ] **VER-3.24** (FX-2; needs: bridge)  
  **Steps:** Rectangle, "+" ▸ Bevel and Emboss ▸ Inset Emboss, OK, export PNG 32; same for Raised Emboss; softness 10.  
  **Expected:** Max difference ≤ 2 to the reference; softness makes edges softer (not the angles).
- [ ] **VER-3.25** (FX-4; needs: bridge)  
  **Steps:** Open an object's Curves effect via (i), drag a point, OK.  
  **Expected:** Canvas changes, one history step.
- [ ] **VER-3.26** (FX-4; needs: bridge)  
  **Steps:** Filters ▸ Adjust Color ▸ Hue/Saturation, "Colorize" on/off; insert effect "Add Noise", open (i).  
  **Expected:** Colorize: hue takes 300, slider to 360; off: −60. Noise: "Color" unchecked, amount 50.
- [ ] **VER-3.27** (FX-T; needs: bridge)  
  **Steps:** Open `fx_text.fw.png`, select the text, opacity 100 (forces re-raster); set Solid Shadow and Drop Shadow (defaults), export PNG 32.  
  **Expected:** Same image as on open (max 16 levels on ≤ 10 pixels); export within 16 levels on ≤ 10 pixels of the reference.
- [ ] **VER-3.28** (FX-T; needs: real window)  
  **Steps:** Look at text on the canvas at 100 % and zoomed.  
  **Expected:** Slightly sharper at 100 % (hinting), unchanged smooth when zoomed.
- [ ] **VER-3.29** (RT-1 (P1/P2); needs: bridge)  
  **Steps:** Brush tool (simple brush), Edit Brush Stroke: tips 3, angle 45, OK; reopen.  
  **Expected:** Spacing/flow greyed, values saved, brush stays simple; field "Ink amount" present.
- [ ] **VER-3.30** (F17 (TW-53); needs: bridge)  
  **Steps:** T04, R1, Modify ▸ Animation ▸ Animate Selection, Enter; then OK or Cancel in the follow-up prompt.  
  **Expected:** Instance 50,50–222,120 at once, "*", 1 state, prompt open; OK → 5 states, Cancel → 1.
- [ ] **VER-3.31** (F17 (TW-42); needs: bridge)  
  **Steps:** Select two objects, Edit ▸ Insert ▸ Slice; Esc.  
  **Expected:** Message with "Single"/"Multiple", no Cancel; Esc = one slice over both.
- [ ] **VER-3.32** (F17 (TW-43); needs: bridge)  
  **Steps:** File ▸ Import a vector test file (`par.ai`), OK, place with scale 200 %.  
  **Expected:** "Vector File Options" like the original; placement cursor; 200 % doubles the size (page choice, "Convert layers to states", "Render as images" still without effect, see FINDINGS U-1).
- [ ] **VER-3.33** (F17 (TW-45…49); needs: bridge, images)  
  **Steps:** Check the wording of effect pop-ups ("Knock out", "Offset:", "Solid color"), Keyboard Shortcuts ("Current set:", "Shortcuts:", "Press key:", "Change"), Preferences (Text, Launch and Edit, Plug-ins), New Style ("Properties:", "Text - Other", empty name → "Style n"), Export Wizard page 1.  
  **Expected:** Wording matches the original; Export Wizard page 1 without "•", without "k", no "Back".
- [ ] **VER-3.34** (F18 (Q-25); needs: real window)  
  **Steps:** Filters ▸ Blur ▸ Gaussian Blur on a bitmap; then Modify ▸ Canvas ▸ Image Size.  
  **Expected:** Canvas not dimmed for the blur dialog; dimmed for Image Size as before.
- [ ] **VER-3.35** (B-4 (BA-049); needs: real keyboard)  
  **Steps:** Keyboard Shortcuts: Tab into the tree; ↓/↑/→/←/Home/End; Esc.  
  **Expected:** Visible focus frame; keys behave like a Windows tree view, focus stays in the tree; Esc closes.
- [ ] **VER-3.36** (B-4 (BA-050); needs: bridge)  
  **Steps:** Restore Bitmap Selection with a saved selection: change operation / "Invert"; Cancel; OK.  
  **Expected:** Marching ants show the result immediately; Cancel restores; OK = exactly one step "Load Selection".
- [ ] **VER-3.37** (B-4 (BA-052); needs: real keyboard)  
  **Steps:** Live effects Hue/Saturation, Brightness/Contrast, Unsharp Mask: Tab through.  
  **Expected:** Same tab order as the menu dialogs.
- [ ] **VER-3.38** (B-4 (F11); needs: real keyboard)  
  **Steps:** Pop-up menu editor: type in a cell, Enter.  
  **Expected:** Stays on "Content", value applied, "Next >" not triggered.

## 4. Panels, start page, file dialogs and window/OS integration

|---|---|---|---|---|
- [ ] **VER-4.01** (DLG-3; needs: real keyboard)  
  **Steps:** Release build, changed document, Ctrl+W with a real keyboard; film the screen (~10 ms).  
  **Expected:** Save prompt within 100 ms of the key press.
- [ ] **VER-4.02** (EB-2; needs: bridge)  
  **Steps:** In-app Save dialog, type Reduced PNG, "Options..."; Cancel and OK; OK with a GIF setting.  
  **Expected:** File dialog hides, Image Preview opens with the document settings; both buttons return to the file dialog with path/type/checkbox unchanged; the saved file is PNG 8 (color type 3).
- [ ] **VER-4.03** (EB-2; needs: real window)  
  **Steps:** Native Windows dialog (`FP_FILE_DIALOG=native`): Save As `logo` or `logo.png`; FW-PNG / Reduced PNG; name `x.fw.png` or `x.jpg`; Cancel in the follow-up.  
  **Expected:** Follow-up dialog "Which file type should "logo.png" be saved as?" with FW-PNG/Reduced PNG, hint, "Options...", "Save as copy"; FW-PNG → `logo.fw.png`, Reduced PNG → `logo.png`; explicit extensions: no follow-up; Cancel saves nothing.
- [ ] **VER-4.04** (DES-2; needs: real window)  
  **Steps:** Recent file of 8000 × 8000 px in the start page list.  
  **Expected:** Start page stays responsive, thumbnail appears shortly after.
- [ ] **VER-4.05** (NZ-3; needs: bridge, images)  
  **Steps:** In 3 themes look at the scroll bars of Layers, History, Styles, Preferences categories, Keyboard Shortcuts, font list.  
  **Expected:** Thumb clearly visible without hover, stronger under the mouse.
- [ ] **VER-4.06** (NZ-3; needs: bridge, images)  
  **Steps:** Checkboxes "Include in export" (States), "Don't show again" (Welcome), "Save as copy" (file dialog), slider in the arrow pop-up.  
  **Expected:** White resp. dark field background as before.
- [ ] **VER-4.07** (NZ-4/NZ-5; needs: bridge)  
  **Steps:** File ▸ Open (in-app dialog), tick "Open as Untitled", pick a file; Ctrl+S.  
  **Expected:** Two checkboxes visible; title without file name; Ctrl+S asks for a name.
- [ ] **VER-4.08** (NZ-4/NZ-5; needs: bridge)  
  **Steps:** "Open as Animation" with three files.  
  **Expected:** Greyed until two paths separated by `;` are in the field; result: one untitled document with one state per file, canvas sized like the first file.
- [ ] **VER-4.09** (NZ-4/NZ-5; needs: bridge)  
  **Steps:** Save As, type FW-PNG, "Append .fw.png" off, name `x.png`.  
  **Expected:** File `x.png` saved as FW-PNG; checkbox hidden for other types.
- [ ] **VER-4.10** (NZ-4; needs: bridge)  
  **Steps:** States panel with **one** selected object.  
  **Expected:** Footer button "Distribute to States" enabled (like the menu command).
- [ ] **VER-4.11** (RAND-2b; needs: bridge)  
  **Steps:** Document Library in a low dock group.  
  **Expected:** List stays above the footer, footer buttons clickable.
- [ ] **VER-4.12** (MN-5; needs: bridge)  
  **Steps:** Optimize panel: save settings "Small", then Delete Settings → choose preset → OK; answer No and Yes.  
  **Expected:** Prompt "Delete current settings? …" with Yes/No; No keeps "Small", Yes removes it. (Deviation from the original is known.)
- [ ] **VER-4.13** (START-1 (N30/N09); needs: real window)  
  **Steps:** Release exe with a real profile, maximized and not maximized, software graphics; record the start at ≥ 20 screenshots/s.  
  **Expected:** Window appears exactly once fully drawn, no white frame, no enlarged section, one taskbar button.
- [ ] **VER-4.14** (START-1 (N30); needs: real window)  
  **Steps:** Set `FP_STARTUP_TRACE=C:\temp\start.log`, start from Explorer.  
  **Expected:** "window placed" ≈ 0.85 s (warm) after process start.
- [ ] **VER-4.15** (START-1 / B-6 (BA-095); needs: real window)  
  **Steps:** Double-click the exe; during start click another window.  
  **Expected:** Firepixel comes to the front with keyboard focus; the previously active window receives no stray Alt press.
- [ ] **VER-4.16** (START-1; needs: real window)  
  **Steps:** New profile, start and immediately open *Commands*.  
  **Expected:** Own commands are there (background thread done after ≈ 0.5 s).
- [ ] **VER-4.17** (FIX-A; needs: real window)  
  **Steps:** Start the release exe 5× with `FP_STARTUP_TRACE`, once with `FP_WGPU_ALL_BACKENDS=1`; on a GPU machine use `RUST_LOG=info`.  
  **Expected:** Line `wgpu: DX12 setup` present, window normal, time to `frame 1 begin` shorter than with all backends; GPU machine shows the card's DX12 adapter.
- [ ] **VER-4.18** (FIX-A; needs: bridge)  
  **Steps:** Open `re5a_t4_leer.png` (empty bitmap), draw a rectangle, save as `.fw.png`, reopen.  
  **Expected:** Only the rectangle in the layer; the first document still shows the empty bitmap in Layers, history unchanged.
- [ ] **VER-4.19** (RT-2 (S4); needs: real window)  
  **Steps:** File ▸ Open two files (a.fw.png 200 × 100 with 2 states, b.png 300 × 50), "Open as Animation".  
  **Expected:** Document 300 × 100, 3 states (2 from a, 1 from b), content top left.
- [ ] **VER-4.20** (RT-3 (S2); needs: bridge)  
  **Steps:** Instance at 200,150, Edit in Place; also edit from the Library.  
  **Expected:** Other objects at 50 % behind (red rectangle becomes 255,127,127), content at the instance position; from the Library: surroundings hidden, origin at the canvas centre.
- [ ] **VER-4.21** (RT-3 (S3); needs: bridge)  
  **Steps:** Layer 1 with R1, sub-layer U with R2; click the row "Layer 1".  
  **Expected:** Only R1 selected, U not highlighted.
- [ ] **VER-4.22** (F18 (TW-54); needs: real mouse)  
  **Steps:** T07 Layers panel: eye of layer 2; Alt+click the eye of layer 1; Layers ▸ Hide All / Show All; drag from the "Shared" eye to layer 1.  
  **Expected:** `layers[1].frames[0].visible == true`; Alt+click hides all 6 rows; hide/show all 6; drag hides layers 1 and 2, rest visible.
- [ ] **VER-4.23** (CF-C (CT-C-02/03); needs: real mouse)  
  **Steps:** `par_layers.fw.png`: drag state 3 onto state 1, and "02 …" over "01 …" in the Pages panel (real mouse).  
  **Expected:** Insertion line, reordered, one step "Move State/Page", Ctrl+Z restores.
- [ ] **VER-4.24** (CF-C (CT-C-06, N11); needs: real keyboard)  
  **Steps:** Close all documents, move the mouse off the content, press Tab 5×, then Arrow Down (real keyboard).  
  **Expected:** Visible focus frame on the start page that moves with the keys.
- [ ] **VER-4.25** (CF-C (CT-C-05); needs: real window, original)  
  **Steps:** File ▸ Import…, `par.png`; click; drag; Esc.  
  **Expected:** Hint in the status bar; click places at original size; drag scales proportionally; Esc cancels; compare the cursor image with the original.
- [ ] **VER-4.26** (CF-C/CF-X (CT-C-07); needs: bridge)  
  **Steps:** Change `par_layers.fw.png`, File ▸ Close All.  
  **Expected:** Prompt "Save changes to par_layers.fw?" as in the original.
- [ ] **VER-4.27** (CF-C (CT-C-10); needs: real window)  
  **Steps:** Start with `--mcp`, Ctrl+Q (No, No), or WM_CLOSE without a document.  
  **Expected:** `mcp-bridge.json` is deleted.
- [ ] **VER-4.28** (CF-C (CT-C-11/19/21); needs: real window, original)  
  **Steps:** Window menu entries; open Info, Color Mixer, URL; window width 1200 px.  
  **Expected:** ✓ = visible and in front (also Symbol Properties); Info/Mixer/URL float staggered with title bar arrow (collapse) and × (close); compare look with the original; tabs shortened at 1200 px.
- [ ] **VER-4.29** (CF-C (CT-C-12/13); needs: bridge)  
  **Steps:** Save a copy; trigger an unavailable command; bridge `ui_action {"panel": "window.properties"}`.  
  **Expected:** "Saved as copy" appears and disappears after ≈ 6 s; "not available" names the menu text; panel shown, `visible_panels` lists only front tabs.
- [ ] **VER-4.30** (CF-C (CT-C-17); needs: bridge)  
  **Steps:** Fresh profile, start page ▸ Firepixel document ▸ Templates….  
  **Expected:** Start folder `…\Templates`.
- [ ] **VER-4.31** (CF-C (CT-C-20); needs: bridge)  
  **Steps:** Common Library.  
  **Expected:** Folder "Standard library" (de: "Standardbibliothek").
- [ ] **VER-4.32** (F19 (TW-77); needs: real window)  
  **Steps:** Select an instance without parameters, open Symbol Properties.  
  **Expected:** Empty table "Name | Value" instead of a hint.
- [ ] **VER-4.33** (CF-FP4; needs: real window)  
  **Steps:** Color Mixer panel: arrow at field R; share a layer across states, then unshare.  
  **Expected:** Slider like the original (`mixer.field.r.slider`); prompt "Current/All" with title "Firepixel".
- [ ] **VER-4.34** (B-9; needs: real window, original)  
  **Steps:** Layers: "New Sub Layer", click the layer row and the sub-layer row, draw a rectangle.  
  **Expected:** Clicked row highlighted with the active marker; rectangle lands in the last clicked (sub-)layer. Compare highlighting and object selection with the original (not measured), and whether the sub-layer stays highlighted after clicking the layer.
- [ ] **VER-4.35** (B-9; needs: bridge, original)  
  **Steps:** Sub-layer active: Alt+"New Layer" (duplicate), panel menu "Duplicate Layer…", "Share Across States".  
  **Expected:** No error, copy in the same parent layer, check mark follows the sub-layer; `layer.move` index counts inside the parent (original not measured).
- [ ] **VER-4.36** (B-9; needs: bridge, original)  
  **Steps:** Sub-layer active: panel menu "Share Layer to Pages".  
  **Expected:** Greyed (original not measured).
- [ ] **VER-4.37** (B-6 (BA-096); needs: bridge)  
  **Steps:** Bridge `screenshot {}` and `ui_input {"screenshot": true}` while the color picker's eyedropper is outside the window.  
  **Expected:** Each screenshot reaches the right caller.
- [ ] **VER-4.38** (B-5; needs: bridge)  
  **Steps:** Bridge script `fw.setActiveViewScale(1, {x:300, y:200})`; import an SVG with rectangle – text – rectangle.  
  **Expected:** Zoom 100 %, point 300/200 in the window centre; text between the rectangles.

## 5. Image Preview, Optimize and Export

|---|---|---|---|---|
- [ ] **VER-5.01** (BV-2; needs: bridge)  
  **Steps:** Color table: click a swatch; press Lock, Web-safe, Transparent; Ctrl+click multi-select, Delete; double-click; click in the preview and in empty space; right-click table/preview and the ≡ button.  
  **Expected:** Status line "R:… G:… B:… Hex:… Index:…"; buttons pressed, swatch markers (squares, diamond, checks), status "Locked Modified Web-safe"; palette becomes "Custom"; "Color" dialog, OK → modified; clicking the preview selects the pixel's entry (swatch feedback highlights its pixels); empty click restores the eyedroppers; palette menu with greying rules and checks ("Unsorted", "Swatch feedback").
- [ ] **VER-5.02** (BV-2; needs: bridge)  
  **Steps:** "Select transparent color" eyedropper + click; add/remove; "Add color". "Optimize to Size" on all three tabs with target 40 % and 0.01; load/save palette (ACT/ASE/ACO).  
  **Expected:** "Index transparency" list, holes in the image; add/remove work; swatch pop-up opens; size prefilled (one decimal); 40 % → fewer colors near the target; 0.01 → "The resulting file would be too small …", values unchanged; file dialog for palettes.
- [ ] **VER-5.03** (BV-3; needs: bridge)  
  **Steps:** "Export Area" tool (crop icon at the bottom): drag corner, edge and inside (also past the image edge); in 2-Up; then back to Pointer.  
  **Expected:** Dashed frame with 8 handles over the whole image, matching cursors; frame stays inside the image; only the active pane changes; File tab shows the check mark and values; pane then shows only the area with the new size.
- [ ] **VER-5.04** (BV-3; needs: bridge, images)  
  **Steps:** Document with 3 states (on state 2): open Image Preview; navigation buttons; type "3"; Animation tab: click a row, delay 50, trash ▾, eye off, Once/Loop, ▶/■; document with one state.  
  **Expected:** Opens on state 1; first/previous greyed on 1, last/next on 3; row selects the state; list "50", first row "/100 sec", "Duration: 0.64 sec"; four delete methods; hidden state excluded from duration and playback; playback uses the delays; single state: tab and buttons greyed.
- [ ] **VER-5.05** (RAND-1 (N23); needs: bridge)  
  **Steps:** New document, Optimize panel, format JPEG (window ≈ 1100 × 800, default dock), then PNG 32 and AVIF.  
  **Expected:** "Interlaced"/"Progressive" fully visible, color table 2 px below; GIF unchanged.
- [ ] **VER-5.06** (BV-4; needs: real mouse)  
  **Steps:** Export Area tool: drag a handle and press Esc while holding the mouse.  
  **Expected:** Frame snaps back, dialog stays open.
- [ ] **VER-5.07** (BV-4; needs: real mouse)  
  **Steps:** Mouse wheel over the image at 400 %; Ctrl+wheel (also touchpad); Shift+wheel.  
  **Expected:** Wheel pans; one Ctrl notch = one zoom step; Shift+wheel does nothing.
- [ ] **VER-5.08** (BV-4; needs: bridge, images)  
  **Steps:** GIF options in de and en.  
  **Expected:** Checkbox "Interlaced browser display" not clipped.
- [ ] **VER-5.09** (E3; needs: bridge)  
  **Steps:** Edit ▸ Copy HTML Code → choose folder.  
  **Expected:** Clipboard holds the table, images are in the chosen folder.
- [ ] **VER-5.10** (E3; needs: bridge)  
  **Steps:** File ▸ Update HTML on an exported page edited by hand; then on a text file.  
  **Expected:** Own text kept, table replaced; text file: "… is not an HTML page …".
- [ ] **VER-5.11** (E3; needs: bridge)  
  **Steps:** Slice with a simple rollover, export via `file.export` (CLI/MCP).  
  **Expected:** Extra image `…_s2.png`.
- [ ] **VER-5.12** (NZ-4; needs: bridge)  
  **Steps:** Change CSS sprite options, save as FW-PNG, close, reopen; save a file without sprite values.  
  **Expected:** Same values in the options dialog (own extension `fpSv`); file without sprite values stays byte-identical.
- [ ] **VER-5.13** (NZ-4; needs: bridge)  
  **Steps:** Export with no previous export/open folder, untitled document, fresh profile.  
  **Expected:** Export dialog starts in "Pictures".
- [ ] **VER-5.14** (BV-5 (N25); needs: real mouse)  
  **Steps:** Large image (1200 × 900), 4-Up, zoom 150 %; in **each** pane: wheel (3 notches), Space+drag, Hand drag, Zoom click/Alt+click, Ctrl+wheel, zoom list; switch 2-Up ↔ 4-Up ↔ Original; same in File ▸ Image Preview, with one pane's preview off and one with an export area; DPI 125/150 %.  
  **Expected:** All panes always show the same section and zoom, none jumps or lags; switching keeps the centre; at 125/150 % the export image is as large as the original.
- [ ] **VER-5.15** (RT-2 (E1); needs: bridge, browser)  
  **Steps:** Drag a button from Library ▸ Common onto the canvas, File ▸ Export "HTML and Images", open in a browser; repeat with "Include Down state" (and Over-While-Down).  
  **Expected:** HTML uses `MM_swapImage`/`MM_swapImgRestore`, images `…_r?_c?_s2.gif`, rollover visible in the browser; with Down state `MM_nbGroup`, `_s2`/`_s3` (and `_s4`).
- [ ] **VER-5.16** (RT-2 (E3); needs: bridge)  
  **Steps:** Export document A (with slice) as "Images only"; File ▸ New, draw a slice, File ▸ Export.  
  **Expected:** Preselected type "HTML and Images".
- [ ] **VER-5.17** (RT-2 (E5); needs: bridge)  
  **Steps:** Export a 150 dpi document as BMP (24 and 32 bit); reopen.  
  **Expected:** Bytes 38–45 = 5904, byte 14 = 40; reopens at 150 dpi; 32-bit file keeps transparency.
- [ ] **VER-5.18** (RT-3 (D4); needs: bridge)  
  **Steps:** `re5b_test.fw.png`, export scale 50 %, 2-Up.  
  **Expected:** Right pane matches the original, rectangles 160 × 200.
- [ ] **VER-5.19** (RT-3 (E6); needs: bridge)  
  **Steps:** `sp.fw.png` (5 slices), export CSS sprites, grid, padding 10; PDF with JPEG compression, quality Medium.  
  **Expected:** Image 180 × 410, r3_c8 at 120,200; PDF quantization table first row 9 6 6 9 12 12 17 15. See FINDINGS E6 for the padding question.
- [ ] **VER-5.20** (RT-3 (E7); needs: real window)  
  **Steps:** Image Preview at 50 %, Export…, name, Enter; again with Esc in the save dialog.  
  **Expected:** File 600 × 450, Image Preview closes, document "*"; with Esc it stays open.
- [ ] **VER-5.21** (B-4 (BA-048); needs: real mouse)  
  **Steps:** 2-Up/4-Up preview: drag a frame with the Zoom tool.  
  **Expected:** Dashed, animated frame.
- [ ] **VER-5.22** (B-5; needs: real window, original)  
  **Steps:** Place a Library button (Bevel Button), File ▸ Export "HTML and Images".  
  **Expected:** Own cell sized to the active area (87 × 36), image and link like the original; canvas shows the green active area.

## 6. Menus and keyboard input

|---|---|---|---|---|
- [ ] **VER-6.01** (MN-3; needs: real mouse)  
  **Steps:** Right-click the "Layers" tab; ≡ menu; right-click the "Properties" tab, choose "Half Height".  
  **Expected:** Close / Close Tab Group work; the ≡ menu no longer has them, last entry "Help"; Properties: Full Height ✓, Half Height, Close; Half Height halves the Inspector.
- [ ] **VER-6.02** (MN-3; needs: bridge)  
  **Steps:** Select an object, Inspector "+" ▸ Options; "All Off".  
  **Expected:** Save as Style… │ All On, All Off │ Find Plug-ins…; All Off disables the effects.
- [ ] **VER-6.03** (MN-4; needs: bridge)  
  **Steps:** Start with an old profile whose `.firepixel-commands` has no `set-2` and old names.  
  **Expected:** Unchanged commands under new names, the 5 new commands present, no duplicates.
- [ ] **VER-6.04** (MN-4 / B-6 (BA-094); needs: real window)  
  **Steps:** Commands ▸ Demo – current document with an open document (shell open allowed).  
  **Expected:** The browser opens the preview of all pages.
- [ ] **VER-6.05** (MN-4; needs: real window)  
  **Steps:** File ▸ Browse in File Browser... (Ctrl+Alt+O).  
  **Expected:** The document's folder opens in Explorer.
- [ ] **VER-6.06** (MN-5; needs: bridge)  
  **Steps:** Language English, Commands ▸ Web ▸ Shared Palette (German or English folder); "Resize selected objects".  
  **Expected:** English message; prompt "New size in percent".
- [ ] **VER-6.07** (MN-5; needs: bridge)  
  **Steps:** Start with an old profile containing the MN-4 command folder.  
  **Expected:** The five new scripts and the five with prompts are updated to the `fp.t` version; scripts changed by the user stay.
- [ ] **VER-6.08** (MN-6; needs: real window)  
  **Steps:** Window 800 × 600 at 100 %: open File, hover the bottom arrow; click an arrow; wheel; click "Exit". 1024 × 768: Window menu.  
  **Expected:** Scrolls smoothly to "Exit", arrow greys out; click keeps the menu open; wheel scrolls; Exit works. Window menu ends at the window edge with an arrow bar, document list reachable.
- [ ] **VER-6.09** (MN-6; needs: real window)  
  **Steps:** Text ▸ Font (long list); right-click at the bottom of the canvas at 800 × 600; ≡ menu of a panel in a low window.  
  **Expected:** Arrow bars, style submenu opens visibly and shifted up; context menu shifted up, at most window height, last entry reachable; panel menu fully inside the window.
- [ ] **VER-6.10** (MN-6; needs: real keyboard)  
  **Steps:** Alt+F, then ↑; Home and End; open submenus near the right edge.  
  **Expected:** Highlight jumps to "Exit" and the menu scrolls; Home/End likewise; submenus open to the left when there is no space.
- [ ] **VER-6.11** (NZ-3; needs: bridge)  
  **Steps:** Text ▸ Font without text: pick a family with styles; pick a system font, restart Firepixel, create text.  
  **Expected:** ✓ at the family header; new text uses that font after restart.
- [ ] **VER-6.12** (PERF-1; needs: real window)  
  **Steps:** Thousands of objects (Ctrl+A): open a menu, switch menus by hover and by Alt / F10.  
  **Expected:** Entries appear immediately and are correctly enabled.
- [ ] **VER-6.13** (RT-1 (D6); needs: bridge)  
  **Steps:** Doc A with a saved bitmap selection, new doc B of another size, open the Select menu in B.  
  **Expected:** "Restore/Delete Bitmap Selection" enabled.
- [ ] **VER-6.14** (F18 (TW-50); needs: bridge)  
  **Steps:** T05/T09 with a slice: right-click; run the behavior entries; hotspot.  
  **Expected:** Export entries, Arrange ▸, behaviors block, "Exit Full Screen"; Behaviors panel lists them, "Delete All Behaviors" clears; hotspot without rollover and navigation bar.
- [ ] **VER-6.15** (F18 (TW-51); needs: bridge)  
  **Steps:** Filters ▸ Adjust Color.  
  **Expected:** 6 entries, no "Color Fill…".
- [ ] **VER-6.16** (F19 (TW-66); needs: bridge)  
  **Steps:** Pixel selection, Save Selection, Ctrl+D, Select menu; second document without frame and saved selection.  
  **Expected:** "Restore Bitmap Selection…" enabled, dialog opens; second document: greyed.
- [ ] **VER-6.17** (CF-B (CT-B-03); needs: real keyboard)  
  **Steps:** Three documents A, B, C with A active; Ctrl+Shift+Tab and Ctrl+Tab with a real keyboard.  
  **Expected:** Backwards A → C → B; Ctrl+Tab forwards.
- [ ] **VER-6.18** (CF-B (CT-B-12); needs: bridge)  
  **Steps:** Bridge `wheel` with `modifiers ["alt"]`, then Ctrl+2.  
  **Expected:** No menu highlighted, Ctrl+2 works.
- [ ] **VER-6.19** (CF-FP4 (FP4-04); needs: real keyboard)  
  **Steps:** Brush tool; tap Alt with a physical keyboard and via `keybd_event`; press "F"; press the tool key; Alt-drag an object.  
  **Expected:** Menu bar active, the access key opens File, the tool does not change; Alt-drag still duplicates.
- [ ] **VER-6.20** (F21 (TW-78); needs: real keyboard)  
  **Steps:** Ctrl+A inside Inspector field W.  
  **Expected:** Selects the field text, the object selection stays.

## 7. Batch Studio

|---|---|---|---|---|
- [ ] **VER-7.01** (BATCH-5; needs: real window)  
  **Steps:** Real phone photos (portrait, with GPS), template "For the web"; inspect in Explorer (Properties ▸ Details) and a color-managed viewer.  
  **Expected:** Output upright, no GPS, camera/date present; same colors (ICC kept).
- [ ] **VER-7.02** (BATCH-5; needs: real window)  
  **Steps:** Target size 100 KB over 20 mixed photos; 5 KB + "quality only"; PNG screenshots with `optimize.png`; open TIFF (LZW/Deflate) and TGA output in another program.  
  **Expected:** All ≤ 100 KB, report without warning; 5 KB: warning with the reached size; PNG 10–40 % smaller with identical pixels; TIFF/TGA open fine.
- [ ] **VER-7.03** (BATCH-6; needs: bridge)  
  **Steps:** Template "Remove background" on 5 product photos; change tolerance/expand; empty the model folder and run again.  
  **Expected:** Transparent PNG, clean edges, no light fringe; preview updates without a new model run, result = preview; without model: card warning "AI model missing … Manage AI Models…", each file "Error", no download attempt.
- [ ] **VER-7.04** (BATCH-6; needs: bridge)  
  **Steps:** Watermark "© {year} {name}" bottom right (60 %) and tiled at −30° on portrait and landscape, de/en; logo from an embedded recipe (save/load with "Embed image files").  
  **Expected:** Correct placement and text; embedded logo loads.
- [ ] **VER-7.05** (BATCH-7; needs: real window)  
  **Steps:** With ffmpeg installed: a portrait phone MP4/MOV, `video.import` 0:01–0:03 at 10 fps → GIF; cancel while reading a long video.  
  **Expected:** 20 frames, 2 s, portrait, **no console window**; cancel kills `ffmpeg.exe` at once, no `.fptmp`.
- [ ] **VER-7.06** (BATCH-7; needs: bridge)  
  **Steps:** Without ffmpeg: add a video to the list.  
  **Expected:** Error row "ffmpeg not found – choose the path in the settings." (de/en).
- [ ] **VER-7.07** (BATCH-8; needs: real window)  
  **Steps:** GIF → GIF and video → GIF with `preset` smallest/balanced/best and `maxKB`; play in browsers and in Firepixel.  
  **Expected:** No leftovers in alpha animations, no flicker in static backgrounds, loop correct, same duration.
- [ ] **VER-7.08** (BATCH-9; needs: bridge)  
  **Steps:** Recipes ▸ Templates; "Trace logos → SVG" with 3 logos; Start; method "Smooth curves".  
  **Expected:** 11 templates (de/en, nothing clipped); card "Trace → SVG" with preset/method, preview shows paths, "After" size; SVGs in `batch-output` contain only paths; smooth curves → fewer nodes.
- [ ] **VER-7.09** (BATCH-9; needs: real window)  
  **Steps:** `firepixel.exe --batch logos-to-svg C:\logos`; `firepixel.exe --batch C:\r\my.fpbatch.json a.png`.  
  **Expected:** Studio in front with recipe and files; recipe file loaded (title without •).
- [ ] **VER-7.10** (CF-C (CT-C-01); needs: real window)  
  **Steps:** 120 JPEGs 2400 × 1600, recipe with "Remove background", start, press "Cancel" after 2–3 s; run once without cancel.  
  **Expected:** End card "Cancelled" within seconds, no `.fptmp`; without cancel it completes; memory drops afterwards.
- [ ] **VER-7.11** (CF-C (CT-C-08/09); needs: bridge)  
  **Steps:** Card "Save as" WebP; Ctrl+A outside a text field.  
  **Expected:** Header and field 82; Ctrl+A turns all files on, never off.

## 8. Themes, languages and image series

|---|---|---|---|---|
- [ ] **VER-8.01** (PINSEL-1; needs: images)  
  **Steps:** Dialog "Edit Brush Stroke" in de/en/qps and 3 themes.  
  **Expected:** Nothing clipped or overlapping.
- [ ] **VER-8.02** (FARB-1b (9); needs: images)  
  **Steps:** Color pop-up from the toolbar, Inspector stroke, Glow, New Document, Mixer in 3 themes × de/en.  
  **Expected:** Same layout, footer per call, nothing overflows ("HEX:"/"RGB:" in en).
- [ ] **VER-8.03** (RAND-2a (N23); needs: real window)  
  **Steps:** Window 1024 × 768 at 150 % (800 × 512 pt), Text tool; wheel over the Inspector.  
  **Expected:** Thin scroll bar at the bottom of the Inspector, "Auto kern" fully visible, wheel scrolls until the filter fields appear.
- [ ] **VER-8.04** (RAND-2a (N23); needs: real window)  
  **Steps:** Same narrow window, select a slice; then maximize.  
  **Expected:** "Edit..." does not touch "Target:"; maximized: Inspector unchanged (no bar, nothing moved).
- [ ] **VER-8.05** (BV-2; needs: images)  
  **Steps:** Image Preview options for GIF (with selected color), JPEG, PNG 32, WBMP, palette menu open, dialogs "Color" and "Optimize to Size", 3 themes × de/en, plus 800 × 600.  
  **Expected:** Nothing clipped or overlapping.
- [ ] **VER-8.06** (BATCH-9; needs: images)  
  **Steps:** Templates menu and card "Trace → SVG" (pro mode) in 3 themes × de/en.  
  **Expected:** Nothing clipped.
- [ ] **VER-8.07** (B-4 (BA-064/065); needs: images)  
  **Steps:** Re-shoot the dialog design series (dialogs with empty fields can now get a footer) and the start page with the row "Batch Processing…".  
  **Expected:** Footers correct; start page row present.
- [ ] **VER-8.08** (B-4 (BA-062); needs: images)  
  **Steps:** Manage AI Models at 1024 × 768 / 150 % and in qps.  
  **Expected:** Model column, "…" with tooltip.

## 9. Measurements in the reference application

These items need the reference application side by side. Measure, write down the values, and if Firepixel differs, open a "Parity gap" issue.

|---|---|---|---|
- [ ] **VER-9.01** (FV-14 (L4/L35))  
  **Measure:** Snapping: R1 50,50–150,120, E1 200,50–300,120, G 350–470 × 50–100; drag from 100,85 by +0/+3, +47/0, +0/+22, +0/−7, +0/+17, +97/0; Pen clicks at 153,20 · 253,20 · 585,53 · 585,77.  
  **Firepixel today:** Edge-to-edge and centre-to-centre snap, edge-to-centre does not; Pen x 150, x 253, y 50, y 77 (`tools\snap.rs`).
- [ ] **VER-9.02** (FV-14 (L33))  
  **Measure:** Path 320,240 · 400,240 · 480,240 (corner points), subselection; Alt-drag the middle point to 400,270, the end point to 510,240, the start point to 290,240.  
  **Firepixel today:** Only the outgoing handle follows the pointer, the incoming handle stays.
- [ ] **VER-9.03** (FV-14 (L34))  
  **Measure:** Repeat L31–L33 and read `fw.selection` before the Pen click.  
  **Firepixel today:** A new path starts only when the path is no longer selected.
- [ ] **VER-9.04** (FV-14)  
  **Measure:** Transparent bitmap pixels: click the corner of an ellipse bitmap (404,254 in 400,250–500,330); Shift-, Ctrl- and subselection-click in a gap; drag from the gap. Take paired screenshots of L4, L13, L25, L33, L35.  
  **Firepixel today:** All select or move the bitmap; nothing outside the frame.
- [ ] **VER-9.05** (FV-16)  
  **Measure:** Gradient drag: fill #808080, stroke #C04040, select a rectangle, drag the Gradient tool across, read `fill.gradient.nodes`; on a bitmap with "Fill selection" off: does the gradient fill only the color area at the start point (tolerance)?  
  **Firepixel today:** Black → white (`FillTool`, `bitmap.gradient`).
- [ ] **VER-9.06** (AU-3)  
  **Measure:** Texture amount 0: fresh rectangle, set a texture via `setFillNColorNTexture` without amount, read the amount.  
  **Firepixel today:** Amount unchanged (0 = invisible for new fills).
- [ ] **VER-9.07** (FV-15 + AU-3 (F7))  
  **Measure:** Doc 500 × 300, R1 red at 50,50–150,120, R2 blue at 250,50–350,120. A: type opacity "30" without Enter/Tab, click R2; B: Shift+click R2; C: width "60"; D: press and hold on R2, screenshot before release; E: click empty space. 3× each, log the history.  
  **Firepixel today:** If 30 lands on R1, Firepixel is right; if on R2, adjust `canvas\mod.rs` / `inspector\fields.rs`.
- [ ] **VER-9.08** (DLG-6)  
  **Measure:** Motion Blur angle 0, amount 20 on B1.  
  **Firepixel today:** 301,200,420,300 (left edge 19 px). If the original shows 300,200,420,300, `blur.rs` must be adjusted.
- [ ] **VER-9.09** (DLG-8)  
  **Measure:** Bounds after Expand Marquee by 1/5/10 and Contract by 5.  
  **Firepixel today:** +1/+6/+11, contract +6 with soft edge.
- [ ] **VER-9.10** (DLG-11 (D10))  
  **Measure:** File ▸ File Info: tab names in order, field labels with type/position/size, buttons, initial focus, Esc with a real key, resizing (screenshots without product names).  
  **Firepixel today:** Basis for a follow-up task.
- [ ] **VER-9.11** (FORM-1)  
  **Measure:** Tab order of D45, D47 (Motion Blur), D48 (Gaussian), D49, D50.  
  **Firepixel today:** Fields › OK › Cancel › Preview (`with_side_order` in `catalog.rs`).
- [ ] **VER-9.12** (FORM-1)  
  **Measure:** Expand Marquee, "500", Enter: does the dialog stay open with the field focused and selected?  
  **Firepixel today:** Yes.
- [ ] **VER-9.13** (FX-2)  
  **Measure:** Color of a new "Outer Bevel" added via the menu: fill color or `#df0000`?  
  **Firepixel today:** Fill color of the tool.
- [ ] **VER-9.14** (NZ-5)  
  **Measure:** "Open as Animation" with files of different sizes and with symbols: canvas size (first file or union?) and do the other files' symbols enter the library?  
  **Firepixel today:** First file's size; symbols not copied.
- [ ] **VER-9.15** (MN-5)  
  **Measure:** Optimize ▸ Delete Settings: buttons of the prompt (Yes/No or OK/Cancel); is a preset chooser shown first?  
  **Firepixel today:** See VER-4.12.
- [ ] **VER-9.16** (NZ-4)  
  **Measure:** Open an FW-PNG with sprite values (extension `fpSv`).  
  **Firepixel today:** Expected to open without error, unknown chunk skipped (not measured).
- [ ] **VER-9.17** (NZ-4)  
  **Measure:** Save 72 and 96 ppi documents as BMP and FW-PNG, read the header value (pHYs / BMP header): truncated (2834) or rounded (2835)?  
  **Firepixel today:** Firepixel still rounds for BMP and FW-PNG.
- [ ] **VER-9.18** (RE-BATCH (M1))  
  **Measure:** Batch file selection: window list of the "Batch" dialog with IDs and rectangles; add 3 files, one twice; "Add All" in a folder with 5 PNGs; screenshot of the list.  
  **Firepixel today:** Basis for the batch dialog parity (display, duplicates, visibility, enable rules for Next/Remove).
- [ ] **VER-9.19** (RE-BATCH (M2))  
  **Measure:** Fresh profile: file selection → Next; window list in the initial state.  
  **Firepixel today:** Initial contents, selection, focus, checkbox state, meaning of `BatchWhichFiles`.
- [ ] **VER-9.20** (RE-BATCH (M3))  
  **Measure:** Task page: add each task once, then again; one command twice; double-click left/right; ▲/▼ at first/last position.  
  **Firepixel today:** Multiple add, removal from the left list, double-click, grey rules.
- [ ] **VER-9.21** (RE-BATCH (M4))  
  **Measure:** Expand the folder "Commands"; list the entries.  
  **Firepixel today:** Tree or flat, which folders, effect of `//--not for batch--`.
- [ ] **VER-9.22** (RE-BATCH (M5))  
  **Measure:** Export page: expand the list (screenshot); "Edit" → which dialog; close with OK.  
  **Firepixel today:** List contents/separators, new list entry, "Custom...".
- [ ] **VER-9.23** (RE-BATCH (M6))  
  **Measure:** Scale page: all 4 modes, sliders to the end; type 0, 5000, −3 and switch pages.  
  **Firepixel today:** Visibility, limits, slider range.
- [ ] **VER-9.24** (RE-BATCH (M7))  
  **Measure:** Step 2: "Custom location" (does a folder dialog open immediately? title?), cancel; backups on/off.  
  **Firepixel today:** Enable rules, folder dialog.
- [ ] **VER-9.25** (RE-BATCH (M8))  
  **Measure:** Change settings, close the wizard with Cancel, reopen; then "Save Script" (title, default name, filter); does the wizard stay open? Compare the settings file before/after.  
  **Firepixel today:** What is remembered when.
- [ ] **VER-9.26** (RE-BATCH (M9))  
  **Measure:** Small run (2 PNG, 1 broken file), export GIF to its own folder; photograph the progress window during/after; find the batch log file.  
  **Firepixel today:** Progress UI, OK/Cancel states, log location and format.
- [ ] **VER-9.27** (RE-BATCH (M10))  
  **Measure:** Run "Export JPEG" on `a.png` in the same folder twice.  
  **Firepixel today:** Output name/extension, overwrite without prompt, backup folder "Original Files".
- [ ] **VER-9.28** (RE-BATCH (M11))  
  **Measure:** Look only at the data part of the script saved in M8 (keep it private).  
  **Firepixel today:** Bracket form of the operation list, URL form of `outputDir` and `commandfile:`, non-ASCII path as `String.fromCharCode`.
- [ ] **VER-9.29** (RE-BATCH (M12))  
  **Measure:** Run the saved script via Commands ▸ Run Script…, click "...".  
  **Firepixel today:** Which file dialog appears.
- [ ] **VER-9.30** (RE-BATCH (M13))  
  **Measure:** Run "rename only" (prefix) in the same location and in an own folder; run "rename + scale" without export in the same location.  
  **Firepixel today:** Confirmation prompt (move or copy, two files).
- [ ] **VER-9.31** (MN-5)  
  **Measure:** Does Ctrl+Shift+K (Disable Mask) work although no shortcut text is shown?  
  **Firepixel today:** If not, remove the key from the Firepixel set too (`shortcuts.json`).
- [ ] **VER-9.32** (B-3 (BA-039))  
  **Measure:** Brush: new bitmap on a locked active layer; effect of the `BezFit*` preferences.  
  **Firepixel today:** New bitmap on the next free layer below (no message); brush strokes without curve fitting.
- [ ] **VER-9.33** (B-4)  
  **Measure:** Does Image Preview close after export (BA-054)? Blend-mode list with 56 names in the brush pop-up (BA-055)? Default shortcut set and the deviations Ctrl+Alt+H, Ctrl+Shift+O, Ctrl+Shift+K (BA-059).  
  **Firepixel today:** Not measured.
- [ ] **VER-9.34** (B-5)  
  **Measure:** Paired runs of the dialog/menu units D31–D40, File and Edit menus: `toSource` identical character by character; `name` of a slice from `addNewHotspot("slice")` (original shows "_r2_c2"); `reorderLayer` of a sub-layer to a top-level layer.  
  **Firepixel today:** Gaps listed in the run results.
- [ ] **VER-9.35** (B-7)  
  **Measure:** `rawHeight` of text on a path; pHYs with unit pixels/cm; `createFirepixelDocument` with `pixelsPerUnit` / `units: "centimeter"`.  
  **Firepixel today:** Text on path always 5.
- [ ] **VER-9.36** (F20 (TW-70))  
  **Measure:** P1 selected, open Modify ▸ Alter Path with the mouse, press Esc 3×; measure the selection after each Esc.  
  **Firepixel today:** Mouse: 1/0/0; keyboard: 1/1/0.
- [ ] **VER-9.37** (T-5)  
  **Measure:** Text 72 pt "Fx Ag" with Noise: compare the noise pattern (buffer 11,36–257,148) as a paired image.  
  **Firepixel today:** Same buffer bounds.

## 10. Pending scripted and automated runs

These items do not need a human; they are listed so that they are not lost. They are picked up by the next full test run, image run or comparison-runner pass.

|---|---|---|
- [ ] **VER-10.01** (PERF-1) `select_all_perf::select_all_3000 --release` before and after the PERF-1 change; compare timings.
- [ ] **VER-10.02** (FX-4) End-to-end run of effect group 4 via `firepixel-cli script`: all vector and bitmap cases max ≤ 1.
- [ ] **VER-10.03** (NEUTRAL-1) `cargo test -p fp-neutral` green; a screenshot that shows a blocked name turns `screenshots_show_no_blocked_names` red (remove it afterwards).
- [ ] **VER-10.04** (BATCH-1) Large run (≥ 1000 mixed photos) on all cores: memory stays below the limit; cancel leaves no `.fptmp`; `batch-log.txt` under `%LOCALAPPDATA%\Firepixel\Logs` is appended (English).
- [ ] **VER-10.05** (BATCH-2) Full runs that trigger all menu commands: `file.batchProcess` opens the Studio; close it (`st.batch = None`) or exclude the command; drop the obsolete clipping-audit allowances for it.
- [ ] **VER-10.06** (BATCH-4) CLI recipe `resize longEdge 1600` + `filter.sharpen` + JPEG on phone photos: outputs ≤ 1600 px, sharp, EXIF orientation correct, nothing rotated twice.
- [ ] **VER-10.07** (BATCH-4) `effect.live` dropShadow on transparent PNG logos (canvas grows; same size with "Expand canvas" off); `script.command` with a saved command works, one marked `//--not for batch--` is refused.
- [ ] **VER-10.08** (BATCH-6) CLI `firepixel-cli batch --recipe tools\golden\batch\overlay-image-tiled\recipe.json --in <photos>`.
- [ ] **VER-10.09** (BATCH-7) `cargo test -p fp-batch --test anim_video real_ffmpeg_if_installed -- --nocapture` with ffmpeg installed → 10 frames.
- [ ] **VER-10.10** (BATCH-8) `cargo test -p fp-batch --test gifopt_measure -- --ignored --nocapture` (release build for timings).
- [ ] **VER-10.11** (BATCH-9) MCP `batch {"action":"templates"}`, `preview` returns two images, async run with `status` until done; `cancel` during a large run leaves no `.fptmp`.
- [ ] **VER-10.12** (CF-C (CT-C-04)) `firepixel-cli batch --recipe web-webp --in <folder> --recursive` twice: run 2 "7 done", `batch-output-2` without a nested `batch-output`.
- [ ] **VER-10.13** (B-6) Full run: menu report shows File ▸ Batch Processing… as passing (Studio), entries after View ▸ Language in the right language; `width_field_limited_to_6000_with_message` and `batch_ui::trockenlauf_und_lauf_zweimal` green in the whole run; Commands ▸ Demo listed as "check in the real window".
- [ ] **VER-10.14** (B-7) Script checks: import 320,158–370,196 (type Image, same bounds); auto line raw 50,400,350×5 (same after reopen); animation 20,−16,182,100 with frames = 1; new document + `addNewPage` → `exportOptions` PNG 32 bit; pHYs 2834/3779/5905/11771.
- [ ] **VER-10.15** (F20 (TW-68)) Unit `panel-info-align-path/06_*`, R1, E1, PG, "To canvas" off: centre horizontally → E1 242…343, PG 257…328; distribute bottom edges → E1 top 113; same height → PG 465…534.
- [ ] **VER-10.16** (T-8 (TW-65)) `menu-modify-a/09` Animate Selection → instance without name; `/10` Convert to Symbol with visible Inspector → "Symbol"; with the Brush tool no name.
- [ ] **VER-10.17** (T-8 (TW-67)) `panel-info-align-path/06_*` distribute/spacing with ties (R1/E1 x = 50, E1/PG centre y = 205) → same object moved as in the original.
- [ ] **VER-10.18** (T-8 (TW-75)) `inspector-bitmap/03` B1 480 → 240 via W/H → bitmap pixel-equal.
- [ ] **VER-10.19** (F21 (TW-78)) `xs-shortcuts/16`, `inspector-bitmap/03`, `inspector-web/11`: Ctrl+A in field W selects the field text (confirm once in the real window, VER-6.20).
- [ ] **VER-10.20** (F21 (TW-80)) `tool-path-reshape/11` → message "The path scrubber tools …" with OK and "Don't show again", rectangle unchanged.
- [ ] **VER-10.21** (F21 (TW-84/TW-87)) `inspector-group-symbol/04`, `panel-misc/16`: column widths of Symbol Properties (Name 35 px vs. ≈ 18 px in the original?); `inspector-web/01` checkerboard tile 8 px.
- [ ] **VER-10.22** (F21 (TW-69/TW-85)) `panel-layers/04` with intermediate drag steps → all rows passed over are hidden, one history step.
- [ ] **VER-10.23** (F22 (TW-79)) `xs-file-formats/05` open PSD → path `pixelRect` 9,9–90,90.
- [ ] **VER-10.24** (F22 (TW-70)) `menu-modify-b/17`, `menu-text/19`: menu by mouse, Esc step by step → selection stays until the last Esc.
- [ ] **VER-10.25** (K-1 (FP1-02)) `export-results/01` PNG32 → row filters 0/3/285/0/12 like the original.
- [ ] **VER-10.26** (F23 (TW-88/TW-90)) `tool-autoshapes/14` (Shift squaring), `tool-autoshapes/04,13` (tooltips); see VER-1.31/1.32.
- [ ] **VER-10.27** (K-7 (TW-89)) `xs-file-formats/09` import drag; see VER-1.33.
- [ ] **VER-10.28** (T-10 (TW-98/Q-45)) `xs-canvas/12` → 0,20 like the original (tolerance `SnapDistance` 5/zoom), `view_zoom` and `drag` of the original in the values, no focus disturbance.
- [ ] **VER-10.29** (T-10) `xs-file-formats/09` drag 100,100 → 420,340 → target rectangle right 435, result 121,120–417,322 on both sides, same drag points.
- [ ] **VER-10.30** (T-10 (Q-44)) `panel-layers/09` → both sides unchanged (drop into locked layer 3 rejected); `/15` both unchanged (web layer not movable); in the window: drop zones top/bottom 6 px, drag only from the name.
- [ ] **VER-10.31** (R-7) The 9 under-measured units with ≥ 60 % measurement rate (new count without `unsupported`/`skipped`); focus `+stable`, retries with `retry_of`, `xs-canvas/08…12` not "ok" with the wrong tool; "default colours … tools button" works (auto shapes #999999 with stroke).
- [ ] **VER-10.32** (R-7 (TW-97/TW-99)) `xs-shortcuts/13` tool and selection logged before the drag, Shift-drag moves R1, Alt-drag 8 objects on both sides; `panel-library-styles/03` 4|3 → 5|3 on both sides, instance 571,371–631,431.

## 11. User-reported fixes awaiting a real-window check

|---|---|---|
- [ ] **N01**  
  **Steps:** Apply the live effect Glow, change its size several times, then Cancel; Ctrl+Y.  
  **Expected:** The effect is not stacked; Cancel leaves no redo step; Ctrl+Y does not bring the effect back.
- [ ] **N21**  
  **Steps:** AI "Select Subject" on a photo; cancel in the middle of a run; Dark/OLED and English.  
  **Expected:** Progress card with time and Cancel; cancelling works mid-run; marching ants visible; fine-tuning fields in the Inspector apply at once.
- [ ] **N27**  
  **Steps:** Color pop-up open, move the eyedropper over panels, the menu bar and a dialog.  
  **Expected:** Live color of what is under the pointer (see VER-2.05).
- [ ] **N34**  
  **Steps:** Grey rectangle below a green one; drag the grey one.  
  **Expected:** It stays behind the green one while dragging (stacking order kept).
- [ ] **N35**  
  **Steps:** Two vector rectangles (large + small green on top); cut the green one (Ctrl+X), select the large one, Paste Inside (Ctrl+Shift+V).  
  **Expected:** **One** object "Rectangle" with thumbnail, chain and mask thumbnail; the large rectangle keeps its fill, the green one sits inside it.
