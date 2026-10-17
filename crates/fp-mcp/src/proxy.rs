//! stdio→HTTP-Weiterleitung zur laufenden Oberfläche (`firepixel-cli mcp --bridge`).
//!
//! MCP-Clients, die nur stdio-Server starten können, verbinden sich so mit der laufenden
//! Firepixel-Oberfläche: Jede JSON-RPC-Zeile von stdin geht als `POST /mcp` an den
//! Streamable-HTTP-Endpunkt der Oberfläche (Adresse und Token aus der Token-Datei
//! [`crate::default_token_file`]), die Antworten (JSON oder Server-Sent Events) kommen zeilenweise
//! auf stdout zurück. `Mcp-Session-Id` und `Mcp-Protocol-Version` werden mitgeführt.
//!
//! Bewusst ohne HTTP-Client-Bibliothek: nur `127.0.0.1`, HTTP/1.1, eine Verbindung je Anfrage.

use crate::error::McpError;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// Ziel der Weiterleitung.
#[derive(Clone, Debug, PartialEq)]
pub struct BridgeTarget {
    /// Port auf 127.0.0.1.
    pub port: u16,
    /// Pfad des Endpunkts (`/mcp`).
    pub path: String,
    /// Token für `Authorization: Bearer`.
    pub token: String,
}

impl BridgeTarget {
    /// Aus URL (`http://127.0.0.1:47800/mcp`) und Token.
    pub fn from_url(url: &str, token: &str) -> Result<BridgeTarget, McpError> {
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| McpError::Config(fp_i18n::tr!("err-mcp-only-http", url = url)))?;
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], rest[i..].to_string()),
            None => (rest, crate::MCP_PATH.to_string()),
        };
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or_else(|| McpError::Config(fp_i18n::tr!("err-mcp-port-missing", url = url)))?;
        if !matches!(host, "127.0.0.1" | "localhost" | "[::1]") {
            return Err(McpError::Config(fp_i18n::tr!(
                "err-mcp-only-local",
                host = host
            )));
        }
        let port = port
            .parse()
            .map_err(|_| McpError::Config(fp_i18n::tr!("err-mcp-bad-port", url = url)))?;
        if token.len() < 16 {
            return Err(McpError::Config(fp_i18n::t("err-mcp-token-short")));
        }
        Ok(BridgeTarget {
            port,
            path,
            token: token.to_string(),
        })
    }

    /// Liest Adresse und Token aus der Token-Datei der laufenden Oberfläche.
    pub fn from_token_file(path: &Path) -> Result<BridgeTarget, McpError> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            McpError::Config(fp_i18n::tr!(
                "err-mcp-token-file-unreadable",
                path = path.display().to_string(),
                error = e.to_string()
            ))
        })?;
        let v: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| {
            McpError::Config(fp_i18n::tr!("err-mcp-token-file", error = e.to_string()))
        })?;
        let url = v["url"].as_str().ok_or_else(|| {
            McpError::Config(fp_i18n::tr!("err-mcp-token-file-field", field = "url"))
        })?;
        let token = v["token"].as_str().ok_or_else(|| {
            McpError::Config(fp_i18n::tr!("err-mcp-token-file-field", field = "token"))
        })?;
        BridgeTarget::from_url(url, token)
    }
}

/// Antwort eines POST.
#[derive(Debug, Default)]
struct HttpReply {
    status: u16,
    session: Option<String>,
    /// JSON-RPC-Nachrichten aus der Antwort (JSON-Körper oder SSE-Ereignisse).
    messages: Vec<Value>,
    /// Körper bei Fehlerstatus.
    error_text: String,
}

/// Liest chunked-kodierte Körper schrittweise.
struct Chunked<R: BufRead> {
    inner: R,
    left: usize,
    done: bool,
}

impl<R: BufRead> Read for Chunked<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.done || buf.is_empty() {
            return Ok(0);
        }
        if self.left == 0 {
            let mut line = String::new();
            // Leerzeile nach dem vorigen Block überspringen
            loop {
                line.clear();
                if self.inner.read_line(&mut line)? == 0 {
                    self.done = true;
                    return Ok(0);
                }
                if !line.trim().is_empty() {
                    break;
                }
            }
            let hex = line.trim().split(';').next().unwrap_or("0");
            self.left = usize::from_str_radix(hex, 16)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "chunk"))?;
            if self.left == 0 {
                self.done = true;
                return Ok(0);
            }
        }
        let n = buf.len().min(self.left);
        let got = self.inner.read(&mut buf[..n])?;
        if got == 0 {
            self.done = true;
        }
        self.left -= got;
        Ok(got)
    }
}

fn ids_of(msg: &Value) -> HashSet<String> {
    let mut s = HashSet::new();
    let mut add = |m: &Value| {
        if m.get("method").is_some()
            && let Some(id) = m.get("id")
            && !id.is_null()
        {
            s.insert(id.to_string());
        }
    };
    match msg {
        Value::Array(a) => a.iter().for_each(&mut add),
        m => add(m),
    }
    s
}

/// Weiterleitung mit Sitzungszustand.
pub struct Proxy {
    target: BridgeTarget,
    session: Option<String>,
    protocol: Option<String>,
    /// Zeitlimit je Anfrage.
    pub timeout: Duration,
}

impl Proxy {
    /// Neue Weiterleitung.
    pub fn new(target: BridgeTarget) -> Proxy {
        Proxy {
            target,
            session: None,
            protocol: None,
            timeout: Duration::from_secs(120),
        }
    }

    fn post(
        &self,
        body: &str,
        pending: &HashSet<String>,
        method: &str,
    ) -> Result<HttpReply, McpError> {
        let mut s = TcpStream::connect(("127.0.0.1", self.target.port)).map_err(|e| {
            McpError::Config(fp_i18n::tr!(
                "err-mcp-no-connection",
                port = self.target.port.to_string(),
                error = e.to_string()
            ))
        })?;
        s.set_read_timeout(Some(self.timeout))?;
        let _ = s.set_nodelay(true);
        let mut req = format!(
            "{method} {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nAuthorization: Bearer {}\r\nConnection: close\r\nContent-Length: {}\r\n",
            self.target.path,
            self.target.port,
            self.target.token,
            body.len()
        );
        if let Some(sid) = &self.session {
            req += &format!("Mcp-Session-Id: {sid}\r\n");
        }
        if let Some(p) = &self.protocol {
            req += &format!("Mcp-Protocol-Version: {p}\r\n");
        }
        req += "\r\n";
        req += body;
        s.write_all(req.as_bytes())?;
        let mut r = BufReader::new(s);
        let mut line = String::new();
        r.read_line(&mut line)?;
        let status: u16 = line
            .split_whitespace()
            .nth(1)
            .and_then(|c| c.parse().ok())
            .unwrap_or(0);
        let mut reply = HttpReply {
            status,
            ..HttpReply::default()
        };
        let (mut chunked, mut len, mut sse) = (false, None::<usize>, false);
        loop {
            line.clear();
            if r.read_line(&mut line)? == 0 {
                break;
            }
            let l = line.trim_end();
            if l.is_empty() {
                break;
            }
            if let Some((k, v)) = l.split_once(':') {
                let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
                match k.as_str() {
                    "mcp-session-id" => reply.session = Some(v.to_string()),
                    "transfer-encoding" => chunked = v.eq_ignore_ascii_case("chunked"),
                    "content-length" => len = v.parse().ok(),
                    "content-type" => sse = v.starts_with("text/event-stream"),
                    _ => {}
                }
            }
        }
        let body: Box<dyn Read> = if chunked {
            Box::new(Chunked {
                inner: r,
                left: 0,
                done: false,
            })
        } else if let Some(n) = len {
            Box::new(r.take(n as u64))
        } else {
            Box::new(r)
        };
        let mut body = BufReader::new(body);
        if !(200..300).contains(&status) {
            let mut t = String::new();
            let _ = body.read_to_string(&mut t);
            reply.error_text = t;
            return Ok(reply);
        }
        if sse {
            // Ereignisse lesen, bis alle Antworten auf unsere IDs da sind
            let mut open = pending.clone();
            let mut data = String::new();
            loop {
                line.clear();
                let n = body.read_line(&mut line)?;
                let l = line.trim_end_matches(['\r', '\n']);
                if n == 0 || l.is_empty() {
                    if !data.is_empty() {
                        if let Ok(v) = serde_json::from_str::<Value>(&data) {
                            if v.get("method").is_none()
                                && let Some(id) = v.get("id")
                            {
                                open.remove(&id.to_string());
                            }
                            reply.messages.push(v);
                        }
                        data.clear();
                        if open.is_empty() {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                    continue;
                }
                if let Some(d) = l.strip_prefix("data:") {
                    if !data.is_empty() {
                        data.push('\n');
                    }
                    data.push_str(d.strip_prefix(' ').unwrap_or(d));
                }
            }
        } else {
            let mut t = String::new();
            body.read_to_string(&mut t)?;
            if !t.trim().is_empty() {
                match serde_json::from_str::<Value>(&t) {
                    Ok(Value::Array(a)) => reply.messages.extend(a),
                    Ok(v) => reply.messages.push(v),
                    Err(e) => {
                        return Err(McpError::Protocol(fp_i18n::tr!(
                            "err-mcp-bad-reply",
                            error = e.to_string()
                        )));
                    }
                }
            }
        }
        Ok(reply)
    }

    /// Leitet eine JSON-RPC-Nachricht (oder ein Array) weiter und liefert die Antworten.
    pub fn forward(&mut self, msg: &Value) -> Vec<Value> {
        let body = msg.to_string();
        let ids = ids_of(msg);
        let is_init = msg.get("method").and_then(Value::as_str) == Some("initialize");
        match self.post(&body, &ids, "POST") {
            Ok(r) => {
                if let Some(sid) = r.session {
                    self.session = Some(sid);
                }
                if is_init
                    && let Some(p) = r
                        .messages
                        .iter()
                        .find_map(|m| m["result"]["protocolVersion"].as_str())
                {
                    self.protocol = Some(p.to_string());
                }
                if !(200..300).contains(&r.status) {
                    let text = if r.status == 401 {
                        "Token abgelehnt – Firepixel wurde neu gestartet? firepixel-cli mcp --bridge neu starten".to_string()
                    } else {
                        format!("HTTP {}: {}", r.status, r.error_text.trim())
                    };
                    return ids.iter().map(|id| error_reply(id, &text)).collect();
                }
                r.messages
            }
            Err(e) => ids
                .iter()
                .map(|id| error_reply(id, &e.to_string()))
                .collect(),
        }
    }

    /// Beendet die MCP-Sitzung am Server (`DELETE`).
    pub fn close(&mut self) {
        if self.session.is_some() {
            let _ = self.post("", &HashSet::new(), "DELETE");
            self.session = None;
        }
    }
}

fn error_reply(id: &str, text: &str) -> Value {
    let id: Value = serde_json::from_str(id).unwrap_or(Value::Null);
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": text}})
}

/// Weiterleitung über beliebige Ein-/Ausgabe (Tests) bis zum Eingabeende.
pub fn run_proxy<R: BufRead, W: Write>(
    target: BridgeTarget,
    input: R,
    mut output: W,
) -> Result<(), McpError> {
    let mut proxy = Proxy::new(target);
    for line in input.lines() {
        let line = line?;
        // Windows-Pipes (PowerShell) setzen gern ein BOM vor die erste Zeile
        let t = line.trim().trim_start_matches('\u{feff}');
        if t.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(t) {
            Ok(v) => v,
            Err(e) => {
                writeln!(
                    output,
                    "{}",
                    json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("Parse error: {e}")}})
                )?;
                output.flush()?;
                continue;
            }
        };
        for m in proxy.forward(&msg) {
            writeln!(output, "{m}")?;
        }
        output.flush()?;
    }
    proxy.close();
    Ok(())
}

/// stdio-Weiterleitung zur laufenden Oberfläche (blockiert bis stdin endet).
pub fn run_stdio_proxy(target: BridgeTarget) -> Result<(), McpError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_proxy(target, stdin.lock(), stdout.lock())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_urls() {
        let t = BridgeTarget::from_url("http://127.0.0.1:47800/mcp", "0123456789abcdef0123").ok();
        assert_eq!(
            t.map(|t| (t.port, t.path)),
            Some((47800, "/mcp".to_string()))
        );
        assert!(
            BridgeTarget::from_url("http://example.com:80/mcp", "0123456789abcdef0123").is_err()
        );
        assert!(BridgeTarget::from_url("http://127.0.0.1:1/mcp", "kurz").is_err());
    }

    #[test]
    fn chunked_decoding() {
        let raw = b"5\r\nhallo\r\n6\r\n welt\n\r\n0\r\n\r\n";
        let mut c = Chunked {
            inner: BufReader::new(&raw[..]),
            left: 0,
            done: false,
        };
        let mut s = String::new();
        assert!(c.read_to_string(&mut s).is_ok());
        assert_eq!(s, "hallo welt\n");
    }
}
