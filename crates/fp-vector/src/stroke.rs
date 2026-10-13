//! Strich eines Pfads: Stempel-Engine (Spezifikation 07 §6, Referenz `re\vektor\stroke.py`).
//!
//! Ablauf (`@0084e290` → `@0078a7b0` → `@007852f0`):
//! 1. **Wirksame Platzierung** ([`effective_placement`], `@0084df60`): eine offene Kontur → mittig;
//!    alle geschlossen, `BRPi = 1`, `PTTi = 1` und `BDIi ≤ 1` → innen; sonst `BRPi`.
//! 2. **Wirksamer Durchmesser** ([`effective_diameter`]): mittig `D`, innen/außen `2D`, beim
//!    simple-Pinsel ohne AA innen `2D − 1`, außen `2D + 1`.
//! 3. **Läufer**: simple ([`crate::brush::walk_simple`]), natural ([`crate::natural`]), oder
//!    Walker-Modus 2 (simple ohne AA, innen/außen, `@788350`): Pfad hart rastern und auf jedes
//!    gesetzte Pixel mit ungesetztem 4-Nachbarn stempeln.
//! 4. **Stempeln** in einen 8-Bit-Strichpuffer: `sa = mul(Maske, O)`,
//!    `a′ = da + sa − mul(da, sa)`, bei `BFBi = 0` gedeckelt bei `O`. Deckkraft `O` über die
//!    Überlappungstabelle und den Promille-Umweg ([`fp_color::blend::brush_stamp_opacity`]).
//!    `BFBi = 2` zeichnet nichts.
//! 5. **Puffergrenzen** (`@0078ab20`, T5): Vereinigung der Stempelrechtecke
//!    `[(p16 − h)/16, (p16 − h)/16 + w)`, auf ganze Pixel erweitert; darüber hinaus wird abgeschnitten.
//! 6. **Innen/außen-Maske**: innen × Füllmaske des Pfads, außen × (255 − Füllmaske); Kantenglättung
//!    der Maske = Pinsel-AA (0 nur bei simple ohne AA), nicht `FETi`.
//! 7. **Farbe**: Strichfarbe `BCLi`, Puffer-Alpha × Farb-Alpha.

use std::collections::HashMap;

use crate::attrs::{BrushType, Feedback, Stroke, StrokePlacement, VectorPath};
use crate::brush::{shift_tip, tip_mask, walk_simple, Dash, Tip};
use crate::coverage::{fill_coverage, Coverage};
use crate::paint::mul;
use fp_geom::IRect;

/// Wirksame Strichposition (`@0084df60`, B; `PTTi`-Zuordnung A, 8/8 Fälle).
pub fn effective_placement(path: &VectorPath, stroke: &Stroke) -> StrokePlacement {
    if !path.contours.iter().all(|c| c.closed) {
        return StrokePlacement::Center;
    }
    if path.placement == StrokePlacement::Center && path.ptt && stroke.brush.diameter <= 1 {
        return StrokePlacement::Inside;
    }
    path.placement
}

/// Wirksamer Durchmesser `D′` (`@785470`, `@7853e0`, `@786010`).
pub fn effective_diameter(stroke: &Stroke, placement: StrokePlacement) -> f64 {
    let d = stroke.brush.diameter as f64;
    let b = &stroke.brush;
    match placement {
        StrokePlacement::Center => d,
        _ if !b.antialiased && b.kind == BrushType::Simple => {
            if placement == StrokePlacement::Inside {
                2.0 * d - 1.0
            } else {
                2.0 * d + 1.0
            }
        }
        _ => 2.0 * d,
    }
}

/// Ein Stempel-Ereignis: Position in 1/16 px (vor dem Abzug der halben Spitze), Deckkraft, Spitze.
#[derive(Debug, Clone, Copy)]
pub struct Dab {
    pub x16: i64,
    pub y16: i64,
    /// Deckkraft-Byte des Stempels.
    pub opacity: u8,
    /// Index der Spitze in der Spitzenliste (Größen-/Winkelstufe).
    pub tip: usize,
}

/// 8-Bit-Strichpuffer über einem Fenster mit Grenzen der Stempelrechtecke.
pub(crate) struct InkBuffer<'a> {
    pub rect: IRect,
    pub w: usize,
    pub h: usize,
    pub data: Vec<u8>,
    /// Vereinigung der Stempelrechtecke (links, oben, rechts, unten) in Dokumentpixeln.
    pub bounds: Option<(i64, i64, i64, i64)>,
    /// Strichtextur (Nachschlag, `BTBi`, `BTEi`), wirkt je Stempelpixel auf den Maskenwert.
    pub tex: Option<(crate::natural::TexLookup<'a>, u32, u32)>,
}

impl InkBuffer<'_> {
    pub fn new(rect: IRect) -> Self {
        let (w, h) = (rect.width() as usize, rect.height() as usize);
        Self { rect, w, h, data: vec![0; w * h], bounds: None, tex: None }
    }

    /// Stempelrechteck `[(p16)/16, p16/16 + tipw)` auf ganze Pixel erweitert vormerken.
    pub fn note_bounds(&mut self, px16: i64, py16: i64, tipw: f64) {
        let fx0 = px16 as f64 / 16.0;
        let fy0 = py16 as f64 / 16.0;
        let b = (fx0.floor() as i64, fy0.floor() as i64, (fx0 + tipw).ceil() as i64, (fy0 + tipw).ceil() as i64);
        self.bounds = Some(match self.bounds {
            None => b,
            Some(o) => (o.0.min(b.0), o.1.min(b.1), o.2.max(b.2), o.3.max(b.3)),
        });
    }

    /// Maske an `(x, y)` (Dokument) mit Deckkraft `o` stempeln.
    #[inline]
    pub fn stamp(&mut self, x: i64, y: i64, m: &Tip, o: u8, cap: bool) {
        let (x0, y0) = (self.rect.left as i64, self.rect.top as i64);
        // Stempel ganz außerhalb des Fensters: nichts zu tun (Teilfenster der Live-Vorschau)
        if x >= x0 + self.w as i64 || y >= y0 + self.h as i64 || x + m.n as i64 <= x0 || y + m.n as i64 <= y0 {
            return;
        }
        for j in 0..m.n {
            let yy = y + j as i64 - y0;
            if yy < 0 || yy >= self.h as i64 {
                continue;
            }
            let row = yy as usize * self.w;
            for i in 0..m.n {
                let mut v = m.data[j * m.n + i];
                if v == 0 {
                    continue;
                }
                let xx = x + i as i64 - x0;
                if xx < 0 || xx >= self.w as i64 {
                    continue;
                }
                if let Some((t, btb, bte)) = &self.tex {
                    v = crate::natural::textured_mask(v, t.value((x0 + xx) as i32, (y0 + yy) as i32), *btb, *bte);
                }
                let k = row + xx as usize;
                let sa = mul(v, o);
                let da = self.data[k];
                let a2 = da as u32 + sa as u32 - mul(da, sa) as u32;
                self.data[k] = if cap { a2.min(o as u32) as u8 } else { a2.min(255) as u8 };
            }
        }
    }

    /// Alles außerhalb der Stempelrechtecke löschen (`STAMP_BOUNDS`, T5).
    pub fn clip_to_bounds(&mut self) {
        let Some((bx0, by0, bx1, by1)) = self.bounds else { return };
        let (x0, y0) = (self.rect.left as i64, self.rect.top as i64);
        for j in 0..self.h {
            let yy = y0 + j as i64;
            let row_out = yy < by0 || yy >= by1;
            for i in 0..self.w {
                let xx = x0 + i as i64;
                if row_out || xx < bx0 || xx >= bx1 {
                    self.data[j * self.w + i] = 0;
                }
            }
        }
    }
}

/// Strichebene als RGBA über `rect` (Referenz `stroke_layer`). Natürliche Pinsel laufen über
/// [`crate::natural`]; `None` bedeutet „keine Pixel“.
pub fn stroke_layer(path: &VectorPath, stroke: &Stroke, rect: IRect) -> Vec<[u8; 4]> {
    let (w, h) = (rect.width() as usize, rect.height() as usize);
    let mut out = vec![[0u8; 4]; w * h];
    let b = &stroke.brush;
    if b.feedback == Feedback::Background || w == 0 || h == 0 {
        return out;
    }
    let placement = effective_placement(path, stroke);
    let natural = b.kind == BrushType::Natural;
    let ink = if natural {
        crate::natural::natural_ink(path, stroke, placement, rect)
    } else {
        simple_ink(path, stroke, placement, rect)
    };
    let mut buf = ink.data;
    if placement != StrokePlacement::Center {
        let polys = path.polys();
        let fc = fill_coverage(&polys, rect, path.even_odd, path.agm, natural || b.antialiased);
        for (v, c) in buf.iter_mut().zip(&fc.data) {
            if *v != 0 {
                let m = if placement == StrokePlacement::Inside { *c } else { 255 - *c };
                *v = mul(*v, m);
            }
        }
    }
    if b.alpha_remap != 0 {
        // alphaRemap im Endcomposite (`@006e96e0`, Flag 0x1c), nach der Innen/außen-Maske
        // (Laufzeit T2 3D Glow/Toxic Waste: Kreis innen)
        let table = crate::natural::alpha_remap_table(b.alpha_remap, b.concentration);
        for v in buf.iter_mut() {
            *v = table[*v as usize];
        }
    }
    let col = stroke.color;
    let ca = col.a();
    for (k, (o, &a)) in out.iter_mut().zip(&buf).enumerate() {
        if a != 0 {
            let a = if ca != 255 { mul(a, ca) } else { a };
            let rgb = match &ink.rgb {
                Some(v) => v[k],
                None => [col.r(), col.g(), col.b()],
            };
            *o = [rgb[0], rgb[1], rgb[2], a];
        }
    }
    out
}

/// Strichpuffer der simple-Pinsel (`@7876b0`, `@788350`, `@788ae0`).
pub(crate) fn simple_ink(path: &VectorPath, stroke: &Stroke, placement: StrokePlacement, rect: IRect) -> InkBufferOut {
    let b = &stroke.brush;
    let aa = b.antialiased;
    let dp = effective_diameter(stroke, placement);
    let tip = tip_mask(dp, b.shape == crate::attrs::TipShape::Circle, b.softness, b.bell_curve);
    let n_tip = tip.n as i64 - 1;
    let o = fp_color::blend::brush_stamp_opacity(b.concentration, feedback_code(b.feedback), b.spacing, dp);
    let cap = b.feedback != Feedback::Brush;
    let mut ink = InkBuffer::new(rect);
    let tex = stroke
        .texture
        .as_ref()
        .filter(|_| b.texture_blend > 0 || b.texture_edge > 0)
        .map(|t| crate::natural::TexLookup::new(&t.mask, path.texture_offset));
    ink.tex = tex.map(|t| (t, b.texture_blend, b.texture_edge));
    let walks = simple_walks(path, stroke, placement, rect, n_tip);
    let tipw = (tip.n - 1) as f64;
    let mut cache: HashMap<(usize, usize), Tip> = HashMap::new();
    let half = (n_tip as f64 * 0.5 * 16.0) as i64;
    for walk in &walks {
        for &(px, py) in walk {
            let (mut px16, mut py16) = (px - half, py - half);
            ink.note_bounds(px16, py16, tipw);
            if !aa {
                if n_tip % 2 == 1 {
                    px16 += 8;
                    py16 += 8;
                }
                let x = (px16 as f64 / 16.0 + 0.5).floor() as i64;
                let y = (py16 as f64 / 16.0 + 0.5).floor() as i64;
                ink.stamp(x, y, &tip, o, cap);
            } else {
                let (x, y) = (px16 >> 4, py16 >> 4);
                let key = ((px16 & 15) as usize, (py16 & 15) as usize);
                let m = cache.entry(key).or_insert_with(|| shift_tip(&tip, key.0, key.1));
                ink.stamp(x, y, m, o, cap);
            }
        }
    }
    ink.clip_to_bounds();
    InkBufferOut { data: ink.data, rgb: None }
}

/// Ergebnis eines Läufers: 8-Bit-Strichpuffer über dem Fenster, bei natürlichen Pinseln mit
/// Farbe je Pixel (Farbjitter, Mehrfachspitzen).
pub(crate) struct InkBufferOut {
    pub data: Vec<u8>,
    pub rgb: Option<Vec<[u8; 3]>>,
}

/// `BFBi` als Zahl.
pub(crate) fn feedback_code(f: Feedback) -> u32 {
    match f {
        Feedback::None => 0,
        Feedback::Brush => 1,
        Feedback::Background => 2,
    }
}

/// Stempelpositionen der simple-Pinsel je Teilpfad bzw. Walker-Modus 2.
pub(crate) fn simple_walks(path: &VectorPath, stroke: &Stroke, placement: StrokePlacement, rect: IRect, n_tip: i64) -> Vec<Vec<(i64, i64)>> {
    let b = &stroke.brush;
    let mut walks = Vec::new();
    if !b.antialiased && b.kind == BrushType::Simple && placement != StrokePlacement::Center {
        walks.push(edge_walk(path, rect, n_tip));
    } else {
        for c in &path.contours {
            let mut dash = Dash::new(b.diameter, &b.dashes);
            walks.push(walk_simple(&c.points, c.closed, n_tip, b.antialiased, dash.as_mut()));
        }
    }
    walks
}

/// Walker-Modus 2 (`@788350`): Pfad hart rastern (Fenster um `N + 2` erweitert), Stempel auf jedes
/// gesetzte Pixel mit ungesetztem 4-Nachbarn (oder am Fensterrand), Position `(x·16, y·16)`.
pub(crate) fn edge_walk(path: &VectorPath, rect: IRect, n_tip: i64) -> Vec<(i64, i64)> {
    let mg = (n_tip + 2).clamp(0, 1 << 20) as i32;
    let big = rect.inflate(mg, mg);
    let polys = path.polys();
    let hm: Coverage = fill_coverage(&polys, big, path.even_odd, path.agm, false);
    let (wx, hx) = (hm.width(), hm.height());
    let mut out = Vec::new();
    for j in 0..hx {
        for i in 0..wx {
            if hm.data[j * wx + i] == 0 {
                continue;
            }
            let edge = i == 0
                || j == 0
                || i == wx - 1
                || j == hx - 1
                || hm.data[j * wx + i - 1] == 0
                || hm.data[j * wx + i + 1] == 0
                || hm.data[(j - 1) * wx + i] == 0
                || hm.data[(j + 1) * wx + i] == 0;
            if edge {
                out.push(((big.left as i64 + i as i64) * 16, (big.top as i64 + j as i64) * 16));
            }
        }
    }
    out
}
