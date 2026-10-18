# Roadmap

Where Firepixel stands and what remains. Figures as of **18 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **46 / 48** (0 real deviations, 2 unclear, 0 under-measured; run 16) |
| Proven 1:1 | **95.8 %** (46 / 48) |
| Measurement quota, all 48 units | **85.0 %** (1042 / 1226 steps; was 84.8 %) |
| Canvas image pairs within 1 % | about 177 / 201 |
| Tests | full workspace run green at `e38dc531` (fp-ui 854); re-run including V-1 in progress (last full count 2510 green, 0 red, 133 ignored) |
| Latest release | v0.2.1 pre-release (18 Oct 2026, Windows x64, macOS universal arm64 + x86_64) |
| Open findings | 1 fixable · 5 deferred · 20 informational · 845+ closed |

## Next (in order)

1. **Evaluation of full parity run 17** over all 48 units (runner R-12, Firepixel shortcut set in the runner
   profile): check the fixes after run 16 (TW-122, TW-123 including the app-wide 33 px menu arrow column,
   TW-115/124, Q-49, Q-55, Q-56), new tweaks and questions, a review of the `xs-canvas` canvas pairs, updated
   figures.
2. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2), including
   the wave-3 Batch Studio UI, the PDF/PSD export options and the wave-4 features in section A4.
3. **Remaining deferred items:** BA-101 (JPEG 2000 in PDF, PSD stroke), BA-108 (writing AVIF/HEIF/JXL metadata
   boxes), BA-110 rest (lossy LZW with an SSIM bound), BA-083 (resampler hashes on another OS), U-1 PDF import
   layers; then Linux builds.

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).

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
