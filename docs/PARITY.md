# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 14, 17 Oct 2026):** 48 units — **42 equal except for design**, 4 real deviations, 1 unclear,
1 under-measured. **Proven 1:1: 87.5 %** (42 / 48; 89.4 % of the 47 sufficiently measured units). Measurement quota
over all 48 units: **84.1 %** (1031 / 1226 steps). Canvas image pairs within 1 %: about **179 / 200**.

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
measured units. The headline figure is the strict one over all 48 units (87.5 %); over the 47 sufficiently measured
units it is 89.4 %.

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

## Per-unit result (run 14)

State per unit from its newest run: run 10 (16/17 Oct) for 24 units, run 11 (17 Oct morning) for 3, run 12
(17 Oct afternoon) for 7, run 13 (17 Oct evening, 14 units) for 6, run 14 (17 Oct, 19:48–20:38, 8 units, 215 steps,
109 image pairs, preview build `2026-10-17_19-47`) for 8.

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
| `panel-layers` | panels | equal except for design | drag onto a locked layer fixed (TW-86); run 14 unchanged (74.1 % measured, locator gaps in the scrolled panel) |
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
| `tool-autoshapes` | tools | equal except for design | run 14: equal start colours (R-9, TW-107 closed), 27 of 31 pairs within 1 %; runner gaps TW-116 |
| `xs-context-menus` | cross-cutting | equal except for design | |
| `xs-preferences` | cross-cutting | equal except for design | |
| `xs-windows-tabs` | cross-cutting | equal except for design | |
| `xs-file-formats` | cross-cutting | equal except for design | run 13: import by dragging lands on the same pixels 121,120–417,322 (TW-101 fixed) |
| `xs-shortcuts` | cross-cutting | **real deviation** | run 14: 84.0 %; Alt rectangle now equal (TW-109 closed); Alt-drag still duplicates only in the original (8 vs. 7, TW-97/Q-50), press point still on R1's gradient handle (runner R-10) |
| `color-popup` | colour | **real deviation** | run 14: 71.4 %; Firepixel commits on mouse press, the original on release (TW-112); hex field Q-49 still open |
| `panel-swatches-mixer` | panels | **real deviation** | run 14: 96.2 %; `.act` import ignores the colour count and adds 240 black swatches (TW-113); unequal start sets on the runner side (TW-115, Q-52) |
| `export-results` | preview / export | **real deviation** | run 14: 66.7 % (was 25.0 %); images-only and PNG 8/24/32 pixel-identical in the same run; GIF frames, delay, loop, disposal and transparency equal, frame-1 palette differs (TW-114) |
| `inspector-group-symbol` | property inspector | unclear | run 14: 89.5 %; header "Name / Value" now present (TW-104 closed), table look TW-110; Firepixel keeps the Symbol Properties tab in front for steps 05–14, so six pairs measure the wrong panel (TW-111, Q-51) |
| `xs-canvas` | cross-cutting | under-measured | run 14: 58.3 %; the reference still stays at zoom 1.0 instead of fitting the large test document (runner, TW-98, Q-48) |

Totals: 42 / 4 / 1 / 1. Step level over all 48 units (newest run per unit): 1031 of 1226 measurable steps `ok` on
both sides (**84.1 %**, was 83.5 %); in the 8 units of run 14: 163 of 201 (81.1 %, the same units in run 13: 77.6 %).
Canvas image pairs within 1 % over all 48 units: about 179 of 200 (run 14 units: 35 of 41). `tool-autoshapes`
moved to "equal", `panel-swatches-mixer` moved out because TW-113 turned out to be a real Firepixel deviation
(it was hidden behind the runner explanation TW-105).

Real differences in results that remain known: the Alt-drag duplicate (TW-97), the colour pop-up commit on press
(TW-112), the `.act` swatch loader (TW-113) and the frame-1 palette of animated GIF exports (TW-114); see
[FINDINGS.md](FINDINGS.md).

## History of the headline figure

| Run | Date | Equal except for design | Proven 1:1 |
|---|---|---|---|
| 10 (full) | 16/17 Oct 2026 | 29 of 48 (9 under-measured) | 74 % (29 / 39) |
| 11 (23 units) | 17 Oct 2026 | 31 of 48 (9 under-measured) | 79 % (31 / 39) |
| 12 (20 units) | 17 Oct 2026 | 39 of 48 (0 under-measured) | 81 % (39 / 48) |
| 13 (14 units) | 17 Oct 2026 | 42 of 48 (2 under-measured) | 88 % (42 / 48; 91 % = 42 / 46 of the sufficiently measured units) |
| 14 (8 units) | 17 Oct 2026 | 42 of 48 (1 under-measured) | **87.5 %** (42 / 48; 89.4 % = 42 / 47 of the sufficiently measured units); step level 84.1 % |

The next steps are in [ROADMAP.md](../ROADMAP.md): package F29 (TW-110/112/113), runner R-10, the behaviour
description RE-11 for the GIF animation quantiser, comparison run 15, then the human click-test.
