# Status

Last updated: **19 Oct 2026**. This page is the single place for the current figures; the history of how they
came about is kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)).

## In one paragraph

Firepixel is feature-complete as a preview: all menus, tools, panels, the property inspector and the dialogs of
the reference application exist and work, FW-PNG files round-trip, and everything is drivable over CLI, JSF and
MCP. Work is now parity polishing: a scripted comparison runner measures Firepixel against the reference
application unit by unit, and the remaining differences are tracked as findings. Release **v0.2.4** was published on
19 Oct 2026 for Windows x64, macOS universal arm64 + x86_64 (macOS 10.15 or later) and Linux x86_64 (tar.gz and
AppImage), with the three screen modes, the text default rules and the macOS fixes; the earlier release v0.2.2
(18 Oct 2026) was the first with Linux. Further preview builds are made on request. The Intel slice of the macOS bundle is verified with tools only, not yet
run on Intel hardware, and the Linux build has run only in CI, not yet on a real Linux desktop.

## Figures

| Area | Value | Source |
|---|---|---|
| Parity units | 49 (all classified since full run 24; the measurement unit `xs-select-behind` was added on 19 Oct 2026) | [PARITY.md](PARITY.md) |
| Equal except for design | **49** (0 real deviations, 0 unclear, 0 under-measured) | run 27 (3 units) on top of runs 26, 25 and 24, 19 Oct 2026 |
| Proven 1:1 | **100.0 %** (49 / 49; no unit under-measured; over the 48 units before run 24: 100.0 %; runs 26 and 27: 100.0 %; run 25: 98.0 %; run 24: 93.9 %; run 17: 87.5 %) | run 27 |
| Canvas image pairs within 1 % | **185 / 212** (run 26: 186 / 211; run 27 adds `tool-path-reshape/04` within 1 %, while `tool-text/14, 16` move above 1 % because only the reference's text tool is reset after a failed size step, TW-165) | run 27 |
| Measurement quota, all 49 units | **87.3 %** (1077 / 1233 measurable steps both `ok`; over the 48 old units 87.3 %, 1068 / 1224; unchanged since run 26; run 24: 87.0 %; run 17 alone 86.3 %; in the 3 units of run 27: 99 / 106, as in run 26) | run 27 |
| Errors | 33 reference-side, 21 Firepixel-side (both unchanged since run 25); 2 `runner error` lines in run 27 (run 26: 8): `tool-text/11` (no size route reaches the selected "123", TW-146 rest) and `toolbox/13` (the Standard reset after full screen, TW-166) | run 27 |
| Latest release | **v0.2.4** (19 Oct 2026, pre-release; Windows x64 ZIP, macOS universal ZIP, Linux x86_64 tar.gz and AppImage; the three screen modes, the text default rules and the macOS fixes TW-160 to TW-162 with the own app menu); before: v0.2.3 (19 Oct 2026) | GitHub releases |
| Workspace tests | Last full run: **2776 passed, 0 failed, 134 ignored** at `439a3ec5` (runner R-23, package F49, macOS fixes; the code of preview `2026-10-19_16-07`) | `main`, 19 Oct 2026 |
| Open findings | 9 rows in `[F]`: 4 open (PDF layers, U-1 rest; the runner items TW-146 rest, TW-165 and TW-166); TW-138 rest closed except the unmeasured R3 note; TW-151 / FB-01 and the macOS fixes TW-160 to TW-162 (FB-04, FB-05, FB-03) waiting for the click-test and the macOS re-test by the tester; 5 deferred `[V]`, 20 informational `[N]`; run 27 closed TW-159 rest, TW-163 and TW-164 | [FINDINGS.md](FINDINGS.md) |
| Real differences known | None measured open (runs 26 and 27; the last one, full screen mode TW-158, was confirmed fixed in run 26). Unmeasured: whether indent and paragraph spacing on pointer-selected text become text defaults (TW-138 R3) | [PARITY.md](PARITY.md) |
| Closed findings (history) | 850+ | kept outside the repository ([ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)) |
| Startup time | about 0.8 s to the first frame (release build, warm) | smoke test 17 Oct 2026 |

## Last test build

Release **v0.2.4** was published on 19 Oct 2026 as a pre-release (tag `v0.2.4` at `1f26223c`, the version bump on top
of `aaa9b3b5`): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage. It contains package F49 (the
three screen modes as in the reference, TW-158; the text default rules, TW-138 rest) and the macOS fixes TW-160 to
TW-162 with Firepixel's own macOS app menu (`e11f8d1b`); the Mac checks wait for the tester's re-test (A4-19 to
A4-21). Release **v0.2.3** was published on 19 Oct 2026 from the state of preview `2026-10-19_06-06` (`main` at
`44d7c5f5` plus documentation): Windows x64 ZIP, macOS universal ZIP and Linux x86_64 tar.gz and AppImage, all
assets verified after publishing, with thanks to the tester in the release notes. The previous release was **v0.2.2** (18 Oct 2026, pre-release on GitHub, tag from
`c031a987`, version bump `2cfff08d`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64) and Linux x86_64 tar.gz
and AppImage. All assets were verified after publishing (fat binary, CLI version, archive contents). The Linux build
is tested in CI only, the Intel macOS slice by tools only (not run on Intel hardware). Earlier releases were v0.2.1
(18 Oct 2026, Windows x64, macOS universal) and v0.2.0 (17 Oct 2026, Windows x64, macOS arm64).

**Linux CI:** `prerelease.yml` now also calls `linux-build.yml`, which builds Linux x86_64 on Ubuntu 22.04 (glibc
2.35 baseline): release build of `firepixel` and `firepixel-cli`, a tar.gz (manuals, licence, `.desktop` file,
icons) and an AppImage (pinned, checksum-verified `appimagetool`). TLS for the model download uses rustls on all
non-Windows targets, so the binaries do not link OpenSSL (the build fails if `ldd` lists it). A startup probe runs
the app under Xvfb and checks that the control channel comes up. The first CI runs are green. Not yet tested on a
real Linux desktop; file dialogs need `xdg-desktop-portal`. v0.2.2 is the first release with Windows x64, macOS
universal and Linux x86_64.

Latest test build: preview build **`2026-10-19_16-07`** (release profile, single `firepixel.exe` plus
`firepixel-cli.exe`) from `main` at `37c0ee81` (package F49: the three screen modes as in the reference, full screen
shows only the canvas on black; text preference rules, TW-138 rest; runner R-23). The full workspace test run on its
code (`439a3ec5`) gave 2776 passed, 0 failed, 134 ignored. Follow-up runs 26 and 27 used this build. Since then `main`
has the macOS app menu (`e11f8d1b`, FB-05; the macOS CI build with it is green, released in v0.2.4) and runner R-24
(`f7d13d61`); the workspace tests are unchanged. Before that came preview `2026-10-19_12-16`
from `a225a01c` (package F48: consecutive key nudges merge into one History row, crisp Select Behind hover line
without a frame right after a click; runner R-22; 2762 passed, 0 failed, 134 ignored), used by follow-up run 25.
Before that came preview
`2026-10-19_06-06` from `44d7c5f5` (packages F46 and F47; 2761 passed, 0 failed, 134 ignored), used by full run 24
and published as v0.2.3. Before that came preview `2026-10-19_01-32` from
`cd86ce75` (package F46: Escape keeps the selection and only commits text, cancels crop or transform or leaves bitmap
mode; one History row per arrow-key nudge), used by run 23. Before that came preview `2026-10-18_23-25`
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

After run 23 (on `main` up to `44d7c5f5`, preview `2026-10-19_06-06`): runner R-21 (`b1916aa6`) reads the History row
labels on both sides, checks for a caret before typing on the reference side, resets Firepixel's text tool at unit
start to the reference's font, size, style and colour, allows 2 s more per substitute and fixes the console
code-page error in the log; run 24 confirmed TW-149 and TW-150, decided TW-148 from the logged
row labels and left TW-146 rest open (see below).

**Tester feedback (19 Oct 2026, E-Man, credited in the README):** two reports, both written down as RE spec 52 and
fixed. FB-01 (TW-151, `c3a9e264`): with the pen tool, clicking the last anchor while drawing now retracts its outgoing
handle, so the next segment leaves the point straight; a drag re-pulls it as a symmetric curve point (Alt, Ctrl and
Shift variants), interior points are converted or deleted, a segment click inserts a point, another open path is
joined or an open path continued, with matching cursors; the hit radius follows the snap distance. Limits: a join
across elements appends instead of merging, no drag after a join, a close or an interior press, a double-click still
ends the path. FB-02 (TW-152, `44d7c5f5`): Select Behind shows a hover frame (#E00000, 1 px, filled 5 × 5 px corner
squares) on the object a click would select, also while the pointer is over the selected object; with nothing
selected it frames the topmost object; the Pointer tool is unchanged. The rule was measured in the reference
application with the new parity unit `xs-select-behind` (`c7515986`, 9 steps, the 49th unit). Open: whether the
reference draws the blue selection frame together with the red one (its runner shots never show a selection
outline) and one preview-build case without a frame with nothing selected that tests do not reproduce; both are in
the click-test ([VERIFICATION.md](VERIFICATION.md) A4-15 to A4-17).

**Tester feedback, second batch (19 Oct 2026, macOS):** three reports, fixed directly on `main`, all only
type-checked for macOS (no Mac at hand). FB-04 (TW-160, `b9f085f4`): after the app's own minimise and a restore from
the Dock the window stayed frozen, because the minimised flag was never refreshed; the stale flag is now healed
against the real window state. FB-05 (TW-161, `0352e7c9`): Cmd+Q quit through the default app menu without the save
prompt; the default menu is off and Cmd+Q goes through the Quit action with the prompt; since `e11f8d1b` the app has
its own macOS app menu (About, Services, Hide, Hide Others, Show All and Quit Firepixel with the save prompt), and the
macOS CI build with it is green. FB-03 (TW-162, `53e08484`): native title bar with traffic lights on
macOS, without Firepixel's own window buttons. Hands-on checks on a Mac: [VERIFICATION.md](VERIFICATION.md) A4-19
to A4-21.

After run 25 (on `main` up to `439a3ec5`): runner R-23 (`5f498cd7`) for TW-159 and TW-146 rest; package F49
(`8a9cf670`, `439a3ec5`) with the three screen modes as in the reference (TW-158: full screen shows only the canvas
on black, full screen with menus keeps menu bar and toolbar with a floating Tools panel and an icon dock, F needs a
document, back to Standard when the last document closes) and the text preference rules of TW-138 (the inspector
never writes the baseline shift default, font changes store the PostScript name and the recent font list, a style
applied to text writes the text defaults). The full test run on `439a3ec5` gave 2776 passed, 0 failed, 134 ignored;
preview `2026-10-19_16-07` (from `37c0ee81`, the same code plus documentation) was built for run 26.

**Full run 24 (19 Oct 2026, 06:09–11:10, all 49 units, 1295 steps, 698 image pairs)** used preview
`2026-10-19_06-06` and runner R-21: 46 / 2 / 1 / 0, proven 1:1 93.9 % (46 / 49; 95.8 % over the 48 old units),
quota 87.0 % (86.9 % over the 48 old units, unchanged in 46 of them). It decided TW-148: the History row labels show
that the reference merges three Shift+arrow presses on one selection into a single row (`moveSelectionBy({x:20,
y:10})`), so `panel-pages-states-history` is a real deviation and F46's one row per press has to go back to merging.
The first run of `xs-select-behind` confirms the hover rule (TW-152 closed: the same #E00000 frame in both programs)
but shows two smaller differences (TW-157: Firepixel's hover line on a vector path is anti-aliased over two pixel rows,
and Firepixel shows the frame right after a click). Closed: TW-149 (Firepixel's text tool starts at Arial 12 pt like
the reference) and TW-150 (`xs-shortcuts/08` within the new limit). `tool-text` stays unclear: the caret check passes,
but the reference still loses the keys of 4 of 16 new texts (TW-146 rest), so TW-145 is not compared. No regression
from F47 in the pen-related units. New runner items: TW-153 (the reference's unit-start text reset sets its fill to
#000000; `tool-autoshapes` canvas 7 / 27), TW-154 (Text Editor and Edit Guides have no key in the "Web Standard" set),
TW-155 (plain arrow keys outside nudges, History read-back with two documents) and TW-156 (screen-mode steps without
a defined start mode).

After run 24 (on `main` up to `ace7873c`, not yet in a preview build): package F48 fixes TW-148 (`0419534c`:
consecutive arrow-key nudges of an unchanged selection merge into one Move row with the summed offset; a selection
change, another command or an undo starts a new row; script moves stay one row each; Alt+arrow creates the copy on
the first press and merges further presses into that row; this corrects F46) and TW-157 (`ace7873c`: crisp
pixel-aligned hover line, no frame right after a Select Behind click until the pointer moves). Runner R-22
(`72c695dd`) fixes TW-153 (the text reset no longer touches the fill colour, default colours restored and read
back), closes TW-154 as not comparable (Text Editor and Edit Guides have no menu entry and no key in the reference;
Firepixel opens them by command, the reference side is unsupported), fixes TW-155 (arrow substitutes only with the
Pointer or Subselection tool or `nudge: true`; `inspector-stroke/01` hovers instead of using arrows; `menu-edit`
closes extra documents) and TW-156 (standard screen mode at the start and end of `toolbox/13`), and retries the
reference-side text entry (TW-146 rest: first character alone and read back, 35 ms per character). The TW-149
leftover (a bridge error with the Web/locked layer active) is handled by reading the scratch text back from other
layers. Run 25 over the R-22 units is to confirm all of them.

**Follow-up run 25 (19 Oct 2026, 12:19–14:39, 20 units, 556 steps, 317 image pairs)** used preview
`2026-10-19_12-16` and runner R-22: 48 / 1 / 0 / 0 over all 49 units, proven 1:1 98.0 % (48 / 49), quota 87.3 %
(1075 / 1232), canvas pairs 186 / 211. Confirmed and closed: TW-148 (one History row per selection for consecutive key
nudges in both programs), TW-157 (680 px #E00000 hover line on FR in both, no frame right after a click), TW-153
(`tool-autoshapes` 26 / 27), TW-154, TW-155, TW-156 and the TW-149 leftover; TW-145 is decided (the next text after
the size edit is 72 pt in both programs). `panel-pages-states-history`, `xs-select-behind` and `tool-text` are now
equal except for design. New: `toolbox` is a real deviation because Firepixel's full screen mode keeps the menu bar,
the document tab and the status bar while the reference shows only the document on black (TW-158, behaviour
description first); runner rests TW-159 (screen-mode read-back, a real arrow with the Subselection tool in
`xs-shortcuts/10_nudge_point`, the Freeform size keys, a 24 pt text size after one reset). TW-146 rest is down to one
lost text in 17.

**Follow-up run 26 (19 Oct 2026, 16:11–16:55, 5 units, 155 steps, 72 image pairs)** used preview
`2026-10-19_16-07` and runner R-23 over `toolbox`, `xs-shortcuts`, `tool-path-reshape`, `tool-text` and `menu-text`:
49 / 0 / 0 / 0 over all 49 units, proven 1:1 100.0 % (49 / 49), quota 87.3 % (1077 / 1233), canvas pairs 186 / 211.
Confirmed and closed: TW-158 (after the second F both programs show only the canvas on black, `toolbox` is equal except
for design again) and TW-159 (b) and (d) (the Subselection nudge reads 201,210 in both; the reference's text reset goes
from 72 pt to 12 pt). The entry part of TW-146 rest is confirmed (all 15 new texts and three edits arrive as typed).
Open, all runner: TW-146 rest (text selections in `tool-text/11, 13`), TW-159 rest (the Freeform size keys keep the
tool but record no size), new TW-163 (the reference's screen-mode button reads "full screen with menus" in Standard)
and TW-164 (the reference shows the Path Scrubber head in `tool-path-reshape/12_inspector_heads_freeform`).

**Follow-up run 27 (19 Oct 2026, 20:49–21:18, 3 units, 106 steps, 62 image pairs)** used the same preview
`2026-10-19_16-07` and runner R-24 (`f7d13d61`) over `tool-text`, `tool-path-reshape` and `toolbox`: 49 / 0 / 0 / 0
over all 49 units, proven 1:1 100.0 % (49 / 49), quota 87.3 % (1077 / 1233; the three quotas unchanged), canvas pairs
185 / 212. Confirmed and closed: TW-159 rest (with the mouse held both programs keep the Freeform tool and read size 72
after the size keys), TW-164 (the reference shows the Freeform head in `12_inspector_heads_freeform`) and TW-163 (the
reference's Standard mode is read from the window; one `runner error` instead of six). TW-146 rest stays open: none of
the three size routes (field, keys, Text Editor) reaches the selected "123" in the reference. New, both runner: TW-165
(only the reference's text tool is reset after the failed size step, so `tool-text/14, 16` compare 12 pt with 72 pt
text and move above 1 %) and TW-166 (the Standard reset after full screen finds neither the screen-mode button nor
the menu bar).

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
exported files ([PARITY.md](PARITY.md)). A full run over 49 units takes about four hours and runs unattended
(usually overnight); follow-up runs cover only affected units. Each run is evaluated by script into tweaks
(`TW-…`) and questions (`Q-…`), which become findings or fixes. Runs need a licensed installation of the
reference application and test files that are not part of this repository.

## What is not verified automatically

Embedded panel dialogs of the reference application, real keyboard paths (Alt tap, access keys), brush strokes
with a real mouse and drag and drop between windows need a human click-test: [VERIFICATION.md](VERIFICATION.md).

## Next

1. Human click-test of [VERIFICATION.md](VERIFICATION.md), including section A4 (wave-4 features, URL panel menu,
   typing after a double-click, crop option off, Linux start, pen anchor modes, the Select Behind hover frame,
   the History rows of key nudges, the screen modes) and the open A2 checks, and the macOS re-test of v0.2.4 by the
   tester (A4-19 to A4-21).
2. Runner items TW-146 rest, TW-165 and TW-166, then a short follow-up run over `tool-text` and `toolbox`.
3. Remaining parity items: U-1 rest and the unmeasured R3 note of TW-138 (indent and spacing as text defaults).
4. Remaining deferred items: BA-101, BA-108 (metadata box writing), BA-110 rest, BA-083, U-1 PDF import layers.

Wave 3 (18 Oct 2026) added 27 batch steps, chained collect steps, more metadata formats, the MCP gate for external
programs and unsandboxed scripts, opt-in GIF optimiser modes, the remaining Batch Studio UI conveniences, selectable
PDF text and editable PSD layers ([FINDINGS.md](FINDINGS.md), [BATCH.md](BATCH.md)). Wave 4 (18 Oct 2026)
added HTML Setup defaults, an external page style sheet and a Generic HTML5 style, working screenshot, symbol
script and shared palette commands, the ffmpeg row in Preferences, an optional AI object-selection preview on hover
and the brush live preview in the bitmap's stacking position.

See [ROADMAP.md](../ROADMAP.md) for the complete list.
