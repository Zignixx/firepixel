# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 27 over 3 units on top of runs 26, 25 and 24, 19 Oct 2026, all 49 units):** **49 equal except
for design**, 0 real deviations, 0 unclear, 0 under-measured. **Proven 1:1: 100.0 %** (49 / 49; all 49 units are
sufficiently measured). Measurement quota over all 49 units: **87.3 %** (1077 / 1233 steps). Canvas image pairs within
1 %: **185 / 212**.

**Units: 49.** Over the 48 units classified before run 24 (without the measurement unit `xs-select-behind`, added on
19 Oct 2026): 48 / 0 / 0 / 0, proven 1:1 100.0 %, quota 87.3 % (1068 / 1224; run 17 alone: 86.3 %), canvas pairs
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
   32 levels. Canvas pairs below 1 % count as equal.
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
measured units. The headline figure is the strict one over all classified units (all 49 since run 24; 100.0 % after runs 26 and 27); since
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

## Per-unit result (run 27 on top of runs 26, 25 and 24)

Each row is the unit's newest run. Three rows (`tool-path-reshape`, `tool-text`, `toolbox`) come from follow-up run
27 (19 Oct 2026, 20:49–21:18, 3 units, 106 steps, 62 image pairs) with the same preview build as run 26 and runner
R-24 at `f7d13d61` (the size on a text selection by three routes: size field, size keys and the Text Editor dialog;
drags over the glyph box of 72 pt text; the Freeform size keys with the mouse held, a size read-back and a shot while
the button is held; the reference's Standard screen mode read from the window; Freeform chosen through flyout entry 0
and read back). Two rows (`menu-text`, `xs-shortcuts`) come from follow-up run 26 (19 Oct 2026, 16:11–16:55, 5 units,
155 steps, 72 image pairs), preview build `2026-10-19_16-07` from `main` at `37c0ee81` (package F49: three screen modes,
full screen shows only the canvas on black, TW-158; text preference rules, TW-138 rest), runner R-23 at `5f498cd7`
(screen mode read back from the screen-mode buttons, the window against the monitor and the application properties;
Subselection nudges by `moveSelectedBezierPointsBy`; Freeform size keys with the mouse held; the text size reset read
back per attempt, TW-159; settle waits and retries for text entry, TW-146 rest). Two other applications' windows were
open at the start of run 26 (logged). Fifteen rows come from follow-up run 25 (19 Oct 2026, 12:19–14:39, 20 units, 556
steps, 317 image pairs), preview build `2026-10-19_12-16` from `main` at `a225a01c` (package F48: consecutive key
nudges of one selection merge into one History row, TW-148; Select Behind draws a crisp hover line and no frame right
after a click, TW-157), runner R-22 at `72c695dd` (text reset without touching the fill colour and default colours
restored, TW-153; `menu-text/05` and `menu-view/20` unsupported on the reference side, TW-154; arrow substitutes only
with the Pointer or Subselection tool or `nudge: true`, hover in `inspector-stroke/01`, `close_all` in `menu-edit`,
TW-155; standard screen mode around `toolbox/13`, TW-156; text entry retries at 35 ms per character, TW-146 rest;
Firepixel's text reset falls back to another layer, TW-149). The other 29 rows come from full run 24 (19 Oct 2026,
06:09–11:10, preview `2026-10-19_06-06` from `44d7c5f5`, runner R-21). Another application's window was open at the
start of run 25 (logged). Canvas pairs are compared with the offset search (± 3 px, share of pixels deviating by more
than 32 levels). "Was" compares with the unit's previous run; the findings of the earlier runs are summarised below the table.

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | run 24: 92.9 % (26 / 28), unchanged |
| `dialogs-d11-d20` | dialogs | equal except for design | run 24: 88.9 % (32 / 36), unchanged |
| `dialogs-d21-d30` | dialogs | equal except for design | run 24: 100.0 % (27 / 27), unchanged; text runs fixed (TW-02) |
| `dialogs-d31-d40` | dialogs | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 %; Feather, Expand, Save and Restore Bitmap Selection now open on the reference side after the marquee drag (TW-129 closed); `06_d33` equal (twelve I1 on both sides, Ctrl+Shift+D clones, TW-126 closed); `11` and `16` fail on both sides as before (the dialog is closed before the read-back); history read-backs differ only by the runner's own script steps in the reference's history |
| `dialogs-d41-d50` | dialogs | equal except for design | run 24: 91.3 % (21 / 23), unchanged |
| `filters-adjust-color` | filters | equal except for design | run 24: 69.6 % (16 / 23), unchanged; 4 / 4 canvas pairs within 1 % |
| `filters-blur-sharpen-noise` | filters | equal except for design | run 25: 79.3 % (23 / 29), unchanged; 10 / 11 canvas pairs within 1 % (`16_preview_box_on_result` 19.9 % as before) |
| `fx-bevel-emboss-fill` | live effects | equal except for design | run 24: 90.5 % (19 / 21), unchanged; 2 / 4 canvas pairs within 1 % |
| `fx-shadow-glow` | live effects | equal except for design | run 24: 100.0 % (23 / 23), unchanged; 5 / 6 canvas pairs within 1 % |
| `fx-stack` | live effects | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 % |
| `inspector-bitmap` | property inspector | equal except for design | run 24: 100.0 % (15 / 15), unchanged; 3 / 3 canvas pairs within 1 % |
| `inspector-fields` | property inspector | equal except for design | run 24: 86.7 % (26 / 30), unchanged; 1 / 1 canvas pairs within 1 %; the ten reference-side field errors of run 11 are gone; one error per side remains (`10_edge_popup`, `20_undo_each`) |
| `inspector-stroke` | property inspector | equal except for design | run 25: 88.2 % (45 / 51), unchanged; 31 / 31 canvas pairs within 1 %; `01` now hovers the Airbrush entry, no real Down key and no runner error (TW-155 confirmed); `03` and `12` fail on the reference side as before (script errors); values as in run 24 |
| `inspector-text` | property inspector | equal except for design | run 25: 90.0 % (27 / 30), unchanged; 10 / 11 canvas pairs within 1 % (`21_live_result` 1.88 % as before) |
| `inspector-web` | property inspector | equal except for design | run 25: 81.2 % (13 / 16), unchanged |
| `menu-commands-window-help` | menus | equal except for design | run 24: 75.0 % (15 / 20), unchanged |
| `menu-edit` | menus | equal except for design | run 25: 91.7 % (22 / 24), unchanged; with `close_all` one document is open and the reference's History reads again: `01` the runner's select script and Clone (Firepixel: Clone), `22` the script, Clone, Copy, Paste and Delete (Firepixel: Clone, Copy, Paste, Clear), so the rows are equal apart from the runner's script row (TW-155 confirmed); `16` and `17` fail as before (Crop Document and Crop Selected Bitmap grey in the reference, Firepixel opens no dialog the bridge finds) |
| `menu-file` | menus | equal except for design | run 24: 81.8 % (18 / 22), unchanged; `04_reopen_open` is `unmapped` on the reference side instead of an error (Revert was grey in run 17); File menu with Share submenu 637 vs. 613 px (Firepixel's longer own item "Browse in file browser…"); arrow and shortcut gaps equal |
| `menu-modify-a` | menus | equal except for design | run 24: 96.2 % (25 / 26), unchanged; 4 / 6 canvas pairs within 1 %; `11_symbol_tween` equal (twelve I1 on both sides, TW-126 closed); `24`: Group as Mask is grey in the reference (unchanged) |
| `menu-modify-b` | menus | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 5 / 5 canvas pairs within 1 % |
| `menu-select` | menus | equal except for design | run 24: 100.0 % (16 / 16), unchanged; 6 / 6 canvas pairs within 1 %; `06_feather_result` now 0.90 % (run 17: 10.3 %) |
| `menu-text` | menus | equal except for design | run 26: 90.9 % (20 / 22), unchanged; 4 / 4 canvas pairs within 1 %; the reference's unit-start reset reads 72 pt before (taken over from `tool-text`) and Arial 12 pt after, read back per attempt (TW-159 d confirmed); `05_editor_open` `unsupported` on the reference side, `01` (Firepixel's scrolled font menu not found) and `07` (Attach to Path grey in the reference) as before |
| `menu-view` | menus | equal except for design | run 25: 96.0 % (24 / 25), was 92.3 % (24 / 26); 6 / 6 canvas pairs within 1 %; `20_edit_guides_dialog_open` is `unsupported` on the reference side and Firepixel opens Edit Guides by command, shot present (TW-154 confirmed) |
| `panel-info-align-path` | panels | equal except for design | run 24: 83.3 % (30 / 36), unchanged; 2 / 2 canvas pairs within 1 %; rewritten `for`-loop read-backs equal; the reference's undo step-count read-back is unchanged (runner) |
| `panel-layers` | panels | equal except for design | run 25: 74.1 % (20 / 27), unchanged; Firepixel's text tool reset now succeeds at unit start (Arial 12 pt, layer fallback; TW-149 rest confirmed); read-backs equal as in run 24 |
| `panel-layers-menu` | panels | equal except for design | run 25: 81.0 % (17 / 21), unchanged; Firepixel's text tool reset succeeds (TW-149 rest confirmed); `10_options_open` still opens no dialog the bridge finds; values as in run 24 |
| `panel-misc` | panels | equal except for design | run 24: 87.5 % (14 / 16), unchanged; embedded panels `unsupported`; panel menus now read from the pop-up (R-12) |
| `panel-optimize` | panels | equal except for design | run 24: 83.3 % (20 / 24), unchanged; `19` equal: both slices report JPEG and GIF as export format (TW-128 closed); Optimize menu 289 vs. 284 px |
| `panel-library-styles` | panels | equal except for design | run 24: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 %; all values as in run 21 (`16`: nine URL menu entries with equal labels, `ellipsis` and `enabled`); shot 327 × 248 vs. 321 × 231 px (≡ row and shadow included, TW-136) |
| `panel-pages-states-history` | panels | equal except for design | run 25: 75.0 % (18 / 24), unchanged; was real deviation. `21` confirms TW-148 from the History row labels: after Shift+Right, Shift+Right, Shift+Down on R1 and Shift+Right on E1 the reference lists the select script for R1, `moveSelectionBy({x:20, y:10})`, the select script for E1 and `moveSelectionBy({x:10, y:0})`; Firepixel lists Move, Deselect, Move (run 24: Move, Move, Move, Deselect, Move), so both have one row per selection with the summed offset (the remaining rows are the runner's selection steps). `16` stays not comparable (script substitutes on the reference side; Firepixel's replay reads one `edit.move` 2,1 and one 10,0). Firepixel's text tool reset succeeds (TW-149 rest confirmed). Errors as before (reference: onion-skin and loop pop-ups not found; Firepixel: `18` Save as Command opens no dialog the bridge finds) |
| `preview-options` | preview / export | equal except for design | run 24: 76.9 % (30 / 39), unchanged |
| `preview-views` | preview / export | equal except for design | run 24: 69.6 % (16 / 23), unchanged |
| `color-popup-contexts` | colour | equal except for design | run 24: 68.2 % (15 / 22), unchanged |
| `tool-path-reshape` | tools | equal except for design | run 27: 100.0 % (23 / 23), unchanged; 13 / 14 canvas pairs within 1 % (`11_rect_to_path` 13.7 % as before; the new pair `04_freeform_size_keys_result` 0.46 %). **TW-159 rest confirmed**: with the mouse held the reference's three Up and two ] keys keep the Freeform tool, and the size read while the button is held is 72 in both programs (the reference's Properties panel field and Firepixel's size field; the keys change the size on neither side). **TW-164 confirmed**: R-24 chooses Freeform through flyout entry 0 and reads the tool back before every Freeform step; `12_inspector_heads_freeform` now shows the Freeform head on the reference side, pixel-identical to its own `01` shot (9.7 %, the design difference of `01`; run 26: 10.8 % with the Path Scrubber head) |
| `tool-text` | tools | equal except for design | run 27: 85.2 % (23 / 27), unchanged; 13 / 16 canvas pairs within 1 % (run 26: 15 / 16): `11_size_live` 2.36 % as before, and `14_bounds` 1.10 % and `16_overflow` 3.82 % (run 26: 0.19 % and 0.27 %), because R-24 resets the reference's text tool to 12 pt after its failed size step while Firepixel's tool keeps 72 pt from its successful selection edit (the F43 rule, equal to the reference in run 25; Firepixel's texts in `14` and `16` read 72 pt as in run 26; TW-165, runner). Text entry stays complete (all new texts arrive on the first attempt). TW-146 rest stays open: in `11` the drag over "123" reads 24 changed pixels (cover 1.0) before each route, but none of the three routes sets 72 pt (`text_size.method` null): the size field with Enter and 60 × Ctrl+Shift+Period leave the runs at "Abc 123" 12 pt, and Ctrl+Shift+E opens no Text Editor (no key in the "Web Standard" set). In `13` the drag finds the highlight (24 px, cover 1.0) on the 12 pt text, so the 72 pt glyph-box drag is not exercised, and the reference's export equals that of `03` (the grey does not reach "123"; pair 0.81 %). `15` and `19` fail on both sides as before (frame handle not found), `12` Firepixel's font field not found, `17` `unmapped` on both sides |
| `tool-web` | tools | equal except for design | run 25: 96.4 % (27 / 28), unchanged; 14 / 16 canvas pairs within 1 % (`07` guide drag 3.7 % as before); `11` fails on both sides as before |
| `toolbox` | tools | equal except for design | run 27: 94.6 % (53 / 56), unchanged; 0 / 3 canvas pairs within 1 % (cursor shots, 2.6 % as before). **TW-163 confirmed**: R-24 reads the reference's Standard mode from the window against the monitor (1600 × 1000, not covering it); the Standard resets from Standard and from full screen with menus and `13_view_modes_standard_normal` read Standard on both sides, although the screen-mode button still reads "full screen with menus". One `runner error` is left (run 26: six): after the second F the Standard reset clicks the screen-mode button at 17,611, which full screen hides (button means 0, 0, 0), and the Window menu does not open without a menu bar; the next step starts in Standard again (TW-166, runner). Mode pairs (window pairs): full 1.97 % (unchanged), full with menus 47.4 % (run 26: 36.8 %), Standard 64.3 % (run 26: 25.5 %; the reference's right panel dock is collapsed to icons in this shot). The Freeform flyout opens on both sides. The reference was restarted once at unit start after a script access violation, without lost steps |
| `tool-autoshapes` | tools | equal except for design | run 25: 100.0 % (40 / 40), unchanged; **26 / 27** canvas pairs within 1 % (run 24: 7 / 27, run 17: 27 / 27): the reference draws with fill #ffffff and stroke #000000 again, as Firepixel (TW-153 confirmed). The one pair above 1 % is `05_handle_drag_rounded_corner_result` at 1.01 % (run 17: 0.97 %, the same picture: the reference's shot after the handle drag shows the shape without handles and tooltip, Firepixel with both) |
| `export-results` | preview / export | equal except for design | run 25: 80.0 % (8 / 10), unchanged; 4 / 4 canvas pairs within 1 %; values as in run 24 (animated GIF and PNG 8/24/32 pairs within 1 %); `13_export_area` `unsupported` on the reference side as before |
| `color-popup` | colour | equal except for design | run 24: 75.0 % (21 / 28), unchanged; 0 / 2 canvas pairs within 1 %; hex field: the reference's pop-up closes without applying the text the runner sets (#80808000), so Q-49 needs a real keyboard; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | run 25: 70.4 % (19 / 27), unchanged |
| `xs-preferences` | cross-cutting | equal except for design | run 24: 85.0 % (17 / 20), unchanged; 2 / 3 canvas pairs within 1 % (`06` 3.10 %, Firepixel's selection handles and cursor; `07` 0.13 %, `08` 0.09 %); all values as in run 22: `06` crops the page to 280 × 220 with P1, E1, R2, R1 left in both programs; the Escape substitutes keep the selection (`15`) or close nothing (`16`) |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 24: 73.7 % (14 / 19), unchanged; `08_unsaved_prompt_open` is `unmapped` on the reference side instead of an error |
| `xs-file-formats` | cross-cutting | equal except for design | run 24: 85.7 % (18 / 21), unchanged; 4 / 4 canvas pairs within 1 %; import by dragging equal (TW-101) |
| `inspector-group-symbol` | property inspector | equal except for design | run 24: 90.0 % (18 / 20), unchanged; 1 / 1 canvas pairs within 1 %; steps 05–14 all `ok` on both sides with equal values |
| `panel-swatches-mixer` | panels | equal except for design | run 24: 96.3 % (26 / 27), unchanged; the "add swatches" file dialog now opens on the reference side (TW-129 closed); Swatches menu 229 vs. 226 px |
| `xs-shortcuts` | cross-cutting | equal except for design | run 26: 92.0 % (23 / 25), was 91.7 % (22 / 24); `10_nudge_point_result` is `ok` on both sides again: R-23 takes "Teilauswahl" as Subselection and nudges the selected point by script, the first node reads 201,210 in both programs (TW-159 b confirmed); `10_nudge_result` reads R1 51,50 and 51,51 in both; `18` and `19` (Duplicate Set dialog not mapped) and `15_in_text` as before |
| `xs-canvas` | cross-cutting | equal except for design | run 25: 91.7 % (22 / 24), unchanged; 1 / 8 canvas pairs within 1 % (fit zoom and grid phase as reviewed in run 17) |
| `xs-select-behind` | cross-cutting | equal except for design | run 25: 100.0 % (9 / 9), unchanged; 9 / 9 canvas pairs within 1 % (0.14–0.51 %); was real deviation. TW-157 confirmed by pixel counts: on the vector rectangle FR (`08`) both programs draw 680 px of #E00000 in 398,198–602,302 (run 24: Firepixel 100 px plus an anti-aliased line); right after a click (`05`, `06`) neither shows #E00000 (run 24: Firepixel 100 and 1680 px); the frame on B1 in `02` and `04` is 1680 px at 38,38–522,362 in both, `03` and `07` show none. Whether the blue and the red frame show together stays with the click-test (A4-17) |

Totals over all 49 units (newest run per unit): **49 / 0 / 0 / 0** (proven 1:1 100.0 %). Step level: 1077 of 1233
measurable steps `ok` on both sides (**87.3 %**); over the 48 units before run 24: 1068 of 1224 (87.3 %). In the 3
units of run 27: 99 of 106 (93.4 %), the same as in run 26; no quota changed. Reference-side errors: 33,
Firepixel-side errors: 21 (both unchanged; in the three units 2 and 3: `tool-text/15, 19` frame handle not found on
both sides, `tool-text/12` Firepixel's font field). Canvas image pairs within 1 %: **185 of 212** (run 26: 186 of
211): `tool-path-reshape/04_freeform_size_keys_result` adds a pair within 1 % (0.46 %), `tool-text/14_bounds` and
`/16_overflow` move above 1 % (1.10 % and 3.82 %, runner state, TW-165). No class changed. The 2 `runner error`
lines (run 26: 8): `tool-text/11` (72 pt not on the selected "123" by field, keys or dialog; TW-146 rest) and
`toolbox/13` (the Standard reset from full screen; TW-166). The reference was restarted once after a script access
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

The next steps are in [ROADMAP.md](../ROADMAP.md): the human click-test, including the macOS re-test by the tester;
the remaining runner items (the text selection and size of TW-146 rest, the one-sided text tool reset of TW-165 and
the Standard reset from full screen of TW-166); U-1 rest; and the unmeasured R3 note of the text tool defaults
(TW-138 rest).
