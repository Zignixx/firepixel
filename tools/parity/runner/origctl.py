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


class CrashError(OrigError):
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
        r = r if isinstance(r, list) else ([] if r is None else [r])
        return [x for x in r if x != {}]  # an empty PowerShell array arrives as {}


class Original:
    def __init__(self, cfg: dict, tmp_dir: Path, log=print):
        self.cfg = cfg
        self.exe = cfg["exe"]
        self.tmp = Path(tmp_dir)
        self.tmp.mkdir(parents=True, exist_ok=True)
        self.log = log
        self.host = Host({"process": cfg["process"], "main_class": cfg.get("main_class", ""),
                          "view_class": cfg.get("view_class", ""), "panel_class": cfg.get("panel_class", ""),
                          # menu entries that would end the program are never executed
                          "forbidden_menu": cfg.get("forbidden_menu", ["Datei > Beenden"])})
        # decided by the first start(): True only if no instance was running when the run began (a later
        # restart after a crash does not change it)
        self.started_by_us = None
        self.own_pids: set[int] = set()  # processes this runner started (instances and script launchers)

    # ---------------------------------------------------------------- process
    def pid(self) -> int:
        """Process owning the visible main window (searched by class across all processes), else 0."""
        return int(self.host.call("main_pid") or 0)

    def running(self) -> bool:
        return self.pid() > 0

    def procs(self) -> list[dict]:
        return [p for p in self.host.list("procs", timeout=60) if isinstance(p, dict) and p.get("pid")]

    def _own(self, p: dict) -> bool:
        return p["pid"] in self.own_pids or str(self.tmp).lower() in str(p.get("cmdline") or "").lower()

    def kill_hung_own(self) -> list[int]:
        """End processes of this runner without any window (script launchers waiting for a blocked instance,
        restarts that never showed a window). Foreign processes are never touched."""
        killed = []
        for p in self.procs():
            if not p.get("has_main") and not p.get("windows") and self._own(p):
                subprocess.run(["taskkill", "/F", "/PID", str(p["pid"])], capture_output=True)
                killed.append(p["pid"])
        if killed:
            self.log(f"ended hung own processes without window: {killed}")
            time.sleep(1.5)
        return killed

    def decline_dialogs(self) -> int:
        try:
            return int(self.host.call("dialog_close", mode="cancel") or 0)
        except OrigError:
            return 0

    def start(self):
        if self.running():
            if self.started_by_us is None:
                self.started_by_us = False
            self.decline_dialogs()
            return
        self.kill_hung_own()
        others = [p for p in self.procs() if not p.get("has_main")]
        if others:
            # a foreign process exists (maybe still starting): wait for its main window, never kill it
            end = time.time() + float(self.cfg.get("start_wait_s", 60))
            while time.time() < end and not self.running():
                self.decline_dialogs()
                time.sleep(2)
            if not self.running():
                raise OrigError(f"reference process without main window that this runner did not start: "
                                f"{[p['pid'] for p in others]}")
            self.decline_dialogs()
            self.place()
            return
        pr = subprocess.Popen([self.exe], cwd=str(Path(self.exe).parent))
        self.own_pids.add(pr.pid)
        if self.started_by_us is None:
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
        self.kill_hung_own()

    def restart(self):
        """Clean restart: decline questions, close all documents unsaved, end the instance, start again."""
        self.log("restart reference program")
        self.decline_dialogs()
        try:
            if self.running():
                self.close_all_docs(timeout=20)
        except OrigError:
            pass
        # a restart does not change who owns the instance: one that was running before the run stays open at the end
        owned = self.started_by_us
        self.kill()
        self.start()
        self.started_by_us = owned

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
                  # a script may be sent twice (lost hand-over): the second copy does nothing
                  f'var FP_STARTED = FP_OUT + {js_str(name + ".started")};\n'
                  'var FP_DUP = Files.exists(FP_STARTED);\n'
                  'if (!FP_DUP) { Files.createFile(FP_STARTED, ".txt", "TEXT"); }\n'
                  'try { if (FP_DUP) { throw "duplicate"; }\n')
        epilog = ('\n} catch (e) { fp_log("ERROR: " + e); }\n'
                  'if (!FP_DUP) (function () {\n'
                  f'  var p = FP_OUT + {js_str(name + ".log")};\n'
                  '  Files.deleteFileIfExisting(p); Files.createFile(p, ".txt", "TEXT");\n'
                  '  var f = Files.open(p, true); f.write(FP_LOG.join("\\n")); f.close();\n'
                  f'  var d = FP_OUT + {js_str(name + ".done")};\n'
                  '  Files.deleteFileIfExisting(d); Files.createFile(d, ".txt", "TEXT");\n'
                  '})();\n')
        src = self.tmp / f"{name}.jsf"
        src.write_text(prolog + code + epilog, encoding="ascii", errors="backslashreplace")
        started = self.tmp / f"{name}.started"
        launcher = subprocess.Popen([self.exe, str(src)], cwd=str(Path(self.exe).parent))
        self.own_pids.add(launcher.pid)
        end = time.time() + timeout
        progress = self.cfg.get("progress_dialog_regex", "Befehlsskript|verarbeitet|Processing|Fortschritt")
        gone_since, resends = None, 0
        while time.time() < end and not done.exists():
            time.sleep(0.4)
            # the launcher hands the script over and ends; if it ended but the script never started
            # (instance was busy finishing an older script), hand it over again
            if launcher.poll() is not None and not started.exists():
                gone_since = gone_since or time.time()
                if time.time() - gone_since > 10 and resends < 2:
                    resends += 1
                    self.log(f"script {name} not started - handing it over again ({resends})")
                    launcher = subprocess.Popen([self.exe, str(src)], cwd=str(Path(self.exe).parent))
                    self.own_pids.add(launcher.pid)
                    gone_since = None
            try:
                self.host.call("dialog_close", mode="cancel", **{"except": progress})
            except OrigError:
                pass
            if not self.running():
                time.sleep(1.0)  # the main window may be re-created; check twice
                if not self.running():
                    self._end_launcher(launcher)
                    raise OrigError("reference program ended during JSF (crash)")
        finished = done.exists()
        if not finished:
            self._end_launcher(launcher)
        lines = logf.read_text(encoding="utf-8", errors="replace").splitlines() if logf.exists() else []
        for f in ((done, logf, started) if resends else (src, done, logf, started)):
            try:
                f.unlink()
            except OSError:
                pass
        if not finished:
            raise OrigError(f"JSF timeout after {timeout}s")
        errs = [ln for ln in lines if ln.startswith("ERROR")]
        if errs:
            msg = "JSF: " + "; ".join(errs)[:500]
            if "-1073741819" in msg:  # access violation inside the program: it is damaged until restarted
                raise CrashError(msg)
            raise OrigError(msg)
        return lines

    def _end_launcher(self, launcher: subprocess.Popen):
        """A script launcher still waiting for the instance (instance blocked) is ended - unless it became the
        instance itself (no instance was running) and owns the main window."""
        try:
            if launcher.poll() is None and launcher.pid != self.pid():
                launcher.kill()
                self.log(f"ended waiting script launcher {launcher.pid}")
        except (OSError, OrigError):
            pass

    def close_all_docs(self, timeout: float = 60):
        """Defined start state: every open document closed without saving (questions are declined)."""
        self.decline_dialogs()
        self.jsf("for (var k = 0; k < 40 && fw.documents.length > 0; k++) {"
                 " fw.documents[fw.documents.length - 1].close(false); }\n"
                 'fp_log("RET " + fw.documents.length);', timeout=timeout)

    def jsf_retry(self, code: str, timeout: float = 60) -> list[str]:
        """jsf(); after an internal access violation restart the program once and retry."""
        try:
            return self.jsf(code, timeout)
        except CrashError:
            self.restart()
            return self.jsf(code, timeout)

    # ------------------------------------------------------------- documents
    @staticmethod
    def _close_code(url: str) -> str:
        return f"""
var u = {js_str(url)};
for (var i = fw.documents.length - 1; i >= 0; i--) {{
  var d = fw.documents[i];
  if (String(d.filePathForSave || "").toLowerCase() == u.toLowerCase()) {{ d.close(false); }}
}}
"""

    def open_doc(self, path: Path):
        """Close earlier copies of the file unsaved, then open it (never with the third argument)."""
        url = file_url(path)
        self.jsf_retry(self._close_code(url) + f"fw.openDocument({js_str(url)});\n", timeout=90)
        time.sleep(1.0)

    def close_doc(self, path: Path):
        self.jsf_retry(self._close_code(file_url(path)), timeout=60)