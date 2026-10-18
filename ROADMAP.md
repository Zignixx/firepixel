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
| Tests | 2510 green, 0 red, 133 ignored |
| Latest release | v0.2.1 pre-release (18 Oct 2026, Windows x64, macOS universal arm64 + x86_64) |
| Open findings | 1 fixable · 13 deferred · 17 informational · 840+ closed |

## Next (in order)

1. **Full parity run over all 48 units** (in progress tonight) with runner R-12 and the Firepixel shortcut set in
   the runner profile; it also checks the fixes after run 16 (TW-122, TW-123 including the app-wide 33 px menu arrow
   column, TW-115/124, Q-49, Q-55, Q-56).
2. **Evaluation** of that run: new tweaks and questions, a review of the `xs-canvas` canvas pairs, updated figures.
3. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2).
4. **Remaining deferred items** (see below): missing batch steps (being implemented), batch metadata, batch UI
   conveniences, PDF/PSD export details, HTML setup, CMD-1, AI mask preview; then Linux builds.

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).

### Deferred expansion items (`[V]`)

- **Batch studio:** streamed long videos, remaining UI conveniences (filters, drag sorting, per-file progress),
  AVIF/JXL/HEIF/TIFF metadata, the remaining steps of the batch spec (see BA-109 in FINDINGS), GIF optimiser target
  (−30 % at equal SSIM). Done since run 16: size token after resize, Ctrl+C cancel in the CLI, close confirmation
  while a batch runs, extra outputs per item (FW-PNG, sprite-sheet atlas), WebP/AVIF alpha options, nine new steps.
- **PDF/PSD export:** PDF "enable text selection" (invisible text layer), JPEG 2000 in PDF, PSD options for effects,
  vector and text layers.
- **HTML setup:** "set defaults", external CSS file, navigation bar HTML.
- **CMD-1:** screenshot command, symbol scripts and the shared web palette currently show notices only.
- **AI mask preview:** live object-mask preview while hovering with the object selection tool.
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
