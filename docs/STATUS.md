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
| Equal except for design | **46** (0 real deviations, 2 unclear, 0 under-measured) | run 16, 18 Oct 2026 |
| Proven 1:1 | **95.8 %** (46 / 48; no unit under-measured; was 91.7 %) | run 16 |
| Canvas image pairs within 1 % | about **177 / 201** (unchanged) | newest run per unit |
| Measurement quota, all 48 units | **85.0 %** (1042 / 1226 measurable steps both `ok`; was 84.8 %) | newest run per unit |
| Measurement quota of run 16 | 86.3 % (126 / 146, 7 units; the same units before: 84.5 %); 0 Firepixel-side errors | run 16 |
| Workspace tests | **2510 green, 0 red, 133 ignored** | full workspace run on `main` at `666c15c6`, 18 Oct 2026 |
| Open findings | 1 fixable `[F]` (PDF layers), 13 deferred `[V]`, 17 informational `[N]`; TW-122/123/115/124 and TW-111 closed after run 16, to be confirmed by the next full run | [FINDINGS.md](FINDINGS.md) |
| Real differences known | none from the runs; unclear: the remaining `xs-canvas` canvas pairs (TW-122 was a shortcut-set mismatch in the runner profile) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 840+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.1** (18 Oct 2026, pre-release on GitHub, Windows x64 and macOS universal) with version bump
`b3919cb6`; the macOS bundle is built by `macos-preview.yml` as one universal app (arm64 + x86_64; the x86_64
slice checked with `lipo`, not run on Intel hardware). The previous release was v0.2.0 (17 Oct 2026, Windows x64,
macOS arm64).

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

1. Full parity run over all 48 units (in progress tonight; runner R-12, Firepixel shortcut set in the runner
   profile); it also checks the fixes made after run 16 (TW-122, TW-123 and the app-wide menu arrow column,
   TW-115/124, Q-49, Q-55, Q-56).
2. Evaluation of that run, including a review of the `xs-canvas` canvas pairs.
3. Human click-test of [VERIFICATION.md](VERIFICATION.md).
4. Remaining deferred items: missing batch steps (being implemented), batch metadata and UI conveniences, PDF/PSD
   export details, HTML setup, CMD-1, AI mask preview.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
