# Roadmap

Where Firepixel stands and what remains. Figures as of **19 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **48 / 49** (1 real deviation, 0 unclear, 0 under-measured; run 25 over 20 units on top of full run 24, 19 Oct 2026) |
| Proven 1:1 | **98.0 %** (48 / 49; 97.9 % over the 48 units before run 24; run 24: 93.9 %) |
| Measurement quota, all 49 units | **87.3 %** (1075 / 1232 steps; 87.2 % over the 48 old units; run 24: 87.0 %; run 17: 86.3 %) |
| Canvas image pairs within 1 % | 186 / 211 (run 24: 164 / 210; `tool-autoshapes` back to 26 / 27, TW-153) |
| Tests | Last full run: 2762 passed, 0 failed, 134 ignored at `a225a01c` (preview `2026-10-19_12-16`); run on `439a3ec5` pending |
| Latest release | **v0.2.3** published on 19 Oct 2026 (state of preview `2026-10-19_06-06`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage, all assets verified; before: **v0.2.2** pre-release (18 Oct 2026, tag from `c031a987`); Linux tested in CI only, the Intel macOS slice verified by tools only |
| Open findings | 9 fixable rows (2 open: U-1 rest, TW-146 rest; TW-138 rest closed except R3; TW-158, TW-159 fixed, to confirm in run 26; TW-151 and TW-160 to TW-162 awaiting the click-test) · 5 deferred · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026; full run 24 classified all of them, including the measurement unit
`xs-select-behind` (9 steps); run 25 repeated 20 of them.

## Next (in order)

1. **Tests and preview:** full workspace test run on `439a3ec5` (runner R-23, package F49 with the three screen modes
   and the text preference rules, the macOS fixes FB-03 to FB-05), then a new preview build.
2. **Run 26** over `toolbox`, `xs-shortcuts`, `tool-path-reshape`, `tool-text` and `menu-text` to confirm TW-158,
   TW-159, TW-146 rest and TW-138.
3. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2), including
   the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4 features, URL panel menu, typing after
   a double-click, the crop option, the Linux start, the pen anchor modes, the Select Behind hover frame and the
   History rows of key nudges, A4-15 to A4-18, the screen modes, A4-22) and the macOS items A4-19 to A4-21 on a real
   Mac.
4. **Remaining `[F]` items:** U-1 rest (PDF import layers) and the unmeasured R3 note of TW-138 (indent and spacing
   as text defaults).
5. **Remaining deferred items:** BA-101 (JPEG 2000 in PDF, PSD stroke), BA-108 (writing AVIF/HEIF/JXL metadata
   boxes), BA-110 rest (lossy LZW with an SSIM bound) and BA-083 (resampler hashes on another OS).

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Open: the runner item TW-146 rest (R-23 retries, confirm in run 26) and the unmeasured R3 note of TW-138.
- Fixed after run 25, to confirm in run 26: TW-158 (three screen modes, full screen shows only the canvas on black,
  F49), TW-138 rest (inspector never writes the baseline shift default, font changes update the recent font list,
  a style applied to text writes the text defaults; F49) and the runner item TW-159 (R-23).
- Tester feedback, second batch (macOS): TW-160 / FB-04 (frozen window after minimise/restore), TW-161 / FB-05 (Cmd+Q
  through the Quit action with the save prompt; a native app menu is in progress) and TW-162 / FB-03 (native title
  bar with traffic lights) are fixed, only type-checked for macOS, and wait for A4-19 to A4-21.
- Confirmed and closed in run 25: TW-148 (consecutive key nudges merge into one History row, F48), TW-157 (Select
  Behind hover line and no frame after a click, F48), TW-145 (the next text after a size edit is 72 pt in both
  programs), the runner items TW-153 to TW-156 and the TW-149 leftover. Full run 24 closed TW-149, TW-150 and TW-152.
- Tester feedback of 19 Oct 2026 (E-Man): TW-151 / FB-01 (pen tool anchor modes: the click on the last anchor
  retracts its outgoing handle) is fixed and kept until the click-test; TW-152 / FB-02 (Select Behind hover frame) is
  confirmed by run 24.

### Deferred expansion items (`[V]`)

- **Batch studio:** `ai.upscale` and `ai.faceBlur`, writing AVIF/HEIF/JXL metadata boxes, lossy LZW with an SSIM
  bound in the GIF optimiser. Done in wave 3: 27 new steps (file, document, perspective with auto-straighten,
  tiles, noise, LUT, style, JPEG/web-safe, boomerang/crossfade, colour depth/profile, multi-page, metadata export,
  contact sheet, script steps with opt-ins), chained collect steps, TIFF/AVIF/HEIF/JXL metadata reading, TIFF
  EXIF/GPS/IPTC writing, MCP refusal of external/IO scripts with a deep script sandbox, opt-in GIF optimiser modes
  (figures in [docs/BATCH.md](docs/BATCH.md)), per-file progress with ETA, file order, the pro filter bar,
  drag/keyboard sorting, Save as Template and Help in the card menu.
- **PDF/PSD export:** JPEG 2000 in PDF (no encoder; written as JPEG and flagged). Done in wave 3: selectable PDF
  text, PSD layers per object with text layers, vector masks and layer effects.
- Done in wave 4: HTML setup (Set Defaults, external `styles.css`, Generic HTML5 style, nav-bar tests), CMD-1
  (screen capture, symbol scripts, shared palette), the ffmpeg row in Preferences, the optional AI hover preview
  and the brush preview in stacking order.
- Brush pressure and speed from a real tablet; resampler golden hashes on other operating systems (needs CI).

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
