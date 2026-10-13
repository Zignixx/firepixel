//! Menü „Auswählen“ sowie die Auswahlbefehle der Werkzeuge (Objekte, Pfadpunkte, Pixel).

use super::{canvas_rect, insert_new, selected_elements, shape_attrs, sel_err, take_ids, with_doc, DOC, SEL};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::geometry::{self, PathShape};
use crate::selection::{PointRef, SelectMode};
use crate::session::{DocumentState, Session};
use fp_geom::{IRect, Point};
use fp_model::element::{Element, ElementKind};
use fp_model::image::{MaskData, Tile, TileEntry, TileGrid, TileKind, TILE_BYTES};
use fp_model::{ElementId, Extra, SavedSelection};
use fp_raster::Mask;
use serde_json::json;
use std::sync::Arc;

fn mode(p: &crate::command::Params) -> Result<SelectMode> {
    Ok(p.opt_str("mode")?.and_then(|m| SelectMode::from_name(&m)).unwrap_or_default())
}

fn mode_param() -> ParamKind {
    ParamKind::Enum { values: ["replace", "add", "subtract", "intersect", "toggle"].iter().map(|s| s.to_string()).collect() }
}

/// Pixelmaske → Kachelraster (256er Maskenkacheln wie `MSKv`).
pub(crate) fn mask_to_data(m: &Mask) -> (MaskData, f32, f32) {
    let b = m.bounds();
    let (w, h) = (b.width(), b.height());
    let ts = 256u32;
    let cols = w.div_ceil(ts);
    let rows = h.div_ceil(ts);
    let mut tiles = Vec::new();
    let mut next = docops::tile_counter();
    for ty in 0..rows {
        for tx in 0..cols {
            let mut buf = vec![0u8; TILE_BYTES];
            let mut first: Option<u8> = None;
            let mut uniform = (tx + 1) * ts <= w && (ty + 1) * ts <= h;
            for yy in 0..ts.min(h - ty * ts) {
                for xx in 0..ts.min(w - tx * ts) {
                    let v = m.get(b.left + (tx * ts + xx) as i32, b.top + (ty * ts + yy) as i32);
                    buf[(yy * ts + xx) as usize] = v;
                    match first {
                        None => first = Some(v),
                        Some(f) if f != v => uniform = false,
                        _ => {}
                    }
                }
            }
            if uniform {
                tiles.push(TileEntry::SolidMask(first.unwrap_or(0) as u32));
            } else {
                next = next.wrapping_add(1);
                tiles.push(TileEntry::Tile(Arc::new(Tile { id: next, kind: TileKind::Mask, pixels: buf.into(), compressed: None, reserved: None })));
            }
        }
    }
    (MaskData { grid: TileGrid { width: w, height: h, tile_size: ts, tiles, tiles_extra: Extra::default() }, extra: Extra::default() }, b.left as f32, b.top as f32)
}

/// Kachelraster → Pixelmaske.
pub(crate) fn data_to_mask(md: &MaskData, left: f32, top: f32) -> Option<Mask> {
    let (gray, _) = md.grid.to_gray();
    let b = IRect::from_xywh(left.round() as i32, top.round() as i32, md.grid.width as i32, md.grid.height as i32);
    Mask::from_raw(b, gray).ok()
}

/// Neue Pixelmaske einer Form.
fn shape_mask(shape: &str, p: &crate::command::Params, antialias: bool) -> Result<Option<Mask>> {
    let contour = match shape {
        "none" => return Ok(None),
        "rect" => geometry::rect_contour(p.rect("rect")?.normalized(), 0.0),
        "ellipse" => geometry::ellipse_contour(p.rect("rect")?.normalized()),
        "polygon" => geometry::polygon_contour(&p.points("points")?, true),
        other => return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::tr!("err-unknown-shape", name = other) }),
    };
    let s = PathShape { contours: vec![contour], even_odd: false };
    let b = geometry::contours_bounds(&s.contours).map(|r| r.round_out()).unwrap_or_default();
    Ok(Some(geometry::basic_coverage(&s, b, antialias && shape != "rect")))
}

fn set_pixels(d: &mut DocumentState, m: Option<Mask>, mode: SelectMode, feather: u32) {
    match m {
        None => d.selection.pixels = None,
        Some(mut m) => {
            if feather > 0
                && let Ok(f) = m.extended(feather as i32).and_then(|x| x.feather(feather))
            {
                m = f;
            }
            d.selection.apply_pixels(m, mode);
        }
    }
}

/// Weiche Kante wie beim Aufziehen eines Rahmens (gemessen 06/19: Rahmen + `f` px nach außen).
pub(crate) fn feather_mask(m: Mask, f: u32) -> Mask {
    if f == 0 {
        return m;
    }
    m.extended(f as i32).and_then(|x| x.feather(f)).unwrap_or(m)
}

trait MaskExt {
    fn extended(&self, by: i32) -> std::result::Result<Mask, fp_raster::RasterError>;
}

impl MaskExt for Mask {
    fn extended(&self, by: i32) -> std::result::Result<Mask, fp_raster::RasterError> {
        let b = self.bounds();
        let nb = b.inflate(by, by);
        let mut out = Mask::new(nb)?;
        for y in b.top..b.bottom {
            for x in b.left..b.right {
                out.set(x, y, self.get(x, y));
            }
        }
        Ok(out)
    }
}

/// Farbabstand des Zauberstabs und Farbeimers zum Klickpixel, gemessen am Original (PAR 06/10,
/// 07/02; Klick 160,120 im Verlauf von T02 pixelgleich für Toleranz 0/32/100): ein Pixel gehört dazu,
/// wenn jeder Kanal höchstens `tol` abweicht **und** die Abweichungen der Farbkanäle untereinander
/// höchstens `tol` auseinanderliegen (max(d) − min(d) ≤ tol mit d = Pixel − Klickpixel je Kanal).
/// Rückgabe: wie weit die Toleranz überschritten ist (≤ 0 = innerhalb).
pub(crate) fn wand_excess(p: [u8; 4], seed: [u8; 4], tol: u32) -> i32 {
    let d = [0, 1, 2].map(|k| p[k] as i32 - seed[k] as i32);
    let da = (p[3] as i32 - seed[3] as i32).abs();
    let mx = d.iter().map(|v| v.abs()).max().unwrap_or(0).max(da);
    let spread = d.iter().max().unwrap_or(&0) - d.iter().min().unwrap_or(&0);
    mx.max(spread) - tol as i32
}

/// Zauberstab-Region: zusammenhängende Pixel (4er-Nachbarschaft) innerhalb der Toleranz
/// ([`wand_excess`]) ab `(x, y)`.
pub(crate) fn flood_region(img: &fp_raster::RgbaImage, x: i32, y: i32, tol: u32) -> Option<Mask> {
    let b = img.bounds();
    let seed = img.get(x, y)?;
    let mut m = Mask::new(b).ok()?;
    let w = img.width();
    let mut stack = vec![(x, y)];
    let near = |p: [u8; 4]| wand_excess(p, seed, tol) <= 0;
    let mut seen = vec![false; img.width() * img.height()];
    while let Some((cx, cy)) = stack.pop() {
        if !b.contains(cx, cy) {
            continue;
        }
        let i = (cy - b.top) as usize * w + (cx - b.left) as usize;
        if seen[i] {
            continue;
        }
        seen[i] = true;
        let Some(px) = img.get(cx, cy) else { continue };
        if !near(px) {
            continue;
        }
        m.set(cx, cy, 255);
        stack.extend_from_slice(&[(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
    }
    Some(m)
}

/// Bitmap für Pixelbefehle: ausgewählte Bitmap bzw. oberste Bitmap unter dem Punkt.
pub(crate) fn bitmap_for(d: &DocumentState, at: Option<Point>, geo: &dyn geometry::GeometryOps) -> Option<(fp_model::traverse::ElementPath, Arc<Element>)> {
    if let Some(b) = super::edit::selected_bitmaps(d).into_iter().next() {
        return Some(b);
    }
    let p = at?;
    docops::hits_at(&d.doc, d.page, d.frame, p, geo, &docops::HitOptions::default())
        .into_iter()
        .filter_map(|path| d.doc.element_at(&path).cloned().map(|e| (path, e)))
        .find(|(_, e)| matches!(e.kind, ElementKind::Bitmap(_)))
}

/// Gibt es gespeicherte Bitmap-Auswahlen in einem offenen Dokument gleicher Größe (aktive Seite)?
/// (re\19-kurven-tonwert.md §5.1: Wiederherstellen/Löschen nur mit gespeicherten Auswahlen.)
fn has_saved_selection(s: &Session) -> bool {
    let Some(a) = s.active() else { return false };
    let size = canvas_size(a);
    s.documents().iter().any(|d| (d.id == a.id || canvas_size(d) == size) && d.doc.pages.get(d.page).and_then(|p| p.saved_selections.as_ref()).is_some_and(|l| !l.is_empty()))
}

/// Leinwandgröße der aktiven Seite (Vergleich „gleiche Größe“ der Auswahl-Dialoge, §5.2).
fn canvas_size(d: &DocumentState) -> (u32, u32) {
    let r = canvas_rect(&d.doc, d.page).round_out();
    (r.width(), r.height())
}

/// Dokument aus dem Parameter `document` (ID, Vorgabe: aktives), nur gleich groß wie das aktive.
fn other_document(s: &Session, p: &crate::command::Params) -> Result<crate::session::DocId> {
    let a = s.doc()?;
    let Some(id) = p.opt_i64("document")? else { return Ok(a.id) };
    let id = crate::session::DocId(id.max(0) as u64);
    let d = s.document(id).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
    if canvas_size(d) != canvas_size(a) {
        return Err(sel_err("err-sel-document-size"));
    }
    Ok(id)
}

/// Gespeicherte Auswahl schreiben: neuer Name → anhängen; vorhandener Name → mit der gespeicherten
/// Maske verknüpfen (ersetzen, hinzufügen, abziehen, schneiden; §5.2).
fn store_saved(d: &mut DocumentState, name: &str, m: &Mask, md: SelectMode) -> Result<()> {
    let page = d.page;
    let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
    let list = pg.saved_selections.get_or_insert_with(Vec::new);
    let pos = list.iter().position(|s| s.name.as_deref() == Some(name));
    let old = pos.and_then(|i| list[i].mask.as_ref().and_then(|x| data_to_mask(x, list[i].left, list[i].top)));
    let out = match (old, md) {
        (Some(o), SelectMode::Add | SelectMode::Toggle) => o.union(m)?,
        (Some(o), SelectMode::Subtract) => o.subtract(m)?,
        (Some(o), SelectMode::Intersect) => o.intersection(m)?,
        _ => m.clone(),
    };
    if out.tight_bounds().is_empty() {
        return Err(sel_err("err-sel-empty-pixels"));
    }
    let (data, l, t) = mask_to_data(&out);
    let entry = SavedSelection { mask: Some(data), left: l, top: t, name: Some(name.to_string()), extra: Extra::default() };
    match pos {
        Some(i) => list[i] = entry,
        None => list.push(entry),
    }
    Ok(())
}

fn all_selectable(d: &DocumentState) -> Vec<ElementId> {
    docops::selectable_ids(&d.doc, d.page, d.frame)
}

fn saved_index(d: &DocumentState, p: &crate::command::Params) -> Result<usize> {
    let list = d.doc.pages.get(d.page).and_then(|pg| pg.saved_selections.as_ref()).cloned().unwrap_or_default();
    if let Some(n) = p.opt_str("name")?
        && let Some(i) = list.iter().position(|s| s.name.as_deref() == Some(n.as_str()))
    {
        return Ok(i);
    }
    let i = p.i64_or("index", 0)?.max(0) as usize;
    if i < list.len() { Ok(i) } else { Err(EngineError::NotFound(fp_i18n::t("err-nf-saved-selection"))) }
}

/// Verlaufsname wie im Original (PM-6 D35–D39): „Auswahl erweitern“ usw. statt des Menünamens.
fn adjust(s: &mut Session, p: &crate::command::Params, step: &str, f: impl Fn(&Mask, u32, u8) -> std::result::Result<Mask, fp_raster::RasterError>) -> Result<CommandOutput> {
    let amount = p.i64_or("amount", 1)?.clamp(1, 1000) as u32;
    let tol = s.prefs.get_int("SelectionAdjustTolerance", 0).clamp(0, 255) as u8;
    with_doc(s, |d| {
        let m = d.selection.pixels.clone().ok_or_else(|| sel_err("err-req-pixel-selection"))?;
        let ext = m.extended(amount as i32 * 2)?;
        d.selection.pixels = Some(f(&ext, amount, tol)?).filter(|m| !m.tight_bounds().is_empty());
        Ok(CommandOutput::doc().labeled(fp_i18n::t(step)))
    })
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(cmd("select.all").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            if d.bitmap_mode {
                let b = super::edit::selected_bitmaps(d)
                    .first()
                    .and_then(|(_, e)| docops::element_bounds(&d.doc, e))
                    .unwrap_or_else(|| canvas_rect(&d.doc, d.page));
                d.selection.pixels = Mask::from_rect(b.round_out()).ok();
            } else {
                let ids = all_selectable(d);
                d.selection.set(ids);
            }
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("select.none").requires(DOC).menu_when(|s| s.active().is_some_and(|d| !d.selection.is_empty())).run(|s, _| {
        with_doc(s, |d| {
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("select.superselect").requires(SEL).menu_when(super::no_pixel_selection).run(|s, _| {
        with_doc(s, |d| {
            let mut out = Vec::new();
            for (path, e) in selected_elements(d) {
                let id = if path.steps.len() > 1 {
                    let mut pp = path.clone();
                    pp.steps.pop();
                    while matches!(pp.steps.last(), Some(fp_model::PathStep::SubLayerCel(_))) {
                        pp.steps.pop();
                    }
                    d.doc.element_at(&pp).map(|x| x.id).unwrap_or(e.id)
                } else {
                    e.id
                };
                if !out.contains(&id) {
                    out.push(id);
                }
            }
            d.selection.set(out);
            Ok(CommandOutput::doc())
        })
    }));
    // wie im Original nur für Objekte mit Kindern (Gruppen, Masken …)
    r.add(cmd("select.subselect").requires(SEL).available(|s| s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| !e.children().is_empty()))).run(|s, _| {
        with_doc(s, |d| {
            let mut out = Vec::new();
            for (_, e) in selected_elements(d) {
                let ch = e.children();
                if ch.is_empty() {
                    out.push(e.id);
                } else {
                    out.extend(ch.iter().map(|c| c.id));
                }
            }
            d.selection.set(out);
            Ok(CommandOutput::doc())
        })
    }));
    // im Original immer aktiv; ohne Auswahl Meldung statt Wirkung
    r.add(cmd("select.similar").requires(DOC).opt("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, "Toleranz (Vorgabe wie Zauberstab: 32)").run(|s, p| {
        let tol = p.i64_or("tolerance", 32)?.clamp(0, 255) as i32;
        with_doc(s, |d| {
            if d.selection.is_empty() {
                return Err(sel_err("err-sel-nothing"));
            }
            // Pixelauswahl: alle Pixel der Bitmap, deren Farbe einer Farbe im Auswahlrahmen ähnelt
            if d.selection.has_pixels() {
                let geo = crate::geometry::BasicGeometry;
                let m = d.selection.pixels.clone().ok_or_else(|| sel_err("err-req-pixel-selection"))?;
                let (_, e) = bitmap_for(d, None, &geo).or_else(|| {
                    let b = m.tight_bounds();
                    bitmap_for(d, Some(Point::new(b.left as f32 + 0.5, b.top as f32 + 0.5)), &geo)
                }).ok_or_else(|| sel_err("err-sel-no-bitmap"))?;
                let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("err-sel-no-bitmap")) };
                let img = docops::bitmap_image(b).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable-short")))?;
                let mut colors: Vec<[u8; 4]> = Vec::new();
                let mb = m.tight_bounds();
                'outer: for y in mb.top..mb.bottom {
                    for x in mb.left..mb.right {
                        if m.get(x, y) > 127
                            && let Some(px) = img.get(x, y)
                            && !colors.contains(&px)
                        {
                            colors.push(px);
                            if colors.len() >= 1024 {
                                break 'outer;
                            }
                        }
                    }
                }
                let ib = img.bounds();
                let mut out = Mask::new(ib)?;
                for y in ib.top..ib.bottom {
                    for x in ib.left..ib.right {
                        if let Some(px) = img.get(x, y)
                            && colors.iter().any(|c| (0..4).all(|k| (c[k] as i32 - px[k] as i32).abs() <= tol))
                        {
                            out.set(x, y, 255);
                        }
                    }
                }
                d.selection.apply_pixels(out, SelectMode::Add);
                return Ok(CommandOutput::doc());
            }
            let refs: Vec<_> = selected_elements(d).into_iter().map(|(_, e)| e).collect();
            let key = |e: &Element| (e.type_code(), docops::path_attrs_of(e).map(|a| (a.fill_color, a.stroke_color, a.fill.is_some(), a.brush.is_some())));
            let keys: Vec<_> = refs.iter().map(|e| key(e)).collect();
            let mut out = Vec::new();
            for id in all_selectable(d) {
                if let Some(e) = docops::element_on_page(&d.doc, d.page, id)
                    && keys.contains(&key(&e))
                {
                    out.push(id);
                }
            }
            d.selection.set(out);
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("select.inverse").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            if let Some(m) = d.selection.pixels.clone().filter(|_| d.selection.has_pixels()) {
                let within = canvas_rect(&d.doc, d.page).round_out().union(&m.bounds());
                d.selection.pixels = m.inverted(within).ok();
            } else {
                let cur = d.selection.elements.clone();
                let ids: Vec<_> = all_selectable(d).into_iter().filter(|i| !cur.contains(i)).collect();
                d.selection.set(ids);
            }
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("select.feather")
            // im Original immer aktiv; ohne Pixelauswahl Meldung statt Wirkung
            .requires(DOC)
            .opt_default("radius", ParamKind::Integer { min: Some(1), max: Some(1000) }, json!(10), "Radius der weichen Kante")
            .run(|s, p| {
                let r = p.i64_or("radius", 10)?.clamp(1, 1000) as u32;
                with_doc(s, |d| {
                    let m = d.selection.pixels.clone().filter(|_| d.selection.has_pixels()).ok_or_else(|| sel_err("err-req-pixel-selection"))?;
                    d.selection.pixels = Some(m.extended(r as i32)?.feather(r)?);
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-feather-selection")))
                })
            }),
    );
    let amt = || ParamKind::Integer { min: Some(1), max: Some(1000) };
    r.add(cmd("select.expandMarquee").requires(&[Requirement::Document, Requirement::PixelSelection]).opt_default("amount", amt(), json!(1), "Pixel").run(|s, p| adjust(s, p, "msg-step-expand-selection", |m, a, t| m.expand(a, t))));
    r.add(cmd("select.contractMarquee").requires(&[Requirement::Document, Requirement::PixelSelection]).opt_default("amount", amt(), json!(1), "Pixel").run(|s, p| adjust(s, p, "msg-step-contract-selection", |m, a, t| m.contract(a, t))));
    r.add(cmd("select.borderMarquee").requires(&[Requirement::Document, Requirement::PixelSelection]).opt_default("amount", amt(), json!(1), "Pixel").run(|s, p| adjust(s, p, "msg-step-border-selection", |m, a, t| m.border(a, t))));
    r.add(cmd("select.smoothMarquee").requires(&[Requirement::Document, Requirement::PixelSelection]).opt_default("amount", amt(), json!(1), "Pixel").run(|s, p| adjust(s, p, "msg-step-smooth-selection", |m, a, t| m.smooth(a, t))));
    r.add(cmd("select.convertMarqueeToPath").requires(&[Requirement::Document, Requirement::PixelSelection]).run(|s, _| {
        let attrs = shape_attrs(s, false, 1.0);
        with_doc(s, |d| {
            let m = d.selection.pixels.clone().ok_or_else(|| sel_err("err-req-pixel-selection"))?;
            let contours = geometry::trace_mask(&m);
            if contours.is_empty() {
                return Err(sel_err("err-sel-empty-pixels"));
            }
            let doc = Arc::make_mut(&mut d.doc);
            let mut idg = take_ids(doc);
            let e = Element::new_path(&mut idg, contours, attrs);
            doc.ids = idg;
            d.selection.pixels = None;
            let id = insert_new(d, e)?;
            Ok(CommandOutput::doc_with(json!({"id": id.0})))
        })
    }));
    // re\19-kurven-tonwert.md §5: Speicherort je Seite (aktive Seite des Zieldokuments), Speichern und
    // Laden sind eigene Verlaufsschritte („Auswahl speichern“/„Auswahl laden“)
    r.add(
        cmd("select.saveBitmapSelection")
            .requires(&[Requirement::Document, Requirement::PixelSelection])
            .opt("name", ParamKind::String, "Name (Vorgabe „Standard“; vorhandener Name wird ersetzt bzw. verknüpft)")
            .opt("mode", mode_param(), "Verknüpfung mit der vorhandenen gleichnamigen Auswahl (Vorgabe ersetzen)")
            .opt("document", ParamKind::Integer { min: Some(0), max: None }, "Zieldokument (ID, gleiche Größe; Vorgabe: aktives)")
            .run(|s, p| {
                let name = p.opt_str("name")?.unwrap_or_else(|| fp_i18n::t("sel-saved-default-name"));
                let md = mode(p)?;
                let target = other_document(s, p)?;
                let d = s.doc()?;
                let src = d.id;
                let m = d.selection.pixels.clone().filter(|_| d.selection.has_pixels()).ok_or_else(|| sel_err("err-req-pixel-selection"))?;
                let label = fp_i18n::t("msg-step-save-selection");
                if target == src {
                    with_doc(s, |d| store_saved(d, &name, &m, md))?;
                } else {
                    s.modify_document(target, &label, "select.saveBitmapSelection", |d| store_saved(d, &name, &m, md))?;
                }
                Ok(CommandOutput::doc().labeled(label))
            }),
    );
    r.add(
        cmd("select.restoreBitmapSelection")
            .requires(DOC)
            .available(has_saved_selection)
            .opt("name", ParamKind::String, "Name")
            .opt("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .opt("mode", mode_param(), "Verknüpfung mit der Auswahl")
            .opt("invert", ParamKind::Bool, "geladene Maske vorher umkehren")
            .opt("document", ParamKind::Integer { min: Some(0), max: None }, "Quelldokument (ID, gleiche Größe; Vorgabe: aktives)")
            .run(|s, p| {
                let md = mode(p)?;
                let invert = p.bool_or("invert", false)?;
                let source = other_document(s, p)?;
                let src = s.document(source).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-document")))?;
                let i = saved_index(src, p)?;
                let sv = src.doc.pages.get(src.page).and_then(|pg| pg.saved_selections.as_ref()).and_then(|l| l.get(i)).cloned();
                let mut m = sv.and_then(|s| s.mask.as_ref().and_then(|md| data_to_mask(md, s.left, s.top))).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-mask")))?;
                with_doc(s, |d| {
                    if invert {
                        let within = canvas_rect(&d.doc, d.page).round_out().union(&m.bounds());
                        m = m.inverted(within)?;
                    }
                    d.selection.apply_pixels(m, md);
                    Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-load-selection")))
                })
            }),
    );
    r.add(
        cmd("select.deleteBitmapSelection")
            .requires(DOC)
            .available(has_saved_selection)
            .opt("name", ParamKind::String, "Name")
            .opt("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .opt("document", ParamKind::Integer { min: Some(0), max: None }, "Dokument (ID, gleiche Größe; Vorgabe: aktives)")
            .run(|s, p| {
                let target = other_document(s, p)?;
                let remove = |d: &mut DocumentState| -> Result<()> {
                    let i = saved_index(d, p)?;
                    let page = d.page;
                    let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                    if let Some(l) = &mut pg.saved_selections {
                        l.remove(i);
                    }
                    Ok(())
                };
                if Some(target) == s.active_id() {
                    with_doc(s, remove)?;
                } else {
                    s.modify_document(target, &fp_i18n::t(&fp_i18n::command_key("select.deleteBitmapSelection")), "select.deleteBitmapSelection", remove)?;
                }
                Ok(CommandOutput::doc())
            }),
    );

    // Werkzeug-Befehle
    r.add(
        cmd("select.set")
            .label("Auswählen")
            .requires(DOC)
            .param("ids", ParamKind::Ids, "Element-IDs")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung mit der Auswahl")
            .run(|s, p| {
                let ids = p.opt_ids("ids")?.unwrap_or_default();
                let m = mode(p)?;
                with_doc(s, |d| {
                    let mut usable = Vec::new();
                    for id in &ids {
                        let path = docops::find_on_page(&d.doc, d.page, *id).ok_or(EngineError::ElementNotFound(id.0))?;
                        // gesperrte Objekte sind wie im Original nicht auswählbar
                        if !docops::is_locked(&d.doc, d.page, &path) {
                            usable.push(*id);
                        }
                    }
                    d.selection.apply(&usable, m);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("select.inRect")
            .label("Auswahlrechteck")
            .requires(DOC)
            .param("rect", ParamKind::Rect, "Rechteck (berührte Objekte wie im Original)")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung")
            .opt_default("contain", ParamKind::Bool, json!(false), "nur vollständig umschlossene Objekte")
            .run(|s, p| {
                let rr = p.rect("rect")?.normalized();
                let m = mode(p)?;
                // Original: jedes vom Rahmen berührte Objekt zählt (UX-A L1)
                let contain = p.bool_or("contain", false)?;
                with_doc(s, |d| {
                    let ids: Vec<_> = all_selectable(d)
                        .into_iter()
                        .filter(|id| docops::element_on_page(&d.doc, d.page, *id).and_then(|e| docops::element_bounds(&d.doc, &e)).is_some_and(|b| if contain { rr.contains_rect(&b) } else { rr.intersects(&b) }))
                        .collect();
                    d.selection.apply(&ids, m);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("select.pointAt")
            .label("Objekt unter dem Punkt auswählen")
            .requires(DOC)
            .param("point", ParamKind::Point, "Punkt")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung")
            .opt_default("deep", ParamKind::Bool, json!(false), "in Gruppen absteigen (Teilauswahl)")
            .run(|s, p| {
                let pt = p.point("point")?;
                let m = mode(p)?;
                let deep = p.bool_or("deep", false)?;
                let geo_tol = s.prefs.pick_distance();
                let hits = {
                    let d = s.doc()?;
                    let opt = docops::HitOptions { tolerance: geo_tol / d.view.zoom.max(1e-6), ..Default::default() };
                    let h = docops::hits_at(&d.doc, d.page, d.frame, pt, s.geometry(), &opt);
                    h.first().map(|f| if deep { docops::deep_hit(&d.doc, f, pt, s.geometry(), &opt) } else { f.clone() }).and_then(|path| d.doc.element_at(&path).map(|e| e.id))
                };
                with_doc(s, |d| {
                    match hits {
                        Some(id) => d.selection.apply(&[id], m),
                        None if m == SelectMode::Replace => d.selection.clear(),
                        None => {}
                    }
                    Ok(CommandOutput::doc_with(json!({"id": hits.map(|i| i.0)})))
                })
            }),
    );
    r.add(
        cmd("select.points")
            .label("Pfadpunkte auswählen")
            .requires(DOC)
            .param("points", ParamKind::Json, "[{element, contour, node}, …]")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung")
            .run(|s, p| {
                let pts: Vec<PointRef> = p.parse("points")?.unwrap_or_default();
                let m = mode(p)?;
                with_doc(s, |d| {
                    if m == SelectMode::Replace {
                        d.selection.points.clear();
                    }
                    if m == SelectMode::Intersect {
                        d.selection.points.retain(|x| pts.contains(x));
                        return Ok(CommandOutput::doc());
                    }
                    for pr in pts {
                        if !d.selection.elements.contains(&pr.element) {
                            d.selection.elements.push(pr.element);
                        }
                        match m {
                            SelectMode::Subtract => d.selection.points.retain(|x| *x != pr),
                            SelectMode::Toggle => {
                                if let Some(i) = d.selection.points.iter().position(|x| *x == pr) {
                                    d.selection.points.remove(i);
                                } else {
                                    d.selection.points.push(pr);
                                }
                            }
                            _ => {
                                if !d.selection.points.contains(&pr) {
                                    d.selection.points.push(pr);
                                }
                            }
                        }
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("select.pixels")
            .label("Auswahlrahmen")
            .requires(DOC)
            .param("shape", ParamKind::Enum { values: ["rect", "ellipse", "polygon", "none"].iter().map(|s| s.to_string()).collect() }, "Form")
            .opt("rect", ParamKind::Rect, "Rechteck (rect/ellipse)")
            .opt("points", ParamKind::Json, "Punkte (polygon)")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung")
            .opt_default("antialias", ParamKind::Bool, json!(false), "geglätteter Rand")
            .opt_default("feather", ParamKind::Integer { min: Some(0), max: Some(1000) }, json!(0), "weiche Kante")
            .run(|s, p| {
                let shape = p.str("shape")?;
                let m = shape_mask(&shape, p, p.bool_or("antialias", false)?)?;
                let md = mode(p)?;
                let fe = p.i64_or("feather", 0)?.max(0) as u32;
                with_doc(s, |d| {
                    set_pixels(d, m, md, fe);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("select.magicWand")
            .label("Zauberstab")
            .requires(DOC)
            .param("point", ParamKind::Point, "Klickpunkt")
            .opt_default("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, json!(32), "Toleranz")
            .opt_default("mode", mode_param(), json!("replace"), "Verknüpfung")
            .opt_default("feather", ParamKind::Integer { min: Some(0), max: Some(1000) }, json!(0), "weiche Kante")
            .run(|s, p| {
                let pt = p.point("point")?;
                let tol = p.i64_or("tolerance", 32)?.clamp(0, 255) as u32;
                let md = mode(p)?;
                let fe = p.i64_or("feather", 0)?.max(0) as u32;
                let geo = Arc::new(crate::geometry::BasicGeometry);
                with_doc(s, |d| {
                    let (_, e) = bitmap_for(d, Some(pt), geo.as_ref()).ok_or_else(|| sel_err("err-sel-no-bitmap-under-wand"))?;
                    let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("err-sel-no-bitmap")) };
                    let img = docops::bitmap_image(b).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable-short")))?;
                    let m = flood_region(&img, pt.x.floor() as i32, pt.y.floor() as i32, tol).ok_or_else(|| sel_err("err-sel-point-outside-bitmap"))?;
                    let id = e.id;
                    set_pixels(d, Some(m), md, fe);
                    // wie im Original („Bitmap bearbeiten“ + „Zauberstab“): die Bitmap ist danach
                    // das gewählte Objekt und bleibt es nach Esc (PARITAET_A L13, fw.selection = [B])
                    if d.selection.elements != [id] {
                        d.selection.elements = vec![id];
                        d.selection.points.clear();
                    }
                    d.bitmap_mode = true;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    for (name, dir) in [("select.next", 1i64), ("select.previous", -1)] {
        r.add(cmd(name).requires(DOC).run(move |s, _| {
            with_doc(s, |d| {
                let all = all_selectable(d);
                if all.is_empty() {
                    return Ok(CommandOutput::doc());
                }
                let cur = d.selection.elements.first().and_then(|id| all.iter().position(|x| x == id));
                let n = all.len() as i64;
                let i = match cur {
                    Some(c) => (c as i64 + dir).rem_euclid(n) as usize,
                    None => 0,
                };
                d.selection.set(vec![all[i]]);
                Ok(CommandOutput::doc())
            })
        }));
    }
}
