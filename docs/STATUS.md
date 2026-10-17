# Status

Last updated: **17 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Nothing has been released
publicly yet; preview builds are made on request.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **42** (2 real deviations, 2 unclear, 2 under-measured by runner bugs fixed in R-9, not yet re-run) | run 13, 17 Oct 2026 |
| Proven 1:1 | **88 %** (42 / 48; 91 % of the 46 sufficiently measured units) | run 13 |
| Canvas image pairs within 1 % | 155 / 196 | newest run per unit |
| Measurement quota of run 13 | 79.7 % (283 / 355 measurable steps both `ok`, 14 units) | run 13 |
| Workspace tests | **2486 green, 0 red, 133 ignored** | full workspace run, 17 Oct 2026 |
| Open findings | 4 fixable `[F]` (TW-108 GIF animation palette, TW-100 half-pixel drop, PDF layers, Alt-drag), 19 deferred `[V]`, 16 informational `[N]` | [FINDINGS.md](FINDINGS.md) |
| Real differences known | Alt-drag duplicate (TW-97, pending run 14), animated GIF palette contents (TW-108) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 830+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Preview build `2026-10-17_17-10` from commit `a389253` (17 Oct 2026), release profile, single `firepixel.exe`
plus `firepixel-cli.exe`; full workspace test run on that commit: 2482 passed, 0 failed, 133 ignored. Package F28
(symbol properties panel rectangle, animated GIF transparency slot, pointer log) came after that build. Full
workspace test run on this snapshot: 2486 passed, 0 failed, 133 ignored.

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

1. Comparison run 14 for the units touched by F28 and runner R-9 (`xs-shortcuts`, `inspector-group-symbol`,
   `xs-canvas`, `export-results`, `tool-autoshapes`, `color-popup`).
2. Behaviour descriptions from measurements for TW-108 (GIF animation palette) and TW-100 (half-pixel drop), then
   the fixes.
3. Human click-test of [VERIFICATION.md](VERIFICATION.md).
4. Pre-release v0.2.0 on request.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
