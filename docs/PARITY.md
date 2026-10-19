# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 23, 19 Oct 2026, 5 units; 2 units from run 22, 4 from run 18, the other 37 from full run 17):**
**46 equal except for design**, 0 real deviations, 2 unclear, 0 under-measured. **Proven 1:1: 95.8 %** (46 / 48; all
48 units are sufficiently measured). Measurement quota over all 48 units: **86.9 %** (1065 / 1226 steps, newest run
per unit). Canvas image pairs within 1 %: **175 / 201**.

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
measured units. The headline figure is the strict one over all 48 units (95.8 %); since run 15 no unit is
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

## Per-unit result (runs 17 to 23)

Five units come from follow-up run 23 (19 Oct 2026, 01:34–02:10, 5 units, 123 steps, 49 image pairs), preview build
`2026-10-19_01-32` from `main` at `cd86ce75` (package F46 after RE spec 51: Escape keeps the selection and only
commits text, cancels crop or transform or leaves bitmap mode; one History row per arrow-key nudge, no merging;
runner R-20 at `b1d34247`, which on the reference side polls for the text editor before typing, types as keys and
checks the commit (`orig text entry` / `orig text commit` lines), verifies a drag selection by its highlight pixels
instead of the clipboard, picks the Pointer tool from the flyout when the slot holds another tool, keeps the
selection on Escape (`orig.escape_deselects=false`), nudges with real Shift+arrows in
`panel-pages-states-history/21`, flags History row counts that contain script substitutes as
`history_not_comparable`, waits for the paste in `menu-edit/03` and reads `MaxUndos`). For these five rows "was"
compares with run 22. Two more units (`panel-library-styles`, `xs-canvas`) come from follow-up run 22 (18/19 Oct 2026, 23:27–00:10, 7 units, 170 steps, 77 image pairs), preview
build `2026-10-18_23-25` from `main` at `a4c2bf2b` (package F45: element-list read-backs see the live text element
while editing, TW-143, on top of `65db4012`; runner R-19 at `7bb38c49`, which replaces plain Delete, arrow and
Escape keys on the reference side by Edit ▸ Clear or `deleteSelection`, `moveSelectionBy` and a Pointer tool click,
selects text by a mouse drag with a clipboard read-back, commits the crop by a double-click and resets the text
colour at unit start; every replaced action is logged as `orig key substitute` and carries `substituted` in the step
record). For those rows "was" compares with run 21. The other figures below come from the earlier runs:
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
| `menu-edit` | menus | equal except for design | run 23: 91.7 % (22 / 24), unchanged; `03`: R-20's paste check reads one selected object at once and both programs read R1 at 50,50–150,120 after Ctrl+X, Ctrl+V (empty on the reference side in runs 20 and 22; TW-147 d closed); `09` and `22` as in run 22 (Edit ▸ Clear substitute deletes R1; both Histories list Clone, Copy, Paste, Clear, the reference with the runner's select script row first, 5 vs. 4 steps); `16` (Crop Document grey on the reference side) and `17_warnings_no_target_open_crop` (both sides) fail as before |
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
| `panel-pages-states-history` | panels | unclear | run 23: 75.0 % (18 / 24), unchanged; was equal except for design. `21`: the reference now nudges R1 with real Shift+arrows (50,50 → 60,50 → 70,50 → 70,60, positions equal) and reads **4** History steps, Firepixel **5** (TW-148). Firepixel's five are the three nudge rows (one per press since F46), a Deselect row from the runner's select of E1 and the Shift+Right on E1; the reference's runner selects are one script row each (as `menu-edit/22` shows), so its four leave one row for the three nudges, while RE spec 51 expects one row per press (6 steps); the row list is not read back in `21`, so the count is not conclusive. `14`, `16`, `17`, `19`, `20` carry `history_not_comparable` (plain-arrow substitutes are script rows); `16` leaves R2 at left 200 in both programs. `22`: the read-back now uses `MaxUndos`, 20 in both (TW-147 e closed). Menus and errors unchanged (reference: onion-skin and loop pop-ups not found; Firepixel: `18` Save as Command opens no dialog the bridge finds) |
| `preview-options` | preview / export | equal except for design | run 17: 76.9 % (30 / 39), unchanged from run 12 |
| `preview-views` | preview / export | equal except for design | run 17: 69.6 % (16 / 23), unchanged from run 10 |
| `color-popup-contexts` | colour | equal except for design | run 17: 68.2 % (15 / 22), unchanged from run 12 |
| `tool-path-reshape` | tools | equal except for design | run 17: 100.0 % (23 / 23), unchanged from run 10; 12 / 13 canvas pairs within 1 % |
| `tool-text` | tools | unclear | run 23: 81.5 % (22 / 27), unchanged; 13 / 15 canvas pairs within 1 %, unchanged, but the reference's texts now exist: R-20 creates 14 of 17 new texts as typed (15 commits `ok`), `02` reads the full "Abc 123 Abc 123 Abc 123 Abc 123" (0.34 %), `03` 0.10 %, and the drag selections of "Welt" (`07`, 0.16 %) and "123" (`11`) are verified by highlight pixels (24 px, cover 1.0) without inserting characters. Still on the reference side (TW-146 rest): in `09`, `13` and `20` the poll reports an editor but the keys reach the tool shortcuts (Crop or Subselection afterwards), so the text is discarded at the commit (01:37:51, 01:39:01, 01:42:02; `13` then finds nothing to select and no colour pop-up, reference error as before), and the "Q" after the pointer double-click in `06` does not arrive. `11`: the size edit to 72 pt does not reach the selected "123" (the shot shows the caret before "123", no highlight, all 12 pt), so TW-145 is still not compared; the reference's next texts in `14` and `16` are 12 pt, Firepixel's 72 pt (`14` 0.92 %, `16` 3.40 %). Firepixel's new texts use Myriad Pro, the reference's ArialMT after the unit-start reset (TW-149). `12`: Firepixel's font field is not found (as before) |
| `tool-web` | tools | equal except for design | run 17: 96.4 % (27 / 28), unchanged from run 10; 14 / 16 canvas pairs within 1 % |
| `toolbox` | tools | equal except for design | run 17: 92.9 % (52 / 56), was 80.4 % in run 13; 0 / 3 canvas pairs within 1 %; seven formerly failing steps now `ok` on both sides (hand, zoom, colour section, collapse) |
| `tool-autoshapes` | tools | equal except for design | run 17: 100.0 % (40 / 40), unchanged from run 15; 27 / 27 canvas pairs within 1 % |
| `export-results` | preview / export | equal except for design | run 17: 80.0 % (8 / 10), was 72.7 % in run 16; 4 / 4 canvas pairs within 1 %; the animated GIF is again **byte-identical** (SHA-1 equal) and PNG 8/24/32 pixel-identical (4 of 4 pairs within 1 %); `13_export_area` is now `unsupported` on the reference side (R-12), Firepixel `ok` |
| `color-popup` | colour | equal except for design | run 17: 75.0 % (21 / 28), unchanged from run 15; 0 / 2 canvas pairs within 1 %; hex field: the reference's pop-up closes without applying the text the runner sets (#80808000), so Q-49 needs a real keyboard; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | run 17: 70.4 % (19 / 27), unchanged from run 10 |
| `xs-preferences` | cross-cutting | equal except for design | run 23: 85.0 % (17 / 20), unchanged; 2 / 3 canvas pairs within 1 % (`06` 3.15 %, Firepixel's selection handles and cursor; `07` 0.17 %, `08` 0.12 %); all values as in run 22: `06` crops the page to 280 × 220 with P1, E1, R2, R1 left in both programs; the Escape substitutes keep the selection (`15`) or close nothing (`16`) |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 17: 73.7 % (14 / 19), unchanged from run 12 |
| `xs-file-formats` | cross-cutting | equal except for design | run 17: 85.7 % (18 / 21), unchanged from run 13; 4 / 4 canvas pairs within 1 %; import by dragging equal (TW-101) |
| `inspector-group-symbol` | property inspector | equal except for design | run 17: 90.0 % (18 / 20), unchanged from run 16; 1 / 1 canvas pairs within 1 %; steps 05–14 all `ok` on both sides with equal values |
| `panel-swatches-mixer` | panels | equal except for design | run 18: 96.3 % (26 / 27), was 92.6 % in run 17; the "add swatches" file dialog now opens on the reference side (TW-129 closed); Swatches menu 229 vs. 226 px |
| `xs-shortcuts` | cross-cutting | equal except for design | run 23: 88.0 % (22 / 25), was 92.0 %: `08_tools` is `slow` on the reference side (94.1 s against the 90 s step limit; run 22: 73.6 s) because about 20 Pointer tool clicks follow the tool letters, one through the flyout (TW-150); its values are as before. `07`: R-20 finds Select Behind in the Pointer slot and takes the Pointer tool from the flyout, no runner error (13 in run 22; TW-147 a closed); `21`: after the Escape chain R1 stays selected in both programs (1 / 1; F46 and `orig.escape_deselects=false`, TW-147 b closed); `10_nudge`, `10_nudge_point` and `15_in_text_tx2` equal as in run 22 |
| `xs-canvas` | cross-cutting | equal except for design | run 22: 91.7 % (22 / 24), unchanged; 1 / 8 canvas pairs within 1 % (same values as runs 18 to 21); `15_snap_pixels_disabled`: Snap to Pixel reads `enabled: false` in both programs |

Totals: 46 / 0 / 2 / 0. Step level over all 48 units, newest run per unit: 1065 of 1226 measurable steps `ok` on
both sides (**86.9 %**; run 22: 1066; run 17 alone: 86.3 %). In the five units of run 23: 101 of 120 (84.2 %; the
same units in run 22: 102 of 120); `xs-shortcuts` lost `08_tools` (`slow`, −4.0 points), the other four are
unchanged. Reference-side errors in the five units: 7 (unchanged) plus the `slow` step; Firepixel-side errors: 5
(unchanged, all known); over all 48 units 38 reference-side errors plus one `slow` step and 21 Firepixel-side errors.
Canvas image pairs within 1 %: **175 of 201** (unchanged; `tool-text/02`, `03`, `07` and `14` now compare against
the reference's typed texts instead of blank shots). One class changed: `panel-pages-states-history` equal except
for design → unclear (the History step count after three nudges differs, 4 vs. 5, TW-148); `tool-text` stays unclear
because the reference's size edit on the partial selection did not take effect, so TW-145 is still not compared. The
9 `runner error` lines (run 22: 15) are all in `tool-text` and all reference-side text entry (`06`: 1, `09`: 2,
`13`: 4, `20`: 2; TW-146 rest); `xs-shortcuts/07` has none any more.

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
| 23 (5 units) | 19 Oct 2026 | 46 of 48 (0 under-measured) | **95.8 %** (46 / 48); step level 86.9 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): read the History rows of `panel-pages-states-history/21` and check
the nudge rows in the reference (TW-148, RE spec check), runner R-21 for the rest of the reference-side text entry and
the size edit on a partial selection (TW-146 rest, then TW-145), the text tool font at unit start (TW-149) and the
step limit of `xs-shortcuts/08` (TW-150); the *Text* menu remnant of the text tool defaults (TW-138 rest); release
and the human click-test.
