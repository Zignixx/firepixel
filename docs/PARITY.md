# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (follow-up run 44 over 4 units on top of full run 43, 24 Oct 2026, all 49 units; TW-231 settled by the hand check CHK-1):**
**49 equal except for design**, 0 real deviations, 0 unclear, 0 under-measured.
**Proven 1:1: 100.0 %** (49 / 49; all 49 units are sufficiently measured). Measurement quota over all 49 units:
**88.0 %** (1095 / 1244 steps). Canvas image pairs within 1 %: **195 / 224**. Run 44 repeated `panel-pages-states-history`,
`panel-layers`, `panel-layers-menu` and `inspector-bitmap` (92 steps) with runner R-36 and the new F100 setting
`fp.layers_group_tree` off (Firepixel's Layers options ▸ Show Group Contents switched off at every unit start) on preview
`2026-10-24_16-54` (`main` at `28691c50`: v0.2.12 plus tester feedback batch 8, F98 to F100). No step status, no class
and no compared value changed; every pair reads its run 43 value except where batch 8 shows. Batch 8 where the runner
reaches it: a selected object's own Layers thumbnail is framed blue as in the reference (F100, TW-238; 9 shots with the
Layers rows, 2 px in #0078d7 against the reference's #00a8ff), and the Layers options menu carries the extra entry Show Group
Contents, unchecked (TW-237; `panel-layers-menu/01`, `02` 15.6 → 23.4 %). Not reached: F98's page placement, names and
History row "Change Page Selection" (TW-233; no History step clicks a page row, the page count of
`panel-pages-states-history/03` reads as before), F99's strokes at the bitmap bounds (TW-234, TW-235), the Confine
checkbox (TW-236; no shot holds a retouch or brush tool's inspector) and the row highlight during mask editing (TW-238;
no step selects a mask in the Layers panel). `panel-layers-menu/10_options_open` fails on Firepixel's side as in every
run since run 17 (the row double-click opens no dialog); it does not use the options menu, so the new entry plays no
part. The reference was restarted once (an access violation while opening the test file at the start of
`inspector-bitmap`, no step lost).

**Units: 49.** Over the 48 units classified before run 24 (without the measurement unit `xs-select-behind`, added on
19 Oct 2026): 48 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.9 % (1086 / 1235; run 17 alone: 86.3 %), canvas pairs
186 / 215.

## Method: scripted comparison runs

The parity runner (`tools/parity/runner/`, see its README) plays the same step script in **both programs** on
one Windows machine, unattended:

1. **Units.** The UI is cut into 49 units (one tool group, one panel, one menu, ten dialogs …). Each unit has a
   JSON script in `tools/parity/scripts/<unit>.json` with about 20–40 steps (about 1305 steps in total; the
   measurement unit `xs-select-behind` has 9) and a test
   document (T01…T10, created for this purpose, not shipped).
2. **Steps.** A step performs actions (menu, click, drag with modifiers, key, field input, script call) on both
   sides, then records: a screenshot of a defined area (canvas, panel, dialog, menu, popup), values read
   back through the scripting API (`expr`: document tree, object bounds, colours, layer order …), dialog,
   menu and tooltip texts, and exported files. The reference application is driven with real input
   (`SendInput`) and its own script interface; Firepixel only through the MCP bridge (`ui_input`, `ui_action`).
3. **Comparison.** Per step: status of each side (`ok`, `error`, `unsupported`, `unmapped`, `skipped`); values
   compared one by one; texts as word sets; export files by pixels, colour depth, frame count and PNG row
   filters; every image pair with an offset search (± 3 px) and the share of pixels deviating by more than
   32 levels. Canvas pairs below 1 % count as equal. The runner's own `tools/parity/compare.py` writes only the
   share at a 16-level threshold without offset search (`cmp/summary.csv`: mean and maximum difference, differing
   pixels, share, `equal`, size notes); the offset search and the 32-level share are recomputed in the evaluation.
4. **Evaluation.** A script-only evaluation per run (no eyeballing except a handful of sampled pairs) classifies
   each unit and lists new deviations as tweaks (`TW-…`) and open questions (`Q-…`). Deviations become findings
   ([FINDINGS.md](FINDINGS.md)); non-trivial ones are first written down as a behaviour description from
   measurements, then fixed with a test that pins the measured values.

Full runs over all 49 units take about four hours; follow-up runs repeat only the affected units. A run's
state for a unit is always its newest run.

**Unit start (runner R-32).** Besides the earlier resets (documents closed, layout and dock mode, `MaxUndos`,
colours, tool stroke, text tool), the unit-start layout check reads the reference's Properties inspector and
double-clicks its tab until it is at its expanded height (125 px; it cycles expanded → half → tab row only), logged
as "orig Properties inspector … px -> … px" (TW-197). The text tool reset sets the alignment to left on both sides
(the reference keeps the alignment last chosen in its Properties panel, e.g. by `inspector-text/15`; TW-211).
View ▸ Edges stays **off** on both sides: `view_defaults` has switched it off at every opened test file since the
first run, so selection frames and handles are not in the canvas shots of either program; switching it on would
shift the baseline of every canvas pair, so steps that judge selection marks switch it on themselves
(`setShowEdges(true)`, as `xs-select-behind` does), and the reference is left with it off.

**Host fixes in R-32 and the pairs they invalidate.** Until run 36 the reference side's `set_field` wrote the value
with WM_SETTEXT before typing it; fields that apply only a changed text (the Properties panel's leading, paragraph
indent and space before / after) then saw no change and committed nothing. Run 36's
`inspector-text/09_leading_result`, `13_indent_result` and `14_space_before_after_result` (and the same steps in every
earlier run) therefore compared Firepixel's edited text with the reference's unedited one: the reference's exports
carry the unchanged document's hash in all three. The run 36 note that both programs had indented TX2 before F69 holds
for Firepixel only. `inspector-fields/05`, `06` and `08` have no pair; there the reference's document now reads
modified. R-32 types first and uses WM_SETTEXT only when the typed text did not arrive (logged as
`typed_text_missing`). The colour pop-up's `popup.hex_field` was the pop-up's first Edit control, which is the alpha
field: since run 24 `color-popup/05_hex_field_result` and `_invalid_result` typed the hex value into the alpha field
and read the reference's fill #80808000 (alpha 0), so those two values never compared the HEX entry; R-32 takes the
Edit right of the "HEX:" label. Units not repeated since R-32 keep their older rows; where they set Properties fields
on the reference side, the next full run re-checks them.

**R-32 in run 37.** The reference's Properties inspector read 125 px at all nine unit starts and at the run end
(nothing to restore; TW-197), the text tool reset found the reference right-aligned once, at the start of
`inspector-fields` after `inspector-text/15`, and set it left (TW-211), and the Layers row lookup expanded a collapsed
parent layer on Firepixel's side three times ("Ebene 1" in `panel-layers/07` and `14`, "Ebene 3" in `08`; TW-210).

**R-33 in run 38.** The new verb `layers_disclosure` set "Ebene 1" and "Ebene 3" expanded and "Gemeinsam", "Unterebene"
and "Ebene 2" collapsed by script on both sides before `panel-layers/09_drag_reorder_*`, `12` and `15` (four uses) and
found Firepixel's panel already showing the document's flags each time (no triangle clicked; F79); the Layers row
lookup expanded no parent layer (run 37: three times, TW-210). The layer drags logged both rows in view (the
reference's list at top index 0 in all five drags, Firepixel's list from the "Webebene" row in `09`, `12` and `15`),
except Firepixel's `04_eye_drag_result`, where its "Ebene 1" row was cut by the list's edge (logged; the eye drag's
values agree). The selection dot was found
on both sides (the reference's ring in the row image at x 1473, Firepixel's painted dot at x 1582.5), and `08` logs the
selected names (`1: L1obj` on both sides after the Shift-click). The inspector, alignment, tool stroke and Tooltips
resets behaved as in run 37.

**R-34 in run 39.** The Layers row targets (`{layer_row}`, `{object_row}`, `{layer_name}` and the name behind
`{layer_expand}`) come from the Layers panel only (`layers_row_scan`, then the expand fallback; Q-66). In
`panel-layers/08` the Ctrl-click on L1obj after the Shift-click now reaches its row in Firepixel's 268 × 291 px panel,
and Firepixel reads `0: ` like the reference (run 38: the window-wide lookup had found the Properties inspector's name
field). `layers_disclosure` found Firepixel's panel showing the document's flags at all four uses, the row lookup
expanded no parent layer, the layer drags logged the list positions of run 38 (Firepixel's "Ebene 1" row again cut by
the list's edge in `04_eye_drag_result`), and the selection dot was found at the run 38 places on both sides (x 1473
and x 1582.5). The right-click on the "Ebene 1" row in `panel-layers-menu/03` opens Firepixel's layer context menu 24
px higher than in run 38 (shot top 29 against 53), with the same entries and states. The inspector, alignment, tool
stroke and Tooltips resets behaved as in run 38.

**R-35 in run 40.** Five new `tool-text` steps scale the point text TS "Ag" (ArialMT 60 pt at 100,100 in T01, made by
script on both sides) and log its size, horizontal scale, runs, matrix, raw rect, bounds and ink before and after, the
last History rows and the state after the undo: `26` by the Numeric Transform dialog (150 %, Enter), `26_fresh90` a
fresh 90 pt text for the pixel comparison, `27` by Modify ▸ Transform ▸ Scale (the frame shot, then the bottom-right
corner dragged by +39/+38 px from each side's own handle, the reference's at 184,182 as measured by RE-34, Firepixel's
at 176,174 on the object's bounds, Enter; no script inside the transform session, which a script would end in the
reference) and `28` by `scaleSelection(1.5, 1, "autoTrimImages transformAttributes")` on both sides. All five were `ok`
on both sides, and every undo gave 60 pt at raw 100,100 again. The reference side's pop-up slider lookup now waits
2.5 s and then clicks the arrow once more or replays the step's open chain (value `popup_slider_retry`, log line "orig
popup slider: none open …"): it fired once, in `fx-bevel-emboss-fill/09_live_pressed` (the arrow clicked again, the
slider open after 0.42 s); `color-popup/23_alpha_slider_drag_canvas_pressed` and `inspector-fields/14` found their
sliders within the wait. The inspector, alignment (left at all four unit starts), tool stroke ("same" at all four) and
Tooltips ("off already") resets behaved as in run 39.

**R-36 with the F90 settings in run 43.** Since F90 (`e27198cb`) Firepixel's View ▸ Tooltips switches its widget
tooltips (on by default again) and the coordinate tag next to the pointer has its own switch View ▸ Coordinate Tag
(default on). The runner now sets `view.coordinateTag` to config `fp.coordinate_tag` (default off) and `view.tooltips`
to `fp.view_tooltips` (default on) after every start; in run 43 it logged "View > Coordinate Tag: switched on -> off" and
"View > Tooltips: on already" at all 49 unit starts. Until run 42 View ▸ Tooltips was off, which also hid the tag (R-28),
so canvas shots carry no tag in either setting; no hover shot of run 43 shows a new Firepixel tooltip (the tooltip
steps of `toolbox/03` are pixel-identical to run 35). The visible effect is in the View menu: Firepixel lists
View ▸ Tooltips checked (the reference's stays unchecked; the runner does not touch it) and the new Coordinate Tag entry
(`menu-view/10_guides_menu_result` 40.9 → 45.0 %). The other resets behaved as in run 42: the reference's Properties
inspector read 125 px at all 52 unit starts and restarts (TW-197; found at 56 px at the run end after
`xs-windows-tabs` and expanded), the text alignment was found right once, at the start of `inspector-web` after
`inspector-text/15`, and set left (TW-211), `layers_disclosure` found Firepixel's panel showing the document's flags at
all four uses and no row lookup expanded a layer (TW-210, TW-216), `transform_handles` derived the run 42 frame
(TW-221), and the tool stroke was "same" at 48 unit starts ("not set" on both sides at the start of `xs-file-formats`
after the planned restart).

**Tester feedback batch 8 (24 Oct 2026, follow-up run 44).** F100 (`d7b80e77`) adds Firepixel's Layers panel switch
Show Group Contents (`panel.layers.groupTree`, expandable group and mask rows, default on, TW-237); the runner sets it
to config `fp.layers_group_tree` (default off) after every start, next to the Coordinate Tag switch, so the Layers
panel lists the reference's rows. In run 44 it logged "Layers options > Show Group Contents: switched on -> off" at all
four unit starts, and the Layers rows read as in run 43 (`layers_disclosure`
found the document's flags at all four uses, no row lookup expanded a layer). Seen in run 44: the Layers panel menu
carries the extra entry Show Group Contents, unchecked, under Thumbnail Options (the panel-menu shots differ there, like
View ▸ Coordinate Tag), and a selected object's thumbnail is framed blue as in the reference (TW-238). Not reached: the
row of a masked object kept highlighted while its mask is edited (TW-238; no step selects a mask in the Layers panel),
F98's page placement, names and History rows (TW-233; the History steps of `panel-pages-states-history` click no page
row), the strokes at the bitmap bounds of F99 (TW-234, TW-235; no unit paints with these tools) and the Confine checkbox
of TW-236 (no shot of run 44 holds the inspector of Blur, the Rubber Stamp, the Brush or the Pencil).

## What the classes mean

| Class | Meaning |
|---|---|
| **equal except for design** | The run finds no wrong value, no missing element and no canvas deviation in this unit. Windows, menus and dialogs are *not* pixel-identical (fonts, spacing, own icons, theme), so most non-canvas pairs stay above 3 %. |
| **real deviation** | A measured, reproducible difference in behaviour or result that is Firepixel's fault. |
| **unclear** | A difference exists, but the evidence is not conclusive (runner state, reference side invalid, 1 px). |
| **under-measured** | Fewer than 60 % of the measurable steps have `ok` on both sides. |

**Measurement quota** = steps with both sides `ok` / steps without an `unsupported` or `skipped` side (repeated
steps count with their second attempt). **Proven 1:1** = units "equal except for design" / sufficiently
measured units. The headline figure is the strict one over all classified units (all 49 since run 24; 100.0 % after runs 26 to 29, 32, 34 to 36, 39, 42, 43 (after the hand check CHK-1) and 44, 98.0 % after runs 30, 33, 38, 40 and 41, 95.9 % after runs 31 and 37); since
run 15 no unit is under-measured, so both figures are the same.

## Limits

The runner cannot reach everything; these parts need a human (see [VERIFICATION.md](VERIFICATION.md)):

- **Embedded panel dialogs** of the reference application (e.g. File Info, Preview in Browser, Special
  Characters, the auto shape panels) have no standard window controls; the runner can only screenshot and close
  them. Such steps are `unsupported` on the reference side.
- **Real keyboard handling:** Alt tap to the menu bar, access keys, IME, some shortcuts. Firepixel receives
  synthetic events through the bridge, which is not the same as the OS keyboard path.
- **Real mouse dynamics:** brush and pen strokes depend on event timing and pressure; the runner draws with
  fixed steps.
- **Owner-drawn lists** (layers, library, swatches, history rows) are located by image, not by control, so
  results there are less certain. Up to v0.2.10 Firepixel's Layers panel kept a layer collapsed in an earlier step
  collapsed in the reopened test file (the reference reopens the stored state); since R-32 the runner expands such
  parent layers with their triangles before giving up on a row and logs it (TW-210, until run 36 the cause of the
  `unmapped` rows `panel-layers/07, 08, 10, 12`; run 37 expanded three times). F79 `7337428e` makes the panel show
  the document's stored disclosure; in runs 38 and 39 both programs open T07 as stored and the lookup expanded nothing. Since
  R-33 (TW-216) the drag steps `09`, `12` and `15` set the same disclosure on both sides first (`layers_disclosure`:
  script, then Firepixel's panel checked row by row and corrected with the triangles; runs 38 and 39: nothing to correct), layer
  drags log the list's position and keep both rows fully in view, and the selection dot is a target on both sides.
  Since R-34 the Layers row targets come from the panel only, never from a window-wide text search (in run 38 the
  Ctrl-click of `panel-layers/08` had hit the Properties inspector's name field, Q-66); in run 39 every row target lay
  inside the panel.
  Firepixel's Layers panel stays 268 × 291 px against the reference's 245 × 389 px flyout, so its list scrolls (since
  F83 to the current layer) and the panel pairs follow the scroll position more than the drawing.
- **Focus:** other applications taking the foreground make reference steps fail; such steps are repeated or
  counted as errors, never as `ok`.
- **Transform frames:** with View ▸ Edges off the reference's Scale / Free Transform frame is not in its canvas shots
  (`tool-text/27_text_scale_tool_frame_open` shows Firepixel's frame only), so the frame of a text is compared through
  the result of a drag from each side's handle. Since R-36 (run 42) the verb `transform_handles` reads selection count,
  text, raw rect, matrix and bounds on each side by script and derives the frame by the rule of RE spec 67 (one point
  text: the raw rect × 1.3 about its centre through its matrix; otherwise the bounds), and the canvas point
  `{"transform_handle": "br"}` presses the shown handle 1 screen pixel up and left on both sides (document 184,182 for
  the 60 pt "Ag"; TW-219, TW-221). This checks the drag result under one rule, not where each program draws its frame;
  R-35 had taken Firepixel's handle from the bounds, so in run 41 its press landed inside the F87 frame and moved the
  text. The frame's drawing (line and handle colour) is seen only in Firepixel's shot and in the click-test A4-66
  (TW-220).
- **Text clicks into context menus:** after a `right_click` the runner sets no `popup_rect`, so a label click such as
  "Löschen" falls back to a window-wide `find(text)` with non-exact matching and can hit any "… löschen" control in the
  window (TW-231: in run 43 the Properties inspector's "Stil löschen", in earlier builds the dock's "Auswahl löschen",
  which deleted the selection and made `xs-context-menus/15_effect_delete_result` read 6 since run 3; the canvas context
  menu has no Delete entry). Until runner item R-37 limits such clicks to the open menu and reports a missing entry as
  `unmapped`, a Firepixel-only value after a context-menu label click is no evidence.
- The reference application's own rendering is the oracle only where it is deterministic; cursor images
  cannot be captured.

## Per-unit result (run 44 (follow-up) on top of run 43 (full))

Four rows (`inspector-bitmap`, `panel-layers`, `panel-layers-menu`, `panel-pages-states-history`) come from follow-up
run 44 (24 Oct 2026, 16:55–17:18, 4 units, 92 scripted steps, 53 image pairs plus 5 shots without a partner), preview
build `2026-10-24_16-54` from `main` at `28691c50` (v0.2.12 plus tester feedback batch 8: F100 `d7b80e77` the Layers
group and mask tree, a Firepixel extension behind the switch Show Group Contents, plus the selected object's blue
thumbnail frame and the row kept highlighted during mask editing; F98 `3a6cd637` New Page and Duplicate Page after the
current page with the smallest free number and copy names, the History row "Change Page Selection", the page row context
menu and a row dragged onto the New Page button; F99 `b8da4c53` and `30d369c6` the smudge, Rubber Stamp and Brush growth
at the bitmap bounds, and `e840210d` Confine to boundaries for Blur, Rubber Stamp, Brush and Pencil, a Firepixel
extension), runner R-36 (`f528cb2b`) with the F90 settings and the new F100 setting `fp.layers_group_tree` off (see the
method notes). "Was" compares each row with run 43, whose steps the run repeats unchanged. The units ran in the order
`panel-pages-states-history`, `panel-layers`, `panel-layers-menu`, `inspector-bitmap`. The reference was restarted once,
after a script access violation while opening the test file at the start of `inspector-bitmap` (unit-start reset
repeated, no step lost); its Properties inspector read 125 px at all unit starts (TW-197). One other application's window
was open at the start (logged); none took the foreground. No `runner error` line. Canvas pairs are recomputed with the
offset search (± 3 px, 32 levels); all six read their run 43 values.
The other 45 rows come from full run 43 (24 Oct 2026, 08:38–14:02, 49 units, 1310 scripted steps, 715 image pairs plus 50 shots
without a partner), preview build `2026-10-24_08-38` from `main` at `22355408` (v0.2.11 plus tester feedback batch 7:
F88a `d8ca200a` and F88b `2bc47925` masks, F89 `e76821af` slider undo, F90 `e27198cb` tooltips and coordinate tag, F91
`362d1b97` retouch performance, F92 `9699d53d` panel scroll, F93 `7a858f99` pause key, F94 `22355408` cursor badges,
marquee rules and the lasso option, F95 `5e7c28c7` dither density and Screentone, F96 `d8615cab` pattern lock and the h1
drag, F97 `3c627a8b` handle-line rotate and the click outside an open pop-up), runner R-36 (`f528cb2b`) with the F90
settings (`fp.coordinate_tag` off, `fp.view_tooltips` on; see the method notes). "Was" compares each row with the
unit's newest earlier run: run 42 for `tool-text`, run 40 for `color-popup`, `fx-bevel-emboss-fill` and
`inspector-fields`, run 39 for `panel-layers`, `panel-layers-menu`, `color-popup-contexts`, `inspector-text` and
`xs-select-behind`, run 35 for `tool-path-reshape` and `toolbox`, run 34 for `tool-autoshapes`, `tool-web`,
`menu-modify-a`, `xs-canvas` and `xs-windows-tabs`, run 33 for the other 33 units; for those 40 units the Firepixel side
also carries every fix between their run and batch 7 (F61 to F87). The units ran in alphabetical order. The reference
was restarted six times without lost steps: three planned restarts before `menu-view`, `panel-misc` and
`xs-file-formats`, and three after a script access violation (in `dialogs-d11-d20` before `06`, in `dialogs-d31-d40`
before `05` and at the start of `toolbox`), each with the unit-start reset repeated. One other application's window was
open at the start (logged); none took the foreground. The reference's Properties inspector was expanded at every unit
start (TW-197), so the runs 34 values of the inspector and overview pairs in `tool-autoshapes`, `tool-web` and
`menu-modify-a` are back at their run 33 values; its dock group heights varied between units. Canvas pairs are recomputed with the
offset search (± 3 px, share of pixels deviating by more than 32 levels); the other percentages are the runner's
`cmp/summary.csv` values.

**Firepixel's shots.** 553 of Firepixel's 753 shots are pixel-identical to the baseline; the 200 others change for these
reasons. *Batch 7:* the dither density field "0 %" with its checker icon under the Texture list of the fill row (F95,
717 px in the inspector shots with a solid fill: `color-popup`, `fx-stack`, `inspector-fields`, `inspector-group-symbol`,
`inspector-stroke`, `tool-autoshapes`, and in the overviews with a selected object); the Pointer inspector's new row
"Marquee: Rectangle" (F94, 176 × 20 px, in most of the 45 changed overviews and in the window shots with the Pointer and nothing selected);
the docked panels' lists 2 px shorter, so the Layers, Pages and States footers and, in a list scrolled to the current
layer, the Layers rows lie 2 px higher (F92, every overview and Layers shot), and the Path and Image Editing panels with a
vertical scroll bar and re-wrapped buttons (F92, 15 885–16 941 px); the Screentone entry in the effects menu (F95,
`fx-stack/02`); the mask thumbnail as luma (F88a, `panel-layers/18`); the View menu with View ▸ Coordinate Tag and
View ▸ Tooltips checked (F90 and the runner setting, `menu-view/10`). *Earlier fixes, against baselines before them:*
the Layers selection circles, grey circles and current-layer tints (F83, F85; units of runs 33 to 35), the selection
display of free paths and polygons as outline and anchors (F65, `inspector-stroke`, `menu-modify-b`,
`panel-info-align-path/11`), slider pop-ups flush with their arrows (F64, `dialogs-*`, `panel-swatches-mixer/13`),
F87's transform frame (`menu-modify-b/01`), the corner handles at one size (`fx-shadow-glow`, `fx-stack/04`), the tool
colour wells white / black with R-31's unit-start stroke, the Auto Shapes previews with the 1 px anti-aliased stroke
(F61) and "Version 0.2.11" on the start screen. *Other:* marching ants, carets, the running animation, the painted
cursor at the pointer's last place, one-pixel columns at menu edges, and the shot area of
`xs-context-menus/05_bitmap_marquee_open` (18 px narrower, its entries unchanged). The reference's own changes come from
its restarts, its expanded inspector, its dock groups (overview pairs −2 to +2.6 points), its dialog and menu positions,
its History read-backs (`panel-layers/22`, `inspector-fields/25`, `color-popup/23` read 0) and its text entry
(`tool-text`).

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | run 43: 92.9 % (26 / 28), unchanged since run 33; every compared value as in run 33. Firepixel's shots: the overview (Marquee list, Layers footer, circles; see the shot notes above the table) and the list pop-up `07_d04_fields_open` (its scroll bar 849 px, F92); the reference's dialog shots moved with its dialog positions (`17_d10_fields_normal` 94.7 → 73.5 %, `_tab2` 94.7 → 65.0 %) |
| `dialogs-d11-d20` | dialogs | equal except for design | run 43: 88.9 % (32 / 36), unchanged since run 33; every compared value as in run 33 (only the `ui_state` tool stroke colour of `21`, #c04040 → #000000 with R-31's unit-start stroke, not compared). The reference was restarted after a script access violation before `06_d14_fields_normal` (unit-start reset repeated, no step lost); its effect dialog `13_d17_effect_result` reads 82.8 → 95.2 %. Firepixel's shots pixel-identical to run 33 except the overview |
| `dialogs-d21-d30` | dialogs | equal except for design | run 43: 100.0 % (27 / 27), unchanged since run 33. Firepixel's text TX1 of `06_d23_effect_result` reads 14 px wide and 339 px high (run 33: 4 and 340; the zero-width `addNewText` frame of F82, not compared, the reference reads no bounds); symbol ids differ as always. Firepixel's slider pop-ups `05_d23_fields_open` and `17_d29_fields_open` sit flush with their arrows (F64, after run 33; pairs 31.0 → 34.2 % and 48.1 → 50.4 %); overview as above |
| `dialogs-d31-d40` | dialogs | equal except for design | run 43: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 %; `11` and `16` fail on both sides as before. The reference was restarted after a script access violation before `05_d33_msg_open` (no step lost). Firepixel's shots: overview, and the marching ants' phase in `12_d38_result` and `13_d39_result` |
| `dialogs-d41-d50` | dialogs | equal except for design | run 43: 91.3 % (21 / 23), unchanged; Firepixel's slider pop-up `08_d45_fields_open` flush with its arrow (F64), the Layers rows in `16_preview_result` (F83 circles and tints, F92 footer; pair 23.8 → 25.9 %) and the overview changed; every compared value as in run 33 |
| `filters-adjust-color` | filters | equal except for design | run 45 (25 Oct 2026, runner R-37): 69.6 % (16 / 23) again - the live-filter "+" menu opened with the first click in all five uses (0.40-0.87 s), `02`, `05`, `06`, `09` `ok` on the reference side, `09_color_fill_result` 0.35 %, 4 / 4 canvas pairs within 1 %; TW-232 closed. Before, run 43: 68.4 % (13 / 19), was 69.6 % (16 / 23): `02_brightness_live_open`, `05_curves_live_open`, `06_levels_live_open` and `09_color_fill_result` are `unsupported` on the reference side ("choose: target opened no menu and is no Win32 combo box": the reference's live-filter "+" button `inspector.fx_add` opened no menu; runner row TW-232), so four steps leave the measurable ones and `09` loses its canvas pair (0.35 %); 3 / 3 canvas pairs within 1 %. Firepixel's side unchanged (`02`, `06`, `09` `ok`, `05` `unmapped` as before); its shots pixel-identical to run 33 except the overview and the marching ants in `11_selection_only_result` |
| `filters-blur-sharpen-noise` | filters | equal except for design | run 43: 79.3 % (23 / 29), unchanged; 10 / 11 canvas pairs within 1 % (`16_preview_box_on_result` 20.0 % as before); Firepixel's shots pixel-identical to run 33 except the overview and the marching ants in `15_selection_only_result` |
| `fx-bevel-emboss-fill` | live effects | equal except for design | run 43: 90.5 % (19 / 21), unchanged since run 40; 2 / 4 canvas pairs within 1 % (`01` 21.1 %, `09_live_pressed` 21.8 % as before; the reference's slider found without R-35's retry); every compared value as in run 40; Firepixel's shots pixel-identical to run 40 except the overview (Marquee list, Layers footer). `03` (Firepixel: no second combo box) and `11` (both sides) fail as before |
| `fx-shadow-glow` | live effects | equal except for design | run 43: 100.0 % (23 / 23), unchanged; 5 / 6 canvas pairs within 1 % (`05_shadow_live_pressed` 23.3 % as before); Firepixel's selected rectangle in `05`, `08`, `16`, `17` shows its four corner handles at one size (15 px; run 33: the bottom-right one larger), overview as above |
| `fx-stack` | live effects | equal except for design | run 43: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 %. Firepixel's effects menu `02_submenus_other_open` lists the new Screentone entry ("Rasterfolie", F95, a Firepixel extension; pair 63.0 → 61.6 %); the fill row of the inspector shots `03`, `07` carries the dither density field (F95, 717 px); `04` the corner handles (15 px); the `ui_state` tool fill of `14` reads #22aa44 (run 33: #ff9900; not compared) |
| `inspector-bitmap` | property inspector | equal except for design | run 44: 100.0 % (15 / 15), unchanged; 3 / 3 canvas pairs within 1 % at their run 43 values (0.25, 0.35, 0.36 %); the six exports are pixel-identical to run 43 on both sides (the reference's files differ in their bytes only). **TW-238 seen (F100):** with B1 selected Firepixel now frames B1's own thumbnail with a 2 px #0078d7 line in `08_mask_layers_normal` and `09_thumbnail_normal` (176 px each), as the reference does (2 px #00a8ff in `09`); pairs 54.350 → 54.353 and 53.505 → 53.507 %. `08` also shows the open TW-225 rest that Reveal All does not select the new mask: after Modify ▸ Mask ▸ Reveal All the reference edits the mask (its Layers row frames the white mask thumbnail green, `08_mask_inspector_normal` shows "Bitmap-Maske" with Alpha channel / Grayscale), while Firepixel keeps B1 selected (its Bitmap inspector, and now the blue frame on B1's thumbnail instead of no frame); both sides' inspector shots are pixel-identical to run 33, so this is no batch 8 change (the run 43 note "no step selects the mask itself" holds for Firepixel only). **TW-236 not reached:** every inspector shot is an object inspector, pixel-identical to run 43. The reference was restarted at the unit start (access violation while opening `par_bitmap.fw.png`, no step lost); its overview reads 33.17 % (run 43: 33.16 %). Firepixel's other shots pixel-identical to run 43 |
| `inspector-fields` | property inspector | equal except for design | run 43: 87.5 % (28 / 32), unchanged since run 40; 2 / 3 canvas pairs within 1 % (`14_popup_drag_live_canvas_pressed` 1.58 % as before, TW-171). **TW-207 still confirmed:** `25_pattern_move_*` read the pattern handles of runs 37 to 40 in both programs, pairs 0.09 % (the reference's History read-back 0 → 0, run 40: 2 → 6). **F95 seen:** the fill row of every inspector shot with a solid fill (`17`, `21`, `23`, `24`) shows the dither density field "0 %" with its checker icon under the Texture list (717 px; `23_disabled_look_path_no_fill_disabled` 243 px); no step shows a pattern or gradient fill in the inspector, so F96's padlock is in no shot. The pop-up steps `14` and the History read-backs as in run 40; `10` (reference side) and `20_undo_each_result` (Firepixel: no text field at the stroke size) fail as before |
| `inspector-stroke` | property inspector | equal except for design | run 43: 88.2 % (45 / 51), unchanged; 31 / 31 canvas pairs within 1 %, `04_edge_e0`, `e50`, `e100` 0.89 → 0.80 % and `05_texture_result` 0.27 → 0.20 %: Firepixel's selected free path P1 now shows outline and anchors instead of the bounding box (F65, TW-202; after run 33) in the path result shots `02` to `05`, `09` and `13` and PG its outline in the position shots `06` and `14`; `06_position_buttons_normal` and `18_disabled` carry the dither density field. The `ui_state` tool fill reads #808080 after the selected objects (F58 follow; not compared). `03` and `12` fail on the reference side as before |
| `inspector-text` | property inspector | equal except for design | run 43: 90.3 % (28 / 31), unchanged since run 39; 11 / 12 canvas pairs within 1 % (`21_live_result` 1.87 %), all at their run 39 values. **TW-214 still confirmed** (`25`: left 448, top 378, ink 450,382–531,430 in both), **TW-204** as documented. Every compared value as in run 39; Firepixel's shots pixel-identical to run 39 except the overview and the font list `02_font_list_open` (its scroll bar, F92). The reference shows its Text tool inspector in `23_tool_vs_object_normal` again (24.1 → 19.9 %). `07`, `08`, `11` (`unmapped`) and `18` as before |
| `inspector-web` | property inspector | equal except for design | run 43: 81.2 % (13 / 16), unchanged; values as in run 33; Firepixel's shots pixel-identical to run 33 except the overview |
| `menu-commands-window-help` | menus | equal except for design | run 43: 75.0 % (15 / 20), unchanged; every compared value as in run 33 (the `ui_state` tool colours of `15` white / black with R-31, not compared); Firepixel's `07_window_toolbars_result` differs in the tool colour wells (R-31) and the Marquee list, the overview as above; `01` as before |
| `menu-edit` | menus | equal except for design | run 43: 91.7 % (22 / 24), unchanged; `02_redo_last_result` as in run 31 (TW-173); the reference's Edit menu shots are 20 px narrower (pairs −1.7 to −3.0 points); Firepixel's shots pixel-identical to run 33 except the overview. `16` and `17` fail as before |
| `menu-file` | menus | equal except for design | run 43: 81.8 % (18 / 22), unchanged; every compared value as in run 33; Firepixel's `20_share_open` differs in one 16 px column at the menu's edge, the overview as above |
| `menu-modify-a` | menus | equal except for design | run 43: 96.2 % (25 / 26), unchanged since run 34; 5 / 6 canvas pairs within 1 %; `05_rotate_canvas_result` 1.69 → 3.53 %, its run 33 value: the reference's page at 100 % lies at its run 33 place again (page top 183; run 34: 217 with its collapsed inspector, TW-197), as in the overview (29.0 → 19.1 %). Firepixel's values as in run 34 (symbol ids aside); its shots pixel-identical to run 34 except the overview and `12_symbol_edit_open` (Marquee list, Layers footer) |
| `menu-modify-b` | menus | equal except for design | run 43: 92.0 % (23 / 25), unchanged; 5 / 5 canvas pairs within 1 %; every compared value as in run 33. Firepixel's canvas shots show the selection display since F65 (outline and anchors for the combined paths `07` to `09` and the expanded stroke `14`, no bounding box) and F87's transform frame in `01_free_transform_result` (1 screen pixel up and left, the handles outlined); not in the reference's shots (View ▸ Edges off) |
| `menu-select` | menus | equal except for design | run 43: 100.0 % (16 / 16), unchanged; 6 / 6 canvas pairs within 1 %; Firepixel's canvas shots differ in the phase of the marching ants, as in every run |
| `menu-text` | menus | equal except for design | run 43: 90.9 % (20 / 22), unchanged; 4 / 4 canvas pairs within 1 %. Firepixel's TX1 reads 14 px wide (run 33: 4) and 1 px lower at 24 pt in `02`, `03`, `16` (F82's zero-width frame; not compared, the reference reads no bounds); shots pixel-identical to run 33 except the overview |
| `menu-view` | menus | equal except for design | run 43: 96.0 % (24 / 25), unchanged; 6 / 6 canvas pairs within 1 %. `10_guides_menu_result` 40.9 → 45.0 %: Firepixel's View menu is 21 px taller with the new entry View ▸ Coordinate Tag (F90, a Firepixel extension, unchecked by the runner) and shows View ▸ Tooltips checked (the runner's new `fp.view_tooltips` on; the reference's is unchecked). The painted cursor lies elsewhere in `11`, `12`; `17_screen_modes_result` adds the Marquee list. The menu read-backs equal as in run 33 |
| `panel-info-align-path` | panels | equal except for design | run 43: 83.3 % (30 / 36), unchanged; 2 / 2 canvas pairs within 1 %. **F92 seen:** Firepixel's Path panel (`10_path_layout_normal`, `13`, `14`) now has a vertical scroll bar and wraps Crop to a third row of Combine Paths (16 941 px); the panel pairs move with it. `11_path_buttons_union_result` shows the union's outline and anchors (F65). The History read-back of `15` reads 1 → 2 against 0 → 1 as in run 33 |
| `panel-layers` | panels | equal except for design | run 44: 96.7 % (29 / 30), unchanged since run 39; every compared value as in run 43 (only the bridge's frame counter and the reference's window class name differ). **TW-205, TW-208, TW-215, TW-216 and Q-65 still confirmed**; `layers_disclosure` found the document's flags at all four uses and no row lookup expanded a layer; Show Group Contents was switched off at the unit start, so Firepixel lists the reference's rows. **TW-238 seen (F100):** the selected object's own thumbnail carries a 2 px #0078d7 frame in six shots, as the reference frames it (2 px #00a8ff): L1obj in `02_row_states_normal`, `02_row_states_pressed`, `11_rename_focus` and `23_opacity_slider_drag_panel_pressed` (48–176 px; cut by the list's lower edge where the list is scrolled), R1 in `18_mask_row_normal` and B1 in `19_bitmap_mode_row_normal` (176 px each); the pairs move by at most 0.03 points (`18` 62.38 → 62.41 %). No step selects a mask, so the row kept highlighted during mask editing is not reached. The reference's pressed shot `23_opacity_slider_drag_panel_pressed` shows no slider pop-up and the "Pfad" row of Ebene 2 highlighted (run 43: the pop-up at 30 with L1obj selected), pair 64.1 → 60.4 %; its slider results read as before (the three `23` canvas pairs 2.10, 2.10 and 0.10 %). `14`, `17`, `21` as before; Firepixel's other shots pixel-identical to run 43 |
| `panel-layers-menu` | panels | equal except for design | run 44: 81.0 % (17 / 21), unchanged since run 39; every compared value as in run 43 except Firepixel's menu read-back. **TW-237 seen:** Firepixel's options menu in `01_menu_open_open` and `02_menu_states_no_sel_open` lists the extra entry "Gruppeninhalte anzeigen" (Show Group Contents, unchecked, after a separator under Thumbnail Options) next to the reference's 24 entries, which read the same on both sides; the menu is 33 px taller and the shot 375 × 635 px (run 43: 362 × 602; 13 px of the dock behind the menu included), pairs 15.6 → 23.4 % and 15.5 → 23.3 %; the read-back no longer picks up the texts "800 x 600" and "100%" behind the menu (run 43). `04_context_object_open` shows the blue frame on L1obj's thumbnail below the context menu (88 px, TW-238; pair 39.38 % unchanged). `10_options_open` fails on Firepixel's side with "no dialog window found" as in every run since run 17: the step double-clicks the "Ebene 1" row, where the reference opens its Layer Options dialog and Firepixel none; it does not use the options menu, so the new entry plays no part (neither a runner artefact of batch 8 nor a regression). `05_footer_hover`, `09`, `18` `unmapped` as before; Firepixel's other shots pixel-identical to run 43 |
| `panel-misc` | panels | equal except for design | run 43: 87.5 % (14 / 16), unchanged; embedded panels `unsupported`. Firepixel's Image Editing panel `06_image_editing_normal` has the F92 scroll bar and re-wrapped buttons (15 885 px), the Auto Shapes grid `01` draws its previews with the 1 px anti-aliased stroke (F61, after run 33), `18_menus_find_open` a grey circle (F85); overview as above |
| `panel-optimize` | panels | equal except for design | run 43: 83.3 % (20 / 24), unchanged; Firepixel's shots pixel-identical to run 33 except the overview |
| `panel-library-styles` | panels | equal except for design | run 43: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 %; Firepixel's shots differ only in the overview, `04_doclib_props_open` and the Layers circles under the menus `12`, `16` (F83, F85) |
| `panel-pages-states-history` | panels | equal except for design | run 44: 75.0 % (18 / 24), unchanged; every compared value as in run 43, and Firepixel's 14 shots are pixel-identical to run 43. **TW-233 (F98) not reached:** the page steps read only the page count (`03`: New Page, Duplicate Page from the panel menu, Delete; Firepixel 3 → 4 → 3, the reference 2 → 3 → 3, both as in run 43) and take no shot after New Page, and no History step clicks a page row, so neither the new page's place and name nor the History row "Change Page Selection" is in a shot or a value; the History read-backs of `16` and `21` (the vector test file, nudges only) read "Move", "Deselect", "Move" against the reference's four script rows, as since run 31. `09_states_onion_open`, `10_states_loop_open` (reference `error`), `09_states_onion_result`, `12`, `15` (`unmapped`) and `18_history_save_cmd_open` (Firepixel `error`) as before |
| `preview-options` | preview / export | equal except for design | run 43: 76.9 % (30 / 39), unchanged; Firepixel's 19 shots pixel-identical to run 33; Firepixel-side errors as in run 24 |
| `preview-views` | preview / export | equal except for design | run 43: 69.6 % (16 / 23), unchanged; Firepixel's shots pixel-identical to run 33 except the running animation `17_anim_buttons_play_pressed` |
| `color-popup-contexts` | colour | equal except for design | run 43: 68.2 % (15 / 22), unchanged since run 39; every pair (except the overview, 21.7 → 19.6 %) and every compared value as in run 39; Firepixel's shots pixel-identical to run 39 except the overview. `01_toolbox_stroke` fails on the reference side as before |
| `tool-path-reshape` | tools | equal except for design | run 43: 100.0 % (23 / 23), unchanged since run 35; 13 / 14 canvas pairs within 1 % at the run 35 values (`11_rect_to_path_result` 10.93 %); Firepixel's shots pixel-identical to run 35 except the overview; TW-202 (the Pen) and TW-177 not reached |
| `tool-text` | tools | equal except for design | run 43: 87.5 % (28 / 32), unchanged since run 40. **TW-219 and TW-212 still confirmed, TW-221 (R-36) fine:** `transform_handles` and the corner drag read as in run 42 on both sides (83.74 pt, matrix [0.993, 0, 0, 1.005], t 5.085 / 23.883; pair 0.19 %), the numeric and script paths as in runs 40 to 42; every Firepixel value as in run 42. 18 / 21 canvas pairs within 1 % (`11` 2.29 %, `14` 1.07 %, `16` 3.75 % as before); `02_drag_frame_result` 0.12 → 0.41 % with the reference's first character typed at the second try. TW-146 rest: 9 `runner error` lines (run 42: 8; the six selection lines, the size line, a discarded "Abcd" and one step without a text editor after the caret click). Firepixel's shots pixel-identical to run 42 except the overview and the caret (`07`, `13`). `12`, `15`, `19`, `17` as before |
| `tool-web` | tools | equal except for design | run 43: 96.4 % (27 / 28), unchanged since run 34; 14 / 16 canvas pairs within 1 % at the run 34 values (`07` 3.7 %). The inspector pairs `13_inspector_hotspot` and `_slice` 63.1 / 63.6 → 17.5 / 17.9 % (the reference's inspector expanded again, TW-197; their run 33 values), the overview 29.6 → 22.1 %; Firepixel's shots pixel-identical to run 34 except the overview. **TW-192 (refined)** as in run 34 |
| `toolbox` | tools | equal except for design | run 43: 94.6 % (53 / 56), unchanged since run 35; 0 / 3 canvas pairs within 1 % (cursor shots 2.49 % as before). The reference was restarted after a script access violation at the unit start (reset repeated, no step lost). Firepixel's shots pixel-identical to run 35 except the overview and the screen-mode shots `13_view_modes_full_menus` and `_standard` (the Marquee list of F94 in the Pointer inspector); with View ▸ Tooltips on the tooltip shots `03` are pixel-identical to run 35 |
| `tool-autoshapes` | tools | equal except for design | run 43: 100.0 % (40 / 40), was 97.5 % (39 / 40): `10_scale_result` is `ok` on the reference side again (run 34: another application's window took the foreground); 27 / 27 canvas pairs within 1 % at the run 34 values. The reference's inspector is expanded again (TW-197; `07_inspector_head_normal` 65.5 → 30.2 %, the overview 26.6 → 19.4 %, its drags and page 34 px higher). Firepixel's shots pixel-identical to run 34 except the overview and the dither density field in `07` |
| `export-results` | preview / export | equal except for design | run 43: 81.8 % (9 / 11), unchanged; 4 / 4 canvas pairs within 1 %. The reference needed the documented double-click to open its export-area preview in `13` (value `if_no_dialog` fallback, run 33: Enter sufficed); Firepixel's shots pixel-identical to run 33 except the overview |
| `color-popup` | colour | equal except for design | run 43: 77.4 % (24 / 31), unchanged since run 40; 1 / 4 canvas pairs within 1 % as in run 40. **TW-206 still confirmed** (#80808080 after Enter); the reference's History read-back of `23_*` reads 0 (run 40: 1; Firepixel 0 in both). **F89 / F97:** the pop-up slider steps `23` read every value of run 40 (no Undo inside them, no click outside an open slider), the slider shots pixel-identical apart from the dither density field (`23_alpha_slider_drag_pressed`, `17_drag_red_result`); overview and `01` as above. `15_click_out_result` and `21_undo` (reference side) as before |
| `xs-context-menus` | cross-cutting | equal except for design | run 45 (25 Oct 2026, runner R-37): 85.2 % (23 / 27) - text targets after a right-click stay in the opened menu on both sides, `15_effect_delete_result` re-specified (no Delete entry in either program, menu and Bearbeiten submenu recorded, 7 / 7 objects), `15_effect_duplicate_result` via the Bearbeiten submenu (8 / 8), `15_effect_arrange_back_result` with "In den Hintergrund" (equal order), `15_effect_flip_h_result` equal; all five `ok` on both sides. Before, run 43: 70.4 % (19 / 27), unchanged; first read as unclear by TW-231, equal except for design again after the hand check CHK-1 (24 Oct 2026). **TW-231 is a runner artefact (runner item R-37):** in `15_effect_delete_result` (T04, L1 selected by script, right-click at 175,330, then the label click "Löschen") Firepixel's element count reads 7 (runs 3 to 33: 6); the canvas context menu has the same 28 entries and no Delete entry in both previews `2026-10-24_08-38` and `2026-10-23_22-35` (none in its Bearbeiten submenu either), and without a `popup_rect` the runner's window-wide non-exact fallback clicked the Properties "Stil löschen" button (907,973; nothing happens) in run 43 and the dock's "Auswahl löschen" (1572,641; deletes the selection, 6) in the earlier builds; the reference side is `unmapped` ("label not found") as always. Every other value as in run 33 (`15_effect_duplicate_result` 7 in all runs; Duplicate sits in the Bearbeiten submenu, which the step never opens). Firepixel's shot of the marquee context menu `05_bitmap_marquee_open` starts 18 px further right (the check-mark column cut off; entries and states unchanged; pair 35.5 → 38.2 %), `03_path_open` differs in one column, `04_text_open` behind the menu |
| `xs-preferences` | cross-cutting | equal except for design | run 43: 85.0 % (17 / 20), unchanged since run 32; 2 / 3 canvas pairs within 1 % at the run 33 values (`06` 3.10 %); Firepixel's pen preview `07_edit_pen_preview_result` differs in one anchor (11 px), the rest pixel-identical; `03`, `04`, `10`, `18` as before |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 43: 73.7 % (14 / 19), unchanged since run 34. Firepixel's start screen reads "Version 0.2.11" (`16`, `19`; run 34: 0.2.6), its window shots carry the Marquee list (F94) and `12_workspace_switch_open` its menu 2 164 px; values as in run 34. The reference's Properties inspector was found collapsed at the run end (56 px, after this unit's panel steps) and expanded again |
| `xs-file-formats` | cross-cutting | equal except for design | run 43: 85.7 % (18 / 21), unchanged; 4 / 4 canvas pairs within 1 %. Firepixel's opened JPEG `02` reads "Hintergrund" as current layer (`activeLayer` 1, run 33: "Ebene 1"; F83's bottom-most unlocked layer; not compared); start screen "Version 0.2.11"; the opened files' window shots carry the Marquee list. The reference was restarted as planned before the unit, and the unit-start tool stroke logged "not set" on both sides; no step status changed. `05` and `15` fail on both sides as before |
| `inspector-group-symbol` | property inspector | equal except for design | run 43: 90.0 % (18 / 20), unchanged; 1 / 1 canvas pairs within 1 %. Firepixel's inspector shots with a fill (`01`, `10` to `14`) carry the dither density field (F95), the symbol windows `06`, `15` the tool colour wells (R-31), the Layers flyout `17_thumbnail_normal` the current-layer tint and grey circle (F83, F85) and the footer 2 px higher (F92; pair 61.9 → 60.3 %). The `ui_state` tool colours now white / black or the selected object's (not compared) |
| `panel-swatches-mixer` | panels | equal except for design | run 43: 96.3 % (26 / 27), unchanged; `19_selected_object_result` #efffbf on both sides (TW-174); Firepixel's Mixer slider pop-up `13_mixer_sliders_open` sits flush with its arrow (F64); History read-back of `20` as in run 33 |
| `xs-shortcuts` | cross-cutting | equal except for design | run 43: 92.0 % (23 / 25), unchanged; every compared value as in run 33 (the `ui_state` tool fill of `12` #808080, not compared); `20_alt_menus_open` differs in one column; `18`, `19` as before |
| `xs-canvas` | cross-cutting | equal except for design | run 43: 91.7 % (22 / 24), unchanged since run 34; 1 / 8 canvas pairs within 1 % at the run 34 values (BA-098: the zoom shots pixel-identical). Firepixel's window shots carry the Marquee list (F94), `17_drag_clip_pressed` the Layers rows (F83, F92), `11_grid_show_normal`, `05` and `20` the painted Pointer at another place (the pointer's last position) |
| `xs-select-behind` | cross-cutting | equal except for design | run 43: 100.0 % (9 / 9), unchanged; 9 / 9 canvas pairs within 1 % at their run 39 values; both programs' shots pixel-identical to run 39 and every compared value as before |

Totals over all 49 units (run 44 on top of full run 43): **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**, 49 / 49). Step level:
1095 of 1244 measurable steps `ok` on both sides (**88.0 %**, unchanged; the four units 79 / 90 as in run 43); over the 48
units before run 24: 1086 of 1235 (87.9 %). The 92 steps of run 44: 79 `ok` on both sides, 6 `unmapped` and 1
`unsupported` on both sides, 2 reference-side `error` against `ok`, 2 `ok` against a Firepixel-side `error`
(`panel-pages-states-history/18_history_save_cmd_open`, `panel-layers-menu/10_options_open`), 1 `unmapped` / `ok` and 1
`unsupported` / `ok`; no step status changed. Errors: **33 reference-side and 21 Firepixel-side**, unchanged. Canvas
image pairs within 1 %: **195 of 224**, unchanged (the six canvas pairs of the four units read their run 43 values); over
the 48 old units 186 of 215. No class changed. Compared values: none changed on either side (only the bridge's frame
counter, the reference's window class name and export bytes, the runner's list click 0.5 px and the runner script names
in the reference's History rows); Firepixel's menu read-back of `panel-layers-menu/01`, `02` adds the entry Show Group
Contents (TW-237). Firepixel's shots are pixel-identical to run 43 except nine shots with the blue thumbnail frame
(TW-238: `panel-layers/02` normal and pressed, `11`, `18`, `19`, `23` panel, `panel-layers-menu/04`,
`inspector-bitmap/08` Layers, `09`) and the two options-menu shots (TW-237); the reference's except its overviews (dock
groups after its restart) and the pressed panel shot `panel-layers/23` (no slider pop-up). No `runner error` line in run
44 (over the newest run of every unit: 9, all in `tool-text`).

Run 43's totals (full run, after the hand check CHK-1), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**, 49 / 49). Step level: 1095 of 1244
measurable steps `ok` on both sides (**88.0 %**; counted as before, with the Firepixel-only step
`toolbox/17_ai_tools_normal` as `ok`); over the 48 units before run 24: 1086 of 1235 (87.9 %). The 1310 steps: 1094 `ok`
on both sides, 79 `unmapped` and 37 `unsupported` on both sides, 10 `error` on both sides, 20 reference-side `error`
against `ok`, 10 `ok` against a Firepixel-side `error`, 20 `unmapped` / `ok`, 15 `unsupported` / `ok`, 11 `unsupported`
/ `unmapped`, 6 `ok` / `unmapped`, 3 `error` / `unmapped`, 3 `unmapped` / `unsupported`, 1 `unmapped` / `error` and the
Firepixel-only step. Five step statuses changed against the units' newest earlier runs, all on the reference side:
`filters-adjust-color/02_brightness_live_open`, `05_curves_live_open`, `06_levels_live_open` and `09_color_fill_result`
`ok` → `unsupported` (the live-filter "+" menu did not open; TW-232), so four steps leave the measurable ones, and
`tool-autoshapes/10_scale_result` `error` → `ok`; no Firepixel-side status changed. Errors: **33 reference-side and 21
Firepixel-side** (run 42: 34 and 21; the 21 Firepixel-side errors are the same steps). Canvas image pairs within 1 %:
**195 of 224** (run 42: 196 of 225): `filters-adjust-color/09_color_fill_result` (0.35 %) lost its reference shot,
`menu-modify-a/05_rotate_canvas_result` is back at its run 33 value (1.69 → 3.53 %, the reference's page place), the
four `inspector-stroke/04, 05` pairs fall by 0.07–0.09 points (Firepixel's path display of F65), `tool-text/02` rises
to 0.41 % (the reference's text entry), and every other canvas pair reads its baseline value; over the 48 old units 186
of 215. No class changed: `xs-context-menus` first read as unclear (TW-231), and the hand check CHK-1 showed a runner
artefact (runner item R-37), so it stays equal except for design. Compared values: none changed on Firepixel's side;
Firepixel-only values changed in `xs-context-menus/15_effect_delete_result` (7, was 6; TW-231, the runner's label click
hit another button), in
the text bounds of F82 (`dialogs-d21-d30/06`, `menu-text/02, 03, 16`), the current layer of F83 after opening a JPEG
(`xs-file-formats/02`), the `ui_state` tool colours (R-31 and the follow rules) and the runner's Layers row geometry
(F92). 9 `runner error` lines, all reference-side text entry and selection in `tool-text` (TW-146 rest; run 42: 8).

Run 42's totals (on top of runs 41, 40, 39, 38, 37, 36, 35, 34 and 33), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**).
Step level: 1097 of 1248 measurable steps `ok` on both sides (**87.9 %**; `tool-text` 28 / 32 as in runs 40 and 41,
with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24:
1088 of 1239 (87.8 %). The 32 steps of run 42 (the `tool-text` steps of run 41, step 27 with R-36's handle source):
28 `ok` on both sides, 2 `error` on both sides, 1 `ok` against a Firepixel-side `error` and 1 `unmapped` on both sides;
no step status changed. Errors: **34 reference-side and 21 Firepixel-side**, unchanged. Canvas image pairs within 1 %:
**196 of 225**, unchanged: `27_text_scale_tool_corner_result` 0.71 → 0.19 % (both programs at 83.74 pt, TW-219), the
other 20 canvas pairs read their run 41 values (`27_text_scale_tool_frame_open` 0.27 %); over the 48 old units 187 of
216. One class changed: `tool-text` real deviation → equal except for design (TW-219 confirmed, TW-221 fixed by R-36).
Compared values: on Firepixel's side only `27_text_scale_tool_corner_result` (83.74 pt with the reference's leftover
matrix; run 41: moved by +39/+38 px at 60 pt), and both sides add the `transform_handles` values (equal); on the
reference's side the text entry of `04_esc_result` ("Abc" again; run 41: the first character lost) and of
`10_enter_result` (the first character lost, no text; run 41: "a\rb"). Firepixel's shots are pixel-identical to run 41
except the corner result and the caret in `07_select_text_double_click_focus`; the reference's except its overview (its
History tab in front of the top dock group, 58 983 px, pair 19.3 → 21.4 %). 8 `runner error` lines, all reference-side
in `tool-text` (run 41: 9; over the newest run of every unit: 8).

Run 41's totals (on top of runs 40, 39, 38, 37, 36, 35, 34 and 33), for comparison: **48 / 1 / 0 / 0** (proven 1:1 **98.0 %**).
Step level: 1097 of 1248 measurable steps `ok` on both sides (**87.9 %**; `tool-text` 28 / 32 as in run 40, with the
Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24: 1088 of
1239 (87.8 %). The 32 steps of run 41 (the `tool-text` steps of run 40): 28 `ok` on both sides, 2 `error` on both
sides, 1 `ok` against a Firepixel-side `error` and 1 `unmapped` on both sides; no step status changed. Errors: **34
reference-side and 21 Firepixel-side**, unchanged. Canvas image pairs within 1 %: **196 of 225**, unchanged: the two
`27` pairs change and stay within 1 % (`27_text_scale_tool_frame_open` 0.16 → 0.27 % with F87's larger frame,
`27_text_scale_tool_corner_result` 0.47 → 0.71 % with Firepixel's moved text), the other 19 canvas pairs read their run
40 values; over the 48 old units 187 of 216. No class changed: `tool-text` stays a real deviation, because the Scale-tool
drag on Firepixel's side started inside the new frame (TW-221) and so did not re-measure TW-219. Compared values: on
Firepixel's side only `27_text_scale_tool_corner_result` (moved by +39/+38 px at 60 pt; run 40: 90 pt); on the
reference's side the text entry of `04_esc_result` (the first character lost, no text; run 40: "Abc"). Firepixel's shots
are pixel-identical to run 40 except the two `27` shots and the caret in `01` and `07_select_text_double_click_focus`;
the reference's except its overview (725 px in the dock, pair 19.3 % as before). 9 `runner error` lines, all
reference-side in `tool-text` (run 40: 7; over the newest run of every unit: 9).

Run 40's totals (on top of runs 39, 38, 37, 36, 35, 34 and 33), for comparison: **48 / 1 / 0 / 0** (proven 1:1 **98.0 %**).
Step level: 1097 of 1248 measurable steps `ok` on both sides (**87.9 %**; the four units 99 / 116, run 39: 92 / 111,
with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24:
1088 of 1239 (87.8 %). The 117 steps of run 40 (the 112 of run 39 in these units and five new ones): 99 `ok` on both
sides, 9 `unmapped` on both sides, 3 `error` on both sides, 2 `ok` against a Firepixel-side `error`, 2 reference-side
`error` against `ok`, 1 `unsupported` on both sides and 1 `ok` / `unmapped`. Two step statuses changed against run 39,
both on the reference side and both back to `ok` by R-35's pop-up slider wait and retry:
`color-popup/23_alpha_slider_drag_canvas_pressed` and `fx-bevel-emboss-fill/09_live_pressed`; the five new `tool-text`
steps are `ok` on both sides. Errors: **34 reference-side and 21 Firepixel-side** (run 39: 36 and 21). Canvas image
pairs within 1 %: **196 of 225** (run 39: 191 of 218): the two restored steps bring back their canvas pairs above 1 %
(`color-popup/23_alpha_slider_drag_canvas_pressed` 7.71 %, `fx-bevel-emboss-fill/09_live_pressed` 21.8 %), the five new
`tool-text` pairs are within 1 % (0.11–0.47 %), and the other 25 canvas pairs of the four units stay on their side of
1 % within 0.02 points of run 39 (`tool-text/13_color_live_result` 0.76 → 0.78 %, the caret); over the 48 old units
187 of 216. One class changed: `tool-text` equal except for design → real deviation (TW-219: the same Scale-tool corner
drag gives 83.74 pt in the reference and 90 pt in Firepixel, because Firepixel's transform frame of a text is its
bounds). Compared values: on Firepixel's side none changed (the new steps add the values of TW-212 and TW-219); on the
reference's side the text read before typing in `tool-text/07_select_text_double_click_focus` (one empty text, as in run
38) and the History read-back of `inspector-fields/25` (2, then 2 → 6; run 39: 0 → 0). Firepixel's shots are
pixel-identical to run 39 except the caret in `tool-text/13`. The other changed pairs come from the reference: the
overview pairs of `tool-text` (21.5 → 19.3 %, its dock groups at the unit start), `fx-bevel-emboss-fill` (19.7 → 21.8 %)
and `inspector-fields` (19.6 → 21.7 %, its groups at 228–231 px after the restarts), and `color-popup/01` and
`23_alpha_slider_drag_pressed` (232 and 247 px, within 0.01 points). 7 `runner error` lines, all reference-side in
`tool-text` (run 39: 8; over the newest run of every unit: 7).

Run 39's totals (on top of runs 38, 37, 36, 35, 34 and 33), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**).
Step level: 1090 of 1243 measurable steps `ok` on both sides (**87.7 %**; the nine units 190 / 224, run 38: 191 / 224,
with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24:
1081 of 1234 (87.6 %). The 228 steps of run 39 (the steps of run 38): 190 `ok` on both sides, 22 `unmapped` on both
sides, 5 reference-side `error` against `ok`, 3 `ok` against a Firepixel-side `error`, 3 `error` on both sides, 2
`unsupported` on both sides, 1 `ok` / `unmapped`, 1 `unsupported` / `unmapped` and 1 `unsupported` / `ok`. One step
status changed against run 38: `fx-bevel-emboss-fill/09_live_pressed` is a reference-side `error` ("no popup slider
open"; was `ok`; it failed the same way in run 33). Errors: **36 reference-side and 21 Firepixel-side** (run 38: 35
and 21). Canvas image pairs within 1 %: **191 of 218** (run 38: 191 of 219): `fx-bevel-emboss-fill/09_live_pressed`
(21.8 %, above 1 %) lost its reference shot, and the remaining 49 canvas pairs of the nine units stay on their side of
1 % within 0.02 points of run 38 (`tool-text/13_color_live_result` 0.78 → 0.76 %, the caret); over the 48 old units
182 of 209. One class changed: `panel-layers` unclear → equal except for design (TW-208 confirmed with R-34; Q-66 was
the runner's click). Compared values: on Firepixel's side only `panel-layers/08` (1, then 0; run 38: 1, then 1; the
reference 1, then 0); on the reference's side the History read-back of `panel-layers/22` (0 → 2 after a retry, run 38:
0 → 0), the text read before typing in `tool-text/07_select_text_double_click_focus` (no text yet; run 38: one empty
text) and the export of `tool-text/12`. Firepixel's shots are pixel-identical to run 38 except the layer context menu
of `panel-layers-menu/03` (24 px higher, pair 17.5 → 16.3 %), F85's grey circle on the empty "Ebene 1" in the
`tool-text` overview (60 px) and the caret in `tool-text/07`, `13`. The other changed pairs come from the reference:
`tool-text/00_overview_normal` 19.3 → 21.5 % (its dock after the restart), `inspector-text/23_tool_vs_object_normal`
19.9 → 24.1 % (its document inspector, as in run 37), `inspector-text/04` 85.2 → 85.3 % and
`fx-bevel-emboss-fill/08_angle_pressed` 92.5 → 92.8 % (its run 37 value). 8 `runner error` lines, all reference-side
in `tool-text` (run 38: 10; over the newest run of every unit: 8).

Run 38's totals (on top of runs 37, 36, 35, 34 and 33), for comparison: **48 / 0 / 1 / 0** (proven 1:1 **98.0 %**).
Step level: 1091 of 1243 measurable steps `ok` on both sides (**87.8 %**; the nine units 191 / 224, run 37: 190 / 223,
with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24:
1082 of 1234 (87.7 %). The 228 steps of run 38 (the steps of run 37): 191 `ok` on both sides, 22 `unmapped` on both
sides, 4 reference-side `error` against `ok`, 3 `ok` against a Firepixel-side `error`, 3 `error` on both sides, 2
`unsupported` on both sides, 1 `ok` / `unmapped`, 1 `unsupported` / `unmapped` and 1 `unsupported` / `ok`. Three step
statuses changed against run 37: `panel-layers/09_drag_reorder_pressed` is `ok` on Firepixel's side again (was
`unmapped`) and `panel-layers/12_selection_dot_result` `ok` on both sides (was `unmapped` / `unsupported`, so one more
measurable step), both by R-33, and `color-popup/23_alpha_slider_drag_canvas_pressed` is a reference-side `error` ("no
popup slider open"; was `ok`). Errors: **35 reference-side and 21 Firepixel-side** (run 37: 34 and 21). Canvas image
pairs within 1 %: **191 of 219** (run 37: 191 of 220): `color-popup/23_alpha_slider_drag_canvas_pressed` (7.71 %,
above 1 %) lost its reference shot, `inspector-text/25` fell from 0.52 to 0.09 % (TW-214), and the other 49 canvas
pairs of the nine units stay on their side of 1 % within 0.02 points of run 37 (`tool-text/14_bounds` 1.05 → 1.07 %,
now with the reference's text); over the 48 old units 182 of 210. Two classes changed: `inspector-text` real deviation
→ equal except for design (TW-214 confirmed, TW-204 as documented) and `panel-layers` real deviation → unclear (TW-205
confirmed; TW-208's range confirmed, the Ctrl-click after it 0 against 1 although Firepixel's UI test reads 0, Q-66).
Compared values: on Firepixel's side only where the fixes act: `panel-layers/08` (1, then 1; run 37: 2, then 1; the
reference 1, then 0), `09_drag_reorder_result` (the order unchanged, as in the reference), `12` (newly compared,
`currentLayerNum` 0 in both), `23_opacity_slider_press_drag_result` (80, was 81.18) and `inspector-text/25` (the
reference's bounds and ink); on the reference's side the History read-back of `panel-layers/22` (0 → 0, run 37: 0 → 2),
the click-mode pop-up state of `panel-layers/23`, the export of `tool-text/12` and the modified title in
`inspector-fields/15`. Firepixel's shots are pixel-identical to run 37 except the Layers panel (F79's disclosure, F83's
current layer, circles, tints and scroll), the slider pop-ups and live previews of F79 (`color-popup/23`,
`inspector-fields/09`, `14`, `panel-layers/23`), the frames of script-made text of F82 (`inspector-text/06`, `10`, `12`,
`21`, `tool-text/02`), the shot area of `panel-layers-menu/03` and the caret in `tool-text/01`, `13`. 10 `runner error`
lines, all reference-side in `tool-text` (run 37: 11; over the newest run of every unit: 10).

Run 37's totals (on top of runs 36, 35, 34 and 33), for comparison: **47 / 2 / 0 / 0** (proven 1:1 **95.9 %**). Step
level: 1090 of 1242 measurable steps `ok` on both sides (**87.8 %**; the nine units 190 / 223, run 36: 177 / 215, with
the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24: 1081
of 1233 (87.7 %). The 228 steps of run 37 (the 218 of run 36 and ten new ones): 190 `ok` on both sides, 22 `unmapped`
on both sides, 3 reference-side `error` against `ok`, 3 `ok` against a Firepixel-side `error`, 3 `error` on both
sides, 2 `unsupported` on both sides, 2 `ok` / `unmapped`, 1 `unsupported` / `unmapped`, 1 `unmapped` / `unsupported`
and 1 `unsupported` / `ok`. Seven step statuses changed against run 36, all in `panel-layers`: `07`, `08`, `10` and
`16` are `ok` on Firepixel's side (were `unmapped`), `09_drag_reorder_pressed` is `unmapped` on Firepixel's side (was
`ok`), and `14_opacity_blend_result` (`unsupported` / `ok`) and `12_selection_dot_result` (`unmapped` /
`unsupported`), both `unmapped` on both sides before, leave the measurable steps; the ten new steps are `ok` on both
sides. Errors: **34 reference-side and 21 Firepixel-side** (unchanged; the six errors of each side in the nine units
are the same steps). Canvas image pairs within 1 %: **191 of 220** (run 36: 186 of 212): the eight new canvas pairs
add five within 1 % (`panel-layers/23_opacity_slider_press_drag_result` 0.10 %,
`color-popup/23_alpha_slider_drag_result` 0.18 %, `inspector-text/25` 0.52 %,
`inspector-fields/25_pattern_move_normal` and `_result` 0.09 %) and three above it, all by Firepixel's live preview
(`panel-layers/23_opacity_slider_drag_pressed` and `_click_drag_result` 2.10 %,
`color-popup/23_alpha_slider_drag_canvas_pressed` 7.71 %); the other 43 canvas pairs stay on their side of 1 %; over
the 48 old units 182 of 211. Two classes changed: `panel-layers` and `inspector-text` equal except for design → real
deviation (TW-205 and TW-208; TW-214, with the bounds of TW-204 as documented). Compared values: `panel-layers/07`,
`08`, `10` and `14` now read on Firepixel's side (`08` unequal, TW-208), `09_drag_reorder_result` is unequal now
(TW-216), the reference's `color-popup/05` values now equal Firepixel's (#3366cc, #000000) and its text exports in
`inspector-text/09`, `13`, `14` show the edits; the new steps add the values of TW-204 to TW-207. Firepixel's shots
are pixel-identical to run 36 except the four `panel-layers` shots with the expanded layers and the caret in
`tool-text/07`, `13`. The reference-side changes come from R-32 (the HEX field, the typed fields, the left alignment),
its dock groups (overview pairs), no restart in `inspector-text`, its lost first characters in `tool-text/12`, `14`
and its pop-up shot in `inspector-fields/09`. 11 `runner error` lines, all reference-side in `tool-text` (run 36: 7;
over the newest run of every unit: 11).

Run 36's totals (on top of runs 35, 34 and 33), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**). Step
level: 1077 of 1234 measurable steps `ok` on both sides (**87.3 %**; the nine units 177 / 215, their baselines
174 / 215, with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units
before run 24: 1068 of 1225 (87.2 %). The 218 steps of run 36: 177 `ok` on both sides, 24 `unmapped` on both sides,
5 `ok` / `unmapped`, 3 reference-side `error` against `ok`, 3 `ok` against a Firepixel-side `error`, 3 `error` on both
sides, 2 `unsupported` on both sides and 1 `unsupported` / `unmapped`. Three step statuses changed against the
baselines, all on the reference side and all back to `ok`: `fx-bevel-emboss-fill/09_live_pressed`,
`tool-text/11_size_live_result` and `tool-text/13_color_live_result` (the three reference-side errors new in run 33);
no Firepixel-side status changed. Errors: **34 reference-side and 21 Firepixel-side** (run 35: 37 and 21). Canvas image
pairs within 1 %: **186 of 212** (run 35: 185 of 209): the three steps bring back their canvas pairs, `tool-text/13`
within 1 % (0.80 %, although the reference's edit did not happen, see the unit row), `tool-text/11` (2.35 %) and
`fx-bevel-emboss-fill/09` (21.8 %) above it; the other 40 canvas pairs of the nine units stay on their side of 1 %; over
the 48 old units 177 of 203. No class changed. Compared values: two changed, both on the reference side:
`panel-layers/22_undo_result` (History count 0 → 2, now equal to Firepixel's) and the text bounds of `tool-text/07`
(right-aligned, now unequal; runner item TW-211). Firepixel's values and exports read as in the baselines, and its shots
are pixel-identical to them except the Pointer display of F65 in `fx-bevel-emboss-fill` and the colour wells in
`color-popup-contexts/04` (both against run 33), the Edge pop-up of `inspector-fields/10` (Dither entry, F67), the font
list's scroll bar in `inspector-text/02` and 4 caret pixels in `tool-text/07`; no shot shows an effect of batch 6. The
reference-side changes come from its dock groups (overview pairs), the restored steps, its stored text alignment, its
document inspector in `inspector-text/23`, its angle dial in `fx-bevel-emboss-fill/08`, its Mixer in
`color-popup-contexts/12`, its History read-back in `panel-layers/22` and its export hashes. 7 `runner error` lines, all
reference-side in `tool-text` (run 35: 0; over the newest run of every unit: 7).

Run 35's totals (on top of runs 34 and 33), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**). Step level:
1074 of 1234 measurable steps `ok` on both sides (**87.0 %**; the six units 159 / 176, the same as in their baselines,
with the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24:
1065 of 1225 (86.9 %). The 178 steps of run 35: 158 `ok` on both sides, 14 `unmapped` on both sides, 2 reference-side
`error` against `ok` (`color-popup/21_undo_result`, `inspector-fields/10_edge_popup_open`), 1 `unsupported` on both
sides, 1 `unsupported` / `unmapped`, 1 `ok` against a Firepixel-side `error` (`inspector-fields/20_undo_each_result`:
no text field at the stroke size, the known error of runs 30 and 33) and the Firepixel-only step. No step status
changed against the baselines. Errors: **37 reference-side and 21 Firepixel-side** (unchanged). Canvas image pairs
within 1 %: **185 of 209** (unchanged; the six units 32 of 40, the same pairs on the same side of 1 %;
`inspector-fields/14_popup_drag_live_canvas_pressed` 2.03 → 1.58 % stays above); over the 48 old units 176 of 200. No
class changed. Firepixel-side value changes: only the `ui_state` tool fill in `inspector-fields/21` (#808080 →
#000000, F61) and the fit zoom of `toolbox` (back at run 33); every other Firepixel value and export reads as in the
baselines. Firepixel's shots are pixel-identical to the baselines except the Pointer display of a selected rectangle
(F65), the slider pop-ups (F64), the width field's tooltip (F66) and, against run 33, the stroke well (F61). The
reference-side changes come from its expanded inspector (TW-197 did not recur), its dock group heights, the Text tool
inspector in `inspector-text/23`, the export hashes in `inspector-text` and the Freeform push in
`tool-path-reshape/02` (20 px). 0 `runner error` lines (run 34: 0; over the newest run of every unit: 4, all in
`tool-text` from run 33).

Run 34's totals (on top of run 33), for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**). Step level: 1074 of
1234 measurable steps `ok` on both sides (**87.0 %**; the eight units 212 / 225, run 33: 213 / 225, with the
Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24: 1065 of
1225 (86.9 %). The 234 steps of run 34: 211 `ok` on both sides, 7 `unsupported` and 6 `unmapped` on both sides,
4 `unmapped` / `ok`, 2 `unsupported` / `ok`, 2 reference-side `error` against `ok`, 1 `error` on both sides and the
Firepixel-only step. One step status changed against run 33, on the reference side: `tool-autoshapes/10_scale_result`
`ok` → `error` (another application's window took the foreground in the step and in both repeats); no Firepixel-side
status changed. Errors: **37 reference-side and 21 Firepixel-side** (run 33: 36 and 21). Canvas image pairs within 1 %:
**185 of 209** (run 33: 184 of 209): `tool-autoshapes/05_handle_drag_rounded_corner_result` moved back below 1 %
(0.97 %, TW-195 fixed); the other 82 canvas pairs of the eight units stay on their side of 1 %; over the 48 old units
176 of 200. One class changed: `tool-autoshapes` real deviation → equal except for design (TW-195 fixed and confirmed).
Firepixel-side value changes: the TW-195 fix in `tool-autoshapes` (the strokes of the Arrow Line and of every later
shape, the brush names, the Measure shape no longer selected after drawing), the refined follow rules of TW-192 (the
tool fill #808080 after P1 in `tool-path-reshape/02` to `07`, the tool colours white / black after Insert ▸ Hotspot in
`tool-web/12` to `15`) and the zoom of `toolbox` before the restart (TW-197); every other Firepixel value and export
reads as in run 33, and Firepixel's shots are pixel-identical to run 33 except the stroke well, the anti-aliasing of FR
in `xs-select-behind/00, 05`, the auto shapes and the canvas behind the `toolbox` flyouts. The reference-side changes
come from R-31 (the first six auto shapes and FR drawn with 1 px), R-30 and the restarts (no stored state after the
`toolbox` restart; no restart in `xs-canvas` and `xs-windows-tabs`), its collapsed Properties inspector (TW-197), its
History read-back (`menu-modify-a/18, 21`) and the focus loss in `tool-autoshapes/10`. 0 `runner error` lines (run 33:
4, all in `tool-text`, which run 34 did not repeat; over the newest run of every unit: 4).

Run 33's own totals, for comparison: **48 / 1 / 0 / 0** (proven 1:1 **98.0 %**). Step level: 1075 of 1234
measurable steps `ok` on both sides (**87.1 %**; counted as before, with the Firepixel-only step
`toolbox/17_ai_tools_normal` as `ok`); over the 48 units before run 24: 1066 of 1225 (87.0 %). The 1295 steps: 1074
`ok` on both sides, 81 `unmapped` and 37 `unsupported` on both sides, 10 `error` on both sides, 23 reference-side
`error` against `ok`, 10 `ok` against a Firepixel-side `error`, 20 `unmapped` / `ok`, 11 `unsupported` / `ok`, 11 `ok` /
`unmapped`, 10 `unsupported` / `unmapped`, 3 `error` / `unmapped`, 3 `unmapped` / `unsupported`, 1 `unmapped` / `error`
and the Firepixel-only step. Seven step statuses changed against the units' newest earlier runs, all on the reference
side: `ok` again in `dialogs-d01-d10/19`, `inspector-fields/14_popup_drag_live_popup` and
`panel-pages-states-history/18` (where Firepixel's error stays), `ok` for the first time in `export-results/13` (was
`unsupported`, so one more measurable step), and new reference-side errors in `fx-bevel-emboss-fill/09` and
`tool-text/11, 13`; no Firepixel-side status changed. Errors: **36 reference-side and 21 Firepixel-side** (the same
totals; the 21 Firepixel-side errors are the same steps with the same texts as before). Canvas image pairs within 1 %:
**184 of 209** (run 32: 185 of 212): three pairs lost their reference shot (`fx-bevel-emboss-fill/09` and
`tool-text/11`, both above 1 % before, and `tool-text/13`, below), `menu-modify-a/20` moved below 1 % (0.17 %, TW-192)
and `tool-autoshapes/05` above it (1.01 %, TW-195); over the 48 old units 175 of 200. One class changed:
`tool-autoshapes` equal except for design → real deviation (TW-195). Firepixel-side value changes: the TW-192 to TW-194
fixes (`menu-modify-a/20, 22`, `tool-path-reshape/11`, `tool-autoshapes/11`, `xs-select-behind`,
`panel-layers-menu/04`), the TW-189 Enter rule in `inspector-group-symbol/02`, the TW-195 chain in `tool-autoshapes`
and the tool colours in `ui_state`, which now follow the selection (F58); every other Firepixel value and export reads
as in the baseline runs, and every Firepixel shot is pixel-identical to its baseline except where these changes, the
colour wells, the marching ants or the running animation show. The reference-side changes come from the real keys
(menus closed at the read-backs of the `menu-*` units, the hidden panels of `menu-view/15`, the Escape of
`menu-modify-a/22`), the icon dock of R-29 (`panel-layers/04`, `panel-swatches-mixer/19`, the panel pairs), its
restarts (TW-196), its History read-back (`menu-edit/01, 22`, `panel-swatches-mixer/20`), its shot of edited text
(`tool-text/11, 13`) and its text entry. 4 `runner error` lines (run 32: 2; over the newest run of every unit before
run 33: 12), all reference-side text entry and selection in `tool-text` (TW-146 rest).

Run 32's own totals, for comparison: **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**). Step level:
1075 of 1233 measurable steps `ok` on both sides (**87.2 %**, unchanged; the three units 93 / 101 in both runs, with
the Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24: 1066
of 1224 (87.1 %). No step status changed against run 31 in the three units (103 steps: 92 `ok`, 8 `unmapped` and 2
`unsupported` on both sides, and the Firepixel-only step); they have no `error` step on either side, so the totals stay
**36 reference-side and 21 Firepixel-side errors**. Canvas image pairs within 1 %: **185 of 212** (unchanged; the six
canvas pairs of the three units have 2 within 1 % as in runs 30 and 31 and are all back at their run 30 values, because
Firepixel's canvas shots are pixel-identical to run 30 again); over the 48 old units 176 of 203. Two classes changed:
`xs-shortcuts` real deviation → equal except for design (TW-189 fixed) and `xs-preferences` unclear → equal except for
design (Q-60 settled as a runner matter). The only Firepixel value read-back that changed is that of TW-189
(`xs-shortcuts/22`); the reference-side changes come from R-28 (no re-selection before Delete in `xs-shortcuts/02`),
its text entry (`xs-shortcuts/15`) and its File menu state (`xs-shortcuts/20`). 2 `runner error` lines in run 32 (run
31: 10, all in `tool-text`, which run 32 did not repeat), both in the shot-only step `xs-shortcuts/15_in_text_result`:
the reference's text entry lost the "V" in two attempts and the empty text was discarded at the commit (TW-146 rest);
over the newest run of every unit that makes 12 lines (`tool-text` 10 from run 31, `xs-shortcuts` 2).

Run 31's own totals, for comparison: **47 / 1 / 1 / 0** (proven 1:1 **95.9 %**). Step level: 1075 of
1233 measurable steps `ok` on both sides (**87.2 %**, unchanged; the eight units 201 / 223 in both runs, with the
Firepixel-only step `toolbox/17_ai_tools_normal` counted as `ok` as before); over the 48 units before run 24: 1066 of
1224 (87.1 %). No step status changed against run 30 in any of the eight units (229 steps: 200 `ok` on both sides,
13 `unmapped` and 5 `unsupported` on both sides, 4 `error` on both sides, 3 reference-side and 1 Firepixel-side
`error` against `ok`, 1 `unsupported` / `ok`, 1 `unmapped` / `ok`, and the Firepixel-only step); their 7
reference-side and 5 Firepixel-side errors are the same steps with the same texts as in run 30, so the totals stay
**36 reference-side and 21 Firepixel-side errors**. Canvas image pairs within 1 %: **185 of 212** (unchanged; the 44
canvas pairs of the eight units have 29 within 1 % in both runs; 35 of them carry Firepixel's coordinate tag, TW-190,
and moved by −0.05 to +0.31 points, the falls from reference-side changes; none crossed 1 %); over the 48 old units 176 of 203. Three classes changed: `menu-edit`
real deviation → equal except for design (TW-173 fixed), `xs-shortcuts` equal except for design → real deviation
(TW-189 confirmed) and `xs-preferences` equal except for design → unclear (Q-60). The only value read-backs that
changed on Firepixel's side are those of TW-173 (`menu-edit/02` and the Edit menu of `menu-edit/01, 07`); every other
Firepixel value of the eight units, and every Firepixel export, reads as in run 30. The reference-side changes come
from the real keys of R-27 (`xs-shortcuts/22`, `xs-preferences/06, 07, 20`, the History rows of
`panel-pages-states-history`), its History read-back (`menu-edit/01, 22`), its panel layout (TW-174) and its text
entry (`tool-text`). R-27 sent 307 plain keys on the scan-code path and 104 modified keys on the vk path, with no
refused input; run 30 had used 165 R-19 substitutes in 45 steps of these units. 10 `runner error` lines (run 30: 11),
all reference-side text entry and selection in `tool-text`: three entry lines (`02`, and `09` twice) and the seven
selection and size lines of TW-146 rest.

Run 30's own totals, for comparison: **48 / 1 / 0 / 0** (proven 1:1 **98.0 %**). Step level: 1075 of 1233
measurable steps `ok` on both sides (**87.2 %**; counted as before, with the Firepixel-only step
`toolbox/17_ai_tools_normal` as `ok`); over the 48 units before run 24: 1066 of 1224 (87.1 %). Two quotas dropped by
one step each, both by a reference-side miss (`dialogs-d01-d10/19`, `inspector-fields/14_popup_drag_live_popup`);
none rose. Only three step statuses changed against the baseline runs, all on the reference side (those two `ok` →
`error`, and `panel-pages-states-history/18` `ok` → `error`, where Firepixel's error is unchanged). Reference-side
errors: 36 (run 29: 33); Firepixel-side errors: 21, unchanged: the same 21 steps failed with the same error text in the
baseline runs, and no step that was `ok` there fails on Firepixel's side now. Canvas image pairs within 1 %: **185 of
212** (count unchanged; `inspector-fields/14_popup_drag_live_canvas` moved above 1 %, 2.03 %, from the deliberate live
preview of TW-171, and `tool-autoshapes/05_handle_drag_rounded_corner` moved below, 0.97 %); over the 48 old units
176 of 203. One class changed: `menu-edit` equal except for design → real deviation (TW-173). Value read-backs that
changed on Firepixel's side: `menu-edit/02` (TW-173), `xs-shortcuts/02, 17` and `panel-layers-menu/03` (now equal to
the reference) and `inspector-stroke/03` (no step for a clamped value, TW-171); the reference-side changes in
`panel-layers/04`, `panel-swatches-mixer/19` and `xs-select-behind` come from its panel layout and tool state
(TW-174, Q-58). 11 `runner error` lines (run 29: 0; run 28: 7), all reference-side text entry and selection in
`tool-text`: four lost first characters in `03`, `04`, `05` and `20` (new; the entry part had worked in runs 26 to
28), six selection tries in `07`, `11` and `13` and the size step of `11` (TW-146 rest, click-test A1-19).

Run 29's own totals, for comparison: 49 / 0 / 0 / 0 over 49 units; 1077 of 1233 steps (87.3 %); in its 3 units 85 of
88 steps `ok` on both sides (`toolbox` 53 / 56, `tool-path-reshape` 23 / 23, `xs-select-behind` 9 / 9; the three
`toolbox` misses are the unmapped flyouts and the collapse button, as before), all quotas unchanged; 33 reference-side
and 21 Firepixel-side errors (no Firepixel-side error in its units); 185 of 212 canvas pairs within 1 %; no `runner
error`. The reference was restarted once (script access violation at the start of `xs-select-behind`), without lost
steps.

Run 28's own totals, for comparison: 49 / 0 / 0 / 0 over 49 units; 1077 of 1233 steps (87.3 %); in its 2 units 76 of
83 (91.6 %), the same as in run 27; 33 reference-side and 21 Firepixel-side errors (in the two units 2 and 3:
`tool-text/15, 19` frame handle not found on both sides, `tool-text/12` Firepixel's font field); 185 of 212 canvas
pairs within 1 % (the 19 canvas pairs of the two units read as in run 27). No class changed. The 7 `runner error`
lines (run 27: 2) are all reference-side text selections in `tool-text` (TW-146 rest): `07` ("Welt"), four selections
and the size step in `11`, and `13`; R-25 no longer accepts the 24-pixel caret as a highlight. `toolbox` has none
(TW-166 confirmed). The reference was not restarted.

Run 27's own totals, for comparison: 49 / 0 / 0 / 0 over 49 units; 1077 of 1233 steps (87.3 %); in its three units
99 of 106 (93.4 %); 33 reference-side and 21 Firepixel-side errors; 185 of 212 canvas pairs within 1 %
(`tool-path-reshape/04` added, `tool-text/14, 16` above 1 %, TW-165). No class changed. 2 `runner error` lines:
`tool-text/11` (TW-146 rest) and `toolbox/13` (TW-166). The reference was restarted once after a script access
violation at the start of `toolbox`, without lost steps.

Run 26's own totals, for comparison: 49 / 0 / 0 / 0 over 49 units; 1077 of 1233 steps (87.3 %); in its five units
142 of 153 (92.8 %; `toolbox` had 53 / 56 in run 25 already, its row and the run-25 totals used run 24's 52, which
added one step to the totals); `xs-shortcuts` regained `10_nudge_point_result` (+0.3 points, TW-159 b); 33
reference-side and 21 Firepixel-side errors; 186 of 211 canvas pairs within 1 %. One class changed: `toolbox` real
deviation → equal except for design (TW-158). 8 `runner error` lines: six from the reference's Standard reset in
`toolbox/13` (TW-163) and two reference-side text selections in `tool-text/11, 13` (TW-146 rest). The reference was
restarted once after a script access violation in `toolbox`, without lost steps.

Run 25's own totals, for comparison: 48 / 1 / 0 / 0 over 49 units; 1075 of 1232 steps (87.3 %, with `toolbox` counted
at 52); 33 reference-side and 21 Firepixel-side errors; 186 of 211 canvas pairs within 1 % (`tool-autoshapes` 26 / 27,
TW-153). Four classes changed: `panel-pages-states-history` and `xs-select-behind` real deviation → equal except for
design (TW-148, TW-157), `tool-text` unclear → equal except for design (TW-145 decided) and `toolbox` equal except for
design → real deviation (TW-158). 4 `runner error` lines, all reference-side text entry in `tool-text` (TW-146 rest).
The reference was restarted three times after a script access violation at unit start, without lost steps.

Run 24's own totals, for comparison: 46 / 2 / 1 / 0 over 49 units; 1074 of 1235 steps (87.0 %); 37 reference-side
and 23 Firepixel-side errors; 164 of 210 canvas pairs within 1 % (`tool-autoshapes` 7 / 27, TW-153). Two classes
changed: `panel-pages-states-history` unclear → real deviation (TW-148) and the first classification of
`xs-select-behind`, real deviation (TW-157). 13 `runner error` lines (12 reference-side text entries in `tool-text`,
one real Down key in `inspector-stroke/01`).

Run 23's own totals, for comparison: 46 / 0 / 2 / 0 over 48 units; in its five units 101 of 120 (84.2 %; the same
units in run 22: 102 of 120); 175 of 201 canvas pairs within 1 % (newest run per unit). One class changed:
`panel-pages-states-history` equal except for design → unclear (TW-148). 9 `runner error` lines, all reference-side
text entry in `tool-text`.

Run 22's own totals, for comparison: 47 / 0 / 1 / 0; in its seven units 138 of 159 (86.8 %), the same as in runs 20
and 21; 175 of 201 canvas pairs within 1 % (`tool-text/13` without reference shot). Two classes changed:
`xs-shortcuts` real deviation → equal except for design (TW-143) and `xs-preferences` unclear → equal except for
design (TW-134). The 15 `runner error` lines were runner artefacts (13 in `xs-shortcuts/07`, 2 in `tool-text/11, 13`).

Run 21's own totals, for comparison: 45 / 1 / 2 / 0; in its seven units 138 of 159 (86.8 %), the same as in run 20;
the quota was unchanged in all seven; 176 of 202 canvas pairs within 1 % (`tool-text/02` moved below 1 % with the
reference's text tool at 12 pt). No class changed.

Run 20's own totals, for comparison: 45 / 1 / 2 / 0; in its seven units 138 of 159 (86.8 %), unchanged from their
previous runs; 175 of 202 canvas pairs within 1 %. The class changed in one unit: `menu-edit` real deviation →
equal except for design (both Histories list the Copy row, TW-137).

Run 19's own totals, for comparison: 44 / 2 / 2 / 0; in its six units 121 of 139 (87.1 %), unchanged from their
previous runs; 174 of 202 canvas pairs within 1 %. The class changed in two units:
`panel-library-styles` real deviation → equal except for design (TW-131), `menu-edit` unclear → real deviation (the
history read-back now works and shows the missing Copy row, TW-137); `panel-pages-states-history` stays equal with
the States menu fixed.

Run 18's own totals, for comparison: 43 / 2 / 3 / 0; in its ten units 214 of 237 (90.3 %; the same units in run
17: 205 of 236). The quota is unchanged in 7 of the 10 units and improved in 3 (`dialogs-d31-d40` +24.0,
`xs-canvas` +4.7, `panel-swatches-mixer` +3.7 points); none dropped. Reference-side errors in the ten units: 8 (run
17: 16); Firepixel-side errors: 6 (unchanged, all known). Over all 48 units: 38 reference-side and 21 Firepixel-side
errors. Canvas image pairs within 1 %: **174 of 202** (run 17: 176 of 202; the two pairs that moved above 1 % are
`tool-text/02` and `/14`, caused by the reference's text-tool state).

Run 17's own totals, for comparison: 42 / 1 / 5 / 0; 1057 of 1225 steps (86.3 %); 46 reference-side and 21
Firepixel-side errors; 176 of 202 canvas pairs within 1 %.

**Menu widths after the 33 px arrow column (TW-123).** Panel menus are now within 5 px of the reference (the extra
pixels are Firepixel's frame in the shot): Pages 285 vs. 280, History 282 vs. 277, Swatches 231 vs. 226, Layers
362 vs. 354, Optimize colour table 282 vs. 270 (was about 600 px). Main menus stay narrower than the reference
(Edit 339 vs. 393, View ▸ Guides 639 vs. 699, Filters 540 vs. 625); arrow and shortcut gaps are equal. No menu
became wider than the reference because of the arrow column. Short panel menus kept a minimum width of about
230 px in run 17 (Common Library 231 vs. 179, States 231 vs. 207); after the TW-125 fix run 18 measures the Common
Library menu at 183 vs. 179 px and run 19 the States menu at 210 vs. 207 px.

**What run 17 found.** Most new differences come from one runner change: since R-12 the runner writes the
"Firepixel" shortcut set into Firepixel's profile, and in that set Ctrl+Shift+D is not Clone, Ctrl+Shift+Period
does not enlarge text, and Ctrl+Shift+E, Ctrl+Alt+Shift+S and Ctrl+Shift+I do other things than on the reference
machine (TW-126; the reference machine's active set needs a behaviour description). `panel-optimize` shows one real
deviation: export formats chosen per slice in the Optimize panel are not stored on the slices (TW-128).
`xs-preferences` clicks stored settings without a fixed start state (TW-127, runner).

**What run 44 found.** The follow-up run over the four units touched by tester feedback batch 8 changes no step status,
no class and no compared value; every pair reads its run 43 value except where batch 8 shows, so no Firepixel-side
regression is found. With the new runner setting `fp.layers_group_tree` off (Show Group Contents switched off at every
unit start) Firepixel lists the reference's Layers rows, and the confirmed rows of `panel-layers` (TW-205, TW-208,
TW-215, TW-216, Q-65) read as in runs 38 to 43. **Batch 8 as far as the runner reaches it:** F100 frames a selected
object's own thumbnail blue (2 px #0078d7; the reference 2 px #00a8ff) in nine shots with the Layers rows, as in the
reference's shots of the same steps (**TW-238**, first part); the Layers options menu lists the extra entry Show Group
Contents, unchecked, under Thumbnail Options (**TW-237**; the options-menu pairs 15.6 → 23.4 %, like View ▸ Coordinate
Tag in run 43). `panel-layers-menu/10_options_open` fails on Firepixel's side as in every run since run 17 (the step
double-clicks the "Ebene 1" row; the reference opens its Layer Options dialog, Firepixel none); it does not open the
options menu, so the extra entry does not shift its lookup. Not reached: the row kept highlighted during mask editing
(TW-238, second part; no step selects a mask in the Layers panel), F98's page placement, page names, page row context
menu, drag onto the New Page button and History row "Change Page Selection" (**TW-233**; the page steps read only the page
count, which reads 3 → 4 → 3 as before, and the History steps of `panel-pages-states-history` click no page row, so its
History shots and read-backs are unchanged), F99's strokes at the bitmap bounds (TW-234, TW-235) and the Confine checkbox
(**TW-236**; no shot holds a retouch or brush tool's inspector). `inspector-bitmap/08` shows the open TW-225 rest that
Reveal All does not select the new mask (the reference edits the mask with a green frame and its Bitmap mask inspector,
Firepixel keeps the bitmap selected; both sides as in run 33). Reference-side: one restart (access violation while
opening `par_bitmap.fw.png` at the start of `inspector-bitmap`, no step lost) and a pressed shot without the slider pop-up
(`panel-layers/23`, pair 64.1 → 60.4 %). Headline unchanged: 49 / 0 / 0 / 0, proven 1:1 100.0 %, quota 88.0 %
(1095 / 1244), canvas pairs 195 / 224.

**What run 43 found.** The full run on the batch 7 preview changes no compared value and no Firepixel-side step
status. The confirmed rows of the follow-up runs hold in their units: TW-205, TW-208, TW-215, TW-216 and Q-65
(`panel-layers`), TW-206 (`color-popup`), TW-207 (`inspector-fields`), TW-204 and TW-214 (`inspector-text`), TW-212 and
TW-219 (`tool-text`, 83.74 pt again), and the runner rows TW-197, TW-210, TW-211, TW-216 and TW-221 behaved as in runs 37
to 42. **Batch 7 as far as the runner reaches it:** F95's dither density field shows in every fill row with a solid fill
and its Screentone entry in the effects menu (TW-227); F94's Marquee list shows in the Pointer inspector (TW-223); F92's
scroll bar shows in the Path and Image Editing panels, and the docked panels' lists are 2 px shorter, which moves the
Layers footer and rows by 2 px and the runner's row positions by 1 px without changing a value (TW-224); F88a's luma
mask thumbnail shows in the Layers panel (TW-225; no step selects a mask, so the green frame and the mask inspectors are
in no shot); F90's Coordinate Tag entry shows in the View menu, and with the runner's new setting Firepixel's
View ▸ Tooltips is checked (TW-226). Not reached: the pause key (TW-222), the retouch and smudge tools (TW-228, no unit
paints with them), F96's padlock and h1 drag (no step shows or drags a pattern fill's handles) and F97's line rotate
(TW-229), the slider undo of F89 and F97's click outside an open pop-up (TW-230; the pop-up slider steps read as before),
the cursor badges and marquee modifiers of F94 (no marquee drag with a modifier). **New: TW-231** (`xs-context-menus`,
first read as unclear, now a runner row): after the right-click with L1 selected, the label click "Löschen" left L1 in
place (7 objects; 6 in runs 3 to 33); the reference side is `unmapped`. The hand check CHK-1 (24 Oct 2026, both
previews in a real window) found no Delete entry in the canvas context menu of either build; the runner's window-wide
fallback clicked the Properties "Stil löschen" button in `2026-10-24_08-38` and the dock's "Auswahl löschen" in the
earlier builds (batch 7 only reordered the accessibility nodes), so it is no regression (runner item R-37), and run 43
reads 49 / 0 / 0 / 0, proven 1:1 100.0 %. **New: TW-232** (runner): the reference's live-filter "+" button opened no menu in four
`filters-adjust-color` steps. The rest is reference-side: `tool-autoshapes/10` measured again, the inspector and page
place of run 33 back in the units of run 34, six restarts without lost steps and 9 `runner error` lines in `tool-text`
(TW-146 rest).

**What run 42 found.** Runner R-36 reaches F87's frame, and TW-219 is confirmed. **TW-219:** R-36's `transform_handles`
reads the same selection on both sides (the 60 pt point text "Ag", raw 100,100 74 × 72, identity, bounds 98,98–176,174)
and derives the same frame by the rule of RE spec 67 (88.9–185.1 × 89.2–182.8, shown at 89/137/185 × 89/136/183); both
sides press the bottom-right handle at document 184,182 and drag it to 223,220 under Modify ▸ Transform ▸ Scale, and both
programs end at 83.74 pt, horizontal scale 1, the leftover matrix [0.993, 0, 0, 1.005], t 5.085 / 23.883, raw 100,80
104 × 100, bounds 102.406,102.263 107.251 × 104.507 and `pixelRect` 105,116–202,195 (the reference's row
`setQuadrangle(…, "autoTrimImages transformAttributes rememberQuad")`, Firepixel's `transform.matrix` [1.3956560, 0, 0,
1.3956560, −35.174, −35.293] with `rememberQuad`); the undo restores 60 pt at raw 100,100 in both, and the pair falls
from 0.71 to 0.19 %. The runner row **TW-221** is fixed by R-36, the Scale-tool factor of **TW-212** is confirmed (its
numeric and script paths read as in runs 40 and 41), and `tool-text` is equal except for design again (49 / 0 / 0 / 0).
Only the script read-back format differs: Firepixel's size `83.73999786376953pt` against `83.739998pt` (the same
single-precision value) and `fontsize` 84 against 83 (Firepixel rounds, the reference truncates; a small note under
TW-219). The frame shot shows Firepixel's frame at the rounded frame − 1 px, a 2 px light-blue line with 5 × 5
selection-blue handles centred on 88/136/184 × 88/135/182, against the reference's black line and squares (RE spec 67
§1.1), recorded as the measured state of the design-level row **TW-220**. Everything else reads as in run 41: no step
status changed, the other pairs are at their run 41 values, Firepixel's other shots are pixel-identical to run 41 except
the caret, and the reference lost its first character in `10_enter_result` instead of `04_esc_result` (TW-146 rest;
8 `runner error` lines, run 41: 9). No reference restart. No measured open deviation in behaviour remains.

**What run 41 found.** F87 changes Firepixel's transform frame of a text as measured, but the run's drag does not reach
it. **TW-219:** under Modify ▸ Transform ▸ Scale Firepixel's frame of the 60 pt point text "Ag" now lies on its layout box
× 1.3 (`27_text_scale_tool_frame_open`: the line and the handles at 89/137/185 × 89/136/183, drawn 1 screen pixel up and
left, the positions RE-34 measured for the reference's handles; run 40: on the bounds 98,98–176,174), and the
reference again ends at 83.74 pt with the leftover matrix [0.993, 0, 0, 1.005], t 5.085 / 23.883, after its +39/+38 px
drag from 184,182. Runner R-35, unchanged, still takes Firepixel's handle from the object's bounds and presses at
176,174, which since F87 lies inside the frame, so Firepixel's Scale tool treats the press as a move: the text stays
60 pt with the identity and moves to raw 139,138 (row `transform.matrix` [1, 0, 0, 1, 39, 38] with `rememberQuad`). This
is a runner artefact, recorded as the runner row **TW-221** (R-36: take Firepixel's handle from the frame rule or read
the overlay's handle position through the bridge), not a regression: F87's engine test pins 83.74 pt and the measured
matrix for a drag that starts on the handle. TW-219 is therefore not yet confirmed, and `tool-text` keeps the class of
run 40 (48 / 1 / 0 / 0). **TW-212** stays confirmed on the numeric and script paths (90 pt plain text, 60 pt with
horizontal scale 1.5, pairs as in run 40). Everything else in the unit reads as in run 40: no step status changed, the
canvas pairs other than the two `27` pairs are at their run 40 values, Firepixel's other shots are pixel-identical to
run 40 except the caret, and the reference lost its first character once more (`04_esc_result`, TW-146 rest; 9 `runner
error` lines, run 40: 7). No reference restart.

**What run 40 found.** F86 is confirmed where the runner's steps reach its fold, and one of the three new paths shows a
new deviation. **TW-212:** Numeric Transform 150 % turns the 60 pt point text "Ag" into a plain 90 pt text in both
programs (horizontal scale 1, identity matrix, raw 82,82 111 × 108, `pixelRect` 83,95–188,179), each side's shot
pixel-equal to its own fresh 90 pt text, and `scaleSelection(1.5, 1, …)` gives 60 pt with horizontal scale 1.5 and no
matrix in both; every undo restores 60 pt at raw 100,100 in both. Reference against Firepixel the glyphs differ by 42
grey pixels (7 above 32 levels) at 90 pt and by about one pixel column at horizontal scale 1.5 (`pixelRect` right 188
against 187), the known text-rendering design level; all five new canvas pairs are within 1 %. **TW-219 (new):** under
Modify ▸ Transform ▸ Scale the text folds in both programs (the reference's row `setQuadrangle(…, "autoTrimImages
transformAttributes rememberQuad")`, Firepixel's `transform.matrix` with `rememberQuad`), but the same corner drag of
+39/+38 px from each side's own bottom-right handle gives 83.74 pt with a leftover matrix [0.993, 0, 0, 1.005] in the
reference and 90 pt with the identity in Firepixel: Firepixel's transform frame of a text is the object's bounds
(98,98–176,174, 78 px wide, so 117 / 78 = 1.5), the reference's is about 10 px larger (handles at 88/136/184 ×
88/135/182, 96 px wide, factor 1.3957; RE spec 66 §5). The canvas pair stays within 1 % (0.47 %), the values do not, so
`tool-text` is a real deviation (48 / 1 / 0 / 0). The frame shot `27_text_scale_tool_frame_open` shows Firepixel's frame
only, the reference's View ▸ Edges being off. **R-35's pop-up slider retry** fired once, in
`fx-bevel-emboss-fill/09_live_pressed` (no slider 2.5 s after the arrow click, the second click opened it after
0.42 s); `color-popup/23_alpha_slider_drag_canvas_pressed` and `inspector-fields/14` found their sliders within the
longer wait. All three are `ok` on both sides, and the two restored reference shots are pixel-identical to runs 37 and
38, so the quota rises to 87.9 % and two canvas pairs above 1 % come back. TW-206, TW-207 and TW-171 stay as in run 39.
No Firepixel regression: its shots are pixel-identical to run 39 except the caret in `tool-text/13`, and none of its
values changed. Reference-side changes: a restart after a script access violation before `tool-text/04`, its dock
groups (overview pairs), its History read-back in `inspector-fields/25` and one empty text read in `tool-text/07`.

**What run 39 found.** Runner R-34 closes the last open item of run 38, and F85 changes nothing the nine units
measure. **TW-208:** with the Layers row targets taken from the panel only, the Ctrl-click on L1obj after the
Shift-click reaches its row in Firepixel's 268 × 291 px panel, and both programs read `1: L1obj` after the Shift-click
and `0: ` after the Ctrl-click; run 38's 1 in Firepixel was the runner's click in the Properties inspector's name
field (**Q-66**, closed by R-34 and now confirmed), so `panel-layers` is equal except for design again and so is every
unit (49 / 0 / 0 / 0). **F85** (TW-217, TW-218, the chosen layer per page of Q-65) is not reached: no step clicks a
layer's circle (`12` drags the dot onto the locked "Gemeinsam", a no-op in both programs, `currentLayerNum` 0 in
both), T07's web layer reads unlocked in both programs throughout, and no step changes the page or the state. Its grey
circle shows once, on the empty "Ebene 1" of Firepixel's `tool-text` overview; T07's only layer without objects of its
own is the web layer, grey already, so the Layers shots of both programs are pixel-identical to run 38. **TW-205**
stays confirmed (80 in both programs after the press-drag; mid-drag 30 against 29 from the same per-side target
points), and so do **TW-214**, **TW-215**, **TW-216**, **Q-65** ("Ebene 1" current on open), **TW-206**, **TW-207**
and **TW-211**; **TW-204** keeps its documented bounds. Reference-side changes: `fx-bevel-emboss-fill/09_live_pressed`
fails ("no popup slider open", as in run 33 and as `color-popup/23` since run 38), a restart after a script access
violation at the start of `tool-text` left its dock groups at 228–231 px for that unit (overview pair 19.3 → 21.5 %),
its document inspector is back in `inspector-text/23` (as in run 37), its History read-back in `panel-layers/22` reads
0 → 2 again (as in run 37), and its lost first character in `tool-text` moved from `09`, `20` to `08`. Firepixel's
shots are pixel-identical to run 38 except the grey circle in `tool-text/00`, the caret in `tool-text/07`, `13` and
the layer context menu of `panel-layers-menu/03`, which opens 24 px higher with the same entries. TW-176, TW-177,
TW-186, TW-187, the Pen display of TW-202, the circle click of TW-217 and the text scale of TW-212 (F86 is not in the
build) are not reached.

**What run 38 found.** The fixes F79 to F83 and runner R-33 are confirmed in the nine units with one exception.
**TW-205:** the Layers opacity slider's 41 px press-drag commits 80 in both programs (run 37: Firepixel 81); mid-drag
the field reads 30 against 29 (run 37: 31), a one-per-cent residue of the runner's per-side target point, while the
committed values agree. **TW-214:** the script-made point text of `inspector-text/25` has the reference's bounds and
ink exactly (left 448, top 378, `pixelRect` 450,382–531,430), so `inspector-text` is equal except for design again;
**TW-204** keeps its documented bounds (the ink moves 33 px in both). **TW-208:** F81's Shift range is confirmed (the
Shift-click on the locked shared layer's object leaves L1obj alone in both programs), but the Ctrl-click on L1obj after
it deselects it only in the reference (0 against 1). Firepixel's UI test of the same T07 sequence reads 0 in a tall
panel, so the run shows a difference whose cause is open: the runner's 268 × 291 px panel (since F83 scrolled to the
current layer), the input path or the panel code. `panel-layers` is therefore unclear, and the new question **Q-66**
asks to reproduce it over the bridge with the runner's panel size. **TW-216** is confirmed: with the same disclosure on
both sides the Layers drag steps compare the same rows, Firepixel refuses the drop into the locked "Ebene 3" like the
reference (run 37's reorder was the runner's), and the selection dot is a target on both sides. **TW-210 / TW-215
(F79):** Firepixel opens T07 with the stored disclosure; neither the lookup nor `layers_disclosure` had to click a
triangle. **Q-65 (F83):** Firepixel opens T07 with "Ebene 1" current like the reference and `12` reads
`currentLayerNum` 0 on both sides; the Layers shots now show a circle on every layer row and the reference's tints. One
press-time observation goes to the click-test A4-63: at the press on "Ebene 1"'s row the reference has already
selected L1obj, Firepixel has made the layer current without a selection (`09_drag_reorder_pressed`). **TW-206**,
**TW-207** and **TW-211** stay confirmed. Reference-side changes: `color-popup/23_alpha_slider_drag_canvas_pressed`
fails ("no popup slider open"), its inspector pop-up of `inspector-fields/09` and its Text tool inspector of
`inspector-text/23` are back, its History read-back in `panel-layers/22` reads 0 again, and its lost first characters
in `tool-text` moved from `12`, `14` to `09`, `20`. TW-176, TW-177, TW-186, TW-187, the Pen display of TW-202 and the
circle click of TW-217 are not reached.

**What run 37 found.** Runner R-32 reaches the interactions of tester feedback batch 6 for the first time, and two of
them show real deviations. **TW-205:** the Layers panel's opacity slider counts 31 where the reference counts 30
mid-drag, and a 41 px press-drag commits 81 against 80: Firepixel ran the 212 px track of the other pop-up sliders,
the reference 2 px per per cent (RE spec 60 §2.3, RE spec 61 §4.1); F79 `7337428e` fixed the scale after the build (to
confirm in run 38). The read-back 81.18 is the 8-bit storage (81 % = 207/255), which the reference shares, not a
deviation; the live canvas mid-drag is Firepixel's deliberate preview (TW-171 `[N]`). **TW-208:** a Shift-click from
L1obj on "Ebene 1" to the object of the shared, locked layer "Gemeinsam" leaves the reference with L1obj alone (1,
then 0 after the Ctrl-click on L1obj) and Firepixel with two objects (2, then 1). The count is `fw.selection.length`,
the number of selected objects, not of rows. RE spec 61 §3 measured ranges inside one layer only; F72's rule across
layers (the objects of the rows in between, objects on locked or shared layers excluded) is our own, and in T07 the
only object between the two rows that it does not exclude is the one on the hidden "Ebene 2". The reference is the
oracle: its range either does not cross layers or skips hidden layers too; to measure (TW-208). **TW-214 (new):** the
same `addNewText` rectangle puts a point text 13 px higher in Firepixel (top 365 against 378) and 1 px further left.
**TW-204** behaves as documented: the indent moves the ink by 33 px in both programs, the reference moves the object's
left with the width kept, Firepixel widens it. **TW-206 and TW-207 are confirmed:** identical alpha, HEX and RGB
fields and #80808080 after Enter; identical pattern handles before and after an X / Y move and 0 differing pixels
inside the moved shape in both programs. The runner items of R-32 are confirmed: the reference's inspector at 125 px
at every unit start (TW-197), the Layers object rows found after expanding their parent layers (TW-210; with F79 the
panel keeps the document's own disclosure, so the expansion will not be needed), the reference's texts left-aligned
again (TW-211: equal bounds in `tool-text/07`). The host fixes change four earlier comparisons: the HEX field of
`color-popup/05` (now equal) and the text fields of `inspector-text/09`, `13`, `14` (the reference's edits now happen;
pairs 0.15–0.16 %). New runner item **TW-216**: the Layers drag steps `09` and `15` collapse three layers by script on
the reference's side only, so after the runner's expansions Firepixel's `09_drag_reorder_pressed` does not find "Ebene
3" and its `09_drag_reorder_result` drops at the top edge of a longer list and moves "Ebene 1" above "Ebene 2" (the
reference refuses the drop into the locked "Ebene 3", RE spec 42 §2); `12_selection_dot_result` has no selection-dot
target on either side. From R-32's probes: **TW-213** (Firepixel's `addNewText(r, true)` keeps the paragraph indent
the inspector last wrote, 20 after `inspector-text/13`, where the reference starts at 0; run 37's step sets the indent
0 in its script, so it does not show). F79 is recorded as **TW-215** (click-test A4-60). TW-176, TW-177, TW-186,
TW-187 and the Pen display of TW-202 are not reached.

**What run 36 found.** Tester feedback batch 6 (F68 to F72) and F67 change no class, no Firepixel-side step status
and no Firepixel-side compared value; no new deviation. None of the batch 6 interactions is reached by a runner step:
the Layers opacity field of `panel-layers/14` has no UI map entry on either side (TW-205), no step drags or types the
colour pop-up's alpha (TW-206), no step uses a pattern fill (TW-207), the indent step `inspector-text/13` uses a
fixed-width text of three paragraphs, which both programs indented before F69 too (TW-204), and the Layers object rows
that the selection steps click are `unmapped` on Firepixel's side (TW-208). The last is not new: since run 24
`panel-layers/07`, `08`, `10` and `12` fail on Firepixel's side because its Layers panel (268 × 291 px) is shorter than
the reference's flyout (245 × 389 px) and the object row L1obj of "Ebene 1" lies below its visible part; the wheel
scroll of the runner's `object_row` lookup does not bring it into view. New runner item **TW-210**: give Firepixel's
Layers panel the flyout height or scroll the row into view through the bridge. In the same shots the reference makes
"Ebene 1" current after opening T07 and Firepixel "Ebene 3", in every run with a full Layers shot; no step reads the
current layer and the pairs hold it only inside their design-level percentages: new question **Q-65** (what the file
stores, which layer the reference makes current on open). The three reference-side errors new in run 33
(`fx-bevel-emboss-fill/09`, `tool-text/11`, `13`) are `ok` again, which raises the quota to 87.3 % (1077 / 1234) and the
canvas count to 186 / 212; the two `tool-text` pairs do not compare the edits, because the reference's drag selection of
"123" failed again (TW-146 rest; 7 `runner error` lines). The reference's Text tool was right-aligned, as in runs 27, 28
and 30 (stored state; the unit-start text tool reset sets face, size, style and colour only): its new texts in
`tool-text` sit right of the click point and the bounds read in `07` differ from Firepixel's, a runner matter recorded
as **TW-211**. The reference's History count in `panel-layers/22` now equals Firepixel's (2 after two clicks; run 33:
0). Firepixel's Edge list holds the Dither entry of F67 (`inspector-fields/03`, `10`) and its toolbox wells in
`color-popup-contexts/04` now match the reference's (F61, against run 33). TW-197 did not recur; TW-176, TW-177,
TW-186, TW-187 and the Pen display of TW-202 are not reached.

**What run 35 found.** Tester feedback batch 5 (F62 to F66) changes no step status, no compared value and no class
and moves no canvas pair across 1 %; no new deviation. **TW-202 (F65)** is visible only where the Pointer holds a
selected rectangle: in `xs-select-behind/01, 07` and `color-popup/00, 01, 18, 20` Firepixel now draws it as outline +
four 5 × 5 px corner squares centred on its corners (run 34: the bounding box, its right and bottom handles 6 px; 15 px
per shot), and the pairs do not move because the reference's canvas shots carry no selection display. The
`tool-path-reshape` canvas shots hold P1 selected under the Freeform, Reshape Area, Path Scrubber and Knife tools,
which F65 does not cover: Firepixel's 20 shots are pixel-identical to run 34, box included, and no pair was above 1 %
because of the box (the only one, `11_rect_to_path_result` 10.93 %, is unchanged). No step draws with the Pen or
hovers it, so the Pen display and the pen cursor variants are not reached (click-test A4-49). **TW-200 and TW-201
(F64, F66):** the committed values of `inspector-text` and `inspector-fields` read as in run 33; the slider pop-ups
sit flush with their arrow (`inspector-fields/09` 36 × 228 px, run 33: 36 × 248), which lowers
`inspector-fields/14_popup_drag_live_canvas_pressed` from 2.03 to 1.58 % (the pop-up no longer reaches into the canvas
shot; the deliberate live preview keeps the pair above 1 %, TW-171); the width field's tooltip "Auswahlbreite" now
appears in Firepixel's `inspector-fields/24_hover_field_hover` as in the reference's (run 33: none), and the slider
arrow has none on either side; View ▸ Tooltips is off at every Firepixel start ("off already" six times; run 34:
"switched on -> off" eight times). **TW-198 and TW-199 (F62, F63):** no step touches patterns, and the eyedropper shots
of `color-popup/06, 07` are pixel-identical to run 33 (no focus loss in the unit). **TW-197 did not recur:** the
reference's Properties inspector was expanded in every unit, so the inspector pairs of `tool-path-reshape` are back at
9.7–10.6 % (run 34: 60.2–60.7 %), `toolbox/00` at 25.9 % (run 34: 49.8 %), Firepixel's `toolbox` fit zoom and all 25
of its shots at run 33, and the reference's page at its run 33 place (page top 183); the runner item stays open until
R-32 sets the inspector. Firepixel's `ui_state` tool fill with the multi-selection R1 + R2 in `inspector-fields/21`
reads #000000 (run 33: #808080), a consequence of F61's rule that multi-selections are not followed (TW-192 refined);
the reference logs no tool colour there. The reference's text exports in `inspector-text` changed their hash at the
same byte size with identical canvas shots, and its `inspector-text/23` shows the Text tool's inspector again (run 33:
the document inspector). No `runner error` line, no unplanned reference restart; TW-176, TW-177, TW-186 and TW-187 are
not reached by any step.

**What run 34 found.** F61 and runners R-30 and R-31 are confirmed in the eight units; no new deviation, one new
runner item. **TW-195 fixed and confirmed:** the Arrow Line is drawn black on both sides (exports 141 black and no red
pixels each) and turns the tool stroke back to black in both programs after the Measure had turned it red; the tool
colours agree in all 37 logged steps of `tool-autoshapes` (run 33: 29 differed), every later shape is black on both
sides, the canvas pairs fall to 0.01–0.97 % (`05_handle_drag_rounded_corner_result` 1.01 → 0.97 %) and 13 of the 17
exports are equal above 32 levels (run 33: 1); the unit is equal except for design again. **TW-192 refined rules
confirmed:** after selecting the path P1 with fill none Firepixel's tool fill reads the path's stored #808080 like the
reference's (`tool-path-reshape/02` to `07`; run 33: white), and Insert ▸ Hotspot / Slice from selection leaves the tool
colours white / black in both programs (`tool-web/12` to `15`; run 33: Firepixel #ff9900 / #c04040, the hotspot
followed); the auto shapes are not followed either (the tool stays as it was after each shape). Over the eight units the
tool colours now agree in every step that logs them (run 33: 42 steps differed, 37 by Firepixel's follow and 5 in
`xs-canvas` by the reference's stored colours). **R-31 confirmed:** all eight unit starts log the same tool stroke on
both sides (`bn_1-Pixel Anti-Aliased` 1 px centred, stroke #000000, fill solid #ffffff); Firepixel's fresh start already
had it (F61), the reference needed the set three times (`bn_Hard Line` after `tool-autoshapes`, 2 px from its stored
profile after the restart, `bn_Soft Rounded` after `toolbox`). The script rectangle FR of `xs-select-behind` reads
400,200–600,300 on both sides (run 33: the reference 399,199–601,301) and the reference's first six auto shapes have
the 1 px anti-aliased stroke like Firepixel's. **TW-196 confirmed:** the one reference restart (a script access
violation while `toolbox` reopened its test file before `05_letter_cycle_o_result`) was followed by the log line
"unit-start reset repeated", the reopened file and the unit-start state (`MaxUndos` 20, dock 105 → 271 px, default
colours #808080 / #c04040 → white / black, text tool, tool stroke 2 → 1 px); without restarts `xs-canvas` and
`xs-windows-tabs` are back at their run 30/31 window values (`xs-canvas/04, 05, 17, 21, 22` 26.0–40.0 %,
`xs-windows-tabs/03` 26.4 %) and the reference's tool colours in `xs-canvas/08` to `13` read white / black like
Firepixel's. `toolbox/00` stays raised (49.8 %, run 32: 25.2 %), for another reason. **New: TW-197** (runner): the
reference started the run with its Properties inspector collapsed to the tab row (inspector shots 1256 × 56 px instead of
1256 × 125, the document view 69 px taller) and kept it until the restart in `toolbox`; the runner sets the right dock
at unit start but not the inspector. It raised the inspector pairs (`tool-autoshapes/07` 29.9 → 65.5 %,
`tool-path-reshape/01, 12` to 60.2–60.7 %, `tool-web/13` to 63.1–63.6 %) and the overview pairs, gave the reference a
larger fit zoom in `toolbox` (47.1 % against Firepixel's 44.9 %) and moved the reference's page 34 px lower in
`menu-modify-a` (`05_rotate_canvas` 3.53 → 1.69 %); no compared value and no canvas pair crossed 1 % by it.
`menu-modify-a/20` (0.17 %) and `/22` (B1 stays selected after the real Escape) read as in run 33, and the `xs-canvas`
zoom shots are pixel-identical to run 33 (BA-098: `02` 1.50 %, `18` 0.00 %). Firepixel's stroke well now shows None
where the tool stroke is none, as the reference's does. No `runner error` line; TW-176, TW-177, TW-186 and TW-187 are
not reached by any step.

**What run 33 found.** The first full run with real plain keys in every unit and the R-29 dock layout confirms the
fixes the runner reaches and finds one new real deviation, one runner item and one question. **TW-193 confirmed:** the
object context menu of the Layers panel (`panel-layers-menu/04`) reads the same 23 entries and states in both
programs, Flatten Selection and Add Mask enabled. **TW-194 confirmed:** after a marquee on B1 the real Escape keeps B1
selected in both programs (`menu-modify-a/22`: `[object Image]` 40,40–520,360). **TW-192 confirmed for the colours:**
Firepixel's tool attributes follow the selected object as the reference's do, and script and converted shapes take
them (`menu-modify-a/20` 6.32 → 0.17 %, `tool-path-reshape/11`, `tool-autoshapes/11`; the Measure and Arrow shapes now
red in both programs), but the script rectangle FR of `xs-select-behind` has a 2 px centred stroke in the reference and
a 1 px inner one in Firepixel (Q-63). **TW-174 confirmed:** with the dock in icon mode `panel-layers/04` hides both
layers and `panel-swatches-mixer/19` reads #efffbf on both sides, and the panel pairs are back at the run 24/25 values.
**New: TW-195** (real deviation, `tool-autoshapes`): the reference draws the Arrow Line with a black stroke while the
tool stroke is red (taken over from the Measure shape) and its tool stroke follows back to black; Firepixel draws it red
and keeps the red for every later shape of the unit. Before F58 Firepixel drew every shape black, which hid this.
**New: TW-196** (runner): after a reference restart in the middle of a unit the reference keeps its stored state (dock
collapsed, tool colours #808080 / #c04040, `MaxUndos` 3) until the next unit start; it raised window pairs in `toolbox`,
`xs-canvas` and `xs-windows-tabs`. **New: Q-63:** tool stroke attributes carried between steps and units (FR in
`xs-select-behind`, the reference's first six auto shapes, the tool fill after a path with fill none in
`tool-path-reshape`, the tool colours after Insert ▸ Hotspot from selection in `tool-web`); the runner resets only the
colours at unit start. The real keys in the 17 units not repeated since run 30 changed no Firepixel comparison to the
worse: menus are closed at the read-backs, so 17 menu read-backs that differed on the reference side are equal now, F4
hides the reference's panels (`menu-view/15` 56.2 → 7.8 %), the export-area preview opens on the reference side
(`export-results/13` measured for the first time) and Paste is grey in both context menus of `xs-context-menus`. Not
reached by any step: TW-176 (no Pointer drag below the threshold or back to its start), TW-177 (no Enter or Escape while
the pen draws), TW-186 (no colour swap with a selection) and TW-187 (no Subselection anchor drag). The tiled canvas of
BA-098 stays without a sampling offset (Firepixel's `xs-canvas` zoom shots pixel-identical to run 30). TW-146 rest: 4
`runner error` lines in `tool-text` (run 31: 10).

**What run 32 found.** F55 and runner R-28 settle the real deviation and the open question of run 31; no new
deviation. **TW-189 fixed and confirmed:** with the group G of T06 selected, Enter on the canvas keeps the group
selected in both programs (`xs-shortcuts/22`: `[object Group]` on both sides; run 31: Firepixel `null`). **TW-191
closed:** in `xs-shortcuts/02` the reference's key check no longer re-selects R1 before the final Delete; with nothing
selected the Delete removes nothing on either side (9 elements before and after in both; run 31: the reference deleted
the three R1, 9 → 6). **Q-60 closed as a runner matter:** Firepixel's click on a Preferences category now lands on the
list row instead of the bold page heading with the same text, and Down moves both category lists from "Allgemein" to
"Bearbeitung" (`xs-preferences/20`); the product behaviour is pinned by the F55 UI test `q60_pref_category_click` (a
pointer click on a row gives the list the keyboard focus, Down moves to the next category and page). **TW-190
closed:** Firepixel's View ▸ Tooltips is switched off at each unit start (logged "on -> off" three times), so its
canvas shots carry no coordinate tag and are pixel-identical to run 30 (`xs-preferences/06, 07, 08`: 3.10, 0.14 and
0.09 %; `toolbox/15` cursor shots 2.49 %; run 31: 3.41, 0.19, 0.15 and 2.53 %), and its flyout shots show no tooltip:
the ten `toolbox/02_flyouts_*` pairs fell by 1.3 to 25.9 points against run 31 and lie 0 to 7.4 points below run 30,
whose shots still carried the button's short name tooltip over the flyout. All other values of the three units read
as in run 31. Reference-side only: its text entry lost the first character in `xs-shortcuts/15` (shot only, two
`runner error` lines, TW-146 rest), and its File menu showed Save enabled in `xs-shortcuts/20` (grey in runs 30 and 31;
the state flips between runs, as in runs 13, 14, 17 and 19 to 21; Firepixel grey). With the tag off the runner no
longer measures it; the reference's default and hover behaviour stay question Q-61.

**What run 31 found.** The F53 fixes and runner R-27 are confirmed in the eight units, with one new real deviation
and one open question. **TW-173 fixed:** after a Clone with nothing undone, Ctrl+Y clones again in both programs
(three R1 in `menu-edit/02`) and the Edit menu reads the enabled "Wiederholen: Klonen" in both, while Ctrl+Y right
after a redo still adds nothing and the menu reads "Can't Redo" (`xs-shortcuts/02, 17`). With real keys on the
reference side (no R-19 substitutes for the first time) every compared value of the eight units reads as in run 30
except two: **TW-189 confirmed** (`xs-shortcuts/22`: Enter keeps a selected group in the reference, Firepixel drops the
selection; Firepixel's rule in the Pointer tool was derived from run 10, when the reference's Enter still arrived as
the letter R) and **Q-60** (`xs-preferences/20`: after a click on "Allgemein" the real Down moves the reference's
category list, Firepixel's list does not move and shows no focus frame). **TW-188 closed as a runner matter:**
Firepixel's "popup visible" in `xs-shortcuts/21` is the change box of the fill swatch at the click point, not an open
pop-up. The real nudges merge into one Move row in the reference's History as in Firepixel's
(`panel-pages-states-history`), the crop in `xs-preferences/06` commits with the real Enter, and the Escape after a
started pen path keeps the pen tool in the reference (`xs-preferences/07`). TW-176 and TW-177 are not exercised: no
step drags the Pointer below the threshold or back to its start, and no step draws a pen path and presses Enter or
Escape; they stay with the click-test. **TW-190** (runner): Firepixel's canvas shots now carry the View ▸ Tooltips
coordinate tag of TW-184 (on by default), the reference's do not (its switch is off on the reference machine), which
moves 35 of the 44 canvas pairs by at most 0.31 points without crossing 1 %; the flyout shots of `toolbox` also carry the
new description tooltip. **TW-191** (runner): the key check re-selects the step's last selection before a Delete with
nothing selected on the reference side only. **Q-61**: the reference's default for View ▸ Tooltips and whether its tag
also shows while the pointer only hovers. The tiled canvas stays pixel-identical outside the tag in all zoom shots of
`xs-canvas`. The reference's text entry still loses the first character in some `tool-text` steps on the scan-code
path (`02`, `09`; a second try in `12`), in other steps than in run 30 (`03`, `04`, `05`, `20`), so the loss is not
tied to the key path (TW-146 rest, click-test A1-19).

**What run 30 found.** The first full run since run 24, on preview `2026-10-20_19-06` with fixes F51a to F51c, F52
and the performance work, confirms 48 units with unchanged values and no new Firepixel-side error: all 21
Firepixel-side errors are the same steps with the same texts as in the baseline runs. New: **TW-173** (real
deviation): in `menu-edit/02` the reference repeats a clone with Ctrl+Y when nothing was undone (three R1 in every run
since run 1), Firepixel no longer does since F51a made Redo redo only (TW-170); the redo cases of `xs-shortcuts/02, 17`
(Ctrl+Y right after a redo adds nothing, the Edit menu reads "Can't Redo") became equal with the same fix. The
deliberate live opacity preview of TW-171 moves `inspector-fields/14_popup_drag_live_canvas` above 1 % (2.03 %).
**TW-174** (runner): the reference's docked panels have other sizes than in runs 24 and 25, so `panel-layers/04`
(eye drag) and `panel-swatches-mixer/19` (swatch 5) hit other rows on its side. **Q-58**: the reference draws the
script rectangle FR of `xs-select-behind` with the current stroke (2 px this time), Firepixel without one. **Q-59**
(not new, seen while reading the values): with an object row right-clicked, the reference's Layers context menu
enables Flatten Selection and Add Mask, Firepixel's greys them. The reference's text entry lost the first character
in four `tool-text` steps (`03`, `04`, `05`, `20`; four new `runner error` lines). **Tiled canvas (BA-098): no
offset.** Firepixel's canvas shots at the fit zoom 42.83 % (`xs-canvas/03, 11, 13, 14, 16`), at 50 %
(`menu-modify-a/02`) and at 35 % (`xs-preferences/06`) and the `toolbox` full screen and cursor shots are
pixel-identical to the baseline runs, which were rendered without tiles; `xs-canvas/02_zoom_center` changed only in the
anti-aliasing of its labels (grid lines on the same device pixels), and its share fell from 1.57 % to 1.50 % at the
same offset (−2, −3) that run 25 found. No canvas pair of a zoom step rose and the offset search found no offset that
it had not found before; the `preview-views` and window pairs that moved changed on the reference side only.

**What run 29 found.** Runner R-26 closes **TW-167**: the layout check now measures the reference's right panel
dock window and expands it with the dock's own button when it is collapsed to icons; at the start of `toolbox` it
found 105 px and restored 271 px, and after full screen with menus (which leaves the dock collapsed in the reference)
it restored it again before the Standard shot. The overview and Standard window pairs are back to 25.5 % and 25.9 %.
`tool-path-reshape` and `xs-select-behind` are unchanged on preview `2026-10-20_06-08`.

**What run 28 found.** Runner R-25 is confirmed in its screen-mode item. **TW-166**: the reference leaves full screen
by F (two F from full screen with menus, one from full screen), every Standard reset reads Standard from the window,
and `toolbox/13` has no `runner error`. Not confirmed: **TW-146 rest**: the new area test (at least 60 % of the
expected inverted area) rejects the 24 changed pixels as the caret, but none of the six selection tries (glyph-box
drags, drags from the end, the word double-click) gives more (ratio 0.12–0.14), so the size routes are skipped and
"123" stays 12 pt; whether the reference selects without a visible highlight in the capture or does not select at all
is still open. **TW-165**: the two-sided reset is armed only by a failed field or menu route, so with all routes
skipped neither side is reset and `tool-text/14, 16` stay at 1.10 % and 3.82 %. New: **TW-167** (runner): the
reference's panel dock started collapsed to icons (left over from run 27), R-25 takes that 110 px width as the
reference, and after the screen modes the dock is expanded (276 px) and not restored; the overview and Standard
window pairs of `toolbox` swap (64.3 % and 25.6 %).

**What run 27 found.** Runner R-24 is confirmed in its Freeform and screen-mode items. **TW-159 rest**: the Freeform size keys
are compared now; with the mouse held both programs keep the Freeform tool and read size 72 afterwards, and the shot
pair `tool-path-reshape/04` is 0.46 %. **TW-164**: Freeform through flyout entry 0 with a tool read-back puts the
Freeform head on the reference side (`12_inspector_heads_freeform` pixel-identical to `01`). **TW-163**: the
reference's Standard mode is read from the window, so the Standard resets and `toolbox/13_view_modes_standard_normal`
read Standard on both sides; one `runner error` is left from the reset after full screen, where the screen-mode button
and the menu bar are hidden (TW-166). Not confirmed: the size on a text selection (**TW-146 rest**): in
`tool-text/11` the field, the keys and the dialog route all leave "Abc 123" at 12 pt in the reference (the dialog route
has no key in the "Web Standard" set), and the 24 changed pixels read as the highlight are far fewer than an inverted
three-glyph block would give, so the selection itself is in doubt; `13` ran on 12 pt text and its grey did not reach
"123". New: **TW-165** (runner): R-24 resets only the reference's text tool after the failed size step, so its later
texts are 12 pt while Firepixel's are 72 pt, and `tool-text/14, 16` move above 1 %.

**What run 26 found.** Package F49 and runner R-23 are confirmed in the five units. **TW-158**: after the second F
both programs show only the canvas on black without bars, tabs and panels (`toolbox/13_view_modes_full_normal` 1.97 %,
run 25: 77.1 %), so `toolbox` is equal except for design and all 49 units are. **TW-159**: (b) the Subselection nudge
moves the first node to 201,210 in both programs (`xs-shortcuts/10_nudge_point` `ok` on both sides); (d) the reference's
text reset in `menu-text` goes from 72 pt to 12 pt; (a) Firepixel's screen-mode read-back is exact and the reference's
recognises both full screen modes from the window. **TW-146 rest**, entry part: all 15 new texts and three edits in
`tool-text` arrive as typed, none is discarded, and the double-click in `07` finds the editor. Left for the runner:
the reference's screen-mode button reads "full screen with menus" in Standard (TW-163, six `runner error` lines), the
Freeform size keys keep the tool but record no size (TW-159 rest), the reference shows the Path Scrubber head in
`tool-path-reshape/12_inspector_heads_freeform` (TW-164), and the selections in `tool-text/11, 13` (TW-146 rest).

**What run 25 found.** Package F48 and runner R-22 are confirmed. **TW-148**: both programs keep one History row
per selection for consecutive key nudges (the reference `moveSelectionBy({x:20, y:10})` and `({x:10, y:0})`,
Firepixel Move, Deselect, Move). **TW-157**: the Select Behind hover line on FR is 680 px of #E00000 in both programs and
neither shows a frame right after a click. **TW-153**: the reference draws with fill #ffffff again (`tool-autoshapes`
26 / 27). **TW-154**: Text Editor and Edit Guides are `unsupported` on the reference side, Firepixel opens them.
**TW-155**: `inspector-stroke/01` without a key, `menu-edit` History readable with one document. **TW-156**:
`toolbox/13` starts and ends in Standard mode on both sides. **TW-149** rest: Firepixel's text reset succeeds in all
20 units. **TW-145** is decided: after the size edit in `tool-text/11` the reference's next texts are 72 pt like
Firepixel's (the F43 rule gives the same next-text size). **TW-146** rest is reduced to one lost text in 17 (`07`).
New: TW-158 (Firepixel's full screen mode keeps menu bar, tab and status bar; the reference shows only the document
on black) and TW-159 (runner: the reference's screen-mode read-back, a real arrow with the Subselection tool in
`xs-shortcuts/10_nudge_point`, the Freeform size keys in `tool-path-reshape/04` and a 24 pt text size left by the
reset in `menu-text`).
*R-27 correction:* the real arrow that selected another tool in `xs-shortcuts/10_nudge_point` and the Freeform tool
switches in `tool-path-reshape/04` (TW-159 b, c) were not the reference's handling of the keys: our reference host
sent every unmodified named key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape ->
E or C, Enter -> R; TW-179, fixed in runner R-27).

**What run 24 found.** The full run confirms the state of the follow-up runs in 46 of 48 old units with unchanged
quota and values. Confirmed and closed: the History row labels decide **TW-148** against F46 (the reference lists one
row `moveSelectionBy({x:20, y:10})` for three Shift+arrow presses on R1 and a new row only for the nudge of E1 after
the selection change; Firepixel one row per press), so Firepixel has to merge consecutive key nudges again; the hover
rule of Select Behind is equal (**TW-152**: the same 1680 px #E00000 frame on B1 in `02` and `04`, no frame in `03` and
`07`, the click cycle B1 → FR); the "Q" after the pointer double-click arrives on the reference side and both programs
read "ZeilQe eins" (`tool-text/06`); Firepixel's text tool starts at Arial 12 pt like the reference (**TW-149**, 46 of
49 units; the reset failed with a bridge error in `panel-layers`, `panel-layers-menu` and
`panel-pages-states-history`, units without new text); `xs-shortcuts/08` is within the new limit (**TW-150**, 92.8 s)
and the run-start window list has no code-page error. Pen-related units show no change after F47 (`tool-path-reshape`,
`inspector-stroke`). Still open: the reference loses the keys of 4 of 16 new texts in `tool-text` although the caret
check passes (TW-146 rest), so the size rule for new text is still not compared (TW-145). New: TW-153 (the reference's
unit-start text reset sets its fill colour to #000000; `tool-autoshapes` canvas 7 / 27; the reference's size reset
also left 23 pt in eight units), TW-154 (Text Editor and Edit Guides have no key in the "Web Standard" set), TW-155
(plain arrow keys where they are no nudge, and the reference's History read-back in `menu-edit` with two documents
open), TW-156 (screen-mode steps in `toolbox/13` without a defined start mode) and TW-157 (Select Behind: hover line on
vector paths anti-aliased over two pixel rows instead of a crisp #E00000 line; frame shown right after a click).
*R-27 correction:* the plain arrows of TW-155 a switched the reference's tools because our reference host sent every
unmodified named key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or C,
Enter -> R; TW-179, fixed in runner R-27); since R-27 the arrows are real keys and the R-22 `tool_key` check is off.

**What run 23 found.** Runner R-20 makes text entry on the reference side work in most steps: 14 of 17 new texts in
`tool-text` are created as typed, the commit check confirms them, and drag selections are verified by highlight
pixels without the stray U+0003 of run 22. Confirmed: Escape keeps the selection in both programs
(`xs-shortcuts/21`, F46 after RE spec 51), the Pointer tool is reached through the flyout when the slot holds Select
Behind (`xs-shortcuts/07`, no runner error), the paste read-back in `menu-edit/03` is R1 at 50,50–150,120 on both
sides, and the history limit reads `MaxUndos` 20 in both programs (TW-147 closed). Not yet compared: the size of a
new text after a partial-selection edit (TW-145), because the reference's size field edit in `tool-text/11` does not
reach the selected "123" (its next texts stay 12 pt, Firepixel's take 72 pt). New: with real Shift+arrows the
reference reads 4 History steps after three nudges and a nudge of a second object, Firepixel 5 (TW-148: the count
suggests that the reference merges the three presses, against RE spec 51 and F46; the row list is needed); the
reference still loses the keys of 3 of 17 new texts and the "Q" after a double-click (TW-146 rest); Firepixel's text
tool starts with Myriad Pro while the reference is reset to ArialMT (TW-149, runner); `xs-shortcuts/08` exceeds the
90 s step limit (TW-150, runner).
*R-27 correction:* the reference side of the Escape in `xs-shortcuts/21` was the runner's `kept` substitute, not a
key, because our reference host sent every unmodified named key as the last letter of its name (Delete -> E, Right ->
T, Down -> N, Up -> P, Escape -> E or C, Enter -> R; TW-179, fixed in runner R-27); the real Escape was first measured
by R-27 and keeps the selection there too.

**What run 22 found.** Runner R-19's substitutes work on the reference side: Edit ▸ Clear deletes R1
(`menu-edit/09, 22`), `moveSelectionBy` nudges by 1 px with values equal to Firepixel's arrow keys
(`xs-shortcuts/10`, `panel-pages-states-history/14–21`), the Pointer tool click closes the text editor without a
stray "c", Escape no longer cycles the reference's tools in `xs-preferences`, and the double-click commits the crop
(TW-140 closed). Confirmed: the crop option (TW-134; page 280 × 220 and P1, E1, R2, R1 in both programs) and the live
bounds of an edited text through the element list (TW-143; 50,150–120,194 on both sides while "Vr" is typed). The
unit-start text reset finds and leaves the reference's text tool at ArialMT 12 pt regular #000000 in all seven
units (run 21: the colour stayed #808080). New runner artefacts: text entry on the reference side fails in
`tool-text` (texts created by a click stay empty, the drag selection reads back nothing and its Ctrl+C inserts
U+0003; TW-146), so the size rule for new text (TW-145) is still not compared; the Pointer slot holding Select Behind,
the do-nothing Escape that leaves a selection, and script rows in the History that hide the nudge merge (TW-147).
*R-27 correction:* the substitutes only replaced keys that failed because our reference host sent every unmodified
named key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or C, Enter -> R;
TW-179, fixed in runner R-27); TW-140 was a runner bug, and the values of the substituted steps show what the
substitute did, not what the reference does with the key. The real keys were measured from run 31 on.

**What run 21 found.** Confirmed: Ctrl+Alt+Shift+S leaves the text "left" in both programs (TW-133), the URL panel
menu reads the same labels, "..." flags and enabled states on both sides (TW-142), and with CR/LF kept in the
read-backs both programs separate paragraphs with `\r` (TW-144; the reference's `\n` was a read-back artefact).
Not confirmed: the live bounds of an edited text (TW-143; Firepixel's element-list `pixelRect` still reads the
committed bounds until Escape) and the crop option (TW-134; the reference's page stays 800 × 600). The runner's new
key lines pin down TW-140: the keys arrive with the sent virtual-key and scan codes and German key names, the layout
is German everywhere, and the document view has keyboard focus before every key, yet the reference takes unmodified
Delete, Right, Down and Escape as the tool letters E (Eraser), T (Text), N (Line) and C (Crop ↔ Export Area; a "c"
in an open text editor); with Shift no tool changes and Ctrl+D works. R-17's Properties-panel reset sets the
reference's text tool to 12 pt regular (not the colour), and the "Q" typed after a double-click arrives. New: in
`tool-text` Firepixel's new texts take the size set on a partial selection (the F43 rule from the RE spec), which the
reference side cannot show while its selections fail (TW-145).
*R-27 correction:* the reference did not take the keys as tool letters; our reference host sent every unmodified named
key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or C, Enter -> R; TW-179,
fixed in runner R-27). The key-echo lines went through another path of the host and therefore looked right. With the
fix the real keys work in the reference.

**What run 20 found.** The fixes after run 19 are confirmed: both Histories list Clone, Copy, Paste (TW-137), a new
text keeps the tool's 12 pt black after attributes were set on a partial selection (TW-138; `tool-text/14` now
0.54 %), Firepixel's read-back shows the typed text while the editor is open (TW-139) and Modify ▸ Snap to Pixel
reads `enabled: false` in both programs (TW-141). The crop option (TW-134) is not confirmed: Firepixel crops and
removes the three paths outside, the reference does not apply the crop at all. Runner R-15 did not fix the
reference-side keys (TW-140 stays open): `key_echo` failed with a script error (fixed in R-16), the log has no
layout or tool-switch lines, Delete still selects the Eraser, the plain Right arrow the Text tool, a "c" still
arrives after Escape, a typed "Q" was lost in `tool-text/06`, and the text-attribute and default-colour resets at
unit start have no effect (36 pt bold italic underlined text in every unit). New: Firepixel's panel pop-up menus
report no enabled state and the reference's menu texts lack the "..." (TW-142, runner / bridge), the live read-back
keeps the old bounds of an edited text until commit (TW-143), and the paragraph separator in text read-backs
differs (`\r` vs. `\n`, TW-144).
*R-27 correction:* the Eraser, the Text tool and the "c" after Escape, and probably the crop that was not applied (its
commit was a plain Enter), come from the host bug fixed in runner R-27: our reference host sent every unmodified named
key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or C, Enter -> R;
TW-179).

**What run 19 found.** Runner R-14 makes the reference's dialogs, history and text read-backs visible: the
warnings on Ctrl+Shift+B and Ctrl+Shift+I appear on both sides, the history read-back works, and the typed text of
`xs-shortcuts/15_in_text_tx2` lands in line 2 at the click point in both programs. Confirmed: the URL panel menu
(TW-131), the text size steps and the Select Inverse warning (TW-133 in part), the caret after a pointer
double-click on Firepixel's side (TW-135), the States menu width (TW-125) and the text read-back (TW-130 / Q-57).
Still different: Ctrl+Alt+Shift+S sets "stretch" on TX1 (TW-133 rest). New: the URL menu labels end in "..."
(TW-136), Copy is missing from Firepixel's History (TW-137), Firepixel's text tool keeps size and colour set on a
partial selection for the next text (TW-138), and its scripting read-back sees typed text only after commit
(TW-139). On the reference side, Delete selects the Eraser tool and the plain Right arrow the Text tool, and a "c"
arrives after Escape (TW-140, runner); Firepixel's menu read-back still has no enabled state (TW-141, runner /
bridge). `xs-preferences` (TW-134) was not part of run 19.
*R-27 correction:* these reference-side key effects were our host's, not the reference's: our reference host sent
every unmodified named key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or
C, Enter -> R; TW-179, fixed in runner R-27).

**What run 18 found.** With both programs on the "Web Standard" set, Ctrl+Shift+D clones on both sides and the
Clone-dependent steps are equal (TW-126 closed); per-slice export formats (TW-128), short panel menus (TW-125),
defined option start states (TW-127) and the reference-side retries (TW-129) are confirmed. Still different under
the same set: four text shortcuts in `xs-shortcuts/06` (TW-133). A second look at the menu read-backs found the URL
panel menu with only "Help" in Firepixel (TW-131, present since at least run 17), and the shots of `tool-text/06`
show that a pointer double-click on a text does not insert the typed character (TW-135). `xs-preferences/06` keeps
the crop difference with the option measured as on in both programs (TW-134). The remaining differences are
reference-side runner effects (TW-132: history read-back script error, Delete, Ctrl+D and plain arrow keys without
effect, text-tool state carried between units).
*R-27 correction:* Delete and the plain arrow keys had no effect because our reference host sent every unmodified
named key as the last letter of its name (Delete -> E, Right -> T, Down -> N, Up -> P, Escape -> E or C, Enter -> R;
TW-179, fixed in runner R-27); this bug was already in the host's first version.

## History of the headline figure

| Run | Date | Equal except for design | Proven 1:1 |
|---|---|---|---|
| 10 (full) | 16/17 Oct 2026 | 29 of 48 (9 under-measured) | 74 % (29 / 39) |
| 11 (23 units) | 17 Oct 2026 | 31 of 48 (9 under-measured) | 79 % (31 / 39) |
| 12 (20 units) | 17 Oct 2026 | 39 of 48 (0 under-measured) | 81 % (39 / 48) |
| 13 (14 units) | 17 Oct 2026 | 42 of 48 (2 under-measured) | 88 % (42 / 48; 91 % = 42 / 46 of the sufficiently measured units) |
| 14 (8 units) | 17 Oct 2026 | 42 of 48 (1 under-measured) | 87.5 % (42 / 48; 89.4 % = 42 / 47 of the sufficiently measured units); step level 84.1 % |
| 15 (7 units) | 17/18 Oct 2026 | 44 of 48 (0 under-measured) | 91.7 % (44 / 48); step level 84.8 % |
| 16 (7 units) | 18 Oct 2026 | 46 of 48 (0 under-measured) | 95.8 % (46 / 48); step level 85.0 % |
| 17 (full) | 18 Oct 2026 | 42 of 48 (0 under-measured) | 87.5 % (42 / 48); step level 86.3 % (run 17 alone) |
| 18 (10 units) | 18 Oct 2026 | 43 of 48 (0 under-measured) | 89.6 % (43 / 48); step level 86.9 % |
| 19 (6 units) | 18 Oct 2026 | 44 of 48 (0 under-measured) | 91.7 % (44 / 48); step level 86.9 % |
| 20 (7 units) | 18 Oct 2026 | 45 of 48 (0 under-measured) | 93.8 % (45 / 48); step level 86.9 % |
| 21 (7 units) | 18 Oct 2026 | 45 of 48 (0 under-measured) | 93.8 % (45 / 48); step level 86.9 % |
| 22 (7 units) | 18/19 Oct 2026 | 47 of 48 (0 under-measured) | 97.9 % (47 / 48); step level 86.9 % |
| 23 (5 units) | 19 Oct 2026 | 46 of 48 (0 under-measured) | 95.8 % (46 / 48); step level 86.9 % |
| 24 (full, 49 units) | 19 Oct 2026 | 46 of 49 (0 under-measured; 2 real deviations, 1 unclear) | **93.9 %** (46 / 49); 95.8 % over the 48 old units; step level 87.0 % (86.9 % over the 48 old units) |
| 25 (20 units) | 19 Oct 2026 | 48 of 49 (0 under-measured; 1 real deviation, 0 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.3 % (87.2 % over the 48 old units) |
| 26 (5 units) | 19 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.3 % (87.3 % over the 48 old units; one `toolbox` step recounted) |
| 27 (3 units) | 19 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.3 % (87.3 % over the 48 old units); canvas pairs 185 / 212 |
| 28 (2 units) | 19 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.3 % (87.3 % over the 48 old units); canvas pairs 185 / 212 |
| 29 (3 units) | 20 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.3 % (87.3 % over the 48 old units); canvas pairs 185 / 212 |
| 30 (full, 49 units) | 20/21 Oct 2026 | 48 of 49 (0 under-measured; 1 real deviation, 0 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.2 % (87.1 % over the 48 old units); canvas pairs 185 / 212 |
| 31 (8 units) | 21 Oct 2026 | 47 of 49 (0 under-measured; 1 real deviation, 1 unclear) | **95.9 %** (47 / 49); 95.8 % over the 48 old units; step level 87.2 % (87.1 % over the 48 old units); canvas pairs 185 / 212 |
| 32 (3 units) | 21 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.2 % (87.1 % over the 48 old units); canvas pairs 185 / 212 |
| 33 (full, 49 units) | 21/22 Oct 2026 | 48 of 49 (0 under-measured; 1 real deviation, 0 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.1 % (87.0 % over the 48 old units); canvas pairs 184 / 209 |
| 34 (8 units) | 22 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.0 % (86.9 % over the 48 old units); canvas pairs 185 / 209 |
| 35 (6 units) | 22 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.0 % (86.9 % over the 48 old units); canvas pairs 185 / 209 |
| 36 (9 units) | 23 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.3 % (87.2 % over the 48 old units); canvas pairs 186 / 212 |
| 37 (9 units) | 23 Oct 2026 | 47 of 49 (0 under-measured; 2 real deviations, 0 unclear) | **95.9 %** (47 / 49); 95.8 % over the 48 old units; step level 87.8 % (87.7 % over the 48 old units); canvas pairs 191 / 220 |
| 38 (9 units) | 23 Oct 2026 | 48 of 49 (0 under-measured; 0 real deviations, 1 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.8 % (87.7 % over the 48 old units); canvas pairs 191 / 219 |
| 39 (9 units) | 23 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.7 % (87.6 % over the 48 old units); canvas pairs 191 / 218 |
| 40 (4 units) | 23 Oct 2026 | 48 of 49 (0 under-measured; 1 real deviation, 0 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.9 % (87.8 % over the 48 old units); canvas pairs 196 / 225 |
| 41 (1 unit) | 23 Oct 2026 | 48 of 49 (0 under-measured; 1 real deviation, 0 unclear) | **98.0 %** (48 / 49); 97.9 % over the 48 old units; step level 87.9 % (87.8 % over the 48 old units); canvas pairs 196 / 225 |
| 42 (1 unit) | 23 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 87.9 % (87.8 % over the 48 old units); canvas pairs 196 / 225 |
| 43 (full, 49 units) | 24 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear; TW-231 closed as a runner artefact by the hand check CHK-1) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 88.0 % (87.9 % over the 48 old units); canvas pairs 195 / 224 |
| 44 (4 units) | 24 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 88.0 % (87.9 % over the 48 old units); canvas pairs 195 / 224 |
| 45 (2 units) | 25 Oct 2026 | 49 of 49 (0 under-measured; 0 real deviations, 0 unclear; runner R-37: TW-232 closed, TW-231's runner item done) | **100.0 %** (49 / 49); 100.0 % over the 48 old units; step level 88.3 % (1102 / 1248; 88.2 % over the 48 old units, 1093 / 1239); canvas pairs 196 / 225 (`filters-adjust-color/09_color_fill_result` 0.35 % back; 187 / 216 over the 48 old units) |

The next steps are in [ROADMAP.md](../ROADMAP.md): the hands-on checks A4-76 to A4-81 of tester feedback batch 8 (follow-up run 44 reached only TW-237 and the thumbnail frame of TW-238), then a release with batch 8; the TW-225 rest that Reveal All does not select the new mask (seen in `inspector-bitmap/08` since run 33); the runner items R-37 (TW-231, closed as a runner artefact by the hand check CHK-1: text clicks after a right-click only inside the open context menu, no non-exact fallback for menu entries, `xs-context-menus/15` re-specified) and TW-232; the human click-test, including the batch 7 items A4-67 to A4-75, including the macOS re-test by the tester;
the text selection and size of TW-146 rest and the text tool reset of TW-165 (moved to the click-test A1-19 on
20 Oct 2026 as runner measurement gaps, not deviations; TW-167 was closed by run 29); the text tool reset of RE spec 56
(TW-138 R5 to R7, F52, click-test A4-29; known difference: for a new text the reference writes two History rows,
"Text" and "Text Change", Firepixel one; the leading conversion from px back to % is not measured); the click-test
items of TW-195 and the refined TW-192 (A4-43, A4-44) and of tester feedback batch 5 (A4-45 to A4-49; the Pen display
and the pen cursor variants of TW-202 are reached by no runner step), of the Dither fill edge (A4-50, A4-56) and of
tester feedback batch 6 (A4-51 to A4-55; run 37 reached TW-204 to TW-208) and of F79 to F86 (A4-60 to A4-65); TW-219
(the transform frame of a text, measured by RE-35 in RE spec 67, fixed by F87 and confirmed by follow-up run 42 with
runner R-36) for the click-test A4-66 and TW-212 for A4-65; TW-220 (the transform handle colour, design level:
Firepixel's light-blue frame line and selection-blue handles against the reference's black line and squares; to check
the rule and align); TW-217 and TW-218 (F85), which no runner step reaches, for the click-test A4-64; the questions Q-62 (re-measure with real keys) and Q-64 (gradient and pattern
fills on text). TW-173 was confirmed by run 31, TW-189 by
run 32, TW-192 (colours), TW-193, TW-194 and the runner item TW-174 by run 33, and TW-195, the refined follow rules of
TW-192 and the runner item TW-196 by run 34; run 35 saw TW-202 (the Pointer display of a selected rectangle), TW-200
(the slider pop-up placement) and TW-201 (the tooltips) without a changed value, run 36 none of TW-204 to TW-208, run 37 confirmed TW-206, TW-207 and the runner items TW-197, TW-210 and TW-211, run 38 TW-205, TW-214, TW-215, the runner item TW-216 and Q-65 (TW-208 in its range), run 39 TW-208 in full (with the runner item Q-66, R-34), run 40 TW-212 on the numeric and script paths (with the fold under the Scale tool, whose factor differs by TW-219), run 41 TW-212 again on these paths (TW-219 not reached: the runner's handle position, TW-221), run 42 TW-219 and the Scale-tool factor of TW-212 (with the runner item TW-221, R-36), full run 43 all of these again and the batch 7 changes the runner reaches (TW-223 to TW-227 as display only), follow-up run 44 the batch 8 changes the runner reaches (the blue thumbnail frame of TW-238 and the Show Group Contents entry of TW-237 as display only; TW-233 to TW-236 and the mask-editing row of TW-238 not reached); Q-63 is closed; TW-176, TW-177, TW-186 and TW-187 are not
reached by the runner and stay with the click-test. Run 30 answered the BA-098 question: the tiled canvas shifts no
sampling position at
fractional zoom. U-1 rest (PDF layers) and the R3 note of TW-138 were closed on 20 Oct 2026.
