//! Skalieren und Exportbereich der Bildvorschau (re\17-bildvorschau.md §3, §6 „Exportbereich“).
//!
//! - Gespeichert wird wie beim Export: `crop` = (links, oben, rechts, unten) in **Dokumentpixeln**,
//!   `scale` = Prozent bzw. feste Zielgröße für das ganze Bild ([`fp_model::ExportScale`]).
//! - Angezeigt werden B/H der Skalierung und X/Y/B/H des Exportbereichs in **Ausgabepixeln nach
//!   der Skalierung** (§3: nach 50 % zeigt der Exportbereich 240 × 160). Die Größen kommen aus
//!   [`fp_export::doc_export::crop_scale_size`], also genau aus der Rechnung des Exports.
//! - Werkzeug „Exportbereich“: gestrichelter Rahmen mit 8 Griffen im aktiven Feld; Ziehen an
//!   Griffen oder Kanten setzt den Exportbereich **dieses** Feldes, Ziehen im Inneren verschiebt ihn.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use fp_export::doc_export::crop_scale_size;
use fp_export::options::OptimizeSettings;
use fp_model::ExportScale;

/// Kleinste Prozentangabe bzw. größte (Feld „%“).
pub const PERCENT_RANGE: (f64, f64) = (1.0, 1000.0);
/// Größte Zielgröße in Pixeln (Felder B/H).
pub const MAX_SIZE: f64 = 30000.0;

fn doc_wh(doc: (usize, usize)) -> (usize, usize) {
    (doc.0.max(1), doc.1.max(1))
}

/// Skalierung ohne Exportbereich gerechnet (Größe des ganzen Bildes nach dem Skalieren).
fn without_crop(s: &OptimizeSettings) -> OptimizeSettings {
    OptimizeSettings {
        crop: None,
        palette: Vec::new(),
        ..s.clone()
    }
}

/// Faktoren Dokumentpixel → Ausgabepixel (x, y).
pub fn factors(doc: (usize, usize), s: &OptimizeSettings) -> (f64, f64) {
    let (w, h) = doc_wh(doc);
    match s.scale {
        Some(sc) if sc.use_scale && sc.percent > 0 && sc.percent != 100 => {
            (sc.percent as f64 / 100.0, sc.percent as f64 / 100.0)
        }
        Some(sc) if sc.use_scale && sc.width > 0 && sc.height > 0 => {
            (sc.width as f64 / w as f64, sc.height as f64 / h as f64)
        }
        _ => (1.0, 1.0),
    }
}

/// Größe des ganzen Bildes nach dem Skalieren (Felder B/H der Gruppe „Skalieren“).
pub fn scaled_size(doc: (usize, usize), s: &OptimizeSettings) -> (usize, usize) {
    let (w, h) = doc_wh(doc);
    crop_scale_size(w, h, &without_crop(s))
}

/// Ausgabegröße mit Exportbereich und Skalierung (= Größe der exportierten Datei in Pixeln).
pub fn output_size(doc: (usize, usize), s: &OptimizeSettings) -> (usize, usize) {
    let (w, h) = doc_wh(doc);
    crop_scale_size(w, h, s)
}

/// Wert des Feldes „%“: Prozent bzw. bei fester Zielgröße das Verhältnis der Breite.
pub fn percent_shown(doc: (usize, usize), s: &OptimizeSettings) -> f64 {
    match s.scale {
        Some(sc) if sc.use_scale && sc.percent > 0 && sc.percent != 100 => sc.percent as f64,
        Some(sc) if sc.use_scale && sc.width > 0 && sc.height > 0 => {
            (sc.width as f64 * 100.0 / doc_wh(doc).0 as f64).round()
        }
        _ => 100.0,
    }
}

/// „Proportionen beibehalten“ (Standard an).
pub fn constrain(s: &OptimizeSettings) -> bool {
    s.scale.is_none_or(|sc| sc.constrain)
}

/// Feld „%“ geändert: B/H folgen sofort (§3: 50 % ⇒ 240/160).
pub fn set_percent(s: &mut OptimizeSettings, doc: (usize, usize), p: f64) {
    let (w, h) = doc_wh(doc);
    let p = p.round().clamp(PERCENT_RANGE.0, PERCENT_RANGE.1) as i32;
    let c = constrain(s);
    let pw = (w as i64 * p as i64 / 100).max(1) as i32;
    let ph = (h as i64 * p as i64 / 100).max(1) as i32;
    s.scale = Some(ExportScale {
        use_scale: p != 100,
        constrain: c,
        percent: p,
        width: pw,
        height: ph,
    });
}

/// Feld „B“ bzw. „H“ geändert (`horizontal` = B). Mit „Proportionen beibehalten“ folgt die andere
/// Seite, „%“ folgt aus der Breite. Gespeichert als feste Zielgröße für das ganze Bild.
pub fn set_size(s: &mut OptimizeSettings, doc: (usize, usize), horizontal: bool, v: f64) {
    let (w, h) = doc_wh(doc);
    let v = v.round().clamp(1.0, MAX_SIZE) as i64;
    let c = constrain(s);
    let (cur_w, cur_h) = scaled_size(doc, s);
    let (nw, nh) = match (horizontal, c) {
        (true, true) => (v, ((v * h as i64 * 2 + w as i64) / (2 * w as i64)).max(1)),
        (false, true) => (((v * w as i64 * 2 + h as i64) / (2 * h as i64)).max(1), v),
        (true, false) => (v, cur_h as i64),
        (false, false) => (cur_w as i64, v),
    };
    let same = nw == w as i64 && nh == h as i64;
    s.scale = Some(ExportScale {
        use_scale: !same,
        constrain: c,
        percent: 100,
        width: nw as i32,
        height: nh as i32,
    });
}

/// Kästchen „Proportionen beibehalten“.
pub fn set_constrain(s: &mut OptimizeSettings, doc: (usize, usize), on: bool) {
    let (w, h) = scaled_size(doc, s);
    let mut sc = s.scale.unwrap_or(ExportScale {
        use_scale: false,
        constrain: true,
        percent: 100,
        width: w as i32,
        height: h as i32,
    });
    sc.constrain = on;
    s.scale = Some(sc);
    if on {
        // Kopplung wieder herstellen: H folgt aus B
        set_size(s, doc, true, w as f64);
    }
}

/// Exportbereich in Dokumentpixeln (ohne Haken: das ganze Bild).
pub fn area_doc(doc: (usize, usize), s: &OptimizeSettings) -> [i32; 4] {
    let (w, h) = doc_wh(doc);
    let r = fp_export::doc_export::crop_rect(w, h, s);
    [r.left, r.top, r.right, r.bottom]
}

/// X, Y, B, H des Exportbereichs in Ausgabepixeln (Felder 4658/4660/4619/4623).
pub fn area_shown(doc: (usize, usize), s: &OptimizeSettings) -> [i64; 4] {
    let [l, t, ..] = area_doc(doc, s);
    let (fx, fy) = factors(doc, s);
    let (ow, oh) = if s.crop.is_some() {
        output_size(doc, s)
    } else {
        scaled_size(doc, s)
    };
    [
        (l as f64 * fx).round() as i64,
        (t as f64 * fy).round() as i64,
        ow as i64,
        oh as i64,
    ]
}

/// Kästchen „Exportbereich“: an = ganzes Bild als Anfangsbereich, aus = kein Zuschnitt.
pub fn set_area_enabled(s: &mut OptimizeSettings, doc: (usize, usize), on: bool) {
    let (w, h) = doc_wh(doc);
    s.crop = on.then_some([0, 0, w as i32, h as i32]);
}

/// Ein Feld des Exportbereichs geändert (`k` 0 = X, 1 = Y, 2 = B, 3 = H; Wert in Ausgabepixeln).
/// X/Y verschieben den Bereich (Größe bleibt), B/H ändern die Größe; alles bleibt im Bild.
pub fn set_area_value(s: &mut OptimizeSettings, doc: (usize, usize), k: usize, v: f64) {
    let (w, h) = doc_wh(doc);
    let (fx, fy) = factors(doc, s);
    let [l, t, r, b] = area_doc(doc, s);
    let (aw, ah) = (r - l, b - t);
    let to_doc = |v: f64, f: f64| (v / f).round() as i32;
    let rect = match k {
        0 => {
            let nl = to_doc(v, fx).clamp(0, w as i32 - aw);
            [nl, t, nl + aw, b]
        }
        1 => {
            let nt = to_doc(v, fy).clamp(0, h as i32 - ah);
            [l, nt, r, nt + ah]
        }
        2 => [l, t, (l + to_doc(v, fx).max(1)).min(w as i32), b],
        _ => [l, t, r, (t + to_doc(v, fy).max(1)).min(h as i32)],
    };
    s.crop = Some(rect);
}

// ---------------------------------------------------------------------- Werkzeug

/// Was beim Ziehen gegriffen wurde.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// Innenfläche: verschieben.
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

impl Handle {
    fn edges(self) -> (i8, i8) {
        match self {
            Handle::Move => (0, 0),
            Handle::N => (0, -1),
            Handle::S => (0, 1),
            Handle::E => (1, 0),
            Handle::W => (-1, 0),
            Handle::NE => (1, -1),
            Handle::NW => (-1, -1),
            Handle::SE => (1, 1),
            Handle::SW => (-1, 1),
        }
    }

    /// Mauszeiger über dem Griff.
    pub fn cursor(self) -> egui::CursorIcon {
        match self {
            Handle::Move => egui::CursorIcon::Move,
            Handle::N | Handle::S => egui::CursorIcon::ResizeVertical,
            Handle::E | Handle::W => egui::CursorIcon::ResizeHorizontal,
            Handle::NE | Handle::SW => egui::CursorIcon::ResizeNeSw,
            Handle::NW | Handle::SE => egui::CursorIcon::ResizeNwSe,
        }
    }
}

/// Größe eines Griffs (Bildschirmpunkte) und Fangabstand an den Kanten.
pub const HANDLE: f32 = 6.0;
const GRAB: f32 = 5.0;

/// Mitten der 8 Griffe eines Rahmens.
pub fn handle_points(r: Rect) -> [(Handle, Pos2); 8] {
    let (l, rr, t, b, cx, cy) = (
        r.left(),
        r.right(),
        r.top(),
        r.bottom(),
        r.center().x,
        r.center().y,
    );
    [
        (Handle::NW, pos2(l, t)),
        (Handle::N, pos2(cx, t)),
        (Handle::NE, pos2(rr, t)),
        (Handle::E, pos2(rr, cy)),
        (Handle::SE, pos2(rr, b)),
        (Handle::S, pos2(cx, b)),
        (Handle::SW, pos2(l, b)),
        (Handle::W, pos2(l, cy)),
    ]
}

/// Griff, Kante bzw. Innenfläche unter `p` (Bildschirmrahmen `r`).
pub fn hit(r: Rect, p: Pos2) -> Option<Handle> {
    for (h, c) in handle_points(r) {
        if (p - c).abs().max_elem() <= HANDLE / 2.0 + 2.0 {
            return Some(h);
        }
    }
    let near_x = |x: f32| (p.x - x).abs() <= GRAB;
    let near_y = |y: f32| (p.y - y).abs() <= GRAB;
    let in_x = p.x >= r.left() - GRAB && p.x <= r.right() + GRAB;
    let in_y = p.y >= r.top() - GRAB && p.y <= r.bottom() + GRAB;
    if in_y && near_x(r.left()) {
        return Some(Handle::W);
    }
    if in_y && near_x(r.right()) {
        return Some(Handle::E);
    }
    if in_x && near_y(r.top()) {
        return Some(Handle::N);
    }
    if in_x && near_y(r.bottom()) {
        return Some(Handle::S);
    }
    r.contains(p).then_some(Handle::Move)
}

/// Neuer Exportbereich nach dem Ziehen von `h` um (`dx`, `dy`) Dokumentpixel, ausgehend von
/// `start`. Bleibt im Bild und mindestens 1 × 1 groß; Kanten über die Gegenkante hinaus kehren um.
pub fn drag(start: [i32; 4], h: Handle, dx: f64, dy: f64, doc: (usize, usize)) -> [i32; 4] {
    let (w, hh) = (doc.0.max(1) as i32, doc.1.max(1) as i32);
    let (dx, dy) = (dx.round() as i32, dy.round() as i32);
    let [l, t, r, b] = start;
    if h == Handle::Move {
        let nx = (l + dx).clamp(0, w - (r - l));
        let ny = (t + dy).clamp(0, hh - (b - t));
        return [nx, ny, nx + (r - l), ny + (b - t)];
    }
    let (ex, ey) = h.edges();
    let (mut l, mut t, mut r, mut b) = (l, t, r, b);
    match ex {
        -1 => l = (l + dx).clamp(0, w),
        1 => r = (r + dx).clamp(0, w),
        _ => {}
    }
    match ey {
        -1 => t = (t + dy).clamp(0, hh),
        1 => b = (b + dy).clamp(0, hh),
        _ => {}
    }
    let (l, r) = (l.min(r), l.max(r));
    let (t, b) = (t.min(b), t.max(b));
    let (r, b) = (r.max(l + 1).min(w), b.max(t + 1).min(hh));
    let (l, t) = (l.min(r - 1), t.min(b - 1));
    [l, t, r, b]
}

/// Bildschirmrahmen des Exportbereichs: `img` = Lage des ganzen (nicht zugeschnittenen) Bildes auf
/// dem Bildschirm, `zoom` Bildschirmpunkte je Ausgabepixel.
pub fn screen_rect(
    area: [i32; 4],
    img: Rect,
    doc: (usize, usize),
    s: &OptimizeSettings,
    zoom: f32,
) -> Rect {
    let (fx, fy) = factors(doc, s);
    let p = |x: i32, y: i32| {
        img.min + vec2((x as f64 * fx) as f32 * zoom, (y as f64 * fy) as f32 * zoom)
    };
    Rect::from_min_max(p(area[0], area[1]), p(area[2], area[3]))
}

/// Gestrichelter Rahmen mit 8 Griffen (§6); außerhalb abgedunkelt.
pub fn paint(ui: &Ui, clip: Rect, img: Rect, r: Rect) {
    let p = ui.painter().with_clip_rect(clip.intersect(ui.clip_rect()));
    let shade = Color32::from_black_alpha(70);
    let outside = [
        Rect::from_min_max(img.min, pos2(img.right(), r.top())),
        Rect::from_min_max(pos2(img.left(), r.bottom()), img.max),
        Rect::from_min_max(pos2(img.left(), r.top()), pos2(r.left(), r.bottom())),
        Rect::from_min_max(pos2(r.right(), r.top()), pos2(img.right(), r.bottom())),
    ];
    for o in outside {
        if o.is_positive() {
            p.rect_filled(o, 0.0, shade);
        }
    }
    let rr = r.shrink(0.5);
    let pts = [
        rr.left_top(),
        rr.right_top(),
        rr.right_bottom(),
        rr.left_bottom(),
        rr.left_top(),
    ];
    p.add(egui::Shape::line(
        pts.to_vec(),
        Stroke::new(1.0, Color32::WHITE),
    ));
    p.extend(egui::Shape::dashed_line(
        &pts,
        Stroke::new(1.0, Color32::BLACK),
        4.0,
        4.0,
    ));
    for (_, c) in handle_points(r) {
        let hr = Rect::from_center_size(c, vec2(HANDLE, HANDLE));
        p.rect_filled(hr, 0.0, Color32::WHITE);
        p.rect_stroke(
            hr,
            0.0,
            Stroke::new(1.0, Color32::BLACK),
            StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_sets_width_and_height_like_the_export() {
        let mut s = OptimizeSettings::default();
        set_percent(&mut s, (480, 320), 50.0);
        assert_eq!(scaled_size((480, 320), &s), (240, 160));
        assert_eq!(percent_shown((480, 320), &s), 50.0);
        set_size(&mut s, (480, 320), true, 120.0);
        assert_eq!(scaled_size((480, 320), &s), (120, 80));
        assert_eq!(percent_shown((480, 320), &s), 25.0);
        set_constrain(&mut s, (480, 320), false);
        set_size(&mut s, (480, 320), false, 50.0);
        assert_eq!(scaled_size((480, 320), &s), (120, 50));
    }

    #[test]
    fn export_area_is_shown_in_output_pixels() {
        let mut s = OptimizeSettings::default();
        set_percent(&mut s, (480, 320), 50.0);
        assert_eq!(area_shown((480, 320), &s), [0, 0, 240, 160]);
        set_area_enabled(&mut s, (480, 320), true);
        set_area_value(&mut s, (480, 320), 2, 100.0);
        set_area_value(&mut s, (480, 320), 0, 20.0);
        assert_eq!(s.crop, Some([40, 0, 240, 320]));
        assert_eq!(area_shown((480, 320), &s), [20, 0, 100, 160]);
        assert_eq!(output_size((480, 320), &s), (100, 160));
    }

    #[test]
    fn dragging_handles_stays_inside_the_image() {
        let d = (100, 80);
        assert_eq!(
            drag([0, 0, 100, 80], Handle::SE, -30.0, -20.0, d),
            [0, 0, 70, 60]
        );
        assert_eq!(
            drag([0, 0, 100, 80], Handle::NW, 10.0, 5.0, d),
            [10, 5, 100, 80]
        );
        assert_eq!(
            drag([10, 10, 50, 50], Handle::Move, 100.0, -100.0, d),
            [60, 0, 100, 40]
        );
        assert_eq!(
            drag([10, 10, 50, 50], Handle::E, -60.0, 0.0, d),
            [0, 10, 10, 50]
        );
        let r = Rect::from_min_max(pos2(10.0, 10.0), pos2(110.0, 90.0));
        assert_eq!(hit(r, pos2(110.0, 90.0)), Some(Handle::SE));
        assert_eq!(hit(r, pos2(60.0, 11.0)), Some(Handle::N));
        assert_eq!(hit(r, pos2(60.0, 50.0)), Some(Handle::Move));
        assert_eq!(hit(r, pos2(200.0, 50.0)), None);
    }
}
