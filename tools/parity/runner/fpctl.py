"""Firepixel side of the parity runner: start a private copy of the newest preview build
with its own profile and talk to its MCP bridge (HTTP, token from mcp-bridge.json)."""

from __future__ import annotations

import ctypes
import ctypes.wintypes as wt
import glob
import json
import os
import shutil
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

user32 = ctypes.WinDLL("user32", use_last_error=True)
WNDENUMPROC = ctypes.WINFUNCTYPE(wt.BOOL, wt.HWND, wt.LPARAM)


class BridgeError(RuntimeError):
    pass


def newest_build(pattern: str) -> Path:
    """Folder matching `pattern` whose firepixel.exe is the newest."""
    best, best_t = None, -1.0
    for d in glob.glob(pattern):
        exe = Path(d) / "firepixel.exe"
        if exe.is_file() and exe.stat().st_mtime > best_t:
            best, best_t = exe, exe.stat().st_mtime
    if best is None:
        raise FileNotFoundError(f"no firepixel.exe under {pattern}")
    return best


def windows_of(pid: int) -> list[int]:
    out: list[int] = []

    def cb(h, _l):
        p = wt.DWORD()
        user32.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and user32.IsWindowVisible(h) and user32.GetWindowTextLengthW(h) > 0:
            out.append(h)
        return True

    user32.EnumWindows(WNDENUMPROC(cb), 0)
    return out


class Firepixel:
    def __init__(self, cfg: dict, log=print):
        self.cfg = cfg
        self.log = log
        self.proc: subprocess.Popen | None = None
        self.work = Path(cfg["work_dir"])
        self.home = self.work / "profile"
        self.exe: Path | None = None
        self.url = self.token = self.sid = None
        self._id = 0

    # ---------------------------------------------------------------- process
    def prepare_exe(self) -> Path:
        src = newest_build(self.cfg["preview_glob"])
        dst_dir = self.work / "app"
        dst = dst_dir / "firepixel.exe"
        dst_dir.mkdir(parents=True, exist_ok=True)
        stamp = dst_dir / "source.txt"
        tag = f"{src}|{src.stat().st_mtime}"
        if not dst.exists() or not stamp.exists() or stamp.read_text(encoding="utf-8") != tag:
            shutil.copy2(src, dst)
            stamp.write_text(tag, encoding="utf-8")
        self.exe = dst
        return dst

    def _profile(self):
        """A fresh profile for every start (= every unit): default panel layout, no floating panels left over from
        earlier units, theme and language from the config."""
        if self.home.exists() and self.cfg.get("fresh_profile", True):
            shutil.rmtree(self.home, ignore_errors=True)
        roaming = self.home / "Roaming" / "Firepixel"
        local = self.home / "Local" / "Firepixel"
        roaming.mkdir(parents=True, exist_ok=True)
        local.mkdir(parents=True, exist_ok=True)
        pf = roaming / "preferences.json"
        try:
            prefs = json.loads(pf.read_text(encoding="utf-8"))
        except Exception:
            prefs = {"version": 1, "theme": "normal", "recent_files": [], "values": {}}
        prefs["theme"] = self.cfg.get("theme", "normal")
        prefs.setdefault("values", {})["Language"] = self.cfg.get("language", "de")
        # TW-122 (re 46 §1): the reference machine runs the program's own shortcut set, not the fresh-install
        # default "Web Standard" (there Ctrl+Shift+B is Break Apart, which warned in run 16 xs-shortcuts/06)
        # R-13b (RE-14): the reference machine uses the bundled "Web Standard" set (pref CurrentShortcutSet, empty
        # user set folder) - default changed from "Firepixel"; config `fp.shortcut_set` overrides
        prefs["values"]["CurrentShortcutSet"] = self.cfg.get("shortcut_set", "Web Standard")
        pf.write_text(json.dumps(prefs, indent=2), encoding="utf-8")
        bridge = local / "mcp-bridge.json"
        if bridge.exists():
            bridge.unlink()
        return bridge

    def alive(self) -> bool:
        return self.proc is not None and self.proc.poll() is None

    def start(self, timeout: float = 40.0):
        if self.alive():
            return
        exe = self.prepare_exe()
        bridge = self._profile()
        env = dict(os.environ)
        env["APPDATA"] = str(self.home / "Roaming")
        env["LOCALAPPDATA"] = str(self.home / "Local")
        env["FP_NO_SHELL_OPEN"] = "1"
        # SW_SHOWNOACTIVATE: visible, but does not take the foreground
        si = subprocess.STARTUPINFO()
        si.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        si.wShowWindow = 4
        self.proc = subprocess.Popen([str(exe), "--mcp"], env=env, cwd=str(exe.parent), startupinfo=si)
        end = time.time() + timeout
        while time.time() < end and not bridge.exists():
            if self.proc.poll() is not None:
                raise BridgeError(f"firepixel exited at start (code {self.proc.returncode})")
            time.sleep(0.25)
        if not bridge.exists():
            raise BridgeError("bridge file did not appear")
        time.sleep(0.8)
        info = json.loads(bridge.read_text(encoding="utf-8"))
        self.url, self.token = info["url"], info["token"]
        self.initialize()
        self.place()
        self.view_tooltips_reset()

    # View switches set after every start: (command, menu path for the log, config key, default).
    # R-28 (TW-190): the reference's View > Tooltips is off during the runs, so no coordinate tag at the
    # pointer; since FB-41 Firepixel has the tag on its own switch View > Coordinate Tag (default on) and
    # View > Tooltips switches the widget tooltips (FB-37), which the reference shows regardless.
    VIEW_SWITCHES = (
        ("view.coordinateTag", "View > Coordinate Tag", "coordinate_tag", False),
        ("view.tooltips", "View > Tooltips", "view_tooltips", True),
        # FB-59 to FB-61: Firepixel's expandable group and mask rows in the Layers panel (default on) are off
        # during the runs, so the panel lists the rows of the reference (config `fp.layers_group_tree`)
        ("panel.layers.groupTree", "Layers options > Show Group Contents", "layers_group_tree", False),
    )

    def view_switch_state(self, cmd: str) -> bool | None:
        """Check state of a View switch (`describe_command` -> `checked`)."""
        val, _ = self.call("describe_command", {"name": cmd}, timeout=20)
        v = val.get("checked") if isinstance(val, dict) else None
        return None if v is None else bool(v)

    def view_tooltips_reset(self):
        """R-28 (TW-190): after every start (unit start, restart after a crash) each switch of `VIEW_SWITCHES` is set
        to its config value with its command when the read-back state differs, then read back again. Not restored:
        the instance and its profile are private to the run."""
        for cmd, label, key, default in self.VIEW_SWITCHES:
            want = bool(self.cfg.get(key, default))
            try:
                before = self.view_switch_state(cmd)
                if before is None:
                    self.log(f"fp {label}: state not readable (describe_command gave no `checked`) - left as is")
                    continue
                if before == want:
                    self.log(f"fp {label}: {'on' if before else 'off'} already (wanted {'on' if want else 'off'})")
                    continue
                self.call("run_command", {"name": cmd}, timeout=20)
                after = self.view_switch_state(cmd)
                self.log(f"fp {label}: switched {'on' if before else 'off'} -> {'on' if after else 'off'} "
                         f"({cmd}, wanted {'on' if want else 'off'})"
                         + ("" if after == want else " - NOT SWITCHED"))
            except (BridgeError, OSError, ValueError) as e:
                self.log(f"fp {label}: {e}")

    def hwnd(self) -> int | None:
        if not self.alive():
            return None
        hs = windows_of(self.proc.pid)
        return hs[0] if hs else None

    def place(self):
        x, y, w, h = self.cfg.get("rect", [960, 400, 1600, 1000])
        rc = wt.RECT()
        for _ in range(40):
            hw = self.hwnd()
            if hw:
                if user32.IsZoomed(hw):
                    user32.ShowWindow(hw, 4)  # SW_SHOWNOACTIVATE: restore without activating
                # SWP_NOACTIVATE, bottom of the z-order (HWND_BOTTOM=1)
                user32.SetWindowPos(hw, 1, x, y, w, h, 0x0010)
                time.sleep(0.4)
                user32.GetWindowRect(hw, ctypes.byref(rc))
                if not user32.IsZoomed(hw) and rc.right - rc.left == w and rc.bottom - rc.top == h:
                    return
            time.sleep(0.25)
        self.log("firepixel window could not be placed")

    def resize(self, w=None, h=None):
        """Window size at the configured position (None = configured size): SWP_NOACTIVATE, z-order kept."""
        x, y, cw, ch = self.cfg.get("rect", [960, 400, 1600, 1000])
        hw = self.hwnd()
        if hw:
            user32.SetWindowPos(hw, 0, x, y, w or cw, h or ch, 0x0010 | 0x0004)

    def stop(self):
        if self.proc is None:
            return
        if self.proc.poll() is None:
            self.proc.kill()
            try:
                self.proc.wait(10)
            except Exception:
                pass
        self.proc = None
        self.sid = None

    # ----------------------------------------------------------------- bridge
    def _post(self, body: dict, timeout: float):
        data = json.dumps(body).encode("utf-8")
        hdr = {"Authorization": f"Bearer {self.token}", "Accept": "application/json, text/event-stream",
               "Content-Type": "application/json; charset=utf-8"}
        if self.sid:
            hdr["Mcp-Session-Id"] = self.sid
        req = urllib.request.Request(self.url, data=data, headers=hdr, method="POST")
        with urllib.request.urlopen(req, timeout=timeout) as r:
            sid = r.headers.get("Mcp-Session-Id")
            raw = r.read().decode("utf-8", "replace")
        return sid, raw

    def initialize(self):
        self.sid = None
        sid, _ = self._post({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                             "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                                        "clientInfo": {"name": "parity-runner", "version": "1"}}}, 20)
        self.sid = sid
        try:
            self._post({"jsonrpc": "2.0", "method": "notifications/initialized"}, 10)
        except Exception:
            pass

    def call(self, tool: str, args: dict | None = None, timeout: float = 60.0):
        """Call a bridge tool. Returns (parsed_json_or_text, images[list of bytes])."""
        self._id += 1
        body = {"jsonrpc": "2.0", "id": self._id, "method": "tools/call",
                "params": {"name": tool, "arguments": args or {}}}
        try:
            _, raw = self._post(body, timeout)
        except urllib.error.HTTPError:
            self.initialize()
            _, raw = self._post(body, timeout)
        lines = [ln[5:] for ln in raw.splitlines() if ln.startswith("data:")]
        msg = json.loads(lines[-1] if lines else raw)
        if msg.get("error"):
            raise BridgeError(f"{tool}: {json.dumps(msg['error'], ensure_ascii=False)[:400]}")
        res = msg.get("result", {})
        texts, images = [], []
        import base64
        for c in res.get("content", []):
            if c.get("type") == "text":
                texts.append(c["text"])
            elif c.get("type") == "image":
                images.append(base64.b64decode(c["data"]))
        txt = "\n".join(texts)
        if res.get("isError"):
            raise BridgeError(f"{tool}: {txt[:400]}")
        try:
            val = json.loads(txt)
        except Exception:
            val = txt
        return val, images
