//! Schlagschatten `{a7944db8-…}` und Innerer Schatten `{5600f702-…}` (05 §3, §4, §11).
//!
//! Schlagschatten (CS6, B): Objekt-Alpha mit dem Dreiecksfilter (Radius `ShadowBlur`, Rand-Ring
//! eingeschlossen) weichzeichnen, um `(trunc(d·cos α), trunc(−d·sin α))` verschieben, mit der
//! Schattenfarbe einfärben (Alpha `round(m·a/255)`), dann das Objekt per Original-over (MMX)
//! darüber; „Aussparen“ (`ShadowType 1`) zieht das Objekt-Alpha ab und zeichnet das Objekt nicht.
//! Laufzeit T1/T1b: die letzte Zeile der senkrechten Weichzeichnung ist `screen(v, v)`, sichtbar,
//! wenn der Schatten unten nicht über das Objekt hinausragt (`dy + r ≤ 0`).
//!
//! Innerer Schatten (B): Maske `255 − verschobenes weiches Alpha`, Farbe per skalarem
//! Original-over auf das Objekt, Ergebnis-Alpha = Objekt-Alpha; Aussparen: nur der Schatten,
//! mit dem Objekt-Alpha multipliziert.

use fp_color::Rgba8;
use fp_color::blend::{orig_over, over_place_cs5};
use fp_raster::RgbaImage;

use crate::EffectContext;
use crate::params::Params;
use crate::util::{mul, new_image, on_frame, over, px, tent, union};

/// Parameter von Schlag- und Innerem Schatten (05 §3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowParams {
    /// `ShadowColor`; Alpha = Deckkraft.
    pub color: Rgba8,
    /// `ShadowBlur`: Radius des Dreiecksfilters in px.
    pub blur: u32,
    /// `ShadowDistance` in px.
    pub distance: f64,
    /// `ShadowAngle` in Grad (gegen den Uhrzeigersinn, 0 = rechts).
    pub angle: f64,
    /// `ShadowType 1` = Aussparen („Knockout“).
    pub knockout: bool,
}

impl ShadowParams {
    /// Vorgaben des Schlagschattens (`#000000a6`, 4, 7, 315).
    pub const DROP_DEFAULT: ShadowParams = ShadowParams {
        color: Rgba8::new(0, 0, 0, 0xa6),
        blur: 4,
        distance: 7.0,
        angle: 315.0,
        knockout: false,
    };
    /// Vorgaben des Inneren Schattens (`#000000a5`, 4, 7, 315).
    pub const INNER_DEFAULT: ShadowParams = ShadowParams {
        color: Rgba8::new(0, 0, 0, 0xa5),
        blur: 4,
        distance: 7.0,
        angle: 315.0,
        knockout: false,
    };

    /// Aus den Datei-Parametern; `defaults` liefert fehlende Werte.
    pub fn read(p: Params, defaults: ShadowParams) -> ShadowParams {
        ShadowParams {
            color: p.color("ShadowColor", defaults.color),
            blur: p.int("ShadowBlur", defaults.blur as i64).clamp(0, 1024) as u32,
            distance: p.num("ShadowDistance", defaults.distance),
            angle: p.num("ShadowAngle", defaults.angle),
            knockout: p
                .text("ShadowType")
                .map(|s| s == "1")
                .unwrap_or(defaults.knockout),
        }
    }

    /// Versatz des Schattens (05 §2.3).
    pub fn offset(&self) -> (i32, i32) {
        offset_trunc(self.distance, self.angle)
    }
}

/// Versatz aus Distanz und Winkel wie `@008055d0`: `dx = trunc(d·sin(α+90°))`,
/// `dy = trunc(d·cos(α+90°))` (= `trunc(d·cos α)`, `trunc(−d·sin α)`).
pub fn offset_trunc(distance: f64, angle_deg: f64) -> (i32, i32) {
    let a = (angle_deg + 90.0).to_radians();
    let (fx, fy) = (distance * a.sin(), distance * a.cos());
    (sat_i32(fx), sat_i32(fy))
}

fn sat_i32(v: f64) -> i32 {
    if v.is_finite() {
        v.trunc().clamp(-1.0e9, 1.0e9) as i32
    } else {
        0
    }
}

/// Schlagschatten auf ein Objektbild (Dokumentkoordinaten). Ergebnis im Rahmen
/// Objekt ∪ Schattenebene.
pub fn drop_shadow(obj: &RgbaImage, sp: &ShadowParams, ctx: &EffectContext) -> RgbaImage {
    if obj.is_empty() {
        return obj.clone();
    }
    let (dx, dy) = sp.offset();
    let quirk = ctx.shadow_last_row_screen && dy + sp.blur as i32 <= 0;
    let mut m = tent(&obj.alpha_mask(), sp.blur, quirk);
    if ctx.legacy_cs5 && sp.blur > 0 {
        // CS5-Altdaten: äußerer Ring fehlt (05 §3.4).
        m = crate::util::mask_on(&m, m.bounds().inflate(-1, -1));
    }
    let sb = m.bounds().translate(dx, dy);
    let frame = union(obj.bounds(), sb);
    let mut out = new_image(frame);
    let o = on_frame(obj, frame);
    let (c, a) = (sp.color, sp.color.a);
    let fw = frame.width() as usize;
    for (i, d) in out.data_mut().iter_mut().enumerate() {
        let x = frame.left + (i % fw.max(1)) as i32;
        let y = frame.top + (i / fw.max(1)) as i32;
        let mv = m.get(x - dx, y - dy);
        let s = Rgba8::new(c.r, c.g, c.b, mul(mv, a));
        let op = px(o.data()[i]);
        let r = if sp.knockout {
            // Modus 1 „kopieren“ mit 00FFFFFF durch die Objektmaske (@0088fc91).
            let na = mul(s.a, 255 - op.a);
            if na == 0 {
                Rgba8::TRANSPARENT
            } else {
                Rgba8::new(s.r, s.g, s.b, na)
            }
        } else if ctx.legacy_cs5 {
            over_place_cs5(op, s)
        } else {
            over(op, s, ctx.mmx)
        };
        *d = r.to_array();
    }
    out
}

/// Innerer Schatten auf ein Objektbild; Ergebnis im Objektrahmen.
pub fn inner_shadow(obj: &RgbaImage, sp: &ShadowParams, ctx: &EffectContext) -> RgbaImage {
    if obj.is_empty() {
        return obj.clone();
    }
    let (dx, dy) = sp.offset();
    let alpha = obj.alpha_mask();
    let mut m = tent(&alpha, sp.blur, false);
    if ctx.legacy_cs5 && sp.blur > 0 {
        m = crate::util::mask_on(&m, m.bounds().inflate(-1, -1));
    }
    let b = obj.bounds();
    let w = b.width() as usize;
    let c = sp.color;
    let mut out = obj.clone();
    for (i, d) in out.data_mut().iter_mut().enumerate() {
        let x = b.left + (i % w.max(1)) as i32;
        let y = b.top + (i / w.max(1)) as i32;
        let mask = 255 - m.get(x - dx, y - dy);
        let sa = mul(mask, c.a);
        let p = px(*d);
        let r = if sp.knockout {
            Rgba8::new(c.r, c.g, c.b, mul(sa, p.a))
        } else {
            let o = orig_over(Rgba8::new(c.r, c.g, c.b, sa), p);
            Rgba8::new(o.r, o.g, o.b, p.a)
        };
        *d = r.to_array();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_geom::IRect;

    #[test]
    fn offsets_like_reference() {
        assert_eq!(offset_trunc(1.0, 315.0), (0, 0));
        assert_eq!(offset_trunc(2.0, 315.0), (1, 1));
        assert_eq!(offset_trunc(7.0, 270.0), (0, 7));
        assert_eq!(offset_trunc(7.0, 90.0), (0, -7));
    }

    #[test]
    fn ring_is_present_in_cs6() {
        // Laufzeit T1 A_ring: 20×20 deckend, Blur 4, Distanz 0 → Ergebnis 28×28 (Ring sichtbar).
        let obj = RgbaImage::filled(IRect::new(40, 40, 60, 60), Rgba8::rgb(255, 0, 0)).unwrap();
        let sp = ShadowParams {
            color: Rgba8::new(0, 0, 0, 255),
            blur: 4,
            distance: 0.0,
            angle: 315.0,
            knockout: false,
        };
        let r = drop_shadow(&obj, &sp, &EffectContext::default());
        assert_eq!(r.alpha_bounds(), IRect::new(36, 36, 64, 64));
        let mid = r.pixel(50, 36);
        assert!(mid.a > 0);
    }
}
