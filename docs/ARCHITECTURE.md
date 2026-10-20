# Architecture

Firepixel is a Cargo workspace of small crates in clear layers, two binaries and no runtime dependencies beyond
the operating system. The design follows three ideas: **pure data model**, **one command registry for every
caller**, and a **CPU reference compositor** that is tested against measured output.

## 1. Workspace and layers

Dependencies point downwards only. Libraries live in `crates/`, programs in `apps/`.

```text
apps/firepixel        GUI (eframe/wgpu), single exe, static C runtime, assets embedded
apps/firepixel-cli    run / batch / render / info / commands / convert / script / mcp

fp-ui        egui user interface: main window, menus, docking, canvas interaction, inspector, panels,
             dialogs, themes, icons, start page, MCP bridge endpoint (bridge.rs, bridge/)
fp-mcp       MCP server (rmcp): stdio and Streamable HTTP on 127.0.0.1 with token; headless or bridge
fp-script    JSF compatibility: JavaScript (boa_engine) with fw / document DOM / Files / Errors → commands
fp-batch     Batch studio core: recipes, steps, pipeline, parallel processing, renaming, report,
             MCP caller gate for external programs and unsandboxed scripts (security.rs)
fp-engine    command registry, undo/redo history, selection, clipboard, tool state machines,
             preferences, session (UI-independent)
fp-export    export: optimize settings, palettes, quantizers, dithering, GIF/JPEG/PNG8/24/32, animation,
             slices, HTML/CSS/behaviours, PSD (layer per object, type layers, vector masks, layer effects),
             PDF (invisible text layer), WebP, AVIF, APNG
fp-io        import: PSD, GIF, JPEG, PNG, BMP, TIFF, TGA, WBMP, AI/EPS/PDF, SVG, WebP, APNG, JPEG XL, QOI,
             ICO, clipboard
fp-ai        on-device AI selection (pure-Rust CPU inference), model management with verified download
fp-trace     bitmap tracing to editable Bezier paths
fp-render    document compositor (CPU reference): pages × states, layers, masks, symbols/9-slice,
             blend modes, live effects, caches, partial rendering, hit testing
fp-text      fonts at runtime, text layout (point, frame, on path), glyph outlines, rasterisation
fp-vector    path rasterizer, fills/gradients/patterns/textures, strokes/brushes, boolean path operations
fp-effects   live effects (shadows, glows, bevel/emboss, colour fill, imported layer effects)
fp-raster    pixel buffers (RGBA8, copy-on-write tiles), masks, compositing, resampling, bitmap filters
fp-fwpng     FW-PNG reader/writer (.fw.png): PNG container, private chunks, tag tree ↔ fp-model
fp-model     document model (pure data, serde)
fp-color     colours, colour models, exact CMYK, blend formulas, swatch files (ACT/ASE/ACO/GIF)
fp-geom      points, rectangles, matrices, Bezier curves, flattening
fp-i18n      Fluent localisation, language selection, fallback chain, user profile paths

fp-neutral   guard tests: no third-party product names in UI texts and manuals; tests never touch the real profile
fp-testlist  build helper: one test program per crate
```

Nothing below `fp-ui` uses egui, eframe or winit. The workspace forbids `unsafe` code.

## 2. Document model

`fp-model` holds pure data without UI or render dependencies: document → pages → states (frames) → layers →
elements (paths, bitmaps, text, groups, symbol instances, slices, hotspots), plus symbols, styles, swatches
and the web layer. IDs are stable `u64` handles instead of pointers. Fields read from a file are kept even when
Firepixel does not use them, including unknown tags, so files round-trip unchanged.

## 3. Commands: one registry for everyone

Every menu item, tool action, dialog and panel action is a **command** in the `fp-engine` registry:

- a dotted name (`modify.group`, `filter.blur.gaussian`, `file.open` …),
- JSON parameters (serde) with a schema, requirements (document, selection, element types) and an example,
- `apply` on the session, producing **one undo step** (several commands can be grouped into one step),
- a localised label and description, and its default shortcut.

The UI, the CLI (`firepixel-cli run --cmd …`), JSF scripts and MCP (`run_command`) all call this registry and
nothing else. `firepixel-cli commands` and the MCP resource `firepixel://docs/commands.md` list the full
catalogue. The history panel shows agent commands like user actions; the maximum depth comes from preferences.

Tools are state machines in `fp-engine/src/tools` fed with raw pointer and key events; `fp-ui` only translates
OS input into these events and draws overlays. This is what makes `tool_event` and `ui_input` over MCP behave
like a human. UI-side helpers that belong to one tool (for example the background hover preview of the AI
object selection) live in `fp-ui/src/canvas/tools/`. Which parts of the window each screen mode shows (Standard, full screen with
menus, full screen) is decided in one place, `fp-ui/src/screen_mode.rs`.

A tool whose state mirrors the document implements `ToolMachine::sync`, which the session calls after undo, redo, a
jump in the History panel, a document command and the tool's own emitted commands. The Pen keeps the path being
drawn in the document (one History step per point) and rebuilds its nodes, handles included, from it in `sync`, so
Ctrl+Z while drawing removes only the last point and drawing goes on from there. `ToolMachine::blocks_history`
reports a running stroke during which undo and redo are ignored (Vector Path with the button held), and
`ToolResponse.group` runs the response's commands as one named History step (for example the Pen closing a path it
started, or joining another open path).

Commands that reach outside the document (screen capture through the system's capture tool, the symbol-script
generator, the shared palette over all open documents) live in `fp-engine/src/commands/extras.rs`.

## 4. Rendering

`fp-render` is a CPU reference compositor, bit-exact to the measured behaviour where it is known, and tested
against the flat PNG preview stored in FW-PNG files (corpus and golden tests). The canvas shows its result as
512-px texture tiles on a device-pixel grid anchored at the page corner (`Renderer::render_device`, so parts of
one view fit together and equal a whole render): scrolling uploads only new tiles, an edit only the changed
rectangle of the tiles it touches, and after a zoom the visible tiles come first and the margin follows (BA-098;
without a GPU, texture upload runs at only ~30 MB/s). The drag preview texture likewise holds only the part of the
dragged objects in view plus half a view of margin and is re-cut when scrolling or zooming leaves it (BA-097).
Partial rendering and caches keep large documents interactive. For live bitmap painting the canvas renders the changed rectangle with `Renderer::render_region_with` and a `PixelOverride` that swaps in the
scratch pixels of the painted bitmap, so the preview sits in the right stacking position with its opacity, blend
mode, effects and masks. The inspector's opacity and stroke width sliders preview their pending value the same
render-only way: the inspector stores it in `AppState::inspector_preview` (`fp-ui/src/inspector/preview.rs`), the
canvas renders a copy of the document with the value applied to the selection under its own `RenderKey.preview`, and
the session's document, history and dirty flag are never touched; the preview is reset every frame, and the single
real apply goes through the normal command on release or when the pop-up closes. Pixel buffers are RGBA8 in
128 × 128 copy-on-write tiles (cheap undo). `rayon` parallelises filters and large areas; long operations run
in the background with progress so the UI never blocks.

## 5. File format (FW-PNG)

A `.fw.png` file is a valid PNG (the flattened image of the first page) with private chunks that hold the
editable document as a tag tree (document tree, tiles of bitmap data, fonts and previews). `fp-fwpng` maps the
tag tree onto `fp-model` and back. Files written by Firepixel identify themselves in a `tEXt Software` chunk
("Firepixel r2" since the symbol-origin change; older Firepixel files are converted on load). Files from the
reference application are read and written without conversion.

Other formats: see `fp-io` (import) and `fp-export` (export). Only pure-Rust crates or statically linked C
libraries are used; no DLLs ship next to the executable.

## 6. Automation

- **CLI:** `firepixel-cli run <file> --cmd <name> --params <json> --out <file>`, `batch`, `render`, `info`
  (document tree as JSON), `commands`, `convert`, `script`, `mcp`.
- **JSF:** `.jsf` scripts run on boa_engine; `fw` and `dom` map onto the command registry with the signatures
  and type rules of the original scripting API. Saved commands live in the `Commands` folder of the profile.
- **MCP** (`fp-mcp`): deliberately small tool set — `list_commands`, `describe_command`, `run_command`,
  `get_document`, `select`, `render`, `open`/`save`/`export`, `undo`/`redo`/`history`, `new_document`,
  `documents`, `tool_event`, `run_script`, `batch` — plus the built-in docs as resources.
- **Bridge:** with `firepixel.exe --mcp` (or *Commands ▸ AI Control Channel (MCP)*) the running app serves MCP
  over Streamable HTTP on 127.0.0.1 with a 256-bit token written to `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`.
  Requests run on the UI thread against the user's session, so changes are visible and undoable. Extra
  bridge tools: `screenshot`, `ui_state`, `ui_action`, `ui_find`, `ui_input` (raw mouse/keyboard events),
  `ui_file_dialog`. `firepixel-cli mcp --bridge` relays stdio to the bridge without copying the token.
  Details: [AGENTS.en.md](AGENTS.en.md).

Security model: localhost only, token on every request (constant-time compare), `Host` header check against DNS
rebinding, no shell commands; files are touched only through open/save/export and JSF `Files`.

## 7. Internationalisation

All visible text comes from Fluent files in `crates/fp-i18n/locales/<lang>/*.ftl`, split by area. German is the
source language, English is built in, further languages are loaded at runtime from
`%APPDATA%\Firepixel\Languages\`. Switching is immediate (no restart). Technical names never change with the
language. A pseudo-locale (`qps`) and tests find hard-coded strings, missing keys and clipped labels. MCP answers
default to English independent of the UI language. Guide: `crates/fp-i18n/README.md`.

## 8. User interface

`fp-ui` reproduces the layout of the reference application: menu bar, main toolbar, tools panel with groups and
flyouts, document tabs with Original/Preview/2-up/4-up, property inspector at the bottom (layout depends on the
selection), dockable panel groups on the right, status bar. Shortcuts and mouse handling (drag, Shift/Alt
modifiers, double-click) follow measurements of the original. Three themes — Normal (light grey), Dark and OLED
(pure black, colourful accents) — come from one central `Theme` structure; widgets contain no hard-coded colours.
All icons are our own vector drawings. Design rules: [UI_DESIGN.md](UI_DESIGN.md).

## 9. Profiles and files

Settings, languages, saved commands, AI models and bridge files live under `%APPDATA%\Firepixel` and
`%LOCALAPPDATA%\Firepixel` (macOS/Linux: the platform equivalents), always with English folder names. Test runs
get their own temporary profile automatically; writing to the real profile from a test panics.

## 10. Tests and measurement

- Unit tests per crate; corpus/golden tests compare output with measured reference results.
- UI tests offscreen (kittest, wgpu); `fp-ui` and `fp-engine` link all integration tests into one `all` binary.
- Parity runs against the reference application: [PARITY.md](PARITY.md).
- Behaviour descriptions and measurements of the reference application, the test corpus and the comparison
  run data are kept outside this repository (they describe a third-party program and use files that cannot be
  redistributed). The code cites them by section number only.
