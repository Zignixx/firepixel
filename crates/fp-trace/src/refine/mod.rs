//! # Logo-Nachbearbeitung
//!
//! Eigener Nachzeichen-Ablauf für flächige Motive (Logos, Symbole, Schrift), eingeschaltet über
//! [`RefineOptions::enabled`] (Vorgabe „Logo“: an). Statt der Farbcluster und Splines der
//! Bibliothek:
//!
//! 1. **Farben einrasten** ([`palette`]): wenige, saubere Originalfarben aus den flachen Flächen;
//!    Kantenpixel (Kantenglättung, JPEG-Säume) werden einer Farbe ihrer Nachbarschaft zugeordnet.
//! 2. **Flächen und gemeinsame Kanten** ([`outline`]): Flecken verschmelzen, Umrisse entlang der
//!    Pixelkanten, geteilt in Randketten zwischen je zwei Flächen. Jede Kette wird einmal angepasst
//!    und von beiden Nachbarn benutzt → keine Haarrisse (Schalter `close_gaps`).
//! 3. **Grundformen** ([`shapes`]): Kreis, Ellipse, Rechteck, abgerundetes Rechteck exakt.
//! 4. **Geraden und Kurven** ([`fit`]): Geraden erkennen und auf 0°/45°/90° bzw. rechtwinklig
//!    einrasten, Ecken im Schnittpunkt, Kurven mit möglichst wenigen Bézier-Knoten (glatt).
//! 5. **Symmetrie** ([`symmetry`], optional): Spiegelachse erkennen und beide Seiten angleichen.
//!
//! „Stapeln“: Löcher, die ganz von darüberliegenden Flächen gefüllt sind, entfallen (die größere
//! Fläche liegt darunter, keine Nahtstellen). „Ausschneiden“: alle Löcher bleiben, Nachbarn teilen
//! sich exakt dieselbe Kante.

mod fit;
mod outline;
pub(crate) mod palette;
mod shapes;
mod symmetry;

use crate::{check, CurveMode, Hierarchy, Mapper, Progress, Stage, TraceError, TraceOptions, TracedPath};
use fit::{FitParams, FitStats, Seg};
use fp_color::Rgba8;
use fp_geom::{BezierContour, BezierNode};
use fp_raster::RgbaImage;
use serde::{Deserialize, Serialize};
use shapes::Shape;
use std::sync::atomic::AtomicBool;

/// Kein Eintrag (transparent bzw. außerhalb).
pub(crate) const NONE: u32 = u32::MAX;

/// Höchstzahl der Pixel bei [`RefineOptions::fast`] (schnelle Vorschau).
pub const FAST_PREVIEW_PIXELS: u64 = 1_000_000;

/// Schalter der Logo-Nachbearbeitung (Teil von [`TraceOptions`], JSON `refine`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RefineOptions {
    /// Nachbearbeitung an (Vorgabe „Logo“). Aus: Ablauf wie bisher (Farbcluster + Splines).
    pub enabled: bool,
    /// Farben auf wenige saubere Originalfarben einrasten (sonst Median-Cut wie `colors`).
    pub snap_colors: bool,
    /// Farbabstand (RGB, 8–128), bis zu dem ähnliche Töne zusammengefasst werden.
    pub color_tolerance: f64,
    /// Geraden einrasten: fast waagerecht/senkrecht/45° bzw. fast rechtwinklig → exakt.
    pub straighten: bool,
    /// Winkeltoleranz fürs Einrasten (Grad, 0–15; 45° mit ¾ davon).
    pub snap_angle: f64,
    /// Kreise, Ellipsen, Rechtecke, abgerundete Rechtecke durch exakte Grundformen ersetzen.
    pub shapes: bool,
    /// Höchste Abweichung einer Grundform vom Pixelumriss (px, 0,25–5).
    pub shape_tolerance: f64,
    /// Knoten sparen: jeweils die längste Kurve, die in die Toleranz passt (sonst klassisches Teilen).
    pub reduce_nodes: bool,
    /// Höchste Abweichung der Kurven vom Pixelumriss (px, 0,2–4).
    pub fit_tolerance: f64,
    /// Lücken schließen: benachbarte Flächen teilen sich exakt dieselbe Kante.
    pub close_gaps: bool,
    /// Spiegelachse (senkrecht/waagerecht) erkennen und beide Seiten angleichen.
    pub symmetry: bool,
    /// Schnelle Vorschau: höchstens [`FAST_PREVIEW_PIXELS`], weniger Nachbesserungsschritte, keine Symmetrie.
    pub fast: bool,
}

impl Default for RefineOptions {
    /// Alle Schalter wie bei „Logo“, aber **aus** (`enabled = false`).
    fn default() -> Self {
        RefineOptions { enabled: false, ..RefineOptions::logo() }
    }
}

impl RefineOptions {
    /// Werte der Vorgabe „Logo“ (an).
    pub fn logo() -> RefineOptions {
        RefineOptions {
            enabled: true,
            snap_colors: true,
            color_tolerance: 40.0,
            straighten: true,
            snap_angle: 4.0,
            shapes: true,
            shape_tolerance: 1.25,
            reduce_nodes: true,
            fit_tolerance: 1.0,
            close_gaps: true,
            symmetry: false,
            fast: false,
        }
    }

    /// Alle Werte in ihren Bereichen (NaN → Vorgabewert).
    pub fn validated(&self) -> RefineOptions {
        let d = RefineOptions::logo();
        let f = |v: f64, lo: f64, hi: f64, def: f64| if v.is_finite() { v.clamp(lo, hi) } else { def };
        RefineOptions {
            color_tolerance: f(self.color_tolerance, 8.0, 128.0, d.color_tolerance),
            snap_angle: f(self.snap_angle, 0.0, 15.0, d.snap_angle),
            shape_tolerance: f(self.shape_tolerance, 0.25, 5.0, d.shape_tolerance),
            fit_tolerance: f(self.fit_tolerance, 0.2, 4.0, d.fit_tolerance),
            ..self.clone()
        }
    }
}

/// Erkannte Spiegelsymmetrie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Symmetry {
    Vertical,
    Horizontal,
    Both,
}

/// Kennzahlen der Logo-Nachbearbeitung ([`crate::trace_with_report`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RefineReport {
    /// Eingerastete Palette (Reihenfolge der Erkennung, häufigste zuerst).
    pub palette: Vec<Rgba8>,
    /// Knoten vorher: Eckpunkte der Pixelumrisse (Treppenstufen) aller ausgegebenen Konturen.
    pub nodes_before: usize,
    /// Knoten nachher (= [`crate::TraceResult::node_count`]).
    pub nodes_after: usize,
    pub circles: usize,
    pub ellipses: usize,
    pub rectangles: usize,
    pub rounded_rectangles: usize,
    /// Erkannte Geraden (je Randkette) und davon exakt eingerastete (0°/45°/90°).
    pub lines: usize,
    pub snapped_lines: usize,
    /// Randketten, die zwei Flächen exakt gemeinsam benutzen.
    pub shared_edges: usize,
    pub symmetry: Option<Symmetry>,
    /// Größe des bearbeiteten Bildes (nach Verkleinerung).
    pub width: u32,
    pub height: u32,
}

impl RefineReport {
    /// Anzahl aller erkannten Grundformen.
    pub fn shape_count(&self) -> usize {
        self.circles + self.ellipses + self.rectangles + self.rounded_rectangles
    }
}

pub(crate) fn trace(img: &RgbaImage, o: &TraceOptions, map: &Mapper, cancel: &AtomicBool, progress: &mut dyn FnMut(Progress)) -> Result<(Vec<crate::TracedPath>, RefineReport), TraceError> {
    let r = &o.refine;
    let (w, h) = (img.width(), img.height());
    let mut rep = RefineReport { width: w as u32, height: h as u32, ..RefineReport::default() };
    progress(Progress { stage: Stage::Cluster, fraction: 0.05 });
    let mut lab = palette::build(img, o);
    rep.palette = lab.palette.iter().map(|c| Rgba8::new(c[0], c[1], c[2], 255)).collect();
    check(cancel)?;
    progress(Progress { stage: Stage::Cluster, fraction: 0.20 });
    let axes = if r.symmetry && !r.fast { symmetry::detect_and_apply(&mut lab.label, w, h) } else { symmetry::Axes::default() };
    rep.symmetry = match (axes.vertical.is_some(), axes.horizontal.is_some()) {
        (true, true) => Some(Symmetry::Both),
        (true, false) => Some(Symmetry::Vertical),
        (false, true) => Some(Symmetry::Horizontal),
        _ => None,
    };
    // Flecken: n × n Pixel, bei winzigen Bildern (Symbole) höchstens 0,5 % der Fläche
    let speck = ((o.filter_speckle * o.filter_speckle) as usize).min((w * h / 200).max(2));
    outline::merge_speckles(&mut lab.label, w, h, speck);
    check(cancel)?;
    let reg = outline::components(&lab.label, w, h);
    progress(Progress { stage: Stage::Cluster, fraction: 0.30 });
    let contours = outline::trace_contours(&reg.comp, w, h);
    let (chains, refs) = outline::build_chains(&contours, &reg.comp, w, h, r.close_gaps);
    rep.shared_edges = chains.iter().filter(|c| c.uses >= 2).count();
    check(cancel)?;

    // Ketten anpassen
    let polygon = o.mode == CurveMode::Polygon;
    let fp = FitParams {
        tol: r.fit_tolerance,
        corner_deg: o.corner_threshold.clamp(20.0, 179.0),
        snap_deg: if r.straighten { r.snap_angle } else { 0.0 },
        reduce: r.reduce_nodes,
        iters: if r.fast { 1 } else { 4 },
        polygon,
    };
    let mut stats = FitStats::default();
    let mut fitted: Vec<Vec<Seg>> = Vec::with_capacity(chains.len());
    let nch = chains.len().max(1);
    for (k, ch) in chains.iter().enumerate() {
        if k % 64 == 0 {
            check(cancel)?;
            progress(Progress { stage: Stage::Paths, fraction: 0.30 + 0.65 * k as f32 / nch as f32 });
        }
        let samples = ch.samples();
        if ch.closed && r.shapes && !ch.pts.iter().any(|&(x, y)| x == 0 || y == 0 || x == w as i32 || y == h as i32) {
            let snap = if r.straighten { r.snap_angle } else { 0.0 };
            if let Some((shape, segs)) = shapes::detect(&ch.pts, &samples, r.shape_tolerance, snap, !polygon) {
                match shape {
                    Shape::Circle => rep.circles += 1,
                    Shape::Ellipse => rep.ellipses += 1,
                    Shape::Rect => rep.rectangles += 1,
                    Shape::RoundedRect => rep.rounded_rectangles += 1,
                }
                fitted.push(segs);
                continue;
            }
        }
        if axes != symmetry::Axes::default()
            && let Some(segs) = symmetry::fit_mirrored(ch, &axes, &fp, &mut stats)
        {
            fitted.push(segs);
            continue;
        }
        fitted.push(fit::fit_chain(&samples, ch.closed, &fp, &mut stats, [None, None]));
    }
    rep.lines = stats.lines;
    rep.snapped_lines = stats.snapped;
    if axes != symmetry::Axes::default() {
        symmetry::equalize(&chains, &mut fitted, &axes);
    }
    check(cancel)?;

    // Löcher, die transparente Pixel enthalten, bleiben auch beim Stapeln.
    let stacked = o.hierarchical == Hierarchy::Stacked;
    let enclosed_none = if stacked { enclosed_transparent(&reg.comp, w, h) } else { Vec::new() };

    // Flächenfarben
    let ncomp = reg.color.len();
    let colors: Vec<[u8; 3]> = if lab.exact {
        reg.color.iter().map(|&l| lab.palette[l as usize]).collect()
    } else {
        let mut sums = vec![[0u64; 4]; ncomp];
        for (i, &c) in reg.comp.iter().enumerate() {
            if c != NONE {
                let s = &mut sums[c as usize];
                for (acc, &v) in s.iter_mut().zip(&lab.rgb[i]) {
                    *acc += v as u64;
                }
                s[3] += 1;
            }
        }
        sums.iter().map(|s| [0, 1, 2].map(|k| (s[k] / s[3].max(1)) as u8)).collect()
    };

    // Reihenfolge: größte Außenfläche zuunterst
    let mut outer_area = vec![0i64; ncomp];
    let mut per_region: Vec<Vec<usize>> = vec![Vec::new(); ncomp];
    for (k, cr) in refs.iter().enumerate() {
        if cr.area2 > 0 {
            outer_area[cr.region as usize] = cr.area2;
        }
        per_region[cr.region as usize].push(k);
    }
    let mut order: Vec<usize> = (0..ncomp).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(outer_area[i]), i));

    let mut out = Vec::with_capacity(ncomp);
    for ri in order {
        let mut cs: Vec<BezierContour> = Vec::new();
        let mut list = per_region[ri].clone();
        list.sort_by_key(|&k| std::cmp::Reverse(refs[k].area2));
        for k in list {
            let cr = &refs[k];
            if stacked && cr.area2 < 0 {
                let poly = &contours[cr.contour].corners;
                if enclosed_none.len() <= 4096 && !enclosed_none.iter().any(|&(x, y)| outline::inside(poly, x, y)) {
                    continue;
                }
            }
            rep.nodes_before += contours[cr.contour].turns;
            let mut segs: Vec<Seg> = Vec::new();
            for &(ci, rev) in &cr.uses {
                if rev {
                    segs.extend(fit::reverse(&fitted[ci]));
                } else {
                    segs.extend_from_slice(&fitted[ci]);
                }
            }
            if let Some(c) = to_contour(&segs, map) {
                cs.push(c);
            }
        }
        if cs.is_empty() {
            continue;
        }
        let c = colors[ri];
        out.push(TracedPath { contours: cs, color: Rgba8::new(c[0], c[1], c[2], 255), pixels: reg.size[ri] });
    }
    rep.nodes_after = out.iter().map(TracedPath::node_count).sum();
    Ok((out, rep))
}

/// Segmente → geschlossene Bézier-Kontur in Dokumentkoordinaten (ohne Nullsegmente).
fn to_contour(segs: &[Seg], map: &Mapper) -> Option<BezierContour> {
    let m = |p: fit::V| map.map(p.x, p.y);
    let mapped: Vec<[fp_geom::Point; 4]> = segs.iter().map(|s| [m(s.p0), m(s.c1), m(s.c2), m(s.p3)]).filter(|q| !(q[0] == q[3] && q[1] == q[0] && q[2] == q[3])).collect();
    let n = mapped.len();
    if n < 2 {
        return None;
    }
    let nodes = (0..n).map(|i| BezierNode::new(mapped[i][0], mapped[(i + n - 1) % n][2], mapped[i][1])).collect();
    Some(BezierContour::new(nodes, true))
}

/// Mittelpunkte je eingeschlossener transparenter Fläche (berührt den Bildrand nicht).
fn enclosed_transparent(comp: &[u32], w: usize, h: usize) -> Vec<(f64, f64)> {
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for s in 0..w * h {
        if comp[s] != NONE || seen[s] {
            continue;
        }
        seen[s] = true;
        stack.push(s);
        let mut border = false;
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            if x == 0 || y == 0 || x + 1 == w || y + 1 == h {
                border = true;
            }
            for (ok, j) in [(x > 0, i.wrapping_sub(1)), (x + 1 < w, i + 1), (y > 0, i.wrapping_sub(w)), (y + 1 < h, i + w)] {
                if ok && comp[j] == NONE && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        if !border {
            out.push(((s % w) as f64 + 0.5, (s / w) as f64 + 0.5));
            if out.len() > 4096 {
                break;
            }
        }
    }
    out
}
