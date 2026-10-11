//! Inneres Glühen `{2ba87123-…}` nach `@00803890` (05 §6, Laufzeit T2: beide Zweige bestätigt).
//!
//! ```text
//! a = trunc(BevelContrast·255/100)
//! B = Fläche(Alpha)                                   Morphologie Modus 5: Alpha > Toleranz
//! GlowStart = 0: M = 255 − Dreieck(Zusammenziehen(B, Breite), MaskSoftness)   im Objektrechteck
//!                Farbe (Alpha mul(M, a)) „Farbe über, Alpha bleibt“ aufs Objekt
//! GlowStart > 0: C = Rand(Zusammenziehen(B, Breite/2 + Start), Breite)        Modus 1, dann 4
//!                M = mul(Dreieck(C, MaskSoftness), Alpha); Farbe normal aufs Objekt
//! ```
//! Die Toleranz ist die Voreinstellung `SelectionAdjustTolerance` (Standard 1).

use fp_color::blend::orig_over;
use fp_color::Rgba8;
use fp_raster::{Mask, MorphMode, RgbaImage};

use crate::params::Params;
use crate::util::{mask_on, mul, on_frame, px, tent_same};
use crate::EffectContext;

/// Parameter des Inneren Glühens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InnerGlowParams {
    /// `OuterBevelColor` (Alpha wird nicht benutzt).
    pub color: Rgba8,
    /// `MaskSoftness`.
    pub softness: u32,
    /// `GlowWidth` (Werte < 3 werden zu 2, Leser `@008033d0`).
    pub width: u32,
    /// `GlowStartDistance`.
    pub start: u32,
    /// `BevelContrast` in %.
    pub contrast: i64,
}

impl Default for InnerGlowParams {
    fn default() -> Self {
        InnerGlowParams { color: Rgba8::new(0, 0, 0, 255), softness: 4, width: 4, start: 0, contrast: 65 }
    }
}

impl InnerGlowParams {
    /// Aus den Datei-Parametern.
    pub fn read(p: Params) -> InnerGlowParams {
        let mut width = p.int("GlowWidth", 4);
        if width < 3 {
            width = 2;
        }
        InnerGlowParams {
            color: p.color("OuterBevelColor", Rgba8::new(0, 0, 0, 255)),
            softness: p.int("MaskSoftness", 4).clamp(0, 1024) as u32,
            width: width.clamp(0, 1024) as u32,
            start: p.int("GlowStartDistance", 0).clamp(0, 1024) as u32,
            contrast: p.int("BevelContrast", 65),
        }
    }
}

/// Inneres Glühen auf ein Objektbild; Ergebnis im Objektrahmen.
pub fn inner_glow(obj: &RgbaImage, gp: &InnerGlowParams, ctx: &EffectContext) -> RgbaImage {
    if obj.is_empty() {
        return obj.clone();
    }
    let tol = ctx.selection_adjust_tolerance;
    let a = ((gp.contrast as f64) * 255.0 / 100.0).trunc().clamp(0.0, 255.0) as u8;
    let pad = (gp.softness + 2 * gp.width + gp.start + 8) as i32;
    let ob = obj.bounds();
    let frame = ob.inflate(pad, pad);
    let img = on_frame(obj, frame);
    let alpha = img.alpha_mask();
    let morph = |m: &Mask, amount: u32, mode: MorphMode| -> Mask {
        match m.morph(amount, mode, tol) {
            Ok(r) => mask_on(&r, frame),
            Err(_) => m.clone(),
        }
    };
    let b = morph(&alpha, 0, MorphMode::Area);
    let c = gp.color;
    let mut out = img.clone();
    let fw = frame.width() as usize;
    if gp.start == 0 {
        let d = tent_same(&morph(&b, gp.width, MorphMode::Contract), gp.softness);
        for (i, p) in out.data_mut().iter_mut().enumerate() {
            let x = frame.left + (i % fw.max(1)) as i32;
            let y = frame.top + (i / fw.max(1)) as i32;
            let f = if ob.contains(x, y) { 255u8 } else { 0 };
            let m = f.saturating_sub(d.data()[i]);
            let s = mul(m, a);
            let o = px(*p);
            let r = orig_over(Rgba8::new(c.r, c.g, c.b, s), o);
            *p = if o.a == 0 { [0, 0, 0, 0] } else { [r.r, r.g, r.b, o.a] };
        }
    } else {
        let cm = morph(&morph(&b, gp.width / 2 + gp.start, MorphMode::Contract), gp.width, MorphMode::BorderRaw);
        let d = tent_same(&cm, gp.softness);
        for (i, p) in out.data_mut().iter_mut().enumerate() {
            let o = px(*p);
            let m = mul(d.data()[i], o.a);
            let s = mul(m, a);
            let r = orig_over(Rgba8::new(c.r, c.g, c.b, s), o);
            *p = if r.a == 0 { [0, 0, 0, 0] } else { r.to_array() };
        }
    }
    on_frame(&out, ob)
}
