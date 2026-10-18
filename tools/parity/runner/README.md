# Parity runner

Plays the scripts in `tools\parity\scripts\<unit>.json` unattended in **both** programs (the reference
program and Firepixel), stores equally named screenshots, runs `tools\parity\compare.py` and builds a
gallery. No path or name of the reference program is stored in the repository: everything program
specific is read from `..\re\_testdateien\par\runner.config.json` (outside the repo, next to the test files).

## Call

```
python -I tools\parity\runner\runner.py --units a,b,c      # selected units
python -I tools\parity\runner\runner.py --all              # every script except _selftest*
           [--steps id1,id2] [--dry-run] [--no-gallery] [--gallery-only] [--config <json>]
python -I tools\parity\runner\runner.py --write-testfile-sums  # TESTFILES.sha256 from the (checked) test files
```

Python: `C:\Users\Administrator\AppData\Local\Programs\Python\Python314\python.exe` (Pillow). `--dry-run`
only lists the steps and verbs. `--gallery-only` rebuilds `index.html` from existing results.

## What happens per unit

1. Take the system-wide lock (same named mutex as the other automation tools; name in the config).
2. Reference program: start if needed (decline recovery questions), restore to `orig.rect` (1600 × 1000 at 0,0),
   bring it to the foreground. Firepixel: copy the newest `Firepixel-Vorschau-*` build into
   `fp.work_dir\app`, start it with `--mcp`, own profile (`APPDATA`/`LOCALAPPDATA` redirected, theme
   `normal`, language `de`), shown without activation at `fp.rect` and pushed to the bottom of the z-order.
3. Open the unit's `testfile` (T01…T10 → file via `testfiles` in the config) in both: reference with
   `fw.openDocument(url)` (never with the third argument), Firepixel with the bridge tool `open`; zoom 100 %.
4. Per step and side (reference first, then Firepixel): light reset (close menus and dialogs, Escape), actions
   `both` + `orig`/`fp`, screenshot of `shot.area`, then `after`. A failing action ends that side of the
   step; the side is recovered (crash → restart, reopen the test file) and the run continues.
   Before every step (light reset) the properties panel is made visible again (`Fenster > Eigenschaften`, only
   when it is not visible – the entry toggles), because some steps switch its dock group to another tab
   (e.g. `Fenster > Symboleigenschaften`).
   **Unit start in the reference program (R-2):** default layout via `ui_map → orig_layout` (the 'hide panels'
   switch off when its entry is checked or a 'show' label is offered, the listed toolbars on, tools column
   visible), then the tools anchor is measured. There is no fallback anchor: if the column is not found after
   restoring the layout, the unit is **aborted for the reference side** (status `aborted`, log line
   `UNIT ABORTED (orig)`), Firepixel still runs. The align panel's 'to canvas' switch is set to off (state read from
   `orig.align_state_file`, switched with the panel's own button). Before every step the runner checks that no other
   program (a browser) holds the foreground and brings the reference program forward (3 tries).
   Canvas shots (`area: canvas`) are cropped to the document area (page rectangle inside the view, plus
   `ui_map → canvas_shot_margin`); in the reference program floating panel groups over the view are closed first.
   `area: canvas_view` gives the whole view as before. A shot that is still one flat colour is grabbed again
   (up to 5 × 300 ms). Import: a page question after the file dialog (`ui_map → orig_import_confirm`) is confirmed
   before the next canvas action, which then places the import.
5. End of unit: close menus/dialogs, close the test documents without saving, stop the Firepixel instance,
   release the lock, run `compare.py`. At the end of the run: `index.html`; the reference program is
   closed only if no instance was running when the run began (a restart during the run does not change that).

Modal windows: besides standard dialogs (`#32770`) the program has embedded panel dialogs (top-level class
`SWFPLAYER_SWFWINDOWCONTAINER`, e.g. `Datei > Datei-Info`) without Win32 buttons; they count as dialogs
(shot area `dialog`, which waits up to 4 s for a late dialog) and are cancelled with `WM_CLOSE`. An embedded panel dialog
left open blocks the menu bar, so `menu` closes them first and tries opening the bar entry up to 3 times
(closing menus/embedded panel dialogs and giving the foreground back to the main window in between); the error then
names whether the main window is enabled and which windows are open.

Robustness: the target process is the one owning the visible main window (`orig.main_class`, searched
across all processes of `orig.process`), so a script launcher still waiting for a busy instance is never
taken for the program. Questions are declined (`No` first – save/recovery questions –, then `Cancel`;
never `Yes`/`Save`/`OK`). Processes of the runner without any window (waiting launchers, hung restarts)
are ended; foreign processes never. No second instance is started while one is running. Every unit
starts with all documents closed unsaved. After `orig.max_consecutive_errors` (default 3) failed steps
in a row with a systemic error (program gone, timeouts, foreground lost, `menu did not open`, panel not shown
even after its menu entry) the program is restarted cleanly and the unit goes on. A script whose hand-over got lost
(launcher ended, script never started) is handed over again; a `.started` marker makes the second copy
a no-op.

Real input to the reference program (SendInput) is only sent while its window (or one of its dialogs/menus/
panels) owns the foreground; otherwise the action fails with "input refused". A briefly empty foreground
(dialog or menu just closed) is waited for (≤ 2 s) and then given back to the main window; a foreground of
another process is never taken over. Ctrl+Alt+Space is never sent.
Firepixel never gets real input: everything goes through `ui_input`/`ui_action` of the bridge.

## Results

```
re\_testdateien\par\run\<unit>\orig\<step id>.png    reference program
re\_testdateien\par\run\<unit>\fp\<step id>.png      Firepixel
re\_testdateien\par\run\<unit>\cmp\                  compare.py: <id>.png (orig | fp | diff ×4), summary.csv/md
re\_testdateien\par\run\<unit>\log.jsonl             one line per step (see below)
re\_testdateien\par\run\index.html                   gallery: failures (red) and notable pairs first
```

`log.jsonl` line: `{"unit", "step", "n", "check", "area", "time", "result": {"orig": R, "fp": R}}` with
`R = {"status", "error"?, "actions": [{"verb", "ms"}], "rect"?, "area_from"?, "shot"?, "values": [...], "s"}`.

| status | meaning |
|---|---|
| `ok` | all actions done, screenshot written |
| `error` | program/automation failure (red in the gallery; side recovered afterwards) |
| `unsupported` | verb or target form the runner does not implement (yet) |
| `unmapped` | logical UI key without a locator for this side in `ui_map.json` |
| `skipped` | step with `cursor` (mouse pointer images cannot be captured) |
| `slow` | finished, but longer than `step_timeout_s` |

## Script format

```json
{ "unit": "inspector-text", "testfile": "T05", "note": "…",
  "steps": [
    { "id": "03_font_list_open", "check": "font list", "both": [ {"select": "TX1"}, {"open": "inspector.font"} ],
      "orig": [ {"jsf": "…"} ], "fp": [ {"ui_find_click": "Schrift"} ],
      "shot": {"area": "popup", "delay_ms": 300}, "after": [ {"key": "Escape"} ], "log_menu_items": false } ] }
```

Step keys: `id` (file name), `check`, `method`, `note` (ignored), `both`/`orig`/`fp` (actions), `shot`
(`null` or missing = value-only checkpoint), `after` (actions after the shot), `log_menu_items` (texts +
enabled/checked of the open menus into `values`), `cursor` (step is logged as skipped), `orig_only`/`fp_only`.

Shot areas: `window` (client area), `canvas` (document view), `inspector` (= `panel:properties`),
`panel:<id>` (panel group incl. tab header; ids see `ui_map.json → panels`), `dialog` (top dialog), `menu`
(union of open menus), `popup` (other top-level windows of the reference program; Firepixel: difference box).
For `menu`/`popup` in Firepixel, and whenever no window is found, the area is the bounding box of what
changed compared to a window image taken before the step's actions (`area_from: difference` in the log;
the image is taken 0.3 s after the previous action so that pending repaints are not counted).
Firepixel `dialog`: the frame is found in the window image starting at the title bar node (which covers only
the title text) incl. close button and button row; the in-app file dialog is anchored at its title text.
Firepixel `panel:<id>` of a panel group floating over the canvas: its own frame (window image), not the column.

Defined layout: Firepixel starts with a fresh profile for every unit (`fp.fresh_profile`, default true:
`runner_work\fp\profile` is deleted). In the reference program floating panel groups (`orig.floating_dock_class`,
default `OWL.Dock`) are closed at the start of every unit, and before every step when they cover the document view;
a floating group holding the properties panel is never closed.

View switches: the reference program keeps grid, guides, rulers, smart guides, edges … across documents (Firepixel per
document). After every opened test file and before a step that follows an action touching them (`Ansicht` menu/`view.*`
command, `setShow*`/`setSnap*`/`lockGuides` JSF, Ctrl+Ä/Ü keys) both sides set them to `ui_map.json → view_defaults`
(JS API `get…`/`set…`; changes are logged as `view switches reset: …`). Not covered: slice overlay (no API).

`set_field` `commit`: `enter` | `tab` | `none` (alias `then`); default `tab` for a field in an open dialog (the value is taken
over, the dialog stays open, the script's next action - Enter, Escape, Vorschau … - decides), else `enter`.

An action is an object whose first known key is the verb; further keys are options (`mods`, `repeat`,
`item`, `value`, `target`, `by`, `hold_key`, `ms`). A plain string is a verb without argument.

### Verbs

Aliases: `reset <T..|null>` = `open_testfile` (null: only close menus/dialogs), `record` = `log_value`,
`command` with `orig_menu` uses that menu path on the reference side, `file_dialog "cancel"`, step key `cleanup` = `after`.

| verb | argument | reference program | Firepixel |
|---|---|---|---|
| `open_testfile` / `reopen` | `"T04"` (or `true` = unit file) | close same file unsaved, `fw.openDocument(url)` | `documents close` + `open` |
| `close_all` / `close_all_documents` | – | closes **every** open document unsaved (also files a script opened itself) | same (`documents close`, error if one stays open) |
| `menu` | `"A > B > C"` (also `A/B` without `>`, list) | click bar entry, hover sub menus; a **leaf entry at the end is executed** (hover + click, Enter if the menu stays open), a sub menu at the end stays open; abbreviated entries (`Gaußscher Weichz.`) match | same; leaf = no new texts appear next to the hovered entry |
| `menu_open` / `hover_menu` | same | only opens/hovers (leaves the last open, for menu shots) | same |
| `menu_item` | `"A > B"` | like `menu`, last entry clicked | same |
| `command` | Firepixel command id | menu path from `ui_map.json → commands` | `ui_action {run}` |
| `close_dialogs` | – | cancel dialogs, close menus | `ui_action {close_dialogs}` |
| `key` | `"ctrl+shift+z"`, `"Escape"`, `"F1"` (`repeat`) | SendInput; plain Enter/Escape with a dialog open (no popup menu) press its OK/Cancel button, Tab/Shift+Tab with the focus in a dialog = `WM_NEXTDLGCTL` (the program's dialogs do not translate these keys: SendInput Enter/Tab end up as a character in the field) | `ui_input key` (egui names) |
| `type` | text | Unicode SendInput | `ui_input type` |
| `wait` | ms | sleep | sleep |
| `select` | object name or list; `{index: i}` = i-th element over all layers; `"slice:N"` = N-th element of the web layer(s) (R-11) | JSF `fw.selection` by name | same JSF via `run_script` (exactly the named objects; `select {ids}` only as fallback) |
| `select_none` / `select_all` | – | JSF | `select` |
| `tool` | `tool.rectangle` | click on the toolbox button (`ui_map.json → tools`; single-key shortcuts do not reliably reach the program) | `ui_action {tool}` |
| `panel` | `window.layers` / id / title | visible? else menu `Fenster > …` | `ui_action {panel}` if not visible (fallback: tab / menu, see below) |
| `jsf` / `run_script` | code | JSF in the running instance | `run_script` |
| `log_value` / `log` | JSF expression | value into `values` | `run_script` value (objects/arrays serialised with `toSource()`, R-12) |
| `click` / `double_click` / `right_click` / `hover` / `open` / `click_text` / `click_button` / `toolbox_click` | target | SendInput at the target | `ui_input` at the target |
| `mouse_down` / `mouse_up` | target | unsupported | `ui_input down/up` |
| `canvas_click` / `canvas_double_click` / `canvas_move` / `canvas_hover` | `[x, y]` document px (+`mods`) | page rectangle measured in the view image (+ `orig.canvas_offset`) | `page_on_screen` |
| `drag` / `canvas_drag` | `{from, to | by, mods, steps, allow_outside_end}` (points = `[x,y]` document px or targets; `allow_outside_end: true` = the end point may lie outside the document view, R-12) | SendInput drag | `ui_input drag` |
| `choose` | `{target, item}` (`item` may be `A > B`) | Win32 combo box: select entry | click target, click item texts |
| `set_field` / `set` / `set_dialog_field` | `{target, value, commit?}` | edit in a dialog: click, `WM_SETTEXT`, then `commit` (Tab = `WM_NEXTDLGCTL`, Enter = Tab + OK button); other fields: double click, Ctrl+A, type, `commit` key | click (double click if needed) - **only if a text field took the focus** (else error, so Ctrl+A never reaches the canvas), Ctrl+A, type, `commit` key |
| `record_ui` | logical key (`ui_state.*`, `document.summary`, `menu.open`, `dialog.fields`, `focus`, …) or `{"focus_cycle": n}` | dialogs (title + control texts)/menu items/focused control (Tab n times) | `dialog.*`: title, texts in reading order and controls (role, text, state) from `ui_find` inside the dialog; `menu.*`: texts of the open menus (columns, shortcuts) inside the difference box; `focus_cycle`: `focus_gained` of `ui_input`; else `ui_state`/`get_document` |
| `clear_dir` / `record_dir` / `record_file` / `record_file_text` | path (relative = `run\<unit>\files\…`, only below the test folder) | file listing / size + SHA-1 + pixels | same |
| `file_dialog` / `file_dialog_choose` / `file_dialog_cancel` | path | edit field of the open file dialog + OK / Cancel | `ui_file_dialog` |
| `ui_action`, `ui_find`, `ui_state`, `get_document`, `ui_file_dialog`, `ui_find_click` | bridge arguments | – (put under `fp`) | bridge tool |

Verbs for popups, flyouts, handles and scrolling:

| verb | argument | reference program | Firepixel |
|---|---|---|---|
| `choose` on a non-combo target | `{target, item: "A > B"}` (+ `orig_index`) | opens the target (press-and-hold if the locator has `hold_ms`), then walks the popup menu by item text (hover sub menus, click the last); `orig_index` = raw item index (separators count) for owner-drawn menus without texts (tool flyouts) | opens the target the same way, clicks the item texts |
| `fx_add` | `"Schatten und Glühen > Schlagschatten"` | `choose {target: inspector.fx_add, item}` | same |
| `flyout_open` / `flyout` / `flyout_item` | target (`toolbox.<tool>_flyout`) [+ `item`] | press-and-hold on the tool button (`hold_ms`), optional item as in `choose` | same (`ui_input` down / wait / up) |
| `panel_menu` | panel id [+ `item`] | shows the panel, clicks the group's menu button (`orig_panel_menu_offset` from the group's top-right corner), optional item path | clicks the `Bedienfeldmenü` button in the panel's tab row |
| `drag_popup_slider` / `slider_drag` | `{to_fraction, hold?, from_fraction?}` | the open popup slider (smallest extra window of the process): press near the target, move to the fraction of its long axis (vertical: top = maximum); `hold: true` keeps the button down until the next reset | same with the `slider` element from `ui_find` |
| `scroll` / `wheel` | `{area | target, delta, mods?, horizontal?}` (`area` = shot area or `{preview_pane: i}`) | mouse wheel at the centre; `delta` > 0 scrolls the content down (wheel towards the user) | `ui_input wheel` (same sign rule) |
| `canvas_wheel` | `{pos: [x, y] document px, delta, modifiers?}` or `{view_frac: [fx, fy], …}` | wheel at the document point; `delta` > 0 = wheel away from the user | same |
| `if_no_dialog` | `{wait_ms, do: [actions], why, remeasure, unsupported_if_none}` | R-11: waits up to `wait_ms` for a dialog; only when none opened, `do` runs (a second way to open it; logged as `if_no_dialog` in the values). R-12: `remeasure: true` measures the reference's page again before `do`; `unsupported_if_none: reason` makes the side `unsupported` with that reason when still no dialog is open after `do` (nested `if_no_dialog` in `do` = further ways) | same (`dialog_open` of `ui_state`) |
| `swatch_readback` | `{n, panel, pref_want, retry_item}` | R-11 (TW-115): logs JSF `fw.swatches` (count, first 8), the colours of swatch cells 0..n-1 (grid `panel_swatch`) and the pref `SwatchesPanelColorPaletteChoice`; pref not `pref_want` -> panel menu again, `retry_item` clicked once more | same without pref/retry |
| `drag_handle` / `click_handle` / `hover_handle` | `{handle, by?}` | control point of the selected object (`fw.selection[0].controlPoints`); `handle` = index or role (tooltip prefix via `handle_roles`, e.g. `points` → „Punkte“); without control points the 8 frame handles of `pixelRect` (0 = top left, clockwise) | `run_command autoShape.describe` of the selected element, else its bounds |
| `mouse_down` / `mouse_up` | target | SendInput down/up (released at the next reset) | `ui_input down/up` |
| `export_png` | `"doc"` | `fw.exportDocumentAs(dom, url, null)` (document's own export settings) into `run\<unit>\files\<step>_orig.png`, size + SHA-1 + pixels into `values` | bridge `render {}` into `…_fp.png` |
| `log_menu_items` (as action) | – | texts/states of the open popup menus into `values` | `ui_find` menu items |

Target forms `{handle: i|role}`, `{menu_item: "Text"}` (item of an open popup menu), `{preview_pane: i}`
(reference: i-th preview child window of the open dialog, Firepixel: `Vorschaufeld <i+1>`) and
`{dialog_rel: [x, y]}` (offset from the top-left of the largest open dialog) work on both sides.

More target forms (RUN-5), both sides:

| form | reference program | Firepixel |
|---|---|---|
| `{layer_row: "Ebene 1"}`, `{object_row: 0 \| "Name", layer?}`, `{layer_eye \| layer_lock \| layer_expand \| layer_name: "Ebene 1"}` | the layers panel is an owner-drawn Win32 list box without item texts: the row list is rebuilt from the document (JSF `Layer.elems` = sub layers and objects in display order, `disclosure` = expanded) and checked against the list box (row count, layer rows 24 px, object rows taller); the row is scrolled into view (`LB_SETTOPINDEX`, host `lb_info`), the column x offsets are fixed (eye 11, lock 32, triangle 51 (+16 for a sub layer), name 110) | row text via `ui_find`; rows scrolled out of the small panel are reached with the mouse wheel; eye/lock at fixed x, triangle 28 px left of the name |
| `{popup_swatch: "#RRGGBB"}` | the open colour popup (own window) is captured, the first cell of exactly that colour is used | the same in the window image, region above the popup's HEX field |
| `{pref_category: "Bearbeiten"}` | row of the preferences category list (`SysListView32`, items not readable): index from `ui_map.json → orig_pref_categories` (script names and their aliases), row height 17 px; plain labels of these categories work too | text |
| `{field_arrow: i}` | popup slider arrow next to the i-th edit field of the open dialog (the `Button` without text right of it) | 10 px right of the i-th text field |

`{list_row}`, `{tree_item}`, `{history_row}` (reference: looked up by name with UI Automation, works only for standard
list/tree controls), `{panel_swatch}` and grid cells (`style_cell`, `char_cell`, `color_table_cell` …) are still open.

Other RUN-5 rules:
* `{"repeat": n, "do": [actions]}` runs the actions n times (both sides).
* `menu`/`menu_item` with the menu path of a panel in `ui_map.json → panels` (`Fenster > Ausrichten` …) is handled
  like `panel` (only shown when not visible; the menu entry would hide a visible panel). `"toggle": true` keeps the
  plain menu command.
* Firepixel `panel` for panels without an engine command (Ausrichten, Pfad, Farbpalette, Sonderzeichen,
  Bildbearbeitung, Symboleigenschaften …, unknown to `ui_action panel` and missing in `visible_panels`): click its tab
  when it is there, else its menu entry.
* Every opened test file starts with the pointer tool on both sides (the reference program keeps the last tool,
  runner finding 3). After `select` the reference side waits until the properties panel changed (≤ 3 s).
* Reference `open` (no field under the point): if neither a menu nor a new window appeared within 2.5 s it clicks once
  more (the first click after a selection change sometimes only activates the rebuilt panel). Menus opened by
  `choose`/`open` are polled for up to 2.5 s (they appear with a delay).
* Owner-drawn sub menus (stroke categories `Einfach > Präzise abger.` …): item order in `ui_map.json →
  orig_owner_drawn_menus`; `orig_index` with a path `A > B` = index in the sub menu of A.
* `ui_map.json → menu_aliases`: groups of names (reference first, then Firepixel) of the same menu entry where they differ (bevel live
  filters); tried when the script's name is not found.
* `ui_map.json → orig_labels`: label → reference locator for icon buttons without Win32 text (align panel).
* Test file `T11` is a folder (`par_import`, not opened at unit start); `"T11:par.psd"` = a file inside it; `T12` =
  swatch file. Firepixel targets `dialog.slider.<i>`.
* The tools column of the reference program has no window class of its own: `panel:tools` = client left edge to the
  document view.

RUN-6 additions:

| verb / form | reference program | Firepixel |
|---|---|---|
| `export {dir, name, type, slices?}` | only `type` „Nur Bilder“ without slices/options: `fw.exportDocumentAs(dom, <dir>\<name>.<ext>, null)` with the document's export settings, `ext` from `exportOptions.exportFormat`; folder listing into `values`. Other types (HTML, Segmente, Status/Ebenen/Seiten in Dateien, PDF, CSS-Sprites) stay `unsupported` | bridge `export {path}` with the same extension |
| `window_size [w, h]` | main window resized at `orig.rect`'s position (host `place`) | `SetWindowPos` without activation at `fp.rect`'s position |
| `"{side}"` / `"{unit}"` in any action argument | replaced by `orig`/`fp` and the unit id (separate export folders per side) | same |
| `record_ui "dialog.controls"` | child windows (class, text, rect) of the open dialogs - for measuring locators | – |
| `{dialog_child: {class, index}}` | negative `index` counts from the last match (`-1` = last slider arrow of an effect popup) | – |
| `{find, from_left: dx}` | – | point `dx` right of the label's left edge (controls without own node, e.g. colour well) |
| `tools` entry `{flyout, orig_index}` | tool inside a toolbox flyout: press-and-hold the group button, click entry `orig_index` | `ui_action {tool}` |

After `window_size` the next step first restores the configured size on both sides.

Not supported yet (logged as `unsupported`, the run continues): `read_dialog_fields`, `export` of other types,
`resize_panel` and `drag_text` (they change the reference program's saved workspace layout, which would leak into
later units), `resize_dialog`, `drag_dialog_*`, `restart_app`, `log_focus_cycle`, `set_clipboard_image`,
`drop_files` and the other rare verbs from the scripts' notes.

### Targets

A target is either
* a **logical UI key** from `ui_map.json → targets` (e.g. `inspector.fill_color`, `dialog.ok`),
* a **label** (button, menu entry or other visible text; compared without `&` and trailing `...`/`…`):
  reference program = text of a Win32 child window or of an item in an open menu; Firepixel = `ui_find`,
* `{"canvas": [x, y]}` (document pixels), `{"window_rel": [x, y]}` (client coordinates),
  `{"area": "panel:layers", "pos": [x, y]}` (offset inside a shot area),
* `{"target": <target>, "mods": ["shift"]}`,
* `dialog.field.<i>` / `dialog.combo.<i>` / `dialog.check.<i>` / `dialog.button.<i>` (built-in pattern, no map entry
  needed): reference = i-th Win32 `Edit` / `ComboBox` / `Button` child of the open dialog (window order);
  Firepixel = i-th `text_input` / `combo_box` / `check_box` / `button` from `ui_find`, sorted top-to-bottom,
  left-to-right. `choose {target: "dialog.combo.0", item}` selects the entry directly in the reference program,
  `set_field {target: "dialog.field.1", value}` sets the edit text,
* `{"dialog_field": i}`, `{"button": "OK"}`, `{"text": "…"}` (forms used by some scripts).

## ui_map.json

The mapping table. Script authors add entries here instead of hard-coding coordinates in scripts.

* `menu_bar.orig`: `[name, x, y]` of the reference program's custom-drawn menu bar (client coordinates).
  Open sub menus are standard popup menus: their items, states and rectangles are read directly.
* `panels`: panel id → `fp` panel command, German `title` (window title of the reference panel / tab text
  in Firepixel), `menu` path to show it. Ids are matched without `window.`/`panel:` and with `-` = `_`
  (`window.colorMixer` = `color-mixer`).
* `tools`: Firepixel tool id → reference shortcut key (or `{"jsf": …}`).
* `commands`: Firepixel command id → reference menu path.
* `targets`: logical key → `{"orig": locator, "fp": locator}`; `null` or a missing side = `unmapped`. A
  locator that is a string is an alias of another target (e.g. `"dialog.radius": {"orig": "dialog.field.0", …}`).
  * reference locators: `{"panel": "properties", "rel": [x, y]}` (offset from the panel window's
    top-left), `{"window_rel": [x, y]}`, `{"tools_rel": [x, y]}` (offset from the anchor of the docked tools
    column = centre/top of its first group heading, measured from the column image left of the document view;
    the runner shows the column at unit start; R-1), `{"child": {"title"?, "class"?, "index"?}, "panel"?}` (Win32
    child window), `{"dialog_child": {…}}`, `{"dialog_button": "ok"|"cancel"}`, `{"label": "Text"}`,
    `{"canvas": [x, y] | "NAME" | "NAME.node.N" | "NAME.center"}` (named points from the document: path = its
    first node, other objects = centre of the pixel rect; also for `canvas_click` & co.), `{"dialog_rel": [x, y]}`,
    `{"popup_child": {"class", "index"}}` (control in a popup window of the program), `{"area", "pos"}` with
    `pos` = [dx, dy] or a name from `ui_map → area_points`. For `rel`/`dialog_rel` points the runner asks which Win32
    window lies under the point: an `Edit` gets its text set directly, a `ComboBox` is chosen with
    `CB_SELECTSTRING` (so `set_field`/`choose` work on the custom-drawn inspector, too).
  * Firepixel locators: `{"find": "Text", "role"?: "button", "exact"?: true, "panel"?: "layers", "index"?: 0}`,
    `{"window_rel": [x, y]}`, `{"canvas": [x, y]}`, `{"dialog_rel": [x, y]}`.
  * options on either side: `"hold_ms": 900` (open = press-and-hold, tool flyouts), `"select": "100%"`
    (click = choose this combo entry, e.g. `preview.zoom_100`).
  * The inspector keys (`inspector.*`) are measured for the docked properties panel of the 1600 × 1000 window:
    vector keys with a rectangle (T04 R1), text keys with a text object (T05 TX2); live filter rows
    `inspector.fx_row.<i>` / `fx_row_info.<i>` / `fx_row_check.<i>`; `fx_popup.*` are the edit fields of the
    reference program's filter popup (in Firepixel the filter settings are a dialog: not mapped yet).
* `handle_roles`: handle role → German tooltip prefix of the control point; `orig_panel_menu_offset`;
  `orig_neutral_point` (empty spot of the menu bar, clicked to close a popup slider the reference program leaves open).

The reference program exposes **no UI Automation tree** (its panels, menu bar and inspector are custom
drawn); only Win32 child windows (buttons, edits, combo boxes inside panels/dialogs, panel windows titled
with their German name) and standard popup menus can be found by name. Everything else needs a `rel`
locator measured once from a screenshot of the 1600 × 1000 window.

## Configuration (`runner.config.json`, outside the repo)

`orig.exe`, `orig.process`, `orig.main_class`, `orig.view_class` (document view), `orig.panel_class`,
`orig.lock` (mutex name), `orig.rect`, `orig.start_wait_s`, `orig.progress_dialog_regex` (script progress
dialog that must stay open), `orig.canvas_offset` (optional correction for document → screen),
`orig.align_state_file` / `orig.align_state_key` (store file of the align panel's 'to canvas' switch, R-2),
`fp.preview_glob`, `fp.work_dir`, `fp.rect`, `fp.theme`, `fp.language`, `testfile_dir`, `testfiles`
(T-code → file), `out_dir`, `tmp_dir`, `step_timeout_s`, `fp.shortcut_set` (R-13: shortcut set written into
Firepixel's runner profile, default "Firepixel"), `orig.shortcut_set_pref` (R-13: reference preference holding
its active set, default `CurrentShortcutSet`).

## Files

| file | role |
|---|---|
| `runner.py` | command line, unit/step loop, verbs for both sides, recovery, compare call |
| `origctl.py` | lock, start/restart, JSF execution (done marker + log), host process wrapper |
| `orig_host.ps1` | long-running PowerShell host: Win32 windows/panels/dialogs/popup menus, SendInput with foreground check, screen capture |
| `fpctl.py` | Firepixel copy + profile + start, MCP bridge client |
| `gallery.py` | `index.html` |
| `ui_map.json` | mapping table (see above) |

Self test: `python -I runner.py --units _selftest_menu,_selftest_inspector`.

## Limits

* Document → screen: after opening, documents larger than the view (T10, 2400 × 1600) are fitted in both
  programs (reference: `orig.fit_all_key`, default Ctrl+Alt+0; Firepixel: `ui_action {fit}`); the reference
  page rectangle is then measured from the view image (pixels differing from the pasteboard colour) and
  re-measured after keys/menus/commands. If the page touches the view border (scrolled), the centre rule
  is used, which is only right at 100 %.
* `log_menu_items` in Firepixel only finds entries exposed as `menu_item` by `ui_find` (often none);
  the reference side reads the real popup menu items.
* Tooltips and mouse pointer images are not captured (flyouts are: they open on press-and-hold).
* Popup sliders: the reference program's slider opens below its arrow (it may reach beyond the 1000 px window
  on a taller screen), Firepixel's above; `drag_popup_slider` works with fractions, so both get the same value.
* The reference program must not be used by a person while the runner holds the lock (it needs the foreground).

## R-2 additions (16.10.)

* Firepixel: `{"find", "panel"?, "below_role", "fallback"?}` = nearest node of that role below the found label
  (inspector-web/04: export format under 'Typ:'). `area: popup` = the dialog rectangle while a dialog is open.
  `panel_menu` checks in the window image that the menu opened (again, then the other buttons of the row).
  `record_ui popup.visible`: reference = new top-level windows/menus since the step began; Firepixel = change
  against the window image before the step's last `open` (or a dialog open), `{"visible": bool, …}` on both sides.
* `choose`: entries are matched loosely when the exact text is missing (no spaces/ellipsis/case, unique prefix,
  `ui_map → combo_aliases`, e.g. "PNG 32" = "PNG32"); the reference error lists the combo's entries.
* Error traces in `log.jsonl` keep 8 frames (1500 characters).

## R-4 additions (16.10.)

* Reference side: every input command (key, type, click, move, drag, wheel, down/up, menu) first reads the host's
  foreground guard and takes a foreign foreground back; a command refused because another process is in front is
  sent once more after bringing the program forward (`GuardedHost`).
* Drags end with a 60 ms wait and one more move at the target before the button is released, on both sides (the
  reference program uses only the second-to-last position of a drag, RE-6). Firepixel drags are sent as
  move/down/moves/up events (held keys via a `modifiers` event).
* Panels hidden with Tab are shown again before the next step (Firepixel: `ui_state.panels_hidden`; reference:
  `ensure_layout` after a Tab outside dialogs). The view switches are reset only with an open document.
* Unit start, reference: stroke/fill colours back to the defaults (`setDefaultBrushAndFillColors`) with the first
  opened document. Canvas shots: the reference re-reads the document size first (image size / new document);
  Firepixel switches off panel groups floating over the canvas (window commands; groups without one stay).
* Import (reference): after the file dialog the runner waits for the program, then moves onto the target point
  before the click/drag that places the import.
* Firepixel `set_field`: End + Shift+Home instead of Ctrl+A. `panel_menu` remembers the popup's area: the next
  text click and `log_menu_items` look there first (`log_menu_items` reads only the changed region that holds
  the popup and its sub menus). Target `{"list_row": name, "icon": true}` = the row's icon (double click there).

## R-5 additions (16.10.)

* Reference foreground: `Original.focus()` asks the host for a hard take-back up to 6 times (pauses 0.3/0.6/1.2/2.4/
  4.8 s): Esc when a taskbar window (`Shell_TrayWnd` …) is in front, `AttachThreadInput` + `BringWindowToTop` +
  `SetForegroundWindow`, and as last resort one click on a neutral spot of the program's caption (WM_NCHITTEST =
  caption, the point must belong to that window). A refused input is resent up to 5 times with the same pauses.
  The same take-back runs before `open_testfile` and each `reset_light`; every unit logs which window is in front.
* Reference drags wait after every input until the program's UI thread takes messages again (WM_NULL with
  timeout); a long wait is logged (`program busy … ms`). Run 9: on the 2400 x 1600 bitmap the moves were
  coalesced and nothing moved (`xs-canvas/08,12`).
* Canvas shots of a zoomed page (page cut by the view): on a cut axis both sides take the same centred span
  (`ui_map.canvas_clip_max`, default 1000 x 640). The reference's centre rule uses the current zoom.
* Firepixel: a text inside an open panel menu prefers the entry with the ellipsis / a non-button over a button of
  the panel below (`Neuer Stil…` vs. footer button `Neuer Stil`); `dialog_rect` falls back to the bridge's
  `ui_state.dialog.rect` (K-2). Floating groups without a window command are closed with the `×` of their title
  bar (`ui_map.fp_float_title_h`, default 12).
* Reference `list_row` without UI Automation items (owner-drawn document library): row found in the panel image
  (text bands below the list frame line, library names sorted by name via JSF).

## R-6 additions (17.10.)

* Test files are protected (Q-41): every run (also `--dry-run`) starts only when all test files (config
  `testfiles`, `par_import\*`, `par_swatches.act`) match `TESTFILES.sha256` in the test folder (config
  `testfiles_sha256` to move it); otherwise the run stops with exit 2 and names the files. The checked files are
  copied to `tmp_dir\testfiles_ref`; after each unit (documents closed) a changed test file is kept in
  `tmp_dir\testfiles_changed` and put back from the copy (`TEST FILE CHANGED …` in the log); a file that cannot be
  put back stops the run. `--write-testfile-sums` writes the list - only from checked files.
* Placeholders `{tmp_url}` (file URL) and `{tmp}` (path) of `tmp_dir` in actions; `menu-file/07` saves its copy
  with `fw.saveDocument(doc, '{tmp_url}/menu-file_07_{side}.fw.png')` and records it.
* Foreign foreground (Q-39): the host reports process name, pid, class and title of the window in front (`fg`,
  refused input, every take-back); the run start lists visible `Chrome_WidgetWin_1` windows with their process.
  The caption click of R-5 never fired in run 10 (the covering window failed the `WindowFromPoint` check without
  a trace): now fresh window rect, vertical middle of the caption band at 50/35/65 % width, the covering window is
  reported (`caption covered: …`), the target is raised (`HWND_TOPMOST` without activation, reset right after)
  and checked again; after the click the foreground is checked (`+caption x,y ok|nofocus`, `+nocaption`).
* Canvas shots: the centred span (`canvas_clip_max`) also applies when a fitted/zoomed page is whole but larger
  than the span (`xs-canvas/03,11,13,16`, `toolbox/15`).
* Firepixel menus from the difference box: only the menu columns (painted texts sharing a left edge) and the
  changed region touching them, left edge at the menu frame (`filters-blur-sharpen-noise/02,03`). Firepixel
  `popup` area uses the bridge's `popup.rect` (inspector field lists). Reference `popup`: new windows larger than
  40 % of the main window are left out (`inspector-fields/22`).
* Firepixel panel shots from `panel_rects` start at the first tab of the tab line (TW-84 symbol properties).
* `list_row` by image: library names also from the symbol names of the document's instances and from
  `ui_map.orig_library_names` (per document title).
* Firepixel drags with held keys (TW-97): the bridge holds a `modifiers` event for one frame only, so the keys are
  sent again before every pointer event of the drag (even count of key/move pairs, the keyboard menu stays off).
* Unit start colours in the reference: the log shows `default colours: before … / default / after …` (path
  attributes of the document).
* `log_value` in the reference: script error `-2147216344` (history palette) is recorded as missing, the step
  goes on.
* A fresh reference instance before the script-heavy units `color-popup`, `menu-view`, `panel-misc`,
  `xs-file-formats` (config `orig.restart_before_units`).

## R-7 additions (17.10.)

- **`unsupported` per step:** `"unsupported": {"orig"|"fp"|"both": reason}` - the side cannot reach the target (embedded
  panels of the reference, menus it does not have). Failing actions are skipped (`skipped` in the log), the shot is
  taken anyway, the status is `unsupported` with the reason, so the step does not count as a gap. Measured quote of a
  unit: steps with both sides `ok` / steps without an `unsupported` or `skipped` side.
- **Focus:** the host's `Focus` uses `SwitchToThisWindow` when `SetForegroundWindow` was not honoured and checks that
  the foreground is still the reference 400 ms later (`how` `+stable` / `+lost to <process>`, two more rounds with the
  Alt tap). Every take-back is logged. A step during which the foreground was taken back, or whose pointer tool could
  not be chosen after opening the test file, is played again (`retry_of` in the log, the active tool is logged), up to
  `orig_step_retries` times (config, default 2 since R-11); disturbed again -> `error`. R-11: a take-back in the
  reset before a step (`foreground held by …`) is followed by the `FG_SETTLE` wait and marks the step
  (`fg_taken_before`); an `error` in such a step is retried the same way.
- **Canvas points (R-11):** every canvas point (click, double-click, hover, wheel, `{canvas: …}` targets) is checked
  against the side's document view; a point outside is not sent and the action is `unsupported` with the reason.
  `{view_frac: [fx, fy]}` = that fraction of the page part shown in each side's view (zoom independent). After a
  reference menu command the page is measured again at once (rulers move it). Foreign programs are never minimised or closed (Q-43), they are only reported.
- **Values:** reference drags carry `from_win`/`to_win` (client), `from_scr`/`to_scr`, `from_doc`/`to_doc` and the
  measured `zoom`; every reference step that touches the canvas adds `view_zoom` (JSF candidate properties, '@ n %' in
  a window title, page width / document width).
- **Drag pauses:** `{"drag": {…, "press_ms": n, "release_ms": n}}` - pause after the press with two small moves past
  the drag threshold, pause before the release (reference; list drag-and-drop).
- **Targets:** ui_map `grid_targets` (`panel_swatch`, `color_table_cell[@preview]`, `history_row`, `page_row`,
  `page_name`, `state_row`, `state_delay`; a target `{form: i, "grid": ctx}` picks `form@ctx`); reference locator
  `{"dialog_label": text, "class": cls, "contains": bool}` (control by its Win32 text, or the nearest `cls` right of
  the label); Firepixel `{"find": label, "right_role": role}`; ui targets given as `{area, pos}` also on the reference
  side; a reference combo locator may carry `notify` (codes sent to the parent after the selection, default
  CBN_SELCHANGE; config `orig.combo_notify` for all).
- **Unit start colours:** when `setDefaultBrushAndFillColors` changes nothing, the tools' 'default colours' button is
  clicked with nothing selected and the result is logged.
- **Fixes:** `set_field` in a dialog moves the focus in the largest dialog when the field's root is no dialog any more
  (xs-preferences/02,17); Firepixel panel rect (TW-84) falls back to the group's left frame from the window image;
  host `shot` without a rectangle takes the main window found by its class.

## R-8 additions (17.10.)

- **Verified take-back (run 12: 75 take-backs from `claude`/`Chrome_WidgetWin_1` reported `set+stable -> ok`, the
  next input was refused):** the host's `Focus` ends with a settle check (`orig.focus_settle_ms`, default 450 ms):
  `+settled <ms>` or `+relost to <proc> <class>` with `ok=false`; it also calls `AllowSetForegroundWindow` for the
  target while attached to the foreground thread. The runner's guard reads the foreground again `FG_SETTLE` (0.4 s)
  after every take-back and sends the input only when it is still the reference's (`lost again … to …` otherwise).
  After the foreground was lost again twice - at the first loss for a process in `orig.yield_to` (default
  `["claude"]`) - the guard yields with long pauses (`FG_YIELD` 5/10/20/30/40/60 s) instead of fighting, then
  gives up with the holder and the counts in the error.
- **Restart reasons:** every `restart reference program` line carries its reason (`access violation in script: …`,
  `<n> reference errors in a row (…)`, `planned before script-heavy unit …`); a start before a script because the
  program was not running is logged too.
- **Fit and zoom (Q-48, `xs-canvas/12`):** large documents are fitted in the reference with `orig.fit_menu`
  (default Ansicht > Alles anpassen), then `orig.fit_all_key` and `orig.fit_all_key2` (Ctrl+0); after each the
  page is measured and must be fully visible (`orig fit: …` in the log). Firepixel takes the reference's fitted
  zoom when it is smaller (`fp fit: …`); Firepixel canvas drags record `zoom` like the reference. A reference canvas
  drag with a point outside the document view is an error (`drag point outside the document view …`).
- **Popup edit (Q-46, colour popup hex field):** after `type` into an edit of a program popup the text is read back
  (WM_GETTEXT) with the keyboard focus (`focus_of`); when the typed text is missing it is set with WM_SETTEXT and
  read again; Enter/Tab afterwards is recorded too (`popup_edit` values).
- **Export (TW-102/103):** both sides record `options` (colorMode, numPaletteEntries, number of states) before an
  export; Firepixel records the clicked item and the export options after a `choose` in the format field.
- **Grid click probe (TW-105):** clicks on `panel_swatch`, `popup_swatch`, `color_table_cell` record the colour
  under the point right before the click on both sides (`click_probe`).
- **History replay (TW-106):** `panel-pages-states-history/16` logs the history after the replay (step count,
  selection, steps as script). Every runner script is a history step of its own in the reference.
- **ui_map:** `behaviors.add` on both sides.

## R-9 additions (17.10.)

- **Export protocol (reference):** the probe script before an export was built with nested double quotes
  (`TypeError 'str' | 'str'`, every reference export in run 13 failed) - fixed.
- **Fit all (reference):** `Ansicht > Alles anpassen` fits the page exactly, so the page touches both borders of one
  axis; `calibrate()` now accepts that (other axis with margins, document aspect ratio within 3 %) instead of calling
  it a scrolled page. Up to three measurements per way; the fit log shows page and view. The fitted zoom becomes
  `fit_zoom` / `view_zoom`, the Firepixel side takes it over.
- **Default colours (TW-107):** Firepixel runs `color.defaults` with the unit's first document, like the reference.
- **Tool state per canvas drag (TW-107/109, Q-50):** `state_before` / `state_after` in the drag record: reference by
  JSF (`fw.activeTool`, `pathAttributes` fill/stroke colour and kind), Firepixel from `ui_state` (`tool`,
  `fill_color`, `stroke_color` and, when present, key-state / fill-kind fields).
- **Held keys (TW-109):** the host checks the held keys with `GetAsyncKeyState` before the press (sends them up to
  twice more) and before the release; the drag record has `keys` {before, at_press, before_release, resent}.
- **Keys:** `hold_key` strings like `ctrl+Space` are split (`norm_mods`); plain held keys (Space) are pressed and
  released around pointer events and drags on the Firepixel side; `Apostrophe`/`Quote`, `Semicolon`, `Slash` ...
  mapped for egui and by key position (US names) in the host.
- **Screenshots:** an empty region on the Firepixel side falls back to the whole window instead of an error.

## R-12 additions (18.10.)

- **Swatch grid (TW-115):** Firepixel's swatches panel draws 9 px cells (8 px colour + 1 px line, index 0 top left,
  columns = (client width - 17) / 9). ui_map `grid_targets.panel_swatch.fp` is now `origin [8, 53]`, `cell [9, 9]`
  (measured in run 16, panel-swatches-mixer/01) with the new grid key `cols_width`: columns =
  (shot width of the area - `cols_width`) // cell width (268 px shot -> 27 columns). Click targets, the click probes
  and `swatch_readback` use it; the reference grid is unchanged.
- **Object read-back (TW-124 a):** Firepixel's `log_value` wraps the expression so objects and arrays come back as
  `toSource()` (JSON as fallback) instead of `[object Object]`, like the reference side.
- **Menu reading (TW-124 b):** `log_menu_items` and the menu shot box on the Firepixel side ask `ui_find` for all painted
  texts (`role: text`; context/button menu entries carry a widget name and were dropped before) and keep only the
  topmost paint block (`top_paint_block`, order gap 40): the menu, not the panel texts under it.
- **No `Array.map` (TW-124 c):** the reference's script engine has no `map` on element lists; every `log_value` in the
  step scripts that used `X.map(function(e){...}).join(sep)` is now a plain `for` loop.
- **Drag end outside the view (TW-124 d):** `{"canvas_drag": {..., "allow_outside_end": true}}` - the start point is
  checked against the document view as before, the end point may lie outside it (reference: inside the program window);
  xs-canvas/10 drags a guide onto the ruler.
- **Dialog fallbacks (TW-124 e):** `if_no_dialog` takes `remeasure` and `unsupported_if_none` (table above);
  xs-canvas/07 (pointer tool first, guides logged, neighbouring rows) and export-results/13 (area drawn again slowly)
  use them.
- **Shot regions (TW-124 f):** a region with swapped corners is normalised (`norm_rect`) on both sides instead of
  falling back to the whole window; the Firepixel menu box (`tight_popup`) returns nothing for an inverted result.

## R-13 additions (18.10.)

- **Stored options (TW-127, `xs-preferences/06, 15`):** `{"set_option": {"target", "value": true|false}}` sets a check box
  whose state persists in the program's profile to a known value instead of toggling it: the state is read (reference:
  `BM_GETCHECK` on the Win32 control found by the target's locator, e.g. `dialog_label`; Firepixel: `toggled` of the
  `ui_find` node), the control is clicked only when it differs and read again (up to two clicks, error when it still
  differs). Values: `{"option", "before", "want", "after", "clicks"}`, also in the run log. The state found first is
  remembered per side; `{"restore_option": target}` sets it back (no-op with a log line when it is unknown).
  06 sets 'delete objects when cropping' on and restores the start state in `after`; 15 sets 'scale attributes' on and
  logs the stroke before and after.
- **`retry_if` (TW-129):** `{"retry_if": {"check": JSF expr, "retry_when": [values] | "retry_when_falsy": true, "do":
  [actions], "why", "remeasure"}}` reads the expression on the side; when the value is one of `retry_when` (as text) or
  means nothing (`""`, `0`, `false`, `null`, `undefined`), `do` runs once (`remeasure`: the reference's page is measured
  again first) and the value is read again. Logged as `retry_if` {value, retried, value_after}; it never fails the step.
- **`menu_state` (TW-129 e):** `{"menu_state": "A > B"}` opens the menu up to the parent, reads the last entry's
  enabled/checked state (reference: popup menu item state; Firepixel: `ui_find` node) and closes the menu again without
  executing anything. Values `{"menu_state": path, "value": {text, enabled, checked}}`.
- **Zoom log vs. dialogs (TW-129 a):** the per-step reference zoom log (`view_zoom`, R-7) runs a script, and the script
  hand-over cancels open dialogs (`origctl.jsf`). In run 17 this closed Feather / Save Bitmap Selection between the
  actions and the dialog shot (`dialogs-d31-d40/09, 14, 15, 16`: 'no dialog open'). With a dialog open the zoom log is
  now skipped (`view_zoom: {skipped: …}`). Scripts must not run `log_value`/`jsf` while the dialog to be shot is open:
  `dialogs-d31-d40/15` reads history and document after the shot.
- **Marquee steps (`dialogs-d31-d40/09, 10, 14, 15, 16, 18_d40`):** after the marquee drag a `retry_if` logs the
  document's `isSelectionEmpty` (both sides) and drags once more when it is `true`; on the reference side an
  `if_no_dialog` (3 s) draws the marquee and invokes the entry again, then `unsupported_if_none`. 10 and 18_d40 had no
  marquee at all (entry disabled in the reference) and now draw one first.
- **File dialog wait (TW-129 b):** the reference's `file_dialog` waits up to `wait_ms` (default 6000) for the dialog;
  `panel-swatches-mixer/06` (both variants) also opens it once more through the panel menu when none appeared within 6 s.
- **Library drop (TW-129 c, `panel-library-styles/03`):** the panel is shown again right before the drag (its rectangle
  is measured fresh) and a `retry_if` on the element count drags once more when it is still `4|3`.
- **Nudge (TW-129 d, `xs-shortcuts/10_nudge_result`):** R1 is selected again (and the selection checked) before every arrow
  key; Escape after each nudge cleared it. R1's position is read by name together with the selection count.
- **Snap to pixel (TW-129 e, `xs-canvas/15`):** `15_snap_pixels_result` selects R1 of T04 first (the entry is disabled
  without a selection; T10 has no vector object); the new `15_snap_pixels_disabled` reads the entry's state with nothing
  selected on both sides (`menu_state`, no `unsupported`).
- **Shortcut sets (TW-126):** the set written into Firepixel's profile comes from `fp.shortcut_set` (config, default
  "Firepixel"). Once per run the log says which set each side uses: `shortcut sets: fp '<set>' (fp.shortcut_set), orig …`
  - reference: `fw.getPref(orig.shortcut_set_pref)` (default key `CurrentShortcutSet`) when it returns a value, else
  `unknown (…)`; a dry run does not read it.
