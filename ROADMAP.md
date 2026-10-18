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
| Latest release | v0.2.0 pre-release (17 Oct 2026, Windows x64, macOS arm64); CI now builds a universal macOS bundle |
| Open findings | 5 fixable · 20 deferred · 17 informational · 830+ closed |

## Next (in order)

1. **Runner R-12:** probe and read back Firepixel's swatches at the 9 px pitch (TW-115); serialise objects over the
   bridge, read panel pop-up menus instead of the panel, reference-safe read-back scripts (no `Array.map`), allow
   drag end points outside the view, guide double-click and export-area dialog in the reference (TW-124).
2. **Behaviour description** of text style shortcuts on a selected text object (TW-122, Q-54), then a small
   package with the fix and the Optimize colour-table context menu width (TW-123).
3. **Questions** Q-49 (hex field), Q-51 (inspector tab), Q-55 (caret after a text-tool click) and Q-56 (Snap to
   Pixel without a selection).
4. **Comparison run 17** for `xs-shortcuts`, `xs-canvas`, `panel-swatches-mixer` and `panel-optimize`; one review
   of the `xs-canvas` canvas pairs.
5. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the run-15/16 checks in section A2).
6. **Deferred expansion block** (see below), then a new full parity run over all 48 units and Linux builds.

## What remains

### Fixable remnants (`[F]`, small, mostly need a measurement first)

- Text style shortcuts on a selected text object (TW-122, behaviour description first).
- Optimize panel: colour-table context menu about twice as wide as the original's (TW-123).
- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Runner: swatch cell pitch for Firepixel (TW-115) and the remaining gaps of run 16 (TW-124).

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
- Inspector tab after the Symbol Properties panel (TW-111, Q-51; the runner works around it).
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
