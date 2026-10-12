# Firepixel document model (short version for agents)

Firepixel reproduces the document model of the original. Files are FW-PNGs (`.fw.png`):
a normal PNG (flat image) with embedded, editable data.

## Structure

```
Document
└─ Pages (pages[], one current page; at most one master page)
   ├─ Canvas: width, height, origin, resolution, canvas color
   ├─ States (states, called “frames” in the original too; for animation/rollovers)
   └─ Layers (layers[], order bottom → top; the web layer is the last one)
      └─ one content per state (only one for layers “shared across states”)
         └─ Elements top → bottom
            ├─ rectangle   rectangle primitive (corner roundness)
            ├─ path        vector path (contours of Bézier nodes)
            ├─ text        text (point or area text, runs with font/size/color)
            ├─ bitmap      pixel image
            ├─ group       group (children); maskGroup: the first element is the mask
            ├─ autoShape   auto shape (group with script and control points)
            ├─ compoundShape  compound shape (boolean operands)
            ├─ instance    symbol instance (graphic, button, animation)
            └─ slice / hotspot  web objects (only on the web layer)
```

## IDs and selection

- Every element has an **ID** (integer) that stays stable during the session (also after moving,
  grouping neighbors, undo). New objects return their ID in the result of `run_command`.
- Layers have IDs too (`layer` parameter of the `layer.*` commands).
- Most commands act on the **selection** (`select`, `select.set`). Some take `ids` directly.
- After a command, `run_command` returns the new selection with bounds.

## Coordinates and values

- Unit: document pixels, origin at the top left of the canvas (the canvas origin can be ≠ 0).
- Rectangle: `[left, top, right, bottom]` or `{left, top, right, bottom}` or `{x, y, width, height}`.
- Point: `[x, y]` or `{x, y}`.
- Color: `"#rrggbb"` or `"#rrggbbaa"` (alpha last), `"none"` = transparent.
- Opacity: percent 0–100. Blending modes: `normal`, `multiply`, `screen`, `overlay`, `darken`, `lighten`, …
- Bounds in `get_document` are the visible bounds (including stroke), rounded to 0.01 px.

## History

- Every document or selection command is an undo step (`undo`, `redo`, `history`).
- `run_command` with `steps` combines several commands into one step.
- View commands (zoom, grid) create no step.

## States and pages

- `page.setCurrent {index}`, `state.setCurrent {index}` switch page/state; `get_document` shows
  `currentPage`/`currentState`. `render` and `get_document` also take `page`/`state` directly.
