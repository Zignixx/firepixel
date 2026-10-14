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
5. End of unit: close menus/dialogs, close the test documents without saving, stop the Firepixel instance,
   release the lock, run `compare.py`. At the end of the run: `index.html`; the reference program is
   closed only if no instance was running when the run began (a restart during the run does not change that).

Modal windows: besides standard dialogs (`#32770`) the program has Flash based dialogs (top-level class
`SWFPLAYER_SWFWINDOWCONTAINER`, e.g. `Datei > Datei-Info`) without Win32 buttons; they count as dialogs
(shot area `dialog`, which waits up to 4 s for a late dialog) and are cancelled with `WM_CLOSE`. A Flash dialog
left open blocks the menu bar, so `menu` closes them first and tries opening the bar entry up to 3 times
(closing menus/Flash dialogs and giving the foreground back to the main window in between); the error then
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
| `select` | object name or list | JSF `fw.selection` by name | same JSF via `run_script` (exactly the named objects; `select {ids}` only as fallback) |
| `select_none` / `select_all` | – | JSF | `select` |
| `tool` | `tool.rectangle` | click on the toolbox button (`ui_map.json → tools`; single-key shortcuts do not reliably reach the program) | `ui_action {tool}` |
| `panel` | `window.layers` / id / title | visible? else menu `Fenster > …` | `ui_action {panel}` if not visible (fallback: tab / menu, see below) |
| `jsf` / `run_script` | code | JSF in the running instance | `run_script` |
| `log_value` / `log` | JSF expression | value into `values` | `run_script` value |
| `click` / `double_click` / `right_click` / `hover` / `open` / `click_text` / `click_button` / `toolbox_click` | target | SendInput at the target | `ui_input` at the target |
| `mouse_down` / `mouse_up` | target | unsupported | `ui_input down/up` |
| `canvas_click` / `canvas_double_click` / `canvas_move` / `canvas_hover` | `[x, y]` document px (+`mods`) | page rectangle measured in the view image (+ `orig.canvas_offset`) | `page_on_screen` |
| `drag` / `canvas_drag` | `{from, to | by, mods, steps}` (points = `[x,y]` document px or targets) | SendInput drag | `ui_input drag` |
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
| `canvas_wheel` | `{pos: [x, y] document px, delta, modifiers?}` | wheel at the document point; `delta` > 0 = wheel away from the user | same |
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
    top-left), `{"window_rel": [x, y]}`, `{"child": {"title"?, "class"?, "index"?}, "panel"?}` (Win32
    child window), `{"dialog_child": {…}}`, `{"dialog_button": "ok"|"cancel"}`, `{"label": "Text"}`,
    `{"canvas": [x, y]}`, `{"dialog_rel": [x, y]}`. For `rel`/`dialog_rel` points the runner asks which Win32
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
`fp.preview_glob`, `fp.work_dir`, `fp.rect`, `fp.theme`, `fp.language`, `testfile_dir`, `testfiles`
(T-code → file), `out_dir`, `tmp_dir`, `step_timeout_s`.

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
