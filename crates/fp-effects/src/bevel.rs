//! Abschrägung, Prägung und Glühen `{7fe61102-…}` (05 §5 und Laufzeit T2, Referenz
//! `effekte/bevel.py`).
//!
//! `BevelType` 0 innere Abschrägung, 1 äußere Abschrägung, 2 erhabene, 3 vertiefte Prägung, 4 Glühen.
//!
//! Ablauf (`@0077e3f0`):
//! 1. Kantenmaske (`@0077cac0`): Schwelle aus `EdgeThreshold`, Zellen „innen“ (zu füllen), „Kante“
//!    (Saat, Abstand 0) und „aus“.
//! 2. Abstandsfeld (`@0077d0b0`/`@00779640`): ringweise, laufend in Zeilenreihenfolge; je Zelle der
//!    kürzeste Vektor zur Saat, Abstand in 1/4096 px; bei genau zwei gleich langen Kandidaten
//!    (nicht beim Glühen) ein gemittelter Vektor.
//! 3. Normalenwinkel als Byte (`@006d04a0` Tabellen-Arkustangens, `@00778c40`), Glättung über ein
//!    (2s+1)²-Fenster (`@0077d260`).
//! 4. Licht-/Schattenmaske je Zelle (`@0077aaf0` Abschrägung, `@0077b530` Prägung,
//!    `@00778eb0` Glühen), Werte mit x87-Abschneiden.
//! 5. Masken mit dem Dreiecksfilter weichzeichnen, außerhalb der Breite ausblenden.
//! 6. Mischen (`@0077ad90`, `@0077b780`, `@0077ba90`) inklusive Schaltflächenstatus.
//!
//! Gleiche Werte wie die Laufzeittests T2: innere/äußere Abschrägung flach, alle Schaltflächen­
//! status, Prägung erhaben/vertieft exakt; Profile 1/2/5/6 bis auf ±1 an Ecken (05 §11).

use fp_color::blend::orig_over;
use fp_color::Rgba8;
use fp_raster::{Mask, RgbaImage};

use crate::params::Params;
use crate::util::{div255, on_frame, px, tent_same};
use crate::x87;

/// Zelle „noch zu füllen“.
pub const INSIDE: i64 = 0x7FFE;
/// Zelle „ausgeschlossen“.
pub const OUTSIDE: i64 = 0x7FFF;

/// Variante (`BevelType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BevelType {
    /// 0: innere Abschrägung.
    Inner,
    /// 1: äußere Abschrägung.
    Outer,
    /// 2: erhabene Prägung.
    Raised,
    /// 3: vertiefte Prägung.
    Inset,
    /// 4: Glühen.
    Glow,
}

impl BevelType {
    /// Aus der Dateinummer (unbekannt → innere Abschrägung).
    pub fn from_code(v: i64) -> BevelType {
        match v {
            1 => BevelType::Outer,
            2 => BevelType::Raised,
            3 => BevelType::Inset,
            4 => BevelType::Glow,
            _ => BevelType::Inner,
        }
    }
    /// Dateinummer.
    pub fn code(self) -> i64 {
        match self {
            BevelType::Inner => 0,
            BevelType::Outer => 1,
            BevelType::Raised => 2,
            BevelType::Inset => 3,
            BevelType::Glow => 4,
        }
    }
}

/// Schaltflächenstatus (`ButtonState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    /// 0 normal.
    Up,
    /// 1 darüber.
    Over,
    /// 2 gedrückt.
    Down,
    /// 3 darüber gedrückt.
    OverDown,
}

impl ButtonState {
    /// Aus der Dateinummer.
    pub fn from_code(v: i64) -> ButtonState {
        match v {
            1 => ButtonState::Over,
            2 => ButtonState::Down,
            3 => ButtonState::OverDown,
            _ => ButtonState::Up,
        }
    }
}

/// Parameter der Abschrägungsfamilie (05 §5.1, Vorgaben des Lesers `@00779ce0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelParams {
    pub kind: BevelType,
    /// `BevelWidth`.
    pub width: i64,
    /// `BevelContrast` in %.
    pub contrast: i64,
    /// `EdgeThreshold` 0..1 (wird laut Leser nur beim Glühen gelesen, sonst 0).
    pub edge: f64,
    /// `AngleSoftness`.
    pub angle_softness: i64,
    /// `MaskSoftness`.
    pub mask_softness: i64,
    /// `LightDistance` (wird nicht gerendert, nur durchgereicht).
    pub light_distance: i64,
    /// `LightAngle` in Grad.
    pub light_angle: i64,
    /// `SlopeType` 0..6.
    pub slope: i64,
    /// `SlopeMultiplier`.
    pub slope_multiplier: f64,
    /// `OuterBevelColor`.
    pub outer_color: Rgba8,
    /// `DownBlendColor`.
    pub down_color: Rgba8,
    /// `HitBlendColor`.
    pub hit_color: Rgba8,
    /// `HiliteColor`.
    pub hilite: Rgba8,
    /// `ShadowColor`.
    pub shadow: Rgba8,
    /// `EmbossFaceColor`.
    pub face: Rgba8,
    /// `ShowObject`.
    pub show_object: bool,
    /// `ButtonState`.
    pub button_state: ButtonState,
    /// `GlowStartDistance`.
    pub glow_start: i64,
    /// `GlowWidth`.
    pub glow_width: i64,
}

impl Default for BevelParams {
    fn default() -> Self {
        BevelParams {
            kind: BevelType::Inner,
            width: 10,
            contrast: 75,
            edge: 0.0,
            angle_softness: 25,
            mask_softness: 0,
            light_distance: 100,
            light_angle: 135,
            slope: 0,
            slope_multiplier: 1.0,
            outer_color: Rgba8::new(0xdf, 0, 0, 0xff),
            down_color: Rgba8::new(0, 0, 0, 0x3f),
            hit_color: Rgba8::new(0xff, 0xff, 0xff, 0x3f),
            hilite: Rgba8::new(255, 255, 255, 255),
            shadow: Rgba8::new(0, 0, 0, 255),
            face: Rgba8::new(255, 255, 255, 0),
            show_object: false,
            button_state: ButtonState::Up,
            glow_start: 0,
            glow_width: 0,
        }
    }
}

impl BevelParams {
    /// Aus den Datei-Parametern.
    pub fn read(p: Params) -> BevelParams {
        let d = BevelParams::default();
        let kind = BevelType::from_code(p.int("BevelType", 0));
        BevelParams {
            kind,
            width: p.int("BevelWidth", d.width),
            contrast: p.int("BevelContrast", d.contrast),
            // 05 §5.1/5d: der Leser liest EdgeThreshold nur beim Glühen, sonst gilt 0.
            edge: if kind == BevelType::Glow { p.num("EdgeThreshold", 0.0) } else { 0.0 },
            angle_softness: p.int("AngleSoftness", d.angle_softness),
            mask_softness: p.int("MaskSoftness", d.mask_softness),
            light_distance: p.int("LightDistance", d.light_distance),
            light_angle: p.int("LightAngle", d.light_angle),
            slope: p.int("SlopeType", d.slope),
            slope_multiplier: p.num("SlopeMultiplier", d.slope_multiplier),
            outer_color: p.color("OuterBevelColor", d.outer_color),
            down_color: p.color("DownBlendColor", d.down_color),
            hit_color: p.color("HitBlendColor", d.hit_color),
            hilite: p.color("HiliteColor", d.hilite),
            shadow: p.color("ShadowColor", d.shadow),
            face: p.color("EmbossFaceColor", d.face),
            show_object: p.text("ShowObject").map(|s| s.eq_ignore_ascii_case("true")).unwrap_or(false),
            button_state: ButtonState::from_code(p.int("ButtonState", 0)),
            glow_start: p.int("GlowStartDistance", d.glow_start),
            glow_width: p.int("GlowWidth", d.glow_width),
        }
    }

    /// Ausdehnung über das Objekt hinaus in px (Schritt 1, höchstens 60).
    pub fn extent(&self) -> i64 {
        let e = match self.kind {
            BevelType::Glow => self.mask_softness + self.glow_start + self.glow_width,
            BevelType::Raised | BevelType::Inset => self.mask_softness + self.width,
            _ => self.width,
        };
        e.clamp(0, 60)
    }
}

// ---------------------------------------------------------------- Winkel

const ATAN_BASE: [u32; 16] = [
    0x800000, 0x4138aa6, 0x7a00112, 0xb1ea1ba, 0xe894740, 0x11daa15c, 0x150e58fb, 0x18211ee6, 0x1b10a732, 0x1ddb95bc,
    0x208160cb, 0x23022e90, 0x255eb19d, 0x27980723, 0x29af98cb, 0x2ba70303,
];
const ATAN_SLOPE: [u32; 16] =
    [0x3939, 0x38c7, 0x37ea, 0x36aa, 0x3516, 0x333b, 0x312c, 0x2ef9, 0x2caf, 0x2a5d, 0x280d, 0x25c8, 0x2395, 0x2179, 0x1f77, 0x1d90];

/// Arkustangens in ganzen Grad aus Tabelle (`small < large`, `@006ced40`).
fn iatan(small: i64, large: i64) -> i64 {
    let l = (large & 0xFFFF) as u64;
    if l == 0 {
        return 0;
    }
    let t = (((small & 0xFFFF) as u64) << 16) / l;
    let i = ((t >> 12) as usize).min(15);
    ((((t & 0xFFF) as u32).wrapping_mul(ATAN_SLOPE[i]).wrapping_add(ATAN_BASE[i])) >> 24) as i64
}

/// Ganzzahliger Winkel des Vektors (x, y) in Grad, y nach unten (`@006d04a0`): (0, y<0) → −90,
/// sonst 0..359.
pub fn angle_int(x: i64, y: i64) -> i64 {
    if y == 0 {
        return if x >= 0 { 0 } else { 180 };
    }
    if x == 0 {
        return if y > 0 { 90 } else { -90 };
    }
    let (mut ax, mut ay) = (x.abs(), y.abs());
    while (ax | ay) > 0x7FFE {
        ax >>= 1;
        ay >>= 1;
    }
    let mut a = if ax == ay {
        45
    } else if ax < ay {
        (90 - iatan(ax, ay)) & 0xFFFF
    } else {
        iatan(ay, ax) & 0xFFFF
    };
    if x < 0 {
        a = (180 - a) & 0xFFFF;
    }
    if y < 0 {
        a = (360 - a) & 0xFFFF;
    }
    if a >= 0x8000 { a - 0x10000 } else { a }
}

/// Winkel → Byte (`@00778c40`/`@00778be0`): außen −180 mit Umbruch, dann `(a·255)/359` als
/// C-Division.
pub fn angle_byte(a: i64, outer: bool) -> u8 {
    let mut a = a;
    if outer {
        let b = a - 180;
        a = if b < 0 { a + 180 + ((-b - 1).div_euclid(360)) * 360 } else { b };
    }
    let v = a * 255;
    let q = v.abs() / 359;
    let q = if v >= 0 { q } else { -q };
    (q & 0xFF) as u8
}

// ---------------------------------------------------------------- Abstandsfeld

/// Abstandsfeld einer Alphamaske (Schritte 1–2). Zustand je Zelle: [`INSIDE`], [`OUTSIDE`], 0 =
/// Kante oder Abstand in 1/4096 px; dazu der Vektor zur nächsten Kantenzelle.
pub struct DistanceField {
    pub w: usize,
    pub h: usize,
    pub state: Vec<i64>,
    pub vx: Vec<i64>,
    pub vy: Vec<i64>,
}

const STEPS: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)];

impl DistanceField {
    /// Baut das Feld bis `width` Ringe (`outer`: von außen, Schwelle aus `edge`).
    pub fn build(alpha: &[u8], w: usize, h: usize, width: i64, outer: bool, edge: f64, ties: bool) -> DistanceField {
        let ins: Vec<bool> = if !outer {
            let mut t = (edge * 255.0).round_ties_even() as i64;
            if t == 255 {
                t = 254;
            }
            alpha.iter().map(|&a| a as i64 > t).collect()
        } else {
            let t = (((1.0 - edge) * 255.0).round_ties_even() as i64).max(1);
            alpha.iter().map(|&a| (a as i64) < t).collect()
        };
        let mut state = vec![OUTSIDE; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if ins[i] {
                    state[i] = INSIDE;
                    continue;
                }
                let mut nb = false;
                for &(dx, dy) in &STEPS {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h && ins[ny as usize * w + nx as usize] {
                        nb = true;
                        break;
                    }
                }
                if nb {
                    state[i] = 0;
                }
            }
        }
        let mut f = DistanceField { w, h, state, vx: vec![0; w * h], vy: vec![0; w * h] };
        f.propagate(width, ties);
        f
    }

    fn propagate(&mut self, width: i64, ties: bool) {
        let (w, h) = (self.w as i64, self.h as i64);
        for lvl in 0..width.max(0) {
            let (lo, hi) = (lvl << 12, (lvl << 12) + 0xFFF);
            let lim = (lvl + 2) * (lvl + 2);
            for y in 0..h {
                for x in 0..w {
                    let v = self.state[(y * w + x) as usize];
                    if v == INSIDE || v == OUTSIDE || v < lo || v > hi {
                        continue;
                    }
                    for &(qx, qy) in &STEPS {
                        let (nx, ny) = (x + qx, y + qy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < h && self.state[(ny * w + nx) as usize] == INSIDE {
                            self.update_cell(ny, nx, lim, ties);
                        }
                    }
                }
            }
        }
    }

    fn update_cell(&mut self, y: i64, x: i64, limit: i64, ties: bool) {
        let (w, h) = (self.w as i64, self.h as i64);
        let (mut cnt, mut bx, mut by, mut bd, mut sx, mut sy) = (0i64, 0i64, 0i64, 0i64, 0i64, 0i64);
        for &(qx, qy) in &STEPS {
            let (my, mx) = (y + qy, x + qx);
            if !(my >= 0 && my < h && mx >= 0 && mx < w) {
                continue;
            }
            let k = (my * w + mx) as usize;
            let s = self.state[k];
            if s == INSIDE || s == OUTSIDE {
                continue;
            }
            let (cx, cy) = (self.vx[k] + qx, self.vy[k] + qy);
            let d2 = cx * cx + cy * cy;
            if cnt == 0 || d2 < bd {
                cnt = 1;
                bx = cx;
                by = cy;
                bd = d2;
                sx = cx;
                sy = cy;
            } else if ties && d2 == bd {
                cnt += 1;
                sx += cx;
                sy += cy;
            }
        }
        if cnt == 0 || bd >= limit {
            return;
        }
        let dist = (bd as f64).sqrt();
        if ties && cnt == 2 {
            let l = ((sx * sx + sy * sy) as f64).sqrt();
            // Konstante wie im Original (`@00779640`), bewusst nicht `SQRT_2`.
            #[allow(clippy::approx_constant)]
            let s2 = 1.41421356;
            if dist * s2 <= l {
                let a = angle_int(sx, sy) as f64;
                let rx = a.to_radians().cos() * dist;
                let ry = a.to_radians().sin() * dist;
                bx = round_away(rx);
                by = round_away(ry);
            }
        }
        let k = (y * w + x) as usize;
        self.state[k] = (dist * 4096.0) as i64;
        self.vx[k] = bx;
        self.vy[k] = by;
    }

    /// Gültige Feldzelle (nicht innen/aus, nicht Kante)?
    #[inline]
    pub fn valid(&self, i: usize) -> bool {
        let s = self.state[i];
        s != INSIDE && s != OUTSIDE && s != 0
    }

    /// Abstand in px (nur für gültige Zellen sinnvoll).
    #[inline]
    pub fn dist(&self, i: usize) -> f64 {
        self.state[i] as f64 / 4096.0
    }
}

fn round_away(v: f64) -> i64 {
    if v >= 0.0 { (v + 0.5).floor() as i64 } else { (v - 0.5).ceil() as i64 }
}

/// Normalenwinkel je Zelle in Grad (Byte · 1,407843…) wie `bevel.normal_angles_fw`.
fn normal_angles(f: &DistanceField, outer: bool, soft: i64) -> Vec<f64> {
    let (w, h) = (f.w as i64, f.h as i64);
    let n = f.w * f.h;
    let mut ang: Vec<Option<i64>> = vec![None; n];
    for i in 0..n {
        let s = f.state[i];
        if s != INSIDE && s != OUTSIDE {
            ang[i] = Some(angle_int(f.vx[i], f.vy[i]));
        }
    }
    let mut out = vec![0.0f64; n];
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) as usize;
            let Some(ac) = ang[i] else { continue };
            let a = if soft > 0 {
                let (mut sx, mut sy) = (0i64, 0i64);
                for dy in -soft..=soft {
                    let yy = y + dy;
                    if yy < 0 || yy >= h {
                        continue;
                    }
                    for dx in -soft..=soft {
                        let xx = x + dx;
                        if xx < 0 || xx >= w {
                            continue;
                        }
                        let k = (yy * w + xx) as usize;
                        let Some(an) = ang[k] else { continue };
                        let mut d = ac - an;
                        if d < -180 {
                            d = d + (-181 - d).div_euclid(360) * 360 + 360;
                        }
                        if d > 180 {
                            d = d - 360 - (d - 181).div_euclid(360) * 360;
                        }
                        if d.abs() > 90 {
                            continue;
                        }
                        let wgt = soft + 1 - dx.max(dy).abs();
                        sx += f.vx[k] * wgt;
                        sy += f.vy[k] * wgt;
                    }
                }
                angle_int(sx, sy)
            } else {
                ac
            };
            out[i] = angle_byte(a, outer) as f64 * 1.407843137254902;
        }
    }
    out
}

/// Profilfunktion `SlopeType` (`@00778dc0`), `t = d/Breite`.
pub fn slope(kind: i64, t: f64) -> f64 {
    match kind {
        1 => 1.0 - t,
        2 => t,
        3 => {
            if t > 0.65 {
                -1.0
            } else {
                1.0
            }
        }
        4 => {
            if t > 0.7 {
                -1.0
            } else if t > 0.4 {
                0.0
            } else {
                1.0
            }
        }
        5 => x87::cos_pi(t, 1),
        6 => {
            let v = x87::cos_pi(t, 3);
            if 0.33333 < t && t < 0.66667 { -v } else { v }
        }
        _ => 1.0,
    }
}

fn wrap180(mut a: f64) -> f64 {
    let mut guard = 0;
    while a < -180.0 && guard < 100 {
        a += 360.0;
        guard += 1;
    }
    while a > 180.0 && guard < 200 {
        a -= 360.0;
        guard += 1;
    }
    a
}

fn round_half_up(v: f64) -> u32 {
    (v + 0.5).floor().clamp(0.0, 1.0e6) as u32
}

/// Licht- und Schattenmaske (Schritt 4) für Abschrägung und Prägung.
fn masks(p: &BevelParams, f: &DistanceField, ang: &[f64]) -> (Vec<u8>, Vec<u8>) {
    let n = f.w * f.h;
    let mut hi = vec![0u8; n];
    let mut sh = vec![0u8; n];
    let width = p.width.max(1) as f64;
    let c = p.contrast as f64 / 100.0;
    for i in 0..n {
        if !f.valid(i) {
            continue;
        }
        let dd = f.dist(i);
        match p.kind {
            BevelType::Inner | BevelType::Outer => {
                if dd >= width + 1.0 {
                    continue;
                }
                let mut t = dd / width;
                if p.kind == BevelType::Outer {
                    t = 1.0 - t;
                }
                let mut s = slope(p.slope, t) * p.slope_multiplier;
                if matches!(p.button_state, ButtonState::Down | ButtonState::OverDown) {
                    s = -s;
                }
                if s.abs() > 1.0 {
                    s = 1.0 / s;
                }
                let mut a = p.light_angle as f64 + ang[i];
                if s < 0.0 {
                    a -= 180.0;
                }
                let a = wrap180(a);
                let h1 = round_half_up((1.0 - a.abs() / 90.0).max(0.0) * 255.0);
                let s1 = round_half_up((a.abs() / 120.0 - 0.5).max(0.0) * 255.0);
                hi[i] = (x87::mul_trunc(h1, s.abs(), c) & 255) as u8;
                sh[i] = (x87::mul_trunc(s1, s.abs(), c) & 255) as u8;
            }
            BevelType::Raised | BevelType::Inset => {
                let sgn: f64 = if dd - 1.0 <= p.width as f64 {
                    if p.kind == BevelType::Inset { -1.0 } else { 1.0 }
                } else {
                    0.0
                };
                let mut a = p.light_angle as f64 + ang[i];
                if sgn < 0.0 {
                    a -= 180.0;
                }
                let a = wrap180(a);
                let h1 = round_half_up((1.0 - a.abs() / 90.0).max(0.0) * 255.0) as f64;
                let s1 = round_half_up((a.abs() / 90.0 - 1.0).max(0.0) * 255.0) as f64;
                hi[i] = ((h1 * sgn.abs() * c) as i64 & 255) as u8;
                sh[i] = ((s1 * sgn.abs() * c) as i64 & 255) as u8;
            }
            BevelType::Glow => {}
        }
    }
    (hi, sh)
}

fn blur_vec(v: &[u8], frame: fp_geom::IRect, r: i64) -> Vec<u8> {
    if r <= 0 {
        return v.to_vec();
    }
    match Mask::from_raw(frame, v.to_vec()) {
        Ok(m) => tent_same(&m, r.min(1024) as u32).data().to_vec(),
        Err(_) => v.to_vec(),
    }
}

/// Abschrägung/Prägung/Glühen auf ein Objektbild. Ergebnis auf Alpha > 0 zugeschnitten.
pub fn bevel(obj: &RgbaImage, p: &BevelParams) -> RgbaImage {
    if obj.is_empty() {
        return obj.clone();
    }
    if p.kind == BevelType::Glow {
        return glow(obj, p);
    }
    let ext = p.extent();
    let soft_a = p.angle_softness.clamp(0, 10);
    let blur_r = if matches!(p.kind, BevelType::Inner | BevelType::Outer) { soft_a / 2 } else { p.mask_softness };
    let m = (ext + 2) as i32;
    let frame = obj.bounds().inflate(m, m);
    let img = on_frame(obj, frame);
    let (w, h) = (frame.width() as usize, frame.height() as usize);
    let alpha: Vec<u8> = img.data().iter().map(|q| q[3]).collect();
    let outer = p.kind != BevelType::Inner;
    let f = DistanceField::build(&alpha, w, h, ext, outer, p.edge, true);
    let ang = normal_angles(&f, outer, soft_a);
    let (mut hi, mut sh) = masks(p, &f, &ang);
    if blur_r > 0 {
        hi = blur_vec(&hi, frame, blur_r);
        sh = blur_vec(&sh, frame, blur_r);
    }
    let extf = ext as f64;
    let n = w * h;
    let mut out = img.clone();
    let data_in = img.data();
    let dout = out.data_mut();
    let base_alpha_outer = p.outer_color.a;
    for i in 0..n {
        let valid = f.valid(i);
        let dd = if valid { f.dist(i) } else { 0.0 };
        let fade = if valid && dd > extf { (1.0 - (dd - extf)).clamp(0.0, 1.0) } else { 1.0 };
        let band = valid && dd < extf + 1.0;
        let pa = data_in[i][3];
        let mut region = band || pa > 0;
        let (mut hf, mut sf) = (hi[i], sh[i]);
        if fade < 1.0 {
            hf = (hi[i] as f64 * fade).floor() as u8;
            sf = (sh[i] as f64 * fade).floor() as u8;
        }
        if matches!(p.kind, BevelType::Inner | BevelType::Outer) {
            region = band;
        }
        if p.kind == BevelType::Outer {
            hf = hi[i];
            sf = sh[i];
        }
        let lay0 = Rgba8::new(p.shadow.r, p.shadow.g, p.shadow.b, sf);
        let lay = orig_over(Rgba8::new(p.hilite.r, p.hilite.g, p.hilite.b, hf), lay0);
        let src = px(data_in[i]);
        let res = match p.kind {
            BevelType::Inner => {
                let r = orig_over(lay, src);
                Rgba8::new(r.r, r.g, r.b, src.a)
            }
            BevelType::Outer => {
                let ba = if !valid {
                    0
                } else if fade < 1.0 {
                    (fade * 255.0).floor() as u8
                } else {
                    base_alpha_outer
                };
                let base = if valid { Rgba8::new(p.outer_color.r, p.outer_color.g, p.outer_color.b, ba) } else { Rgba8::TRANSPARENT };
                let r = orig_over(lay, base);
                let r = orig_over(src, r);
                Rgba8::new(r.r, r.g, r.b, base.a)
            }
            BevelType::Raised | BevelType::Inset => {
                let base = if p.show_object { src } else { p.face };
                let ov_a = if pa > 0 { div255(lay.a as u32 * (255 - pa as u32)) as u8 } else { lay.a };
                let ov = Rgba8::new(lay.r, lay.g, lay.b, ov_a);
                let mut r = if pa > 0 { orig_over(ov, base) } else { ov };
                if pa > 0 && !band {
                    r = base;
                }
                r
            }
            BevelType::Glow => src,
        };
        let mut o = if region { res } else { src };
        if matches!(p.kind, BevelType::Raised | BevelType::Inset) && !region {
            o = if p.show_object { src } else { Rgba8::TRANSPARENT };
        }
        // Schaltflächenstatus (nur Abschrägungen): Farbe darüber, Alpha bleibt.
        if matches!(p.kind, BevelType::Inner | BevelType::Outer) && p.button_state != ButtonState::Up && (band || pa > 0) {
            let c = if matches!(p.button_state, ButtonState::Over | ButtonState::OverDown) { p.hit_color } else { p.down_color };
            let t = orig_over(Rgba8::new(c.r, c.g, c.b, c.a), o);
            o = Rgba8::new(t.r, t.g, t.b, o.a);
        }
        dout[i] = o.to_array();
    }
    crate::util::clear_transparent(&mut out);
    out.trimmed()
}

/// Glühen (`BevelType 4`, `@0077d9b0`/`@00778eb0`/`@0077ba90`).
pub fn glow(obj: &RgbaImage, p: &BevelParams) -> RgbaImage {
    let e = (p.glow_width + p.glow_start + p.mask_softness).clamp(0, 60);
    let m = (e + 2) as i32;
    let frame = obj.bounds().inflate(m, m);
    let img = on_frame(obj, frame);
    let (w, h) = (frame.width() as usize, frame.height() as usize);
    let alpha: Vec<u8> = img.data().iter().map(|q| q[3]).collect();
    let f = DistanceField::build(&alpha, w, h, e, true, p.edge, false);
    let mval = ((p.contrast as f64 / 100.0 * 255.0) as i64).clamp(0, 255) as u8;
    let (g0, gw) = (p.glow_start as f64, p.glow_width as f64);
    let ef = e as f64;
    let n = w * h;
    let mut hi = vec![0u8; n];
    let mut reg = vec![false; n];
    let mut inside = vec![false; n];
    for i in 0..n {
        let valid = f.valid(i);
        let dd = if valid { f.dist(i) } else { 1e9 };
        reg[i] = valid && dd < ef + 1.0;
        // Band (Versatz, Versatz + Breite + 1]: innen streng größer (FX-1, Messreihe re\23 „halo“/„gross“:
        // mit `>=` lag das Band 1 px zu weit innen, Abweichung bis 26 Stufen; mit `>` pixelgleich).
        if reg[i] && dd - 1.0 <= g0 + gw && dd > g0 {
            hi[i] = mval;
        }
        inside[i] = !valid && alpha[i] > 0;
        if p.glow_start == 0 && inside[i] {
            hi[i] = mval;
        }
    }
    if p.mask_softness > 0 {
        hi = blur_vec(&hi, frame, p.mask_softness);
    }
    let mut out = img.clone();
    let din = img.data();
    let dout = out.data_mut();
    for i in 0..n {
        if !(reg[i] || inside[i]) {
            continue;
        }
        let dd = if f.valid(i) { f.dist(i) } else { 1e9 };
        let fade = if reg[i] && dd > ef { 1.0 - (dd - ef) } else { 1.0 };
        let hf = if fade < 1.0 { (hi[i] as f64 * fade).floor().clamp(0.0, 255.0) as u8 } else { hi[i] };
        let gl = Rgba8::new(p.outer_color.r, p.outer_color.g, p.outer_color.b, hf);
        dout[i] = orig_over(px(din[i]), gl).to_array();
    }
    crate::util::clear_transparent(&mut out);
    out.trimmed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles() {
        assert_eq!(angle_int(1, 0), 0);
        assert_eq!(angle_int(0, 1), 90);
        assert_eq!(angle_int(0, -1), -90);
        assert_eq!(angle_int(-1, 0), 180);
        assert_eq!(angle_int(1, 1), 45);
        assert_eq!(angle_int(-1, -1), 225);
        assert_eq!(angle_byte(-90, false), 193);
        assert_eq!(angle_byte(0, true), 127);
        assert_eq!(angle_byte(-180, false), 129);
    }

    #[test]
    fn distance_field_square() {
        // 7×7-Feld mit deckendem 5×5 bei (1,1): Saat = Ring außen, innen Abstände ab 1.
        let mut a = vec![0u8; 49];
        for y in 1..6 {
            for x in 1..6 {
                a[y * 7 + x] = 255;
            }
        }
        let f = DistanceField::build(&a, 7, 7, 3, false, 0.0, true);
        assert_eq!(f.state[0], 0);
        assert!((f.dist(7 + 1) - 1.0).abs() < 1e-9);
        assert!((f.dist(2 * 7 + 2) - 2.0).abs() < 1e-9);
        assert!(f.valid(3 * 7 + 3));
    }
}
