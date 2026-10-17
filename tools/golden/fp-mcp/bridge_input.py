# End-to-end test of the input injection (ui_input/ui_find) against a running Firepixel UI.
# Usage (under the GUI lock, see bridge_input.ps1):
#   python -I bridge_input.py <output folder>
# Reads address/token from %LOCALAPPDATA%\Firepixel\mcp-bridge.json, speaks Streamable HTTP directly.
# The ui_find search texts ("Datei", "Titelleiste", "Abbrechen", "Öffnen") are labels of the German UI
# and must stay German.
import base64, http.client, json, os, sys, time

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
tok = json.load(open(os.path.join(os.environ["LOCALAPPDATA"], "Firepixel", "mcp-bridge.json"), encoding="utf-8-sig"))
port, token = tok["port"], tok["token"]
session = None
results = []


def post(msg):
    global session
    c = http.client.HTTPConnection("127.0.0.1", port, timeout=120)
    h = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream", "Authorization": "Bearer " + token}
    if session:
        h["Mcp-Session-Id"] = session
        h["Mcp-Protocol-Version"] = "2025-06-18"
    c.request("POST", "/mcp", json.dumps(msg), h)
    r = c.getresponse()
    if r.getheader("Mcp-Session-Id"):
        session = r.getheader("Mcp-Session-Id")
    body = r.read().decode("utf-8")
    c.close()
    if "id" not in msg:
        return None
    if r.getheader("Content-Type", "").startswith("text/event-stream"):
        for line in body.splitlines():
            if line.startswith("data:"):
                d = line[5:].strip()
                if d:
                    m = json.loads(d)
                    if m.get("id") == msg["id"]:
                        return m
        raise RuntimeError("no response in the SSE stream: " + body[:300])
    return json.loads(body)


nid = [10]


def call(name, args, image=None):
    nid[0] += 1
    m = post({"jsonrpc": "2.0", "id": nid[0], "method": "tools/call", "params": {"name": name, "arguments": args}})
    if "error" in m:
        raise RuntimeError(f"{name}: {m['error']}")
    res = m["result"]
    k = 0
    for c in res.get("content", []):
        if c.get("type") == "image" and image:
            p = os.path.join(out, f"{image}_{k}.png")
            open(p, "wb").write(base64.b64decode(c["data"]))
            k += 1
    if res.get("isError"):
        raise RuntimeError(f"{name}: " + " ".join(c.get("text", "") for c in res.get("content", [])))
    return res.get("structuredContent")


def check(name, ok, info=""):
    results.append({"test": name, "ok": bool(ok), "info": info})
    print(("OK    " if ok else "FAILED") + " " + name + ("  - " + str(info)[:300] if info else ""))


def count():
    return int(float(call("run_script", {"code": "var d=fw.getDocumentDOM(); var n=0; for (var i=0;i<d.layers.length;i++){ if (d.layers[i].layerType!='web') n+=d.layers[i].frames[0].elements.length;} n"})["script"]["result"]))


def state():
    s = call("ui_state", {})
    d = [x for x in s["documents"] if x["active"]][0]
    return s, d


def screen(d, x, y):
    l, t, r, b = d["page_on_screen"]
    return [l + (r - l) * x / 500.0, t + (b - t) * y / 300.0]


post({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "e2e-input", "version": "1"}}})
post({"jsonrpc": "2.0", "method": "notifications/initialized"})

call("new_document", {"width": 500, "height": 300, "background": "#ffffff"})
call("run_command", {"name": "draw.rectangle", "params": {"rect": [150, 100, 350, 200], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Object"}})
call("run_command", {"name": "select.none", "params": {}})
call("ui_action", {"fit": True, "tool": "tool.pointer"})
time.sleep(0.5)
s, d = state()
check("ui_state reports the page position", d.get("page_on_screen") is not None, d.get("page_on_screen"))
center = screen(d, 250, 150)

# 1) ui_find: menu "Datei" (File)
f = call("ui_find", {"text": "Datei", "exact": True})
check("ui_find menu File", f["count"] >= 1 and f["hits"][0]["center"][1] < 60, f["hits"][:1])
file_menu = f["hits"][0]["center"] if f["count"] else [20, 10]

# 2) Ctrl+C / Ctrl+V on the object
n0 = count()
r = call("ui_input", {"events": [
    {"type": "click", "pos": center},
    {"type": "key", "key": "C", "modifiers": ["ctrl"]},
    {"type": "key", "key": "V", "modifiers": ["ctrl"]},
], "screenshot": True}, image="01_ctrl_c_v")
n1 = count()
check("Ctrl+C/Ctrl+V duplicates the object", n1 == n0 + 1, f"{n0} → {n1}; steps {r['steps']}")
check("no stuck modifier keys", r["held_modifiers"] == [], r["held_modifiers"])
call("ui_input", {"events": [{"type": "key", "key": "Z", "modifiers": ["ctrl"]}]})
check("Ctrl+Z undoes the paste", count() == n0, count())
call("ui_input", {"events": [{"type": "key", "key": "Y", "modifiers": ["ctrl"]}]})
check("Ctrl+Y redoes it", count() == n1, count())

# 3) Ctrl+mouse wheel zooms around the pointer position
z0 = state()[1]["zoom"]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, 1], "pos": center, "modifiers": ["ctrl"]}], "screenshot": True}, image="02_ctrl_wheel")
z1 = state()[1]["zoom"]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, -1], "pos": center, "modifiers": ["ctrl"]}]})
z2 = state()[1]["zoom"]
# zoom levels snap to fixed values like the original (e.g. 370 % fitted → 400 % → 300 %)
check("Ctrl+mouse wheel zooms in and out", z1 > z0 and z2 < z1, f"{z0} → {z1} → {z2}")
call("ui_action", {"zoom": 8})  # page larger than the window so there is something to scroll
time.sleep(0.3)
d0 = state()[1]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, -3], "pos": center}]})
d1 = state()[1]
check("mouse wheel without Ctrl scrolls instead of zooming", abs(d1["zoom"] - d0["zoom"]) < 1e-6 and d1["origin"] != d0["origin"], f"origin {d0['origin']} → {d1['origin']}")
call("ui_action", {"fit": True})
time.sleep(0.3)
center = screen(state()[1], 250, 150)

# 4) drag a dialog by its title bar ("Titelleiste" is the German accessibility label)
call("ui_action", {"run": "modify.canvas.imageSize"})
time.sleep(0.3)
t0 = call("ui_find", {"text": "Titelleiste"})
check("dialog title bar found", t0["count"] >= 1, t0["hits"][:1])
if t0["count"]:
    rc = t0["hits"][0]["rect"]
    start = [rc[0] + 40, (rc[1] + rc[3]) / 2]
    call("ui_input", {"events": [{"type": "drag", "from": start, "to": [start[0] - 160, start[1] + 90], "steps": 10, "ms": 16}], "screenshot": True}, image="03_dialog_dragged")
    t1 = call("ui_find", {"text": "Titelleiste"})
    rc1 = t1["hits"][0]["rect"]
    dx, dy = rc1[0] - rc[0], rc1[1] - rc[1]
    check("dialog follows the drag (-160, +90)", abs(dx + 160) < 3 and abs(dy - 90) < 3, f"dx={dx:.1f} dy={dy:.1f}")
    cancel = call("ui_find", {"text": "Abbrechen", "role": "button"})
    if cancel["count"]:
        call("ui_input", {"events": [{"type": "click", "pos": cancel["hits"][0]["center"]}]})
    else:
        call("ui_input", {"events": [{"type": "key", "key": "Escape"}]})
    check("dialog closed", not call("ui_state", {})["dialog_open"], "via Cancel" if cancel["count"] else "via Escape")

# 5) open a menu by click, find an entry, close it
call("ui_input", {"events": [{"type": "click", "pos": file_menu}], "screenshot": True}, image="04_menu_file")
m = call("ui_find", {"text": "Öffnen"})
check("menu File open, entry Open found", m["count"] >= 1, m["hits"][:1])
call("ui_input", {"events": [{"type": "key", "key": "Escape"}, {"type": "pointer_gone"}]})

# 6) type text with the text tool
call("ui_action", {"tool": "tool.text"})
left = screen(state()[1], 30, 40)
n2 = count()
r = call("ui_input", {"events": [{"type": "click", "pos": left}, {"type": "type", "text": "Hello world"}, {"type": "key", "key": "Escape"}], "screenshot": True}, image="05_text")
txt = call("run_script", {"code": "var d=fw.getDocumentDOM(); var t=[]; for (var i=0;i<d.layers.length;i++){ var e=d.layers[i].frames[0].elements; for (var k=0;k<e.length;k++) if (e[k].textChars) t.push(e[k].textChars);} t.join('|')"})["script"]["result"]
check("typed text ends up in the document", "Hello world" in str(txt), txt)
call("ui_action", {"tool": "tool.pointer"})

# 7) drag a file onto the canvas and drop it
source = os.path.join(out, "drop_source.png")
call("export", {"path": source})
n3 = count()
target = screen(state()[1], 400, 230)
call("ui_input", {"events": [{"type": "drop_files", "paths": [source], "pos": target}, {"type": "wait", "ms": 400}, {"type": "frames", "n": 3}], "screenshot": True}, image="06_file_dropped")
n4 = count()
check("dropped file is imported as an object", n4 == n3 + 1, f"{n3} → {n4}")

# 8) double click + screenshot after every step
r = call("ui_input", {"events": [{"type": "move", "pos": center}, {"type": "double_click", "pos": center}, {"type": "key", "key": "Escape"}], "screenshot_each": True}, image="07_each_step")
check("screenshot_each returns one image per step", r["images"] == 3, r["images"])

json.dump(results, open(os.path.join(out, "result.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
failed = [e for e in results if not e["ok"]]
print(f"{len(results) - len(failed)}/{len(results)} checks passed")
sys.exit(1 if failed else 0)
