# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (follow-up run 32 over 3 units on top of runs 31 and 30, 21 Oct 2026, all 49 units):** **49 equal
except for design**, 0 real deviations, 0 unclear, 0 under-measured. **Proven 1:1: 100.0 %** (49 / 49; all 49 units
are sufficiently measured). Measurement quota over all 49 units: **87.2 %** (1075 / 1233 steps). Canvas image pairs
within 1 %: **185 / 212**. TW-189 (Enter on the canvas keeps the selection) and TW-173 (Ctrl+Y repeats a clone) are
fixed and confirmed. The tiled canvas of BA-098 shows no one-pixel sampling offset at fractional zoom (see `xs-canvas`
below).

**Units: 49.** Over the 48 units classified before run 24 (without the measurement unit `xs-select-behind`, added on
19 Oct 2026): 48 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.1 % (1066 / 1224; run 17 alone: 86.3 %), canvas pairs
176 / 203.

## Method: scripted comparison runs

The parity runner (`tools/parity/runner/`, see its README) plays the same step script in **both programs** on
one Windows machine, unattended:

1. **Units.** The UI is cut into 49 units (one tool group, one panel, one menu, ten dialogs …). Each unit has a
   JSON script in `tools/parity/scripts/<unit>.json` with about 20–40 steps (about 1295 steps in total; the
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

## What the classes mean

| Class | Meaning |
|---|---|
| **equal except for design** | The run finds no wrong value, no missing element and no canvas deviation in this unit. Windows, menus and dialogs are *not* pixel-identical (fonts, spacing, own icons, theme), so most non-canvas pairs stay above 3 %. |
| **real deviation** | A measured, reproducible difference in behaviour or result that is Firepixel's fault. |
| **unclear** | A difference exists, but the evidence is not conclusive (runner state, reference side invalid, 1 px). |
| **under-measured** | Fewer than 60 % of the measurable steps have `ok` on both sides. |

**Measurement quota** = steps with both sides `ok` / steps without an `unsupported` or `skipped` side (repeated
steps count with their second attempt). **Proven 1:1** = units "equal except for design" / sufficiently
measured units. The headline figure is the strict one over all classified units (all 49 since run 24; 100.0 % after runs 26 to 29, 98.0 % after run 30); since
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
  results there are less certain.
- **Focus:** other applications taking the foreground make reference steps fail; such steps are repeated or
  counted as errors, never as `ok`.
- The reference application's own rendering is the oracle only where it is deterministic; cursor images
  cannot be captured.

## Per-unit result (run 32 on top of runs 31 and 30)

Each row is the unit's newest run. Three rows (`xs-shortcuts`, `xs-preferences`, `toolbox`) come from follow-up run 32
(21 Oct 2026, 08:22–08:50, 3 units, 103 scripted steps, 41 image pairs, no shot without a partner), preview build
`2026-10-21_08-16` from `main` at `f543d7b9` (v0.2.6 plus the F53 fixes and F55: TW-189 Enter on the canvas keeps the
selection; the Q-60 UI test `q60_pref_category_click`), runner R-28 (`4e1a35af`: Firepixel's View ▸ Tooltips is read
and switched off at each unit start, TW-190; after the press-and-hold that opens a tool flyout the pointer leaves
Firepixel's window, so no tooltip lies over the flyout shot; the key check presses Delete, Backspace and the arrows as
scripted on the reference side too, without re-selecting, TW-191; a `pref_category` target takes the list row's
accessibility hit instead of the page heading with the same text, Q-60). "Was" compares these rows with run 31. The
reference was not restarted (one process for all three units), its right panel dock read 271 px (expanded) at every
unit start, and three other applications' windows were open at the start (logged). R-28 sent 239 plain keys on the
scan-code path and 72 modified keys on the vk path. Five rows (`menu-edit`, `tool-path-reshape`, `xs-canvas`,
`tool-text`, `panel-pages-states-history`) come from follow-up run 31 (21 Oct 2026,
06:28–07:30, 8 units, 229 scripted steps, 110 image pairs plus 5 shots without a partner), preview build
`2026-10-21_06-27` from `main` at `e1340a21` (the code of release v0.2.6 plus the F53 fixes: TW-173 Ctrl+Y repeats the
last step when nothing is undone, TW-177 Enter / Escape do not end a pen path, TW-176 the Pointer drag threshold
3·√zoom with a zero-move row, TW-178 the text reset values), runner R-27 (`b61d8e01`: plain keys reach the reference
as scan-code `SendInput`, and the R-19 key substitutes are off for the first time; the old plain-key problem was a bug
in our own reference host, which sent unmodified named keys as their last letter, TW-179). "Was" compares these rows
with run 30. The reference was not restarted; three other applications' windows were open at the start (logged).
In run 31 Firepixel's canvas shots carried the View ▸ Tooltips coordinate tag at the pointer (TW-184, released in
v0.2.6), which the reference's shots lack (its switch is off on the reference machine; TW-190, closed by runner R-28
in run 32). The other 41 rows come from
full run 30 (20/21 Oct 2026, 19:22–00:40, 49 units, 1295 scripted steps, 699 image pairs
plus 51 shots without a partner), preview build `2026-10-20_19-06` from `main` at `3b7386fa` (version 0.2.5 plus JPEG
2000 in the PDF export, BA-101, and the AI batch steps, BA-109; the first full run with fixes F51a to F51c, the text
tool reset F52 and the performance work BA-097 to BA-099, among it the canvas texture in 512-px tiles, BA-098), runner
R-26 (unchanged since run 29). "Was" compares each unit with its newest earlier run: run 29 for `toolbox`,
`tool-path-reshape` and `xs-select-behind`, run 28 for `tool-text`, run 26 for `menu-text` and `xs-shortcuts`, run 25
for fourteen units and run 24 for the other 29. Canvas pairs are compared with the offset search (± 3 px, share of
pixels deviating by more than 32 levels); the percentages of non-canvas pairs are the runner's own `cmp/summary.csv`
values. Two other applications' windows were open at the start (logged). The reference was restarted four times:
three planned restarts before the script-heavy units `menu-view`, `panel-misc` and `xs-file-formats`, and once after a
script access violation at the start of `xs-windows-tabs`, without lost steps. Its right panel dock read 271 px
(expanded) at every unit start, but its docked panels have other sizes than in runs 24 and 25 (Layers panel shot
270 × 113 px, run 25: 245 × 389), so the panel and overview pairs moved (overviews 19–23 %, runs 24 and 25: 29–34 %)
and two owner-drawn panel steps hit other rows (TW-174, runner). The findings of the earlier runs are summarised below
the table.

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | run 30: 89.3 % (25 / 28), was 92.9 % (26 / 28): `19_tab_enter_esc_focus_d10` fails on the reference side ("no dialog open"; Firepixel `ok`); `18_d10_effect` fails on the reference side as before |
| `dialogs-d11-d20` | dialogs | equal except for design | run 30: 88.9 % (32 / 36), unchanged |
| `dialogs-d21-d30` | dialogs | equal except for design | run 30: 100.0 % (27 / 27), unchanged; text runs fixed (TW-02) |
| `dialogs-d31-d40` | dialogs | equal except for design | run 30: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 %; `11` and `16` fail on both sides as before (the dialog is closed before the read-back) |
| `dialogs-d41-d50` | dialogs | equal except for design | run 30: 91.3 % (21 / 23), unchanged |
| `filters-adjust-color` | filters | equal except for design | run 30: 69.6 % (16 / 23), unchanged; 4 / 4 canvas pairs within 1 % |
| `filters-blur-sharpen-noise` | filters | equal except for design | run 30: 79.3 % (23 / 29), unchanged; 10 / 11 canvas pairs within 1 % (`16_preview_box_on_result` 20.0 % as before) |
| `fx-bevel-emboss-fill` | live effects | equal except for design | run 30: 90.5 % (19 / 21), unchanged; 2 / 4 canvas pairs within 1 % (the live pop-up shots `01` and `09` as before); `11` fails on both sides as before |
| `fx-shadow-glow` | live effects | equal except for design | run 30: 100.0 % (23 / 23), unchanged; 5 / 6 canvas pairs within 1 % |
| `fx-stack` | live effects | equal except for design | run 30: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 % |
| `inspector-bitmap` | property inspector | equal except for design | run 30: 100.0 % (15 / 15), unchanged; 3 / 3 canvas pairs within 1 % |
| `inspector-fields` | property inspector | equal except for design | run 30: 83.3 % (25 / 30), was 86.7 % (26 / 30); 0 / 1 canvas pairs within 1 % (run 24: 1 / 1). `14_popup_drag_live_canvas_pressed` 2.03 % (run 24: 0.64 %): with the mouse held on the opacity pop-up slider at 30 % Firepixel now previews the opacity on R1 (lighter fill), while the reference's canvas stays unchanged as in run 24; this is the deliberate live preview of TW-171 (F51b, listed in `[N]`), not a new deviation. The reference's pop-up shot `14_popup_drag_live_popup_pressed` fails this time ("no popup slider open"). `10_edge_popup` (reference) and `20_undo_each` (Firepixel: no text field at the stroke size) fail as before |
| `inspector-stroke` | property inspector | equal except for design | run 30: 88.2 % (45 / 51), unchanged; 31 / 31 canvas pairs within 1 %; `03` and `12` fail on the reference side as before (script errors); in `03` Firepixel now records no History step for the clamped width (TW-171, deliberate) |
| `inspector-text` | property inspector | equal except for design | run 30: 90.0 % (27 / 30), unchanged; 10 / 11 canvas pairs within 1 % (`21_live_result` 1.88 % as before) |
| `inspector-web` | property inspector | equal except for design | run 30: 81.2 % (13 / 16), unchanged |
| `menu-commands-window-help` | menus | equal except for design | run 30: 75.0 % (15 / 20), unchanged; `01` (Firepixel's Commands submenu not found) as before |
| `menu-edit` | menus | equal except for design | run 31: 91.7 % (22 / 24), unchanged; was real deviation. **TW-173 fixed and confirmed**: `02_redo_last_result` (select R1, Ctrl+Shift+D, Ctrl+Y) reads three R1 at 50,50–150,120 in both programs (run 30: Firepixel two), and the Edit menu shots `01_undo_redo_open` and `07_paste_special_open` show the enabled "Wiederholen: Klonen" in both (run 30: Firepixel the grey "Can't Redo"). The reference's History read-back in `01_undo_redo_result` and `22_undo_names_result` returned no rows in run 30 and is back to the run-25 values (the runner's select script, then Clone; Clone, Copy, Paste, Delete), Firepixel's unchanged; `09` and `22` delete with the real Delete key now (R-27). `16` and `17` fail as before |
| `menu-file` | menus | equal except for design | run 30: 81.8 % (18 / 22), unchanged |
| `menu-modify-a` | menus | equal except for design | run 30: 96.2 % (25 / 26), unchanged; 4 / 6 canvas pairs within 1 % (`05_rotate_canvas` and `20_marquee_to_path` as before); `24`: Group as Mask is grey in the reference (unchanged) |
| `menu-modify-b` | menus | equal except for design | run 30: 92.0 % (23 / 25), unchanged; 5 / 5 canvas pairs within 1 % |
| `menu-select` | menus | equal except for design | run 30: 100.0 % (16 / 16), unchanged; 6 / 6 canvas pairs within 1 % |
| `menu-text` | menus | equal except for design | run 30: 90.9 % (20 / 22), unchanged; 4 / 4 canvas pairs within 1 %; `01` (Firepixel's scrolled font menu not found), `05_editor_open` (`unsupported` on the reference side) and `07` (Attach to Path grey in the reference) as before |
| `menu-view` | menus | equal except for design | run 30: 96.0 % (24 / 25), unchanged; 6 / 6 canvas pairs within 1 % (`11_slice_guides` 0.30 %, `12_slice_overlay` 0.57 %; run 25: 0.18 % and 0.46 %); window pair `15_hide_panels_result` 56.2 % (run 25: 39.5 %; the reference's shot changed, Firepixel's is pixel-identical) |
| `panel-info-align-path` | panels | equal except for design | run 30: 83.3 % (30 / 36), unchanged; 2 / 2 canvas pairs within 1 % |
| `panel-layers` | panels | equal except for design | run 30: 74.1 % (20 / 27), unchanged. `04_eye_drag_result`: the reference's eye drag now leaves "Unterebene" and "Gemeinsam" visible (Firepixel, and the reference in run 25: hidden), with the reference's Layers panel shot at 270 × 113 px (run 25: 245 × 389), so its panel pairs rise to 82–89 % (TW-174, runner; Firepixel's shots unchanged) |
| `panel-layers-menu` | panels | equal except for design | run 30: 81.0 % (17 / 21), unchanged; `10_options_open` still opens no dialog the bridge finds. The layer context menu `03` now reads the same 23 entries and states in both programs (run 25: a stray "Page 1" entry in Firepixel's read-back). The object context menu `04` reads as in run 25: the reference enables Flatten Selection and Add Mask, Firepixel greys both (Q-59, not new) |
| `panel-misc` | panels | equal except for design | run 30: 87.5 % (14 / 16), unchanged; embedded panels `unsupported` |
| `panel-optimize` | panels | equal except for design | run 30: 83.3 % (20 / 24), unchanged |
| `panel-library-styles` | panels | equal except for design | run 30: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 % |
| `panel-pages-states-history` | panels | equal except for design | run 31: 75.0 % (18 / 24), unchanged. With real arrow keys (R-27) the reference's History lists the three nudges as one Move row (`moveSelectionBy({x:2, y:1})`) like Firepixel's "Move" (run 30: three runner script rows of the R-19 substitutes): `16_history_replay` reads 5 rows in the reference (select script, Move 2,1, select script, Move 10,0, the replay script) against Firepixel's 4 (run 30: 7 against 4), `21_history_merge_nudge` 4 against 3 as before (the reference's runner select scripts against Firepixel's Deselect row; run 30's reference nudged with Shift, 20,10), and the History shots `14` and `19` show the same Move rows in both. The reference's docked panels changed size again (Pages, States and History shots 270 × 180 px; run 30: 270 × 113 and 270 × 262; TW-174), so the Pages / States pairs read 55.6–62.7 % (run 30: 77.1–86.5 %) and History 92.8–93.1 %. `18_history_save_cmd_open` fails on both sides and the onion-skin and loop pop-ups are not found on the reference side, as before |
| `preview-options` | preview / export | equal except for design | run 30: 76.9 % (30 / 39), unchanged; Firepixel-side errors as in run 24 (`08` and `13`: no text field at dither / smoothing; `24_result_hash_gif/jpeg/png8/png32`: no file dialog for the bridge) |
| `preview-views` | preview / export | equal except for design | run 30: 69.6 % (16 / 23), unchanged; Firepixel's shots pixel-identical to run 24 except the running animation `17_anim_buttons_play_pressed`, so the small rises (about +0.4 points) are on the reference side |
| `color-popup-contexts` | colour | equal except for design | run 30: 68.2 % (15 / 22), unchanged |
| `tool-path-reshape` | tools | equal except for design | run 31: 100.0 % (23 / 23), unchanged; 13 / 14 canvas pairs within 1 %, the same pairs (`11_rect_to_path` 11.1 % as before; the others 0.28–0.45 %, run 30: 0.19–0.41 %): Firepixel's exported PNGs are byte-identical to run 30 and its shots differ only by the coordinate tag at the pointer (TW-190); History rows and inspector heads as in run 30 (`12_inspector_heads_freeform` 9.7 %). No step draws with the Pen, so TW-177 is not exercised here |
| `tool-text` | tools | equal except for design | run 31: 85.2 % (23 / 27), unchanged; 13 / 16 canvas pairs within 1 %, the same pairs as in run 30 (`11_size_live` 2.29 %, `14_bounds` 1.11 %, `16_overflow` 3.80 %; TW-146 rest and TW-165, click-test A1-19); Firepixel's shots are unchanged except the coordinate tag (TW-190). 10 `runner error` lines (run 30: 11): on the scan-code path the reference's text entry lost the first "A" in `02_drag_frame` (two tries; the first found the Subselection tool active) and in `09_backspace` (two lines: not typed, then the empty text discarded at the commit), and needed a second try in `12`; the four steps that lost it in run 30 (`03`, `04`, `05`, `20`) typed it this time, so `03_typing_result` compares two "Abc 123" texts (0.11 %). The six selection tries in `07`, `11`, `13` and the size step of `11` as before. The reference's "Hallo Welt" in `07` now lies at 101,91 like Firepixel's (run 30: 45,91). `15` and `19` fail on both sides (frame handle not found), `12` Firepixel's font field not found, `17` `unmapped`, as before |
| `tool-web` | tools | equal except for design | run 30: 96.4 % (27 / 28), unchanged; 14 / 16 canvas pairs within 1 % (`07` guide drag 3.7 % as before); `11` fails on both sides as before |
| `toolbox` | tools | equal except for design | run 32: 94.6 % (53 / 56), unchanged; 0 / 3 canvas pairs within 1 % (cursor shots 2.49 %, back at the run 30 value; run 31: 2.53 %); no `runner error`. **TW-190 closed**: Firepixel's View ▸ Tooltips is off and after the press-and-hold the pointer leaves its window (R-28), so its flyout shots `02_flyouts_*` show the bare flyout without any tooltip: the ten pairs read 31.1–72.7 % (run 31: 36.8–85.1 %, run 30: 31.1–78.0 %), 1.3 to 25.9 points below run 31 and 0 to 7.4 points below run 30, whose shots still carried the button's short name tooltip over the flyout. Firepixel's cursor shots and its full screen shot are pixel-identical to run 30; the tooltip pairs `03` keep Firepixel's second description line (TW-184, deliberate; as in run 31). Window pairs: overview 25.2 %, full 1.97 %, full with menus 46.8 %, Standard 25.5 % (run 31: 25.2, 2.02, 46.8 and 25.5 %). No step drags the Pointer (TW-176 not exercised); the reference restored its collapsed dock once after full screen with menus (105 → 271 px), as in runs 29 to 31 |
| `tool-autoshapes` | tools | equal except for design | run 30: 100.0 % (40 / 40), unchanged; **27 / 27** canvas pairs within 1 % (run 25: 26 / 27): `05_handle_drag_rounded_corner_result` 0.97 % (run 25: 1.01 %). Firepixel's shots are unchanged; the reference's shapes moved by about one pixel (in run 25 the offset search improved 13 of the 27 pairs with a 1–2 px offset, now one; all 27 pairs 0.13–0.97 %) |
| `export-results` | preview / export | equal except for design | run 30: 80.0 % (8 / 10), unchanged; 4 / 4 canvas pairs within 1 %; `13_export_area` `unsupported` on the reference side as before. JPEG 2000 in the PDF export (BA-101) is not exercised: `10_pdf_result` is `unsupported` on both sides |
| `color-popup` | colour | equal except for design | run 30: 75.0 % (21 / 28), unchanged; 0 / 2 canvas pairs within 1 % (capture method only, TW-117); `21_undo` fails on the reference side as before |
| `xs-context-menus` | cross-cutting | equal except for design | run 30: 70.4 % (19 / 27), unchanged |
| `xs-preferences` | cross-cutting | equal except for design | run 32: 85.0 % (17 / 20), unchanged; was unclear. **Q-60 settled** (runner, R-28): Firepixel's `pref_category` click now takes the list row from the accessibility hit (logged "list row [445, 312, 613, 333] (accesskit, 1 other hit(s) left out)") instead of the bold page heading with the same text, and in `20_tab_focus_focus` the Down after the click on "Allgemein" moves both category lists to "Bearbeitung" (pair 47.4 %, run 31: 51.2 %); the F55 UI test `q60_pref_category_click` pins the row behaviour. **TW-190 closed**: Firepixel's View ▸ Tooltips is switched off at the unit start (logged "switched on -> off"), so its canvas shots carry no coordinate tag and are pixel-identical to run 30: 2 / 3 canvas pairs within 1 % at the run 30 values (`06` 3.10 %, `07` 0.14 %, `08` 0.09 %; run 31: 3.41, 0.19 and 0.15 %). The Edit page in `20` shows "Delete objects when cropping" off in the reference's profile and on in Firepixel's (stored profile state, TW-127; not a compared value). All values as in run 31; `03`, `04` and `10` (`unmapped`) and `18` (`unsupported`) as before |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 30: 73.7 % (14 / 19), unchanged |
| `xs-file-formats` | cross-cutting | equal except for design | run 30: 85.7 % (18 / 21), unchanged; 4 / 4 canvas pairs within 1 %; `05` and `15` fail on both sides as before |
| `inspector-group-symbol` | property inspector | equal except for design | run 30: 90.0 % (18 / 20), unchanged; 1 / 1 canvas pairs within 1 % |
| `panel-swatches-mixer` | panels | equal except for design | run 30: 96.3 % (26 / 27), unchanged; `19_selected_object_result`: the reference's swatch 5 sits elsewhere in its resized panel and reads #ffffbf, Firepixel #efffbf as both read in run 24 (TW-174, runner) |
| `xs-shortcuts` | cross-cutting | equal except for design | run 32: 92.0 % (23 / 25), unchanged; was real deviation. **TW-189 fixed and confirmed**: `22_enter_canvas_result` (the group G of T06 selected, Enter) reads `[object Group]` in both programs (run 31: Firepixel `null`). **TW-191 closed** (R-28): in `02_edit_result` the reference's key check no longer re-selects R1 before the final Delete; with nothing selected after Ctrl+A and Ctrl+D the Delete removes nothing on either side (9 elements before and after in both, Firepixel undo 6 / redo 0 unchanged; run 31: the reference re-selected R1 and deleted the three R1, 9 → 6). Every other compared value reads as in run 31, the TW-173 redo cases (`02`, `17`) stay equal, and Firepixel's four shots are pixel-identical to runs 30 and 31. Two `runner error` lines in the shot-only step `15_in_text_result` (`unsupported` on both sides): the reference's text entry lost the "V" in two attempts (run 31: typed at once) and the empty text was discarded at the commit (TW-146 rest); `15_in_text_tx2` typed it on the second try. The reference's File menu in `20_alt_menus_open` shows Save enabled this time (runs 30 and 31: grey; the state flips between runs on the reference side, as in runs 13, 14, 17 and 19 to 21; Firepixel grey as before). `18` and `19` (`unmapped`) as before |
| `xs-canvas` | cross-cutting | equal except for design | run 31: 91.7 % (22 / 24), unchanged; 1 / 8 canvas pairs within 1 %, as before. Tiled canvas (BA-098): Firepixel's shots at the fit zoom 42.83 % (`03`, `11`, `13`, `14`, `16`) and the zoomed `02_zoom_center` differ from run 30 only by the coordinate tag (at most 1122 px in a 66 × 17 px box, TW-190) and are pixel-identical outside it; `02_zoom_center_result` 1.54 % (run 30: 1.50 %) at the same offset (−2, −3), `02_zoom_center_result_center` 1.49 % and `18_checker` 0.00 % unchanged. The Pointer drags of `08` (guide snap) and `12` (grid snap) leave TILES at 500,0 and 12,20 in both programs as before; no drag stays below the threshold or ends on its start (TW-176 not exercised). Window pairs +1.0 to +2.5 points from the reference's dock layout (TW-174) |
| `xs-select-behind` | cross-cutting | equal except for design | run 30: 100.0 % (9 / 9), unchanged; 9 / 9 canvas pairs within 1 % (0.22–0.63 %, run 29: 0.14–0.51 %). The rise is on the reference side: its front rectangle FR, drawn by `addNewRectangle`, has a 2 px black stroke centred on the edge this time (selection read-back 399,199–601,301; run 29: a 1 px stroke inside, 400,200–600,300), while Firepixel draws FR without a stroke in both runs (Q-58) |

Totals over all 49 units (run 32 on top of runs 31 and 30): **49 / 0 / 0 / 0** (proven 1:1 **100.0 %**). Step level:
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

**What run 18 found.** With both programs on the "Web Standard" set, Ctrl+Shift+D clones on both sides and the
Clone-dependent steps are equal (TW-126 closed); per-slice export formats (TW-128), short panel menus (TW-125),
defined option start states (TW-127) and the reference-side retries (TW-129) are confirmed. Still different under
the same set: four text shortcuts in `xs-shortcuts/06` (TW-133). A second look at the menu read-backs found the URL
panel menu with only "Help" in Firepixel (TW-131, present since at least run 17), and the shots of `tool-text/06`
show that a pointer double-click on a text does not insert the typed character (TW-135). `xs-preferences/06` keeps
the crop difference with the option measured as on in both programs (TW-134). The remaining differences are
reference-side runner effects (TW-132: history read-back script error, Delete, Ctrl+D and plain arrow keys without
effect, text-tool state carried between units).

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

The next steps are in [ROADMAP.md](../ROADMAP.md): the human click-test, including the macOS re-test by the tester;
the text selection and size of TW-146 rest and the text tool reset of TW-165 (moved to the click-test A1-19 on
20 Oct 2026 as runner measurement gaps, not deviations; TW-167 was closed by run 29); the text tool reset of RE spec 56
(TW-138 R5 to R7, F52, click-test A4-29; known difference: for a new text the reference writes two History rows,
"Text" and "Text Change", Firepixel one; the leading conversion from px back to % is not measured); the runner item
TW-174 (the reference's panel layout), the questions Q-58, Q-59 and Q-61, the measurement of TW-186 and TW-187 and the
decision on TW-185. TW-173 was confirmed by run 31 and TW-189 by run 32 (TW-190, TW-191 and Q-60 closed by runner R-28
in run 32); TW-176 and TW-177 are not
reached by the runner and stay with the click-test. Run 30 answered the BA-098 question: the tiled canvas shifts no sampling position at
fractional zoom. U-1 rest (PDF layers) and the R3 note of TW-138 were closed on 20 Oct 2026.
