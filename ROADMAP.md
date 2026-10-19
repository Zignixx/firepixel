# Roadmap

Where Firepixel stands and what remains. Figures as of **19 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **46 / 49** (2 real deviations, 1 unclear, 0 under-measured; full run 24, 19 Oct 2026) |
| Proven 1:1 | **93.9 %** (46 / 49; 95.8 % over the 48 units before run 24) |
| Measurement quota, all 49 units | **87.0 %** (1074 / 1235 steps; 86.9 % over the 48 old units; run 17: 86.3 %) |
| Canvas image pairs within 1 % | 164 / 210 (`tool-autoshapes` 7 / 27 from a runner artefact, TW-153) |
| Tests | Full run pending on `ace7873c`; last full run 2761 passed, 0 failed, 134 ignored at `44d7c5f5` (preview `2026-10-19_06-06`) |
| Latest release | **v0.2.3** published on 19 Oct 2026 (state of preview `2026-10-19_06-06`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage, all assets verified; before: **v0.2.2** pre-release (18 Oct 2026, tag from `c031a987`); Linux tested in CI only, the Intel macOS slice verified by tools only |
| Open findings | 11 fixable rows (3 open: U-1 rest, TW-138 rest, TW-145; 8 fixed or closed awaiting run 25 or the click-test) · 5 deferred · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026; full run 24 classified all of them, including the measurement unit
`xs-select-behind` (9 steps).

## Next (in order)

1. **Run 25 of the R-22 units** after a full test run and a preview build from `ace7873c`: `panel-pages-states-history`,
   `xs-select-behind`, `tool-text`, `tool-autoshapes`, `toolbox`, `menu-edit`, `menu-text`, `menu-view`,
   `tool-path-reshape` and `inspector-stroke`, to confirm package F48 (TW-148: consecutive key nudges merge into one
   History row; TW-157: crisp Select Behind hover line, no frame right after a click) and runner R-22 (TW-146 rest,
   TW-153 to TW-156).
2. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2), including
   the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4 features, URL panel menu, typing after
   a double-click, the crop option, the Linux start, the pen anchor modes, the Select Behind hover frame and the
   History rows of key nudges, A4-15 to A4-18).
3. **Remaining `[F]` items:** U-1 rest (PDF import layers), the *Text* menu remnant of the text tool defaults
   (TW-138 rest) and the size rule for new text (TW-145, once the reference's text entry works in `tool-text/11`).
4. **Remaining deferred items:** BA-101 (JPEG 2000 in PDF, PSD stroke), BA-108 (writing AVIF/HEIF/JXL metadata
   boxes), BA-110 rest (lossy LZW with an SSIM bound) and BA-083 (resampler hashes on another OS).

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Open: TW-138 rest (*Text* menu commands and the text tool defaults) and TW-145 (size of a new text after a
  partial-selection edit, not yet compared).
- Fixed, to confirm in run 25: TW-148 (consecutive key nudges merge into one History row, F48) and TW-157 (Select
  Behind hover line on vector paths, frame after a click, F48); runner R-22 for TW-146 rest (text entry retries),
  TW-153 (fill colour after the text reset), TW-155 (arrow keys outside nudges, History read-back) and TW-156
  (screen-mode steps); TW-154 closed as not comparable (no menu entry or key in the reference). Full run 24 closed
  TW-149, TW-150 and TW-152.
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
