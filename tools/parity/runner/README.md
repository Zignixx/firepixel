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
Firepixel's runner profile, default "Web Standard" since R-13b), `orig.shortcut_set_pref` (R-13: reference preference holding
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
  "Web Standard" since R-13b). Once per run the log says which set each side uses: `shortcut sets: fp '<set>' (fp.shortcut_set), orig …`
  - reference: `fw.getPref(orig.shortcut_set_pref)` (default key `CurrentShortcutSet`) when it returns a value, else
  `unknown (…)`; a dry run does not read it.

## R-13b additions (18.10.)

- **Shortcut set (RE-14):** the reference machine uses the bundled "Web Standard" set (pref `CurrentShortcutSet`,
  key confirmed; its user set folder is empty). `fp.shortcut_set` now defaults to "Web Standard" (fpctl and runner).
- **Key scan codes:** the host sends every virtual key (`Combo`, held modifiers) with its scan code
  (`MapVirtualKey`, Escape = 0x01). With scan code 0 an Escape after typed text left a stray "c" in the reference's
  text (`xs-shortcuts/15` "Vrc", `tool-text/07` "Hallo Weltc"); both steps log the texts before Escape now.
- **`xs-shortcuts/15_in_text_result`** is `unsupported` on both sides (TX1 of T05 has zero width and an invalid
  pixelRect in the reference file; T05 needs a rebuild); `15_in_text_tx2_result` does the same on TX2.

## R-14 additions (18.10.)

- **Dialogs and read-back scripts (RE-15, TW-132):** the script hand-over (`origctl.Original.jsf`) cancels open dialogs
  every 0.4 s while a script runs (a modal dialog would block it). Notices the reference shows after a shortcut (Break
  Apart / Select Inverse warning) were therefore closed before `record_ui` saw them; a `log_value` of ~845 ms was the
  only trace. Now:
  - before the reference's `log_value` script, open dialogs are recorded: values `{"dialog_before_log_value": [{title,
    rect, texts, controls}], "shot": path}`, the largest one grabbed to `<unit>/orig/dialogs/<step>_<n>.png` (not part
    of the compared shots, cleared at unit start), log line `orig: dialog open before log_value: <title> | <texts>`;
  - every dialog a script cancels is logged as `dialog closed by <log_value|key check|jsf|jsf_value>: <title> | <texts>`
    and listed in the step record (`dialogs_closed_by_scripts`);
  - step flag `"keep_dialog": true`: the scripts of that step leave open dialogs alone (logged once as `dialog kept
    open during …`). A modal dialog blocks the script until it is closed, so a script then ends with a JSF timeout -
    only for steps whose scripts run while a non-modal window is to be kept.
- **History read-back (TW-132 a, `menu-edit/01_undo_redo_result, 22`):** `getStepCount()` stood outside the try and gave
  script error -2147216344. Every history call is guarded; the value is `[count, steps, {count_err, sel | sel_err,
  docs}]`. When the error comes back, the reference side shows the History panel and reads once more (values `first`,
  `retry: "history panel shown"`, log line).
- **Checked keys (TW-132 b, c; `menu-edit/09, 20`, `xs-shortcuts/10`):** on the reference side Delete/Backspace,
  Ctrl+D and the plain arrow keys are checked when the pointer tool is active, a document is open and no dialog is:
  tool, object count, `fw.selection.length` and the selection rectangles are read before and after the key, with the
  focused window. Nothing selected before Delete/arrow: the step's last `select` is restored first. No effect (no
  object removed / selection not cleared / selection not moved): click on an empty canvas spot outside the page (gives
  the view the focus), selection restored (`select` of the step, or select all when everything was selected), key
  pressed once more. Values `{"key_check": {key, kind, focus, before, after, effect, focus_click, restored,
  after_retry, effect_retry}}` and a log line `orig key <combo>: …`. Other tools (text editing, subselection) are not
  checked.
- **Text tool state (TW-132 d):** at unit start the reference gets Escape twice (host key with scan code) before the
  pointer tool is chosen with the first document; the log line `unit start text reset (Escape x2, pointer tool):
  active tool …` follows. The script API has no setter for the text tool's defaults (font/size/style setters act on
  selected text only), so these are not reset by script. After every step with a `type` action the reference gets
  Escape twice and the pointer tool when the text tool is still active; `text_cleanup` {before, after: {tool, focus},
  pointer_selected, editor_closed} is in the step record and the log.
- **Firepixel menu state (TW-132 e, `xs-canvas/15_snap_pixels_disabled`):** when the entry's best `ui_find` hit is a
  painted text without state, the runner looks for an accessibility node over the text (`menu_item`, `button`, any
  role) and takes its `enabled`/`toggled` (`enabled_from`); without one the value carries `enabled_why` (what the
  bridge returned) and the log says so.
- **Multi-line read-back (TW-130):** `log_value` on the reference side kept only the first line of a value (the script
  log is read line by line; a text's `textChars` holds line breaks - TX2 showed only "Zeile eins"). The lines after
  the `RET` line now belong to the value (joined with `\n`).
- **Text read-back (TW-130):** `xs-shortcuts/15_in_text_tx2_result`, `tool-text/06_edit_existing_focus`,
  `07_select_text_shift_arrows_focus` and `07_select_text_double_click_focus` log every text object (layer/index, name,
  pixelRect, `textChars`, selection count) before typing, after typing and after Escape on both sides (06 and 07 shift
  arrows: Escape and the last read-back in `after`, the shot is unchanged).

## R-15 additions (18.10.)

- **Reference keys (TW-140, run 19):** Delete selected the Eraser, the plain Right arrow the Text tool (both dropping
  the selection), Escape left a "c" in an open editor or the Crop tool active. The extended-key flags and the scan
  codes were already right (R-13b); every failed key had the keyboard focus on the main frame (a click on the tools
  column puts it there). The pattern fits key-name matching: the reference names a plain key with the window
  system's key name of its scan code in the reference's keyboard layout and falls back to the tool letter of the
  name's last letter when no shortcut matches ("Delete" -> e = Eraser, "Right" -> t = Text, "Esc" -> c = Crop - the
  English names; the names in the German layout are "ENTF", "NACH-RECHTS", "ESC"). `key_echo` logs the names the reference
  gets, so the next run confirms or refutes this. Changes:
  - host: scan codes from the **target thread's** keyboard layout (`MapVirtualKeyEx`; letters differ between layouts,
    e.g. Y/Z), extended flag from an explicit table (Insert, Delete, Home, End, PgUp, PgDn, arrows, Win, Apps,
    NumPad /, NumLock, right Ctrl/Alt), Escape always scan 0x01 without the flag;
  - host verb `layout {want}`: the reference's layout (`GetKeyboardLayout` of its UI thread) and, when its language
    differs from `want` (KLID), a `WM_INPUTLANGCHANGEREQUEST` to its main window (only that window's input language,
    like the user switching it). Config `orig.key_layout` (default `"00000407"`, German; `""` = never change); the
    runner requests it at every unit start (a restarted instance starts with the default layout again), log line
    `key layout: target <before> -> <after> …` when something was requested;
  - host verb `key_echo {keys}`: once per run (first unit) a low-level keyboard hook records the keys as the system
    delivers them and **swallows** them (nothing reaches any program): sent vk/scan/extended vs arrived, key name
    and character in the reference's layout, the focused window class, double-click time and rectangle. Logged
    before and (when a layout change was requested) after the layout request: `key echo (before layout request;
    target layout 0x…, host 0x…, focus …, double-click time … ms): ok|MISMATCH - escape 0x1b/0x01 -> … name 'Esc' …`.
    A real text field of the reference is not used: every field there commits into the document or a preference;
  - host verb `view_focus`: keyboard focus to the document view (`SetFocus` with attached input, no click). The runner
    uses it before every plain key (no Ctrl/Alt) and before the cleanup Escapes when the frame itself holds the
    focus (an open editor, a panel field or a dialog keep theirs); a failure is logged.
- **Key check (TW-140):** after a checked key the pointer tool must still be active; otherwise the step record gets
  `runner_errors: ["tool switched by key: <tool> (key <combo>)"]`, the log `runner error: tool switched by key: …`, the
  key check `tool_switched`/`tool_switched_retry`, and the pointer tool is chosen again. Such a key never counts as
  an effect; an arrow key counts only when the same number of objects stays selected with other rectangles (a dropped
  selection is no move). The log line shows the tool after the key.
- **Double-click (TW-140, `tool-text/06`):** the host sends all clicks of a double-click in one `SendInput` batch.
  After a pointer double-click on the canvas the runner reads `fw.activeTool` and whether one text is selected;
  a selected text with the pointer still active means no editor: the double-click is sent once more, values
  `{"double_click_retry": {at, before, after, after_retry, editor}}`, a log line, and a runner error when the
  editor stays closed.
- **Text attributes across units (TW-140, `tool-text/02, 14, 16`):** with the unit's first document the reference
  creates a scratch document (`fw.createDocument`), a text in it, sets its attributes to config `orig.text_defaults`
  (default `{"face": "ArialMT", "size": "12pt", "bold": false, "italic": false, "underline": false, "fillColor":
  "#000000"}`; `setTextRuns` acts like an edit in the text editor, the last used attributes become the tool's),
  deletes it, creates one more text from the tool's defaults (`addNewText(.., true)`), logs its attributes and closes
  the scratch document without saving (the test document gets no history step). Log line `unit start text defaults
  (<s> s, wanted …): before … | after …`.
- **Firepixel menu enabled state (TW-141) - needs a UI change:** `ui_find` reports `enabled`/`toggled` only for
  AccessKit nodes. Menu rows are painted by `crates/fp-ui/src/menu.rs::paint_item` with `ui.allocate_response(..)`
  and no `widget_info`, so they produce no labelled AccessKit node (role `unknown`, empty label - dropped by
  `bridge/inject.rs::find_hits`); the painted text has no state. Run 19 (`xs-canvas/15_snap_pixels_disabled`):
  `menu_item: 0 hits; button: 0 hits; any: 5 hits (button accesskit, text)` - none over the row. Missing: an
  `enabled` (and `toggled`) field on the `ui_find` hit of a menu row, i.e. `resp.widget_info(|| ..)` in `paint_item`
  (`WidgetInfo::selected(WidgetType::Checkbox, enabled, checked, label)` for checkable rows, `WidgetInfo::labeled(
  WidgetType::Button, enabled, label)` otherwise, label without the `&` marker, as `dialogs/popup_menu.rs` does), and
  a node also for disabled rows (they use `Sense::hover()`). The runner already looks for such a node (`role:
  menu_item`/`button`, `enabled_from`); no runner change is needed once it exists. Panel menus
  (`panel-library-styles/16`, `log_menu_items`) are drawn by the same function.

## R-17 additions (18.10.)

- **Layout and tool lines (TW-140, run 20 had neither):** the `key layout` line was written only after a layout
  request, and the tool-switch check sat behind `key_effect(..) and not tool_switch_check(..)` - a key without
  effect (Delete -> Eraser, `effect False`) never reached it. Now:
  - every unit start logs `key layout: requested <KLID> -> active <hkl> (change sent: <hkl> | not needed; main thread
    <before> -> <after>; focus <class> <before> -> <after>; host <hkl>)`. Host verb `layout` requests the language
    on the main window **and** the window holding its keyboard focus (`WM_INPUTLANGCHANGEREQUEST` to both) when
    either differs; `KeyHkl` (scan codes) follows the focus thread's layout;
  - after every plain reference-side key (no Ctrl/Alt): `tool after key: <tool> (key <combo>)` (`fw.activeTool`; not
    read while a dialog or a menu is open: `not read (dialog open)`); checked keys log it from their own read-back,
    and `tool_switch_check` runs on every checked key (`runner error: tool switched by key: …`);
  - before every plain key: `key probe <combo>: foreground <class>, focus <class>, layout <hkl>` (host verb
    `key_probe {want}`: foreground window, `GetGUIThreadInfo` focus, `GetKeyboardLayout` of the focus thread); a
    language other than `orig.key_layout` is requested on the focused window and read again (`… -> <hkl> (ok|NOT
    CHANGED)`);
  - the `key_echo` self test (fixed in R-16) is repeated at the next unit when no echo came back (3 units at most);
    the second echo runs only when the layout request sent a change. `selftest_host.ps1` also runs `key_probe` and
    `layout` without `want` (nothing is posted).
- **Text tool attributes (TW-140):** R-15's scratch-document reset (`setTextRuns`) did not reach the tool's defaults
  (36 pt bold italic underlined before and after in all seven units of run 20). Replaced by the UI path, with the
  unit's first document (the tools column anchor needs a document view): nothing selected, text tool from the tools
  column, Properties panel `inspector.size` = size of `orig.text_defaults` (double-click, Ctrl+A, type, Enter),
  `inspector.bold` / `italic` / `underline` clicked when on, text colour through `inspector.text_color` and the
  popup's hex field (`popup.hex_field`, Enter; a second Enter when the popup stays open), pointer tool back. Read
  before and after with a scratch text from the tool's defaults (`addNewText(.., true)`, attributes, deleted); the
  test document is then opened afresh (no history from the reset). Log line `unit start text tool reset (Properties
  panel, <s> s, wanted …): before … | <actions>[; errors: …] | after …`.
- **Crop (TW-134, `xs-preferences/06`):** run 20's reference did not crop (page stayed 800 × 600). New step action
  `crop {from, to}` on both sides: crop tool, canvas drag, Enter. Reference: tool from the tools column, drag, Enter
  with the keyboard focus on the document view (`view_focus` when the frame holds it; `key_probe` logged); page size
  `getDocumentDOM().width/height` before and after; unchanged -> once more with the view focus forced. Firepixel:
  page size from `get_document`. Value `{"crop": {before, after, tries}}`, log `orig crop [..] -> [..]: page WxH ->
  WxH; try 1: {tool, after, …}` / `fp crop …`. `xs-preferences/06` uses it.
- **Menu entries (TW-142):** the reference host strips a trailing "..." on purpose (`Clean`, for path matching), so
  its read-back lacked the ellipsis Firepixel's kept. Recorded menu entries (`menu_items`, `record_ui menu.open`,
  `log_menu_items`) now carry `text` without a trailing ellipsis and `ellipsis: true|false` on both sides
  (reference: from the raw menu text before the tab) - the texts compare equal, the ellipsis is compared separately.
- **Text read-backs keep CR / LF / U+0003 (R-17b, TW-144):** the reference's `textChars` separates paragraphs with
  `\r` (U+0003 = forced line break), but `jsf_value` split the script output with `splitlines()` and joined it with
  `\n`, so every logged break was `\n` (runner artefact). The reference script now escapes the value into one log
  line (`RETE <value>`: backslash doubled, then `\r`, `\n`, `\x03` as text; `text_escape`/`text_unescape`, JSF
  `JSF_TEXT_ESCAPE`) and the runner decodes it, so `value` holds the exact characters (old multi-line `RET` read as
  fallback). Firepixel's `run_script` result already arrives as JSON. On both sides a `log_value` text containing
  one of these characters also gets `value_escaped` (same escaping) and a run-log line `<side> log_value text:
  <escaped>` - e.g. the `textChars` read-backs of `tool-text/06` and `xs-shortcuts/15_in_text_tx2`.

## R-18 additions (18.10.)

- **Firepixel menu entry state (TW-141/TW-142):** since F40 Firepixel's menu and panel pop-up rows have accessibility
  nodes with `enabled`/`toggled`, but R-12's `menu_items()` read only the painted texts (`role: text`, topmost paint
  block), which carry no state - hence `enabled: None` in run 20 (`panel-library-styles/16`, `xs-canvas/15`).
  `menu_entries_from_hits(texts, nodes)` keeps the painted texts for the geometry (columns, shortcuts; `tight_popup`
  and clicks unchanged) and takes each entry's `enabled` / `toggled` (also as `checked`) from the `ui_find` hit of
  role `menu_item`, `button` or `check_box` in the menu region whose label matches on the entry's row (exact, label
  plus shortcut, or an unlabelled node around the text), inside the topmost popup block (paint order not below the
  block's lowest text order minus 40); `state_from: "<role> node"`. Entries without a node keep the text's state and
  a log line counts them. `--dry-run` prints `menu state self check: ok` (mocked `ui_find`: a pop-up over a panel,
  button / check_box / unlabelled / disabled nodes, a panel button outside the block).

## R-19 additions (18.10.)

- **Plain keys on the reference side (TW-140, run 21):** the reference application treats unmodified Delete/arrow/Escape
  keys sent by the runner as tool shortcuts (Delete -> Eraser, Right -> Text, Down -> Line, Escape -> Crop / Export
  Area, and a "c" typed into an open text editor), although virtual key, scan code, extended flag, keyboard layout and
  focus are right (`key echo`, `key layout`, `key probe` of run 21). Keys with Shift or Ctrl work (Shift+arrows move
  by 10 px, Ctrl+D deselects). On the reference side these keys are therefore replaced; **Firepixel keeps the real
  keys**. `key_substitute` decides per key (pure function `key_substitute_plan`, checked offline by `--dry-run`:
  `key substitute self check: ok`):

  | key (no modifier) | situation | reference side |
  |---|---|---|
  | Delete / Backspace | document open, no dialog / menu / new popup window, text tool not active | Edit ▸ Clear through the menu (`ui_map.json → commands → edit.clear`; keeps the History rows comparable); when the menu fails or removes nothing: script `deleteSelection(false)` |
  | Left / Right / Up / Down | same | script `moveSelectionBy({x: ±1, y: ±1}, false, false)` |
  | Escape | text tool active (editor open) | click on the Pointer tool in the tools column, verified: pointer active and the keyboard focus not in an edit control (one more click otherwise; runner error when the editor stays open) |
  | Escape | pointer tool, nothing open | nothing (`none`; the real key would select a tool) |
  | Escape | another tool active | Pointer tool click |
  | any of them | dialog, popup menu or a window that appeared during the step (colour popup, slider) open; no document; text editor open (Delete / Backspace / arrows edit the text there) | the real key, as before (it works in menus and dialogs) |

  Nothing selected before Delete / an arrow: `none (nothing selected)`. Every substitute is logged as
  `orig key substitute: <key> -> <method> (…)`, recorded as value `{"key_substitute": {key, method, before, after,
  effect, …}}` and marked in the step record: the action entry gets `arg` and `substituted: "<method>"`, the record a
  list `substituted: [{verb, arg, method}]`. `key_check` (R-14) does not run for a replaced key. Config
  `orig.key_substitutes: false` sends the real keys again. The windows open at step start are taken as permanent
  (`step_windows`), so only windows that appear during the step keep a key real.
- **Text editor ends without Escape:** the cleanup after a step with `type` (R-14) and the unit start no longer press
  Escape twice; when the text tool is active, the Pointer tool click ends the edit (`text_cleanup` / `unit start:
  orig key substitute: escape -> Pointer tool click`).
- **Text selection by mouse (`tool-text/07`, `11`, `13`):** new verb `text_select_drag {text, chars, start_frac?,
  line_frac?}` (reference side): the text object whose `textChars` contains `text` (else the selected one) is read
  (`pixelRect`, length), then a mouse drag runs from 1 px left of its right edge to the estimated start of the last
  `chars` characters (share of the characters; `start_frac` overrides) at the middle of the line. Read-back: the
  clipboard is emptied, Ctrl+C sent, its text read; value `{"text_select": {rect, length, drag, selected, expected,
  ok}}`, log line `orig key substitute: shift+left xN -> mouse drag over the characters (…); selection read-back:
  '<text>' (expected '<text>', ok|DIFFERENT)`. Shift+Left moved the reference's caret without selecting.
- **Per-side actions:** `{"per_side": {"orig": [actions], "fp": [actions]}}` runs different actions on each side at
  that point of the step (the scripts above: `text_select_drag` on the reference side, Shift+Left in Firepixel).
- **Crop commit (`xs-preferences/06`, TW-134):** `crop {from, to}` commits on the reference side with a double-click
  in the middle of the crop box instead of Enter; page size before and after; unchanged -> one retry (a second
  double-click while the crop tool is still active, else crop tool, drag and double-click again), then a runner error.
  Value `crop.commit`, log `orig key substitute: enter -> double-click inside the crop box [x, y]`.
- **Text colour in the unit-start reset (R-17):** the colour is set by a click on the swatch of that colour in the open
  colour popup; without one, by the hex field (click for the focus, `WM_SETTEXT`, Ctrl+A, typed, Enter). When the
  read-back still differs, a scratch text from the tool's defaults gets the colour by script (`setFillColor`, then
  deleted) and the attributes are read again; a remaining difference is listed under `errors` in the
  `unit start text tool reset` line.

## R-20 additions (19.10.)

- **Text entry on the reference side (TW-146, run 22 `tool-text/02, 03, 06, 11, 13, 14`):** texts created by a
  text-tool click stayed empty and were discarded at the commit, the letters reached the tool shortcuts ("A" =
  Subselection, the tool after the typing read Subselection or Crop) or the first letter was lost ("bc 123 Abc 123 …"
  in `02`). Run 21 and `07` of run 22 (a read-back script between the click and the typing) typed correctly: the
  keys came before the editor was open. With the text tool active, `type` on the reference side now runs
  `text_entry` (other `type` actions - fields, colour popup - are unchanged):
  - **editor wait:** `EDITOR_STATE_JS` (active tool, object count, selection count and the selected text's
    `textChars`) is polled every 0.2 s up to `orig.text_editor_wait_s` (default 3 s) until the text tool is active
    and exactly one text is selected (the editor's object exists while still empty); host verb `caret`
    (`GetGUIThreadInfo` of the main window's thread: caret window class, blinking flag, focus class) is recorded
    with it. No editor -> the step's last canvas click / drag (`last_canvas`) once more, wait again; still none ->
    nothing is typed (`runner error: text entry: no text editor open … not typed (the letters would select tools)`);
  - **typing:** host `type` with `mode: "keys"`: every character as its real key - virtual key and shift state of
    the target's layout (`VkKeyScanEx` with the layout used for the scan codes), scan code from `MapVirtualKeyEx`
    (`Key`), Shift/Ctrl/Alt as needed; Unicode only for a character without a key; LF is sent as Enter;
    `orig.type_delay_ms` (default 20) between the characters. The host returns `{keys, unicode, delay_ms}`;
  - **read-back:** `textChars` of the selected text 0.3 s later (`text_entry_ok`: a new text holds exactly the typed
    text, an existing one grew by its length and contains it). Nothing arrived -> typed once more; a new text with
    other characters -> Ctrl+A in the editor and typed once more; an existing text is not retried. Value
    `text_entry` {text, editor, retry_click?, first?, before, after, ok, how}, log line
    `orig text entry: <how> -> "<escaped textChars>"` (`text_escape`: `\r`, `\n`, `\x03` as text), runner error
    when it differs;
  - **commit:** the Pointer tool click that ends the edit (Escape substitute `leave_text_editor`, or `text_cleanup`
    after the step) reads the object count: a new text must have raised it above the count before the text was
    created (`commit` {elements_before, elements_after, new_text, grew} in the substitute / cleanup record, log
    `orig text commit: elements a -> b (ok|TEXT DISCARDED)`, runner error when discarded).
  - Config `orig.text_entry_check: false` types as before (Unicode, no wait).
- **Drag selection read-back (TW-146, `tool-text/07, 11, 13`):** run 22's `text_select_drag` read an empty clipboard
  and its Ctrl+C typed U+0003 into the text ("Hallo \x03Welt"). The script API has no read-back of the editor's
  selection (re\46-50 list none; `textRuns`/`setTextRuns` act on the whole text and would change it), so no key is
  sent into the editor any more: the text's box (2 px margin) is grabbed before and after the drag and
  `selection_highlight` counts the changed pixels - they must cover at least half of the span of the characters
  to be selected and be more than a caret line. None -> a click at the text's end and the drag once more (40
  steps), then a runner error. Value `text_select` {rect, length, drag, expected, highlight, highlight_2?, tries,
  screen_box, readback, ok}; log `… selection read-back (highlight, no clipboard): <n> px changed in [box], cover
  <c> of the '<text>' span (ok|NO SELECTION)`.
- **Text fields of the Properties panel (TW-146, `tool-text/11, 13`):** `inspector.size`, `size_slider`, `font`,
  `font_style` and `text_color` (`TEXT_FIELDS`) are clicked / set on the reference side only with the text tool
  active or a text selected; otherwise the action is skipped with a runner error (run 22: with no text the size
  field position held the document's Image Size button). Config `orig.text_field_guard: false` switches it off.
- **Pointer slot (TW-147 a, `xs-shortcuts/07`):** after the F-key panel toggles the Pointer slot of the reference's
  tools column held Select Behind ("Hintere auswählen"). Every Pointer selection by the slot (`tool.pointer`, the
  Escape substitutes, `text_cleanup`, the unit start) reads `fw.activeTool` afterwards; not the Pointer -> the
  slot's flyout entry (`toolbox.pointer_flyout`, entry `orig.pointer_flyout_index`, default 0), then the script
  `fw.activeTool = <orig.pointer_tool_name>` (default "Zeiger"); log `orig pointer tool: slot click gave … ;
  flyout entry -> …`, runner error when another tool stays active. Keyboard V is not used (plain keys select tools).
- **Escape with a selection (TW-147 b, `xs-shortcuts/21`):** with the Pointer tool and a selection the R-19 `none`
  substitute kept R1 selected where Firepixel's Escape deselects. New config `orig.escape_deselects` (default
  **false** until RE-18 shows whether the reference deselects on Escape): false -> nothing is done, method `kept
  (selection stays; orig.escape_deselects false)`, the step record gets `escape_not_comparable: true`; true -> the
  selection is cleared by `orig.escape_deselect_method`: `"menu"` (default, `commands → select.none`, script
  `selectNone()` as fallback) or `"real_key"` (the real Escape, tool switch checked). The decision is logged once
  per run: `orig escape with a selection: orig.escape_deselects false -> …`. Without a selection Escape stays `none`.
- **Nudges and History rows (TW-147 c, `panel-pages-states-history/16, 21`):** every `moveSelectionBy` substitute is a
  History row of its own in the reference. Step flag `"orig_nudge_keys": "shift"` (used by `21`, only the step count
  is compared): the reference side sends a real Shift+arrow (10 px) per plain arrow, method `real Shift+Right (10 px,
  step orig_nudge_keys shift)`. Otherwise the script nudge stays and the step record gets `history_not_comparable:
  true` (set automatically by any script nudge; `16` also sets the flag in the script) - the evaluation skips the
  History row count of such steps.
- **Paste read-back (TW-147 d, `menu-edit/03`):** after Ctrl+V the script waits 300 ms and a `retry_if` on
  `fw.selection.length` waits another 300 ms and reads again when it is empty (both sides).
- **History limit (TW-147 e, `panel-pages-states-history/22`):** reads `fw.getPref('MaxUndos')` (`UndoSteps` does not
  exist).
- `--dry-run` prints `text entry self check: ok` (editor read-back parser, `text_entry_ok`, `selection_highlight`
  with a synthetic highlight, a caret line and an unchanged box); `selftest_host.ps1` checks the typed keys of
  "Ab 1" (scan code not 0, Shift only for "A").

## R-21 additions (19.10.)

- **History rows (TW-148, `panel-pages-states-history/16, 21`):** new action `{"history_rows": true}` (both sides),
  in `16` right after the nudges (Shift+Right on E1), in `21` after the step count. Reference: `HISTORY_ROWS_JS`
  reads every row alone (`historyPalette.getStepsAsJavaScript([i])`, line breaks as blanks; -2147216344 -> History
  panel shown, read again); a runner script row (`fw.runScript(".../par_<time>_<n>.jsf")`) starts with the
  script's name, followed by the runner's caller and the start of the script - origctl `jsf()` keeps `script_whys`
  (name -> `why: code start`) and `select` passes `why="select [names]"`. Firepixel: bridge `history` (label and
  command per step). Value `history_rows` {count, rows, (commands, position), source}, log
  `orig history rows: [..]` / `fp history rows: [..]`.
- **Caret check before typing (TW-146 a, b; run 23 `tool-text/06, 09, 13, 20`):** an open editor (one selected text)
  is not enough any more. `caret_check` up to `orig.caret_wait_s` (default 1.5 s): host `caret` with a caret
  window that blinks = ok; else `orig.caret_shots` (default 3) shots of the text box (selected text's pixelRect
  plus the last canvas point, 4 px margin) `orig.caret_interval_ms` (default 300) apart - a thin vertical change
  between two shots (at most 4 px wide, at least 5 px high, `caret_blink`) is the blinking caret. No caret -> one
  canvas click into the text (the step's last click / double-click point, the middle of a text-tool drag; none
  after keys in the editor, the caret would move), editor and caret checked again; still none -> the first
  character alone, read back; the rest is typed only when it arrived, else runner error `no caret and the first
  character … did not arrive` and nothing more is typed (no "typed once more" without a caret either). Values
  `caret_check`, `caret_click`, `caret_check_2`, `probe` in `text_entry`; the log line `orig text entry:` names
  each step (`caret (…)`, `no caret (…) - click into the text at …`, `first character arrived, rest typed …`).
  The double-click "Q" of `06` goes the same way (caret wait, click at the double-click point with the text tool).
  Config `orig.caret_check: false` = old behaviour.
- **Size on a drag selection (TW-146 c, TW-145, `tool-text/11`):** `set_field inspector.size` after a
  `text_select_drag` of the same step: the text box is grabbed again and compared with the shot before the drag
  (`selection_highlight`; lost -> the drag once more), the size field clicked once, Ctrl+A, the size typed with
  Enter, the selected text's runs read back (`TEXT_RUN_SIZES_JS`, [[characters, size]]). The tail (`expected`, e.g.
  "123") not at the size -> selection checked again and `orig.text_size_menu` (default `Text > Größe > {size}`)
  invoked, read back once more. Value `text_size` {selection_before, runs_before, runs_field, runs_menu?, ok, how},
  log `orig text size on the selection: …`, runner error when the size is not on the characters. Config
  `orig.text_size_on_selection: false` = the plain field edit.
- **Firepixel text tool reset (TW-149):** with the unit's first document Firepixel's text tool gets the reference's
  reset attributes (`TEXT_DEFAULTS` + `orig.text_defaults` + `fp.text_defaults`: ArialMT 12 regular #000000) by the
  bridge command `text.setToolDefaults` {font, size, bold, italic, underline, color} with nothing selected; read
  before and after from a scratch text (`addNewText(r, true)`, deleted), then the document is opened afresh. Value
  `text_tool_reset`, log `fp unit start text tool reset (text.setToolDefaults, …): before … | params … | after …`.
  Config `fp.text_tool_reset: false` switches it off.
- **Step time limit (TW-150 a, run 23 `xs-shortcuts/08` 94.1 s with 26 substitutes):** limit = `step_timeout_s`
  (90) + `step_timeout_per_substitute_s` (default 2 s) x the step's reference-side substitutes
  (`step_time_limit`); the record gets `time_limit_s` when it is raised, the `slow` error names the substitutes.
- **Foreign-window log (TW-150 b):** window titles go through UTF-8 with `errors="replace"`, and `Runner.log`
  writes a line its console code page cannot hold with "?" instead of failing (`'charmap' codec can't encode
  character '​'`); no foreign window logs `foreign windows at run start: none`.
- `--dry-run` prints `R-21 self check: ok` (row labels, run sizes, caret blink with a caret line / no change / a
  wide change, step limit).

## R-22 additions (19.10.)

- **Reference text tool reset without fill side effect (TW-153, run 24: shapes drawn after the reset had fill
  #000000 in eight units, `tool-autoshapes` 20 of 27 canvas pairs above 1 %; 23 pt kept in eight units):** the text
  attributes are still set only with the text tool active through the Properties panel (size field, faux style
  buttons, the text colour control); selecting the text tool hands its colour to the fill swatch, so afterwards
  `default_colors_restore` clicks the tools' "default colours" button (Pointer tool, nothing selected) and reads
  `pathAttributes` back (want `orig.default_fill` #ffffff / `orig.default_stroke` #000000, one retry; log
  `default colours (after the text tool reset): ...`). The size is read back: not 12 -> once more with the text tool,
  first a different size (13), then 12 (the field already showed 12, so typing 12 changed nothing). Fill / stroke,
  text colour and size after the reset are checked and named in the `unit start text tool reset` line. The colour
  reset runs once more with the reopened document.
- **Steps without a key in the "Web Standard" set (TW-154):** `menu-text/05_editor_open` and
  `menu-view/20_edit_guides_dialog_open` - neither program has a menu entry for the Text Editor or Edit Guides (re 46,
  re 11), so there is no menu path: Firepixel opens the dialogs by its commands (`text.editor`,
  `view.guides.editGuides`), the reference side is `unsupported` with the reason (shot only).
- **Arrow substitutes only for nudges (TW-155 a):** `key_substitute_plan(..., nudge)`: an arrow becomes
  `moveSelectionBy` only with the Pointer or Subselection tool (`NUDGE_TOOLS`) or with `nudge: true` (step or key
  action); `nudge: false` = always the real key. With another tool the arrow is `tool_key`: the real key, the tool
  read back; a tool switch (the key taken as a tool shortcut, not consumed) -> Pointer tool back and the step
  `unsupported` with the reason; no switch -> value `tool_key`. `tool-path-reshape/04` carries `nudge: false`.
- **No plain arrows in `inspector-stroke/01_category_submenu_open` (TW-155 b):** the open category list's
  "Airbrush" entry is hovered (`hover {menu_item}`, wait 600 ms), which opens its submenu, on both sides.
- **History read-back with one document (TW-155 c):** `menu-edit/01_undo_redo_result`, `09_clear_result`,
  `22_undo_names_result` start with `close_all` before the reopen.
- **Screen mode start (TW-156):** new action `{"screen_mode_standard": true}` (both sides), at the start of the three
  `toolbox/13` steps and in their `after`. Reference: the tools' "standard screen mode" button (F only when the
  button cannot be reached, read back after each press, at most twice); read back as the main window's caption
  (`WS_CAPTION`, `screen_mode_state`). Firepixel: `ui_state.screen_mode`; not 0 -> the button, else
  `view.exitFullScreen`. Value `screen_mode`, log `orig/fp screen mode standard: ...`, runner error when not standard.
- **Reference text entry retries (TW-146 rest, run 24 `tool-text/11` only "Ab", `12, 13, 17` nothing although the
  caret check passed):** after the caret check the first character is always typed alone and read back; lost ->
  `orig.text_retry_settle_ms` (default 400) settle, the text tool again if the key switched it, one click into the
  text (`caret_retry_point`), editor and caret checked, the first character typed again; the rest only once it
  arrived, else runner error and nothing more typed. Each attempt in `text_entry.attempts` and the log line
  (`attempt n: first character 'A' arrived/lost ...`). `orig.type_delay_ms` default 20 -> 35 ms.
- **Firepixel text tool reset in T07 units (TW-149 leftovers, run 24 `panel-layers`, `panel-layers-menu`,
  `panel-pages-states-history`):** the read-back failed with -2147216344 = 0x80041428 `EBadSelection` (`draw.text`
  unavailable): T07's active layer after opening is the Web layer / the locked layer, so the scratch text could not be
  added - not a menu or dialog state. `FP_TEXT_ATTRS_JS` now tries the other layers top down (`currentLayerNum`,
  active layer restored, `layer=<n>` in the result); `text_tool_attrs` and `text.setToolDefaults` close menus /
  dialogs (`reset_light`) and try once more on an error.
- `--dry-run`: the key substitute self check covers `tool_key` and the nudge flag.

## R-23 additions (19.10.)

- **Screen-mode read-back (TW-159 a, run 25 `toolbox/13`: the reference's main window keeps style 0x96cf0000 with
  its caption in all three modes, so "standard" was read everywhere):** `screen_mode_state` gathers three kinds of
  evidence and logs all of them: (1) the tools column's three screen-mode buttons, grabbed 15 x 15 px each - the
  pressed one differs from the other two in mean brightness (`screen_button_pressed`, config
  `orig.screen_button_diff`, default 10 levels); (2) the main window: style, caption, `IsZoomed`, `GetMenu`, window
  and client rect against the monitor and its work area (`covers_monitor`), the document view's rect; (3) script:
  every `fw` property whose name holds "screen" or "full" (`SCREEN_MODE_JS`; no such property is documented, the
  list shows what exists). Mode = the pressed button when decisive, else "full?" when the window covers the monitor,
  else None (not decidable). `screen_mode_standard` raises a runner error only for a decisive other mode and presses
  F (no button) only then. New action `{"screen_mode_read": "<standard|full_menus|full>"}` (both sides) before the
  shot of each `toolbox/13` step: value `screen_mode_read` {want, verdict ok / MISMATCH / unknown, mode, source,
  evidence}, log `orig screen mode read (want …): …` with every evidence item; Firepixel logs `ui_state.screen_mode`.
- **Subselection nudges (TW-159 b, run 25 `xs-shortcuts/10_nudge_point`: the reference names the Subselection tool
  "Teilauswahl", which was not in `NUDGE_TOOLS`, so the real Right selected the Text tool):** `SUBSELECTION_TOOLS`
  = teilauswahl / unterauswahl / subselection. With the Subselection tool a plain arrow first runs
  `moveSelectedBezierPointsBy({x, y})` (the selected points move); the node coordinates of the selected paths
  (`PATH_NODES_JS`) are read before and after - changed = done (`script moveSelectedBezierPointsBy …` in the
  substitute log, value `points`), unchanged or an error (no point selected) -> `moveSelectionBy` as before.
- **Freeform size keys (TW-159 c, run 25 `tool-path-reshape/04`: with `nudge: false` the real Up went through
  unchecked and switched Freeform -> Pen -> Vector Path -> Redraw Path):** `nudge: false` now plans `tool_key` (the
  real key, the tool read back, a switch -> Pointer tool back and the step `unsupported`). New step flag
  `orig_tool_keys: "held"` (set on `04`): the reference side sends the key with the left mouse button held at the
  cursor (the step's `canvas_hover` point, `cursor_pos`), then releases it - hypothesis: the size keys work only
  during a drag. A switch all the same logs the Properties panel fields (`record_ui inspector.fields`) in the value
  `tool_key` and makes the step `unsupported` with the reason ("the size is compared by the Properties panel
  read-back only").
- **Text tool size reset (TW-159 d, run 25 `menu-text`):** the unit-start reset was not skipped - it ran (22.8 s,
  bold / italic / underline cleared), but neither the size field nor the R-22 retry 24 -> 13 -> 12 reached the tool,
  so 24 pt from `inspector-text` stayed; the last key probes of `inspector-text` had left the keyboard focus in an
  Edit control. Now every size attempt is read back with a scratch text and logged (`size retries from …: field … ->
  …; edit_control … -> …; menu … -> …`), stopping at the first that gives 12: (1) the R-22 field retry after
  `close_menus` and `view_focus`, 13 read back in between; (2) the size field's Edit control set directly
  (WM_SETTEXT) and committed with Enter; (3) the Text menu's size entry (`orig.text_size_menu`) with the text tool
  and nothing selected (run 25 `tool-text/11` showed that entry reaching the tool's size).
- **Text entry rests (TW-146 rest, run 25 `tool-text/07`, `11`, `13`):**
  - `objects_settled(why, want_min)`: the object count read every 150 ms until two reads agree (and reach
    `want_min`), at most `orig.commit_wait_s` (default 2 s); log `orig objects settled (<why>): {n, polls, ms,
    settled}`. Used by `commit_check` (a new text: one object more), before the first typed character, before the
    click retry of a lost first character, and before a Pointer double-click on the canvas.
  - The click retry of a lost first character is clicked once more when the first click opened no editor
    (`click_2` in the attempt).
  - Pointer double-click (`double_click_check`): the editor is polled for up to `orig.dbl_editor_wait_s` (default
    1.5 s) and the caret checked (`caret_check`); no editor -> the batch double-click (one SendInput batch) once more,
    editor and caret waited for again; value `double_click` / `double_click_retry` with the caret.
  - Text selection drags (`text_select_drag`): the highlight (changed pixels, cover) is checked after every drag; up
    to three tries - plain, a click at the text's end plus a pause of `orig.text_drag_pause_ms` (default 100 ms)
    after the press before the first move (new host `drag` parameter `pause_ms`), the pause again; every try in the
    log line. `text_size_on_selection` logs the highlight pixel count before the field and before the menu, and checks
    a new drag once more.
- `--dry-run` prints `screen mode self check: ok`; the key substitute self check covers "Teilauswahl" and
  `nudge: false` -> `tool_key`.

## R-24 additions (19.10.)

- **Size on a drag selection (TW-146 rest, run 26 `tool-text/11`: the drag highlighted "123", but the size field
  click dropped the editor selection and the 72 went to the text tool's default - the next text of the unit came
  out at 72 pt -, and Text > Size > 72 after a new drag left the runs at 12 pt):** `text_size_on_selection` tries
  routes in order (config `orig.text_size_routes`, default `["field", "keys", "dialog"]`; `"menu"` = the R-21 Text
  menu entry `orig.text_size_menu`). Before each route the highlight is checked (lost -> the drag once more), after
  it the selected text's runs are read back (`textRuns`); the first route that puts the size on the tail wins.
  `field` = size field click, size + Enter; `keys` = keyboard focus back to the document view (host `view_focus`, no
  click), then Text > Size > Larger / Smaller (Ctrl+Shift+Period / Comma, +-1 pt on the selection) as often as the
  tail's size is away from the wanted one (`tail_size`; at most `orig.text_size_keys_max`, default 100 - 60 presses
  for 12 -> 72); `dialog` = the Text Editor (Ctrl+Shift+E) with the selection kept, its numeric field holding the
  tail's size set by WM_SETTEXT, OK (no dialog -> logged; the key is missing from some shortcut sets). Value
  `text_size` {routes [{route, selection, how, runs, ok}], method, runs_before, runs_field, runs_after}, log `orig
  text size on the selection: … (ok by <route> / NOT ON …)`. A failed `field` / `menu` route sets
  `pending_text_defaults`, so the unit's text tool reset runs again with the next test document.
- **Drag over large text (TW-146 rest, run 26 `tool-text/13`: three drags 360 -> 249 from the right end of the
  72 pt text left only a caret column at 245, cover 0.0):** `text_select_drag` computes the glyph boxes of the
  characters (`glyph_spans`: advance widths of `ARIAL_ADVANCE` scaled by the run size from
  `textRuns.initialAttrs.size`, or by the `pixelRect` width when the two differ by more than 15 %) and drags left to
  right from 2 px inside the first selected glyph to 2 px past the last one, the highlight cover measured over those
  glyphs. For a tall line (height >= `glyph_min_px`, default 30; or `"glyphs": true` in the action) the glyph drags
  come first (plain, then a click at the end plus the pause) and the three right-to-left drags follow; otherwise the
  glyph drag is the fourth try. Values `text_select.glyphs` {size, scale, span, drag}, `text_select.plan`,
  `text_select.method`.
- **Freeform size keys with the button held (TW-159 rest c, run 26 `tool-path-reshape/04`: the keys kept the
  Freeform tool, but no shot and no size were recorded):** with `orig_tool_keys: "held"` the reference's left button
  goes down once at the first tool key and stays down (`held`, `tool_keys_held_at`) for the following keys - also the
  plain `]` - until the new action `{"release_mouse": true}` (both sides; step cleanup) or the next reset. New action
  `{"tool_size_read": {"label": "Größe", "orig_rel": [x, y]}}` (both sides): reference = the Properties panel's
  Edit / ComboBox children read by WM_GETTEXT (host `kids`, `get_text`; no input, no script), the numeric one nearest
  to `orig_rel` or else the leftmost (`pick_size_field`, `size_digits`); Firepixel = `ui_find` of the label (lowest
  hit) and the numeric texts right of it. Value `tool_size` {size, fields, held / label}, logs `orig tool size read
  (mouse button held): …` / `fp tool size read: …`; `unsupported` only when no numeric field is readable. Step `04`
  reads the size before `record_ui`, gets `shot: canvas` (taken while the button is still held) and
  `cleanup: release_mouse`.
- **Standard-mode detection (TW-163, run 26 `toolbox/13`: the screen-mode buttons read "full screen with menus"
  (181.2, 167.3, 180.2) in Standard, six `runner error` lines "not standard after the reset"):** the window against
  the monitor is now the primary source (`screen_mode_from_window`): not covering the monitor -> standard; covering
  it with a visible caption (client top below the window top) -> standard; covering it without one -> full (the
  document view at the client top) or full with menus (view more than 20 px lower; run 26: 89 px), "full?" when the
  view is unknown. The buttons are secondary: used only when the window is unreadable or reads "full?"; a
  disagreement is logged as `buttons_agree: false` (also in the short form of `orig screen mode standard: …`).
  `screen_mode_standard` raises its runner error only when this verdict is not standard.
- **Freeform through the flyout (TW-164, runs 24-26 `tool-path-reshape/12_inspector_heads_freeform`: the Freeform
  slot click selected the Path Scrubber the slot still showed from steps `06` / `07`):** ui_map `tool.freeform` =
  `toolbox.freeform_flyout` entry 0 (order Freeform 0, Reshape Area 1, Path Scrubber add 2, subtract 3, unchanged).
  Tool entries may carry `orig_name` (set for the four group tools: "Freiform", "Bereich umformen",
  "Pfadausbesserung"): after the `tool` action the active tool is read back (`tool_name_check`); wrong -> the flyout
  entry once more, then `fw.activeTool = orig_name`; value `tool_check`, log `orig tool check <tool>: …`, runner
  error when the tool stays wrong.
- `--dry-run` prints `R-24 self check: ok` (glyph boxes of run 26's 72 pt and 12 pt texts, `tail_size`, the
  Freeform flyout order in ui_map); `screen mode self check` covers the window cases of run 26 and the size field
  pick.

## R-25 additions (19.10.)

- **Word selection for text edits (TW-146 rest, run 27 `tool-text/11`, `13`: the "highlight" after the drag was 24
  changed px with cover 1.0 - the caret moving from the end to the start of "123", two 12 px columns - so no size or
  colour route could reach the characters):** `text_select_drag` now drags over the glyph boxes first for every size
  (Arial advance widths x run size, from 2 px inside the first selected glyph to 2 px past the last, y in the middle
  of the x-height zone, `xheight_y`), then the R-23 drags from the end, then - when the tail is one whole word
  (`tail_is_word`, e.g. "123" of "Abc 123") - a double-click on it (word selection, `text_select.method` "word").
  Every try is checked with the expected inverted area (glyph span width x line height on the screen, value
  `text_select.expected_area`): `selection_highlight(..., area=, area_min=)` needs at least `orig.text_highlight_min`
  (default 0.6) of it changed (values `expected_area`, `area_ratio`); each try logs measured px, expected area and
  ratio. `text_size_on_selection` runs a size route only on a checked selection (lost -> selected again; still none
  -> the route is skipped, so no size reaches the tool's default).
- **Two-sided text tool reset (TW-165, run 27: after the failed size step of `11` only the reference's text tool
  went back to 12 pt, Firepixel's kept 72 pt by the F43 rule, so `14` and `16` compared 12 pt with 72 pt):** a
  pending reset from a failed size step is mirrored to Firepixel before the next step (`fp_text_reset_pending`; log
  `text tool reset pending on both sides (after the failed size step in <step>) …`); both sides reset with their
  next document (reference: Properties panel, Firepixel: `text.setToolDefaults`), logged as `text tool reset after
  the failed size step in <step> (…)` and `fp text tool reset after … (…)`.
- **Leaving full screen by key (TW-166, run 27 `toolbox/13_view_modes_full_normal`: the Standard reset clicked the
  screen-mode button that full screen hides and tried the Window menu without a menu bar):** `screen_mode_standard`
  plans its moves from the mode read before (`screen_standard_plan`): full -> F once, full with menus or "full?" ->
  F up to twice (F cycles standard -> full with menus -> full -> standard), each read back by the window test; the
  tools button only from Standard or an unreadable mode, never in full; F as before when nothing else works.
  Afterwards the unit-start layout restore (`ensure_layout`, R-2) and the panel dock width (client right - view
  right against the first width read this run; `dock_collapse_px`, default 100) are logged (`| layout {…}`, value
  `screen_mode.layout` with `collapsed`). This runs in the `after` of all three `toolbox/13` steps.
- `--dry-run` prints `R-25 self check: ok` (run 27's caret move rejected and an inverted block accepted by the area
  test, `tail_is_word`, `xheight_y`, the Standard plan per mode).
