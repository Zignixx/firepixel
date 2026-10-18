# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 22, 18/19 Oct 2026, 7 units; 4 units from run 18, the other 37 from full run 17):** **47 equal
except for design**, 0 real deviations, 1 unclear, 0 under-measured. **Proven 1:1: 97.9 %** (47 / 48; all 48 units
are sufficiently measured). Measurement quota over all 48 units: **86.9 %** (1066 / 1226 steps, newest run per unit).
Canvas image pairs within 1 %: **175 / 201**.

## Method: scripted comparison runs

The parity runner (`tools/parity/runner/`, see its README) plays the same step script in **both programs** on
one Windows machine, unattended:

1. **Units.** The UI is cut into 48 units (one tool group, one panel, one menu, ten dialogs …). Each unit has a
   JSON script in `tools/parity/scripts/<unit>.json` with about 20–40 steps (1282 steps in total) and a test
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

Full runs over all 48 units take about four hours; follow-up runs repeat only the affected units. A run's
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
measured units. The headline figure is the strict one over all 48 units (97.9 %); since run 15 no unit is
under-measured, so both figures are the same.

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

## Per-unit result (runs 17 to 22)

Seven units come from follow-up run 22 (18/19 Oct 2026, 23:27–00:10, 7 units, 170 steps, 77 image pairs), preview
build `2026-10-18_23-25` from `main` at `a4c2bf2b` (package F45: element-list read-backs see the live text element
while editing, TW-143, on top of `65db4012`; runner R-19 at `7bb38c49`, which replaces plain Delete, arrow and
Escape keys on the reference side by Edit ▸ Clear or `deleteSelection`, `moveSelectionBy` and a Pointer tool click,
selects text by a mouse drag with a clipboard read-back, commits the crop by a double-click and resets the text
colour at unit start; every replaced action is logged as `orig key substitute` and carries `substituted` in the step
record). For these seven rows "was" compares with run 21. The other figures below come from the earlier runs:
follow-up run 21 (18 Oct 2026, 22:06–22:51, 7 units, 170 steps, 78 image pairs), preview build
`2026-10-18_22-05` from `main` at `65db4012` (packages F42–F44: panel pop-up menu state TW-142, live bounds TW-143,
alignment and text defaults TW-133 / TW-138, paragraph separator tests TW-144; runner R-17 / R-17b / R-18 with
`key echo`, `key layout`, `key probe` and `tool after key` lines, a Properties-panel text reset at unit start, a
`crop` action with page-size log, CR/LF-preserving text read-backs and Firepixel's menu entries from accessibility
nodes). The other figures below come from the earlier runs:
follow-up run 20 (18 Oct 2026, 15:30–16:09, 7 units, 170 steps, 78 image pairs), preview build
`2026-10-18_15-28` from `main` at `054bdc05` (packages F37–F40: crop option TW-134, live text read-back TW-139,
History rows TW-137 and text tool defaults TW-138, menu enabled state TW-141; runner R-15 with an explicit
extended-key table, a keyboard layout request for the reference and `key_echo`; the `key_echo` self-test failed in
this run with a script error that R-16 fixes, and the log has no layout or tool-switch lines). For these seven rows
"was" compares with run 19 (run 18 for `xs-preferences`). The other figures below come from the earlier runs:
follow-up run 19 (18 Oct 2026, 13:04–13:40, 6 units, 149 steps, 66 image pairs), preview build
`2026-10-18_13-04` from `main` at `a3510515` (packages F34–F36: URL panel menu TW-131, Select Inverse warning, text
size ±1 pt and no stretch on point text TW-133, caret and typing after a pointer double-click TW-135; runner R-14
with dialogs recorded before read-back scripts, a guarded history read-back, effect checks for Delete, Ctrl+D and
the arrow keys, a text-tool reset at unit start and after typing, text read-backs before and after typing and after
Escape, and Firepixel's menu enabled state from the accessibility tree). Four units come from follow-up run 18 (18 Oct 2026, 11:00–11:54, 10 units, 252 steps, 126 image pairs), preview
build `2026-10-18_10-59` from `main` at `4e677046` (packages F33 and F34: per-slice export options TW-128, short
panel menus TW-125, filter shortcuts TW-126, the text-tool caret TW-130; runner R-13/R-13b with the "Web Standard"
shortcut set in Firepixel's profile, the set the reference machine uses, defined option start states, dialog
retries and keys sent with scan codes). For those rows "was" compares with run 17. All other units' state comes
from run 17 (18 Oct 2026, 04:59–09:40), the first full run since run 13: 48 units,
1284 steps, 683 image pairs, preview build `2026-10-18_04-57` from `main` at `7e3d2d12` with the fixes made after
run 16 (packages F29–F32, runner R-10 to R-12 with the Firepixel shortcut set in the runner profile, menus with the
33 px arrow column, swatches rework, Shift-drag and inspector rules). Another application's window was open and
builds ran in parallel, so timings are noisy; the reference program was restarted twice after a script access
violation and three times as planned. "Was" compares with the unit's previous newest run.

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | run 17: 92.9 % (26 / 28), unchanged from run 10 |
| `dialogs-d11-d20` | dialogs | equal except for design | run 17: 88.9 % (32 / 36), unchanged from run 10 |
| `dialogs-d21-d30` | dialogs | equal except for design | run 17: 100.0 % (27 / 27), unchanged from run 12; text runs fixed (TW-02) |
| `dialogs-d31-d40` | dialogs | equal except for design | run 18: 92.0 % (23 / 25), was 68.0 % in run 17; 2 / 2 canvas pairs within 1 %; Feather, Expand, Save and Restore Bitmap Selection now open on the reference side after the marquee drag (TW-129 closed); `06_d33` equal (twelve I1 on both sides, Ctrl+Shift+D clones, TW-126 closed); `11` and `16` fail on both sides as before (the dialog is closed before the read-back); history read-backs differ only by the runner's own script steps in the reference's history |
| `dialogs-d41-d50` | dialogs | equal except for design | run 17: 91.3 % (21 / 23), unchanged from run 10 |
| `filters-adjust-color` | filters | equal except for design | run 17: 69.6 % (16 / 23), unchanged from run 10; 4 / 4 canvas pairs within 1 % |
| `filters-blur-sharpen-noise` | filters | equal except for design | run 17: 79.3 % (23 / 29), unchanged from run 10; 10 / 11 canvas pairs within 1 % |
| `fx-bevel-emboss-fill` | live effects | equal except for design | run 17: 90.5 % (19 / 21), unchanged from run 10; 2 / 4 canvas pairs within 1 % |
| `fx-shadow-glow` | live effects | equal except for design | run 17: 100.0 % (23 / 23), unchanged from run 10; 5 / 6 canvas pairs within 1 % |
| `fx-stack` | live effects | equal except for design | run 17: 92.0 % (23 / 25), unchanged from run 10; 2 / 2 canvas pairs within 1 % |
| `inspector-bitmap` | property inspector | equal except for design | run 17: 100.0 % (15 / 15), unchanged from run 10; 3 / 3 canvas pairs within 1 % |
| `inspector-fields` | property inspector | equal except for design | run 17: 86.7 % (26 / 30), was 60.0 % in run 11; 1 / 1 canvas pairs within 1 %; the ten reference-side field errors of run 11 are gone; one error per side remains (`10_edge_popup`, `20_undo_each`) |
| `inspector-stroke` | property inspector | equal except for design | run 17: 88.2 % (45 / 51), unchanged from run 10; 31 / 31 canvas pairs within 1 % |
| `inspector-text` | property inspector | equal except for design | run 17: 90.0 % (27 / 30), unchanged from run 10; 10 / 11 canvas pairs within 1 % |
| `inspector-web` | property inspector | equal except for design | run 17: 81.2 % (13 / 16), unchanged from run 10 |
| `menu-commands-window-help` | menus | equal except for design | run 17: 75.0 % (15 / 20), unchanged from run 10 |
| `menu-edit` | menus | equal except for design | run 22: 91.7 % (22 / 24), unchanged; `09`: R-19's Edit ▸ Clear substitute deletes R1 on the reference side (7 → 6 objects; the remaining six with equal bounds in both programs; TW-140 closed); `22`: both Histories list Clone, Copy, Paste and Clear (the reference adds the runner's script row first); `03`: the reference's selection read-back after Ctrl+X, Ctrl+V is empty again (R1 in run 21, empty in run 20; runner timing, TW-147); `16` (Crop Document grey on the reference side) and `17_warnings_no_target_open_crop` (both sides) fail as before |
| `menu-file` | menus | equal except for design | run 17: 81.8 % (18 / 22), unchanged from run 13; File menu with Share submenu 637 vs. 613 px (Firepixel's longer own item "Browse in file browser…"); arrow and shortcut gaps equal |
| `menu-modify-a` | menus | equal except for design | run 18: 96.2 % (25 / 26), unchanged; 4 / 6 canvas pairs within 1 %; `11_symbol_tween` equal (twelve I1 on both sides, TW-126 closed); `24`: Group as Mask is grey in the reference (unchanged) |
| `menu-modify-b` | menus | equal except for design | run 17: 92.0 % (23 / 25), unchanged from run 10; 5 / 5 canvas pairs within 1 % |
| `menu-select` | menus | equal except for design | run 17: 100.0 % (16 / 16), unchanged from run 10; 5 / 6 canvas pairs within 1 % |
| `menu-text` | menus | equal except for design | run 17: 87.0 % (20 / 23), unchanged from run 12; 4 / 4 canvas pairs within 1 % |
| `menu-view` | menus | equal except for design | run 17: 92.3 % (24 / 26), unchanged from run 10; 6 / 6 canvas pairs within 1 % |
| `panel-info-align-path` | panels | equal except for design | run 17: 83.3 % (30 / 36), unchanged from run 11; 2 / 2 canvas pairs within 1 %; rewritten `for`-loop read-backs equal; the reference's undo step-count read-back is unchanged (runner) |
| `panel-layers` | panels | equal except for design | run 17: 74.1 % (20 / 27), unchanged from run 14; rewritten read-backs (visibility, lock, order, element counts) equal on both sides; locator gaps in the scrolled panel unchanged |
| `panel-layers-menu` | panels | equal except for design | run 17: 81.0 % (17 / 21), was 76.2 % in run 10; rewritten read-backs equal; `11_share` now equal (Gemeinsam and Webebene shared on both sides) and `15_merge_down` measured; Firepixel's `10_options_open` still opens no dialog the bridge finds; panel menu 362 vs. 354 px (frame included) |
| `panel-misc` | panels | equal except for design | run 17: 87.5 % (14 / 16), unchanged from run 16; embedded panels `unsupported`; panel menus now read from the pop-up (R-12) |
| `panel-optimize` | panels | equal except for design | run 18: 83.3 % (20 / 24), unchanged; `19` equal: both slices report JPEG and GIF as export format (TW-128 closed); Optimize menu 289 vs. 284 px |
| `panel-library-styles` | panels | equal except for design | run 22: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 % (`03` 0.87 %); all values as in run 21 (`16`: nine URL menu entries with equal labels, `ellipsis` and `enabled`); shot 327 × 248 vs. 321 × 231 px (≡ row and shadow included, TW-136) |
| `panel-pages-states-history` | panels | equal except for design | run 22: 75.0 % (18 / 24), unchanged; in `14–21` R-19's `moveSelectionBy` substitutes move R1 by 1 px per arrow on the reference side (50,50 → 52,51 in every step; TW-140 closed), and `16` leaves R2 at left 200 in both programs; the History read-backs still differ because every substitute is a script row of its own in the reference's History (`16`: 7 vs. 4 rows; `21`: 6 vs. 3 steps, so the merging of nudges is not compared; TW-147); `14` was repeated once after another window took the foreground; menus and errors unchanged (reference: onion-skin and loop pop-ups not found; Firepixel: `18` Save as Command opens no dialog the bridge finds) |
| `preview-options` | preview / export | equal except for design | run 17: 76.9 % (30 / 39), unchanged from run 12 |
| `preview-views` | preview / export | equal except for design | run 17: 69.6 % (16 / 23), unchanged from run 10 |
| `color-popup-contexts` | colour | equal except for design | run 17: 68.2 % (15 / 22), unchanged from run 12 |
| `tool-path-reshape` | tools | equal except for design | run 17: 100.0 % (23 / 23), unchanged from run 10; 12 / 13 canvas pairs within 1 % |
| `tool-text` | tools | unclear | run 22: 81.5 % (22 / 27), was 85.2 %; 13 / 15 canvas pairs within 1 % (was 14 / 16: `13` has no reference shot); new reference error `13_color_live` (no colour pop-up; its text selection had failed); the Pointer-click substitute closes the editor in every step and no stray "c" appears (TW-140 closed), but text entry fails on the reference side in this run (TW-146): the texts created by a click in `03`, `11`, `13` and `14` stay empty and are discarded (blank reference shots; `03` 0.09 % and `14` 0.90 % only because Firepixel's texts cover few pixels), `02` reads "bc 123 Abc 123 Abc 123 Abc 123", the "Q" after the pointer double-click in `06` does not arrive (no editor open), and in `07` the drag selection reads back an empty string and the clipboard read-back's Ctrl+C inserts U+0003 into the text ("Hallo \x03Welt", wrapped to two lines); in `11` and `13` `text_select` finds no text and the size field click opens Image Size, so TW-145 is still not compared (Firepixel's texts in `14` and `16` at the 72 pt set in `11`, the reference's `16` at 12 pt); `06`: Firepixel's live bounds now read 50,150–114,194 while "ZeilQe eins" is edited (TW-143); `12`: Firepixel's font field is not found (as before) |
| `tool-web` | tools | equal except for design | run 17: 96.4 % (27 / 28), unchanged from run 10; 14 / 16 canvas pairs within 1 % |
| `toolbox` | tools | equal except for design | run 17: 92.9 % (52 / 56), was 80.4 % in run 13; 0 / 3 canvas pairs within 1 %; seven formerly failing steps now `ok` on both sides (hand, zoom, colour section, collapse) |
| `tool-autoshapes` | tools | equal except for design | run 17: 100.0 % (40 / 40), unchanged from run 15; 27 / 27 canvas pairs within 1 % |
| `export-results` | preview / export | equal except for design | run 17: 80.0 % (8 / 10), was 72.7 % in run 16; 4 / 4 canvas pairs within 1 %; the animated GIF is again **byte-identical** (SHA-1 equal) and PNG 8/24/32 pixel-identical (4 of 4 pairs within 1 %); `13_export_area` is now `unsupported` on the reference side (R-12), Firepixel `ok` |
| `color-popup` | colour | equal except for design | run 17: 75.0 % (21 / 28), unchanged from run 15; 0 / 2 canvas pairs within 1 %; hex field: the reference's pop-up closes without applying the text the runner sets (#80808000), so Q-49 needs a real keyboard; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | run 17: 70.4 % (19 / 27), unchanged from run 10 |
| `xs-preferences` | cross-cutting | equal except for design | run 22: 85.0 % (17 / 20), unchanged; 2 / 3 canvas pairs within 1 %; `06` equal: R-19's double-click commits the reference's crop, the page goes from 800 × 600 to 280 × 220 in both programs and both keep P1, E1, R2, R1 (TW-134 confirmed); the pair falls from 93.08 to 3.15 %, the rest being Firepixel's selection handles and cursor in its shot (capture method, as TW-117); Escape no longer changes the reference's tool (`07`: Pointer click; `15`, `16`: nothing to close) |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 17: 73.7 % (14 / 19), unchanged from run 12 |
| `xs-file-formats` | cross-cutting | equal except for design | run 17: 85.7 % (18 / 21), unchanged from run 13; 4 / 4 canvas pairs within 1 %; import by dragging equal (TW-101) |
| `inspector-group-symbol` | property inspector | equal except for design | run 17: 90.0 % (18 / 20), unchanged from run 16; 1 / 1 canvas pairs within 1 %; steps 05–14 all `ok` on both sides with equal values |
| `panel-swatches-mixer` | panels | equal except for design | run 18: 96.3 % (26 / 27), was 92.6 % in run 17; the "add swatches" file dialog now opens on the reference side (TW-129 closed); Swatches menu 229 vs. 226 px |
| `xs-shortcuts` | cross-cutting | equal except for design | run 22: 92.0 % (23 / 25), was 88.0 %; `15_in_text_tx2` equal: while "Vr" is typed both read-backs give TX2 50,150–120,194, and no stray "c" ("ZeileVr zwei"; TW-143 closed); `10_nudge` and `10_nudge_point` now `ok` and equal: the substitutes move R1 to 51,50 / 51,51 and the point to 201,210 as Firepixel's arrow keys do, Shift+arrows stay real keys (61,51 / 61,61; TW-140 closed); `07`: the Pointer slot of the reference's toolbox holds Select Behind, so 13 Escape substitutes select that tool and log `runner error: text editor not closed` (values equal; TW-147); `21`: after the second Escape the reference keeps R1 selected (the substitute does nothing), Firepixel deselects (runs 20 and 21: 0 in both; TW-147) |
| `xs-canvas` | cross-cutting | equal except for design | run 22: 91.7 % (22 / 24), unchanged; 1 / 8 canvas pairs within 1 % (same values as runs 18 to 21); `15_snap_pixels_disabled`: Snap to Pixel reads `enabled: false` in both programs |

Totals: 47 / 0 / 1 / 0. Step level over all 48 units, newest run per unit: 1066 of 1226 measurable steps `ok` on
both sides (**86.9 %**, unchanged; run 17 alone: 86.3 %). In the seven units of run 22: 138 of 159 (86.8 %), the
same as in runs 20 and 21; `xs-shortcuts` gained `10_nudge_point` (+4.0 points), `tool-text` lost `13_color_live`
(−3.7 points), the other five are unchanged. Reference-side errors in the seven units: 7 (`tool-text/13` new,
`xs-shortcuts/10_nudge_point` gone), Firepixel-side errors: 5 (unchanged, all known); over all 48 units still 38
reference-side and 21 Firepixel-side errors. Canvas image pairs within 1 %: **175 of 201** (run 21: 176 of 202;
`tool-text/13`, 0.70 % in run 21, has no reference shot). Two classes changed: `xs-shortcuts` real deviation →
equal except for design (the live bounds are equal, TW-143) and `xs-preferences` unclear → equal except for design
(the crop option is compared and equal, TW-134); `tool-text` stays unclear because text entry failed on the
reference side (TW-146), so the size rule TW-145 is still not compared. The 15 `runner error` lines are all runner
artefacts (13 in `xs-shortcuts/07`, 2 in `tool-text/11, 13`).

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
| 22 (7 units) | 18/19 Oct 2026 | 47 of 48 (0 under-measured) | **97.9 %** (47 / 48); step level 86.9 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): runner R-20 for reference-side text entry and selection in
`tool-text` (TW-146) and the substitute side effects (TW-147), then a run of `tool-text`, `xs-shortcuts`,
`panel-pages-states-history` and `menu-edit`, which compares the size rule for new text (TW-145); the *Text* menu
remnant of the text tool defaults (TW-138 rest); release and the human click-test.
