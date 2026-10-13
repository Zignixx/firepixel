//! HTML-/CSS-Export der Slices (08 §6.3, 15 §2.6, §3.5) mit eigenen Vorlagen (keine Vorlagen des Originals):
//! Tabellen mit transparenten Abstandhaltern, ohne Abstandhalter oder verschachtelt, Imagemaps,
//! Links, Verhalten (Rollover, Bildaustausch, Navigationsleiste, Statusleiste, Sound), Pop-up-Menüs
//! (CSS- oder JS-Variante) und CSS-Layer-Ausgabe („CSS und Bilder“).
//!
//! Der Aufbau folgt der Ausgabe des Originals (Laufzeit 08 T5: `<table … width>` mit Magie-Kommentar
//! `<!-- fwtable … -->`, Abstandhalterzeile oben und -spalte rechts, `rowspan`/`colspan`, `td img
//! {display: block;}`, Imagemap `m_<Zelle>` mit Koordinaten relativ zur Zelle, Polygon geschlossen).

use std::collections::{BTreeSet, HashMap};

use crate::behavior::{event_code, expand_for_export, runtime_js, CellResolver, ExportAction, JsNeeds, CSS_MENU_RUNTIME, JS_MENU_RUNTIME};
use crate::popup::{menus_js, MenuDate, MenuView};
use crate::slices::{Grid, SliceSpec};
use fp_model::{PopupMenu, WebShape};

/// Abstandhalter (`shimGeneration`, 08 §1.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shims {
    /// 0: keine.
    None,
    /// 1: transparente Abstandhalter (`spacer.gif`).
    Transparent,
    /// 2: intern (wie transparent, A).
    Internal,
    /// 3: verschachtelte Tabellen ohne Abstandhalter.
    Nested,
}

impl Shims {
    pub fn from_code(v: i32) -> Shims {
        match v {
            0 => Shims::None,
            2 => Shims::Internal,
            3 => Shims::Nested,
            _ => Shims::Transparent,
        }
    }
}

/// Eine Imagemap-Fläche (Dokumentkoordinaten).
#[derive(Clone, Debug, PartialEq)]
pub struct MapArea {
    pub shape: WebShape,
    pub rect: [f32; 4],
    pub polygon: Vec<(f32, f32)>,
    pub url: String,
    pub alt: String,
    pub target: String,
    pub behaviors: Vec<fp_model::Behavior>,
}

/// Inhalt einer Zelle, vom Exporttreiber vorbereitet.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CellContent {
    /// Zellname (`name`/`id`, Dateiname ohne Endung).
    pub name: String,
    /// Bilddatei (relativ zur HTML-Datei) oder `None` bei HTML-Zellen.
    pub image: Option<String>,
    /// HTML-Text (HTML-Slice).
    pub html: Option<String>,
    /// Slice der Zelle.
    pub slice: Option<u32>,
}

/// Optionen des HTML-Schreibers.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlOptions {
    pub shims: Shims,
    pub xhtml: bool,
    /// Zeichensatz (`ENCi`-Name, z. B. „utf-8“).
    pub charset: String,
    /// Seitentitel.
    pub title: String,
    /// Quelldatei, Seitenname, Basisname, Stil, Dokument-ID (für den `fwtable`-Kommentar).
    pub src_file: String,
    pub page_name: String,
    pub base_name: String,
    pub style_name: String,
    pub doc_id: u32,
    /// Hintergrundfarbe der Seite (`#rrggbb`).
    pub bgcolor: String,
    /// CSS- statt JS-Pop-up-Menüs (`CSMb`).
    pub css_popups: bool,
    /// Pfad des Abstandhalters und der Menüpfeile/-skripte.
    pub spacer: String,
    pub arrow: String,
    pub images_dir: String,
    /// Tabellenausrichtung (`TALi`: 0 links, 1 Mitte, 2 rechts).
    pub table_align: i32,
    /// Erstellungszeit für Menüs ohne `CDTs`.
    pub now: MenuDate,
    /// Schalter `param_6` (expliziter Austausch auf Status 1 statt `MM_swapImgRestore`).
    pub explicit_restore: bool,
    /// „HTML-Kommentare einschließen“ (HTML-Einrichtung, re\18 §6.2): Hinweise um die Tabelle.
    pub comments: bool,
    /// Leere Zellen (HTML-Einrichtung „Tabelle“, re\18 §6.3).
    pub empty_cells: EmptyCells,
    /// „Standard-ALT-Tag“ für Bilder ohne eigenen Alt-Text (re\18 §6.4).
    pub default_alt: String,
    /// CSS-Regeln für `body` (Dokumenteigenschaften: Hintergrundbild, Wiederholen, Anhang,
    /// Seitenausrichtung, re\18 §6.2); leer = keine.
    pub body_css: String,
    /// „CSS in externe Datei schreiben“ (`externalCSS`, re8 §6.2): Name der CSS-Datei für die
    /// Stile der CSS-Pop-up-Menüs (im Kopf nur `@import`); leer = Stile im Kopf.
    pub external_css: String,
}

/// Inhalt leerer Tabellenzellen (re\18 §6.3 „Inhalt:“).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EmptyContents {
    /// „Ohne“: `<td></td>`.
    #[default]
    None,
    /// „Platzhalter-Grafik“: transparenter Abstandhalter in Zellgröße.
    Spacer,
    /// „Geschütztes Leerzeichen“: `&nbsp;`.
    Nbsp,
}

/// Leere Zellen: Hintergrundfarbe (`None` = keine) und Inhalt.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct EmptyCells {
    pub color: Option<String>,
    pub contents: EmptyContents,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        HtmlOptions {
            shims: Shims::Transparent,
            xhtml: true,
            charset: "utf-8".into(),
            title: String::new(),
            src_file: String::new(),
            page_name: String::new(),
            base_name: String::new(),
            style_name: "Firepixel".into(),
            doc_id: 0,
            bgcolor: "#ffffff".into(),
            css_popups: true,
            spacer: "spacer.gif".into(),
            arrow: "arrows.gif".into(),
            images_dir: String::new(),
            table_align: 0,
            now: MenuDate { year: 2026, month: 1, day: 1, ..MenuDate::default() },
            explicit_restore: false,
            comments: false,
            empty_cells: EmptyCells::default(),
            default_alt: String::new(),
            body_css: String::new(),
            external_css: String::new(),
        }
    }
}

/// Ergebnis des HTML-Schreibers.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct HtmlOutput {
    pub html: String,
    /// Benötigte Zusatzdateien (Dateiname, Inhalt): `spacer.gif`, Menüskripte, Pfeilbild.
    pub support_files: Vec<(String, Vec<u8>)>,
    /// Bilder, die zusätzlich gebraucht werden: (Slice, 0-basierter Status) für Rollover/Austausch.
    pub needed_frames: BTreeSet<(Option<u32>, i32)>,
    /// Grafik-Menübilder (15 §3.2): je Grafik-Pop-up-Menü eine Zelle Up und Over.
    pub menu_images: Vec<MenuImage>,
}

/// Auftrag für ein Grafik-Menübild (`mmmenu<n>_<B>x<H>_up/_over`).
#[derive(Clone, Debug, PartialEq)]
pub struct MenuImage {
    pub path_up: String,
    pub path_over: String,
    pub width: u32,
    pub height: u32,
    pub menu: PopupMenu,
}

/// 1×1 transparentes GIF (eigener Schreiber).
pub fn spacer_gif() -> Vec<u8> {
    let f = crate::gif::GifFrame { indices: vec![0], w: 1, h: 1, transparent: Some(0), disposal: 0, ..Default::default() };
    crate::gif::write_gif(1, 1, &[[255, 255, 255], [0, 0, 0]], &[f], &crate::gif::GifOptions::default())
}

/// 8×8-Pfeil für Untermenüs (eigenes Bild, schwarzes Dreieck nach rechts).
pub fn arrow_gif() -> Vec<u8> {
    let mut idx = vec![0u8; 64];
    for y in 0..8usize {
        for x in 0..8usize {
            let d = (y as i32 - 3).abs().max((y as i32 - 4).abs().min((y as i32 - 3).abs()));
            if x >= 2 && (x as i32 - 2) <= 3 - d.min(3) {
                idx[y * 8 + x] = 1;
            }
        }
    }
    let f = crate::gif::GifFrame { indices: idx, w: 8, h: 8, transparent: Some(0), ..Default::default() };
    crate::gif::write_gif(8, 8, &[[255, 255, 255], [0, 0, 0]], &[f], &crate::gif::GifOptions::default())
}

fn esc_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

struct Resolver<'a> {
    own: Option<u32>,
    cells: &'a HashMap<u32, (String, Vec<String>)>,
    menus: &'a HashMap<usize, (String, String, f64, f64)>,
    menu_key: &'a dyn Fn(&PopupMenu) -> Option<usize>,
    needed: std::cell::RefCell<BTreeSet<(Option<u32>, i32)>>,
    frame_name: &'a dyn Fn(Option<u32>, i32) -> Option<String>,
}

impl CellResolver for Resolver<'_> {
    fn cell_name(&self, slice: Option<u32>) -> Option<String> {
        let id = slice.or(self.own)?;
        self.cells.get(&id).map(|c| c.0.clone())
    }
    fn frame_file(&self, slice: Option<u32>, frame: i32) -> Option<String> {
        let id = slice.or(self.own);
        if frame > 0 {
            self.needed.borrow_mut().insert((id, frame));
        }
        (self.frame_name)(id, frame)
    }
    fn menu_ref(&self, menu: &PopupMenu, _css: bool) -> Option<(String, String, f64, f64)> {
        let k = (self.menu_key)(menu)?;
        self.menus.get(&k).cloned()
    }
}

/// Schreibt die HTML-Seite.
///
/// * `grid`/`cells`: Raster und Zellinhalte (`cells[r][c]`, nur Ursprungszellen relevant).
/// * `slices`: Slices (für Verhalten, Links, Polygone); `areas`: Hotspots.
/// * `frame_name(slice, frame)`: Bildpfad eines Status (0 = Grundbild).
/// * `num_frames`: Anzahl Status; `measure`: Textbreite für Pop-up-Menüs.
#[allow(clippy::too_many_arguments)]
pub fn write_html(
    grid: &Grid,
    cells: &[Vec<CellContent>],
    slices: &[SliceSpec],
    areas: &[MapArea],
    num_frames: usize,
    frame_name: &dyn Fn(Option<u32>, i32) -> Option<String>,
    measure: &dyn Fn(&str, &str, f64, bool, bool) -> f64,
    o: &HtmlOptions,
) -> HtmlOutput {
    let ev = |s: &str| if o.xhtml { s.to_ascii_lowercase() } else { s.to_string() };
    let close = if o.xhtml { " />" } else { ">" };
    let x0 = grid.xs.first().copied().unwrap_or(0);
    let y0 = grid.ys.first().copied().unwrap_or(0);
    // Zellnamen und Rechtecke je Slice
    let mut cellmap: HashMap<u32, (String, Vec<String>)> = HashMap::new();
    let mut cell_rects: HashMap<(usize, usize), [i32; 4]> = HashMap::new();
    for (r, row) in grid.cells.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell.skip {
                continue;
            }
            let (w, h) = grid.span_size(r, c);
            cell_rects.insert((r, c), [grid.xs[c], grid.ys[r], grid.xs[c] + w, grid.ys[r] + h]);
            if let Some(id) = cell.slice()
                && let Some(cc) = cells.get(r).and_then(|v| v.get(c))
            {
                cellmap.entry(id).or_insert_with(|| (cc.name.clone(), vec![]));
            }
        }
    }
    let slice_ids: BTreeSet<u32> = slices.iter().map(|s| s.id).collect();
    // Pop-up-Menüs sammeln (Menü-IDs: kleinste freie Zahl ≥ 0)
    let mut menu_views: Vec<(MenuView, f64, f64)> = Vec::new();
    let mut menu_refs: HashMap<usize, (String, String, f64, f64)> = HashMap::new();
    let mut menu_models: Vec<PopupMenu> = Vec::new();
    let mut used_ids: BTreeSet<i64> = BTreeSet::new();
    let mut menu_images: Vec<MenuImage> = Vec::new();
    for s in slices {
        for b in &s.behaviors {
            if let Some(p) = &b.popup
                && let Some(id) = p.menu_id
                && id >= 0
            {
                used_ids.insert(id as i64);
            }
        }
    }
    for s in slices {
        for b in s.behaviors.iter().filter(|b| b.action == 9) {
            let Some(p) = &b.popup else { continue };
            let mut id = p.menu_id.unwrap_or(-1) as i64;
            if id < 0 {
                id = (0..).find(|i| !used_ids.contains(i)).unwrap_or(0);
                used_ids.insert(id);
            }
            let mut v = MenuView::from_model(p, b.event, id, o.now, measure);
            if !v.text_only {
                let n = menu_views.len() + 1;
                let (mw, mh) = (v.menu_width as i64, v.menu_height as i64);
                v.image_path = format!("mmmenu{n}_{mw}x{mh}_up.gif");
                v.image_path2 = format!("mmmenu{n}_{mw}x{mh}_over.gif");
                menu_images.push(MenuImage { path_up: format!("{}{}", o.images_dir, v.image_path), path_over: format!("{}{}", o.images_dir, v.image_path2), width: mw.max(1) as u32, height: mh.max(1) as u32, menu: (**p).clone() });
            }
            let (sl, st) = (s.rect[0].floor() as f64 - x0 as f64, s.rect[1].floor() as f64 - y0 as f64);
            let key = menu_models.len();
            menu_models.push((**p).clone());
            let (cont, name) = if o.css_popups { v.css_names() } else { (String::new(), v.js_name()) };
            let (ox, oy) = v.top_origin();
            let (x, y) = if o.css_popups { (v.horz_offset - ox, v.vert_offset - oy) } else { (v.horz_offset, v.vert_offset) };
            menu_refs.insert(key, (cont, name, x, y));
            menu_views.push((v, sl, st));
        }
    }
    let menu_key = |p: &PopupMenu| menu_models.iter().position(|q| q == p);
    let mut needs = JsNeeds::default();
    let mut preload: Vec<String> = Vec::new();
    let mut load_init = String::new();
    let mut needed_all: BTreeSet<(Option<u32>, i32)> = BTreeSet::new();
    let mut slice_events: HashMap<u32, crate::behavior::EventCode> = HashMap::new();
    let all_ids = |i: u32| slice_ids.contains(&i);
    for s in slices {
        let acts: Vec<ExportAction> = s.behaviors.iter().flat_map(|b| expand_for_export(b, o.explicit_restore, num_frames, &all_ids)).collect();
        if acts.is_empty() {
            continue;
        }
        let r = Resolver { own: Some(s.id), cells: &cellmap, menus: &menu_refs, menu_key: &menu_key, needed: Default::default(), frame_name };
        let ec = event_code(&acts, &r, o.css_popups, &mut needs);
        needed_all.extend(r.needed.into_inner());
        preload.extend(ec.preload.iter().cloned());
        load_init.push_str(&ec.on_load);
        slice_events.insert(s.id, ec);
    }
    // Flächen (Hotspots + Polygon-Slices) den Zellen zuordnen
    let mut maps: HashMap<(usize, usize), Vec<(MapArea, crate::behavior::EventCode)>> = HashMap::new();
    let mut all_areas: Vec<MapArea> = areas.to_vec();
    for s in slices.iter().filter(|s| s.poly && !s.polygon.is_empty()) {
        all_areas.push(MapArea { shape: WebShape::Polygon, rect: s.rect, polygon: s.polygon.clone(), url: s.url.clone(), alt: s.alt.clone(), target: s.target.clone(), behaviors: vec![] });
    }
    for a in all_areas {
        let (cx, cy) = (((a.rect[0] + a.rect[2]) / 2.0) as i32, ((a.rect[1] + a.rect[3]) / 2.0) as i32);
        let Some((&key, _)) = cell_rects.iter().find(|(_, r)| cx >= r[0] && cx < r[2] && cy >= r[1] && cy < r[3]) else { continue };
        let own = grid.cells[key.0][key.1].slice();
        let acts: Vec<ExportAction> = a.behaviors.iter().flat_map(|b| expand_for_export(b, o.explicit_restore, num_frames, &all_ids)).collect();
        let r = Resolver { own, cells: &cellmap, menus: &menu_refs, menu_key: &menu_key, needed: Default::default(), frame_name };
        let ec = event_code(&acts, &r, o.css_popups, &mut needs);
        needed_all.extend(r.needed.into_inner());
        preload.extend(ec.preload.iter().cloned());
        maps.entry(key).or_default().push((a, ec));
    }
    let mut seen = BTreeSet::new();
    preload.retain(|p| seen.insert(p.clone()));
    if !preload.is_empty() {
        needs.preload = true;
    }

    // Zellen-HTML
    let cell_html = |r: usize, c: usize| -> String {
        let cell = &grid.cells[r][c];
        let cc = cells.get(r).and_then(|v| v.get(c)).cloned().unwrap_or_default();
        let (w, h) = grid.span_size(r, c);
        let mut attrs = String::new();
        if cell.rowspan > 1 {
            attrs.push_str(&format!(" rowspan=\"{}\"", cell.rowspan));
        }
        if cell.colspan > 1 {
            attrs.push_str(&format!(" colspan=\"{}\"", cell.colspan));
        }
        if let Some(html) = &cc.html {
            return format!("<td{attrs} width=\"{w}\" height=\"{h}\">{html}</td>");
        }
        let Some(src) = &cc.image else {
            let mut attrs = attrs;
            if let Some(c) = &o.empty_cells.color {
                attrs.push_str(&format!(" bgcolor=\"{}\"", esc_attr(c)));
            }
            return match o.empty_cells.contents {
                EmptyContents::None => format!("<td{attrs}></td>"),
                EmptyContents::Spacer => format!("<td{attrs}><img src=\"{}\" width=\"{w}\" height=\"{h}\" alt=\"\"{close}</td>", o.spacer),
                EmptyContents::Nbsp => format!("<td{attrs}>&nbsp;</td>"),
            };
        };
        let spec = cell.slice().and_then(|id| slices.iter().find(|s| s.id == id));
        let ec = cell.slice().and_then(|id| slice_events.get(&id));
        let mut src = src.clone();
        let mut img_onload = String::new();
        if let Some(e) = ec
            && let Some((down, onload)) = &e.down_on_load
        {
            src = down.clone();
            img_onload = format!(" {}=\"{}\"", ev("onLoad"), onload);
        }
        let usemap = if maps.contains_key(&(r, c)) { format!(" usemap=\"#m_{}\"", cc.name) } else { String::new() };
        let alt = esc_attr(spec.map(|s| s.alt.as_str()).filter(|a| !a.is_empty()).unwrap_or(&o.default_alt));
        let img = format!(
            "<img name=\"{n}\" src=\"{src}\" width=\"{w}\" height=\"{h}\" id=\"{n}\"{usemap}{img_onload} alt=\"{alt}\"{close}",
            n = cc.name
        );
        let url = spec.map(|s| s.url.clone()).unwrap_or_default();
        let target = spec.map(|s| s.target.clone()).unwrap_or_default();
        let has_events = ec.is_some_and(|e| !e.on_mouse_over.is_empty() || !e.on_mouse_out.is_empty() || !e.on_click.is_empty());
        if url.is_empty() && !has_events || !usemap.is_empty() {
            return format!("<td{attrs}>{img}</td>");
        }
        let mut a = format!("<a href=\"{}\"", if url.is_empty() { "javascript:;".to_string() } else { esc_attr(&url) });
        if !target.is_empty() {
            a.push_str(&format!(" target=\"{}\"", esc_attr(&target)));
        }
        if let Some(e) = ec {
            for (k, v) in [("onMouseOut", &e.on_mouse_out), ("onMouseOver", &e.on_mouse_over), ("onClick", &e.on_click)] {
                if !v.is_empty() {
                    a.push_str(&format!(" {}=\"{}\"", ev(k), v));
                }
            }
        }
        format!("<td{attrs}>{a}>{img}</a></td>")
    };

    let total_w = grid.xs.last().copied().unwrap_or(0) - x0;
    let align = match o.table_align {
        1 => " align=\"center\"",
        2 => " align=\"right\"",
        _ => "",
    };
    let comment = |nested: bool| {
        format!(
            "<!-- fwtable fwsrc=\"{}\" fwpage=\"{}\" fwbase=\"{}\" fwstyle=\"{}\" fwdocid = \"{}\" fwnested=\"{}\" -->\n",
            esc_attr(&o.src_file),
            esc_attr(&o.page_name),
            esc_attr(&o.base_name),
            esc_attr(&o.style_name),
            o.doc_id,
            u8::from(nested)
        )
    };
    let mut table = String::new();
    match o.shims {
        Shims::Nested => {
            table.push_str(&nested_table(grid, &cell_html, 0, grid.rows(), 0, grid.cols(), true, align, &comment(true)));
        }
        shims => {
            let spacer = shims != Shims::None;
            table.push_str(&format!("<table style=\"display: inline-table;\"{align} border=\"0\" cellpadding=\"0\" cellspacing=\"0\" width=\"{total_w}\">\n"));
            table.push_str(&comment(false));
            if spacer {
                table.push_str("  <tr>\n");
                for w in grid.col_widths() {
                    table.push_str(&format!("   <td><img src=\"{}\" width=\"{w}\" height=\"1\" alt=\"\"{close}</td>\n", o.spacer));
                }
                table.push_str(&format!("   <td><img src=\"{}\" width=\"1\" height=\"1\" alt=\"\"{close}</td>\n", o.spacer));
                table.push_str("  </tr>\n\n");
            }
            let heights = grid.row_heights();
            for (r, row_h) in heights.iter().enumerate().take(grid.rows()) {
                table.push_str("  <tr>\n");
                for c in 0..grid.cols() {
                    if !grid.cells[r][c].skip {
                        table.push_str(&format!("   {}\n", cell_html(r, c)));
                    }
                }
                if spacer {
                    table.push_str(&format!("   <td><img src=\"{}\" width=\"1\" height=\"{}\" alt=\"\"{close}</td>\n", o.spacer, row_h));
                }
                table.push_str("  </tr>\n");
            }
            table.push_str("</table>\n");
        }
    }
    // Imagemaps
    let mut map_html = String::new();
    let mut keys: Vec<&(usize, usize)> = maps.keys().collect();
    keys.sort();
    for key in keys {
        let name = cells.get(key.0).and_then(|v| v.get(key.1)).map(|c| c.name.clone()).unwrap_or_default();
        let r = cell_rects[key];
        map_html.push_str(&format!("<map name=\"m_{name}\" id=\"m_{name}\">\n"));
        for (a, ec) in &maps[key] {
            let rel = |x: f32, y: f32| ((x - r[0] as f32).round() as i32, (y - r[1] as f32).round() as i32);
            let (shape, coords) = match a.shape {
                WebShape::Circle => {
                    let (cx, cy) = rel((a.rect[0] + a.rect[2]) / 2.0, (a.rect[1] + a.rect[3]) / 2.0);
                    ("circle", format!("{cx},{cy},{}", ((a.rect[2] - a.rect[0]) / 2.0).round() as i32))
                }
                WebShape::Polygon if !a.polygon.is_empty() => {
                    let mut pts: Vec<String> = a.polygon.iter().map(|&(x, y)| { let (px, py) = rel(x, y); format!("{px},{py}") }).collect();
                    pts.push(pts[0].clone());
                    ("poly", pts.join(","))
                }
                _ => {
                    let (l, t) = rel(a.rect[0], a.rect[1]);
                    let (rr, b) = rel(a.rect[2], a.rect[3]);
                    ("rect", format!("{l},{t},{rr},{b}"))
                }
            };
            let href = if a.url.is_empty() { "javascript:;".to_string() } else { esc_attr(&a.url) };
            let mut s = format!("<area shape=\"{shape}\" coords=\"{coords}\" href=\"{href}\"");
            if !a.target.is_empty() {
                s.push_str(&format!(" target=\"{}\"", esc_attr(&a.target)));
            }
            for (k, v) in [("onMouseOut", &ec.on_mouse_out), ("onMouseOver", &ec.on_mouse_over), ("onClick", &ec.on_click)] {
                if !v.is_empty() {
                    s.push_str(&format!(" {}=\"{}\"", ev(k), v));
                }
            }
            s.push_str(&format!(" alt=\"{}\"{close}\n", esc_attr(&a.alt)));
            map_html.push_str(&s);
        }
        map_html.push_str("</map>\n");
    }
    // Kopf und Skripte
    let mut head = String::new();
    let mut support: Vec<(String, Vec<u8>)> = Vec::new();
    let has_empty = grid.cells.iter().enumerate().any(|(r, row)| row.iter().enumerate().any(|(c, cell)| !cell.skip && cells.get(r).and_then(|v| v.get(c)).is_none_or(|cc| cc.image.is_none() && cc.html.is_none())));
    if o.shims == Shims::Transparent || o.shims == Shims::Internal || (has_empty && o.empty_cells.contents == EmptyContents::Spacer) {
        support.push((o.spacer.clone(), spacer_gif()));
    }
    let rt = runtime_js(&needs);
    if !rt.is_empty() {
        head.push_str(&format!("<script type=\"text/javascript\">\n<!--\n{rt}//-->\n</script>\n"));
    }
    let mut menus_html = String::new();
    if !menu_views.is_empty() {
        support.push((o.arrow.clone(), arrow_gif()));
        if o.css_popups {
            support.push(("fp_css_menu.js".into(), CSS_MENU_RUNTIME.as_bytes().to_vec()));
            head.push_str("<script type=\"text/javascript\" src=\"fp_css_menu.js\"></script>\n");
            let mut css = String::new();
            for (v, sl, st) in &menu_views {
                css.push_str(&v.css(*sl, *st, &o.images_dir));
                menus_html.push_str(&v.html(o.xhtml, &format!("{}{}", o.images_dir, o.arrow)));
            }
            if o.external_css.is_empty() {
                head.push_str(&format!("<style type=\"text/css\">\n{css}</style>\n"));
            } else {
                // wie die Original-Vorlage: im Kopf nur der Import, die Regeln in der eigenen Datei
                head.push_str(&format!("<style type=\"text/css\" media=\"screen\">\n\t@import url(\"{}\");\n</style>\n", o.external_css.replace('"', "%22")));
                support.push((o.external_css.clone(), css.into_bytes()));
            }
        } else {
            support.push(("fp_menu.js".into(), JS_MENU_RUNTIME.as_bytes().to_vec()));
            let views: Vec<MenuView> = menu_views.iter().map(|m| m.0.clone()).collect();
            head.push_str("<script type=\"text/javascript\" src=\"fp_menu.js\"></script>\n<script type=\"text/javascript\">\n<!--\n");
            head.push_str(&menus_js(&views, &o.images_dir, &format!("{}{}", o.images_dir, o.arrow)));
            head.push_str("//-->\n</script>\n");
        }
    }
    let mut body_attr = format!(" bgcolor=\"{}\"", o.bgcolor);
    let mut onload = String::new();
    if !preload.is_empty() {
        onload.push_str(&format!("MM_preloadImages({});", preload.iter().map(|p| format!("'{p}'")).collect::<Vec<_>>().join(",")));
    }
    onload.push_str(&load_init);
    if !onload.is_empty() {
        body_attr.push_str(&format!(" {}=\"{onload}\"", ev("onLoad")));
    }
    let mut html = String::new();
    if o.xhtml {
        html.push_str("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Transitional//EN\"\n\"http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd\">\n<html xmlns=\"http://www.w3.org/1999/xhtml\">\n");
    } else {
        html.push_str("<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.01 Transitional//EN\">\n<html>\n");
    }
    html.push_str("<head>\n");
    html.push_str(&format!("<title>{}</title>\n", esc_attr(&o.title)));
    html.push_str(&format!("<meta http-equiv=\"Content-Type\" content=\"text/html; charset={}\"{close}\n", o.charset));
    html.push_str("<style type=\"text/css\">td img {display: block;}</style>\n");
    if !o.body_css.is_empty() {
        html.push_str(&format!("<style type=\"text/css\">body {{ {} }}</style>\n", o.body_css));
    }
    html.push_str("<!--Firepixel HTML-Export (eigene Vorlage)-->\n");
    html.push_str(&head);
    html.push_str("</head>\n");
    html.push_str(&format!("<body{body_attr}>\n"));
    if !menus_html.is_empty() {
        html.push_str(&format!("<div id=\"FWTableContainer{}\">\n", o.doc_id));
    }
    if !menus_html.is_empty() && !o.css_popups {
        html.push_str("<script type=\"text/javascript\">mmLoadMenus();</script>\n");
    }
    if o.comments {
        html.push_str("<!-- Firepixel: begin of the sliced table. To reuse it, copy this table and the scripts from the head into the target page. -->\n");
    }
    html.push_str(&table);
    if o.comments {
        html.push_str("<!-- Firepixel: end of the sliced table -->\n");
    }
    html.push_str(&menus_html);
    if !menus_html.is_empty() {
        html.push_str("</div>\n");
    }
    html.push_str(&map_html);
    html.push_str("</body>\n</html>\n");
    HtmlOutput { html, support_files: support, needed_frames: needed_all, menu_images }
}

/// Verschachtelte Tabellen: rekursiv an Zeilen-/Spaltengrenzen teilen, die keine Zelle überspannt
/// (A: Verfahren des Originals `tableNested` ist nicht spezifiziert; Zellnamen behalten die
/// Raster-Koordinaten wie in der Laufzeitdatei T5).
#[allow(clippy::too_many_arguments)]
fn nested_table(grid: &Grid, cell_html: &dyn Fn(usize, usize) -> String, r0: usize, r1: usize, c0: usize, c1: usize, rows_first: bool, align: &str, comment: &str) -> String {
    let crosses_row = |k: usize| (c0..c1).any(|c| (r0..k).any(|r| !grid.cells[r][c].skip && r + grid.cells[r][c].rowspan > k));
    let crosses_col = |k: usize| (r0..r1).any(|r| (c0..k).any(|c| !grid.cells[r][c].skip && c + grid.cells[r][c].colspan > k));
    let width = grid.xs[c1] - grid.xs[c0];
    let row_cuts: Vec<usize> = (r0 + 1..r1).filter(|&k| !crosses_row(k)).collect();
    let col_cuts: Vec<usize> = (c0 + 1..c1).filter(|&k| !crosses_col(k)).collect();
    let mut out = format!("<table{align} border=\"0\" cellpadding=\"0\" cellspacing=\"0\" width=\"{width}\">\n{comment}");
    let try_rows = |out: &mut String| {
        let mut bounds = vec![r0];
        bounds.extend(&row_cuts);
        bounds.push(r1);
        for w in bounds.windows(2) {
            out.push_str("  <tr>\n   <td>");
            out.push_str(&nested_table(grid, cell_html, w[0], w[1], c0, c1, false, "", ""));
            out.push_str("</td>\n  </tr>\n");
        }
    };
    let try_cols = |out: &mut String| {
        let mut bounds = vec![c0];
        bounds.extend(&col_cuts);
        bounds.push(c1);
        out.push_str("  <tr>\n");
        for w in bounds.windows(2) {
            out.push_str("   <td>");
            out.push_str(&nested_table(grid, cell_html, r0, r1, w[0], w[1], true, "", ""));
            out.push_str("</td>\n");
        }
        out.push_str("  </tr>\n");
    };
    if rows_first && !row_cuts.is_empty() {
        try_rows(&mut out);
    } else if !col_cuts.is_empty() {
        try_cols(&mut out);
    } else if !row_cuts.is_empty() {
        try_rows(&mut out);
    } else {
        for r in r0..r1 {
            let mut row = String::new();
            for c in c0..c1 {
                if !grid.cells[r][c].skip {
                    row.push_str(&format!("   {}\n", cell_html(r, c)));
                }
            }
            if !row.is_empty() {
                out.push_str("  <tr>\n");
                out.push_str(&row);
                out.push_str("  </tr>\n");
            }
        }
    }
    out.push_str("</table>");
    if !comment.is_empty() {
        out.push('\n');
    }
    out
}

/// CSS-Layer-Ausgabe („CSS und Bilder“): absolut positionierte `div`s je Zelle mit Bild bzw. HTML.
pub fn write_css_layers(grid: &Grid, cells: &[Vec<CellContent>], o: &HtmlOptions) -> String {
    let close = if o.xhtml { " />" } else { ">" };
    let mut css = String::new();
    let mut body = String::new();
    let mut z = 1;
    for (r, row) in grid.cells.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            if cell.skip {
                continue;
            }
            let Some(cc) = cells.get(r).and_then(|v| v.get(c)) else { continue };
            let (w, h) = grid.span_size(r, c);
            let id = if cc.name.is_empty() { format!("r{}_c{}", r + 1, c + 1) } else { cc.name.clone() };
            css.push_str(&format!(
                "#{id} {{\n\tposition:absolute;\n\tleft:{}px;\n\ttop:{}px;\n\twidth:{w}px;\n\theight:{h}px;\n\tz-index:{z};\n}}\n",
                grid.xs[c] - grid.xs[0],
                grid.ys[r] - grid.ys[0]
            ));
            z += 1;
            match (&cc.image, &cc.html) {
                (_, Some(hh)) => body.push_str(&format!("<div id=\"{id}\">{hh}</div>\n")),
                (Some(src), _) => body.push_str(&format!("<div id=\"{id}\"><img src=\"{src}\" width=\"{w}\" height=\"{h}\" alt=\"\"{close}</div>\n")),
                _ => body.push_str(&format!("<div id=\"{id}\"></div>\n")),
            }
        }
    }
    let total_w = grid.xs.last().copied().unwrap_or(0) - grid.xs.first().copied().unwrap_or(0);
    let total_h = grid.ys.last().copied().unwrap_or(0) - grid.ys.first().copied().unwrap_or(0);
    let mut html = String::from("<!DOCTYPE html>\n<html>\n<head>\n");
    html.push_str(&format!("<meta charset=\"{}\"{close}\n<title>{}</title>\n", o.charset, esc_attr(&o.title)));
    html.push_str(&format!("<style type=\"text/css\">\nbody {{ background-color:{}; }}\n#FWContainer {{\n\tposition:relative;\n\twidth:{total_w}px;\n\theight:{total_h}px;\n}}\n{css}</style>\n", o.bgcolor));
    html.push_str("</head>\n<body>\n<div id=\"FWContainer\">\n");
    html.push_str(&body);
    html.push_str("</div>\n</body>\n</html>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slices::build_grid;

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
            polygon: if poly { vec![(50.0, 20.0), (84.641014, 80.0), (15.358984, 80.0)] } else { vec![] },
            behaviors: vec![],
        }
    }

    fn t5() -> (Grid, Vec<Vec<CellContent>>, Vec<SliceSpec>) {
        let s = vec![spec([100.0, 60.0, 160.0, 120.0], 1, false), spec([110.0, 10.0, 180.0, 50.0], 2, false), spec([15.358984, 20.0, 84.641014, 80.0], 3, true)];
        let g = build_grid([0.0, 0.0, 200.0, 150.0], &s);
        let cells: Vec<Vec<CellContent>> = (0..g.rows())
            .map(|r| {
                (0..g.cols())
                    .map(|c| {
                        let n = format!("slices_spacer_r{}_c{}", r + 1, c + 1);
                        CellContent { image: Some(format!("{n}.gif")), name: n, html: None, slice: g.cells[r][c].slice() }
                    })
                    .collect()
            })
            .collect();
        (g, cells, s)
    }

    #[test]
    fn spacer_table_like_t5() {
        let (g, cells, s) = t5();
        let o = HtmlOptions { base_name: "slices_spacer.gif".into(), ..HtmlOptions::default() };
        let out = write_html(&g, &cells, &s, &[], 1, &|_, _| None, &|t, _, _, _, _| t.len() as f64 * 6.0, &o);
        assert!(out.html.contains("<td rowspan=\"3\"><img name=\"slices_spacer_r3_c2\" src=\"slices_spacer_r3_c2.gif\" width=\"70\" height=\"60\" id=\"slices_spacer_r3_c2\" usemap=\"#m_slices_spacer_r3_c2\" alt=\"\" /></td>"), "{}", out.html);
        assert!(out.html.contains("<area shape=\"poly\" coords=\"35,0,70,60,0,60,35,0\" href=\"javascript:;\" alt=\"\" />"));
        assert!(out.html.contains("<td><img src=\"spacer.gif\" width=\"1\" height=\"10\" alt=\"\" /></td>"));
        assert_eq!(out.support_files[0].0, "spacer.gif");
        let nested = write_html(&g, &cells, &s, &[], 1, &|_, _| None, &|_, _, _, _, _| 10.0, &HtmlOptions { shims: Shims::Nested, ..HtmlOptions::default() });
        assert!(!nested.html.contains("spacer.gif"));
        assert!(nested.html.matches("<table").count() > 1);
        assert!(nested.html.contains("fwnested=\"1\""));
        let css = write_css_layers(&g, &cells, &o);
        assert!(css.contains("#slices_spacer_r3_c2"));
    }

    /// „CSS in externe Datei schreiben“ (TW-03): Stile der CSS-Pop-up-Menüs in eine eigene Datei,
    /// im Kopf nur `@import`; aus bzw. bei JS-Menüs bleibt alles wie bisher.
    #[test]
    fn external_css_for_popup_menus() {
        let (g, cells, mut s) = t5();
        let mut b = fp_model::Behavior::new(fp_model::BehaviorAction::PopupMenu, 1);
        b.popup = Some(Box::new(PopupMenu::default()));
        s[0].behaviors = vec![b];
        let run = |o: &HtmlOptions| write_html(&g, &cells, &s, &[], 1, &|_, _| None, &|t, _, _, _, _| t.len() as f64 * 6.0, o);
        let inline = run(&HtmlOptions::default());
        assert!(inline.html.contains("<style type=\"text/css\">
#"), "{}", inline.html);
        assert!(!inline.support_files.iter().any(|f| f.0.ends_with(".css")));
        let ext = run(&HtmlOptions { external_css: "par_vector.css".into(), ..HtmlOptions::default() });
        assert!(ext.html.contains("<style type=\"text/css\" media=\"screen\">
	@import url(\"par_vector.css\");
</style>"), "{}", ext.html);
        let (_, css) = ext.support_files.iter().find(|f| f.0 == "par_vector.css").expect("CSS-Datei");
        let css = String::from_utf8_lossy(css);
        assert!(css.contains("position:absolute"), "{css}");
        assert!(!ext.html.contains(css.lines().next().unwrap_or("-")), "Regeln nicht doppelt im Kopf");
        let js = run(&HtmlOptions { external_css: "par_vector.css".into(), css_popups: false, ..HtmlOptions::default() });
        assert!(!js.support_files.iter().any(|f| f.0.ends_with(".css")), "nur mit CSS-Pop-ups");
    }
}
