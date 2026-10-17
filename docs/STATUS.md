# Status

Last updated: **18 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.0** (Windows x64,
macOS arm64) was published as a pre-release on 17 Oct 2026; further preview builds are made on request.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **44** (2 real deviations, 2 unclear, 0 under-measured) | run 15, 17/18 Oct 2026 |
| Proven 1:1 | **91.7 %** (44 / 48; no unit under-measured) | run 15 |
| Canvas image pairs within 1 % | about **177 / 201** (was 179 / 200) | newest run per unit |
| Measurement quota, all 48 units | **84.8 %** (1041 / 1228 measurable steps both `ok`; was 84.1 %) | newest run per unit |
| Measurement quota of run 15 | 86.9 % (153 / 176, 7 units; the same units in run 14: 82.2 %) | run 15 |
| Workspace tests | **2486 green, 0 red, 133 ignored** | full workspace run, 17 Oct 2026 |
| Open findings | 7 fixable `[F]` (TW-113 `.act` swatch loader and sort, TW-118 Shift-drag, TW-120 inspector with several objects, TW-121 button instance properties, PDF layers, runner TW-115/119), 20 deferred `[V]` (incl. TW-111), 17 informational `[N]` | [FINDINGS.md](FINDINGS.md) |
| Real differences known | inspector shows the first instead of the last selected object (TW-120), button instance properties (TW-121), `.act` swatch loader and sort order (TW-113); unclear: Shift-drag of a selected object (TW-118) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 830+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.0** (17 Oct 2026, pre-release on GitHub, Windows x64 and macOS arm64) from commit `2c54845`
plus the version bump `10bbd8f`; it contains package F28 (symbol properties panel rectangle, animated GIF
transparency slot, pointer log). Full workspace test run on that snapshot: 2486 passed, 0 failed, 133 ignored.

Latest test build: preview build **`2026-10-17_23-53`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `3695c7de`, with package F29 (colour pop-up commits on release TW-112, `.act`
colour count TW-113, symbol properties table look TW-110) and F30 (animated GIF palette TW-114). Comparison run 15
used this build.

## How tests run

- Every crate has unit tests; `fp-ui` and `fp-engine` bundle their integration tests into one program `all`
  (`cargo test -p fp-ui --test all`). Corpus and golden tests (`#[ignore]`) run with `-- --ignored` and compare
  against measured reference output.
- UI tests run offscreen (egui kittest and a wgpu renderer) and drive the app through the same command registry
  as the user. Tests never write into the real user profile.
- Guards: `fp-neutral` (no third-party product names in UI texts and manuals), `fp-i18n` (every key in German and
  English), clipping audit for dialogs.
- A **full test run** builds everything in debug with optimisations and runs all suites; it precedes every
  preview build.

## How parity runs work

A parity run plays scripted steps in both programs on the same machine and compares screenshots, values, texts and
exported files ([PARITY.md](PARITY.md)). A full run over 48 units takes about four hours and runs unattended
(usually overnight); follow-up runs cover only affected units. Each run is evaluated by script into tweaks
(`TW-…`) and questions (`Q-…`), which become findings or fixes. Runs need a licensed installation of the
reference application and test files that are not part of this repository.

## What is not verified automatically

Embedded panel dialogs of the reference application, real keyboard paths (Alt tap, access keys), brush strokes
with a real mouse and drag and drop between windows need a human click-test: [VERIFICATION.md](VERIFICATION.md).

## Next

1. Behaviour descriptions from measurements: the original's swatch file loader and sort key (TW-113), the
   inspector with several selected objects (TW-120) and button instances (TW-121); then package **F31** with
   tests that pin the measured values.
2. Runner **R-11**: read back a swatch colour after the reset (TW-115), keep drag distances clear of the snap
   tolerance and check wheel points against the view (TW-119).
3. Questions Q-49 (hex field), Q-51 (inspector tab), Q-52 (swatch add) and Q-53 (Shift-drag, TW-118).
4. Comparison run 16 for `panel-swatches-mixer`, `inspector-group-symbol`, `xs-shortcuts` and `xs-canvas`, then
   the human click-test of [VERIFICATION.md](VERIFICATION.md).

See [ROADMAP.md](../ROADMAP.md) for the complete list.
