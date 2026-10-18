# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 16, 18 Oct 2026):** 48 units — **46 equal except for design**, 0 real deviations,
2 unclear, 0 under-measured. **Proven 1:1: 95.8 %** (46 / 48; all 48 units are sufficiently measured). Measurement
quota over all 48 units: **85.0 %** (1042 / 1226 steps). Canvas image pairs within 1 %: about **177 / 201**.

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

## Per-unit result (run 16)

State per unit from its newest run: run 10 (16/17 Oct) for 24 units, run 11 (17 Oct morning) for 3, run 12
(17 Oct afternoon) for 6, run 13 (17 Oct evening, 14 units) for 5, run 14 (17 Oct, 19:48–20:38) for 1
(`panel-layers`), run 15 (17/18 Oct, 23:54–00:38, preview build `2026-10-17_23-53`, runner R-10) for 2
(`tool-autoshapes`, `color-popup`), run 16 (18 Oct, 02:06–02:39, 7 units, 170 steps, 86 image pairs, preview build
`2026-10-18_02-04` from `main` at `788f4fa1` with package F31, runner R-11) for 7.

| Unit | Area | Class | Note |
|---|---|---|---|
| `dialogs-d01-d10` | dialogs | equal except for design | |
| `dialogs-d11-d20` | dialogs | equal except for design | |
| `dialogs-d21-d30` | dialogs | equal except for design | text runs fixed (TW-02) |
| `dialogs-d31-d40` | dialogs | equal except for design | |
| `dialogs-d41-d50` | dialogs | equal except for design | |
| `filters-adjust-color` | filters | equal except for design | |
| `filters-blur-sharpen-noise` | filters | equal except for design | |
| `fx-bevel-emboss-fill` | live effects | equal except for design | |
| `fx-shadow-glow` | live effects | equal except for design | |
| `fx-stack` | live effects | equal except for design | |
| `inspector-bitmap` | property inspector | equal except for design | |
| `inspector-fields` | property inspector | equal except for design | |
| `inspector-stroke` | property inspector | equal except for design | |
| `inspector-text` | property inspector | equal except for design | |
| `inspector-web` | property inspector | equal except for design | |
| `menu-commands-window-help` | menus | equal except for design | |
| `menu-edit` | menus | equal except for design | |
| `menu-file` | menus | equal except for design | run 13 |
| `menu-modify-a` | menus | equal except for design | |
| `menu-modify-b` | menus | equal except for design | |
| `menu-select` | menus | equal except for design | |
| `menu-text` | menus | equal except for design | |
| `menu-view` | menus | equal except for design | |
| `panel-info-align-path` | panels | equal except for design | |
| `panel-layers` | panels | equal except for design | drag onto a locked layer fixed (TW-86); run 14 unchanged (74.1 % measured, locator gaps in the scrolled panel); not part of run 15 |
| `panel-layers-menu` | panels | equal except for design | |
| `panel-misc` | panels | equal except for design | embedded panels `unsupported`; run 16 unchanged from run 13: 87.5 % (14 / 16), no error on either side; the undo step's step-count read-back returns an error code in the reference (the step is `unsupported` there anyway); panel menu texts are read from the panel instead of the pop-up on Firepixel's side (runner, TW-124) |
| `panel-optimize` | panels | equal except for design | run 16: 79.2 % (19 / 24, as in run 12); `slice:N` now selects the slices on both sides and Firepixel's step 19 is `ok`; the reference fails only in the runner's read-back script (`Array.map` is missing in its script engine, TW-124); the colour-table context menu opens about 600 px wide in Firepixel (reference about 270 px, TW-123) |
| `panel-library-styles` | panels | equal except for design | run 13: equal at an integer drop point; half-pixel rounding still open (TW-100) |
| `panel-pages-states-history` | panels | equal except for design | run 13: replay offset came from script steps in the original's history (TW-106) |
| `preview-options` | preview / export | equal except for design | |
| `preview-views` | preview / export | equal except for design | |
| `color-popup-contexts` | colour | equal except for design | |
| `tool-path-reshape` | tools | equal except for design | |
| `tool-text` | tools | equal except for design | |
| `tool-web` | tools | equal except for design | |
| `toolbox` | tools | equal except for design | run 13 |
| `tool-autoshapes` | tools | equal except for design | run 15: 100 % measured (was 95.0 %); the size handle with modifiers and the spiral's `center` handle are now reached on both sides (TW-116 closed); 27 of 31 pairs within 1 % |
| `export-results` | preview / export | equal except for design | run 16: 72.7 % (was 66.7 %); animated GIF and PNG 8/24/32 stay pixel-identical (4 of 4 pairs within 1 %); `12_selected_slices` is now `unsupported` on both sides (`slice:0` is selected, but the runner's export verb has no "selected slices only"); `13_export_area`: `if_no_dialog` fell back to the double-click in the reference and still no preview opened (reference side only, Firepixel `ok`) |
| `color-popup` | colour | equal except for design | run 15: 75.0 % (was 71.4 %); drag black → red now gives #ff0000 on both sides (TW-112 closed); hex field question Q-49 still open; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | |
| `xs-preferences` | cross-cutting | equal except for design | |
| `xs-windows-tabs` | cross-cutting | equal except for design | |
| `xs-file-formats` | cross-cutting | equal except for design | run 13: import by dragging lands on the same pixels 121,120–417,322 (TW-101 fixed) |
| `inspector-group-symbol` | property inspector | equal except for design | run 16: 90.0 % (18 / 20, unchanged; the two missing steps are unmapped animation fields); with several objects selected both programs now show the fill and stroke of the topmost object (TW-120 closed: R + TX fill black; E1 + R1 red 1 px stroke) and the button instance shows symbol name, disabled text field, "PNG32" and "Add filter…" on both sides (TW-121 closed); 1 of 1 canvas pair within 1 % |
| `panel-swatches-mixer` | panels | equal except for design | run 16: 96.3 % (26 / 27, unchanged); the screenshots show equal swatch sets: after the reset both start with the 21-hue colour-cube ramp (#ffbfbf, #ffcfbf, #ffdfbf …), after "replace" + "sort by colour" both show the identical first row (#ffff00, #f0dcb4, #00ffff, #00ff00, #ff8000, #4080c0, …, #ffffff, #808080) followed by black, in 9 px cells (TW-113 and Q-52 closed); Firepixel's wider panel shows 27 columns instead of 21 (design); the probe and read-back colours still differ (#efffbf vs. #cfffbf) only because the runner reads Firepixel's panel at the old 12 px pitch (TW-115) |
| `xs-shortcuts` | cross-cutting | unclear | run 16: 88.0 % (was 84.0 %); the Shift-drag of the selected R1 now leaves it at 50,50–150,120 in both programs (TW-118 and Q-53 closed), Alt-drag still 8 vs. 8; step 06 is measured for the first time and differs: with the text object selected, Ctrl+Shift+B opens a warning in Firepixel and Ctrl+Shift+I / Ctrl+Shift+U change the selection, while the original keeps the text selected (TW-122, Q-54); `15_in_text` gives "Vrc" vs. "Vr" (unchanged since run 15, Q-55) |
| `xs-canvas` | cross-cutting | unclear | run 16: 82.6 % (was 79.2 %); `12_grid_snap` is equal (12,20–2412,1620 on both sides, TW-119 a closed); the Ctrl+wheel point is now taken inside each view (`view_frac`) and the zoom-centre pair drops from 6.3 % to 1.6 % (TW-119 b closed); no value differs any more, but 7 of 8 canvas pairs stay between 1.6 % and 12.7 % at the fractional fit zoom (page phase and view size), so the unit stays unclear until one pair review; reference-side errors: the guide double-click opens no dialog (`07`) and Modify ▸ Snap to Pixel is disabled without a selection (`15`, Q-56); `10_guides_remove` is now `unsupported` on both sides (the drag end on the ruler lies outside the view, TW-124) |

Totals: 46 / 0 / 2 / 0. Step level over all 48 units (newest run per unit): 1042 of 1226 measurable steps `ok` on
both sides (**85.0 %**, was 84.8 %); in the 7 units of run 16: 126 of 146 (86.3 %; the same units in their previous
newest runs: 125 of 148, 84.5 %). Reference-side errors in run 16: 5 (the same units before: 8); Firepixel-side
errors: 0 (was 3). Canvas image pairs within 1 % over all 48 units: about 177 of 201, unchanged (run 16 units: 6 of
13, as before; the zoom-centre pair improved from 6.3 % to 1.6 %). `inspector-group-symbol` and
`panel-swatches-mixer` moved to "equal"; `xs-shortcuts` and `xs-canvas` stay unclear. The runner's new foreground
retry recovered twice from another application taking the foreground (`inspector-group-symbol`, before step 13).

No real difference in results remains known from the runs. Still unclear: text style shortcuts on a selected text
object (TW-122, Q-54) and the remaining `xs-canvas` canvas pairs; small UI deviation: the width of the Optimize
colour-table context menu (TW-123). See [FINDINGS.md](FINDINGS.md).

## History of the headline figure

| Run | Date | Equal except for design | Proven 1:1 |
|---|---|---|---|
| 10 (full) | 16/17 Oct 2026 | 29 of 48 (9 under-measured) | 74 % (29 / 39) |
| 11 (23 units) | 17 Oct 2026 | 31 of 48 (9 under-measured) | 79 % (31 / 39) |
| 12 (20 units) | 17 Oct 2026 | 39 of 48 (0 under-measured) | 81 % (39 / 48) |
| 13 (14 units) | 17 Oct 2026 | 42 of 48 (2 under-measured) | 88 % (42 / 48; 91 % = 42 / 46 of the sufficiently measured units) |
| 14 (8 units) | 17 Oct 2026 | 42 of 48 (1 under-measured) | 87.5 % (42 / 48; 89.4 % = 42 / 47 of the sufficiently measured units); step level 84.1 % |
| 15 (7 units) | 17/18 Oct 2026 | 44 of 48 (0 under-measured) | 91.7 % (44 / 48); step level 84.8 % |
| 16 (7 units) | 18 Oct 2026 | 46 of 48 (0 under-measured) | **95.8 %** (46 / 48); step level 85.0 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): runner R-12 (Firepixel's swatch pitch, TW-115, and the gaps in
TW-124), a behaviour description for text style shortcuts on a selected text object (TW-122), the Optimize context
menu width (TW-123), the open questions Q-49, Q-51 and Q-54 to Q-56, comparison run 17 for the affected units, then
the human click-test.
