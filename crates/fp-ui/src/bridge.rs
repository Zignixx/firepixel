//! KI-Steuerkanal: MCP-Brücke zur laufenden Oberfläche.
//!
//! Ist der Steuerkanal eingeschaltet (Umgebungsvariable `FIREPIXEL_MCP=1`, Programmschalter
//! `--mcp` über [`enable`] oder ein Menü-/Voreinstellungsschalter), startet Firepixel einen
//! Streamable-HTTP-MCP-Server **nur auf 127.0.0.1** mit zufälligem Token (fp-mcp). Adresse und
//! Token stehen in `%LOCALAPPDATA%\Firepixel\mcp-bridge.json` (wird beim Beenden gelöscht).
//!
//! Aufträge des Agenten laufen im UI-Thread auf derselben [`fp_engine::Session`] wie Menüs und
//! Werkzeuge: Der Nutzer sieht jede Änderung sofort, alles landet im Verlauf. Zusätzlich
//! beantwortet die Oberfläche die Brücken-Werkzeuge `screenshot` (echtes Fensterbild),
//! `ui_state`, `ui_action`, `ui_input` (rohe Maus-/Tastatur-/Datei-Ereignisse über ein
//! egui-Plugin vor jedem Frame, siehe `inject`) und `ui_find` (Widget-Positionen).
//!
//! Einbindung (HG): in `FirepixelApp::root_ui` direkt nach `input::handle(&ctx, st);`
//! `crate::bridge::frame(&ctx, st);` aufrufen.

mod inject;

use crate::app_state::{Action, AppState};
use fp_mcp::{BridgeHost, BridgeReceiver, HttpOptions, HttpServer, PendingUi, UiReply, UiRequest};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

/// Umgebungsvariable zum Einschalten (`1`, `true`, `ja`).
pub const ENV_ENABLE: &str = "FIREPIXEL_MCP";
/// Umgebungsvariable für den Port (Vorgabe 47800, belegt → freier Port).
pub const ENV_PORT: &str = "FIREPIXEL_MCP_PORT";

static REQUESTED: AtomicBool = AtomicBool::new(false);
static REQUESTED_OFF: AtomicBool = AtomicBool::new(false);
static PORT: AtomicU16 = AtomicU16::new(0);
static GLOBAL: Mutex<Option<UiBridge>> = Mutex::new(None);
static ENV_CHECKED: AtomicBool = AtomicBool::new(false);

/// Schaltet den Steuerkanal ein (wirkt im nächsten Frame). `port = 0` → Vorgabe-Port.
pub fn enable(port: u16) {
    PORT.store(port, Ordering::SeqCst);
    REQUESTED_OFF.store(false, Ordering::SeqCst);
    REQUESTED.store(true, Ordering::SeqCst);
}

/// Schaltet den Steuerkanal aus (wirkt im nächsten Frame).
pub fn disable() {
    REQUESTED.store(false, Ordering::SeqCst);
    REQUESTED_OFF.store(true, Ordering::SeqCst);
}

/// Läuft der Steuerkanal? Liefert die URL.
pub fn running_url() -> Option<String> {
    GLOBAL.lock().ok().and_then(|g| g.as_ref().and_then(|b| b.url()))
}

fn env_wants_bridge() -> bool {
    std::env::var(ENV_ENABLE).map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "ja" | "yes" | "on")).unwrap_or(false)
}

/// Je Frame aufrufen (vor dem Zeichnen). Startet/stoppt den Steuerkanal nach Wunsch und arbeitet
/// die Aufträge des Agenten ab.
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    // JSF-Laufzeit für „Befehle ▸ Skript ausführen“ und gespeicherte Befehle einstecken
    fp_script::install_host(&mut st.session);
    // Datei-Dialoge ohne Blockieren: Ergebnisse abholen, modale Sperre zeigen
    crate::file_dialog::frame(ctx, st);
    if !ENV_CHECKED.swap(true, Ordering::SeqCst) && env_wants_bridge() {
        let port = std::env::var(ENV_PORT).ok().and_then(|p| p.trim().parse().ok()).unwrap_or(0);
        enable(port);
    }
    let Ok(mut g) = GLOBAL.lock() else { return };
    if REQUESTED_OFF.swap(false, Ordering::SeqCst)
        && let Some(b) = g.take()
    {
        b.stop();
        st.set_status(fp_i18n::t("ui-mcp-stopped"), ctx.input(|i| i.time));
    }
    if REQUESTED.swap(false, Ordering::SeqCst) && g.is_none() {
        let port = PORT.load(Ordering::SeqCst);
        match UiBridge::start(ctx, if port == 0 { fp_mcp::DEFAULT_PORT } else { port }, true) {
            Ok(b) => {
                let msg = fp_i18n::tr!("ui-mcp-active", url = b.url().unwrap_or_default(), file = fp_mcp::bridge_token_file().display().to_string());
                log::info!("{msg}");
                eprintln!("{msg}");
                st.set_status(msg, ctx.input(|i| i.time));
                *g = Some(b);
            }
            Err(e) => {
                let msg = fp_i18n::tr!("ui-mcp-failed", error = e.to_string());
                log::warn!("{msg}");
                eprintln!("{msg}");
                st.set_status(msg, ctx.input(|i| i.time));
            }
        }
    }
    if let Some(b) = g.as_mut() {
        b.process(ctx, st);
    }
}

/// Wartendes Bildschirmfoto.
struct PendingShot {
    id: u64,
    rect: Option<[f32; 4]>,
    reply: PendingUi,
    frames: u32,
}

/// Brücke einer Oberflächen-Instanz.
pub struct UiBridge {
    receiver: BridgeReceiver,
    server: Option<HttpServer>,
    shots: Vec<PendingShot>,
    next_shot: u64,
}

impl UiBridge {
    /// Startet Brücke + HTTP-Server. Ist `port` belegt, wird ein freier Port gewählt.
    pub fn start(ctx: &egui::Context, port: u16, write_token_file: bool) -> Result<UiBridge, fp_mcp::McpError> {
        inject::install(ctx);
        let (host, receiver) = BridgeHost::new();
        let c = ctx.clone();
        let host: Arc<BridgeHost> = Arc::new(host.with_waker(move || c.request_repaint()));
        let opts = HttpOptions { port, token: None, token_file: Some(fp_mcp::bridge_token_file()), write_token_file };
        let server = match fp_mcp::start_http_background(host.clone(), opts.clone()) {
            Ok(s) => s,
            Err(_) if port != 0 => fp_mcp::start_http_background(host, HttpOptions { port: 0, ..opts })?,
            Err(e) => return Err(e),
        };
        Ok(UiBridge { receiver, server: Some(server), shots: Vec::new(), next_shot: 1 })
    }

    /// Brücke ohne Server (Tests): Aufträge kommen über den zurückgegebenen Host.
    pub fn detached(ctx: &egui::Context) -> (Arc<BridgeHost>, UiBridge) {
        inject::install(ctx);
        let (host, receiver) = BridgeHost::new();
        let c = ctx.clone();
        let host = Arc::new(host.with_waker(move || c.request_repaint()));
        (host, UiBridge { receiver, server: None, shots: Vec::new(), next_shot: 1 })
    }

    /// MCP-URL (`http://127.0.0.1:<port>/mcp`).
    pub fn url(&self) -> Option<String> {
        self.server.as_ref().map(HttpServer::url)
    }

    /// Token des Servers.
    pub fn token(&self) -> Option<String> {
        self.server.as_ref().map(|s| s.token().to_string())
    }

    /// Beendet den Server (Token-Datei wird gelöscht).
    pub fn stop(mut self) {
        for s in self.shots.drain(..) {
            s.reply.respond(UiReply::Error("Steuerkanal beendet".into()));
        }
        inject::cancel_all();
        if let Some(s) = self.server.take() {
            s.shutdown();
        }
    }

    /// Arbeitet Aufträge und Oberflächen-Anfragen ab (je Frame im UI-Thread).
    pub fn process(&mut self, ctx: &egui::Context, st: &mut AppState) {
        let n = self.receiver.process(&mut st.session);
        if n > 0 {
            // Leinwand und Bedienfelder sofort nachziehen
            ctx.request_repaint();
        }
        for p in self.receiver.take_ui_requests() {
            match p.request.clone() {
                UiRequest::State => {
                    let v = ui_state(ctx, st);
                    p.respond(UiReply::Json(v));
                }
                UiRequest::Action(a) => {
                    let r = ui_action(ctx, st, &a);
                    p.respond(r);
                }
                UiRequest::Screenshot { rect } => {
                    let id = self.next_shot;
                    self.next_shot += 1;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(id)));
                    ctx.request_repaint();
                    self.shots.push(PendingShot { id, rect, reply: p, frames: 0 });
                }
                UiRequest::Input(a) => inject::start_input(ctx, &a, p),
                UiRequest::Find(a) => inject::start_find(ctx, &a, p),
                UiRequest::FileDialog(a) => p.respond(ui_file_dialog(&a)),
            }
        }
        inject::frame(ctx, st.dialogs.any_open(), st.status.as_ref().map(|(m, _)| m.clone()));
        if self.shots.is_empty() {
            return;
        }
        // eingetroffene Bildschirmfotos zuordnen
        let images: Vec<(Option<u64>, Arc<egui::ColorImage>)> = ctx.input(|i| {
            i.raw
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Screenshot { user_data, image, .. } => {
                        let id = user_data.data.as_ref().and_then(|d| d.downcast_ref::<u64>().copied());
                        Some((id, Arc::clone(image)))
                    }
                    _ => None,
                })
                .collect()
        });
        let ppp = ctx.pixels_per_point();
        for (id, img) in images {
            let pos = self.shots.iter().position(|s| Some(s.id) == id).or(if id.is_none() { Some(0) } else { None });
            if let Some(i) = pos {
                let s = self.shots.remove(i);
                s.reply.respond(crop_image(&img, s.rect, ppp));
            }
        }
        // Zeitüberschreitung: nach vielen Frames ohne Bild aufgeben
        let mut keep = Vec::new();
        for mut s in self.shots.drain(..) {
            s.frames += 1;
            if s.frames > 120 {
                s.reply.respond(UiReply::Error("kein Bildschirmfoto erhalten (Fenster minimiert?)".into()));
            } else {
                keep.push(s);
            }
        }
        self.shots = keep;
        if !self.shots.is_empty() {
            ctx.request_repaint();
        }
    }
}

/// Schneidet ein Fensterbild auf einen Ausschnitt in Punkten zu und liefert RGBA8.
fn crop_image(img: &egui::ColorImage, rect: Option<[f32; 4]>, ppp: f32) -> UiReply {
    let [w, h] = img.size;
    let (x0, y0, x1, y1) = match rect {
        Some([l, t, r, b]) => {
            let c = |v: f32, max: usize| ((v * ppp).round().max(0.0) as usize).min(max);
            (c(l.min(r), w), c(t.min(b), h), c(l.max(r), w), c(t.max(b), h))
        }
        None => (0, 0, w, h),
    };
    if x1 <= x0 || y1 <= y0 {
        return UiReply::Error("leerer Ausschnitt".into());
    }
    let mut rgba = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let c = img.pixels[y * w + x];
            let [r, g, b, _] = c.to_srgba_unmultiplied();
            // Fensterbild ist deckend
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    UiReply::Image { width: (x1 - x0) as u32, height: (y1 - y0) as u32, rgba }
}

/// `#rrggbbaa` aus ARGB.
fn argb_hex(c: fp_color::Argb) -> String {
    let v = c.0;
    format!("#{:06x}{:02x}", v & 0x00ff_ffff, v >> 24)
}

fn theme_name(k: crate::theme::ThemeKind) -> &'static str {
    match k {
        crate::theme::ThemeKind::Normal => "normal",
        crate::theme::ThemeKind::Dark => "dunkel",
        crate::theme::ThemeKind::Oled => "oled",
    }
}

/// Zustand der Oberfläche als JSON.
pub fn ui_state(ctx: &egui::Context, st: &AppState) -> Value {
    let s = &st.session;
    let rect = ctx.content_rect();
    let docs: Vec<Value> = s
        .documents()
        .iter()
        .map(|d| {
            // Lage der Leinwand und der Seite im Fenster (Punkte), für ui_input-Koordinaten
            let area = st.canvases.get(&d.id).and_then(|c| c.area);
            let page_on_screen = area.and_then(|a| {
                let p = d.doc.pages.get(d.page)?;
                let (x0, y0) = d.view.doc_to_screen(fp_geom::Point::new(0.0, 0.0));
                let (x1, y1) = d.view.doc_to_screen(fp_geom::Point::new(p.canvas.width, p.canvas.height));
                Some([a.min.x + x0 as f32, a.min.y + y0 as f32, a.min.x + x1 as f32, a.min.y + y1 as f32])
            });
            json!({
                "canvas_area": area.map(|a| [a.min.x, a.min.y, a.max.x, a.max.y]),
                "page_on_screen": page_on_screen,
                "id": d.id.0, "title": d.title, "path": d.path.as_ref().map(|p| p.display().to_string()),
                "dirty": d.is_dirty(), "active": Some(d.id) == s.active_id(),
                "zoom": d.view.zoom, "origin": [d.view.origin.0, d.view.origin.1], "viewport": [d.view.viewport.0, d.view.viewport.1],
                "page": d.page, "state": d.frame, "selection": d.selection.elements.iter().map(|e| e.0).collect::<Vec<_>>(),
                "grid": d.view.show_grid, "guides": d.view.show_guides, "rulers": d.view.show_rulers,
            })
        })
        .collect();
    let panels: Vec<&str> = s.ui.panels.iter().filter(|(_, v)| **v).map(|(k, _)| k.as_str()).collect();
    json!({
        "mode": "bridge",
        "window": {"width": rect.width(), "height": rect.height(), "pixels_per_point": ctx.pixels_per_point()},
        "theme": theme_name(st.settings.theme),
        "tool": fp_engine::tools::info(s.tools.active).map(|i| i.name),
        "stroke_color": argb_hex(s.tools.stroke_color),
        "fill_color": argb_hex(s.tools.fill_color),
        "documents": docs,
        "active_document": s.active_id().map(|d| d.0),
        "visible_panels": panels,
        "screen_mode": s.ui.screen_mode,
        "dialog_open": st.dialogs.any_open(),
        "message": st.dialogs.message.as_ref().map(|(t, m)| json!({"title": t, "text": m})),
        "status": st.status.as_ref().map(|(m, _)| m.clone()),
        "frame": st.frame_counter,
        "window_title": st.window_title(),
        "file_dialog": crate::file_dialog::status_json(),
    })
}

/// Führt eine Oberflächen-Aktion aus (`ui_action`).
pub fn ui_action(ctx: &egui::Context, st: &mut AppState, a: &Value) -> UiReply {
    let mut done: Vec<String> = Vec::new();
    let mut actions: Vec<Action> = Vec::new();
    if a.get("close_dialogs").and_then(Value::as_bool) == Some(true) {
        st.dialogs = crate::dialogs::Dialogs::default();
        done.push("Dialoge geschlossen".into());
    }
    if let Some(name) = a.get("run").and_then(Value::as_str) {
        if st.session.registry().get(name).is_none() {
            return UiReply::Error(format!("unbekannter Befehl „{name}“ – list_commands"));
        }
        actions.push(Action::Run(name.to_string(), a.get("params").cloned().unwrap_or(json!({}))));
        done.push(format!("Befehl {name}"));
    }
    if let Some(z) = a.get("zoom").and_then(Value::as_f64) {
        if !(0.01..=64.0).contains(&z) {
            return UiReply::Error("zoom zwischen 0.01 und 64 (1 = 100 %)".into());
        }
        actions.push(Action::SetZoom(z));
        done.push(format!("Zoom {:.0} %", z * 100.0));
    }
    if a.get("fit").and_then(Value::as_bool) == Some(true) {
        actions.push(Action::Run("view.fitAll".into(), json!({})));
        done.push("eingepasst".into());
    }
    if let Some(p) = a.get("panel").and_then(Value::as_str) {
        let id = crate::panels::PanelId::from_command(p).or_else(|| crate::panels::PanelId::from_command(&format!("window.{p}")));
        match id {
            Some(id) => {
                actions.push(Action::ShowPanel(id));
                done.push(format!("Bedienfeld {p}"));
            }
            None => return UiReply::Error(format!("unbekanntes Bedienfeld „{p}“ (z. B. window.layers, window.history)")),
        }
    }
    if let Some(t) = a.get("tool").and_then(Value::as_str) {
        match fp_engine::tools::by_name(t).or_else(|| fp_engine::tools::by_name(&format!("tool.{t}"))) {
            Some(id) => {
                actions.push(Action::SetTool(id));
                done.push(format!("Werkzeug {t}"));
            }
            None => return UiReply::Error(format!("unbekanntes Werkzeug „{t}“")),
        }
    }
    if let Some(t) = a.get("theme").and_then(Value::as_str) {
        let k = match t.to_lowercase().as_str() {
            "normal" | "hell" => crate::theme::ThemeKind::Normal,
            "dunkel" | "dark" => crate::theme::ThemeKind::Dark,
            "oled" => crate::theme::ThemeKind::Oled,
            other => return UiReply::Error(format!("unbekanntes Design „{other}“ (normal, dunkel, oled)")),
        };
        actions.push(Action::SetTheme(k));
        done.push(format!("Design {t}"));
    }
    if let Some(m) = a.get("status").and_then(Value::as_str) {
        actions.push(Action::Status(m.to_string()));
        done.push("Statuszeile".into());
    }
    if done.is_empty() {
        return UiReply::Error("keine Aktion angegeben (run, zoom, fit, panel, tool, theme, close_dialogs, status)".into());
    }
    for act in actions {
        crate::actions::execute(st, ctx, act);
    }
    st.handle_session_events();
    ctx.request_repaint();
    UiReply::Json(json!({"ok": true, "done": done, "dialog_open": st.dialogs.any_open(), "status": st.status.as_ref().map(|(m, _)| m.clone())}))
}

/// MCP-Werkzeug `ui_file_dialog`: Zustand des Datei-Dialogs bzw. Antwort darauf.
fn ui_file_dialog(a: &Value) -> UiReply {
    match a.get("action").and_then(Value::as_str).unwrap_or("status") {
        "status" => UiReply::Json(crate::file_dialog::status_json()),
        "cancel" => match crate::file_dialog::answer(Vec::new()) {
            Ok(v) => UiReply::Json(v),
            Err(e) => UiReply::Error(e),
        },
        "choose" => {
            let paths: Vec<std::path::PathBuf> = match a.get("path").or_else(|| a.get("paths")) {
                Some(Value::String(s)) => vec![s.into()],
                Some(Value::Array(v)) => v.iter().filter_map(Value::as_str).map(std::path::PathBuf::from).collect(),
                _ => Vec::new(),
            };
            if paths.is_empty() {
                return UiReply::Error("path".into());
            }
            match crate::file_dialog::answer(paths) {
                Ok(v) => UiReply::Json(v),
                Err(e) => UiReply::Error(e),
            }
        }
        other => UiReply::Error(format!("action: status | cancel | choose ({other})")),
    }
}
