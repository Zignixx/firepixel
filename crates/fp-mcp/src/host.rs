//! Zugang zur Sitzung: headless (eigener Thread) oder als Brücke zur laufenden Oberfläche.
//!
//! Der MCP-Server hält die [`Session`] nie selbst. Er schickt Aufträge ([`SessionJob`]) an einen
//! [`SessionHost`], der sie mit exklusivem Zugriff ausführt:
//!
//! * [`LocalHost`]: Die Sitzung lebt auf einem eigenen Arbeits-Thread (CLI `mcp`, Tests).
//! * [`BridgeHost`] + [`BridgeReceiver`]: Die Sitzung gehört der Oberfläche (fp-ui). Der Server
//!   stellt Aufträge in eine Warteschlange, die Oberfläche arbeitet sie in ihrem UI-Thread ab.
//!
//! # Brückenmodus in fp-ui verdrahten
//!
//! ```no_run
//! # use fp_engine::Session;
//! # let mut session = Session::default();
//! // 1. beim Start (z. B. wenn der Nutzer „KI-Steuerkanal“ einschaltet):
//! let (host, receiver) = fp_mcp::BridgeHost::new();
//! // optional: Neuzeichnen anstoßen, sobald ein Auftrag ankommt (egui: ctx.request_repaint())
//! let host = host.with_waker(|| { /* egui_ctx.request_repaint(); */ });
//! let server = fp_mcp::start_http_background(std::sync::Arc::new(host), fp_mcp::HttpOptions::default())?;
//! println!("MCP läuft auf {} (Token in {})", server.url(), server.token_file().display());
//!
//! // 2. in jedem Frame (eframe::App::update), vor dem Zeichnen:
//! let n = receiver.process(&mut session);
//! if n > 0 { /* Ereignisse mit session.drain_events() wie bei Menübefehlen auswerten */ }
//!
//! // 3. beim Beenden:
//! server.shutdown();
//! # Ok::<(), fp_mcp::McpError>(())
//! ```

use crate::error::McpError;
use fp_engine::Session;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Ein Auftrag mit exklusivem Zugriff auf die Sitzung.
pub type SessionJob = Box<dyn FnOnce(&mut Session) + Send + 'static>;

/// Gibt Aufträgen exklusiven Zugriff auf eine Sitzung.
///
/// Umsetzungen müssen Aufträge **nacheinander** in Eingangsreihenfolge ausführen und dürfen
/// einen Auftrag nicht verwerfen, ohne ihn zu droppen (dann erhält der Aufrufer einen Fehler).
pub trait SessionHost: Send + Sync + 'static {
    /// Stellt einen Auftrag ein. Fehler nur, wenn der Host beendet ist.
    fn submit(&self, job: SessionJob) -> Result<(), McpError>;

    /// Kurzbeschreibung für `initialize` (z. B. „headless“ oder „Brücke zur Oberfläche“).
    fn mode(&self) -> &'static str {
        "headless"
    }

    /// Anfrage an die Oberfläche selbst (Bildschirmfoto, Fensterzustand). Nur im Brückenmodus;
    /// headless liefert [`McpError::NoUi`]. `reply` wird genau einmal (oder nie, wenn die
    /// Oberfläche vorher endet) aufgerufen.
    fn ui_request(&self, req: UiRequest, reply: UiResponder) -> Result<(), McpError> {
        let _ = (req, reply);
        Err(McpError::NoUi)
    }
}

/// Anfrage an die laufende Oberfläche (Brückenmodus).
#[derive(Clone, Debug, PartialEq)]
pub enum UiRequest {
    /// Bildschirmfoto des Hauptfensters (optional Ausschnitt in Fensterpunkten `[l, t, r, b]`).
    Screenshot { rect: Option<[f32; 4]> },
    /// Zustand der Oberfläche als JSON (Werkzeug, Zoom, Fenstergröße, Bedienfelder, Dialoge …).
    State,
    /// Aktion der Oberfläche auslösen (wie ein Menüklick, inklusive Dialogen):
    /// `{"run": "file.open"}`, `{"zoom": 2}`, `{"panel": "window.layers"}`, `{"tool": "tool.pen"}`.
    Action(serde_json::Value),
    /// Rohe Eingabe-Ereignisse über mehrere Frames einspeisen (`ui_input`): `{"events": [...]}`.
    Input(serde_json::Value),
    /// Widgets/Texte der Oberfläche suchen (`ui_find`): `{"text": "Datei", "role": "button"}`.
    Find(serde_json::Value),
    /// Datei-Dialog der Oberfläche abfragen/beantworten (`ui_file_dialog`): `{"action": "status"|"set"|"cancel"|"choose", "path"?, "append_fw_png"?, "open_untitled"?, "open_as_animation"?}`.
    FileDialog(serde_json::Value),
}

/// Ein Bild der Oberfläche (RGBA8, nicht vormultipliziert).
#[derive(Clone, Debug, PartialEq)]
pub struct UiImage {
    /// Breite in Pixeln.
    pub width: u32,
    /// Höhe in Pixeln.
    pub height: u32,
    /// Pixel Zeile für Zeile.
    pub rgba: Vec<u8>,
}

/// Antwort der Oberfläche.
#[derive(Clone, Debug, PartialEq)]
pub enum UiReply {
    /// Bild als RGBA8 (nicht vormultipliziert), Zeile für Zeile.
    Image {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    /// JSON-Ergebnis.
    Json(serde_json::Value),
    /// JSON-Ergebnis mit Bildern (z. B. `ui_input` mit Bildschirmfotos zwischen den Schritten).
    JsonImages(serde_json::Value, Vec<UiImage>),
    /// Fehler mit Meldung.
    Error(String),
}

/// Rückruf für die Antwort der Oberfläche.
pub type UiResponder = Box<dyn FnOnce(UiReply) + Send + 'static>;

/// Eine wartende Oberflächen-Anfrage (siehe [`BridgeReceiver::take_ui_requests`]).
pub struct PendingUi {
    /// Die Anfrage.
    pub request: UiRequest,
    reply: UiResponder,
}

impl PendingUi {
    /// Neue wartende Anfrage (für eigene Hosts und Tests).
    pub fn new(request: UiRequest, reply: UiResponder) -> PendingUi {
        PendingUi { request, reply }
    }
    /// Beantwortet die Anfrage.
    pub fn respond(self, r: UiReply) {
        (self.reply)(r);
    }
}

impl std::fmt::Debug for PendingUi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingUi")
            .field("request", &self.request)
            .finish_non_exhaustive()
    }
}

/// Stellt eine Oberflächen-Anfrage und wartet höchstens `timeout` asynchron auf die Antwort.
pub async fn ui_request(
    host: &dyn SessionHost,
    req: UiRequest,
    timeout: Duration,
) -> Result<UiReply, McpError> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    host.ui_request(
        req,
        Box::new(move |r| {
            let _ = tx.send(r);
        }),
    )?;
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(r)) => Ok(r),
        Ok(Err(_)) => Err(McpError::HostGone),
        Err(_) => Err(McpError::Timeout),
    }
}

/// Führt `f` auf der Sitzung aus und wartet asynchron auf das Ergebnis.
pub async fn with_session<R, F>(host: &dyn SessionHost, f: F) -> Result<R, McpError>
where
    R: Send + 'static,
    F: FnOnce(&mut Session) -> R + Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    host.submit(Box::new(move |s| {
        let _ = tx.send(f(s));
    }))?;
    rx.await.map_err(|_| McpError::HostGone)
}

/// Führt `f` auf der Sitzung aus und wartet blockierend (für synchrone Aufrufer, nicht im
/// Tokio-Laufzeitkontext aufrufen).
pub fn with_session_blocking<R, F>(host: &dyn SessionHost, f: F) -> Result<R, McpError>
where
    R: Send + 'static,
    F: FnOnce(&mut Session) -> R + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    host.submit(Box::new(move |s| {
        let _ = tx.send(f(s));
    }))?;
    rx.recv().map_err(|_| McpError::HostGone)
}

// ------------------------------------------------------------------ headless

/// Sitzung auf einem eigenen Arbeits-Thread (headless).
pub struct LocalHost {
    tx: Mutex<Option<mpsc::Sender<SessionJob>>>,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl LocalHost {
    /// Startet den Arbeits-Thread mit der übergebenen Sitzung.
    pub fn new(mut session: Session) -> LocalHost {
        // JSF-Laufzeit für commands.runScript/commands.run
        fp_script::install_host(&mut session);
        let (tx, rx) = mpsc::channel::<SessionJob>();
        let thread = std::thread::Builder::new()
            .name("firepixel-session".into())
            .spawn(move || {
                let mut session = session;
                while let Ok(job) = rx.recv() {
                    job(&mut session);
                }
            })
            .ok();
        LocalHost {
            tx: Mutex::new(Some(tx)),
            thread: Mutex::new(thread),
        }
    }

    /// Neue leere Sitzung mit Vorgabe-Voreinstellungen (keine Voreinstellungsdatei).
    pub fn with_default_session() -> LocalHost {
        LocalHost::new(Session::default())
    }

    /// Beendet den Arbeits-Thread (laufende Aufträge werden noch abgearbeitet).
    pub fn stop(&self) {
        if let Ok(mut g) = self.tx.lock() {
            g.take();
        }
        let handle = self.thread.lock().ok().and_then(|mut g| g.take());
        if let Some(h) = handle {
            let _ = h.join();
        }
    }
}

impl SessionHost for LocalHost {
    fn submit(&self, job: SessionJob) -> Result<(), McpError> {
        let g = self.tx.lock().map_err(|_| McpError::HostGone)?;
        match g.as_ref() {
            Some(tx) => tx.send(job).map_err(|_| McpError::HostGone),
            None => Err(McpError::HostGone),
        }
    }
}

impl Drop for LocalHost {
    fn drop(&mut self) {
        if let Ok(mut g) = self.tx.lock() {
            g.take();
        }
    }
}

// ------------------------------------------------------------------ Brücke

type Waker = Arc<dyn Fn() + Send + Sync>;

/// Brücke zur laufenden Oberfläche: Aufträge landen in einer Warteschlange, die die Oberfläche
/// mit [`BridgeReceiver::process`] in ihrem UI-Thread abarbeitet.
pub struct BridgeHost {
    tx: Mutex<mpsc::Sender<SessionJob>>,
    ui_tx: Mutex<mpsc::Sender<PendingUi>>,
    waker: Option<Waker>,
}

/// Empfangsseite der Brücke (gehört der Oberfläche).
pub struct BridgeReceiver {
    rx: mpsc::Receiver<SessionJob>,
    ui_rx: mpsc::Receiver<PendingUi>,
}

impl BridgeHost {
    /// Neue Brücke: `(Host für den Server, Empfänger für die Oberfläche)`.
    pub fn new() -> (BridgeHost, BridgeReceiver) {
        let (tx, rx) = mpsc::channel();
        let (ui_tx, ui_rx) = mpsc::channel();
        (
            BridgeHost {
                tx: Mutex::new(tx),
                ui_tx: Mutex::new(ui_tx),
                waker: None,
            },
            BridgeReceiver { rx, ui_rx },
        )
    }

    /// Rückruf, der nach jedem eingestellten Auftrag aufgerufen wird (egui: `ctx.request_repaint()`),
    /// damit die Oberfläche auch ohne Mausbewegung einen Frame zeichnet.
    pub fn with_waker(mut self, f: impl Fn() + Send + Sync + 'static) -> BridgeHost {
        self.waker = Some(Arc::new(f));
        self
    }
}

impl SessionHost for BridgeHost {
    fn submit(&self, job: SessionJob) -> Result<(), McpError> {
        {
            let g = self.tx.lock().map_err(|_| McpError::HostGone)?;
            g.send(job).map_err(|_| McpError::HostGone)?;
        }
        if let Some(w) = &self.waker {
            w();
        }
        Ok(())
    }

    fn mode(&self) -> &'static str {
        "bridge"
    }

    fn ui_request(&self, req: UiRequest, reply: UiResponder) -> Result<(), McpError> {
        {
            let g = self.ui_tx.lock().map_err(|_| McpError::HostGone)?;
            g.send(PendingUi::new(req, reply))
                .map_err(|_| McpError::HostGone)?;
        }
        if let Some(w) = &self.waker {
            w();
        }
        Ok(())
    }
}

impl BridgeReceiver {
    /// Arbeitet alle wartenden Aufträge ab; liefert deren Zahl. Im UI-Thread je Frame aufrufen.
    pub fn process(&self, session: &mut Session) -> usize {
        self.process_for(session, Duration::from_millis(50))
    }

    /// Wie [`BridgeReceiver::process`], aber höchstens etwa `budget` lang (Rest im nächsten Frame).
    pub fn process_for(&self, session: &mut Session, budget: Duration) -> usize {
        let start = Instant::now();
        let mut n = 0;
        while let Ok(job) = self.rx.try_recv() {
            job(session);
            n += 1;
            if start.elapsed() >= budget {
                break;
            }
        }
        n
    }

    /// Holt alle wartenden Oberflächen-Anfragen (Bildschirmfoto, Zustand, Aktionen). Die
    /// Oberfläche beantwortet sie mit [`PendingUi::respond`], auch über mehrere Frames hinweg.
    pub fn take_ui_requests(&self) -> Vec<PendingUi> {
        let mut v = Vec::new();
        while let Ok(p) = self.ui_rx.try_recv() {
            v.push(p);
        }
        v
    }

    /// Wartet blockierend höchstens `timeout` auf den nächsten Auftrag und führt ihn aus
    /// (für Tests und Hintergrund-Schleifen ohne Oberfläche).
    pub fn process_one_blocking(&self, session: &mut Session, timeout: Duration) -> bool {
        match self.rx.recv_timeout(timeout) {
            Ok(job) => {
                job(session);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_host_runs_jobs_in_order() {
        let host = LocalHost::with_default_session();
        let n = with_session_blocking(&host, |s| {
            s.execute("file.new", serde_json::json!({"width": 10, "height": 10}))
                .map(|_| s.documents().len())
        });
        assert!(matches!(n, Ok(Ok(1))));
        host.stop();
        assert!(host.submit(Box::new(|_| {})).is_err());
    }

    #[test]
    fn bridge_queues_until_processed() {
        let (host, rx) = BridgeHost::new();
        let woke = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let w = Arc::clone(&woke);
        let host = host.with_waker(move || {
            w.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let (tx, res) = mpsc::channel();
        host.submit(Box::new(move |s| {
            let _ = tx.send(s.documents().len());
        }))
        .ok();
        assert_eq!(woke.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(res.try_recv().is_err());
        let mut session = Session::default();
        assert_eq!(rx.process(&mut session), 1);
        assert_eq!(res.try_recv().ok(), Some(0));
    }
}
