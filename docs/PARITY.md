# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (full run 24, 19 Oct 2026, all 49 units):** **46 equal except for design**, 2 real deviations,
1 unclear, 0 under-measured. **Proven 1:1: 93.9 %** (46 / 49; all 49 units are sufficiently measured). Measurement
quota over all 49 units: **87.0 %** (1074 / 1235 steps). Canvas image pairs within 1 %: **164 / 210** (the 20 pairs of
`tool-autoshapes` above 1 % are a runner artefact, TW-153).

**Units: 49.** Over the 48 units classified before run 24 (without the measurement unit `xs-select-behind`, added on
19 Oct 2026): 46 / 1 / 1 / 0, proven 1:1 95.8 %, quota 86.9 % (1065 / 1226; run 17 alone: 86.3 %), canvas pairs
155 / 201.

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
measured units. The headline figure is the strict one over all classified units (all 49 since run 24; 93.9 %); since
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

## Per-unit result (full run 24)

All rows come from full run 24 (19 Oct 2026, 06:09–11:10, 49 units, 1295 steps, 698 image pairs), preview build
`2026-10-19_06-06` from `main` at `44d7c5f5` (package F46 after RE spec 51: Escape keeps the selection, one History
row per arrow-key nudge; package F47 after RE spec 52: pen tool anchor modes, FB-01 / TW-151, and the Select Behind
hover frame, FB-02 / TW-152), runner R-21 at `b1916aa6` (History row labels on both sides, a caret check before
typing on the reference side, Firepixel's text tool reset at unit start to the reference's font, size, style and
colour, slow limit 90 s plus 2 s per substitute, foreign-window titles without the console code-page error) on top of
R-19 / R-20 (key substitutes, text entry with commit check). It is the first full run since run 17 and the first run
of `xs-select-behind`. Another application's window was open at the start (logged). Canvas pairs are compared with
the offset search (± 3 px, share of pixels deviating by more than 32 levels). "Was" compares with the unit's previous
newest run (run 23 for five units, run 22 for two, run 18 for four, run 17 for the others); the findings of the
earlier runs are summarised below the table.

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | run 24: 92.9 % (26 / 28), unchanged |
| `dialogs-d11-d20` | dialogs | equal except for design | run 24: 88.9 % (32 / 36), unchanged |
| `dialogs-d21-d30` | dialogs | equal except for design | run 24: 100.0 % (27 / 27), unchanged; text runs fixed (TW-02) |
| `dialogs-d31-d40` | dialogs | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 %; Feather, Expand, Save and Restore Bitmap Selection now open on the reference side after the marquee drag (TW-129 closed); `06_d33` equal (twelve I1 on both sides, Ctrl+Shift+D clones, TW-126 closed); `11` and `16` fail on both sides as before (the dialog is closed before the read-back); history read-backs differ only by the runner's own script steps in the reference's history |
| `dialogs-d41-d50` | dialogs | equal except for design | run 24: 91.3 % (21 / 23), unchanged |
| `filters-adjust-color` | filters | equal except for design | run 24: 69.6 % (16 / 23), unchanged; 4 / 4 canvas pairs within 1 % |
| `filters-blur-sharpen-noise` | filters | equal except for design | run 24: 79.3 % (23 / 29), unchanged; 10 / 11 canvas pairs within 1 % |
| `fx-bevel-emboss-fill` | live effects | equal except for design | run 24: 90.5 % (19 / 21), unchanged; 2 / 4 canvas pairs within 1 % |
| `fx-shadow-glow` | live effects | equal except for design | run 24: 100.0 % (23 / 23), unchanged; 5 / 6 canvas pairs within 1 % |
| `fx-stack` | live effects | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 2 / 2 canvas pairs within 1 % |
| `inspector-bitmap` | property inspector | equal except for design | run 24: 100.0 % (15 / 15), unchanged; 3 / 3 canvas pairs within 1 % |
| `inspector-fields` | property inspector | equal except for design | run 24: 86.7 % (26 / 30), unchanged; 1 / 1 canvas pairs within 1 %; the ten reference-side field errors of run 11 are gone; one error per side remains (`10_edge_popup`, `20_undo_each`) |
| `inspector-stroke` | property inspector | equal except for design | run 24: 88.2 % (45 / 51), unchanged; 31 / 31 canvas pairs within 1 %; one runner error: in `01` a real Down key selected the Line tool on the reference side before the substitute nudged P1 (TW-155); values as in run 17 |
| `inspector-text` | property inspector | equal except for design | run 24: 90.0 % (27 / 30), unchanged; 10 / 11 canvas pairs within 1 % |
| `inspector-web` | property inspector | equal except for design | run 24: 81.2 % (13 / 16), unchanged |
| `menu-commands-window-help` | menus | equal except for design | run 24: 75.0 % (15 / 20), unchanged |
| `menu-edit` | menus | equal except for design | run 24: 91.7 % (22 / 24), unchanged; values as in run 23 (`03`: R1 at 50,50–150,120 after Ctrl+X, Ctrl+V in both programs; `09`, `22`: the Edit ▸ Clear substitute deletes R1). The reference's History read-back in `01` and `22` reads 0 steps with two documents open (run 23: 2 and 5 steps; TW-155, runner), Firepixel 1 and 4 as before, so the History rows are not compared in this run; `16` and `17` fail as before |
| `menu-file` | menus | equal except for design | run 24: 81.8 % (18 / 22), unchanged; `04_reopen_open` is `unmapped` on the reference side instead of an error (Revert was grey in run 17); File menu with Share submenu 637 vs. 613 px (Firepixel's longer own item "Browse in file browser…"); arrow and shortcut gaps equal |
| `menu-modify-a` | menus | equal except for design | run 24: 96.2 % (25 / 26), unchanged; 4 / 6 canvas pairs within 1 %; `11_symbol_tween` equal (twelve I1 on both sides, TW-126 closed); `24`: Group as Mask is grey in the reference (unchanged) |
| `menu-modify-b` | menus | equal except for design | run 24: 92.0 % (23 / 25), unchanged; 5 / 5 canvas pairs within 1 % |
| `menu-select` | menus | equal except for design | run 24: 100.0 % (16 / 16), unchanged; 6 / 6 canvas pairs within 1 %; `06_feather_result` now 0.90 % (run 17: 10.3 %) |
| `menu-text` | menus | equal except for design | run 24: 87.0 % (20 / 23), unchanged; 4 / 4 canvas pairs within 1 %; `05_editor_open` (Ctrl+Shift+E) now fails on both sides: the "Web Standard" set that both programs use since runner R-13 has no key for the Text Editor, so Firepixel opens no dialog either (run 17 used Firepixel's own set; TW-154, runner) |
| `menu-view` | menus | equal except for design | run 24: 92.3 % (24 / 26), unchanged; 6 / 6 canvas pairs within 1 %; `20_edit_guides_dialog_open` (Ctrl+Alt+Shift+G) now fails on both sides: the "Web Standard" set has no key for Edit Guides (as `menu-text/05`; TW-154, runner) |
| `panel-info-align-path` | panels | equal except for design | run 24: 83.3 % (30 / 36), unchanged; 2 / 2 canvas pairs within 1 %; rewritten `for`-loop read-backs equal; the reference's undo step-count read-back is unchanged (runner) |
| `panel-layers` | panels | equal except for design | run 24: 74.1 % (20 / 27), unchanged; rewritten read-backs (visibility, lock, order, element counts) equal on both sides; locator gaps in the scrolled panel unchanged |
| `panel-layers-menu` | panels | equal except for design | run 24: 81.0 % (17 / 21), unchanged; rewritten read-backs equal; `11_share` now equal (Gemeinsam and Webebene shared on both sides) and `15_merge_down` measured; Firepixel's `10_options_open` still opens no dialog the bridge finds; panel menu 362 vs. 354 px (frame included) |
| `panel-misc` | panels | equal except for design | run 24: 87.5 % (14 / 16), unchanged; embedded panels `unsupported`; panel menus now read from the pop-up (R-12) |
| `panel-optimize` | panels | equal except for design | run 24: 83.3 % (20 / 24), unchanged; `19` equal: both slices report JPEG and GIF as export format (TW-128 closed); Optimize menu 289 vs. 284 px |
| `panel-library-styles` | panels | equal except for design | run 24: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 %; all values as in run 21 (`16`: nine URL menu entries with equal labels, `ellipsis` and `enabled`); shot 327 × 248 vs. 321 × 231 px (≡ row and shadow included, TW-136) |
| `panel-pages-states-history` | panels | real deviation | run 24: 75.0 % (18 / 24), unchanged; was unclear. `21` decides TW-148 from the History row labels: after three real Shift+arrow presses on R1 (50,50 → 70,60) the reference lists **one** row `moveSelectionBy({x:20, y:10})` between the runner's select scripts for R1 and E1, and one row `moveSelectionBy({x:10, y:0})` for the Shift+Right on E1 (4 rows); Firepixel lists Move, Move, Move, Deselect, Move (5). The reference merges consecutive key nudges of the same selection into one row, across directions; F46's one row per press is wrong. `16` (script substitutes, one script row each on the reference side) stays not comparable; R2 at left 200 in both programs. `22`: `MaxUndos` 20 in both. Menus and errors unchanged (reference: onion-skin and loop pop-ups not found; Firepixel: `18` Save as Command opens no dialog the bridge finds) |
| `preview-options` | preview / export | equal except for design | run 24: 76.9 % (30 / 39), unchanged |
| `preview-views` | preview / export | equal except for design | run 24: 69.6 % (16 / 23), unchanged |
| `color-popup-contexts` | colour | equal except for design | run 24: 68.2 % (15 / 22), unchanged |
| `tool-path-reshape` | tools | equal except for design | run 24: 100.0 % (23 / 23), unchanged; 12 / 13 canvas pairs within 1 %; no change after the pen fixes (F47): values and canvas pairs as in run 17 (`11_rect_to_path` 11.1 % as before); `04_freeform_size_keys`: the reference's Up and Down are replaced by `moveSelectionBy` although with the Freeform tool they change the tool size (TW-155, runner) |
| `tool-text` | tools | unclear | run 24: 81.5 % (22 / 27), unchanged; 12 / 15 canvas pairs within 1 %; R-21's caret check passes before every entry, yet the reference loses keys in 4 of 16 new texts: `11` receives only "Ab" (the retry after Ctrl+A leaves nothing), `12`, `13` and `17` nothing, and three texts are discarded at the commit (10:05:48, 10:06:05, 10:07:34; run 23 also three); `13` again finds no colour pop-up (TW-146 rest). `06`: the "Q" after the pointer double-click now arrives, both programs read "ZeilQe eins" with TX2 at 50,150–114,194. Without the text in `11` there is no size edit on "123", so TW-145 is still not compared (the reference's next texts in `14` and `16` 12 pt, Firepixel's 72 pt; `14` 1.07 %, `16` 3.74 %). Firepixel's text tool now starts at Arial 12 pt regular black like the reference (TW-149 closed). New reference error in `17` (the runner opens a combo with export formats instead of the orientation); `12`: Firepixel's font field is not found (as before) |
| `tool-web` | tools | equal except for design | run 24: 96.4 % (27 / 28), unchanged; 14 / 16 canvas pairs within 1 % |
| `toolbox` | tools | equal except for design | run 24: 91.1 % (51 / 56), was 92.9 % in run 17; 0 / 3 canvas pairs within 1 %; `13_view_modes_standard_normal` is `unmapped` on Firepixel's side: the F of the step before leaves Firepixel in a screen mode without the Tools panel, the reopen keeps the mode and the screen-mode button is not found (in run 17 F changed nothing in Firepixel; the reference's mode sequence differs as well, TW-156); letter cycles follow the "Web Standard" set of the reference machine (B: Pencil ↔ Brush); the canvas shots are cursor shots |
| `tool-autoshapes` | tools | equal except for design | run 24: 100.0 % (40 / 40), unchanged; **7 / 27** canvas pairs within 1 % (run 17: 27 / 27). Runner artefact: the reference drew every shape with fill #000000 (`state_before` fill #000000, Firepixel #ffffff) because R-21's unit-start text tool reset through the Properties panel sets the reference's fill colour after the default-colour reset (TW-153); the 20 pairs above 1 % (1.1–3.3 %) are the black fill. Shapes, handles, handle tooltips and values are otherwise as in run 17 |
| `export-results` | preview / export | equal except for design | run 24: 80.0 % (8 / 10), unchanged; 4 / 4 canvas pairs within 1 %; the animated GIF is again **byte-identical** (SHA-1 equal) and PNG 8/24/32 pixel-identical (4 of 4 pairs within 1 %); `13_export_area` is now `unsupported` on the reference side (R-12), Firepixel `ok` |
| `color-popup` | colour | equal except for design | run 24: 75.0 % (21 / 28), unchanged; 0 / 2 canvas pairs within 1 %; hex field: the reference's pop-up closes without applying the text the runner sets (#80808000), so Q-49 needs a real keyboard; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | run 24: 70.4 % (19 / 27), unchanged |
| `xs-preferences` | cross-cutting | equal except for design | run 24: 85.0 % (17 / 20), unchanged; 2 / 3 canvas pairs within 1 % (`06` 3.10 %, Firepixel's selection handles and cursor; `07` 0.13 %, `08` 0.09 %); all values as in run 22: `06` crops the page to 280 × 220 with P1, E1, R2, R1 left in both programs; the Escape substitutes keep the selection (`15`) or close nothing (`16`) |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 24: 73.7 % (14 / 19), unchanged; `08_unsaved_prompt_open` is `unmapped` on the reference side instead of an error |
| `xs-file-formats` | cross-cutting | equal except for design | run 24: 85.7 % (18 / 21), unchanged; 4 / 4 canvas pairs within 1 %; import by dragging equal (TW-101) |
| `inspector-group-symbol` | property inspector | equal except for design | run 24: 90.0 % (18 / 20), unchanged; 1 / 1 canvas pairs within 1 %; steps 05–14 all `ok` on both sides with equal values |
| `panel-swatches-mixer` | panels | equal except for design | run 24: 96.3 % (26 / 27), unchanged; the "add swatches" file dialog now opens on the reference side (TW-129 closed); Swatches menu 229 vs. 226 px |
| `xs-shortcuts` | cross-cutting | equal except for design | run 24: 92.0 % (23 / 25), was 88.0 % in run 23; `08_tools` is `ok` again (92.8 s, within R-21's limit of 90 s plus 2 s per substitute; TW-150 closed); values as in run 23 (`07`: Pointer tool from the flyout, no runner error; `21`: R1 stays selected after the Escape chain; `10_nudge`, `10_nudge_point`, `15_in_text_tx2` equal) |
| `xs-canvas` | cross-cutting | equal except for design | run 24: 91.7 % (22 / 24), unchanged; 1 / 8 canvas pairs within 1 %; `15_snap_pixels_disabled`: Snap to Pixel reads `enabled: false` in both programs |
| `xs-select-behind` | cross-cutting | real deviation | run 24: 100.0 % (9 / 9), first run; 9 / 9 canvas pairs within 1 %. The hover rule is equal: in `02` and `04` both programs frame B1 with 1680 px of #E00000 at 38,38–522,362 (1 px line on the bounds and filled 5 × 5 px corner squares), `03` and `07` show no frame in either, and the click cycle reads B1, then FR in both (TW-152 confirmed). Two differences, all pairs still below 1 % (0.26–0.64 %): (a) on the vector rectangle FR (`08`, nothing selected) the reference draws a crisp 1 px #E00000 line on the pixel row and column inside the coordinates (680 px with the squares), Firepixel the squares (100 px) and an anti-aliased line split over two rows (#EF8000 / #BD664D at the top edge); (b) right after a click with the mouse not moved (`05`, `06`) the reference shows no frame, Firepixel at once the frame of the next candidate (TW-157). The reference's selection outline is never captured, so whether the blue and the red frame show together stays with the click-test (A4-17) |

Totals over all 49 units: **46 / 2 / 1 / 0** (proven 1:1 93.9 %). Step level: 1074 of 1235 measurable steps `ok`
on both sides (**87.0 %**); over the 48 units of run 23: 1065 of 1226 (**86.9 %**, the same count as the carried
newest-run figure; run 17 alone: 86.3 %, 1057 / 1225). The quota is unchanged in 46 of the 48 old units: `xs-shortcuts`
gained `08_tools` (no longer `slow`, +4.0 points), `toolbox` lost `13_view_modes_standard_normal` (−1.8 points,
TW-156). Reference-side errors: 37 (newest-run figure before: 38 plus one `slow` step): `menu-file/04` and
`xs-windows-tabs/08` are now `unmapped`, `tool-text/17` is new. Firepixel-side errors: 23 (before: 21): `menu-text/05`
and `menu-view/20` fail on both sides now because the "Web Standard" set has no key for the Text Editor and Edit
Guides (TW-154, runner). `xs-select-behind` has no error. Canvas image pairs within 1 %: **164 of 210**, over the 48
old units 155 of 201 (carried before: 175 of 201): `tool-autoshapes` 7 / 27 instead of 27 / 27 (the reference drew
with fill #000000 after the unit-start text reset, TW-153), `tool-text/14` 1.07 % (run 23: 0.91 %), `menu-select/06`
now below 1 % (0.90 %; run 17: 10.3 %); all other units have the same count as in their previous run. Two classes
changed: `panel-pages-states-history` unclear → real deviation (the reference merges consecutive key nudges into one
History row, TW-148) and the first classification of `xs-select-behind`, real deviation (anti-aliased hover line on
vector paths and a frame right after a click, TW-157; the hover rule itself is equal, TW-152 confirmed). `tool-text`
stays unclear (TW-146 rest, TW-145). The 13 `runner error` lines are 12 reference-side text entries in `tool-text`
(`11`: 3, `12`: 3, `13`: 4, `17`: 2; TW-146 rest) and one real Down key in `inspector-stroke/01` that selected the Line
tool before the substitute (TW-155).

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

The next steps are in [ROADMAP.md](../ROADMAP.md): release v0.2.3 (being published on 19 Oct 2026 from the state
measured here), then a fix package for the two real deviations (merge consecutive key nudges into one History row,
TW-148; the Select Behind hover line and the frame after a click, TW-157) and the runner items TW-153 to TW-156, a
follow-up run over `panel-pages-states-history`, `xs-select-behind`, `tool-text`, `tool-autoshapes`, `toolbox`,
`menu-edit`, `menu-text` and `menu-view`, the human click-test, the size rule for new text (TW-145) and the *Text*
menu remnant of the text tool defaults (TW-138 rest).
