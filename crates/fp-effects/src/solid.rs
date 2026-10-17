//! Massiver Schatten `{b90c950e-…}` (Filter `CSolidShadowFilter`, Zeichenroutine `@00b27bb0`,
//! 05 §7 und Laufzeit T3).
//!
//! Zielbild = Kopie des Objekts; eine (bei gesetzter Farbe eingefärbte) Kopie wird entlang einer
//! Linie vom fernsten Punkt zum Objekt hin wiederholt per Original-over (MMX) daraufgemischt,
//! danach – nur bei eingefärbtem Schatten – das Original noch einmal obenauf. Die Positionen werden
//! mit `float32`-Zwischenwerten und Abschneiden auf **absoluten** Koordinaten bestimmt; das
//! Ergebnis wird auf die relativ gerechnete Begrenzung (`@00b286c0`) beschnitten.

use fp_color::Rgba8;
use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::EffectContext;
use crate::params::{Params, parse_color};
use crate::util::{new_image, over, px};

/// Parameter des massiven Schattens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidShadowParams {
    /// `angle` in Grad.
    pub angle: f64,
    /// `distance` in px.
    pub distance: f64,
    /// `color` (Alpha wird ignoriert).
    pub color: Rgba8,
    /// Farbe ist der Platzhalter `#ffffff00` („keine Farbe“): Kopien in Originalfarben.
    pub uncolored: bool,
}

impl Default for SolidShadowParams {
    fn default() -> Self {
        SolidShadowParams {
            angle: 45.0,
            distance: 30.0,
            color: Rgba8::new(255, 255, 255, 0),
            uncolored: true,
        }
    }
}

impl SolidShadowParams {
    /// Aus den Datei-Parametern (`usecolor` wird wie im Original nicht gelesen).
    pub fn read(p: Params) -> SolidShadowParams {
        let raw = p.text("color").unwrap_or("#ffffff00");
        let h = raw.trim().trim_start_matches('#');
        let color = parse_color(raw).unwrap_or(Rgba8::new(255, 255, 255, 0));
        let uncolored = h.len() >= 8 && color == Rgba8::new(255, 255, 255, 0);
        SolidShadowParams {
            angle: p.num("angle", 45.0),
            distance: p.num("distance", 30.0),
            color,
            uncolored,
        }
    }
}

fn f32r(v: f64) -> f64 {
    v as f32 as f64
}

fn trunc_i(v: f64) -> i64 {
    if v.is_finite() {
        v.trunc().clamp(-1.0e12, 1.0e12) as i64
    } else {
        0
    }
}

/// Versätze der Schattenkopien relativ zur Objektecke, fernste zuerst (`fx.solid_shadow_offsets`).
pub fn solid_shadow_offsets(
    x0: i32,
    y0: i32,
    w: i32,
    h: i32,
    distance: f64,
    angle: f64,
) -> Vec<(i32, i32)> {
    let mut ang = trunc_i(angle);
    if ang >= 360 {
        ang -= 360;
    }
    let d = trunc_i(distance) as f64;
    let (x0f, y0f) = (x0 as f64, y0 as f64);
    let cx = trunc_i(f32r((x0f + (x0f + w as f64)) * 0.5));
    let cy = trunc_i(f32r((y0f + (y0f + h as f64)) * 0.5));
    let ra = -(ang as f64 * 0.017453292519943295);
    let ex = trunc_i(f32r(d * ra.cos() + cx as f64));
    let ey = trunc_i(f32r(cy as f64 + d * ra.sin()));
    let (dxn, dyn_) = (cx - ex, cy - ey);
    let a = if dxn != 0 {
        dyn_ as f64 / dxn as f64
    } else {
        0.0
    };
    let b = if dyn_ != 0 {
        dxn as f64 / dyn_ as f64
    } else {
        0.0
    };
    let (mut xs, mut ys, n) = if b.abs() >= 1.0 || dyn_ == 0 {
        (1.0, a, dxn.abs())
    } else {
        (b, 1.0, dyn_.abs())
    };
    if (cx < ex && xs > 0.0) || (cx > ex && xs < 0.0) {
        xs = -xs;
    }
    if (cy < ey && ys > 0.0) || (cy > ey && ys < 0.0) {
        ys = -ys;
    }
    let n = n.clamp(0, 100_000) as usize;
    let mut out = Vec::with_capacity(n);
    let (mut px_, mut py_) = (ex as f64, ey as f64);
    for _ in 0..n {
        let tx = trunc_i(px_ - (cx - x0 as i64) as f64);
        let ty = trunc_i(py_ - (cy - y0 as i64) as f64);
        out.push(((tx - x0 as i64) as i32, (ty - y0 as i64) as i32));
        px_ += xs;
        py_ += ys;
    }
    out
}

/// Begrenzung `@00b286c0` relativ zur Objektecke: Objektrechteck ∪ (Rechteck +
/// (`trunc(d·cos α)`, `trunc(−d·sin α)`)).
fn clip_offset(sp: &SolidShadowParams) -> (i32, i32) {
    let mut a = sp.angle;
    if a >= 360.0 {
        a -= 360.0;
    }
    let r = a.to_radians();
    (
        trunc_i(sp.distance * r.cos()) as i32,
        trunc_i(-sp.distance * r.sin()) as i32,
    )
}

/// Massiver Schatten auf ein Objektbild.
pub fn solid_shadow(obj: &RgbaImage, sp: &SolidShadowParams, ctx: &EffectContext) -> RgbaImage {
    if obj.is_empty() {
        return obj.clone();
    }
    let ob = obj.bounds();
    let (w, h) = (ob.width() as i32, ob.height() as i32);
    let offs = solid_shadow_offsets(ob.left, ob.top, w, h, sp.distance, sp.angle);
    if offs.is_empty() {
        return obj.clone();
    }
    let mut src = obj.clone();
    if !sp.uncolored {
        for p in src.data_mut() {
            if p[3] > 0 {
                p[0] = sp.color.r;
                p[1] = sp.color.g;
                p[2] = sp.color.b;
            }
        }
    }
    let (mut minx, mut miny, mut maxx, mut maxy) = (0, 0, 0, 0);
    for &(x, y) in &offs {
        minx = minx.min(x);
        miny = miny.min(y);
        maxx = maxx.max(x);
        maxy = maxy.max(y);
    }
    let frame = IRect::new(
        ob.left + minx,
        ob.top + miny,
        ob.right + maxx,
        ob.bottom + maxy,
    );
    let mut r = new_image(frame);
    r.paste(obj);
    let fw = frame.width() as usize;
    let blend_layer = |r: &mut RgbaImage, layer: &RgbaImage, dx: i32, dy: i32| {
        let lb = layer.bounds().translate(dx, dy);
        let isect = frame.intersect(&lb);
        if isect.is_empty() {
            return;
        }
        let lw = lb.width() as usize;
        let data = r.data_mut();
        for y in isect.top..isect.bottom {
            for x in isect.left..isect.right {
                let s = layer.data()[(y - lb.top) as usize * lw + (x - lb.left) as usize];
                let di = (y - frame.top) as usize * fw + (x - frame.left) as usize;
                // Ziel außerhalb der Ebene: oben transparent → Ziel bleibt (wie over mit at = 0).
                data[di] = over(px(s), px(data[di]), ctx.mmx).to_array();
            }
        }
    };
    for &(ox, oy) in &offs {
        blend_layer(&mut r, &src, ox, oy);
    }
    if !sp.uncolored {
        blend_layer(&mut r, obj, 0, 0);
    }
    // Begrenzung (Laufzeit T3, B): fernste Kopie wird ggf. abgeschnitten.
    let (bx, by) = clip_offset(sp);
    let clip = IRect::new(
        ob.left + bx.min(0),
        ob.top + by.min(0),
        ob.left + w.max(w + bx),
        ob.top + h.max(h + by),
    );
    r.cropped(clip)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_45_degrees() {
        // 45°/12 px in positiven Koordinaten: Endversatz (8, −9) (05 §7).
        let o = solid_shadow_offsets(100, 100, 30, 20, 12.0, 45.0);
        assert_eq!(o.first(), Some(&(8, -9)));
        assert_eq!(o.len(), 9);
        assert!(solid_shadow_offsets(0, 0, 10, 10, 0.0, 45.0).is_empty());
    }
}
