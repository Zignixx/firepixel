//! Slices und Tabellenraster (08 §6.1, §6.2, §6.4; Referenz `re\export\fw_slices.py`, Laufzeit T5
//! 13/13 Zellen).
//!
//! 1. Schnittlinien je Achse: Dokumentränder plus Kanten jeder Slice (Rechteck-Slices `_ftol` = Richtung 0,
//!    Polygon-Slices Hülle nach außen gerundet), nur echt innerhalb, sortiert, eindeutig.
//! 2. Jede Zelle merkt sich bis zu 8 überdeckende Slices in Stapelreihenfolge.
//! 3. Verschmelzen von links oben: erst nach rechts, dann zeilenweise nach unten (Vergleich über die
//!    oberste Slice); leere Bereiche werden „undefinierte“ Zellen.
//! 4. Nur eine Zelle ⇒ Raster 1×1.

use fp_geom::Rect;
use fp_model::{Document, ElementKind, LayerKind, LayerSlot, WebShape};

/// Eine Slice für den Rasterbau.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceSpec {
    /// Rechteck (links, oben, rechts, unten) in Dokumentkoordinaten.
    pub rect: [f32; 4],
    /// Kennung (Slice-ID `MSNi` bzw. laufende Nummer).
    pub id: u32,
    /// Polygon-Slice (Hülle nach außen runden).
    pub poly: bool,
    /// Eigener Basisname (`FILs`).
    pub base_name: Option<String>,
    /// Link, Alt-Text, Ziel.
    pub url: String,
    pub alt: String,
    pub target: String,
    /// Slice-Art (`TSLi`): 0 Bild, 2 Hintergrund, 3 HTML.
    pub kind: i32,
    /// HTML-Text einer HTML-Slice.
    pub html: String,
    /// Polygonpunkte (Dokumentkoordinaten) für Imagemaps.
    pub polygon: Vec<(f32, f32)>,
    /// Verhalten der Slice.
    pub behaviors: Vec<fp_model::Behavior>,
}

/// Eine Zelle des Rasters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    /// Überdeckende Slices (IDs, oberste zuerst, höchstens 8).
    pub slices: Vec<u32>,
    pub colspan: usize,
    pub rowspan: usize,
    /// Von einer anderen Zelle überdeckt (`skipCell`).
    pub skip: bool,
    /// Ohne Slice (`isUndefined`).
    pub undefined: bool,
    /// Lage relativ zum Raster und Größe der Grundzelle.
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Cell {
    /// Oberste Slice.
    pub fn slice(&self) -> Option<u32> {
        self.slices.first().copied()
    }
}

/// Tabellenraster.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    /// Spaltengrenzen (x) und Zeilengrenzen (y) in Dokumentkoordinaten.
    pub xs: Vec<i32>,
    pub ys: Vec<i32>,
    /// `cells[zeile][spalte]`.
    pub cells: Vec<Vec<Cell>>,
}

impl Grid {
    pub fn rows(&self) -> usize {
        self.cells.len()
    }
    pub fn cols(&self) -> usize {
        self.cells.first().map(|r| r.len()).unwrap_or(0)
    }
    /// Spaltenbreiten.
    pub fn col_widths(&self) -> Vec<i32> {
        self.xs.windows(2).map(|w| w[1] - w[0]).collect()
    }
    /// Zeilenhöhen.
    pub fn row_heights(&self) -> Vec<i32> {
        self.ys.windows(2).map(|w| w[1] - w[0]).collect()
    }
    /// Breite/Höhe einer Ursprungszelle mit Spans.
    pub fn span_size(&self, r: usize, c: usize) -> (i32, i32) {
        let cell = &self.cells[r][c];
        let w = self.xs[(c + cell.colspan).min(self.xs.len() - 1)] - self.xs[c];
        let h = self.ys[(r + cell.rowspan).min(self.ys.len() - 1)] - self.ys[r];
        (w, h)
    }
    /// Zeile/Spalte der linken oberen Zelle einer Slice.
    pub fn origin_of(&self, id: u32) -> Option<(usize, usize)> {
        for (r, row) in self.cells.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if !cell.skip && cell.slice() == Some(id) {
                    return Some((r, c));
                }
            }
        }
        None
    }
}

fn slice_rect(s: &SliceSpec) -> [i32; 4] {
    if s.poly {
        [s.rect[0].floor() as i32, s.rect[1].floor() as i32, s.rect[2].ceil() as i32, s.rect[3].ceil() as i32]
    } else {
        s.rect.map(|v| v as i32)
    }
}

/// Baut das Raster (`@00562dc0`/`@00561700`). `doc` = (x0, y0, x1, y1); `slices` oberste zuerst.
pub fn build_grid(doc: [f32; 4], slices: &[SliceSpec]) -> Grid {
    let [x0, y0, x1, y1] = doc.map(|v| v as i32);
    let mut xs = vec![x0, x1];
    let mut ys = vec![y0, y1];
    for s in slices {
        let [l, t, r, b] = slice_rect(s);
        for v in [l, r] {
            if x0 < v && v < x1 && !xs.contains(&v) && xs.len() < 396 {
                xs.push(v);
            }
        }
        for v in [t, b] {
            if y0 < v && v < y1 && !ys.contains(&v) && ys.len() < 396 {
                ys.push(v);
            }
        }
    }
    xs.sort_unstable();
    ys.sort_unstable();
    let (nr, nc) = (ys.len().saturating_sub(1), xs.len().saturating_sub(1));
    let mut cells: Vec<Vec<Cell>> = (0..nr)
        .map(|_| {
            (0..nc)
                .map(|_| Cell { slices: vec![], colspan: 1, rowspan: 1, skip: false, undefined: true, left: 0, top: 0, width: 0, height: 0 })
                .collect()
        })
        .collect();
    for s in slices {
        let [l, t, r, b] = slice_rect(s);
        for i in 0..nr {
            if !(ys[i] < b && ys[i + 1] > t) {
                continue;
            }
            for j in 0..nc {
                if xs[j] < r && xs[j + 1] > l && cells[i][j].slices.len() < 8 {
                    cells[i][j].slices.push(s.id);
                }
            }
        }
    }
    let same = |a: &Cell, b: &Cell| a.slices.first() == b.slices.first();
    for i in 0..nr {
        for j in 0..nc {
            if cells[i][j].skip {
                continue;
            }
            let mut je = j;
            while je + 1 < nc && !cells[i][je + 1].skip && same(&cells[i][j], &cells[i][je + 1]) {
                je += 1;
                cells[i][je].skip = true;
                cells[i][je].colspan = 0;
                cells[i][je].rowspan = 0;
            }
            cells[i][j].colspan = je - j + 1;
            let mut ie = i;
            while ie + 1 < nr && (j..=je).all(|k| !cells[ie + 1][k].skip && same(&cells[i][j], &cells[ie + 1][k])) {
                ie += 1;
                for cell in &mut cells[ie][j..=je] {
                    cell.skip = true;
                    cell.colspan = 0;
                    cell.rowspan = 0;
                }
            }
            cells[i][j].rowspan = ie - i + 1;
        }
    }
    if nr > 0 && nc > 0 && cells[0][0].rowspan == nr && cells[0][0].colspan == nc {
        xs = vec![xs[0], xs[xs.len() - 1]];
        ys = vec![ys[0], ys[ys.len() - 1]];
        let mut c = cells[0][0].clone();
        c.rowspan = 1;
        c.colspan = 1;
        cells = vec![vec![c]];
    }
    for (i, row) in cells.iter_mut().enumerate() {
        for (j, c) in row.iter_mut().enumerate() {
            c.undefined = c.slices.is_empty();
            c.left = xs[j] - xs[0];
            c.top = ys[i] - ys[0];
            c.width = xs[j + 1] - xs[j];
            c.height = ys[i + 1] - ys[i];
        }
    }
    Grid { xs, ys, cells }
}

/// Datum für die Namensbausteine 8–14.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameDate {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Slice-Dateiname aus bis zu 6 Bausteinen (`NM4i`, 08 §6.4); `row`/`col` 1-basiert.
pub fn auto_name(tokens: &[i32], row: usize, col: usize, doc_name: &str, seq: usize, total: usize, date: NameDate) -> String {
    let yy = date.year.rem_euclid(100);
    let mut out = String::new();
    for &k in tokens {
        match k {
            1 => out.push_str(&format!("r{row}_c{col}")),
            2 | 3 => {
                let base = if k == 2 { b'A' } else { b'a' };
                let mut n = seq.saturating_sub(1);
                let mut s = Vec::new();
                loop {
                    s.push(base + (n % 26) as u8);
                    n /= 26;
                    if n == 0 {
                        break;
                    }
                }
                s.reverse();
                out.push_str(&String::from_utf8_lossy(&s));
            }
            4 => out.push_str(&seq.to_string()),
            5 => out.push_str(&if total < 99 { format!("{seq:02}") } else { format!("{seq:03}") }),
            6 => out.push_str(doc_name),
            7 => out.push_str("slice"),
            8 => out.push_str(&format!("{}{}{}", date.month, date.day, yy)),
            9 => out.push_str(&format!("{}{}", date.month, date.day)),
            10 => out.push_str(&format!("{}{}{}", date.day, date.month, yy)),
            11 => out.push_str(&format!("{}{}", date.day, date.month)),
            12 => out.push_str(&format!("{}{}{}", date.year, date.month, date.day)),
            13 => out.push_str(&format!("{}{}{}", yy, date.month, date.day)),
            14 => out.push_str(&format!("{}{}{}", yy, date.day, date.month)),
            15 => out.push('_'),
            16 => out.push('.'),
            17 => out.push(' '),
            18 => out.push('-'),
            _ => {}
        }
    }
    out.chars().take(240).collect()
}

/// Status-Suffix (`NMFi`, 2 Bausteine), `frame` 1-basiert; Vorgabe `[5, 1]` ⇒ „_s2“ (gemessen,
/// re\18 §3 und §6.4).
pub fn frame_suffix(tokens: &[i32], frame: usize, state_names: &[String; 4]) -> String {
    frame_suffix_with(tokens, frame, state_names, 's')
}

/// Wie [`frame_suffix`], Buchstabe des Bausteins „Status-Nr.“ frei wählbar (der Exportdialog
/// schreibt gemessen „_s2“, re\18 §3).
pub fn frame_suffix_with(tokens: &[i32], frame: usize, state_names: &[String; 4], letter: char) -> String {
    let mut out = String::new();
    for &k in tokens {
        match k {
            1 => out.push_str(&format!("{letter}{frame}")),
            2 => out.push_str(&frame.to_string()),
            // Rollover (over, down, overdown) und abgekürzt (o, d, od), re\18 §6.4
            3 => out.push_str(["", "over", "down", "overdown"][(frame.max(1) - 1).min(3)]),
            4 => out.push_str(["", "o", "d", "od"][(frame.max(1) - 1).min(3)]),
            // Benutzerdefiniert: eigene Statusnamen (Up, Over, Down, OverDown)
            9 => out.push_str(&state_names[(frame.max(1) - 1).min(3)]),
            5 => out.push('_'),
            6 => out.push('.'),
            7 => out.push(' '),
            8 => out.push('-'),
            _ => {}
        }
    }
    out
}

/// Slices der Web-Ebene einer Seite (oberste zuerst). Slices ohne `MSNi` bekommen laufende IDs ab 1.
pub fn collect_slices(doc: &Document, page: usize) -> Vec<SliceSpec> {
    let mut out = Vec::new();
    let Some(p) = doc.pages.get(page) else { return out };
    let mut next = 1u32;
    for slot in p.root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else { continue };
            let Some(sp) = &w.slice else { continue };
            if !e.common.visible {
                continue;
            }
            let poly = w.shape == WebShape::Polygon;
            let polygon: Vec<(f32, f32)> = w.polygon.as_ref().map(|c| c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()).unwrap_or_default();
            let rect = if poly && !polygon.is_empty() { hull(&polygon) } else { w.bounds };
            let id = sp.slice_id.unwrap_or_else(|| {
                next += 1;
                next - 1
            });
            out.push(SliceSpec {
                rect: [rect.left, rect.top, rect.right, rect.bottom],
                id,
                poly,
                base_name: w.file_name.clone().filter(|s| !s.is_empty()),
                url: w.url.clone().unwrap_or_default(),
                alt: w.alt_text.clone().unwrap_or_default(),
                target: w.target.clone().unwrap_or_default(),
                kind: sp.slice_type,
                html: sp.html.clone().unwrap_or_default(),
                polygon,
                behaviors: w.behaviors.as_ref().map(|b| b.items.clone()).unwrap_or_default(),
            });
        }
    }
    out
}

/// Ein Hotspot: (Form, Rechteck, Polygon, URL, Alt, Ziel, Verhalten).
pub type Hotspot = (WebShape, Rect, Vec<(f32, f32)>, String, String, String, Vec<fp_model::Behavior>);

/// Hotspots (keine Slices) der Web-Ebene.
pub fn collect_hotspots(doc: &Document, page: usize) -> Vec<Hotspot> {
    let mut out = Vec::new();
    let Some(p) = doc.pages.get(page) else { return out };
    for slot in p.root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else { continue };
            if w.slice.is_some() || !e.common.visible {
                continue;
            }
            let polygon: Vec<(f32, f32)> = w.polygon.as_ref().map(|c| c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect()).unwrap_or_default();
            out.push((
                w.shape,
                w.bounds,
                polygon,
                w.url.clone().unwrap_or_default(),
                w.alt_text.clone().unwrap_or_default(),
                w.target.clone().unwrap_or_default(),
                w.behaviors.as_ref().map(|b| b.items.clone()).unwrap_or_default(),
            ));
        }
    }
    out
}

fn hull(pts: &[(f32, f32)]) -> Rect {
    let mut r = Rect::new(f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for &(x, y) in pts {
        r.left = r.left.min(x);
        r.top = r.top.min(y);
        r.right = r.right.max(x);
        r.bottom = r.bottom.max(y);
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(rect: [f32; 4], id: u32, poly: bool) -> SliceSpec {
        SliceSpec {
            rect,
            id,
            poly,
            base_name: None,
            url: String::new(),
            alt: String::new(),
            target: String::new(),
            kind: 0,
            html: String::new(),
            polygon: vec![],
            behaviors: vec![],
        }
    }

    #[test]
    fn t5_grid_like_original() {
        // Laufzeit 08 T5: FW-Spalten [15,70,15,10,50,20,20], Zeilen [10,10,30,10,20,40,30], 13 Zellen.
        let s = vec![spec([100.0, 60.0, 160.0, 120.0], 1, false), spec([110.0, 10.0, 180.0, 50.0], 2, false), spec([15.358984, 20.0, 84.641014, 80.0], 3, true)];
        let g = build_grid([0.0, 0.0, 200.0, 150.0], &s);
        assert_eq!(g.col_widths(), vec![15, 70, 15, 10, 50, 20, 20]);
        assert_eq!(g.row_heights(), vec![10, 10, 30, 10, 20, 40, 30]);
        let live: usize = g.cells.iter().flatten().filter(|c| !c.skip).count();
        assert_eq!(live, 13);
        let d = NameDate { year: 2026, month: 10, day: 11 };
        assert_eq!(auto_name(&[6, 15, 1, 0, 0, 0], 3, 2, "slices_spacer", 1, 1, d), "slices_spacer_r3_c2");
        assert_eq!(auto_name(&[2], 0, 0, "", 28, 30, d), "BB"); // wie fw_slices.auto_name
        assert_eq!(auto_name(&[8], 0, 0, "", 1, 1, d), "101126");
        let names = [String::new(), "_o".into(), "_d".into(), "_od".into()];
        assert_eq!(frame_suffix(&[5, 1], 2, &names), "_s2");
        assert_eq!(frame_suffix(&[5, 3], 2, &names), "_over");
        assert_eq!(frame_suffix(&[5, 4], 4, &names), "_od");
        assert_eq!(frame_suffix(&[9, 0], 2, &names), names[1]);
        let one = build_grid([0.0, 0.0, 50.0, 50.0], &[]);
        assert_eq!((one.rows(), one.cols()), (1, 1));
    }
}
