# Status

Last updated: **18 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.1** (Windows x64,
macOS universal arm64 + x86_64, macOS 10.15 or later) was published as a pre-release on 18 Oct 2026; further
preview builds are made on request. The Intel slice of the macOS bundle is verified with `lipo` only, not yet run
on Intel hardware.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **42** (1 real deviation, 5 unclear, 0 under-measured) | full run 17, 18 Oct 2026 |
| Proven 1:1 | **87.5 %** (42 / 48; no unit under-measured; run 16 carried 95.8 %) | run 17 |
| Canvas image pairs within 1 % | **176 / 202** (was about 177 / 201) | run 17 |
| Measurement quota, all 48 units | **86.3 %** (1057 / 1225 measurable steps both `ok`, run 17 alone; last full run 13: 79.7 %; carried figure 85.0 %) | run 17 |
| Run 17 errors | 46 reference-side (the same units before: 59), 21 Firepixel-side (was 27, all known from earlier runs) | run 17 |
| Workspace tests | **2660 green, 0 red, 134 ignored** at `cd30c354` | full workspace run on `main`, 18 Oct 2026 |
| Open findings | 7 fixable `[F]` (PDF layers; TW-125 to TW-130 from run 17), 5 deferred `[V]`, 20 informational `[N]`; run 17 confirmed TW-115, TW-123 and TW-124 | [FINDINGS.md](FINDINGS.md) |
| Real differences known | per-slice export formats in the Optimize panel (TW-128), minimum width of short panel menus (TW-125); unclear: shortcut set of the runner profile (TW-126), text-tool click on a selected text (TW-130), runner start states (TW-127) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 845+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.1** (18 Oct 2026, pre-release on GitHub, Windows x64 and macOS universal) with version bump
`b3919cb6`; the macOS bundle is built by `macos-preview.yml` as one universal app (arm64 + x86_64; the x86_64
slice checked with `lipo`, not run on Intel hardware). The previous release was v0.2.0 (17 Oct 2026, Windows x64,
macOS arm64).

Latest test build: preview build **`2026-10-18_09-22`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `cd30c354` (wave-4 features, brush preview in stacking order); full workspace
test run there: 2660 passed, 0 failed, 134 ignored.

Full comparison run 17 (18 Oct 2026, 04:59–09:40, all 48 units, 1284 steps, 683 image pairs) used the earlier
preview build `2026-10-18_04-57` from `main` at `7e3d2d12` (fixes after run 16, runner R-12): 42 units equal except
for design, 1 real deviation, 5 unclear; step level 86.3 %; the animated GIF export is byte-identical again.

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

1. Fixes for the run-17 findings: behaviour descriptions for the reference machine's shortcut set (TW-126) and
   the text-tool click on a selected text (TW-130), direct fixes for per-slice export formats (TW-128) and the
   minimum panel-menu width (TW-125), runner R-13 (TW-127, TW-129); then comparison run 18 of the affected units.
2. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features).
3. Remaining deferred items: BA-101, BA-108 (metadata box writing), BA-110 rest, BA-083, U-1 PDF import layers.

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
