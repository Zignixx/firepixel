# Batch Studio (modern batch mode)

The reference application's batch processing was a good basic idea; Firepixel rebuilds it as a
**modern, clean** workspace. Add many files (videos too), by drag and drop, click a chain of steps
together, see live what comes out, and go. The look follows the start page
(`crates\fp-ui\src\welcome.rs`, [UI_DESIGN.md](UI_DESIGN.md) §12.1): clean, cards, banner, calm
surfaces.

This document is the specification of the Batch Studio. All UI text lives in `.ftl` files; all
technical names (step ids, JSON keys, CLI flags, file names) are English.

Terms used in this document:
- **Studio**: the batch user interface.
- **Recipe** (`recipe`): an ordered list of **steps** (`steps`) plus output settings.
- **Job** (`job`): one run of a recipe over a list of files.
- **Item** (`item`): one input file, video or animation in the list.

---

## 1. Goals and scope

### 1.1 Goals

1. A **dedicated working mode** in the main window (not a small fixed dialog), reachable through
   *File ▸ Batch Processing...*, a card on the start page and `firepixel.exe --batch`.
2. **Files in → recipe → preview → output**: everything on one screen, no wizard pages.
3. **Simple mode** (templates and building blocks, no jargon) and **Pro mode** (regex, all
   parameters, expressions, registry commands, JSF).
4. **Never lose data**: by default the output goes to a new folder and the originals stay untouched;
   overwriting only by explicit choice, with a backup.
5. **One pipeline** for the UI, the CLI (`firepixel-cli batch --recipe`), MCP and JSF.
6. Fast: parallel on all cores, with a memory limit, cancellable at any time.

### 1.2 What the Studio covers from the reference application

| Original feature | In the Studio | Step / section |
|---|---|---|
| File selection with list, Add, Add All, Remove | File list on the left, drag and drop, folders (recursive), filter | §2.3 |
| "Include currently open files" | Button "Open Documents"; unsaved documents are processed as an in-memory snapshot, **without** a "save?" prompt (deliberate deviation, safer) | §2.3 |
| **Export** task with preset, "use settings from each file", "Edit" → Image Preview, optimize to size | Step `format.convert` with the presets of the export list, option "settings of the file", button "Fine-tune..." opens the existing Image Preview; `optimize.targetSize` | §3.2, §3.11 |
| **Scale** task with 4 modes and "only shrink" | `resize` with modes `none`, `exact`, `fit`, `fill`, `percent`, `width`, `height`, `longEdge`, `shortEdge`, checkbox "Only shrink" | §3.3 |
| **Find and replace** task (text, font, color, URL, non-Web216) in object files | `document.findReplace` (FW-PNG/PSD/SVG layers only; hint worded neutrally as in the original) | §3.10 |
| **Rename** task: replace (first occurrence, literal), replace spaces, prefix, suffix | Rename builder (Simple) and regex (Pro), with many more building blocks | §4 |
| **Command** task (`.jsf` from the commands folder, excluding `//--not for batch--` and saved batch scripts) | `script.command` (saved command / `.jsf`), `script.registry` (any registry command), `script.code` (JSF code) | §3.12 |
| Order via ▲/▼ | Drag cards, Alt+↑/↓ | §2.4 |
| Output to same location / custom folder | Output bar: new folder (default), same location, custom folder, keep subfolder structure | §2.6 |
| Backups "Original Files", overwrite / incremental | Conflict rule "replace original" only with a backup in `Original Files`; overwrite the backup or incremental `name-1.ext` ... | §2.7 |
| Save script / run script | Save/load recipes (JSON, `.fpbatch.json`); additionally "Export as JSF" (P3) creates a `.jsf` that calls `fp.batch(recipe)` | §2.5 |
| Progress "x of y files", Cancel, completion message, log | Progress in the output bar and per file in the list, immediate cancel (running files are discarded cleanly), error list, report (HTML/JSON), log `batch-log.txt` | §2.8 |
| "Rename only" without opening the file | Detection of "file steps only" → the file is copied/moved, not decoded (works for unreadable formats too) | §5.3 |
| Export/rename always last | The Studio uses the **visible order**; renaming and saving are fixed end cards ("File name", "Save as") that always stay at the bottom. This reproduces the original's behaviour without surprises | §2.4 |
| Path length > 256 characters | Checked in the dry run; on Windows written with `\\?\` paths, warning instead of abort | §2.7 |

**Deliberately not adopted** (quirks or bugs of the original): aborting the whole run on one broken
file (Studio: mark the file as failed and continue), character-class replacement, "rename without
export" producing two files at the original location, the dead crop branch, hard-coded English in
messages, Mac file types, silently modifying documents that are already open (the Studio always
works on a copy; the open document stays unchanged).

### 1.3 What is new (overview)

- Modern UI with thumbnails, drag and drop, recursive folders, filters, live before/after preview
  with pixel size and **file size**, dry run.
- Recipes as cards, template gallery ("Shrink for the web", "Instagram square", "Rename photos by
  date", "Video → small GIF", "Remove background" ...), sharing as a file.
- Renaming with a builder (numbers, date, EXIF date, image size ...) and regex with a plain-language
  explanation.
- Many formats: PNG, JPEG, GIF, WebP, AVIF, JPEG XL (P2), TIFF, BMP, TGA, QOI, ICO, WBMP, FW-PNG,
  PSD, SVG (via tracing), PDF (P2), animated GIF/APNG/WebP.
- **Video → animated image** (MP4/MOV/WebM/MKV → GIF/APNG/WebP) with time range, frame rate, size.
- **Very small GIFs at very good quality** (own optimizer, §3.9, §5.6).
- Trim/crop animations, change speed and looping, with a timeline preview.
- Keep/remove/set metadata (EXIF/XMP/IPTC/ICC, targeted GPS removal).
- AI: remove background (BiRefNet, available in `fp-ai`), transparency → color, center subject.
- Watermarks (text/image), borders, shadows, Firepixel live effects as steps.
- Optimize to a target size in KB, lossless PNG optimization.
- Trace → SVG in batch (`fp-trace`).
- Every registry command and JSF as a step.

Priorities used throughout: **P1** = first implementation scope, P2 = second round, P3 = later.

---

## 2. User interface

### 2.1 Entry points and mode

- **Location**: when opened, the Studio replaces the document area (like the start page;
  `welcome.rs` draws into a `rect`). Menu bar and toolbars stay visible; panels and the Properties
  inspector are hidden while the Studio is in front (the workspace is restored on close). The
  document tab strip shows a closable tab **"Batch"** with an icon. There is at most one Studio.
- **Invocation**: *File ▸ Batch Processing...* (`file.batchProcess` opens the Studio in the UI;
  called with parameters and without UI it keeps its previous behaviour); the start page card
  "Create New" gets a third entry "Batch Processing" (P2); dragging files onto the "Batch" tab;
  command line `firepixel.exe --batch [files...]`; JSF `fw.doBatchProcess(files)` opens the Studio
  with that list.
- **Minimum size**: 1024 × 640 points for three columns; below that two columns (preview as a tab
  next to the recipe); below 720 px one column with tabs "Files | Recipe | Preview" (§2.9).

### 2.2 Layout (sizes on the 8 px grid, `ui_kit::sp`)

```
┌────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Banner 72 px: [Logo 40] Batch Processing                  [Simple|Pro]  [Templates ▾] [⋯]      │
│               23 files · 412 MB · Recipe: "For the web" (unsaved •)                             │
├──────────────────────────┬───────────────────────────────────────┬─────────────────────────────┤
│ ① FILES  (card)          │ ② RECIPE  (card)                      │ ③ PREVIEW (card)            │
│ [+ Files] [+ Folder] ▾   │ ┌───────────────────────────────────┐ │ [Before | After | Compare]  │
│ [Filter: *.png, >1 MB ▾] │ │ ⠿ ▣ Resize                [●] ⋯ ▾│ │ ┌─────────────────────────┐ │
│ ┌──────────────────────┐ │ │   Long edge 1600 px,              │ │ │                         │ │
│ │▢ [36] holiday_01.jpg │ │ │   only shrink                     │ │ │   image with divider    │ │
│ │       4032×3024 3.1MB│ │ └───────────────────────────────────┘ │ │   ◀────│────▶          │ │
│ │▢ [36] holiday_02.jpg✓│ │ ┌───────────────────────────────────┐ │ │                         │ │
│ │▢ [36] clip.mp4  ▶0:12│ │ │ ⠿ ▣ Metadata              [●] ⋯ ▾│ │ └─────────────────────────┘ │
│ │ ...                  │ │ │   Remove GPS, keep the rest       │ │ Before 4032×3024 · 3.1 MB   │
│ └──────────────────────┘ │ └───────────────────────────────────┘ │ After 1600×1200 · 284 KB    │
│ 23 files, 2 videos       │       [ + Add step ]                  │ −91 %   ⏱ 0.4 s             │
│ [Remove] [Clear]         │ ┌ File name (fixed end card) ───────┐ │ [◀ File 3/23 ▶] [Zoom ▾]   │
│                          │ │  holiday_{n:3} → holiday_001.webp │ │ Timeline (for animations)   │
│                          │ ├ Save as (fixed end card) ─────────┤ │ ▕██████[====]█████▏ 0:02–0:07│
│                          │ │  WebP, quality 82                 │ │                             │
│                          │ └───────────────────────────────────┘ │                             │
├──────────────────────────┴───────────────────────────────────────┴─────────────────────────────┤
│ Output 64 px: Target [New folder "...\batch-output" ▾] [Folder...]  On conflict [Rename ▾]     │
│ [Dry run]   ▕██████████░░░░░░░░▏ 14/23 · 0:41 left         [Cancel]  [▶ Process 23 files]      │
└────────────────────────────────────────────────────────────────────────────────────────────────┘
```

| Area | Size | Rule |
|---|---|---|
| Outer margin | `sp::L` 16 | as on the start page |
| Banner | height 72 (narrow 56), radius 8 | reuse `welcome::paint_banner` (pixel field denser to the right), logo 40 px, title `TITLE` 15 bold, subtitle `CAPTION` muted |
| Column gap | `sp::L` 16 | |
| Card ① Files | width 300 (min 260, max 380), resizable | card header 34 px with icon chip, title, counter on the right |
| Card ② Recipe | remaining width, min 360 | step card width = column − 2·12 |
| Card ③ Preview | width 38 % (min 320, max 640), resizable | |
| Output bar | height 64 (two rows, 96, below 1200 px width) | surface `dialog_footer_bg`, top line as in UI_DESIGN §12.2 |
| File row | 44 px, thumbnail 36 px | like `recent_row` on the start page (checkerboard behind transparency) |
| Step card collapsed | 56 px (title + summary) | expanded: height follows content, label–value grid (UI_DESIGN §5) |
| Buttons | height 23 (main button "Process" 32, fill `focus`) | |

Colours and themes as in UI_DESIGN §12.1: Normal has light cards with a soft shadow, Dark has cards
lighter than the surface, OLED is black with a glow border per card in its group colour (Files
green, Recipe cyan, Preview orange). Step cards carry the colour of their **group** (§3) as a 3 px
accent stripe on the left and in the icon chip. Theme colours only (`theme.rs`, `ui_kit::tint`,
`card_bg`).

### 2.3 Card ① Files

- **Adding**: button "Files..." (multi-select, filter "All supported files", images, animations,
  videos), arrow ▾ with "Folder...", "Folder with Subfolders...", "Open Documents", "From
  Clipboard" (P2). **Drag and drop** from the file manager onto the whole card (or the whole
  Studio): files and folders; folders are read recursively (checkbox in settings ⋯ "Include
  subfolders", default on). While hovering: card tinted (`ui_kit::tint` focus), dashed border, text
  "Release to add".
- **Duplicates** (same canonical path) are not added again; a `notice` (Info) such as "3 files were
  already in the list" is shown for 4 s.
- **Unknown files** (no image/video extension, unreadable) are not added; counter "5 skipped" with
  a tooltip list. Exception: if the recipe contains **only file steps**, all files are allowed
  (renaming arbitrary files).
- **Row**: checkbox (for selection-based actions), 36 px thumbnail (read in the background, cached
  like `welcome\thumbs.rs`; video: first frame + ▶ badge + duration; animation: badge such as "GIF
  48 frames"), name (shortened in the middle), below it muted "4032×3024 · 3.1 MB · JPEG" or the
  folder relative to the root. Status icon on the right: waiting (empty), running (circle), done ✓
  green, warning ⚠ yellow, error ✕ red (tooltip with the message).
- **Filter** (field with ▾): search text in the name, type (image/animation/video/FW-PNG), size
  (> / <), pixels (> / <), with/without transparency. Pro: glob (`**/*.png`) and regex. Filtered-out
  files are **not** processed; counter "23 of 40 files".
- **Sorting** ▾: name (natural: `image2` before `image10`), date, size, pixels, added. The order
  determines numbering when renaming and for "image sequence → animation". Manual dragging (P2).
- **Empty state**: large `ui_kit::empty_state`: 48 px icon, "Drag images, animations or videos
  here", below it the buttons "Files..." / "Folder...", and the supported formats, muted.
- **Keys**: Del removes, Ctrl+A, Ctrl+O adds, Space shows the file in the preview, Enter opens the
  file as a document (P2).
- Large lists: virtual list (only visible rows are drawn), tested with 10,000 files.

### 2.4 Card ② Recipe

- **Step card** (collapsed 56 px): grip ⠿ (drag to reorder), icon chip in the group colour, title
  (`SECTION` 12 bold), one-line plain-language summary (muted, e.g. "Long edge 1600 px, only
  shrink"), on the right an on/off switch (●), menu ⋯ (Duplicate, Move Up, Move Down, Remove, Save
  as Template, Help), arrow ▾ to expand. **At most one** card is expanded at a time (accordion); the
  most recently added one is open.
- **Expanded**: parameters in the label–value grid (UI_DESIGN §5), sliders with number fields,
  units muted. Simple mode shows only the parameters marked **A** (§3); "More options" reveals the
  rest. Invalid values: field with a red edge + error `notice` inside the card; the card header
  shows ⚠, and "Process" is disabled with a tooltip.
- **"+ Add step"** opens the **step gallery** as an overlay over the Recipe card: search field at
  the top (focused), groups as tiles (like "Create New" on the start page, 54 px, icon, title,
  explanation), arrow keys + Enter. Simple mode lists only the common steps (marked **A** in §3),
  Pro mode lists all.
- **Fixed end cards** (cannot be removed, always at the bottom): **"File name"** (§4) and **"Save
  as"** (format and format options, §3.2). The user always sees what will be written at the end.
- **Order**: drag with an insertion line (2 px `focus`), Alt+↑/↓. Context hints: e.g. "Transparency
  → color" after "Save as JPEG" is unnecessary because JPEG has no transparency anyway → hint "JPEG
  fills transparency with white automatically; choose the colour here" (step `background.flatten`
  is suggested automatically with white for a JPEG target).
- **Undo/redo in the recipe**: Ctrl+Z/Ctrl+Y act on the recipe inside the Studio (own history, not
  the document history). One slider drag = **one** undo step.
- **Validation**: each step runs `validate()` against the item kinds (e.g. "time range" only for
  animation/video → greyed out for still images with the hint "only affects animations and
  videos").

### 2.5 Templates and recipes

- **"Templates ▾"** menu in the banner: gallery of built-in templates (cards with icon, title,
  sentence), below it "My Recipes", then "Open Recipe...", "Save Recipe", "Save Recipe As...",
  "Export as JSF" (P3), "Empty Recipe".
- Storage location: `%APPDATA%\Firepixel\BatchRecipes\*.fpbatch.json` (`~/.config/firepixel/
  BatchRecipes` on other systems; path via `fp_i18n::profile::roaming_dir()`).
- **Sharing**: pass the file on; recipes contain no absolute paths except, optionally, the output
  folder and image watermarks (on save: checkbox "Embed image files" → Base64 in the JSON).
- A recipe containing an unknown (newer) step still loads: card "Unknown step `xyz`" (disabled,
  parameters preserved).
- Built-in templates (P1 in bold): **Shrink for the web** (long edge 1920, WebP 82, metadata without
  GPS), **Save as JPEG** (transparency → white, quality 85), **Rename photos by date**
  (`{exif-date:YYYY-MM-DD}_{n:3}`), **Video → small GIF** (480 px, 12 fps, GIF optimizer
  "balanced"), **Remove background** (AI → PNG), **Transparency to colour**, **Remove metadata**,
  Square 1080 (Instagram), Thumbnails 256, Favicon set (P2), Sprite sheet (P2), Logos → SVG (P2).

**Recipe file (JSON, English keys):**

```json
{
  "format": "firepixel-batch-recipe",
  "version": 1,
  "name": "For the web",
  "description": "",
  "steps": [
    { "id": "resize", "enabled": true, "params": { "mode": "longEdge", "size": 1600, "onlyShrink": true, "filter": "lanczos3" } },
    { "id": "metadata.edit", "enabled": true, "params": { "keep": "all", "remove": ["gps"] } },
    { "id": "background.flatten", "enabled": false, "params": { "color": "#ffffff" } }
  ],
  "rename": { "mode": "builder", "parts": [ { "kind": "original" }, { "kind": "text", "text": "_web" } ], "case": "lower", "spaces": "_" },
  "output": {
    "format": "webp", "options": { "quality": 82, "lossless": false },
    "target": { "kind": "newFolder", "path": null, "folderName": "batch-output", "keepStructure": true },
    "conflict": "rename", "backup": { "enabled": false, "incremental": false },
    "keepTimestamps": false
  }
}
```

Unknown keys are ignored when reading and preserved when saving (`#[serde(flatten)] extra`).
`version` is increased on incompatible changes; older versions are migrated.

### 2.6 Output bar and target

- **Target** ▾: **New folder next to the sources** (default, name `batch-output`, or
  `batch-output-2` ... if it already exists; name editable in the field), **Custom folder...**,
  **Same location as original** (then the conflict rule of §2.7 is mandatory). Checkbox "Keep
  folder structure" (for recursively added folders, default on).
- **On conflict** ▾ (§2.7). **Keep timestamps** (⋯, P2): set the modification date of the source.
- **Dry run**: computes only names, targets, conflicts and estimated sizes (extrapolated from the
  preview file, marked "≈"); shows a table (source → target, size, note) in the Preview card under
  the tab "Plan". Nothing is written.
- **Start** "▶ Process 23 files" (number = filtered, enabled items). Disabled when: the list is
  empty, a step is invalid, an AI model is missing (hint with button "Manage AI Models...", download
  only after consent in the existing dialog), a video decoder is missing (§5.5).

### 2.7 Conflict rules

| Rule (`conflict`) | Effect | Default |
|---|---|---|
| `rename` "Choose a new name" | `name (2).ext`, `name (3).ext` ... | **yes** |
| `skip` "Skip" | the existing file stays; the item gets ⚠ "skipped" | |
| `overwrite` "Overwrite" | without prompt only if target ≠ source; target = source → backup required | |
| `ask` "Ask every time" | the run pauses; prompt with "Apply to all" | |

- **Replace original** (target = source) is only possible with a backup: subfolder `Original Files`
  (technical English name, as in the original's preference) in the source folder; "Keep backups" =
  incremental `name-1.ext`. Always written first to a temporary file in the target folder, then
  renamed atomically; only afterwards is the original moved. If anything fails, the original stays
  intact.
- **Conflicts within the run** (two sources produce the same target name, e.g. `a.png` and `a.jpg`
  → `a.webp`): shown as a warning in the dry run and before the start; resolved by the chosen rule
  in list order (deterministic even with parallel processing: targets are assigned **before** the
  start, §5.2).
- Invalid characters (`<>:"/\|?*`, control characters, trailing dot/space, reserved names `CON`,
  `NUL`, `COM1` ...) are replaced with `_`, with a hint in the name preview. Names longer than 255
  characters are shortened (extension kept); full paths > 260 use a `\\?\` path, with a warning in
  the dry run.

### 2.8 Progress, cancel, errors, report

- During the run: recipe and list are locked (muted, hint "Running ..."); the preview shows the
  last finished file (before/after). Bar: progress (`ui_kit::progress`), "14 of 23", remaining time
  (moving average), throughput, Cancel.
- **Cancel**: sets a `CancelToken`; running steps check it between tiles/images/frames (at least
  every 100 ms); temporary files are deleted; finished files remain. Message "Cancelled: 14 done, 9
  not processed".
- **End**: summary card over the preview: ✓ 21 done, ⚠ 1 skipped, ✕ 1 error; total before/after
  (e.g. 412 MB → 38 MB, −91 %), duration; buttons "Open Folder" (only without
  `FP_NO_SHELL_OPEN`), "Save Report...", "Retry Failed", "New Recipe". Items in the list keep their
  status; filter "Only files with errors".
- **Error list**: tab "Errors (1)" in the Preview card: file, step, message (translated), button
  "Copy".
- **Report**: `batch-report.json` (English keys: `recipe`, `started`, `finished`, `items[]` with
  `source`, `target`, `status`, `bytesIn`, `bytesOut`, `ms`, `warnings`, `error`) and optionally
  `batch-report.html` (tiles, error list, per file before/after thumbnails as embedded PNG,
  dimensions, sizes, notes; light/dark following the system; recipe key `output.reportHtml`,
  default on; button "View Report" in the end card) in the output folder; log `batch-log.txt`
  (English, technical) under `%LOCALAPPDATA%\Firepixel\Logs\`.
- Closing the window during a run: prompt "Batch is still running – cancel and close?".

### 2.9 Preview (card ③)

- Tabs: **Before | After | Compare** (draggable divider, default Compare), **Plan** (dry-run
  table), **Errors** (only when > 0).
- Shows the file selected in the list (otherwise the first). ◀ ▶ pages through files, Zoom ▾ (Fit,
  100 %, 200 %), mouse wheel zooms around the pointer, dragging pans; checkerboard behind
  transparency; background selectable (checkerboard, white, black).
- Below the image: "Before 4032×3024 · 3.1 MB" / "After 1600×1200 · 284 KB (−91 %)" / computation
  time. The "After" file size comes from actually encoding the preview file in the background (full
  size); until then "≈" with an estimate.
- **Animations/videos**: play ▶/⏸, timeline with a thumbnail strip, handles for start/end (step
  `anim.trim`), frame counter, frame rate; the After preview plays the output (GIF/APNG/WebP),
  including the actual palette and dithering.
- **Fast preview**: computed on a proxy (long edge ≤ 1024 px; for animations ≤ 480 px and at most
  150 frames), debounced 150 ms after the last change; every change cancels the running preview
  (generation counter). AI steps cache their mask per file. Steps with `preview: false` (§3) are
  skipped in the preview and listed as a hint.
- Narrow width: preview as a tab in the Recipe column; single column: tabs at the top.

### 2.10 Simple and Pro mode

| | Simple (default on first start) | Pro |
|---|---|---|
| Step gallery | ≈ 20 common steps, plain names | all steps, search by id, registry commands, JSF |
| Parameters | only **A** parameters, "More options" | all, numbers with units, expressions in fields (`{width}/2`, P3) |
| Renaming | builder (§4.1) | builder **and** regex (§4.2) |
| Filter | search, type | additionally glob, regex, size/pixels |
| Output | target, conflict | structure, timestamps, parallelism, memory limit, report |

Toggle in the banner (segmented button). The mode is remembered (`BatchMode` in the settings).
Switching never loses values: parameters hidden in Simple mode keep their values and are mentioned
in the summary ("+ 2 Pro settings").

### 2.11 Keyboard, accessibility, text

- Tab order: banner → Files → Recipe → Preview → Output. Focus ring in `focus` as in UI_DESIGN
  §12.1. Ctrl+Enter starts, Esc closes overlays (it does not cancel the run), F5 = dry run.
- Every control has an egui accessibility name (for `ui_find`).
- **All text** lives in `crates\fp-i18n\locales\{de,en}\batch*.ftl` (area `batch`, split into
  several files), key prefix `batch-`; step names `batch-step-<id>`, parameters
  `batch-param-<id>-<param>`, tooltips `.tooltip`. No hard-coded text in code. Numbers, sizes and
  dates are formatted per language (decimal comma in German). `qps` must not overflow anywhere
  (`clipping_audit` 0 findings, all 3 themes, 1024×768 and 1920×1080, 100/150 %).

---

## 3. Step catalogue

Columns: **ID** (technical, English, used in the recipe), **Parameters** (range, *default*; **A** =
visible in Simple mode), **V** = live preview yes/no, **Method / library**, **P** = priority. All
steps act on an `Item` (§5.1: still image, animation, video stream, document or "file only"). Group
colours: Files green, Format blue, Geometry cyan, Color violet, Background orange, Overlay yellow,
Metadata grey, Animation pink, Optimize green-teal, Vector/Script neutral.

### 3.1 Files and renaming (group "Files")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `rename` (end card) | File name | see §4 | name preview | own (`regex` 1.x) | **P1** |
| `file.copyOnly` | Copy/move only | **A** `action` copy/move *copy* | – | `std::fs`, no decoding | **P1** |
| `file.organize` | Sort into subfolders | **A** `by`: date (`YYYY/MM`), type, size class, orientation, EXIF camera; free `pattern` (`{exif-date:YYYY}/{ext}`) | Plan | own | P2 |
| `file.filterSkip` | Process only some | condition (width/height/size/transparency/animation/name regex) → skip the rest or only copy it | – | own | P2 |
| `file.dedupe` | Detect duplicate images | `method` exact (SHA-256) / similar (dHash, distance 0…16 *4*); action: skip/mark | – | `sha2` (present), own dHash | P3 |
| `file.timestamps` | Set date | modification date = source / EXIF capture / fixed | – | `filetime` 0.2 (MIT/Apache) | P2 |

### 3.2 Format and conversion (group "Format"; end card "Save as")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `output` (end card) | Save as | **A** `format`: same, png, jpeg, webp, avif, gif, apng, tiff, bmp, tga, qoi, ico, wbmp, fwpng, psd, jxl (P2), pdf (P2), svg (via `vector.trace`); format options below | yes (size) | `fp-export` (present) | **P1** |
| – PNG | | **A** `bitDepth` 8/24/32/auto *auto*; `palette` 2…256 *256* (8-bit only); `interlace` off; `compression` default/fast/max *default* | yes | `png`, `fp-export::png_out`; max → `optimize.png` | **P1** |
| – JPEG | | **A** `quality` 1…100 *85*; `progressive` on; `subsampling` 4:4:4/4:2:2/4:2:0 *auto (≥ 90 → 4:4:4)*; `background` for transparency *#ffffff* | yes | `fp-export::jpeg` (own) | **P1** |
| – WebP | | **A** `quality` 0…100 *82*; **A** `lossless` off; `alphaQuality` 0…100 *100*; `method` 0…6 *4* (still and animated WebP) | yes | `webp` 0.3.1 (static libwebp, BSD-3) | **P1** |
| – AVIF | | **A** `quality` 1…100 *60*; `speed` 1…10 *6*; `alphaQuality` | yes | `ravif` 0.13 (BSD-3) | **P1** |
| – GIF (still and animated) | | options of the GIF optimizer, table §3.9 (schema `output.gif`) | yes | own encoder §5.6 | **P1** |
| – TIFF/BMP/TGA/QOI/WBMP | | TIFF `compression` default/none/lzw/deflate/packbits *default = lzw*; TGA `compression` default/none *default = RLE*; BMP/WBMP `background` *#ffffff* | yes | `fp-export::formats` (present) | **P1** |
| – ICO | | `sizes` [16,24,32,48,64,128,256] *all ≤ image*; `pngFrom` 256 | yes | `fp-export::write_ico` | P2 |
| – FW-PNG | | – (document stays editable) | – | `fp-fwpng` | **P1** |
| – all except FW-PNG | | `alsoFwpng` off: additionally writes an editable FW-PNG with the same name next to each output (e.g. SVG **and** FW-PNG from `vector.trace` in one run); conflicts follow the recipe's rule, report field `extraTargets` | – | executor (extra-output hook, §5.2) | **P1** |
| – PSD | | `layers` on (from FW-PNG/PSD sources) | – | `fp-export::psd_out` | P2 |
| – JPEG XL | | `lossless` on / `distance` 0…15 *1* | yes | check licence/maturity: `zune-jpegxl` (lossless only, MIT/Apache/Zlib) or `jxl-encoder` if permissive; otherwise P3 | P2 |
| – PDF | | `pageSize` image/A4/Letter, `fit`, one PDF per image or **all in one** | – | own writer (`fp-export` has PDF building blocks for vector output) | P2 |
| `format.colorDepth` | Colour depth | 8/16 bit, greyscale, indexed | yes | own | P3 |
| `format.colorProfile` | Colour profile | `mode` keep / convertToSrgb / assignSrgb / strip *convertToSrgb* | yes | own ICC reader; conversion via `qcms` 0.3 (MPL-2.0, **only** if MPL is allowed, otherwise own matrix/TRC conversion for RGB matrix profiles) | P2 |
| `format.multiPage` | Split pages/layers | source with pages/layers/frames (FW-PNG, PSD, TIFF, ICO, GIF) → one file per page/layer/frame; `{page}`/`{layer}`/`{frame}` in the name | – | `fp-io`, `fp-model` | P2 |

**Fields per format:** the schema of the end card (`output`) attaches every format field to
`format` via `visibleWhen`; fields with the same name exist once per format (e.g. `quality` JPEG
*85*, WebP *82*, AVIF *60*; `compression` for PNG/TIFF/TGA each with its own values), and only one
is visible at a time. For GIF the UI appends the schema `output.gif` (§3.9) as the section "GIF
optimizer". When the format changes, the card drops values the new format does not accept (TIFF
`lzw` → PNG); valid values (quality) are kept. Hints in the card: "Trace" with a pixel format, JPEG
fills transparency with `background`.

**Target = "same as source"** keeps the format (for formats that cannot be written, e.g. HEIC/PSD →
PNG with a hint). The extension always follows the output format (`.jpg`, `.webp`, `.fw.png`);
extension spelling under ⋯ (lower/upper case, `jpg`/`jpeg`).

### 3.3 Size, crop, rotate, canvas (group "Geometry")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `resize` | Resize | **A** `mode`: percent, width, height, longEdge, shortEdge, fit (inside a box), fill (fill + crop), exact (distorting), none; **A** `width`/`height`/`size` 1…32768 px *1600*; `percent` 1…10000 *50*; **A** `onlyShrink` on; `filter` nearest/bilinear/bicubic/lanczos3/auto *auto* (pixel art → nearest); optional `dpi` | yes | `fp-raster` scaler (present; `modify.canvas.imageSize` for documents), own Lanczos, rayon | **P1** |
| `crop` | Crop | **A** `mode`: aspect (1:1, 4:3, 3:2, 16:9, 9:16, free), rect (px/%), edges (top/bottom/left/right px/%); **A** `anchor` 9 points *center*; `smart` on → crop by subject (BiRefNet mask or edge energy) | yes | own; smart: `fp-ai` (P2) | **P1** (smart P2) |
| `trim` | Trim edges | **A** `what` transparent / color (corner) / both; `tolerance` 0…255 *8*; `padding` 0…500 px *0* | yes | own | **P1** |
| `rotate` | Rotate/flip | **A** `angle` 90/180/270/free (−360…360); **A** `flip` none/h/v; `autoExif` on (rotate by EXIF orientation, then set the tag to 1); `background` for free angles *transparent* | yes | own (90° lossless by reordering; free: bilinear/bicubic) | **P1** |
| `canvas` | Canvas/border | **A** `mode`: addBorder (px per side), toSize (W×H), toAspect, square; **A** `color` *transparent*; `anchor` *center* | yes | own; documents: `modify.canvas.canvasSize` | **P1** |
| `canvas.pad` | Pad to multiple | **A** `multiple` 2…256 px *16* (video/tiles); **A** `color` *transparent*; image centred | yes | own | P3 |
| `perspective` | Straighten | angle automatic (Hough) or fixed | yes | own | P3 |
| `tile.split` | Split into tiles | grid N×M or tile size; name `{row}_{col}` | – | own | P2 |

### 3.4 Colour, correction, filters, effects (group "Color and Effects")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `adjust.auto` | Auto enhance | **A** `levels` on, `contrast` on, `whiteBalance` off | yes | `filter.adjustColor.autoLevels` + own white balance (grey world) | **P1** |
| `adjust.basic` | Brightness/contrast/saturation | **A** `brightness` −100…100 *0*; **A** `contrast` −100…100 *0*; **A** `saturation` −100…100 *0*; `hue` −180…180; `gamma` 0.1…5 *1*; `temperature` −100…100; `tint` | yes | registry `filter.adjustColor.brightnessContrast`, `…hueSaturation` + own | **P1** |
| `adjust.levels` / `adjust.curves` | Levels / Curves | as in the dialogs (points as JSON); "from preset" | yes | registry `filter.adjustColor.levels`/`curves` | P2 |
| `adjust.grayscale` | Black & white/sepia/duotone | **A** `style` gray/sepia/duotone; colours | yes | own | **P1** |
| `adjust.invert` | Invert | – | yes | registry `filter.adjustColor.invert` | **P1** |
| `adjust.posterize` / `adjust.threshold` | Posterize/threshold | **A** `levels` 2…64 *4*; **A** `threshold` 0…255 *128* (luminance Rec. 601, ≥ threshold → white; alpha unchanged) | yes | own | P2 |
| `filter.sharpen` | Sharpen | **A** `amount` 0…500 % *60*; `radius` 0.1…10 *1*; `threshold` 0…255 *0* | yes | registry `filter.sharpen.unsharpMask` | **P1** |
| `filter.blur` | Blur | **A** `kind` gaussian/motion/radial/zoom; `radius` 0…250 *2* | yes | registry `filter.blur.*` | **P1** |
| `filter.noise` | Add/remove noise | add: amount; remove: median 1…10 | yes | registry `filter.noise.addNoise`; own median | P2 |
| `filter.pixelate` | Pixelate (whole image) | **A** `size` 2…200 px *8* (alpha-weighted mean per block) | yes | own; area only: not yet | P2 |
| `filter.registry` | Any filter | choice of all `filter.*` registry commands with their schema (form built from `describe_command`) | yes | command registry | **P1** |
| `effect.live` | Firepixel live effect | **A** `effect` dropShadow/innerShadow/glow/innerGlow/bevel/emboss/... (list from `element.addEffect`); `params` per effect; applied to the bitmap as an element, then flattened (canvas grows by the effect margin, checkbox "Extend canvas" on) | yes | `element.addEffect` + `fp-effects`/`fp-render` | **P1** |
| `effect.style` | Apply style | style from the style library | yes | registry | P2 |
| `adjust.lut` | Colour LUT (.cube) | file, strength 0…100 | yes | own `.cube` reader, trilinear | P3 |
| `ai.upscale` | AI upscale ×2/×4 | model (Real-ESRGAN family, BSD-3, via `ai.manageModels`) | yes (region) | `fp-ai` (new network, own CPU inference) | P3 |
| `ai.faceBlur` | Blur faces | strength | yes | needs a model (check licence) | P3 |

### 3.5 Background and transparency (group "Background")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `background.remove` | Remove background (AI) | **A** `model` birefnetLite/birefnet *birefnetLite*; **A** `edge` hard/antiAlias/feather *antiAlias*; `tolerance` 0…100 *50*; `expand` −100…100; `smooth` 0…100; `refineEdge` (hair, `birefnetLiteMatting`) off; `output` transparent / mask only / keep + mask as alpha | yes (mask cached) | `fp-ai` BiRefNet (present, `select.aiSubject`, fine-tuning as in `docs\AGENTS.md`); model missing → hint + "Manage AI Models..." | **P1** |
| `background.removeColor` | Remove colour (no AI) | **A** `color` *top-left corner*/eyedropper; **A** `tolerance` 0…255 *24*; `contiguous` on (only from the edges); `feather` 0…10 | yes | own (flood fill like `bitmap.floodFill`) | **P1** |
| `background.flatten` | Transparency → colour | **A** `color` *#ffffff*; `kind` color/gradient/image/blur (blurred copy of the image) *color*; `onlyIfTransparent` on | yes | own (optional alpha blending in linear light) | **P1** |
| `background.replace` | Replace background | `remove` + `flatten` in one (colour/gradient/image/blur) | yes | combination | P2 |
| `background.subjectCenter` | Center subject | **A** `margin` 0…50 % of the subject's longer side *10*; **A** `square` off; `shadow` (soft shadow below the subject) off; `detect` auto/alpha/color *auto* (alpha if the item has transparency, else "differs from the top-left corner colour"); `tolerance` 0…255 *24*; `fill` auto/transparent/color *auto* (transparent, or the corner colour when detected by colour), `color` | yes | own: subject box (union over all frames of an animation) → crop → centred on the new canvas; no subject → unchanged with a warning. AI mask: run `background.remove` first | P2 |
| `background.alphaCleanup` | Clean up alpha | **A** `low` 0…254 *8* (alpha below → 0); **A** `high` 1…255 *247* (alpha above → 255; must be > `low`); `minIsland` 0…1,000,000 px *0* (remove 8-connected specks smaller than this); `defringe` 0…5 px *0* (semi-transparent edge pixels take the colour of opaque neighbours, one ring per px) | yes | own | P2 |
| `background.shadow` | Add shadow | like `effect.live` dropShadow, presets for product shots | yes | `fp-effects` | P2 |

### 3.6 Watermark, text, border (group "Overlay")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `overlay.text` | Text/watermark | **A** `text` with placeholders (`{name}`, `{date}`, `{exif-date}`, `{n}`, `©`); **A** `position` 9 points / tiled / free; **A** `size` % of image width 1…50 *4*; `font` (system fonts like the Text tool), `color`, `opacity` 0…100 *60*, `margin` %, `rotation`, `shadow`/`outline` | yes | `fp-text` + `fp-render` (present) | **P1** |
| `overlay.image` | Insert logo/image | **A** `image` (file, can be embedded in the recipe); **A** `position`; **A** `scale` % of image width *15*; `opacity`; `margin`; `blend` (registry blend modes) | yes | `fp-render` | **P1** |
| `overlay.border` | Border | **A** `width` px/%; **A** `color`; `radius` (rounded corners, transparent outside) 0…50 %; `inner`/`outer` | yes | own | **P1** |
| `overlay.roundCorners` | Round corners | **A** `radius` 0…10000 *16*; **A** `unit` px/percent (of the shorter side, max 50) *px* | yes | own (anti-aliased, transparent outside; same shape as `overlay.border` `radius`) | P2 |
| `overlay.vignette` | Vignette | **A** `strength` 0…100 % *50*; **A** `size` (untouched centre) 0…100 % *50*; `color` *#000000* | yes | own (smoothstep over the normalised distance to the centre, alpha unchanged) | P3 |
| `overlay.contactSheet` | Contact sheet | grid, spacing, captions; **several files → one image** | yes | own (collect step, §5.1) | P2 |

### 3.7 Metadata (group "Metadata")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `metadata.edit` | Metadata | **A** `keep`: all / none / chosen *all*; **A** `remove`: [gps, camera (make/model/serial number), software, thumbnails, xmp, iptc, comments, icc] *[gps]*; Pro: list of individual EXIF tags | info table before/after | reading: `kamadak-exif` 0.6 (BSD-2); writing/removing: container level with `img-parts` 0.3/0.4 (MIT/Apache, JPEG/PNG/WebP) and an own EXIF TIFF writer (tag list filtered, offsets recomputed); AVIF/JXL/TIFF: own box/IFD code | **P1** (keep/none/gps) · P2 (individual tags) |
| `metadata.set` | Set metadata | **A** `artist`, **A** `copyright`, `description`, `keywords` (XMP `dc:subject` + IPTC), `title`, `rating` 0…5, `dateTaken` (fixed or shifted ± time), set/remove `gps`; placeholders allowed | info | own EXIF; XMP packet with `xmp-writer` 0.3 (MIT/Apache); own simple IPTC-IIM | P2 |
| `metadata.orientation` | Apply orientation | apply EXIF orientation to the pixels, tag = 1 | yes | own (= `rotate.autoExif`) | **P1** |
| `metadata.export` | Metadata as file | JSON/CSV per file or collected | – | `kamadak-exif` | P3 |

Rule: when converting, metadata is **carried over** as far as the target format can hold it
(EXIF/XMP/ICC in JPEG, PNG `eXIf`/`iTXt`, WebP, AVIF, TIFF); GIF/BMP/TGA/QOI/ICO lose it, with the
hint "Format does not store metadata". Without a "Metadata" step, `keep: all` applies; the
orientation is applied and set to 1.

Alternative to the own EXIF writer: `little_exif` (MIT/Apache, reads and writes EXIF in JPEG, PNG,
WebP, JXL, HEIF, TIFF; still pre-1.0, known gaps with JPEG APP12/13). Use `little_exif` only if its
tests pass on our corpus, otherwise own code (≈ 400 lines).

### 3.8 Animation and video (group "Animation")

Animations (GIF, APNG, animated WebP, animated AVIF, JPEG XL animation, FW-PNG with states) and
videos are handled as a **frame sequence** (`Item::Animation`, §5.1). Videos are **streamed** (never
fully in memory).

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `video.import` (automatic for videos, card visible) | Read video | **A** `start`/`end` (time, millisecond-exact) *whole*; **A** `fps` 1…60 *12*; `maxFrames` 1…3000 *600*; `decoder` auto/builtin/ffmpeg | yes (timeline) | §5.5 | **P1** (via ffmpeg) · P2 (built-in H.264/AV1) |
| `anim.trim` | Trim | **A** `start`/`end` (time or frame number) with timeline and handles; `removeFrames` list/ranges (`1-3, 10, 20-`) | yes (timeline) | own | **P1** |
| `anim.speed` | Speed | **A** `factor` 0.1…10 *1*; or fixed `fps`; `minDelay` 20 ms (browsers round < 20 ms up to 100 ms!) | yes | own | **P1** |
| `anim.loop` | Loop | **A** `count` 0 = infinite…65535; `pingPong` (forward-backward) off; `reverse` off | yes | own | **P1** |
| `anim.dropFrames` | Thin out frames | keep every n-th / target fps; merge similar frames (difference < x → add durations) *on, x = 0.5 %* | yes | own | **P1** |
| `anim.frameDelay` | Frame duration | all equal (ms) / first/last longer (pause at the end 0…10 s) | yes | own | P2 |
| `anim.toFrames` | Animation → image sequence | format per frame, name `{name}_{frame:4}` | – | own | **P1** |
| `anim.fromFrames` | Image sequence → animation | **collect step**: all (filtered) items in list order → one animation; `delay` ms *100*; `size` first/largest/fixed | yes | own | **P1** |
| `anim.spriteSheet` | Sprite sheet | direction: animation → sheet (columns, spacing, margin; `atlas` json/css/none *json*: extra file `<name>.json` `{image, size, columns, rows, frames:[{x,y,w,h,delayMs}]}` or `<name>.css` with one class `.<name>-<n>` per frame, written next to the sheet; coordinates follow later resize steps) **and** sheet → frames (grid N×M) | yes | own; atlas via the executor's extra-output hook | P2 |
| `anim.crop`, `anim.resize`, colour ... | – | all geometry/colour steps act on every frame (same parameters, computed in parallel per frame) | yes | as for still images | **P1** |
| `anim.firstFrame` | Still from animation | **A** `pick` first/middle/last/number/time/sharpest *first*; **A** `number` 1… (clamped to the last frame); **A** `time` ms (frame shown at that time, after the end → last) | yes | own (sharpest = highest variance of the Laplacian) | P2 |
| `anim.stabilize` | Calm background | freeze static areas over time (removes noise, helps compression) | yes | own (§5.6 "temporal smoothing") | P2 |
| `anim.boomerang`, `anim.crossfade` | Special | cross-fade at the loop end (n frames) | yes | own | P3 |

Output of an animation through "Save as": **GIF** (with the optimizer, §3.9), **APNG**
(`fp-export::animation::apng`, frame optimization: difference rectangle, blend op), **animated
WebP** (`animated_webp`, lossy/lossless), **animated AVIF** (P3, needs an AV1 sequence encoder;
`ravif` only does stills), **image sequence** (`anim.toFrames`), **MP4** – not planned (encoder
patents; P3 only via the user's ffmpeg).

### 3.9 GIF optimizer "small GIFs, very good quality" (part of "Save as GIF")

Parameters in the end card for format GIF (Simple mode shows **preset**, **quality** and **target
size**; dedicated fields, no JSON). Choosing another preset sets the quality (60/80/100) in the
same undo step. Fields without a value ("Automatic") are chosen by the encoder from quality and
content. Below the fields: the size reached by the preview file, or ⚠ "Target size not reachable:
smallest file ..."; during a run the warning goes into the report (`batch-warn-gif-target-missed`).

| Parameter | Range, *default* | Effect |
|---|---|---|
| **A** `preset` | smallest / balanced / best *balanced* | sets the following values |
| **A** `quality` | 1…100 *80* (smallest 60, best 100) | controls colours, dithering strength, loss |
| **A** `maxKB` | 0 (off) … 100,000 | target size: search over quality → fps → size (§3.11) |
| `colors` | 2…256 *automatic* | maximum palette size; without a value derived from quality and content (graphics 44, photo 92 at quality 80) |
| `palette` | global / perFrame / auto *auto* | auto: global if the error is below a threshold, otherwise per frame (local colour table) |
| `dither` | none / ordered / blueNoise / floydSteinberg / temporal *temporal* | `temporal`: temporally stable dithering (§5.6) |
| `ditherAmount` | 0…100 *70* | |
| `lossy` | 0…200 *automatic* (graphics (100 − q)·2, photo (100 − q)·5) | lossy LZW (§5.6, own implementation) |
| `diffTransparency` | on | unchanged pixels → transparent (better LZW runs) |
| `diffThreshold` | 0…32 *3* (distance in Oklab × 100) | "unchanged" for small differences |
| `cropChanged` | on | store only the rectangle of changes per frame |
| `mergeSimilar` | on | merge nearly identical frames (durations added) |
| `disposal` | auto / keep *auto* | auto picks "do not dispose" or "background" per frame, whichever is smaller |
| `transparency` | on | store transparency as GIF transparency |
| `background` | colour *#ffffff* (only with `transparency` off) | fill colour for semi-transparent pixels |
| `splitFrames` | on/off *automatic* (on below quality 95) | separate changes of one frame as individual sub-images (each 20 ms before the frame change) |

### 3.10 Document steps (FW-PNG/PSD/SVG as documents, group "Document")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `document.findReplace` | Find and replace | `whatToFind` text/font/color/url/nonWeb216 (as in the original), `find`, `replace`, `matchCase`, `wholeWord`, `regex` | yes | existing Find panel / commands in fp-engine | P2 |
| `document.pages` | Choose pages/states | which page/state is exported (all → one file each) | yes | `fp-engine` | P2 |
| `document.layers` | Show/hide layers | layer name (regex) visible/invisible before export | yes | registry `layer.setVisible` | P2 |
| `document.slices` | Export slices | like export "HTML and Images" without HTML | – | `fp-export::slices` | P3 |

### 3.11 Optimize and compress (group "Optimize")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `optimize.targetSize` | Target size | **A** `maxKB` 1…100,000; `strategy`: quality only / quality + resize *quality only*; `minQuality` 10…95 *40*; `minScale` 10…100 % *50* | yes (size reached) | bisection over quality (JPEG/WebP/AVIF; GIF: quality, lossy, colours, then fps/size; PNG: colours 256→16, then size); at most 8 encodes per file; result reports the size reached, ⚠ if not reachable | **P1** |
| `optimize.png` | Optimize PNG losslessly | **A** `level` 1…6 *2*; `strip` safe; `alphaClean` on | yes (size) | `oxipng` 9.x (MIT, pure Rust with the `libdeflater` feature off; check feature licences) | **P1** |
| `optimize.jpeg` | Optimize JPEG losslessly | optimized Huffman tables, progressive, metadata per the "Metadata" step | yes | own (`fp-export::jpeg` has Huffman optimization) | P2 |
| `optimize.webSafe` | For the web | sRGB, 8 bit, metadata without GPS, orientation applied | yes | combined step | P2 |
| `optimize.quantize` | Reduce colours (PNG8) | 2…256, dithering as for GIF | yes | `quantette` 0.6 (MIT/Apache, k-means in Oklab, Wu) + own dithering | **P1** |

### 3.12 Vector and automation (group "Advanced")

| ID | Name | Parameters | V | Method | P |
|---|---|---|---|---|---|
| `vector.trace` | Trace → SVG/FW-PNG | **A** `preset` logo/blackWhite/illustration/photo; `colors`, `filterSpeckle`, `mode` ... as `modify.traceBitmap`; `refine`; output SVG (paths) or FW-PNG (editable) | yes (rendered) | `fp-trace` (present) | **P1** |
| `script.registry` | Firepixel command | `command` (choice/search in the registry), `params` (form from the schema); several as one step | yes | command registry | **P1** |
| `script.command` | Saved command | file from the commands folder (excluding `//--not for batch--` and batch scripts, as in the original) | yes | `fp-script` `commands.run` | **P1** |
| `script.code` | JSF code | code field (Pro); `fw.getDocumentDOM()` is the current item; variables `batch.index`, `batch.total`, `batch.source` | yes | `fp-script` | P2 |
| `script.condition` | If ... then | condition (width > ..., has transparency, is animation, name matches) → following card(s) only then | – | own | P3 |
| `script.external` | External program | command line with `{in}`/`{out}`; Pro only, explicitly confirmed, not via MCP | – | `std::process` with `child_env` | P3 |

---

## 4. Renaming (end card "File name")

### 4.1 Simple mode: builder

The card shows a row of **building blocks** at the top (chips, draggable, × to remove), below it
"+ Block" and the options. At the very bottom is the **live table** (§4.3).

Building blocks (`parts[]`, order = order in the name):

| Block (`kind`) | Chip shows | Options |
|---|---|---|
| `original` | "Original name" | part: whole / first n characters / last n characters / without trailing digits |
| `text` | the text | free; spaces shown as `␣` |
| `counter` | "Number 001" | **start** 0…999,999 *1*, **step** 1…1000 *1*, **digits** 1…9 *3* (with leading zeros), restart per folder off; order = list sorting |
| `date` | "Date 2026-10-13" | source: today / modification date / **EXIF capture date** (falls back to modification date, with hint); format templates `YYYY-MM-DD`, `YYYYMMDD`, `YYYY-MM-DD_HH-mm-ss`, `DD.MM.YYYY`, free (Pro) |
| `size` | "1920x1080" | image size after the steps (`{w}x{h}`) or of the source; width/height only |
| `folder` | "Folder name" | name of the source folder |
| `exif` | "Camera" | make, model, lens, ISO, aperture |
| `format` | "webp" | target format |

Options below the blocks (applied in this order, §4.4):
1. **Replace text**: "Replace [___] with [___]", checkbox "all occurrences" (*on*; off = first only,
   as in the original), "Match case" (*on*). Several rows possible (+).
2. **Replace spaces with** [`_`] (choice `_`, `-`, nothing, free).
3. **Prefix** [___] / **Suffix** [___] (suffix goes before the extension).
4. **Case**: unchanged / lower / UPPER / First letter upper / Title (Each Word) / `kebab-case` /
   `snake_case` / `camelCase` (Pro).
5. **Transliterate umlauts** (ä→ae, ß→ss, é→e) off; **Remove special characters** off.
6. **Extension**: lower (default) / unchanged / upper; `jpeg`→`jpg` on.

Card defaults: one `original` block, everything else off → the name stays the same (only the new
extension).

### 4.2 Pro mode: regex

- Toggle in the card "Builder | Regex". Fields **Find** (regex) and **Replace** (with `$1`,
  `${name}`, `$0`, and the placeholders `{n:3}`, `{date:YYYY-MM-DD}`, `{exif-date:…}`, `{w}`, `{h}`,
  `{folder}`, `{ext}`, `{name}`). Checkboxes: ignore case, all matches (*on*), apply to the whole
  name including the extension (*off*).
- **Plain-language explanation** below the field (from the syntax tree of `regex-syntax`, same crate
  family as `regex`, MIT/Apache): e.g. `^IMG_(\d+)` → "at the start "IMG_", then one or more digits
  (group 1)". Building blocks: literal, character classes (`\d` digit, `\w` letter/digit/_, `\s`
  whitespace, `[a-z]`), repetitions (`+`, `*`, `?`, `{2,4}`), (named) groups, alternatives,
  anchors. Anything unknown → "(expression)". Texts in `batch_rename.ftl`.
- **Cheat sheet** ⓘ with 8 examples to insert (remove trailing number, reorder a date
  `(\d{4})-(\d\d)-(\d\d)` → `$3.$2.$1`, collapse spaces, swap prefix ...).
- Invalid expression: field red, the library's message wrapped in a translated frame ("Error at
  position 7: ..."), the table shows the old names muted; "Process" disabled.
- Regex engine: `regex` 1.x (MIT/Apache; no backreferences/lookaround, but guaranteed linear time –
  no hangs with 10,000 files). Lookaround only if needed: `fancy-regex` (MIT), P3.

### 4.3 Live table old → new

- Below the card (when expanded) or in the preview tab "Plan": columns **Old**, **→**, **New**,
  **Note**. Character-level highlighting of the change (inserted tinted green, removed red
  strikethrough, `ui_kit::tint`). Shows the first 200 rows + "... and 1,834 more" (all of them in
  the dry run).
- Notes per row: ⚠ **conflict** (same target name as row x, or already exists in the target),
  ⚠ **invalid characters replaced**, ⚠ **shortened**, ⚠ **empty name** → `_`, ⚠ **leading dot** →
  `_` (as in the original), ⓘ **EXIF date missing** (fallback used).
- Conflict resolution as in §2.7 (default `name (2)`); in the builder additionally a button "Append
  number" that inserts a `counter` block.
- **Size token** (`size` part, `{w}`/`{h}` in the regex replacement): the name uses the size
  **after** the steps. The dry run (`fp_batch::predict`) computes it from the step parameters when
  it follows from them alone (`resize`, `crop` without smart, `canvas`, `canvas.pad`, `rotate` by
  multiples of 90°, `overlay.border`, size-neutral colour/overlay/metadata steps) and resolves
  conflicts on those names exactly like the run; otherwise the row keeps the source size with the
  note ⓘ **size after processing** (`batch-hint-size-after-processing`). An item the plan skips
  because its source-size name is taken is re-checked after processing (dry run: with the
  predicted name), so it is written when its final name is free.

### 4.4 Order of name construction (binding, used by tests)

1. Base name without extension (`.fw.png` counts as one extension).
2. Assemble the building blocks (Simple) or apply the regex (Pro; to the base name, unless "whole
   name" is set).
3. Replacements (Simple, rows in order) → spaces → prefix/suffix → umlauts/special characters →
   case.
4. Invalid characters → `_`, reserved names → append `_`, shorten the length.
5. Empty name → `_`; leading dot → `_`.
6. Append the extension of the **target format** (for `file.copyOnly`: the original extension).
7. Conflict resolution (§2.7), deterministic in list order.

---

## 5. Technical design

### 5.1 Architecture

Crate **`crates\fp-batch`** (library, no UI):

```
fp-batch/src/
  lib.rs          public API: Recipe, Job, run(), preview(), plan(), StepRegistry
  recipe.rs       serde model, migration, validation
  item.rs         Item = Still(RgbaImage+Meta) | Animation(FrameStream) | Document(session document) | FileOnly(PathBuf)
  step.rs         trait Step { id, group, params_schema(), validate(), apply(&mut Item, &Ctx) -> Result<()>, preview_ok(), kind: PerItem|Collect }
  pipeline.rs     steps per item, proxy preview, generations
  executor.rs     thread pool (rayon), memory budget, cancellation, progress channel
  naming.rs       §4 (rename), conflicts, plan/dry run
  output.rs       encoding, temporary file + atomic rename, backup
  report.rs       batch-report.json/html, batch-log.txt
  steps/          files.rs, geometry.rs, adjust.rs, background.rs, overlay.rs, metadata.rs,
                  anim.rs, gifopt/ (palette.rs, dither.rs, lzw_lossy.rs, frames.rs, size.rs),
                  optimize.rs, vector.rs, script.rs
  video/          mod.rs (trait VideoSource), ffmpeg.rs, mp4.rs (+ h264.rs, av1.rs), webm.rs
```

Dependencies: `fp-engine` (session, command registry), `fp-script` (opening files, JSF), `fp-io`,
`fp-export`, `fp-raster`, `fp-render`, `fp-text`, `fp-effects`, `fp-ai`, `fp-trace`, `fp-i18n`.
`fp-ui` depends on `fp-batch` (module `crates\fp-ui\src\batch\`), as do `firepixel-cli` and
`fp-mcp`.

**Connection to the command registry** (no dependency cycle): `fp-engine` has
`src\commands\batch.rs` with a hook `trait BatchHost` (same pattern as `script_host`), which
`fp_batch::install_host(&mut Session)` sets. Commands:

| Command | Parameters | Result |
|---|---|---|
| `batch.run` | `recipe` (object or path), `files[]` or `folder` + `recursive`, `dryRun`, `threads` | report (like `batch-report.json`) |
| `batch.plan` | as `run` | list source → target with conflicts, nothing written |
| `batch.preview` | `recipe`, `file`, `maxSize` | PNG (Base64) before/after + sizes |
| `batch.steps` | `language?` | catalogue of all steps with schema (for agents) |
| `batch.validate` | `recipe` | errors/warnings per step |
| `file.batchProcess` | (existing) | stays backward compatible; new optional `recipe` → forwards to `batch.run` |

Long runs over MCP: `batch.run` with `async: true` returns `{jobId}`; `batch.status {jobId}`,
`batch.cancel {jobId}` (P2). Without `async` the call blocks until the end.

### 5.2 How a job runs

1. **Collect**: paths → `Item` descriptions (header only: format, size, frames, duration).
2. **Plan** (`naming.rs`): target names for all items **before** the start, conflicts resolved,
   output folder created (not in a dry run). The result is deterministic, independent of
   parallelism.
3. **Execute**: rayon pool with `threads` = cores − 1 (at least 1, configurable). Per item: decode →
   steps in order → encode → temporary file `.<name>.fptmp` → `rename`. Collect steps
   (`anim.fromFrames`, `overlay.contactSheet`, PDF "all in one") run after the per-item steps in a
   second phase.
   **Extra outputs** (hook `write_extras`): after the main file, producers may write further files
   next to it with the same stem: the editable FW-PNG (`alsoFwpng`) and the sprite-sheet atlas
   (`.json`/`.css`). Each extra file is written atomically, follows the recipe's conflict rule
   (rename `name (2).ext`, skip with a warning, overwrite) and is listed in the report under
   `extraTargets`. Without an extra producer the run writes exactly one file per item, byte-identical
   to before.
4. **Memory budget**: estimated need per item = W × H × 4 × (2 + steps that copy), or for animations
   the frames in the window; a byte semaphore (default min(40 % of free RAM, 6 GB)) admits only as
   many items at once as fit. Items larger than the budget run alone. Videos are streamed in blocks
   of 32 frames; the GIF optimizer needs two passes (palette, then encoding) → frames are kept as
   compressed intermediate images (QOI in memory or a temp file).
5. **Cancel**: `CancelToken` (`Arc<AtomicBool>`, pattern from `fp-ai::cancel`), checked between
   steps, frames and tile rows; encoders are not interrupted hard, their result is discarded.
6. **Progress**: `crossbeam-channel` or `std::sync::mpsc` with events `ItemStarted`,
   `ItemProgress(f32)`, `ItemDone{bytes}`, `ItemFailed{msg}`, `Finished`. The UI polls once per frame
   (no blocking), `ctx.request_repaint_after(100 ms)`.
7. **Documents** (FW-PNG/PSD/SVG, steps `document.*`, `script.*`, `effect.live`): a separate headless
   `Session` per worker thread (like `firepixel-cli batch`), no access to the UI session. Open UI
   documents are copied as snapshots.

### 5.3 Special case "file steps only"

If the recipe contains only `rename`, `file.*` and "Save as: same as source" with no other steps,
nothing is decoded: copy (default) or move (only for the same location, chosen explicitly).
Exception: `{w}`/`{h}`/EXIF blocks – then only the header is read.

### 5.4 Preview pipeline

`preview(recipe, item, generation)` runs on its own thread (one slot; a new request discards the
old one): decode a proxy (JPEG with DCT downscaling where possible), apply steps with
`preview_ok()`, show the result as a texture; in parallel a full encode for the size display
(cancellable). Cache per (file, modification time, step-prefix hash) → only recompute from the
changed step onwards.

### 5.5 Video decoding

Requirements: single executable, project licence Apache-2.0 (no GPL/AGPL/LGPL linking, only
permissive licences such as MIT/Apache/BSD/Zlib), no `unsafe` in own code, patents considered.

| Option | Formats | Licence | Assessment |
|---|---|---|---|
| **A. The user's ffmpeg** (process, not linked, not shipped) | everything (H.264, HEVC, VP8/9, AV1, ProRes, MOV, MKV ...) | ffmpeg itself is LGPL/GPL – not an issue because the user installs it and Firepixel only calls it as an external program | **P1**: search in `PATH`, setting "ffmpeg file..." (Preferences ▸ Batch), environment variable `FIREPIXEL_FFMPEG`. Call `ffmpeg -ss … -t … -i in -vf fps=…,scale=… -f rawvideo -pix_fmt rgba -` (pipe, streamed), metadata via `ffprobe -of json`. No shell, arguments as a list, `child_env`. |
| **B. Built-in: MP4/MOV demux + H.264** | H.264 (Baseline, Main incl. CABAC, large parts of High) in MP4/MOV | `re_mp4` (MIT, demux, fork of `mp4`) + `rusty_h264-decoder` (BSD-2, `forbid(unsafe_code)` in the core; feature `asm` off → no `unsafe`) | **P2**. Covers phone and screen recordings. Young (0.x) → behind feature `video-h264`, fuzz corpus, fallback to A. |
| **C. Built-in: AV1** | AV1 in MP4/WebM | `rav1d` (BSD-2, already in the tree via `sqzer-rav1d` for AVIF) + demux (`re_mp4`; WebM: `matroska-demuxer`, MIT/Apache, check licence when adding) | **P2**, almost free because the decoder is already there. |
| D. Operating system (Media Foundation / AVFoundation) | whatever is installed | – | P3; needs FFI/`unsafe` in third-party crates and per-OS code; only if A/B are not enough. |
| E. Built-in HEVC, VP9 | – | no mature permissive Rust solution; HEVC has several patent pools without a free tier | **do not** build in → only via A. |

**Patents**: H.264 decoding is patent-encumbered; the AVC pool has, to our knowledge, a
royalty-free volume threshold for end products (≈ 100,000 units/year), and many patents expire
around 2027–2030 – this is **not legal advice**; have it checked before any sale. AV1 is
royalty-free (AOMedia patent commitment). HEVC: do not build in. Using the user's ffmpeg moves the
question to the user.

**Recommendation**: **P1 = A** (ffmpeg optional, detected or selectable; without ffmpeg the video row
shows "Video decoder missing" with an explanation and a button "Choose ffmpeg file..."; nothing is
downloaded). **P2 = C + B** behind Cargo features `video-av1` (default on) and `video-h264` (default
after the user's decision, §7); order `auto`: built-in if the codec is supported, otherwise ffmpeg.
Animated inputs (GIF/APNG/WebP/AVIF/JXL) need no video decoder (`fp-io` already decodes them).

### 5.6 Small high-quality GIFs (own implementation, no GPL/AGPL components)

Not used: gifski (AGPL), gifsicle/giflossy (GPL), libimagequant/pngquant (GPL or commercial),
zenquant (AGPL). The techniques are known from publications and are implemented independently
(`fp-batch\src\steps\gifopt\`):

1. **Pre-processing**: frames to target size (Lanczos), thin out the frame rate, **merge nearly
   identical frames** (add durations), **temporal smoothing**: pixels that change only minimally
   compared with the previous frame (Oklab distance < `diffThreshold`) are set to the previous
   value → stable background, large transparent areas.
2. **Palette**: weighted colour statistics (edges and skin tones not under-represented; pixels
   unchanged from the previous frame count less). Quantization with `quantette` (k-means in Oklab,
   MIT/Apache, 0.6) or the existing `fp-export::quant` (close to the original). `auto`: global
   palette (saves 768 bytes per frame), local palette only for frames whose mean error with the
   global palette exceeds a threshold (scene changes). One index stays reserved for transparency.
3. **Dithering**: `temporal` = ordered dithering with a **blue-noise threshold matrix** that stays
   **fixed** across frames (no flicker, good LZW runs), adaptive strength (more in smooth areas,
   less at edges); Floyd–Steinberg with serpentine scanning as an option, but only in changed areas
   (static areas take over the indices of the previous frame → no "crawling" patterns). The 64×64
   blue-noise matrix is generated in-house (void-and-cluster, Ulichney 1993) and stored as a file
   in the crate.
4. **Frame difference + transparency**: pixels with the same palette index as the visible previous
   image → transparency index; **crop** to the rectangle of changed pixels; per frame choose
   `disposal` (keep/background) by the smaller encoded size.
5. **Lossy LZW** (own implementation of the known principle): when extending a dictionary entry, a
   pixel is accepted if its colour distance to the "expected" next index is below a threshold
   (derived from `lossy` 0…200) – longer codes, fewer bits; the error is diffused to the neighbours.
   The existing "loss" of the original-style export (`fp-export::animation::prepare_frames`,
   parameter `lossy`) remains unchanged for the Export dialog; the optimizer has its own encoder.
6. **Target size**: bisection over `quality` (controls colors/lossy/ditherAmount together), then
   fps, then size in 10 % steps; at most 8 passes; estimation measured on a subset of frames
   (every 4th), the last pass in full.
7. **Measurement**: test corpus `tools\golden\batch\gif\` (5 short clips: screen recording, photo
   camera pan, cartoon, gradient, pixel art), metrics size and SSIM (own SSIM on Oklab L). Goal:
   "balanced" ≥ 30 % smaller than the classic GIF export at equal or better SSIM.

### 5.7 UI files (`crates\fp-ui\src\batch\`)

`mod.rs` (Studio state, hooked into `app_state`/`workspace` with one line), `banner.rs`,
`files.rs` (list, drag and drop, filter, thumbnails following the `welcome\thumbs.rs` pattern),
`recipe.rs` (cards, gallery, dragging), `params.rs` (form from schema:
number/slider/choice/colour/file/time/blocks), `rename_ui.rs`, `preview.rs` (compare, zoom,
timeline), `output_bar.rs`, `report_ui.rs`. Shared building blocks only from
`ui_kit`/`widgets`/`welcome` (`paint_banner`, cards, `empty_state`, `progress`, `notice`, `table`).
New general building blocks (segmented button, chip, timeline) live in `batch\kit.rs` and move to
`ui_kit` later.

### 5.8 CLI and MCP

```
firepixel-cli batch --recipe recipe.fpbatch.json (--in <folder> [--recursive] | <files...>)
                    [--out <folder>] [--conflict rename|skip|overwrite] [--dry-run] [--threads N]
                    [--report report.json] [--json]
firepixel-cli batch --steps [--json]            # catalogue
firepixel-cli batch --actions ...               (existing, unchanged)
```

Exit code 0 = everything done, 1 = at least one error, 2 = invalid invocation, 3 = cancelled.
Progress on stderr (one line per file; with `--json` as NDJSON). MCP: tool `batch` (shorthand for
`run_command batch.run`) with `recipe`, `files`, `folder`, `dryRun`; resource
`firepixel://docs/batch.md` (catalogue). `docs\AGENTS.md` has a "Batch" section.

### 5.9 Libraries

| Crate | Version (check and pin with `=` when adding) | Licence | Purpose | P |
|---|---|---|---|---|
| `regex` / `regex-syntax` | 1.11+ / 0.8+ | MIT/Apache-2.0 | renaming, filters, plain-language explanation | P1 |
| `walkdir` | 2.5 | MIT/Unlicense | recursive folders | P1 |
| `globset` | 0.4 | MIT/Unlicense | glob filters | P2 |
| `quantette` | 0.6.0 | MIT/Apache-2.0 | palette (k-means Oklab, Wu) | P1 |
| `oxipng` | 9.x, `default-features = false` (without `libdeflater`/C) | MIT | lossless PNG | P1 |
| `kamadak-exif` | 0.6 | BSD-2-Clause | reading EXIF | P1 |
| `img-parts` | 0.3/0.4 | MIT/Apache-2.0 | read/replace/remove metadata segments in JPEG/PNG/WebP | P1 |
| `xmp-writer` | 0.3 | MIT/Apache-2.0 | writing XMP packets | P2 |
| `little_exif` (optional) | 0.x | MIT/Apache-2.0 | writing EXIF (alternative) | P2 |
| `filetime` | 0.2 | MIT/Apache-2.0 | timestamps | P2 |
| `re_mp4` | 0.3+ | MIT | MP4/MOV demux | P2 |
| `rusty_h264-decoder` | 0.16 (without `asm`) | BSD-2-Clause | H.264 | P2 (feature) |
| `matroska-demuxer` | 0.x | MIT/Apache (to check) | WebM/MKV demux | P2 |
| already present | `rayon`, `serde`, `serde_json`, `sha2`, `png`, `webp`, `ravif`, `sqzer-rav1d`, `zune-jpeg`, `image-webp`, `jxl-oxide`, `heic-decoder`, `visioncortex` | permissive | | |

Not allowed: gifski, gifsicle, libimagequant/imagequant, zenquant, ffmpeg bindings (`ffmpeg-next`
links LGPL/GPL), `libheif`, `exiv2`/`rexiv2` (GPL); `qcms` only after approval (MPL-2.0). Every new
dependency is recorded with its licence; `cargo tree` is checked for GPL/AGPL/LGPL licence fields (a
test in `fp-batch\tests\licenses.rs` reads `cargo metadata`, P2).

### 5.10 Tests

- **Unit tests** per step (`fp-batch\tests\steps_<group>.rs`): result pixels/size/metadata are
  checked, not just "no error".
- **Golden files** `tools\golden\batch\<case>\{in,expected}\` + `recipe.json`; comparison with
  tolerance (max. channel deviation 2; for lossy formats SSIM ≥ 0.98 and size ± 5 %); `out\` excluded
  via `.gitignore`. Cases: web shrink, JPEG from transparent PNG (white), rename by EXIF date, video
  → GIF (via ffmpeg only if present, otherwise `#[ignore]` with a reason), trim/speed up GIF, remove
  background (`#[ignore]`, needs a model), metadata without GPS, trace → SVG, target size 100 KB.
- **Logic cases**:
  1. *Repetition*: change a parameter 50× (slider, every key) → the recipe contains the step once,
     the preview shows the final value, the recipe history has one entry per drag; add and remove a
     step 50× → no leftovers; run twice in a row → the second run produces `name (2)` or skips per
     rule, never doubled effects in the image.
  2. *Preview/cancel*: the preview replaces instead of adding (effect step + parameter change → one
     effect); cancelling a run → no `.fptmp` leftovers, finished files complete, originals unchanged
     (checksum); cancelling the gallery leaves the recipe unchanged.
  3. *Undo/redo* in the recipe in sequences (A, B, ↶, C, ↷).
  4. *Selection change*: selecting another file while the preview is computing → the preview shows
     the new file, never the result of the old one; switching cards while a field has focus → the
     value lands in the right step.
  5. *Edge cases*: empty list, 1 file, 10,000 files (plan < 2 s), identical names from two folders,
     Unicode/emoji/umlauts/very long paths, read-only source, target without write permission, a
     broken file in the middle of the list (run continues), 0-byte file, 30,000 × 30,000 px (memory
     budget, runs alone), animation with 1 frame, video without decoder, animation with frames of
     different sizes, CMYK JPEG, 16-bit PNG, greyscale with alpha.
- **UI** (egui_kittest, `crates\fp-ui\tests\batch_*.rs`): layout, drag and drop (`drop_files`),
  gallery, dragging cards, preview divider, progress; `clipping_audit` 0 findings (3 themes × de/en ×
  100/150 % × 1024×768/1920×1080, plus `qps`); images in `docs\screenshots\batch\` per theme/language.
- **CLI/MCP**: `apps\firepixel-cli\tests\batch_recipe.rs`, `fp-mcp` tool test.
- **Neutrality**: `cargo test -p fp-neutral` green; `cargo test -p fp-i18n` (de/en completeness).
- Everything with `FP_NO_SHELL_OPEN=1` and a test profile, never the real user profile.

---

## 6. Relation to the original's batch wizard

The Studio does **not** rebuild the original's wizard control by control; position, sizes, greying
rules and initial states of the original windows are not a requirement. Consequently, none of the
detailed measurements of the original wizard (file list rules, initial content, ▲/▼ ordering,
command list filtering, export list and "Edit" dialog, scale limits, output/backup enabling rules,
what is remembered, progress window and log, extension handling on format change, rename-only
move vs. copy) are needed – the Studio defines its own rules in §2–§4. Only a future P3 feature,
"import old batch scripts", would require the serialization format of the original's saved batch
scripts and its "files to process" handling.

---

## 7. Design decisions

| # | Question | Decision / recommendation |
|---|---|---|
| 1 | Does the Studio fully replace the classic wizard in the menu? | **Yes.** *File ▸ Batch Processing...* opens the Studio; all functions of the original are included (§1.2). The parity entry for the batch dialog reads "replaced by the modern batch mode". |
| 2 | Built-in H.264 (P2, `rusty_h264`, patent question) or only via ffmpeg? | **P1 only ffmpeg (optional, installed by the user)**; built-in AV1 in P2; built-in H.264 as a feature, enabled by default only after checking the patent situation/volume (not legal advice). |
| 3 | Default target | **New folder `batch-output` next to the sources**, conflict "choose a new name"; never replace originals without an explicit choice. |
| 4 | Where does the Studio appear? | **As a "Batch" tab in the main window** (like the start page), not a separate window. |
| 5 | Allow MPL-2.0 (`qcms` for ICC conversion)? | **Not for now**; an own conversion for matrix profiles covers sRGB, Display P3 and other RGB matrix profiles; CMYK via the existing table. |
| 6 | AI upscaling/faces (P3) | Later, only with permissively licensed models; download only after consent in the models dialog. |
