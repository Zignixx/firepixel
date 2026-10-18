# Status

Last updated: **18 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.1** (Windows x64,
macOS universal arm64 + x86_64, macOS 10.15 or later) was published as a pre-release on 18 Oct 2026; **v0.2.2** is
planned for the same day and adds Linux x86_64 (tar.gz and AppImage). Further preview builds are made on request. The Intel slice of the macOS bundle is verified with `lipo` only, not yet run
on Intel hardware, and the Linux build has run only in CI, not yet on a real Linux desktop.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **44** (2 real deviations, 2 unclear, 0 under-measured) | run 19 (6 units) and run 18 (5 units) on top of full run 17, 18 Oct 2026 |
| Proven 1:1 | **91.7 %** (44 / 48; no unit under-measured; run 18: 89.6 %, run 17: 87.5 %) | runs 17–19 |
| Canvas image pairs within 1 % | **174 / 202** (unchanged in run 19; run 17: 176 / 202; two `tool-text` pairs are above 1 % through the reference's text-tool state) | runs 17–19 |
| Measurement quota, all 48 units | **86.9 %** (1066 / 1226 measurable steps both `ok`, newest run per unit; run 17 alone 86.3 %; the six units of run 19: 87.1 %, 121 / 139, unchanged) | runs 17–19 |
| Errors, newest run per unit | 38 reference-side (run 17: 46), 21 Firepixel-side (unchanged, all known) | runs 17–19 |
| Workspace tests | **2695 passed, 1 failed, 134 ignored** at `6e2ad3b1` (the failure was a missing DE/EN command text, added in `0f96dd71`) | full workspace run on `main`, 18 Oct 2026 |
| Open findings | 8 fixable `[F]` (PDF layers; TW-133 rest; TW-136 to TW-141 from run 19, two of them runner items), 5 deferred `[V]`, 20 informational `[N]`; run 19 closed TW-125, TW-130 / Q-57, TW-131, TW-132 and TW-135 | [FINDINGS.md](FINDINGS.md) |
| Real differences known | Ctrl+Alt+Shift+S sets "stretch" on a text the reference leaves alone (TW-133 rest), no Copy row in History (TW-137); smaller: URL menu labels with "..." (TW-136), text-tool defaults after a change on a partial selection (TW-138), scripting read-back while a text is edited (TW-139); the crop option (TW-134) is fixed and waits for the next `xs-preferences` run | [PARITY.md](PARITY.md) |
| Closed findings (history) | 850+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.1** (18 Oct 2026, pre-release on GitHub, Windows x64 and macOS universal) with version bump
`b3919cb6`; the macOS bundle is built by `macos-preview.yml` as one universal app (arm64 + x86_64; the x86_64
slice checked with `lipo`, not run on Intel hardware). The previous release was v0.2.0 (17 Oct 2026, Windows x64,
macOS arm64).

**Linux CI:** `prerelease.yml` now also calls `linux-build.yml`, which builds Linux x86_64 on Ubuntu 22.04 (glibc
2.35 baseline): release build of `firepixel` and `firepixel-cli`, a tar.gz (manuals, licence, `.desktop` file,
icons) and an AppImage (pinned, checksum-verified `appimagetool`). TLS for the model download uses rustls on all
non-Windows targets, so the binaries do not link OpenSSL (the build fails if `ldd` lists it). A startup probe runs
the app under Xvfb and checks that the control channel comes up. The first CI runs are green. Not yet tested on a
real Linux desktop; file dialogs need `xdg-desktop-portal`. v0.2.2 (planned 18 Oct 2026) is the first release with
Windows x64, macOS universal and Linux x86_64.

Latest test build: preview build **`2026-10-18_13-04`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `a3510515` (packages F34–F36: text-tool caret, URL panel menu, Select Inverse
warning, text size ±1 pt, no stretch on point text, caret and typing after a pointer double-click). The previous
preview `2026-10-18_10-59` from `4e677046` passed a full workspace test run (2676 passed, 0 failed, 134 ignored).

Full comparison run 17 (18 Oct 2026, 04:59–09:40, all 48 units, 1284 steps, 683 image pairs) used the earlier
preview build `2026-10-18_04-57` from `main` at `7e3d2d12` (fixes after run 16, runner R-12): 42 units equal except
for design, 1 real deviation, 5 unclear; step level 86.3 %; the animated GIF export is byte-identical again.

Follow-up run 18 (18 Oct 2026, 11:00–11:54, 10 units, 252 steps, 126 image pairs) used preview `2026-10-18_10-59`
and runner R-13/R-13b (the "Web Standard" set in both programs): 5 of the 10 units equal except for design, 2 real
deviations (`panel-library-styles`: URL panel menu, `xs-shortcuts`: text shortcuts), 3 unclear (`menu-edit`,
`xs-preferences`, `tool-text`); TW-125 to TW-129 closed.

Follow-up run 19 (18 Oct 2026, 13:04–13:40, 6 units, 149 steps, 66 image pairs) used preview `2026-10-18_13-04` and
runner R-14: `panel-library-styles` is now equal except for design (URL panel menu), `menu-edit` became a real
deviation (no Copy row in History, TW-137), `xs-shortcuts` stays a real deviation for one shortcut (TW-133 rest),
`tool-text` stays unclear (reference-side text-tool state), `xs-canvas` and `panel-pages-states-history` stay equal
(States menu 210 vs. 207 px). Step quota and canvas pairs unchanged; TW-125, TW-130 / Q-57, TW-131, TW-132 and
TW-135 closed, TW-136 to TW-141 added.

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

1. Release v0.2.2 today (18 Oct 2026) with Windows x64, macOS universal and Linux x86_64.
2. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features, URL panel menu,
   typing after a double-click, crop option off, Linux start) and the open A2 checks from run 19.
3. Remaining parity items from run 19: behaviour descriptions for the Copy history row (TW-137), the text-tool
   defaults (TW-138) and "stretch" / "justified" on point text (TW-133 rest); the URL menu labels (TW-136) and the
   read-back while editing (TW-139); runner R-15 (TW-140, TW-141); then a run of `xs-preferences` (TW-134) and the
   affected units.
4. Remaining deferred items: BA-101, BA-108 (metadata box writing), BA-110 rest, BA-083, U-1 PDF import layers.

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
