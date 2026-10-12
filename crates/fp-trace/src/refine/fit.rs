//! Kurvenanpassung einer Randkette: Geraden erkennen (und einrasten), Ecken finden, die übrigen
//! Stücke mit möglichst wenigen kubischen Bézierkurven annähern (glatte Knoten mit kollinearen
//! Henkeln).
//!
//! Eingabe sind die Mittelpunkte der Pixelkanten einer Randkette (Treppenstufen liegen höchstens
//! ~0,35 px neben der wahren Kante). Ablauf:
//! 1. Douglas-Peucker mit der Fit-Toleranz liefert Kandidaten für Geraden; eine Kante gilt als
//!    Gerade, wenn ihre mittleren 70 % höchstens die Toleranz abweichen und eine angepasste Parabel
//!    keinen Bogen zeigt (Pfeilhöhe ≤ 0,35 × Toleranz). Danach wächst die Gerade in beide
//!    Richtungen, solange die Punkte auf ihr liegen.
//! 2. Kurze, schwache Geraden (nicht eingerastet, mit glatten Übergängen) werden wieder Kurven.
//! 3. Einrasten: Richtungen nahe 0°/90° (Toleranz `snap_deg`) bzw. 45° (¾ davon) werden exakt;
//!    sonst rechtwinklig/parallel zur längsten Geraden der Kette.
//! 4. Zwischen zwei Geraden liegt die Ecke im Schnittpunkt (exakte rechte Winkel); in Kurven
//!    teilen Ecken (Winkel über `corner_deg`, scharf statt Bogen) die Stücke.
//! 5. Kurven: gierige längste Anpassung (Schneider: Sehnenparametrisierung, kleinste Quadrate,
//!    Newton-Nachparametrisierung) mit festen Tangenten an Geraden und glatten Knoten.

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct V {
    pub x: f64,
    pub y: f64,
}

impl V {
    pub fn new(x: f64, y: f64) -> V {
        V { x, y }
    }
    pub fn dot(self, o: V) -> f64 {
        self.x * o.x + self.y * o.y
    }
    pub fn cross(self, o: V) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub fn norm(self) -> V {
        let l = self.len();
        if l > 1e-12 { V::new(self.x / l, self.y / l) } else { V::new(1.0, 0.0) }
    }
    pub fn dist(self, o: V) -> f64 {
        (self - o).len()
    }
    pub fn mid(self, o: V) -> V {
        V::new(0.5 * (self.x + o.x), 0.5 * (self.y + o.y))
    }
}

impl Add for V {
    type Output = V;
    fn add(self, o: V) -> V {
        V::new(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V {
    type Output = V;
    fn sub(self, o: V) -> V {
        V::new(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f64> for V {
    type Output = V;
    fn mul(self, k: f64) -> V {
        V::new(self.x * k, self.y * k)
    }
}
impl Neg for V {
    type Output = V;
    fn neg(self) -> V {
        V::new(-self.x, -self.y)
    }
}

/// Kubisches Segment (Geraden: Henkel auf den Endpunkten).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Seg {
    pub p0: V,
    pub c1: V,
    pub c2: V,
    pub p3: V,
}

impl Seg {
    pub fn line(a: V, b: V) -> Seg {
        Seg { p0: a, c1: a, c2: b, p3: b }
    }
    pub fn reversed(&self) -> Seg {
        Seg { p0: self.p3, c1: self.c2, c2: self.c1, p3: self.p0 }
    }
    pub fn map(&self, f: impl Fn(V) -> V) -> Seg {
        Seg { p0: f(self.p0), c1: f(self.c1), c2: f(self.c2), p3: f(self.p3) }
    }
    fn eval(&self, t: f64) -> V {
        let mt = 1.0 - t;
        self.p0 * (mt * mt * mt) + self.c1 * (3.0 * mt * mt * t) + self.c2 * (3.0 * mt * t * t) + self.p3 * (t * t * t)
    }
    fn d1(&self, t: f64) -> V {
        let mt = 1.0 - t;
        (self.c1 - self.p0) * (3.0 * mt * mt) + (self.c2 - self.c1) * (6.0 * mt * t) + (self.p3 - self.c2) * (3.0 * t * t)
    }
    fn d2(&self, t: f64) -> V {
        (self.c2 - self.c1 * 2.0 + self.p0) * (6.0 * (1.0 - t)) + (self.p3 - self.c2 * 2.0 + self.c1) * (6.0 * t)
    }
}

/// Segmentliste umgekehrt durchlaufen.
pub(crate) fn reverse(segs: &[Seg]) -> Vec<Seg> {
    segs.iter().rev().map(Seg::reversed).collect()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct FitParams {
    /// Höchste Abweichung von den Kantenpunkten (px).
    pub tol: f64,
    /// Ecken in Kurven ab diesem Richtungswechsel (Grad).
    pub corner_deg: f64,
    /// Geraden einrasten (Grad, 0 = aus).
    pub snap_deg: f64,
    /// Gierige längste Anpassung (sonst Teilen am größten Fehler).
    pub reduce: bool,
    /// Newton-Durchläufe je Versuch.
    pub iters: usize,
    /// Nur Ecken, keine Kurven (Modus Polygon).
    pub polygon: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FitStats {
    pub lines: usize,
    pub snapped: usize,
}

#[derive(Clone, Copy, Debug)]
struct Line {
    a: usize,
    b: usize,
    /// Punkt auf der Geraden.
    c: V,
    /// Einheitsrichtung von `a` nach `b`.
    d: V,
    snapped: bool,
}

#[derive(Clone, Copy, Debug)]
enum Item {
    Line(Line),
    /// Kurve von Stichprobe a bis b; `corner_*`: Ecke am Anfang/Ende (freie Tangente).
    Curve { a: usize, b: usize, corner_a: bool, corner_b: bool },
}

impl Item {
    fn range(&self) -> (usize, usize) {
        match *self {
            Item::Line(l) => (l.a, l.b),
            Item::Curve { a, b, .. } => (a, b),
        }
    }
}

fn angle_between(u: V, v: V) -> f64 {
    let (u, v) = (u.norm(), v.norm());
    u.cross(v).atan2(u.dot(v)).abs().to_degrees()
}

/// Kanten der Kette annähern. `closed`: Ring ohne festen Anfang; sonst sind erster und letzter
/// Punkt fest (Knotenpunkte zwischen mehreren Flächen). `hints`: bevorzugte (ungerichtete)
/// Tangente an Anfang/Ende einer offenen Kette, übernommen, wenn die Kurve dort höchstens 35°
/// davon abweicht (glatter Übergang an einer Spiegelachse).
pub(crate) fn fit_chain(input: &[V], closed: bool, p: &FitParams, st: &mut FitStats, hints: [Option<V>; 2]) -> Vec<Seg> {
    let n = input.len();
    if n < 2 {
        return Vec::new();
    }
    if !closed && n == 2 {
        return vec![Seg::line(input[0], input[1])];
    }
    if closed && n < 5 {
        return (0..n).map(|i| Seg::line(input[i], input[(i + 1) % n])).collect();
    }
    let a: Vec<V> = if closed {
        // Anfang an der schärfsten Stelle (meist eine Ecke).
        let t = smooth(input, true);
        let start = (0..n).max_by(|&i, &j| score(&t, i, 4, Some(n)).total_cmp(&score(&t, j, 4, Some(n)))).unwrap_or(0);
        let mut v: Vec<V> = (0..n).map(|i| input[(start + i) % n]).collect();
        v.push(v[0]);
        v
    } else {
        input.to_vec()
    };
    let mut c = Chain::new(a, closed, *p);
    c.hints = hints;
    c.fit(st)
}

struct Chain {
    /// Stichproben; bei Ringen ist a[m] == a[0].
    a: Vec<V>,
    /// Geglättete Stichproben (für Winkel und Tangenten).
    t: Vec<V>,
    m: usize,
    wrap: bool,
    p: FitParams,
    hints: [Option<V>; 2],
}

impl Chain {
    fn new(a: Vec<V>, wrap: bool, p: FitParams) -> Chain {
        let m = a.len() - 1;
        let t = if wrap {
            let mut t = smooth(&a[..m], true);
            t.push(t[0]);
            t
        } else {
            smooth(&a, false)
        };
        Chain { a, t, m, wrap, p, hints: [None, None] }
    }

    fn period(&self) -> Option<usize> {
        self.wrap.then_some(self.m)
    }

    /// Index modulo Periode (Ring) bzw. begrenzt (offen).
    fn at(&self, i: isize) -> usize {
        if self.wrap {
            i.rem_euclid(self.m as isize) as usize
        } else {
            i.clamp(0, self.m as isize) as usize
        }
    }

    fn fit(&self, st: &mut FitStats) -> Vec<Seg> {
        let p = &self.p;
        let m = self.m;
        let mut verts = vec![0, m];
        dp(&self.a, 0, m, p.tol, &mut verts);
        verts.sort_unstable();
        verts.dedup();
        let mut lines = self.find_lines(&verts);
        self.drop_weak(&mut lines);
        if p.snap_deg > 0.0 {
            self.snap(&mut lines);
        }
        st.lines += lines.len();
        st.snapped += lines.iter().filter(|l| l.snapped).count();
        let items = self.items(&lines);
        self.emit(&items)
    }

    // ---------------------------------------------------------------- Geraden

    fn find_lines(&self, verts: &[usize]) -> Vec<Line> {
        let p = &self.p;
        let a = &self.a;
        let m = self.m;
        let l_min = (4.0 * p.tol).max(6.0);
        let sag_max = 0.35 * p.tol;
        let mut lines: Vec<Line> = Vec::new();
        let mut last_end = 0usize;
        for w in verts.windows(2) {
            let (u0, v0) = (w[0].max(last_end), w[1]);
            if v0 < u0 + 5 || a[u0].dist(a[v0]) < l_min {
                continue;
            }
            let cut = (v0 - u0) * 15 / 100;
            let (cu, cv) = (u0 + cut, v0 - cut);
            if cv < cu + 3 {
                continue;
            }
            let core = &a[cu..=cv];
            let (c, d) = tls(core);
            if max_dev(core, c, d) > p.tol || sagitta(core, c, d) > sag_max {
                continue;
            }
            // Kantenmitten einer Geraden liegen ≤ 0,35 px daneben, die erste Mitte der Nachbarkante
            // 0,5 px; bei Rauschen (JPEG) entsprechend der Streuung des Kerns mehr.
            let rms = (core.iter().map(|&q| (q - c).cross(d).powi(2)).sum::<f64>() / core.len() as f64).sqrt();
            let grow = (1.6 * rms + 0.15).clamp(0.45, (0.8 * p.tol).max(0.45));
            let on = |i: usize| (a[i] - c).cross(d).abs() <= grow;
            let mut lo = cu;
            while lo > last_end && on(lo - 1) {
                lo -= 1;
            }
            let mut hi = cv;
            while hi < m && on(hi + 1) {
                hi += 1;
            }
            let (mut lo, mut hi) = (lo, hi);
            let (mut c, mut d) = tls(&a[lo..=hi]);
            if max_dev(&a[lo..=hi], c, d) > p.tol {
                (lo, hi) = (cu, cv);
                (c, d) = tls(&a[lo..=hi]);
            }
            if a[lo].dist(a[hi]) < l_min {
                continue;
            }
            let d = if d.dot(a[hi] - a[lo]) < 0.0 { -d } else { d };
            let line = Line { a: lo, b: hi, c, d, snapped: false };
            if let Some(prev) = lines.last_mut()
                && lo <= prev.b + 2
                && angle_between(prev.d, d) < 3.0
            {
                let (c2, d2) = tls(&a[prev.a..=hi]);
                if max_dev(&a[prev.a..=hi], c2, d2) <= 1.25 * p.tol {
                    let d2 = if d2.dot(a[hi] - a[prev.a]) < 0.0 { -d2 } else { d2 };
                    *prev = Line { a: prev.a, b: hi, c: c2, d: d2, snapped: false };
                    last_end = hi;
                    continue;
                }
            }
            lines.push(line);
            last_end = hi;
        }
        // Zweite Runde: fast gleich gerichtete Geraden mit kleinem Versatz (JPEG-Blöcke) verbinden,
        // wenn die gemeinsame Gerade im Mittel gut passt.
        let mut k = 1;
        while k < lines.len() {
            let (pl, cl) = (lines[k - 1], lines[k]);
            if cl.a <= pl.b + 12 && angle_between(pl.d, cl.d) < 3.0 {
                let pts = &a[pl.a..=cl.b];
                let (c2, d2) = tls(pts);
                let rms = (pts.iter().map(|&q| (q - c2).cross(d2).powi(2)).sum::<f64>() / pts.len() as f64).sqrt();
                if max_dev(pts, c2, d2) <= 2.0 * p.tol && rms <= 0.5 * p.tol {
                    let d2 = if d2.dot(a[cl.b] - a[pl.a]) < 0.0 { -d2 } else { d2 };
                    lines[k - 1] = Line { a: pl.a, b: cl.b, c: c2, d: d2, snapped: false };
                    lines.remove(k);
                    continue;
                }
            }
            k += 1;
        }
        lines
    }

    /// Richtung neben einer Geraden: Nachbargerade oder Kurvenrichtung (~4 Stichproben).
    fn side_dir(&self, lines: &[Line], k: usize, before: bool) -> Option<V> {
        let l = lines[k];
        if before {
            if k > 0 && lines[k - 1].b + 3 >= l.a {
                return Some(lines[k - 1].d);
            }
            if !self.wrap && l.a == 0 {
                return None;
            }
            if self.wrap && k == 0 && lines.len() > 1 {
                let last = lines[lines.len() - 1];
                if last.b + 3 >= self.m + l.a {
                    return Some(last.d);
                }
            }
            let j = self.at(l.a as isize - 4);
            Some(self.a[l.a] - self.a[j])
        } else {
            if k + 1 < lines.len() && l.b + 3 >= lines[k + 1].a {
                return Some(lines[k + 1].d);
            }
            if !self.wrap && l.b == self.m {
                return None;
            }
            if self.wrap && k + 1 == lines.len() && lines.len() > 1 && l.b + 3 >= self.m + lines[0].a {
                return Some(lines[0].d);
            }
            let j = self.at(l.b as isize + 4);
            Some(self.a[j] - self.a[l.b])
        }
    }

    /// Kurze, nicht eingerastete Geraden mit glatten Übergängen sind Teil eines Bogens.
    fn drop_weak(&self, lines: &mut Vec<Line>) {
        let p = &self.p;
        let l_min = (4.0 * p.tol).max(6.0);
        let l_strong = (12.0 * p.tol).max(15.0);
        let mut keep = vec![true; lines.len()];
        for k in 0..lines.len() {
            let l = lines[k];
            let len = self.a[l.a].dist(self.a[l.b]);
            let axis = p.snap_deg > 0.0 && snap_dir(l.d, p.snap_deg).is_some();
            if len >= l_strong || (axis && len >= 2.0 * l_min) {
                continue;
            }
            let sharp = |d: Option<V>| d.is_none_or(|d| angle_between(d, l.d) >= 40.0);
            if !(sharp(self.side_dir(lines, k, true)) && sharp(self.side_dir(lines, k, false))) {
                keep[k] = false;
            }
        }
        let mut i = 0;
        lines.retain(|_| {
            i += 1;
            keep[i - 1]
        });
    }

    fn snap(&self, lines: &mut [Line]) {
        let p = &self.p;
        let fit_with = |l: &Line, d: V| -> Option<Line> {
            let pts = &self.a[l.a..=l.b];
            // Lage: Median der Abstände (robust gegen Ausreißer an den Enden)
            let c0 = centroid(pts);
            let mut offs: Vec<f64> = pts.iter().map(|&q| (q - c0).cross(d)).collect();
            offs.sort_by(f64::total_cmp);
            let med = offs[offs.len() / 2];
            let c = c0 + V::new(d.y, -d.x) * med;
            let before = max_dev(pts, l.c, l.d);
            (max_dev(pts, c, d) <= (1.25 * p.tol).max(before + 0.3 * p.tol)).then_some(Line { c, d, snapped: true, ..*l })
        };
        for l in lines.iter_mut() {
            if let Some(d) = snap_dir(l.d, p.snap_deg)
                && let Some(n) = fit_with(l, d)
            {
                *l = n;
            }
        }
        // Rechtwinklig/parallel zur längsten (nicht eingerasteten) Geraden.
        let Some(main) = lines.iter().filter(|l| !l.snapped).max_by(|x, y| self.a[x.a].dist(self.a[x.b]).total_cmp(&self.a[y.a].dist(self.a[y.b]))).copied() else {
            return;
        };
        let base = main.d.y.atan2(main.d.x);
        for l in lines.iter_mut() {
            if l.snapped || (l.a == main.a && l.b == main.b) {
                continue;
            }
            let ang = l.d.y.atan2(l.d.x) - base;
            let q = (ang / std::f64::consts::FRAC_PI_2).round();
            if (ang - q * std::f64::consts::FRAC_PI_2).abs().to_degrees() <= p.snap_deg {
                let t = base + q * std::f64::consts::FRAC_PI_2;
                if let Some(n) = fit_with(l, V::new(t.cos(), t.sin())) {
                    *l = Line { snapped: false, ..n };
                }
            }
        }
    }

    // ---------------------------------------------------------------- Abschnitte

    fn items(&self, lines: &[Line]) -> Vec<Item> {
        let m = self.m;
        let mut items: Vec<Item> = Vec::new();
        let mut pos = 0usize;
        for l in lines {
            if l.a > pos {
                items.push(Item::Curve { a: pos, b: l.a, corner_a: false, corner_b: false });
            }
            items.push(Item::Line(*l));
            pos = l.b;
        }
        if pos < m || items.is_empty() {
            items.push(Item::Curve { a: pos, b: m, corner_a: false, corner_b: false });
        }
        // Winzige Kurven zwischen zwei Geraden fallen weg (Ecke im Schnittpunkt).
        let nitems = items.len();
        let mut drop = vec![false; nitems];
        for k in 0..nitems {
            if let Item::Curve { a, b, .. } = items[k] {
                let prev = if k > 0 { Some(k - 1) } else if self.wrap { Some(nitems - 1) } else { None };
                let next = if k + 1 < nitems { Some(k + 1) } else if self.wrap { Some(0) } else { None };
                if let (Some(pi), Some(ni)) = (prev, next)
                    && pi != ni
                    && let (Item::Line(l1), Item::Line(l2)) = (items[pi], items[ni])
                    && (b - a <= 3 || self.a[a].dist(self.a[b]) <= 3.0 * self.p.tol + 2.0)
                    && let Some(x) = intersect(&l1, &l2)
                    && x.dist(self.a[(a + b) / 2]) <= 2.5 * self.p.tol + 2.0
                {
                    drop[k] = true;
                }
            }
        }
        let out: Vec<Item> = items.into_iter().enumerate().filter(|(k, _)| !drop[*k]).map(|(_, it)| it).collect();
        // Ecken in Kurven teilen.
        let mut res: Vec<Item> = Vec::new();
        for it in out {
            match it {
                Item::Curve { a, b, corner_a, corner_b } if b >= a + 6 => {
                    let mut cut = a;
                    let corners = self.corners(a, b);
                    for c in corners {
                        res.push(Item::Curve { a: cut, b: c, corner_a: if cut == a { corner_a } else { true }, corner_b: true });
                        cut = c;
                    }
                    res.push(Item::Curve { a: cut, b, corner_a: if cut == a { corner_a } else { true }, corner_b });
                }
                other => res.push(other),
            }
        }
        // Ringanfang: Ecke, wenn dort scharf.
        if self.wrap {
            let c0 = self.is_corner(0);
            if let Some(Item::Curve { corner_a, .. }) = res.first_mut() {
                *corner_a = c0;
            }
            if let Some(Item::Curve { corner_b, .. }) = res.last_mut() {
                *corner_b = c0;
            }
        }
        res
    }

    fn is_corner(&self, i: usize) -> bool {
        let s4 = score(&self.t, i, 4, self.period());
        s4 >= self.p.corner_deg && s4 >= 0.7 * score(&self.t, i, 8, self.period())
    }

    /// Ecken in (a, b): lokale Maxima des Fensterwinkels über der Schwelle, scharf (nicht Bogen).
    fn corners(&self, a: usize, b: usize) -> Vec<usize> {
        let mut out: Vec<usize> = Vec::new();
        let sc: Vec<f64> = (a..=b).map(|i| score(&self.t, i, 4, self.period())).collect();
        for i in a + 2..=b - 2 {
            let s = sc[i - a];
            if s < self.p.corner_deg {
                continue;
            }
            let lo = i.saturating_sub(3).max(a);
            let hi = (i + 3).min(b);
            if (lo..=hi).any(|j| sc[j - a] > s || (sc[j - a] == s && j < i)) {
                continue;
            }
            if s < 0.7 * score(&self.t, i, 8, self.period()) {
                continue;
            }
            if out.last().is_none_or(|&l| i >= l + 2) {
                out.push(i);
            }
        }
        out
    }

    // ---------------------------------------------------------------- Zusammenbau

    /// Zentrale Tangente (geglättet) an Stichprobe i.
    fn central(&self, i: usize) -> V {
        let w = 3isize;
        let (lo, hi) = (self.at(i as isize - w), self.at(i as isize + w));
        let d = self.t[hi] - self.t[lo];
        if d.len() < 1e-9 { self.a[self.at(i as isize + 1)] - self.a[self.at(i as isize - 1)] } else { d }.norm()
    }

    /// Einseitige Richtung von Punkt `from` in die Kurve hinein (Mittel der nächsten Stichproben).
    fn one_sided(&self, from: V, idx: impl Iterator<Item = usize>) -> V {
        let mut s = V::default();
        for i in idx {
            s = s + (self.a[i] - from).norm();
        }
        s.norm()
    }

    fn emit(&self, items: &[Item]) -> Vec<Seg> {
        let p = &self.p;
        let a = &self.a;
        let k = items.len();
        // Knotenlagen an den Grenzen 0..=k
        let mut vpos: Vec<V> = Vec::with_capacity(k + 1);
        for i in 0..=k {
            let (prev, next) = if i == 0 || i == k {
                if self.wrap { (Some(items[k - 1]), Some(items[0])) } else { (None, None) }
            } else {
                (Some(items[i - 1]), Some(items[i]))
            };
            let s = if i == k { items[k - 1].range().1 } else { items[i].range().0 };
            let s = if self.wrap && i == k { 0 } else { s };
            let here = a[s];
            let v = match (prev, next) {
                (Some(Item::Line(l1)), Some(Item::Line(l2))) => match intersect(&l1, &l2) {
                    Some(x) if x.dist(here) <= 3.0 * p.tol + 2.0 => x,
                    _ => project(&l1, here),
                },
                (Some(Item::Line(l)), _) | (_, Some(Item::Line(l))) => project(&l, here),
                _ => here,
            };
            vpos.push(v);
        }
        if self.wrap {
            vpos[k] = vpos[0];
        }
        let mut out: Vec<Seg> = Vec::new();
        for (i, it) in items.iter().enumerate() {
            let (p0, p3) = (vpos[i], vpos[i + 1]);
            match *it {
                Item::Line(_) => {
                    if p0.dist(p3) > 1e-9 {
                        out.push(Seg::line(p0, p3));
                    }
                }
                Item::Curve { a: ca, b: cb, corner_a, corner_b } => {
                    let mut pts: Vec<V> = Vec::with_capacity(cb - ca + 1);
                    pts.push(p0);
                    pts.extend_from_slice(&a[ca + 1..cb]);
                    pts.push(p3);
                    if p.polygon {
                        let mut vs = vec![0, pts.len() - 1];
                        dp(&pts, 0, pts.len() - 1, p.tol, &mut vs);
                        vs.sort_unstable();
                        vs.dedup();
                        for w in vs.windows(2) {
                            if pts[w[0]].dist(pts[w[1]]) > 1e-9 {
                                out.push(Seg::line(pts[w[0]], pts[w[1]]));
                            }
                        }
                        continue;
                    }
                    let prev = if i > 0 { Some(items[i - 1]) } else if self.wrap { Some(items[k - 1]) } else { None };
                    let next = if i + 1 < k { Some(items[i + 1]) } else if self.wrap { Some(items[0]) } else { None };
                    let n_in = (cb - ca).min(4);
                    let free0 = self.one_sided(p0, (ca + 1..=ca + n_in).map(|j| j.min(self.m)));
                    let free1 = self.one_sided(p3, (cb.saturating_sub(n_in)..cb).rev());
                    let hinted = |h: Option<V>, free: V| match h {
                        Some(h) if angle_between(h, free) < 35.0 => h,
                        Some(h) if angle_between(-h, free) < 35.0 => -h,
                        _ => free,
                    };
                    let t0 = match prev {
                        Some(Item::Line(l)) if angle_between(l.d, free0) < 35.0 => l.d,
                        Some(Item::Curve { .. }) if !corner_a => self.central(ca),
                        None => hinted(self.hints[0], free0),
                        _ => free0,
                    };
                    let t1 = match next {
                        Some(Item::Line(l)) if angle_between(-l.d, free1) < 35.0 => -l.d,
                        Some(Item::Curve { .. }) if !corner_b => -self.central(if self.wrap && cb == self.m { 0 } else { cb }),
                        None => hinted(self.hints[1], free1),
                        _ => free1,
                    };
                    if p.reduce {
                        fit_greedy(&pts, t0, t1, p, &mut out);
                    } else {
                        fit_split(&pts, t0, t1, p, &mut out, 0);
                    }
                }
            }
        }
        out
    }
}

// -------------------------------------------------------------------- Hilfen

/// Zwei Durchläufe [1 2 1]/4.
fn smooth(s: &[V], wrap: bool) -> Vec<V> {
    let n = s.len();
    let mut cur = s.to_vec();
    if n < 3 {
        return cur;
    }
    for _ in 0..2 {
        let prev = cur.clone();
        for i in 0..n {
            let (l, r) = if wrap { ((i + n - 1) % n, (i + 1) % n) } else { (i.saturating_sub(1), (i + 1).min(n - 1)) };
            cur[i] = (prev[l] + prev[i] * 2.0 + prev[r]) * 0.25;
        }
    }
    cur
}

/// Richtungswechsel (Grad) an i mit Fenster k. Offene Ketten: 0 ohne volles Fenster.
fn score(t: &[V], i: usize, k: usize, period: Option<usize>) -> f64 {
    let n = period.unwrap_or(t.len());
    if n < 2 * k + 1 {
        return 0.0;
    }
    let (lo, hi) = if period.is_some() {
        ((i + n - k % n) % n, (i + k) % n)
    } else {
        if i < k || i + k >= n {
            return 0.0;
        }
        (i - k, i + k)
    };
    let (u, v) = (t[i] - t[lo], t[hi] - t[i]);
    if u.len() < 1e-9 || v.len() < 1e-9 {
        return 0.0;
    }
    angle_between(u, v)
}

/// Douglas-Peucker: innere Eckpunkte von a[lo..=hi] nach `out`.
fn dp(a: &[V], lo: usize, hi: usize, eps: f64, out: &mut Vec<usize>) {
    let mut stack = vec![(lo, hi)];
    while let Some((l, h)) = stack.pop() {
        if h <= l + 1 {
            continue;
        }
        let (p, q) = (a[l], a[h]);
        let d = q - p;
        let len = d.len();
        let mut best = (0.0, l);
        for (i, &x) in a.iter().enumerate().take(h).skip(l + 1) {
            let dist = if len < 1e-9 { x.dist(p) } else { (x - p).cross(d).abs() / len };
            if dist > best.0 {
                best = (dist, i);
            }
        }
        if best.0 > eps {
            out.push(best.1);
            stack.push((l, best.1));
            stack.push((best.1, h));
        }
    }
}

fn centroid(pts: &[V]) -> V {
    let n = pts.len().max(1) as f64;
    let s = pts.iter().fold(V::default(), |s, &p| s + p);
    s * (1.0 / n)
}

/// Ausgleichsgerade (Hauptachse), Richtung von pts[0] nach pts[last].
fn tls(pts: &[V]) -> (V, V) {
    let c = centroid(pts);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for &p in pts {
        let q = p - c;
        sxx += q.x * q.x;
        syy += q.y * q.y;
        sxy += q.x * q.y;
    }
    let ang = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let mut d = V::new(ang.cos(), ang.sin());
    if let (Some(&f), Some(&l)) = (pts.first(), pts.last())
        && d.dot(l - f) < 0.0
    {
        d = -d;
    }
    (c, d)
}

fn max_dev(pts: &[V], c: V, d: V) -> f64 {
    pts.iter().map(|&p| (p - c).cross(d).abs()).fold(0.0, f64::max)
}

/// Pfeilhöhe einer Ausgleichsparabel v = α u'² + β u' + γ mit u' ∈ [-1, 1]: |α|.
fn sagitta(pts: &[V], c: V, d: V) -> f64 {
    if pts.len() < 5 {
        return 0.0;
    }
    let us: Vec<f64> = pts.iter().map(|&p| (p - c).dot(d)).collect();
    let (umin, umax) = us.iter().fold((f64::MAX, f64::MIN), |(a, b), &u| (a.min(u), b.max(u)));
    let half = 0.5 * (umax - umin);
    if half < 1e-9 {
        return 0.0;
    }
    let mid = 0.5 * (umax + umin);
    let (mut s0, mut s1, mut s2, mut s3, mut s4, mut t0, mut t1, mut t2) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (k, &p) in pts.iter().enumerate() {
        let u = (us[k] - mid) / half;
        let v = (p - c).cross(d);
        let u2 = u * u;
        s0 += 1.0;
        s1 += u;
        s2 += u2;
        s3 += u2 * u;
        s4 += u2 * u2;
        t0 += v;
        t1 += u * v;
        t2 += u2 * v;
    }
    let det = |m: [[f64; 3]; 3]| m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0]) + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let a = [[s4, s3, s2], [s3, s2, s1], [s2, s1, s0]];
    let da = det(a);
    if da.abs() < 1e-12 {
        return 0.0;
    }
    let ax = [[t2, s3, s2], [t1, s2, s1], [t0, s1, s0]];
    (det(ax) / da).abs()
}

/// Richtung auf 0°/90° (Toleranz `deg`) bzw. 45° (¾ davon) einrasten.
pub(crate) fn snap_dir(d: V, deg: f64) -> Option<V> {
    if deg <= 0.0 {
        return None;
    }
    let ang = d.y.atan2(d.x).to_degrees();
    let k = (ang / 45.0).round();
    let diff = (ang - k * 45.0).abs();
    let lim = if (k as i64).rem_euclid(2) == 0 { deg } else { 0.75 * deg };
    if diff > lim {
        return None;
    }
    let t = (k * 45.0).to_radians();
    let (s, c) = t.sin_cos();
    // exakte Achsen (keine Rundungsreste)
    let r = |v: f64| if v.abs() < 1e-12 { 0.0 } else if (v.abs() - 1.0).abs() < 1e-12 { v.signum() } else { v };
    Some(V::new(r(c), r(s)))
}

fn intersect(l1: &Line, l2: &Line) -> Option<V> {
    let den = l1.d.cross(l2.d);
    if den.abs() < (1.0f64).to_radians().sin() {
        return None;
    }
    let t = (l2.c - l1.c).cross(l2.d) / den;
    Some(l1.c + l1.d * t)
}

fn project(l: &Line, p: V) -> V {
    l.c + l.d * (p - l.c).dot(l.d)
}

// -------------------------------------------------------------------- Bézier-Anpassung

fn chord_params(pts: &[V]) -> Vec<f64> {
    let mut u = Vec::with_capacity(pts.len());
    u.push(0.0);
    for i in 1..pts.len() {
        let prev = u[i - 1];
        u.push(prev + pts[i].dist(pts[i - 1]));
    }
    let total = *u.last().unwrap_or(&0.0);
    let n = pts.len();
    if total < 1e-12 {
        return (0..n).map(|i| i as f64 / (n - 1).max(1) as f64).collect();
    }
    u.iter().map(|v| v / total).collect()
}

fn generate(pts: &[V], u: &[f64], t0: V, t1: V) -> Seg {
    let (p0, p3) = (pts[0], pts[pts.len() - 1]);
    let (mut c00, mut c01, mut c11, mut x0, mut x1) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &p) in pts.iter().enumerate() {
        let t = u[i];
        let mt = 1.0 - t;
        let (b0, b1, b2, b3) = (mt * mt * mt, 3.0 * mt * mt * t, 3.0 * mt * t * t, t * t * t);
        let a1 = t0 * b1;
        let a2 = t1 * b2;
        c00 += a1.dot(a1);
        c01 += a1.dot(a2);
        c11 += a2.dot(a2);
        let tmp = p - (p0 * (b0 + b1) + p3 * (b2 + b3));
        x0 += a1.dot(tmp);
        x1 += a2.dot(tmp);
    }
    let det = c00 * c11 - c01 * c01;
    let seg_len = p0.dist(p3);
    let eps = 1e-6 * seg_len.max(1e-9);
    let (mut al, mut ar) = if det.abs() > 1e-12 { ((x0 * c11 - x1 * c01) / det, (c00 * x1 - c01 * x0) / det) } else { (0.0, 0.0) };
    if al < eps || ar < eps || al > 3.0 * seg_len || ar > 3.0 * seg_len {
        al = seg_len / 3.0;
        ar = seg_len / 3.0;
    }
    Seg { p0, c1: p0 + t0 * al, c2: p3 + t1 * ar, p3 }
}

fn max_error(pts: &[V], seg: &Seg, u: &[f64]) -> (f64, usize) {
    let mut best = (0.0, pts.len() / 2);
    for i in 1..pts.len().saturating_sub(1) {
        let d = (seg.eval(u[i]) - pts[i]).len();
        if d > best.0 {
            best = (d, i);
        }
    }
    best
}

fn reparam(pts: &[V], seg: &Seg, u: &mut [f64]) {
    for (i, &p) in pts.iter().enumerate() {
        let t = u[i];
        let q = seg.eval(t) - p;
        let d1 = seg.d1(t);
        let d2 = seg.d2(t);
        let num = q.dot(d1);
        let den = d1.dot(d1) + q.dot(d2);
        if den.abs() > 1e-12 {
            u[i] = (t - num / den).clamp(0.0, 1.0);
        }
    }
}

/// Ein Segment durch pts mit Tangenten t0 (vom Anfang weg) und t1 (vom Ende zurück).
fn fit_one(pts: &[V], t0: V, t1: V, iters: usize, tol: f64) -> (Seg, f64, usize) {
    if pts.len() <= 2 {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let l = a.dist(b) / 3.0;
        return (Seg { p0: a, c1: a + t0 * l, c2: b + t1 * l, p3: b }, 0.0, 0);
    }
    let mut u = chord_params(pts);
    let mut seg = generate(pts, &u, t0, t1);
    let (mut err, mut idx) = max_error(pts, &seg, &u);
    if err <= tol || err > 6.0 * tol {
        return (seg, err, idx);
    }
    for _ in 0..iters {
        reparam(pts, &seg, &mut u);
        let s2 = generate(pts, &u, t0, t1);
        let (e2, i2) = max_error(pts, &s2, &u);
        if e2 < err {
            (seg, err, idx) = (s2, e2, i2);
        }
        if err <= tol {
            break;
        }
    }
    (seg, err, idx)
}

fn local_tangent(pts: &[V], i: usize) -> V {
    let w = 3.min(i).min(pts.len() - 1 - i).max(1);
    let (lo, hi) = (i.saturating_sub(w), (i + w).min(pts.len() - 1));
    (pts[hi] - pts[lo]).norm()
}

/// Gierig: von vorn jeweils das längste Stück, das in die Toleranz passt (Binärsuche).
fn fit_greedy(pts: &[V], t0: V, t1: V, p: &FitParams, out: &mut Vec<Seg>) {
    let m = pts.len() - 1;
    let mut s = 0usize;
    let mut ts = t0;
    while s < m {
        let (seg, err, _) = fit_one(&pts[s..=m], ts, t1, p.iters, p.tol);
        if err <= p.tol {
            out.push(seg);
            return;
        }
        let (mut lo, mut hi) = (s + 1, m);
        let mut best = fit_one(&pts[s..=lo], ts, -local_tangent(pts, lo), 0, p.tol).0;
        while hi - lo > 1 {
            let mid = (lo + hi) / 2;
            let (seg, err, _) = fit_one(&pts[s..=mid], ts, -local_tangent(pts, mid), p.iters, p.tol);
            if err <= p.tol {
                lo = mid;
                best = seg;
            } else {
                hi = mid;
            }
        }
        out.push(best);
        ts = local_tangent(pts, lo);
        s = lo;
    }
}

/// Klassisch: bei zu großem Fehler am schlechtesten Punkt teilen.
fn fit_split(pts: &[V], t0: V, t1: V, p: &FitParams, out: &mut Vec<Seg>, depth: usize) {
    let (seg, err, idx) = fit_one(pts, t0, t1, p.iters, p.tol);
    if err <= p.tol || pts.len() <= 3 || depth > 24 {
        out.push(seg);
        return;
    }
    let idx = idx.clamp(1, pts.len() - 2);
    let tc = local_tangent(pts, idx);
    fit_split(&pts[..=idx], t0, -tc, p, out, depth + 1);
    fit_split(&pts[idx..], tc, t1, p, out, depth + 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> FitParams {
        FitParams { tol: 1.0, corner_deg: 60.0, snap_deg: 4.0, reduce: true, iters: 4, polygon: false }
    }

    #[test]
    fn square_ring_becomes_four_lines() {
        // Kantenmitten eines 20×20-Quadrats
        let mut s = Vec::new();
        for x in 0..20 {
            s.push(V::new(x as f64 + 0.5, 0.0));
        }
        for y in 0..20 {
            s.push(V::new(20.0, y as f64 + 0.5));
        }
        for x in (0..20).rev() {
            s.push(V::new(x as f64 + 0.5, 20.0));
        }
        for y in (0..20).rev() {
            s.push(V::new(0.0, y as f64 + 0.5));
        }
        let mut st = FitStats::default();
        let segs = fit_chain(&s, true, &params(), &mut st, [None, None]);
        assert_eq!(segs.len(), 4, "{segs:?}");
        for sg in &segs {
            assert_eq!(sg.c1, sg.p0);
            for q in [sg.p0, sg.p3] {
                assert!((q.x == 0.0 || q.x == 20.0) && (q.y == 0.0 || q.y == 20.0), "{q:?}");
            }
        }
    }

    #[test]
    fn snap_directions() {
        assert_eq!(snap_dir(V::new(1.0, 0.03), 4.0), Some(V::new(1.0, 0.0)));
        assert_eq!(snap_dir(V::new(0.02, -1.0), 4.0), Some(V::new(0.0, -1.0)));
        assert!(snap_dir(V::new(1.0, 0.2), 4.0).is_none());
        let d = snap_dir(V::new(1.0, 1.03), 4.0).expect("45°");
        assert!((d.x - d.y).abs() < 1e-12);
    }
}
