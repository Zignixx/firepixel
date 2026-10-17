# Parity with the reference application

Firepixel aims to behave like the reference application button by button. This page explains how that is
measured, what the figures mean, where the method stops, and gives the current per-unit result.

**Current state (run 13, 17 Oct 2026):** 48 units — **42 equal except for design**, 2 real deviations, 2 unclear,
2 under-measured (runner bugs, fixed in R-9, not yet re-run). **Proven 1:1: 88 %** (42 / 48). Canvas image pairs
within 1 %: **155 / 196**.

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
measured units. The headline figure is the strict one over all 48 units (88 %); over the 46 sufficiently measured
units it is 91 %.

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

## Per-unit result (run 13)

State per unit from its newest run: run 10 (16/17 Oct) for 24 units, run 11 (17 Oct morning) for 3, run 12
(17 Oct afternoon) for 7, run 13 (17 Oct evening, 14 units, 386 steps, 179 image pairs) for 14.

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
| `panel-layers` | panels | equal except for design | drag onto a locked layer fixed (TW-86) |
| `panel-layers-menu` | panels | equal except for design | |
| `panel-misc` | panels | equal except for design | embedded panels `unsupported`; run 13: 87.5 % measured |
| `panel-optimize` | panels | equal except for design | |
| `panel-library-styles` | panels | equal except for design | run 13: equal at an integer drop point; half-pixel rounding still open (TW-100) |
| `panel-swatches-mixer` | panels | equal except for design | run 13: swatch click was a runner effect (unequal palettes, TW-105) |
| `panel-pages-states-history` | panels | equal except for design | run 13: replay offset came from script steps in the original's history (TW-106) |
| `preview-options` | preview / export | equal except for design | |
| `preview-views` | preview / export | equal except for design | |
| `color-popup-contexts` | colour | equal except for design | |
| `tool-path-reshape` | tools | equal except for design | |
| `tool-text` | tools | equal except for design | |
| `tool-web` | tools | equal except for design | |
| `toolbox` | tools | equal except for design | run 13 |
| `xs-context-menus` | cross-cutting | equal except for design | |
| `xs-preferences` | cross-cutting | equal except for design | |
| `xs-windows-tabs` | cross-cutting | equal except for design | |
| `xs-file-formats` | cross-cutting | equal except for design | run 13: import by dragging lands on the same pixels 121,120–417,322 (TW-101 fixed) |
| `xs-shortcuts` | cross-cutting | **real deviation** | Alt-drag duplicates in the original, not in Firepixel (TW-97, Q-47/Q-50); pointer log added in F28, pending run 14 |
| `inspector-group-symbol` | property inspector | **real deviation** | symbol properties header "Name / Value" missing in the run image (TW-104); F28 found the bridge reported a cut-off panel rectangle and fixed it, pending run 14 |
| `color-popup` | colour | unclear | hex field input on the reference side (Q-46/Q-49) |
| `tool-autoshapes` | tools | unclear | unequal start colours (only the reference got "default colours"; runner fixed in R-9, TW-107); outline and bounds equal |
| `xs-canvas` | cross-cutting | under-measured | the reference could not be fitted to the large test document (runner, fixed in R-9; TW-98, Q-48) |
| `export-results` | preview / export | under-measured | runner exception on the reference side (fixed in R-9); Firepixel PNG 8/24 pixel-identical to the reference files, GIF animation palette contents still differ (TW-108) |

Totals: 42 / 2 / 2 / 2. Step level in the 14 units of run 13: 283 of 355 measurable steps `ok` on both sides
(79.7 %); canvas image pairs within 1 % over all 48 units: 155 of 196 (the drop comes from TW-107 and the runner
gap in `xs-canvas`). The two under-measured units are runner bugs that R-9 fixed; they have not been re-run yet.

Besides the unit classes, two real differences in results remain known: the Alt-drag duplicate (above) and the
palette contents of animated GIF exports (TW-108, see [FINDINGS.md](FINDINGS.md)).

## History of the headline figure

| Run | Date | Equal except for design | Proven 1:1 |
|---|---|---|---|
| 10 (full) | 16/17 Oct 2026 | 29 of 48 (9 under-measured) | 74 % (29 / 39) |
| 11 (23 units) | 17 Oct 2026 | 31 of 48 (9 under-measured) | 79 % (31 / 39) |
| 12 (20 units) | 17 Oct 2026 | 39 of 48 (0 under-measured) | 81 % (39 / 48) |
| 13 (14 units) | 17 Oct 2026 | 42 of 48 (2 under-measured) | **88 %** (42 / 48; 91 % = 42 / 46 of the sufficiently measured units) |

The next steps are in [ROADMAP.md](../ROADMAP.md): comparison run 14 for the units touched by F28 and R-9, the
behaviour description for the GIF animation palette, then the human click-test.
