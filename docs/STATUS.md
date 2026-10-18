# Status

Last updated: **18 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.0** (Windows x64,
macOS arm64) was published as a pre-release on 17 Oct 2026; further preview builds are made on request. Since
18 Oct 2026 the macOS CI job builds a **universal** app bundle (arm64 + x86_64, macOS 10.15 or later).

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **46** (0 real deviations, 2 unclear, 0 under-measured) | run 16, 18 Oct 2026 |
| Proven 1:1 | **95.8 %** (46 / 48; no unit under-measured; was 91.7 %) | run 16 |
| Canvas image pairs within 1 % | about **177 / 201** (unchanged) | newest run per unit |
| Measurement quota, all 48 units | **85.0 %** (1042 / 1226 measurable steps both `ok`; was 84.8 %) | newest run per unit |
| Measurement quota of run 16 | 86.3 % (126 / 146, 7 units; the same units before: 84.5 %); 0 Firepixel-side errors | run 16 |
| Workspace tests | **2510 green, 0 red, 133 ignored** | full workspace run on `main` at `666c15c6`, 18 Oct 2026 |
| Open findings | 5 fixable `[F]` (TW-122 text shortcuts on a selected text object, TW-123 Optimize context menu width, PDF layers, runner TW-115/124), 20 deferred `[V]` (incl. TW-111), 17 informational `[N]` | [FINDINGS.md](FINDINGS.md) |
| Real differences known | none from the runs; unclear: text shortcuts on a selected text object (TW-122) and the remaining `xs-canvas` canvas pairs | [PARITY.md](PARITY.md) |
| Closed findings (history) | 830+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.0** (17 Oct 2026, pre-release on GitHub, Windows x64 and macOS arm64) from commit `2c54845`
plus the version bump `10bbd8f`; it contains package F28 (symbol properties panel rectangle, animated GIF
transparency slot, pointer log). Full workspace test run on that snapshot: 2486 passed, 0 failed, 133 ignored.

Latest test build: preview build **`2026-10-18_02-04`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `788f4fa1`, with package F31 (swatch file loader with 257 entries, built-in
swatch sets, sort by colour and 9 px cells TW-113; Shift on a selected object never moves it TW-118; topmost object
drives the inspector TW-120; button instance tab TW-121). Comparison run 16 used this build. `main` is now at
`666c15c6` (universal macOS CI build, DE/EN texts for two command parameters); full workspace test run there:
2510 passed, 0 failed, 133 ignored.

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

1. Runner **R-12**: probe and read back Firepixel's swatches at the 9 px pitch (TW-115); serialise objects over the
   bridge, read panel pop-up menus, reference-safe read-back scripts, drag end points outside the view, guide
   double-click and export-area dialog in the reference (TW-124).
2. Behaviour description of text style shortcuts on a selected text object (TW-122, Q-54), then the fix; the
   Optimize colour-table context menu width (TW-123).
3. Questions Q-49 (hex field), Q-51 (inspector tab), Q-55 (caret after a text-tool click) and Q-56 (Snap to Pixel
   without a selection).
4. Comparison run 17 for `xs-shortcuts`, `xs-canvas`, `panel-swatches-mixer` and `panel-optimize`, then the human
   click-test of [VERIFICATION.md](VERIFICATION.md).

See [ROADMAP.md](../ROADMAP.md) for the complete list.
