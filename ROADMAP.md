# Roadmap

Where Firepixel stands and what remains. Figures as of **18 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **44 / 48** (2 real deviations, 2 unclear, 0 under-measured; runs 19 and 18 on top of full run 17) |
| Proven 1:1 | **91.7 %** (44 / 48) |
| Measurement quota, all 48 units | **86.9 %** (1066 / 1226 steps, newest run per unit; run 17 alone: 86.3 %) |
| Canvas image pairs within 1 % | 174 / 202 |
| Tests | Run on `2d64547a` pending; last full run: 2695 passed, 1 failed (missing command text, fixed in `0f96dd71`), 134 ignored at `6e2ad3b1` |
| Latest release | **v0.2.2** pre-release (18 Oct 2026, tag from `c031a987`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage; Linux tested in CI only, the Intel macOS slice verified by tools only |
| Open findings | 5 fixable · 5 deferred · 20 informational · 850+ closed |

## Next (in order)

1. **Run 20** of `menu-edit`, `xs-shortcuts`, `tool-text`, `xs-canvas`, `panel-library-styles` and `xs-preferences`
   once the menu enabled-state fix (TW-141, package F40, in progress) is on `main`: confirms the fixes after run 19
   (TW-137 History rows, TW-138 text defaults, TW-139 live read-back), the crop option (TW-134) and runner R-15
   (TW-140).
2. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2), including
   the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4 features, URL panel menu, typing after
   a double-click, the crop option, the Linux start).
3. **Remaining parity items:** "stretch" / "justified" on point text (TW-133 rest) and the *Text* menu remnant of
   the text tool defaults (TW-138 rest).
4. **Remaining deferred items:** BA-101 (JPEG 2000 in PDF, PSD stroke), BA-108 (writing AVIF/HEIF/JXL metadata
   boxes), BA-110 rest (lossy LZW with an SSIM bound), BA-083 (resampler hashes on another OS), U-1 PDF import
   layers.

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- TW-133 rest ("stretch" / "justified" on point text), TW-138 rest (*Text* menu commands and the text tool
  defaults), TW-140 (runner R-15 on `main`, to confirm in run 20) and TW-141 (menu enabled state, Rust fix F40 in
  progress). Fixed after run 19 and to confirm in run 20: TW-137 (History rows for Copy, Cut, Select All,
  Deselect), TW-138 (text tool defaults from the inspector) and TW-139 (live read-back while a text is edited);
  TW-136 closed as no deviation; TW-134 (crop option) waits for the `xs-preferences` run.

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
