import json, sys, base64, os
src, outdir = sys.argv[1], sys.argv[2]
for line in open(src, encoding="utf-16"):
    line = line.strip()
    if not line:
        continue
    try:
        m = json.loads(line)
    except Exception as e:
        print("KEIN JSON:", line[:200]); continue
    i = m.get("id")
    if "error" in m:
        print(i, "FEHLER", m["error"]); continue
    res = m.get("result", {})
    if i == 1:
        print(i, "init", res.get("serverInfo")); continue
    for k, c in enumerate(res.get("content", [])):
        if c.get("type") == "image":
            p = os.path.join(outdir, f"bild_{i}_{k}.png")
            open(p, "wb").write(base64.b64decode(c["data"]))
            print(i, "BILD", p, len(c["data"]))
        else:
            t = c.get("text", "")
            print(i, "err" if res.get("isError") else "ok", t[:600].replace("\n", " "))
