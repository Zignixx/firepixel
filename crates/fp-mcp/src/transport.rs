//! Transporte: stdio und Streamable HTTP (nur 127.0.0.1, Token-Pflicht).
//!
//! Sicherheit des HTTP-Kanals:
//! * Bindung ausschließlich an `127.0.0.1` (Verbindungen von anderen Adressen werden verworfen).
//! * Jede Anfrage braucht `Authorization: Bearer <token>` (alternativ `X-Firepixel-Token: <token>`);
//!   das Token ist zufällig (256 Bit) und wird beim Start in eine Datei im Benutzerprofil
//!   geschrieben und auf stderr ausgegeben. Vergleich in konstanter Zeit.
//! * `Host`-Prüfung gegen DNS-Rebinding (nur `localhost`/`127.0.0.1`/`::1`, über rmcp).
//! * Die Werkzeuge führen keine Shell-Befehle aus; Dateizugriffe nur über Öffnen/Speichern/Export.

use crate::error::McpError;
use crate::host::SessionHost;
use crate::server::FirepixelMcp;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use rmcp::ServiceExt;
use std::convert::Infallible;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Pfad des MCP-Endpunkts.
pub const MCP_PATH: &str = "/mcp";
/// Vorgabe-Port des HTTP-Servers.
pub const DEFAULT_PORT: u16 = 47_800;

// ------------------------------------------------------------------ stdio

/// Bedient MCP über stdin/stdout, bis der Client die Verbindung schließt.
pub async fn serve_stdio(host: Arc<dyn SessionHost>) -> Result<(), McpError> {
    let running = FirepixelMcp::new(host).serve(rmcp::transport::stdio()).await.map_err(|e| McpError::Protocol(e.to_string()))?;
    running.waiting().await.map_err(|e| McpError::Protocol(e.to_string()))?;
    Ok(())
}

/// Bedient MCP über ein beliebiges Lese-/Schreibpaar (Tests, Pipes).
pub async fn serve_io<R, W>(host: Arc<dyn SessionHost>, reader: R, writer: W) -> Result<(), McpError>
where
    R: tokio::io::AsyncRead + Send + Unpin + 'static,
    W: tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    let running = FirepixelMcp::new(host).serve((reader, writer)).await.map_err(|e| McpError::Protocol(e.to_string()))?;
    running.waiting().await.map_err(|e| McpError::Protocol(e.to_string()))?;
    Ok(())
}

/// stdio-Server mit eigener Tokio-Laufzeit (blockiert bis Verbindungsende).
pub fn run_stdio_blocking(host: Arc<dyn SessionHost>) -> Result<(), McpError> {
    let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build()?;
    rt.block_on(serve_stdio(host))
}

// ------------------------------------------------------------------ HTTP

/// Einstellungen des HTTP-Servers.
#[derive(Clone, Debug)]
pub struct HttpOptions {
    /// Port auf 127.0.0.1 (0 = freien Port wählen).
    pub port: u16,
    /// Festes Token (Vorgabe: zufällig).
    pub token: Option<String>,
    /// Token-Datei (Vorgabe: [`default_token_file`]); `None` in [`HttpOptions::token_file`] + `write_token_file = false` schreibt keine Datei.
    pub token_file: Option<PathBuf>,
    /// Token-Datei schreiben.
    pub write_token_file: bool,
}

impl Default for HttpOptions {
    fn default() -> Self {
        HttpOptions { port: DEFAULT_PORT, token: None, token_file: None, write_token_file: true }
    }
}

/// Vorgabe-Datei für Adresse und Token: `%LOCALAPPDATA%\Firepixel\mcp-http.json`
/// (sonst `$XDG_STATE_HOME/firepixel` bzw. `~/.local/state/firepixel`); in Testläufen im
/// Testprofil (`fp_i18n::profile`).
pub fn default_token_file() -> PathBuf {
    let base = fp_i18n::profile::local_dir().unwrap_or_else(|| std::env::temp_dir().join("firepixel"));
    base.join("mcp-http.json")
}

/// Token-Datei der laufenden Oberfläche (Brückenmodus): `%LOCALAPPDATA%\Firepixel\mcp-bridge.json`.
/// Getrennt von [`default_token_file`], damit ein headless `firepixel-cli mcp --http` die Brücke
/// nicht verdeckt.
pub fn bridge_token_file() -> PathBuf {
    default_token_file().with_file_name("mcp-bridge.json")
}

/// Zufälliges Token (64 Hex-Zeichen = 256 Bit).
pub fn generate_token() -> Result<String, McpError> {
    let mut buf = [0u8; 32];
    getrandom::fill(&mut buf).map_err(|e| McpError::Config(fp_i18n::tr!("err-mcp-random", error = e.to_string())))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn authorized<B>(req: &Request<B>, token: &str) -> bool {
    let h = req.headers();
    if let Some(v) = h.get(hyper::header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        let v = v.trim();
        if let Some(t) = v.strip_prefix("Bearer ").or_else(|| v.strip_prefix("bearer ")) {
            return constant_time_eq(t.trim().as_bytes(), token.as_bytes());
        }
    }
    if let Some(v) = h.get("x-firepixel-token").and_then(|v| v.to_str().ok()) {
        return constant_time_eq(v.trim().as_bytes(), token.as_bytes());
    }
    false
}

type BoxBody = http_body_util::combinators::BoxBody<Bytes, Infallible>;

fn plain(status: StatusCode, text: &str) -> Response<BoxBody> {
    let mut r = Response::new(Full::new(Bytes::from(text.to_string())).boxed());
    *r.status_mut() = status;
    r.headers_mut().insert(hyper::header::CONTENT_TYPE, hyper::header::HeaderValue::from_static("text/plain; charset=utf-8"));
    if status == StatusCode::UNAUTHORIZED {
        r.headers_mut().insert(hyper::header::WWW_AUTHENTICATE, hyper::header::HeaderValue::from_static("Bearer realm=\"firepixel\""));
    }
    r
}

/// Laufender HTTP-Server.
pub struct HttpServer {
    addr: SocketAddr,
    token: String,
    token_file: Option<PathBuf>,
    cancel: CancellationToken,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl HttpServer {
    /// Gebundene Adresse.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
    /// MCP-URL (`http://127.0.0.1:<port>/mcp`).
    pub fn url(&self) -> String {
        format!("http://{}{}", self.addr, MCP_PATH)
    }
    /// Token für `Authorization: Bearer …`.
    pub fn token(&self) -> &str {
        &self.token
    }
    /// Datei mit Adresse und Token (falls geschrieben).
    pub fn token_file(&self) -> PathBuf {
        self.token_file.clone().unwrap_or_default()
    }
    /// Beendet den Server und löscht die Token-Datei.
    pub fn shutdown(mut self) {
        self.stop();
    }
    fn stop(&mut self) {
        self.cancel.cancel();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if let Some(f) = self.token_file.take() {
            let _ = std::fs::remove_file(f);
        }
    }
    /// Wartet, bis der Server beendet wird (CLI `mcp --http`).
    pub async fn wait(&self) {
        self.cancel.cancelled().await;
    }
}

impl Drop for HttpServer {
    fn drop(&mut self) {
        if self.thread.is_some() || self.token_file.is_some() {
            self.stop();
        }
    }
}

fn write_token_file(path: &Path, addr: SocketAddr, token: &str, mode: &str) -> Result<(), McpError> {
    fp_i18n::profile::guard_write(path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let v = serde_json::json!({
        "url": format!("http://{addr}{MCP_PATH}"), "port": addr.port(), "token": token,
        "header": format!("Authorization: Bearer {token}"), "pid": std::process::id(), "mode": mode,
    });
    let text = serde_json::to_string_pretty(&v).map_err(|e| McpError::Config(e.to_string()))?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Startet den HTTP-Server im **laufenden** Tokio-Kontext (Accept-Schleife als Task).
pub async fn start_http(host: Arc<dyn SessionHost>, opts: HttpOptions) -> Result<HttpServer, McpError> {
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, opts.port)).await?;
    let addr = listener.local_addr()?;
    let token = match opts.token {
        Some(t) if t.len() >= 16 => t,
        Some(_) => return Err(McpError::Config(fp_i18n::t("err-mcp-token-min"))),
        None => generate_token()?,
    };
    let token_file = if opts.write_token_file {
        let p = opts.token_file.clone().unwrap_or_else(default_token_file);
        write_token_file(&p, addr, &token, host.mode())?;
        Some(p)
    } else {
        None
    };
    let cancel = CancellationToken::new();
    let config = StreamableHttpServerConfig::default().with_cancellation_token(cancel.child_token());
    let h2 = Arc::clone(&host);
    let service: StreamableHttpService<FirepixelMcp, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(FirepixelMcp::new(Arc::clone(&h2))), Arc::new(LocalSessionManager::default()), config);
    let ct = cancel.clone();
    let tok = Arc::new(token.clone());
    tokio::spawn(async move {
        loop {
            let accepted = tokio::select! {
                _ = ct.cancelled() => break,
                a = listener.accept() => a,
            };
            let Ok((stream, peer)) = accepted else { continue };
            if !peer.ip().is_loopback() {
                continue;
            }
            let svc = service.clone();
            let tok = Arc::clone(&tok);
            let conn_ct = ct.clone();
            tokio::spawn(async move {
                let hs = hyper::service::service_fn(move |req: Request<Incoming>| {
                    let mut svc = svc.clone();
                    let tok = Arc::clone(&tok);
                    async move {
                        if !authorized(&req, &tok) {
                            return Ok::<_, Infallible>(plain(StatusCode::UNAUTHORIZED, "Token fehlt oder falsch (Authorization: Bearer <token>)"));
                        }
                        if !req.uri().path().starts_with(MCP_PATH) {
                            return Ok(plain(StatusCode::NOT_FOUND, "MCP-Endpunkt ist /mcp"));
                        }
                        tower_service::Service::call(&mut svc, req).await
                    }
                });
                let conn = hyper::server::conn::http1::Builder::new().serve_connection(TokioIo::new(stream), hs);
                tokio::select! {
                    _ = conn => {},
                    _ = conn_ct.cancelled() => {},
                }
            });
        }
    });
    Ok(HttpServer { addr, token, token_file, cancel, thread: None })
}

/// Startet den HTTP-Server auf einem eigenen Thread mit eigener Tokio-Laufzeit (für die
/// Oberfläche); kehrt zurück, sobald der Port gebunden ist.
pub fn start_http_background(host: Arc<dyn SessionHost>, opts: HttpOptions) -> Result<HttpServer, McpError> {
    let (tx, rx) = std::sync::mpsc::channel::<Result<(SocketAddr, String, Option<PathBuf>, CancellationToken), McpError>>();
    let thread = std::thread::Builder::new().name("firepixel-mcp-http".into()).spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(McpError::Io(e)));
                return;
            }
        };
        rt.block_on(async move {
            match start_http(host, opts).await {
                Ok(mut srv) => {
                    let cancel = srv.cancel.clone();
                    let file = srv.token_file.take();
                    let _ = tx.send(Ok((srv.addr, srv.token.clone(), file, cancel.clone())));
                    cancel.cancelled().await;
                    // Accept-Schleife endet über das Token; kurz Zeit zum Abbauen geben
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
        });
    })?;
    match rx.recv() {
        Ok(Ok((addr, token, token_file, cancel))) => Ok(HttpServer { addr, token, token_file, cancel, thread: Some(thread) }),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(McpError::Protocol(fp_i18n::t("err-mcp-http-thread"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_random_hex() {
        let a = generate_token().unwrap_or_default();
        let b = generate_token().unwrap_or_default();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn auth_header_checks() {
        let ok = Request::builder().header("Authorization", "Bearer abcdefabcdefabcdef").body(()).unwrap_or_default();
        assert!(authorized(&ok, "abcdefabcdefabcdef"));
        let alt = Request::builder().header("X-Firepixel-Token", "abcdefabcdefabcdef").body(()).unwrap_or_default();
        assert!(authorized(&alt, "abcdefabcdefabcdef"));
        let bad = Request::builder().header("Authorization", "Bearer falsch").body(()).unwrap_or_default();
        assert!(!authorized(&bad, "abcdefabcdefabcdef"));
        let none = Request::builder().body(()).unwrap_or_default();
        assert!(!authorized(&none, "abcdefabcdefabcdef"));
    }
}
