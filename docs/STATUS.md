# Status

Last updated: **18 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.2** was published as a
pre-release on 18 Oct 2026 (tag from `c031a987`) for Windows x64, macOS universal arm64 + x86_64 (macOS 10.15 or
later) and, for the first time, Linux x86_64 (tar.gz and AppImage). Further preview builds are made on request. The
Intel slice of the macOS bundle is verified with tools only, not yet run on Intel hardware, and the Linux build has
run only in CI, not yet on a real Linux desktop.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 48 | [PARITY.md](PARITY.md) |
| Equal except for design | **45** (1 real deviation, 2 unclear, 0 under-measured) | run 21 (7 units) and run 18 (4 units) on top of full run 17, 18 Oct 2026 |
| Proven 1:1 | **93.8 %** (45 / 48; no unit under-measured; unchanged in run 21; run 19: 91.7 %, run 17: 87.5 %) | runs 17–21 |
| Canvas image pairs within 1 % | **176 / 202** (run 20: 175; `tool-text/02` moved below 1 % in run 21 with the reference's text tool at 12 pt; `tool-text/16` stays above 1 %, TW-145) | runs 17–21 |
| Measurement quota, all 48 units | **86.9 %** (1066 / 1226 measurable steps both `ok`, newest run per unit; run 17 alone 86.3 %; the seven units of run 21: 86.8 %, 138 / 159, unchanged) | runs 17–21 |
| Errors, newest run per unit | 38 reference-side (run 17: 46), 21 Firepixel-side (unchanged, all known) | runs 17–21 |
| Workspace tests | **2740 passed, 0 failed, 134 ignored** at `65db4012` (the commit of preview `2026-10-18_22-05`) | full workspace run on `main`, 18 Oct 2026 |
| Open findings | 5 fixable `[F]` (PDF layers; TW-138 rest; TW-140 reference-side keys, runner R-19; TW-143 live bounds through the element list; TW-145 size of new text after a partial-selection edit), 5 deferred `[V]`, 20 informational `[N]`; run 21 closed TW-133, TW-142 and TW-144; TW-134 waits for a reference-side crop | [FINDINGS.md](FINDINGS.md) |
| Real differences known | The live read-back keeps an edited text's old bounds in the element list until commit (TW-143); smaller: *Text* menu commands on pointer-selected text do not yet update the text tool defaults (TW-138 rest). Not yet compared: the size of a new text after a partial-selection edit (TW-145) and the crop option (TW-134), both because the reference-side keys fail (TW-140) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 850+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.2** (18 Oct 2026, pre-release on GitHub, tag from `c031a987`, version bump `2cfff08d`): Windows x64
ZIP, macOS universal ZIP (arm64 + x86_64) and Linux x86_64 tar.gz and AppImage. All assets were verified after
publishing (fat binary, CLI version, archive contents). The Linux build is tested in CI only, the Intel macOS slice
by tools only (not run on Intel hardware). The previous releases were v0.2.1 (18 Oct 2026, Windows x64, macOS
universal) and v0.2.0 (17 Oct 2026, Windows x64, macOS arm64).

**Linux CI:** `prerelease.yml` now also calls `linux-build.yml`, which builds Linux x86_64 on Ubuntu 22.04 (glibc
2.35 baseline): release build of `firepixel` and `firepixel-cli`, a tar.gz (manuals, licence, `.desktop` file,
icons) and an AppImage (pinned, checksum-verified `appimagetool`). TLS for the model download uses rustls on all
non-Windows targets, so the binaries do not link OpenSSL (the build fails if `ldd` lists it). A startup probe runs
the app under Xvfb and checks that the control channel comes up. The first CI runs are green. Not yet tested on a
real Linux desktop; file dialogs need `xdg-desktop-portal`. v0.2.2 is the first release with Windows x64, macOS
universal and Linux x86_64.

Latest test build: preview build **`2026-10-18_22-05`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `65db4012` (packages F42–F44: panel pop-up menu state, live bounds of an edited
text, stretched alignment stored as left, text attribute commands writing the text tool defaults, paragraph
separator tests); a full workspace test run on that commit passed (2740 passed, 0 failed, 134 ignored). The previous
preview was `2026-10-18_15-28` from `054bdc05` (packages F37–F40).

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

After run 19 (on `main` up to `2d64547a`, not yet in a preview build): TW-136 closed as no deviation (the labels
already matched, the shot included the ≡ row and the shadow; 323 × 231 vs. 321 × 231 px), TW-137 (History rows for
Copy, Cut, Select All and Deselect), TW-138 (text tool defaults) and TW-139 (live read-back while editing) fixed,
runner R-15 for TW-140; TW-141 needs a Rust change (F40, in progress). Run 20 is to confirm.

Follow-up run 20 (18 Oct 2026, 15:30–16:09, 7 units, 170 steps, 78 image pairs) used preview `2026-10-18_15-28` and
runner R-15: `menu-edit` is now equal except for design (Copy row in both Histories, TW-137), `xs-shortcuts` stays a
real deviation for one shortcut (TW-133 rest), `tool-text` and `xs-preferences` stay unclear (reference-side runner
effects: text attributes carried between units, the stray "c", the crop not applied on the reference side),
`xs-canvas`, `panel-library-styles` and `panel-pages-states-history` stay equal. Step quota unchanged, one more
canvas pair within 1 % (`tool-text/14`); TW-137, TW-138, TW-139 and TW-141 closed; TW-140 stays open (R-15's
`key_echo` failed with a script error, fixed in R-16), TW-142 to TW-144 added.

Follow-up run 21 (18 Oct 2026, 22:06–22:51, 7 units, 170 steps, 78 image pairs) used preview `2026-10-18_22-05` and
runner R-17 / R-17b / R-18: no class changed (45 / 1 / 2 / 0). `xs-shortcuts` stays a real deviation, now for the
live bounds of an edited text (TW-143 not confirmed) after Ctrl+Alt+Shift+S became equal (TW-133 closed); the URL
panel menu reads the same enabled states and "..." flags on both sides (TW-142 closed) and both programs separate
paragraphs with `\r` (TW-144 closed). Step quota unchanged, one more canvas pair within 1 % (`tool-text/02`). The new
key lines show that the reference takes plain Delete, Right, Down and Escape as tool letters although the keys
arrive correctly and the document view has focus (TW-140, runner R-19); the crop option (TW-134) and the size of a
new text after a partial-selection edit (TW-145, new) remain uncompared.

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

1. Runner R-19 (reference-side Delete, nudges, leaving the text editor and the crop commit through menu, script or
   mouse; text selection by mouse; TW-140), then run 22 of `tool-text`, `xs-preferences`, `xs-shortcuts`,
   `menu-edit` and `panel-pages-states-history`: compares TW-134 and TW-145 and confirms TW-143.
2. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features, URL panel menu,
   typing after a double-click, crop option off, Linux start) and the open A2 checks.
3. Remaining parity items: live bounds of an edited text through the element list (TW-143), the size rule for new
   text (TW-145) and the *Text* menu remnant of the text tool defaults (TW-138 rest).
4. Remaining deferred items: BA-101, BA-108 (metadata box writing), BA-110 rest, BA-083, U-1 PDF import layers.

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
