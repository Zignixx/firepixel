# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 15, 17/18 Oct 2026):** 48 units — **44 equal except for design**, 2 real deviations,
2 unclear, 0 under-measured. **Proven 1:1: 91.7 %** (44 / 48; all 48 units are sufficiently measured). Measurement
quota over all 48 units: **84.8 %** (1041 / 1228 steps). Canvas image pairs within 1 %: about **177 / 201**.

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
measured units. The headline figure is the strict one over all 48 units (91.7 %); since run 15 no unit is
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

## Per-unit result (run 15)

State per unit from its newest run: run 10 (16/17 Oct) for 24 units, run 11 (17 Oct morning) for 3, run 12
(17 Oct afternoon) for 7, run 13 (17 Oct evening, 14 units) for 6, run 14 (17 Oct, 19:48–20:38) for 1
(`panel-layers`), run 15 (17/18 Oct, 23:54–00:38, 7 units, 189 steps, 97 image pairs, preview build
`2026-10-17_23-53` with packages F29 and F30, runner R-10) for 7.

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
| `panel-misc` | panels | equal except for design | embedded panels `unsupported`; run 13: 87.5 % measured |
| `panel-optimize` | panels | equal except for design | |
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
| `export-results` | preview / export | equal except for design | run 15: 66.7 %; the animated GIF is **byte-identical** to the original's file (TW-114 closed), images-only and PNG 8/24/32 pixel-identical; the remaining errors are runner gaps (`slice:0` locator, export-area dialog) |
| `color-popup` | colour | equal except for design | run 15: 75.0 % (was 71.4 %); drag black → red now gives #ff0000 on both sides (TW-112 closed); hex field question Q-49 still open; canvas shots differ by capture method only (TW-117) |
| `xs-context-menus` | cross-cutting | equal except for design | |
| `xs-preferences` | cross-cutting | equal except for design | |
| `xs-windows-tabs` | cross-cutting | equal except for design | |
| `xs-file-formats` | cross-cutting | equal except for design | run 13: import by dragging lands on the same pixels 121,120–417,322 (TW-101 fixed) |
| `inspector-group-symbol` | property inspector | **real deviation** | run 15: 90.0 %; table look of the Symbol Properties panel now equal (TW-110 closed); with the runner bringing Properties to the front again (step 04b, TW-111) the inspector pairs measure the right panel and show two real differences: with several objects selected Firepixel shows the first selected object's fill and stroke, the original the last one's (TW-120), and the button instance properties differ (TW-121) |
| `panel-swatches-mixer` | panels | **real deviation** | run 15: 96.3 %; after "replace swatches" with the 16-colour `.act` file the original holds about 256 entries (16 colours plus black padding) and sorts black last, while Firepixel now holds only the 16 colours, in a different sort order (TW-113, reopened with this evidence); the reset step 00a does not yet give equal sets, probe colours still differ (#efffbf vs. #ffff00, TW-115) |
| `xs-shortcuts` | cross-cutting | unclear | run 15: 84.0 %; Alt-drag now duplicates on both sides (8 vs. 8, TW-97 closed); new: a Shift-drag of R1 moves it in Firepixel (+60 px, constrained) but not in the original (TW-118, Q-53) |
| `xs-canvas` | cross-cutting | unclear | run 15: 79.2 % (was 58.3 %, no longer under-measured); the reference now fits the large document, five of the seven formerly failing steps are `ok` and their values equal (guide snap, guide lock); `12_grid_snap` differs only because the reference's whole-pixel drag moves 11.67 instead of 13 document px (runner, TW-119); 7 of 8 canvas pairs stay above 1 % (different view sizes, Ctrl+wheel point outside Firepixel's view, TW-119) |

Totals: 44 / 2 / 2 / 0. Step level over all 48 units (newest run per unit): 1041 of 1228 measurable steps `ok` on
both sides (**84.8 %**, was 84.1 %); in the 7 units of run 15: 153 of 176 (86.9 %; the same units in run 14: 143 of
174, 82.2 %). Canvas image pairs within 1 % over all 48 units: about 177 of 201 (run 15 units: 33 of 42; run 14:
35 of 41 – the difference comes from `xs-canvas` pairs now taken at the fitted zoom). `export-results` and
`color-popup` moved to "equal", `inspector-group-symbol` became a real deviation once it measured the right panel,
`xs-shortcuts` became unclear and `xs-canvas` is no longer under-measured.

Real differences in results that remain known: the inspector shows the first instead of the last selected object
(TW-120), the button instance properties (TW-121) and the `.act` swatch loader and its sort order (TW-113); still
unclear: the Shift-drag of a selected object (TW-118, Q-53). See [FINDINGS.md](FINDINGS.md).

## History of the headline figure

| Run | Date | Equal except for design | Proven 1:1 |
|---|---|---|---|
| 10 (full) | 16/17 Oct 2026 | 29 of 48 (9 under-measured) | 74 % (29 / 39) |
| 11 (23 units) | 17 Oct 2026 | 31 of 48 (9 under-measured) | 79 % (31 / 39) |
| 12 (20 units) | 17 Oct 2026 | 39 of 48 (0 under-measured) | 81 % (39 / 48) |
| 13 (14 units) | 17 Oct 2026 | 42 of 48 (2 under-measured) | 88 % (42 / 48; 91 % = 42 / 46 of the sufficiently measured units) |
| 14 (8 units) | 17 Oct 2026 | 42 of 48 (1 under-measured) | 87.5 % (42 / 48; 89.4 % = 42 / 47 of the sufficiently measured units); step level 84.1 % |
| 15 (7 units) | 17/18 Oct 2026 | 44 of 48 (0 under-measured) | **91.7 %** (44 / 48); step level 84.8 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): behaviour descriptions for the swatch file loader and sort
(TW-113) and for the inspector with several objects and button instances (TW-120/121), runner R-11 (TW-115,
TW-119), the open questions Q-49 and Q-51 to Q-53, comparison run 16 for the affected units, then the human
click-test.
