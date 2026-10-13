"""Reference-program side of the parity runner.

Everything program specific (executable, process name, window classes, lock name) comes from
runner.config.json outside the repository.  This module
  * holds the system-wide lock shared with the other automation tools,
  * starts / restarts the program,
  * runs JSF scripts in the running instance (done marker + log file contract),
  * talks to orig_host.ps1 (UI Automation, SendInput with foreground check, screen capture).
"""

from __future__ import annotations

import ctypes
import json
import os
import queue
import random
import subprocess
import threading
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
kernel32.CreateMutexW.restype = ctypes.c_void_p
kernel32.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_bool, ctypes.c_wchar_p]
kernel32.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
kernel32.ReleaseMutex.argtypes = [ctypes.c_void_p]
kernel32.CloseHandle.argtypes = [ctypes.c_void_p]


class OrigError(RuntimeError):
    pass


class Lock:
    """Named Win32 mutex (same one the PowerShell helpers use)."""

    def __init__(self, name: str):
        self.name = name
        self.h = None

    def acquire(self, timeout_s: float) -> bool:
        self.h = kernel32.CreateMutexW(None, False, self.name)
        r = kernel32.WaitForSingleObject(self.h, int(timeout_s * 1000))
        return r in (0, 0x80)  # WAIT_OBJECT_0 / WAIT_ABANDONED

    def release(self):
        if self.h:
            kernel32.ReleaseMutex(self.h)
            kernel32.CloseHandle(self.h)
            self.h = None


def js_str(s: str) -> str:
    """JavaScript string literal, ASCII only (the script host reads files in the ANSI code page)."""
    return json.dumps(s, ensure_ascii=True)


def file_url(p: Path | str) -> str:
    s = str(p).replace("\\", "/")
    if len(s) > 1 and s[1] == ":":
        s = s[0] + "|" + s[2:]
    return "file:///" + s


class Host:
    """orig_host.ps1 as a long-running child process with per-command timeouts."""

    def __init__(self, init: dict):
        self.init = init
        self.p = None
        self.q: queue.Queue = queue.Queue()
        self.n = 0

    def start(self):
        self.p = subprocess.Popen(
            ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(HERE / "orig_host.ps1")],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            encoding="utf-8", errors="replace", bufsize=1, creationflags=0x08000000)
        q = self.q = queue.Queue()

        def rd(p=self.p):
            for line in p.stdout:
                q.put(line)
            q.put(None)

        threading.Thread(target=rd, daemon=True).start()
        self.call("init", timeout=60, **self.init)

    def stop(self):
        if self.p and self.p.poll() is None:
            try:
                self.p.stdin.write(json.dumps({"id": 0, "cmd": "quit"}) + "\n")
                self.p.stdin.flush()
                self.p.wait(3)
            except Exception:
                self.p.kill()
        self.p = None

    def call(self, cmd: str, timeout: float = 30, **args):
        if self.p is None or self.p.poll() is not None:
            if cmd == "init":
                raise OrigError("host not running")
            self.start()
        self.n += 1
        msg = dict(args, id=self.n, cmd=cmd)
        self.p.stdin.write(json.dumps(msg, ensure_ascii=False) + "\n")
        self.p.stdin.flush()
        end = time.time() + timeout
        while True:
            left = end - time.time()
            if left <= 0:
                self.p.kill()
                self.p = None
                raise OrigError(f"host timeout ({timeout}s) on {cmd}")
            try:
                line = self.q.get(timeout=left)
            except queue.Empty:
                continue
            if line is None:
                self.p = None
                raise OrigError(f"host ended during {cmd}")
            line = line.strip()
            if not line.startswith("{"):
                continue
            r = json.loads(line)
            if r.get("id") != self.n:
                continue
            if not r.get("ok"):
                raise OrigError(f"{cmd}: {r.get('error')}")
            return r.get("result")

    def list(self, cmd: str, timeout: float = 30, **args) -> list:
        """Like call, for commands answering with a list (PowerShell unwraps 1-element arrays)."""
        r = self.call(cmd, timeout, **args)
        return r if isinstance(r, list) else ([] if r is None else [r])


class Original:
    def __init__(self, cfg: dict, tmp_dir: Path, log=print):
        self.cfg = cfg
        self.exe = cfg["exe"]
        self.tmp = Path(tmp_dir)
        self.tmp.mkdir(parents=True, exist_ok=True)
        self.log = log
        self.host = Host({"process": cfg["process"], "main_class": cfg.get("main_class", ""),
                          "view_class": cfg.get("view_class", ""), "panel_class": cfg.get("panel_class", "")})
        self.started_by_us = False

    # ---------------------------------------------------------------- process
    def pid(self) -> int:
        return int(self.host.call("pid") or 0)

    def running(self) -> bool:
        return self.pid() > 0

    def start(self):
        if self.running():
            return
        subprocess.Popen([self.exe], cwd=str(Path(self.exe).parent))
        self.started_by_us = True
        end = time.time() + float(self.cfg.get("start_wait_s", 60))
        while time.time() < end:
            time.sleep(2)
            try:
                m = self.host.call("main")
                if m and (m["rect"][2] - m["rect"][0]) > 900:
                    break
            except OrigError:
                pass
        time.sleep(float(self.cfg.get("settle_s", 5)))
        # recovery / startup questions: decline
        self.host.call("dialog_close", mode="cancel")
        self.place()

    def kill(self):
        pid = self.pid()
        if pid:
            subprocess.run(["taskkill", "/F", "/PID", str(pid)], capture_output=True)
            time.sleep(3)

    def restart(self):
        self.log("restart reference program")
        self.kill()
        self.start()

    def place(self):
        x, y, w, h = self.cfg.get("rect", [0, 0, 1600, 1000])
        return self.host.call("place", x=x, y=y, w=w, h=h)

    def focus(self):
        r = self.host.call("focus")
        if r.get("ok") is False:
            raise OrigError(f"cannot bring reference program to the foreground: {r}")
        return r

    # -------------------------------------------------------------------- JSF
    def jsf(self, code: str, timeout: float = 60) -> list[str]:
        """Run JSF in the running instance; returns the log lines (fp_log / fp_ret)."""
        if not self.running():
            self.start()
        name = f"par_{time.strftime('%H%M%S')}_{random.randrange(100000)}"
        done = self.tmp / f"{name}.done"
        logf = self.tmp / f"{name}.log"
        out = file_url(self.tmp) + "/"
        prolog = (f'var FP_OUT = {js_str(out)};\nvar FP_LOG = [];\n'
                  'function fp_log(t) { FP_LOG.push(String(t)); }\n'
                  'function fp_try(n, f) { try { var r = f(); fp_log("ok " + n + (r === undefined ? "" : " = " + r)); return r; } '
                  'catch (e) { fp_log("ERROR " + n + ": " + e); return null; } }\n'
                  'try {\n')
        epilog = ('\n} catch (e) { fp_log("ERROR: " + e); }\n'
                  '(function () {\n'
                  f'  var p = FP_OUT + {js_str(name + ".log")};\n'
                  '  Files.deleteFileIfExisting(p); Files.createFile(p, ".txt", "TEXT");\n'
                  '  var f = Files.open(p, true); f.write(FP_LOG.join("\\n")); f.close();\n'
                  f'  var d = FP_OUT + {js_str(name + ".done")};\n'
                  '  Files.deleteFileIfExisting(d); Files.createFile(d, ".txt", "TEXT");\n'
                  '})();\n')
        src = self.tmp / f"{name}.jsf"
        src.write_text(prolog + code + epilog, encoding="ascii", errors="backslashreplace")
        subprocess.Popen([self.exe, str(src)], cwd=str(Path(self.exe).parent))
        end = time.time() + timeout
        progress = self.cfg.get("progress_dialog_regex", "Befehlsskript|verarbeitet|Processing|Fortschritt")
        while time.time() < end and not done.exists():
            time.sleep(0.4)
            try:
                self.host.call("dialog_close", mode="cancel", **{"except": progress})
            except OrigError:
                pass
            if not self.running():
                raise OrigError("reference program ended during JSF (crash)")
        finished = done.exists()
        lines = logf.read_text(encoding="utf-8", errors="replace").splitlines() if logf.exists() else []
        for f in (src, done, logf):
            try:
                f.unlink()
            except OSError:
                pass
        if not finished:
            raise OrigError(f"JSF timeout after {timeout}s")
        errs = [ln for ln in lines if ln.startswith("ERROR")]
        if errs:
            raise OrigError("JSF: " + "; ".join(errs)[:500])
        return lines

    # ------------------------------------------------------------- documents
    def open_doc(self, path: Path):
        url = file_url(path)
        # close earlier copies of the same file without saving, then open (never with the 3rd argument)
        self.jsf(f"""
var u = {js_str(url)};
for (var i = fw.documents.length - 1; i >= 0; i--) {{
  var d = fw.documents[i];
  if (String(d.filePathForSave || "").toLowerCase() == u.toLowerCase()) {{ d.close(false); }}
}}
fw.openDocument(u);
fp_log("RET " + fw.documents.length);
""", timeout=90)

    def close_doc(self, path: Path):
        url = file_url(path)
        self.jsf(f"""
var u = {js_str(url)};
for (var i = fw.documents.length - 1; i >= 0; i--) {{
  var d = fw.documents[i];
  if (String(d.filePathForSave || "").toLowerCase() == u.toLowerCase()) {{ d.close(false); }}
}}
""", timeout=60)
