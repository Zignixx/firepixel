# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 18, 18 Oct 2026, 10 units; the other 38 units from full run 17):** **43 equal except for
design**, 2 real deviations, 3 unclear, 0 under-measured. **Proven 1:1: 89.6 %** (43 / 48; all 48 units are
sufficiently measured). Measurement quota over all 48 units: **86.9 %** (1066 / 1226 steps, newest run per unit).
Canvas image pairs within 1 %: **174 / 202**.

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
measured units. The headline figure is the strict one over all 48 units (89.6 %); since run 15 no unit is
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

## Per-unit result (runs 17 and 18)

Ten units come from follow-up run 18 (18 Oct 2026, 11:00–11:54, 10 units, 252 steps, 126 image pairs), preview
build `2026-10-18_10-59` from `main` at `4e677046` (packages F33 and F34: per-slice export options TW-128, short
panel menus TW-125, filter shortcuts TW-126, the text-tool caret TW-130; runner R-13/R-13b with the "Web Standard"
shortcut set in Firepixel's profile, the set the reference machine uses, defined option start states, dialog
retries and keys sent with scan codes). For these ten rows "was" compares with run 17. All other units' state comes
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
| `menu-edit` | menus | unclear | run 18: 91.7 % (22 / 24), unchanged; `02` now equal; Firepixel's history now starts with Clone (Ctrl+Shift+D under "Web Standard"), but the reference's history read-back in `01` and `22` fails with a script error, so the step names cannot be compared; `09` (Delete) and `20` (Ctrl+D) leave the reference's document unchanged as in earlier runs (reference-side keys, TW-132) |
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
| `panel-library-styles` | panels | **real deviation** | run 18: 93.3 % (14 / 15), unchanged; 1 / 1 canvas pairs within 1 %; the library drop now arrives on both sides (5 elements; 571,371 vs. 570,370 at the reference's zoom 1.00125); Common Library menu 183 vs. 179 px (TW-125 closed); the URL panel menu has only "Help" in Firepixel, the reference has nine entries (add used URLs to the library, delete unused URLs, add / edit / delete URL, new URL library, import, export, Help; TW-131, already so in run 17) |
| `panel-pages-states-history` | panels | equal except for design | run 17: 75.0 % (18 / 24), unchanged from run 13; Pages and History menus within 5 px of the reference; States menu 231 vs. 207 px (minimum width, TW-125) |
| `preview-options` | preview / export | equal except for design | run 17: 76.9 % (30 / 39), unchanged from run 12 |
| `preview-views` | preview / export | equal except for design | run 17: 69.6 % (16 / 23), unchanged from run 10 |
| `color-popup-contexts` | colour | equal except for design | run 17: 68.2 % (15 / 22), unchanged from run 12 |
| `tool-path-reshape` | tools | equal except for design | run 17: 100.0 % (23 / 23), unchanged from run 10; 12 / 13 canvas pairs within 1 % |
| `tool-text` | tools | unclear | run 18: 85.2 % (23 / 27), unchanged; 12 / 16 canvas pairs within 1 % (was 14 / 16); the reference's text tool carries state from earlier units (large italic underlined text, a stray "c" after the typed text, text left in edit mode), so `02` (1.8 %) and `14` (1.2 %) move above 1 % (TW-132); `06`: after a pointer double-click inside "Zeile eins" the reference inserts "Q" at the click point ("ZeilQe eins"), Firepixel shows the caret after "Zeile " and no "Q" (same in run 17, TW-135); no stray "c" on Firepixel's side |
| `tool-web` | tools | equal except for design | run 17: 96.4 % (27 / 28), unchanged from run 10; 14 / 16 canvas pairs within 1 % |
| `toolbox` | tools | equal except for design | run 17: 92.9 % (52 / 56), was 80.4 % in run 13; 0 / 3 canvas pairs within 1 %; seven formerly failing steps now `ok` on both sides (hand, zoom, colour section, collapse) |
| `tool-autoshapes` | tools | equal except for design | run 17: 100.0 % (40 / 40), unchanged from run 15; 27 / 27 canvas pairs within 1 % |
| `export-results` | preview / export | equal except for design | run 17: 80.0 % (8 / 10), was 72.7 % in run 16; 4 / 4 canvas pairs within 1 %; the animated GIF is again **byte-identical** (SHA-1 equal) and PNG 8/24/32 pixel-identical (4 of 4 pairs within 1 %); `13_export_area` is now `unsupported` on the reference side (R-12), Firepixel `ok` |
| `color-popup` | colour | equal except for design | run 17: 75.0 % (21 / 28), unchanged from run 15; 0 / 2 canvas pairs within 1 %; hex field: the reference's pop-up closes without applying the text the runner sets (#80808000), so Q-49 needs a real keyboard; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | run 17: 70.4 % (19 / 27), unchanged from run 10 |
| `xs-preferences` | cross-cutting | unclear | run 18: 85.0 % (17 / 20), unchanged; 2 / 3 canvas pairs within 1 %; start states now set and read back (TW-127 closed); `15` equal (stroke 2 on both sides); `06`: with "delete paths when cropping" read back as on in both programs the reference still keeps all seven paths, Firepixel removes the three outside (44 % canvas pair; TW-134) |
| `xs-windows-tabs` | cross-cutting | equal except for design | run 17: 73.7 % (14 / 19), unchanged from run 12 |
| `xs-file-formats` | cross-cutting | equal except for design | run 17: 85.7 % (18 / 21), unchanged from run 13; 4 / 4 canvas pairs within 1 %; import by dragging equal (TW-101) |
| `inspector-group-symbol` | property inspector | equal except for design | run 17: 90.0 % (18 / 20), unchanged from run 16; 1 / 1 canvas pairs within 1 %; steps 05–14 all `ok` on both sides with equal values |
| `panel-swatches-mixer` | panels | equal except for design | run 18: 96.3 % (26 / 27), was 92.6 % in run 17; the "add swatches" file dialog now opens on the reference side (TW-129 closed); Swatches menu 229 vs. 226 px |
| `xs-shortcuts` | cross-cutting | **real deviation** | run 18: 88.0 % (22 / 25), unchanged; both programs now use "Web Standard"; `06` still differs on four shortcuts with a selected text: Ctrl+Shift+B opens a warning in Firepixel (reference: nothing), Ctrl+Shift+I drops Firepixel's selection, Ctrl+Alt+Shift+S sets "stretch" alignment, Ctrl+Shift+Period grows 24 to 36 pt instead of 25 pt (TW-133); Ctrl+Shift+U and Ctrl+Shift+E now equal; `10_nudge`: the reference does not move R1 on the plain arrow keys (Shift+arrows move 10 px on both sides; TW-132); `15` `unsupported` (invalid TX1 in the reference file); `15_in_text_tx2`: Firepixel inserts "Vr" at the click point in line 2, the reference's read-back shows no typed text (TW-130, Q-57) |
| `xs-canvas` | cross-cutting | equal except for design | run 18: 91.7 % (22 / 24), was 87.0 % in run 17; 1 / 8 canvas pairs within 1 % (unchanged; fit zoom and grid phase as reviewed in run 17); `15` with R1 selected equal; `15_snap_pixels_disabled`: grey in the reference, Firepixel's menu read-back reports no enabled state (runner, TW-132) |

Totals: 43 / 2 / 3 / 0. Step level over all 48 units, newest run per unit: 1066 of 1226 measurable steps `ok` on
both sides (**86.9 %**; run 17 alone: 86.3 %). In the ten units of run 18: 214 of 237 (90.3 %; the same units in run
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
Library menu at 183 vs. 179 px (the States menu was not part of run 18).

**What run 17 found.** Most new differences come from one runner change: since R-12 the runner writes the
"Firepixel" shortcut set into Firepixel's profile, and in that set Ctrl+Shift+D is not Clone, Ctrl+Shift+Period
does not enlarge text, and Ctrl+Shift+E, Ctrl+Alt+Shift+S and Ctrl+Shift+I do other things than on the reference
machine (TW-126; the reference machine's active set needs a behaviour description). `panel-optimize` shows one real
deviation: export formats chosen per slice in the Optimize panel are not stored on the slices (TW-128).
`xs-preferences` clicks stored settings without a fixed start state (TW-127, runner).

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
| 18 (10 units) | 18 Oct 2026 | 43 of 48 (0 under-measured) | **89.6 %** (43 / 48); step level 86.9 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): the URL panel menu (TW-131), behaviour descriptions of the four
text shortcuts under "Web Standard" (TW-133), of "delete paths when cropping" (TW-134) and of typing after a
pointer double-click on a text (TW-135), runner R-14 for the reference-side effects (TW-132) and the text-tool
read-back (TW-130, Q-57), comparison run 19 for the affected units, then the human click-test.
