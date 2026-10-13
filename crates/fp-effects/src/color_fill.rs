//! Farbfüllung `{dd54adc0-…}` (`CDbColorFill`, Render `@00790ac0`, 05 §8).
//!
//! `a = Opacity·255/100` (abgeschnitten), konstante Farbe per „Farbe mit Alpha erhalten“
//! (`StBlendConstSrcPreserveAlpha`, 06 §5): Normal mit dem Gewicht `DIV[a][a − div255(A·a) + A]`
//! für halbtransparente Pixel (bei A = 255 einfach `a`), Alpha bleibt. Gemessen (RE-FX Gruppe 4,
//! `farbfuellung_klein`, 20 %): die frühere Formel `div255((Cf − C)·a + C·255)` wich an weichen
//! Kanten um bis zu 147 Stufen ab, diese ist exakt (FX-4). Andere
//! Mischmodi laufen über die allgemeine Mischfunktion von `fp_color` mit Ergebnis-Alpha = Pixel-Alpha (A).

use fp_color::blend::{blend_pixel, OverVariant};
use fp_color::{BlendMode, Rgba8};
use fp_raster::RgbaImage;

use crate::params::Params;

/// Parameter der Farbfüllung.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorFillParams {
    /// `Color`.
    pub color: Rgba8,
    /// `Opacity` in %.
    pub opacity: i64,
    /// `Blendmode` (interne Nummer = `BLDi`).
    pub blend: u8,
}

impl Default for ColorFillParams {
    fn default() -> Self {
        ColorFillParams { color: Rgba8::new(255, 0, 0, 255), opacity: 100, blend: 0 }
    }
}

impl ColorFillParams {
    /// Aus den Datei-Parametern.
    pub fn read(p: Params) -> ColorFillParams {
        ColorFillParams {
            color: p.color("Color", Rgba8::new(255, 0, 0, 255)),
            opacity: p.int("Opacity", 100),
            blend: p.int("Blendmode", 0).clamp(0, 255) as u8,
        }
    }
}

/// Farbfüllung auf ein Objektbild (Rahmen bleibt).
pub fn color_fill(obj: &RgbaImage, cp: &ColorFillParams) -> RgbaImage {
    let a = (cp.opacity.clamp(-1000, 1000) * 255).div_euclid(100).clamp(0, 255) as u8;
    let rgb = [cp.color.r, cp.color.g, cp.color.b];
    let mut out = obj.clone();
    let mode = BlendMode::from_file(Some(cp.blend as i64));
    let raster_fill = fp_raster::filters::ColorFillParams {
        color: Rgba8::rgb(rgb[0], rgb[1], rgb[2]),
        opacity: cp.opacity.clamp(0, 100) as i32,
        blend_mode: BlendMode(0),
    };
    for p in out.data_mut() {
        let d = Rgba8::from_array(*p);
        let r = if mode.0 == 0 {
            Rgba8::from_array(raster_fill.apply_px(*p))
        } else {
            let b = blend_pixel(d, Rgba8::new(rgb[0], rgb[1], rgb[2], 255), mode, a, 255, OverVariant::Scalar);
            Rgba8::new(b.r, b.g, b.b, d.a)
        };
        *p = r.to_array();
    }
    out
}
