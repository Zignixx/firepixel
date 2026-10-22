//! Eigene Mauszeiger der Leinwand – selbst gezeichnet in Anlehnung an die Formen des Originals
//! (Pfeil schwarz mit weißem Rand, Teilauswahl-Anker als hohle Pfeilspitze, Stift mit Zusatzzeichen × ○ + − ^ /, Fadenkreuz,
//! Lupe, Hand, Pipette, Farbeimer …), keine übernommenen Grafiken. Der Systemzeiger wird über der
//! Leinwand ausgeblendet und der Zeiger auf einer Ebene über allem gemalt. Mit
//! `UsePrecisionCursors` (bzw. Feststelltaste) erscheint für Zeichenwerkzeuge das Fadenkreuz.

use crate::theme::Palette;
use egui::{Color32, CursorIcon, Id, Painter, Pos2, Shape, Stroke, ViewportCommand, pos2, vec2};
use fp_engine::tools::Cursor;

/// Window focus and pointer re-entry, as seen by the painted cursor (FB-18, TW-199).
///
/// The painted cursor only works together with a hidden system cursor. Two things broke that:
/// - while the window is inactive the platform shows its own arrow again (macOS ignores the
///   hidden cursor of an inactive window), but the canvas kept painting its cursor next to it;
/// - after the window becomes active again the hidden state is not re-established by the
///   platform, and the windowing layer skips the request because the cursor icon did not change
///   (`CursorIcon::None` frame after frame). Only a different icon (hovering or clicking the
///   toolbar) re-applied it, so system and painted cursor both followed the pointer until then.
#[derive(Clone, Copy, Debug, Default)]
struct FocusSync {
    /// The window lost the focus (no painted cursor; the system cursor is shown).
    unfocused: bool,
    /// The hidden state must be pushed to the window again on the next hiding frame
    /// (after focus gain or after the pointer left the window).
    reapply: bool,
}

fn sync_id() -> Id {
    Id::new("fp-cursor-focus-sync")
}

/// Once per frame before the canvas: follow window focus changes and the pointer leaving the
/// window in this frame's events.
pub(crate) fn track_focus(ctx: &egui::Context) {
    let mut sync: FocusSync = ctx.data(|d| d.get_temp(sync_id())).unwrap_or_default();
    let before = (sync.unfocused, sync.reapply);
    ctx.input(|i| {
        for e in &i.events {
            match e {
                egui::Event::WindowFocused(false) => sync.unfocused = true,
                egui::Event::WindowFocused(true) => {
                    sync.unfocused = false;
                    sync.reapply = true;
                }
                egui::Event::PointerGone => sync.reapply = true,
                _ => {}
            }
        }
    });
    if (sync.unfocused, sync.reapply) != before {
        ctx.data_mut(|d| d.insert_temp(sync_id(), sync));
    }
}

/// May the app paint its own cursor? Not while the window is inactive: the platform shows its
/// own cursor then, and a painted one would be a second cursor.
pub(crate) fn may_paint(ctx: &egui::Context) -> bool {
    !ctx.data(|d| d.get_temp::<FocusSync>(sync_id()))
        .unwrap_or_default()
        .unfocused
}

/// Hide the system cursor for a painted one. After focus gain or re-entry the hidden state is
/// pushed to the window explicitly (visible, then hidden in the same frame: the platform
/// re-evaluates the cursor at once, without a visible frame), because the icon alone does not
/// change and would be skipped.
pub(crate) fn hide_system_cursor(ctx: &egui::Context) {
    ctx.set_cursor_icon(CursorIcon::None);
    let mut sync: FocusSync = ctx.data(|d| d.get_temp(sync_id())).unwrap_or_default();
    if sync.reapply {
        sync.reapply = false;
        ctx.data_mut(|d| d.insert_temp(sync_id(), sync));
        ctx.send_viewport_cmd(ViewportCommand::CursorVisible(true));
        ctx.send_viewport_cmd(ViewportCommand::CursorVisible(false));
    }
}

/// Füllt ein (auch konkaves) einfaches Polygon per Ohrenschneiden.
fn fill_poly(p: &Painter, pts: &[Pos2], color: Color32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let area: f32 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let ccw = area > 0.0;
    let mut idx: Vec<usize> = (0..n).collect();
    let mut mesh = egui::Mesh::default();
    for q in pts {
        mesh.colored_vertex(*q, color);
    }
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let mut guard = 0;
    while idx.len() > 3 && guard < 400 {
        guard += 1;
        let m = idx.len();
        let mut cut = None;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            let cv = cross(a, b, c);
            if (ccw && cv <= 0.0) || (!ccw && cv >= 0.0) {
                continue;
            }
            let inside = idx.iter().any(|&j| {
                if j == ia || j == ib || j == ic {
                    return false;
                }
                let q = pts[j];
                let (d1, d2, d3) = (cross(a, b, q), cross(b, c, q), cross(c, a, q));
                if ccw {
                    d1 > 0.0 && d2 > 0.0 && d3 > 0.0
                } else {
                    d1 < 0.0 && d2 < 0.0 && d3 < 0.0
                }
            });
            if !inside {
                cut = Some(k);
                break;
            }
        }
        let Some(k) = cut else { break };
        let m = idx.len();
        mesh.add_triangle(
            idx[(k + m - 1) % m] as u32,
            idx[k] as u32,
            idx[(k + 1) % m] as u32,
        );
        idx.remove(k);
    }
    if idx.len() == 3 {
        mesh.add_triangle(idx[0] as u32, idx[1] as u32, idx[2] as u32);
    }
    p.add(Shape::mesh(mesh));
}

/// Polygon mit Füllung und Kontur.
fn poly(p: &Painter, at: Pos2, pts: &[(f32, f32)], fill: Color32, outline: Color32) {
    let v: Vec<Pos2> = pts.iter().map(|(x, y)| pos2(at.x + x, at.y + y)).collect();
    fill_poly(p, &v, fill);
    p.add(Shape::closed_line(v, Stroke::new(1.0, outline)));
}

fn line(p: &Painter, at: Pos2, a: (f32, f32), b: (f32, f32), c: &Palette) {
    let (a, b) = (pos2(at.x + a.0, at.y + a.1), pos2(at.x + b.0, at.y + b.1));
    p.line_segment([a, b], Stroke::new(3.0, c.cursor_outline));
    p.line_segment([a, b], Stroke::new(1.0, c.cursor_fill));
}

/// Arrowhead of [`ARROW`] without the shaft (Subselection over an anchor).
const ARROW_HEAD: [(f32, f32); 4] = [(0.0, 0.0), (0.0, 15.0), (3.6, 11.6), (10.8, 10.8)];

/// Pfeil (Spitze im Hotspot).
const ARROW: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.0, 15.0),
    (3.6, 11.6),
    (6.4, 17.6),
    (8.6, 16.6),
    (5.9, 10.8),
    (10.8, 10.8),
];

fn arrow(p: &Painter, at: Pos2, c: &Palette, white: bool) {
    let (f, o) = if white {
        (c.cursor_outline, c.cursor_fill)
    } else {
        (c.cursor_fill, c.cursor_outline)
    };
    poly(p, at, &ARROW, f, o);
}

/// Kleines Zusatzzeichen rechts unten am Pfeil.
fn badge_square(p: &Painter, at: Pos2, c: &Palette, filled: bool) {
    let r = egui::Rect::from_min_size(pos2(at.x + 11.0, at.y + 12.0), vec2(5.0, 5.0));
    p.rect_filled(r.expand(1.0), 0.0, c.cursor_outline);
    if filled {
        p.rect_filled(r, 0.0, c.cursor_fill);
    } else {
        p.rect_stroke(
            r,
            0.0,
            Stroke::new(1.0, c.cursor_fill),
            egui::StrokeKind::Inside,
        );
    }
}

fn badge_plus(p: &Painter, at: Pos2, c: &Palette, minus: bool) {
    let o = pos2(at.x + 13.5, at.y + 14.5);
    let bg = egui::Rect::from_center_size(o, vec2(9.0, 9.0));
    p.rect_filled(bg, 1.0, c.cursor_outline);
    p.line_segment(
        [pos2(o.x - 3.0, o.y), pos2(o.x + 3.0, o.y)],
        Stroke::new(1.0, c.cursor_fill),
    );
    if !minus {
        p.line_segment(
            [pos2(o.x, o.y - 3.0), pos2(o.x, o.y + 3.0)],
            Stroke::new(1.0, c.cursor_fill),
        );
    }
}

fn crosshair(p: &Painter, at: Pos2, c: &Palette, gap: f32, len: f32) {
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        line(p, at, (dx * gap, dy * gap), (dx * len, dy * len), c);
    }
}

/// Doppelpfeil durch den Hotspot in Richtung (dx, dy).
fn double_arrow(p: &Painter, at: Pos2, c: &Palette, dx: f32, dy: f32) {
    let l = (dx * dx + dy * dy).sqrt().max(1e-3);
    let (ux, uy) = (dx / l, dy / l);
    let (nx, ny) = (-uy, ux);
    let s = 8.0;
    let pts = [
        (ux * s, uy * s),
        (ux * (s - 4.0) + nx * 4.0, uy * (s - 4.0) + ny * 4.0),
        (ux * (s - 4.0) + nx * 1.5, uy * (s - 4.0) + ny * 1.5),
        (-ux * (s - 4.0) + nx * 1.5, -uy * (s - 4.0) + ny * 1.5),
        (-ux * (s - 4.0) + nx * 4.0, -uy * (s - 4.0) + ny * 4.0),
        (-ux * s, -uy * s),
        (-ux * (s - 4.0) - nx * 4.0, -uy * (s - 4.0) - ny * 4.0),
        (-ux * (s - 4.0) - nx * 1.5, -uy * (s - 4.0) - ny * 1.5),
        (ux * (s - 4.0) - nx * 1.5, uy * (s - 4.0) - ny * 1.5),
        (ux * (s - 4.0) - nx * 4.0, uy * (s - 4.0) - ny * 4.0),
    ];
    poly(p, at, &pts, c.cursor_fill, c.cursor_outline);
}

/// Stiftspitze (Spitze im Hotspot).
/// Badge stroke: a 3 px dark line under a 1 px light one (readable on light and dark ground).
fn badge_line(p: &Painter, a: Pos2, b: Pos2, c: &Palette) {
    p.line_segment([a, b], Stroke::new(3.0, c.cursor_outline));
    p.line_segment([a, b], Stroke::new(1.0, c.cursor_fill));
}

/// Mark next to the pen nib at its lower right, right of the bar (RE spec 60 section 1.4: every
/// pen cursor is the same nib, the variants differ only in this mark).
fn pen_badge(p: &Painter, at: Pos2, c: &Palette, kind: Cursor) {
    let o = pos2(at.x + 9.0, at.y + 13.0);
    let l = |a: (f32, f32), b: (f32, f32)| {
        badge_line(p, pos2(o.x + a.0, o.y + a.1), pos2(o.x + b.0, o.y + b.1), c)
    };
    match kind {
        // close: a small ring "o"
        Cursor::PenClose => {
            p.circle_stroke(o, 2.5, Stroke::new(3.0, c.cursor_outline));
            p.circle_stroke(o, 2.5, Stroke::new(1.0, c.cursor_fill));
        }
        // add point "+" / delete point "−"
        Cursor::PenAdd => {
            l((-3.0, 0.0), (3.0, 0.0));
            l((0.0, -3.0), (0.0, 3.0));
            p.line_segment(
                [pos2(o.x - 3.0, o.y), pos2(o.x + 3.0, o.y)],
                Stroke::new(1.0, c.cursor_fill),
            );
        }
        Cursor::PenRemove => l((-3.0, 0.0), (3.0, 0.0)),
        // convert point: a caret "^"
        Cursor::PenConvert => {
            l((-3.0, 2.5), (0.0, -2.5));
            l((0.0, -2.5), (3.0, 2.5));
        }
        // join: a small ring with a stub "o-"
        Cursor::PenJoin => {
            l((1.0, 0.0), (5.0, 0.0));
            let ring = pos2(o.x - 1.5, o.y);
            p.circle_stroke(ring, 2.5, Stroke::new(3.0, c.cursor_outline));
            p.circle_stroke(ring, 2.5, Stroke::new(1.0, c.cursor_fill));
        }
        // continue an unselected open path: a slash "/"
        Cursor::PenContinue => l((-3.0, 3.0), (3.0, -3.0)),
        // new path: a small "x"
        Cursor::PenNew => {
            l((-2.5, -2.5), (2.5, 2.5));
            l((-2.5, 2.5), (2.5, -2.5));
            p.line_segment(
                [pos2(o.x - 2.5, o.y - 2.5), pos2(o.x + 2.5, o.y + 2.5)],
                Stroke::new(1.0, c.cursor_fill),
            );
        }
        _ => {}
    }
}

fn magnifier(p: &Painter, at: Pos2, c: &Palette, minus: bool) {
    // Hotspot = Mitte der Linse
    p.circle_filled(at, 6.5, c.cursor_outline);
    p.circle_stroke(at, 5.5, Stroke::new(1.5, c.cursor_fill));
    line(p, at, (4.5, 4.5), (10.0, 10.0), c);
    p.line_segment(
        [pos2(at.x + 4.5, at.y + 4.5), pos2(at.x + 10.0, at.y + 10.0)],
        Stroke::new(2.5, c.cursor_fill),
    );
    p.line_segment(
        [pos2(at.x - 3.0, at.y), pos2(at.x + 3.0, at.y)],
        Stroke::new(1.2, c.cursor_fill),
    );
    if !minus {
        p.line_segment(
            [pos2(at.x, at.y - 3.0), pos2(at.x, at.y + 3.0)],
            Stroke::new(1.2, c.cursor_fill),
        );
    }
}

/// Offene Hand (Hand-Werkzeug bzw. Leertaste ohne Maustaste, PARITAET_A K6): vier gespreizte,
/// getrennte Finger und abgespreizter Daumen.
fn open_hand(p: &Painter, at: Pos2, c: &Palette) {
    let (f, o) = (c.cursor_outline, c.cursor_fill);
    let palm = egui::Rect::from_center_size(pos2(at.x + 0.5, at.y + 4.0), vec2(10.0, 8.0));
    // Finger: Lage x, Länge (Mittel- und Ringfinger länger)
    for (k, len) in [(0.0, 7.0), (1.0, 9.0), (2.0, 8.5), (3.0, 6.0)] {
        let x = palm.left() + 1.0 + k * 3.0;
        let r = egui::Rect::from_min_max(
            pos2(x - 1.0, palm.top() - len),
            pos2(x + 1.0, palm.top() + 2.0),
        );
        p.rect_filled(r.expand(1.0), 2.0, o);
        p.rect_filled(r, 1.0, f);
    }
    let thumb = [
        (palm.left() - 5.0 - at.x, palm.top() - at.y - 0.5),
        (palm.left() - 3.5 - at.x, palm.top() - at.y - 1.5),
        (palm.left() + 0.5 - at.x, palm.center().y - at.y),
        (palm.left() + 0.5 - at.x, palm.bottom() - at.y - 1.0),
    ];
    poly(p, at, &thumb, f, o);
    p.rect_filled(palm.expand(1.0), 3.0, o);
    p.rect_filled(palm, 2.5, f);
}

/// Zuschneide-Symbol wie im Original (PARITAET_A K9, im echten Fenster nachgemessen): zwei
/// verschränkte, 2 px kräftige Winkel und eine kurze Diagonale links oben, pixelgenau auf einem
/// 16×16-Raster; Hotspot in der Mitte des inneren Felds.
fn crop(p: &Painter, at: Pos2, c: &Palette) {
    // Rechtecke im Raster (Spalte, Zeile, Breite, Höhe); Raster-Ursprung 8 px links/oben vom Hotspot
    const PARTS: [(f32, f32, f32, f32); 9] = [
        (0.0, 0.0, 1.0, 1.0),
        (1.0, 1.0, 1.0, 1.0),
        (2.0, 2.0, 1.0, 1.0),
        (3.0, 3.0, 7.0, 2.0),
        (3.0, 3.0, 2.0, 13.0),
        (0.0, 11.0, 13.0, 1.0),
        (3.0, 12.0, 10.0, 1.0),
        (11.0, 0.0, 2.0, 13.0),
        (11.0, 3.0, 5.0, 2.0),
    ];
    // Ursprung auf ganze Pixel (der Hotspot liegt auf einer Pixelmitte)
    let o = pos2(at.x - 8.5, at.y - 8.5);
    let rect = |&(x, y, w, h): &(f32, f32, f32, f32)| {
        egui::Rect::from_min_size(pos2(o.x + x, o.y + y), vec2(w, h))
    };
    // als Netz ohne Kantenglättung: einzelne Pixel der Diagonale bleiben kräftig
    let mut mesh = egui::Mesh::default();
    for (grow, col) in [(1.0, c.cursor_outline), (0.0, c.cursor_fill)] {
        for r in PARTS.iter().map(rect) {
            mesh.add_colored_rect(r.expand(grow), col);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// Federspitze (Stift über leerer Leinwand, PARITAET_A K7): Spitze im Hotspot, nach unten
/// breiter, mit Schlitz, Loch und Schaftkante.
fn nib(p: &Painter, at: Pos2, c: &Palette) {
    let pts = [
        (0.0, 0.0),
        (4.5, 7.0),
        (3.0, 11.0),
        (-3.0, 11.0),
        (-4.5, 7.0),
    ];
    poly(p, at, &pts, c.cursor_fill, c.cursor_outline);
    p.line_segment(
        [pos2(at.x, at.y + 1.0), pos2(at.x, at.y + 6.0)],
        Stroke::new(1.0, c.cursor_outline),
    );
    p.circle_filled(pos2(at.x, at.y + 7.5), 1.2, c.cursor_outline);
    let bar =
        egui::Rect::from_min_max(pos2(at.x - 4.0, at.y + 12.0), pos2(at.x + 4.0, at.y + 14.5));
    p.rect_filled(bar.expand(1.0), 0.0, c.cursor_outline);
    p.rect_filled(bar, 0.0, c.cursor_fill);
}

fn hand(p: &Painter, at: Pos2, c: &Palette, closed: bool) {
    if !closed {
        open_hand(p, at, c);
        return;
    }
    // Hotspot = Handmitte
    let palm = egui::Rect::from_center_size(pos2(at.x, at.y + 3.0), vec2(11.0, 9.0));
    let (f, o) = (c.cursor_outline, c.cursor_fill);
    let finger_len = if closed { 2.5 } else { 6.5 };
    for k in 0..4 {
        let x = palm.left() + 1.5 + k as f32 * 2.7;
        let r = egui::Rect::from_min_max(
            pos2(x - 1.3, palm.top() - finger_len),
            pos2(x + 1.3, palm.top() + 2.0),
        );
        p.rect_filled(r.expand(1.0), 1.5, o);
        p.rect_filled(r, 1.2, f);
    }
    let thumb = [
        (palm.left() - 3.0 - at.x, palm.center().y - at.y - 2.0),
        (palm.left() - at.x, palm.center().y - at.y + 1.0),
        (palm.left() - at.x, palm.bottom() - at.y - 2.0),
        (palm.left() - 4.5 - at.x, palm.center().y - at.y),
    ];
    poly(p, at, &thumb, f, o);
    p.rect_filled(palm.expand(1.0), 3.0, o);
    p.rect_filled(palm, 2.5, f);
}

fn eyedropper(p: &Painter, at: Pos2, c: &Palette) {
    // Spitze links unten im Hotspot
    let pts = [
        (0.0, 0.0),
        (1.0, -3.0),
        (8.0, -10.0),
        (10.0, -8.0),
        (3.0, -1.0),
    ];
    poly(p, at, &pts, c.cursor_outline, c.cursor_fill);
    poly(
        p,
        at,
        &[(8.0, -10.0), (10.5, -14.5), (14.5, -10.5), (10.0, -8.0)],
        c.cursor_fill,
        c.cursor_outline,
    );
}

fn bucket(p: &Painter, at: Pos2, c: &Palette) {
    // Hotspot = Tropfen (Ausguss)
    poly(
        p,
        at,
        &[(2.0, -6.0), (9.0, -13.0), (16.0, -6.0), (9.0, 1.0)],
        c.cursor_outline,
        c.cursor_fill,
    );
    p.circle_filled(at, 2.0, c.cursor_fill);
    p.circle_stroke(at, 2.0, Stroke::new(1.0, c.cursor_outline));
}

fn text_beam(p: &Painter, at: Pos2, c: &Palette) {
    for w in [3.0, 1.0] {
        let col = if w > 1.0 {
            c.cursor_outline
        } else {
            c.cursor_fill
        };
        p.line_segment(
            [pos2(at.x, at.y - 7.0), pos2(at.x, at.y + 7.0)],
            Stroke::new(w, col),
        );
        p.line_segment(
            [pos2(at.x - 3.0, at.y - 8.0), pos2(at.x + 3.0, at.y - 8.0)],
            Stroke::new(w, col),
        );
        p.line_segment(
            [pos2(at.x - 3.0, at.y + 8.0), pos2(at.x + 3.0, at.y + 8.0)],
            Stroke::new(w, col),
        );
        p.line_segment(
            [pos2(at.x - 2.0, at.y + 3.0), pos2(at.x + 2.0, at.y + 3.0)],
            Stroke::new(w, col),
        );
    }
}

fn rotate(p: &Painter, at: Pos2, c: &Palette) {
    // Bogen mit Pfeilspitze, Hotspot im Mittelpunkt
    let pts: Vec<Pos2> = (0..=12)
        .map(|i| {
            let a = std::f32::consts::PI * (0.15 + 0.7 * i as f32 / 12.0) + std::f32::consts::PI;
            pos2(at.x + 7.0 * a.cos(), at.y + 7.0 * a.sin() + 3.0)
        })
        .collect();
    p.add(Shape::line(pts.clone(), Stroke::new(3.5, c.cursor_outline)));
    p.add(Shape::line(pts.clone(), Stroke::new(1.5, c.cursor_fill)));
    if let Some(e) = pts.last() {
        poly(
            p,
            *e,
            &[(0.0, -3.5), (4.0, 1.0), (-2.0, 2.5)],
            c.cursor_fill,
            c.cursor_outline,
        );
    }
}

fn knife(p: &Painter, at: Pos2, c: &Palette) {
    poly(
        p,
        at,
        &[(0.0, 0.0), (10.0, -9.0), (13.0, -6.0), (2.0, 1.0)],
        c.cursor_outline,
        c.cursor_fill,
    );
    poly(
        p,
        at,
        &[(10.0, -9.0), (14.0, -13.0), (17.0, -10.0), (13.0, -6.0)],
        c.cursor_fill,
        c.cursor_outline,
    );
}

fn pencil(p: &Painter, at: Pos2, c: &Palette) {
    poly(
        p,
        at,
        &[
            (0.0, 0.0),
            (1.0, -4.0),
            (11.0, -14.0),
            (14.0, -11.0),
            (4.0, -1.0),
        ],
        c.cursor_outline,
        c.cursor_fill,
    );
    p.line_segment(
        [pos2(at.x + 1.0, at.y - 4.0), pos2(at.x + 4.0, at.y - 1.0)],
        Stroke::new(1.0, c.cursor_fill),
    );
}

fn wand(p: &Painter, at: Pos2, c: &Palette) {
    line(p, at, (0.0, 0.0), (12.0, 12.0), c);
    for (dx, dy) in [(-4.0, 0.0), (4.0, 0.0), (0.0, -4.0), (0.0, 4.0)] {
        p.line_segment(
            [at, pos2(at.x + dx, at.y + dy)],
            Stroke::new(1.0, c.cursor_fill),
        );
    }
}

fn lasso(p: &Painter, at: Pos2, c: &Palette) {
    let center = pos2(at.x + 7.0, at.y - 6.0);
    p.add(Shape::ellipse_stroke(
        center,
        vec2(7.0, 4.5),
        Stroke::new(3.0, c.cursor_outline),
    ));
    p.add(Shape::ellipse_stroke(
        center,
        vec2(7.0, 4.5),
        Stroke::new(1.0, c.cursor_fill),
    ));
    line(p, at, (0.0, 0.0), (3.0, -3.0), c);
}

fn not_allowed(p: &Painter, at: Pos2, c: &Palette) {
    p.circle_stroke(at, 6.0, Stroke::new(3.5, c.cursor_outline));
    p.circle_stroke(at, 6.0, Stroke::new(1.5, c.cursor_fill));
    p.line_segment(
        [pos2(at.x - 4.0, at.y + 4.0), pos2(at.x + 4.0, at.y - 4.0)],
        Stroke::new(1.5, c.cursor_fill),
    );
}

/// Werkzeuge mit Fadenkreuz bei `UsePrecisionCursors` bzw. Feststelltaste.
pub fn has_precision_variant(c: Cursor) -> bool {
    matches!(
        c,
        Cursor::Pen
            | Cursor::PenClose
            | Cursor::PenAdd
            | Cursor::PenRemove
            | Cursor::PenContinue
            | Cursor::PenConvert
            | Cursor::PenJoin
            | Cursor::PenNew
            | Cursor::Pencil
            | Cursor::Eyedropper
            | Cursor::Bucket
            | Cursor::Gradient
            | Cursor::Knife
            | Cursor::Lasso
            | Cursor::Wand
            | Cursor::Hidden
    )
}

/// Zeichnet den Zeiger `c` mit Hotspot `at`.
pub fn paint(p: &Painter, at: Pos2, c: Cursor, colors: &Palette, precise: bool) {
    let at = pos2(at.x.round() + 0.5, at.y.round() + 0.5);
    if precise && has_precision_variant(c) {
        crosshair(p, at, colors, 2.0, 9.0);
        return;
    }
    match c {
        Cursor::Arrow => arrow(p, at, colors, false),
        Cursor::Subselect => arrow(p, at, colors, true),
        // hollow (white) arrowhead without shaft, black border, tip in the hotspot (re 55 §5.4)
        Cursor::SubselectPoint => poly(
            p,
            at,
            &ARROW_HEAD,
            colors.cursor_outline,
            colors.cursor_fill,
        ),
        Cursor::SelectBehind => {
            arrow(p, at, colors, false);
            badge_square(p, at, colors, false);
        }
        Cursor::PointerPoint => {
            arrow(p, at, colors, false);
            badge_square(p, at, colors, true);
        }
        Cursor::PointerSegment => {
            arrow(p, at, colors, true);
            line(p, at, (10.0, 16.0), (17.0, 12.0), colors);
        }
        Cursor::Move => {
            arrow(p, at, colors, false);
            let o = pos2(at.x + 14.0, at.y + 14.0);
            for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                line(p, o, (0.0, 0.0), (dx * 4.0, dy * 4.0), colors);
            }
        }
        Cursor::Copy => {
            arrow(p, at, colors, false);
            badge_plus(p, at, colors, false);
        }
        Cursor::Crosshair => crosshair(p, at, colors, 2.0, 8.0),
        Cursor::Pivot => {
            crosshair(p, at, colors, 0.0, 7.0);
            p.circle_stroke(at, 4.0, Stroke::new(1.0, colors.cursor_fill));
        }
        // Stift ohne Zusatz: Federspitze wie im Original (K7)
        Cursor::Pen => nib(p, at, colors),
        Cursor::Crop => crop(p, at, colors),
        Cursor::PenClose
        | Cursor::PenAdd
        | Cursor::PenRemove
        | Cursor::PenContinue
        | Cursor::PenConvert
        | Cursor::PenJoin
        | Cursor::PenNew => {
            nib(p, at, colors);
            pen_badge(p, at, colors, c);
        }
        Cursor::Text => text_beam(p, at, colors),
        Cursor::Hand => hand(p, at, colors, false),
        Cursor::Grabbing => hand(p, at, colors, true),
        Cursor::ZoomIn => magnifier(p, at, colors, false),
        Cursor::ZoomOut => magnifier(p, at, colors, true),
        Cursor::Rotate => rotate(p, at, colors),
        Cursor::Resize => double_arrow(p, at, colors, 1.0, 1.0),
        Cursor::ResizeNeSw => double_arrow(p, at, colors, 1.0, -1.0),
        Cursor::ResizeH | Cursor::GuideV => double_arrow(p, at, colors, 1.0, 0.0),
        Cursor::ResizeV | Cursor::GuideH => double_arrow(p, at, colors, 0.0, 1.0),
        Cursor::Skew => {
            double_arrow(p, pos2(at.x, at.y - 3.0), colors, 1.0, 0.0);
            double_arrow(p, pos2(at.x + 3.0, at.y + 4.0), colors, 1.0, 0.0);
        }
        Cursor::Distort => {
            arrow(p, at, colors, false);
            poly(
                p,
                at,
                &[(11.0, 11.0), (17.0, 12.0), (16.0, 17.0), (12.0, 16.0)],
                colors.cursor_outline,
                colors.cursor_fill,
            );
        }
        Cursor::Eyedropper => eyedropper(p, at, colors),
        Cursor::Bucket | Cursor::Gradient => {
            bucket(p, at, colors);
            if c == Cursor::Gradient {
                p.line_segment(
                    [pos2(at.x + 4.0, at.y + 6.0), pos2(at.x + 16.0, at.y + 6.0)],
                    Stroke::new(3.0, colors.cursor_outline),
                );
                p.line_segment(
                    [pos2(at.x + 4.0, at.y + 6.0), pos2(at.x + 16.0, at.y + 6.0)],
                    Stroke::new(1.0, colors.cursor_fill),
                );
            }
        }
        Cursor::Brush | Cursor::Eraser | Cursor::Stamp | Cursor::Freeform | Cursor::Reshape => {
            // Größe zeigt die Pinselspitze (BrushTip); hier nur die Mitte
            crosshair(p, at, colors, 1.0, 4.0);
            if c == Cursor::Stamp {
                p.circle_stroke(
                    pos2(at.x + 9.0, at.y - 9.0),
                    3.0,
                    Stroke::new(1.0, colors.cursor_fill),
                );
            }
        }
        Cursor::Pencil => pencil(p, at, colors),
        Cursor::Knife => knife(p, at, colors),
        Cursor::Wand => wand(p, at, colors),
        Cursor::Lasso => lasso(p, at, colors),
        Cursor::NotAllowed => not_allowed(p, at, colors),
        // retouch tools over an editable bitmap: no cursor, the tip outline shows the pointer
        // (RE spec 57 section 2); with precise cursors the crosshair above
        Cursor::Hidden => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cursor_paints() {
        let ctx = egui::Context::default();
        // egui prüft im Debug-Build, dass Textur-Änderungen (Schriftatlas) abgeholt wurden.
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let theme = crate::theme::Theme::new(crate::theme::ThemeKind::Normal);
            let p = ui.painter();
            for c in [
                Cursor::Arrow,
                Cursor::Crosshair,
                Cursor::Move,
                Cursor::Copy,
                Cursor::Hand,
                Cursor::Grabbing,
                Cursor::ZoomIn,
                Cursor::ZoomOut,
                Cursor::Text,
                Cursor::Pen,
                Cursor::PenClose,
                Cursor::Rotate,
                Cursor::Resize,
                Cursor::Eyedropper,
                Cursor::Bucket,
                Cursor::Brush,
                Cursor::NotAllowed,
                Cursor::Hidden,
                Cursor::PointerPoint,
                Cursor::PointerSegment,
                Cursor::Subselect,
                Cursor::SubselectPoint,
                Cursor::SelectBehind,
                Cursor::PenAdd,
                Cursor::PenRemove,
                Cursor::PenContinue,
                Cursor::PenConvert,
                Cursor::PenJoin,
                Cursor::PenNew,
                Cursor::ResizeH,
                Cursor::ResizeV,
                Cursor::ResizeNeSw,
                Cursor::Skew,
                Cursor::Distort,
                Cursor::Pivot,
                Cursor::Knife,
                Cursor::Freeform,
                Cursor::Reshape,
                Cursor::Pencil,
                Cursor::Eraser,
                Cursor::Stamp,
                Cursor::Wand,
                Cursor::Lasso,
                Cursor::Gradient,
                Cursor::GuideH,
                Cursor::GuideV,
                Cursor::Crop,
            ] {
                paint(p, pos2(50.0, 50.0), c, &theme.colors, false);
                paint(p, pos2(50.0, 50.0), c, &theme.colors, true);
            }
        });
        assert!(!out.shapes.is_empty());
        out.textures_delta.clear();
    }
}
