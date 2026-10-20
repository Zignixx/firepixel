# Roadmap

Where Firepixel stands and what remains. Figures as of **20 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **49 / 49** (0 real deviations, 0 unclear, 0 under-measured; run 29 over 3 units on top of runs 28, 27, 26, 25 and 24, 20 Oct 2026) |
| Proven 1:1 | **100.0 %** (49 / 49; 100.0 % over the 48 units before run 24; runs 26 to 29: 100.0 %; run 25: 98.0 %; run 24: 93.9 %) |
| Measurement quota, all 49 units | **87.3 %** (1077 / 1233 steps; 87.3 % over the 48 old units; unchanged in runs 27 to 29; run 24: 87.0 %; run 17: 86.3 %) |
| Canvas image pairs within 1 % | 185 / 212 (unchanged in runs 28 and 29; run 26: 186 / 211; `tool-text/14, 16` above 1 % because no text tool is reset after the reference's size step fails, TW-165) |
| Tests | Merged tree after the roadmap items of 20 Oct 2026: **2818 passed, 1 failed, 136 ignored** at `612b34bd` (GT-1h, 20 Oct 2026, the code of preview `2026-10-20_09-25`): 2674 + 144 from the `fp-ui` lib binary, whose process crashed once with an access violation next to `canvas::tests::space_during_drag_moves_the_shape_in_ui` and passed on the rerun; the one failure is the timing test `image_preview_p1::worker_cancels_outdated_jobs_and_ui_stays_fluid`, green on the rerun (see BA-117); per merge on `main`: F52 2809 / 0 / 134, PERF 2802 / 0 / 136, BA-110 2803 / 0 / 134 (passed / failed / ignored). Last full run: 2797 passed, 0 failed, 134 ignored at `901f0a56`. Batch and export tests also run on macOS and Linux CI, all golden hashes equal (BA-083) |
| Latest release | **v0.2.4** pre-release published on 19 Oct 2026 (tag at `1f26223c`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage, with the three screen modes, the text default rules and the macOS fixes; before: **v0.2.3** (19 Oct 2026); Linux tested in CI only, the Intel macOS slice verified by tools only |
| Open findings | 11 fixable rows, none open (U-1 rest and TW-138 rest fixed on 20 Oct 2026; TW-151, TW-160 to TW-162 and TW-168 to TW-172 awaiting the click-test and the macOS re-test; TW-146 rest and TW-165 are click-test item A1-19) · 5 deferred rows, 2 open (BA-101, BA-109) · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026; full run 24 classified all of them, including the measurement unit
`xs-select-behind` (9 steps); run 25 repeated 20 of them, run 26 five, run 27 three, run 28 two and run 29 three.

## Next (in order)

1. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the text selection edits of A1-19, the open
   checks in section A2), including the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4
   features, URL panel menu, typing after a double-click, the crop option, the Linux start, the pen anchor modes, the
   Select Behind hover frame and the History rows of key nudges, A4-15 to A4-18, the screen modes, A4-22, the
   Subselection display, A4-23, tester feedback batch 3, A4-24 to A4-27, a real layered PDF, A4-28, and the text
   tool reset, A4-29).
2. **macOS re-test of v0.2.4 by the tester** (A4-19 to A4-21 on a real Mac).
3. **Preview** `2026-10-20_09-25` (GT-1h) built; the next pre-release waits for the tester's macOS re-test.
4. **Next full parity run** over all 49 units; it also checks whether the tiled canvas (BA-098) shifts a sampling
   position by one device pixel at fractional zoom. No runner item is open.
5. **Remaining deferred items:** BA-101 (PSD stroke; JPEG 2000 in PDF done in `974af253`); BA-109 closed
   (`ai.upscale` `9f3a1647`, `ai.faceBlur` `4f811360`).

## What remains

### Fixable remnants (`[F]`)

- None open. Closed on 20 Oct 2026: U-1 rest (PDF import creates layers from optional content groups; a real
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
