//! Text-Editor auf der Leinwand (Textwerkzeug): Vorschau über fp-render, Caret, Auswahl und
//! Treffertest über das Layout von fp-text (gleicher Satz wie beim Rendern). Die Zeilengeometrie
//! (Klick in den Text, Pos1/Ende, Pfeil auf/ab) wird hier in Zeichenindizes übersetzt und als
//! `ToolInput::TextCaret` an das Werkzeug geschickt; Texteingabe und Bearbeiten erledigt das
//! Werkzeug selbst.

use egui::{pos2, Pos2, Rect, Stroke};
use fp_engine::tools::TextEditView;
use fp_geom::{Matrix3, Point};
use fp_model::{Document, ElementKind};

/// Satz des bearbeiteten Texts.
pub struct TextLayoutCache {
    pub key: u64,
    pub layout: fp_text::TextLayout,
    /// Textraum → Dokument (`MTXv`).
    pub transform: Option<Matrix3>,
    /// Rahmen in Dokumentkoordinaten (vor der Transformation).
    pub frame: fp_geom::Rect,
}

/// Schlüssel einer Editoransicht (Inhalt, Schrift, Lage, Ziel-Generation).
pub fn view_key(v: &TextEditView, generation: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ generation;
    let mut eat = |b: &[u8]| {
        for x in b {
            h ^= *x as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    eat(v.display_text().as_bytes());
    eat(v.font.as_bytes());
    eat(&v.size.to_le_bytes());
    eat(&v.color.0.to_le_bytes());
    eat(&v.anchor.x.to_le_bytes());
    eat(&v.anchor.y.to_le_bytes());
    eat(&v.width.unwrap_or(-1.0).to_le_bytes());
    eat(&v.target.unwrap_or(0).to_le_bytes());
    // Formatierung (Attribute aus dem Inspektor während der Bearbeitung)
    eat(format!("{:?}{:?}", v.spans, v.base).as_bytes());
    h
}

/// Satz für die Editoransicht berechnen (bzw. aus dem Cache).
pub fn ensure_layout<'a>(slot: &'a mut Option<TextLayoutCache>, renderer: &fp_render::Renderer, doc: &Document, v: &TextEditView, generation: u64) -> Option<&'a TextLayoutCache> {
    let key = view_key(v, generation);
    if slot.as_ref().is_none_or(|c| c.key != key) {
        let e = v.preview_element(doc)?;
        let ElementKind::Text(t) = &e.kind else { return None };
        let block = fp_render::convert::text_block(doc, t);
        let layout = fp_text::layout(&block, renderer.fonts(), &fp_text::LayoutOptions::default());
        let transform = t.transform.filter(|m| !m.is_identity());
        let frame = layout.frame;
        *slot = Some(TextLayoutCache { key, layout, transform, frame });
    }
    slot.as_ref()
}

impl TextLayoutCache {
    fn map(&self, x: f64, y: f64) -> Point {
        let p = Point::new(x as f32, y as f32);
        match &self.transform {
            Some(m) => m.apply(p),
            None => p,
        }
    }

    fn unmap(&self, p: Point) -> Point {
        match self.transform.as_ref().and_then(|m| m.inverse()) {
            Some(inv) => inv.apply(p),
            None => p,
        }
    }

    /// Caret als Strecke (oben, unten) in Dokumentkoordinaten.
    pub fn caret(&self, index: usize, fallback: &TextEditView) -> (Point, Point) {
        match self.layout.caret(index) {
            Some(c) => (self.map(c.top[0], c.top[1]), self.map(c.bottom[0], c.bottom[1])),
            None => {
                // leerer Text: Caret am Anker in Schrifthöhe
                let a = fallback.anchor;
                (a, Point::new(a.x, a.y + fallback.size * 1.2))
            }
        }
    }

    /// Auswahlrechtecke als Vierecke in Dokumentkoordinaten.
    pub fn selection_quads(&self, a: usize, b: usize) -> Vec<[Point; 4]> {
        self.layout
            .selection_rects(a, b)
            .into_iter()
            .map(|r| [self.map(r.left as f64, r.top as f64), self.map(r.right as f64, r.top as f64), self.map(r.right as f64, r.bottom as f64), self.map(r.left as f64, r.bottom as f64)])
            .collect()
    }

    /// Rahmen als Viereck in Dokumentkoordinaten.
    pub fn frame_quad(&self) -> [Point; 4] {
        let f = self.frame;
        [self.map(f.left as f64, f.top as f64), self.map(f.right as f64, f.top as f64), self.map(f.right as f64, f.bottom as f64), self.map(f.left as f64, f.bottom as f64)]
    }

    /// Liegt der Dokumentpunkt im Textrahmen (mit Toleranz)?
    pub fn contains(&self, p: Point, tol: f32) -> bool {
        let q = self.unmap(p);
        let f = self.frame.inflate(tol, tol);
        f.contains(q)
    }

    /// Caret-Index am Dokumentpunkt.
    pub fn hit(&self, p: Point) -> usize {
        let q = self.unmap(p);
        self.layout.hit_test(q.x as f64, q.y as f64).char_index
    }

    /// Pos1/Ende: Anfang bzw. Ende der Zeile (vor dem Umbruchzeichen).
    pub fn line_edge(&self, index: usize, end: bool, text: &str) -> usize {
        match self.layout.line_range(index) {
            Some(r) if end => {
                let last = r.end.saturating_sub(1);
                let is_break = text.chars().nth(last).is_some_and(|c| c == '\r' || c == '\n' || c == '\u{3}');
                if is_break && r.end > r.start { last } else { r.end }
            }
            Some(r) => r.start,
            None => {
                if end {
                    text.chars().count()
                } else {
                    0
                }
            }
        }
    }

    /// Pfeil auf/ab.
    pub fn vertical(&self, index: usize, delta: i32) -> usize {
        self.layout.move_vertical(index, delta)
    }
}

/// Zeichnet Rahmen, Auswahl und Caret des Editors.
pub fn paint(painter: &egui::Painter, lc: &TextLayoutCache, v: &TextEditView, time: f64, colors: &crate::theme::Palette, to_screen: &dyn Fn(Point) -> Pos2) {
    // Rahmen (wie CS6: dünner Rahmen um den Textblock, kleine Griffe an den Ecken)
    let fq = lc.frame_quad().map(to_screen);
    let mut pts = fq.to_vec();
    pts.push(fq[0]);
    painter.add(egui::Shape::line(pts, Stroke::new(1.0, colors.selection)));
    for p in fq {
        painter.rect_filled(Rect::from_center_size(p, egui::vec2(5.0, 5.0)), 0.0, colors.handle_border);
        painter.rect_stroke(Rect::from_center_size(p, egui::vec2(5.0, 5.0)), 0.0, Stroke::new(1.0, colors.selection), egui::StrokeKind::Middle);
    }
    if v.width.is_none() {
        // Punkttext: hohler Kreis rechts oben = automatische Breite
        painter.circle_stroke(pos2(fq[1].x + 6.0, fq[1].y), 3.0, Stroke::new(1.0, colors.selection));
    }
    // Auswahl
    let (a, b) = v.selection();
    for q in lc.selection_quads(a, b) {
        let s = q.map(to_screen);
        painter.add(egui::Shape::convex_polygon(s.to_vec(), colors.text_selection, Stroke::NONE));
    }
    // IME-Komposition unterstreichen
    if let Some(pre) = &v.preedit {
        let n = pre.chars().count();
        for q in lc.selection_quads(v.caret, v.caret + n) {
            let s = q.map(to_screen);
            painter.line_segment([s[3], s[2]], Stroke::new(1.5, colors.text_caret));
        }
    }
    // Caret (blinkt im Halbsekundentakt, nicht bei Auswahl)
    if a == b && (time * 2.0).fract() < 0.6 {
        let ci = v.caret + v.preedit.as_ref().map(|p| p.chars().count()).unwrap_or(0);
        let (t, btm) = lc.caret(ci, v);
        painter.line_segment([to_screen(t), to_screen(btm)], Stroke::new(1.0, colors.text_caret));
    }
}

/// Lage des Carets auf dem Bildschirm (für die IME-Position).
pub fn caret_rect(lc: &TextLayoutCache, v: &TextEditView, to_screen: &dyn Fn(Point) -> Pos2) -> Rect {
    let (t, b) = lc.caret(v.caret, v);
    let (a, c) = (to_screen(t), to_screen(b));
    Rect::from_min_max(pos2(a.x.min(c.x), a.y.min(c.y)), pos2(a.x.max(c.x) + 1.0, a.y.max(c.y)))
}
