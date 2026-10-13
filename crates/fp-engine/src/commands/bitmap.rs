//! Pixelbefehle der Bitmapwerkzeuge: Malen (Pinsel/Bleistift), Radieren, FÃ¼llen, Verlauf, Pipette.

use super::edit::insert_bitmap;
use super::select::{bitmap_for, flood_region};
use super::{sel_err, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
use crate::docops::{self, bitmap_image, set_bitmap_image};
use crate::error::{EngineError, Result};
use crate::geometry::BasicGeometry;
use crate::session::{DocumentState, Session};
use fp_color::{Argb, Rgba8};
use fp_geom::{IRect, Point};
use fp_model::element::ElementKind;
use fp_raster::{CompositeOptions, Mask, RgbaImage};
use serde_json::json;
use std::sync::Arc;

/// Deckungsmaske eines Pinselstrichs: KreisabdrÃ¼cke (Durchmesser `size`) entlang der Punktfolge im
/// Abstand Â¼ Durchmesser; geglÃ¤ttet oder hart (Pixelmitte im Kreis).
#[cfg_attr(all(feature = "vector", feature = "render"), allow(dead_code))]
pub fn stroke_mask(points: &[Point], size: f32, antialias: bool, round: bool) -> Option<Mask> {
    let r = (size / 2.0).max(0.5);
    let first = points.first()?;
    let (mut l, mut t, mut rr, mut b) = (first.x, first.y, first.x, first.y);
    for p in points {
        l = l.min(p.x);
        t = t.min(p.y);
        rr = rr.max(p.x);
        b = b.max(p.y);
    }
    let bounds = IRect::new((l - r - 1.0).floor() as i32, (t - r - 1.0).floor() as i32, (rr + r + 1.0).ceil() as i32, (b + r + 1.0).ceil() as i32);
    let mut m = Mask::new(bounds).ok()?;
    let step = (size / 4.0).max(0.5);
    let mut stamps = vec![*first];
    for w in points.windows(2) {
        let (a, c) = (w[0], w[1]);
        let len = (c.x - a.x).hypot(c.y - a.y);
        let n = (len / step).ceil().max(1.0) as usize;
        for k in 1..=n {
            let f = k as f32 / n as f32;
            stamps.push(Point::new(a.x + (c.x - a.x) * f, a.y + (c.y - a.y) * f));
        }
    }
    for s in stamps {
        let (x0, y0, x1, y1) = ((s.x - r - 1.0).floor() as i32, (s.y - r - 1.0).floor() as i32, (s.x + r + 1.0).ceil() as i32, (s.y + r + 1.0).ceil() as i32);
        for y in y0..y1 {
            for x in x0..x1 {
                let (cx, cy) = (x as f32 + 0.5 - s.x, y as f32 + 0.5 - s.y);
                let cov = if round {
                    let d = cx.hypot(cy);
                    if antialias { (r + 0.5 - d).clamp(0.0, 1.0) } else if d <= r { 1.0 } else { 0.0 }
                } else {
                    let d = cx.abs().max(cy.abs());
                    if antialias { (r + 0.5 - d).clamp(0.0, 1.0) } else if d <= r { 1.0 } else { 0.0 }
                };
                let v = (cov * 255.0).round() as u8;
                if v > m.get(x, y) {
                    m.set(x, y, v);
                }
            }
        }
    }
    Some(m)
}

/// Pinselabdruck der pinselbasierten Bitmapwerkzeuge (Radiergummi, Weichzeichner & Co., Stempel),
/// nach Messungen am Original (FV-11, PARITAET_A L37â€“L39, einzelne AbdrÃ¼cke 13/Rand 2, 13/Rand 100
/// und 16/Rand 50): Das Raster hat `D + 1` Zellen je Achse ab `floor(p âˆ’ D/2)`, die Mitte liegt bei
/// `start + D/2` (D = 13 bei Mauspunkt 185 â†’ Mitte auf Pixel 184, D = 16 bei 240 â†’ zwischen 239
/// und 240). Bis zum Kernradius deckt der Abdruck voll, danach fÃ¤llt er exponentiell ab; Kern und
/// Abfallbreite hÃ¤ngen vom Rand (0..100) ab und skalieren unter 13 und Ã¼ber 16 px mit der GrÃ¶ÃŸe.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Dab {
    cells: i32,
    half: f32,
    round: bool,
    core: f32,
    width: f32,
}

impl Dab {
    /// Abdruck mit Durchmesser `size`, runder oder quadratischer Spitze und Rand `edge` (0..100).
    pub(crate) fn new(size: f32, round: bool, edge: f32) -> Dab {
        let size = size.max(1.0);
        let e = edge.clamp(0.0, 100.0);
        let r = size / 2.0;
        // gemessen bei Durchmesser 13 und 16; kleinere und grÃ¶ÃŸere Spitzen skalieren mit
        let scale = if r < 6.5 { r / 6.5 } else if r > 8.0 { r / 8.0 } else { 1.0 };
        Dab {
            cells: size.ceil() as i32,
            half: size / 2.0,
            round,
            core: (r - (1.0 + 0.0405 * e) * scale).max(0.0),
            width: ((0.47 + 0.0084 * e + 0.00007 * e * e) * scale).max(0.05),
        }
    }

    /// Deckung im Abstand `d` von der Mitte.
    pub(crate) fn weight(&self, d: f32) -> f32 {
        if d <= self.core { 1.0 } else { (-(d - self.core) / self.width).exp() }
    }

    /// Linke obere Rasterzelle und Mitte des Abdrucks bei `p`.
    fn origin(&self, p: Point) -> (i32, i32, f32, f32) {
        let (sx, sy) = ((p.x - self.half).floor() as i32, (p.y - self.half).floor() as i32);
        (sx, sy, sx as f32 + self.half, sy as f32 + self.half)
    }

    /// Rasterzellen des Abdrucks bei `p`.
    pub(crate) fn bounds(&self, p: Point) -> IRect {
        let (sx, sy, _, _) = self.origin(p);
        IRect::new(sx, sy, sx + self.cells + 1, sy + self.cells + 1)
    }

    /// Ruft `f(x, y, deckung)` fÃ¼r jede Zelle mit sichtbarer Deckung (â‰¥ Â½/255) auf.
    pub(crate) fn each(&self, p: Point, mut f: impl FnMut(i32, i32, f32)) {
        let (sx, sy, cx, cy) = self.origin(p);
        for y in sy..=sy + self.cells {
            for x in sx..=sx + self.cells {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                let d = if self.round { dx.hypot(dy) } else { dx.abs().max(dy.abs()) };
                let w = self.weight(d);
                if w >= 0.5 / 255.0 {
                    f(x, y, w);
                }
            }
        }
    }
}

/// Abdruckpositionen entlang der Punktfolge im Abstand `step` (erster Punkt immer).
pub(crate) fn dab_points(pts: &[Point], step: f32) -> Vec<Point> {
    let Some(first) = pts.first() else { return Vec::new() };
    let step = step.max(0.25);
    let mut stamps = vec![*first];
    let mut carry = 0.0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b.x - a.x).hypot(b.y - a.y);
        if len <= 0.0 {
            continue;
        }
        let mut t = step - carry;
        while t <= len {
            let f = t / len;
            stamps.push(Point::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f));
            t += step;
        }
        carry = (carry + len) % step;
    }
    stamps
}

/// Abstand der Radiergummi-AbdrÃ¼cke: das Original setzt je Pixel einen Abdruck (gemessen: Strich
/// 13 / Rand 2 lÃ¶scht bei 6 px Abstand 81 %, bei 7 px 13 %).
pub(crate) const ERASER_STEP: f32 = 1.0;

/// Radiert mit den AbdrÃ¼cken: jeder Abdruck nimmt `deckung Ã— Deckkraft` der verbliebenen Deckkraft
/// weg (Ã¼berlappende AbdrÃ¼cke verstÃ¤rken sich wie im Original).
pub(crate) fn erase_dabs(img: &mut RgbaImage, dab: &Dab, stamps: &[Point], op: f32, sel: Option<&Mask>) {
    let ib = img.bounds();
    let region = stamps.iter().fold(IRect::default(), |r, s| r.union(&dab.bounds(*s))).intersect(&ib);
    if region.is_empty() {
        return;
    }
    let rw = region.width() as usize;
    let mut keep = vec![1.0f32; rw * region.height() as usize];
    for s in stamps {
        dab.each(*s, |x, y, w| {
            if x < region.left || y < region.top || x >= region.right || y >= region.bottom {
                return;
            }
            let w = w * op * sel.map_or(1.0, |m| m.get(x, y) as f32 / 255.0);
            keep[(y - region.top) as usize * rw + (x - region.left) as usize] *= 1.0 - w;
        });
    }
    for y in region.top..region.bottom {
        for x in region.left..region.right {
            let k = keep[(y - region.top) as usize * rw + (x - region.left) as usize];
            if k < 1.0
                && let Some(px) = img.get(x, y)
            {
                img.set_pixel(x, y, Rgba8::new(px[0], px[1], px[2], (px[3] as f32 * k).round() as u8));
            }
        }
    }
}

/// Pinsel, Bleistift und Radiergummi wie im Original: Der Strich wird mit allen Werkzeugwerten
/// ([`crate::paint::PaintJob`]: Spitze, Weichheit, Strichart, Textur, Deckkraft, Mischmodus,
/// â€žTransparenz beibehaltenâ€œ) in die ausgewÃ¤hlte Bitmap gemalt; ohne ausgewÃ¤hlte Bitmap entsteht eine
/// neue Bitmap in StrichgrÃ¶ÃŸe (oben auf der Ebene, ausgewÃ¤hlt; PARITAET_A L36). Ohne Ã„nderung
/// (z. B. â€žTransparenz beibehaltenâ€œ auf leerer FlÃ¤che) gibt es keinen Verlaufsschritt.
fn apply_paint_job(s: &mut Session, job: crate::paint::PaintJob) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let sel = d.selection.pixels.clone().filter(|_| d.selection.has_pixels());
        let target = crate::paint::target_bitmap(&d.doc, d.page, &d.selection.elements);
        match target.and_then(|(id, e)| docops::find_on_page(&d.doc, d.page, id).map(|p| (p, e))) {
            Some((path, e)) => {
                let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("err-sel-no-bitmap-selected")) };
                let base = bitmap_image(b);
                let Some(out) = crate::paint::paint_into(&job, base.as_ref(), sel.as_ref()) else { return Ok(CommandOutput::session()) };
                let doc = Arc::make_mut(&mut d.doc);
                docops::edit_at(doc, &path, |e| {
                    if let ElementKind::Bitmap(b) = &mut e.kind {
                        set_bitmap_image(b, &out);
                    }
                });
            }
            None => {
                if job.tool == crate::paint::PaintTool::Eraser {
                    return Err(sel_err("err-sel-no-bitmap-selected"));
                }
                let Some(out) = crate::paint::paint_into(&job, None, sel.as_ref()) else { return Ok(CommandOutput::session()) };
                insert_bitmap(d, &out)?;
            }
        }
        Ok(CommandOutput::doc())
    })
}

/// Pinsel aus einem Ã¤lteren `draw`-Parameter (`tool.shape` mit Strich-/Pinselvorgabe).
fn legacy_draw_brush(p: &mut serde_json::Value) {
    if p.get("brush").is_some() {
        return;
    }
    let Some(dp) = p.get("draw").and_then(|d| d.get("params")).cloned() else { return };
    let stroke = dp.get("stroke").filter(|v| v.is_object());
    let mut b = super::panel::tool_brush_from(stroke, dp.get("brush").filter(|v| v.is_object()));
    if let Some(a) = stroke.and_then(|s| s.get("textureAmount")).and_then(|v| v.as_i64()) {
        b.texture_blend = (a.clamp(0, 100) * 10) as i32;
    }
    if let Some(t) = stroke.and_then(|s| s.get("texture")).and_then(|v| v.as_str()) {
        p["texture"] = json!(t);
    }
    if let Ok(v) = serde_json::to_value(&b) {
        p["brush"] = v;
    }
}

/// BeschrÃ¤nkt eine Deckung auf die Pixelauswahl (falls vorhanden).
fn clip_to_selection(m: Mask, d: &DocumentState) -> Mask {
    match d.selection.pixels.as_ref().filter(|_| d.selection.has_pixels()) {
        Some(sel) => m.multiply(sel).unwrap_or(m),
        None => m,
    }
}

/// Malt Farbe durch eine Deckung auf die Zielbitmap (wird bei Bedarf erweitert bzw. neu angelegt).
fn paint_mask(d: &mut DocumentState, cov: Mask, color: Argb, opacity: u8) -> Result<()> {
    let cov = clip_to_selection(cov, d);
    let tb = cov.tight_bounds();
    if tb.is_empty() {
        return Ok(());
    }
    let opt = CompositeOptions { opacity: fp_color::Opacity(opacity), variant: fp_color::blend::OverVariant::Scalar, ..CompositeOptions::default() };
    let target = bitmap_for(d, None, &BasicGeometry);
    match target {
        Some((path, _)) => {
            let doc = Arc::make_mut(&mut d.doc);
            docops::edit_at(doc, &path, |e| {
                if let ElementKind::Bitmap(b) = &mut e.kind {
                    let img = bitmap_image(b).unwrap_or_else(|| RgbaImage::empty_at(tb.left, tb.top));
                    let base = if img.is_empty() { RgbaImage::new(tb).unwrap_or(img) } else { img.extended_to(img.bounds().union(&tb)).unwrap_or(img) };
                    let mut out = base;
                    fp_raster::composite_color(&mut out, color.to_rgba8(), &cov, &opt);
                    set_bitmap_image(b, &out);
                }
            });
        }
        None => {
            let mut out = RgbaImage::new(tb)?;
            fp_raster::composite_color(&mut out, color.to_rgba8(), &cov, &opt);
            insert_bitmap(d, &out)?;
        }
    }
    Ok(())
}

/// Farbe an einem Punkt: Renderer (Komposit) oder oberste Bitmap/FÃ¼llung unter dem Punkt, sonst
/// Leinwandfarbe. `n` = 1, 3 oder 5 (Mittelwert Ã¼ber nÃ—n, am Bild beschnitten).
pub fn sample_color(s: &Session, p: Point, n: i32) -> Result<Argb> {
    let d = s.doc()?;
    let avg = |img: &RgbaImage| -> Option<Argb> {
        let (cx, cy) = (p.x.floor() as i32, p.y.floor() as i32);
        let h = n / 2;
        let mut acc = [0u32; 4];
        let mut cnt = 0u32;
        for y in cy - h..=cy + h {
            for x in cx - h..=cx + h {
                if let Some(px) = img.get(x, y) {
                    for k in 0..4 {
                        acc[k] += px[k] as u32;
                    }
                    cnt += 1;
                }
            }
        }
        (cnt > 0).then(|| Rgba8::new((acc[0] / cnt) as u8, (acc[1] / cnt) as u8, (acc[2] / cnt) as u8, (acc[3] / cnt) as u8).to_argb())
    };
    if let Some(r) = s.renderer()
        && let Some(img) = r.render_page(&d.doc, d.page, d.frame)
        && let Some(c) = avg(&img)
    {
        return Ok(c);
    }
    let hits = docops::hits_at(&d.doc, d.page, d.frame, p, s.geometry(), &docops::HitOptions { tolerance: 0.0, ..Default::default() });
    for h in hits {
        let Some(e) = d.doc.element_at(&h) else { continue };
        match &e.kind {
            ElementKind::Bitmap(b) => {
                if let Some(img) = bitmap_image(b)
                    && let Some(c) = avg(&img)
                {
                    return Ok(c);
                }
            }
            _ => {
                if let Some(a) = docops::path_attrs_of(e) {
                    return Ok(if a.fill.is_some() { a.fill_color } else { a.stroke_color });
                }
            }
        }
    }
    Ok(d.doc.pages.get(d.page).map(|pg| pg.root.background).unwrap_or(Argb(0xffff_ffff)))
}

fn points_size(p: &Params, s: &Session) -> Result<(Vec<Point>, f32)> {
    let pts = p.points("points")?;
    if pts.is_empty() {
        return Err(EngineError::InvalidParams { name: p.command.clone(), message: fp_i18n::t("err-param-no-points") });
    }
    Ok((pts, p.f64_or("size", s.tools.options.brush_size as f64)?.clamp(0.5, 5000.0) as f32))
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("bitmap.paint")
            .label("Pinsel")
            .requires(DOC)
            .param("points", ParamKind::Json, "Punktfolge [[x, y], â€¦]")
            .opt("size", ParamKind::Number { min: Some(0.5), max: Some(5000.0) }, "Durchmesser")
            .opt("color", ParamKind::Color, "Farbe (Vorgabe: Strichfarbe)")
            .opt_default("antialias", ParamKind::Bool, json!(true), "geglÃ¤ttete Kante")
            .opt_default("opacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, json!(100), "Deckkraft in %")
            .opt("brush", ParamKind::Json, "Pinsel (Spitze, Weichheit, Strichart, Empfindlichkeiten) wie `element.brush`; Vorgabe: einfacher Pinsel in `size`")
            .opt("texture", ParamKind::String, "Strichtextur (Name oder Datei); Menge Ã¼ber brush.textureBlend")
            .opt("blend", ParamKind::Integer { min: Some(0), max: Some(55) }, "Mischmodus (Nummer wie element.setBlendMode)")
            .opt("preserveTransparency", ParamKind::Bool, "Transparenz beibehalten: nur auf deckende Pixel malen")
            .opt("trimEnd", ParamKind::Bool, "Endpunkt weglassen (Pinsel; Vorgabe an)")
            .opt("tool", ParamKind::Enum { values: vec!["brush".into(), "pencil".into()] }, "Werkzeug (Vorgabe: Pinsel)")
            .opt("draw", ParamKind::Json, "Ã¤lterer Aufruf: Strich als Zeichenbefehl {command: tool.shape, params} â€“ der Pinsel wird daraus gebildet")
            .run(|s, p| {
                let (_pts, _size) = points_size(p, s)?;
                let mut v = if p.value.is_object() { p.value.clone() } else { json!({}) };
                legacy_draw_brush(&mut v);
                let tool = if p.opt_str("tool")?.as_deref() == Some("pencil") { crate::paint::PaintTool::Pencil } else { crate::paint::PaintTool::Brush };
                let job = crate::paint::PaintJob::from_params(tool, &v, s.tools.stroke_color, s.tools.options.brush_size);
                apply_paint_job(s, job)
            }),
    );
    r.add(
        cmd("bitmap.erase")
            .label("Radiergummi")
            .requires(DOC)
            .param("points", ParamKind::Json, "Punktfolge")
            .opt("size", ParamKind::Number { min: Some(0.5), max: Some(5000.0) }, "Durchmesser")
            .opt_default("round", ParamKind::Bool, json!(true), "runde Spitze")
            .opt_default("opacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, json!(100), "Deckkraft in %")
            .opt("edge", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, "Rand 0..100 (Vorgabe: Werkzeugeinstellung)")
            .run(|s, p| {
                let (_pts, _) = points_size(p, s)?;
                let mut v = if p.value.is_object() { p.value.clone() } else { json!({}) };
                if v.get("edge").is_none() {
                    v["edge"] = json!(s.tools.options.eraser_edge);
                }
                let job = crate::paint::PaintJob::from_params(crate::paint::PaintTool::Eraser, &v, Argb(0), s.tools.options.eraser_size);
                with_doc(s, |d| {
                    if bitmap_for(d, None, &BasicGeometry).is_none() {
                        return Err(sel_err("err-sel-no-bitmap-selected"));
                    }
                    Ok(())
                })?;
                apply_paint_job(s, job)
            }),
    );
    r.add(
        cmd("bitmap.floodFill")
            .label("FÃ¼llen")
            .requires(DOC)
            .param("point", ParamKind::Point, "Klickpunkt")
            .opt_default("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, json!(32), "Farbtoleranz")
            .opt("color", ParamKind::Color, "Farbe (Vorgabe: FÃ¼llfarbe)")
            .opt_default("fillSelection", ParamKind::Bool, json!(false), "ausgewÃ¤hlten Bereich fÃ¼llen")
            .run(|s, p| {
                let pt = p.point("point")?;
                let tol = p.i64_or("tolerance", 32)?.clamp(0, 255) as u32;
                let color = p.opt_color("color")?.unwrap_or(s.tools.fill_color);
                let fill_sel = p.bool_or("fillSelection", false)?;
                with_doc(s, |d| {
                    let cov = if fill_sel && d.selection.has_pixels() {
                        d.selection.pixels.clone().unwrap_or_default()
                    } else {
                        let (_, e) = bitmap_for(d, Some(pt), &BasicGeometry).ok_or_else(|| sel_err("err-sel-no-bitmap"))?;
                        let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("err-sel-no-bitmap")) };
                        let img = bitmap_image(b).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable-short")))?;
                        flood_region(&img, pt.x.floor() as i32, pt.y.floor() as i32, tol).ok_or_else(|| sel_err("err-sel-point-outside-bitmap"))?
                    };
                    paint_mask(d, cov, color, 255)?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("bitmap.gradient")
            .label("Verlauf")
            .requires(DOC)
            .param("from", ParamKind::Point, "Anfang")
            .param("to", ParamKind::Point, "Ende")
            .opt("colors", ParamKind::Json, "Farben [#rrggbb, â€¦] (Vorgabe: FÃ¼ll- â†’ Strichfarbe)")
            .opt_default("fillSelection", ParamKind::Bool, json!(true), "ausgewÃ¤hlten Bereich fÃ¼llen (aus: Farbbereich am Anfangspunkt nach Toleranz)")
            .opt_default("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, json!(32), "Farbtoleranz (nur ohne â€žausgewÃ¤hlten Bereich fÃ¼llenâ€œ)")
            .run(|s, p| {
                let (a, b) = (p.point("from")?, p.point("to")?);
                let colors = super::draw::colors_param(p, &[s.tools.fill_color, s.tools.stroke_color])?;
                let fill_sel = p.bool_or("fillSelection", true)?;
                let tol = p.i64_or("tolerance", 32)?.clamp(0, 255) as u32;
                with_doc(s, |d| {
                    let (path, e) = bitmap_for(d, Some(a), &BasicGeometry).ok_or_else(|| sel_err("err-sel-no-bitmap"))?;
                    // wie der Farbeimer: mit â€žAuswahl fÃ¼llenâ€œ die Pixelauswahl (ohne: die ganze
                    // Bitmap), sonst der zusammenhÃ¤ngende Farbbereich am Anfangspunkt
                    let region = if !fill_sel {
                        let ElementKind::Bitmap(bm) = &e.kind else { return Err(sel_err("err-sel-no-bitmap")) };
                        let img = bitmap_image(bm).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-bitmap-unreadable-short")))?;
                        flood_region(&img, a.x.floor() as i32, a.y.floor() as i32, tol).ok_or_else(|| sel_err("err-sel-point-outside-bitmap"))?
                    } else {
                        match &d.selection.pixels {
                            Some(m) if d.selection.has_pixels() => m.clone(),
                            _ => Mask::from_rect(docops::element_bounds(&d.doc, &e).map(|r| r.round_out()).unwrap_or_default())?,
                        }
                    };
                    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
                    let l2 = (dx * dx + dy * dy).max(1e-9);
                    let lerp = |t: f64| -> Rgba8 {
                        let n = colors.len().max(1);
                        if n == 1 {
                            return colors[0].to_rgba8();
                        }
                        let x = t.clamp(0.0, 1.0) * (n - 1) as f64;
                        let i = (x.floor() as usize).min(n - 2);
                        let f = x - i as f64;
                        let (c0, c1) = (colors[i].to_rgba8(), colors[i + 1].to_rgba8());
                        let m = |u: u8, v: u8| (u as f64 + (v as f64 - u as f64) * f).round() as u8;
                        Rgba8::new(m(c0.r, c1.r), m(c0.g, c1.g), m(c0.b, c1.b), m(c0.a, c1.a))
                    };
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_at(doc, &path, |e| {
                        if let ElementKind::Bitmap(bm) = &mut e.kind
                            && let Some(img) = bitmap_image(bm)
                        {
                            let rb = region.tight_bounds();
                            let mut grad = match RgbaImage::new(rb) {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            let w = grad.width().max(1);
                            for (i, px) in grad.data_mut().iter_mut().enumerate() {
                                let (x, y) = (rb.left + (i % w) as i32, rb.top + (i / w) as i32);
                                let t = ((x as f64 + 0.5 - a.x as f64) * dx + (y as f64 + 0.5 - a.y as f64) * dy) / l2;
                                let c = lerp(t);
                                *px = [c.r, c.g, c.b, c.a];
                            }
                            let opt = CompositeOptions { variant: fp_color::blend::OverVariant::Scalar, ..CompositeOptions::default() };
                            let out = fp_raster::composite_extend(&img, &grad, Some(&region), &opt);
                            set_bitmap_image(bm, &out);
                        }
                    });
                    Ok(CommandOutput::doc())
                })
            }),
    );
}
