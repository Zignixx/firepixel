//! Ziehen aus Bedienfeldern auf die Leinwand (wie im Original):
//! Farbfeld auf ein Objekt → Füllfarbe (mit Umschalt: Strichfarbe), Stil auf ein Objekt → Stil
//! anwenden, Symbol aus der Dokumentbibliothek auf die Leinwand → Instanz an der Ablagestelle,
//! Datei der Allgemeinen Bibliothek → Import an der Ablagestelle, automatische Form → einfügen.
//!
//! Die Quelle legt beim Ziehbeginn ein [`Payload`] ab; [`frame`] zeichnet die Vorschau am Zeiger
//! und legt beim Loslassen über der Leinwand ab (ein Verlaufsschritt). Ablegen außerhalb der
//! Leinwand oder Esc bricht ab.

use crate::app_state::{Action, AppState};
use egui::{Color32, Id, Pos2};
use fp_geom::Point;
use serde_json::json;

/// Gezogener Inhalt.
#[derive(Clone, Debug, PartialEq)]
pub enum Payload {
    /// Farbfeld (ARGB).
    Color(u32),
    /// Stil (Name).
    Style(String),
    /// Symbol der Dokumentbibliothek (Index).
    Symbol(usize),
    /// Datei der Allgemeinen Bibliothek.
    LibraryFile(std::path::PathBuf),
    /// Automatische Form (Name).
    AutoShape(String),
}

const KEY: &str = "ibd-panel-drag";
const DONE: &str = "ibd-panel-drag-pass";

/// Ziehen beginnen.
pub fn start(ctx: &egui::Context, p: Payload) {
    ctx.data_mut(|d| d.insert_temp(Id::new(KEY), p));
}

/// Laufendes Ziehen.
pub fn current(ctx: &egui::Context) -> Option<Payload> {
    ctx.data(|d| d.get_temp(Id::new(KEY)))
}

/// Dokumentpunkt unter `pos`, wenn dort die Leinwand des aktiven Dokuments liegt.
pub fn canvas_point(st: &AppState, pos: Pos2) -> Option<Point> {
    let d = st.session.active()?;
    let area = st.canvases.get(&d.id).and_then(|c| c.area)?;
    if !area.contains(pos) {
        return None;
    }
    let q = d
        .view
        .screen_to_doc((pos.x - area.min.x) as f64, (pos.y - area.min.y) as f64);
    Some(Point::new(q.x, q.y))
}

/// Oberstes Objekt unter einem Dokumentpunkt.
fn object_at(st: &AppState, p: Point) -> Option<u64> {
    let d = st.session.active()?;
    let opt = fp_engine::docops::HitOptions {
        tolerance: 2.0,
        ..Default::default()
    };
    let hit = fp_engine::docops::hits_at(&d.doc, d.page, d.frame, p, st.session.geometry(), &opt);
    hit.first()
        .and_then(|path| d.doc.element_at(path))
        .map(|e| e.id.0)
}

/// Befehl für das Ablegen von `p` am Dokumentpunkt `at` (`None` = hier nichts ablegen).
pub fn drop_action(st: &AppState, p: &Payload, at: Point, shift: bool) -> Option<Action> {
    match p {
        Payload::Color(argb) => {
            let id = object_at(st, at)?;
            let hex = crate::ui_commands::hex(fp_color::Argb(*argb));
            Some(Action::Run(
                if shift { "pi.stroke" } else { "pi.fill" }.into(),
                json!({"ids": [id], "color": hex}),
            ))
        }
        Payload::Style(name) => {
            let id = object_at(st, at)?;
            Some(Action::Run(
                "pi.style.apply".into(),
                json!({"ids": [id], "name": name}),
            ))
        }
        Payload::Symbol(i) => Some(Action::Run(
            "pi.symbol.place".into(),
            json!({"index": i, "at": [at.x.round(), at.y.round()]}),
        )),
        Payload::LibraryFile(f) => Some(Action::Run(
            "file.import".into(),
            json!({"path": f.to_string_lossy(), "at": [at.x.round(), at.y.round()]}),
        )),
        Payload::AutoShape(name) => Some(Action::Run(
            "autoShape.insert".into(),
            json!({"shape": name, "at": [at.x.round(), at.y.round()]}),
        )),
    }
}

/// Einmal je Bild (aus jedem sichtbaren Bedienfeld aufgerufen, wirkt nur beim ersten Aufruf).
pub fn frame(ctx: &egui::Context, st: &mut AppState) {
    let Some(p) = current(ctx) else { return };
    let nr = ctx.cumulative_pass_nr();
    if ctx.data(|d| d.get_temp::<u64>(Id::new(DONE))) == Some(nr) {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(Id::new(DONE), nr));
    let (pos, down, released, esc, shift) = ctx.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.primary_down(),
            i.pointer.primary_released(),
            i.key_pressed(egui::Key::Escape),
            i.modifiers.shift,
        )
    });
    if esc {
        ctx.data_mut(|d| d.remove::<Payload>(Id::new(KEY)));
        return;
    }
    if released || !down {
        ctx.data_mut(|d| d.remove::<Payload>(Id::new(KEY)));
        if let Some(at) = pos.and_then(|q| canvas_point(st, q))
            && let Some(a) = drop_action(st, &p, at, shift)
        {
            st.push(a);
        }
        return;
    }
    // Vorschau am Zeiger
    if let Some(q) = pos {
        let theme = crate::theme::Theme::current(ctx);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            Id::new("ibd-drag-ghost"),
        ));
        let r = egui::Rect::from_min_size(q + egui::vec2(10.0, 10.0), egui::vec2(16.0, 16.0));
        match &p {
            Payload::Color(c) => {
                let a = fp_color::Argb(*c);
                painter.rect_filled(r, 0.0, Color32::from_rgb(a.r(), a.g(), a.b()));
                painter.rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(1.0, Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
            }
            other => {
                let label = match other {
                    Payload::Style(n) | Payload::AutoShape(n) => n.clone(),
                    Payload::Symbol(i) => st
                        .session
                        .active()
                        .and_then(|d| d.doc.symbols.get(*i).map(|s| s.name.clone()))
                        .unwrap_or_default(),
                    Payload::LibraryFile(f) => f
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    Payload::Color(_) => String::new(),
                };
                painter.text(
                    r.left_top(),
                    egui::Align2::LEFT_TOP,
                    label,
                    theme.ui_font(),
                    theme.colors.text,
                );
            }
        }
        // über der Leinwand ohne Ziel: Zeiger „nicht erlaubt“
        let ok = canvas_point(st, q).is_some_and(|at| drop_action(st, &p, at, shift).is_some());
        ctx.set_cursor_icon(if ok {
            egui::CursorIcon::Copy
        } else {
            egui::CursorIcon::NoDrop
        });
    }
    ctx.request_repaint();
}
