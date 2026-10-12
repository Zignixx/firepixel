//! Feineinstellungen der KI-Masken (Nachbearbeitung ohne neues Modell-Laufen).
//!
//! Was die Modelle wirklich hergeben (siehe `docs\MODELLE.md`):
//! * **Toleranz** – SAM 2.1 liefert Logits, die Maske entsteht über eine Schwelle (Vorgabe 0);
//!   BiRefNet liefert eine weiche Maske 0…1, die Toleranz verschiebt deren Tonwerte.
//! * **Maskenvariante** – der SAM-Decoder rechnet immer vier Masken (eine Einzelmaske und drei
//!   Mehrfachmasken mit eigener Güte). „Automatisch“ nimmt die des Modells, Klein/Mittel/Groß die
//!   Mehrfachmasken nach Fläche.
//! * **Kante** (Hart/Geglättet/Weiche Kante), **Ausdehnen/Zusammenziehen** und **Glätten** sind
//!   reine Nachbearbeitung der Maske und für beide Modelle gleich.
//!
//! Ausdehnen/Zusammenziehen arbeitet als Grauwert-Morphologie mit kreisförmigem Strukturelement:
//! der weiche Rand (Haare, Halbtransparenz) bleibt erhalten und wandert nur nach außen bzw. innen.

use crate::error::{AiError, Result};
use fp_raster::{Mask, SoftEdge};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Kante der KI-Maske (wie beim Zauberstab).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    /// Harte Kante (Maske 0 oder 255).
    Hard,
    /// Geglättete Kante (Übergang des Modells, Vorgabe).
    #[default]
    AntiAlias,
    /// Weiche Kante mit [`Tuning::feather`] Pixeln.
    Feather,
}

impl Edge {
    /// Name in Parametern und Voreinstellungen (`hard`, `antiAlias`, `feather`).
    pub fn name(self) -> &'static str {
        match self {
            Edge::Hard => "hard",
            Edge::AntiAlias => "antiAlias",
            Edge::Feather => "feather",
        }
    }
    /// Aus dem Namen (auch `antialias`, `smooth`, `soft`).
    pub fn from_name(s: &str) -> Option<Edge> {
        match s.trim().to_ascii_lowercase().as_str() {
            "hard" => Some(Edge::Hard),
            "antialias" | "anti-alias" | "smooth" => Some(Edge::AntiAlias),
            "feather" | "soft" => Some(Edge::Feather),
            _ => None,
        }
    }
}

/// Wahl unter den Masken des SAM-Decoders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Variant {
    /// Die Maske, die das Modell selbst wählt (beste Güte bzw. stabile Einzelmaske).
    #[default]
    Auto,
    /// Kleinste der drei Mehrfachmasken (Teil des Objekts).
    Small,
    /// Mittlere Mehrfachmaske.
    Medium,
    /// Größte Mehrfachmaske (ganzes Objekt mit allem, was dazugehört).
    Large,
}

impl Variant {
    /// Name in Parametern und Voreinstellungen.
    pub fn name(self) -> &'static str {
        match self {
            Variant::Auto => "auto",
            Variant::Small => "small",
            Variant::Medium => "medium",
            Variant::Large => "large",
        }
    }
    /// Aus dem Namen.
    pub fn from_name(s: &str) -> Option<Variant> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" | "best" => Some(Variant::Auto),
            "small" => Some(Variant::Small),
            "medium" => Some(Variant::Medium),
            "large" => Some(Variant::Large),
            _ => None,
        }
    }
}

/// Größte Toleranz, Ausdehnung und Glättung (Pixel), die angenommen werden.
pub const MAX_EXPAND: i32 = 100;
/// Größter Glättungsradius.
pub const MAX_SMOOTH: u32 = 100;
/// Verschiebung der SAM-Schwelle bei Toleranz 0 bzw. 100 (in Logit-Einheiten).
pub const SAM_TOLERANCE_LOGITS: f32 = 10.0;

/// Feineinstellungen einer KI-Maske.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tuning {
    /// 0…100; 50 = Entscheidung des Modells, höher = mehr auswählen.
    pub tolerance: f32,
    /// Maskenvariante (nur SAM).
    pub variant: Variant,
    /// Kante.
    pub edge: Edge,
    /// Radius der weichen Kante (bei [`Edge::Feather`]).
    pub feather: u32,
    /// Ausdehnen (> 0) bzw. zusammenziehen (< 0) in Pixeln.
    pub expand: i32,
    /// Glätten: Radius in Pixeln (0 = aus).
    pub smooth: u32,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning { tolerance: 50.0, variant: Variant::Auto, edge: Edge::AntiAlias, feather: 0, expand: 0, smooth: 0 }
    }
}

impl Tuning {
    /// Werte auf die gültigen Bereiche begrenzen.
    pub fn clamped(mut self) -> Tuning {
        self.tolerance = if self.tolerance.is_finite() { self.tolerance.clamp(0.0, 100.0) } else { 50.0 };
        self.feather = self.feather.min(1000);
        self.expand = self.expand.clamp(-MAX_EXPAND, MAX_EXPAND);
        self.smooth = self.smooth.min(MAX_SMOOTH);
        self
    }

    /// Schwelle der SAM-Logits: 0 bei Toleranz 50, niedriger (mehr Objekt) bei höherer Toleranz.
    pub fn sam_threshold(&self) -> f32 {
        -(self.tolerance - 50.0) / 50.0 * SAM_TOLERANCE_LOGITS
    }

    /// Tonwertabbildung der weichen BiRefNet-Maske (0…1): bei 50 unverändert; höher hebt schwache
    /// Werte an (mehr Motiv), niedriger nimmt sie weg (nur das sichere Motiv). Monoton.
    pub fn soft_level(&self, a: f32) -> f32 {
        let s = ((self.tolerance - 50.0) / 50.0).clamp(-1.0, 1.0) * 0.98;
        let v = if s >= 0.0 { a / (1.0 - s) } else { (a + s) / (1.0 + s) };
        v.clamp(0.0, 1.0)
    }
}

/// Nachbearbeitung einer weichen Maske: Glätten, Ausdehnen/Zusammenziehen, Kante.
pub fn postprocess(m: Mask, t: &Tuning) -> Result<Mask> {
    let t = t.clamped();
    let err = |e: fp_raster::RasterError| AiError::Input(e.to_string());
    let mut m = m;
    if t.smooth > 0 {
        m = smooth_soft(&m, t.smooth).map_err(err)?;
    }
    if t.expand != 0 {
        m = morph_soft(&m, t.expand).map_err(err)?;
    }
    match t.edge {
        Edge::Hard => {
            m.map(|v| if v >= 128 { 255 } else { 0 });
            m.set_soft_edge(SoftEdge::Hard);
        }
        Edge::AntiAlias => m.set_soft_edge(SoftEdge::One),
        Edge::Feather => {
            if t.feather > 0 {
                m = m.feather(t.feather).map_err(err)?;
            }
            m.set_soft_edge(SoftEdge::One);
        }
    }
    Ok(m)
}

/// Glätten einer weichen Maske: Dreiecksfilter mit Radius `r`, danach wird der Übergang um die
/// Mitte wieder auf wenige Pixel gestrafft (rundet Ecken, entfernt Krümel und kleine Löcher).
fn smooth_soft(m: &Mask, r: u32) -> std::result::Result<Mask, fp_raster::RasterError> {
    let mut b = m.tent_blur(r, r, false)?;
    b.map(|v| {
        let t = ((v as f32 - 96.0) / 64.0).clamp(0.0, 1.0);
        (t * t * (3.0 - 2.0 * t) * 255.0).round() as u8
    });
    Ok(b)
}

/// Grauwert-Dilatation (`r > 0`) bzw. -Erosion (`r < 0`) mit einer Kreisscheibe vom Radius `|r|`.
/// Außerhalb der Maske gilt 0; die Dilatation vergrößert die Grenzen um `r`.
pub fn morph_soft(m: &Mask, r: i32) -> std::result::Result<Mask, fp_raster::RasterError> {
    if r == 0 {
        return Ok(m.clone());
    }
    let rad = r.unsigned_abs() as i32;
    let dilate = r > 0;
    let src_b = m.bounds();
    let out_b = if dilate { src_b.inflate(rad, rad) } else { src_b };
    let mut out = Mask::new(out_b)?;
    let (sw, sh) = (m.width() as i32, m.height() as i32);
    let ow = out.width();
    let src = m.data();
    // halbe Breite der Kreisscheibe je Zeilenabstand
    let half: Vec<i32> = (0..=rad).map(|dy| (((rad * rad - dy * dy) as f32).sqrt()).floor() as i32).collect();
    let (ox, oy) = (out_b.left - src_b.left, out_b.top - src_b.top);
    out.data_mut().par_chunks_mut(ow).enumerate().for_each(|(y, row)| {
        let sy0 = y as i32 + oy;
        // Zwischenspeicher für eine Fensterzeile
        let mut acc: Vec<u8> = vec![if dilate { 0 } else { 255 }; ow];
        let mut win = vec![0u8; ow];
        for dy in -rad..=rad {
            let sy = sy0 + dy;
            let hw = half[dy.unsigned_abs() as usize];
            if sy < 0 || sy >= sh {
                if !dilate {
                    // Erosion: außerhalb = 0 → alles in Reichweite wird 0
                    acc.iter_mut().for_each(|v| *v = 0);
                }
                continue;
            }
            let line = &src[(sy * sw) as usize..((sy + 1) * sw) as usize];
            sliding(line, ox, hw, dilate, &mut win);
            if dilate {
                for (a, w) in acc.iter_mut().zip(&win) {
                    *a = (*a).max(*w);
                }
            } else {
                for (a, w) in acc.iter_mut().zip(&win) {
                    *a = (*a).min(*w);
                }
            }
        }
        row.copy_from_slice(&acc);
    });
    let soft = m.soft_edge();
    out.set_soft_edge(soft);
    Ok(out)
}

/// Gleitendes Maximum bzw. Minimum der Breite `2·hw+1` über `line` (außerhalb 0) für die
/// Ausgabepositionen `x = 0…out.len()`, Quellposition `x + ox` (monotone Warteschlange, O(n)).
fn sliding(line: &[u8], ox: i32, hw: i32, max: bool, out: &mut [u8]) {
    let n = line.len() as i32;
    let get = |i: i32| if i < 0 || i >= n { 0u8 } else { line[i as usize] };
    let better = |a: u8, b: u8| if max { a >= b } else { a <= b };
    let mut dq: std::collections::VecDeque<(i32, u8)> = std::collections::VecDeque::new();
    // Fenster für Ausgabe x: Quellen [x+ox-hw, x+ox+hw]
    let mut next = ox - hw;
    for (x, o) in out.iter_mut().enumerate() {
        let hi = x as i32 + ox + hw;
        let lo = x as i32 + ox - hw;
        while next <= hi {
            let v = get(next);
            while dq.back().is_some_and(|b| better(v, b.1)) {
                dq.pop_back();
            }
            dq.push_back((next, v));
            next += 1;
        }
        while dq.front().is_some_and(|f| f.0 < lo) {
            dq.pop_front();
        }
        *o = dq.front().map(|f| f.1).unwrap_or(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    fn disk(r: f32) -> Mask {
        let b = IRect::from_xywh(0, 0, 100, 100);
        let mut m = Mask::new(b).unwrap_or_else(|_| Mask::empty_at(0, 0));
        for y in 0..100 {
            for x in 0..100 {
                let d = ((x as f32 - 50.0).powi(2) + (y as f32 - 50.0).powi(2)).sqrt();
                m.set(x, y, ((r + 0.5 - d).clamp(0.0, 1.0) * 255.0) as u8);
            }
        }
        m
    }

    fn area(m: &Mask) -> u64 {
        m.data().iter().map(|v| *v as u64).sum::<u64>() / 255
    }

    #[test]
    fn expand_and_contract_move_the_edge() {
        let m = disk(20.0);
        let a0 = area(&m) as f32;
        let e = morph_soft(&m, 5).unwrap_or_default();
        let c = morph_soft(&m, -5).unwrap_or_default();
        let (ae, ac) = (area(&e) as f32, area(&c) as f32);
        // Fläche ≈ π·(20±5)²
        assert!((ae - std::f32::consts::PI * 625.0).abs() < 60.0, "{a0} {ae}");
        assert!((ac - std::f32::consts::PI * 225.0).abs() < 60.0, "{a0} {ac}");
        assert_eq!(e.get(50, 50 - 24), 255);
        assert_eq!(c.get(50, 50 - 16), 0);
        // weicher Rand bleibt weich
        assert!(e.data().iter().any(|v| *v > 0 && *v < 255));
    }

    #[test]
    fn tolerance_levels_are_monotone() {
        let lo = Tuning { tolerance: 10.0, ..Tuning::default() };
        let mid = Tuning::default();
        let hi = Tuning { tolerance: 90.0, ..Tuning::default() };
        for a in [0.05f32, 0.3, 0.5, 0.8] {
            assert!((mid.soft_level(a) - a).abs() < 1e-6);
            assert!(lo.soft_level(a) <= a && hi.soft_level(a) >= a);
        }
        assert!(hi.sam_threshold() < 0.0 && lo.sam_threshold() > 0.0 && mid.sam_threshold() == 0.0);
    }

    #[test]
    fn edge_modes() {
        let m = disk(20.0);
        let h = postprocess(m.clone(), &Tuning { edge: Edge::Hard, ..Tuning::default() }).unwrap_or_default();
        assert!(h.data().iter().all(|v| *v == 0 || *v == 255));
        let f = postprocess(m.clone(), &Tuning { edge: Edge::Feather, feather: 6, ..Tuning::default() }).unwrap_or_default();
        let soft = |m: &Mask| m.data().iter().filter(|v| **v > 0 && **v < 255).count();
        assert!(soft(&f) > soft(&m) * 3, "{} {}", soft(&f), soft(&m));
        let s = postprocess(m, &Tuning { smooth: 4, ..Tuning::default() }).unwrap_or_default();
        assert!(area(&s) > 1000);
    }
}
