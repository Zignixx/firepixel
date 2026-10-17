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
    /// Quellelement (Web-Objekt bzw. Schaltflächen-Instanz), sofern bekannt.
    pub element: Option<fp_model::ElementId>,
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
        [
            s.rect[0].floor() as i32,
            s.rect[1].floor() as i32,
            s.rect[2].ceil() as i32,
            s.rect[3].ceil() as i32,
        ]
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
                .map(|_| Cell {
                    slices: vec![],
                    colspan: 1,
                    rowspan: 1,
                    skip: false,
                    undefined: true,
                    left: 0,
                    top: 0,
                    width: 0,
                    height: 0,
                })
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
            while ie + 1 < nr
                && (j..=je).all(|k| !cells[ie + 1][k].skip && same(&cells[i][j], &cells[ie + 1][k]))
            {
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

impl NameDate {
    /// Heutiges Datum (UTC, bürgerlicher Kalender nach H. Hinnant).
    pub fn today() -> NameDate {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        let z = secs.div_euclid(86_400) + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        NameDate {
            year: y as i32,
            month: m as u32,
            day: d as u32,
        }
    }
}

/// Slice-Dateiname aus bis zu 6 Bausteinen (`NM4i`, 08 §6.4, Code `@007d60e0` nach re\27 E2);
/// `row`/`col` 1-basiert. Baustein „Dokumentname“: `doc_name` (leer bei einem Dokument ohne Datei,
/// länger als 240 Zeichen ⇒ leer); „Zeile/Spalte“ schreibt `r<Z>_c<S>`. Gesamtlänge ≤ 240.
pub fn auto_name(
    tokens: &[i32],
    row: usize,
    col: usize,
    doc_name: &str,
    seq: usize,
    total: usize,
    date: NameDate,
) -> String {
    let doc_name = if doc_name.chars().count() > 240 {
        ""
    } else {
        doc_name
    };
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
            5 => out.push_str(&if total < 99 {
                format!("{seq:02}")
            } else {
                format!("{seq:03}")
            }),
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

/// Automatischer Name eines Segments ohne eigenen Namen, wie ihn das Original im Inspektor zeigt
/// (re\27 E2): Rasterzelle wie beim Export (Zeile/Spalte 1-basiert, laufende Nummer in Zellreihenfolge),
/// Bausteine der Seite (Vorgabe Dokumentname, „_“, Zeile/Spalte). `doc_name` ist leer, solange das
/// Dokument keine Datei hat (ungespeichert ⇒ „_r2_c2“). `None` = `elem` ist kein Segment der Seite
/// oder hat einen eigenen Namen.
pub fn auto_name_of(
    doc: &Document,
    page: usize,
    elem: fp_model::ElementId,
    doc_name: &str,
    date: NameDate,
) -> Option<String> {
    let slices = collect_slices(doc, page);
    let spec = slices.iter().find(|s| s.element == Some(elem))?;
    if spec.base_name.is_some() {
        return None;
    }
    let p = doc.pages.get(page)?;
    let (x, y) = (p.canvas.origin.x.floor(), p.canvas.origin.y.floor());
    let grid = build_grid(
        [
            x,
            y,
            x + p.canvas.width.round().max(0.0),
            y + p.canvas.height.round().max(0.0),
        ],
        &slices,
    );
    let tokens = p
        .root
        .export
        .as_ref()
        .and_then(|e| e.html.as_ref())
        .and_then(|h| h.slice_naming.clone())
        .unwrap_or_else(|| vec![6, 15, 1, 0, 0, 0]);
    let total = grid.cells.iter().flatten().filter(|c| !c.skip).count();
    let mut seq = 0;
    for (ri, row) in grid.cells.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            if cell.skip {
                continue;
            }
            seq += 1;
            if cell.slice() == Some(spec.id) {
                return Some(auto_name(
                    &tokens,
                    ri + 1,
                    ci + 1,
                    doc_name,
                    seq,
                    total,
                    date,
                ));
            }
        }
    }
    None
}

/// Status-Suffix (`NMFi`, 2 Bausteine), `frame` 1-basiert; Vorgabe `[5, 1]` ⇒ „_s2“ (gemessen,
/// re\18 §3 und §6.4).
pub fn frame_suffix(tokens: &[i32], frame: usize, state_names: &[String; 4]) -> String {
    frame_suffix_with(tokens, frame, state_names, 's')
}

/// Wie [`frame_suffix`], Buchstabe des Bausteins „Status-Nr.“ frei wählbar (der Exportdialog
/// schreibt gemessen „_s2“, re\18 §3).
pub fn frame_suffix_with(
    tokens: &[i32],
    frame: usize,
    state_names: &[String; 4],
    letter: char,
) -> String {
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

/// Grenzen eines Elements in Dokumentkoordinaten (für Schaltflächen ohne aktiven Bereich).
pub type BoundsFn = fn(&Document, &fp_model::Element) -> Option<Rect>;

/// Slices der Web-Ebene einer Seite (oberste zuerst), danach die Segmente der Schaltflächen-Instanzen
/// (TW-19, [`button_slices`]); Grenzen ohne aktiven Bereich mit [`approx_bounds`].
pub fn collect_slices(doc: &Document, page: usize) -> Vec<SliceSpec> {
    collect_slices_with(doc, page, approx_bounds)
}

/// Wie [`collect_slices`], Grenzen der Schaltflächen ohne aktiven Bereich über `bounds` (die Engine
/// übergibt ihre genauen Elementgrenzen).
pub fn collect_slices_with(doc: &Document, page: usize, bounds: BoundsFn) -> Vec<SliceSpec> {
    let mut out = web_slices(doc, page);
    let next = out.iter().map(|s| s.id).max().unwrap_or(0) + 1;
    out.extend(button_slices(doc, page, next, bounds));
    out
}

/// Aktiver Bereich einer Schaltflächen-Instanz: die Web-Objekte der Web-Ebene des Symbols (re\13
/// §1.4 „Aktiver Bereich“, z. B. Bevel Button −43,−18–44,18), mit der Instanzmatrix abgebildet.
/// `None` = keine Schaltfläche oder kein aktiver Bereich im Symbol (BA-072).
pub fn button_active_area(doc: &Document, e: &fp_model::Element) -> Option<Rect> {
    let ElementKind::Instance(i) = &e.kind else {
        return None;
    };
    let (_, sym) = doc.symbol_by_id(i.symbol_id)?;
    if sym.kind() != fp_model::SymbolKind::Button {
        return None;
    }
    let mut r: Option<Rect> = None;
    for slot in &sym.root.layers {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for c in &cel.elements {
            let ElementKind::WebObject(w) = &c.kind else {
                continue;
            };
            let b = match &w.polygon {
                Some(p) if !p.nodes.is_empty() => {
                    fp_model::new::contour_bounds(std::slice::from_ref(p))
                }
                _ => w.bounds.normalized(),
            };
            r = Some(r.map_or(b, |x| x.union(&b)));
        }
    }
    r.filter(|b| b.width() > 0.0 && b.height() > 0.0)
        .map(|b| i.transform.transform_rect_bounds(&b))
}

/// Segmentfläche einer sichtbaren Schaltflächen-Instanz: aktiver Bereich des Symbols, sonst die
/// Grenzen der Instanz (`bounds`). `None` = keine Schaltfläche.
pub fn button_slice_rect(
    doc: &Document,
    e: &fp_model::Element,
    bounds: impl Fn(&fp_model::Element) -> Option<Rect>,
) -> Option<Rect> {
    let ElementKind::Instance(i) = &e.kind else {
        return None;
    };
    if !doc
        .symbol_by_id(i.symbol_id)
        .is_some_and(|(_, s)| s.kind() == fp_model::SymbolKind::Button)
    {
        return None;
    }
    button_active_area(doc, e)
        .or_else(|| bounds(e))
        .map(|r| r.normalized())
        .filter(|r| r.width() > 0.0 && r.height() > 0.0)
}

/// Schaltflächen-Instanzen sichtbarer Ebenen (auch in Gruppen) als Bild-Segmente wie im Original
/// (TW-19): Fläche nach [`button_slice_rect`], Link/Alt/Ziel/Dateiname der Instanz, sonst des
/// Symbols. Kennungen ab `next`. Ist die Web-Ebene ausgeblendet, fehlen sie wie auf der Leinwand.
pub fn button_slices(doc: &Document, page: usize, next: u32, bounds: BoundsFn) -> Vec<SliceSpec> {
    let mut out = Vec::new();
    let Some(p) = doc.pages.get(page) else {
        return out;
    };
    let own = || {
        p.root.layers.iter().filter_map(|s| {
            if let LayerSlot::Own(l) = s {
                Some(l)
            } else {
                None
            }
        })
    };
    if !own()
        .filter(|l| l.kind == LayerKind::Web)
        .all(|l| l.is_visible())
    {
        return out;
    }
    let mut id = next;
    for l in own().collect::<Vec<_>>().into_iter().rev() {
        if l.kind == LayerKind::Web || !l.is_visible() {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        collect_buttons(doc, &cel.elements, bounds, &mut id, &mut out, 0);
    }
    out
}

fn collect_buttons(
    doc: &Document,
    elements: &[std::sync::Arc<fp_model::Element>],
    bounds: BoundsFn,
    id: &mut u32,
    out: &mut Vec<SliceSpec>,
    depth: u32,
) {
    if depth > 24 {
        return;
    }
    for e in elements {
        if !e.common.visible {
            continue;
        }
        match &e.kind {
            ElementKind::Instance(i) => {
                let Some(r) = button_slice_rect(doc, e, |x| bounds(doc, x)) else {
                    continue;
                };
                let behaviors = doc
                    .symbol_by_id(i.symbol_id)
                    .map(|(_, s)| button_behaviors(s, i))
                    .unwrap_or_default();
                let sym = doc
                    .symbol_by_id(i.symbol_id)
                    .map(|(_, s)| s.button.clone())
                    .unwrap_or_default();
                let pick = |a: &Option<String>, b: &Option<String>| {
                    a.clone().or_else(|| b.clone()).unwrap_or_default()
                };
                out.push(SliceSpec {
                    rect: [r.left, r.top, r.right, r.bottom],
                    id: *id,
                    poly: false,
                    base_name: i
                        .button
                        .file_name
                        .clone()
                        .or(sym.file_name.clone())
                        .filter(|s| !s.is_empty()),
                    url: pick(&i.button.url, &sym.url),
                    alt: pick(&i.button.alt, &sym.alt),
                    target: pick(&i.button.target, &sym.target),
                    kind: 0,
                    html: String::new(),
                    polygon: Vec::new(),
                    behaviors,
                    element: Some(e.id),
                });
                *id += 1;
            }
            ElementKind::Group(g) => collect_buttons(doc, &g.children, bounds, id, out, depth + 1),
            _ => {}
        }
    }
}

/// Verhalten einer Schaltflächen-Instanz aus den Symboloptionen (re\27 E1, re\13 §7.5, B): „Down-Status
/// einschließen“ aus ⇒ Einfaches Rollover, an ⇒ Navigationsleistenbild (OverDown = „Over-While-Down
/// einschließen“, „Down beim Laden“ = Wert der Instanz, sonst des Symbols); Statusleistentext des
/// Symbols ⇒ zusätzlich Statusmeldung (Ereignis onMouseOver, A). Die Zerlegung für die Vorlage
/// übernimmt [`crate::behavior::expand_for_export`].
pub fn button_behaviors(
    sym: &fp_model::Symbol,
    inst: &fp_model::InstanceElem,
) -> Vec<fp_model::Behavior> {
    if sym.kind() != fp_model::SymbolKind::Button {
        return Vec::new();
    }
    use fp_model::{Behavior, BehaviorAction};
    let o = &sym.button_options;
    let mut out = Vec::new();
    if o.include_down {
        let mut b = Behavior::new(BehaviorAction::NavBar, 0);
        b.flags = Some(2 | o.include_over_down as i32);
        b.checked = Some(inst.show_down_on_load.unwrap_or(o.show_down_on_load));
        out.push(b);
    } else {
        out.push(Behavior::new(BehaviorAction::SimpleRollover, 0));
    }
    if let Some(m) = &sym.status_message {
        let mut b = Behavior::new(BehaviorAction::StatusMessage, 0);
        b.status_text = Some(m.clone());
        out.push(b);
    }
    out
}

/// Näherung der Elementgrenzen ohne Engine (Pfade mit Griffpunkten, Text mit 2 px Rand, Bitmaps,
/// Gruppen, Instanzen über ihr Symbol); nur Rückfall für [`collect_slices`].
pub fn approx_bounds(doc: &Document, e: &fp_model::Element) -> Option<Rect> {
    approx_depth(doc, e, 0)
}

fn approx_depth(doc: &Document, e: &fp_model::Element, depth: u32) -> Option<Rect> {
    if depth > 24 {
        return None;
    }
    let union = |a: Option<Rect>, b: Option<Rect>| match (a, b) {
        (Some(a), Some(b)) => Some(a.union(&b)),
        (a, b) => a.or(b),
    };
    match &e.kind {
        ElementKind::Path(p) => Some(fp_model::new::contour_bounds(&p.contours)),
        ElementKind::Text(t) => {
            let b = Rect::new(
                t.bounds.left - 2.0,
                t.bounds.top - 2.0,
                t.bounds.right + 2.0,
                t.bounds.bottom + 2.0,
            );
            Some(t.transform.map_or(b, |m| m.transform_rect_bounds(&b)))
        }
        ElementKind::Bitmap(b) => {
            let p = b.image.position();
            Some(Rect::from_xywh(
                p.x,
                p.y,
                b.image.grid.width as f32,
                b.image.grid.height as f32,
            ))
        }
        ElementKind::Group(g) => g
            .children
            .iter()
            .fold(None, |acc, c| union(acc, approx_depth(doc, c, depth + 1))),
        ElementKind::Instance(i) => {
            let (_, sym) = doc.symbol_by_id(i.symbol_id)?;
            let mut r = None;
            for s in &sym.root.layers {
                if let LayerSlot::Own(l) = s
                    && l.kind == LayerKind::Normal
                    && let Some(cel) = l.cels.first()
                {
                    for c in &cel.elements {
                        r = union(r, approx_depth(doc, c, depth + 1));
                    }
                }
            }
            r.map(|b| i.transform.transform_rect_bounds(&b))
        }
        _ => None,
    }
}

/// Slices der Web-Ebene einer Seite (oberste zuerst). Slices ohne `MSNi` bekommen laufende IDs ab 1.
fn web_slices(doc: &Document, page: usize) -> Vec<SliceSpec> {
    let mut out = Vec::new();
    let Some(p) = doc.pages.get(page) else {
        return out;
    };
    let mut next = 1u32;
    for slot in p.root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else {
                continue;
            };
            let Some(sp) = &w.slice else { continue };
            if !e.common.visible {
                continue;
            }
            let poly = w.shape == WebShape::Polygon;
            let polygon: Vec<(f32, f32)> = w
                .polygon
                .as_ref()
                .map(|c| c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect())
                .unwrap_or_default();
            let rect = if poly && !polygon.is_empty() {
                hull(&polygon)
            } else {
                w.bounds
            };
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
                behaviors: w
                    .behaviors
                    .as_ref()
                    .map(|b| b.items.clone())
                    .unwrap_or_default(),
                element: Some(e.id),
            });
        }
    }
    out
}

/// Ein Hotspot: (Form, Rechteck, Polygon, URL, Alt, Ziel, Verhalten).
pub type Hotspot = (
    WebShape,
    Rect,
    Vec<(f32, f32)>,
    String,
    String,
    String,
    Vec<fp_model::Behavior>,
);

/// Hotspots (keine Slices) der Web-Ebene.
pub fn collect_hotspots(doc: &Document, page: usize) -> Vec<Hotspot> {
    let mut out = Vec::new();
    let Some(p) = doc.pages.get(page) else {
        return out;
    };
    for slot in p.root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind != LayerKind::Web {
            continue;
        }
        let Some(cel) = l.cels.first() else { continue };
        for e in &cel.elements {
            let ElementKind::WebObject(w) = &e.kind else {
                continue;
            };
            if w.slice.is_some() || !e.common.visible {
                continue;
            }
            let polygon: Vec<(f32, f32)> = w
                .polygon
                .as_ref()
                .map(|c| c.nodes.iter().map(|n| (n.anchor.x, n.anchor.y)).collect())
                .unwrap_or_default();
            out.push((
                w.shape,
                w.bounds,
                polygon,
                w.url.clone().unwrap_or_default(),
                w.alt_text.clone().unwrap_or_default(),
                w.target.clone().unwrap_or_default(),
                w.behaviors
                    .as_ref()
                    .map(|b| b.items.clone())
                    .unwrap_or_default(),
            ));
        }
    }
    out
}

fn hull(pts: &[(f32, f32)]) -> Rect {
    let mut r = Rect::new(
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
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
            element: None,
        }
    }

    #[test]
    fn t5_grid_like_original() {
        // Laufzeit 08 T5: FW-Spalten [15,70,15,10,50,20,20], Zeilen [10,10,30,10,20,40,30], 13 Zellen.
        let s = vec![
            spec([100.0, 60.0, 160.0, 120.0], 1, false),
            spec([110.0, 10.0, 180.0, 50.0], 2, false),
            spec([15.358984, 20.0, 84.641014, 80.0], 3, true),
        ];
        let g = build_grid([0.0, 0.0, 200.0, 150.0], &s);
        assert_eq!(g.col_widths(), vec![15, 70, 15, 10, 50, 20, 20]);
        assert_eq!(g.row_heights(), vec![10, 10, 30, 10, 20, 40, 30]);
        let live: usize = g.cells.iter().flatten().filter(|c| !c.skip).count();
        assert_eq!(live, 13);
        let d = NameDate {
            year: 2026,
            month: 10,
            day: 11,
        };
        assert_eq!(
            auto_name(&[6, 15, 1, 0, 0, 0], 3, 2, "slices_spacer", 1, 1, d),
            "slices_spacer_r3_c2"
        );
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
