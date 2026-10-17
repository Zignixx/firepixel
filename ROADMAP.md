# Roadmap

Where Firepixel stands and what remains. Figures as of **18 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **44 / 48** (2 real deviations, 2 unclear, 0 under-measured; run 15) |
| Proven 1:1 | **91.7 %** (44 / 48) |
| Measurement quota, all 48 units | **84.8 %** (1041 / 1228 steps; was 84.1 %) |
| Canvas image pairs within 1 % | about 177 / 201 |
| Tests | 2486 green, 0 red, 133 ignored |
| Latest release | v0.2.0 pre-release (17 Oct 2026, Windows x64, macOS arm64) |
| Open findings | 7 fixable · 20 deferred · 17 informational · 830+ closed |

## Next (in order)

1. **Behaviour descriptions** from measurements: the original's swatch file loader and sort key (TW-113), the
   inspector with several selected objects (TW-120) and the button-instance inspector (TW-121).
2. **Package F31** (Firepixel): implement TW-113, TW-120 and TW-121 from those descriptions, with tests that pin
   the measured values.
3. **Runner R-11:** read back a swatch colour after the reset (TW-115); keep drag distances clear of the snap
   tolerance and check Ctrl+wheel points against the view (TW-119); remaining locator gaps (`slice:0`,
   export-area dialog, guide double-click and guide drag off the canvas in `xs-canvas`).
4. **Questions** Q-49 (hex field), Q-51 (inspector tab), Q-52 (swatch add) and Q-53 (Shift-drag, TW-118).
5. **Comparison run 16** for `panel-swatches-mixer`, `inspector-group-symbol`, `xs-shortcuts` and `xs-canvas`.
6. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the run-14/15 checks in section A2).
7. **Deferred expansion block** (see below), then a new full parity run over all 48 units and Linux builds.

## What remains

### Fixable remnants (`[F]`, small, mostly need a measurement first)

- Swatches: the original keeps the `.act` padding entries and sorts black last (TW-113, behaviour description first).
- Inspector: show the last selected object's fill and stroke with several objects selected (TW-120); button
  instance properties (TW-121).
- Pointer tool: Shift-drag of a selected object (TW-118, Q-53 first).
- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Runner: swatch reset check (TW-115), snap-tolerance drag and wheel point in `xs-canvas` (TW-119).

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
