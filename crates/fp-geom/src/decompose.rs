//! Matrix-Zerlegung und Interpolation für das Tweening (Spezifikation 13 §7.2).
//!
//! Das Original zerlegt die affine Abbildung jeder Instanz nach Shoemake/Duff („Matrix Animation and
//! Polar Decomposition“, Graphics Gems IV) und interpoliert die Teile: Verschiebung `t` linear,
//! Drehung `q` und Streckachsen `u` per Quaternion-Slerp, Streckfaktoren `k` linear, das Vorzeichen
//! der Determinante `f` springt bei `t > 0,5` um. Die 3×3-Matrix wird dazu in eine 4×4-Matrix
//! eingebettet (z-Achse = Einheit); entsprechend rechnet dieses Modul mit 3×3-Linearteilen.
//!
//! Umsetzung nach der veröffentlichten Beschreibung des Verfahrens (Polarzerlegung per
//! Newton-Iteration mit Normskalierung, Jacobi-Eigenzerlegung, „snuggle“-Wahl der Streckachsen),
//! in `f64`. Intern gilt die Spaltenvektor-Konvention des Aufsatzes: `A = T · F · R(q) · U · K · Uᵀ`;
//! [`AffineParts::compose`] liefert wieder eine [`Matrix3`] in Zeilenvektor-Form.
//!
//! Annahme (A): Die Slerp-Variante des Originals (`@005cdb30`) ist nicht spezifiziert; hier wird
//! der kürzere Weg gewählt (Vorzeichenwechsel bei negativem Skalarprodukt) und bei fast parallelen
//! Quaternionen linear interpoliert.

#![allow(clippy::needless_range_loop)] // Matrix-/Kanalindizes sind hier lesbarer

use crate::Matrix3;
use std::ops::Mul;

type M3 = [[f64; 3]; 3];

const TOL: f64 = 1.0e-6;
const SQRTHALF: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Quaternion `(x, y, z, w)`, `w` = Realteil.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Quat {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Default for Quat {
    fn default() -> Self {
        Quat::IDENTITY
    }
}

impl Quat {
    /// Einheitsquaternion (keine Drehung).
    pub const IDENTITY: Quat = Quat {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    pub const fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self { x, y, z, w }
    }

    /// Konjugiertes Quaternion (= Inverse bei Einheitslänge).
    pub fn conj(self) -> Quat {
        Quat::new(-self.x, -self.y, -self.z, self.w)
    }

    /// Skalarprodukt.
    pub fn dot(self, o: Quat) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }

    fn scale(self, s: f64) -> Quat {
        Quat::new(self.x * s, self.y * s, self.z * s, self.w * s)
    }

    fn normalized(self) -> Quat {
        let n = self.dot(self).sqrt();
        if n > 0.0 && n.is_finite() {
            self.scale(1.0 / n)
        } else {
            Quat::IDENTITY
        }
    }

    /// Drehung um die z-Achse (in der Bildebene) um `radians`.
    pub fn from_z_rotation(radians: f64) -> Quat {
        let (s, c) = (radians * 0.5).sin_cos();
        Quat::new(0.0, 0.0, s, c)
    }

    /// Drehmatrix (Spaltenvektor-Konvention) eines Einheitsquaternions.
    fn to_matrix(self) -> M3 {
        let Quat { x, y, z, w } = self;
        [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
            ],
        ]
    }

    /// Quaternion aus einer Drehmatrix (Spaltenvektor-Konvention, Shoemake-Verfahren mit Wahl
    /// der größten Diagonale für numerische Stabilität).
    fn from_matrix(m: &M3) -> Quat {
        let tr = m[0][0] + m[1][1] + m[2][2];
        if tr >= 0.0 {
            let s = (tr + 1.0).sqrt();
            let w = s * 0.5;
            let s = 0.5 / s;
            Quat::new(
                (m[2][1] - m[1][2]) * s,
                (m[0][2] - m[2][0]) * s,
                (m[1][0] - m[0][1]) * s,
                w,
            )
        } else {
            let mut h = 0;
            if m[1][1] > m[0][0] {
                h = 1;
            }
            if m[2][2] > m[h][h] {
                h = 2;
            }
            let (i, j, k) = match h {
                0 => (0, 1, 2),
                1 => (1, 2, 0),
                _ => (2, 0, 1),
            };
            let s = ((m[i][i] - (m[j][j] + m[k][k])) + 1.0).sqrt();
            let mut q = [0.0; 4];
            q[i] = s * 0.5;
            let s = 0.5 / s;
            q[j] = (m[i][j] + m[j][i]) * s;
            q[k] = (m[k][i] + m[i][k]) * s;
            q[3] = (m[k][j] - m[j][k]) * s;
            Quat::new(q[0], q[1], q[2], q[3])
        }
    }

    /// Sphärische Interpolation (kürzester Weg, Annahme A, siehe Moduldoku).
    pub fn slerp(self, o: Quat, t: f64) -> Quat {
        let mut b = o;
        let mut cos = self.dot(o);
        if cos < 0.0 {
            cos = -cos;
            b = b.scale(-1.0);
        }
        let (s0, s1) = if 1.0 - cos > 1e-9 {
            let omega = cos.min(1.0).acos();
            let sin = omega.sin();
            (((1.0 - t) * omega).sin() / sin, (t * omega).sin() / sin)
        } else {
            (1.0 - t, t)
        };
        Quat::new(
            self.x * s0 + b.x * s1,
            self.y * s0 + b.y * s1,
            self.z * s0 + b.z * s1,
            self.w * s0 + b.w * s1,
        )
        .normalized()
    }
}

impl Mul for Quat {
    type Output = Quat;
    /// Hamilton-Produkt `self · r`.
    fn mul(self, r: Quat) -> Quat {
        let l = self;
        Quat {
            w: l.w * r.w - l.x * r.x - l.y * r.y - l.z * r.z,
            x: l.w * r.x + l.x * r.w + l.y * r.z - l.z * r.y,
            y: l.w * r.y + l.y * r.w + l.z * r.x - l.x * r.z,
            z: l.w * r.z + l.z * r.w + l.x * r.y - l.y * r.x,
        }
    }
}

/// Bestandteile einer affinen Abbildung nach der Polarzerlegung.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AffineParts {
    /// Verschiebung.
    pub t: [f64; 2],
    /// Wesentliche Drehung.
    pub q: Quat,
    /// Drehung der Streckachsen.
    pub u: Quat,
    /// Streckfaktoren (x, y, z).
    pub k: [f64; 3],
    /// Vorzeichen der Determinante (+1 oder −1, Spiegelung).
    pub f: f64,
}

fn mat_mul(a: &M3, b: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j];
        }
    }
    r
}

fn transpose(a: &M3) -> M3 {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = a[j][i];
        }
    }
    r
}

fn cross(a: &[f64; 3], b: &[f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn vdot(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Transponierte Adjunkte: Zeilen = Kreuzprodukte der Zeilen von `m`.
fn adjoint_t(m: &M3) -> M3 {
    [
        cross(&m[1], &m[2]),
        cross(&m[2], &m[0]),
        cross(&m[0], &m[1]),
    ]
}

/// Spaltensummen-Norm (`tpose = false`) bzw. Zeilensummen-Norm (`tpose = true`).
fn mat_norm(m: &M3, tpose: bool) -> f64 {
    let mut max = 0.0f64;
    for i in 0..3 {
        let sum = if tpose {
            m[0][i].abs() + m[1][i].abs() + m[2][i].abs()
        } else {
            m[i][0].abs() + m[i][1].abs() + m[i][2].abs()
        };
        if max < sum {
            max = sum;
        }
    }
    max
}

fn norm_inf(m: &M3) -> f64 {
    mat_norm(m, false)
}

fn norm_one(m: &M3) -> f64 {
    mat_norm(m, true)
}

/// Spalte des betragsgrößten Eintrags; `None`, wenn alle 0.
fn find_max_col(m: &M3) -> Option<usize> {
    let mut max = 0.0;
    let mut col = None;
    for row in m.iter() {
        for (j, v) in row.iter().enumerate() {
            let a = v.abs();
            if a > max {
                max = a;
                col = Some(j);
            }
        }
    }
    col
}

/// Householder-Vektor, der `v` auf ein Vielfaches von (0,0,1) spiegelt.
fn make_reflector(v: &[f64; 3]) -> [f64; 3] {
    let s = vdot(v, v).sqrt();
    let mut u = *v;
    u[2] += if v[2] < 0.0 { -s } else { s };
    let d = vdot(&u, &u);
    if d == 0.0 {
        return [0.0; 3];
    }
    let s = (2.0 / d).sqrt();
    [u[0] * s, u[1] * s, u[2] * s]
}

fn reflect_cols(m: &mut M3, u: &[f64; 3]) {
    for i in 0..3 {
        let s = u[0] * m[0][i] + u[1] * m[1][i] + u[2] * m[2][i];
        for j in 0..3 {
            m[j][i] -= u[j] * s;
        }
    }
}

fn reflect_rows(m: &mut M3, u: &[f64; 3]) {
    for i in 0..3 {
        let s = vdot(u, &m[i]);
        for j in 0..3 {
            m[i][j] -= u[j] * s;
        }
    }
}

const IDENT: M3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Orthogonaler Faktor einer Matrix vom Rang 1.
fn do_rank1(m: &mut M3) -> M3 {
    let mut q = IDENT;
    let Some(col) = find_max_col(m) else { return q };
    let v1 = make_reflector(&[m[0][col], m[1][col], m[2][col]]);
    reflect_cols(m, &v1);
    let v2 = make_reflector(&m[2]);
    reflect_rows(m, &v2);
    if m[2][2] < 0.0 {
        q[2][2] = -1.0;
    }
    reflect_cols(&mut q, &v1);
    reflect_rows(&mut q, &v2);
    q
}

/// Orthogonaler Faktor einer Matrix vom Rang 2 (`madj_t` = transponierte Adjunkte).
fn do_rank2(m: &mut M3, madj_t: &M3) -> M3 {
    let Some(col) = find_max_col(madj_t) else {
        return do_rank1(m);
    };
    let v1 = make_reflector(&[madj_t[0][col], madj_t[1][col], madj_t[2][col]]);
    reflect_cols(m, &v1);
    let v2 = make_reflector(&cross(&m[0], &m[1]));
    reflect_rows(m, &v2);
    let (w, x, y, z) = (m[0][0], m[0][1], m[1][0], m[1][1]);
    let mut q = IDENT;
    if w * z > x * y {
        let (c, s) = (z + w, y - x);
        let d = c.hypot(s);
        let (c, s) = if d > 0.0 { (c / d, s / d) } else { (1.0, 0.0) };
        q[0][0] = c;
        q[1][1] = c;
        q[0][1] = -s;
        q[1][0] = s;
    } else {
        let (c, s) = (z - w, y + x);
        let d = c.hypot(s);
        let (c, s) = if d > 0.0 { (c / d, s / d) } else { (1.0, 0.0) };
        q[0][0] = -c;
        q[1][1] = c;
        q[0][1] = s;
        q[1][0] = s;
    }
    reflect_cols(&mut q, &v1);
    reflect_rows(&mut q, &v2);
    q
}

/// Polarzerlegung `M = Q · S` (Q orthogonal, S symmetrisch positiv semidefinit).
/// Rückgabe `(Q, S, det)`; `det` = Determinante von Q (Vorzeichen).
fn polar_decomp(m: &M3) -> (M3, M3, f64) {
    let mut mk = transpose(m);
    let mut mk_one = norm_one(&mk);
    let mut mk_inf = norm_inf(&mk);
    let mut det;
    let mut iter = 0;
    loop {
        let madj_t = adjoint_t(&mk);
        det = vdot(&mk[0], &madj_t[0]);
        if det == 0.0 {
            mk = do_rank2(&mut mk, &madj_t);
            break;
        }
        let madj_one = norm_one(&madj_t);
        let madj_inf = norm_inf(&madj_t);
        let gamma = ((madj_one * madj_inf / (mk_one * mk_inf)).sqrt() / det.abs()).sqrt();
        let g1 = gamma * 0.5;
        let g2 = 0.5 / (gamma * det);
        let ek_prev = mk;
        for i in 0..3 {
            for j in 0..3 {
                mk[i][j] = g1 * mk[i][j] + g2 * madj_t[i][j];
            }
        }
        let mut ek = ek_prev;
        for i in 0..3 {
            for j in 0..3 {
                ek[i][j] -= mk[i][j];
            }
        }
        let e_one = norm_one(&ek);
        mk_one = norm_one(&mk);
        mk_inf = norm_inf(&mk);
        iter += 1;
        if e_one <= mk_one * TOL || iter >= 100 || !e_one.is_finite() {
            break;
        }
    }
    let q = transpose(&mk);
    let mut s = mat_mul(&mk, m);
    for i in 0..3 {
        for j in i..3 {
            let v = 0.5 * (s[i][j] + s[j][i]);
            s[i][j] = v;
            s[j][i] = v;
        }
    }
    let det = if det == 0.0 { determinant(&q) } else { det };
    (q, s, det)
}

fn determinant(m: &M3) -> f64 {
    vdot(&m[0], &cross(&m[1], &m[2]))
}

/// Jacobi-Eigenzerlegung einer symmetrischen Matrix: `S = U · diag(k) · Uᵀ`.
fn spect_decomp(s: &M3) -> (M3, [f64; 3]) {
    const NXT: [usize; 3] = [1, 2, 0];
    let mut u = IDENT;
    let mut diag = [s[0][0], s[1][1], s[2][2]];
    let mut offd = [s[1][2], s[2][0], s[0][1]];
    for _sweep in 0..20 {
        let sm = offd[0].abs() + offd[1].abs() + offd[2].abs();
        if sm == 0.0 {
            break;
        }
        for i in (0..3).rev() {
            let p = NXT[i];
            let q = NXT[p];
            let fabs_offdi = offd[i].abs();
            let g = 100.0 * fabs_offdi;
            if fabs_offdi > 0.0 {
                let h = diag[q] - diag[p];
                let fabsh = h.abs();
                let t = if fabsh + g == fabsh {
                    offd[i] / h
                } else {
                    let theta = 0.5 * h / offd[i];
                    let t = 1.0 / (theta.abs() + (theta * theta + 1.0).sqrt());
                    if theta < 0.0 { -t } else { t }
                };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let sn = t * c;
                let tau = sn / (c + 1.0);
                let ta = t * offd[i];
                offd[i] = 0.0;
                diag[p] -= ta;
                diag[q] += ta;
                let offdq = offd[q];
                offd[q] -= sn * (offd[p] + tau * offd[q]);
                offd[p] += sn * (offdq - tau * offd[p]);
                for j in (0..3).rev() {
                    let a = u[j][p];
                    let b = u[j][q];
                    u[j][p] -= sn * (b + tau * a);
                    u[j][q] += sn * (a - tau * b);
                }
            }
        }
    }
    (u, diag)
}

fn qarr(q: Quat) -> [f64; 4] {
    [q.x, q.y, q.z, q.w]
}

/// Wählt unter den gleichwertigen Achsenvertauschungen von `q` die Drehung, die der Einheit am
/// nächsten liegt, und vertauscht die Streckfaktoren `k` entsprechend. Liefert die Korrektur `p`,
/// mit der `u · p` die neue Streckachsen-Drehung ist.
fn snuggle(q: Quat, k: &mut [f64; 3]) -> Quat {
    let mut ka = [k[0], k[1], k[2]];
    fn swap(a: &mut [f64; 3], i: usize, j: usize) {
        a.swap(i, j);
    }
    fn cycle(a: &mut [f64; 3], p: bool) {
        if p {
            a.rotate_left(1);
        } else {
            a.rotate_right(1);
        }
    }
    let turn: Option<usize> = if ka[0] == ka[1] {
        if ka[0] == ka[2] { Some(3) } else { Some(2) }
    } else if ka[0] == ka[2] {
        Some(1)
    } else if ka[1] == ka[2] {
        Some(0)
    } else {
        None
    };
    let p;
    if let Some(turn) = turn {
        let qxtoz = Quat::new(0.0, SQRTHALF, 0.0, SQRTHALF);
        let qytoz = Quat::new(SQRTHALF, 0.0, 0.0, SQRTHALF);
        let qppmm = Quat::new(0.5, 0.5, -0.5, -0.5);
        let qpppp = Quat::new(0.5, 0.5, 0.5, 0.5);
        let qmpmm = Quat::new(-0.5, 0.5, -0.5, -0.5);
        let qpppm = Quat::new(0.5, 0.5, 0.5, -0.5);
        let q0001 = Quat::new(0.0, 0.0, 0.0, 1.0);
        let q1000 = Quat::new(1.0, 0.0, 0.0, 0.0);
        let mut q = q;
        let qtoz = match turn {
            0 => {
                q = q.mul(qxtoz);
                swap(&mut ka, 0, 2);
                qxtoz
            }
            1 => {
                q = q.mul(qytoz);
                swap(&mut ka, 1, 2);
                qytoz
            }
            2 => q0001,
            _ => return q.conj(),
        };
        let q = q.conj();
        let mut mag = [
            q.z * q.z + q.w * q.w - 0.5,
            q.x * q.z - q.y * q.w,
            q.y * q.z + q.x * q.w,
        ];
        let mut neg = [false; 3];
        for i in 0..3 {
            neg[i] = mag[i] < 0.0;
            if neg[i] {
                mag[i] = -mag[i];
            }
        }
        let win = if mag[0] > mag[1] {
            if mag[0] > mag[2] { 0 } else { 2 }
        } else if mag[1] > mag[2] {
            1
        } else {
            2
        };
        let mut pp = match win {
            0 => {
                if neg[0] {
                    q1000
                } else {
                    q0001
                }
            }
            1 => {
                cycle(&mut ka, false);
                if neg[1] { qppmm } else { qpppp }
            }
            _ => {
                cycle(&mut ka, true);
                if neg[2] { qmpmm } else { qpppm }
            }
        };
        let qp = q.mul(pp);
        let t = (mag[win] + 0.5).sqrt();
        let (z, w) = if t > 0.0 {
            (-qp.z / t, qp.w / t)
        } else {
            (0.0, 1.0)
        };
        pp = pp.mul(Quat::new(0.0, 0.0, z, w));
        p = qtoz.mul(pp.conj());
    } else {
        let mut qa = qarr(q);
        let mut pa = [0.0f64; 4];
        let mut neg = [false; 4];
        let mut par = false;
        for i in 0..4 {
            neg[i] = qa[i] < 0.0;
            if neg[i] {
                qa[i] = -qa[i];
            }
            par ^= neg[i];
        }
        // die beiden größten Komponenten: Indizes hi und lo
        let mut lo: usize = if qa[0] > qa[1] { 0 } else { 1 };
        let mut hi: usize = if qa[2] > qa[3] { 2 } else { 3 };
        if qa[lo] > qa[hi] {
            if qa[lo ^ 1] > qa[hi] {
                hi = lo;
                lo ^= 1;
            } else {
                std::mem::swap(&mut hi, &mut lo);
            }
        } else if qa[hi ^ 1] > qa[lo] {
            lo = hi ^ 1;
        }
        let all = (qa[0] + qa[1] + qa[2] + qa[3]) * 0.5;
        let two = (qa[hi] + qa[lo]) * SQRTHALF;
        let big = qa[hi];
        let sgn = |n: bool, v: f64| if n { -v } else { v };
        if all > two {
            if all > big {
                for i in 0..4 {
                    pa[i] = sgn(neg[i], 0.5);
                }
                cycle(&mut ka, par);
            } else {
                pa[hi] = sgn(neg[hi], 1.0);
            }
        } else if two > big {
            pa[hi] = sgn(neg[hi], SQRTHALF);
            pa[lo] = sgn(neg[lo], SQRTHALF);
            if lo > hi {
                std::mem::swap(&mut hi, &mut lo);
            }
            if hi == 3 {
                hi = [1usize, 2, 0][lo];
                lo = 3 - hi - lo;
            }
            swap(&mut ka, hi, lo);
        } else {
            pa[hi] = sgn(neg[hi], 1.0);
        }
        p = Quat::new(-pa[0], -pa[1], -pa[2], pa[3]);
    }
    *k = ka;
    p
}

impl AffineParts {
    /// Zerlegt den affinen Anteil einer [`Matrix3`] (Zeilenvektor-Form; die perspektivische
    /// Spalte wird ignoriert).
    pub fn decompose(m: &Matrix3) -> AffineParts {
        let mm = &m.m;
        // Spaltenvektor-Form des linearen Anteils, eingebettet in 3×3 mit z = 1
        let a: M3 = [
            [mm[0][0] as f64, mm[1][0] as f64, 0.0],
            [mm[0][1] as f64, mm[1][1] as f64, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let t = [mm[2][0] as f64, mm[2][1] as f64];
        let (mut q, s, det) = polar_decomp(&a);
        let f = if det < 0.0 {
            for row in q.iter_mut() {
                for v in row.iter_mut() {
                    *v = -*v;
                }
            }
            -1.0
        } else {
            1.0
        };
        let qq = Quat::from_matrix(&q);
        let (u, mut k) = spect_decomp(&s);
        let uq = Quat::from_matrix(&u);
        let p = snuggle(uq, &mut k);
        AffineParts {
            t,
            q: qq,
            u: uq.mul(p),
            k,
            f,
        }
    }

    /// Setzt die Teile wieder zu einer affinen [`Matrix3`] zusammen:
    /// Reihenfolge der Anwendung `u⁻¹`, `k`, `u`, `q`, `f`, dann Verschiebung `t` (13 §7.2).
    pub fn compose(&self) -> Matrix3 {
        let ru = self.u.to_matrix();
        let rut = transpose(&ru);
        let kd: M3 = [
            [self.k[0], 0.0, 0.0],
            [0.0, self.k[1], 0.0],
            [0.0, 0.0, self.k[2]],
        ];
        let rq = self.q.to_matrix();
        // Spaltenform: A = f · Rq · Ru · K · Ruᵀ
        let mut a = mat_mul(&rq, &mat_mul(&ru, &mat_mul(&kd, &rut)));
        for row in a.iter_mut() {
            for v in row.iter_mut() {
                *v *= self.f;
            }
        }
        Matrix3::from_affine(
            a[0][0] as f32,
            a[1][0] as f32,
            a[0][1] as f32,
            a[1][1] as f32,
            self.t[0] as f32,
            self.t[1] as f32,
        )
    }

    /// Interpoliert zwischen `self` (t = 0) und `o` (t = 1) wie Tweening des Originals:
    /// `t` linear, `q` und `u` per Slerp, `k` linear, `f` springt bei `t > 0,5` auf `o.f`.
    pub fn interpolate(&self, o: &AffineParts, t: f64) -> AffineParts {
        let lerp = |a: f64, b: f64| a + (b - a) * t;
        AffineParts {
            t: [lerp(self.t[0], o.t[0]), lerp(self.t[1], o.t[1])],
            q: self.q.slerp(o.q, t),
            u: self.u.slerp(o.u, t),
            k: [
                lerp(self.k[0], o.k[0]),
                lerp(self.k[1], o.k[1]),
                lerp(self.k[2], o.k[2]),
            ],
            f: if t > 0.5 { o.f } else { self.f },
        }
    }
}

/// Interpoliert zwei affine Matrizen über ihre Polarzerlegung (Tweening, 13 §7.2).
pub fn interpolate_matrices(a: &Matrix3, b: &Matrix3, t: f64) -> Matrix3 {
    AffineParts::decompose(a)
        .interpolate(&AffineParts::decompose(b), t)
        .compose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Matrix3, b: &Matrix3, eps: f32) -> bool {
        a.m.iter()
            .flatten()
            .zip(b.m.iter().flatten())
            .all(|(x, y)| (x - y).abs() <= eps)
    }

    #[test]
    fn decompose_roundtrip() {
        let cases = [
            Matrix3::IDENTITY,
            Matrix3::translation(5.0, -3.0),
            Matrix3::rotation_degrees(30.0),
            Matrix3::scaling(2.0, 0.5)
                * Matrix3::rotation_degrees(-70.0)
                * Matrix3::translation(1.0, 2.0),
            Matrix3::shear(0.4, 0.0) * Matrix3::scaling(1.5, 3.0),
            Matrix3::scaling(-1.0, 1.0) * Matrix3::rotation_degrees(45.0),
            Matrix3::scaling(-2.0, -3.0),
            Matrix3::scaling(2.0, 2.0) * Matrix3::rotation_degrees(170.0),
            Matrix3::from_affine(0.3, 0.9, -1.7, 0.2, 10.0, 20.0),
            Matrix3::scaling(0.0, 2.0) * Matrix3::rotation_degrees(20.0),
        ];
        for m in cases {
            let p = AffineParts::decompose(&m);
            let back = p.compose();
            assert!(close(&m, &back, 1e-5), "{m:?} -> {p:?} -> {back:?}");
            assert!((p.q.dot(p.q) - 1.0).abs() < 1e-9);
            assert!((p.u.dot(p.u) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn interpolation_endpoints_and_rotation() {
        let a = Matrix3::rotation_degrees(10.0) * Matrix3::translation(0.0, 0.0);
        let b = Matrix3::scaling(2.0, 2.0)
            * Matrix3::rotation_degrees(80.0)
            * Matrix3::translation(100.0, 50.0);
        assert!(close(&interpolate_matrices(&a, &b, 0.0), &a, 1e-5));
        assert!(close(&interpolate_matrices(&a, &b, 1.0), &b, 1e-4));
        // reine Drehungen: Mitte = 45°
        let r0 = Matrix3::rotation_degrees(0.0);
        let r1 = Matrix3::rotation_degrees(90.0);
        let mid = interpolate_matrices(&r0, &r1, 0.5);
        assert!(
            close(&mid, &Matrix3::rotation_degrees(45.0), 1e-5),
            "{mid:?}"
        );
        // Spiegelung springt bei t > 0,5
        let m = Matrix3::scaling(-1.0, 1.0);
        let pa = AffineParts::decompose(&Matrix3::IDENTITY);
        let pb = AffineParts::decompose(&m);
        assert_eq!(pa.interpolate(&pb, 0.5).f, 1.0);
        assert_eq!(pa.interpolate(&pb, 0.51).f, -1.0);
    }
}
