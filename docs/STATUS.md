# Status

Last updated: **17 Oct 2026**. This page is the single place for the current figures; the history of how they
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
| Equal except for design | **42** (4 real deviations, 1 unclear, 1 under-measured) | run 14, 17 Oct 2026 |
| Proven 1:1 | **87.5 %** (42 / 48; 89.4 % of the 47 sufficiently measured units) | run 14 |
| Canvas image pairs within 1 % | about **179 / 200** (was 155 / 196) | newest run per unit |
| Measurement quota, all 48 units | **84.1 %** (1031 / 1226 measurable steps both `ok`; was 83.5 %) | newest run per unit |
| Measurement quota of run 14 | 81.1 % (163 / 201, 8 units; the same units in run 13: 77.6 %) | run 14 |
| Workspace tests | **2486 green, 0 red, 133 ignored** | full workspace run, 17 Oct 2026 |
| Open findings | 7 fixable `[F]` (TW-112 colour pop-up commit, TW-113 `.act` swatch count, TW-110 symbol properties table look, TW-97 Alt-drag, PDF layers, runner TW-115/116), 21 deferred `[V]` (incl. TW-111, TW-114), 17 informational `[N]` | [FINDINGS.md](FINDINGS.md) |
| Real differences known | Alt-drag duplicate (TW-97), colour pop-up commits on press (TW-112), `.act` swatch loader ignores the colour count (TW-113), animated GIF frame-1 palette (TW-114) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 830+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.0** (17 Oct 2026, pre-release on GitHub, Windows x64 and macOS arm64) from commit `2c54845`
plus the version bump `10bbd8f`; it contains package F28 (symbol properties panel rectangle, animated GIF
transparency slot, pointer log). Comparison run 14 used preview build `2026-10-17_19-47`, release profile, single
`firepixel.exe` plus `firepixel-cli.exe`, built from the same F28 state before the history reset. Full workspace
test run on this snapshot: 2486 passed, 0 failed, 133 ignored.

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

1. Package **F29**: colour pop-up commits on release (TW-112), `.act` swatch import honours the colour count
   (TW-113), symbol properties table look (TW-110).
2. Runner **R-10**: swatch reset at unit start (TW-115), tooltip decoding and spiral `center` handle (TW-116),
   Properties tab before `inspector` shots (TW-111), Alt-drag press point beside R1's gradient handle (TW-97),
   fitting the reference to the large canvas (`xs-canvas`).
3. Behaviour description **RE-11** from measurements: animated GIF quantiser (TW-114), questions Q-51 and Q-52.
4. Comparison run 15 for the affected units, then the human click-test of [VERIFICATION.md](VERIFICATION.md).

See [ROADMAP.md](../ROADMAP.md) for the complete list.
