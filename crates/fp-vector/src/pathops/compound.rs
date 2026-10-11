//! Compound Shapes (`CSGv`) und die destruktiven Befehle „Pfade kombinieren“
//! (Spezifikation 16 §2.1, §2.2, §2.4, §2.5; Referenz `re\pfade\compound.py`).
//!
//! **Auswertung** (`@00493730`, B): Genau ein Operand → Ergebnis = Operand. Sonst Akkumulator =
//! unterster Operand (seine `POPi` wird nie gelesen), dann von unten nach oben je Operand nach
//! `POPi`: 1 Vereinigung, 2 Schnittmenge, 3 Ausstanzen, 4 Zuschneiden; **0 und andere Werte: kein
//! Schritt**, der Operand fließt nicht ein. Operanden mit höchstens 1 Knoten werden bei `op ≠ 2`
//! übersprungen. Jeder Operand geht mit seiner eigenen Füllregel ein.
//!
//! **Form des Ergebnisses** (B Korpus): Außenkonturen negative Gauß-Fläche (auf dem Bildschirm gegen
//! den Uhrzeigersinn), Löcher positiv; Start am Startknoten der Ursprungskontur (unterster Operand
//! zuerst); Reihenfolge nach Gruppen-ID in Baum-Vorordnung, stabil sortiert ([`order_contours`]);
//! Ergebnis `EOFb = 1`. Abweichung von der Python-Referenz: dort wird `POPi = 0` wie eine
//! Vereinigung behandelt und ein einzelner Operand normalisiert.
//!
//! **Rendering**: Das Original berechnet beim Laden **nicht** neu (B `@00792460`); fp-render rendert das
//! gespeicherte Ergebnis und ruft [`evaluate_compound`] erst nach einer Bearbeitung.

use std::collections::HashMap;

use super::boolean::{boolean, BoolOp, FillRule, Region, GRID};
use super::geom::{flatten, signed_area, winding_polys, Contour};
use fp_geom::Vec2;

/// Verknüpfung eines Compound-Operanden (`POPi`, Tabelle `@0174bfa0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum PathOp {
    /// 0 = Normal: kein boolescher Schritt.
    #[default]
    Normal = 0,
    /// 1 = Vereinigung.
    Union = 1,
    /// 2 = Überschneiden.
    Intersect = 2,
    /// 3 = Ausstanzen.
    Punch = 3,
    /// 4 = Zuschneiden.
    Crop = 4,
}

impl PathOp {
    /// Aus `POPi`; unbekannte Werte (auch Müll beim untersten Operanden) → Normal.
    pub fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Union,
            2 => Self::Intersect,
            3 => Self::Punch,
            4 => Self::Crop,
            _ => Self::Normal,
        }
    }
}

/// Ein Operand: Konturen, Füllregel (`EOFb`) und Verknüpfung (`POPi`).
#[derive(Debug, Clone, PartialEq)]
pub struct Operand {
    pub contours: Vec<Contour>,
    pub even_odd: bool,
    pub op: PathOp,
}

impl Operand {
    pub fn new(contours: Vec<Contour>, even_odd: bool, op: PathOp) -> Self {
        Self { contours, even_odd, op }
    }

    fn region(&self) -> Region {
        Region::with_eof(self.contours.clone(), self.even_odd)
    }
}

/// Konturen normalisieren: Füllregel auflösen (Vereinigung mit der leeren Region).
pub fn normalize(r: &Region) -> Vec<Contour> {
    boolean(r, &Region::empty(), BoolOp::Union, GRID)
}

fn apply(acc: &[Contour], operand: &Operand, op: PathOp) -> Vec<Contour> {
    let a = Region::new(acc.to_vec(), FillRule::EvenOdd);
    let b = operand.region();
    let bop = match op {
        PathOp::Union => BoolOp::Union,
        PathOp::Intersect | PathOp::Crop => BoolOp::Intersect,
        PathOp::Punch => BoolOp::Subtract,
        PathOp::Normal => return acc.to_vec(),
    };
    boolean(&a, &b, bop, GRID)
}

/// Compound Shape auswerten. `operands` in Dateireihenfolge (**oberster zuerst**, wie `ELMv`).
pub fn evaluate_compound(operands: &[Operand]) -> Vec<Contour> {
    match operands.len() {
        0 => return Vec::new(),
        1 => return operands[0].contours.clone(),
        _ => {}
    }
    let bottom_up: Vec<&Operand> = operands.iter().rev().collect();
    let mut acc = normalize(&bottom_up[0].region());
    for e in &bottom_up[1..] {
        let nodes: usize = e.contours.iter().map(|c| c.nodes.len()).sum();
        if e.op == PathOp::Normal || (nodes <= 1 && e.op != PathOp::Intersect) {
            continue;
        }
        acc = apply(&acc, e, e.op);
    }
    let starts: Vec<Vec2> = bottom_up.iter().flat_map(|e| e.contours.iter().filter_map(|c| c.nodes.first().map(|n| n.anchor))).collect();
    let res = original_order(&acc, &starts);
    let sources: Vec<&[Contour]> = operands.iter().map(|o| o.contours.as_slice()).collect();
    order_contours(&res, &sources)
}

/// Ergebnis wie das Original ausrichten (A, aus dem Korpus): umdrehen (eigene Konturen haben das
/// Innere links) und, wenn möglich, am Startknoten der Ursprungskontur beginnen (Priorität in der
/// Reihenfolge von `start_points`, Toleranz 10⁻⁴).
pub fn original_order(contours: &[Contour], start_points: &[Vec2]) -> Vec<Contour> {
    contours
        .iter()
        .map(|c| {
            let mut c = c.reversed();
            for sp in start_points {
                if let Some(k) = c.nodes.iter().position(|n| (n.anchor.x - sp.x).abs() < 1e-4 && (n.anchor.y - sp.y).abs() < 1e-4) {
                    c.nodes.rotate_left(k);
                    break;
                }
            }
            c
        })
        .collect()
}

fn probe(c: &Contour) -> Vec2 {
    let segs = c.segments();
    let Some(s) = segs.first() else { return c.nodes.first().map(|n| n.anchor).unwrap_or_default() };
    let m = s.eval(0.5);
    let t = s.tangent(0.5);
    let a = signed_area(&c.anchors());
    let k = if a > 0.0 { 1e-3 } else { -1e-3 };
    Vec2::new(m.x - t.y * k, m.y + t.x * k)
}

/// Schlüssel `round(v, 3)` (Dezimalrundung wie Python, −0 = 0).
fn key3(v: f64) -> u64 {
    let s = format!("{v:.3}");
    (s.parse::<f64>().unwrap_or(0.0) + 0.0).to_bits()
}

/// Reihenfolge der Ergebniskonturen wie python.dll (`@10013b10`/`@10011bf0`/`@10012ce0`):
/// Verschachtelungsbaum in Verfolgungsreihenfolge (Rang der Quellkontur, oberster Operand zuerst),
/// Gruppen-IDs in Tiefensuche (Außenkontur neue ID, Loch die der Elternkontur), stabil sortiert.
/// `sources` = Konturen der Operanden, **oberster zuerst**.
pub fn order_contours(contours: &[Contour], sources: &[&[Contour]]) -> Vec<Contour> {
    let mut rank_of: HashMap<(u64, u64), (usize, usize)> = HashMap::new();
    for (oi, cs) in sources.iter().enumerate() {
        for (ci, c) in cs.iter().enumerate() {
            for nd in &c.nodes {
                rank_of.entry((key3(nd.anchor.x), key3(nd.anchor.y))).or_insert((oi, ci));
            }
        }
    }
    let big = (sources.len(), 0);
    let rank = |c: &Contour| -> (usize, usize) {
        c.nodes.iter().map(|nd| rank_of.get(&(key3(nd.anchor.x), key3(nd.anchor.y))).copied().unwrap_or(big)).min().unwrap_or(big)
    };
    let mut items: Vec<usize> = (0..contours.len()).collect();
    items.sort_by_key(|&i| rank(&contours[i]));
    let polys: Vec<Vec<(Vec<Vec2>, bool)>> = contours.iter().map(|c| flatten(std::slice::from_ref(c), 0.05)).collect();
    let areas: Vec<f64> = polys.iter().map(|p| p.first().map(|(pts, _)| signed_area(pts).abs()).unwrap_or(0.0)).collect();
    let probes: Vec<Vec2> = contours.iter().map(probe).collect();
    let mut parent: Vec<Option<usize>> = vec![None; contours.len()];
    for &i in &items {
        let mut best: Option<usize> = None;
        for &j in &items {
            if j != i && areas[j] > areas[i] && winding_polys(&polys[j], probes[i]) != 0 && best.is_none_or(|b| areas[j] < areas[b]) {
                best = Some(j);
            }
        }
        parent[i] = best;
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); contours.len()];
    let mut roots = Vec::new();
    for &i in &items {
        match parent[i] {
            Some(p) => children[p].push(i),
            None => roots.push(i),
        }
    }
    let mut group = vec![0usize; contours.len()];
    let mut counter = 0usize;
    // Tiefensuche mit eigenem Stapel (Vorordnung)
    let mut stack: Vec<(usize, usize)> = roots.iter().rev().map(|&r| (r, 0)).collect();
    while let Some((i, depth)) = stack.pop() {
        if depth % 2 == 0 {
            counter += 1;
            group[i] = counter;
        } else {
            group[i] = parent[i].map(|p| group[p]).unwrap_or(0);
        }
        for &k in children[i].iter().rev() {
            stack.push((k, depth + 1));
        }
    }
    let mut order = items.clone();
    order.sort_by_key(|&i| group[i]);
    order.into_iter().map(|i| contours[i].clone()).collect()
}

/// `POPi` für `createCompoundShape(op)` (`@00456e20`, B Maschinencode `@004574b9`): oberster
/// Operand (Index 0) bekommt `op`; bei Ausstanzen/Zuschneiden bekommen die mittleren 1
/// (Vereinigung), bei Vereinigen/Überschneiden ebenfalls `op`; der unterste bekommt keinen Wert
/// (`Normal`). `count` = Anzahl der Operanden, Rückgabe oberster zuerst.
pub fn create_compound_ops(op: PathOp, count: usize) -> Vec<PathOp> {
    (0..count)
        .map(|i| {
            if i + 1 == count {
                PathOp::Normal
            } else if i == 0 || matches!(op, PathOp::Union | PathOp::Intersect) {
                op
            } else {
                PathOp::Union
            }
        })
        .collect()
}

/// Ergebnisform für destruktive Befehle: Original-Ausrichtung und -Reihenfolge.
fn finish(res: Vec<Contour>, operands_top_first: &[&Operand]) -> Vec<Contour> {
    let starts: Vec<Vec2> =
        operands_top_first.iter().rev().flat_map(|e| e.contours.iter().filter_map(|c| c.nodes.first().map(|n| n.anchor))).collect();
    let res = original_order(&res, &starts);
    let sources: Vec<&[Contour]> = operands_top_first.iter().map(|o| o.contours.as_slice()).collect();
    order_contours(&res, &sources)
}

/// **Vereinigen** (16 §2.2): ein Pfad aus allen Flächen. Kumulativ: Akkumulator = oberstes
/// Element, dann jedes weitere (`ApuUnionArea(acc, cur)`). Attribute nimmt der Aufrufer vom
/// **untersten** Objekt. `paths` oberster zuerst; `None` bei weniger als 2 Pfaden (Meldung 3116).
pub fn union_paths(paths: &[Operand]) -> Option<Vec<Contour>> {
    cumulative(paths, BoolOp::Union)
}

/// **Überschneiden**: Schnittmenge aller Pfade (wie [`union_paths`] kumulativ).
pub fn intersect_paths(paths: &[Operand]) -> Option<Vec<Contour>> {
    cumulative(paths, BoolOp::Intersect)
}

fn cumulative(paths: &[Operand], op: BoolOp) -> Option<Vec<Contour>> {
    if paths.len() < 2 {
        return None;
    }
    let mut acc = paths[0].region();
    let mut res = Vec::new();
    for p in &paths[1..] {
        res = boolean(&acc, &p.region(), op, GRID);
        acc = Region::new(res.clone(), FillRule::EvenOdd);
    }
    let all: Vec<&Operand> = paths.iter().collect();
    Some(finish(res, &all))
}

/// **Ausstanzen**: das oberste Objekt stanzt aus **jedem** weiteren einzeln aus; jedes Ziel bleibt
/// ein eigener Pfad mit seinen Attributen, der Stanzer verschwindet. Ergebnis je Ziel (Reihenfolge
/// wie `paths[1..]`); `None` bei weniger als 2 Pfaden.
pub fn punch_paths(paths: &[Operand]) -> Option<Vec<Vec<Contour>>> {
    per_target(paths, BoolOp::Subtract)
}

/// **Zuschneiden**: das oberste Objekt schneidet jedes weitere auf seine Fläche zu.
pub fn crop_paths(paths: &[Operand]) -> Option<Vec<Vec<Contour>>> {
    per_target(paths, BoolOp::Intersect)
}

fn per_target(paths: &[Operand], op: BoolOp) -> Option<Vec<Vec<Contour>>> {
    if paths.len() < 2 {
        return None;
    }
    let tool = paths[0].region();
    Some(
        paths[1..]
            .iter()
            .map(|t| {
                let res = boolean(&t.region(), &tool, op, GRID);
                finish(res, &[&paths[0], t])
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::super::geom::{area, Contour};
    use super::*;

    fn sq(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
        Contour::polygon(&[Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)], true)
    }

    #[test]
    fn compound_punch_three() {
        // Basis B, Mitte M, Stanzer S: (B ∪ M) − S
        let ops = create_compound_ops(PathOp::Punch, 3);
        assert_eq!(ops, vec![PathOp::Punch, PathOp::Union, PathOp::Normal]);
        let s = Operand::new(vec![sq(4.0, 4.0, 6.0, 6.0)], true, ops[0]);
        let m = Operand::new(vec![sq(5.0, 0.0, 15.0, 10.0)], true, ops[1]);
        let b = Operand::new(vec![sq(0.0, 0.0, 10.0, 10.0)], true, ops[2]);
        let r = evaluate_compound(&[s, m, b]);
        assert!((area(&r, 0.01).abs() - 146.0).abs() < 1e-9);
        // Außenkontur negative Fläche, Loch positiv
        let areas: Vec<f64> = r.iter().map(|c| signed_area(&c.anchors())).collect();
        assert!(areas.iter().any(|a| *a < 0.0) && areas.iter().any(|a| *a > 0.0));
        // POPi 0 → Operand fließt nicht ein
        let n = Operand::new(vec![sq(20.0, 20.0, 30.0, 30.0)], true, PathOp::Normal);
        let b = Operand::new(vec![sq(0.0, 0.0, 10.0, 10.0)], true, PathOp::Normal);
        let r = evaluate_compound(&[n, b]);
        assert!((area(&r, 0.01).abs() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn destructive() {
        let a = Operand::new(vec![sq(0.0, 0.0, 10.0, 10.0)], true, PathOp::Normal);
        let b = Operand::new(vec![sq(5.0, 5.0, 15.0, 15.0)], true, PathOp::Normal);
        let c = Operand::new(vec![sq(-5.0, -5.0, 2.0, 2.0)], true, PathOp::Normal);
        let u = union_paths(&[a.clone(), b.clone(), c.clone()]).unwrap();
        assert!((area(&u, 0.01).abs() - (175.0 + 49.0 - 4.0)).abs() < 1e-9);
        let p = punch_paths(&[a.clone(), b, c]).unwrap();
        assert_eq!(p.len(), 2);
        assert!((area(&p[0], 0.01).abs() - 75.0).abs() < 1e-9);
        assert!((area(&p[1], 0.01).abs() - 45.0).abs() < 1e-9);
        assert!(union_paths(&[a]).is_none());
    }
}
