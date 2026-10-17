# Roadmap

Where Firepixel stands and what remains. Figures as of **17 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **42 / 48** (4 real deviations, 1 unclear, 1 under-measured; run 14) |
| Proven 1:1 | **87.5 %** (89.4 % of the 47 sufficiently measured units) |
| Measurement quota, all 48 units | **84.1 %** (1031 / 1226 steps; was 83.5 %) |
| Canvas image pairs within 1 % | about 179 / 200 |
| Tests | 2486 green, 0 red, 133 ignored |
| Latest release | v0.2.0 pre-release (17 Oct 2026, Windows x64, macOS arm64) |
| Open findings | 7 fixable · 21 deferred · 17 informational · 830+ closed |

## Next (in order)

1. **Package F29** (Firepixel): the colour pop-up commits the swatch under the pointer on release instead of on
   press (TW-112); the `.act` swatch import honours the colour count and transparency trailer (TW-113); table look
   of the Symbol Properties panel (TW-110).
2. **Runner R-10:** reset the swatch set on both sides at unit start (TW-115), decode reference tooltips correctly
   and map the spiral's `center` handle (TW-116), activate Properties before each inspector shot (TW-111), press
   beside R1's gradient handle for the Alt-drag duplicate (TW-97), fit the reference to the large canvas
   (`xs-canvas`, Q-48), and the remaining locator gaps in `export-results` and `xs-shortcuts`.
3. **Behaviour description RE-11** from measurements: the animated GIF quantiser and its sort order (TW-114), then
   the fix; questions Q-51 (inspector tab), Q-52 (swatch load add/replace) and Q-49 (hex field).
4. **Comparison run 15** for the units touched by F29 and R-10 (`color-popup`, `panel-swatches-mixer`,
   `inspector-group-symbol`, `xs-shortcuts`, `xs-canvas`, `tool-autoshapes`, `export-results`).
5. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the run-14 checks in section A2).
6. **Deferred expansion block** (see below), then a new full parity run over all 48 units and Linux builds.

## What remains

### Fixable remnants (`[F]`, small, mostly need a measurement first)

- Colour pop-up: commit on release, not on press (TW-112, F29).
- Swatches: `.act` import ignores the colour count (TW-113, F29).
- Symbol Properties panel: table look (TW-110, F29).
- Alt-drag duplicate (TW-97/Q-50): press point in the runner first (R-10).
- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- Runner: swatch start set (TW-115), tooltip decoding and spiral handle (TW-116).

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
- Animated GIF export: adaptive palette of the first frame (TW-114, needs RE-11 first); inspector tab after the
  Symbol Properties panel (TW-111, Q-51).
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
