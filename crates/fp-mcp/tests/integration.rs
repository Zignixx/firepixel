//! MCP-Integrationstests: Server starten, `initialize`, `tools/list`, `run_command`, `render`
//! (liefert PNG), Ressourcen; dazu der HTTP-Transport mit Token-Pflicht.

use fp_mcp::{LocalHost, SessionHost};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Client {
    w: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    r: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    next: u64,
}

impl Client {
    async fn send(&mut self, v: &Value) {
        let mut line = v.to_string();
        line.push('\n');
        assert!(self.w.write_all(line.as_bytes()).await.is_ok());
        assert!(self.w.flush().await.is_ok());
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .await;
        loop {
            let mut line = String::new();
            let n = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                self.r.read_line(&mut line),
            )
            .await;
            let Ok(Ok(n)) = n else {
                panic!("keine Antwort auf {method}")
            };
            assert!(n > 0, "Verbindung geschlossen");
            let v: Value = serde_json::from_str(line.trim()).unwrap_or(Value::Null);
            if v.get("id").and_then(|x| x.as_u64()) == Some(id) {
                return v;
            }
        }
    }

    async fn call(&mut self, tool: &str, args: Value) -> Value {
        let r = self
            .request("tools/call", json!({"name": tool, "arguments": args}))
            .await;
        assert!(r.get("error").is_none(), "{tool}: {r}");
        r["result"].clone()
    }
}

async fn start() -> Client {
    let (client_side, server_side) = tokio::io::duplex(1 << 22);
    let (sr, sw) = tokio::io::split(server_side);
    let host: Arc<dyn SessionHost> = Arc::new(LocalHost::with_default_session());
    tokio::spawn(async move {
        let _ = fp_mcp::serve_io(host, sr, sw).await;
    });
    let (cr, cw) = tokio::io::split(client_side);
    let mut c = Client {
        w: cw,
        r: BufReader::new(cr),
        next: 0,
    };
    let init = c
        .request("initialize", json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "fp-test", "version": "1.0"}}))
        .await;
    assert_eq!(init["result"]["serverInfo"]["name"], "firepixel", "{init}");
    assert!(
        init["result"]["instructions"]
            .as_str()
            .is_some_and(|s| s.contains("run_command"))
    );
    c.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .await;
    c
}

fn b64_decode(s: &str) -> Vec<u8> {
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        })
    };
    let mut out = Vec::new();
    let bytes: Vec<u32> = s.bytes().filter_map(val).collect();
    for ch in bytes.chunks(4) {
        let n = ch
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, v)| acc | (v << (18 - 6 * i)));
        out.push((n >> 16) as u8);
        if ch.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if ch.len() > 3 {
            out.push(n as u8);
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stdio_flow_initialize_list_run_render() {
    let mut c = start().await;
    let list = c.request("tools/list", json!({})).await;
    let tools: Vec<String> = list["result"]["tools"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|t| t["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    for t in [
        "list_commands",
        "describe_command",
        "run_command",
        "get_document",
        "select",
        "render",
        "open",
        "save",
        "export",
        "undo",
        "redo",
        "history",
        "new_document",
        "tool_event",
    ] {
        assert!(
            tools.iter().any(|x| x == t),
            "Werkzeug {t} fehlt: {tools:?}"
        );
    }
    let schema = &list["result"]["tools"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["name"] == "run_command").cloned())
        .unwrap_or_default()["inputSchema"];
    assert_eq!(schema["type"], "object");

    let nd = c
        .call(
            "new_document",
            json!({"width": 120, "height": 80, "background": "#ffffff"}),
        )
        .await;
    assert_eq!(nd["isError"], false, "{nd}");
    let r = c.call("run_command", json!({"name": "draw.rectangle", "params": {"rect": [10, 10, 70, 50], "fillColor": "#ff0000", "strokeWidth": 0}})).await;
    assert_eq!(r["isError"], false, "{r}");
    let id = r["structuredContent"]["result"]["id"].as_u64();
    assert!(id.is_some(), "{r}");
    assert_eq!(
        r["structuredContent"]["state"]["selection"][0]["id"].as_u64(),
        id
    );

    let doc = c.call("get_document", json!({"depth": 1})).await;
    let txt = doc["content"][0]["text"].as_str().unwrap_or("");
    assert!(txt.contains("\"rectangle\""), "{txt}");

    let img = c.call("render", json!({"zoom": 2})).await;
    let block = &img["content"][0];
    assert_eq!(block["type"], "image");
    assert_eq!(block["mimeType"], "image/png");
    let png = b64_decode(block["data"].as_str().unwrap_or(""));
    assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    // Breite 240 (IHDR ab Byte 16)
    assert_eq!(
        u32::from_be_bytes([png[16], png[17], png[18], png[19]]),
        240
    );

    let bad = c
        .call("run_command", json!({"name": "modify.align.lefft"}))
        .await;
    assert_eq!(bad["isError"], true);
    assert!(
        bad["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("modify.align.left"))
    );

    let u = c.call("undo", json!({})).await;
    assert_eq!(u["isError"], false);

    let res = c.request("resources/list", json!({})).await;
    assert!(
        res["result"]["resources"]
            .as_array()
            .is_some_and(|a| a.len() >= 4),
        "{res}"
    );
    let cat = c
        .request(
            "resources/read",
            json!({"uri": "firepixel://docs/commands.md"}),
        )
        .await;
    assert!(
        cat["result"]["contents"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("draw.rectangle"))
    );
}

fn http_post(
    port: u16,
    body: &str,
    token: Option<&str>,
    session: Option<&str>,
) -> (String, String) {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("connect: {e}"));
    let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let mut req = format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    if let Some(t) = token {
        req += &format!("Authorization: Bearer {t}\r\n");
    }
    if let Some(sid) = session {
        req += &format!("Mcp-Session-Id: {sid}\r\nMcp-Protocol-Version: 2025-06-18\r\n");
    }
    req += "\r\n";
    req += body;
    assert!(s.write_all(req.as_bytes()).is_ok());
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match s.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                let t = String::from_utf8_lossy(&buf);
                if t.contains("\"result\"")
                    || t.contains("\"error\"")
                    || t.starts_with("HTTP/1.1 4")
                {
                    // genug gelesen
                    if t.contains("\r\n\r\n")
                        && (t.contains("}\n") || t.contains("}\r\n") || t.starts_with("HTTP/1.1 4"))
                    {
                        break;
                    }
                }
            }
            Err(_) => break,
        }
    }
    let t = String::from_utf8_lossy(&buf).to_string();
    let (head, body) = t.split_once("\r\n\r\n").unwrap_or((&t, ""));
    (head.to_string(), body.to_string())
}

#[test]
fn http_requires_token_and_serves_mcp() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap_or_else(|e| panic!("{e}"));
    let host: Arc<dyn SessionHost> = Arc::new(LocalHost::with_default_session());
    let srv = rt
        .block_on(fp_mcp::start_http(
            host,
            fp_mcp::HttpOptions {
                port: 0,
                token: None,
                token_file: None,
                write_token_file: false,
            },
        ))
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(srv.addr().ip().is_loopback());
    let port = srv.addr().port();
    let token = srv.token().to_string();
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}"#;
    let (h, _) = http_post(port, init, None, None);
    assert!(h.starts_with("HTTP/1.1 401"), "{h}");
    let (h, _) = http_post(port, init, Some("falsches-token-falsches-token"), None);
    assert!(h.starts_with("HTTP/1.1 401"), "{h}");
    let (h, b) = http_post(port, init, Some(&token), None);
    assert!(h.starts_with("HTTP/1.1 200"), "{h}");
    assert!(b.contains("firepixel"), "{b}");
    let sid = h.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case("mcp-session-id")
            .then(|| v.trim().to_string())
    });
    if let Some(sid) = sid {
        let _ = http_post(
            port,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            Some(&token),
            Some(&sid),
        );
        let (h2, b2) = http_post(
            port,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
            Some(&token),
            Some(&sid),
        );
        assert!(h2.starts_with("HTTP/1.1 200"), "{h2}");
        assert!(b2.contains("run_command"), "{b2}");
    }
    drop(srv);
    drop(rt);
}

/// Zeilenfilter je Zeile (None/Sub/Up/Avg/Paeth) einer RGBA8-PNG-Datei.
fn png_filter_counts(data: &[u8]) -> (Vec<usize>, Vec<u8>) {
    use fp_fwpng::png as pngc;
    let chunks = pngc::read_chunks(data).unwrap_or_else(|e| panic!("{e}"));
    let z: Vec<u8> = chunks
        .iter()
        .filter(|c| c.is(b"IDAT"))
        .flat_map(|c| c.data.clone())
        .collect();
    let raw = fp_fwpng::zlib::inflate(&z).unwrap_or_else(|e| panic!("{e}"));
    let img = pngc::decode_image(&chunks).unwrap_or_else(|e| panic!("{e}"));
    let stride = img.width as usize * 4 + 1;
    let f: Vec<u8> = (0..img.height as usize).map(|y| raw[y * stride]).collect();
    (
        (0..5u8)
            .map(|k| f.iter().filter(|x| **x == k).count())
            .collect(),
        img.rgba,
    )
}

/// Q-37 / FP1-02: der Exportweg der Brücke (`export`, PNG, wie `export-results/01` „Nur Bilder“)
/// kodiert mit der libpng-Filterregel. Golden: Komposit des Originals aus Lauf 8
/// (0/3/285/0/12); übersprungen ohne Datei.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn k1_filters_match_original_export() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/_testdateien/par/run_lauf8_2026-10-16/export-results/files/01_images_only_result_orig_par.png");
    let Ok(orig) = std::fs::read(&p) else { return };
    let (want, orig_px) = png_filter_counts(&orig);
    assert_eq!(want, vec![0, 3, 285, 0, 12]);
    let mut c = start().await;
    let r = c.call("open", json!({"path": p.to_string_lossy()})).await;
    assert_eq!(r["isError"], false, "{r}");
    let out = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("k1_export_par.png");
    let r = c
        .call("export", json!({"path": out.to_string_lossy()}))
        .await;
    assert_eq!(r["isError"], false, "{r}");
    let data = std::fs::read(&out).unwrap_or_else(|e| panic!("{e}"));
    let (got, px) = png_filter_counts(&data);
    let diff = px.iter().zip(&orig_px).filter(|(a, b)| a != b).count();
    assert_eq!((px.len(), diff), (orig_px.len(), 0), "Pixel weichen ab");
    assert_eq!(got, vec![0, 3, 285, 0, 12]);
}
