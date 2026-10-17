//! Animationsinstanzen auf der Leinwand (TW-20): Der Auswahlrahmen umschließt nur das Bild im
//! aktuellen Status, dazu der Bewegungspfad mit grünem Start-, rotem Endpunkt und
//! Zwischenpunkten wie im Original (`inspector-group-symbol/09_animation_path_normal`).

use egui::{Color32, Painter, Pos2, Stroke};
use fp_geom::{Matrix3, Point, Rect};
use fp_model::{Document, Element, ElementKind, InstanceElem, SymbolKind};

/// Startpunkt des Bewegungspfads (Grün wie im Original).
pub const START: Color32 = Color32::from_rgb(0, 200, 0);
/// Endpunkt (Rot).
pub const END: Color32 = Color32::from_rgb(220, 0, 0);

/// Animationsinstanz mit mehr als einem Status?
fn anim(doc: &Document, e: &Element) -> Option<(InstanceElem, u32)> {
    let ElementKind::Instance(i) = &e.kind else {
        return None;
    };
    let (_, sym) = doc.symbol_by_id(i.symbol_id)?;
    (sym.kind() == SymbolKind::Animation && i.anim.frames > 1)
        .then(|| (i.clone(), sym.root.frame_count))
}

/// Grenzen der Instanz mit der Matrix `m` ohne Animationsweg.
fn bounds_with(doc: &Document, e: &Element, i: &InstanceElem, m: Matrix3) -> Option<Rect> {
    let mut c = i.clone();
    c.transform = m;
    c.anim.frames = 1;
    let mut el = e.clone();
    el.kind = ElementKind::Instance(c);
    fp_engine::docops::element_bounds(doc, &el)
}

/// Matrix des Status `k` (wie fp-render: Skalierung/Drehung um die Symbolmitte, dann
/// Instanzmatrix, dann Versatz).
fn frame_matrix(i: &InstanceElem, frames: u32, center: Point, k: usize) -> Option<Matrix3> {
    let af = i.anim.frame(k, frames)?;
    let local = Matrix3::translation(-center.x, -center.y)
        .then(&Matrix3::scaling(af.scale as f32, af.scale as f32))
        .then(&Matrix3::rotation_degrees(af.rotation))
        .then(&Matrix3::translation(center.x, center.y));
    Some(
        local
            .then(&i.transform)
            .then(&Matrix3::translation(af.offset.x, af.offset.y)),
    )
}

/// Mitte des Symbolinhalts (Symbolkoordinaten).
fn center(doc: &Document, e: &Element, i: &InstanceElem) -> Option<Point> {
    bounds_with(doc, e, i, Matrix3::IDENTITY).map(|b| b.center())
}

/// Auswahlrahmen einer Animationsinstanz im Seitenstatus `k`: nur das aktuelle Bild. `None` bei
/// anderen Elementen.
pub fn frame_bounds(doc: &Document, e: &Element, k: usize) -> Option<Rect> {
    let (i, frames) = anim(doc, e)?;
    let c = center(doc, e, &i)?;
    let m = frame_matrix(&i, frames, c, k)?;
    bounds_with(doc, e, &i, m)
}

/// Bewegungspfad: Mitte des Bildes in jedem Status (erster = Start, letzter = Ende).
pub fn motion_path(doc: &Document, e: &Element) -> Option<Vec<Point>> {
    let (i, frames) = anim(doc, e)?;
    let c = center(doc, e, &i)?;
    let pts: Vec<Point> = (0..i.anim.frames.min(360) as usize)
        .filter_map(|k| frame_matrix(&i, frames, c, k).map(|m| m.apply(c)))
        .collect();
    (pts.len() >= 2).then_some(pts)
}

/// Zeichnet den Bewegungspfad: Linie in Auswahlfarbe, Zwischenpunkte als kleine Rauten, Start
/// grün mit Ring, Ende rot.
pub fn paint_path(
    painter: &Painter,
    pts: &[Point],
    to_screen: &dyn Fn(Point) -> Pos2,
    line: Color32,
) {
    let s: Vec<Pos2> = pts.iter().map(|p| to_screen(*p)).collect();
    if s.len() < 2 {
        return;
    }
    painter.add(egui::Shape::line(s.clone(), Stroke::new(1.0, line)));
    let diamond = |p: Pos2, r: f32, c: Color32| {
        painter.add(egui::Shape::convex_polygon(
            vec![
                p + egui::vec2(0.0, -r),
                p + egui::vec2(r, 0.0),
                p + egui::vec2(0.0, r),
                p + egui::vec2(-r, 0.0),
            ],
            c,
            Stroke::NONE,
        ));
    };
    for p in &s[1..s.len() - 1] {
        diamond(*p, 2.5, line);
    }
    if let Some(&end) = s.last() {
        diamond(end, 3.0, END);
    }
    painter.circle_filled(s[0], 2.5, START);
    painter.circle_stroke(s[0], 4.0, Stroke::new(1.0, START));
}
