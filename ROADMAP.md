# Roadmap

Where Firepixel stands and what remains. Figures as of **22 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **48 / 49** (1 real deviation, 0 unclear, 0 under-measured; full run 33 over all 49 units, 21/22 Oct 2026; `tool-autoshapes` real deviation, TW-195: the reference draws the Arrow Line black while the tool stroke is red, Firepixel draws it red and keeps the red; TW-193 and TW-194 confirmed, TW-192 confirmed for the colours, the runner item TW-174 confirmed) |
| Proven 1:1 | **98.0 %** (48 / 49; 97.9 % over the 48 units before run 24; run 32: 100.0 %; run 31: 95.9 %; run 30: 98.0 %; runs 26 to 29: 100.0 %; run 25: 98.0 %; run 24: 93.9 %) |
| Measurement quota, all 49 units | **87.1 %** (1075 / 1234 steps; run 32: 87.2 %, 1075 / 1233; `export-results/13` measurable for the first time; 87.0 % over the 48 old units; runs 26 to 29: 87.3 %; run 24: 87.0 %; run 17: 86.3 %) |
| Canvas image pairs within 1 % | 184 / 209 (run 32: 185 / 212; three pairs without a reference shot in run 33; `menu-modify-a/20` below 1 % by TW-192, `tool-autoshapes/05` above 1 % by TW-195; `tool-text/14, 16` above 1 %, TW-165). Tiled canvas (BA-098): no sampling offset at fractional zoom |
| Errors | 36 reference-side, 21 Firepixel-side (the same totals; no Firepixel-side step status changed in run 33); 4 `runner error` lines in run 33 (reference-side text entry and selection in `tool-text`, TW-146 rest; run 32: 2, run 31: 10) |
| Tests | Tree with F61 (auto shapes, follow rules) and runners R-30/R-31: **2948 passed, 0 failed, 141 ignored** at `84346fec` (GT-1n, 22 Oct 2026, the code of preview `2026-10-22_05-02`). Before: tree with F56 to F59 and runner R-29: **2929 passed, 2 failed, 141 ignored** at `5e9847e6` (GT-1m, 21 Oct 2026): the timing test `image_preview_p1::worker_cancels_outdated_jobs_and_ui_stays_fluid` (green on the rerun, BA-117) and `tw186_swap_colors::rectangle_swaps_with_one_row_repeat_and_undo`, where F57's restored toolbox colours and F58's tool-attribute follow met — reconciled in F60 `6ebdc374` (the follow wins when an object is selected; `fp-engine`/`fp-script` and the touched `fp-ui` modules green); interim run on `36874d34` 2918 / 0 / 141. Before: F55 (TW-189, the Q-60 UI test) on top of the F53 fixes and v0.2.6: 2899 / 0 / 141 at `f543d7b9` (GT-1l; the code of preview `2026-10-21_08-16` and of run 32); the F53 fixes: 2892 / 0 / 141 at `e1340a21` (GT-1k); tester feedback batch 4 (F54a to F54d): 2877 / 1 / 141 at `82b0b5b2` (GT-1j, the code of preview `2026-10-21_05-40` and of v0.2.6; one `fp-ui` lib process crash, BA-117, green on the rerun; the failure was an untranslated parameter text, fixed in `1b95dd37`); JPEG 2000 and the AI batch steps: 2852 / 1 / 141 at `3b7386fa` (GT-1i). Batch and export tests also run on macOS and Linux CI (BA-083) |
| Latest release | **v0.2.6** pre-release published on 21 Oct 2026 (from `02481356`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage, with tester feedback batch 4 (Snap to Pixels for the Subselection drags, live retouch result, one brush tip outline, smudge past the bitmap edge, tooltips with tool descriptions and the coordinate tag) plus JPEG 2000 in PDF and the AI batch steps; before: **v0.2.5** (20 Oct 2026); Linux tested in CI only, the Intel macOS slice verified by tools only; the repository is public since 20 Oct 2026 |
| Open findings | 41 fixable rows, 1 open: Q-62 (rules derived from plain-key runs before R-27, to re-measure), plus the unmeasured rests noted in TW-186, TW-193, TW-195 (gradient or pattern tool fills on auto shapes) and Q-61 (TW-185, the Tool Properties tab, declined on 21 Oct 2026); fixed in the night of 22 Oct 2026 after RE spec 59 (RE-27), pending follow-up run 34 and the click-test A4-43 and A4-44: TW-195 (F61, auto shapes take the tool attributes, the Arrow Line and the Measure tool draw their own strokes and set the tool) and the refined follow rules of TW-192 (F61); the runner item TW-196 fixed by R-30, to be confirmed in run 34; Q-63 closed (runner R-31 sets the tool stroke at unit start, cases c and d by F61); fixed on the evening of 21 Oct 2026 after RE spec 58, pending the click-test A4-38 to A4-42: TW-187 (F56), TW-186 and TW-194 (F57), TW-192 (F58, replaces Q-58) and TW-193 (F59, replaces Q-59), TW-192 (colours), TW-193 and TW-194 confirmed by full run 33; closed: Q-61 (measured, tag per tool in F57), the TW-175 rest (measured, equal) and the runner item TW-174 (R-29, confirmed by run 33); closed by run 32: TW-189 (Enter with a group; fixed in F55 and confirmed, pending the click-test), the runner items TW-190 and TW-191 (runner R-28) and Q-60 (runner matter); TW-188 closed by run 31 (runner matter); TW-179 fixed by runner R-27 (host bug: plain named keys were sent as their last letter); fixed on 21 Oct 2026 pending the click-test: batch 4 (TW-180 to TW-184, TW-175, in v0.2.6) and the RE-24 rules TW-173 (confirmed by run 31), TW-176 to TW-178 (F53a/F53b after v0.2.6); TW-151, TW-160 to TW-162 and TW-168 to TW-172 awaiting the click-test and the macOS re-test; TW-146 rest and TW-165 are click-test item A1-19 · 5 deferred rows, 1 open (BA-101, PSD stroke only) · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026; full run 24 classified all of them, including the measurement unit
`xs-select-behind` (9 steps); run 25 repeated 20 of them, run 26 five, run 27 three, run 28 two and run 29 three;
full run 30 classified all 49 again, follow-up run 31 eight of them, follow-up run 32 three and full run 33 all 49.

## Next (in order)

1. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the text selection edits of A1-19, the open
   checks in section A2), including the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4
   features, URL panel menu, typing after a double-click, the crop option, the Linux start, the pen anchor modes, the
   Select Behind hover frame and the History rows of key nudges, A4-15 to A4-18, the screen modes, A4-22, the
   Subselection display, A4-23, tester feedback batch 3, A4-24 to A4-27, a real layered PDF, A4-28, the text
   tool reset, A4-29, JPEG 2000 in PDF, A4-30, the AI batch steps upscale and face blur, A4-31 and A4-32, and the
   parity fixes of 21/22 Oct 2026, A4-38 to A4-44).
2. **macOS re-test of v0.2.4 / v0.2.5 by the tester** (A4-19 to A4-21 on a real Mac).
3. **Preview** `2026-10-21_05-40` (GT-1j) built with tester feedback batch 4 (FB-12 to FB-15 fixed, FB-13 done; FB-16 declined,
   TW-185) and released as pre-release v0.2.6 on 21 Oct 2026; before it preview `2026-10-20_19-06` (GT-1i)
   with JPEG 2000 and the AI batch steps, and v0.2.5 (20 Oct 2026) from the code of preview `2026-10-20_09-25`.
4. **Follow-up run 34** over `tool-autoshapes`, `xs-select-behind`, `tool-path-reshape`, `tool-web`, `menu-modify-a`,
   `toolbox`, `xs-canvas` and `xs-windows-tabs` (the effects of F61, the R-30 reset after reference restarts and the
   R-31 tool stroke at unit start), then its evaluation. In the night of 22 Oct 2026 measurement RE-27 (RE spec 59)
   led to F61 (`132b4941`, `84346fec`: **TW-195**, auto shapes take the tool attributes, the Arrow Line and the Measure
   tool draw their own strokes and set the tool; the refined follow rules of **TW-192**) and the runner changes R-30
   (`924fbfc7`, **TW-196**) and R-31 (`63f1e059`, tool stroke on both sides at unit start); **Q-63** is closed. Q-62
   (rules from plain-key runs) needs a re-measurement with real keys; TW-176, TW-177, TW-186 and TW-187 are not
   reached by any step and stay with the click-test. Before it: full parity run 33 (21/22 Oct 2026, all 49 units,
   preview `2026-10-21_21-47` from `6ebdc374` with F53, F55 and F56 to F60, runner R-29 with real plain keys in every
   unit; 48 / 1 / 0 / 0, proven 1:1 98.0 %, quota 87.1 %, canvas pairs 184 / 209) confirmed TW-193, TW-194, the
   runner item TW-174 and TW-192 for the colours and found TW-195, TW-196 and Q-63; follow-up run 32 (21 Oct 2026,
   3 units, preview `2026-10-21_08-16`, runner R-28; 49 / 0 / 0 / 0) confirmed TW-189 and closed TW-190, TW-191 and
   Q-60, follow-up run 31 (8 units, preview `2026-10-21_06-27`) confirmed TW-173 and closed TW-188, and on the
   evening of 21 Oct 2026 measurement RE-26 (RE spec 58) led to the fixes F56 to F60 and runner R-29.
5. **After full parity run 30** (20/21 Oct 2026, all 49 units, preview `2026-10-20_19-06`; 48 / 1 / 0 / 0, proven
   1:1 98.0 %, quota 87.2 %, canvas pairs 185 / 212): TW-173 (the reference repeats a clone with Ctrl+Y when nothing
   was undone) is fixed (F53a, after RE spec 55 §7) and confirmed by run 31; the runner item TW-174 is fixed by R-29 and the
   questions Q-58 and Q-59 are measured and fixed as TW-192 and TW-193 (21 Oct 2026). The
   tiled canvas (BA-098) showed no one-device-pixel sampling offset at fractional zoom: the zoom shots are
   pixel-identical to the untiled baseline or differ only in label anti-aliasing, with no new offset.
6. **Remaining deferred items:** BA-101 (PSD stroke; JPEG 2000 in PDF done in `974af253`); BA-109 closed
   (`ai.upscale` `9f3a1647`, `ai.faceBlur` `4f811360`).

## What remains

### Fixable remnants (`[F]`)

- Evening of 21 Oct 2026 (RE-26, RE spec 58): fixed and waiting for the click-test A4-38 to A4-42 are TW-186 (swap
  colours with a selection), TW-187 (point-drag threshold 3·√zoom), TW-194 (the Marquee's Escape keeps the bitmap
  selected; was TW-04), TW-192 (script shapes and tool attributes; was Q-58) and TW-193 (Layers right-click; was
  Q-59); full run 33 confirmed TW-193, TW-194 and TW-192 for the colours; Q-61 is closed as measured (tag per tool),
  the TW-175 rest is equal and the runner item TW-174 is fixed by R-29 and confirmed by run 33. Open: Q-62 (rules
  derived from plain-key runs before R-27, DOC-27) and the unmeasured rests of TW-186, TW-193 and Q-61.
- From full run 33, fixed in the night of 22 Oct 2026 after RE-27 (RE spec 59), pending follow-up run 34: TW-195
  (F61 `132b4941`: auto shapes take the tool attributes in full; the Arrow Line draws black `bn_Hard Line` 1 px and
  the Measure tool red `bn_Hard Line` 1 px inside, both set the tool; click-test A4-43; open rest: gradient or pattern
  tool fills on auto shapes), the refined follow rules of TW-192 (F61: only a single path or text, fill none per
  document, the Default colours button keeps the tip, fresh-start stroke `bn_1-Pixel Anti-Aliased` 1 px centred;
  click-test A4-44) and the runner item TW-196 (R-30 `924fbfc7`: unit-start reset repeated after a reference
  restart); the question Q-63 is closed (runner R-31 `63f1e059` sets the tool stroke on both sides at unit start).
- Closed by follow-up run 32: TW-189 (fixed in F55 and confirmed: Enter on the canvas keeps a selected group in both
  programs), Q-60 (runner matter: the runner's click hit the page heading, not the category row) and the runner items
  TW-190 (View ▸ Tooltips off in Firepixel's shots, no tooltip over the flyouts) and TW-191 (no reference-only
  re-selection before Delete). From run 31: the question Q-61 (closed on 21 Oct 2026, see above). Confirmed in run 31: TW-173 (Ctrl+Y repeats a clone in both programs); closed: TW-188 (runner
  matter).
- From full run 30: the runner item TW-174 (the reference's docked panels had other sizes, so `panel-layers/04`
  and `panel-swatches-mixer/19` hit other rows; fixed by R-29) and the questions Q-58 and Q-59 (now TW-192 and TW-193).
- Closed on 20 Oct 2026: U-1 rest (PDF import creates layers from optional content groups; a real
  layered PDF is click-test A4-28), TW-138 rest (R3 measured; the text tool reset of RE spec 56 in F52, click-test
  A4-29; known: the reference writes two History rows for a new text, Firepixel one) and the runner item TW-167
  (run 29).
- Moved to the click-test A1-19 on 20 Oct 2026 (runner measurement gaps, not deviations): TW-146 rest (text selection
  and size in `tool-text/07, 11, 13`) and TW-165 (the text tool reset derived from it).
- Confirmed and closed in run 28 (runner R-25): TW-166 (the reference leaves full screen by F; no `runner error` in
  `toolbox`).
- Confirmed and closed in run 27 (runner R-24): TW-159 rest (both programs keep the Freeform tool and read size 72
  after the size keys), TW-163 (the reference's Standard mode read from the window) and TW-164 (Freeform head on the
  reference side).
- Confirmed and closed in run 26: TW-158 (three screen modes, full screen shows only the canvas on black, F49), the
  runner item TW-159 except (c) and the entry part of TW-146 rest; TW-138 rest (F49) shows no deviation in `tool-text`
  and `menu-text`.
- Tester feedback, second batch (macOS): TW-160 / FB-04 (frozen window after minimise/restore), TW-161 / FB-05 (Cmd+Q
  through the Quit action with the save prompt; own macOS app menu since `e11f8d1b`, CI build green) and TW-162 / FB-03 (native title
  bar with traffic lights) are fixed, only type-checked for macOS, and wait for A4-19 to A4-21.
- Confirmed and closed in run 25: TW-148 (consecutive key nudges merge into one History row, F48), TW-157 (Select
  Behind hover line and no frame after a click, F48), TW-145 (the next text after a size edit is 72 pt in both
  programs), the runner items TW-153 to TW-156 and the TW-149 leftover. Full run 24 closed TW-149, TW-150 and TW-152.
- Tester feedback of 19 Oct 2026 (E-Man): TW-151 / FB-01 (pen tool anchor modes: the click on the last anchor
  retracts its outgoing handle) is fixed and kept until the click-test; TW-152 / FB-02 (Select Behind hover frame) is
  confirmed by run 24.
- Tester feedback of 20 Oct 2026 (E-Man): TW-168 / FB-06 (the Subselection tool shows the path outline, the anchors
  and the handles of selected points on vector objects instead of the bounding box) is fixed and waits for the
  hands-on comparison A4-23.
- Tester feedback batch 3 of 20 Oct 2026 (E-Man, RE spec 55): TW-169 / FB-07 (one History step per Pen point),
  TW-170 / FB-08 (Redo is redo only), TW-171 / FB-09, FB-10 (inspector opacity and stroke width sliders apply once,
  with a live canvas preview) and TW-172 / FB-11 (free Subselection point drags, View ▸ Snap to Pixels) are fixed in
  F51a to F51c and wait for A4-24 to A4-27.

### Deferred expansion items (`[V]`)

- **Batch studio:** none open. Done on 20 Oct 2026: `ai.upscale` (Real-ESRGAN, `9f3a1647`), `ai.faceBlur` (Haar
  cascade, `4f811360`), EXIF/XMP written into
  AVIF/HEIF and JPEG XL (BA-108) and lossy LZW with an SSIM bound plus cancel inside one encode in the GIF
  optimiser (BA-110, `lossyMinSsim`). Done in wave 3: 27 new steps (file, document, perspective with auto-straighten,
  tiles, noise, LUT, style, JPEG/web-safe, boomerang/crossfade, colour depth/profile, multi-page, metadata export,
  contact sheet, script steps with opt-ins), chained collect steps, TIFF/AVIF/HEIF/JXL metadata reading, TIFF
  EXIF/GPS/IPTC writing, MCP refusal of external/IO scripts with a deep script sandbox, opt-in GIF optimiser modes
  (figures in [docs/BATCH.md](docs/BATCH.md)), per-file progress with ETA, file order, the pro filter bar,
  drag/keyboard sorting, Save as Template and Help in the card menu.
- **PDF/PSD export:** JPEG 2000 in PDF is real JPEG 2000 (`/JPXDecode`, `974af253`). Done in wave 3: selectable PDF
  text, PSD layers per object with text layers, vector masks and layer effects.
- Done in wave 4: HTML setup (Set Defaults, external `styles.css`, Generic HTML5 style, nav-bar tests), CMD-1
  (screen capture, symbol scripts, shared palette), the ffmpeg row in Preferences, the optional AI hover preview
  and the brush preview in stacking order.
- Brush pressure and speed from a real tablet. Done on 20 Oct 2026: the batch and export tests run on macOS and
  Linux CI and all golden hashes are equal on the three systems (BA-083).

### Not feasible / informational (`[N]`)

Differences that are accepted or cannot be closed, e.g. PNG file size (+0.6 % for PNG 32 because the pure-Rust
deflate differs from the original's zlib; row filters are identical), behaviour that depends on the reference
application's embedded panel technology, and runner limitations. Full list in
[docs/FINDINGS.md](docs/FINDINGS.md).

## Decisions taken (17 Oct 2026)

| Topic | Decision |
|---|---|
| Symbol editing as a true view inside the same document | **No** — keep the internal symbol view (own session document) |
| Compact effect popup instead of effect dialogs | **No** — keep the effect dialogs |
| Symbol origin when converting to a symbol | **Centre** of the object bounds (older Firepixel files are converted on load) |
| Opacity storage | **8-bit** like the original (30 % → 77/255) |
| File Info dialog | Hide the tabs that are not implemented (only "Description") |
| CI | **On demand only** (`workflow_dispatch`, called from the pre-release workflow) |
| XMP identifier in PNG | Keep the neutral standard (`XMP` key, Dublin Core only) |
| C zlib backend for smaller PNGs | **No** — size difference stays informational |
| Gradient handles with the pointer tool | Keep: pointer shows and drags the handles, live fill while dragging |
