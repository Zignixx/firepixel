//! Gemeinsame Test-Helfer: Original-Tag-Baum (JSON-Form von `fwfmt.tree.to_json`) → Eingabe-
//! Strukturen von fp-vector. Diese Zuordnung ist zugleich die Vorlage für fp-render.
#![allow(dead_code)]

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::{BezierContour, BezierNode, IRect, Point, Vec2};
use fp_raster::{Mask, RgbaImage};
use fp_vector::*;
use serde_json::Value;

/// Liste von Ein-Schlüssel-Objekten `{"TAGt": wert}`.
#[derive(Clone, Copy)]
pub struct Tags<'a>(pub &'a [Value]);

impl<'a> Tags<'a> {
    pub fn from(v: &'a Value) -> Tags<'a> {
        Tags(v.as_array().map(|a| a.as_slice()).unwrap_or(&[]))
    }

    fn find(&self, tag: &str) -> Option<&'a Value> {
        self.0.iter().find_map(|o| {
            let m = o.as_object()?;
            let (k, v) = m.iter().next()?;
            if k.len() >= 3 && &k[..3] == tag { Some(v) } else { None }
        })
    }

    pub fn all(&self, tag: &str) -> Vec<Tags<'a>> {
        self.0
            .iter()
            .filter_map(|o| {
                let m = o.as_object()?;
                let (k, v) = m.iter().next()?;
                if k.len() >= 3 && &k[..3] == tag { Some(Tags::from(v)) } else { None }
            })
            .collect()
    }

    pub fn child(&self, tag: &str) -> Option<Tags<'a>> {
        self.find(tag).map(Tags::from)
    }

    pub fn f(&self, tag: &str, d: f64) -> f64 {
        self.find(tag).and_then(|v| v.as_f64()).unwrap_or(d)
    }

    pub fn u(&self, tag: &str, d: u32) -> u32 {
        self.find(tag).and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(d)
    }

    pub fn has(&self, tag: &str) -> bool {
        self.find(tag).is_some()
    }

    pub fn b(&self, tag: &str, d: bool) -> bool {
        match self.find(tag) {
            Some(Value::Bool(b)) => *b,
            Some(v) => v.as_u64().map(|x| x != 0).unwrap_or(d),
            None => d,
        }
    }
}

pub fn b64(s: &str) -> Vec<u8> {
    let mut map = [255u8; 256];
    for (i, c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
        map[*c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for &c in s.as_bytes() {
        let v = map[c as usize];
        if v == 255 {
            continue;
        }
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

/// Gebrauchsfertige Zusatzdaten (aufgelöste Textur-/Musterbilder).
#[derive(Default, Clone)]
pub struct Extra {
    pub texture: Option<Arc<Mask>>,
    pub pattern: Option<Arc<RgbaImage>>,
    pub stroke_texture: Option<Arc<Mask>>,
}

fn stops(t: Option<Tags>) -> Vec<GradientStop> {
    let Some(t) = t else { return Vec::new() };
    t.all("FGI").iter().map(|s| GradientStop { pos: s.f("FGP", 0.0), color: Argb(s.u("FGC", 0)) }).collect()
}

/// Bézier-Konturen (`PBLv`), optional mit `BZCi` je Knoten.
pub fn bezier_contours(pth: Tags) -> Vec<(BezierContour, Vec<u32>)> {
    let mut out = Vec::new();
    let Some(pbl) = pth.child("PBL") else { return out };
    for pbp in pbl.all("PBP") {
        let mut nodes = Vec::new();
        let mut counts = Vec::new();
        for t in pbp.all("PBT") {
            let (x, y) = (t.f("XLC", 0.0) as f32, t.f("YLC", 0.0) as f32);
            let pi = Point::new(t.f("XPC", x as f64) as f32, t.f("YPC", y as f64) as f32);
            let po = Point::new(t.f("XSC", x as f64) as f32, t.f("YSC", y as f64) as f32);
            nodes.push(BezierNode::new(Point::new(x, y), pi, po));
            counts.push(t.child("BZL").map(|b| b.u("BZC", 1)).unwrap_or(1));
        }
        if !nodes.is_empty() {
            out.push((BezierContour::new(nodes, pbp.b("ISC", false)), counts));
        }
    }
    out
}

/// Polylinien aus `PCLv` (Werte als `f64` wie die Referenz) bzw. Original-Abflachung aus `PBLv`.
pub fn contours(pth: Tags, as_f32: bool) -> Vec<PathContour> {
    let cv = |v: f64| if as_f32 { v as f32 as f64 } else { v };
    if let Some(pcl) = pth.child("PCL") {
        let ppls = pcl.all("PPL");
        if !ppls.is_empty() {
            let mut out = Vec::new();
            for ppl in ppls {
                let mut c = PathContour::new(Vec::new(), ppl.b("ISC", false));
                for q in ppl.all("PPT") {
                    c.points.push(Vec2::new(cv(q.f("XLC", 0.0)), cv(q.f("YLC", 0.0))));
                    c.dynamics.push(PointDynamics { pressure: q.f("PRS", 1.0) as f32, velocity: q.f("VEL", 0.0) as f32, seed: q.u("RND", 0) });
                }
                if !c.points.is_empty() {
                    out.push(c);
                }
            }
            return out;
        }
    }
    bezier_contours(pth).iter().map(|(c, n)| PathContour::from_bezier(c, Some(n))).collect()
}

const SRC: &[u8] = b"PSHVR";
const TGT: &[u8] = b"ZAOBRHLS";

pub fn brush(b: Tags) -> BrushParams {
    let mut p = BrushParams {
        diameter: b.u("BDI", 1),
        angle: b.u("BAN", 0),
        min_size: b.u("BMS", 0),
        aspect: b.u("BAS", 1000),
        max_count: b.u("BMM", 0),
        softness: b.u("BSE", 0),
        shape: if b.u("BSH", 1) == 1 { TipShape::Circle } else { TipShape::Square },
        bell_curve: b.u("BSF", 1) == 1,
        kind: if b.u("BRT", 1) == 0 { BrushType::Natural } else { BrushType::Simple },
        feedback: Feedback::from_u32(b.u("BFB", 0)),
        tip_spacing_mode: match b.u("BSM", 0) {
            1 => TipSpacingMode::Diagonal,
            2 => TipSpacingMode::Circular,
            _ => TipSpacingMode::Random,
        },
        tip_coloring_mode: match b.u("BCM", 0) {
            1 => TipColoringMode::Uniform,
            2 => TipColoringMode::Complementary,
            3 => TipColoringMode::Hue,
            4 => TipColoringMode::Shadow,
            _ => TipColoringMode::Random,
        },
        alpha_remap: b.u("BEF", 0),
        spacing: b.u("BSP", 100),
        flow_rate: b.u("BFR", 0),
        tip_count: b.u("BNT", 1),
        tip_spacing: b.u("BTS", 0),
        blackness: b.u("BBK", 0),
        concentration: b.u("BCN", 1000),
        texture_blend: b.u("BTB", 0),
        texture_edge: b.u("BTE", 0),
        antialiased: b.b("BIA", false),
        dashes: Vec::new(),
        sense: [[0; 5]; 8],
    };
    let ndi = b.u("NDI", 0).min(3);
    for k in 1..=ndi {
        p.dashes.push((b.u(&format!("DO{k}"), 0), b.u(&format!("DF{k}"), 0)));
    }
    for (si, s) in SRC.iter().enumerate() {
        for (ti, t) in TGT.iter().enumerate() {
            let tag = format!("S{}{}", *s as char, *t as char);
            let v = b.u(&tag, 0);
            if v != 0 {
                p.sense[ti][si] = BrushParams::sense_from_permille(v);
            }
        }
    }
    p
}

/// `PTHv` (JSON-Liste) → [`VectorPath`].
pub fn vector_path(pth: Tags, extra: &Extra, as_f32: bool) -> VectorPath {
    let mut p = VectorPath::new(contours(pth, as_f32));
    p.even_odd = pth.b("EOF", true);
    p.agm = pth.b("AGM", false);
    p.fill_over_stroke = pth.b("FOT", false);
    p.placement = StrokePlacement::from_u32(pth.u("BRP", 1));
    p.ptt = pth.u("PTT", 0) == 1;
    p.random_seed = pth.u("RND", 0);
    p.texture_offset = Vec2::new(pth.f("TOX", 0.0), pth.f("TOY", 0.0));
    let handles = GradientHandles {
        p1: Vec2::new(pth.f("PSX", 0.0), pth.f("PSY", 0.0)),
        p2: Vec2::new(pth.f("PEX", 0.0), pth.f("PEY", 0.0)),
        p3: Vec2::new(pth.f("PFX", 0.0), pth.f("PFY", 0.0)),
    };
    let Some(pat) = pth.child("PAT") else { return p };
    if let Some(fpl) = pat.child("FPL") {
        let sh = fpl.u("FSH", 0);
        let sx = fpl.u("FSX", sh);
        let kind = if sx == 17 || GradientShape::from_tags(sh, sx).is_some() {
            let g = fpl.child("FGL");
            let fgy = g.and_then(|g| g.child("FGY"));
            let mut cst = stops(fgy.and_then(|y| y.child("FG0")));
            let ost = stops(fgy.and_then(|y| y.child("FG1")));
            if cst.is_empty() {
                cst = stops(g.and_then(|g| g.child("FGV")));
            }
            FillKind::Gradient(Gradient { shape: GradientShape::from_tags(sh, sx).unwrap_or(GradientShape::Linear), color_stops: cst, opacity_stops: ost })
        } else if sh == 16 {
            FillKind::WebDither([
                Argb(fpl.u("FD1", 0xFF000000)),
                Argb(fpl.u("FD2", 0xFF000000)),
                Argb(fpl.u("FD3", 0xFF000000)),
                Argb(fpl.u("FD4", 0xFF000000)),
            ])
        } else if sh == 15 {
            FillKind::Pattern(extra.pattern.clone().unwrap_or_else(|| Arc::new(RgbaImage::empty_at(0, 0))))
        } else {
            FillKind::Solid(Argb(pat.u("FCL", 0xFF000000)))
        };
        let ftb = fpl.u("FTB", 0);
        p.fill = Some(Fill {
            kind,
            edge: FillEdge::from_tags(fpl.u("FET", 1), fpl.u("FEF", 0)),
            handles,
            texture: if ftb > 0 { extra.texture.clone().map(|m| FillTexture { mask: m, amount_permille: ftb }) } else { None },
            stamping: StampingMode::from_u32(fpl.u("FSM", 2)),
            transparency_dither: fpl.b("FDT", false),
        });
    }
    if let Some(bpl) = pat.child("BPL") {
        let mut s = Stroke::new(brush(bpl), Argb(pat.u("BCL", 0xFF000000)));
        if let Some(m) = &extra.stroke_texture {
            s.texture = Some(StrokeTexture { mask: m.clone() });
        }
        p.stroke = Some(s);
    }
    p
}

pub fn mask_from(v: &Value) -> Option<Arc<Mask>> {
    let w = v.get("w")?.as_u64()? as i32;
    let h = v.get("h")?.as_u64()? as i32;
    let data = b64(v.get("data")?.as_str()?);
    Mask::from_raw(IRect::new(0, 0, w, h), data).ok().map(Arc::new)
}

pub fn image_from(v: &Value) -> Option<Arc<RgbaImage>> {
    let w = v.get("w")?.as_u64()? as i32;
    let h = v.get("h")?.as_u64()? as i32;
    let data = b64(v.get("rgba")?.as_str()?);
    RgbaImage::from_rgba_bytes(IRect::new(0, 0, w, h), &data).ok().map(Arc::new)
}

/// Pixelvergleich wie `test_paths.cmp`: beide Alpha 0 = gleich, sonst alle vier Kanäle.
/// Liefert (gleich, ±1, ±8, Pixel).
pub fn compare(a: &[[u8; 4]], b: &[u8]) -> (usize, usize, usize, usize) {
    let n = a.len().min(b.len() / 4);
    let (mut same, mut d1, mut d8) = (0, 0, 0);
    for k in 0..n {
        let (p, q) = (a[k], &b[k * 4..k * 4 + 4]);
        if p[3] == 0 && q[3] == 0 {
            same += 1;
            d1 += 1;
            d8 += 1;
            continue;
        }
        let d = (0..4).map(|c| (p[c] as i32 - q[c] as i32).abs()).max().unwrap_or(0);
        same += (d == 0) as usize;
        d1 += (d <= 1) as usize;
        d8 += (d <= 8) as usize;
    }
    (same, d1, d8, n)
}
