//! Brückenmodus Ende-zu-Ende ohne Fenster: `BridgeHost` + HTTP-Server, die „Oberfläche“ ist ein
//! Thread, der Aufträge abarbeitet und Oberflächen-Anfragen beantwortet; der Client ist die
//! stdio-Weiterleitung (`firepixel-cli mcp --bridge`).

use fp_mcp::proxy::{run_proxy, BridgeTarget};
use fp_mcp::{BridgeHost, HttpOptions, UiReply, UiRequest};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn line(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

fn call(id: u64, tool: &str, args: Value) -> String {
    line(id, "tools/call", json!({"name": tool, "arguments": args}))
}

#[test]
fn proxy_drives_bridge_session() {
    let (host, receiver) = BridgeHost::new();
    let host = Arc::new(host);
    let stop = Arc::new(AtomicBool::new(false));
    let st = Arc::clone(&stop);
    // „Oberfläche“: Sitzung im eigenen Thread
    let ui = std::thread::spawn(move || {
        let mut session = fp_engine::Session::default();
        let mut jobs = 0usize;
        while !st.load(Ordering::SeqCst) {
            if receiver.process_one_blocking(&mut session, Duration::from_millis(10)) {
                jobs += 1;
            }
            for p in receiver.take_ui_requests() {
                match p.request.clone() {
                    UiRequest::State => p.respond(UiReply::Json(json!({"mode": "bridge", "documents": session.documents().len()}))),
                    UiRequest::Screenshot { .. } => p.respond(UiReply::Image { width: 4, height: 2, rgba: vec![200; 32] }),
                    UiRequest::Action(_) | UiRequest::Find(_) | UiRequest::FileDialog(_) => p.respond(UiReply::Error("keine Oberfläche im Test".into())),
                    UiRequest::Input(_) => p.respond(UiReply::JsonImages(json!({"ok": true}), vec![fp_mcp::UiImage { width: 2, height: 2, rgba: vec![9; 16] }])),
                }
            }
        }
        (jobs, session.documents().len())
    });
    let dir = std::env::temp_dir().join(format!("fp-mcp-bridge-test-{}", std::process::id()));
    let token_file = dir.join("mcp-bridge.json");
    let srv = fp_mcp::start_http_background(host, HttpOptions { port: 0, token: None, token_file: Some(token_file.clone()), write_token_file: true })
        .unwrap_or_else(|e| panic!("start: {e}"));
    let target = BridgeTarget::from_token_file(&token_file).unwrap_or_else(|e| panic!("token: {e}"));
    assert_eq!(target.port, srv.addr().port());

    let input = [
        line(1, "initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "t", "version": "1"}})),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string(),
        line(2, "tools/list", json!({})),
        call(3, "new_document", json!({"width": 120, "height": 80})),
        call(4, "run_command", json!({"name": "draw.ellipse", "params": {"rect": [10, 10, 60, 50]}})),
        call(5, "ui_state", json!({})),
        call(6, "screenshot", json!({})),
        call(7, "render", json!({"zoom": 1})),
        call(8, "ui_input", json!({"events": [{"type": "click", "pos": [10, 10]}]})),
        "kein json".to_string(),
    ]
    .join("\n");
    let mut out = Vec::new();
    run_proxy(target, std::io::Cursor::new(input), &mut out).unwrap_or_else(|e| panic!("proxy: {e}"));
    let text = String::from_utf8_lossy(&out).to_string();
    let msgs: Vec<Value> = text.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let by_id = |id: u64| msgs.iter().find(|m| m["id"] == json!(id)).cloned().unwrap_or(Value::Null);

    assert!(by_id(1)["result"]["serverInfo"]["title"].as_str().is_some_and(|t| t.contains("bridge")), "{text}");
    let tools = by_id(2)["result"]["tools"].as_array().map(|a| a.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_default();
    for t in ["run_command", "screenshot", "ui_state", "ui_action", "ui_input", "ui_find", "get_document"] {
        assert!(tools.iter().any(|n| n == t), "{t} fehlt: {tools:?}");
    }
    assert_eq!(by_id(3)["result"]["isError"], false, "{text}");
    assert_eq!(by_id(4)["result"]["isError"], false, "{text}");
    assert_eq!(by_id(5)["result"]["structuredContent"]["documents"], 1, "{text}");
    assert_eq!(by_id(6)["result"]["content"][0]["type"], "image", "{text}");
    assert_eq!(by_id(6)["result"]["content"][0]["mimeType"], "image/png");
    assert_eq!(by_id(7)["result"]["content"][0]["type"], "image");
    assert_eq!(by_id(8)["result"]["content"][0]["type"], "image", "{text}");
    assert_eq!(by_id(8)["result"]["structuredContent"]["ok"], true, "{text}");
    assert!(msgs.iter().any(|m| m["error"]["code"] == -32700), "Parse-Fehler fehlt: {text}");

    stop.store(true, Ordering::SeqCst);
    let (jobs, docs) = ui.join().unwrap_or((0, 0));
    assert!(jobs >= 3, "Aufträge im UI-Thread: {jobs}");
    assert_eq!(docs, 1);
    srv.shutdown();
    assert!(!token_file.exists(), "Token-Datei muss beim Beenden verschwinden");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn headless_ui_tools_explain_bridge() {
    let mut s = fp_engine::Session::default();
    let r = fp_mcp::tools::call(&mut s, &fp_mcp::tools::ToolEnv::default(), "screenshot", &json!({}));
    // Antwortsprache der Agenten: Englisch (Vorgabe), Deutsch auf Wunsch (UE-3)
    assert!(r.is_error && r.text().contains("bridge mode"), "{}", r.text());
    let r = fp_mcp::tools::call(&mut s, &fp_mcp::tools::ToolEnv::default(), "screenshot", &json!({"language": "de"}));
    assert!(r.is_error && r.text().contains("Brückenmodus"), "{}", r.text());
}

#[test]
fn proxy_reports_missing_app() {
    let missing = std::env::temp_dir().join("fp-mcp-gibt-es-nicht").join("mcp-bridge.json");
    let e = BridgeTarget::from_token_file(&missing);
    assert!(e.is_err_and(|e| e.to_string().contains("--mcp")));
    // Ziel ohne Server: JSON-RPC-Fehler statt Abbruch
    let t = BridgeTarget::from_url("http://127.0.0.1:1/mcp", "0123456789abcdef0123").unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    let r = run_proxy(t, std::io::Cursor::new(line(9, "tools/list", json!({}))), &mut out);
    assert!(r.is_ok());
    let v: Value = serde_json::from_slice(out.split(|b| *b == b'\n').next().unwrap_or(&[])).unwrap_or(Value::Null);
    assert_eq!(v["id"], 9);
    assert!(v["error"]["message"].as_str().is_some_and(|m| m.contains("keine Verbindung")), "{v}");
}
