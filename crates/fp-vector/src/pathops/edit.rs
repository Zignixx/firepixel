//! Weitere Pfadbefehle (Spezifikation 16 §3.1, §3.5–§3.7): Verbinden, Teilen, Richtung umkehren,
//! Punkte einfügen/entfernen, Freiform (Push/Pull) und Bereich umformen (Gravitation).

use super::geom::{Contour, Node};
use fp_geom::{CubicBez, Vec2};

/// Ende einer offenen Kontur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum End {
    /// Erster Knoten.
    First,
    /// Letzter Knoten.
    Last,
}

/// Ergebnis von **Verbinden** (`joinPaths`, `@00459ab0`).
#[derive(Debug, Clone, PartialEq)]
pub enum JoinResult {
    /// Eine einzelne offene Kontur wurde geschlossen.
    Closed(Contour),
    /// Zwei offene Konturen wurden an den gewählten Enden zu einer verbunden.
    Connected(Contour),
    /// Alle Konturen aller Pfade in einem zusammengesetzten Pfad (Reihenfolge wie übergeben).
    Compound(Vec<Contour>),
}

fn oriented(c: &Contour, sel: End, want_last: bool) -> Contour {
    let is_last = sel == End::Last;
    if is_last == want_last { c.clone() } else { c.reversed() }
}

/// **Verbinden** (16 §3.1, B `@0072e9b0`, Laufzeittest T16):
/// - ein Pfad mit genau einer offenen Kontur → geschlossen;
/// - zwei Pfade mit je einer offenen Kontur **und** unterausgewählten Enden (`ends`, je Pfad das
///   gewählte Ende; oberster Pfad zuerst) → eine offene Kontur: oberer Pfad so ausgerichtet, dass sein
///   gewähltes Ende zuletzt kommt, dann der untere mit seinem gewählten Ende zuerst (T16 (b)/(c));
/// - sonst (auch Zeigerauswahl ohne Endpunkt-Flags) ein zusammengesetzter Pfad.
pub fn join_paths(paths: &[Vec<Contour>], ends: Option<[End; 2]>) -> JoinResult {
    if paths.len() == 1 && paths[0].len() == 1 && !paths[0][0].closed {
        let mut c = paths[0][0].clone();
        c.closed = true;
        return JoinResult::Closed(c);
    }
    if paths.len() == 2
        && paths.iter().all(|p| p.len() == 1 && !p[0].closed)
        && let Some([e_top, e_bottom]) = ends
    {
        let top = oriented(&paths[0][0], e_top, true);
        let bottom = oriented(&paths[1][0], e_bottom, false);
        let mut nodes = top.nodes;
        nodes.extend(bottom.nodes);
        return JoinResult::Connected(Contour::new(nodes, false));
    }
    JoinResult::Compound(paths.iter().flat_map(|p| p.iter().cloned()).collect())
}

/// **Teilen** (`splitPaths`, `@0045a8e0`): je Kontur ein eigener Pfad mit den Attributen der
/// Quelle; Schleife von der letzten zur ersten Kontur (Rückgabe in dieser Reihenfolge).
pub fn split_path(contours: &[Contour]) -> Vec<Contour> {
    contours.iter().rev().cloned().collect()
}

/// Punkt einfügen: Segment `seg` bei `t` per de Casteljau teilen (Geraden linear; Form bleibt).
pub fn insert_node(c: &Contour, seg: usize, t: f64) -> Option<Contour> {
    let segs = c.segments();
    let s = segs.get(seg)?;
    let n = c.nodes.len();
    let (i, j) = (seg, (seg + 1) % n);
    let (l, r) = s.split(t);
    let mut nodes = c.nodes.clone();
    nodes[i].ctrl_out = l.p1;
    nodes[j].ctrl_in = r.p2;
    nodes.insert(i + 1, Node::new(l.p3, l.p2, r.p1));
    Some(Contour::new(nodes, c.closed))
}

/// Punkt entfernen: Knoten löschen, Nachbarn behalten ihre Henkel (A).
pub fn remove_node(c: &Contour, index: usize) -> Contour {
    let nodes: Vec<Node> = c.nodes.iter().enumerate().filter(|(k, _)| *k != index).map(|(_, n)| *n).collect();
    let closed = c.closed && nodes.len() > 2;
    Contour::new(nodes, closed)
}

/// Lange Segmente für die Freiform-Werkzeuge unterteilen: Schrittweite `step` px,
/// t-Schritte `1/ceil(Länge/Schritt)`, mindestens 1/128, höchstens bis 0,98 (16 §3.7).
pub fn subdivide(c: &Contour, step: f64) -> Contour {
    let segs = c.segments();
    if segs.is_empty() || step <= 0.0 {
        return c.clone();
    }
    let mut pieces: Vec<CubicBez> = Vec::new();
    for s in &segs {
        let len = s.length();
        let parts = (len / step - 1e-9).ceil().max(1.0);
        let dt = (1.0 / parts).max(1.0 / 128.0);
        let mut cur = *s;
        let mut consumed = 0.0;
        let mut t = dt;
        // Teilpunkte t = dt, 2·dt, … (< 1, höchstens 0,98)
        while t <= 0.98 + 1e-12 {
            let (l, r) = cur.split((t - consumed) / (1.0 - consumed));
            pieces.push(l);
            cur = r;
            consumed = t;
            t += dt;
        }
        pieces.push(cur);
    }
    let m = pieces.len();
    let n0 = c.nodes.len();
    let mut nodes: Vec<Node> = (0..m)
        .map(|k| {
            let cin = if k > 0 || c.closed { pieces[(k + m - 1) % m].p2 } else { c.nodes[0].ctrl_in };
            Node::new(pieces[k].p0, cin, pieces[k].p1)
        })
        .collect();
    if !c.closed {
        let last = pieces[m - 1];
        nodes.push(Node::new(last.p3, last.p2, c.nodes[n0 - 1].ctrl_out));
    }
    Contour::new(nodes, c.closed)
}

fn move_node(n: &mut Node, d: Vec2) {
    n.anchor += d;
    n.ctrl_in += d;
    n.ctrl_out += d;
}

/// **Freiform „Push/Pull“** (`@00d3c5c0`): `R = max(1, Größe·0,5)`; vorher unterteilen
/// (Schritt `clamp(Größe; 0,5; 8)`); jeder Knoten mit Abstand `d ≤ R` zum Cursor wird auf den
/// Kreisrand geschoben (`p′ = c − (c − p)·R/d`); Verschiebungen < 0,1 px entfallen; liegt `p` genau
/// auf `c`, wird um 0,0001 versetzt. Henkel wandern mit dem Anker.
pub fn push_pull(contours: &[Contour], cursor: Vec2, size: f64) -> Vec<Contour> {
    let size = size.clamp(1.0, 1000.0);
    let r = (size * 0.5).max(1.0);
    let step = size.clamp(0.5, 8.0);
    contours
        .iter()
        .map(|c| {
            let mut c = subdivide(c, step);
            for n in &mut c.nodes {
                let mut p = n.anchor;
                if p == cursor {
                    p.x += 0.0001;
                }
                let d = p.distance(cursor);
                if d <= r && d > 0.0 {
                    let np = cursor - (cursor - p) * (r / d);
                    let mv = np - n.anchor;
                    if mv.length() >= 0.1 {
                        move_node(n, mv);
                    }
                }
            }
            c
        })
        .collect()
}

/// Stärke der Gravitation aus der Präferenz `FreeFormGravPower` (Vorgabe 80 → 2,82):
/// `p = Pref·3,4/100 + 0,1`, begrenzt auf [0,1; 3,5] (B `@00d3d850`).
pub fn gravity_power(pref: f64) -> f64 {
    (pref * 3.4 / 100.0 + 0.1).clamp(0.1, 3.5)
}

/// **Bereich umformen** (Gravitation, `@00d3bbd0`): `R = max(1, Durchmesser·0,5)` (Durchmesser
/// auf [1; 1000], Vorgabe 72), Unterteilung `clamp(0,2·R; 0,5; 8)`; Knoten mit `d < R` werden um
/// `Δ·(1 − (d/R)^p)` verschoben (B `@00d3bff0`). Henkel wandern mit dem Anker.
pub fn reshape_area(contours: &[Contour], center: Vec2, delta: Vec2, diameter: f64, power: f64) -> Vec<Contour> {
    let r = (diameter.clamp(1.0, 1000.0) * 0.5).max(1.0);
    let step = (0.2 * r).clamp(0.5, 8.0);
    contours
        .iter()
        .map(|c| {
            let mut c = subdivide(c, step);
            for n in &mut c.nodes {
                let d = n.anchor.distance(center);
                if d < r {
                    let w = 1.0 - (d / r).powf(power);
                    move_node(n, delta * w);
                }
            }
            c
        })
        .collect()
}

/// Segmentlänge (Gauß-Legendre) aller Segmente einer Kontur.
pub fn contour_length(c: &Contour) -> f64 {
    c.segments().iter().map(CubicBez::length).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(a: (f64, f64), b: (f64, f64)) -> Contour {
        Contour::polygon(&[Vec2::new(a.0, a.1), Vec2::new(b.0, b.1)], false)
    }

    #[test]
    fn join_like_t16() {
        let a = line((20.0, 50.0), (90.0, 50.0));
        let b = line((110.0, 80.0), (180.0, 80.0));
        // (b) A-Ende + B-Anfang: 180,80 → 110,80 → 90,50 → 20,50 (B oben)
        let JoinResult::Connected(c) = join_paths(&[vec![b.clone()], vec![a.clone()]], Some([End::First, End::Last])) else { panic!() };
        let xs: Vec<f64> = c.anchors().iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![180.0, 110.0, 90.0, 20.0]);
        // (c) A-Anfang + B-Ende: 110,80 → 180,80 → 20,50 → 90,50
        let JoinResult::Connected(c) = join_paths(&[vec![b.clone()], vec![a.clone()]], Some([End::Last, End::First])) else { panic!() };
        let xs: Vec<f64> = c.anchors().iter().map(|p| p.x).collect();
        assert_eq!(xs, vec![110.0, 180.0, 20.0, 90.0]);
        // (a) Zeigerauswahl: zusammengesetzter Pfad
        assert!(matches!(join_paths(&[vec![b], vec![a.clone()]], None), JoinResult::Compound(v) if v.len() == 2));
        assert!(matches!(join_paths(&[vec![a]], None), JoinResult::Closed(c) if c.closed));
    }

    #[test]
    fn subdivide_keeps_shape() {
        let c = line((0.0, 0.0), (10.0, 0.0));
        let s = subdivide(&c, 2.0);
        assert_eq!(s.nodes.len(), 6);
        assert!((contour_length(&s) - 10.0).abs() < 1e-9);
        let r = reshape_area(&[c], Vec2::new(5.0, 0.0), Vec2::new(0.0, 3.0), 72.0, gravity_power(80.0));
        assert!(r[0].nodes.iter().all(|n| n.anchor.y > 0.0));
        let p = push_pull(&[line((0.0, 0.0), (10.0, 0.0))], Vec2::new(5.0, 1.0), 10.0);
        assert!(p[0].nodes.iter().all(|n| n.anchor.distance(Vec2::new(5.0, 1.0)) >= 4.999));
    }
}
