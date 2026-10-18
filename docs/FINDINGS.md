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
- **[V] Deferred** – a larger feature or expansion that was postponed on purpose (batch extensions, PDF/PSD export details, ...).
- **[N] Not feasible / informational** – limits of the platform or of the project rules (for example `unsafe_code = "forbid"`), performance notes for machines without a GPU, and items that only need a regenerated report.

## How to add a finding

1. Open a GitHub issue with the **"Parity gap"** template (Firepixel behaves differently from the reference application) or the **"Bug report"** template (anything else that is wrong). Describe the steps, the expected result and the current result.
2. A maintainer reviews the issue, gives it a new ID (next free `BA-nnn` or `TW-nn`, or the tag of the work package that will handle it) and adds it to the matching section below with a link to the issue.
3. When the fix is merged and verified, the entry is removed from this file.

---

## [F] Fixable now (7)

Recounted after comparison run 18 (18 Oct 2026, 10 units; the other 38 units from full run 17). The earlier remnants (sprite
padding default, Shift-constrain for L-shape/arrow line, import rubber band, New Style defaults, text runs, SVG
layers, symbol origin, simple brush runner, TW-101 import edge, TW-102/103 export format choice, TW-104 symbol
properties header, TW-100 library drop, TW-105/106 runner effects, TW-107 start colours, TW-109 Alt rectangle) are
closed; their history is in the commit log and the archive (see [ARCHIVE_NOTE.md](ARCHIVE_NOTE.md)). Run 15 closed
TW-97, TW-110, TW-112, TW-114 and TW-116; run 16 (preview `2026-10-18_02-04`, package F31, runner R-11) closed
TW-113/Q-52, TW-118/Q-53, TW-119, TW-120 and TW-121.

The wave after run 16 closed the rest:

- **TW-122 (Q-54):** in the reference application's own shortcut set Ctrl+Shift+B is Arrange ▸ Send Backward;
  Firepixel's bindings were right, but the runner profile used the "Web Standard" set (Ctrl+Shift+B = Break Apart).
  The parity runner now writes the Firepixel shortcut set into its fresh profile. Arrange commands are silent no-ops
  (no history step, document stays clean) when nothing can move, and faux bold/italic/underline on pointer-selected
  texts toggle across the whole selection.
- **TW-123:** the Optimize colour-table context menu is sized to its content (about 270 px); the Frames panel menu
  and the swatch drop-down were fixed the same way. Menus app-wide gained a 33 px arrow column; to be checked in the
  next full run.
- **TW-115 and TW-124 (runner R-12):** 9 px swatch pitch, objects serialised over the bridge, panel pop-up menus
  read instead of the panel, plain `for` loops in read-back scripts, drag end points allowed outside the view,
  remeasure or `unsupported` for the guide double-click and the export-area preview, inverted shot regions normalised.
- **Questions answered:** Q-49 (the colour pop-up HEX field now parses every keystroke like the reference),
  Q-51 (the reference does not switch the inspector tab back by itself, so Firepixel is right; runner step 04b
  stays and TW-111 closes as "no deviation"), Q-55 (the text-tool click behaviour was already right; the runner
  clicked a different point, step `xs-shortcuts/15` fixed), Q-56 (Modify-menu enable rules implemented and pinned by
  tests; Arrange is grey in bitmap edit mode).

Full run 17 (18 Oct 2026, preview `2026-10-18_04-57` from `7e3d2d12`, runner R-12) confirmed **TW-115** (swatch
probe and read-back #efffbf on both sides), **TW-123** (Optimize colour-table menu 282 vs. 270 px; panel menus within
5 px of the reference, none wider because of the 33 px arrow column), **TW-124** (object read-back, `for`-loop
read-backs, guide drag onto the ruler, `unsupported` instead of errors), **TW-111 / Q-51** and the byte-identical
animated GIF. It did not confirm **TW-122**: under the shortcut set the runner now writes, the text style shortcuts
and Ctrl+Shift+D behave differently from the reference machine (continued as TW-126). **Q-55** is continued as
TW-130 / Q-57. **Q-49** cannot be proven by the runner (the reference's pop-up closes without applying the text the
runner sets); it moves to the human click-test. Tags in the table: *RE spec* = write a behaviour description from
the reference first, *direct fix* = fix from the measured values, *runner* = parity runner change (R-13).

Follow-up run 18 (18 Oct 2026, preview `2026-10-18_10-59` from `4e677046`, packages F33/F34, runner R-13/R-13b with
the "Web Standard" set in Firepixel's profile, the reference machine's set) closed **TW-125** (Common Library menu
183 vs. 179 px), **TW-126** (Ctrl+Shift+D clones on both sides: `menu-edit/02`, `dialogs-d31-d40/06` and
`menu-modify-a/11` equal), **TW-127** (option start states set and read back; `xs-preferences/15` equal), **TW-128**
(both slices report JPEG and GIF) and **TW-129** (Feather / Save Bitmap Selection dialogs, the "add swatches" file
dialog, the library drop and Snap to Pixel with a selection all measured on the reference side). What is still
different is listed below as TW-131 to TW-135; the reference-side remainder of TW-129 (plain arrow keys) moved to
TW-132. Tags in the table now refer to runner R-14.

| ID | Area | What is open (expected vs. current) |
|---|---|---|
| TW-130 (Q-57) | Text tool, click on a text (*runner*, then *RE spec*) | Since F34 Firepixel places the caret at the click point: `xs-shortcuts/15_in_text_tx2` (text-tool click at TX2's centre, "V" and "R" typed) gives "Zeile eins / ZeileVr zwei / Zeile drei". The reference's read-back of TX2 shows only "Zeile eins" before and after typing, so where its typed text goes is not measured; the TX1 step is `unsupported` (invalid TX1 in the reference file). The stray "c" of run 17 appears only on the reference side (`tool-text/02, 03`: "Abc 123c"). Needs: a read-back of all lines and the typed text in the reference (R-14), then compare. |
| TW-131 | URL panel, panel menu (*direct fix*) | `panel-library-styles/16`: the reference's URL panel menu has nine entries (add used URLs to the library, delete unused URLs, add URL, edit URL, delete URL, new URL library, import URLs, export URLs, Help; edit and delete grey without a selected URL); Firepixel's has only "Help" (runs 17 and 18). The panel itself has its own buttons. Expected: the same entries, wired to the existing URL library functions, with the same enable rules. |
| TW-132 | Parity runner, reference side (*runner*) | Reference-side effects in run 18: the history read-back (`getStepsAsJavaScript`) fails with script error -2147216344 in `menu-edit/01, 22`; Delete (`menu-edit/09`) and Ctrl+D (`menu-edit/20`) leave the reference's document unchanged; the plain arrow keys do not move R1 (`xs-shortcuts/10`, Shift+arrows do); the text tool keeps state from earlier units (large italic underlined text, stray "c", text left in edit mode; `tool-text/02, 03, 07, 14`); Firepixel's menu read-back reports no enabled state for Snap to Pixel (`xs-canvas/15_snap_pixels_disabled`). Expected: read-back without the script error, keys that reach the canvas, the text tool reset before the unit, the enabled state read on both sides. |
| TW-133 | Shortcut set "Web Standard", text (*RE spec*) | `xs-shortcuts/06` with TX1 selected and both programs on "Web Standard": Ctrl+Shift+B does nothing in the reference, Firepixel opens a warning; Ctrl+Shift+I keeps the text selected in the reference, Firepixel drops the selection; Ctrl+Alt+Shift+S changes nothing in the reference, Firepixel sets "stretch" alignment; Ctrl+Shift+Period grows 24 to 25 pt in the reference (Ctrl+Shift+Comma back to 24), Firepixel jumps to 36 pt. Ctrl+Shift+U and Ctrl+Shift+E are equal. Needs: the reference's "Web Standard" bindings for these keys and its size step, then Firepixel's set corrected. Continues TW-122 / TW-126. |
| TW-134 | Preferences, "delete paths when cropping" (*RE spec*) | `xs-preferences/06`: the option is read back as on in both programs (reference: off at the start, set with one click; Firepixel's profile already on); after cropping 40,40–320,260 the reference keeps all seven paths, Firepixel removes the three outside (L1, ST, PG; 44 % canvas pair). Needs: when the reference applies the option (paths fully outside, bitmap mode, after a restart?), then the same rule in Firepixel. |
| TW-135 | Text editing after a pointer double-click (*RE spec*) | `tool-text/06`: pointer tool, double-click at 70,160 inside "Zeile eins" of TX2, type "Q". The reference inserts the "Q" at the click point ("ZeilQe eins"); Firepixel shows the caret after "Zeile " and no "Q" in the shot (runs 17 and 18; the canvas pair stays below 1 %, so the score did not catch it). Needs: the reference's caret rule for the double-click, then a fix so the typed character lands at that point. |
| U-1 rest | File ▸ Import, PDF | SVG files now import with their layers; **PDF files still arrive as one layer** because the pure-Rust PDF reader exposes no optional content groups (layers). Library limit; needs either an upstream feature or an own OCG reader. |

## [V] Deferred / expansion items (5)

Wave 3 (18 Oct 2026) closed BA-086, BA-100 (PDF selectable text), BA-102 (PSD layers per object with text,
vector and effects), BA-105 and BA-106; BA-108, BA-109 and BA-110 were shortened to what is still open.
Wave 4 (18 Oct 2026) closed the rest of the expansion work:

- **BA-103 HTML Setup:** "Set Defaults" stores the dialog's values as document-independent defaults (used by New
  Document and by the HTML export for pages without settings of their own); a new option writes the page styles
  to an external `styles.css`; a new neutral "Generic HTML5" style; button symbols keep their navigation-bar
  markup and script in every style (tests export a nav-bar document).
- **BA-104 (CMD-1) Commands menu:** `commands.screenCapture` (via the operating system's own screenshot tool),
  `commands.symbolScript` (generates a JSF script for the selected graphic symbol) and `commands.sharedPalette`
  (one adaptive palette over all open documents, filled into the Swatches panel; `.act` file via the `path`
  parameter for CLI/MCP) now work. Limits are listed under [N].
- **BA-107 complete:** the "ffmpeg for videos" row in Preferences (same setting as the Batch Studio).
- **BA-037 AI hover preview:** optional, preference "Show selection preview while hovering" (off by default);
  only with a warm model cache; a click on the same pixel reuses the result.
- **BA-039 brush preview in stacking order:** brush, pencil, eraser and rubber stamp preview at the bitmap's
  layer position with its opacity, blend mode, effects and masks. Limits are listed under [N].

| ID | Area | What is deferred |
|---|---|---|
| BA-101 | PDF/PSD export | No JPEG 2000 encoder: the PDF compression "JPEG 2000 (JPEG)" writes DCT/JPEG on purpose and says so loudly (option label, the PDF's producer entry, both manuals). PSD export: the model has no stroke effect, so nothing maps to a PSD stroke. |
| BA-083 (BATCH-4) | Batch tests | Golden hashes of the resampler depend on `f64::sin` (Lanczos) and may need regenerating on other operating systems. Open until CI runs on another OS. |
| BA-108 (BATCH-5) | Batch metadata | Writing metadata boxes for AVIF/HEIF/JXL is missing (reading works since wave 3; TIFF reads and writes EXIF/GPS/IPTC). |
| BA-109 (BATCH-6) | Batch steps | All steps of the batch spec exist except `ai.upscale` and `ai.faceBlur` (need models). `format.multiPage` only combines; splitting multi-page sources stays with `document.pages`/`anim.toFrames`. Runs started in the Batch Studio UI are not checked by the MCP caller gate (the UI keeps the opt-ins of `script.external`/`script.code`). Minor: `fp_raster::Mask` has no `is_empty()`. |
| BA-110 (BATCH-8) | Batch GIF optimizer | Measured with the new opt-in modes `lzw: flexible` and `perceptualPalette` (off by default; figures in [BATCH.md](BATCH.md) §5.6): screen −54.3 %, cartoon −37.9 % (both modes −38.8 %) vs. the export at SSIM ≥ 0.99; the camera pan reaches SSIM 0.99 in no mode (best 0.986). Not done: lossy LZW with an SSIM bound, cancel inside one encode. |

## [N] Not feasible / informational (20)

| ID | Area | Note |
|---|---|---|
| BA-037 | AI hover preview | Tested with a stub decoder only; not yet tried with real model weights. Works only with a warm cache (model loaded, embedding cached); it never loads a model itself. |
| BA-039 | Brush live preview | Bitmaps with live effects are slower (whole scratch raster, 48 px effect margin); strokes that create a new bitmap or paint into a bitmap inside a symbol use the old overlay above all objects; below 100 % zoom the composite tiles are sampled NEAREST. Frame time on 2000 × 1500 rose from 2.4 to 4.3 ms on average. |
| BA-104 (CMD-1) | Commands menu | Screen capture uses the system's own capture tool, so there is no interactive drag rectangle (the area is entered as numbers before the countdown). The symbol script skips bitmaps, nested instances and compound shapes and writes gradients as solid fills. |
| TW-117 | Comparison runner, colour pop-up | Unchanged in runs 15 and 17. Canvas shots differ by capture method: Firepixel's shot contains the open colour pop-up and the selection handles, the original's does not (6.3 % of pixels above 32 levels); the fill under the hovered swatch is equal. Information only. |
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
