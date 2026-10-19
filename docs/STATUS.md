# Status

Last updated: **19 Oct 2026**. This page is the single place for the current figures; the history of how they
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
| Equal except for design | **46** (0 real deviations, 2 unclear, 0 under-measured) | run 23 (5 units), run 22 (2 units) and run 18 (4 units) on top of full run 17, 18/19 Oct 2026 |
| Proven 1:1 | **95.8 %** (46 / 48; no unit under-measured; run 22: 97.9 %, run 17: 87.5 %) | runs 17–23 |
| Canvas image pairs within 1 % | **175 / 201** (unchanged from run 22; `tool-text/02`, `03`, `07`, `14` now compare against the reference's typed texts; `tool-text/16` stays above 1 %, TW-145) | runs 17–23 |
| Measurement quota, all 48 units | **86.9 %** (1065 / 1226 measurable steps both `ok`, newest run per unit; run 17 alone 86.3 %; the five units of run 23: 84.2 %, 101 / 120, run 22: 102 / 120; `xs-shortcuts/08` `slow`) | runs 17–23 |
| Errors, newest run per unit | 38 reference-side plus one `slow` step (run 17: 46), 21 Firepixel-side (unchanged, all known) | runs 17–23 |
| Workspace tests | **2748 passed, 1 failed, 134 ignored** at `cd86ce75` (full workspace run, the commit of preview `2026-10-19_01-32`, package F46); the one failure was an `fp-i18n` check (missing DE/EN text for the `edit.move` nudge parameter), fixed in `bea53d1c` | `main`, 19 Oct 2026 |
| Open findings | 7 fixable `[F]` (PDF layers; TW-138 rest; TW-145 size of new text after a partial-selection edit; TW-146 rest, reference-side text entry, runner R-21; TW-148 History rows of consecutive nudges, RE spec check; TW-149 text tool font at unit start, runner; TW-150 step limit of `xs-shortcuts/08`, runner), 5 deferred `[V]`, 20 informational `[N]`; run 23 closed TW-147 | [FINDINGS.md](FINDINGS.md) |
| Real differences known | None proven in the newest runs. Unclear: after three arrow nudges the reference reads 4 History steps, Firepixel 5 (TW-148; F46 follows RE spec 51, one row per press). Smaller, from the code: *Text* menu commands on pointer-selected text do not yet update the text tool defaults (TW-138 rest). Not yet compared: the size of a new text after a partial-selection edit (TW-145), because the reference's size edit on the selected characters did not take effect in run 23 | [PARITY.md](PARITY.md) |
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

Latest test build: preview build **`2026-10-19_01-32`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `cd86ce75` (package F46 after RE spec 51: Escape keeps the selection and only
commits text, cancels crop or transform or leaves bitmap mode; one History row per arrow-key nudge). The full
workspace test run on that commit gave 2748 passed, 1 failed, 134 ignored; the failure was an `fp-i18n` check for
the DE/EN text of the `edit.move` nudge parameter, fixed in `bea53d1c`. Before that came preview `2026-10-18_23-25`
from `a4c2bf2b` (package F45: element-list read-backs see the live text element while editing, TW-143). The
preview before was `2026-10-18_22-05` from `65db4012` (packages F42–F44: panel pop-up menu state, live bounds of an
edited text, stretched alignment stored as left, text attribute commands writing the text tool defaults, paragraph
separator tests).

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

Follow-up run 22 (18/19 Oct 2026, 23:27–00:10, 7 units, 170 steps, 77 image pairs) used preview `2026-10-18_23-25`
and runner R-19: two classes changed, 47 / 0 / 1 / 0. `xs-shortcuts` is now equal except for design (the live bounds
of an edited text read 50,150–120,194 in both programs, TW-143 closed; the nudges are equal), `xs-preferences` too
(R-19's double-click commits the reference's crop: 280 × 220 and the same four paths in both programs, TW-134
closed). R-19's substitutes for Delete, nudges, Escape and the crop commit work on the reference side (TW-140
closed). `tool-text` stays unclear: on the reference side texts created by a click stay empty and the drag selection
fails (TW-146, runner R-20), so the size rule TW-145 is still not compared. Step quota unchanged (138 / 159), one
canvas pair fewer (`tool-text/13` without reference shot); further runner side effects as TW-147.

Follow-up run 23 (19 Oct 2026, 01:34–02:10, 5 units, 123 steps, 49 image pairs) used preview `2026-10-19_01-32`
and runner R-20: one class changed, 46 / 0 / 2 / 0. `panel-pages-states-history` became unclear: with real
Shift+arrows the reference reads 4 History steps after three nudges and a nudge of a second object, Firepixel 5
(TW-148; the count points to merged nudges in the reference, against RE spec 51 and F46, but the row list is not
read). `tool-text` stays unclear: R-20 creates 14 of 17 reference texts as typed and verifies drag selections by
highlight pixels, but the size edit on "123" does not take effect, so TW-145 is still not compared (TW-146 rest).
Confirmed and closed (TW-147): Escape keeps the selection in both programs, the Pointer tool is reached through the
flyout, the paste read-back and `MaxUndos` (20) are equal. Step quota 1065 / 1226 (`xs-shortcuts/08` over the 90 s
step limit, TW-150), canvas pairs unchanged; new runner items TW-149 (text tool font) and TW-150.

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

1. Nudge rows in History (TW-148): read the reference's History row list in `panel-pages-states-history/21` and
   check RE spec 51 §2 against it; align F46 if the reference merges consecutive nudges.
2. Runner R-21: the rest of the reference-side text entry (keys reaching the tool shortcuts in 3 of 17 steps, the
   "Q" after a double-click) and a size edit that reaches the selected characters (TW-146 rest), the text tool font
   at unit start (TW-149), the step limit of `xs-shortcuts/08` (TW-150); then a run of `tool-text` and
   `panel-pages-states-history`, which compares the size rule for new text (TW-145).
3. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features, URL panel menu,
   typing after a double-click, crop option off, Linux start) and the open A2 checks.
4. Remaining parity items: the size rule for new text (TW-145, after R-21) and the *Text* menu remnant of the
   text tool defaults (TW-138 rest).
5. Remaining deferred items: BA-101, BA-108 (metadata box writing), BA-110 rest, BA-083, U-1 PDF import layers.

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
