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
5. End of unit: close menus/dialogs, close the test documents without saving, stop the Firepixel instance,
   release the lock, run `compare.py`. At the end of the run: `index.html`; the reference program is
   closed only if the runner started it.

Real input to the reference program (SendInput) is only sent while its window (or one of its dialogs/menus)
owns the foreground; otherwise the action fails with "input refused". Ctrl+Alt+Space is never sent.
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
changed compared to a window image taken before the step's actions (`area_from: difference` in the log).

An action is an object whose first known key is the verb; further keys are options (`mods`, `repeat`,
`item`, `value`, `target`, `by`, `hold_key`, `ms`). A plain string is a verb without argument.

### Verbs

| verb | argument | reference program | Firepixel |
|---|---|---|---|
| `open_testfile` / `reopen` | `"T04"` (or `true` = unit file) | close same file unsaved, `fw.openDocument(url)` | `documents close` + `open` |
| `close_all` / `close_all_documents` | – | closes the test files opened in this unit | same |
| `menu` / `menu_open` / `hover_menu` | `"A > B > C"` (also `A/B`, list) | click bar entry, hover sub menus (leaves the last open) | `ui_find` + `ui_input` |
| `menu_item` | `"A > B"` | like `menu`, last entry clicked | same |
| `command` | Firepixel command id | menu path from `ui_map.json → commands` | `ui_action {run}` |
| `close_dialogs` | – | cancel dialogs, close menus | `ui_action {close_dialogs}` |
| `key` | `"ctrl+shift+z"`, `"Escape"`, `"F1"` (`repeat`) | SendInput | `ui_input key` (egui names) |
| `type` | text | Unicode SendInput | `ui_input type` |
| `wait` | ms | sleep | sleep |
| `select` | object name or list | JSF `fw.selection` by name | `get_document` + `select {ids}` |
| `select_none` / `select_all` | – | JSF | `select` |
| `tool` | `tool.rectangle` | shortcut key from `ui_map.json → tools` | `ui_action {tool}` |
| `panel` | `window.layers` / id / title | visible? else menu `Fenster > …` | `ui_action {panel}` if not visible |
| `jsf` / `run_script` | code | JSF in the running instance | `run_script` |
| `log_value` / `log` | JSF expression | value into `values` | `run_script` value |
| `click` / `double_click` / `right_click` / `hover` / `open` / `click_text` / `click_button` / `toolbox_click` | target | SendInput at the target | `ui_input` at the target |
| `mouse_down` / `mouse_up` | target | unsupported | `ui_input down/up` |
| `canvas_click` / `canvas_double_click` / `canvas_move` / `canvas_hover` | `[x, y]` document px (+`mods`) | view centre − doc/2 (+ `orig.canvas_offset`) | `page_on_screen` |
| `drag` / `canvas_drag` | `{from, to | by, mods, steps}` (points = `[x,y]` document px or targets) | SendInput drag | `ui_input drag` |
| `choose` | `{target, item}` (`item` may be `A > B`) | Win32 combo box: select entry | click target, click item texts |
| `set_field` / `set` / `set_dialog_field` | `{target, value}` | Win32 edit: text; else double click, Ctrl+A, type, Enter | click, Ctrl+A, type, Enter |
| `record_ui` | logical key (`ui_state.*`, `document.summary`, `menu.open`, `dialog.fields`, …) | dialogs/menus/title | `ui_state`/`get_document`/menu items |
| `clear_dir` / `record_dir` / `record_file` / `record_file_text` | path (relative = `run\<unit>\files\…`, only below the test folder) | file listing / size + SHA-1 + pixels | same |
| `file_dialog` / `file_dialog_choose` / `file_dialog_cancel` | path | edit field of the open file dialog + OK / Cancel | `ui_file_dialog` |
| `ui_action`, `ui_find`, `ui_state`, `get_document`, `ui_file_dialog`, `ui_find_click` | bridge arguments | – (put under `fp`) | bridge tool |

Not supported yet (logged as `unsupported`, the run continues): `handle`/`drag_handle`/`click_handle`/
`hover_handle`, `drag_popup_slider`, `slider_drag`, `flyout_open`/`flyout_item`, `panel_menu`, `fx_add`,
`read_dialog_fields`, `export`/`export_png`, `scroll`/`wheel`/`canvas_wheel`, `resize_panel`, `resize_dialog`,
`drag_dialog_*`, `restart_app`, `window_size`, `log_focus_cycle`, `set_clipboard_image`, `drop_files` and the
other rare verbs from the scripts' notes. Target forms `{layer_row}`, `{object_row}`, `{tree_item}`,
`{pref_category}`, `{history_row}` … work in Firepixel (found by their text) but not yet in the reference program.

### Targets

A target is either
* a **logical UI key** from `ui_map.json → targets` (e.g. `inspector.fill_color`, `dialog.ok`),
* a **label** (button, menu entry or other visible text; compared without `&` and trailing `...`/`…`):
  reference program = text of a Win32 child window or of an item in an open menu; Firepixel = `ui_find`,
* `{"canvas": [x, y]}` (document pixels), `{"window_rel": [x, y]}` (client coordinates),
  `{"area": "panel:layers", "pos": [x, y]}` (offset inside a shot area),
* `{"target": <target>, "mods": ["shift"]}`.

## ui_map.json

The mapping table. Script authors add entries here instead of hard-coding coordinates in scripts.

* `menu_bar.orig`: `[name, x, y]` of the reference program's custom-drawn menu bar (client coordinates).
  Open sub menus are standard popup menus: their items, states and rectangles are read directly.
* `panels`: panel id → `fp` panel command, German `title` (window title of the reference panel / tab text
  in Firepixel), `menu` path to show it. Ids are matched without `window.`/`panel:` and with `-` = `_`
  (`window.colorMixer` = `color-mixer`).
* `tools`: Firepixel tool id → reference shortcut key (or `{"jsf": …}`).
* `commands`: Firepixel command id → reference menu path.
* `targets`: logical key → `{"orig": locator, "fp": locator}`; `null` or a missing side = `unmapped`.
  * reference locators: `{"panel": "properties", "rel": [x, y]}` (offset from the panel window's
    top-left), `{"window_rel": [x, y]}`, `{"child": {"title"?, "class"?, "index"?}, "panel"?}` (Win32
    child window), `{"dialog_child": {…}}`, `{"dialog_button": "ok"|"cancel"}`, `{"label": "Text"}`,
    `{"canvas": [x, y]}`.
  * Firepixel locators: `{"find": "Text", "role"?: "button", "exact"?: true, "panel"?: "layers", "index"?: 0}`,
    `{"window_rel": [x, y]}`, `{"canvas": [x, y]}`.

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

* Document → screen mapping in the reference program assumes zoom 100 % and a centred document that
  fits into the view (true for T01–T09 at 1600 × 1000; not for T10).
* Hover-only states (tooltips, flyouts after press-and-hold) and mouse pointer images are not captured.
* The reference program must not be used by a person while the runner holds the lock (it needs the foreground).
