//! Flächen und gemeinsame Randketten.
//!
//! 1. Zusammenhangskomponenten (4er-Nachbarschaft) je Palettenfarbe; Flecken unter der
//!    Mindestfläche übernehmen die Farbe des Nachbarn mit der längsten gemeinsamen Kante.
//! 2. Umrisse je Fläche entlang der Pixelkanten (Fläche rechts in Laufrichtung: Außenumrisse im
//!    Uhrzeigersinn, Löcher dagegen).
//! 3. **Randketten:** Ein Umriss wird an Knotenpunkten (Pixelecken, an denen drei oder vier Flächen
//!    zusammenstoßen) geteilt. Jede Kette trennt genau zwei Flächen und wird nur **einmal**
//!    angepasst; der Nachbar benutzt dieselbe Kette rückwärts. Benachbarte Flächen haben dadurch
//!    exakt dieselbe Kante – keine Haarrisse, keine Überlappung.

use super::NONE;
use super::fit::V;
use std::collections::HashMap;

pub(crate) struct Regions {
    /// Flächennummer je Pixel oder [`NONE`].
    pub comp: Vec<u32>,
    /// Palettennummer je Fläche.
    pub color: Vec<u32>,
    /// Pixel je Fläche.
    pub size: Vec<usize>,
}

pub(crate) fn components(label: &[u32], w: usize, h: usize) -> Regions {
    let n = w * h;
    let mut comp = vec![NONE; n];
    let mut color = Vec::new();
    let mut size = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for s in 0..n {
        if label[s] == NONE || comp[s] != NONE {
            continue;
        }
        let id = color.len() as u32;
        let l = label[s];
        comp[s] = id;
        stack.push(s);
        let mut cnt = 0usize;
        while let Some(i) = stack.pop() {
            cnt += 1;
            let (x, y) = (i % w, i / w);
            let mut visit = |j: usize, comp: &mut Vec<u32>| {
                if comp[j] == NONE && label[j] == l {
                    comp[j] = id;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1, &mut comp);
            }
            if x + 1 < w {
                visit(i + 1, &mut comp);
            }
            if y > 0 {
                visit(i - w, &mut comp);
            }
            if y + 1 < h {
                visit(i + w, &mut comp);
            }
        }
        color.push(l);
        size.push(cnt);
    }
    Regions { comp, color, size }
}

/// Flecken unter `min_area` Pixeln bekommen die Farbe des Nachbarn mit der längsten gemeinsamen
/// Kante (nur transparente Nachbarn → transparent; gar keine Nachbarn → bleibt).
pub(crate) fn merge_speckles(label: &mut [u32], w: usize, h: usize, min_area: usize) {
    if min_area <= 1 {
        return;
    }
    for _ in 0..2 {
        let reg = components(label, w, h);
        let small: Vec<bool> = reg.size.iter().map(|&s| s < min_area).collect();
        if !small.iter().any(|&b| b) {
            return;
        }
        let mut counts: HashMap<u32, HashMap<u32, usize>> = HashMap::new();
        for i in 0..w * h {
            let c = reg.comp[i];
            if c == NONE || !small[c as usize] {
                continue;
            }
            let (x, y) = (i % w, i / w);
            let mut nb = |j: usize| {
                if reg.comp[j] != c {
                    *counts.entry(c).or_default().entry(label[j]).or_default() += 1;
                }
            };
            if x > 0 {
                nb(i - 1);
            }
            if x + 1 < w {
                nb(i + 1);
            }
            if y > 0 {
                nb(i - w);
            }
            if y + 1 < h {
                nb(i + w);
            }
        }
        if counts.is_empty() {
            return;
        }
        let new: HashMap<u32, u32> = counts
            .into_iter()
            .map(|(c, m)| {
                (
                    c,
                    m.into_iter()
                        .max_by_key(|&(l, k)| (k, l != NONE, u32::MAX - l))
                        .map(|v| v.0)
                        .unwrap_or(NONE),
                )
            })
            .collect();
        for (l, c) in label.iter_mut().zip(&reg.comp) {
            if let Some(&nl) = new.get(c) {
                *l = nl;
            }
        }
    }
}

/// Umriss einer Fläche (Pixelecken in Einheitsschritten).
pub(crate) struct Contour {
    pub region: u32,
    pub corners: Vec<(i32, i32)>,
    /// Doppelte Fläche mit Vorzeichen (> 0 außen, < 0 Loch).
    pub area2: i64,
    /// Richtungswechsel = Eckpunkte des Treppenumrisses.
    pub turns: usize,
}

const DX: [i32; 4] = [1, 0, -1, 0];
const DY: [i32; 4] = [0, 1, 0, -1];

pub(crate) fn trace_contours(comp: &[u32], w: usize, h: usize) -> Vec<Contour> {
    let lab = |x: i32, y: i32| -> u32 {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            NONE
        } else {
            comp[y as usize * w + x as usize]
        }
    };
    let mut visited = vec![0u8; w * h];
    let mut out = Vec::new();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let r = comp[i];
            if r == NONE {
                continue;
            }
            let (xi, yi) = (x as i32, y as i32);
            for (bit, sx, sy, d, nx, ny) in [
                (1u8, xi, yi, 0usize, xi, yi - 1),
                (2, xi + 1, yi, 1, xi + 1, yi),
                (4, xi + 1, yi + 1, 2, xi, yi + 1),
                (8, xi, yi + 1, 3, xi - 1, yi),
            ] {
                if visited[i] & bit != 0 || lab(nx, ny) == r {
                    continue;
                }
                let mut corners = Vec::new();
                let (mut cx, mut cy, mut dir) = (sx, sy, d);
                let (mut area2, mut turns) = (0i64, 0usize);
                loop {
                    let (px, py, b) = match dir {
                        0 => (cx, cy, 1u8),
                        1 => (cx - 1, cy, 2),
                        2 => (cx - 1, cy - 1, 4),
                        _ => (cx, cy - 1, 8),
                    };
                    visited[py as usize * w + px as usize] |= b;
                    corners.push((cx, cy));
                    let (nx2, ny2) = (cx + DX[dir], cy + DY[dir]);
                    area2 += cx as i64 * ny2 as i64 - nx2 as i64 * cy as i64;
                    cx = nx2;
                    cy = ny2;
                    let (al, ar) = match dir {
                        0 => ((cx, cy - 1), (cx, cy)),
                        1 => ((cx, cy), (cx - 1, cy)),
                        2 => ((cx - 1, cy), (cx - 1, cy - 1)),
                        _ => ((cx - 1, cy - 1), (cx, cy - 1)),
                    };
                    let nd = if lab(ar.0, ar.1) == r {
                        if lab(al.0, al.1) == r {
                            (dir + 3) % 4
                        } else {
                            dir
                        }
                    } else {
                        (dir + 1) % 4
                    };
                    if nd != dir {
                        turns += 1;
                    }
                    dir = nd;
                    if cx == sx && cy == sy && dir == d {
                        break;
                    }
                }
                out.push(Contour {
                    region: r,
                    corners,
                    area2,
                    turns,
                });
            }
        }
    }
    out
}

/// Gemeinsame Randkette.
pub(crate) struct Chain {
    /// Pixelecken; offen: von Knotenpunkt zu Knotenpunkt (beide enthalten), Ring: ohne Wiederholung.
    pub pts: Vec<(i32, i32)>,
    pub closed: bool,
    /// Anzahl der Umrisse, die die Kette benutzen (1 oder 2).
    pub uses: u8,
}

impl Chain {
    /// Stichproben für die Anpassung: Mitten der Pixelkanten (offen: dazu die festen Endpunkte).
    pub fn samples(&self) -> Vec<V> {
        let p = |q: (i32, i32)| V::new(q.0 as f64, q.1 as f64);
        let n = self.pts.len();
        if self.closed {
            (0..n)
                .map(|i| p(self.pts[i]).mid(p(self.pts[(i + 1) % n])))
                .collect()
        } else {
            let mut v = Vec::with_capacity(n + 1);
            v.push(p(self.pts[0]));
            for i in 0..n - 1 {
                v.push(p(self.pts[i]).mid(p(self.pts[i + 1])));
            }
            v.push(p(self.pts[n - 1]));
            v
        }
    }
}

/// Gerichtete Pixelkante (von, nach).
pub(crate) type EdgeKey = ((i32, i32), (i32, i32));

/// Umriss als Folge von (Kette, rückwärts).
pub(crate) struct ContourRef {
    pub region: u32,
    pub uses: Vec<(usize, bool)>,
    pub area2: i64,
    /// Index in der Umrissliste (für Punkt-in-Polygon).
    pub contour: usize,
}

/// Teilt die Umrisse in Ketten. `share = false`: jede Fläche bekommt eigene Ketten (keine
/// gemeinsamen Kanten).
pub(crate) fn build_chains(
    contours: &[Contour],
    comp: &[u32],
    w: usize,
    h: usize,
    share: bool,
) -> (Vec<Chain>, Vec<ContourRef>) {
    let lab = |x: i32, y: i32| -> u32 {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            NONE
        } else {
            comp[y as usize * w + x as usize]
        }
    };
    let junction = |x: i32, y: i32| -> bool {
        let (nw, ne, sw, se) = (lab(x - 1, y - 1), lab(x, y - 1), lab(x - 1, y), lab(x, y));
        u8::from(nw != ne) + u8::from(sw != se) + u8::from(nw != sw) + u8::from(ne != se) >= 3
    };
    let mut chains: Vec<Chain> = Vec::new();
    let mut map: HashMap<EdgeKey, usize> = HashMap::new();
    let mut refs = Vec::with_capacity(contours.len());
    for (ci, c) in contours.iter().enumerate() {
        let n = c.corners.len();
        let js: Vec<usize> = (0..n)
            .filter(|&i| junction(c.corners[i].0, c.corners[i].1))
            .collect();
        let mut uses = Vec::new();
        let mut add = |pts: Vec<(i32, i32)>,
                       closed: bool,
                       key_fwd,
                       key_rev,
                       uses: &mut Vec<(usize, bool)>| {
            if share && let Some(&id) = map.get(&key_rev) {
                chains[id].uses += 1;
                uses.push((id, true));
                return;
            }
            let id = chains.len();
            map.insert(key_fwd, id);
            chains.push(Chain {
                pts,
                closed,
                uses: 1,
            });
            uses.push((id, false));
        };
        if js.is_empty() {
            let k = (0..n)
                .min_by_key(|&i| (c.corners[i].1, c.corners[i].0))
                .unwrap_or(0);
            let pts: Vec<(i32, i32)> = (0..n).map(|i| c.corners[(k + i) % n]).collect();
            let (kf, kr) = ((pts[0], pts[1 % n]), (pts[0], pts[n - 1]));
            add(pts, true, kf, kr, &mut uses);
        } else {
            for t in 0..js.len() {
                let (s, e) = (js[t], js[(t + 1) % js.len()]);
                let len = if e > s { e - s } else { e + n - s };
                let pts: Vec<(i32, i32)> = (0..=len).map(|i| c.corners[(s + i) % n]).collect();
                let m = pts.len();
                let (kf, kr) = ((pts[0], pts[1]), (pts[m - 1], pts[m - 2]));
                add(pts, false, kf, kr, &mut uses);
            }
        }
        refs.push(ContourRef {
            region: c.region,
            uses,
            area2: c.area2,
            contour: ci,
        });
    }
    (chains, refs)
}

/// Punkt in Polygon (gerade/ungerade).
pub(crate) fn inside(poly: &[(i32, i32)], x: f64, y: f64) -> bool {
    let n = poly.len();
    let mut ins = false;
    for i in 0..n {
        let (x0, y0) = (poly[i].0 as f64, poly[i].1 as f64);
        let (x1, y1) = (poly[(i + 1) % n].0 as f64, poly[(i + 1) % n].1 as f64);
        if (y0 > y) != (y1 > y) && x < x0 + (y - y0) * (x1 - x0) / (y1 - y0) {
            ins = !ins;
        }
    }
    ins
}
