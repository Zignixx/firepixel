# Rauchtest für den HTTP-MCP-Server von Firepixel (CLI `firepixel-cli mcp --http` oder Oberfläche im Brückenmodus).
# Liest URL und Token aus %LOCALAPPDATA%\Firepixel\mcp-http.json, führt initialize, tools/list,
# new_document, run_command und render aus und speichert das Bild als smoke_render.png neben diesem Skript.
# Aufruf: python -I tools\golden\fp-mcp\smoke_http.py [pfad\zur\mcp-http.json]
import base64
import json
import os
import sys
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))


def load_info():
    if len(sys.argv) > 1:
        path = sys.argv[1]
    else:
        path = os.path.join(os.environ.get("LOCALAPPDATA", ""), "Firepixel", "mcp-http.json")
    with open(path, encoding="utf-8") as f:
        return json.load(f)


class Client:
    def __init__(self, url, token):
        self.url = url
        self.token = token
        self.session = None
        self.next_id = 0

    def post(self, msg):
        data = json.dumps(msg).encode("utf-8")
        req = urllib.request.Request(self.url, data=data, method="POST")
        req.add_header("Content-Type", "application/json")
        req.add_header("Accept", "application/json, text/event-stream")
        req.add_header("Authorization", "Bearer " + self.token)
        if self.session:
            req.add_header("Mcp-Session-Id", self.session)
            req.add_header("Mcp-Protocol-Version", "2025-06-18")
        with urllib.request.urlopen(req, timeout=60) as resp:
            sid = resp.headers.get("Mcp-Session-Id")
            if sid:
                self.session = sid
            body = resp.read().decode("utf-8", "replace")
        # SSE: "data: {...}"-Zeilen; JSON: Text direkt
        for line in body.splitlines():
            if line.startswith("data:"):
                payload = line[5:].strip()
                if payload:
                    try:
                        v = json.loads(payload)
                    except ValueError:
                        continue
                    if "id" in v and v.get("id") == msg.get("id"):
                        return v
        try:
            return json.loads(body)
        except ValueError:
            return None

    def request(self, method, params):
        self.next_id += 1
        return self.post({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params})

    def call(self, name, args):
        return self.request("tools/call", {"name": name, "arguments": args})


def main():
    info = load_info()
    c = Client(info["url"], info["token"])
    init = c.request("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "smoke", "version": "1"}})
    print("Server:", init["result"]["serverInfo"])
    c.post({"jsonrpc": "2.0", "method": "notifications/initialized"})
    tools = c.request("tools/list", {})["result"]["tools"]
    print("Werkzeuge:", ", ".join(t["name"] for t in tools))
    c.call("new_document", {"width": 200, "height": 120, "background": "#ffffff"})
    r = c.call("run_command", {"name": "draw.ellipse", "params": {"rect": [20, 20, 180, 100], "fillColor": "#3366cc"}})
    print("run_command:", r["result"]["structuredContent"]["result"])
    img = c.call("render", {"zoom": 1})
    block = img["result"]["content"][0]
    out = os.path.join(HERE, "smoke_render.png")
    with open(out, "wb") as f:
        f.write(base64.b64decode(block["data"]))
    print("Bild:", out)


if __name__ == "__main__":
    main()
