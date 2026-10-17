import json, sys
out = sys.argv[1]
def call(i, name, args):
    return {"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": name, "arguments": args}}
reqs = [
    {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "e2e", "version": "1"}}},
    {"jsonrpc": "2.0", "method": "notifications/initialized"},
    call(2, "new_document", {"width": 500, "height": 300, "background": "#ffffff"}),
    call(3, "run_command", {"name": "draw.rectangle", "params": {"rect": [20, 20, 480, 280], "fillColor": "#1e3a8a", "strokeWidth": 0, "name": "Hintergrund"}}),
    call(4, "run_command", {"name": "draw.text", "params": {"text": "MCP-Bruecke", "anchor": [60, 170], "size": 48, "color": "#fde047"}}),
    call(5, "tool_event", {"tool": "ellipse", "drag": {"from": [380, 40], "to": [460, 120]}}),
    call(6, "get_document", {"depth": 3}),
    call(7, "ui_action", {"fit": True}),
    call(8, "ui_state", {}),
    call(9, "screenshot", {"max_size": 1600}),
    call(10, "render", {"zoom": 1}),
    call(11, "save", {"path": out + "\\bruecke.fw.png"}),
    call(12, "export", {"path": out + "\\bruecke.png"}),
    call(13, "select", {"all": True}),
    call(14, "ui_action", {"panel": "window.layers"}),
    call(15, "undo", {}),
    call(16, "screenshot", {"rect": [0, 0, 700, 400]}),
    call(18, "run_command", {"name": "autoShape.insert", "params": {"shape": "Stern", "at": [300, 150]}}),
    call(19, "run_script", {"code": "var d = fw.getDocumentDOM(); d.dragControlPoint(1, {x: 412, y: 236}, false, false, false); fw.selection[0].customData.radius1"}),
    call(20, "screenshot", {"max_size": 1600}),
    call(17, "run_script", {"code": "var d = fw.getDocumentDOM(); d.addNewRectangle({left:30,top:200,right:120,bottom:260}, 0); d.width + 'x' + d.height"}),
]
for r in reqs:
    print(json.dumps(r))
