# Open findings

Last updated: 2026-10-18

This file lists the findings that are still **open** in Firepixel: known bugs, deviations from the reference application, postponed features and accepted limitations. Closed findings are not kept here; their history lives in the commit log.

## How findings are numbered

Every finding keeps the ID it was given when it was first recorded, so it can be traced through commits, tests and specifications:

- `BA-nnn` – item from the consolidated backlog of open findings (one BA number can cover several related entries).
- `TW-nn` – deviation found in a side-by-side comparison run against the reference application.
- `Nnn` – finding reported directly by the user (for example `N11`).
- Other tags such as `F24`, `U-1`, `L-3`, `K-7` or `E6` name the work package or comparison step that found the issue. Where a finding has both a tag and a BA/TW number, both are given.

Findings are grouped by their state:

- **[F] Fixable** – a bug or deviation that can be fixed in the current architecture (often after one more measurement in the reference application).
- **[V] Deferred** – a larger feature or expansion that was postponed on purpose (batch extensions, PDF/PSD export details, HTML setup, CMD-1, AI mask preview, ...).
- **[N] Not feasible / informational** – limits of the platform or of the project rules (for example `unsafe_code = "forbid"`), performance notes for machines without a GPU, and items that only need a regenerated report.

## How to add a finding

1. Open a GitHub issue with the **"Parity gap"** template (Firepixel behaves differently from the reference application) or the **"Bug report"** template (anything else that is wrong). Describe the steps, the expected result and the current result.
2. A maintainer reviews the issue, gives it a new ID (next free `BA-nnn` or `TW-nn`, or the tag of the work package that will handle it) and adds it to the matching section below with a link to the issue.
3. When the fix is merged and verified, the entry is removed from this file.

---

## [F] Fixable now (7)

Recounted after comparison run 15 (17/18 Oct 2026). The earlier remnants (sprite padding default,
Shift-constrain for L-shape/arrow line, import rubber band, New Style defaults, text runs, SVG layers, symbol
origin, simple brush runner, TW-101 import edge, TW-102/103 export format choice, TW-104 symbol properties
header, TW-100 library drop, TW-105/106 runner effects, TW-107 start colours, TW-109 Alt rectangle) are closed;
their history is in the commit log and the archive (see [ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)). Run 15 closed
TW-97 (Alt-drag duplicates on both sides, 8 vs. 8 objects), TW-110 (symbol properties table look), TW-112 (colour
pop-up commits on release), TW-114 (animated GIF byte-identical) and TW-116 (runner reaches the size and spiral
`center` handles). TW-113 stays open with new evidence from the original.

| ID | Area | What is open (expected vs. current) |
|---|---|---|
| TW-113 (Q-52) | Swatches panel, `.act` import and sort | Run 15 reverses the run-14 assumption. After "replace swatches" with the 16-colour `.act` file (16 colours, 240 padding entries, count trailer) and "sort by colour", the original shows the 16 colours followed by a block of about 240 black swatches (black sorted **last**; order yellow, beige, cyan, green, orange, #4080c0, …). Firepixel (since F29) keeps only the 16 counted colours and sorts them by hue (#ff8000 at index 5 instead of #4080c0). "Add" leaves the visible part of the original's set unchanged, so it most likely appends. Needs a behaviour description of the original's swatch file loader and sort key before the next fix. `panel-swatches-mixer/06–08`. |
| TW-118 (Q-53) | Pointer tool, Shift-drag | New in run 15 (the runner now presses on R1's body, doc 70,100): a Shift-drag of the selected R1 by +60/+10 moves it to 110,50–210,120 in Firepixel (constrained horizontally) but leaves it at 50,50–150,120 in the original. Unclear whether the original treats Shift+press on a selected object as "toggle selection" and then ignores the drag (Q-53). `xs-shortcuts/13_modifiers_drag`. |
| TW-120 | Inspector, several objects | With two objects selected the original shows the fill and stroke of the **last** selected object, Firepixel those of the first: R + TX → original fill black (text), Firepixel blue (rectangle); E1 + R1 → original stroke red 1 px "1-Pixel Hard", Firepixel stroke none. First measured in run 15 because the inspector shots now show Properties. `inspector-group-symbol/11, 12`. |
| TW-121 | Inspector, button instance | Properties of a button instance differ: the original shows the symbol name in the name field, a disabled text field, the export setting "PNG32" and an "Add filter…" button; Firepixel shows the instance name "BTN", an enabled text field, a disabled "GIF Web 216" with "Edit…" and no filter button. Needs a behaviour description of the button-instance inspector. `inspector-group-symbol/05_button`. |
| TW-115 (R-11) | Comparison runner, swatches | The reset step 00a (R-10) takes "Colour cubes" from the panel menu on both sides, but the sets still differ: the original shows a 21-column hue × lightness set (index 0 #ffbfbf, index 5 #efffbf), Firepixel its colour cube (#ffffff, #ffff00). Either the reference click did not take or the original's colour-cube set differs from Firepixel's; the runner must read back a swatch colour after the reset, and the set needs one manual check (VERIFICATION A2). |
| TW-119 (R-11) | Comparison runner, `xs-canvas` | (a) `12_grid_snap`: the reference drag is rounded to whole screen pixels at zoom 0.42833 and moves 11.67 document px, exactly at the snap tolerance (5 / zoom), so the original snaps to 0,20 while Firepixel, dragged by exactly 13 px, ends at 12,20; drag distances must stay clear of the tolerance or be checked in document units. (b) `02_zoom_center`: the Ctrl+wheel point (document 600,400) lies outside Firepixel's visible canvas at 100 %, so Firepixel does not zoom while the original does; the runner must check wheel points against the view as it does for drags. Fit zoom itself is equal (15 px margin on both sides; 0.43875 vs. 0.44875 only because the views differ in height). |
| U-1 rest | File ▸ Import, PDF | SVG files now import with their layers; **PDF files still arrive as one layer** because the pure-Rust PDF reader exposes no optional content groups (layers). Library limit; needs either an upstream feature or an own OCG reader. |

## [V] Deferred / expansion items (20)

| ID | Area | What is deferred |
|---|---|---|
| TW-111 (Q-51) | Inspector, Symbol Properties tab | After the Symbol Properties panel was opened, Firepixel keeps it in front of Properties for the following steps; the original shows Properties. Unclear whether the original switches back by itself when the selection is no rich symbol (Q-51). The runner workaround (step 04b, R-10) works in run 15: steps 05–14 now measure the Properties panel on both sides. Firepixel unchanged until Q-51 is answered. |
| BA-037 | AI object selection | Optional mask preview while hovering with the object-selection tool: when the model cache is warm, compute `select.aiObject` on a copy in the background and show the outline as an overlay. Currently there is no preview. |
| BA-039 | Bitmap brush | The live brush preview is drawn above all objects: hidden parts appear on top, and the bitmap object's opacity, blend mode and effects are missing in the preview. Needs a rebuild of the live preview in stacking order. Pressure is not available from a mouse (tablet only). |
| BA-100 | PDF export | "Enable text selection" is stored but has no effect: the PDF writer only stores the flat page image. Expected: invisible text (render mode 3) for text objects. |
| BA-101 | PDF export | "JPEG 2000" writes plain JPEG (DCT) because there is no JPEG 2000 encoder. |
| BA-102 | PSD export | The options Effects, Vector and Text are stored but do not change the file: the PSD writer always stores rendered pixel layers without layer effects or text layers. "Convert to PSD layers" creates one PSD layer per layer, not per object. |
| BA-103 | HTML Setup | No "Set defaults", external CSS file or navigation-bar HTML, because neither export nor New Document uses them. HTML style only offers HTML/XHTML (the original's template styles carry brand names and only change a comment in the output). |
| BA-104 (CMD-1) | Commands menu | Screen capture, "Create symbol script" and Web ▸ Shared Palette only show a message (`fp-script\src\builtin_commands.rs`). Wanted: real screen capture, symbol scripts for graphic symbols, a shared color table across all slices; texts via `fp.t`. |
| BA-086 | Test infrastructure | German QA area IDs and folders in `crates\fp-ui\tests\i18n_qa_*` (areas, base folder, cache file `dialoge.json`, finding kinds, image prefixes) should become English; existing baseline results must stay readable after the rename. Waiting for a decision. |
| BA-105 (BATCH-1/4) | Batch Studio | The size token `{w}x{h}` in file names uses the source size; after `resize`/`crop` the output size differs. Needs recalculation after the run (name conflicts may change). |
| BA-106 (BATCH-1) | Batch CLI | No Ctrl+C cancel in the CLI: the process ends immediately and temp files `.<name>.fptmp` are left behind. |
| BA-105 (BATCH-7/9) | Batch Studio | Several outputs per item (SVG **and** FW-PNG in one run, atlas `.json` for sprite sheets) need an executor hook for extra files; today this takes two runs. |
| BA-105 (BATCH-7) | Batch Studio, video | Pass `video.import` parameters to `item::load` (avoid reading twice); streamed `Item::Animation` (blocks of 32 frames) for very long videos. |
| BA-106 (BATCH-2) | Batch Studio | Ask "Batch is still running – cancel and close?" when the **window** is closed (hook in `actions.rs`/Quit); today `Drop` silently cancels the run. |
| BA-107 (BATCH-2/7/8/9) | Batch Studio UI | Missing UI: filters by size/pixels/transparency and glob (pro mode), sort by drag, Space/Enter in the file list, "Save as template"/"Help" in the card menu, per-file progress (`ItemProgress`), hint "JPEG fills transparency with white", fields shown per `mode`/`engine`, GIF fields of `output.gif` (`preset`, `quality`, `maxKB`) with the reached size or a "target size not reachable" warning, an ffmpeg row in Preferences and in the video row, and inserting the `video.import` card automatically for videos. |
| BA-083 (BATCH-4) | Batch tests | Golden hashes of the resampler depend on `f64::sin` (Lanczos) and may need regenerating on other operating systems. Open until CI runs on another OS. |
| BA-108 (BATCH-5) | Batch export | WebP `alphaQuality`/`method` and AVIF `alphaQuality` are not passed through to `fp-export::write_webp/write_avif`. |
| BA-108 (BATCH-5) | Batch metadata | Reading/writing metadata of AVIF/JXL/HEIF (box level) is missing; TIFF sources: metadata not read, TIFF output without EXIF IFD; `metadata.set` cannot write GPS/IPTC; `metadata.export` and `format.colorProfile` (to sRGB) are not implemented. |
| BA-109 (BATCH-6) | Batch steps | Steps from the batch spec are missing: `background.subjectCenter`, `background.alphaCleanup`, `overlay.roundCorners` (already possible with `overlay.border` width 0 + radius), `overlay.vignette`, `overlay.contactSheet`. Minor: `fp_raster::Mask` has no `is_empty()`. |
| BA-110 (BATCH-8) | Batch GIF optimizer | The spec target of −30 % size at equal SSIM is not reached (ideas: optimal instead of greedy LZW parsing, perceptual palette, a corpus of real clips); no per-frame parallelism; no cancel inside one encode (encoder hook without `CancelToken`). |

## [N] Not feasible / informational (17)

| ID | Area | Note |
|---|---|---|
| TW-117 | Comparison runner, colour pop-up | Unchanged in run 15. Canvas shots differ by capture method: Firepixel's shot contains the open colour pop-up and the selection handles, the original's does not (6.3 % of pixels above 32 levels); the fill under the hovered swatch is equal. Information only. |
| BA-097 | Performance (no GPU) | The first drag of large objects takes up to ~1–2 s with software rendering: the drag preview texture (`canvas\ghost.rs::drag_sprite`) holds the whole selection (up to ~6000 × 1500 px) and uploads at ~30 MB/s; later steps take ~100–200 ms. Idea: limit the preview to the visible area plus a margin. |
| BA-098 | Performance (no GPU) | The first frame after zoom/scroll with a large canvas texture takes up to 1.4 s (`canvas\mod.rs` renders the visible area plus half a view of margin on each side). Edits only upload the changed rectangle; zoom/scroll still upload everything. |
| BA-021 | English UI texts | The English labels of D39 ("Sample radius:") and the English history names of the Select commands were not measured in the original (only the German UI of the original is installed). |
| BA-099 | Performance | With 3200 selected objects, Inspector and Layers panel together still cost ≈ 45 ms per frame (dev profile); the Layers panel builds all rows, including invisible ones. |
| BA-114 (FARB-1b A1/A2) | Color picker | Deliberate deviations: own color dialog instead of the system color picker (`ChooseColor` only via `unsafe`); the eyedropper outside the app window shows the working color. All other points of this entry are done. |
| BA-099 | Performance | Command enabling (`command_state`) costs up to 5 ms per command with 3200 selected objects, mainly `modify.alterPath.*Fill`, `modify.transform.removeTransformations`, `text.orientation.*`. Only noticeable with an open menu. |
| BA-099 | Performance | After a move, `edit_at` drops the render cache of every element; the next render re-rasterizes all objects (costs background work and the first drag step). |
| BA-116 | Text rendering | A CFF test font reaches only ≈ 52–72 % exactly matching ink pixels (different font build on Windows, different CFF hinter than the original). Rotated/skewed text and faux italic stay unhinted; anti-aliasing "Strong" is deliberately unhinted. |
| BA-092 | Test images | Image series and paired images that contain text may change slightly because of hinting; they were not regenerated. Regenerate in the next image run. |
| BA-112 | Native file dialogs | Native file dialogs appear at the top-left corner of the main window (Windows places the Common Item Dialog there even with an owner); the original centres them. Moving them needs Win32 `SetWindowPos`, i.e. `unsafe` or a new dependency – both rejected. |
| BA-113 | Native Open/Import dialog | `rfd` cannot preselect a filter, so "All readable files" comes first instead of 13th. The in-app dialog and `ui_file_dialog` use the original order via `default_filter`. |
| BA-111 | Graphics backend | If DX12 initializes but the window surface fails, there is no fallback to Vulkan/GL in the same start (workaround: `FP_WGPU_ALL_BACKENDS=1` or `WGPU_BACKEND`). Watch on machines with a GPU. |
| BA-087 | Generated reports | Automatically generated test reports and the release package notes still show old German folder and `.ftl` names; the release folders next to the exe are `Languages\` and `Models\`. Regenerated by the next full test run. |
| BA-114 (F5) | Eyedropper | The eyedropper outside the window still returns the working color instead of the screen color. Reading the screen outside the window needs `unsafe` or an extra dependency (`unsafe_code = "forbid"`). |
| TW-58 | PNG file size | PNG8 is ≈ 5 % and PNG32 ≈ 0.6–1.3 % larger than the original's output. Row filters are identical; the difference comes from the deflate implementation (miniz instead of zlib). Decision: no zlib C backend; kept as information. |
| T-3 / T-5 | English UI texts | English step names and messages (`msg-step-add-states`, `msg-step-export-options` "Export Options", `ui-script-*` and other new EN texts) were not measured against the original, because only its German UI is installed (same as BA-021). |
