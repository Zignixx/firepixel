"""Parity runner: plays the scripts in tools/parity/scripts/<unit>.json in both programs
(reference program and Firepixel), stores equally named screenshots, runs compare.py and
builds a gallery.  See README.md in this folder.

  python -I runner.py --units a,b,c | --all [--dry-run] [--steps id1,id2] [--no-gallery]
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import shutil
import re
import subprocess
import sys
import time
import traceback
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from PIL import Image, ImageChops  # noqa: E402

import gallery  # noqa: E402
from fpctl import BridgeError, Firepixel  # noqa: E402
from origctl import Lock, OrigError, Original, file_url as file_url_of, js_str  # noqa: E402

REPO = HERE.parents[2]
SCRIPTS = HERE.parent / "scripts"
COMPARE = HERE.parent / "compare.py"
DEFAULT_CONFIG = REPO.parent / "re" / "_testdateien" / "par" / "runner.config.json"


def subst_placeholders(o, side, unit, tmp=None):
    """"{side}", "{unit}"; R-6: "{tmp_url}" (file URL) and "{tmp}" (path) of the runner's tmp_dir - copies saved
    by a step go there, never over a test file (Q-41)."""
    if isinstance(o, str):
        if "{" not in o:
            return o
        if tmp is not None:
            o = o.replace("{tmp_url}", file_url_of(tmp)).replace("{tmp}", str(tmp))
        return o.replace("{side}", side).replace("{unit}", unit)
    if isinstance(o, list):
        return [subst_placeholders(x, side, unit, tmp) for x in o]
    if isinstance(o, dict):
        return {k: subst_placeholders(v, side, unit, tmp) for k, v in o.items()}
    return o


class TestfilesChanged(Exception):
    """R-6 (Q-41): a test file differs from TESTFILES.sha256 - the run must not go on with it."""


# R-8 (TW-105, panel-swatches-mixer/19: reference #4080c0, Firepixel #000000): colour under a grid click point,
# read from a window image right before the click, on both sides
SWATCH_KEYS = ("panel_swatch", "popup_swatch", "color_table_cell", "swatch")


def probe_color(side, pt, img=None) -> str:
    """Colour '#rrggbb' at a side point (reference: screen, Firepixel: window); `img` = a Firepixel window image
    already taken (several probes from one image)."""
    if side.name == "orig":
        tmp = Path(side.r.cfg["tmp_dir"]) / "orig_probe.png"
        side.grab([int(pt[0]) - 1, int(pt[1]) - 1, int(pt[0]) + 2, int(pt[1]) + 2], tmp)
        with Image.open(tmp) as im:
            px = im.convert("RGB").getpixel((1, 1))
    else:
        img = img if img is not None else side.window_image()[0]
        px = img.getpixel((min(img.width - 1, int(pt[0])), min(img.height - 1, int(pt[1]))))
    return "#%02x%02x%02x" % tuple(px[:3])


def click_probe(side, target, pt):
    rec = {"target": target, "at": [int(pt[0]), int(pt[1])]}
    try:
        rec["color"] = probe_color(side, pt)
    except Exception as e:  # noqa: BLE001
        rec["error"] = str(e)[:100]
    side.r.log(f"{side.name} click probe: {rec}")
    return rec


# R-11 (TW-115): swatch set read-back after the reset step - JSF `fw.swatches` (reference; Firepixel may not have it)
# and the reference's palette choice pref (0 = the 'colour cubes' table, re 14 section 2.1)
SWATCH_COUNT_JS = ("(function(){try{var s=fw.swatches;if(!s)return 'none';var a=[];for(var i=0;i<8&&i<s.length;i++)"
                   "a.push(String(s[i]));return s.length+'|'+a.join(',');}catch(e){return 'error '+e;}})()")
SWATCH_PREF_JS = "(function(){try{return fw.getPref('SwatchesPanelColorPaletteChoice');}catch(e){return 'error '+e;}})()"


def fp_serialised_js(expr: str) -> str:
    """R-12 (TW-124 a): Firepixel's `run_script` returns String(result), so an object read back by `log_value`
    (`textRuns.initialAttrs`) came back as "[object Object]". The expression is wrapped so that objects and
    arrays come back serialised - toSource() like the reference side's read-back, JSON.stringify as fallback;
    other values stay unchanged."""
    return ("(function(){var __v=(%s);if(typeof __v=='object'&&__v!==null){try{if(__v.toSource)return __v.toSource();}"
            "catch(e){}try{return JSON.stringify(__v);}catch(e){}}return __v;})()" % expr)


# R-8 (TW-102/103): export options read on both sides before an export (and in Firepixel after a format choice)
EXPORT_PROBES = (("colorMode", "fw.getDocumentDOM().exportOptions.colorMode"),
                 ("numPaletteEntries", "fw.getDocumentDOM().exportOptions.numPaletteEntries"),
                 ("states", "fw.getDocumentDOM().frames.length"))


def export_spec(arg):
    """`export {dir, name, type, slices?}`: only the plain image export (type 'Nur Bilder', no slices, no options
    dialog) is supported - it uses the document's own export settings on both sides (RUN-6)."""
    a = arg if isinstance(arg, dict) else {"dir": arg}
    if (a.get("type") or "Nur Bilder") != "Nur Bilder" or (a.get("slices") or "Ohne") != "Ohne"             or a.get("open_options") or a.get("selected_slices_only"):
        raise Unsupported(f"export type {a.get('type')} / slices {a.get('slices')} (only 'Nur Bilder' without slices)")
    return a


EXPORT_EXT = {"GIF": "gif", "GIF ANIMATION": "gif", "ANIMATED GIF": "gif", "JPEG": "jpg", "JPG": "jpg", "PNG": "png",
              "BMP": "bmp", "TIFF": "tif", "WBMP": "wbmp", "WEBP": "webp", "AVIF": "avif"}


class Unsupported(Exception):
    """Action the runner cannot perform (logged as 'unsupported', not as a failure of the program)."""


class Unmapped(Exception):
    """Logical UI key without an entry for this side in ui_map.json."""


# R-13 (TW-126): shortcut set written into Firepixel's runner profile (config `fp.shortcut_set`, same default as
# fpctl) and the reference preference read for its set (config `orig.shortcut_set_pref`)
FP_SHORTCUT_SET_DEFAULT = "Web Standard"  # R-13b (RE-14): the reference machine's set
ORIG_SHORTCUT_SET_PREF_DEFAULT = "CurrentShortcutSet"


def bm_getcheck(hwnd: int):
    """R-13 (TW-127): BM_GETCHECK of a Win32 check box / radio button of the reference (True / False, None for
    'indeterminate' or no answer). The message carries no pointers, so it works across processes."""
    import ctypes
    from ctypes import wintypes
    u = ctypes.windll.user32
    u.SendMessageTimeoutW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM, wintypes.UINT,
                                      wintypes.UINT, ctypes.POINTER(ctypes.c_size_t)]
    u.SendMessageTimeoutW.restype = ctypes.c_size_t
    res = ctypes.c_size_t(0)
    ok = u.SendMessageTimeoutW(wintypes.HWND(int(hwnd)), 0x00F0, 0, 0, 0x0002, 1500, ctypes.byref(res))
    if not ok:
        return None
    return {0: False, 1: True}.get(int(res.value))


def falsy_value(v) -> bool:
    """A read-back value that means 'nothing' (retry_if `retry_when_falsy`)."""
    return v is None or str(v).strip().lower() in ("", "0", "false", "null", "undefined", "[]", "none")


# --------------------------------------------------------------------------- helpers
def select_names(names) -> list:
    """`select` names: object names, {"index": i} (i-th element over all layers) and, R-11 (run 15
    export-results/12, panel-optimize: 'slice:0' was looked up as an object name and not found on both sides),
    "slice:N" / {"slice": N} = the N-th element of the web layer(s) (slices and hotspots in layer order)."""
    out = []
    for n in names if isinstance(names, list) else [names]:
        m = re.fullmatch(r"slice:(\d+)", n.strip()) if isinstance(n, str) else None
        out.append({"slice": int(m.group(1))} if m else n)
    return out


def clean_label(s: str) -> str:
    return re.sub(r"(\.\.\.|…)$", "", str(s).replace("&", "")).strip()


def split_path(p) -> list[str]:
    if isinstance(p, list):
        return [clean_label(x) for x in p]
    # 'A > B > C' (entries may contain '/', e.g. 'Farbton/Sättigung'); 'A/B' only without '>'
    parts = re.split(r"\s*(?:>|▸)\s*", str(p)) if (">" in str(p) or "▸" in str(p)) else str(p).split("/")
    return [clean_label(x) for x in parts if x.strip()]


def norm_panel(p: str) -> str:
    p = str(p)
    for pre in ("window.", "panel:", "panel_tab:"):
        if p.startswith(pre):
            p = p[len(pre):]
    return re.sub(r"([a-z])([A-Z])", r"\1-\2", p).replace("_", "-").lower()


def bbox_union(rects):
    rects = [r for r in rects if r]
    if not rects:
        return None
    return [min(r[0] for r in rects), min(r[1] for r in rects), max(r[2] for r in rects), max(r[3] for r in rects)]


def norm_rect(rect):
    """R-12 (TW-124 f, run 16 panel-optimize/13: [1438, 263, 1357, 631]): a region with swapped corners is
    normalised (left <= right, top <= bottom) instead of falling back to the whole window."""
    if not rect or len(rect) < 4:
        return rect
    x0, y0, x1, y1 = (float(v) for v in rect[:4])
    return [min(x0, x1), min(y0, y1), max(x0, x1), max(y0, y1)]


# R-12 (TW-124 b): order gap (painted shapes) that ends the topmost block of texts in a region
POPUP_ORDER_GAP = 40


def top_paint_block(hits, gap: int = POPUP_ORDER_GAP):
    """R-12 (TW-124 b, run 16 panel-optimize/13,15, panel-misc/14,18): Firepixel's popup menus are painted over the
    panels, and `ui_find` in the menu's box also returns the panel texts underneath ("Sortieren nach:",
    "Webebene", the find panel's labels). The menu is painted last: the texts with the highest paint `order`
    form one block (entries, shortcuts, sub menus a few shapes apart); the block ends at the first order gap
    larger than `gap`. Hits without an order are kept as they are."""
    if not hits or any("order" not in h for h in hits):
        return hits
    srt = sorted(hits, key=lambda h: -int(h["order"]))
    out = [srt[0]]
    for h in srt[1:]:
        if int(out[-1]["order"]) - int(h["order"]) > gap:
            break
        out.append(h)
    return out


def diff_bbox(a: Image.Image | None, b: Image.Image | None, pad: int = 6):
    """Bounding box of what changed between two equally sized images (used for popup/menu areas)."""
    if a is None or b is None or a.size != b.size:
        return None
    d = ImageChops.difference(a.convert("RGB"), b.convert("RGB")).convert("L").point(lambda v: 255 if v > 24 else 0)
    bb = d.getbbox()
    if not bb:
        return None
    return [max(0, bb[0] - pad), max(0, bb[1] - pad), min(a.width, bb[2] + pad), min(a.height, bb[3] + pad)]


def popup_component(a: Image.Image | None, b: Image.Image | None, seed=None, cell: int = 6, pad: int = 2):
    """Area of the opened popup only (R-4, `log_menu_items` in Firepixel): the changed pixels are grouped into
    connected regions (cells of `cell` px, 8-neighbourhood, so gaps below one cell are bridged). With `seed` (a
    box known to lie in the popup) every region touching it counts, otherwise the largest region. Other changes in
    the window (hover highlights, status texts, a panel repainted) are left out - run 8 read window texts as menu
    entries (URL, Verhalten, Suchen) and cut the Stile list."""
    if a is None or b is None or a.size != b.size:
        return None
    d = ImageChops.difference(a.convert("RGB"), b.convert("RGB")).convert("L").point(lambda v: 255 if v > 24 else 0)
    gw, gh = max(1, a.width // cell), max(1, a.height // cell)
    g = d.resize((gw, gh), Image.BOX)
    px = g.load()
    on = {(x, y) for y in range(gh) for x in range(gw) if px[x, y] > 0}
    if not on:
        return None
    comps, seen = [], set()
    for start in on:
        if start in seen:
            continue
        seen.add(start)
        stack, cells = [start], []
        while stack:
            x, y = stack.pop()
            cells.append((x, y))
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    n = (x + dx, y + dy)
                    if n in on and n not in seen:
                        seen.add(n)
                        stack.append(n)
        xs, ys = [c[0] for c in cells], [c[1] for c in cells]
        comps.append((len(cells), [min(xs) * cell, min(ys) * cell, (max(xs) + 1) * cell, (max(ys) + 1) * cell]))
    if seed:
        hit = [r for _, r in comps if r[0] < seed[2] and r[2] > seed[0] and r[1] < seed[3] and r[3] > seed[1]]
        if hit:
            bb = bbox_union(hit)
            return [max(0, bb[0] - pad), max(0, bb[1] - pad), min(a.width, bb[2] + pad), min(a.height, bb[3] + pad)]
    bb = max(comps, key=lambda c: c[0])[1]
    return [max(0, bb[0] - pad), max(0, bb[1] - pad), min(a.width, bb[2] + pad), min(a.height, bb[3] + pad)]


OPENERS = {"menu", "menu_open", "hover_menu", "open", "click", "right_click", "double_click", "click_text", "right_click_text",
           "click_button", "toolbox_click", "choose", "key", "ui_find_click", "ui_action", "command", "menu_item", "hover"}

KEY_ALIASES = {"escape": "esc", "enter": "enter", "return": "enter", "del": "delete", "control": "ctrl", "strg": "ctrl",
               "cmd": "ctrl", "umschalt": "shift", "option": "alt"}
EGUI_KEYS = {"esc": "Escape", "enter": "Enter", "tab": "Tab", "space": "Space", "delete": "Delete", "backspace": "Backspace",
             "left": "ArrowLeft", "right": "ArrowRight", "up": "ArrowUp", "down": "ArrowDown", "home": "Home", "end": "End",
             "pageup": "PageUp", "pagedown": "PageDown", "insert": "Insert", "plus": "Plus", "minus": "Minus",
             "comma": "Comma", "period": "Period",
             # R-9 (run 13 xs-shortcuts: 'unknown key Apostrophe'): egui names of the punctuation keys
             "apostrophe": "Quote", "quote": "Quote", "semicolon": "Semicolon", "slash": "Slash", "backslash": "Backslash",
             "equals": "Equals", "backtick": "Backtick", "openbracket": "OpenBracket", "closebracket": "CloseBracket"}


def parse_combo(c: str) -> tuple[list[str], str]:
    parts = [p.strip() for p in re.split(r"\+(?!$)", str(c)) if p.strip()]
    parts = [KEY_ALIASES.get(p.lower(), p) for p in parts]
    mods = [p.lower() for p in parts[:-1]]
    key = parts[-1]
    lk = key.lower()
    if lk.startswith("arrow"):
        lk = lk[5:]
    return mods, KEY_ALIASES.get(lk, lk)


HELD_MODS = ("ctrl", "shift", "alt")


def norm_mods(m) -> list[str]:
    """R-9 (run 13: the `hold_key` 'ctrl+Space' came out as the characters c, t, r, l, ...): modifier / held-key
    list from a string ('ctrl+Space') or a list, lower case, aliases resolved."""
    if not m:
        return []
    out = []
    for p in (m if isinstance(m, (list, tuple)) else [m]):
        for q in re.split(r"\+(?!$)", str(p)):
            q = q.strip().lower()
            if q:
                out.append(KEY_ALIASES.get(q, q))
    return out


def egui_key(k: str) -> str:
    return EGUI_KEYS.get(k, k.upper() if len(k) == 1 else k[:1].upper() + k[1:])


# actions that may switch a view setting (grid, guides, rulers, edges …): the side resets the switches before the next step
VIEW_TOGGLE = re.compile(r'^(menu\w*|hover_menu|command) .*("Ansicht|"view\.)|setShow|setSnap|lockGuides|'
                         r'^key "ctrl\+(alt\+)?(shift\+)?[äüöÄÜÖ]', re.I)


def set_field_commit(a: dict):
    """How `set_field` ends: option `commit` (alias `then`) = "enter" | "tab" | "none"; None = side default
    (field in an open dialog: "tab" - the value is taken over, the dialog stays open and the script's next action
    decides (Enter/OK, Escape, Vorschau …); any other field: "enter")."""
    c = a.get("commit", a.get("then")) if isinstance(a, dict) else None
    if c is None:
        return None
    c = str(c).lower()
    if c in ("return", "ok"):
        c = "enter"
    if c in ("", "false", "no"):
        c = "none"
    if c not in ("enter", "tab", "none"):
        raise Unsupported(f"set_field commit {c!r}")
    return c


def pick_handle(h, pts, rect, roles):
    """pts = [(x, y, tooltip, visible)] in document pixels; h = index or role ('points', 'turns' ...,
    matched as tooltip prefix via ui_map handle_roles) or a frame handle name. Without control points
    the 8 frame handles of the bounding box are used (0 = top left, clockwise)."""
    vis = [p for p in pts if p[3]]
    if isinstance(h, int) or (isinstance(h, str) and h.isdigit()):
        i = int(h)
        if i < len(pts):
            return pts[i][0], pts[i][1]
        if rect:
            l, t, r, b = rect
            frame = [(l, t), ((l + r) / 2, t), (r, t), (r, (t + b) / 2), (r, b), ((l + r) / 2, b), (l, b), (l, (t + b) / 2)]
            return frame[i % 8]
        raise RuntimeError(f"handle {h}: object has {len(pts)} control points and no bounds")
    want = [w.lower() for w in ([roles.get(h)] if isinstance(roles.get(h), str) else roles.get(h) or []) + [str(h)]]
    for p in vis:
        tip = (p[2] or "").lower()
        if any(tip.startswith(w) for w in want):
            return p[0], p[1]
    raise RuntimeError(f"handle '{h}' not found among {[p[2] for p in pts]}")


def scan_edge(img: Image.Image, x: int, y: int, dx: int, dy: int, thr: int = 30, ref=None):
    """Walk from (x, y) in direction (dx, dy) until the luminance differs from the start colour by more than
    `thr` (frame line / shadow of a dialog or floating panel). Returns the last point inside."""
    g = img.convert("L") if img.mode != "L" else img
    w, h = g.size
    x, y = max(0, min(w - 1, int(x))), max(0, min(h - 1, int(y)))
    ref = g.getpixel((x, y)) if ref is None else ref
    while 0 <= x + dx < w and 0 <= y + dy < h:
        if abs(g.getpixel((x + dx, y + dy)) - ref) > thr:
            return x, y
        x, y = x + dx, y + dy
    return x, y


def slider_points(r, frac, from_frac=None):
    """Press and target point inside a popup slider window rect (long axis; vertical: top = maximum)."""
    l, t, rr, b = r
    frac = max(0.0, min(1.0, frac))
    pad = 6
    if (b - t) >= (rr - l):
        x = (l + rr) // 2
        y_of = lambda f: int(b - pad - f * (b - t - 2 * pad))
        start = y_of(from_frac) if from_frac is not None else y_of(frac) + (8 if frac < 0.5 else -8)
        return (x, start), (x, y_of(frac))
    y = (t + b) // 2
    x_of = lambda f: int(l + pad + f * (rr - l - 2 * pad))
    start = x_of(from_frac) if from_frac is not None else x_of(frac) + (-8 if frac > 0.5 else 8)
    return (start, y), (x_of(frac), y)


def find_color_cell(img: Image.Image, region, hexcol: str, half: int = 2):
    """Centre of the first swatch cell (reading order) inside `region` (image coordinates) whose pixels around the
    centre all have the colour `hexcol` ('#RRGGBB'). Swatch grids are drawn without anti-aliasing in both programs."""
    h = hexcol.lstrip("#")
    want = (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))
    im = img.convert("RGB")
    l, t, r, b = [int(v) for v in region]
    l, t, r, b = max(0, l), max(0, t), min(im.width, r), min(im.height, b)
    px = im.load()
    for y in range(t + half, b - half):
        for x in range(l + half, r - half):
            if px[x, y] == want and all(px[x + dx, y + dy] == want for dx in (-half, 0, half) for dy in (-half, 0, half)):
                # walk to the cell's centre
                x2 = x
                while x2 + 1 < r and px[x2 + 1, y] == want:
                    x2 += 1
                y2 = y
                while y2 + 1 < b and px[x, y2 + 1] == want:
                    y2 += 1
                return (x + x2) // 2, (y + y2) // 2
    return None


# --------------------------------------------------------------------------- sides
def tools_anchor_in(g: Image.Image):
    """Anchor of a docked tools column image (grey scale, column only): top of the first row with dark text in
    the middle band (heading of the first tool group), x = centre of that heading. None if not found."""
    w, hh = g.size
    lo, hi = int(w * 0.28), int(w * 0.72)
    # start below the panel's title strip: two full-width dark lines 6-16 px apart without dark glyphs between
    # them (a toolbar docked above the column has icons between its lines and is skipped)
    full = [y for y in range(hh) if sum(1 for x in range(w) if g.getpixel((x, y)) < 120) >= 0.85 * w]
    y0 = 20
    for a, b in zip(full, full[1:]):
        if 6 <= b - a <= 16 and all(min(g.getpixel((x, y)) for x in range(w)) > 40 for y in range(a + 1, b)):
            y0 = b + 1
            break
    for y in range(y0, hh - 8):
        if sum(1 for x in range(lo, hi) if g.getpixel((x, y)) < 80) >= 3:
            xs = [x for yy in range(y, y + 7) for x in range(3, w - 3) if g.getpixel((x, yy)) < 80]
            return ((min(xs) + max(xs)) // 2, y) if xs else None
    return None


class LayoutError(Exception):
    """The reference program's layout (panels, tools column, main toolbar) could not be restored: the unit is
    aborted for that side instead of clicking with a guessed anchor (R-2)."""


def norm_item(s) -> str:
    """Combo/list entry for loose matching: no accelerator '&', no ellipsis/colon, no spaces, lower case
    ("PNG 32" = "PNG32", "Weiche Kante..." = "weichekante")."""
    s = clean_label(str(s or "")).replace("…", "").replace("...", "").rstrip(":")
    return re.sub(r"\s+", "", s).lower()


def match_item(items, want: str, aliases=()):
    """Best entry of `items` for `want`: exact, alias, normalised equal, then one unique normalised prefix match."""
    items = [i for i in items if i and not set(str(i)) <= set("-")]
    for w in [want, *aliases]:
        if w in items:
            return w
    nw = [norm_item(w) for w in [want, *aliases]]
    for i in items:
        if norm_item(i) in nw:
            return i
    pre = [i for i in items if any(len(n) >= 3 and (norm_item(i).startswith(n) or n.startswith(norm_item(i)) and len(norm_item(i)) >= 3) for n in nw)]
    return pre[0] if len(pre) == 1 else None


def canvas_point_js(spec: str) -> str:
    """JS expression (both programs' script API) giving "x,y" in document coordinates for a named canvas point:
    `NAME` = a point of the object (path: its first node, i.e. on the stroke; other objects: centre of the pixel
    rect), `NAME.node.N` = node N of the first contour, `NAME.center` = centre of the pixel rect."""
    name, _, rest = str(spec).partition(".")
    node = None
    m = re.match(r"^node\.(\d+)$", rest)
    if m:
        node = int(m.group(1))
    center = rest == "center"
    return ("(function(){var d = fw.getDocumentDOM(), want = %s, e = null;"
            "for (var i = 0; i < d.layers.length && !e; i++) { var es = d.layers[i].frames[d.currentFrameNum].elements;"
            "  for (var j = 0; j < es.length; j++) if (es[j].name == want) { e = es[j]; break; } }"
            "if (!e) return 'none'; var n = %s, c = %s;"
            "if (!c && e.contours && e.contours.length && e.contours[0].nodes && e.contours[0].nodes.length > (n === null ? 0 : n)) {"
            "  var p = e.contours[0].nodes[n === null ? 0 : n]; return p.x + ',' + p.y; }"
            "var r = e.pixelRect; return ((r.left + r.right) / 2) + ',' + ((r.top + r.bottom) / 2);})()"
            % (json.dumps(name, ensure_ascii=True), "null" if node is None else node, "true" if center else "false"))


def parse_xy(v, spec):
    s = str(v if not isinstance(v, dict) else v.get("result")).strip().strip("'\"")
    try:
        x, y = s.split(",")[:2]
        return float(x), float(y)
    except ValueError:
        raise Unmapped(f"canvas point '{spec}' not found (script returned {s!r})") from None


class Side:
    name = "?"

    def __init__(self, runner):
        self.r = runner
        self.values: list = []
        self.doc_size = (0, 0)
        self.view_dirty = False

    def view_reset_code(self) -> str:
        """JSF that puts the view switches (grid, guides, rulers, smart guides, edges …) back to `ui_map.json →
        view_defaults` (Firepixel's defaults of a freshly opened document) and logs what it changed. The reference
        program keeps these switches across documents, so a step that toggled one changed every later step
        (runner finding 2)."""
        parts = []
        for k, want in (self.r.ui_map.get("view_defaults") or {}).items():
            if k.startswith("_"):
                continue
            w = "true" if want else "false"
            setter = "lockGuides" if k == "LockGuides" else f"set{k}"
            if k in ("ShowSliceGuides", "ShowEdges"):  # no getter: set unconditionally
                parts.append(f"try {{ d.{setter}({w}); }} catch (e) {{}}")
            else:
                parts.append(f"try {{ if (d.get{k}() != {w}) {{ d.{setter}({w}); ch.push('{k}'); }} }} catch (e) {{}}")
        return "var ch = []; if (d) { " + " ".join(parts) + " }"

    def script_eval(self, expr):
        raise NotImplementedError

    def option_state(self, target):
        """R-13 (TW-127): checked state (True/False/None) of a stored option control of the open dialog."""
        raise NotImplementedError

    def menu_entry_state(self, path):
        """R-13 (TW-129 e): {"enabled", "checked", "text"} of the last entry of a menu path, read with the menu
        open (opened for this and closed again; nothing is executed)."""
        raise NotImplementedError

    def canvas_point(self, arg, what="canvas point", allow_outside=False):
        """R-11 (TW-119): side point (reference: screen, Firepixel: window) of a canvas argument. Besides document
        points (`canvas_doc_point`) `{"view_frac": [fx, fy]}` (also as `pos`/`at`/`canvas`) = that fraction of the
        page part shown in the document view, measured on each side (zoom/scroll independent; [0.5, 0.5] = centre
        of the page as shown). Every point is checked against the side's document view: a point outside is not
        sent, the action is 'unsupported' with the reason (run 15 xs-canvas/02: the wheel point lay outside
        Firepixel's view and the step counted as a program difference). R-12 (TW-124 d): `allow_outside` lets a
        drag end point lie outside the view (a guide dragged onto the ruler, `allow_outside_end`)."""
        a = arg
        if isinstance(a, dict) and "view_frac" not in a:
            inner = a.get("pos") or a.get("at") or a.get("canvas")
            if isinstance(inner, dict):
                a = inner
        if isinstance(a, dict) and "view_frac" in a:
            pt = self.view_frac_point(a["view_frac"])
        else:
            pt = self.doc_point_on_side(*self.canvas_doc_point(arg))
        if allow_outside:
            return pt
        view = self.view_rect_now()
        if view and not (view[0] <= pt[0] < view[2] and view[1] <= pt[1] < view[3]):
            raise Unsupported(f"{what}: point {[int(v) for v in pt]} outside the {self.name} document view "
                              f"{[int(v) for v in view]} - not sent (use view_frac or fit first)")
        return pt

    def view_frac_point(self, frac):
        view, page = self.view_and_page()
        r = list(view)
        if page:
            r = [max(view[0], page[0]), max(view[1], page[1]), min(view[2], page[2]), min(view[3], page[3])]
            if r[2] - r[0] < 2 or r[3] - r[1] < 2:
                r = list(view)
        fx, fy = (float(frac[0]), float(frac[1])) if isinstance(frac, (list, tuple)) else (float(frac), float(frac))
        return self.round_point(r[0] + (r[2] - r[0]) * fx, r[1] + (r[3] - r[1]) * fy)

    def round_point(self, x, y):
        return x, y

    def doc_point_on_side(self, x, y):
        raise NotImplementedError

    def view_rect_now(self):
        return None

    def view_and_page(self):
        """(document view rect, page rect shown or None) in side coordinates."""
        raise NotImplementedError

    def dialog_open(self) -> bool:
        raise NotImplementedError

    def canvas_doc_point(self, arg):
        """Document point of a canvas argument: [x, y], {"pos"/"at"/"canvas": ...} or an object name
        ("P1", "TX1", "P1.node.0"; run 7: the name was spread into doc_to_screen(*"TX1"))."""
        if isinstance(arg, dict):
            arg = arg.get("pos") or arg.get("at") or arg.get("canvas")
        if isinstance(arg, (list, tuple)) and len(arg) >= 2 and all(isinstance(v, (int, float)) for v in arg[:2]):
            return float(arg[0]), float(arg[1])
        if isinstance(arg, str) and arg:
            return parse_xy(self.script_eval(canvas_point_js(arg)), arg)
        raise Unmapped(f"canvas point not understood: {arg!r}")

    def grid_point(self, target):
        """R-7 (A3-10, 9 under-measured units): cell / row targets of owner-drawn panel lists without item texts -
        `{"panel_swatch": i}`, `{"color_table_cell": i}`, `{"history_row": i}`, `{"page_row": i}`, ... - from ui_map
        `grid_targets` → {form: {side: {area, origin: [dx, dy] (centre of cell 0 inside the shot area), cell: [w, h],
        cols}}}, measured in the run-11 images. None when the form has no grid for this side (or no int index)."""
        if not isinstance(target, dict):
            return None
        grids = self.r.ui_map.get("grid_targets") or {}
        ctx = target.get("grid")
        for k, v in target.items():
            g = grids.get(f"{k}@{ctx}" if ctx else k)
            e = g.get(self.name) if isinstance(g, dict) else None
            if not e or isinstance(v, bool) or not isinstance(v, int):
                continue
            r = self.area_rect(e["area"])
            if not r:
                raise Unmapped(f"grid {k}: area {e['area']} not found for {self.name}")
            cols = max(1, int(e.get("cols", 1)))
            # R-12 (TW-115): a grid whose column count follows the panel width (Firepixel's swatches: 9 px pitch,
            # columns = (client width - scrollbar) / 9) gives `cols_width` = shot width minus the grid's width
            if e.get("cols_width") is not None and float(e["cell"][0]) > 0:
                cols = max(1, int((r[2] - r[0] - float(e["cols_width"])) // float(e["cell"][0])))
            i = int(v)
            x = r[0] + float(e["origin"][0]) + float(e["cell"][0]) * (i % cols)
            y = r[1] + float(e["origin"][1]) + float(e["cell"][1]) * (i // cols)
            return int(round(x)), int(round(y))
        return None

    def area_pos(self, area, pos):
        """Offset inside a shot area: [dx, dy] or a named point from ui_map `area_points` → {area: {name: {side:
        [dx, dy] | {"frac": [fx, fy]}}}} (run 7: "spectrum_center" was added to a number)."""
        if isinstance(pos, (list, tuple)):
            return float(pos[0]), float(pos[1])
        e = ((self.r.ui_map.get("area_points") or {}).get(area) or {}).get(str(pos)) or {}
        v = e.get(self.name) if isinstance(e, dict) else None
        if v is None:
            raise Unmapped(f"named point '{pos}' of area {area} not in ui_map area_points for {self.name}")
        if isinstance(v, dict) and "frac" in v:
            r = self.area_rect(area)
            return (r[2] - r[0]) * float(v["frac"][0]), (r[3] - r[1]) * float(v["frac"][1])
        return float(v[0]), float(v[1])

    def crop_to_page(self, page, view):
        """Canvas shot = document area (page rectangle on screen, inside the view) plus ui_map
        `canvas_shot_margin` (R-2: the whole view held pasteboard, floating panels and a different view size)."""
        m = float(self.r.ui_map.get("canvas_shot_margin", 0))
        r = [max(view[0], page[0] - m), max(view[1], page[1] - m), min(view[2], page[2] + m), min(view[3], page[3] + m)]
        # R-5 (run 9, xs-canvas/02,03,11,13,14,16): a zoomed page fills the whole view, and the views differ in size
        # (reference 1420 x 732, Firepixel 1077 x 718). On an axis where the page is cut by the view, both sides take
        # the same centred span (ui_map `canvas_clip_max`, default 1000 x 640)
        # R-6 (run 10, xs-canvas/03,11,13,16, toolbox/15): a fitted or zoomed page that is not cut but larger than
        # the span (reference 1028 x 685, Firepixel 1077 x 718 or 1506 x 1004) gets the same centred span; the test
        # documents at 100 % are at most 800 x 600, so only zoomed/fitted shots change
        cw, ch = (self.r.ui_map.get("canvas_clip_max") or [1000, 640])[:2]
        for i, span in ((0, float(cw)), (1, float(ch))):
            cut = page[i] < view[i] - 1 or page[i + 2] > view[i + 2] + 1
            zoomed = page[i + 2] - page[i] > span
            if (cut or zoomed) and r[i + 2] - r[i] > span:
                c = (r[i] + r[i + 2]) / 2
                r[i] = int(round(c - span / 2))
                r[i + 2] = r[i] + int(span)
        return r if r[2] - r[0] > 4 and r[3] - r[1] > 4 else list(view)

    def ui(self, key):
        t = self.r.ui_map.get("targets", {})
        if key not in t:
            raise Unmapped(f"ui key not in ui_map.json: {key}")
        e = t[key]
        loc = e.get(self.name) if isinstance(e, dict) else None
        if not loc:
            raise Unmapped(f"ui key {key} not located for {self.name}")
        return loc

    def aliases(self, label):
        """Other names of the same menu entry (ui_map `menu_aliases`: groups, reference name first, then Firepixel's)."""
        lab = clean_label(label)
        for g in self.r.ui_map.get("menu_aliases") or []:
            if lab in g:
                return [x for x in g if x != lab]
        return []

    def combo_aliases(self, item):
        """Other spellings of a combo/list entry (ui_map `combo_aliases`: groups like the menu aliases)."""
        lab = clean_label(str(item or ""))
        for g in self.r.ui_map.get("combo_aliases") or []:
            if isinstance(g, list) and lab in g:
                return [x for x in g if x != lab]
        return []

    def panel_info(self, p):
        info = self.r.ui_map.get("panels", {}).get(norm_panel(p))
        if info is None:
            # a title given directly
            for v in self.r.ui_map.get("panels", {}).values():
                if isinstance(v, dict) and v.get("title") == p:
                    return v
            return {"fp": p if str(p).startswith("window.") else None, "title": p, "menu": f"Fenster > {p}"}
        return info


def fg_who(r: dict) -> str:
    """R-6 (Q-39): 'class (pid N, process) 'title'' of the window that held the foreground before a take-back."""
    if not r.get("was_pid") and not r.get("was_proc"):
        return str(r.get("was"))
    return f"{r.get('was')} (pid {r.get('was_pid')}, {r.get('was_proc')}) '{str(r.get('was_title') or '')[:60]}'"


class GuardedHost:
    """Host of the reference side with a foreground check before every input command (R-4). Run 8: a browser
    (Chrome_WidgetWin_1) took the foreground in the middle of a step, the step-level check of R-2 was too early
    and the host refused the input ('input refused: foreground belongs to process …'). Now: before each input
    command the host's own guard is read; a foreign foreground is taken back. When the host still refuses because
    of a foreign process, the program is brought forward and the command is sent exactly once more."""

    INPUT = {"key", "type", "click", "move", "drag", "wheel", "down", "held_move", "up", "menu"}

    def __init__(self, host, orig, log):
        self._h = host
        self._o = orig
        self._log = log
        # R-7 (run 11, xs-canvas/08…12): every take-back of the foreground is counted; a step during which the
        # count grows is repeated once (`retry`), the reference may have run it with a wrong tool or state
        self.lost = 0

    def __getattr__(self, name):
        return getattr(self._h, name)

    def _take_back(self, why):
        """R-5: one hard take-back (AttachThreadInput, Esc on a taskbar flyout, caption click as last resort)."""
        self._log(f"foreground: {why} - bringing the reference program forward")
        self.lost += 1
        try:
            r = self._h.call("focus", hard=True)
        except OrigError:
            return False
        # R-7 (run 11: no line 'taken back … via' although the focus was lost three times): logged every time
        if r.get("how") or r.get("ok") is False:
            # R-6 (Q-39): who held the foreground (process, pid, class, title) and why a caption click was skipped
            self._log(f"foreground: taken back from {fg_who(r)} via {r.get('how')} -> "
                      f"{'ok' if r.get('ok') is not False else r.get('class')}"
                      + (f"; caption covered: {'; '.join(r.get('covered') or [])}" if r.get("covered") else ""))
        if r.get("ok") is not False:
            time.sleep(0.15)
            return True
        return False

    def _foreign(self):
        """R-8: the foreign foreground (process, class) or None when the reference program owns it."""
        try:
            fg = self._h.call("fg") or {}
        except OrigError:
            return None
        if str(fg.get("guard") or "").startswith("foreground belongs"):
            return fg
        return None

    def _verify(self):
        """R-8 (run 12: 75 take-backs 'via set+stable -> ok', the next input was refused anyway - claude
        (Chrome_WidgetWin_1) re-activated itself within a second): after a take-back wait FG_SETTLE s and read the
        foreground again; input is sent only when it is still ours. Returns the foreign holder or None."""
        time.sleep(float(getattr(self._o, "FG_SETTLE", 0.4)))
        fg = self._foreign()
        if fg is not None:
            self._log(f"foreground: lost again {getattr(self._o, 'FG_SETTLE', 0.4)} s after the take-back to "
                      f"{fg.get('class')} (pid {fg.get('pid')}, {fg.get('proc')}) '{str(fg.get('title') or '')[:60]}'")
        return fg

    def _yield_to(self, fg) -> bool:
        names = [str(x).lower() for x in self._o.cfg.get("yield_to", ["claude"])]
        return str((fg or {}).get("proc") or "").lower() in names

    def _precheck(self):
        try:
            fg = self._h.call("fg") or {}
        except OrigError:
            return
        g = str(fg.get("guard") or "")
        if g.startswith("foreground belongs"):
            why = f"{fg.get('class')} (pid {fg.get('pid')}, {fg.get('proc')}) '{str(fg.get('title') or '')[:60]}' in front before input"
            for pause in (0.0,) + tuple(self._o.FG_PAUSES[:2]):
                time.sleep(pause)
                # R-8: a take-back counts only when the foreground is still ours after FG_SETTLE s
                if self._take_back(why) and self._verify() is None:
                    return
                why = "still foreign before input"

    def call(self, cmd, timeout=30, **args):
        if cmd not in self.INPUT:
            return self._h.call(cmd, timeout, **args)
        self._precheck()
        # R-5 (run 9: one retry helped in ~27 %): up to 5 retries with growing pauses (0.3/0.6/1.2/2.4/4.8 s),
        # each after a hard take-back of the foreground.
        # R-8 (run 12: all 75 retries failed, the foreign program took the foreground back each time): the input
        # is sent again only after a verified take-back (`_verify`); once the foreground is lost again - at the
        # first loss counted twice for a process in `orig.yield_to` (default claude) - the runner yields with long
        # pauses (FG_YIELD, 165 s in all) instead of fighting, then gives up with the holder in the error.
        pauses = list(self._o.FG_PAUSES)
        yields = list(getattr(self._o, "FG_YIELD", ()))
        relost, n = 0, 0
        while True:
            try:
                return self._h.call(cmd, timeout, **args)
            except OrigError as e:
                if "foreground belongs to process" not in str(e):
                    raise
                last = e
            if cmd == "menu":
                try:
                    self._h.call("close_menus")
                except OrigError:
                    pass
            while True:
                if relost >= 2 and yields:
                    pause, phase = yields.pop(0), "yield"
                elif pauses:
                    pause, phase = pauses.pop(0), "retry"
                else:
                    raise OrigError(f"{last} (after {n} take-backs, lost again {relost} times)")
                n += 1
                if phase == "yield":
                    self._log(f"foreground: the other program keeps taking it - waiting {pause} s before the next take-back")
                time.sleep(pause)
                if not self._take_back(f"input refused ({cmd}), {phase} {n} after {pause} s"):
                    relost += 2 if self._yield_to(self._foreign()) else 1
                    continue
                fg = self._verify()
                if fg is None:
                    break
                relost += 2 if self._yield_to(fg) else 1

    def list(self, cmd, timeout=30, **args):
        r = self.call(cmd, timeout, **args)
        r = r if isinstance(r, list) else ([] if r is None else [r])
        return [x for x in r if x != {}]


class OrigSide(Side):
    name = "orig"

    def __init__(self, runner, orig: Original):
        super().__init__(runner)
        self.o = orig
        self.h = GuardedHost(orig.host, orig, runner.log)

    # ------------------------------------------------------------ state
    def prepare(self):
        if not self.o.running():
            self.o.start()
        m = self.h.call("main")
        x, y, w, hh = self.o.cfg.get("rect", [0, 0, 1600, 1000])
        if m.get("maximized") or m["rect"] != [x, y, x + w, y + hh]:
            self.o.place()
        self.o.focus()

    def reset_light(self):
        try:
            self.ensure_foreground()  # R-5: the reset clicks (pointer tool, panels) need the foreground too
        except OrigError as e:
            self.r.log(f"reset: {e}")
        self.release_held()
        try:
            self.h.call("close_menus")
        except OrigError:
            pass
        self.h.call("dialog_close", mode="cancel")
        time.sleep(0.2)
        # closing a dialog may raise a message box (e.g. "internal error" after cancelling the brush stroke
        # editor): repeat until no dialog is left
        for _ in range(4):
            try:
                if not self.h.list("dialogs"):
                    break
            except OrigError:
                break
            time.sleep(0.5)
            self.h.call("dialog_close", mode="cancel")
        try:
            self.close_floating(only_over_view=True)
        except OrigError:
            pass
        if self.view_dirty:
            # the last step switched a view setting (grid, guides, rulers …): back to the defaults. Only with an
            # open document (R-4: 51 x 'view reset: JSF: ERROR' at unit start, no document there); the next
            # open_testfile resets the switches with its own script anyway
            try:
                if self.view_or_none() is not None:
                    self.log_view(self.o.jsf("var d = fw.getDocumentDOM(); " + self.view_reset_code() + ' fp_log("VIEW " + ch.join(","));'))
                    self.view_dirty = False
            except OrigError as e:
                self.r.log(f"view reset: {e}")
        if getattr(self, "tab_pressed", False):
            # Tab on the canvas hides every panel (xs-shortcuts/07): show them again before the next step (R-4)
            self.tab_pressed = False
            try:
                if self.ensure_layout():
                    self.tools_anchor = None
            except OrigError as e:
                self.r.log(f"layout after Tab: {e}")
        self.import_pending = False
        self.doc_maybe_changed = False
        # the properties panel (inspector) is the program's default layout: a step may have switched its dock
        # group to another tab (e.g. Fenster > Symboleigenschaften) or closed it
        try:
            self.ensure_panel("properties")
        except OrigError as e:
            self.r.log(f"properties panel: {e}")

    # tools column of the reference program: `tools_rel` = offset from the anchor, i.e. the centre/top of the first
    # group heading of the docked tools column (R-1: the layout moved, fixed client coordinates broke)
    def menu_entries(self, path):
        """Entries of a menu of the menu bar (opened, read, closed again): {clean text: checked}."""
        bar = self.r.ui_map["menu_bar"]["orig"]
        self.h.call("menu", path=list(path), final="open", bar=bar, timeout=40)
        try:
            time.sleep(0.25)
            ms = self.h.call("menus").get("menus") or []
            items = (ms[-1].get("items") or []) if ms else []
            return {clean_label(i.get("text") or ""): bool(i.get("checked")) for i in items
                    if not i.get("separator") and i.get("text")}
        finally:
            self.h.call("close_menus")

    def view_or_none(self):
        """Client rectangle of the document view, or None when no document is open (unit start): the view window
        only exists with a document, so a missing view is not a layout fault."""
        try:
            return self.h.call("view")
        except OrigError as e:
            if "view window not found" in str(e):
                return None
            raise

    def tools_visible(self):
        c, v = self.client(), self.view_or_none()
        if v is None:
            return self.tools_rect(c, None) is not None and self.tools_anchor_probe(c) is not None
        return v[0] - c[0] >= 50

    def tools_rect(self, c, v):
        """Screen rectangle of the docked tools column: left client edge up to the view, or (without a document)
        a fixed 140 px strip."""
        right = v[0] if v else c[0] + 140
        rect = [c[0], c[1], right, min(c[3], c[1] + 260)]
        return rect if rect[2] - rect[0] >= 50 else None

    def tools_anchor_probe(self, c, v=None):
        rect = self.tools_rect(c, v)
        if not rect:
            return None
        try:
            tmp = Path(self.r.cfg["tmp_dir"]) / "orig_tools.png"
            self.grab(rect, tmp)
            with Image.open(tmp) as im:
                a = tools_anchor_in(im.convert("L"))
            return (rect[0] + a[0], rect[1] + a[1]) if a else None
        except Exception as e:  # noqa: BLE001
            self.r.log(f"tools anchor: {e}")
            return None

    def ensure_layout(self):
        """Default layout of the reference program (R-2): panels shown (the 'hide panels' switch off), main toolbar
        docked, tools column visible. A unit like menu-commands-window-help/06-08 may leave panels, toolbar and
        tools column hidden; every later click on the tools column then missed (run 7: 70 flyout errors).
        Raises LayoutError when the tools column cannot be shown."""
        lay = self.r.ui_map.get("orig_layout") or {}
        bar = self.r.ui_map["menu_bar"]["orig"]
        changed = []
        hide = lay.get("hide_panels")
        if hide:
            parts = split_path(hide)
            try:
                ent = self.menu_entries(parts[:-1])
                leaf = clean_label(parts[-1])
                shows = [clean_label(x) for x in lay.get("show_panels_labels") or []]
                show = next((k for k in ent if k in shows), None)
                if show or ent.get(leaf):
                    self.h.call("menu", path=parts[:-1] + [show or leaf], final="invoke", bar=bar, timeout=40)
                    time.sleep(0.6)
                    changed.append("panels shown")
            except OrigError as e:
                self.r.log(f"layout: hide-panels state not readable: {e}")
        for tb in lay.get("toolbars_on") or []:
            parts = split_path(tb)
            try:
                ent = self.menu_entries(parts[:-1])
                leaf = clean_label(parts[-1])
                if leaf in ent and not ent[leaf]:
                    self.h.call("menu", path=parts, final="invoke", bar=bar, timeout=40)
                    time.sleep(0.5)
                    changed.append(f"toolbar on: {parts[-1]}")
            except OrigError as e:
                self.r.log(f"layout: toolbar {tb}: {e}")
        if not self.tools_visible():
            info = self.panel_info("tools")
            self.h.call("menu", path=split_path(info["menu"]), final="invoke", bar=bar, timeout=40)
            for _ in range(10):
                time.sleep(0.3)
                if self.tools_visible():
                    changed.append("tools column shown")
                    break
            else:
                raise LayoutError("tools column not visible (panels/tools menu entries did not show it)")
        if changed:
            self.page = None
            self.r.log(f"orig layout restored: {', '.join(changed)}")
        return changed

    def ensure_tools(self):
        """The docked tools column must be visible (see ensure_layout); the anchor is measured again."""
        self.tools_anchor = None
        return bool(self.ensure_layout())

    def tools_origin(self):
        """Screen point of the tools anchor, measured from the column image: first row with dark text in the middle
        of the column (the heading of the first group), centre of that heading's text. No silent fallback (R-2):
        if the column is not found, the layout is restored and the anchor measured again; otherwise LayoutError."""
        a = getattr(self, "tools_anchor", None)
        if a:
            return a
        anchor = None
        for attempt in range(2):
            c, v = self.client(), self.view_or_none()
            anchor = self.tools_anchor_probe(c, v)
            if anchor or attempt:
                break
            self.r.log("tools anchor not found - restoring the layout and measuring again")
            self.ensure_layout()
        if not anchor:
            raise LayoutError("tools column anchor not found after restoring the layout")
        if v is None:
            # measured without a document (unit start): the strip may include the main toolbar, so the anchor is
            # not cached - it is measured again as soon as a document view exists (run 8 probe: (93, 20) vs (35, 101))
            self.r.log(f"orig tools anchor (provisional, no document) {anchor}")
            return anchor
        self.tools_anchor = anchor
        self.r.log(f"orig tools anchor {anchor}")
        return anchor

    def tools_point(self, rel):
        ax, ay = self.tools_origin()
        return int(ax + rel[0]), int(ay + rel[1])

    def align_to_canvas_state(self):
        """Stored state of the align panel's 'to canvas' switch (the panel keeps it in a small local store file;
        path and key in runner.config.json `orig.align_state_file` / `orig.align_state_key`). None = unknown."""
        f = self.r.cfg["orig"].get("align_state_file")
        if not f or not Path(f).exists():
            return None
        key = str(self.r.cfg["orig"].get("align_state_key", "toCanvas")).encode("ascii")
        b = Path(f).read_bytes()
        i = b.find(key)
        if i < 0 or i + len(key) >= len(b):
            return None
        return {0x03: True, 0x02: False}.get(b[i + len(key)])

    def align_to_canvas_off(self):
        """Unit start (R-2): the align panel's 'to canvas' switch off. The program keeps it across sessions; in run
        7 every align step aligned to the canvas. Switched with the panel's own button, checked in the store file."""
        if self.align_to_canvas_state() is not True:
            return False
        lab = (self.r.ui_map.get("orig_labels") or {}).get("Zur Leinwand ausrichten")
        if not lab:
            raise OrigError("align 'to canvas' button not in ui_map orig_labels")
        self.ensure_panel(lab["panel"])
        pt, _ = self.locate(lab)
        self.o.focus()
        self.click_at(pt)
        for _ in range(10):
            time.sleep(0.3)
            if self.align_to_canvas_state() is False:
                self.r.log("align 'to canvas' switched off")
                return True
        raise OrigError("align 'to canvas' still on after clicking its button")

    def ensure_foreground(self):
        """Before every step (R-2): no other program (a browser) in front; the reference program is brought forward
        (up to 3 tries). Logs who held the foreground."""
        try:
            fg = self.h.call("fg")
            own = self.o.pid()
        except OrigError:
            return
        if not isinstance(fg, dict) or fg.get("pid") == own:
            return
        self.r.log(f"foreground held by {fg.get('class')} (pid {fg.get('pid')}, {fg.get('proc')}) "
                   f"'{str(fg.get('title') or '')[:60]}' - bringing the reference program forward")
        # R-11 (run 15, xs-shortcuts/10_nudge_point at 23:58:30: taken back before the step, the step's first canvas
        # click then selected nothing and the step failed without a retry - the take-back happened before the step's
        # own count started): flagged for step_side, an error in that step is played again
        self.fg_taken = True
        try:
            self.o.focus(log=self.r.log)  # R-5: up to 6 hard tries with growing pauses
        except OrigError as e:
            raise OrigError(f"foreground stays with {fg.get('class')} (pid {fg.get('pid')}): {e}") from e
        # the same settle time as after a take-back during input (GuardedHost._verify)
        time.sleep(float(getattr(self.o, "FG_SETTLE", 0.4)))
        self.r.log("foreground: reference program in front again")

    def log_foreground(self, where):
        """R-5: one protocol line at unit start: which window is in front."""
        try:
            fg = self.h.call("fg") or {}
            own = self.o.pid()
        except OrigError as e:
            self.r.log(f"{where}: foreground unknown ({e})")
            return
        who = "reference program" if fg.get("pid") == own else "FOREIGN"
        self.r.log(f"{where}: foreground {fg.get('class')} (pid {fg.get('pid')}, {fg.get('proc')}, {who}) "
                   f"'{str(fg.get('title') or '')[:60]}'")

    def reset_unit(self):
        """Unit start (A3-5 'states kept from earlier units', R-2): default layout (panels, main toolbar, tools column
        with a measured anchor), align 'to canvas' off, pointer tool, view switches back to the defaults with the
        next opened document. LayoutError aborts the unit for this side."""
        try:
            self.ensure_tools()
            self.tools_origin()
        except OrigError as e:
            raise LayoutError(f"layout not restorable: {e}") from e
        try:
            self.align_to_canvas_off()
        except OrigError as e:
            self.r.log(f"align to canvas: {e}")
        # the pointer tool is selected once a document view exists (open_testfile): without a document the tools
        # anchor is only provisional and a click could hit the main toolbar instead
        self.pending_pointer = True
        self.pending_colors = True
        # no document is open here: the view switches are reset by the next open_testfile (R-4: not before)
        self.view_dirty = False

    def select_pointer_if_pending(self):
        if not getattr(self, "pending_pointer", False):
            return
        self.pending_pointer = False
        ptr = (self.r.ui_map.get("tools") or {}).get("tool.pointer")
        if isinstance(ptr, dict) and "tools_rel" in ptr:
            try:
                self.o.focus()
                self.click_at(self.tools_point(ptr["tools_rel"]))
                time.sleep(0.2)
            except OrigError as e:
                self.r.log(f"pointer tool: {e}")

    def log_view(self, lines):
        ch = [ln[5:] for ln in lines or [] if ln.startswith("VIEW ") and ln[5:].strip()]
        if ch:
            self.r.log(f"view switches reset: {ch[-1]}")

    PROTECTED_PANELS = ("properties",)

    def panel(self, p):
        """Panel window info (host `panel`) by panel id or title. A panel that is not visible (closed, or hidden
        behind another tab of its dock group) is shown with its `Fenster > …` menu entry first - only then, so the
        menu entry never toggles a visible panel off."""
        info = self.panel_info(p)
        title = info.get("title") or p
        try:
            return self.h.call("panel", name=title)
        except OrigError as e:
            if "not visible" not in str(e) or not info.get("menu"):
                raise
        self.h.call("menu", path=split_path(info["menu"]), final="invoke", bar=self.r.ui_map["menu_bar"]["orig"], timeout=40)
        for _ in range(10):
            time.sleep(0.3)
            try:
                return self.h.call("panel", name=title)
            except OrigError:
                pass
        raise OrigError(f"panel not visible after {info['menu']}: {title}")

    def ensure_panel(self, p):
        return self.panel(p)

    def close_floating(self, only_over_view=False):
        """Floating panel groups (top-level dock windows of the program, e.g. Info/Farbmischer/URL) that cover the
        document view are closed with their own close button (WM_CLOSE as fallback), so canvas clicks and canvas
        shots are not blocked. Docked panels are child windows and are never touched."""
        cls = self.r.cfg["orig"].get("floating_dock_class", "OWL.Dock")
        main = self.h.call("main")["hwnd"]
        try:
            v = self.h.call("view") if only_over_view else None
        except OrigError:
            v = None
        n = 0
        for w in self.h.list("windows"):
            r = w["rect"]
            if w["hwnd"] == main or w["class"] != cls or (r[2] - r[0]) < 20:
                continue
            if v and (r[2] <= v[0] or r[0] >= v[2] or r[3] <= v[1] or r[1] >= v[3]):
                continue
            # never close a floating group that holds the properties panel
            keep = {(self.panel_info(k).get("title") or k) for k in self.PROTECTED_PANELS}
            try:
                if any(k.get("title") in keep for k in self.h.list("kids", hwnd=w["hwnd"])):
                    continue
            except OrigError:
                pass
            try:
                self.click_at((r[2] - 9, r[1] + 7))
                time.sleep(0.4)
            except OrigError:
                pass
            if any(x["hwnd"] == w["hwnd"] for x in self.h.list("windows")):
                self.h.call("close_window", hwnd=w["hwnd"])
            n += 1
        return n

    def open_testfile(self, path: Path):
        # R-5: preparation steps are secured like inputs (run 9: 107 preparations failed at a foreign window)
        try:
            self.ensure_foreground()
        except OrigError as e:
            self.r.log(f"open test file: {e}")
        self.reset_light()
        self.o.open_doc(path)
        # unit start (R-4): stroke and fill colour of the tools back to the program's defaults, like a freshly
        # started Firepixel (run 8: auto shapes without stroke and with fill #808080 left by earlier runs)
        colors = ""
        if getattr(self, "pending_colors", False):
            self.pending_colors = False
            # R-6 (A3-9: not confirmed in run 10): the result is logged in any case, with the document's path
            # attributes before and after (fill, stroke colour, stroke set or not)
            pa = ('(function () { try { var p = d.pathAttributes; return "fill " + p.fillColor + " stroke " + p.brushColor '
                  '+ " brush " + (p.brush ? "set" : "none") + " fillset " + (p.fill ? "set" : "none"); } catch (e) { return "? " + e; } })()')
            colors = (f'fp_log("COLORS before " + {pa}); '
                      'try { d.selectNone(); d.setDefaultBrushAndFillColors(); '
                      f'fp_log("COLORS default"); }} catch (e) {{ fp_log("COLORS " + e); }} fp_log("COLORS after " + {pa}); ')
        # the view switches go back to their defaults with the same script (no extra script run per step)
        lines = self.o.jsf('var d = fw.getDocumentDOM(); fp_log("RET " + d.width + "," + d.height); ' + colors
                           + self.view_reset_code() + ' fp_log("VIEW " + ch.join(","));')
        col = {}
        for ln in lines or []:
            if ln.startswith("COLORS "):
                self.r.log(f"default colours: {ln[7:]}")
                k, _, v = ln[7:].partition(" ")
                col[k] = v
        if colors and col.get("before") is not None and col.get("before") == col.get("after"):
            # R-7 (run 10/11: 'setDefaultBrushAndFillColors' changes nothing): the tools' own button for the
            # default colours (the same as the key D) with nothing selected, then read again and logged
            try:
                pt = self.resolve("toolbox.default_colors")[0]
                self.click_at(pt)
                time.sleep(0.3)
                after = self.o.jsf(f'var d = fw.getDocumentDOM(); fp_log("COLORS " + {pa});')
                got = ([ln[7:] for ln in after or [] if ln.startswith("COLORS ")] or ["?"])[-1]
                self.r.log(f"default colours: script without effect - tools button 'default colours' at {pt}: {got}"
                           f"{' (unchanged)' if got == col.get('after') else ''}")
            except Exception as e:  # noqa: BLE001
                self.r.log(f"default colours: tools button failed: {e}")
        self.view_dirty = False
        self.view_zoom = None
        self.log_view(lines)
        ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
        if ret:
            w, h = ret[-1].split(",")
            self.doc_size = (float(w), float(h))
        self.o.focus()
        self.select_pointer_if_pending()
        self.fitted = self.fit_if_needed()
        self.page = None
        self.calibrate()

    def close_doc(self, path: Path):
        self.o.close_doc(path)

    # ------------------------------------------------------------ geometry
    def client(self):
        return self.h.call("main")["client"]

    def calibrate(self):
        """Page rectangle on screen: bounding box of everything that differs from the pasteboard colour
        in the document view (works at any zoom as long as the page is fully visible)."""
        v = self.h.call("view")
        tmp = Path(self.r.cfg["tmp_dir"]) / "orig_view.png"
        self.grab(v, tmp)
        with Image.open(tmp) as im:
            im = im.convert("RGB")
            bg = im.getpixel((3, 3))
            # R-10 (run 14 xs-canvas: the reference was fitted to 43 %, but a floating panel group over the view
            # reached the view border, so the page counted as cut and the centre rule with zoom 1.0 was used):
            # other top-level windows of the program over the view are painted with the pasteboard colour first
            for r in self.view_overlaps(v):
                box = (max(0, int(r[0] - v[0])), max(0, int(r[1] - v[1])), min(im.width, int(r[2] - v[0])), min(im.height, int(r[3] - v[1])))
                if box[2] > box[0] and box[3] > box[1]:
                    im.paste(bg, box)
            d = ImageChops.difference(im, Image.new("RGB", im.size, bg)).convert("L").point(lambda q: 255 if q > 10 else 0)
            bb = d.getbbox()
        w, h = self.doc_size
        # R-5: the centre rule below uses the zoom read by canvas_shot_rect (zoom steps cut the page)
        z = float(getattr(self, "view_zoom", None) or 1.0)
        self.page_cut = True  # R-8: the page is not fully visible (no box, or the box touches the view border)
        if bb and w:
            page = [v[0] + bb[0], v[1] + bb[1], v[0] + bb[2], v[1] + bb[3]]
            # a page touching the view border is cut off (scrolled) - then the centre rule below is used
            cut = bb[0] <= 1 or bb[1] <= 1 or bb[2] >= im.width - 1 or bb[3] >= im.height - 1
            # R-9 (run 13, 32 failed 'fit all' tries): 'Alles anpassen' fits the page exactly into the view, so the
            # page touches both borders of one axis. That is a fully visible page, not a scrolled one, when the other
            # axis keeps a margin on both sides and the box has the document's aspect ratio.
            if cut and h:
                bw, bh = bb[2] - bb[0], bb[3] - bb[1]
                span_x = bb[0] <= 1 and bb[2] >= im.width - 1
                span_y = bb[1] <= 1 and bb[3] >= im.height - 1
                free_x = bb[0] > 1 and bb[2] < im.width - 1
                free_y = bb[1] > 1 and bb[3] < im.height - 1
                if bh and ((span_x and free_y) or (span_y and free_x)) and abs(bw / bh - w / h) <= 0.03 * (w / h):
                    cut = False
            if not cut:
                self.page_cut = False
                self.page = page
                return page
        ox = (v[0] + v[2]) / 2 - w * z / 2
        oy = (v[1] + v[3]) / 2 - h * z / 2
        self.page = [ox, oy, ox + w * z, oy + h * z]
        return self.page

    def view_overlaps(self, v):
        """R-10: rectangles of visible top-level windows of the reference (floating panel groups, tooltips) that
        overlap the document view `v`; a window that covers the whole view is not listed (nothing left to measure)."""
        try:
            main = self.h.call("main")["hwnd"]
            out = []
            for w in self.h.list("windows"):
                r = w.get("rect") or []
                if w.get("hwnd") == main or len(r) != 4 or r[2] - r[0] < 4 or r[3] - r[1] < 4:
                    continue
                if r[2] <= v[0] or r[0] >= v[2] or r[3] <= v[1] or r[1] >= v[3]:
                    continue
                if r[0] <= v[0] and r[1] <= v[1] and r[2] >= v[2] and r[3] >= v[3]:
                    continue
                out.append(r)
            return out
        except Exception:  # noqa: BLE001
            return []

    def fit_if_needed(self):
        """Large documents (e.g. T10): 'fit all' in both programs so that every document point is reachable.
        R-8 (Q-48, run 12 xs-canvas/12: the reference stayed at zoom 1, page at -420,-317, the drag ran outside
        the window and still counted as 'ok'): the menu command `orig.fit_menu` (default Ansicht > Alles anpassen,
        re 10 cmd 35065) first, the key `orig.fit_all_key` as second way; after each the page is measured again and
        must be fully visible. The fitted zoom is kept (`fit_zoom`) - the Firepixel side takes the same zoom."""
        self.fit_zoom = None
        v = self.h.call("view")
        w, h = self.doc_size
        if not (w > (v[2] - v[0]) or h > (v[3] - v[1])):
            return False
        oc = self.r.cfg["orig"]
        ways = [("menu", oc.get("fit_menu", ["Ansicht", "Alles anpassen"])), ("key", oc.get("fit_all_key", "ctrl+alt+0")),
                ("key", oc.get("fit_all_key2", "ctrl+0"))]
        tried = []
        for how, what in ways:
            try:
                if how == "menu":
                    self.h.call("menu", path=split_path(what), final="auto", bar=self.r.ui_map["menu_bar"]["orig"], timeout=40)
                else:
                    self.key(what)
            except Exception as e:  # noqa: BLE001
                tried.append(f"{how} {what}: {str(e)[:80]}")
                continue
            # R-9: the view repaints late after the command - measure up to three times
            for _ in range(3):
                time.sleep(0.6)
                self.page = None
                page = self.calibrate()
                if not self.page_cut:
                    break
            z = round((page[2] - page[0]) / float(w), 5) if w else None
            tried.append(f"{how} {what}: zoom {z}{' (page cut)' if self.page_cut else ''}"
                         f" page {[int(x) for x in page]} view {[int(x) for x in self.h.call('view')]}")
            if not self.page_cut:
                self.fit_zoom = z
                self.view_zoom = z  # the centre rule of calibrate() uses this zoom from now on
                break
        self.r.log(f"orig fit: {'; '.join(tried)}")
        return True

    def check_drag_points(self, pts):
        """R-8 (Q-48): a canvas drag point outside the reference's document view (negative screen points in
        run 12 xs-canvas/08,10,12) makes the step an error instead of a silent 'ok' without effect."""
        v = self.h.call("view")
        bad = [list(map(int, p)) for p in pts if not (v[0] <= p[0] < v[2] and v[1] <= p[1] < v[3])]
        if bad:
            raise OrigError(f"drag point outside the document view {v}: {bad} (zoom {getattr(self, 'fit_zoom', None)},"
                            f" page {[int(x) for x in (getattr(self, 'page', None) or [])]})")

    def check_on_window(self, pt):
        """R-12 (TW-124 d): a drag end allowed outside the document view must still lie in the program's window
        (a ruler, not the desktop or another program)."""
        c = self.client()
        if not (c[0] <= pt[0] < c[2] and c[1] <= pt[1] < c[3]):
            raise Unsupported(f"drag end {[int(v) for v in pt]} outside the reference window {[int(v) for v in c]}")

    def doc_point_on_side(self, x, y):
        return self.doc_to_screen(x, y)

    def round_point(self, x, y):
        return int(round(x)), int(round(y))

    def view_rect_now(self):
        try:
            return self.h.call("view")
        except OrigError:
            return None

    def view_and_page(self):
        """R-11: document view and the page measured now (None when it is cut - then the whole view is the page)."""
        v = self.h.call("view")
        page = None
        if self.doc_size[0]:
            self.page = None
            try:
                p = self.calibrate()
                page = None if self.page_cut else p
            except Exception:  # noqa: BLE001
                page = None
        return v, page

    def dialog_open(self) -> bool:
        try:
            return bool(self.h.list("dialogs"))
        except OrigError:
            return False

    def drag_record(self, fr, to, mods):
        """R-7 (TW-98, Q-44, import offset): drag points of the reference like the Firepixel side (window client
        relative) plus screen and document points."""
        d = {"from_scr": [int(fr[0]), int(fr[1])], "to_scr": [int(to[0]), int(to[1])], "mods": [str(m).lower() for m in mods]}
        try:
            c = self.client()
            d["from_win"] = [fr[0] - c[0], fr[1] - c[1]]
            d["to_win"] = [to[0] - c[0], to[1] - c[1]]
        except Exception:  # noqa: BLE001
            pass
        page, (w, h) = getattr(self, "page", None), self.doc_size
        if page and w and h and page[2] > page[0]:
            sx, sy = (page[2] - page[0]) / w, (page[3] - page[1]) / h
            d["from_doc"] = [round((fr[0] - page[0]) / sx, 2), round((fr[1] - page[1]) / sy, 2)]
            d["to_doc"] = [round((to[0] - page[0]) / sx, 2), round((to[1] - page[1]) / sy, 2)]
            d["zoom"] = round(sx, 5)
        return d

    TOOL_STATE_JS = ('(function () { var r = "tool=" + fw.activeTool; try { var p = fw.getDocumentDOM().pathAttributes; '
                     'r += ";fill=" + p.fillColor + ";stroke=" + p.brushColor + ";fill_kind=" + (p.fill ? (p.fill.category || "set") : "none")'
                     ' + ";brush_kind=" + (p.brush ? (p.brush.category || "set") : "none"); } catch (e) { r += ";error=" + e; } return r; })()')

    def tool_state(self):
        """R-9 (TW-107, Q-50): active tool, fill and stroke colour and fill/stroke kind of the reference."""
        try:
            got = str(self.jsf_value(self.TOOL_STATE_JS) or "")
            return {k: v for k, v in (p.split("=", 1) for p in got.split(";") if "=" in p)}
        except Exception as e:  # noqa: BLE001
            return {"error": str(e)[:80]}

    ZOOM_PROPS = ("zoom", "zoomFactor", "currentZoom", "magnification", "viewZoom", "zoomLevel")

    def log_zoom(self):
        """R-7 (TW-98/RE-9, Q-45): zoom of the reference per canvas step. `fw.getDocumentDOM().zoom` is undefined
        in the reference, so: every candidate property by JSF, the zoom in the main window / document tab title
        ('@ 100%'), and the zoom measured from the page on screen (page width / document width).
        R-13 (TW-129 a, run 17 dialogs-d31-d40/09, 14, 15, 16): not while a dialog is open - the JSF hand-over
        cancels open dialogs (origctl.jsf), so Feather / Save Bitmap Selection were gone before the dialog shot
        ('no dialog open'); the zoom is then logged as skipped."""
        if self.dialog_open():
            z = {"skipped": "dialog open (a script would cancel it)"}
            self.values.append({"view_zoom": z})
            self.r.log(f"orig zoom: {z}")
            return z
        z = {}
        try:
            props = " + ".join(f'"{k}=" + d.{k} + ";"' for k in self.ZOOM_PROPS)
            lines = self.o.jsf(f'var d = fw.getDocumentDOM(); fp_log("ZOOMS " + {props});')
            got = [ln[6:] for ln in lines or [] if ln.startswith("ZOOMS ")]
            if got:
                z["jsf"] = {k: v for k, v in (p.split("=", 1) for p in got[-1].split(";") if "=" in p)
                            if v not in ("undefined", "null", "")}
        except Exception as e:  # noqa: BLE001
            z["jsf_error"] = str(e)[:120]
        try:
            texts = [str(self.h.call("main").get("title") or "")]
            for w in self.h.list("windows"):
                texts.append(str(w.get("title") or ""))
            hit = next((m for t in texts for m in [re.search(r"@\s*([\d.,]+)\s*%", t)] if m), None)
            if hit:
                z["title"] = float(hit.group(1).replace(",", "."))
        except Exception as e:  # noqa: BLE001
            z["title_error"] = str(e)[:120]
        try:
            if self.doc_size[0]:
                self.page = None
                page = self.calibrate()
                z["measured"] = round((page[2] - page[0]) / float(self.doc_size[0]), 5)
                z["page"] = [int(v) for v in page]
        except Exception as e:  # noqa: BLE001
            z["measured_error"] = str(e)[:120]
        self.values.append({"view_zoom": z})
        self.r.log(f"orig zoom: {z}")
        return z

    def popup_edit_record(self, what, extra=None):
        """R-8 (Q-46): text and keyboard focus of the popup edit (WM_GETTEXT, GetGUIThreadInfo) in the values."""
        pe = self.popup_edit
        rec = {"what": what}
        if extra is not None:
            rec["arg"] = extra
        try:
            rec["text"] = self.h.call("get_text", hwnd=pe["hwnd"])
        except OrigError as e:
            rec["text_error"] = str(e)[:100]
        try:
            fo = self.h.call("focus_of", hwnd=pe["hwnd"]) or {}
            rec["has_focus"] = int(fo.get("focus") or 0) == int(pe["hwnd"])
            rec["focus_class"] = fo.get("focus_class")
        except OrigError as e:
            rec["focus_error"] = str(e)[:100]
        self.values.append({"popup_edit": rec})
        self.r.log(f"orig popup edit {what}: {rec}")
        return rec

    def popup_edit_type(self, text):
        """R-8 (Q-46, color-popup/05: in the reference neither the valid nor the invalid hex value arrived): after
        typing, the edit's text is read back; when the typed text is not in it (the keys went elsewhere - the field
        has no keyboard focus when the popup opens, M9), the text is set with WM_SETTEXT (the edit reports EN_CHANGE
        to the popup) and read again. The following Enter/Tab is recorded too."""
        rec = self.popup_edit_record("typed", text)
        if text.lower().lstrip("#") not in str(rec.get("text") or "").lower():
            try:
                self.h.call("set_text", hwnd=self.popup_edit["hwnd"], text=text)
                time.sleep(0.2)
                self.popup_edit_record("set_text", text)
            except OrigError as e:
                self.r.log(f"orig popup edit: WM_SETTEXT failed: {e}")

    def active_tool(self):
        """R-7: the reference's active tool (JSF `fw.activeTool`), for the log after a focus loss."""
        try:
            lines = self.o.jsf('fp_log("TOOL " + fw.activeTool);')
            return ([ln[5:] for ln in lines or [] if ln.startswith("TOOL ")] or ["?"])[-1]
        except Exception as e:  # noqa: BLE001
            return f"? ({str(e)[:60]})"

    def doc_to_screen(self, x, y):
        w, h = self.doc_size
        if not w:
            raise OrigError("document size unknown")
        if getattr(self, "page", None) is None:
            self.calibrate()
        l, t, r, b = self.page
        dx, dy = self.r.cfg["orig"].get("canvas_offset", [0, 0])
        return int(round(l + (r - l) * x / w + dx)), int(round(t + (b - t) * y / h + dy))

    def resolve(self, target):
        """target -> screen point (x, y) [, hwnd info]."""
        if isinstance(target, dict):
            g = self.grid_point(target)
            if g:
                return g, self.win_at(g)
            if "canvas" in target:
                return self.canvas_point(target["canvas"], allow_outside=bool(target.get("allow_outside"))), None
            if "window_rel" in target:
                c = self.client()
                return (c[0] + target["window_rel"][0], c[1] + target["window_rel"][1]), None
            if "tools_rel" in target:
                return self.tools_point(target["tools_rel"]), None
            if "area" in target and "pos" in target:
                r = self.area_rect(target["area"])
                dx, dy = self.area_pos(target["area"], target["pos"])
                return (int(r[0] + dx), int(r[1] + dy)), None
            if "target" in target:
                return self.resolve(target["target"])
            if "field_arrow" in target:
                # popup slider arrow right next to the i-th edit field of the open dialog (text size, simplify ...)
                ed = self.dialog_child("Edit", int(target["field_arrow"]))
                d = max(self.h.list("dialogs"), key=lambda d: (d["rect"][2] - d["rect"][0]) * (d["rect"][3] - d["rect"][1]))
                er = ed["rect"]
                for k in self.h.list("kids", hwnd=d["hwnd"]):
                    r = k["rect"]
                    if k["class"] == "Button" and not k.get("title") and abs(r[0] - er[2]) <= 6 and r[1] < er[3] and r[3] > er[1]:
                        return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), None
                return (er[2] + 8, (er[1] + er[3]) // 2), None
            if "dialog_field" in target:
                ch = self.h.call("find_child", scope="dialog", title=None, **{"class": "Edit"}, index=int(target["dialog_field"]))
                r = ch["rect"]
                return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
            if "button" in target or "text" in target:
                return self.label_point(target.get("button") or target.get("text"))
            if "menu_item" in target:
                return self.menu_item_point(target["menu_item"])
            if "handle" in target:
                return self.handle_point(target["handle"]), None
            if "preview_pane" in target:
                ch = self.h.call("find_child", scope="dialog", title=None,
                                 **{"class": self.r.ui_map.get("orig_preview_pane_class", "Afx:00400000:b:00000000:00900010:00000000")},
                                 index=int(target["preview_pane"]))
                r = ch["rect"]
                return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
            if "dialog_rel" in target or ("rel" in target and "panel" in target) or "child" in target or "dialog_child" in target:
                return self.locate(target)
            if "pref_category" in target:
                # preferences dialog: category list (SysListView32, items not readable cross-process); row order from
                # ui_map `orig_pref_categories` (script names and their aliases), row height measured 17 px
                cats = self.r.ui_map.get("orig_pref_categories") or {}
                i = cats.get(clean_label(str(target["pref_category"])))
                if i is not None:
                    lv = self.h.call("find_child", scope="dialog", title=None, **{"class": "SysListView32"}, index=0)
                    r = lv["rect"]
                    return (r[0] + 40, r[1] + 10 + 17 * int(i)), None
            for k in ("pref_category", "list_row", "tree_item", "history_row"):
                if k in target and isinstance(target[k], str):
                    # standard list/tree controls (preferences categories, library lists) expose their items to UI
                    # Automation even though the program's own panels do not: look the item up by name in the dialogs
                    # first, then in the main window
                    for scope in ("tops", "main"):
                        for typ in ("ListItem", "TreeItem", "DataItem"):
                            try:
                                e = self.h.call("uia_find", name=target[k], type=typ, scope=scope, exact=True, timeout=30)
                            except OrigError:
                                continue
                            if e.get("rect"):
                                r = e["rect"]
                                if target.get("icon"):  # R-4: the item's icon at its left end
                                    return (r[0] + 8, (r[1] + r[3]) // 2), None
                                return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), None
                    if k == "list_row":
                        pt = self.list_row_by_image(target[k], bool(target.get("icon")))
                        if pt:
                            return pt, None
                    raise Unmapped(f"{k} '{target[k]}' not found as list/tree item in orig")
            for k in ("layer_row", "object_row", "layer_eye", "layer_lock", "layer_expand", "layer_name"):
                if k in target:
                    return self.layers_point(k, target[k], target.get("layer")), None
            if "popup_swatch" in target:
                # swatch of the open colour popup (a window of its own): found by its colour in the popup image
                main = self.h.call("main")["hwnd"]
                ws = [w for w in self.h.list("windows") if w["hwnd"] != main and w["class"] not in ("tooltips_class32", "#32768")]
                for w in sorted(ws, key=lambda w: -(w["rect"][2] - w["rect"][0]) * (w["rect"][3] - w["rect"][1])):
                    rr = w["rect"]
                    tmp = Path(self.r.cfg["tmp_dir"]) / "orig_popup.png"
                    self.grab(rr, tmp)
                    with Image.open(tmp) as im:
                        pt = find_color_cell(im, [0, 0, im.width, im.height], str(target["popup_swatch"]))
                    if pt:
                        return (rr[0] + pt[0], rr[1] + pt[1]), None
                raise OrigError(f"popup swatch {target['popup_swatch']} not found in the open popup")
            for k in ("layer_row", "object_row", "pref_category", "popup_swatch", "panel_swatch", "list_row",
                      "tree_item", "history_row"):
                if k in target:
                    raise Unsupported(f"target form {{{k}}} not supported for orig yet")
            loc = target
        elif isinstance(target, str) and re.match(r"^dialog\.(field|combo|check|button)\.\d+$", target):
            kind, i = target.split(".")[1:]
            cls = {"field": "Edit", "combo": "ComboBox", "check": "Button", "button": "Button"}[kind]
            ch = self.dialog_child(cls, int(i))
            r = ch["rect"]
            return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
        elif isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            loc = self.ui(target)
            if isinstance(loc, str):  # alias of another target (e.g. "dialog.field.0")
                return self.resolve(loc)
            if isinstance(loc, dict) and ("area" in loc and "pos" in loc or self.grid_point(loc)):
                return self.resolve(loc)
        elif isinstance(target, str) and re.match(r"^[a-z_]+(\.[a-z0-9_]+)+$", target):
            raise Unmapped(f"ui key not in ui_map.json: {target}")
        else:
            return self.label_point(str(target))
        return self.locate(loc)

    # x offsets inside a row of the layers panel list (measured 14.10. at 1600 x 1000): eye, lock, expand triangle,
    # name; the triangle of a sub layer is indented by one level (16 px)
    LAYER_COLS = {"layer_eye": 11, "layer_lock": 32, "layer_expand": 51, "layer_name": 110, "layer_row": 150, "object_row": 150}

    def layers_point(self, kind, name, layer=None):
        """Row of the layers panel, an owner-drawn Win32 list box without item texts (setting the current layer by
        JSF does not move its selection). The row list is rebuilt from the document (JSF `Layer.elems` holds sub
        layers and objects in display order, `disclosure` = expanded) and checked against the list box: same row
        count, layer rows 24 px, object rows taller. The row is scrolled into view (LB_SETTOPINDEX)."""
        p = self.panel("layers")
        lb = next((k for k in self.h.list("kids", hwnd=p["hwnd"]) if k["class"] == "ListBox"), None)
        if not lb:
            raise OrigError("layers panel: no list box")
        lines = self.o.jsf(
            "var d = fw.getDocumentDOM(), rows = [], child = {};NL"
            "for (var i = 0; i < d.layers.length; i++) { var es = d.layers[i].elems || [];NL"
            "  for (var j = 0; j < es.length; j++) if (es[j].isLayer) child[es[j].name] = 1; }NL"
            "function walk(L, par) { rows.push('L' + String.fromCharCode(1) + L.name + String.fromCharCode(1) + par);NL"
            "  if (!L.disclosure) return; var es = L.elems || [];NL"
            "  for (var j = 0; j < es.length; j++) { if (es[j].isLayer) walk(es[j], L.name);NL"
            "    else rows.push('O' + String.fromCharCode(1) + (es[j].name || '') + String.fromCharCode(1) + L.name); } }NL"
            "for (var i = d.layers.length - 1; i >= 0; i--) if (!child[d.layers[i].name]) walk(d.layers[i], '');NL"
            "fp_log('RET ' + rows.join(String.fromCharCode(2)));".replace("NL", "\n"))
        ret = ([ln[4:] for ln in lines if ln.startswith("RET ")] or [""])[-1]
        rows = [r.split(chr(1)) for r in ret.split(chr(2)) if r]
        info = self.h.call("lb_info", hwnd=lb["hwnd"])
        hs = info["heights"]
        if len(rows) != len(hs) or any((r[0] == "L") != (h <= 26) for r, h in zip(rows, hs)):
            raise OrigError(f"layers panel: rows {[r[0] + ':' + r[1] for r in rows]} do not match the list {hs}")
        if kind == "object_row":
            objs = [i for i, r in enumerate(rows) if r[0] == "O" and (layer is None or r[2] == layer)]
            cand = objs[int(name):int(name) + 1] if isinstance(name, int) else [i for i in objs if rows[i][1] == name]
        else:
            cand = [i for i, r in enumerate(rows) if r[0] == "L" and r[1] == name]
        if not cand:
            raise OrigError(f"layers panel: {kind} {name!r} not found")
        idx = cand[0]
        self.h.call("lb_info", hwnd=lb["hwnd"], top=int(idx))
        time.sleep(0.2)
        top = self.h.call("lb_info", hwnd=lb["hwnd"])["top"]
        r = lb["rect"]
        y = r[1] + sum(hs[top:idx]) + hs[idx] // 2
        if not (top <= idx) or y > r[3]:
            raise OrigError(f"layers panel: row {idx} not visible (top {top})")
        x = r[0] + self.LAYER_COLS.get(kind, 150)
        if kind == "layer_expand" and rows[idx][2]:
            x += 16  # sub layer: triangle indented
        return x, y

    def dialog_label_point(self, loc):
        """R-7 (xs-preferences): control of the open dialog found by its text - `{"dialog_label": text, "class":
        cls, "contains": bool}`. A child whose text matches (without '&') and is of `class` is taken itself (check
        boxes, buttons); otherwise the nearest child of `class` right of the matching label on the same row (the
        edit/combo next to 'Interpolation:')."""
        want = clean_label(str(loc["dialog_label"])).lower()
        cls = loc.get("class")
        kids = []
        for d in self.h.list("dialog_kids"):
            kids += [k for k in d.get("kids") or [] if k.get("rect")]

        def hit(k):
            t = clean_label(str(k.get("title") or "")).lower().rstrip(":").strip()
            w = want.rstrip(":").strip()
            return bool(t) and (w in t if loc.get("contains") else t == w)

        labels = [k for k in kids if hit(k)]
        if not labels:
            raise Unmapped(f"dialog label '{loc['dialog_label']}' not found in orig")
        for k in labels:
            if not cls or k.get("class") == cls:
                if not cls or loc.get("self", True):
                    r = k["rect"]
                    return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), k
        lab = labels[0]["rect"]
        cands = [k for k in kids if k.get("class") == cls and k["rect"][0] >= lab[2] - 4
                 and k["rect"][1] < lab[3] + 4 and k["rect"][3] > lab[1] - 4]
        if not cands:
            raise Unmapped(f"no {cls} right of '{loc['dialog_label']}' in orig")
        k = min(cands, key=lambda k: k["rect"][0] - lab[2])
        r = k["rect"]
        return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), k

    def dialog_child(self, cls, index):
        """i-th Win32 child of a class in the open dialog; a dialog opened by the previous action may need a moment."""
        for k in range(10):
            try:
                return self.h.call("find_child", scope="dialog", title=None, **{"class": cls}, index=index)
            except OrigError:
                if k == 9:
                    raise
                time.sleep(0.3)

    def win_at(self, pt):
        """Win32 window under a point (edit fields / combo boxes inside panels are real child windows)."""
        try:
            w = self.h.call("at", x=int(pt[0]), y=int(pt[1]))
        except OrigError:
            return None
        if w and w.get("class") == "Edit" and (w.get("parent") or {}).get("class") == "ComboBox":
            w = dict(w["parent"], title="")
        if w and w.get("class") in ("Edit", "ComboBox"):
            return w
        try:
            w = self.h.call("field_at", x=int(pt[0]), y=int(pt[1]))
        except OrigError:
            return None
        if w and w.get("class") == "Edit" and (w.get("parent") or {}).get("class") == "ComboBox":
            w = dict(w["parent"], title="")
        return w or None

    def locate(self, loc):
        if "rel" in loc and "panel" in loc:
            # the inspector recreates its child windows after a selection change: wait for a field under the point
            for k in range(8):
                p = self.panel(loc["panel"])
                pt = (p["palette"][0] + loc["rel"][0], p["palette"][1] + loc["rel"][1])
                w = self.win_at(pt)
                if w or loc.get("no_field"):
                    return pt, w
                time.sleep(0.3)
            return pt, None
        if "dialog_rel" in loc:
            ds = self.h.list("dialogs")
            if not ds:
                raise OrigError("no dialog open")
            big = max(ds, key=lambda d: (d["rect"][2] - d["rect"][0]) * (d["rect"][3] - d["rect"][1]))
            pt = (big["rect"][0] + loc["dialog_rel"][0], big["rect"][1] + loc["dialog_rel"][1])
            return pt, self.win_at(pt)
        if "window_rel" in loc:
            c = self.client()
            return (c[0] + loc["window_rel"][0], c[1] + loc["window_rel"][1]), None
        if "tools_rel" in loc:
            return self.tools_point(loc["tools_rel"]), None
        if "child" in loc or "dialog_child" in loc:
            spec = loc.get("child") or loc.get("dialog_child")
            scope = "dialog" if "dialog_child" in loc else (self.panel_info(loc["panel"]).get("title") if loc.get("panel") else "main")
            ch = self.h.call("find_child", scope=scope, title=spec.get("title"), **{"class": spec.get("class")}, index=spec.get("index", 0))
            r = ch["rect"]
            return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
        if "popup_child" in loc:
            # control inside a popup window of the program (colour popup hex field, R-2): child of a top-level window
            # other than the main window and the floating panel groups
            spec = loc["popup_child"]
            main = self.h.call("main")["hwnd"]
            skip = {"tooltips_class32", self.r.cfg["orig"].get("floating_dock_class", "OWL.Dock"), "OWL.FrameDrawer", "OWL.ShadowView"}
            for w in self.h.list("windows"):
                if w["hwnd"] == main or w["class"] in skip:
                    continue
                kids = [k for k in self.h.list("kids", hwnd=w["hwnd"]) if k.get("class") == spec.get("class", "Edit")]
                if len(kids) > int(spec.get("index", 0)):
                    k = dict(kids[int(spec.get("index", 0))], popup=True)
                    r = k["rect"]
                    return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), k
            raise OrigError(f"no popup window with a {spec.get('class', 'Edit')} control open")
        if "dialog_label" in loc:
            return self.dialog_label_point(loc)
        if "dialog_button" in loc:
            return None, {"dialog_button": loc["dialog_button"]}
        if "label" in loc:
            return self.label_point(loc["label"])
        if "canvas" in loc:
            return self.doc_to_screen(*self.canvas_doc_point(loc["canvas"])), None
        raise Unmapped(f"orig locator not understood: {loc}")

    def menu_item_point(self, label, skip=()):
        """Item by text in the open popup menus; menus in `skip` (the ones already walked) are searched last, so a
        sub menu entry wins over an equally named entry of its parent ('Airbrush > Einfach')."""
        lab = clean_label(label)
        for _ in range(8):
            ms = self.h.call("menus").get("menus") or []
            ms = [m for m in ms if m.get("hwnd") not in skip] + [m for m in ms if m.get("hwnd") in skip]
            new = [m for m in ms if m.get("hwnd") not in skip]
            for m in ms:
                if skip and new and m.get("hwnd") in skip and all(not (it.get("text") or it.get("separator"))
                                                                  for m2 in new for it in m2.get("items") or []):
                    # the sub menu that just opened is owner-drawn (no texts): never fall back to the parent
                    raise Unmapped(f"sub menu without item texts (owner-drawn), '{label}' needs orig_index")
                for it in m.get("items") or []:
                    if not it.get("separator") and it.get("text") == lab:
                        r = it["rect"]
                        return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), {"menu_item": it}
            time.sleep(0.15)
        if not getattr(self, "_in_alias", False):
            self._in_alias = True
            try:
                for al in self.aliases(label):
                    try:
                        return self.menu_item_point(al, skip)
                    except Unmapped:
                        pass
            finally:
                self._in_alias = False
        raise Unmapped(f"menu item not found in open menus: {label}")

    def wait_menus(self, timeout=2.5):
        """Open popup menus; a menu opened by a click/press-and-hold appears with a delay of up to ~1 s
        (the inspector rebuilds its child windows after a selection change), so poll instead of looking once."""
        t_end = time.time() + timeout
        while True:
            ms = self.h.call("menus").get("menus") or []
            if ms or time.time() >= t_end:
                return ms
            time.sleep(0.15)

    def new_menu_index_point(self, index, walked):
        """Raw item index in the popup menu that opened after the menus in `walked` (a sub menu is listed first
        by the host, so `menus[-1]` would be its parent)."""
        for _ in range(16):
            new = [m for m in self.h.call("menus").get("menus") or [] if m.get("hwnd") not in walked]
            if new:
                items = new[0].get("items") or []
                if index >= len(items):
                    raise OrigError(f"sub menu has only {len(items)} items")
                r = items[index]["rect"]
                return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2)
            time.sleep(0.15)
        raise OrigError("sub menu did not open")

    def menu_index_point(self, index):
        """Owner-drawn popup menus (tool flyouts) have no item texts: raw item index (separators count)."""
        ms = self.wait_menus()
        if not ms:
            raise OrigError("no menu open")
        items = ms[-1].get("items") or []
        if index >= len(items):
            raise OrigError(f"menu has only {len(items)} items")
        r = items[index]["rect"]
        return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2)

    def handle_point(self, h):
        """Control point of the selected object (fw.selection[0].controlPoints): index or role (tooltip prefix)."""
        lines = self.o.jsf('var s = fw.selection[0]; var cp = s ? s.controlPoints : null; var o = [];\n'
                           'if (cp) for (var i = 0; i < cp.length; i++) o.push(cp[i].x + "," + cp[i].y + "," + String(cp[i].toolTip || "").replace(/[,;|]/g, " ") + "," + (cp[i].visible === false ? 0 : 1));\n'
                           'var b = s ? s.pixelRect : null; fp_log("RET " + o.join(";") + "|" + (b ? [b.left, b.top, b.right, b.bottom].join(",") : ""));')
        ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
        pts_s, _, rect_s = (ret[-1] if ret else "|").partition("|")
        pts = []
        for t in filter(None, pts_s.split(";")):
            x, y, tip, vis = (t.split(",", 3) + ["", "1"])[:4]
            pts.append((float(x), float(y), tip, vis != "0"))
        rect = [float(v) for v in rect_s.split(",")] if rect_s.count(",") == 3 else None
        pt = pick_handle(h, pts, rect, self.r.ui_map.get("handle_roles", {}))
        return self.doc_to_screen(*pt)

    def release_held(self):
        if getattr(self, "held", None):
            try:
                self.h.call("up", button="left")
            except OrigError:
                pass
            self.held = None
        # popup sliders stay open over their arrow button (Escape does not close them): click on a neutral spot
        # (empty part of the menu bar), otherwise the next click lands on the slider
        try:
            main = self.h.call("main")["hwnd"]
            for w in self.h.list("windows"):
                r = w["rect"]
                if w["hwnd"] != main and str(w["class"]).startswith("Afx:") and (r[2] - r[0]) * (r[3] - r[1]) < 120000:
                    c = self.client()
                    nx, ny = self.r.ui_map.get("orig_neutral_point", [900, 15])
                    self.o.focus()
                    self.h.call("click", x=c[0] + nx, y=c[1] + ny, button="left", mods=[], count=1)
                    time.sleep(0.2)
                    break
        except OrigError:
            pass

    def popup_slider_rect(self):
        main = self.h.call("main")["hwnd"]
        for _ in range(10):
            ws = [w for w in self.h.list("windows") if w["hwnd"] != main and w["class"] not in ("tooltips_class32", "#32768", "#32770")]
            ws = [w for w in ws if (w["rect"][2] - w["rect"][0]) * (w["rect"][3] - w["rect"][1]) < 120000]
            if ws:
                break
            time.sleep(0.2)
        if not ws:
            raise OrigError("no popup slider open")
        return min(ws, key=lambda w: (w["rect"][2] - w["rect"][0]) * (w["rect"][3] - w["rect"][1]))["rect"]

    def label_point(self, label):
        lab = clean_label(label)
        menus = self.h.call("menus")
        for m in menus.get("menus") or []:
            for it in m.get("items") or []:
                if not it.get("separator") and it.get("text") == lab:
                    r = it["rect"]
                    return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), {"menu_item": it}
        for scope in ("dialog", "main"):
            try:
                ch = self.h.call("find_child", scope=scope, title=lab, index=0)
                r = ch["rect"]
                return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
            except OrigError:
                continue
        loc = (self.r.ui_map.get("orig_labels") or {}).get(lab)
        if isinstance(loc, dict):
            return self.locate(loc)
        if lab in (self.r.ui_map.get("orig_pref_categories") or {}):
            try:
                return self.resolve({"pref_category": lab})
            except OrigError:
                pass
        raise Unmapped(f"label not found as window text in orig: {label}")

    # ------------------------------------------------------------ areas
    def area_rect(self, area: str):
        if area == "window":
            return self.client()
        if area == "canvas":
            return self.h.call("view")
        if area == "inspector":
            return self.panel("properties")["rect"]
        if area.startswith("panel:"):
            info = self.panel_info(area[6:])
            if info.get("title"):
                return self.panel(area[6:])["rect"]
            if info.get("orig_class"):
                try:
                    return self.h.call("find_child", scope="main", title=None, **{"class": info["orig_class"]}, index=0)["rect"]
                except OrigError:
                    if norm_panel(area[6:]) != "tools":
                        raise
                # tools column docked left (no own window class): from the client's left edge to the document view,
                # from the document tab row down (like Firepixel's tools column)
                c, v = self.client(), self.h.call("view")
                return [c[0], max(c[1], v[1] - 30), v[0] - 2, c[3]]
            raise Unmapped(f"panel area {area} unknown for orig")
        if area == "dialog":
            # Flash based dialogs (file info) appear only after a few seconds
            ds = self.h.list("dialogs")
            for _ in range(10):
                if ds:
                    break
                time.sleep(0.4)
                ds = self.h.list("dialogs")
            if not ds:
                raise OrigError("no dialog open")
            return ds[0]["rect"]
        if area == "menu":
            ms = self.wait_menus()
            r = bbox_union([m["rect"] for m in ms])
            if not r:
                raise OrigError("no menu open")
            return r
        if area == "popup":
            main = self.h.call("main")["hwnd"]
            # only windows that appeared during the step (permanent small tool windows are left out)
            old = getattr(self, "windows_before", None) or set()
            ws = [w for w in self.h.list("windows") if w["hwnd"] != main and w["hwnd"] not in old
                  and w["class"] not in ("tooltips_class32",)]
            # R-6 (run 10, inspector-fields/22: 1273 x 927): a new window as large as the main window is no popup
            try:
                c = self.client()
                big = [w for w in ws if (w["rect"][2] - w["rect"][0]) * (w["rect"][3] - w["rect"][1])
                       > 0.4 * (c[2] - c[0]) * (c[3] - c[1])]
                if big and len(big) < len(ws):
                    self.r.log(f"popup: left out large new windows {[(w['class'], w['rect']) for w in big]}")
                    ws = [w for w in ws if w not in big]
                elif big:
                    self.r.log(f"popup: only large new windows {[(w['class'], w['rect']) for w in big]} - difference box")
                    ws = []
            except OrigError:
                pass
            r = bbox_union([w["rect"] for w in ws])
            if r:
                return r
            return None  # caller falls back to the difference box
        raise Unsupported(f"shot area {area}")

    def canvas_shot_rect(self):
        """Floating panel groups over the view are closed first (R-2), then the page is measured again."""
        try:
            if self.close_floating(only_over_view=True):
                time.sleep(0.3)
        except OrigError as e:
            self.r.log(f"close floating panels: {e}")
        v = self.h.call("view")
        # R-4: a command of the step may have changed the document or its size (menu-modify-a/02 image size,
        # xs-file-formats/07 a new document opened by script): the size known from open_testfile is stale, the
        # page fallback then cut 800 x 600 for a 400 x 300 page. Read the active document's size again.
        try:
            lines = self.o.jsf('var d = fw.getDocumentDOM(); fp_log("RET " + (d ? d.width + "," + d.height : "")); '
                               'try { fp_log("ZOOM " + d.zoom); } catch (e) {}')
            ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
            zm = [ln[5:] for ln in lines if ln.startswith("ZOOM ")]
            try:
                self.view_zoom = float(zm[-1]) if zm else None
            except ValueError:
                self.view_zoom = None
            if ret and "," in ret[-1]:
                w, h = ret[-1].split(",")
                if (float(w), float(h)) != tuple(self.doc_size):
                    self.r.log(f"document size changed in the step: {self.doc_size} -> ({w}, {h})")
                self.doc_size = (float(w), float(h))
        except (OrigError, ValueError) as e:
            self.r.log(f"document size: {e}")
        if not self.doc_size[0]:
            return v
        self.page = None
        return self.crop_to_page(self.calibrate(), v)

    def grab(self, rect, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        rect = norm_rect(rect)  # R-12 (TW-124 f)
        self.h.call("shot", rect=[int(v) for v in rect], path=str(path))

    def window_image(self):
        c = self.client()
        tmp = Path(self.r.cfg["tmp_dir"]) / "orig_before.png"
        self.grab(c, tmp)
        with Image.open(tmp) as im:
            return im.convert("RGB"), c

    # ------------------------------------------------------------ input
    def click_at(self, pt, button="left", count=1, mods=()):
        self.o.focus()
        self.h.call("click", x=pt[0], y=pt[1], button=button, mods=list(mods), count=count)

    def key(self, combo):
        mods, k = parse_combo(combo)
        if set(mods) >= {"ctrl", "alt"} and k == "space":
            raise Unsupported("Ctrl+Alt+Space is forbidden")
        if not mods and k in ("enter", "esc"):
            # the program's dialogs ignore Enter/Escape sent as keys (default button is not triggered): with a
            # dialog open and no popup menu, press its OK / Cancel button instead
            try:
                if self.h.list("dialogs") and not self.h.call("menus").get("menus"):
                    self.h.call("dialog_close", mode="ok" if k == "enter" else "cancel")
                    time.sleep(0.5)
                    return
            except OrigError:
                pass
        if k == "tab" and mods in ([], ["shift"]):
            # Tab is not translated by the program's dialogs either (types a character into the field): move the
            # focus with WM_NEXTDLGCTL when the focused control belongs to an open dialog
            try:
                f = self.h.call("focused") or {}
                if f.get("hwnd") and self.in_dialog(f["hwnd"]) and not self.h.call("menus").get("menus"):
                    self.h.call("next_ctl", hwnd=f["hwnd"], back=bool(mods))
                    return
            except OrigError:
                pass
            self.tab_pressed = True
        self.o.focus()
        self.h.call("key", combo="+".join(mods + [k]))

    def window_size(self, w, h):
        """Main window size at the configured position (None = configured size)."""
        x, y, cw, ch = self.r.cfg["orig"].get("rect", [0, 0, 1600, 1000])
        self.h.call("place", x=x, y=y, w=w or cw, h=h or ch)
        time.sleep(0.5)

    def script_eval(self, expr):
        return self.jsf_value(expr)

    def jsf_value(self, expr):
        lines = self.o.jsf(f'var __v = ({expr}); fp_log("RET " + (typeof __v == "object" && __v !== null ? (__v.toSource ? __v.toSource() : String(__v)) : String(__v)));')
        ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
        return ret[-1] if ret else None

    def select(self, names):
        names = select_names(names)
        code = ("var d = fw.getDocumentDOM(); var want = %s; var sel = [], cnt = 0, sc = 0;\n"
                "for (var i = 0; i < d.layers.length; i++) { var fr = d.layers[i].frames[d.currentFrameNum]; var web = d.layers[i].layerType == 'web';\n"
                "  for (var j = 0; j < fr.elements.length; j++) { var e = fr.elements[j];\n"
                "    cnt++; if (web) sc++; for (var k = 0; k < want.length; k++) if (e.name == want[k] || (want[k] !== null && typeof want[k] == 'object' && (want[k].index == cnt - 1 || (web && want[k].slice == sc - 1)))) sel.push(e); } }\n"
                "d.selectNone(); fw.selection = sel; fp_log('RET ' + sel.length);") % json.dumps(names, ensure_ascii=True)
        before = self.panel_image()
        lines = self.o.jsf(code)
        n = [ln for ln in lines if ln.startswith("RET ")]
        if not n or n[-1] == "RET 0":
            raise OrigError(f"objects not found: {names}")
        # the inspector rebuilds its child windows for the new selection after the script returns - sometimes only
        # after a few seconds (right after opening a document it may keep showing the marquee inspector meanwhile):
        # wait until the properties panel changed (at most ~3 s), then let it settle
        time.sleep(0.4)
        if before is not None:
            for _ in range(12):
                now = self.panel_image()
                if now is None or now.size != before.size or ImageChops.difference(now, before).getbbox():
                    break
                time.sleep(0.25)
        time.sleep(0.3)

    def list_row_by_image(self, name, icon=False):
        """R-5: the document library of the reference program is owner-drawn (no UI Automation items, run 8/9
        `panel-library-styles/04` unmapped). The row is found in the panel image: text bands below the list frame
        line (first band = column header), the n-th data band for the n-th library name in the program's order
        (sorted by name, case-insensitive). Returns a screen point or None (logged)."""
        p = getattr(self, "last_panel", None)
        if not p:
            return None
        try:
            lines = self.o.jsf('var d = fw.getDocumentDOM(), n = []; '
                               'try { for (var k = 0; k < d.library.length; k++) n.push(d.library[k].name); } catch (e) {} '
                               'if (!n.length) try { for (var k = 0; k < d.library.items.length; k++) n.push(d.library.items[k].name); } catch (e) {} '
                               'if (!n.length) try { for (var k = 0; k < d.symbols.length; k++) n.push(d.symbols[k].name); } catch (e) {} '
                               # R-6 (run 10: [] for all three): symbol names of the instances in the document
                               'if (!n.length) { var s = {}; var walk = function (els) { for (var i = 0; i < els.length; i++) { '
                               'try { if (els[i].symbolName) s[els[i].symbolName] = 1; } catch (e) {} '
                               'try { if (els[i].elements) walk(els[i].elements); } catch (e) {} } }; '
                               'try { for (var l = 0; l < d.layers.length; l++) walk(d.layers[l].frames[d.currentFrameNum].elements); } catch (e) {} '
                               'for (var q in s) n.push(q); if (n.length) fp_log("SRC instances"); } '
                               'try { fp_log("DOC " + d.docTitle); } catch (e) {} '
                               'fp_log("NAMES " + n.join("|"));')
            names = [ln[6:] for ln in lines or [] if ln.startswith("NAMES ")]
            names = [x for x in (names[-1].split("|") if names else []) if x]
            doc = next((ln[4:] for ln in reversed(lines or []) if ln.startswith("DOC ")), "")
            # library items without an instance (an unused symbol) are missing from the instance scan: the names
            # seen in the reference's library image (ui_map `orig_library_names`, per document) complete the list
            extra = (self.r.ui_map.get("orig_library_names") or {}).get(doc) or []
            if extra and not set(extra) <= set(names):
                self.r.log(f"list_row by image: library names {names} + ui_map {extra}")
                names = list(dict.fromkeys(names + list(extra)))
            pr = self.panel(p)["rect"]
        except OrigError as e:
            self.r.log(f"list_row by image: {e}")
            return None
        order = sorted(names, key=lambda s: s.lower())
        if name not in order:
            self.r.log(f"list_row by image: '{name}' not among the library names {order}")
            return None
        img = self.panel_image(p)
        if img is None:
            return None
        w, h = img.size
        x0, x1 = 4, min(w, 120)
        px = img.load()
        def stats(y):
            dark = blue = 0
            for x in range(x0, x1):
                c = px[x, y]
                dark += max(c) < 140
                blue += c[2] > 150 and c[0] < 120
            return dark, blue
        rows = [stats(y) for y in range(h)]
        # list frame: the first (almost) full dark line below the preview area
        top = next((y for y in range(h // 6, h) if rows[y][0] >= 0.9 * (x1 - x0)), None)
        if top is None:
            self.r.log("list_row by image: list frame not found in the panel image")
            return None
        bands, cur = [], None
        for y in range(top + 2, h):
            on = rows[y][0] >= 2 or rows[y][1] >= 40
            if on and rows[y][0] >= 0.9 * (x1 - x0):
                on = False  # another frame line (footer)
            if on:
                cur = [y, y] if cur is None else [cur[0], y]
            elif cur is not None:
                bands.append(cur)
                cur = None
        if len(bands) < 2:
            self.r.log(f"list_row by image: {len(bands)} text bands below the list frame")
            return None
        data = bands[1:]
        starts = [b[0] for b in data]
        pitch = (starts[-1] - starts[0]) / (len(starts) - 1) if len(starts) > 1 else 16.0
        pitch = pitch if 12 <= pitch <= 24 else 16.0
        i = order.index(name)
        cy = (data[0][0] + data[0][1]) / 2 + i * pitch
        if cy >= h - 4:
            self.r.log(f"list_row by image: row {i} of {order} below the visible list")
            return None
        x = pr[0] + (14 if icon else 50)
        self.r.log(f"list_row by image: '{name}' = row {i} of {order}, pitch {pitch}, y {int(pr[1] + cy)}")
        return int(x), int(pr[1] + cy)

    def panel_image(self, p="properties"):
        try:
            r = self.panel(p)["rect"]
            tmp = Path(self.r.cfg["tmp_dir"]) / "orig_panel.png"
            self.grab(r, tmp)
            with Image.open(tmp) as im:
                return im.convert("RGB")
        except Exception:  # noqa: BLE001
            return None

    # ------------------------------------------------------------ verbs
    def confirm_import(self):
        """Import (R-2, menu-file/10, xs-file-formats/09): after the file dialog the reference program may ask for the
        page to import (ui_map `orig_import_confirm`: dialog titles and the confirming button). A canvas action
        then means 'place it here': confirm that dialog first, otherwise the click goes nowhere and the import stays
        open."""
        conf = self.r.ui_map.get("orig_import_confirm") or {}
        titles = [clean_label(t) for t in conf.get("titles") or []]
        if not titles:
            return False
        try:
            ds = self.h.list("dialogs")
        except OrigError:
            return False
        for d in ds:
            if clean_label(d.get("title") or "") not in titles:
                continue
            for name in conf.get("buttons") or ["OK"]:
                try:
                    b = self.h.call("find_child", scope="dialog", title=name, **{"class": "Button"}, index=0)
                except OrigError:
                    continue
                r = b["rect"]
                self.click_at(((r[0] + r[2]) // 2, (r[1] + r[3]) // 2))
                time.sleep(0.6)
                self.r.log(f"import dialog '{d.get('title')}' confirmed with '{name}'")
                return True
            self.h.call("dialog_close", mode="ok")
            time.sleep(0.6)
            return True
        return False

    def do(self, verb, arg, act):
        r = self.r
        if verb.startswith("canvas_"):
            self.confirm_import()
            if getattr(self, "import_pending", False) and verb in ("canvas_click", "canvas_drag", "canvas_double_click"):
                # R-4 (menu-file/10, xs-file-formats/09): the click came 0.04 s after the file dialog, before the
                # program had loaded the file and switched to its placing cursor - nothing was placed. Wait, move
                # onto the target point first (the placing cursor follows the mouse), then click/drag
                self.import_pending = False
                try:
                    a = arg.get("from") if verb == "canvas_drag" and isinstance(arg, dict) else arg
                    pt = self.doc_to_screen(*self.canvas_doc_point(a))
                    self.o.focus()
                    self.h.call("move", x=pt[0], y=pt[1])
                    time.sleep(0.5)
                    self.h.call("move", x=pt[0] + 1, y=pt[1] + 1)
                    self.h.call("move", x=pt[0], y=pt[1])
                    time.sleep(0.3)
                except (OrigError, Unmapped) as e:
                    self.r.log(f"import placing: {e}")
        if verb in ("open_testfile", "reopen"):
            r.open_testfile(self, arg)
        elif verb in ("close_all", "close_all_documents"):
            # every open document, also the ones a script opened itself (jsf / file dialog), runner finding 5
            self.reset_light()
            self.o.close_all_docs()
            r.opened[self.name] = set()
        elif verb in ("menu", "menu_open", "hover_menu"):
            # `menu`: a leaf entry at the end of the path is executed (scripts check the result), a sub menu
            # stays open; `menu_open`/`hover_menu` only open/hover
            self.h.call("menu", path=split_path(arg), final="auto" if verb == "menu" else "open",
                        bar=r.ui_map["menu_bar"]["orig"], timeout=40)
            if verb == "menu":
                self.after_menu()
        elif verb == "menu_item":
            self.h.call("menu", path=split_path(arg), final="invoke", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
            self.after_menu()
        elif verb == "command":
            path = r.ui_map.get("commands", {}).get(arg)
            if not path:
                raise Unmapped(f"command {arg} has no orig menu path in ui_map.json")
            self.h.call("menu", path=split_path(path), final="invoke", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
            self.after_menu()
        elif verb == "close_dialogs":
            self.reset_light()
        elif verb == "key":
            for _ in range(int(act.get("repeat", 1) or 1)):
                self.key(arg)
            if getattr(self, "popup_edit", None) and str(arg).lower() in ("enter", "return", "tab"):
                self.popup_edit_record("after_key", str(arg))
        elif verb == "type":
            self.o.focus()
            self.h.call("type", text=str(arg), timeout=60)
            if getattr(self, "popup_edit", None):
                self.popup_edit_type(str(arg))
        elif verb == "wait":
            time.sleep(float(arg if not isinstance(arg, dict) else arg.get("ms", 0)) / 1000)
        elif verb == "select":
            self.select(arg)
        elif verb == "select_none":
            self.o.jsf("fw.getDocumentDOM().selectNone();")
        elif verb == "select_all":
            self.o.jsf("fw.getDocumentDOM().selectAll();")
        elif verb == "tool":
            t = arg if str(arg).startswith("tool.") else f"tool.{arg}"
            k = r.ui_map.get("tools", {}).get(t)
            if not k:
                raise Unmapped(f"tool {t} has no orig shortcut in ui_map.json")
            if isinstance(k, dict) and "jsf" in k:
                self.o.jsf(k["jsf"])
            elif isinstance(k, dict) and "flyout" in k:
                # tool hidden in a toolbox flyout (RUN-6): press-and-hold the group button, click the
                # owner-drawn entry by index
                self.reset_light()
                self.choose({"target": k["flyout"], "item": k.get("item", "")}, {"orig_index": k["orig_index"]})
                time.sleep(0.3)
            elif isinstance(k, dict):
                self.reset_light()
                self.click_at(self.locate(k)[0])
            else:
                self.reset_light()
                self.key(k)
        elif verb == "panel":
            info = self.panel_info(arg)
            self.last_panel = arg  # R-5: list_row by image looks in the panel shown last
            if info.get("title"):
                try:
                    self.h.call("panel", name=info["title"])
                    return
                except OrigError:
                    pass
            if not info.get("menu"):
                raise Unmapped(f"panel {arg}: no orig menu path")
            self.h.call("menu", path=split_path(info["menu"]), final="invoke", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
        elif verb in ("jsf", "run_script"):
            code = arg if isinstance(arg, str) else arg.get("code")
            lines = self.o.jsf(code if "fp_log" in code else f"var __v = (function(){{ {code} }})(); if (__v !== undefined) fp_log('RET ' + __v);")
            self.values.append({"jsf": code[:200], "out": lines[-5:]})
        elif verb in ("log_value", "log"):
            expr = arg if isinstance(arg, str) else arg.get("value") or arg.get("expr")
            if isinstance(arg, dict) and not expr:
                raise Unsupported(f"log {arg}")
            try:
                self.values.append({"expr": expr, "value": self.jsf_value(expr)})
            except OrigError as e:
                # R-6 (run 10 color-popup/21, panel-info-align-path/15, panel-misc/20): the history palette is not
                # readable by script in the reference program (-2147216344); the value is recorded as missing and the
                # step goes on (the undo itself runs as a key / menu command)
                if "-2147216344" not in str(e):
                    raise
                self.values.append({"expr": expr, "value": None, "error": "script error -2147216344"})
                self.r.log(f"log_value: {expr[:60]} -> -2147216344 (recorded as missing)")
        elif verb in ("click", "double_click", "right_click", "hover", "mouse_down", "mouse_up", "click_text",
                      "double_click_text", "right_click_text", "hover_text", "click_button", "toolbox_click", "uia_click",
                      "open", "canvas_click", "canvas_double_click", "canvas_move", "canvas_hover", "window_click"):
            self.pointer(verb, arg, act)
        elif verb in ("drag", "canvas_drag"):
            a = arg
            fr = self.resolve({"canvas": a["from"]} if verb == "canvas_drag" or isinstance(a.get("from"), list) else a["from"])[0]
            if "to" in a:
                # R-12 (TW-124 d): `allow_outside_end` - the end point may lie outside the document view (xs-canvas/10
                # drags a guide onto the ruler); the start point is checked as before
                out_end = bool(a.get("allow_outside_end"))
                to = self.resolve({"canvas": a["to"], "allow_outside": out_end}
                                  if verb == "canvas_drag" or isinstance(a.get("to"), list) else a["to"])[0]
            elif "by" in a:
                to = (fr[0] + a["by"][0], fr[1] + a["by"][1])
            else:
                raise Unsupported("drag without to/by")
            if verb == "canvas_drag" or isinstance(a.get("from"), list):
                self.check_drag_points([fr] if a.get("allow_outside_end") else [fr, to])
                if a.get("allow_outside_end"):
                    self.check_on_window(to)
            self.o.focus()
            dmods = norm_mods(a.get("mods") or a.get("hold_key"))
            drec = self.drag_record(fr, to, dmods)
            canvas = verb == "canvas_drag" or isinstance(a.get("from"), list)
            if canvas:
                drec["state_before"] = self.tool_state()  # R-9 (TW-107/109, Q-50): tool, fill, stroke before the drag
            self.values.append({"drag": drec})
            # R-7 (M9): `press_ms` / `release_ms` = pauses after the press (with two small moves) and before the release
            res = self.h.call("drag", x1=fr[0], y1=fr[1], x2=to[0], y2=to[1], mods=dmods, steps=a.get("steps", 20),
                              press_ms=a.get("press_ms"), release_ms=a.get("release_ms"), timeout=90)
            # R-9 (TW-109): state of the held keys (GetAsyncKeyState) at the press and before the release; the host
            # sends them again when they were not down
            if isinstance(res, dict) and res.get("keys"):
                drec["keys"] = res["keys"]
                if dmods and not (res["keys"].get("at_press") and res["keys"].get("before_release")):
                    self.r.log(f"drag {fr} -> {to}: held keys {dmods} not down in the reference: {res['keys']}")
            if canvas:
                drec["state_after"] = self.tool_state()
            # R-5: the host waits for the program after every drag input; a long wait is logged (large bitmap)
            if isinstance(res, dict) and int(res.get("busy_ms") or 0) >= 300:
                self.r.log(f"drag {fr} -> {to}: program busy {res['busy_ms']} ms during the drag")
        elif verb == "choose":
            self.choose(arg, act)
        elif verb in ("set_field", "set", "set_dialog_field"):
            self.set_field(dict(act, **arg) if isinstance(arg, dict) else act)
        elif verb == "record_ui":
            self.values.append({"record_ui": arg, "value": self.record_ui(arg)})
        elif verb == "log_menu_items":
            self.values.append({"menu_items": self.menu_items()})
        elif verb == "clear_dir":
            r.clear_dir(arg)
        elif verb in ("record_dir", "record_file", "record_file_text"):
            self.values.append({verb: arg, "value": r.record_path(verb, arg)})
        elif verb in ("file_dialog", "file_dialog_choose"):
            path = arg if isinstance(arg, str) else arg.get("path")
            # R-13 (TW-129 b, run 17 panel-swatches-mixer/06): the dialog opened by a panel menu entry can take a few
            # seconds - wait up to `wait_ms` (default 6000) before giving up
            wait = float((arg.get("wait_ms") if isinstance(arg, dict) else None) or act.get("wait_ms") or 6000) / 1000
            end = time.time() + wait
            ds = [d for d in self.h.list("dialogs")]
            while not ds and time.time() < end:
                time.sleep(0.3)
                ds = [d for d in self.h.list("dialogs")]
            if not ds:
                raise OrigError(f"no file dialog open (waited {wait:.1f} s)")
            ed = self.h.call("find_child", scope="dialog", title=None, **{"class": "Edit"}, index=0)
            self.h.call("set_text", hwnd=ed["hwnd"], text=path)
            self.h.call("dialog_close", mode="ok")
            # R-4: let the program read the file (an import switches to its placing cursor only afterwards)
            titles = {clean_label(t) for t in (r.ui_map.get("orig_import_confirm") or {}).get("titles") or []}
            for _ in range(15):
                time.sleep(0.2)
                try:
                    left = [d for d in self.h.list("dialogs") if clean_label(d.get("title") or "") not in titles]
                except OrigError:
                    break
                if not left:
                    break
            time.sleep(0.4)
            self.import_pending = True
        elif verb == "file_dialog_cancel":
            self.h.call("dialog_close", mode="cancel")
        elif verb == "export":
            a = export_spec(arg)
            d = r.safe_dir(a["dir"])
            d.mkdir(parents=True, exist_ok=True)
            fmt = str(self.jsf_value("fw.getDocumentDOM().exportOptions.exportFormat") or "PNG").upper()
            # R-8 (TW-102/103): colour depth and number of states before the export, next to the format
            try:  # one script for all probes (each script costs ~0.4 s in the reference)
                got = str(self.jsf_value("[" + ", ".join(f"String({e})" for _, e in EXPORT_PROBES) + '].join("|")') or "")
                opts = dict(zip([k for k, _ in EXPORT_PROBES], got.split("|")))
            except OrigError as ex:
                opts = {"error": str(ex)[:80]}
            out = d / f'{a.get("name") or "export"}.{EXPORT_EXT.get(fmt, "png")}'
            self.o.jsf(f'var d = fw.getDocumentDOM(); fw.exportDocumentAs(d, {js_str(file_url_of(out))}, null);', timeout=90)
            self.values.append({"export": a, "format": fmt, "options": opts, "value": r.record_path("record_dir", str(d)),
                                "kept": r.keep_export(self, out)})
        elif verb == "export_png":
            out = r.export_path(self)
            if out.exists():
                out.unlink()
            self.o.jsf(f'var d = fw.getDocumentDOM(); fw.exportDocumentAs(d, {js_str(file_url_of(out))}, null);', timeout=90)
            self.values.append({"export_png": arg, "value": r.record_path("record_file", str(out))})
        elif verb in ("drag_popup_slider", "slider_drag"):
            a = arg if isinstance(arg, dict) else {"to_fraction": arg}
            r = self.popup_slider_rect()
            p1, p2 = slider_points(r, float(a.get("to_fraction", a.get("fraction", 0.5))), a.get("from_fraction"))
            self.o.focus()
            self.h.call("down", x=p1[0], y=p1[1])
            self.held = "left"
            self.h.call("held_move", x1=p1[0], y1=p1[1], x2=p2[0], y2=p2[1], steps=12)
            if not a.get("hold"):
                self.h.call("up", x=p2[0], y=p2[1])
                self.held = None
            time.sleep(0.3)
        elif verb in ("panel_menu", "panel_menu_item"):
            p = arg if isinstance(arg, str) else arg.get("panel")
            info = self.panel_info(p)
            if not info.get("title"):
                raise Unmapped(f"panel {p}: no title for the panel menu")
            g = self.panel(p)
            dx, dy = self.r.ui_map.get("orig_panel_menu_offset", [-13, 8])
            self.click_at((g["rect"][2] + dx, g["rect"][1] + dy))
            time.sleep(0.4)
            item = act.get("item") or (arg.get("item") if isinstance(arg, dict) else None)
            if item:
                self.menu_path(split_path(item))
        elif verb in ("flyout_open", "flyout", "flyout_item"):
            a = arg if isinstance(arg, dict) else {"target": arg}
            target = a.get("target") or a.get("flyout")
            if a.get("item") or act.get("item"):
                self.choose({"target": target, "item": a.get("item") or act.get("item")}, act)
            else:
                self.pointer("open", target, act)
        elif verb == "fx_add":
            item = arg if isinstance(arg, str) else arg.get("item")
            self.choose({"target": "inspector.fx_add", "item": item}, act)
        elif verb in ("scroll", "wheel", "canvas_wheel"):
            a = arg if isinstance(arg, dict) else {"delta": arg}
            if verb == "canvas_wheel":
                pt = self.canvas_point(a, "canvas_wheel")
            elif isinstance(a.get("area"), dict):
                pt = self.resolve(a["area"])[0]
            elif a.get("target") is not None:
                pt = self.resolve(a["target"])[0]
            else:
                rr = self.area_rect(a.get("area", "canvas"))
                pt = ((rr[0] + rr[2]) // 2, (rr[1] + rr[3]) // 2)
            mods = a.get("mods") or a.get("modifiers") or act.get("mods") or []
            d = int(a.get("delta", 1))
            if verb != "canvas_wheel":
                d = -d  # scroll: positive = content moves up (wheel towards the user)
            self.o.focus()
            self.h.call("wheel", x=pt[0], y=pt[1], delta=d, mods=list(mods), horizontal=bool(a.get("horizontal")))
            time.sleep(0.3)
        elif verb in ("drag_handle", "click_handle", "hover_handle"):
            a = arg if isinstance(arg, dict) else {"handle": arg}
            pt = self.handle_point(a.get("handle", a.get("index", 0)))
            if verb == "drag_handle":
                by = a.get("by") or act.get("by") or [0, 0]
                self.o.focus()
                self.h.call("drag", x1=pt[0], y1=pt[1], x2=pt[0] + by[0], y2=pt[1] + by[1], mods=a.get("mods") or [], steps=20, timeout=60)
            elif verb == "click_handle":
                self.click_at(pt)
            else:
                self.o.focus()
                self.h.call("move", x=pt[0], y=pt[1])
        elif verb in ("ui_action", "ui_find_click", "ui_find", "ui_state", "get_document", "ui_file_dialog", "fp_only"):
            raise Unsupported(f"{verb} is Firepixel-only (put it under 'fp')")
        else:
            raise Unsupported(verb)

    def after_menu(self):
        """An executed menu command (filters, transforms) is processed by the program's UI thread: give it time,
        and wait until no busy/progress window is left (dialogs the command opens stay open)."""
        time.sleep(0.5)
        for _ in range(20):
            try:
                if not (self.h.call("menus").get("menus")):
                    break
            except OrigError:
                break
            time.sleep(0.2)

    def pointer(self, verb, arg, act):
        mods = norm_mods(act.get("mods") or act.get("hold_key"))
        if verb.startswith("canvas_"):
            pt, info = self.canvas_point(arg, verb), None
            verb = {"canvas_click": "click", "canvas_double_click": "double_click"}.get(verb, "hover")
        else:
            target = arg
            if isinstance(arg, dict) and "target" in arg:
                mods = norm_mods(arg.get("mods") or arg.get("hold_key") or mods)
                target = arg["target"]
            pt, info = self.resolve(target)
            if isinstance(target, dict) and any(k in target for k in SWATCH_KEYS):
                self.values.append({"click_probe": click_probe(self, target, pt)})
            # R-8 (Q-46): an edit inside a popup of the program (colour popup hex field) is remembered for `type`/`key`
            self.popup_edit = info if isinstance(info, dict) and info.get("popup") and info.get("class") == "Edit" \
                and info.get("hwnd") else None
        if info and "dialog_button" in info:
            self.h.call("dialog_close", mode=info["dialog_button"])
            return
        loc = {} if verb.startswith("canvas_") else self.loc_of(arg.get("target") if isinstance(arg, dict) and "target" in arg else arg)
        if loc.get("select") and info and info.get("class") == "ComboBox" and verb in ("click", "open"):
            self.h.call("combo_select", hwnd=info["hwnd"], text=clean_label(loc["select"]))
            return
        if loc.get("hold_ms") and verb in ("click", "open"):
            # toolbox flyouts open while the button is held; the menu is owner-drawn. A first press after a
            # focus change sometimes only activates the window: up to three presses, the hold growing each time
            hold = float(loc["hold_ms"]) / 1000
            for k in range(3):
                self.o.focus()
                before = {w["hwnd"] for w in self.h.list("windows")}
                self.h.call("move", x=pt[0], y=pt[1])
                self.h.call("down", x=pt[0], y=pt[1])
                time.sleep(hold * (1 + 0.6 * k))
                opened = bool(self.h.call("menus").get("menus")) or any(w["hwnd"] not in before for w in self.h.list("windows"))
                self.h.call("up", x=pt[0], y=pt[1])
                time.sleep(0.3)
                if opened or self.h.call("menus").get("menus"):
                    return
                self.h.call("close_menus")
            raise OrigError(f"flyout did not open at {pt} (tools anchor {getattr(self, 'tools_anchor', None)})")
        if verb in ("hover", "hover_text", "canvas_hover", "canvas_move"):
            self.o.focus()
            self.h.call("move", x=pt[0], y=pt[1])
        elif verb == "mouse_down":
            self.o.focus()
            self.h.call("down", x=pt[0], y=pt[1], mods=list(mods))
            self.held = "left"
        elif verb == "mouse_up":
            self.h.call("up", x=pt[0], y=pt[1], mods=list(mods))
            self.held = None
        elif verb == "open" and not info:
            # `open` = something must pop up (popup window, menu, dialog). The first click after a selection change
            # often only activates the rebuilt properties panel: if nothing appeared within ~2.5 s, click once more
            before = {w["hwnd"] for w in self.h.list("windows")}
            self.click_at(pt, "left", 1, mods)
            for _ in range(16):
                time.sleep(0.15)
                if self.h.call("menus").get("menus") or any(w["hwnd"] not in before for w in self.h.list("windows")):
                    break
            else:
                self.click_at(pt, "left", 1, mods)
        else:
            button = "right" if verb.startswith("right") else "left"
            count = 2 if verb.startswith("double") else 1
            self.click_at(pt, button, count, mods)
        time.sleep(0.25)

    def loc_of(self, target):
        if isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            e = self.r.ui_map["targets"][target]
            loc = (e.get(self.name) if isinstance(e, dict) else None) or {}
            # a string locator is an alias ("dialog.combo.1", another key): no options of its own (RUN-6)
            return self.loc_of(loc) if isinstance(loc, str) and loc != target else (loc if isinstance(loc, dict) else {})
        return target if isinstance(target, dict) else {}

    def menu_path(self, parts, final_click=True):
        """Walk open popup menus by item text: hover sub menus, click the last entry."""
        walked = set()
        owner = self.r.ui_map.get("orig_owner_drawn_menus") or {}
        for i, part in enumerate(parts):
            if i > 0 and parts[i - 1] in owner and part in owner[parts[i - 1]]:
                # owner-drawn sub menu (no item texts, e.g. stroke categories): item order from ui_map
                pt, info = self.new_menu_index_point(owner[parts[i - 1]].index(part), walked), {}
            else:
                pt, info = self.menu_item_point(part, walked)
            walked = {m.get("hwnd") for m in self.h.call("menus").get("menus") or []}
            last = i == len(parts) - 1
            if last and final_click:
                if (info.get("menu_item") or {}).get("grayed"):
                    raise OrigError(f"menu item disabled: {part}")
                self.click_at(pt)
            else:
                self.o.focus()
                self.h.call("move", x=pt[0], y=pt[1])
                time.sleep(0.45)

    def combo_notify(self, target=None):
        """R-7 (run 11, panel-optimize/01…: the format combo showed GIF, the panel kept its PNG 32 layout):
        notification codes sent to the combo's parent after the selection, config `orig.combo_notify` (default
        CBN_SELCHANGE only, as before); a ui_map target may carry its own `notify` list in its orig locator."""
        e = (self.r.ui_map.get("targets") or {}).get(target) if isinstance(target, str) else None
        n = (e.get("orig") or {}).get("notify") if isinstance(e, dict) and isinstance(e.get("orig"), dict) else None
        n = n or self.r.cfg["orig"].get("combo_notify")
        return {"notify": [int(x) for x in n]} if n else {}

    def choose(self, a, act=None):
        act = act or {}
        target, item = a.get("target"), a.get("item")
        pt, info = self.resolve(target) if target else (None, None)
        if info and info.get("class") == "ComboBox":
            try:
                self.h.call("combo_select", hwnd=info["hwnd"], text=clean_label(item), **self.combo_notify(target))
                return
            except OrigError as first:
                # spelling differs ("PNG 32" / "PNG32", R-2): the combo's own entries, loosely matched
                try:
                    items = self.h.call("combo_items", hwnd=info["hwnd"]).get("items") or []
                except OrigError:
                    items = []
                hit = match_item(items, clean_label(item), self.combo_aliases(item)) if items else None
                if hit:
                    self.h.call("combo_select", hwnd=info["hwnd"], text=hit, **self.combo_notify(target))
                    return
                if items and len(items) > 1:
                    raise OrigError(f"combo entry not found: {item} (entries: {' | '.join(items[:30])})") from first
                # owner-drawn combo boxes with one entry (stroke/fill category, texture) open a popup menu
                # when clicked: walk it like a menu
                self.click_at(pt)
                time.sleep(0.3)
                if not self.wait_menus():
                    raise
                oi = (act or {}).get("orig_index", a.get("orig_index"))
                parts = split_path(item or "")
                if oi is not None:
                    self.click_index_path(parts, int(oi))
                    return
                self.menu_path(parts)
                return
        if target:
            self.pointer("open", target, {})
            time.sleep(0.3)
        oi = act.get("orig_index", a.get("orig_index"))
        parts = split_path(item or "")
        if oi is not None:
            # owner-drawn menus without item texts: `orig_index` = raw index (separators count) in the last menu;
            # a path 'A > B' walks the text entries before it ('Einfach' in the stroke category menu, whose sub
            # menu is owner-drawn)
            self.click_index_path(parts, int(oi))
            return
        if not self.wait_menus():
            raise Unsupported("choose: target opened no menu and is no Win32 combo box")
        self.menu_path(parts)

    def click_index_path(self, parts, oi):
        """`orig_index` = raw index (separators count) in the last menu of the path; the text entries before it are
        hovered first ('A > B': B by index in the owner-drawn sub menu of A)."""
        if len(parts) > 1:
            before = {m.get("hwnd") for m in self.wait_menus()}
            if len(parts) > 2:
                self.menu_path(parts[:-2], final_click=False)
            walked = {m.get("hwnd") for m in self.h.call("menus").get("menus") or []} | before
            self.menu_path(parts[-2:-1], final_click=False)
            time.sleep(0.3)
            self.click_at(self.new_menu_index_point(oi, walked))
        else:
            self.click_at(self.menu_index_point(oi))

    def in_dialog(self, hwnd) -> bool:
        try:
            return any(hwnd and self.h.call("next_ctl_root", hwnd=hwnd) == d["hwnd"] for d in self.h.list("dialogs"))
        except OrigError:
            return False

    def set_field(self, a):
        target, value = a.get("target") or a.get("set_field") or a.get("set"), a.get("value")
        pt, info = self.resolve(target)
        if pt is None:
            raise Unsupported("set_field without a point")
        commit = set_field_commit(a)
        if info and info.get("class") == "Edit" and self.in_dialog(info["hwnd"]):
            # dialog edit: the program's dialogs translate neither Enter nor Tab (SendInput Enter/Tab end up as a
            # character in the field: "60r", runner finding F1) - focus the field, set its text, then end like
            # the script says: Tab = WM_NEXTDLGCTL (the field commits on focus loss), Enter = the OK button
            commit = commit or "tab"
            self.click_at(pt)
            self.h.call("set_text", hwnd=info["hwnd"], text=str(value))
            if commit in ("tab", "enter"):
                try:
                    self.h.call("next_ctl", hwnd=info["hwnd"])
                except OrigError as e:
                    # R-7 (run 11, xs-preferences/02,17): the field's root window is no dialog any more (page of
                    # the preferences rebuilt) - the focus moves on in the largest open dialog instead
                    ds = self.h.list("dialogs")
                    if not ds:
                        raise
                    big = max(ds, key=lambda d: (d["rect"][2] - d["rect"][0]) * (d["rect"][3] - d["rect"][1]))
                    self.r.log(f"set_field: {e} - next control of dialog {big.get('title')!r} instead")
                    self.h.call("next_ctl", hwnd=big["hwnd"])
                time.sleep(0.2)
            try:
                text = self.h.call("get_text", hwnd=info["hwnd"])
            except OrigError as e:
                text = f"? ({e})"
            if commit == "enter":
                self.h.call("dialog_close", mode="ok")
                time.sleep(0.5)
            self.values.append({"set_field": str(target), "commit": commit, "text": text})
            return
        if info and info.get("class") in ("Edit",):
            self.h.call("set_text", hwnd=info["hwnd"], text=str(value))
        self.click_at(pt, count=2)
        self.h.call("key", combo="ctrl+a")
        self.h.call("type", text=str(value))
        if (commit or "enter") != "none":
            self.h.call("key", combo=commit or "enter")
        loc = self.loc_of(target)
        if loc.get("jsf_set"):
            # some inspector fields keep typed text without applying it to the selection (no commit from
            # synthetic input): apply the value with the equivalent JSF call (ui_map `jsf_set`, %s = value)
            v = str(value)
            lit = v if re.fullmatch(r"-?\d+(\.\d+)?", v) else json.dumps(v)
            self.o.jsf(loc["jsf_set"].replace("%s", lit))
            self.values.append({"set_field_via": "jsf", "target": target, "value": value})

    def record_ui(self, key):
        if isinstance(key, dict):
            if "focus_cycle" in key:
                out = []
                for _ in range(int(key["focus_cycle"] or 1)):
                    self.key("tab")
                    time.sleep(0.12)
                    f = self.h.call("focused") or {}
                    out.append({"class": f.get("class"), "text": f.get("title")})
                return {"focus_cycle": out}
            raise Unsupported(f"record_ui {key}")
        if key == "dialog.controls":
            # child windows (class, text, rect) of the open dialogs - used to measure locators (RUN-6)
            return self.h.list("dialog_kids")
        if key in ("dialog.fields", "dialog.open", "dialog.title", "dialog.text"):
            return self.h.list("dialogs")
        if key in ("menu.open", "menu.disabled_items"):
            return self.menu_items()
        if key == "focus":
            f = self.h.call("focused") or {}
            return {"class": f.get("class"), "text": f.get("title")}
        if key == "list.items":
            return None
        if key == "popup.visible":
            # popup = a window of the program that appeared during the step (colour/slider popups, menus)
            main = self.h.call("main")["hwnd"]
            old = getattr(self, "windows_before", None) or set()
            ws = [w for w in self.h.list("windows") if w["hwnd"] != main and w["hwnd"] not in old
                  and w["class"] not in ("tooltips_class32",)]
            menus = self.h.call("menus").get("menus") or []
            return {"visible": bool(ws or menus), "windows": sorted({w["class"] for w in ws}), "menus": len(menus)}
        return {"title": self.h.call("main")["title"], "dialogs": [d.get("title") for d in self.h.list("dialogs")]}

    def option_state(self, target):
        loc = self.loc_of(target)
        if "dialog_label" in loc:
            _pt, info = self.dialog_label_point(loc)
        else:
            _pt, info = self.resolve(target)
        hwnd = (info or {}).get("hwnd") if isinstance(info, dict) else None
        if not hwnd:
            raise OrigError(f"option {target}: no Win32 control to read its state")
        if info.get("class") and info.get("class") != "Button":
            raise OrigError(f"option {target}: control is a {info.get('class')}, not a check box")
        return bm_getcheck(int(hwnd))

    def menu_entry_state(self, path):
        parts = split_path(path)
        bar = self.r.ui_map["menu_bar"]["orig"]
        try:
            self.h.call("menu", path=parts[:-1], final="open", bar=bar, timeout=40)
            time.sleep(0.3)
            want = clean_label(parts[-1]).lower()
            items = [i for m in self.h.call("menus").get("menus") or [] for i in m.get("items") or [] if not i.get("separator")]
            hit = next((i for i in items if clean_label(i["text"]).lower() == want), None) \
                or next((i for i in items if clean_label(i["text"]).lower().startswith(want)), None)
            if not hit:
                raise OrigError(f"menu entry '{parts[-1]}' not found in the open menu")
            return {"text": hit["text"], "enabled": not hit["grayed"], "checked": hit["checked"]}
        finally:
            self.h.call("close_menus")

    def menu_items(self):
        out = []
        for m in self.h.call("menus").get("menus") or []:
            out.append([{"text": i["text"], "enabled": not i["grayed"], "checked": i["checked"], "submenu": i["submenu"]}
                        for i in m.get("items") or [] if not i.get("separator")])
        return out


class FpSide(Side):
    name = "fp"

    def __init__(self, runner, fp: Firepixel):
        super().__init__(runner)
        self.fp = fp
        self.doc_id = None

    def call(self, tool, args=None, timeout=60):
        return self.fp.call(tool, args or {}, timeout=timeout)[0]

    def prepare(self):
        if not self.fp.alive():
            self.fp.start()

    def reset_light(self):
        self.last_panel = None
        if getattr(self, "held", None):
            self.held = None
            try:
                self.inp([{"type": "up"}])
            except BridgeError:
                pass
        self.call("ui_action", {"close_dialogs": True})
        try:
            st = self.call("ui_state")
            if (st.get("file_dialog") or {}).get("open"):
                self.call("ui_file_dialog", {"action": "cancel"})
        except BridgeError:
            pass
        self.call("ui_input", {"events": [{"type": "key", "key": "Escape"}, {"type": "pointer_gone"}]})
        self.popup_rect = None
        self.ensure_panels_shown()
        if self.view_dirty:
            try:
                ch = self.script_value("(function(){var d = fw.getDocumentDOM(); " + self.view_reset_code() + " return ch.join(',');})()")
                if ch:
                    self.r.log(f"fp view switches reset: {ch}")
                self.view_dirty = False
            except BridgeError as e:
                self.r.log(f"fp view reset: {e}")

    def ensure_panels_shown(self):
        """R-4: panels hidden with Tab ('Bedienfelder ausblenden', xs-shortcuts/07) stayed hidden for the rest of the
        unit - every inspector field and panel button of xs-shortcuts/16, 21 (TW-71) was then missing. Shown again
        at unit start and before every step (the step that pressed Tab has taken its shot by then)."""
        try:
            st = self.state()
        except BridgeError:
            return False
        if not st.get("panels_hidden") or st.get("screen_mode") == 2:
            return False
        self.call("ui_action", {"panel": "window.properties"})
        time.sleep(0.2)
        self.r.log("fp panels were hidden (Tab) - shown again")
        return True

    def close_floating(self):
        """R-4: panel groups floating over the canvas (Info in panel-info-align-path/06_*, auto shape properties in
        tool-autoshapes/11) are switched off with their window command before a canvas shot - like the reference
        side, which closes its floating groups over the view. Docked panels (right of the canvas) stay."""
        st = self.state()
        doc = next((d for d in st.get("documents", []) if d.get("active")), None)
        if not doc:
            return 0
        ca = doc["canvas_area"]
        n = 0
        for name, r in (st.get("panel_rects") or {}).items():
            if name in ("window.properties", "properties") or not str(name).startswith("window."):
                continue
            inside = r[0] >= ca[0] + 2 and r[2] <= ca[2] - 2 and r[1] < ca[3] and r[3] > ca[1]
            if not inside:
                continue
            try:
                self.call("ui_action", {"run": name})
                n += 1
            except BridgeError as e:
                # R-5 (run 9: 'window.autoShapeProperties: unknown command', the group covered 38 % of the canvas
                # in tool-autoshapes/11): groups without a window command are closed with the '×' at the right end
                # of their title bar (docking.rs float_title: strip of `dock_strip_height` = 12 above the group
                # rect, close box = last strip height + 6 px)
                th = float(self.r.ui_map.get("fp_float_title_h", 12))
                pt = [r[2] - (th + 6) / 2, r[1] - th / 2]
                try:
                    self.inp([{"type": "click", "pos": pt}, {"type": "wait", "ms": 250}])
                    gone = name not in (self.state().get("panel_rects") or {})
                    self.r.log(f"fp floating panel {name}: {e} - closed with its title bar x at {pt}: {'ok' if gone else 'still open'}")
                    n += gone
                except BridgeError as e2:
                    self.r.log(f"fp floating panel {name}: {e}; x click: {e2}")
        if n:
            time.sleep(0.3)
            self.r.log(f"fp floating panels closed before the canvas shot: {n}")
        return n

    def open_testfile(self, path: Path):
        self.reset_light()
        self.view_dirty = False  # a freshly opened document has the default view switches
        docs = self.call("documents") or {}
        for d in (docs.get("documents") if isinstance(docs, dict) else []) or []:
            if str(d.get("path", "")).lower() == str(path).lower():
                self.call("documents", {"close": d.get("doc") or d.get("id"), "discard": True})
        v = self.call("open", {"path": str(path)})
        d = v.get("document", {})
        self.doc_id = d.get("doc")
        self.doc_size = (d.get("width", 0), d.get("height", 0))
        self.call("ui_action", {"zoom": 1.0})
        st = self.state()
        doc = next((x for x in st.get("documents", []) if x.get("active")), None)
        if doc:
            ca = doc["canvas_area"]
            if self.doc_size[0] > ca[2] - ca[0] or self.doc_size[1] > ca[3] - ca[1]:
                self.call("ui_action", {"fit": True})
                # R-8 (Q-48): the same zoom as the reference after its 'fit all' (grid/guide snap distances are
                # screen pixels / zoom); only downwards, so the page stays fully visible here too
                o = self.r.sides.get("orig") if getattr(self.r, "sides", None) else None
                oz = getattr(o, "fit_zoom", None) if o is not None else None
                try:
                    st2 = self.state()
                    d2 = next((x for x in st2.get("documents", []) if x.get("active")), None)
                    pz = (d2["page_on_screen"][2] - d2["page_on_screen"][0]) / float(self.doc_size[0]) if d2 else None
                except Exception:  # noqa: BLE001
                    pz = None
                if oz and pz and oz < pz - 0.002:
                    self.call("ui_action", {"zoom": float(oz)})
                self.r.log(f"fp fit: zoom {round(pz, 5) if pz else None}, reference {oz}"
                           f"{f' -> set to {oz}' if oz and pz and oz < pz - 0.002 else ''}")
        if getattr(self, "pending_colors", False):
            # R-9 (TW-107): unit start - the default colours here too (the reference sets them in its open_testfile)
            self.pending_colors = False
            try:
                before = self.tool_state()
                self.call("ui_action", {"run": "color.defaults"})
                after = self.tool_state()
                self.r.log(f"fp default colours: before fill {before.get('fill_color')} stroke {before.get('stroke_color')}"
                           f" - after fill {after.get('fill_color')} stroke {after.get('stroke_color')}")
            except Exception as e:  # noqa: BLE001
                self.r.log(f"fp default colours failed: {e}")

    def close_doc(self, path: Path):
        docs = self.call("documents") or {}
        for d in (docs.get("documents") if isinstance(docs, dict) else []) or []:
            if str(d.get("path", "")).lower() == str(path).lower():
                self.call("documents", {"close": d.get("doc") or d.get("id"), "discard": True})

    def state(self):
        return self.call("ui_state")

    def doc_to_win(self, x, y):
        st = self.state()
        doc = next((d for d in st["documents"] if d.get("active")), None)
        if not doc:
            raise BridgeError("no active document")
        l, t, r, b = doc["page_on_screen"]
        w, h = self.doc_size if self.doc_size[0] else (r - l, b - t)
        return [l + (r - l) * x / w, t + (b - t) * y / h]

    def doc_point_on_side(self, x, y):
        return self.doc_to_win(x, y)

    def round_point(self, x, y):
        return [x, y]

    def active_doc_state(self):
        return next((d for d in (self.state().get("documents") or []) if d.get("active")), None)

    def view_rect_now(self):
        """R-11: the active document's canvas area (window coordinates, like doc_to_win)."""
        try:
            doc = self.active_doc_state()
        except BridgeError:
            return None
        return list(doc["canvas_area"]) if doc and doc.get("canvas_area") else None

    def view_and_page(self):
        doc = self.active_doc_state()
        if not doc or not doc.get("canvas_area"):
            raise BridgeError("no active document view")
        return list(doc["canvas_area"]), (list(doc["page_on_screen"]) if doc.get("page_on_screen") else None)

    def dialog_open(self) -> bool:
        try:
            return bool(self.state().get("dialog_open"))
        except BridgeError:
            return False

    def find(self, text, role=None, exact=True, region=None, index=0):
        q = {"text": clean_label(text), "exact": exact}
        if role:
            q["role"] = role
        if region:
            q["region"] = region
        hits = []
        for k in range(4):
            # a panel shown by the previous action (tab click / menu) is laid out one or two frames later (RUN-6:
            # align buttons not found in the run, found in the single test)
            if k:
                time.sleep(0.4)
            q["exact"] = exact
            hits = (self.call("ui_find", q) or {}).get("hits") or []
            if not hits and exact:
                q["exact"] = False
                hits = (self.call("ui_find", q) or {}).get("hits") or []
            if len(hits) > index:
                break
        if len(hits) <= index:
            raise Unmapped(f"fp ui_find: '{text}' not found")
        return hits[index]

    def texts_right_of(self, hit):
        """Texts right of a menu entry at its height; compared before/after hovering, new texts = a sub menu opened
        (shortcut texts of the entry itself are there in both)."""
        r = hit["rect"]
        hits = (self.call("ui_find", {"region": [r[2] + 1, r[1] - 6, r[2] + 600, r[3] + 60], "limit": 40}) or {}).get("hits") or []
        return {(h.get("text"), round(h["rect"][0]), round(h["rect"][1])) for h in hits if h.get("text")}

    def window_size(self, w, h):
        """Firepixel window size at the configured position (None = configured size), never activated."""
        self.fp.resize(w, h)
        time.sleep(0.6)

    def script_eval(self, expr):
        return self.script_value(expr)

    def script_value(self, code):
        v = self.call("run_script", {"code": code})
        sc = v.get("script") if isinstance(v, dict) else None
        if isinstance(sc, dict):
            if sc.get("error") or sc.get("thrown"):
                raise BridgeError(f"run_script: {sc.get('error') or sc.get('thrown')}")
            out = sc.get("result")
            return out if not sc.get("notImplemented") else {"result": out, "notImplemented": sc["notImplemented"]}
        return v

    def find_role(self, role, index=0, in_dialog=True):
        hits = (self.call("ui_find", {"role": role, "limit": 200}) or {}).get("hits") or []
        # dialog.field.N etc. count only the controls of the open dialog (like the reference side's child
        # windows); otherwise fields of panels/inspector above the dialog came first (TW-14, runner finding 6)
        if in_dialog and self.state().get("dialog_open"):
            try:
                l, t, r, b = self.dialog_rect()
                hits = [h for h in hits if l - 1 <= h["center"][0] <= r + 1 and t - 1 <= h["center"][1] <= b + 1]
                # panel fields lying under the dialog were drawn before its title bar
                tb = [h for h in (self.call("ui_find", {"text": "Titelleiste", "exact": False, "limit": 20}) or {}).get("hits") or []
                      if str(h.get("text", "")).startswith("Titelleiste") and "order" in h]
                if tb:
                    o = max(h["order"] for h in tb)
                    after = [h for h in hits if h.get("order", o) >= o]
                    # sliders of the effect dialogs carry an earlier order than the title bar (RUN-6): keep the
                    # rectangle filter alone when the order filter leaves too few
                    hits = after if len(after) > index else hits
            except BridgeError:
                pass
        hits.sort(key=lambda h: (round(h["rect"][1] / 4), h["rect"][0]))
        if len(hits) <= index:
            raise Unmapped(f"fp: no {role} #{index}")
        return hits[index]

    def resolve(self, target):
        if isinstance(target, dict):
            g = self.grid_point(target)
            if g:
                return list(g)
            if "canvas" in target:
                return self.canvas_point(target["canvas"], allow_outside=bool(target.get("allow_outside")))
            if "window_rel" in target:
                return list(target["window_rel"])
            if "area" in target and "pos" in target:
                r = self.area_rect(target["area"])
                dx, dy = self.area_pos(target["area"], target["pos"])
                return [r[0] + dx, r[1] + dy]
            if "target" in target:
                return self.resolve(target["target"])
            if "dialog_field" in target:
                return self.find_role("text_input", int(target["dialog_field"]))["center"]
            if "field_arrow" in target:
                # popup slider arrow right next to the i-th text field of the dialog (not exposed by ui_find)
                r = self.find_role("text_input", int(target["field_arrow"]))["rect"]
                return [r[2] + 10, (r[1] + r[3]) / 2]
            if "button" in target:
                return self.find(target["button"], "button")["center"]
            if "text" in target:
                return self.find(target["text"])["center"]
            if "find" in target and "right_role" in target:
                # R-7 (xs-preferences): control right of its label on the same row (pick distance field)
                anchor = self.find(target["find"], target.get("role"), target.get("exact", True), None, target.get("index", 0))["rect"]
                hits = [h for h in (self.call("ui_find", {"role": target["right_role"], "limit": 200}) or {}).get("hits") or []
                        if h["rect"][0] >= anchor[2] - 4 and h["rect"][1] < anchor[3] + 4 and h["rect"][3] > anchor[1] - 4]
                if not hits:
                    raise Unmapped(f"fp: no {target['right_role']} right of '{target['find']}'")
                return min(hits, key=lambda h: h["rect"][0] - anchor[2])["center"]
            if "find" in target and "below_role" in target:
                # control right below a label/control (R-2, inspector-web/04: the slice's export format moved under
                # 'Typ' with T-1): the nearest `below_role` node under the anchor, else `fallback`
                try:
                    reg = self.area_rect("panel:" + target["panel"]) if target.get("panel") else None
                    anchor = self.find(target["find"], target.get("role"), target.get("exact", True), reg, target.get("index", 0))["rect"]
                    q = {"role": target["below_role"], "limit": 200}
                    if reg:
                        q["region"] = reg
                    hits = [h for h in (self.call("ui_find", q) or {}).get("hits") or []
                            if h["rect"][1] >= anchor[3] - 2 and h["rect"][2] > anchor[0] - 40]
                    if hits:
                        return min(hits, key=lambda h: (h["rect"][1] - anchor[3], abs(h["rect"][0] - anchor[0])))["center"]
                except (Unmapped, BridgeError):
                    pass
                if target.get("fallback"):
                    return self.resolve(target["fallback"])
                raise Unmapped(f"fp: no {target['below_role']} below '{target['find']}'")
            if "find" in target:
                reg = None
                if target.get("panel"):
                    reg = self.area_rect("panel:" + target["panel"])
                hit = self.find(target["find"], target.get("role"), target.get("exact", True), reg, target.get("index", 0))
                if "from_left" in target and hit.get("rect"):
                    # control without own node right of its label (colour well of the effect dialogs, RUN-6)
                    return [hit["rect"][0] + target["from_left"], (hit["rect"][1] + hit["rect"][3]) / 2]
                return hit["center"]
            if "menu_item" in target:
                return self.find(target["menu_item"], None, True, [0, 30, 10000, 10000])["center"]
            if "handle" in target:
                return self.handle_point(target["handle"])
            if "preview_pane" in target:
                return self.find(f"Vorschaufeld {int(target['preview_pane']) + 1}")["center"]
            if "dialog_rel" in target:
                d = self.area_rect("dialog")
                return [d[0] + target["dialog_rel"][0], d[1] + target["dialog_rel"][1]]
            if "popup_swatch" in target:
                # swatch grid of the colour popup: above its HEX field, found by its colour in the window image
                hexf = [h for h in (self.call("ui_find", {"text": "#", "exact": False, "role": "text_input", "limit": 20}) or {}).get("hits") or []
                        if str(h.get("text", "")).startswith("#")]
                img = self.window_image()[0]
                if hexf:
                    hr = hexf[0]["rect"]
                    reg = [hr[0] - 160, hr[1] - 230, hr[2] + 40, hr[1] - 4]
                else:
                    reg = [0, 0, img.width, img.height]
                pt = find_color_cell(img, reg, str(target["popup_swatch"]))
                if not pt:
                    raise BridgeError(f"popup swatch {target['popup_swatch']} not found")
                return [pt[0], pt[1]]
            for k in ("layer_eye", "layer_lock", "layer_expand"):
                if k in target:
                    # columns of the layers panel row (measured 14.10.): eye and lock at fixed x, the triangle left of the name
                    c = self.resolve({"layer_row": target[k]})
                    hit = self.find(target[k], None, True, None)
                    pr = self.area_rect("panel:layers")
                    x = {"layer_eye": pr[0] + 17, "layer_lock": pr[0] + 38}.get(k, hit["rect"][0] - 28)
                    return [x, c[1]]
            for k in ("layer_row", "object_row", "page_row", "state_row", "history_row", "list_row", "tree_item", "pref_category",
                      "layer_name", "page_name"):
                if k in target:
                    v = target[k]
                    v = v[-1] if isinstance(v, list) else v
                    if isinstance(v, int) and k == "object_row":
                        # n-th object (top first) of the layer `layer` (or of the current layer): its name
                        lay = json.dumps(target.get("layer"))
                        v = self.script_value("(function(){var d = fw.getDocumentDOM(), f = d.currentFrameNum, n = %d, lay = %s;"
                                              "for (var i = d.layers.length - 1; i >= 0; i--) { var L = d.layers[i];"
                                              " if (lay !== null && L.name != lay) continue; var es = L.frames[f].elements;"
                                              " if (es.length > n) return es[n].name; } return null;})()" % (v, lay))
                        if not v:
                            raise Unmapped(f"fp object_row {target[k]} not found")
                    region = None
                    if k in ("list_row", "tree_item") and getattr(self, "last_panel", None):
                        # the same name may be a layer/object row too (panel-library-styles/04: 'Grafik1' in the
                        # layers list started a rename): look inside the panel shown last (TW-59 `panel_rects`)
                        region = (self.state().get("panel_rects") or {}).get(self.last_panel)
                    def at(hit):
                        # {"list_row": name, "icon": true}: the row's icon left of the name (R-4, panel-library-styles/
                        # 04: since FP1-05 a double click on the name starts renaming, on the icon it opens the
                        # properties). `icon_dx` = distance of the icon centre from the name's left edge (default 12)
                        if target.get("icon"):
                            return [hit["rect"][0] - float(target.get("icon_dx", 12)), hit["center"][1]]
                        return hit["center"]
                    try:
                        if region:
                            try:
                                return at(self.find(v, None, True, region))
                            except Unmapped:
                                pass
                        return at(self.find(v))
                    except Unmapped:
                        if k not in ("layer_row", "object_row", "layer_name"):
                            raise
                    # the row is scrolled out of the (small) layers panel: wheel through the list and look again
                    pr = self.area_rect("panel:layers")
                    pos = [(pr[0] + pr[2]) / 2, (pr[1] + pr[3]) / 2]
                    for d in [3] * 6 + [-3] * 12:
                        self.inp([{"type": "wheel", "pos": pos, "delta": [0, -d], "unit": "line"}])
                        time.sleep(0.15)
                        hits = [h for h in (self.call("ui_find", {"text": clean_label(str(v)), "exact": True}) or {}).get("hits") or []
                                if h["center"][0] >= pr[0] and h["center"][1] >= pr[1]]
                        if hits:
                            return hits[0]["center"]
                    raise Unmapped(f"fp ui_find: '{v}' not found (layers panel scrolled through)")
            raise Unsupported(f"fp target form {list(target)}")
        if isinstance(target, str) and re.match(r"^dialog\.(field|combo|check|button|slider)\.\d+$", target):
            kind, i = target.split(".")[1:]
            role = {"field": "text_input", "combo": "combo_box", "check": "check_box", "button": "button", "slider": "slider"}[kind]
            return self.find_role(role, int(i))["center"]
        if isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            return self.resolve(self.ui(target))
        if isinstance(target, str) and re.match(r"^[a-z_]+(\.[a-z0-9_]+)+$", target):
            raise Unmapped(f"ui key not in ui_map.json: {target}")
        return self.find_in_popup(str(target))["center"]

    LAYER_TARGETS = ("layer_eye", "layer_lock", "layer_expand", "layer_row", "layer_name")

    def layer_drag_points(self, a_from, a_to, fr, to):
        """K-4 (TW-85, panel-layers/04): both ends of a drag between rows of the layers panel are resolved one after
        the other, and resolving the second one may scroll the list (the row was out of view), so the first point
        then lies on another row. Scroll the list line by line until both rows are visible at once and take both
        points again; unchanged when the ends are no layer rows or do not fit into the list together."""
        def name_of(t):
            if not isinstance(t, dict):
                return None
            k = next((k for k in self.LAYER_TARGETS if k in t), None)
            return (k, t[k]) if k else None
        nf, nt = name_of(a_from), name_of(a_to)
        if not nf or not nt:
            return fr, to
        pr = self.area_rect("panel:layers")
        pos = [(pr[0] + pr[2]) / 2, (pr[1] + pr[3]) / 2]

        def visible(name):
            hits = [h for h in (self.call("ui_find", {"text": clean_label(str(name)), "exact": True}) or {}).get("hits") or []
                    if pr[0] <= h["center"][0] <= pr[2] and pr[1] <= h["center"][1] <= pr[3]]
            return hits[0]["center"] if hits else None

        def both():
            return visible(nf[1]) is not None and visible(nt[1]) is not None

        if not both():
            # the second end is in view: scroll towards the first one (up when it lies above, else down)
            above = visible(nt[1]) is not None and fr[1] <= to[1]
            for _ in range(40):
                self.inp([{"type": "wheel", "pos": pos, "delta": [0, 1 if above else -1], "unit": "line"}])
                time.sleep(0.1)
                if both():
                    break
            else:
                self.r.log("layer drag: both rows do not fit into the layers list")
                return fr, to
        return self.resolve(a_from), self.resolve(a_to)

    def tight_popup(self, before, img, bb):
        """R-6: box of the open menu (with sub menus) inside the difference box `bb`. Menu entries are painted
        texts (no accessibility node) sharing a left edge (>= 3 per column); the changed region touching them is
        taken (popup_component, 3 px cells), its left edge is the menu frame found left of the first column."""
        # R-12 (TW-124 b/f): all painted texts (menu entries with a widget name included), topmost block only - the
        # panel columns under a context menu made a seed beside the menu and an inverted box (panel-optimize/13)
        hits = (self.call("ui_find", {"region": list(bb), "role": "text", "limit": 500}) or {}).get("hits") or []
        hits = top_paint_block([h for h in hits if h.get("text") and h.get("source") != "accesskit"])
        cols: dict = {}
        for h in hits:
            col = next((c for c in cols if abs(c - h["rect"][0]) <= 2), h["rect"][0])
            cols.setdefault(col, []).append(h["rect"])
        cols = {c: rs for c, rs in cols.items() if len(rs) >= 3}
        if not cols:
            return None
        seed = bbox_union([r for rs in cols.values() for r in rs])
        comp = popup_component(before, img, seed=seed, cell=3, pad=1)
        if not comp:
            return None
        first = cols[min(cols)]
        row = sorted(first, key=lambda r: r[1])[0]
        y = int((row[1] + row[3]) / 2)
        x0 = int(min(cols)) - 3
        # walk left over the menu background to its frame (not over the highlighted entry: first row)
        fx, _ = scan_edge(img, x0, y, -1, 0, thr=30)
        left = max(comp[0], fx - 2)
        if left >= seed[0] or left >= comp[2] - 4 or comp[1] >= comp[3] - 4:
            return None
        return [left, comp[1], comp[2], comp[3]]

    def find_in_popup(self, text):
        """A text inside the popup opened last in this step (panel menu, R-4) first, then anywhere below the menu
        bar. The popup area grows by the sub menus opened from it."""
        pr = getattr(self, "popup_rect", None)
        if pr:
            # R-5 (panel-library-styles/11): the styles panel's footer button 'Neuer Stil' lies under the menu entry
            # 'Neuer Stil…' and matched the cleaned text exactly, so the click made a style without the dialog. In
            # the popup an entry with the ellipsis (when asked with one) and a menu entry wins over a button
            want = str(text).replace("&", "").strip()
            ell = want.endswith(("...", "…"))
            base = clean_label(want)
            def rank(h):
                t = str(h.get("text") or "").replace("&", "").strip()
                has_ell = t.endswith(("...", "…"))
                return (0 if clean_label(t) == base else 1, 0 if has_ell == ell else 1,
                        0 if "button" not in str(h.get("role") or "") else 1)
            for region in ([pr[0] - 2, pr[1] - 2, pr[2] + 2, pr[3] + 2],      # the popup itself
                           [pr[2] - 4, pr[1] - 40, pr[2] + 400, pr[3] + 300]):  # a sub menu right of it
                hits = (self.call("ui_find", {"text": base, "exact": False, "region": region, "limit": 40}) or {}).get("hits") or []
                hits = [h for h in hits if clean_label(str(h.get("text") or "").replace("&", "")).startswith(base)]
                if hits:
                    best = min(hits, key=rank)
                    if len(hits) > 1:
                        self.r.log(f"fp popup text '{text}': {len(hits)} hits, took '{best.get('text')}' ({best.get('role')})")
                    return best
                try:
                    return self.find(text, None, True, region)
                except Unmapped:
                    pass
        return self.find(text, None, True, [0, 30, 10000, 10000]) if pr else self.find(text)

    def area_rect(self, area):
        st = self.state()
        W, H = st["window"]["width"], st["window"]["height"]
        doc = next((d for d in st.get("documents", []) if d.get("active")), None)
        if area == "window":
            return [0, 0, W, H]
        if area == "canvas":
            if not doc:
                raise BridgeError("no document")
            return doc["canvas_area"]
        if area == "inspector":
            area = "panel:properties"
        if area.startswith("panel:"):
            return self.panel_rect(self.panel_info(area[6:]), st, doc)
        if area == "dialog":
            return self.dialog_rect(st)
        if area == "popup" and isinstance(st.get("popup"), dict) and st["popup"].get("rect"):
            # R-6: an open list of an inspector field is reported by the bridge (`popup.rect`)
            return [round(v) for v in st["popup"]["rect"]]
        if area == "popup" and st.get("dialog_open"):
            try:
                return self.dialog_rect(st)
            except BridgeError:
                return None
        if area in ("menu", "popup"):
            return None  # difference box
        raise Unsupported(f"shot area {area}")

    def canvas_shot_rect(self):
        try:
            self.close_floating()
        except BridgeError as e:
            self.r.log(f"fp close floating panels: {e}")
        st = self.state()
        doc = next((d for d in st.get("documents", []) if d.get("active")), None)
        if not doc:
            raise BridgeError("no document")
        view = doc["canvas_area"]
        page = doc.get("page_on_screen")
        return self.crop_to_page(page, view) if page else view

    def dialog_rect(self, st=None):
        """Whole dialog incl. close button and button row. The accessibility node 'Titelleiste <title>' covers
        only the title text part; the frame is found in the window image (scan from the title bar to the frame
        line). The in-app file dialog has no title bar: anchored at its title text."""
        st = st or self.state()
        W, H = st["window"]["width"], st["window"]["height"]
        img = self.window_image()[0]
        tb = [h for h in (self.call("ui_find", {"text": "Titelleiste", "exact": False, "limit": 20}) or {}).get("hits") or []
              if str(h.get("text", "")).startswith("Titelleiste")]
        if tb:
            l, t, r, b = tb[0]["rect"]
            y = t + 3
            left = scan_edge(img, l + 4, y, -1, 0)[0]
            right = scan_edge(img, r - 2, y, 1, 0)[0]
            top = scan_edge(img, l + 4, y, 0, -1)[1]
            # along both side borders (a control or page header may touch one of them): the longer run wins
            bottom = max(scan_edge(img, left + 3, b + 4, 0, 1)[1], scan_edge(img, right - 3, b + 4, 0, 1)[1])
        else:
            fd = (st.get("file_dialog") or {}).get("open") or {}
            title = fd.get("title")
            dr = (st.get("dialog") or {}).get("rect") if isinstance(st.get("dialog"), dict) else None
            if dr and not title and dr[2] - dr[0] > 20 and dr[3] - dr[1] > 20:
                # R-5: no title bar node, but the bridge reports the dialog's own rectangle (K-2)
                return [max(0, dr[0] - 1), max(0, dr[1] - 1), min(W, dr[2] + 2), min(H, dr[3] + 2)]
            if not (st.get("dialog_open") and title):
                raise BridgeError(f"no dialog window found (dialog_open={st.get('dialog_open')}, message={bool(st.get('message'))})")
            tr = self.find(title, None, True)["rect"]
            left = scan_edge(img, tr[0] - 1, tr[1] - 2, -1, 0)[0]
            right = scan_edge(img, tr[0] - 1, tr[1] - 2, 1, 0)[0]
            top = scan_edge(img, tr[0] - 1, tr[1] - 2, 0, -1)[1]
            # bottom: below the first OK/Cancel button pair (the frame line is too close to the texts to scan)
            ok = [h["rect"] for h in (self.call("ui_find", {"region": [left, top, right, H], "role": "button", "limit": 40}) or {}).get("hits") or []
                  if clean_label(h.get("text") or "") in ("OK", "Abbrechen", "Öffnen", "Speichern")]
            bottom = (min(x[3] for x in ok) + 4) if ok else scan_edge(img, left + 1, tr[3] + 1, 0, 1)[1]
            return [max(0, left - 1), max(0, top - 1), min(W, right + 2), min(H, bottom + 2)]
        # never cut off the dialog's own controls (buttons at the bottom/right, inside the dialog's height)
        ins = (self.call("ui_find", {"region": [left, top, right + 200, min(H, bottom + 40)], "role": "button", "limit": 200}) or {}).get("hits") or []
        btn = [h["rect"] for h in ins if h["rect"][0] >= left - 2 and h["rect"][0] < right and h["rect"][1] < bottom + 30]
        if btn:
            right = max(right, max(x[2] for x in btn) + 4)
            bottom = max(bottom, max(x[3] for x in btn) + 4)
        return [max(0, left - 1), max(0, top - 1), min(W, right + 2), min(H, bottom + 2)]

    def floating_panel_rect(self, tab_rect):
        """Panel group floating over the canvas: its frame from the window image (from the tab to the frame line)."""
        img = self.window_image()[0]
        l, t, r, b = tab_rect
        left = scan_edge(img, l - 2, (t + b) / 2, -1, 0)[0]
        right = scan_edge(img, r + 8, t - 1, 1, 0)[0]
        top = scan_edge(img, l - 2, (t + b) / 2, 0, -1)[1]
        # bottom: scan near both side borders (controls may touch one of them), the longer run wins
        bottom = max(scan_edge(img, left + 2, b + 6, 0, 1)[1], scan_edge(img, right - 3, b + 6, 0, 1)[1])
        return [left - 1, top - 1, right + 2, bottom + 2]

    def panel_rect(self, info, st, doc):
        W, H = st["window"]["width"], st["window"]["height"]
        ca = doc["canvas_area"] if doc else [60, 100, W - 280, H - 120]
        if info.get("title") is None:  # tools column
            return [0, ca[1] - 4, ca[0], H]
        # the bridge reports the drawn rect of each front panel (tab strip + content of its own group, TW-59)
        pr = (st.get("panel_rects") or {}).get(info.get("fp") or "")
        if pr and pr[2] - pr[0] > 20 and pr[3] - pr[1] > 10:
            # R-6 (TW-84, run 10 panel-misc/16): the rect of a later tab starts at its own tab (x 152), the group
            # at the first tab of the strip (x 64, like the reference shot with "Eigenschaften"): other panel tabs
            # on the same tab line left of it move the left edge
            try:
                titles = {v["title"] for v in self.r.ui_map.get("panels", {}).values() if isinstance(v, dict) and v.get("title")}
                hs = (self.call("ui_find", {"region": [max(0, pr[0] - 400), pr[1], pr[0] + 2, pr[1] + 24], "limit": 50})
                      or {}).get("hits") or []
                left = [h["rect"][0] for h in hs if h.get("text") in titles and h["rect"][2] <= pr[0] + 2]
                if left:
                    return [max(0, min(left) - 10), pr[1], pr[2], pr[3]]
                # R-7 (run 11, inspector-group-symbol/04: no tab text found, the shot still began at x 152 without
                # the 'Name' column): the group's left frame from the window image - walk left along the panel's
                # content (60 % of its height) until the frame line; only a move of 8…400 px counts
                img = self.window_image()[0]
                y = int(pr[1] + (pr[3] - pr[1]) * 0.6)
                gx = scan_edge(img, pr[0] + 2, y, -1, 0, thr=24)[0]
                if 8 <= pr[0] - gx <= 400:
                    return [max(0, gx - 1), pr[1], pr[2], pr[3]]
            except Exception:  # noqa: BLE001
                pass
            return [pr[0], pr[1], pr[2], pr[3]]
        tab = self.find(info["title"], None, True)
        r = tab["rect"]
        titles = {v["title"] for v in self.r.ui_map.get("panels", {}).values() if isinstance(v, dict) and v.get("title")}
        cx = (r[0] + r[2]) / 2
        if cx > ca[2]:
            col = [ca[2], W]
        elif cx < ca[0]:
            col = [0, ca[0]]
        else:
            col = [0, ca[2]]
        below = []
        allh = (self.call("ui_find", {"role": "text", "region": [col[0], r[3] + 20, col[1], H], "limit": 500}) or {}).get("hits") or []
        cand = [h for h in allh if h.get("text") in titles and h["rect"][1] > r[3] + 20]
        for h in cand:
            # a tab row of the next group: starts at the column's left edge or has another tab on its line
            # (object rows named like a panel, e.g. "Pfad" in the layers list, are no group border)
            same_line = [o for o in cand if o is not h and abs(o["rect"][1] - h["rect"][1]) < 4]
            if h["rect"][0] <= col[0] + 30 or same_line:
                below.append(h["rect"][1])
        bottom = min(below) - 4 if below else H
        if cx >= ca[0] and cx <= ca[2]:
            # floating over the canvas: the group's own frame (not the whole canvas column)
            try:
                fr = self.floating_panel_rect(r)
                if fr[2] - fr[0] > 60 and fr[3] - fr[1] > 40:
                    return fr
            except Exception:  # noqa: BLE001
                pass
            return [ca[0] - 26, r[1] - 4, ca[2] + 12, H]
        return [col[0] + 1, r[1] - 4, col[1], bottom]

    # R-9 (Q-50): optional key-state / fill fields of the bridge's ui_state (F28 adds one; read when present)
    STATE_OPTIONAL = ("modifiers", "held_keys", "keys_held", "keys", "fill_mode", "fill_kind", "fill_category", "stroke_kind")

    def tool_state(self):
        """R-9 (TW-107, Q-50): active tool, fill and stroke colour (and key state / fill kind when the bridge has them)."""
        try:
            st = self.state()
            out = {k: st.get(k) for k in ("tool", "fill_color", "stroke_color")}
            out.update({k: st[k] for k in self.STATE_OPTIONAL if k in st})
            return out
        except Exception as e:  # noqa: BLE001
            return {"error": str(e)[:80]}

    def grab(self, rect, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        args = {"max_size": 8192}
        # R-12 (TW-124 f): swapped corners are normalised first (they fell back to the whole window)
        rect = norm_rect(rect)
        if rect and (float(rect[2]) - float(rect[0]) < 1 or float(rect[3]) - float(rect[1]) < 1):
            # R-9 (run 13 toolbox/14,16: 'screenshot: empty region'): an empty area falls back to the whole window
            self.r.log(f"fp screenshot: empty region {[round(float(v), 1) for v in rect]} - whole window instead")
            rect = None
        if rect:
            args["rect"] = [float(v) for v in rect]
        try:
            _, imgs = self.fp.call("screenshot", args, timeout=60)
        except BridgeError as e:
            if "empty region" not in str(e) or "rect" not in args:
                raise
            self.r.log(f"fp screenshot: {e} for {args['rect']} - whole window instead")
            _, imgs = self.fp.call("screenshot", {"max_size": 8192}, timeout=60)
        if not imgs:
            raise BridgeError("screenshot returned no image")
        with Image.open(io.BytesIO(imgs[0])) as im:
            im.save(path)

    def window_image(self):
        _, imgs = self.fp.call("screenshot", {"max_size": 8192}, timeout=60)
        with Image.open(io.BytesIO(imgs[0])) as im:
            img = im.convert("RGB")
        return img, [0, 0, img.width, img.height]

    def inp(self, events, **extra):
        return self.call("ui_input", dict({"events": events}, **extra))

    def key(self, combo):
        mods, k = parse_combo(combo)
        if set(mods) >= {"ctrl", "alt"} and k == "space":
            raise Unsupported("Ctrl+Alt+Space is forbidden")
        name = EGUI_KEYS.get(k, k.upper() if len(k) == 1 else k[:1].upper() + k[1:])
        self.inp([{"type": "key", "key": name, "modifiers": mods}])

    def select(self, names):
        """Exactly the named objects, chosen like in the reference program (JSF fw.selection by name on the
        current state of all layers); the bridge's `select {ids}` is only the fallback."""
        names = select_names(names)
        code = ("(function(){var d = fw.getDocumentDOM(); var want = %s; var sel = [], cnt = 0, sc = 0;"
                "for (var i = 0; i < d.layers.length; i++) { var fr = d.layers[i].frames[d.currentFrameNum]; var web = d.layers[i].layerType == 'web';"
                "  for (var j = 0; j < fr.elements.length; j++) { var e = fr.elements[j];"
                "    cnt++; if (web) sc++; for (var k = 0; k < want.length; k++) if (e.name == want[k] || (want[k] !== null && typeof want[k] == 'object' && (want[k].index == cnt - 1 || (web && want[k].slice == sc - 1)))) sel.push(e); } }"
                "d.selectNone(); fw.selection = sel; return [sel.length, fw.selection.length].join(',');})()") % json.dumps(names, ensure_ascii=True)
        try:
            v = str(self.script_value(code))
            n, got = (v.split(",") + ["0"])[:2]
            if int(n) > 0 and int(got) == int(n):
                return
        except (BridgeError, ValueError):
            pass
        doc = self.call("get_document", {"depth": 6})
        ids = []

        def walk(els):
            for e in els or []:
                if e.get("name") in names:
                    ids.append(e["id"])
                walk(e.get("children"))

        for pg in doc.get("pages", []):
            if pg.get("index", 0) != doc.get("currentPage", 0):
                continue
            for ly in pg.get("layers", []):
                walk(ly.get("elements"))
                for sub in ly.get("sublayers", []) or []:
                    walk(sub.get("elements"))
        if not ids:
            raise BridgeError(f"objects not found: {names}")
        self.call("select", {"ids": ids})

    def do(self, verb, arg, act):
        r = self.r
        if verb in ("open_testfile", "reopen"):
            r.open_testfile(self, arg)
        elif verb in ("close_all", "close_all_documents"):
            # every open document, also the ones a script opened itself (file dialog), runner finding 5
            for _ in range(3):
                docs = (self.call("documents") or {}).get("documents") or []
                if not docs:
                    break
                for d in docs:
                    self.call("documents", {"close": d.get("doc") or d.get("id"), "discard": True})
            left = (self.call("documents") or {}).get("documents") or []
            if left:
                raise BridgeError(f"close_all: {len(left)} documents still open")
            r.opened[self.name] = set()
        elif verb in ("menu", "menu_open", "hover_menu", "menu_item"):
            parts = split_path(arg)
            h = self.find(parts[0], None, True, [0, 0, 2000, 40])
            self.inp([{"type": "click", "pos": h["center"]}, {"type": "wait", "ms": 150}])
            for i, p in enumerate(parts[1:], 1):
                hit = self.find(p, None, True, [0, 30, 10000, 10000])
                last = i == len(parts) - 1
                if last and verb == "menu_item":
                    self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 200}])
                else:
                    before = self.texts_right_of(hit) if last and verb == "menu" else None
                    self.inp([{"type": "move", "pos": hit["center"]}, {"type": "wait", "ms": 350}])
                    if last and verb == "menu" and not (self.texts_right_of(hit) - before):
                        # a leaf entry at the end of `menu` is executed (like the reference side)
                        self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 250}])
        elif verb == "command":
            self.call("ui_action", {"run": arg} if isinstance(arg, str) else arg)
        elif verb == "close_dialogs":
            self.reset_light()
        elif verb == "key":
            for _ in range(int(act.get("repeat", 1) or 1)):
                self.key(arg)
        elif verb == "type":
            self.inp([{"type": "type", "text": str(arg)}])
        elif verb == "wait":
            time.sleep(float(arg if not isinstance(arg, dict) else arg.get("ms", 0)) / 1000)
        elif verb == "select":
            self.select(arg)
        elif verb == "select_none":
            self.call("select", {"none": True})
        elif verb == "select_all":
            self.call("select", {"all": True})
        elif verb == "tool":
            t = arg if str(arg).startswith("tool.") else f"tool.{arg}"
            self.call("ui_action", {"tool": t})
        elif verb == "panel":
            info = self.panel_info(arg)
            pid = info.get("fp") or arg
            self.last_panel = pid
            st = self.state()
            if pid not in st.get("visible_panels", []):
                try:
                    self.call("ui_action", {"panel": pid})
                except BridgeError:
                    # panels without an engine command (Ausrichten, Pfad, Farbpalette, Sonderzeichen, Bildbearbeitung,
                    # Symboleigenschaften …) are unknown to `ui_action panel` and missing in `visible_panels`: bring
                    # its tab to the front when it is there, else use its menu entry (shows it)
                    title = info.get("title")
                    probe = info.get("fp_probe")
                    if probe and ((self.call("ui_find", {"text": probe, "exact": True, "limit": 2}) or {}).get("hits")):
                        # already in front: clicking the active tab again would fold the group (RUN-6, align buttons
                        # not found in every second step)
                        return
                    tabs = [h for h in (self.call("ui_find", {"text": title, "exact": True, "limit": 20}) or {}).get("hits") or []
                            if title and h.get("text") == title and h["rect"][1] > 60] if title else []
                    if tabs:
                        t = min(tabs, key=lambda h: h["rect"][1])
                        self.inp([{"type": "click", "pos": t["center"], "button": "left"}])
                        time.sleep(0.3)
                    elif info.get("menu"):
                        self.do("menu", info["menu"], {"toggle": True})
                    else:
                        raise
        elif verb in ("jsf", "run_script"):
            code = arg if isinstance(arg, str) else arg.get("code")
            self.values.append({"jsf": code[:200], "out": self.script_value(code)})
        elif verb in ("log_value", "log"):
            expr = arg if isinstance(arg, str) else arg.get("value") or arg.get("expr")
            if isinstance(arg, dict) and not expr:
                raise Unsupported(f"log {arg}")
            # R-12 (TW-124 a, run 16 xs-shortcuts/06): the bridge returns String(value), an object came back as
            # "[object Object]"; objects/arrays are serialised in the script like the reference side (toSource)
            self.values.append({"expr": expr, "value": self.script_value(fp_serialised_js(expr))})
        elif verb in ("click", "double_click", "right_click", "hover", "mouse_down", "mouse_up", "click_text",
                      "double_click_text", "right_click_text", "hover_text", "click_button", "toolbox_click",
                      "open", "canvas_click", "canvas_double_click", "canvas_move", "canvas_hover", "window_click",
                      "ui_find_click"):
            self.pointer(verb, arg, act)
        elif verb in ("drag", "canvas_drag"):
            a = arg
            fr = self.resolve({"canvas": a["from"]} if verb == "canvas_drag" or isinstance(a.get("from"), list) else a["from"])
            if "to" in a:
                # R-12 (TW-124 d): `allow_outside_end` - the end point may lie outside the document view (xs-canvas/10
                # drags a guide onto the ruler); the start point is checked as before
                out_end = bool(a.get("allow_outside_end"))
                to = self.resolve({"canvas": a["to"], "allow_outside": out_end}
                                  if verb == "canvas_drag" or isinstance(a.get("to"), list) else a["to"])
            elif "by" in a:
                to = [fr[0] + a["by"][0], fr[1] + a["by"][1]]
            else:
                raise Unsupported("drag without to/by")
            fr, to = self.layer_drag_points(a.get("from"), a.get("to"), fr, to)
            # R-4: press, then moves with waits (each a frame of its own), one more move at the target, release.
            # TW-85 (panel-layers/04 eye drag): panel lists evaluate a drag frame by frame, the bridge `drag` event
            # arrived without frames in between. RE-6 (re 36): the reference program takes only the second-to-last
            # position of a drag; its host now waits 60 ms and moves once more at the target before releasing -
            # the same sequence here, so both sides end on the same position
            # (move/down/up carry no modifiers in the bridge: held keys are set with a `modifiers` event around it)
            # R-6 (TW-97, run 10: Alt drag without duplicate): a `modifiers` event holds only for its own frame - the
            # next frame starts with the window's real (empty) key state, the moves of the drag came without Alt.
            # The held keys are now sent again right before every pointer event (each its own frame)
            mods = norm_mods(a.get("mods") or a.get("hold_key"))
            held_keys = [m for m in mods if m not in HELD_MODS]  # R-9: Space drag (hand) holds a plain key
            mods = [m for m in mods if m in HELD_MODS]
            n = max(4, int(a.get("steps", 20)))
            hold = [{"type": "modifiers", "modifiers": mods}] if mods else []
            ev = hold + [{"type": "move", "pos": list(fr)}, {"type": "wait", "ms": 60}] + hold + [{"type": "down"}, {"type": "wait", "ms": 60}]
            for i in range(1, n + 1):
                ev += hold + [{"type": "move", "pos": [fr[0] + (to[0] - fr[0]) * i / n, fr[1] + (to[1] - fr[1]) * i / n]}]
                ev.append({"type": "wait", "ms": 25})
            ev += [{"type": "wait", "ms": 60}] + hold + [{"type": "move", "pos": list(to)}, {"type": "wait", "ms": 30}]
            # the keyboard menu counts a held-key frame followed by a frame without it as "Alt alone" and switches
            # the menu bar on/off (each move frame); an even number of such pairs leaves it off after the drag
            if mods and (n + 2) % 2:
                ev += hold + [{"type": "move", "pos": list(to)}, {"type": "wait", "ms": 30}]
            ev += hold + [{"type": "up"}]
            rec = {"from_win": [round(v, 2) for v in fr], "to_win": [round(v, 2) for v in to], "mods": mods}
            if verb == "canvas_drag" and self.doc_size[0]:
                # R-8 (Q-48): zoom of this side next to the reference's `zoom`
                try:
                    d2 = next((x for x in self.state().get("documents", []) if x.get("active")), None)
                    if d2:
                        rec["zoom"] = round((d2["page_on_screen"][2] - d2["page_on_screen"][0]) / float(self.doc_size[0]), 5)
                except Exception:  # noqa: BLE001
                    pass
            if held_keys:
                ev = [{"type": "key", "key": egui_key(k), "action": "press"} for k in held_keys] + [{"type": "wait", "ms": 60}] \
                    + ev + [{"type": "key", "key": egui_key(k), "action": "release"} for k in held_keys]
                rec["held_keys"] = held_keys
            canvas = verb == "canvas_drag" or isinstance(a.get("from"), list)
            if canvas:
                rec["state_before"] = self.tool_state()  # R-9 (TW-107/109, Q-50): tool, fill, stroke before the drag
            self.values.append({"drag": rec})
            if mods:
                ev.append({"type": "modifiers", "modifiers": []})
            ev.append({"type": "wait", "ms": 250})
            self.inp(ev)
            if canvas:
                rec["state_after"] = self.tool_state()
        elif verb == "choose":
            a = arg
            if a.get("target"):
                self.pointer("open", a["target"], {})
            for part in split_path(a.get("item", "")):
                hit = None
                for name in [part] + self.aliases(part) + self.combo_aliases(part):
                    try:
                        hit = self.find(name)
                        break
                    except Unmapped:
                        continue
                if hit is None:
                    # loose match over the texts below the menu bar ("PNG 32" / "PNG32", R-2)
                    texts = [h for h in (self.call("ui_find", {"region": [0, 30, 10000, 10000], "limit": 600}) or {}).get("hits") or []
                             if h.get("text")]
                    want = match_item([h["text"] for h in texts], part, self.aliases(part) + self.combo_aliases(part))
                    hit = next((h for h in sorted(texts, key=lambda h: -(h.get("order") or 0)) if h["text"] == want), None) if want else None
                if hit is None:
                    raise Unmapped(f"fp ui_find: '{part}' not found")
                self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 200}])
                # R-8 (TW-102: PNG 8/24 exported as PNG 32): the hit that was clicked and, for the format field, the
                # export options right after the choice
                rec = {"item": part, "hit": hit.get("text"), "at": hit.get("center")}
                if a.get("target") in ("optimize.format", "preview.format"):
                    for k, e in EXPORT_PROBES:
                        try:
                            rec[k] = self.script_value(e)
                        except Exception as ex:  # noqa: BLE001
                            rec[k] = f"error: {str(ex)[:60]}"
                self.values.append({"choose": rec})
        elif verb in ("set_field", "set", "set_dialog_field"):
            a = dict(act, **arg) if isinstance(arg, dict) else act
            self.set_field(a, a.get("target") or (arg if isinstance(arg, str) else None))
        elif verb == "record_ui":
            self.values.append({"record_ui": arg, "value": self.record_ui(arg)})
        elif verb == "log_menu_items":
            self.values.append({"menu_items": self.menu_items()})
        elif verb == "clear_dir":
            r.clear_dir(arg)
        elif verb in ("record_dir", "record_file", "record_file_text"):
            self.values.append({verb: arg, "value": r.record_path(verb, arg)})
        elif verb in ("file_dialog", "file_dialog_choose"):
            path = arg if isinstance(arg, str) else arg.get("path")
            self.call("ui_file_dialog", {"action": "choose", "path": path})
        elif verb == "file_dialog_cancel":
            self.call("ui_file_dialog", {"action": "cancel"})
        elif verb in ("ui_action", "ui_find", "ui_state", "get_document", "ui_file_dialog"):
            v = self.call(verb, arg if isinstance(arg, dict) else {})
            if verb != "ui_action":
                self.values.append({verb: arg, "value": v})
        elif verb == "export":
            a = export_spec(arg)
            d = r.safe_dir(a["dir"])
            d.mkdir(parents=True, exist_ok=True)
            fmt = "PNG"
            try:
                v = self.script_value("fw.getDocumentDOM().exportOptions.exportFormat")
                if isinstance(v, str) and v.strip():
                    fmt = v.strip().upper()
            except Exception:
                pass
            # R-8 (TW-102/103): colour depth and number of states before the export, next to the format
            opts = {}
            for k, e in EXPORT_PROBES:
                try:
                    opts[k] = self.script_value(e)
                except Exception as ex:  # noqa: BLE001
                    opts[k] = f"error: {str(ex)[:60]}"
            out = d / f'{a.get("name") or "export"}.{EXPORT_EXT.get(fmt, "png")}'
            self.call("export", {"path": str(out)}, timeout=90)
            self.values.append({"export": a, "format": fmt, "options": opts, "value": r.record_path("record_dir", str(d)),
                                "kept": r.keep_export(self, out)})
        elif verb == "export_png":
            out = r.export_path(self)
            _, imgs = self.fp.call("render", {}, timeout=90)
            if not imgs:
                raise BridgeError("render returned no image")
            out.write_bytes(imgs[0])
            self.values.append({"export_png": arg, "value": r.record_path("record_file", str(out))})
        elif verb in ("drag_popup_slider", "slider_drag"):
            a = arg if isinstance(arg, dict) else {"to_fraction": arg}
            p1, p2 = slider_points(self.popup_slider_rect(), float(a.get("to_fraction", a.get("fraction", 0.5))), a.get("from_fraction"))
            ev = [{"type": "move", "pos": list(p1)}, {"type": "down"}, {"type": "wait", "ms": 60}]
            for i in range(1, 13):
                ev.append({"type": "move", "pos": [p1[0] + (p2[0] - p1[0]) * i / 12, p1[1] + (p2[1] - p1[1]) * i / 12]})
                ev.append({"type": "wait", "ms": 20})
            if a.get("hold"):
                self.held = "left"
            else:
                ev.append({"type": "up"})
            ev.append({"type": "wait", "ms": 250})
            self.inp(ev)
        elif verb in ("panel_menu", "panel_menu_item"):
            p = arg if isinstance(arg, str) else arg.get("panel")
            info = self.panel_info(p)
            self.do("panel", p, {})
            # the panel's own front rect (TW-59 `panel_rects`): the menu button of its group lies in the tab row of
            # that rect. Without it in front the runner opened the menu of the group's front tab (A3-4: Ebenen/
            # Optimieren/Stile instead of Seiten/Status/Verlauf/Farbfelder) - bring its tab forward first
            fid = info.get("fp") or p
            pr = None
            for attempt in range(2):
                pr = (self.state().get("panel_rects") or {}).get(fid)
                if pr or attempt:
                    break
                title = info.get("title")
                tabs = [h for h in (self.call("ui_find", {"text": title, "exact": True, "limit": 20}) or {}).get("hits") or []
                        if title and h.get("text") == title and h["rect"][1] > 60] if title else []
                if not tabs:
                    break
                t = min(tabs, key=lambda h: h["rect"][1])
                self.inp([{"type": "click", "pos": t["center"], "button": "left"}, {"type": "wait", "ms": 300}])
            allb = (self.call("ui_find", {"text": "Bedienfeldmenü", "role": "button", "limit": 50}) or {}).get("hits") or []
            if pr:
                btns = [h for h in allb if pr[0] - 2 <= (h["rect"][0] + h["rect"][2]) / 2 <= pr[2] + 2
                        and pr[1] - 2 <= (h["rect"][1] + h["rect"][3]) / 2 <= pr[1] + 30]
            else:
                tab = self.find(info.get("title") or p, None, True)
                tr = tab["rect"]
                cy = (tr[1] + tr[3]) / 2
                btns = [h for h in allb if abs((h["rect"][1] + h["rect"][3]) / 2 - cy) < 12 and h["rect"][0] > tr[0]]
            if not btns and allb and pr:
                # floating group: its menu button may sit right of the reported rect's tab row
                btns = [h for h in allb if abs((h["rect"][1] + h["rect"][3]) / 2 - (pr[1] + 12)) < 16]
            if not btns:
                raise BridgeError(f"panel menu button for {p} not found (panel rect {pr})")
            # R-2 (panel-library-styles/11,12): the click did not open the menu. Check that something popped up
            # below the button (window image), otherwise press again, then try the other candidates
            opened = False
            for b in sorted(btns, key=lambda h: h["rect"][0]) * 2:
                before = self.window_image()[0]
                self.inp([{"type": "click", "pos": b["center"]}, {"type": "wait", "ms": 350}])
                img = self.window_image()[0]
                bb = diff_bbox(before, img, pad=0)
                if bb and (bb[2] - bb[0]) >= 40 and (bb[3] - bb[1]) >= 30:
                    opened = True
                    # R-4: the open popup's area - the next text click and log_menu_items look there first
                    # (panel-library-styles/11: 'Neuer Stil...' also names the panel's own button, which made a
                    # style without the dialog)
                    self.popup_rect = popup_component(before, img) or list(bb)
                    break
            if not opened:
                raise BridgeError(f"panel menu of {p} did not open (buttons {[h['center'] for h in btns]})")
            item = act.get("item") or (arg.get("item") if isinstance(arg, dict) else None)
            if item:
                parts = split_path(item)
                for i, part in enumerate(parts):
                    hit = self.find_in_popup(part)
                    t = "click" if i == len(parts) - 1 else "move"
                    self.inp([{"type": t, "pos": hit["center"]}, {"type": "wait", "ms": 350}])
        elif verb in ("flyout_open", "flyout", "flyout_item"):
            a = arg if isinstance(arg, dict) else {"target": arg}
            target = a.get("target") or a.get("flyout")
            item = a.get("item") or act.get("item")
            if item:
                self.do("choose", {"target": target, "item": item}, act)
            else:
                self.pointer("open", target, act)
        elif verb == "fx_add":
            item = arg if isinstance(arg, str) else arg.get("item")
            self.do("choose", {"target": "inspector.fx_add", "item": item}, act)
        elif verb in ("scroll", "wheel", "canvas_wheel"):
            a = arg if isinstance(arg, dict) else {"delta": arg}
            if verb == "canvas_wheel":
                pos = self.canvas_point(a, "canvas_wheel")
            elif isinstance(a.get("area"), dict):
                pos = self.resolve(a["area"])
            elif a.get("target") is not None:
                pos = self.resolve(a["target"])
            else:
                rr = self.area_rect(a.get("area", "canvas"))
                pos = [(rr[0] + rr[2]) / 2, (rr[1] + rr[3]) / 2]
            d = int(a.get("delta", 1))
            if verb != "canvas_wheel":
                d = -d  # scroll: positive = content moves up (wheel towards the user)
            mods = a.get("mods") or a.get("modifiers") or act.get("mods") or []
            ev = {"type": "wheel", "pos": pos, "delta": [d, 0] if a.get("horizontal") else [0, d], "unit": "line"}
            if mods:
                ev["modifiers"] = [m.lower() for m in mods]
            self.inp([{"type": "move", "pos": pos}, ev, {"type": "wait", "ms": 250}])
        elif verb in ("drag_handle", "click_handle", "hover_handle"):
            a = arg if isinstance(arg, dict) else {"handle": arg}
            pos = self.handle_point(a.get("handle", a.get("index", 0)))
            if verb == "drag_handle":
                by = a.get("by") or act.get("by") or [0, 0]
                ev = {"type": "drag", "from": pos, "to": [pos[0] + by[0], pos[1] + by[1]], "steps": 20}
                if a.get("mods"):
                    ev["modifiers"] = a["mods"]
                self.inp([ev])
            else:
                self.inp([{"type": "click" if verb == "click_handle" else "move", "pos": pos}, {"type": "wait", "ms": 200}])
        elif verb == "uia_click":
            raise Unsupported("uia_click is orig-only")
        else:
            raise Unsupported(verb)

    def loc_of(self, target):
        if isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            e = self.r.ui_map["targets"][target]
            loc = (e.get(self.name) if isinstance(e, dict) else None) or {}
            # a string locator is an alias ("dialog.combo.1", another key): no options of its own (RUN-6)
            return self.loc_of(loc) if isinstance(loc, str) and loc != target else (loc if isinstance(loc, dict) else {})
        return target if isinstance(target, dict) else {}

    def set_field(self, a, target):
        """Click the field, mark its text (End + Shift+Home), type, then end like the reference side
        (`set_field_commit`). The keys are only sent when the click focused a text field: otherwise it reached the canvas and selected every object (runner
        finding 1: `select P1` + `set_field inspector.stroke_edge` left all 7 objects selected)."""
        pos = self.resolve(target)
        in_dialog = bool(self.state().get("dialog_open"))
        focused = False
        for ev in ({"type": "click", "pos": pos}, {"type": "double_click", "pos": pos}):
            v = self.inp([ev, {"type": "wait", "ms": 60}]) or {}
            # a field that already had the focus reports only `clicked`
            if any(w.get("event") in ("focus_gained", "clicked") and re.search(r"text|drag|spin|value", str(w.get("widget")))
                   for w in v.get("widget_events") or []):
                focused = True
                break
        if not focused:
            raise BridgeError(f"set_field: no text field at {target} ({pos}) - nothing typed")
        commit = set_field_commit(a) or ("tab" if in_dialog else "enter")
        # R-4: the field text is marked with End + Shift+Home (like the UI tests), not Ctrl+A - in the box fields
        # B/H/X/Y Ctrl+A selected every object (CF-FP1 FP1-01, TW-78) and the typed value was appended
        ev = [{"type": "key", "key": "End"}, {"type": "key", "key": "Home", "modifiers": ["shift"]},
              {"type": "type", "text": str(a.get("value"))}]
        if commit != "none":
            ev.append({"type": "key", "key": "Enter" if commit == "enter" else "Tab"})
        self.inp(ev)

    def selected_id(self):
        doc = next((d for d in self.state().get("documents", []) if d.get("active")), None)
        sel = (doc or {}).get("selection") or []
        if not sel:
            raise BridgeError("nothing selected")
        return sel[0]

    def handle_point(self, h):
        eid = self.selected_id()
        pts, rect = [], None
        try:
            v = self.call("run_command", {"name": "autoShape.describe", "params": {"id": eid}})
            for c in ((v or {}).get("result") or {}).get("controlPoints") or []:
                pts.append((c["x"], c["y"], c.get("toolTip") or "", c.get("visible", True)))
        except BridgeError:
            pass
        doc = self.call("get_document", {"depth": 6})

        def walk(els):
            for e in els or []:
                if e.get("id") == eid:
                    return e
                f = walk(e.get("children"))
                if f:
                    return f
            return None
        for pg in doc.get("pages", []):
            for ly in pg.get("layers", []):
                e = walk(ly.get("elements")) or next(filter(None, (walk(sub.get("elements")) for sub in ly.get("sublayers") or [])), None)
                if e and e.get("bounds"):
                    b = e["bounds"]
                    rect = list(b) if isinstance(b, list) else [b.get("left", b.get("x")), b.get("top", b.get("y")),
                                                                b.get("right", b.get("x", 0) + b.get("width", 0)),
                                                                b.get("bottom", b.get("y", 0) + b.get("height", 0))]
        x, y = pick_handle(h, pts, rect, self.r.ui_map.get("handle_roles", {}))
        return self.doc_to_win(x, y)

    def popup_slider_rect(self):
        hits = []
        for _ in range(6):
            hits = (self.call("ui_find", {"role": "slider", "limit": 50}) or {}).get("hits") or []
            if hits:
                break
            time.sleep(0.2)
        if not hits:
            raise BridgeError("no popup slider open")
        return min(hits, key=lambda h: (h["rect"][2] - h["rect"][0]) * (h["rect"][3] - h["rect"][1]))["rect"]

    def pointer(self, verb, arg, act):
        mods = norm_mods(act.get("mods") or act.get("hold_key"))
        loc = {} if verb.startswith("canvas_") or verb == "ui_find_click" else \
            self.loc_of(arg.get("target") if isinstance(arg, dict) and "target" in arg else arg)
        if verb == "ui_find_click":
            pos = self.find(arg if isinstance(arg, str) else arg.get("text"))["center"]
            verb = "click"
        elif verb.startswith("canvas_"):
            pos = self.canvas_point(arg, verb)
            verb = {"canvas_click": "click", "canvas_double_click": "double_click"}.get(verb, "hover")
        else:
            target = arg
            if isinstance(arg, dict) and "target" in arg:
                mods = norm_mods(arg.get("mods") or arg.get("hold_key") or mods)
                target = arg["target"]
            pos = self.resolve(target)
            if isinstance(target, dict) and any(k in target for k in SWATCH_KEYS):
                self.values.append({"click_probe": click_probe(self, target, pos)})
        if verb == "open":
            try:
                self.open_before = self.window_image()[0]
            except Exception:  # noqa: BLE001
                self.open_before = None
        if loc.get("hold_ms") and verb in ("click", "open"):
            self.inp([{"type": "move", "pos": pos}, {"type": "down"}, {"type": "wait", "ms": int(loc["hold_ms"])},
                      {"type": "up"}, {"type": "wait", "ms": 250}])
            return
        if verb in ("hover", "hover_text", "canvas_hover", "canvas_move"):
            ev = [{"type": "move", "pos": pos}]
        elif verb == "mouse_down":
            self.held = "left"
            ev = [{"type": "move", "pos": pos}, {"type": "down"}]
        elif verb == "mouse_up":
            ev = [{"type": "move", "pos": pos}, {"type": "up"}]
        elif verb.startswith("right"):
            ev = [{"type": "right_click", "pos": pos}]
        elif verb.startswith("double"):
            ev = [{"type": "double_click", "pos": pos}]
        else:
            ev = [{"type": "click", "pos": pos}]
        keys = [m for m in mods if m not in HELD_MODS]
        mods = [m for m in mods if m in HELD_MODS]
        if mods:
            for e in ev:
                if e["type"] not in ("move",):
                    e["modifiers"] = [m.lower() for m in mods]
        if keys:
            # R-9: held keys that are no modifiers (Space = temporary hand) are pressed around the pointer events
            ev = [{"type": "key", "key": egui_key(k), "action": "press"} for k in keys] + [{"type": "wait", "ms": 60}] + ev \
                + [{"type": "key", "key": egui_key(k), "action": "release"} for k in keys]
        ev.append({"type": "wait", "ms": 200})
        self.inp(ev)
        if loc.get("select") and verb in ("click", "open"):
            hit = self.find(loc["select"], None, True, [0, 30, 10000, 10000])
            self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 200}])

    def record_ui(self, key):
        if isinstance(key, dict):
            if "focus_cycle" in key:
                out = []
                for _ in range(int(key["focus_cycle"] or 1)):
                    res = self.inp([{"type": "key", "key": "Tab"}, {"type": "wait", "ms": 80}]) or {}
                    ev = [e for e in (res.get("widget_events") or []) if isinstance(e, dict) and e.get("event") == "focus_gained"]
                    e = ev[-1] if ev else {}
                    out.append({"role": e.get("widget"), "text": e.get("label") or e.get("text")})
                return {"focus_cycle": out}
            raise Unsupported(f"record_ui {key}")
        if key.startswith("ui_state"):
            st = self.state()
            sub = key.split(".", 1)[1] if "." in key else "summary"
            return st if sub == "summary" else st.get(sub)
        if key in ("menu.open", "menu.disabled_items"):
            return self.menu_items()
        if key in ("dialog.fields", "dialog.open", "dialog.title", "dialog.text"):
            return self.dialog_fields()
        if key == "document.summary":
            return self.call("get_document", {"depth": 1})
        if key == "popup.visible":
            # egui popups are no windows: compared with the window image right before the last `open` (xs-shortcuts/21)
            st = self.state()
            if st.get("dialog_open"):
                return {"visible": True, "dialog": True}
            before = getattr(self, "open_before", None)
            if before is None:
                return {"visible": None, "error": "no `open` in this step"}
            img, _ = self.window_image()
            bb = diff_bbox(before, img, pad=0)
            big = bool(bb) and (bb[2] - bb[0]) >= 12 and (bb[3] - bb[1]) >= 12 \
                and (bb[2] - bb[0]) * (bb[3] - bb[1]) < 0.6 * img.width * img.height
            return {"visible": big, "box": bb}
        return self.state()

    def option_state(self, target):
        loc = self.loc_of(target)
        if "find" not in loc:
            raise BridgeError(f"option {target}: fp locator without 'find' ({loc})")
        hit = self.find(loc["find"], loc.get("role"), loc.get("exact", True), None, loc.get("index", 0))
        v = hit.get("toggled")
        return None if v is None else bool(v)

    def menu_entry_state(self, path):
        parts = split_path(path)
        try:
            h = self.find(parts[0], None, True, [0, 0, 2000, 40])
            self.inp([{"type": "click", "pos": h["center"]}, {"type": "wait", "ms": 200}])
            for p in parts[1:-1]:
                hit = self.find(p, None, True, [0, 30, 10000, 10000])
                self.inp([{"type": "move", "pos": hit["center"]}, {"type": "wait", "ms": 350}])
            want = clean_label(parts[-1])
            hits = []
            for exact in (True, False):
                hits = (self.call("ui_find", {"text": want, "exact": exact, "region": [0, 30, 10000, 10000], "limit": 20}) or {}).get("hits") or []
                if hits:
                    break
            if not hits:
                raise BridgeError(f"menu entry '{parts[-1]}' not found in the open menu")
            # painted texts may carry no state; prefer the node that reports one
            hit = next((x for x in hits if x.get("enabled") is not None), hits[0])
            return {"text": hit.get("text"), "enabled": hit.get("enabled"), "checked": hit.get("toggled"),
                    "role": hit.get("role")}
        finally:
            self.inp([{"type": "key", "key": "Escape"}, {"type": "wait", "ms": 120},
                      {"type": "key", "key": "Escape"}, {"type": "wait", "ms": 120}])

    def dialog_fields(self):
        """Like the reference side's dialog list: title and the texts of the dialog's controls/labels in reading
        order (top to bottom, left to right), plus the controls with role, text and state."""
        st = self.state()
        if not st.get("dialog_open"):
            return []
        # K-2: the bridge reports the dialog rectangle from the window state (an open drop-down list over
        # the frame no longer shortens it) and the open list separately; image scan only for older builds
        rect = ((st.get("dialog") or {}).get("rect"))
        try:
            rect = [round(v) for v in rect] if rect else self.dialog_rect(st)
        except BridgeError:
            return [{"title": None, "texts": "", "controls": []}]
        popup = st.get("popup")
        pr = popup.get("rect") if popup else None

        def in_popup(h):
            return pr is not None and (h["rect"][0] >= pr[0] - 1 and h["rect"][2] <= pr[2] + 1
                and h["rect"][1] >= pr[1] - 1 and h["rect"][3] <= pr[3] + 1)
        hits = (self.call("ui_find", {"region": rect, "limit": 600}) or {}).get("hits") or []
        hits = [h for h in hits if h["rect"][0] >= rect[0] - 1 and h["rect"][2] <= rect[2] + 1
                and h["rect"][1] >= rect[1] - 1 and h["rect"][3] <= rect[3] + 1 and h.get("role") != "text_run"
                and not in_popup(h)]
        hits.sort(key=lambda h: (round(h["rect"][1] / 6), h["rect"][0]))
        title = next((h["text"][len("Titelleiste "):] for h in hits if str(h.get("text", "")).startswith("Titelleiste ")),
                     ((st.get("file_dialog") or {}).get("open") or {}).get("title"))
        seen, texts, controls = set(), [], []
        for h in hits:
            t = h.get("text") or ""
            if t.startswith("Titelleiste"):
                continue
            k = (t, round(h["rect"][0]), round(h["rect"][1]))
            if t and (t, round(h["rect"][1] / 6)) not in seen:
                seen.add((t, round(h["rect"][1] / 6)))
                texts.append(t)
            if h.get("role") not in (None, "text", "label", "unknown"):
                controls.append({"role": h.get("role"), "text": t, "enabled": h.get("enabled"), "checked": h.get("toggled")})
        out = {"title": title, "rect": [round(v) for v in rect], "texts": " | ".join(texts), "controls": controls}
        if popup:
            out["popup"] = {"rect": [round(v) for v in pr], "items": popup.get("items") or [], "selected": popup.get("selected")}
        return [out]

    def menu_items(self):
        """Texts of the open menus. egui menus expose plain texts (no menu_item role): the menu region is what
        changed against the window image before the opening action; its texts are grouped by row (entry left,
        shortcut right)."""
        hits = (self.call("ui_find", {"role": "menu_item", "limit": 200}) or {}).get("hits") or []
        if hits:
            return [{"text": h.get("text"), "enabled": h.get("enabled"), "checked": h.get("toggled")} for h in hits]
        before = getattr(self, "before_img", None)
        img, _ = self.window_image()
        # R-4: only the open popup (and the sub menus touching it), not every change in the window
        pr = getattr(self, "popup_rect", None)
        bb = popup_component(before, img, seed=pr) if before is not None else None
        if not bb:
            return []
        try:
            bar = self.find(self.r.ui_map["menu_bar"]["orig"][0][0], None, True, [0, 0, 2000, 40])["rect"]
            bb[1] = max(bb[1], bar[3] + 2)
        except Exception:  # noqa: BLE001
            pass
        # R-12 (TW-124 b): `role: text` returns every painted text - entries of context/button menus carry a widget
        # name and were dropped as duplicates of their accessibility node (panel-optimize/13, panel-misc/14 read
        # only panel texts); only the topmost paint block (the menu, not the panel under it) is kept
        hits = (self.call("ui_find", {"region": bb, "role": "text", "limit": 500}) or {}).get("hits") or []
        # menu entries are painted texts; accessibility nodes in the box are toolbar/panel controls
        hits = [h for h in hits if h.get("text") and h["rect"][1] >= bb[1] - 1 and h.get("source") != "accesskit"]
        hits = top_paint_block(hits)
        def count(h, i, tol):
            return sum(1 for o in hits if abs(o["rect"][i] - h["rect"][i]) <= tol)

        def row_of(h):
            return (h["rect"][1] + h["rect"][3]) / 2

        # entries are left aligned (shared left edge per column), shortcuts right aligned (shared right edge)
        short = [h for h in hits if count(h, 2, 1.5) >= 2 and count(h, 0, 2) < 3
                 and any(o is not h and o["rect"][0] < h["rect"][0] and abs(row_of(o) - row_of(h)) < 4 for o in hits)]
        entries = [h for h in hits if not any(h is s for s in short)]
        cols: dict = {}
        for h in sorted(entries, key=lambda h: (h["rect"][0], h["rect"][1])):
            col = next((c for c in cols if abs(c - h["rect"][0]) <= 2), h["rect"][0])
            cols.setdefault(col, []).append({"text": h["text"], "enabled": h.get("enabled"), "checked": h.get("toggled"),
                                             "_r": h["rect"]})
        for s in short:
            cand = [e for c in cols.values() for e in c if e["_r"][0] < s["rect"][0] and abs((e["_r"][1] + e["_r"][3]) / 2 - row_of(s)) < 4]
            if cand:
                max(cand, key=lambda e: e["_r"][0])["shortcut"] = s["text"]
        out = []
        for c in sorted(cols):
            col = sorted(cols[c], key=lambda e: e["_r"][1])
            for e in col:
                e.pop("_r")
            out.append(col)
        return out


# --------------------------------------------------------------------------- runner
class Runner:
    def __init__(self, cfg: dict, dry: bool, only_steps: set | None):
        self.cfg = cfg
        self.dry = dry
        self.only = only_steps
        self.ui_map = json.loads((HERE / "ui_map.json").read_text(encoding="utf-8"))
        self.out = Path(cfg["out_dir"])
        self.tmp = Path(cfg["tmp_dir"])
        self.tmp.mkdir(parents=True, exist_ok=True)
        self.opened: dict[str, set] = {}
        self.orig = self.fp = None
        self.lock = Lock(cfg["orig"]["lock"])

    def log(self, *a):
        print(time.strftime("%H:%M:%S"), *a, flush=True)

    # ---- files
    # test files of TESTFILES.md that the config may not list: T11 = import folder, T12 = swatch file
    EXTRA_TESTFILES = {"T11": "par_import", "T12": "par_swatches.act"}

    def testfile(self, code) -> Path:
        code = str(code)
        sub = None
        if re.fullmatch(r"T\d\d:.+", code):  # "T11:par.psd" = file inside the test folder T11
            code, sub = code.split(":", 1)
        tf = self.cfg["testfiles"].get(code) or self.EXTRA_TESTFILES.get(code, code)
        p = Path(tf)
        p = p if p.is_absolute() else Path(self.cfg["testfile_dir"]) / tf
        return p / sub if sub else p

    # ---- R-6 (Q-41): test files protected by checksums. Run 10: `menu-file/07` (saveDocumentAs with the copy
    # flag) saved over par_vector.fw.png in the reference program without a dialog. The run now starts only when
    # every test file matches the stored list; a copy of the checked files is kept in tmp_dir and a file changed by
    # a unit is put back from it at the unit's end.
    def sums_path(self) -> Path:
        return Path(self.cfg.get("testfiles_sha256") or Path(self.cfg["testfile_dir"]) / "TESTFILES.sha256")

    def testfile_list(self) -> list[Path]:
        out: list[Path] = []
        for code in list(self.cfg["testfiles"]) + list(self.EXTRA_TESTFILES):
            p = self.testfile(code)
            if p.is_dir():
                out += sorted(f for f in p.iterdir() if f.is_file())
            else:
                out.append(p)
        return list(dict.fromkeys(out))

    def testfile_rel(self, p: Path) -> str:
        try:
            return p.resolve().relative_to(Path(self.cfg["testfile_dir"]).resolve()).as_posix()
        except ValueError:
            return str(p)

    @staticmethod
    def sha256(p: Path) -> str:
        h = hashlib.sha256()
        with open(p, "rb") as f:
            for chunk in iter(lambda: f.read(1 << 20), b""):
                h.update(chunk)
        return h.hexdigest()

    def write_testfile_sums(self) -> Path:
        lines = [f"{self.sha256(p)} *{self.testfile_rel(p)}" for p in self.testfile_list() if p.is_file()]
        sp = self.sums_path()
        sp.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return sp

    def read_testfile_sums(self) -> dict[str, str]:
        sp = self.sums_path()
        if not sp.is_file():
            raise TestfilesChanged(f"checksum list {sp} missing - create it from checked files with --write-testfile-sums")
        sums = {}
        for ln in sp.read_text(encoding="utf-8").splitlines():
            m = re.fullmatch(r"([0-9a-fA-F]{64}) [ *](.+)", ln.strip())
            if m:
                sums[m.group(2)] = m.group(1).lower()
        return sums

    def testfile_problems(self) -> list[tuple[str, str]]:
        """(relative path, problem) for every listed or configured test file that is missing or differs."""
        sums = self.read_testfile_sums()
        root = Path(self.cfg["testfile_dir"])
        probs = []
        for rel, want in sums.items():
            p = Path(rel) if Path(rel).is_absolute() else root / rel
            if not p.is_file():
                probs.append((rel, "missing"))
                continue
            got = self.sha256(p)
            if got != want:
                probs.append((rel, f"checksum {got[:12]}... instead of {want[:12]}..."))
        for p in self.testfile_list():
            if self.testfile_rel(p) not in sums:
                probs.append((self.testfile_rel(p), "not in the checksum list"))
        return probs

    def protect_testfiles(self):
        """Run start: every test file must match TESTFILES.sha256 (else TestfilesChanged); the checked files are
        copied to tmp_dir/testfiles_ref for the restore at each unit's end."""
        probs = self.testfile_problems()
        if probs:
            raise TestfilesChanged("test files differ from " + str(self.sums_path()) + ": "
                                   + "; ".join(f"{r}: {w}" for r, w in probs)
                                   + " - restore them from the saved copies, then start again")
        sums = self.read_testfile_sums()
        self.log(f"test files: {len(sums)} match {self.sums_path().name}")
        if self.dry:
            self.log_shortcut_sets()
            return
        ref = self.tmp / "testfiles_ref"
        root = Path(self.cfg["testfile_dir"])
        for rel in sums:
            src = Path(rel) if Path(rel).is_absolute() else root / rel
            dst = ref / rel.replace(":", "_")
            dst.parent.mkdir(parents=True, exist_ok=True)
            if not dst.is_file() or self.sha256(dst) != sums[rel]:
                shutil.copy2(src, dst)
        self.testfiles_ref = ref

    def restore_testfiles(self, where: str):
        """Unit end: a test file changed by the unit is put back from the checked copy (and logged); the changed
        file is kept in tmp_dir/testfiles_changed. A file that cannot be put back stops the run."""
        ref = getattr(self, "testfiles_ref", None)
        if ref is None:
            return
        sums = self.read_testfile_sums()
        root = Path(self.cfg["testfile_dir"])
        bad = []
        for rel, what in self.testfile_problems():
            if rel not in sums:
                self.log(f"TEST FILE {where}: {rel} {what} (left alone)")
                continue
            dst = Path(rel) if Path(rel).is_absolute() else root / rel
            src = ref / rel.replace(":", "_")
            self.log(f"TEST FILE CHANGED {where}: {rel} {what} - putting back the checked copy")
            try:
                if dst.is_file():
                    keep = self.tmp / "testfiles_changed" / f"{time.strftime('%Y%m%d_%H%M%S')}_{dst.name}"
                    keep.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copy2(dst, keep)
                shutil.copy2(src, dst)
                if self.sha256(dst) != sums[rel]:
                    bad.append(rel)
            except OSError as e:
                self.log(f"TEST FILE {rel}: restore failed: {e}")
                bad.append(rel)
        if bad:
            raise TestfilesChanged(f"{where}: test files could not be restored: {', '.join(bad)}")

    def open_testfile(self, side: Side, code):
        code = code if isinstance(code, str) and code not in ("", "true") else self.unit.get("testfile")
        p = self.testfile(code)
        side.open_testfile(p)
        self.opened.setdefault(side.name, set()).add(p)
        # defined tool: the reference program keeps the last tool across documents and steps (a marquee left active
        # shows its own inspector instead of the selection's, runner finding 3); every opened test file starts with
        # the pointer tool on both sides
        try:
            side.do("tool", "tool.pointer", {})
        except Exception as e:  # noqa: BLE001
            self.log(f"{side.name}: pointer tool after opening: {e}")
            # R-7 (run 11, xs-canvas/08,10,12 ran with the wrong tool and counted as ok): the step is repeated
            side.tool_failed = True

    def safe_dir(self, d) -> Path:
        p = Path(d)
        if not p.is_absolute():
            p = self.out / self.unit["unit"] / "files" / d
        root = Path(self.cfg["testfile_dir"]).resolve()
        if root not in p.resolve().parents and p.resolve() != root:
            raise Unsupported(f"directory outside the test folder: {p}")
        return p

    def clear_dir(self, d):
        p = self.safe_dir(d)
        p.mkdir(parents=True, exist_ok=True)
        for f in p.iterdir():
            if f.is_file():
                f.unlink()

    def record_path(self, verb, arg):
        p = Path(arg if isinstance(arg, str) else arg.get("path"))
        if verb == "record_dir":
            return sorted(f.name for f in p.iterdir()) if p.is_dir() else None
        if not p.exists():
            return None
        if verb == "record_file_text":
            return p.read_text(encoding="utf-8", errors="replace")[:4000]
        b = p.read_bytes()
        info = {"size": len(b), "sha1": hashlib.sha1(b).hexdigest()}
        try:
            with Image.open(p) as im:
                info["pixels"] = list(im.size)
        except Exception:
            pass
        return info

    RESTART_BEFORE = ("color-popup", "menu-view", "panel-misc", "xs-file-formats")

    def log_foreign_windows(self):
        """R-6 (Q-39): visible windows of Chromium/Electron programs (class Chrome_WidgetWin_1) at run start, with
        process name, pid and title - run 10 lost 3 steps to one of them. The runner ends no process."""
        try:
            import ctypes
            from ctypes import wintypes
            u = ctypes.windll.user32
            found = []
            proto = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)

            def cb(h, _):
                if u.IsWindowVisible(h):
                    c = ctypes.create_unicode_buffer(256)
                    u.GetClassNameW(h, c, 256)
                    if c.value == "Chrome_WidgetWin_1":
                        t = ctypes.create_unicode_buffer(256)
                        u.GetWindowTextW(h, t, 256)
                        pid = wintypes.DWORD()
                        u.GetWindowThreadProcessId(h, ctypes.byref(pid))
                        found.append((pid.value, t.value))
                return True
            u.EnumWindows(proto(cb), 0)
            names = {}
            if found:
                out = subprocess.run(["tasklist", "/fo", "csv", "/nh"], capture_output=True, text=True, timeout=20).stdout
                for ln in out.splitlines():
                    parts = [x.strip('"') for x in ln.split('","')]
                    if len(parts) > 1 and parts[1].isdigit():
                        names[int(parts[1])] = parts[0]
            for pid, title in found:
                self.log(f"foreign window at run start: Chrome_WidgetWin_1 pid {pid} ({names.get(pid, '?')}) "
                         f"'{title[:60]}' - close it or switch off its notifications before a long run")
        except Exception as e:  # noqa: BLE001
            self.log(f"foreign windows: {e}")

    # ---- programs
    def ensure_programs(self):
        if self.orig is None:
            self.orig = Original(self.cfg["orig"], self.tmp, log=self.log)
            self.orig.host.start()
        if self.fp is None:
            self.fp = Firepixel(self.cfg["fp"], log=self.log)
        self.sides = {"orig": OrigSide(self, self.orig), "fp": FpSide(self, self.fp)}

    SYSTEMIC = re.compile(r"main window not found|process not running|JSF timeout|host timeout|host ended|"
                          r"crash|cannot bring|input refused|not started|without main window|menu did not open|"
                          r"panel not visible after", re.I)

    def count_orig(self, status: str, error: str = ""):
        """After `orig.max_consecutive_errors` (default 3) failed steps in a row whose errors point at the
        program/automation (not at a single locator) the reference program is restarted cleanly
        (documents closed unsaved, instance ended, started again) and the unit goes on."""
        if status == "error" and self.SYSTEMIC.search(error or ""):
            self.orig_fails = getattr(self, "orig_fails", 0) + 1
        elif status in ("ok", "slow"):
            self.orig_fails = 0
        if self.orig_fails >= int(self.cfg["orig"].get("max_consecutive_errors", 3)):
            self.log(f"{self.orig_fails} reference errors in a row - clean restart")
            self.orig_fails = 0
            try:
                self.orig.restart(f"{int(self.cfg['orig'].get('max_consecutive_errors', 3))} reference errors in a row ({str(error)[:160]})")
                self.sides["orig"].prepare()
                if self.unit.get("testfile") and not self.testfile(self.unit["testfile"]).is_dir():
                    self.open_testfile(self.sides["orig"], self.unit["testfile"])
            except Exception as e:  # noqa: BLE001
                self.log(f"restart orig failed: {e}")

    def recover(self, side: Side):
        """After a failure: crash check, close menus/dialogs, reopen the unit's test file."""
        try:
            if side.name == "orig":
                self.orig.decline_dialogs()
                if not self.orig.running():
                    self.orig.start()
                side.prepare()
            else:
                if not self.fp.alive():
                    self.log("restart Firepixel")
                    self.fp.stop()
                    self.fp.start()
            side.reset_light()
            if self.unit.get("testfile") and not self.testfile(self.unit["testfile"]).is_dir():
                self.open_testfile(side, self.unit["testfile"])
        except Exception as e:  # noqa: BLE001
            self.log(f"recover {side.name} failed: {e}")

    # ---- steps
    def run_actions(self, side: Side, actions, rec):
        for act in actions or []:
            if isinstance(act, str):
                act = {act: True}
            # "{side}" / "{unit}" in paths (export-results): each side gets its own folder (RUN-6)
            act = subst_placeholders(act, side.name, self.unit["unit"], self.tmp)
            verb = next((k for k in act if k not in ("mods", "ms", "repeat", "hold_key", "item", "by", "at", "do",
                                                    "region", "to_fraction", "hold", "orig_index", "row", "from", "to",
                                                    "value", "target", "note", "orig_menu", "commit", "then")), None)
            if verb is None:
                verb = "repeat" if "repeat" in act and "do" in act else next(iter(act))
            arg = act[verb]
            if verb == "orig_index":
                continue
            if verb == "repeat":
                # {"repeat": n, "do": [actions]}: the actions n times (both sides)
                for _ in range(int(arg or 0)):
                    self.run_actions(side, act.get("do") or [], rec)
                continue
            if verb == "reset":
                if arg in (None, False, "", "null"):
                    verb = "close_dialogs"
                else:
                    verb = "open_testfile"
            if verb == "record":
                verb = "log_value"
            if verb == "window_size":
                # [w, h] of the main window at its configured position; the next step restores the configured size
                w, h = (arg if isinstance(arg, list) else [arg.get("width"), arg.get("height")])
                side.window_size(int(w), int(h))
                side.resized = (int(w), int(h))
                rec["actions"].append({"verb": verb, "ms": 0})
                continue
            if verb in ("menu", "menu_item") and isinstance(arg, str) and not act.get("toggle"):
                # "Fenster > <panel>" toggles: a panel that is already visible would be hidden. Scripts mean "show
                # it" (then click its buttons): handled like the `panel` verb (only opened when not visible);
                # `"toggle": true` keeps the plain menu command
                want = " > ".join(split_path(arg))
                for pid, info in (self.ui_map.get("panels") or {}).items():
                    if isinstance(info, dict) and info.get("menu") and " > ".join(split_path(info["menu"])) == want:
                        verb, arg = "panel", pid
                        break
            if verb == "command" and side.name == "orig" and act.get("orig_menu"):
                verb, arg = "menu_item", act["orig_menu"]
            if verb in ("file_dialog", "file_dialog_choose") and arg == "cancel":
                verb = "file_dialog_cancel"
            if verb == "wait" and isinstance(arg, (int, float)):
                time.sleep(arg / 1000)
                continue
            if verb == "set" and "value" in act and isinstance(arg, str):
                arg = {"target": arg, "value": act["value"]}
            if verb == "choose" and not isinstance(arg, dict):
                arg = {"target": arg, "item": act.get("item")}
            t0 = time.time()
            if verb in ("key", "menu_item", "command", "panel", "ui_action") and hasattr(side, "page"):
                side.page = None
            if rec.get("want_before") and verb in OPENERS:
                # the difference box is measured against the state right before the opening action
                try:
                    # let pending repaints (selection handles, inspector after `select`) settle first, otherwise
                    # they end up in the difference box
                    time.sleep(0.3)
                    rec["_before"] = side.before_img = side.window_image()[0]
                except Exception:  # noqa: BLE001
                    pass
            if VIEW_TOGGLE.search(f"{verb} {json.dumps(arg, ensure_ascii=False)}"):
                side.view_dirty = True
            if verb == "if_no_dialog":
                # R-11 (export-results/13): {"wait_ms": n, "do": [actions], "why": text} - the actions run only when
                # no dialog opened within wait_ms (a second way to open it); logged in the side's values
                a = arg if isinstance(arg, dict) else {"do": arg}
                end = time.time() + float(a.get("wait_ms", 2000)) / 1000
                opened = side.dialog_open()
                while not opened and time.time() < end:
                    time.sleep(0.3)
                    opened = side.dialog_open()
                side.values.append({"if_no_dialog": {"dialog": opened, "fallback": not opened, "why": a.get("why")}})
                if not opened:
                    self.log(f"  {side.name}: no dialog after the first way - {a.get('why') or 'fallback actions'}")
                    if a.get("remeasure") and side.name == "orig" and getattr(side, "page", None) is not None:
                        # R-12 (TW-124 e, xs-canvas/07): page measured again before the second way (a guide
                        # double-click must hit the line at its current screen position)
                        side.page = None
                        try:
                            time.sleep(0.3)
                            side.calibrate()
                        except Exception as e:  # noqa: BLE001
                            self.log(f"  page before the fallback: {e}")
                    self.run_actions(side, a.get("do") or [], rec)
                    if a.get("unsupported_if_none"):
                        # R-12 (TW-124 e): no scripted way opened the dialog on this side - the step is
                        # 'unsupported' with the reason instead of an error (or a program difference)
                        end = time.time() + float(a.get("wait_ms", 2000)) / 1000
                        opened = side.dialog_open()
                        while not opened and time.time() < end:
                            time.sleep(0.3)
                            opened = side.dialog_open()
                        side.values.append({"if_no_dialog": {"dialog": opened, "after_fallback": True}})
                        if not opened:
                            raise Unsupported(f"{side.name}: {a['unsupported_if_none']}")
                rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})
                continue
            if verb in ("set_option", "restore_option"):
                self.set_option(side, verb, arg)
                rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})
                continue
            if verb == "retry_if":
                self.retry_if(side, arg if isinstance(arg, dict) else {}, rec)
                rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})
                continue
            if verb == "menu_state":
                path = arg if isinstance(arg, (str, list)) else arg.get("path")
                st = side.menu_entry_state(path)
                side.values.append({"menu_state": " > ".join(split_path(path)), "value": st})
                self.log(f"  {side.name} menu state {' > '.join(split_path(path))}: {st}")
                rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})
                continue
            if verb == "swatch_readback":
                side.values.append({"swatch_readback": self.swatch_readback(side, arg if isinstance(arg, dict) else {})})
                rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})
                continue
            # R-11 (run 15 xs-canvas/07: View > Rulers moved the reference's page 2,9 px, the guide double-click used
            # the page measured before and missed the guide): after a menu command the page is measured again at once
            # (not lazily - a guide or object added by the next action would enter the measurement)
            remeasure = verb == "menu" and side.name == "orig" and getattr(side, "page", None) is not None
            side.do(verb, arg, act)
            if remeasure:
                side.page = None
                try:
                    time.sleep(0.3)
                    side.calibrate()
                except Exception as e:  # noqa: BLE001
                    self.log(f"  page after menu: {e}")
            rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})

    def set_option(self, side: Side, verb, arg):
        """R-13 (TW-127, run 17 xs-preferences/06, 15): a stored option (check box whose state persists in the
        program's profile) is set to a known value instead of being toggled with a click: the state is read
        (reference: BM_GETCHECK, Firepixel: `toggled` of the ui_find node), the control clicked only when it differs,
        and read again (up to two clicks). `{"set_option": {"target", "value": bool}}`; the state found first is
        remembered per side and target, `{"restore_option": target}` sets it back (no-op when unknown)."""
        prev = side.__dict__.setdefault("option_prev", {})
        if verb == "restore_option":
            target = arg if isinstance(arg, str) else arg.get("target")
            if prev.get(target) is None:
                prev.pop(target, None)
                side.values.append({"option": target, "restore": "unknown start state - left as is"})
                self.log(f"  {side.name} option {target}: start state unknown, not restored")
                return
            want = bool(prev.pop(target))
        else:
            target, want = arg.get("target"), bool(arg.get("value", True))
        err = OrigError if side.name == "orig" else BridgeError
        before = side.option_state(target)
        if verb == "set_option":
            prev.setdefault(target, before)
        if before is None:
            raise err(f"option {target}: state not readable")
        now, clicks = before, 0
        while now != want and clicks < 2:
            side.do("click", target, {})
            clicks += 1
            time.sleep(0.25)
            now = side.option_state(target)
        side.values.append({"option": target, "verb": verb, "before": before, "want": want, "after": now, "clicks": clicks})
        self.log(f"  {side.name} option {target}: {before} -> {now} (want {want}, {clicks} click(s))")
        if now != want:
            raise err(f"option {target}: still {now} after {clicks} clicks (want {want})")

    def retry_if(self, side: Side, a: dict, rec):
        """R-13 (TW-129): `{"retry_if": {"check": JSF expr, "retry_when": [values] | "retry_when_falsy": true,
        "do": [actions], "why": text, "remeasure": bool}}` - the expression is read (both sides, `log_value` rules);
        when its value is one of `retry_when` (compared as text) or means 'nothing', `do` runs once (remeasure: the
        reference's page is measured again first) and the value is read again. Both values go into the side's
        values; nothing is raised (the step's own checks decide)."""
        expr = a.get("check")

        def read():
            try:
                return side.script_eval(expr) if expr else None
            except Exception as e:  # noqa: BLE001
                return f"error {str(e)[:100]}"

        def bad(v):
            if a.get("retry_when_falsy") and falsy_value(v):
                return True
            return str(v) in [str(x) for x in (a.get("retry_when") or [])]

        v = read()
        out = {"why": a.get("why"), "check": expr, "value": v, "retried": False}
        if bad(v):
            out["retried"] = True
            self.log(f"  {side.name}: retry_if {a.get('why') or expr}: value {v!r} - running the retry once")
            if a.get("remeasure") and side.name == "orig" and getattr(side, "page", None) is not None:
                side.page = None
                try:
                    time.sleep(0.3)
                    side.calibrate()
                except Exception as e:  # noqa: BLE001
                    self.log(f"  page before the retry: {e}")
            try:
                self.run_actions(side, a.get("do") or [], rec)
            except Exception as e:  # noqa: BLE001
                out["retry_error"] = f"{type(e).__name__}: {e}"[:200]
            out["value_after"] = read()
            self.log(f"  {side.name}: retry_if after the retry: {out['value_after']!r}")
        side.values.append({"retry_if": out})

    def log_shortcut_sets(self):
        """R-13 (TW-126): which shortcut set each side uses, once per run. Firepixel: config `fp.shortcut_set`
        (written into the runner profile by fpctl, default "Firepixel"). Reference: its preference
        (`orig.shortcut_set_pref`, default "CurrentShortcutSet") read with fw.getPref when it can be read, else
        'unknown'."""
        if getattr(self, "shortcut_sets_logged", False):
            return
        self.shortcut_sets_logged = True
        fp_set = (self.cfg.get("fp") or {}).get("shortcut_set") or FP_SHORTCUT_SET_DEFAULT
        orig_set = "unknown (dry run, not read)" if self.dry else "unknown"
        key = (self.cfg.get("orig") or {}).get("shortcut_set_pref", ORIG_SHORTCUT_SET_PREF_DEFAULT)
        side = (getattr(self, "sides", None) or {}).get("orig")
        if not self.dry and key and side is not None:
            try:
                v = side.script_eval("(function(){try{return fw.getPref(%s);}catch(e){return 'ERR '+e;}})()" % json.dumps(key))
                if v is not None and not str(v).startswith("ERR") and str(v) not in ("undefined", "null", ""):
                    orig_set = f"'{v}' (pref {key})"
                else:
                    orig_set = f"unknown (pref {key}: {v})"
            except Exception as e:  # noqa: BLE001
                orig_set = f"unknown ({str(e)[:80]})"
        self.log(f"shortcut sets: fp '{fp_set}' (fp.shortcut_set), orig {orig_set}")

    def swatch_readback(self, side: Side, a: dict) -> dict:
        """R-11 (TW-115): swatch set after the reset - JSF `fw.swatches` (count + first 8), the colours of cells
        0..n-1 from the panel image (the click-probe grid of ui_map `grid_targets.panel_swatch`) and, in the
        reference, the palette choice pref. When the pref is not `pref_want` (the menu click did not take), the
        panel menu is opened again and `retry_item` clicked once more."""
        n = int(a.get("n", 8))

        def read():
            out = {}
            try:
                out["swatches_js"] = side.script_eval(SWATCH_COUNT_JS)
            except Exception as e:  # noqa: BLE001
                out["swatches_js"] = f"error {str(e)[:80]}"
            cols = []
            try:
                img = side.window_image()[0] if side.name == "fp" else None
                for i in range(n):
                    pt = side.grid_point({"panel_swatch": i})
                    cols.append(probe_color(side, pt, img) if pt else None)
            except Exception as e:  # noqa: BLE001
                out["colors_error"] = str(e)[:100]
            out["colors"] = cols
            if side.name == "orig":
                try:
                    out["pref"] = side.script_eval(SWATCH_PREF_JS)
                except Exception as e:  # noqa: BLE001
                    out["pref"] = f"error {str(e)[:80]}"
            return out

        got = read()
        want = a.get("pref_want", 0)
        if side.name == "orig" and a.get("retry_item") and str(got.get("pref")) != str(want):
            self.log(f"  orig swatch read-back: pref {got.get('pref')} (want {want}) - menu click did not take, "
                     f"panel menu again and '{a['retry_item']}'")
            got["first"] = dict(got)
            try:
                side.do("panel_menu", a.get("panel", "swatches"), {})
                side.do("click", a["retry_item"], {})
                time.sleep(0.4)
            except Exception as e:  # noqa: BLE001
                got["retry_error"] = str(e)[:120]
            got.update(read())
            got["retried"] = True
        self.log(f"{side.name} swatch read-back: {got.get('swatches_js')}; cells 0-{n - 1} {got.get('colors')}"
                 + (f"; pref {got.get('pref')}" if "pref" in got else ""))
        return got

    def keep_export(self, side: Side, f: Path):
        """Copy of an exported file into the unit's `files` folder (`<step>_<side>_<name>`): both sides export to
        the same name, the second would overwrite the first (TW-58)."""
        if not f.exists():
            return None
        d = self.out / self.unit["unit"] / "files"
        d.mkdir(parents=True, exist_ok=True)
        dst = d / f"{self.cur_step}_{side.name}_{f.name}"
        shutil.copyfile(f, dst)
        info = self.record_path("record_file", str(dst)) or {}
        info["path"] = str(dst)
        return info

    def export_path(self, side: Side) -> Path:
        d = self.out / self.unit["unit"] / "files"
        d.mkdir(parents=True, exist_ok=True)
        return d / f"{self.cur_step}_{side.name}.png"

    def step_side(self, side: Side, step, before_img):
        self.cur_step = step["id"]
        rec = {"side": side.name, "actions": [], "status": "ok", "want_before": before_img is not None}
        actions = list(step.get("both") or []) + list(step.get(side.name) or [])
        side.values = []
        side.popup_edit = None  # R-8: popup edit of the previous step is forgotten
        side.before_img = before_img
        side.open_before = None
        t0 = time.time()
        if getattr(side, "resized", None):
            side.resized = None
            side.window_size(None, None)
        # R-7: `"unsupported": {"orig"|"fp"|"both": reason}` - the target cannot be reached on that side (Flash
        # panels/dialogs of the reference, system dialogs): actions that fail there are skipped, the shot is taken
        # anyway and the side is logged as 'unsupported' (not 'unmapped'/'error'), so it does not count as a gap
        unsup = step.get("unsupported") or {}
        unsup = (unsup.get(side.name) or unsup.get("both")) if isinstance(unsup, dict) else None
        side.tool_failed = False
        lost0 = 0
        try:
            if side.name == "orig":
                side.ensure_foreground()
                side.o.focus()
                lost0 = getattr(side.h, "lost", 0)
                if getattr(side, "fg_taken", False):
                    # R-11: taken back in the reset before this step (or right now) - an error is retried
                    rec["fg_taken_before"] = True
                    side.fg_taken = False
            if unsup:
                for act in actions:
                    try:
                        self.run_actions(side, [act], rec)
                    except Exception as e:  # noqa: BLE001
                        rec.setdefault("skipped", []).append(f"{next(iter(act)) if isinstance(act, dict) else act}: {e}"[:200])
                        if side.name == "orig":
                            try:
                                side.h.call("close_menus")
                            except Exception:  # noqa: BLE001
                                pass
            else:
                self.run_actions(side, actions, rec)
            if side.name == "orig" and '"canvas' in json.dumps(step):
                # R-7 (TW-98, RE-9): zoom of the reference per canvas step
                side.log_zoom()
            if step.get("log_menu_items"):
                side.values.append({"menu_items": side.menu_items()})
            shot = step.get("shot")
            if shot:
                time.sleep(float(shot.get("delay_ms", 300)) / 1000)
                area = shot.get("area", "window")
                if area == "canvas":
                    rect = side.canvas_shot_rect()
                elif area == "canvas_view":
                    rect = side.area_rect("canvas")
                else:
                    rect = side.area_rect(area)
                if rect is None:
                    img, wr = side.window_image()
                    bb = diff_bbox(rec.get("_before") or before_img, img)
                    if not bb:
                        raise RuntimeError(f"area {area}: nothing opened (no difference to the state before)")
                    if side.name == "fp" and hasattr(side, "tight_popup"):
                        # R-6 (run 10, filters-blur-sharpen-noise/02,03): the difference box took the tool bar
                        # and the document tab along; only the menu columns (and sub menus touching them)
                        try:
                            tb = side.tight_popup(rec.get("_before") or before_img, img, bb)
                        except Exception as e:  # noqa: BLE001
                            tb = None
                            self.log(f"fp popup box: {e}")
                        if tb:
                            bb = tb
                            rec["area_from_tight"] = True
                    rect = [wr[0] + bb[0], wr[1] + bb[1], wr[0] + bb[2], wr[1] + bb[3]]
                    rec["area_from"] = "difference"
                    if area == "menu" and side.name == "fp":
                        # like the reference (popup windows only): leave out the highlighted menu bar entry
                        try:
                            bar = side.find(self.ui_map["menu_bar"]["orig"][0][0], None, True, [0, 0, 2000, 40])["rect"]
                            rect[1] = max(rect[1], bar[3] + 2)
                        except Exception:  # noqa: BLE001
                            pass
                rect = norm_rect(rect)
                rec["rect"] = [round(v) for v in rect]
                out = self.out / self.unit["unit"] / side.name / f"{step['id']}.png"
                side.grab(rect, out)
                # a window that is not painted yet (run 7: 'Vektordatei-Optionen' empty): wait and grab again
                for _ in range(5):
                    with Image.open(out) as im:
                        lo, hi = im.convert("L").getextrema()
                    if hi - lo > 8:
                        break
                    time.sleep(0.3)
                    side.grab(rect, out)
                rec["shot"] = str(out)
            self.run_actions(side, (step.get("after") or []) + (step.get("cleanup") or []), rec)
        except Unsupported as e:
            rec.update(status="unsupported", error=str(e))
        except Unmapped as e:
            rec.update(status="unmapped", error=str(e))
        except Exception as e:  # noqa: BLE001
            rec.update(status="error", error=f"{type(e).__name__}: {e}", trace=traceback.format_exc(limit=8)[-1500:])
        if unsup:
            # shot (if any) kept; the status says why the side is not measured
            rec["unsupported_reason"] = str(unsup)
            if rec["status"] in ("ok", "unmapped", "error"):
                if rec["status"] != "ok":
                    rec["skipped_error"] = rec.get("error")
                rec.update(status="unsupported", error=f"{unsup} (shot only)")
                rec.pop("trace", None)
        lost = getattr(side.h, "lost", 0) - lost0 if side.name == "orig" else 0
        if lost > 0 or getattr(side, "tool_failed", False):
            rec["focus_lost"] = {"take_backs": lost, "tool_failed": bool(getattr(side, "tool_failed", False))}
        rec.pop("_before", None)
        rec.pop("want_before", None)
        rec["values"] = side.values
        rec["s"] = round(time.time() - t0, 2)
        limit = float(self.cfg.get("step_timeout_s", 90))
        if rec["s"] > limit and rec["status"] == "ok":
            rec.update(status="slow", error=f"step took {rec['s']}s (limit {limit}s)")
        return rec

    def run_unit(self, unit_name: str):
        path = SCRIPTS / f"{unit_name}.json"
        unit = json.loads(path.read_text(encoding="utf-8"))
        unit.setdefault("unit", unit_name)
        self.unit = unit
        udir = self.out / unit_name
        steps = [s for s in unit["steps"] if not self.only or s["id"] in self.only]
        self.log(f"unit {unit_name}: {len(steps)} steps")
        if self.dry:
            for s in steps:
                acts = [next(iter(a)) if isinstance(a, dict) else a for k in ("both", "orig", "fp", "after") for a in s.get(k) or []]
                self.log(f"  {s['id']}: {', '.join(acts)} -> {(s.get('shot') or {}).get('area')}")
            return
        for sub in ("orig", "fp", "cmp"):
            d = udir / sub
            d.mkdir(parents=True, exist_ok=True)
            for f in d.glob("*.png"):
                f.unlink()
        logf = udir / "log.jsonl"
        logf.write_text("", encoding="utf-8")
        if not self.lock.acquire(float(self.cfg["orig"].get("lock_wait_min", 30)) * 60):
            self.log("lock not free - unit skipped")
            return
        try:
            self.ensure_programs()
            # R-6 (run 10: access violations / restarts in color-popup, menu-view, panel-misc, xs-file-formats):
            # a fresh reference instance before these script-heavy units (config `orig.restart_before_units`)
            rb = self.cfg["orig"].get("restart_before_units", self.RESTART_BEFORE)
            if unit_name in rb and getattr(self, "units_played", 0):
                try:
                    self.log(f"unit {unit_name}: fresh reference instance before a script-heavy unit")
                    self.orig.restart(f"planned before script-heavy unit {unit_name}")
                except Exception as e:  # noqa: BLE001
                    self.log(f"restart before unit failed: {e}")
            self.units_played = getattr(self, "units_played", 0) + 1
            self.orig_fails = 0
            aborted: dict = {}
            for side in self.sides.values():
                try:
                    side.prepare()
                    if side.name != "orig":
                        side.pending_colors = True  # R-9 (TW-107): default colours with the unit's first document
                    if side.name == "orig":
                        side.log_foreground(f"unit start {unit_name}")
                        self.log_shortcut_sets()
                        try:
                            side.ensure_foreground()
                        except Exception as e:  # noqa: BLE001
                            self.log(f"unit start: {e}")
                        self.orig.kill_hung_own()
                        # defined start state: no questions open, every document closed without saving
                        side.reset_light()
                        self.orig.close_all_docs()
                        # defined layout: no floating panel groups left over from earlier units
                        try:
                            side.close_floating()
                        except Exception as e:  # noqa: BLE001
                            self.log(f"close floating panels: {e}")
                        try:
                            side.ensure_panel("properties")
                        except Exception as e:  # noqa: BLE001
                            self.log(f"properties panel: {e}")
                        try:
                            side.reset_unit()
                        except LayoutError as e:
                            # no guessed anchor (R-2): this unit is not played in the reference program
                            aborted["orig"] = f"unit aborted in the reference program: {e}"
                            self.log(f"UNIT ABORTED (orig): {e}")
                        except Exception as e:  # noqa: BLE001
                            self.log(f"unit reset orig: {e}")
                    if unit.get("testfile") and not self.testfile(unit["testfile"]).is_dir():
                        # a folder (T11 = import files) is no document: the steps open its files themselves
                        self.open_testfile(side, unit["testfile"])
                except Exception as e:  # noqa: BLE001
                    self.log(f"start {side.name}: {e} - recovering")
                    self.recover(side)
            for i, step in enumerate(steps):
                entry = {"unit": unit_name, "step": step["id"], "n": i, "check": step.get("check"),
                         "area": (step.get("shot") or {}).get("area"), "time": time.strftime("%Y-%m-%d %H:%M:%S")}
                if step.get("cursor") or (step.get("shot") or {}).get("cursor"):
                    entry["result"] = {"orig": {"status": "skipped", "error": "cursor image cannot be captured"},
                                       "fp": {"status": "skipped", "error": "cursor image cannot be captured"}}
                    self.write_log(logf, entry)
                    continue
                entry["result"] = {}
                for name, side in self.sides.items():
                    if name == "fp" and step.get("orig_only"):
                        continue
                    if name == "orig" and step.get("fp_only"):
                        continue
                    if name in aborted:
                        entry["result"][name] = {"side": name, "actions": [], "status": "aborted", "error": aborted[name]}
                        continue
                    if name == "orig":
                        # the program may have ended (crash, quit) since the last step: start it again
                        try:
                            alive = self.orig.running()
                        except Exception:  # noqa: BLE001
                            alive = False
                        if not alive:
                            self.log("reference program not running - starting it again")
                            self.recover(side)
                    try:
                        side.reset_light()
                    except Exception as e:  # noqa: BLE001
                        self.recover(side)
                    before = None
                    if name == "orig":
                        side.windows_before = set()
                        if (step.get("shot") or {}).get("area") == "popup" or "popup.visible" in json.dumps(step):
                            try:
                                side.windows_before = {w["hwnd"] for w in side.h.list("windows")}
                            except Exception:  # noqa: BLE001
                                pass
                    if (step.get("shot") or {}).get("area") in ("menu", "popup") or step.get("log_menu_items") \
                            or re.search(r'"(log_menu_items|menu\.open|menu\.disabled_items)"', json.dumps(step)):
                        try:
                            before, _ = side.window_image()
                        except Exception:  # noqa: BLE001
                            before = None
                    rec = self.step_side(side, step, before)
                    # R-7 (run 11, xs-canvas/08…12): the focus was lost during the step (or the pointer tool could
                    # not be chosen): the result may come from a wrong tool or state. The step is marked 'retry' and
                    # played again after a reset; still disturbed -> error.
                    # R-11 (run 15, xs-shortcuts/10_nudge_point): also an error right after a take-back before the
                    # step (`fg_taken_before`); up to `orig_step_retries` repeats (default 2, was 1)
                    tries, max_tries = 0, int(self.cfg.get("orig_step_retries", 2))
                    while name == "orig" and tries < max_tries and (
                            (rec.get("focus_lost") and rec["status"] in ("ok", "error", "unmapped"))
                            or (rec.get("fg_taken_before") and rec["status"] == "error")):
                        tries += 1
                        why = (f"focus lost during the step {rec['focus_lost']}" if rec.get("focus_lost")
                               else f"error after a foreground take-back before the step ({str(rec.get('error'))[:80]})")
                        self.log(f"  {step['id']}: {why} - tool {side.active_tool()} - repeating the step "
                                 f"({tries}/{max_tries})")
                        first = dict(rec, status="retry")
                        try:
                            side.reset_light()
                            side.ensure_foreground()
                            side.do("tool", "tool.pointer", {})
                        except Exception as e:  # noqa: BLE001
                            self.log(f"  retry reset: {e}")
                        rec = self.step_side(side, step, before)
                        rec["retry_of"] = {k: first.get(k) for k in ("status", "error", "focus_lost", "fg_taken_before", "shot")}
                        rec["retries"] = tries
                    if tries and rec.get("focus_lost") and rec["status"] == "ok":
                        rec.update(status="error", error=f"focus lost during the step again (retried {tries}x): {rec['focus_lost']}")
                    entry["result"][name] = rec
                    if rec["status"] in ("error",):
                        self.recover(side)
                    if name == "orig":
                        self.count_orig(rec["status"], rec.get("error", ""))
                st = {k: v["status"] for k, v in entry["result"].items()}
                self.log(f"  {step['id']}: {st}")
                self.write_log(logf, entry)
        finally:
            self.cleanup()
            try:
                # R-6 (Q-41): the documents are closed now - a test file the unit changed is put back
                self.restore_testfiles(f"after unit {unit_name}")
            finally:
                self.lock.release()
        self.compare(unit_name)

    def write_log(self, logf: Path, entry):
        with open(logf, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False, default=str) + "\n")

    def cleanup(self):
        """No open menus/dialogs, test documents closed, Firepixel instance ended."""
        if self.orig is not None:
            try:
                side = self.sides["orig"]
                side.reset_light()
                for p in self.opened.get("orig", set()):
                    side.close_doc(p)
                self.orig.host.call("dialog_close", mode="cancel")
            except Exception as e:  # noqa: BLE001
                self.log(f"cleanup orig: {e}")
        self.opened = {}
        if self.fp is not None:
            self.fp.stop()

    def compare(self, unit_name):
        u = self.out / unit_name
        if not any((u / "orig").glob("*.png")) and not any((u / "fp").glob("*.png")):
            return
        subprocess.run([sys.executable, "-I", str(COMPARE), ".", "--ref-root", str(u / "orig"), "--fp-root", str(u / "fp"),
                        "--out-root", str(u / "cmp")], check=False)

    def shutdown(self):
        if self.orig is not None:
            if self.orig.started_by_us is False:
                # an instance was running when the run began: leave one running (restart it if it ended)
                try:
                    if not self.orig.running():
                        self.log("reference program ended during the run - starting it again")
                        self.orig.start()
                    self.orig.decline_dialogs()
                except Exception as e:  # noqa: BLE001
                    self.log(f"restart orig at the end failed: {e}")
            if self.orig.started_by_us:
                try:
                    self.orig.jsf("fw.quit();", timeout=10)
                except Exception:  # noqa: BLE001
                    pass
            self.orig.host.stop()


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--units", help="comma separated unit ids (file names in tools/parity/scripts without .json)")
    ap.add_argument("--all", action="store_true", help="every script except _selftest*")
    ap.add_argument("--steps", help="only these step ids")
    ap.add_argument("--dry-run", action="store_true", help="only list what would be done")
    ap.add_argument("--config", type=Path, default=DEFAULT_CONFIG)
    ap.add_argument("--no-gallery", action="store_true")
    ap.add_argument("--gallery-only", action="store_true", help="rebuild index.html from existing results")
    ap.add_argument("--write-testfile-sums", action="store_true",
                    help="write TESTFILES.sha256 from the current test files (only after checking them) and stop")
    a = ap.parse_args(argv)
    cfg = json.loads(a.config.read_text(encoding="utf-8"))
    if a.gallery_only:
        print(gallery.build(Path(cfg["out_dir"])))
        return 0
    if a.write_testfile_sums:
        print(Runner(cfg, True, None).write_testfile_sums())
        return 0
    units = [u.strip() for u in (a.units or "").split(",") if u.strip()]
    if a.all:
        units += sorted(p.stem for p in SCRIPTS.glob("*.json") if not p.stem.startswith("_"))
    if not units:
        ap.error("give --units or --all")
    r = Runner(cfg, a.dry_run, set(a.steps.split(",")) if a.steps else None)
    # R-6 (Q-41): no run with a changed test file
    try:
        r.protect_testfiles()
    except TestfilesChanged as e:
        r.log(f"RUN ABORTED: {e}")
        return 2
    if not a.dry_run:
        r.log_foreign_windows()
    try:
        for u in dict.fromkeys(units):
            try:
                r.run_unit(u)
            except TestfilesChanged as e:
                r.log(f"RUN ABORTED: {e}")
                break
            except Exception as e:  # noqa: BLE001
                r.log(f"unit {u} aborted: {e}")
                traceback.print_exc()
    finally:
        if not a.dry_run:
            r.shutdown()
    if not a.dry_run and not a.no_gallery:
        print(gallery.build(Path(cfg["out_dir"])))
    return 0


if __name__ == "__main__":
    sys.exit(main())
