//! Kleine CPU-Inferenz in reinem Rust.
//!
//! Tensoren sind dichte `f32`-Puffer in Zeilenreihenfolge. Bildmerkmale liegen je nach Netz als
//! `[C, H, W]` (Faltungen, BiRefNet-Decoder) oder als Token `[H·W, C]` (Transformer) vor. Die
//! Matrixmultiplikation übernimmt `faer` (SIMD zur Laufzeit gewählt), alle anderen Operationen
//! laufen mit `rayon` parallel im eigenen Thread-Pool ([`pool`]), damit Oberfläche und Leinwand-
//! Renderer nicht ausgebremst werden.

use faer::linalg::matmul::matmul;
use faer::{Accum, MatMut, MatRef, Par};
use rayon::prelude::*;
use std::sync::OnceLock;

/// Eigener Thread-Pool der KI-Berechnungen (alle Kerne bis auf einen).
pub fn pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let n = if n > 4 { n - 1 } else { n };
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(|i| format!("fp-ai-{i}"))
            .build()
            .unwrap_or_else(|_| rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap_or_else(|_| panic_free_pool()))
    })
}

fn panic_free_pool() -> rayon::ThreadPool {
    // Letzter Rückfall: Standardpool-Konfiguration; scheitert auch das, gibt es keinen Pool – dann
    // bricht rayon ohnehin ab. Der Pfad wird praktisch nie erreicht.
    #[allow(clippy::expect_used)]
    rayon::ThreadPoolBuilder::new().build().expect("rayon-Pool")
}

/// Zeitmessung für die Entwicklung (`FP_AI_PROFILE=1`): gibt die Zeit seit `t` mit Beschriftung
/// auf stderr aus und liefert den neuen Startpunkt.
pub fn prof(label: &str, t: std::time::Instant) -> std::time::Instant {
    static ON: OnceLock<bool> = OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("FP_AI_PROFILE").is_some()) {
        eprintln!("[fp-ai] {label}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    }
    std::time::Instant::now()
}

// ------------------------------------------------------------------ Puffervorrat
//
// Gemessen (24 Kerne, Windows): eine frisch angeforderte 40-MB-Ausgabe, die faer parallel
// beschreibt, kostet 37 ms statt 6,6 ms mit wiederverwendetem Puffer (Seitenfehler aller Threads
// gleichzeitig). Große Tensoren kommen deshalb aus einem Vorrat und gehen nach Gebrauch zurück.

const POOL_MIN: usize = 64 * 1024;
const POOL_LIMIT_BYTES: usize = 1536 << 20;

fn tensor_pool() -> &'static std::sync::Mutex<Vec<Vec<f32>>> {
    static P: OnceLock<std::sync::Mutex<Vec<Vec<f32>>>> = OnceLock::new();
    P.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Puffer der Länge `n` mit **unbestimmtem Inhalt** (nur für Ergebnisse, die vollständig
/// überschrieben werden). Neue Puffer werden parallel vorab berührt.
pub fn take(n: usize) -> Vec<f32> {
    if n >= POOL_MIN
        && let Ok(mut p) = tensor_pool().lock()
    {
        let best = p.iter().enumerate().filter(|(_, b)| b.capacity() >= n).min_by_key(|(_, b)| b.capacity()).map(|(i, _)| i);
        if let Some(i) = best {
            let mut v = p.swap_remove(i);
            drop(p);
            if v.len() >= n {
                v.truncate(n);
            } else {
                v.resize(n, 0.0);
            }
            return v;
        }
    }
    let mut v = vec![0.0f32; n];
    if n >= POOL_MIN {
        v.par_chunks_mut(1024).for_each(|c| c[0] = 0.0);
    }
    v
}

/// Puffer der Länge `n`, mit Nullen gefüllt.
pub fn take_zeroed(n: usize) -> Vec<f32> {
    let mut v = take(n);
    v.par_chunks_mut(CHUNK).for_each(|c| c.fill(0.0));
    v
}

/// Kopie von `x` aus dem Vorrat.
pub fn take_copy(x: &[f32]) -> Vec<f32> {
    let mut v = take(x.len());
    v.par_chunks_mut(CHUNK).zip(x.par_chunks(CHUNK)).for_each(|(a, b)| a.copy_from_slice(b));
    v
}

/// Puffer an den Vorrat zurückgeben.
pub fn give(v: Vec<f32>) {
    if v.capacity() < POOL_MIN {
        return;
    }
    if let Ok(mut p) = tensor_pool().lock() {
        p.push(v);
        let mut total: usize = p.iter().map(|b| b.capacity() * 4).sum();
        while total > POOL_LIMIT_BYTES && !p.is_empty() {
            // kleinsten Puffer verwerfen
            if let Some((i, _)) = p.iter().enumerate().min_by_key(|(_, b)| b.capacity()) {
                total -= p[i].capacity() * 4;
                p.swap_remove(i);
            }
        }
    }
}

/// Vorrat leeren (nach einer Berechnung: Speicher an das System zurück).
pub fn trim() {
    if let Ok(mut p) = tensor_pool().lock() {
        p.clear();
    }
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Vec<Vec<f32>>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Zwischenpuffer (mit Nullen) aus dem Vorrat des Threads. Große Puffer bei jeder Aufgabe neu
/// anzufordern und freizugeben ist unter Windows mit vielen Threads sehr teuer (Seitenfehler und
/// TLB-Abgleich über alle Kerne) – deshalb werden sie je Arbeits-Thread wiederverwendet.
pub fn scratch_take(n: usize) -> Vec<f32> {
    let mut v = SCRATCH.with(|p| {
        let mut p = p.borrow_mut();
        // passenden (großen genug) Puffer bevorzugen
        let i = p.iter().position(|b| b.capacity() >= n).or(if p.is_empty() { None } else { Some(p.len() - 1) });
        i.map(|i| p.swap_remove(i))
    })
    .unwrap_or_default();
    v.clear();
    v.resize(n, 0.0);
    v
}

/// Puffer an den Vorrat des Threads zurückgeben.
pub fn scratch_give(v: Vec<f32>) {
    if v.capacity() > (64 << 20) {
        return;
    }
    SCRATCH.with(|p| {
        let mut p = p.borrow_mut();
        if p.len() < 6 {
            p.push(v);
        }
    });
}

fn threads() -> usize {
    rayon::current_num_threads().max(1)
}

fn par() -> Par {
    Par::rayon(threads())
}

/// Dichter Tensor.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
}

impl Tensor {
    /// Nullen.
    pub fn zeros(shape: &[usize]) -> Tensor {
        Tensor { shape: shape.to_vec(), data: vec![0.0; shape.iter().product()] }
    }
    /// Aus Daten.
    pub fn new(shape: &[usize], data: Vec<f32>) -> Tensor {
        debug_assert_eq!(shape.iter().product::<usize>(), data.len());
        Tensor { shape: shape.to_vec(), data }
    }
    /// Anzahl Elemente.
    pub fn len(&self) -> usize {
        self.data.len()
    }
    /// Leer?
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    /// Dimension `i`.
    pub fn dim(&self, i: usize) -> usize {
        self.shape.get(i).copied().unwrap_or(1)
    }
    /// Neue Form (gleiche Anzahl Elemente).
    pub fn reshaped(mut self, shape: &[usize]) -> Tensor {
        debug_assert_eq!(shape.iter().product::<usize>(), self.data.len());
        self.shape = shape.to_vec();
        self
    }
}

const CHUNK: usize = 16 * 1024;

// ------------------------------------------------------------------ Matrixmultiplikation

/// `c[m×n] (+)= a[m×k] · b`, `b` als `[k×n]` bzw. (bei `b_t`) als `[n×k]` gespeichert. Alles in
/// Zeilenreihenfolge. `parallel` = faer verteilt selbst auf den Pool.
#[allow(clippy::too_many_arguments)]
pub fn gemm(a: &[f32], m: usize, k: usize, b: &[f32], n: usize, b_t: bool, c: &mut [f32], acc: bool, parallel: bool) {
    if m == 0 || n == 0 {
        return;
    }
    let a = MatRef::from_row_major_slice(&a[..m * k], m, k);
    let c = MatMut::from_row_major_slice_mut(&mut c[..m * n], m, n);
    let p = if parallel && m * n * k > 1 << 18 { par() } else { Par::Seq };
    let accum = if acc { Accum::Add } else { Accum::Replace };
    if b_t {
        let b = MatRef::from_row_major_slice(&b[..n * k], n, k).transpose();
        matmul(c, accum, a, b, 1.0f32, p);
    } else {
        let b = MatRef::from_row_major_slice(&b[..k * n], k, n);
        matmul(c, accum, a, b, 1.0f32, p);
    }
}

/// Lineare Schicht auf Zeilen: `x[n×cin] · wᵀ + b`, `w` als `[cout×cin]` (PyTorch-Layout).
pub fn linear(x: &[f32], n: usize, cin: usize, w: &[f32], bias: Option<&[f32]>, cout: usize) -> Vec<f32> {
    linear_t(x, n, cin, &transpose(w, cout, cin), bias, cout)
}

/// Lineare Schicht mit vorab transponierten Gewichten `wt[cin×cout]`.
///
/// Gemessen (24 Kerne): bei sehr vielen Zeilen und kleinen Matrizen sind unabhängige
/// 1024-Zeilen-Blöcke etwa 4× schneller als die interne Parallelisierung von faer, bei
/// mittleren Größen ist es umgekehrt.
pub fn linear_t(x: &[f32], n: usize, cin: usize, wt: &[f32], bias: Option<&[f32]>, cout: usize) -> Vec<f32> {
    let mut out = take(n * cout);
    if n >= 32 * 1024 || (n >= 8192 && cin * cout <= 96 * 1024) {
        let rows = 1024;
        out.par_chunks_mut(rows * cout).enumerate().for_each(|(i, oc)| {
            let r0 = i * rows;
            let nr = oc.len() / cout;
            gemm(&x[r0 * cin..(r0 + nr) * cin], nr, cin, wt, cout, false, oc, false, false);
        });
    } else {
        gemm(x, n, cin, wt, cout, false, &mut out, false, true);
    }
    if let Some(b) = bias {
        add_row_bias(&mut out, b);
    }
    out
}

/// Addiert `b` auf jede Zeile.
pub fn add_row_bias(x: &mut [f32], b: &[f32]) {
    let c = b.len();
    if c == 0 {
        return;
    }
    x.par_chunks_mut(c * (CHUNK / c).max(1)).for_each(|ch| {
        for row in ch.chunks_mut(c) {
            for (v, bb) in row.iter_mut().zip(b) {
                *v += *bb;
            }
        }
    });
}

/// Addiert `b[c]` auf jede Ebene eines `[C, HW]`-Tensors.
pub fn add_channel_bias(x: &mut [f32], b: &[f32]) {
    let c = b.len();
    if c == 0 {
        return;
    }
    let hw = x.len() / c;
    x.par_chunks_mut(hw.max(1)).zip(b.par_iter()).for_each(|(plane, bb)| {
        for v in plane {
            *v += *bb;
        }
    });
}

// ------------------------------------------------------------------ elementweise

/// Elementweise Funktion (parallel).
pub fn map_inplace(x: &mut [f32], f: impl Fn(f32) -> f32 + Sync) {
    x.par_chunks_mut(CHUNK).for_each(|c| {
        for v in c {
            *v = f(*v);
        }
    });
}

/// `a += b`.
pub fn add_inplace(a: &mut [f32], b: &[f32]) {
    a.par_chunks_mut(CHUNK).zip(b.par_chunks(CHUNK)).for_each(|(x, y)| {
        for (p, q) in x.iter_mut().zip(y) {
            *p += *q;
        }
    });
}

/// `a *= b`.
pub fn mul_inplace(a: &mut [f32], b: &[f32]) {
    a.par_chunks_mut(CHUNK).zip(b.par_chunks(CHUNK)).for_each(|(x, y)| {
        for (p, q) in x.iter_mut().zip(y) {
            *p *= *q;
        }
    });
}

/// `a + b` als neuer Puffer.
pub fn add(a: &[f32], b: &[f32]) -> Vec<f32> {
    let mut o = take_copy(a);
    add_inplace(&mut o, b);
    o
}

/// Exponentialfunktion in reinem Rust (relative Abweichung < 3·10⁻⁷).
///
/// Gemessen: `f32::exp` der statisch gelinkten C-Laufzeit wird bei vielen gleichzeitigen Threads
/// für manche Eingaben um ein Vielfaches langsamer (Softmax 228×4096 auf 23 Threads: 246 ms statt
/// 35 ms). Diese Fassung kommt ohne Bibliotheksaufruf aus und lässt sich vektorisieren.
#[inline]
pub fn exp(x: f32) -> f32 {
    const LOG2E: f32 = std::f32::consts::LOG2_E;
    const LN2_HI: f32 = 0.693_145_75;
    const LN2_LO: f32 = 1.428_606_8e-6;
    const MAGIC: f32 = 12_582_912.0; // 1,5·2²³: Runden auf ganze Zahl durch Addition
    let x = x.clamp(-87.3, 88.7);
    let n = (x * LOG2E + MAGIC) - MAGIC;
    let f = x - n * LN2_HI - n * LN2_LO;
    let p = 1.0 + f * (1.0 + f * (0.5 + f * (1.0 / 6.0 + f * (1.0 / 24.0 + f * (1.0 / 120.0 + f * (1.0 / 720.0))))));
    let bits = ((n as i32 + 127) as u32) << 23;
    p * f32::from_bits(bits)
}

/// Fehlerfunktion (Abramowitz/Stegun 7.1.26, absoluter Fehler ≤ 1,5·10⁻⁷ – so genau wie `f32`).
#[inline]
pub fn erf(x: f32) -> f32 {
    let z = x.abs();
    if z >= 4.0 {
        // erfc(4) ≈ 1,5·10⁻⁸; außerdem kein subnormales exp (sehr langsam auf x86)
        return if x >= 0.0 { 1.0 } else { -1.0 };
    }
    let t = 1.0 / (1.0 + 0.327_591_1 * z);
    let poly = ((((1.061_405_4 * t - 1.453_152_1) * t + 1.421_413_8) * t - 0.284_496_74) * t + 0.254_829_6) * t;
    let r = 1.0 - poly * exp(-z * z);
    if x >= 0.0 { r } else { -r }
}

/// GELU (exakte Form mit erf, wie `nn.GELU()`).
#[inline]
pub fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + erf(x * std::f32::consts::FRAC_1_SQRT_2))
}

/// Sigmoid.
#[inline]
pub fn sigmoid(x: f32) -> f32 {
    if x < -80.0 {
        0.0
    } else if x > 80.0 {
        1.0
    } else {
        1.0 / (1.0 + exp(-x))
    }
}

/// GELU auf alle Werte.
pub fn gelu_inplace(x: &mut [f32]) {
    map_inplace(x, gelu);
}

/// ReLU auf alle Werte.
pub fn relu_inplace(x: &mut [f32]) {
    map_inplace(x, |v| v.max(0.0));
}

/// LayerNorm über Zeilen der Länge `c` (Token-Layout).
pub fn layer_norm_rows(x: &mut [f32], c: usize, w: &[f32], b: &[f32], eps: f32) {
    x.par_chunks_mut(c * (CHUNK / c).max(1)).for_each(|ch| layer_norm_rows_seq(ch, c, w, b, eps));
}

/// LayerNorm über Zeilen der Länge `c`, ohne Parallelisierung (für kleine Blöcke).
pub fn layer_norm_rows_seq(x: &mut [f32], c: usize, w: &[f32], b: &[f32], eps: f32) {
    for row in x.chunks_mut(c) {
        let mut s = 0.0f32;
        for v in row.iter() {
            s += *v;
        }
        let mean = s / c as f32;
        let mut q = 0.0f32;
        for v in row.iter() {
            let d = *v - mean;
            q += d * d;
        }
        let inv = 1.0 / (q / c as f32 + eps).sqrt();
        for ((v, ww), bb) in row.iter_mut().zip(w).zip(b) {
            *v = (*v - mean) * inv * ww + bb;
        }
    }
}

/// LayerNorm über die Kanäle eines `[C, HW]`-Tensors (channels_first).
pub fn layer_norm_channels(x: &mut [f32], c: usize, w: &[f32], b: &[f32], eps: f32) {
    let hw = x.len() / c.max(1);
    let mut stats = vec![(0.0f32, 0.0f32); hw];
    stats.par_chunks_mut(1024).enumerate().for_each(|(ci, st)| {
        for (j, s) in st.iter_mut().enumerate() {
            let p = ci * 1024 + j;
            let mut mean = 0.0f64;
            for k in 0..c {
                mean += x[k * hw + p] as f64;
            }
            mean /= c as f64;
            let mut var = 0.0f64;
            for k in 0..c {
                var += (x[k * hw + p] as f64 - mean).powi(2);
            }
            var /= c as f64;
            *s = (mean as f32, (1.0 / (var + eps as f64).sqrt()) as f32);
        }
    });
    x.par_chunks_mut(hw.max(1)).enumerate().for_each(|(k, plane)| {
        for (p, v) in plane.iter_mut().enumerate() {
            let (m, inv) = stats[p];
            *v = (*v - m) * inv * w[k] + b[k];
        }
    });
}

/// Softmax über Zeilen der Länge `n`.
pub fn softmax_rows(x: &mut [f32], n: usize) {
    for row in x.chunks_mut(n) {
        let m = row.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let mut s = 0.0f32;
        for v in row.iter_mut() {
            let d = *v - m;
            // unter −80 ist der Beitrag 0 (und subnormale Zahlen wären sehr langsam)
            *v = if d < -80.0 { 0.0 } else { exp(d) };
            s += *v;
        }
        let inv = 1.0 / s;
        for v in row.iter_mut() {
            *v *= inv;
        }
    }
}

/// Transponiert `[r×c]` → `[c×r]`.
pub fn transpose(x: &[f32], r: usize, c: usize) -> Vec<f32> {
    let mut o = take(r * c);
    const B: usize = 32;
    o.par_chunks_mut(r * B).enumerate().for_each(|(bi, oc)| {
        let c0 = bi * B;
        let nc = oc.len() / r;
        for rb in (0..r).step_by(B) {
            for cc in 0..nc {
                let col = c0 + cc;
                let orow = &mut oc[cc * r..(cc + 1) * r];
                for rr in rb..(rb + B).min(r) {
                    orow[rr] = x[rr * c + col];
                }
            }
        }
    });
    o
}

/// Kanäle aneinanderhängen (`[C_i, HW]` → `[ΣC_i, HW]`).
pub fn concat_channels(parts: &[&[f32]]) -> Vec<f32> {
    let mut o = take(parts.iter().map(|p| p.len()).sum());
    let mut rest: &mut [f32] = &mut o;
    for p in parts {
        let (a, b) = rest.split_at_mut(p.len());
        a.par_chunks_mut(CHUNK).zip(p.par_chunks(CHUNK)).for_each(|(x, y)| x.copy_from_slice(y));
        rest = b;
    }
    o
}

// ------------------------------------------------------------------ Faltungen

/// Gewichte einer 2D-Faltung (PyTorch-Layout `[O, C, kh, kw]`).
#[derive(Clone, Debug, Default)]
pub struct Conv2d {
    pub w: Vec<f32>,
    pub b: Option<Vec<f32>>,
    pub cout: usize,
    pub cin: usize,
    pub k: usize,
    pub stride: usize,
    pub pad: usize,
    pub dil: usize,
}

impl Conv2d {
    /// Ausgabegröße.
    pub fn out_size(&self, h: usize, w: usize) -> (usize, usize) {
        let eff = self.dil * (self.k - 1) + 1;
        ((h + 2 * self.pad).saturating_sub(eff) / self.stride + 1, (w + 2 * self.pad).saturating_sub(eff) / self.stride + 1)
    }

    /// Faltet eine BatchNorm (eval) in Gewichte und Bias.
    pub fn fold_bn(&mut self, gamma: &[f32], beta: &[f32], mean: &[f32], var: &[f32], eps: f32) {
        let per = self.w.len() / self.cout.max(1);
        let mut b = self.b.take().unwrap_or_else(|| vec![0.0; self.cout]);
        for o in 0..self.cout {
            let s = gamma[o] / (var[o] + eps).sqrt();
            for v in &mut self.w[o * per..(o + 1) * per] {
                *v *= s;
            }
            b[o] = (b[o] - mean[o]) * s + beta[o];
        }
        self.b = Some(b);
    }

    /// Anwenden auf `x[C, H, W]` → `[O, H', W']`.
    pub fn forward(&self, x: &[f32], h: usize, w: usize) -> (Vec<f32>, usize, usize) {
        let (ho, wo) = self.out_size(h, w);
        let npos = ho * wo;
        let kk = self.k * self.k;
        let kdim = self.cin * kk;
        let direct = !(self.k == 1 && self.stride == 1 && self.pad == 0) && self.stride == 1 && (self.cout <= 4 || kdim <= 32) && npos >= 4096;
        let mut out = if direct { take_zeroed(self.cout * npos) } else { take(self.cout * npos) };
        if self.k == 1 && self.stride == 1 && self.pad == 0 {
            gemm(&self.w, self.cout, self.cin, x, npos, false, &mut out, false, true);
        } else if direct {
            // wenige Ausgänge bzw. sehr kurze Kerne: direkt zeilenweise (Spaltenmatrix lohnt nicht)
            out.par_chunks_mut(wo).enumerate().for_each(|(r, orow)| {
                let (o, oy) = (r / ho, r % ho);
                for c in 0..self.cin {
                    let plane = &x[c * h * w..(c + 1) * h * w];
                    for ky in 0..self.k {
                        let iy = (oy + ky * self.dil) as isize - self.pad as isize;
                        if iy < 0 || iy as usize >= h {
                            continue;
                        }
                        let irow = &plane[iy as usize * w..(iy as usize + 1) * w];
                        for kx in 0..self.k {
                            let wv = self.w[((o * self.cin + c) * self.k + ky) * self.k + kx];
                            let sh = (kx * self.dil) as isize - self.pad as isize;
                            // gültige Ausgabespalten: 0 ≤ ox + sh < w
                            let lo = (-sh).max(0) as usize;
                            let hi = ((w as isize - sh).min(wo as isize)).max(0) as usize;
                            if lo >= hi {
                                continue;
                            }
                            let src = &irow[(lo as isize + sh) as usize..(hi as isize + sh) as usize];
                            for (a, b) in orow[lo..hi].iter_mut().zip(src) {
                                *a += wv * b;
                            }
                        }
                    }
                }
            });
        } else {
            let im2col = |p0: usize, n: usize, col: &mut [f32]| {
                for c in 0..self.cin {
                    let plane = &x[c * h * w..(c + 1) * h * w];
                    for ky in 0..self.k {
                        for kx in 0..self.k {
                            let row = &mut col[((c * self.k + ky) * self.k + kx) * n..][..n];
                            for (j, r) in row.iter_mut().enumerate() {
                                let p = p0 + j;
                                let (oy, ox) = (p / wo, p % wo);
                                let iy = (oy * self.stride + ky * self.dil) as isize - self.pad as isize;
                                let ix = (ox * self.stride + kx * self.dil) as isize - self.pad as isize;
                                *r = if iy >= 0 && ix >= 0 && (iy as usize) < h && (ix as usize) < w { plane[iy as usize * w + ix as usize] } else { 0.0 };
                            }
                        }
                    }
                }
            };
            conv_by_columns(&self.w, self.cout, kdim, npos, &mut out, im2col);
        }
        if let Some(b) = &self.b {
            add_channel_bias(&mut out, b);
        }
        (out, ho, wo)
    }
}

/// Gemeinsamer Rahmen für Faltungen über eine Spaltenmatrix: `fill(p0, n, col)` füllt die
/// Spalten `p0..p0+n` (`col` hat das Layout `[kdim, n]`).
fn conv_by_columns(wt: &[f32], cout: usize, kdim: usize, npos: usize, out: &mut [f32], fill: impl Fn(usize, usize, &mut [f32]) + Sync) {
    let t = threads();
    // Spaltenblöcke: groß genug für einen effizienten SIMD-Kern, klein genug für den Cache und
    // mindestens zwei Blöcke je Thread.
    let budget = ((1usize << 20) / kdim.max(1)).max(32);
    let per_thread = npos.div_ceil(2 * t).max(16);
    let n = budget.min(per_thread).min(npos).max(1);
    if npos.div_ceil(n) < t && kdim * npos <= 48 << 20 {
        // Wenig Positionen: eine Spaltenmatrix (blockweise parallel gefüllt), faer verteilt die
        // Multiplikation selbst.
        let nb = npos.div_ceil(16).min(t * 2).max(1);
        let bw = npos.div_ceil(nb);
        let parts: Vec<Vec<f32>> = (0..npos.div_ceil(bw))
            .into_par_iter()
            .map(|bi| {
                let p0 = bi * bw;
                let nn = bw.min(npos - p0);
                let mut c = take(kdim * nn);
                fill(p0, nn, &mut c);
                c
            })
            .collect();
        let mut col = take(kdim * npos);
        col.par_chunks_mut(npos).enumerate().for_each(|(r, row)| {
            for (bi, part) in parts.iter().enumerate() {
                let p0 = bi * bw;
                let nn = bw.min(npos - p0);
                row[p0..p0 + nn].copy_from_slice(&part[r * nn..(r + 1) * nn]);
            }
        });
        parts.into_iter().for_each(give);
        gemm(wt, cout, kdim, &col, npos, false, out, false, true);
        give(col);
        return;
    }
    let nblk = npos.div_ceil(n);
    let blocks: Vec<Vec<f32>> = (0..nblk)
        .into_par_iter()
        .map(|bi| {
            let p0 = bi * n;
            let nn = n.min(npos - p0);
            let mut col = scratch_take(kdim * nn);
            fill(p0, nn, &mut col);
            let mut o = take(cout * nn);
            gemm(wt, cout, kdim, &col, nn, false, &mut o, false, false);
            scratch_give(col);
            o
        })
        .collect();
    out.par_chunks_mut(npos).enumerate().for_each(|(o, orow)| {
        for (bi, b) in blocks.iter().enumerate() {
            let p0 = bi * n;
            let nn = n.min(npos - p0);
            orow[p0..p0 + nn].copy_from_slice(&b[o * nn..(o + 1) * nn]);
        }
    });
    blocks.into_iter().for_each(give);
}

/// Modulierte deformierbare Faltung (DCNv2, wie `torchvision.ops.deform_conv2d` mit einer
/// Offset-Gruppe): `offset[2·kk, H', W']` (je Abtastpunkt dy, dx), `mask[kk, H', W']`.
pub fn deform_conv2d(x: &[f32], h: usize, w: usize, conv: &Conv2d, offset: &[f32], mask: &[f32]) -> (Vec<f32>, usize, usize) {
    let (ho, wo) = conv.out_size(h, w);
    let npos = ho * wo;
    let k = conv.k;
    let kk = k * k;
    let kdim = conv.cin * kk;
    let mut out = take(conv.cout * npos);
    let fill = |p0: usize, n: usize, col: &mut [f32]| {
        // Abtastgewichte je (Kernpunkt, Position) einmal berechnen, dann für alle Kanäle nutzen.
        let mut idx = vec![[0usize; 4]; kk * n];
        let mut wgt = vec![[0.0f32; 4]; kk * n];
        for t in 0..kk {
            let (ky, kx) = (t / k, t % k);
            for j in 0..n {
                let p = p0 + j;
                let (oy, ox) = (p / wo, p % wo);
                let dy = offset[(2 * t) * npos + p];
                let dx = offset[(2 * t + 1) * npos + p];
                let m = mask[t * npos + p];
                let py = (oy * conv.stride + ky * conv.dil) as f32 - conv.pad as f32 + dy;
                let px = (ox * conv.stride + kx * conv.dil) as f32 - conv.pad as f32 + dx;
                let e = t * n + j;
                if py <= -1.0 || px <= -1.0 || py >= h as f32 || px >= w as f32 {
                    wgt[e] = [0.0; 4];
                    continue;
                }
                let y0 = py.floor();
                let x0 = px.floor();
                let (ly, lx) = (py - y0, px - x0);
                let (hy, hx) = (1.0 - ly, 1.0 - lx);
                let (y0, x0) = (y0 as isize, x0 as isize);
                let corners = [(y0, x0, hy * hx), (y0, x0 + 1, hy * lx), (y0 + 1, x0, ly * hx), (y0 + 1, x0 + 1, ly * lx)];
                for (c, (yy, xx, ww)) in corners.iter().enumerate() {
                    if *yy >= 0 && *xx >= 0 && (*yy as usize) < h && (*xx as usize) < w {
                        idx[e][c] = *yy as usize * w + *xx as usize;
                        wgt[e][c] = ww * m;
                    } else {
                        idx[e][c] = 0;
                        wgt[e][c] = 0.0;
                    }
                }
            }
        }
        for c in 0..conv.cin {
            let plane = &x[c * h * w..(c + 1) * h * w];
            for t in 0..kk {
                let row = &mut col[(c * kk + t) * n..][..n];
                for (j, r) in row.iter_mut().enumerate() {
                    let e = t * n + j;
                    let (ii, ww) = (&idx[e], &wgt[e]);
                    *r = plane[ii[0]] * ww[0] + plane[ii[1]] * ww[1] + plane[ii[2]] * ww[2] + plane[ii[3]] * ww[3];
                }
            }
        }
    };
    conv_by_columns(&conv.w, conv.cout, kdim, npos, &mut out, fill);
    if let Some(b) = &conv.b {
        add_channel_bias(&mut out, b);
    }
    (out, ho, wo)
}

/// Transponierte Faltung mit Kern 2 und Schrittweite 2 (`w` als `[C, O, 2, 2]`), `x[C, H, W]` →
/// `[O, 2H, 2W]`.
pub fn conv_transpose_2x2(x: &[f32], c: usize, h: usize, w: usize, wt: &[f32], bias: &[f32], o: usize) -> Vec<f32> {
    // Gewichte als [O·4, C]
    let mut wr = vec![0.0f32; o * 4 * c];
    for ci in 0..c {
        for oi in 0..o {
            for t in 0..4 {
                wr[(oi * 4 + t) * c + ci] = wt[(ci * o + oi) * 4 + t];
            }
        }
    }
    let hw = h * w;
    let mut tmp = take(o * 4 * hw);
    gemm(&wr, o * 4, c, x, hw, false, &mut tmp, false, true);
    let (h2, w2) = (2 * h, 2 * w);
    let mut out = take(o * h2 * w2);
    out.par_chunks_mut(h2 * w2).enumerate().for_each(|(oi, plane)| {
        for t in 0..4 {
            let (i, j) = (t / 2, t % 2);
            let src = &tmp[(oi * 4 + t) * hw..(oi * 4 + t + 1) * hw];
            for y in 0..h {
                for xx in 0..w {
                    plane[(2 * y + i) * w2 + 2 * xx + j] = src[y * w + xx] + bias[oi];
                }
            }
        }
    });
    out
}

// ------------------------------------------------------------------ Skalierung

/// Bilineare Skalierung eines `[C, H, W]`-Tensors (PyTorch `interpolate(mode="bilinear")`).
pub fn resize_bilinear(x: &[f32], c: usize, h: usize, w: usize, ho: usize, wo: usize, align_corners: bool) -> Vec<f32> {
    if h == ho && w == wo {
        return take_copy(x);
    }
    let coord = |o: usize, n_in: usize, n_out: usize| -> (usize, usize, f32) {
        let s = if align_corners {
            if n_out > 1 { o as f32 * (n_in as f32 - 1.0) / (n_out as f32 - 1.0) } else { 0.0 }
        } else {
            ((o as f32 + 0.5) * n_in as f32 / n_out as f32 - 0.5).max(0.0)
        };
        let i0 = (s.floor() as usize).min(n_in - 1);
        let i1 = (i0 + 1).min(n_in - 1);
        (i0, i1, s - i0 as f32)
    };
    let ys: Vec<_> = (0..ho).map(|o| coord(o, h, ho)).collect();
    let xs: Vec<_> = (0..wo).map(|o| coord(o, w, wo)).collect();
    let mut out = take(c * ho * wo);
    out.par_chunks_mut(wo).enumerate().for_each(|(row, orow)| {
        let (ch, oy) = (row / ho, row % ho);
        let plane = &x[ch * h * w..(ch + 1) * h * w];
        let (y0, y1, fy) = ys[oy];
        let r0 = &plane[y0 * w..(y0 + 1) * w];
        let r1 = &plane[y1 * w..(y1 + 1) * w];
        for (ox, v) in orow.iter_mut().enumerate() {
            let (x0, x1, fx) = xs[ox];
            let a = r0[x0] + (r0[x1] - r0[x0]) * fx;
            let b = r1[x0] + (r1[x1] - r1[x0]) * fx;
            *v = a + (b - a) * fy;
        }
    });
    out
}

/// Nächster-Nachbar-Vergrößerung um Faktor 2 (`[C, H, W]`).
pub fn upsample_nearest_2x(x: &[f32], c: usize, h: usize, w: usize) -> Vec<f32> {
    let (h2, w2) = (2 * h, 2 * w);
    let mut out = take(c * h2 * w2);
    out.par_chunks_mut(w2).enumerate().for_each(|(row, orow)| {
        let (ch, oy) = (row / h2, row % h2);
        let src = &x[ch * h * w + (oy / 2) * w..][..w];
        for (ox, v) in orow.iter_mut().enumerate() {
            *v = src[ox / 2];
        }
    });
    out
}

/// Bikubische Skalierung (PyTorch `mode="bicubic"`, `align_corners=False`, A = −0,75).
pub fn resize_bicubic(x: &[f32], c: usize, h: usize, w: usize, ho: usize, wo: usize) -> Vec<f32> {
    let a = -0.75f32;
    let cc1 = |t: f32| ((a + 2.0) * t - (a + 3.0)) * t * t + 1.0;
    let cc2 = |t: f32| ((a * t - 5.0 * a) * t + 8.0 * a) * t - 4.0 * a;
    let weights = |t: f32| [cc2(t + 1.0), cc1(t), cc1(1.0 - t), cc2(2.0 - t)];
    let mut out = vec![0.0f32; c * ho * wo];
    let sy = h as f32 / ho as f32;
    let sx = w as f32 / wo as f32;
    for ch in 0..c {
        let plane = &x[ch * h * w..(ch + 1) * h * w];
        for oy in 0..ho {
            let fy = (oy as f32 + 0.5) * sy - 0.5;
            let iy = fy.floor();
            let wy = weights(fy - iy);
            for ox in 0..wo {
                let fx = (ox as f32 + 0.5) * sx - 0.5;
                let ix = fx.floor();
                let wx = weights(fx - ix);
                let mut s = 0.0f32;
                for (m, wyy) in wy.iter().enumerate() {
                    let yy = (iy as isize - 1 + m as isize).clamp(0, h as isize - 1) as usize;
                    let mut r = 0.0f32;
                    for (n, wxx) in wx.iter().enumerate() {
                        let xx = (ix as isize - 1 + n as isize).clamp(0, w as isize - 1) as usize;
                        r += plane[yy * w + xx] * wxx;
                    }
                    s += r * wyy;
                }
                out[ch * ho * wo + oy * wo + ox] = s;
            }
        }
    }
    out
}

/// 2×2-Max-Pooling auf Token im Layout `[B, H, W, C]` (Schrittweite 2, ohne Aufrunden).
pub fn max_pool_2x2_nhwc(x: &[f32], b: usize, h: usize, w: usize, c: usize) -> (Vec<f32>, usize, usize) {
    let (ho, wo) = (h / 2, w / 2);
    let mut out = take(b * ho * wo * c);
    out.par_chunks_mut(wo * c).enumerate().for_each(|(row, orow)| {
        let (bi, oy) = (row / ho.max(1), row % ho.max(1));
        let base = bi * h * w * c;
        for ox in 0..wo {
            for k in 0..c {
                let i00 = base + ((2 * oy) * w + 2 * ox) * c + k;
                let v = x[i00].max(x[i00 + c]).max(x[i00 + w * c]).max(x[i00 + w * c + c]);
                orow[ox * c + k] = v;
            }
        }
    });
    (out, ho, wo)
}

// ------------------------------------------------------------------ Aufmerksamkeit

/// Lage von Q, K oder V in einem gepackten Puffer: Zeile `r` von Kopf `h` beginnt bei
/// `r · stride + off + h · d`.
#[derive(Clone, Copy)]
pub struct Packed<'a> {
    pub buf: &'a [f32],
    pub stride: usize,
    pub off: usize,
}

impl<'a> Packed<'a> {
    /// Dichtes Layout `[n, heads·d]`.
    pub fn dense(buf: &'a [f32], dm: usize) -> Packed<'a> {
        Packed { buf, stride: dm, off: 0 }
    }
    fn view(&self, row0: usize, n: usize, col: usize, d: usize) -> MatRef<'a, f32> {
        let start = row0 * self.stride + self.off + col;
        let len = if n == 0 { 0 } else { (n - 1) * self.stride + d };
        MatRef::from_row_major_slice_with_stride(&self.buf[start..start + len], n, d, self.stride)
    }
}

/// Zusatz zu den Aufmerksamkeitsgewichten: `bias(gruppe, kopf, zeile0, s)` addiert auf den Block
/// `s[nn × nk]` ab Anfragezeile `zeile0`.
pub type AttnBias<'a> = &'a (dyn Fn(usize, usize, usize, &mut [f32]) + Sync);

/// Skalarprodukt mit acht unabhängigen Summen (vom Compiler vektorisiert).
#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0.0f32; 8];
    let (ca, ra) = a.as_chunks::<8>();
    let (cb, rb) = b.as_chunks::<8>();
    for (x, y) in ca.iter().zip(cb) {
        for l in 0..8 {
            acc[l] += x[l] * y[l];
        }
    }
    let mut s = (acc[0] + acc[4]) + (acc[1] + acc[5]) + (acc[2] + acc[6]) + (acc[3] + acc[7]);
    for (x, y) in ra.iter().zip(rb) {
        s += x * y;
    }
    s
}

/// Ein Kopf, ein Anfrageblock: `o[nn × d]` (Zeilenabstand `os`) = softmax(scale·Q·Kᵀ + bias)·V.
#[allow(clippy::too_many_arguments)]
fn attend_block(q: &Packed, k: &Packed, v: &Packed, qrow0: usize, krow0: usize, nn: usize, nk: usize, col: usize, d: usize, scale: f32, s: &mut [f32], o: &mut [f32], os: usize, bias: impl Fn(&mut [f32])) {
    if nk <= 256 && d <= 128 {
        // kleine Fenster: direkte Schleifen (faer-Aufrufe wären hier teurer als die Rechnung)
        for i in 0..nn {
            let qi = &q.buf[(qrow0 + i) * q.stride + q.off + col..][..d];
            for j in 0..nk {
                let kj = &k.buf[(krow0 + j) * k.stride + k.off + col..][..d];
                s[i * nk + j] = dot(qi, kj) * scale;
            }
        }
        bias(s);
        softmax_rows(s, nk);
        for i in 0..nn {
            let oi = &mut o[i * os..i * os + d];
            oi.fill(0.0);
            for j in 0..nk {
                let w = s[i * nk + j];
                if w == 0.0 {
                    continue;
                }
                let vj = &v.buf[(krow0 + j) * v.stride + v.off + col..][..d];
                for (a, b) in oi.iter_mut().zip(vj) {
                    *a += w * b;
                }
            }
        }
    } else {
        let qm = q.view(qrow0, nn, col, d);
        let km = k.view(krow0, nk, col, d);
        matmul(MatMut::from_row_major_slice_mut(s, nn, nk), Accum::Replace, qm, km.transpose(), scale, Par::Seq);
        bias(s);
        softmax_rows(s, nk);
        let vm = v.view(krow0, nk, col, d);
        let len = (nn - 1) * os + d;
        // Hinweis: `MatMut::from_row_major_slice_with_stride_mut` legt in faer 0.24.4 eine
        // spaltenweise Ansicht an (Schreibzugriffe außerhalb des Bereichs) – deshalb hier über die
        // spaltenweise Ansicht der Transponierten.
        let om = MatMut::from_column_major_slice_with_stride_mut(&mut o[..len], d, nn, os).transpose_mut();
        matmul(om, Accum::Replace, MatRef::from_row_major_slice(s, nn, nk), vm, 1.0f32, Par::Seq);
    }
}

/// Mehrkopf-Aufmerksamkeit für `nb` unabhängige Gruppen (z. B. Fenster) direkt auf gepackten
/// Puffern (kein Umkopieren der Köpfe). Ergebnis `[nb·nq, heads·d]`.
#[allow(clippy::too_many_arguments)]
pub fn attention_packed(q: Packed, k: Packed, v: Packed, nb: usize, nq: usize, nk: usize, heads: usize, d: usize, scale: f32, bias: Option<AttnBias>) -> Vec<f32> {
    let dm = heads * d;
    if nb == 0 || nq == 0 || nk == 0 {
        return vec![0.0f32; nb * nq * dm];
    }
    let mut out = take(nb * nq * dm);
    let t = threads();
    let call_bias = |g: usize, h: usize, q0: usize, s: &mut [f32]| {
        if let Some(f) = bias {
            f(g, h, q0, s);
        }
    };
    if nb >= 2 * t {
        // viele Gruppen (Fenster): je Gruppe eine Aufgabe, alle Köpfe darin
        out.par_chunks_mut(nq * dm).enumerate().for_each(|(g, gc)| {
            let mut s = scratch_take(nq * nk);
            for h in 0..heads {
                attend_block(&q, &k, &v, g * nq, g * nk, nq, nk, h * d, d, scale, &mut s, &mut gc[h * d..], dm, |s| call_bias(g, h, 0, s));
            }
            scratch_give(s);
        });
        return out;
    }
    // wenige Gruppen (globale Aufmerksamkeit): Aufgaben je (Gruppe, Kopf, Anfrageblock), Ergebnisse
    // danach einsortieren; Zwischenspeicher je Aufgabe höchstens 4 MB.
    let per = (nb * heads).max(1);
    let mut qb = nq.div_ceil((3 * t).div_ceil(per)).max(32);
    qb = qb.min(((1usize << 20) / nk).max(8)).min(nq);
    let nblk = nq.div_ceil(qb);
    // Kᵀ und V je (Gruppe, Kopf) einmal dicht ablegen: faer ist auf zusammenhängenden Matrizen um
    // ein Vielfaches schneller als auf gestaffelten Ansichten.
    let tp = std::time::Instant::now();
    let kv: Vec<(Vec<f32>, Vec<f32>)> = (0..if nk > 256 { nb * heads } else { 0 })
        .into_par_iter()
        .map(|gh| {
            let (g, h) = (gh / heads, gh % heads);
            let mut kt = vec![0.0f32; d * nk];
            let mut vv = vec![0.0f32; nk * d];
            for j in 0..nk {
                let kr = &k.buf[(g * nk + j) * k.stride + k.off + h * d..][..d];
                for (t, val) in kr.iter().enumerate() {
                    kt[t * nk + j] = *val;
                }
                vv[j * d..(j + 1) * d].copy_from_slice(&v.buf[(g * nk + j) * v.stride + v.off + h * d..][..d]);
            }
            (kt, vv)
        })
        .collect();
    let tp = prof("attn kv", tp);
    let parts: Vec<(usize, usize, usize, Vec<f32>)> = (0..nb * heads * nblk)
        .into_par_iter()
        .map(|task| {
            let g = task / (heads * nblk);
            let h = (task / nblk) % heads;
            let q0 = (task % nblk) * qb;
            let nn = qb.min(nq - q0);
            let mut s = scratch_take(nn * nk);
            let mut o = vec![0.0f32; nn * d];
            if nk > 256 {
                let mut qq = scratch_take(nn * d);
                for i in 0..nn {
                    let src = &q.buf[(g * nq + q0 + i) * q.stride + q.off + h * d..][..d];
                    for (a, b) in qq[i * d..(i + 1) * d].iter_mut().zip(src) {
                        *a = *b * scale;
                    }
                }
                let (kt, vv) = &kv[g * heads + h];
                gemm(&qq, nn, d, kt, nk, false, &mut s, false, false);
                call_bias(g, h, q0, &mut s);
                softmax_rows(&mut s, nk);
                gemm(&s, nn, nk, vv, d, false, &mut o, false, false);
                scratch_give(qq);
            } else {
                attend_block(&q, &k, &v, g * nq + q0, g * nk, nn, nk, h * d, d, scale, &mut s, &mut o, d, |s| call_bias(g, h, q0, s));
            }
            scratch_give(s);
            (g, h, q0, o)
        })
        .collect();
    let tp = prof("attn parts", tp);
    for (g, h, q0, o) in parts {
        for (i, row) in o.chunks(d).enumerate() {
            out[(g * nq + q0 + i) * dm + h * d..][..d].copy_from_slice(row);
        }
    }
    prof("attn scatter", tp);
    out
}

/// Mehrkopf-Aufmerksamkeit auf dichten Puffern: `q[nb·nq, heads·d]`, `k`/`v[nb·nk, heads·d]`.
#[allow(clippy::too_many_arguments)]
pub fn attention(q: &[f32], k: &[f32], v: &[f32], nb: usize, nq: usize, nk: usize, heads: usize, d: usize, scale: f32, bias: Option<AttnBias>) -> Vec<f32> {
    let dm = heads * d;
    attention_packed(Packed::dense(q, dm), Packed::dense(k, dm), Packed::dense(v, dm), nb, nq, nk, heads, d, scale, bias)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_conv(x: &[f32], h: usize, w: usize, c: &Conv2d) -> Vec<f32> {
        let (ho, wo) = c.out_size(h, w);
        let mut o = vec![0.0; c.cout * ho * wo];
        for oc in 0..c.cout {
            for oy in 0..ho {
                for ox in 0..wo {
                    let mut s = c.b.as_ref().map(|b| b[oc]).unwrap_or(0.0);
                    for ic in 0..c.cin {
                        for ky in 0..c.k {
                            for kx in 0..c.k {
                                let iy = (oy * c.stride + ky * c.dil) as isize - c.pad as isize;
                                let ix = (ox * c.stride + kx * c.dil) as isize - c.pad as isize;
                                if iy >= 0 && ix >= 0 && (iy as usize) < h && (ix as usize) < w {
                                    s += x[ic * h * w + iy as usize * w + ix as usize] * c.w[((oc * c.cin + ic) * c.k + ky) * c.k + kx];
                                }
                            }
                        }
                    }
                    o[oc * ho * wo + oy * wo + ox] = s;
                }
            }
        }
        o
    }

    fn rnd(n: usize, seed: u32) -> Vec<f32> {
        let mut s = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
        (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                (s % 2000) as f32 / 1000.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn conv_matches_naive() {
        for (k, stride, pad, dil, h, w) in [(3, 1, 1, 1, 9, 7), (7, 4, 3, 1, 17, 13), (1, 1, 0, 1, 5, 6), (3, 1, 2, 2, 8, 8), (2, 2, 0, 1, 8, 6), (3, 1, 1, 1, 70, 64), (3, 1, 2, 2, 66, 67), (7, 1, 3, 1, 64, 80)] {
            let c = Conv2d { w: rnd(4 * 3 * k * k, 1), b: Some(rnd(4, 2)), cout: 4, cin: 3, k, stride, pad, dil };
            let x = rnd(3 * h * w, 3);
            let (o, _, _) = c.forward(&x, h, w);
            let r = naive_conv(&x, h, w, &c);
            for (a, b) in o.iter().zip(&r) {
                assert!((a - b).abs() < 1e-4, "{a} {b}");
            }
        }
    }

    #[test]
    fn deform_zero_offset_is_conv() {
        let c = Conv2d { w: rnd(5 * 4 * 9, 4), b: None, cout: 5, cin: 4, k: 3, stride: 1, pad: 1, dil: 1 };
        let (h, w) = (6, 9);
        let x = rnd(4 * h * w, 5);
        let off = vec![0.0; 18 * h * w];
        let m = vec![1.0; 9 * h * w];
        let (o, _, _) = deform_conv2d(&x, h, w, &c, &off, &m);
        let r = naive_conv(&x, h, w, &c);
        for (a, b) in o.iter().zip(&r) {
            assert!((a - b).abs() < 1e-4);
        }
        // Verschiebung um genau ein Pixel nach rechts = Faltung des verschobenen Bildes
        let mut off = vec![0.0; 18 * h * w];
        for t in 0..9 {
            for p in 0..h * w {
                off[(2 * t + 1) * h * w + p] = 1.0;
            }
        }
        let (o2, _, _) = deform_conv2d(&x, h, w, &c, &off, &m);
        let mut xs = vec![0.0; x.len()];
        for ch in 0..4 {
            for y in 0..h {
                for xx in 0..w - 1 {
                    xs[ch * h * w + y * w + xx] = x[ch * h * w + y * w + xx + 1];
                }
            }
        }
        let r2 = naive_conv(&xs, h, w, &c);
        // am linken Rand liest die verschobene Abtastung echte Pixel statt Auffüllung
        for (i, (a, b)) in o2.iter().zip(&r2).enumerate() {
            if i % w >= 1 {
                assert!((a - b).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn exp_accuracy() {
        let mut x = -87.0f32;
        while x < 88.0 {
            let (a, b) = (exp(x), x.exp());
            assert!(((a - b) / b).abs() < 3e-7, "{x}: {a} {b}");
            x += 0.0137;
        }
        assert_eq!(exp(-200.0), exp(-87.3));
    }

    #[test]
    fn erf_accuracy() {
        for (x, e) in [(0.0f32, 0.0f32), (0.5, 0.520_499_9), (1.0, 0.842_700_8), (-2.0, -0.995_322_3), (3.0, 0.999_977_9)] {
            assert!((erf(x) - e).abs() < 4e-7, "{x}");
        }
    }

    #[test]
    fn attention_matches_naive() {
        let (nb, nq, nk, heads, d) = (2, 5, 7, 2, 3);
        let q = rnd(nb * nq * heads * d, 7);
        let k = rnd(nb * nk * heads * d, 8);
        let v = rnd(nb * nk * heads * d, 9);
        let o = attention(&q, &k, &v, nb, nq, nk, heads, d, 0.5, None);
        for g in 0..nb {
            for hh in 0..heads {
                for i in 0..nq {
                    let mut s: Vec<f32> = (0..nk)
                        .map(|j| (0..d).map(|t| q[(g * nq + i) * heads * d + hh * d + t] * 0.5 * k[(g * nk + j) * heads * d + hh * d + t]).sum())
                        .collect();
                    softmax_rows(&mut s, nk);
                    for t in 0..d {
                        let e: f32 = (0..nk).map(|j| s[j] * v[(g * nk + j) * heads * d + hh * d + t]).sum();
                        assert!((o[(g * nq + i) * heads * d + hh * d + t] - e).abs() < 1e-5);
                    }
                }
            }
        }
    }

    #[test]
    fn packed_attention_matches_dense() {
        let (nb, n, heads, d) = (3, 49, 3, 32);
        let c = heads * d;
        let qkv = rnd(nb * n * 3 * c, 11);
        let mut q = Vec::new();
        let mut k = Vec::new();
        let mut v = Vec::new();
        for r in qkv.chunks(3 * c) {
            q.extend_from_slice(&r[..c]);
            k.extend_from_slice(&r[c..2 * c]);
            v.extend_from_slice(&r[2 * c..]);
        }
        let a = attention(&q, &k, &v, nb, n, n, heads, d, 0.3, None);
        let pk = |off| Packed { buf: &qkv, stride: 3 * c, off };
        let b = attention_packed(pk(0), pk(c), pk(2 * c), nb, n, n, heads, d, 0.3, None);
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 1e-5);
        }
        // eine Gruppe, viele Anfragen (Blockbildung)
        let (n2, h2) = (300, 2);
        let qq = rnd(n2 * h2 * 8, 12);
        let o = attention(&qq, &qq, &qq, 1, n2, n2, h2, 8, 0.5, None);
        assert_eq!(o.len(), n2 * h2 * 8);
        assert!(o.iter().all(|x| x.is_finite()));
    }

    #[test]
    fn bilinear_align_corners() {
        let x = vec![0.0, 1.0, 2.0, 3.0];
        let o = resize_bilinear(&x, 1, 2, 2, 3, 3, true);
        assert_eq!(o, vec![0.0, 0.5, 1.0, 1.0, 1.5, 2.0, 2.0, 2.5, 3.0]);
        let o = resize_bilinear(&x, 1, 2, 2, 4, 4, false);
        assert!((o[0] - 0.0).abs() < 1e-6 && (o[1] - 0.25).abs() < 1e-6 && (o[15] - 3.0).abs() < 1e-6);
    }
}
