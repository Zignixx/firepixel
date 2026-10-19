# Roadmap

Where Firepixel stands and what remains. Figures as of **19 Oct 2026**; details in
[docs/STATUS.md](docs/STATUS.md), [docs/PARITY.md](docs/PARITY.md) and [docs/FINDINGS.md](docs/FINDINGS.md).

## Stage

**Feature-complete preview, parity polishing.** All menus, tools, panels, the property inspector and the dialogs
exist and work; FW-PNG round-trips; CLI, JSF and MCP drive everything.

| Measure | Value |
|---|---|
| Parity units equal except for design | **46 / 48** (0 real deviations, 2 unclear, 0 under-measured; runs 23, 22 and 18 on top of full run 17) |
| Proven 1:1 | **95.8 %** (46 / 48; run 22: 97.9 %) |
| Measurement quota, all 48 units | **86.9 %** (1065 / 1226 steps, newest run per unit; run 17 alone: 86.3 %) |
| Canvas image pairs within 1 % | 175 / 201 |
| Tests | 2748 passed, 1 failed (`fp-i18n`, fixed in `bea53d1c`), 134 ignored at `cd86ce75` (preview `2026-10-19_01-32`) |
| Latest release | **v0.2.2** pre-release (18 Oct 2026, tag from `c031a987`): Windows x64 ZIP, macOS universal ZIP (arm64 + x86_64), Linux x86_64 tar.gz and AppImage; Linux tested in CI only, the Intel macOS slice verified by tools only |
| Open findings | 9 fixable rows (7 open, 2 closed awaiting the click-test) · 5 deferred · 20 informational · 850+ closed |

Unit count: 49 since 19 Oct 2026. The figures above are over the 48 classified units; the new measurement unit
`xs-select-behind` (9 steps) has its first comparison run pending.

## Next (in order)

1. **Run 24:** preview build from `main` (pen anchor modes, Select Behind hover frame), then a follow-up run with
   runner R-21 over `tool-text`, `panel-pages-states-history`, `xs-shortcuts` and the first run of
   `xs-select-behind`. It confirms TW-146 rest, TW-149, TW-150 and TW-152, decides TW-148 from the logged History row
   labels (align F46 if the reference merges consecutive key nudges) and compares the size of a new text after a
   partial-selection edit (TW-145).
2. **Full run** over all 49 units, then release **v0.2.3**.
3. **Human click-test** of [docs/VERIFICATION.md](docs/VERIFICATION.md) (embedded panel dialogs, real keyboard
   paths, brush strokes with a real mouse, drag and drop between windows, the open checks in section A2), including
   the wave-3 Batch Studio UI, the PDF/PSD export options and section A4 (wave-4 features, URL panel menu, typing after
   a double-click, the crop option, the Linux start, the pen anchor modes and the Select Behind hover frame, A4-15 to
   A4-17).
4. **Remaining parity items:** the *Text* menu remnant of the text tool defaults (TW-138 rest) and the size rule for
   new text (TW-145, after run 24).
5. **Remaining deferred items:** BA-101 (JPEG 2000 in PDF, PSD stroke), BA-108 (writing AVIF/HEIF/JXL metadata
   boxes), BA-110 rest (lossy LZW with an SSIM bound), BA-083 (resampler hashes on another OS), U-1 PDF import
   layers.

## What remains

### Fixable remnants (`[F]`)

- PDF import: no layers (the pure-Rust PDF reader exposes no optional content groups).
- TW-138 rest (*Text* menu commands and the text tool defaults), TW-145 (size of a new text after a
  partial-selection edit), TW-146 rest (reference-side text entry, runner R-21), TW-148 (History rows of consecutive
  nudges, RE spec check), TW-149 (text tool font at unit start, runner) and TW-150 (step limit, runner). Run 23
  closed TW-147 (Pointer tool through the flyout, Escape keeps the selection in both programs, paste read-back,
  `MaxUndos`). Runner R-21 fixes TW-146 rest, TW-149 and TW-150 (confirm in run 24) and logs the History row labels
  for TW-148 (decide after run 24).
- Tester feedback of 19 Oct 2026 (E-Man), fixed after RE spec 52 and kept until the click-test: TW-151 / FB-01 (pen
  tool anchor modes: the click on the last anchor retracts its outgoing handle) and TW-152 / FB-02 (Select Behind
  hover frame on the object a click would select).

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
