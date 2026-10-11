# Ende-zu-Ende-Test der Eingabe-Einspeisung (ui_input/ui_find) gegen eine laufende Firepixel-Oberfläche.
# Aufruf (unter der GUI-Sperre, siehe bruecke_eingabe.ps1):
#   python -I bruecke_eingabe.py <Ausgabeordner>
# Liest Adresse/Token aus %LOCALAPPDATA%\Firepixel\mcp-bridge.json, spricht Streamable HTTP direkt.
import base64, http.client, json, os, sys, time

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
tok = json.load(open(os.path.join(os.environ["LOCALAPPDATA"], "Firepixel", "mcp-bridge.json"), encoding="utf-8-sig"))
port, token = tok["port"], tok["token"]
session = None
ergebnisse = []


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
        raise RuntimeError("keine Antwort im SSE-Strom: " + body[:300])
    return json.loads(body)


nid = [10]


def call(name, args, bild=None):
    nid[0] += 1
    m = post({"jsonrpc": "2.0", "id": nid[0], "method": "tools/call", "params": {"name": name, "arguments": args}})
    if "error" in m:
        raise RuntimeError(f"{name}: {m['error']}")
    res = m["result"]
    k = 0
    for c in res.get("content", []):
        if c.get("type") == "image" and bild:
            p = os.path.join(out, f"{bild}_{k}.png")
            open(p, "wb").write(base64.b64decode(c["data"]))
            k += 1
    if res.get("isError"):
        raise RuntimeError(f"{name}: " + " ".join(c.get("text", "") for c in res.get("content", [])))
    return res.get("structuredContent")


def pruefe(name, ok, info=""):
    ergebnisse.append({"test": name, "ok": bool(ok), "info": info})
    print(("OK    " if ok else "FEHLER") + " " + name + ("  – " + str(info)[:300] if info else ""))


def anzahl():
    return int(float(call("run_script", {"code": "var d=fw.getDocumentDOM(); var n=0; for (var i=0;i<d.layers.length;i++){ if (d.layers[i].layerType!='web') n+=d.layers[i].frames[0].elements.length;} n"})["script"]["result"]))


def zustand():
    s = call("ui_state", {})
    d = [x for x in s["documents"] if x["active"]][0]
    return s, d


def bildschirm(d, x, y):
    l, t, r, b = d["page_on_screen"]
    return [l + (r - l) * x / 500.0, t + (b - t) * y / 300.0]


post({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "e2e-eingabe", "version": "1"}}})
post({"jsonrpc": "2.0", "method": "notifications/initialized"})

call("new_document", {"width": 500, "height": 300, "background": "#ffffff"})
call("run_command", {"name": "draw.rectangle", "params": {"rect": [150, 100, 350, 200], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Objekt"}})
call("run_command", {"name": "select.none", "params": {}})
call("ui_action", {"fit": True, "tool": "tool.pointer"})
time.sleep(0.5)
s, d = zustand()
pruefe("ui_state liefert Seitenlage", d.get("page_on_screen") is not None, d.get("page_on_screen"))
mitte = bildschirm(d, 250, 150)

# 1) ui_find: Menü „Datei“
f = call("ui_find", {"text": "Datei", "exact": True})
pruefe("ui_find Menü Datei", f["count"] >= 1 and f["hits"][0]["center"][1] < 60, f["hits"][:1])
datei = f["hits"][0]["center"] if f["count"] else [20, 10]

# 2) Strg+C / Strg+V am Objekt
n0 = anzahl()
r = call("ui_input", {"events": [
    {"type": "click", "pos": mitte},
    {"type": "key", "key": "C", "modifiers": ["ctrl"]},
    {"type": "key", "key": "V", "modifiers": ["ctrl"]},
], "screenshot": True}, bild="01_strg_c_v")
n1 = anzahl()
pruefe("Strg+C/Strg+V verdoppelt das Objekt", n1 == n0 + 1, f"{n0} → {n1}; Schritte {r['steps']}")
pruefe("keine hängenden Umschalttasten", r["held_modifiers"] == [], r["held_modifiers"])
call("ui_input", {"events": [{"type": "key", "key": "Z", "modifiers": ["ctrl"]}]})
pruefe("Strg+Z nimmt das Einfügen zurück", anzahl() == n0, anzahl())
call("ui_input", {"events": [{"type": "key", "key": "Y", "modifiers": ["ctrl"]}]})
pruefe("Strg+Y stellt es wieder her", anzahl() == n1, anzahl())

# 3) Strg+Mausrad zoomt um die Zeigerposition
z0 = zustand()[1]["zoom"]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, 1], "pos": mitte, "modifiers": ["ctrl"]}], "screenshot": True}, bild="02_strg_rad")
z1 = zustand()[1]["zoom"]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, -1], "pos": mitte, "modifiers": ["ctrl"]}]})
z2 = zustand()[1]["zoom"]
# Zoomstufen rasten wie im Original auf feste Werte ein (z. B. 370 % eingepasst → 400 % → 300 %)
pruefe("Strg+Mausrad vergrößert und verkleinert", z1 > z0 and z2 < z1, f"{z0} → {z1} → {z2}")
call("ui_action", {"zoom": 8})  # Seite größer als das Fenster, damit es etwas zu rollen gibt
time.sleep(0.3)
d0 = zustand()[1]
call("ui_input", {"events": [{"type": "wheel", "delta": [0, -3], "pos": mitte}]})
d1 = zustand()[1]
pruefe("Mausrad ohne Strg rollt statt zu zoomen", abs(d1["zoom"] - d0["zoom"]) < 1e-6 and d1["origin"] != d0["origin"], f"Ursprung {d0['origin']} → {d1['origin']}")
call("ui_action", {"fit": True})
time.sleep(0.3)
mitte = bildschirm(zustand()[1], 250, 150)

# 4) Dialog an der Titelleiste ziehen
call("ui_action", {"run": "modify.canvas.imageSize"})
time.sleep(0.3)
t0 = call("ui_find", {"text": "Titelleiste"})
pruefe("Dialog-Titelleiste gefunden", t0["count"] >= 1, t0["hits"][:1])
if t0["count"]:
    rc = t0["hits"][0]["rect"]
    von = [rc[0] + 40, (rc[1] + rc[3]) / 2]
    call("ui_input", {"events": [{"type": "drag", "from": von, "to": [von[0] - 160, von[1] + 90], "steps": 10, "ms": 16}], "screenshot": True}, bild="03_dialog_gezogen")
    t1 = call("ui_find", {"text": "Titelleiste"})
    rc1 = t1["hits"][0]["rect"]
    dx, dy = rc1[0] - rc[0], rc1[1] - rc[1]
    pruefe("Dialog folgt dem Ziehen (−160, +90)", abs(dx + 160) < 3 and abs(dy - 90) < 3, f"dx={dx:.1f} dy={dy:.1f}")
    ab = call("ui_find", {"text": "Abbrechen", "role": "button"})
    if ab["count"]:
        call("ui_input", {"events": [{"type": "click", "pos": ab["hits"][0]["center"]}]})
    else:
        call("ui_input", {"events": [{"type": "key", "key": "Escape"}]})
    pruefe("Dialog geschlossen", not call("ui_state", {})["dialog_open"], "über Abbrechen" if ab["count"] else "über Escape")

# 5) Menü öffnen per Klick, Eintrag finden, schließen
call("ui_input", {"events": [{"type": "click", "pos": datei}], "screenshot": True}, bild="04_menue_datei")
m = call("ui_find", {"text": "Öffnen"})
pruefe("Menü Datei offen, Eintrag Öffnen gefunden", m["count"] >= 1, m["hits"][:1])
call("ui_input", {"events": [{"type": "key", "key": "Escape"}, {"type": "pointer_gone"}]})

# 6) Text tippen mit dem Textwerkzeug
call("ui_action", {"tool": "tool.text"})
links = bildschirm(zustand()[1], 30, 40)
n2 = anzahl()
r = call("ui_input", {"events": [{"type": "click", "pos": links}, {"type": "type", "text": "Hallo Welt"}, {"type": "key", "key": "Escape"}], "screenshot": True}, bild="05_text")
txt = call("run_script", {"code": "var d=fw.getDocumentDOM(); var t=[]; for (var i=0;i<d.layers.length;i++){ var e=d.layers[i].frames[0].elements; for (var k=0;k<e.length;k++) if (e[k].textChars) t.push(e[k].textChars);} t.join('|')"})["script"]["result"]
pruefe("Getippter Text landet im Dokument", "Hallo Welt" in str(txt), txt)
call("ui_action", {"tool": "tool.pointer"})

# 7) Datei auf die Leinwand ziehen und ablegen
quelle = os.path.join(out, "ablage_quelle.png")
call("export", {"path": quelle})
n3 = anzahl()
ziel = bildschirm(zustand()[1], 400, 230)
call("ui_input", {"events": [{"type": "drop_files", "paths": [quelle], "pos": ziel}, {"type": "wait", "ms": 400}, {"type": "frames", "n": 3}], "screenshot": True}, bild="06_datei_abgelegt")
n4 = anzahl()
pruefe("Abgelegte Datei wird als Objekt importiert", n4 == n3 + 1, f"{n3} → {n4}")

# 8) Doppelklick + Bildschirmfoto nach jedem Schritt
r = call("ui_input", {"events": [{"type": "move", "pos": mitte}, {"type": "double_click", "pos": mitte}, {"type": "key", "key": "Escape"}], "screenshot_each": True}, bild="07_jeder_schritt")
pruefe("screenshot_each liefert ein Bild je Schritt", r["images"] == 3, r["images"])

json.dump(ergebnisse, open(os.path.join(out, "ergebnis.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
fehler = [e for e in ergebnisse if not e["ok"]]
print(f"{len(ergebnisse) - len(fehler)}/{len(ergebnisse)} Prüfungen bestanden")
sys.exit(1 if fehler else 0)
