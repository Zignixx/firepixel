//! # Zweites Verfahren: Konturen nach Selinger („Potrace-Verfahren“, VT-6)
//!
//! Eigenständige Umsetzung nach dem veröffentlichten Fachartikel P. Selinger, „Potrace: a
//! polygon-based tracing algorithm“ (2003). Es wird **kein Code und keine Bibliothek** des
//! Programms Potrace (GPL) verwendet; die Schritte des Artikels sind hier neu hergeleitet:
//!
//! 1. **Binarisierung:** Schwarzweiß über die Helligkeitsschwelle. Farbe: Farbreduktion (k-Means) auf
//!    höchstens `colors` Farben, dann **je Farbe eine Maske, gestapelt**: Die Farben werden nach
//!    Fläche sortiert (größte zuunterst); die Maske einer Ebene enthält ihre Farbe und alle Farben
//!    darüber. Die unterste Ebene deckt damit alle deckenden Pixel ab, und zwischen benachbarten
//!    Farbflächen entstehen keine Haarrisse.
//! 2. **Pfadzerlegung:** Die Grenzen zwischen gesetzten und leeren Pixeln werden als geschlossene
//!    Pfade auf den Pixelecken abgelaufen (gesetzte Pixel rechts). Wo sich zwei Pixel nur diagonal
//!    berühren, entscheidet die Abbiegeregel ([`TurnPolicy`]). Jeder Pfad wird im Arbeitsbild
//!    invertiert (XOR); Löcher werden so eigene Pfade, und die Fläche ist genau die
//!    gerade/ungerade-Füllung aller Pfade. Pfade mit höchstens `turdSize` Pixeln Fläche entfallen.
//! 3. **Optimales Polygon:** Ein Teilpfad ist gerade, wenn höchstens drei der vier Richtungen
//!    vorkommen und eine Gerade im Abstand ≤ ½ (Maximumnorm) an allen Punkten vorbeiläuft
//!    (geprüft als Kegel der Richtungen einer Geraden durch den ersten Punkt, die jedes Quadrat
//!    ±1 um die folgenden Punkte trifft; nur an den Knickpunkten, dazwischen binäre Suche). Eine Polygonkante i → j
//!    ist zulässig, wenn der um je einen Punkt verlängerte Teilpfad gerade ist. Gesucht wird die
//!    Polygonfolge mit den wenigsten Kanten, darunter die mit der kleinsten Strafe
//!    (Summe von √(mittleres Abstandsquadrat · Länge²) je Kante), mit dem Startpunkt als Ecke.
//! 4. **Eckpunkte verfeinern:** Jede Kante bekommt die Ausgleichsgerade ihrer Pfadpunkte; jeder
//!    Polygonpunkt wandert an die Stelle im Quadrat ±½ um den ursprünglichen Punkt, an der die
//!    Summe der Abstandsquadrate zu beiden Geraden am kleinsten ist.
//! 5. **Glätten und Ecken:** Zwischen den Kantenmitten entsteht je Polygonpunkt ein Bézierbogen,
//!    dessen Henkel auf den Polygonkanten liegen (Anteil α). α ist so gewählt, dass der Bogen das
//!    Quadrat ±½ um den Punkt gerade berührt: α = 4/3 · (1 − e/D) (D Abstand des Punkts zur Sehne,
//!    e halbe Quadratbreite quer zur Sehne). Ab α ≥ `alphaMax` bleibt der Punkt eine **Ecke**,
//!    sonst wird α auf 0,55…1 begrenzt.
//! 6. **Kurvenoptimierung** (`optCurve`): Aufeinanderfolgende Bögen gleicher Krümmungsrichtung
//!    mit zusammen weniger als 179° Drehung werden zu **einem** Bogen zusammengefasst – gleiche
//!    Tangenten an den Enden, Henkellänge aus gleicher Fläche –, wenn er höchstens `optTolerance`
//!    Pixel von den Mitten und Übergängen der ursprünglichen Bögen abweicht. Dynamische
//!    Programmierung: wenigste Bögen, dann kleinste Abweichung.

use crate::{check, luma, over_white, ColorMode, Mapper, Progress, Stage, TraceError, TraceOptions, TracedPath};
use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode};
use fp_raster::RgbaImage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;

/// Ab diesem `alphaMax` gibt es keine Ecken mehr (α bleibt immer darunter).
pub const ALPHA_MAX_LIMIT: f64 = 4.0 / 3.0;
/// Größte zusammengefasste Drehung eines Bogens (Grad).
const MAX_JOIN_TURN_DEG: f64 = 179.0;

/// Abbiegeregel an Stellen, an denen sich zwei Pixel nur diagonal berühren.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TurnPolicy {
    /// Die in der Umgebung seltenere Farbe wird verbunden (Vorgabe).
    #[default]
    Minority,
    /// Die in der Umgebung häufigere Farbe wird verbunden.
    Majority,
    /// Gesetzte (schwarze) Pixel werden verbunden.
    Black,
    /// Leere (weiße) Pixel werden verbunden.
    White,
    /// Immer links abbiegen.
    Left,
    /// Immer rechts abbiegen.
    Right,
}

impl TurnPolicy {
    /// Alle Regeln in Dialogreihenfolge.
    pub const ALL: [TurnPolicy; 6] = [TurnPolicy::Minority, TurnPolicy::Majority, TurnPolicy::Black, TurnPolicy::White, TurnPolicy::Left, TurnPolicy::Right];

    /// Name im JSON (Befehlsparameter).
    pub fn name(self) -> &'static str {
        match self {
            TurnPolicy::Minority => "minority",
            TurnPolicy::Majority => "majority",
            TurnPolicy::Black => "black",
            TurnPolicy::White => "white",
            TurnPolicy::Left => "left",
            TurnPolicy::Right => "right",
        }
    }

    /// Regel zu einem JSON-Namen.
    pub fn from_name(s: &str) -> Option<TurnPolicy> {
        TurnPolicy::ALL.into_iter().find(|p| p.name() == s)
    }
}

/// Einstellungen des Konturverfahrens (Namen wie im Fachartikel).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PotraceOptions {
    /// Flecken unterdrücken: Pfade mit höchstens so vielen Pixeln Fläche entfallen (0–10000).
    pub turd_size: u32,
    /// Abbiegeregel an diagonalen Berührungen.
    pub turn_policy: TurnPolicy,
    /// Eckenschwelle α (0 = nur Ecken, ab 4/3 keine Ecken; Vorgabe 1).
    pub alpha_max: f64,
    /// Aufeinanderfolgende Bögen zusammenfassen.
    pub opt_curve: bool,
    /// Höchstabweichung beim Zusammenfassen in Pixeln (0–5; Vorgabe 0,2).
    pub opt_tolerance: f64,
}

impl Default for PotraceOptions {
    fn default() -> Self {
        PotraceOptions { turd_size: 2, turn_policy: TurnPolicy::Minority, alpha_max: 1.0, opt_curve: true, opt_tolerance: 0.2 }
    }
}

impl PotraceOptions {
    /// Alle Werte in ihren Bereichen (NaN → Vorgabe).
    pub fn validated(&self) -> PotraceOptions {
        let d = PotraceOptions::default();
        PotraceOptions {
            turd_size: self.turd_size.min(10_000),
            alpha_max: if self.alpha_max.is_finite() { self.alpha_max.clamp(0.0, ALPHA_MAX_LIMIT) } else { d.alpha_max },
            opt_tolerance: if self.opt_tolerance.is_finite() { self.opt_tolerance.clamp(0.0, 5.0) } else { d.opt_tolerance },
            ..self.clone()
        }
    }
}

// ------------------------------------------------------------------ Einstieg

/// Zeichnet ein (vorbereitetes, ggf. verkleinertes) Bild nach dem Konturverfahren nach.
pub(crate) fn trace(img: &RgbaImage, o: &TraceOptions, map: &Mapper, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<Vec<TracedPath>, TraceError> {
    let (w, h) = (img.width(), img.height());
    let po = o.potrace.validated();
    let mut px = img.as_bytes().to_vec();
    let mut transparent = vec![false; w * h];
    for (i, p) in px.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        if o.ignore_transparent && p[3] < o.alpha_threshold {
            transparent[i] = true;
        } else if o.ignore_transparent {
            p[3] = 255;
        } else {
            over_white(p);
        }
    }
    check(cancel)?;
    progress(Progress { stage: Stage::Cluster, fraction: 0.05 });
    let mut out = Vec::new();
    match o.color_mode {
        ColorMode::BlackWhite => {
            let mask: Vec<bool> = px.as_chunks::<4>().0.iter().zip(&transparent).map(|(p, &t)| !t && luma(p[0], p[1], p[2]) < o.threshold as u32).collect();
            let pixels = mask.iter().filter(|&&b| b).count();
            progress(Progress { stage: Stage::Paths, fraction: 0.15 });
            let contours = trace_mask(mask, w, h, &po, map, cancel)?;
            if !contours.is_empty() {
                out.push(TracedPath { contours, color: Rgba8::new(0, 0, 0, 255), pixels });
            }
        }
        ColorMode::Color => {
            let (ranks, palette) = color_ranks(&mut px, &transparent, o.colors as usize);
            check(cancel)?;
            let total = palette.len().max(1);
            for (layer, &(rgb, pixels)) in palette.iter().enumerate() {
                check(cancel)?;
                progress(Progress { stage: Stage::Paths, fraction: 0.15 + 0.85 * layer as f32 / total as f32 });
                // gestapelt: diese Farbe und alle darüber liegenden
                let mask: Vec<bool> = ranks.iter().map(|&r| r != u16::MAX && r as usize >= layer).collect();
                let contours = trace_mask(mask, w, h, &po, map, cancel)?;
                if !contours.is_empty() {
                    out.push(TracedPath { contours, color: Rgba8::new(rgb[0], rgb[1], rgb[2], 255), pixels });
                }
            }
        }
    }
    Ok(out)
}

/// Farbreduktion (nur wenn nötig) und Rang jeder Farbe nach Fläche (0 = größte, zuunterst).
/// Ergebnis: Rang je Pixel (`u16::MAX` = transparent) und die Palette `(Farbe, Pixel)` nach Rang.
fn color_ranks(px: &mut [u8], transparent: &[bool], colors: usize) -> (Vec<u16>, Vec<([u8; 3], usize)>) {
    let limit = if colors >= 2 { colors.min(256) } else { 256 };
    let count = |px: &[u8]| {
        let mut m: HashMap<[u8; 3], usize> = HashMap::new();
        for (p, &t) in px.as_chunks::<4>().0.iter().zip(transparent) {
            if !t {
                *m.entry([p[0], p[1], p[2]]).or_default() += 1;
            }
        }
        m
    };
    let mut counts = count(px);
    if counts.len() > limit {
        reduce_kmeans(px, transparent, limit);
        counts = count(px);
    }
    let mut palette: Vec<([u8; 3], usize)> = counts.into_iter().collect();
    palette.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let rank: HashMap<[u8; 3], u16> = palette.iter().enumerate().map(|(i, (c, _))| (*c, i as u16)).collect();
    let ranks = px.as_chunks::<4>().0.iter().zip(transparent).map(|(p, &t)| if t { u16::MAX } else { rank.get(&[p[0], p[1], p[2]]).copied().unwrap_or(u16::MAX) }).collect();
    (ranks, palette)
}

/// Farbreduktion auf höchstens `k` Farben für die Farbebenen: k-Means über ein 5-Bit-Histogramm.
/// Startfarben nacheinander die Fächer mit dem größten Produkt aus Pixelzahl und Abstandsquadrat
/// zu den bisherigen (zuerst die häufigste Farbe) – so bekommen kleine, aber deutlich andere
/// Flächen (Logo-Farben) eine eigene Farbe, Kantenmischtöne nicht. Danach 8 Lloyd-Schritte.
fn reduce_kmeans(px: &mut [u8], transparent: &[bool], k: usize) {
    let bin = |p: &[u8; 4]| ((p[0] as usize >> 3) << 10) | ((p[1] as usize >> 3) << 5) | (p[2] as usize >> 3);
    let mut sums = vec![[0f64; 4]; 1 << 15];
    for (p, &t) in px.as_chunks::<4>().0.iter().zip(transparent) {
        if !t {
            let s = &mut sums[bin(p)];
            s[0] += p[0] as f64;
            s[1] += p[1] as f64;
            s[2] += p[2] as f64;
            s[3] += 1.0;
        }
    }
    let bins: Vec<(usize, [f64; 3], f64)> = sums.iter().enumerate().filter(|(_, s)| s[3] > 0.0).map(|(i, s)| (i, [s[0] / s[3], s[1] / s[3], s[2] / s[3]], s[3])).collect();
    if bins.is_empty() {
        return;
    }
    let d2 = |a: &[f64; 3], b: &[f64; 3]| (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);
    let mut centers: Vec<[f64; 3]> = Vec::with_capacity(k);
    let mut near = vec![f64::INFINITY; bins.len()];
    for _ in 0..k.min(bins.len()) {
        let pick = if centers.is_empty() {
            bins.iter().enumerate().max_by(|a, b| a.1.2.total_cmp(&b.1.2)).map(|(i, _)| i)
        } else {
            bins.iter().enumerate().map(|(i, b)| (i, b.2 * near[i])).max_by(|a, b| a.1.total_cmp(&b.1)).filter(|(_, s)| *s > 0.0).map(|(i, _)| i)
        };
        let Some(i) = pick else { break };
        let c = bins[i].1;
        centers.push(c);
        for (j, b) in bins.iter().enumerate() {
            near[j] = near[j].min(d2(&b.1, &c));
        }
    }
    let nearest = |c: &[f64; 3], centers: &[[f64; 3]]| (0..centers.len()).min_by(|&a, &b| d2(c, &centers[a]).total_cmp(&d2(c, &centers[b]))).unwrap_or(0);
    for _ in 0..8 {
        let mut acc = vec![[0f64; 4]; centers.len()];
        for b in &bins {
            let a = &mut acc[nearest(&b.1, &centers)];
            for (s, v) in a.iter_mut().zip(b.1) {
                *s += v * b.2;
            }
            a[3] += b.2;
        }
        for (c, a) in centers.iter_mut().zip(&acc) {
            if a[3] > 0.0 {
                *c = [a[0] / a[3], a[1] / a[3], a[2] / a[3]];
            }
        }
    }
    let mut map = vec![[0u8; 3]; 1 << 15];
    for b in &bins {
        let c = centers[nearest(&b.1, &centers)];
        map[b.0] = [c[0].round() as u8, c[1].round() as u8, c[2].round() as u8];
    }
    for (p, &t) in px.as_chunks_mut::<4>().0.iter_mut().zip(transparent) {
        if !t {
            let c = map[bin(p)];
            p[..3].copy_from_slice(&c);
        }
    }
}

/// Eine Maske → geschlossene Bézier-Konturen in Dokumentkoordinaten (gerade/ungerade).
fn trace_mask(mask: Vec<bool>, w: usize, h: usize, po: &PotraceOptions, map: &Mapper, cancel: &AtomicBool) -> Result<Vec<BezierContour>, TraceError> {
    let orig = mask.clone();
    let mut bm = Bits { w: w as i32, h: h as i32, d: mask };
    let paths = find_paths(&mut bm, &orig, po, cancel)?;
    let mut out = Vec::with_capacity(paths.len());
    for (k, p) in paths.iter().enumerate() {
        if k % 64 == 0 {
            check(cancel)?;
        }
        let nodes = map_nodes(&curve_nodes(&p.pts, po), map);
        if nodes.len() >= 2 {
            out.push(BezierContour::new(nodes, true));
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ Pfadzerlegung

/// Arbeitsbild (gesetzt = Vordergrund).
struct Bits {
    w: i32,
    h: i32,
    d: Vec<bool>,
}

impl Bits {
    fn get(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.d[(y * self.w + x) as usize]
    }

    /// Pixel der Zeile `y` im Bereich [min(a, b), max(a, b)) invertieren.
    fn flip_row(&mut self, y: i32, a: i32, b: i32) {
        let (lo, hi) = (a.min(b).max(0), a.max(b).min(self.w));
        let base = (y * self.w) as usize;
        for v in &mut self.d[base + lo as usize..base + hi as usize] {
            *v = !*v;
        }
    }
}

/// Geschlossener Pfad auf den Pixelecken (Einheitsschritte), Fläche mit Vorzeichen.
struct RawPath {
    pts: Vec<(i32, i32)>,
    area: i64,
}

fn find_paths(bm: &mut Bits, orig: &[bool], po: &PotraceOptions, cancel: &AtomicBool) -> Result<Vec<RawPath>, TraceError> {
    let mut out = Vec::new();
    let w = bm.w as usize;
    let mut idx = 0usize;
    let mut found = 0u32;
    while idx < bm.d.len() {
        if !bm.d[idx] {
            idx += 1;
            continue;
        }
        let (x, y) = ((idx % w) as i32, (idx / w) as i32);
        let path = trace_path(bm, x, y, po.turn_policy, orig[idx]);
        xor_path(bm, &path);
        if path.area.unsigned_abs() > po.turd_size as u64 {
            out.push(path);
        }
        found += 1;
        if found.is_multiple_of(128) {
            check(cancel)?;
        }
    }
    Ok(out)
}

/// Läuft die Grenze ab der linken oberen Ecke des Pixels (x0, y0) ab (erster gesetzter Pixel in
/// Leserichtung: oben und links davon ist alles leer). Gesetzte Pixel liegen rechts der Laufrichtung.
fn trace_path(bm: &Bits, x0: i32, y0: i32, policy: TurnPolicy, positive: bool) -> RawPath {
    let start = (x0, y0);
    let (mut p, mut d) = (start, (1i32, 0i32));
    let mut pts = Vec::new();
    let mut area = 0i64;
    loop {
        pts.push(p);
        area += p.0 as i64 * d.1 as i64;
        p = (p.0 + d.0, p.1 + d.1);
        if p == start {
            break;
        }
        // n1 = links, n2 = rechts der Laufrichtung (y nach unten)
        let (n1, n2) = ((d.1, -d.0), (-d.1, d.0));
        // Pixel an der nächsten Kante auf der Seite n: Mitte p + (d + n)/2
        let pix = |v: (i32, i32)| bm.get(p.0 + (v.0 - 1) / 2, p.1 + (v.1 - 1) / 2);
        let ahead_right = pix((d.0 + n2.0, d.1 + n2.1));
        let ahead_left = pix((d.0 + n1.0, d.1 + n1.1));
        d = match (ahead_right, ahead_left) {
            (true, true) => n1,
            (false, false) => n2,
            (true, false) => d,
            (false, true) => {
                if turn_left(bm, p, policy, positive) {
                    n1
                } else {
                    n2
                }
            }
        };
    }
    RawPath { pts, area }
}

/// Diagonale Berührung: links abbiegen verbindet die gesetzten Pixel, rechts die leeren.
/// `positive`: Der Pfad begann auf einem im Original gesetzten Pixel (in Löchern ist das
/// Arbeitsbild invertiert).
fn turn_left(bm: &Bits, p: (i32, i32), policy: TurnPolicy, positive: bool) -> bool {
    match policy {
        TurnPolicy::Black => positive,
        TurnPolicy::White => !positive,
        TurnPolicy::Left => true,
        TurnPolicy::Right => false,
        TurnPolicy::Majority => majority_set(bm, p),
        TurnPolicy::Minority => !majority_set(bm, p),
    }
}

/// Sind in der Umgebung des Eckpunkts mehr gesetzte als leere Pixel? Ringe mit Radius 2, 3, 4,
/// bis einer eine Mehrheit hat.
fn majority_set(bm: &Bits, p: (i32, i32)) -> bool {
    for r in 2..=4 {
        let mut ct = 0i32;
        for j in -r..r {
            for i in -r..r {
                if i == -r || i == r - 1 || j == -r || j == r - 1 {
                    ct += if bm.get(p.0 + i, p.1 + j) { 1 } else { -1 };
                }
            }
        }
        if ct != 0 {
            return ct > 0;
        }
    }
    false
}

/// Inneres des Pfads invertieren: je senkrechtem Schritt die Zeile zwischen dem Schritt und der
/// Spalte des Startpunkts (die Paare je Zeile heben sich außerhalb auf).
fn xor_path(bm: &mut Bits, path: &RawPath) {
    let x0 = path.pts[0].0;
    let n = path.pts.len();
    for k in 0..n {
        let (a, b) = (path.pts[k], path.pts[(k + 1) % n]);
        if a.0 == b.0 && a.1 != b.1 {
            bm.flip_row(a.1.min(b.1), a.0, x0);
        }
    }
}

// ------------------------------------------------------------------ Geometrie

#[derive(Clone, Copy, Debug, PartialEq)]
struct P {
    x: f64,
    y: f64,
}

impl P {
    fn new(x: f64, y: f64) -> P {
        P { x, y }
    }
    fn add(self, o: P) -> P {
        P::new(self.x + o.x, self.y + o.y)
    }
    fn sub(self, o: P) -> P {
        P::new(self.x - o.x, self.y - o.y)
    }
    fn mul(self, f: f64) -> P {
        P::new(self.x * f, self.y * f)
    }
    fn cross(self, o: P) -> f64 {
        self.x * o.y - self.y * o.x
    }
    fn dot(self, o: P) -> f64 {
        self.x * o.x + self.y * o.y
    }
    fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
    fn mid(self, o: P) -> P {
        P::new((self.x + o.x) / 2.0, (self.y + o.y) / 2.0)
    }
    fn lerp(self, o: P, t: f64) -> P {
        self.add(o.sub(self).mul(t))
    }
}

fn bezier(p0: P, p1: P, p2: P, p3: P, t: f64) -> P {
    let m = 1.0 - t;
    let (a, b, c, d) = (m * m * m, 3.0 * m * m * t, 3.0 * m * t * t, t * t * t);
    P::new(a * p0.x + b * p1.x + c * p2.x + d * p3.x, a * p0.y + b * p1.y + c * p2.y + d * p3.y)
}

/// Abstand eines Punkts zu einer Strecke.
fn seg_dist(p: P, a: P, b: P) -> f64 {
    let d = b.sub(a);
    let l2 = d.dot(d);
    let t = if l2 > 0.0 { (p.sub(a).dot(d) / l2).clamp(0.0, 1.0) } else { 0.0 };
    p.sub(a.add(d.mul(t))).len()
}

// ------------------------------------------------------------------ Optimales Polygon

/// Präfixsummen über die zweimal durchlaufenen Pfadpunkte (relativ zum ersten Punkt).
struct Sums {
    x: Vec<f64>,
    y: Vec<f64>,
    xx: Vec<f64>,
    xy: Vec<f64>,
    yy: Vec<f64>,
}

impl Sums {
    fn new(pts: &[P]) -> Sums {
        let n = pts.len();
        let mut s = Sums { x: vec![0.0], y: vec![0.0], xx: vec![0.0], xy: vec![0.0], yy: vec![0.0] };
        for k in 0..2 * n {
            let p = pts[k % n];
            s.x.push(s.x[k] + p.x);
            s.y.push(s.y[k] + p.y);
            s.xx.push(s.xx[k] + p.x * p.x);
            s.xy.push(s.xy[k] + p.x * p.y);
            s.yy.push(s.yy[k] + p.y * p.y);
        }
        s
    }

    /// Summen über die Punkte i..=j (j < 2n): (Anzahl, Σx, Σy, Σxx, Σxy, Σyy).
    fn range(&self, i: usize, j: usize) -> (f64, f64, f64, f64, f64, f64) {
        let d = |v: &Vec<f64>| v[j + 1] - v[i];
        ((j - i + 1) as f64, d(&self.x), d(&self.y), d(&self.xx), d(&self.xy), d(&self.yy))
    }
}

/// Richtung eines Einheitsschritts als Index 0–3.
fn dir_index(dx: i32, dy: i32) -> usize {
    match (dx, dy) {
        (1, _) => 0,
        (_, 1) => 1,
        (-1, _) => 2,
        _ => 3,
    }
}

/// Je Punkt i: größte Länge r (Schritte), sodass die Punkte i..=i+r einen geraden Teilpfad bilden.
fn straight_reach(pts: &[(i32, i32)]) -> Vec<usize> {
    let n = pts.len();
    let at = |k: usize| pts[k % n];
    // Länge des geraden Laufs ab jedem Schritt k → k+1 (gleiche Richtung), zyklisch
    let step_dir = |k: usize| {
        let (a, b) = (at(k), at(k + 1));
        dir_index(b.0 - a.0, b.1 - a.1)
    };
    let mut run = vec![1usize; n];
    {
        // rückwärts über zwei Umläufe: im zweiten sind alle Läufe vollständig
        let mut next_run = 1usize;
        for k in (0..2 * n).rev() {
            let r = if k + 1 < 2 * n && step_dir(k) == step_dir(k + 1) { next_run + 1 } else { 1 };
            if k < n {
                run[k] = r.min(n);
            }
            next_run = r;
        }
    }
    let mut out = vec![1usize; n];
    // Knickpunkte (Pfadindex) mit dem Kegel nach ihrer Aufnahme
    let mut hull: Vec<(usize, Cone)> = Vec::new();
    let mut corners: Vec<(i32, i32)> = Vec::new();
    #[allow(clippy::needless_range_loop)] // i ist zugleich Pfadindex (zyklisch über `at`)
    for i in 0..n {
        let vi = at(i);
        let mut dirs = [false; 4];
        let mut cone = Cone::default();
        hull.clear();
        hull.push((i, cone));
        let limit = i + n - 1;
        let mut k = i;
        // 1. schnelle, großzügige Prüfung (Kegel durch den Anfangspunkt, Quadrate ±1)
        while k < limit {
            dirs[step_dir(k)] = true;
            if dirs.iter().all(|&d| d) {
                break;
            }
            let next = (k + run[k % n]).min(limit);
            if cone.admits(vi, at(next)) {
                cone.add(vi, at(next));
                hull.push((next, cone));
                k = next;
                continue;
            }
            // Der Lauf k..next ist gerade: zulässig ist ein Anfangsstück (binäre Suche).
            let (mut lo, mut hi) = (k, next);
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if cone.admits(vi, at(mid)) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            if lo > k {
                cone.add(vi, at(lo));
                hull.push((lo, cone));
            }
            break;
        }
        // 2. genaue Prüfung nur noch am Ende, sonst binäre Suche über die Knickpunkte und im
        //    letzten geraden Lauf (Geradheit vererbt sich auf Anfangsstücke)
        corners.clear();
        corners.extend(hull.iter().map(|(q, _)| at(*q)));
        let exact = |c: usize, extra: Option<((i32, i32), Cone)>| -> bool {
            match extra {
                Some((p, cn)) => cn.straight(&corners[..=c], Some(p)),
                None => hull[c].1.straight(&corners[..=c], None),
            }
        };
        let last = hull.len() - 1;
        let best = if exact(last, None) {
            hull[last].0
        } else {
            let (mut lo, mut hi) = (0usize, last);
            while hi - lo > 1 {
                let mid = (lo + hi) / 2;
                if exact(mid, None) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            let (mut a, mut b) = (hull[lo].0, hull[hi].0);
            while b - a > 1 {
                let mid = (a + b) / 2;
                let mut cn = hull[lo].1;
                cn.add(vi, at(mid));
                if exact(lo, Some((at(mid), cn))) {
                    a = mid;
                } else {
                    b = mid;
                }
            }
            a
        };
        out[i] = best.max(i + 1) - i;
    }
    out
}

/// Pseudowinkel (−2, 2] eines Vektors mit Längs- und Queranteil (d, c) zur Bezugsrichtung:
/// streng monoton im Winkel, ohne Winkelfunktionen.
fn pseudo_angle(d: f64, c: f64) -> f64 {
    let s = d.abs() + c.abs();
    if s == 0.0 {
        return 0.0;
    }
    let t = c / s;
    if d >= 0.0 { t } else if c >= 0.0 { 2.0 - t } else { -2.0 - t }
}

/// Umkehrung von [`pseudo_angle`]: (Längs-, Queranteil) einer Richtung (nicht normiert).
fn pseudo_dir(a: f64) -> (f64, f64) {
    if a.abs() <= 1.0 {
        (1.0 - a.abs(), a)
    } else if a > 0.0 {
        (1.0 - a, 2.0 - a)
    } else {
        (1.0 + a, -2.0 - a)
    }
}

/// Kegel der Richtungen (Pseudowinkel relativ zur ersten Bedingung), in denen eine Gerade durch den
/// Anfangspunkt alle bisherigen Quadrate ±1 trifft.
#[derive(Clone, Copy, Default)]
struct Cone {
    base: Option<(f64, f64)>,
    lo: f64,
    hi: f64,
}

impl Cone {
    /// Winkelbereich des Quadrats ±1 um `p`, gesehen von `vi`; `None` = enthält `vi` (keine
    /// Bedingung). ±1 statt ±½: Die Gerade darf auch am Anfangspunkt um ½ abweichen – eine Gerade
    /// im Abstand ≤ ½ von `vi` und `p` gibt es genau dann, wenn eine Parallele durch `vi` das
    /// Quadrat ±1 um `p` trifft.
    fn square(&self, vi: (i32, i32), p: (i32, i32)) -> Option<(f64, f64)> {
        let cur = (p.0 - vi.0, p.1 - vi.1);
        if cur.0.abs() <= 1 && cur.1.abs() <= 1 {
            return None;
        }
        let base = self.base.unwrap_or((cur.0 as f64, cur.1 as f64));
        let (mut a0, mut a1) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut back_pos, mut back_neg) = (false, false);
        for (sx, sy) in [(-1, -1), (1, -1), (1, 1), (-1, 1)] {
            let q = ((cur.0 + sx) as f64, (cur.1 + sy) as f64);
            let (d, c) = (base.0 * q.0 + base.1 * q.1, base.0 * q.1 - base.1 * q.0);
            if d < 0.0 {
                back_pos |= c >= 0.0;
                back_neg |= c <= 0.0;
            }
            let ang = pseudo_angle(d, c);
            a0 = a0.min(ang);
            a1 = a1.max(ang);
        }
        if back_pos && back_neg {
            // Quadrat hinter dem Anfangspunkt (überdeckt die Gegenrichtung)
            return Some((f64::INFINITY, f64::NEG_INFINITY));
        }
        Some((a0, a1))
    }

    fn admits(&self, vi: (i32, i32), p: (i32, i32)) -> bool {
        match self.square(vi, p) {
            None => true,
            Some((a0, a1)) => {
                if self.base.is_none() {
                    a0 <= a1
                } else {
                    self.lo.max(a0) <= self.hi.min(a1)
                }
            }
        }
    }

    /// Genaue Prüfung (Definition des Artikels) mit diesem Kegel (er enthält alle Punkte): Gibt es
    /// eine Richtung im Kegel, in der eine Gerade im Abstand ≤ ½ (Maximumnorm) an allen
    /// Knickpunkten `hull` (und `p`) vorbeiläuft? Das ist der Fall, wenn die Breite der Punkte quer
    /// zur Richtung u höchstens |ux| + |uy| ist (Breite des Pixelquadrats in dieser Richtung).
    /// Geprüft an 9 Richtungen im Kegel.
    fn straight(&self, hull: &[(i32, i32)], p: Option<(i32, i32)>) -> bool {
        let c = self;
        let Some(base) = c.base else { return true };
        let bl = base.0.hypot(base.1);
        let b = (base.0 / bl, base.1 / bl);
        for s in 0..=8 {
            let (d, cr) = pseudo_dir(c.lo + (c.hi - c.lo) * s as f64 / 8.0);
            let (ux, uy) = (b.0 * d - b.1 * cr, b.0 * cr + b.1 * d);
            let ul = ux.hypot(uy);
            let u = (ux / ul, uy / ul);
            let (mut mn, mut mx) = (f64::INFINITY, f64::NEG_INFINITY);
            for q in hull.iter().chain(p.iter()) {
                let v = u.0 * q.1 as f64 - u.1 * q.0 as f64;
                mn = mn.min(v);
                mx = mx.max(v);
            }
            if mx - mn <= u.0.abs() + u.1.abs() + 1e-9 {
                return true;
            }
        }
        false
    }

    fn add(&mut self, vi: (i32, i32), p: (i32, i32)) {
        let Some((a0, a1)) = self.square(vi, p) else { return };
        if self.base.is_none() {
            self.base = Some(((p.0 - vi.0) as f64, (p.1 - vi.1) as f64));
            self.lo = a0;
            self.hi = a1;
        } else {
            self.lo = self.lo.max(a0);
            self.hi = self.hi.min(a1);
        }
    }
}

/// Strafe der Kante i → j: √(Σ Abstandsquadrat · |vj − vi|² / Anzahl) = Länge · mittlere
/// quadratische Abweichung der Pfadpunkte i..=j von der Sehne.
fn penalty(s: &Sums, pts: &[P], i: usize, j: usize) -> f64 {
    let n = pts.len();
    let (a, b) = (pts[i % n], pts[j % n]);
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (m, sx, sy, sxx, sxy, syy) = s.range(i, j);
    let suu = sxx - 2.0 * a.x * sx + m * a.x * a.x;
    let sww = syy - 2.0 * a.y * sy + m * a.y * a.y;
    let suw = sxy - a.x * sy - a.y * sx + m * a.x * a.y;
    let sum = dx * dx * sww - 2.0 * dx * dy * suw + dy * dy * suu;
    (sum.max(0.0) / m).sqrt()
}

/// Je Pfadpunkt i: größte zulässige Kantenlänge i → j (j − i ≤ maxlen[i]): der um je einen Punkt
/// verlängerte Teilpfad i−1 … j+1 ist gerade, höchstens n − 3 Schritte, mindestens 1.
fn max_edge_lengths(pts: &[(i32, i32)]) -> Vec<usize> {
    let n = pts.len();
    let reach = straight_reach(pts);
    (0..n).map(|i| (reach[(i + n - 1) % n].saturating_sub(2)).min(n.saturating_sub(3)).max(1)).collect()
}

/// Optimales Polygon mit dem Pfadpunkt `start` als Ecke: wenigste Kanten, dann kleinste Strafe.
/// Ergebnis: Indizes start ≤ … < start + n (aufsteigend; `% n` ergibt den Pfadpunkt).
fn optimal_polygon(fp: &[P], sums: &Sums, maxlen: &[usize], start: usize) -> Vec<usize> {
    let n = fp.len();
    let mut cnt = vec![usize::MAX; n + 1];
    let mut pen = vec![f64::INFINITY; n + 1];
    let mut prev = vec![0usize; n + 1];
    cnt[0] = 0;
    pen[0] = 0.0;
    for i in 0..n {
        if cnt[i] == usize::MAX {
            continue;
        }
        let end = (i + maxlen[(start + i) % n]).min(n);
        for j in i + 1..=end {
            let c = cnt[i] + 1;
            if c > cnt[j] {
                continue;
            }
            let p = pen[i] + penalty(sums, fp, start + i, start + j);
            if c < cnt[j] || p < pen[j] {
                cnt[j] = c;
                pen[j] = p;
                prev[j] = i;
            }
        }
    }
    let mut poly = Vec::new();
    let mut j = n;
    while j > 0 {
        j = prev[j];
        poly.push(start + j);
    }
    poly.reverse();
    poly
}

/// Ausgleichsgerade der Pfadpunkte i..=j: (Schwerpunkt, Einheitsnormale).
fn fit_line(s: &Sums, pts: &[P], i: usize, j: usize) -> (P, P) {
    let (m, sx, sy, sxx, sxy, syy) = s.range(i, j);
    let c = P::new(sx / m, sy / m);
    let (cxx, cxy, cyy) = (sxx / m - c.x * c.x, sxy / m - c.x * c.y, syy / m - c.y * c.y);
    let dir = if cxx.abs() + cxy.abs() + cyy.abs() > 1e-12 {
        let th = 0.5 * (2.0 * cxy).atan2(cxx - cyy);
        P::new(th.cos(), th.sin())
    } else {
        let n = pts.len();
        let d = pts[j % n].sub(pts[i % n]);
        let l = d.len().max(1e-12);
        d.mul(1.0 / l)
    };
    (c, P::new(-dir.y, dir.x))
}

/// Punkt im Quadrat ±½ um `v` mit der kleinsten Summe der Abstandsquadrate zu den Geraden.
fn adjust_vertex(v: P, lines: &[(P, P); 2]) -> P {
    const EPS: f64 = 1e-4;
    // Q(p) = Σ (n·p − n·c)² + ε |p − v|²
    let q = |p: P| lines.iter().map(|(c, nm)| (nm.dot(p) - nm.dot(*c)).powi(2)).sum::<f64>() + EPS * p.sub(v).dot(p.sub(v));
    let (mut a, mut b, mut c, mut r0, mut r1) = (EPS, 0.0, EPS, EPS * v.x, EPS * v.y);
    for (ce, nm) in lines {
        let h = nm.dot(*ce);
        a += nm.x * nm.x;
        b += nm.x * nm.y;
        c += nm.y * nm.y;
        r0 += h * nm.x;
        r1 += h * nm.y;
    }
    let det = a * c - b * b;
    if det.abs() > 1e-12 {
        let p = P::new((r0 * c - b * r1) / det, (a * r1 - b * r0) / det);
        if (p.x - v.x).abs() <= 0.5 && (p.y - v.y).abs() <= 0.5 {
            return p;
        }
    }
    // Minimum auf dem Rand des Quadrats
    let corners = [P::new(v.x - 0.5, v.y - 0.5), P::new(v.x + 0.5, v.y - 0.5), P::new(v.x + 0.5, v.y + 0.5), P::new(v.x - 0.5, v.y + 0.5)];
    let mut best = (f64::INFINITY, v);
    for k in 0..4 {
        let (e0, e1) = (corners[k], corners[(k + 1) % 4]);
        let d = e1.sub(e0);
        // Q(e0 + t d) = Σ (α + β t)²
        let (mut sab, mut sbb) = (EPS * e0.sub(v).dot(d), EPS * d.dot(d));
        for (ce, nm) in lines {
            let al = nm.dot(e0) - nm.dot(*ce);
            let be = nm.dot(d);
            sab += al * be;
            sbb += be * be;
        }
        let t = if sbb > 1e-15 { (-sab / sbb).clamp(0.0, 1.0) } else { 0.0 };
        let p = e0.add(d.mul(t));
        let val = q(p);
        if val < best.0 {
            best = (val, p);
        }
    }
    best.1
}

// ------------------------------------------------------------------ Glätten und Kurven

/// Ein Bogen um einen Polygonpunkt: von der Mitte der vorigen Kante `a` zur Mitte der nächsten `b`.
#[derive(Clone, Copy, Debug)]
struct Seg {
    a: P,
    v: P,
    b: P,
    corner: bool,
    alpha: f64,
}

impl Seg {
    fn c1(&self) -> P {
        self.a.lerp(self.v, self.alpha)
    }
    fn c2(&self) -> P {
        self.b.lerp(self.v, self.alpha)
    }
    /// Krümmungsrichtung (+1/−1, 0 = gerade).
    fn sign(&self) -> i32 {
        let c = self.v.sub(self.a).cross(self.b.sub(self.v));
        if c > 1e-12 {
            1
        } else if c < -1e-12 {
            -1
        } else {
            0
        }
    }
    /// Drehung der Tangente über den Bogen (Bogenmaß).
    fn turn(&self) -> f64 {
        let (u, w) = (self.v.sub(self.a), self.b.sub(self.v));
        u.cross(w).abs().atan2(u.dot(w))
    }
    /// Fläche zwischen Bogen und Sehne (mit Vorzeichen).
    fn bulge(&self) -> f64 {
        0.5 * self.v.sub(self.a).cross(self.b.sub(self.a)) * 0.3 * self.alpha * (4.0 - self.alpha)
    }
}

fn make_segs(v: &[P], alpha_max: f64) -> Vec<Seg> {
    let m = v.len();
    (0..m)
        .map(|k| {
            let (pv, cv, nv) = (v[(k + m - 1) % m], v[k], v[(k + 1) % m]);
            let (a, b) = (pv.mid(cv), cv.mid(nv));
            let chord = b.sub(a);
            let len = chord.len();
            let mut alpha = if len < 1e-12 {
                f64::INFINITY
            } else {
                let nm = P::new(-chord.y / len, chord.x / len);
                let d = nm.dot(cv.sub(a)).abs();
                let e = 0.5 * (nm.x.abs() + nm.y.abs());
                if d > e { 4.0 / 3.0 * (1.0 - e / d) } else { 0.0 }
            };
            let corner = alpha >= alpha_max;
            if !corner {
                alpha = alpha.clamp(0.55, 1.0);
            }
            Seg { a, v: cv, b, corner, alpha: if corner { 0.0 } else { alpha } }
        })
        .collect()
}

/// Ein Bogen über die Bögen i..=j (alle glatt, gleiche Krümmungsrichtung): Henkel auf den
/// Endtangenten, gleiche Fläche; `None`, wenn er mehr als `tol` abweicht. Ergebnis: Henkel und
/// Summe der Abweichungsquadrate.
fn join(segs: &[&Seg], i: usize, j: usize, tol: f64) -> Option<(P, P, f64)> {
    let (p0, p3) = (segs[i].a, segs[j].b);
    let (u0, u1) = (segs[i].v.sub(p0), p3.sub(segs[j].v));
    let det = u0.cross(u1);
    if det.abs() < 1e-12 {
        return None;
    }
    let d = p3.sub(p0);
    let (lam, mu) = (d.cross(u1) / det, u0.cross(d) / det);
    if lam <= 0.0 || mu <= 0.0 {
        return None;
    }
    let x = p0.add(u0.mul(lam));
    let tri = 0.5 * x.sub(p0).cross(p3.sub(p0));
    // Fläche des ursprünglichen Verlaufs gegenüber der Sehne p0 → p3
    let mut area = 0.0;
    for s in &segs[i..=j] {
        area += 0.5 * s.a.sub(p0).cross(s.b.sub(p0)) + s.bulge();
    }
    if tri.abs() < 1e-12 {
        return None;
    }
    let r = area / tri;
    let disc = 4.0 - 10.0 / 3.0 * r;
    if r <= 0.0 || disc < 0.0 {
        return None;
    }
    let beta = 2.0 - disc.sqrt();
    let (p1, p2) = (p0.lerp(x, beta), p3.lerp(x, beta));
    // Abweichung: Bogenmitten und Übergänge des ursprünglichen Verlaufs
    let samples = 12 * (j - i + 1) + 8;
    let poly: Vec<P> = (0..=samples).map(|k| bezier(p0, p1, p2, p3, k as f64 / samples as f64)).collect();
    let dist = |q: P| poly.windows(2).map(|w| seg_dist(q, w[0], w[1])).fold(f64::INFINITY, f64::min);
    let mut pen = 0.0;
    for (k, s) in segs[i..=j].iter().enumerate() {
        let mid = bezier(s.a, s.c1(), s.c2(), s.b, 0.5);
        let dm = dist(mid);
        if dm > tol {
            return None;
        }
        pen += dm * dm;
        if k > 0 {
            let da = dist(s.a);
            if da > tol {
                return None;
            }
            pen += da * da;
        }
    }
    Some((p1, p2, pen))
}

/// Eintrag der Kurvenoptimierung je Übergang: (Bögen, Abweichung, Vorgänger, Henkel des
/// zusammengefassten Bogens).
type Step = (usize, f64, usize, Option<(P, P)>);

/// Teilstück der fertigen Kontur.
#[derive(Clone, Copy, Debug)]
enum Piece {
    Line(P, P),
    /// Anfang, Henkel danach, Henkel vor dem Ende (das Ende ist der Anfang des nächsten Stücks).
    Cubic(P, P, P),
}

impl Piece {
    fn start(&self) -> P {
        match *self {
            Piece::Line(a, _) | Piece::Cubic(a, _, _) => a,
        }
    }
    fn out_ctrl(&self) -> P {
        match *self {
            Piece::Line(a, _) => a,
            Piece::Cubic(_, c, _) => c,
        }
    }
    fn in_ctrl(&self) -> P {
        match *self {
            Piece::Line(_, b) => b,
            Piece::Cubic(_, _, c) => c,
        }
    }
}

/// Pfadpunkte → Knoten (Anker, Henkel davor, Henkel danach) in Bildpixeln.
fn curve_nodes(pts: &[(i32, i32)], po: &PotraceOptions) -> Vec<(P, P, P)> {
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }
    let origin = P::new(pts[0].0 as f64, pts[0].1 as f64);
    let fp: Vec<P> = pts.iter().map(|p| P::new(p.0 as f64 - origin.x, p.1 as f64 - origin.y)).collect();
    let sums = Sums::new(&fp);
    let maxlen = max_edge_lengths(pts);
    // Erst mit dem Startpunkt als Ecke; dann noch einmal ab einer Ecke aus der Mitte dieses
    // Polygons, damit kein künstlicher Eckpunkt am Startpunkt bleibt (zyklisch optimal genug).
    let mut poly = optimal_polygon(&fp, &sums, &maxlen, 0);
    if poly.len() >= 3 {
        let again = optimal_polygon(&fp, &sums, &maxlen, poly[poly.len() / 2] % n);
        if again.len() <= poly.len() {
            poly = again;
        }
    }
    if poly.len() < 3 {
        poly = (0..n).collect();
    }
    let m = poly.len();
    // Ausgleichsgeraden der Kanten, dann verfeinerte Eckpunkte
    let lines: Vec<(P, P)> = (0..m).map(|k| fit_line(&sums, &fp, poly[k], if k + 1 < m { poly[k + 1] } else { poly[0] + n })).collect();
    let verts: Vec<P> = (0..m).map(|k| adjust_vertex(fp[poly[k] % n], &[lines[(k + m - 1) % m], lines[k]])).collect();
    let segs = make_segs(&verts, po.alpha_max);
    // Beginn an einer Ecke (dort darf nichts zusammengefasst werden), sonst beim Startpunkt
    let start = segs.iter().position(|s| s.corner).unwrap_or(0);
    let order: Vec<&Seg> = (0..m).map(|k| &segs[(start + k) % m]).collect();
    // dynamische Programmierung über die Übergänge 0..=m: (Bögen, Abweichung, Vorgänger, Henkel)
    let mut best: Vec<Step> = vec![(usize::MAX, f64::INFINITY, 0, None); m + 1];
    best[0] = (0, 0.0, 0, None);
    let limit = MAX_JOIN_TURN_DEG.to_radians();
    for j in 1..=m {
        best[j] = (best[j - 1].0 + 1, best[j - 1].1, j - 1, None);
        let last = order[j - 1];
        if !po.opt_curve || last.corner || last.sign() == 0 {
            continue;
        }
        let sign = last.sign();
        let mut turn = last.turn();
        for i in (0..j - 1).rev() {
            let s = order[i];
            if s.corner || s.sign() != sign {
                break;
            }
            turn += s.turn();
            if turn >= limit {
                break;
            }
            if let Some((p1, p2, pen)) = join(&order, i, j - 1, po.opt_tolerance) {
                let cand = (best[i].0 + 1, best[i].1 + pen);
                if cand.0 < best[j].0 || (cand.0 == best[j].0 && cand.1 < best[j].1) {
                    best[j] = (cand.0, cand.1, i, Some((p1, p2)));
                }
            }
        }
    }
    let mut pieces = Vec::new();
    let mut j = m;
    while j > 0 {
        let (_, _, i, h) = best[j];
        match h {
            Some((p1, p2)) => pieces.push(Piece::Cubic(order[i].a, p1, p2)),
            None => {
                let s = order[i];
                if s.corner {
                    pieces.push(Piece::Line(s.v, s.b));
                    pieces.push(Piece::Line(s.a, s.v));
                } else {
                    pieces.push(Piece::Cubic(s.a, s.c1(), s.c2()));
                }
            }
        }
        j = i;
    }
    pieces.reverse();
    let k = pieces.len();
    let mut nodes: Vec<(P, P, P)> = (0..k).map(|i| (pieces[i].start(), pieces[(i + k - 1) % k].in_ctrl(), pieces[i].out_ctrl())).collect();
    // Kantenmitten zwischen zwei Geraden auf einer Linie sind überflüssig
    let mut changed = true;
    while changed && nodes.len() > 3 {
        changed = false;
        let c = nodes.len();
        for i in 0..c {
            let (p, cin, cout) = nodes[i];
            if cin != p || cout != p {
                continue;
            }
            let (a, b) = (nodes[(i + c - 1) % c].0, nodes[(i + 1) % c].0);
            let l = b.sub(a).len();
            if b.sub(a).cross(p.sub(a)).abs() <= 1e-9 * l.max(1.0) && p.sub(a).dot(b.sub(p)) >= 0.0 {
                nodes.remove(i);
                changed = true;
                break;
            }
        }
    }
    nodes.into_iter().map(|(p, i, o)| (p.add(origin), i.add(origin), o.add(origin))).collect()
}

/// Bildpixel → Dokumentkoordinaten; aufeinanderfolgende gleiche Anker werden verschmolzen.
fn map_nodes(nodes: &[(P, P, P)], map: &Mapper) -> Vec<BezierNode> {
    let mut out: Vec<BezierNode> = Vec::with_capacity(nodes.len());
    for (p, i, o) in nodes {
        let n = BezierNode::new(map.map(p.x, p.y), map.map(i.x, i.y), map.map(o.x, o.y));
        match out.last_mut() {
            Some(last) if last.pt == n.pt => last.ctrl_out = n.ctrl_out,
            _ => out.push(n),
        }
    }
    while out.len() > 1 && out.first().map(|f| f.pt) == out.last().map(|l| l.pt) {
        if let Some(l) = out.pop() {
            out[0].ctrl_in = l.ctrl_in;
        }
    }
    out
}
