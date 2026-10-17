# Roadmap

Where Firepixel stands and what remains. Figures as of **17 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **42 / 48** (2 real deviations, 2 unclear, 2 under-measured by runner bugs fixed in R-9) |
| Proven 1:1 | **88 %** (91 % of the sufficiently measured units) |
| Canvas image pairs within 1 % | 155 / 196 |
| Tests | 2486 green, 0 red, 133 ignored |
| Open findings | 4 fixable · 19 deferred · 16 informational · 830+ closed |

## Next (in order)

1. **Comparison run 14** for the units touched by F28 and runner R-9: `xs-shortcuts` (Alt-drag with the new
   pointer log, TW-97/Q-50), `inspector-group-symbol` (symbol properties header, TW-104), `xs-canvas` and
   `export-results` (under-measured by runner bugs fixed in R-9), `tool-autoshapes` (start colours, TW-107),
   `color-popup` (hex field, Q-46/Q-49).
2. **Behaviour descriptions, then fixes:** animated GIF palette contents and order (TW-108), half-pixel rounding of
   a library drop (TW-100).
3. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows).
4. **Pre-release v0.2.0** (Windows) on request; CI stays on demand. Then a new full parity run over all 48 units.
5. **Deferred expansion block** (see below), then macOS and Linux builds.

## What remains

### Fixable remnants (`[F]`, small, mostly need a measurement first)

- Animated GIF export: palette contents and order of the first frame (TW-108).
- Library drop at a half pixel: rounding (TW-100).
- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Alt-drag duplicate (TW-97/Q-50): pending comparison run 14.

### Deferred expansion items (`[V]`)

- **Batch studio:** size token after resize/crop, Ctrl+C cancel in the CLI, several outputs per item, streamed long
  videos, close-window confirmation while a batch runs, remaining UI conveniences (filters, drag sorting,
  per-file progress), WebP/AVIF alpha options, AVIF/JXL/HEIF/TIFF metadata, missing steps (subject centre, alpha
  clean-up, vignette, contact sheet), GIF optimiser target (−30 % at equal SSIM).
- **PDF/PSD export:** PDF "enable text selection" (invisible text layer), JPEG 2000 in PDF, PSD options for effects,
  vector and text layers.
- **HTML setup:** "set defaults", external CSS file, navigation bar HTML.
- **CMD-1:** screenshot command, symbol scripts and the shared web palette currently show notices only.
- **AI mask preview:** live object-mask preview while hovering with the object selection tool.
- Brush pressure and speed from a real tablet; resampler golden hashes on other operating systems (needs CI).
- Internal German test-area identifiers in the i18n QA tests.

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
