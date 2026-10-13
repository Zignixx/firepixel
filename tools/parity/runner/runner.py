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


class Unsupported(Exception):
    """Action the runner cannot perform (logged as 'unsupported', not as a failure of the program)."""


class Unmapped(Exception):
    """Logical UI key without an entry for this side in ui_map.json."""


# --------------------------------------------------------------------------- helpers
def clean_label(s: str) -> str:
    return re.sub(r"(\.\.\.|…)$", "", str(s).replace("&", "")).strip()


def split_path(p) -> list[str]:
    if isinstance(p, list):
        return [clean_label(x) for x in p]
    parts = re.split(r"\s*(?:>|▸|/)\s*", str(p)) if (">" in str(p) or "▸" in str(p)) else str(p).split("/")
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


def diff_bbox(a: Image.Image | None, b: Image.Image | None, pad: int = 6):
    """Bounding box of what changed between two equally sized images (used for popup/menu areas)."""
    if a is None or b is None or a.size != b.size:
        return None
    d = ImageChops.difference(a.convert("RGB"), b.convert("RGB")).convert("L").point(lambda v: 255 if v > 24 else 0)
    bb = d.getbbox()
    if not bb:
        return None
    return [max(0, bb[0] - pad), max(0, bb[1] - pad), min(a.width, bb[2] + pad), min(a.height, bb[3] + pad)]


OPENERS = {"menu", "menu_open", "hover_menu", "open", "click", "right_click", "double_click", "click_text", "right_click_text",
           "click_button", "toolbox_click", "choose", "key", "ui_find_click", "ui_action", "command", "menu_item", "hover"}

KEY_ALIASES = {"escape": "esc", "enter": "enter", "return": "enter", "del": "delete", "control": "ctrl", "strg": "ctrl",
               "cmd": "ctrl", "umschalt": "shift", "option": "alt"}
EGUI_KEYS = {"esc": "Escape", "enter": "Enter", "tab": "Tab", "space": "Space", "delete": "Delete", "backspace": "Backspace",
             "left": "ArrowLeft", "right": "ArrowRight", "up": "ArrowUp", "down": "ArrowDown", "home": "Home", "end": "End",
             "pageup": "PageUp", "pagedown": "PageDown", "insert": "Insert", "plus": "Plus", "minus": "Minus",
             "comma": "Comma", "period": "Period"}


def parse_combo(c: str) -> tuple[list[str], str]:
    parts = [p.strip() for p in re.split(r"\+(?!$)", str(c)) if p.strip()]
    parts = [KEY_ALIASES.get(p.lower(), p) for p in parts]
    mods = [p.lower() for p in parts[:-1]]
    key = parts[-1]
    lk = key.lower()
    if lk.startswith("arrow"):
        lk = lk[5:]
    return mods, KEY_ALIASES.get(lk, lk)


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


# --------------------------------------------------------------------------- sides
class Side:
    name = "?"

    def __init__(self, runner):
        self.r = runner
        self.values: list = []
        self.doc_size = (0, 0)

    def ui(self, key):
        t = self.r.ui_map.get("targets", {})
        if key not in t:
            raise Unmapped(f"ui key not in ui_map.json: {key}")
        e = t[key]
        loc = e.get(self.name) if isinstance(e, dict) else None
        if not loc:
            raise Unmapped(f"ui key {key} not located for {self.name}")
        return loc

    def panel_info(self, p):
        info = self.r.ui_map.get("panels", {}).get(norm_panel(p))
        if info is None:
            # a title given directly
            for v in self.r.ui_map.get("panels", {}).values():
                if isinstance(v, dict) and v.get("title") == p:
                    return v
            return {"fp": p if str(p).startswith("window.") else None, "title": p, "menu": f"Fenster > {p}"}
        return info


class OrigSide(Side):
    name = "orig"

    def __init__(self, runner, orig: Original):
        super().__init__(runner)
        self.o = orig
        self.h = orig.host

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
        self.release_held()
        try:
            self.h.call("close_menus")
        except OrigError:
            pass
        self.h.call("dialog_close", mode="cancel")
        time.sleep(0.2)

    def open_testfile(self, path: Path):
        self.reset_light()
        self.o.open_doc(path)
        lines = self.o.jsf('var d = fw.getDocumentDOM(); fp_log("RET " + d.width + "," + d.height);')
        ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
        if ret:
            w, h = ret[-1].split(",")
            self.doc_size = (float(w), float(h))
        self.o.focus()
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
            d = ImageChops.difference(im, Image.new("RGB", im.size, bg)).convert("L").point(lambda q: 255 if q > 10 else 0)
            bb = d.getbbox()
        w, h = self.doc_size
        if bb and w:
            page = [v[0] + bb[0], v[1] + bb[1], v[0] + bb[2], v[1] + bb[3]]
            # a page touching the view border is cut off (scrolled) - then the centre rule below is used
            cut = bb[0] <= 1 or bb[1] <= 1 or bb[2] >= im.width - 1 or bb[3] >= im.height - 1
            if not cut:
                self.page = page
                return page
        ox = (v[0] + v[2]) / 2 - w / 2
        oy = (v[1] + v[3]) / 2 - h / 2
        self.page = [ox, oy, ox + w, oy + h]
        return self.page

    def fit_if_needed(self):
        """Large documents (e.g. T10): 'fit all' in both programs so that every document point is reachable."""
        v = self.h.call("view")
        w, h = self.doc_size
        if w > (v[2] - v[0]) or h > (v[3] - v[1]):
            self.key(self.r.cfg["orig"].get("fit_all_key", "ctrl+alt+0"))
            time.sleep(0.6)
            return True
        return False

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
            if "canvas" in target:
                return self.doc_to_screen(*target["canvas"]), None
            if "window_rel" in target:
                c = self.client()
                return (c[0] + target["window_rel"][0], c[1] + target["window_rel"][1]), None
            if "area" in target and "pos" in target:
                r = self.area_rect(target["area"])
                return (r[0] + target["pos"][0], r[1] + target["pos"][1]), None
            if "target" in target:
                return self.resolve(target["target"])
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
            for k in ("layer_row", "object_row", "pref_category", "popup_swatch", "panel_swatch", "list_row",
                      "tree_item", "history_row"):
                if k in target:
                    raise Unsupported(f"target form {{{k}}} not supported for orig yet")
            loc = target
        elif isinstance(target, str) and re.match(r"^dialog\.(field|combo|check|button)\.\d+$", target):
            kind, i = target.split(".")[1:]
            cls = {"field": "Edit", "combo": "ComboBox", "check": "Button", "button": "Button"}[kind]
            ch = self.h.call("find_child", scope="dialog", title=None, **{"class": cls}, index=int(i))
            r = ch["rect"]
            return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
        elif isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            loc = self.ui(target)
        elif isinstance(target, str) and re.match(r"^[a-z_]+(\.[a-z0-9_]+)+$", target):
            raise Unmapped(f"ui key not in ui_map.json: {target}")
        else:
            return self.label_point(str(target))
        return self.locate(loc)

    def win_at(self, pt):
        """Win32 window under a point (edit fields / combo boxes inside panels are real child windows)."""
        try:
            w = self.h.call("at", x=int(pt[0]), y=int(pt[1]))
        except OrigError:
            return None
        if not w:
            return None
        if w.get("class") == "Edit" and (w.get("parent") or {}).get("class") == "ComboBox":
            w = dict(w["parent"], title="")
        return w if w.get("class") in ("Edit", "ComboBox") else None

    def locate(self, loc):
        if "rel" in loc and "panel" in loc:
            p = self.h.call("panel", name=self.panel_info(loc["panel"]).get("title") or loc["panel"])
            pt = (p["palette"][0] + loc["rel"][0], p["palette"][1] + loc["rel"][1])
            return pt, self.win_at(pt)
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
        if "child" in loc or "dialog_child" in loc:
            spec = loc.get("child") or loc.get("dialog_child")
            scope = "dialog" if "dialog_child" in loc else (self.panel_info(loc["panel"]).get("title") if loc.get("panel") else "main")
            ch = self.h.call("find_child", scope=scope, title=spec.get("title"), **{"class": spec.get("class")}, index=spec.get("index", 0))
            r = ch["rect"]
            return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), ch
        if "dialog_button" in loc:
            return None, {"dialog_button": loc["dialog_button"]}
        if "label" in loc:
            return self.label_point(loc["label"])
        if "canvas" in loc:
            return self.doc_to_screen(*loc["canvas"]), None
        raise Unmapped(f"orig locator not understood: {loc}")

    def menu_item_point(self, label):
        lab = clean_label(label)
        for _ in range(8):
            for m in self.h.call("menus").get("menus") or []:
                for it in m.get("items") or []:
                    if not it.get("separator") and it.get("text") == lab:
                        r = it["rect"]
                        return ((r[0] + r[2]) // 2, (r[1] + r[3]) // 2), {"menu_item": it}
            time.sleep(0.15)
        raise Unmapped(f"menu item not found in open menus: {label}")

    def menu_index_point(self, index):
        """Owner-drawn popup menus (tool flyouts) have no item texts: raw item index (separators count)."""
        ms = self.h.call("menus").get("menus") or []
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
        raise Unmapped(f"label not found as window text in orig: {label}")

    # ------------------------------------------------------------ areas
    def area_rect(self, area: str):
        if area == "window":
            return self.client()
        if area == "canvas":
            return self.h.call("view")
        if area == "inspector":
            return self.h.call("panel", name="Eigenschaften")["rect"]
        if area.startswith("panel:"):
            info = self.panel_info(area[6:])
            if info.get("title"):
                return self.h.call("panel", name=info["title"])["rect"]
            if info.get("orig_class"):
                return self.h.call("find_child", scope="main", title=None, **{"class": info["orig_class"]}, index=0)["rect"]
            raise Unmapped(f"panel area {area} unknown for orig")
        if area == "dialog":
            ds = self.h.list("dialogs")
            if not ds:
                raise OrigError("no dialog open")
            return ds[0]["rect"]
        if area == "menu":
            ms = self.h.call("menus").get("menus") or []
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
            r = bbox_union([w["rect"] for w in ws])
            if r:
                return r
            return None  # caller falls back to the difference box
        raise Unsupported(f"shot area {area}")

    def grab(self, rect, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
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
        self.o.focus()
        self.h.call("key", combo="+".join(mods + [k]))

    def jsf_value(self, expr):
        lines = self.o.jsf(f'var __v = ({expr}); fp_log("RET " + (typeof __v == "object" && __v !== null ? (__v.toSource ? __v.toSource() : String(__v)) : String(__v)));')
        ret = [ln[4:] for ln in lines if ln.startswith("RET ")]
        return ret[-1] if ret else None

    def select(self, names):
        names = names if isinstance(names, list) else [names]
        code = ("var d = fw.getDocumentDOM(); var want = %s; var sel = [];\n"
                "for (var i = 0; i < d.layers.length; i++) { var fr = d.layers[i].frames[d.currentFrameNum];\n"
                "  for (var j = 0; j < fr.elements.length; j++) { var e = fr.elements[j];\n"
                "    for (var k = 0; k < want.length; k++) if (e.name == want[k]) sel.push(e); } }\n"
                "d.selectNone(); fw.selection = sel; fp_log('RET ' + sel.length);") % json.dumps(names, ensure_ascii=True)
        lines = self.o.jsf(code)
        n = [ln for ln in lines if ln.startswith("RET ")]
        if not n or n[-1] == "RET 0":
            raise OrigError(f"objects not found: {names}")

    # ------------------------------------------------------------ verbs
    def do(self, verb, arg, act):
        r = self.r
        if verb in ("open_testfile", "reopen"):
            r.open_testfile(self, arg)
        elif verb in ("close_all", "close_all_documents"):
            for p in r.opened.get(self.name, set()):
                self.close_doc(p)
        elif verb in ("menu", "menu_open", "hover_menu"):
            self.h.call("menu", path=split_path(arg), final="open", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
        elif verb == "menu_item":
            self.h.call("menu", path=split_path(arg), final="invoke", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
        elif verb == "command":
            path = r.ui_map.get("commands", {}).get(arg)
            if not path:
                raise Unmapped(f"command {arg} has no orig menu path in ui_map.json")
            self.h.call("menu", path=split_path(path), final="invoke", bar=r.ui_map["menu_bar"]["orig"], timeout=40)
        elif verb == "close_dialogs":
            self.reset_light()
        elif verb == "key":
            for _ in range(int(act.get("repeat", 1) or 1)):
                self.key(arg)
        elif verb == "type":
            self.o.focus()
            self.h.call("type", text=str(arg), timeout=60)
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
            elif isinstance(k, dict):
                self.reset_light()
                self.click_at(self.locate(k)[0])
            else:
                self.reset_light()
                self.key(k)
        elif verb == "panel":
            info = self.panel_info(arg)
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
            self.values.append({"expr": expr, "value": self.jsf_value(expr)})
        elif verb in ("click", "double_click", "right_click", "hover", "mouse_down", "mouse_up", "click_text",
                      "double_click_text", "right_click_text", "hover_text", "click_button", "toolbox_click", "uia_click",
                      "open", "canvas_click", "canvas_double_click", "canvas_move", "canvas_hover", "window_click"):
            self.pointer(verb, arg, act)
        elif verb in ("drag", "canvas_drag"):
            a = arg
            fr = self.resolve({"canvas": a["from"]} if verb == "canvas_drag" or isinstance(a.get("from"), list) else a["from"])[0]
            if "to" in a:
                to = self.resolve({"canvas": a["to"]} if verb == "canvas_drag" or isinstance(a.get("to"), list) else a["to"])[0]
            elif "by" in a:
                to = (fr[0] + a["by"][0], fr[1] + a["by"][1])
            else:
                raise Unsupported("drag without to/by")
            self.o.focus()
            self.h.call("drag", x1=fr[0], y1=fr[1], x2=to[0], y2=to[1], mods=a.get("mods") or [], steps=a.get("steps", 20), timeout=60)
        elif verb == "choose":
            self.choose(arg, act)
        elif verb in ("set_field", "set", "set_dialog_field"):
            self.set_field(arg if isinstance(arg, dict) else act)
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
            ds = [d for d in self.h.list("dialogs")]
            if not ds:
                raise OrigError("no file dialog open")
            ed = self.h.call("find_child", scope="dialog", title=None, **{"class": "Edit"}, index=0)
            self.h.call("set_text", hwnd=ed["hwnd"], text=path)
            self.h.call("dialog_close", mode="ok")
        elif verb == "file_dialog_cancel":
            self.h.call("dialog_close", mode="cancel")
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
            try:
                g = self.h.call("panel", name=info["title"])
            except OrigError:
                self.do("panel", p, {})
                g = self.h.call("panel", name=info["title"])
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
                pt = self.doc_to_screen(*a["pos"])
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

    def pointer(self, verb, arg, act):
        mods = act.get("mods") or act.get("hold_key") or []
        if isinstance(mods, str):
            mods = [mods]
        if verb.startswith("canvas_"):
            pt, info = self.doc_to_screen(*(arg if isinstance(arg, list) else arg.get("pos") or arg.get("at"))), None
            verb = {"canvas_click": "click", "canvas_double_click": "double_click"}.get(verb, "hover")
        else:
            target = arg
            if isinstance(arg, dict) and "target" in arg:
                mods = arg.get("mods") or arg.get("hold_key") or mods
                target = arg["target"]
            pt, info = self.resolve(target)
        if info and "dialog_button" in info:
            self.h.call("dialog_close", mode=info["dialog_button"])
            return
        loc = {} if verb.startswith("canvas_") else self.loc_of(arg.get("target") if isinstance(arg, dict) and "target" in arg else arg)
        if loc.get("select") and info and info.get("class") == "ComboBox" and verb in ("click", "open"):
            self.h.call("combo_select", hwnd=info["hwnd"], text=clean_label(loc["select"]))
            return
        if loc.get("hold_ms") and verb in ("click", "open"):
            self.o.focus()
            self.h.call("down", x=pt[0], y=pt[1])
            time.sleep(float(loc["hold_ms"]) / 1000)
            self.h.call("up", x=pt[0], y=pt[1])
            time.sleep(0.3)
            return
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
        else:
            button = "right" if verb.startswith("right") else "left"
            count = 2 if verb.startswith("double") else 1
            self.click_at(pt, button, count, mods)
        time.sleep(0.25)

    def loc_of(self, target):
        if isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            e = self.r.ui_map["targets"][target]
            return (e.get(self.name) if isinstance(e, dict) else None) or {}
        return target if isinstance(target, dict) else {}

    def menu_path(self, parts, final_click=True):
        """Walk open popup menus by item text: hover sub menus, click the last entry."""
        for i, part in enumerate(parts):
            pt, info = self.menu_item_point(part)
            last = i == len(parts) - 1
            if last and final_click:
                if (info.get("menu_item") or {}).get("grayed"):
                    raise OrigError(f"menu item disabled: {part}")
                self.click_at(pt)
            else:
                self.o.focus()
                self.h.call("move", x=pt[0], y=pt[1])
                time.sleep(0.45)

    def choose(self, a, act=None):
        act = act or {}
        target, item = a.get("target"), a.get("item")
        pt, info = self.resolve(target) if target else (None, None)
        if info and info.get("class") == "ComboBox":
            self.h.call("combo_select", hwnd=info["hwnd"], text=clean_label(item))
            return
        if target:
            self.pointer("open", target, {})
            time.sleep(0.3)
        oi = act.get("orig_index", a.get("orig_index"))
        if oi is not None:
            self.click_at(self.menu_index_point(int(oi)))
            return
        if not self.h.call("menus").get("menus"):
            raise Unsupported("choose: target opened no menu and is no Win32 combo box")
        self.menu_path(split_path(item or ""))

    def set_field(self, a):
        target, value = a.get("target") or a.get("set_field") or a.get("set"), a.get("value")
        pt, info = self.resolve(target)
        if info and info.get("class") in ("Edit",):
            self.h.call("set_text", hwnd=info["hwnd"], text=str(value))
        if pt is None:
            raise Unsupported("set_field without a point")
        self.click_at(pt, count=2)
        self.h.call("key", combo="ctrl+a")
        self.h.call("type", text=str(value))
        self.h.call("key", combo="enter")

    def record_ui(self, key):
        if key in ("dialog.fields", "dialog.open", "dialog.title"):
            return self.h.list("dialogs")
        if key in ("menu.open", "menu.disabled_items"):
            return self.menu_items()
        if key == "list.items":
            return None
        return {"title": self.h.call("main")["title"], "dialogs": [d.get("title") for d in self.h.list("dialogs")]}

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

    def open_testfile(self, path: Path):
        self.reset_light()
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

    def find(self, text, role=None, exact=True, region=None, index=0):
        q = {"text": clean_label(text), "exact": exact}
        if role:
            q["role"] = role
        if region:
            q["region"] = region
        hits = (self.call("ui_find", q) or {}).get("hits") or []
        if not hits and exact:
            q["exact"] = False
            hits = (self.call("ui_find", q) or {}).get("hits") or []
        if len(hits) <= index:
            raise Unmapped(f"fp ui_find: '{text}' not found")
        return hits[index]

    def script_value(self, code):
        v = self.call("run_script", {"code": code})
        sc = v.get("script") if isinstance(v, dict) else None
        if isinstance(sc, dict):
            if sc.get("error") or sc.get("thrown"):
                raise BridgeError(f"run_script: {sc.get('error') or sc.get('thrown')}")
            out = sc.get("result")
            return out if not sc.get("notImplemented") else {"result": out, "notImplemented": sc["notImplemented"]}
        return v

    def find_role(self, role, index=0):
        hits = (self.call("ui_find", {"role": role, "limit": 200}) or {}).get("hits") or []
        hits.sort(key=lambda h: (round(h["rect"][1] / 4), h["rect"][0]))
        if len(hits) <= index:
            raise Unmapped(f"fp: no {role} #{index}")
        return hits[index]

    def resolve(self, target):
        if isinstance(target, dict):
            if "canvas" in target:
                return self.doc_to_win(*target["canvas"])
            if "window_rel" in target:
                return list(target["window_rel"])
            if "area" in target and "pos" in target:
                r = self.area_rect(target["area"])
                return [r[0] + target["pos"][0], r[1] + target["pos"][1]]
            if "target" in target:
                return self.resolve(target["target"])
            if "dialog_field" in target:
                return self.find_role("text_input", int(target["dialog_field"]))["center"]
            if "button" in target:
                return self.find(target["button"], "button")["center"]
            if "text" in target:
                return self.find(target["text"])["center"]
            if "find" in target:
                reg = None
                if target.get("panel"):
                    reg = self.area_rect("panel:" + target["panel"])
                return self.find(target["find"], target.get("role"), target.get("exact", True), reg, target.get("index", 0))["center"]
            if "menu_item" in target:
                return self.find(target["menu_item"], None, True, [0, 30, 10000, 10000])["center"]
            if "handle" in target:
                return self.handle_point(target["handle"])
            if "preview_pane" in target:
                return self.find(f"Vorschaufeld {int(target['preview_pane']) + 1}")["center"]
            if "dialog_rel" in target:
                d = self.area_rect("dialog")
                return [d[0] + target["dialog_rel"][0], d[1] + target["dialog_rel"][1]]
            for k in ("layer_row", "object_row", "page_row", "state_row", "history_row", "list_row", "tree_item", "pref_category",
                      "layer_name", "page_name"):
                if k in target:
                    v = target[k]
                    return self.find(v[-1] if isinstance(v, list) else v)["center"]
            raise Unsupported(f"fp target form {list(target)}")
        if isinstance(target, str) and re.match(r"^dialog\.(field|combo|check|button)\.\d+$", target):
            kind, i = target.split(".")[1:]
            role = {"field": "text_input", "combo": "combo_box", "check": "check_box", "button": "button"}[kind]
            return self.find_role(role, int(i))["center"]
        if isinstance(target, str) and target in self.r.ui_map.get("targets", {}):
            return self.resolve(self.ui(target))
        if isinstance(target, str) and re.match(r"^[a-z_]+(\.[a-z0-9_]+)+$", target):
            raise Unmapped(f"ui key not in ui_map.json: {target}")
        return self.find(str(target))["center"]

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
            hits = (self.call("ui_find", {"role": "window", "limit": 50}) or {}).get("hits") or []
            hits = [h for h in hits if h.get("text")]
            if hits:
                return hits[0]["rect"]
            tb = [h for h in (self.call("ui_find", {"text": "Titelleiste", "exact": False, "limit": 20}) or {}).get("hits") or []
                  if str(h.get("text", "")).startswith("Titelleiste")]
            if not tb:
                raise BridgeError("no dialog window found")
            l, t, r, _ = tb[0]["rect"]
            inside = (self.call("ui_find", {"region": [l, t, r, H], "limit": 600}) or {}).get("hits") or []
            bottom = max([h["rect"][3] for h in inside if h["rect"][0] >= l - 2 and h["rect"][2] <= r + 2] + [t + 40])
            return [l - 1, t - 1, r + 1, min(H, bottom + 12)]
        if area in ("menu", "popup"):
            return None  # difference box
        raise Unsupported(f"shot area {area}")

    def panel_rect(self, info, st, doc):
        W, H = st["window"]["width"], st["window"]["height"]
        ca = doc["canvas_area"] if doc else [60, 100, W - 280, H - 120]
        if info.get("title") is None:  # tools column
            return [0, ca[1] - 4, ca[0], H]
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
        for h in allh:
            if h.get("text") in titles and h["rect"][1] > r[3] + 20:
                below.append(h["rect"][1])
        bottom = min(below) - 4 if below else H
        if cx >= ca[0] and cx <= ca[2]:
            return [ca[0] - 26, r[1] - 4, ca[2] + 12, H]
        return [col[0] + 1, r[1] - 4, col[1], bottom]

    def grab(self, rect, path: Path):
        path.parent.mkdir(parents=True, exist_ok=True)
        args = {"max_size": 8192}
        if rect:
            args["rect"] = [float(v) for v in rect]
        _, imgs = self.fp.call("screenshot", args, timeout=60)
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
        names = names if isinstance(names, list) else [names]
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
            for p in r.opened.get(self.name, set()):
                self.close_doc(p)
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
                    self.inp([{"type": "move", "pos": hit["center"]}, {"type": "wait", "ms": 350}])
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
            st = self.state()
            if pid not in st.get("visible_panels", []):
                self.call("ui_action", {"panel": pid})
        elif verb in ("jsf", "run_script"):
            code = arg if isinstance(arg, str) else arg.get("code")
            self.values.append({"jsf": code[:200], "out": self.script_value(code)})
        elif verb in ("log_value", "log"):
            expr = arg if isinstance(arg, str) else arg.get("value") or arg.get("expr")
            if isinstance(arg, dict) and not expr:
                raise Unsupported(f"log {arg}")
            self.values.append({"expr": expr, "value": self.script_value(expr)})
        elif verb in ("click", "double_click", "right_click", "hover", "mouse_down", "mouse_up", "click_text",
                      "double_click_text", "right_click_text", "hover_text", "click_button", "toolbox_click",
                      "open", "canvas_click", "canvas_double_click", "canvas_move", "canvas_hover", "window_click",
                      "ui_find_click"):
            self.pointer(verb, arg, act)
        elif verb in ("drag", "canvas_drag"):
            a = arg
            fr = self.resolve({"canvas": a["from"]} if verb == "canvas_drag" or isinstance(a.get("from"), list) else a["from"])
            if "to" in a:
                to = self.resolve({"canvas": a["to"]} if verb == "canvas_drag" or isinstance(a.get("to"), list) else a["to"])
            elif "by" in a:
                to = [fr[0] + a["by"][0], fr[1] + a["by"][1]]
            else:
                raise Unsupported("drag without to/by")
            ev = {"type": "drag", "from": fr, "to": to, "steps": a.get("steps", 20)}
            if a.get("mods"):
                ev["modifiers"] = a["mods"]
            self.inp([ev])
        elif verb == "choose":
            a = arg
            if a.get("target"):
                self.pointer("open", a["target"], {})
            for part in split_path(a.get("item", "")):
                hit = self.find(part)
                self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 200}])
        elif verb in ("set_field", "set", "set_dialog_field"):
            a = arg if isinstance(arg, dict) else act
            target = a.get("target") or (arg if isinstance(arg, str) else None)
            pos = self.resolve(target)
            self.inp([{"type": "click", "pos": pos}, {"type": "key", "key": "A", "modifiers": ["ctrl"]},
                      {"type": "type", "text": str(a.get("value"))}, {"type": "key", "key": "Enter"}])
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
            tab = self.find(info.get("title") or p, None, True)
            tr = tab["rect"]
            cy = (tr[1] + tr[3]) / 2
            btns = [h for h in (self.call("ui_find", {"text": "Bedienfeldmenü", "role": "button", "limit": 50}) or {}).get("hits") or []
                    if abs((h["rect"][1] + h["rect"][3]) / 2 - cy) < 12 and h["rect"][0] > tr[0]]
            if not btns:
                raise BridgeError(f"panel menu button for {p} not found")
            b = min(btns, key=lambda h: h["rect"][0])
            self.inp([{"type": "click", "pos": b["center"]}, {"type": "wait", "ms": 300}])
            item = act.get("item") or (arg.get("item") if isinstance(arg, dict) else None)
            if item:
                parts = split_path(item)
                for i, part in enumerate(parts):
                    hit = self.find(part, None, True, [0, 30, 10000, 10000])
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
                pos = self.doc_to_win(*a["pos"])
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
            return (e.get(self.name) if isinstance(e, dict) else None) or {}
        return target if isinstance(target, dict) else {}

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
        mods = act.get("mods") or act.get("hold_key") or []
        if isinstance(mods, str):
            mods = [mods]
        loc = {} if verb.startswith("canvas_") or verb == "ui_find_click" else \
            self.loc_of(arg.get("target") if isinstance(arg, dict) and "target" in arg else arg)
        if verb == "ui_find_click":
            pos = self.find(arg if isinstance(arg, str) else arg.get("text"))["center"]
            verb = "click"
        elif verb.startswith("canvas_"):
            pos = self.doc_to_win(*(arg if isinstance(arg, list) else arg.get("pos") or arg.get("at")))
            verb = {"canvas_click": "click", "canvas_double_click": "double_click"}.get(verb, "hover")
        else:
            target = arg
            if isinstance(arg, dict) and "target" in arg:
                mods = arg.get("mods") or arg.get("hold_key") or mods
                target = arg["target"]
            pos = self.resolve(target)
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
        if mods:
            for e in ev:
                if e["type"] not in ("move",):
                    e["modifiers"] = [m.lower() for m in mods]
        ev.append({"type": "wait", "ms": 200})
        self.inp(ev)
        if loc.get("select") and verb in ("click", "open"):
            hit = self.find(loc["select"], None, True, [0, 30, 10000, 10000])
            self.inp([{"type": "click", "pos": hit["center"]}, {"type": "wait", "ms": 200}])

    def record_ui(self, key):
        if key.startswith("ui_state"):
            st = self.state()
            sub = key.split(".", 1)[1] if "." in key else "summary"
            return st if sub == "summary" else st.get(sub)
        if key in ("menu.open", "menu.disabled_items"):
            return self.menu_items()
        if key == "document.summary":
            return self.call("get_document", {"depth": 1})
        return self.state()

    def menu_items(self):
        hits = (self.call("ui_find", {"role": "menu_item", "limit": 200}) or {}).get("hits") or []
        return [{"text": h.get("text"), "enabled": h.get("enabled"), "checked": h.get("toggled")} for h in hits]


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
    def testfile(self, code) -> Path:
        tf = self.cfg["testfiles"].get(code, code)
        p = Path(tf)
        return p if p.is_absolute() else Path(self.cfg["testfile_dir"]) / tf

    def open_testfile(self, side: Side, code):
        code = code if isinstance(code, str) and code not in ("", "true") else self.unit.get("testfile")
        p = self.testfile(code)
        side.open_testfile(p)
        self.opened.setdefault(side.name, set()).add(p)

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

    # ---- programs
    def ensure_programs(self):
        if self.orig is None:
            self.orig = Original(self.cfg["orig"], self.tmp, log=self.log)
            self.orig.host.start()
        if self.fp is None:
            self.fp = Firepixel(self.cfg["fp"], log=self.log)
        self.sides = {"orig": OrigSide(self, self.orig), "fp": FpSide(self, self.fp)}

    SYSTEMIC = re.compile(r"main window not found|process not running|JSF timeout|host timeout|host ended|"
                          r"crash|cannot bring|input refused|not started|without main window", re.I)

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
                self.orig.restart()
                self.sides["orig"].prepare()
                if self.unit.get("testfile"):
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
            if self.unit.get("testfile"):
                self.open_testfile(side, self.unit["testfile"])
        except Exception as e:  # noqa: BLE001
            self.log(f"recover {side.name} failed: {e}")

    # ---- steps
    def run_actions(self, side: Side, actions, rec):
        for act in actions or []:
            if isinstance(act, str):
                act = {act: True}
            verb = next((k for k in act if k not in ("mods", "ms", "repeat", "hold_key", "item", "by", "at", "do",
                                                    "region", "to_fraction", "hold", "orig_index", "row", "from", "to",
                                                    "value", "target", "note", "orig_menu")), None)
            if verb is None:
                verb = next(iter(act))
            arg = act[verb]
            if verb == "orig_index":
                continue
            if verb == "reset":
                if arg in (None, False, "", "null"):
                    verb = "close_dialogs"
                else:
                    verb = "open_testfile"
            if verb == "record":
                verb = "log_value"
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
                    rec["_before"] = side.window_image()[0]
                except Exception:  # noqa: BLE001
                    pass
            side.do(verb, arg, act)
            rec["actions"].append({"verb": verb, "ms": int((time.time() - t0) * 1000)})

    def export_path(self, side: Side) -> Path:
        d = self.out / self.unit["unit"] / "files"
        d.mkdir(parents=True, exist_ok=True)
        return d / f"{self.cur_step}_{side.name}.png"

    def step_side(self, side: Side, step, before_img):
        self.cur_step = step["id"]
        rec = {"side": side.name, "actions": [], "status": "ok", "want_before": before_img is not None}
        actions = list(step.get("both") or []) + list(step.get(side.name) or [])
        side.values = []
        t0 = time.time()
        try:
            if side.name == "orig":
                side.o.focus()
            self.run_actions(side, actions, rec)
            if step.get("log_menu_items"):
                side.values.append({"menu_items": side.menu_items()})
            shot = step.get("shot")
            if shot:
                time.sleep(float(shot.get("delay_ms", 300)) / 1000)
                area = shot.get("area", "window")
                rect = side.area_rect(area)
                if rect is None:
                    img, wr = side.window_image()
                    bb = diff_bbox(rec.get("_before") or before_img, img)
                    if not bb:
                        raise RuntimeError(f"area {area}: nothing opened (no difference to the state before)")
                    rect = [wr[0] + bb[0], wr[1] + bb[1], wr[0] + bb[2], wr[1] + bb[3]]
                    rec["area_from"] = "difference"
                    if area == "menu" and side.name == "fp":
                        # like the reference (popup windows only): leave out the highlighted menu bar entry
                        try:
                            bar = side.find(self.ui_map["menu_bar"]["orig"][0][0], None, True, [0, 0, 2000, 40])["rect"]
                            rect[1] = max(rect[1], bar[3] + 2)
                        except Exception:  # noqa: BLE001
                            pass
                rec["rect"] = [round(v) for v in rect]
                out = self.out / self.unit["unit"] / side.name / f"{step['id']}.png"
                side.grab(rect, out)
                rec["shot"] = str(out)
            self.run_actions(side, (step.get("after") or []) + (step.get("cleanup") or []), rec)
        except Unsupported as e:
            rec.update(status="unsupported", error=str(e))
        except Unmapped as e:
            rec.update(status="unmapped", error=str(e))
        except Exception as e:  # noqa: BLE001
            rec.update(status="error", error=f"{type(e).__name__}: {e}", trace=traceback.format_exc(limit=3)[-600:])
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
            self.orig_fails = 0
            for side in self.sides.values():
                try:
                    side.prepare()
                    if side.name == "orig":
                        self.orig.kill_hung_own()
                        # defined start state: no questions open, every document closed without saving
                        side.reset_light()
                        self.orig.close_all_docs()
                    if unit.get("testfile"):
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
                    try:
                        side.reset_light()
                    except Exception as e:  # noqa: BLE001
                        self.recover(side)
                    before = None
                    if name == "orig":
                        side.windows_before = set()
                        if (step.get("shot") or {}).get("area") == "popup":
                            try:
                                side.windows_before = {w["hwnd"] for w in side.h.list("windows")}
                            except Exception:  # noqa: BLE001
                                pass
                    if (step.get("shot") or {}).get("area") in ("menu", "popup"):
                        try:
                            before, _ = side.window_image()
                        except Exception:  # noqa: BLE001
                            before = None
                    rec = self.step_side(side, step, before)
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
    a = ap.parse_args(argv)
    cfg = json.loads(a.config.read_text(encoding="utf-8"))
    if a.gallery_only:
        print(gallery.build(Path(cfg["out_dir"])))
        return 0
    units = [u.strip() for u in (a.units or "").split(",") if u.strip()]
    if a.all:
        units += sorted(p.stem for p in SCRIPTS.glob("*.json") if not p.stem.startswith("_"))
    if not units:
        ap.error("give --units or --all")
    r = Runner(cfg, a.dry_run, set(a.steps.split(",")) if a.steps else None)
    try:
        for u in dict.fromkeys(units):
            try:
                r.run_unit(u)
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
