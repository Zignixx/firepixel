//! Pixelbefehle der Bitmapwerkzeuge: Malen (Pinsel/Bleistift), Radieren, Füllen, Verlauf, Pipette.

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

/// Deckungsmaske eines Pinselstrichs: Kreisabdrücke (Durchmesser `size`) entlang der Punktfolge im
/// Abstand ¼ Durchmesser; geglättet oder hart (Pixelmitte im Kreis).
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

/// Beschränkt eine Deckung auf die Pixelauswahl (falls vorhanden).
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

/// Farbe an einem Punkt: Renderer (Komposit) oder oberste Bitmap/Füllung unter dem Punkt, sonst
/// Leinwandfarbe. `n` = 1, 3 oder 5 (Mittelwert über n×n, am Bild beschnitten).
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
        return Err(EngineError::InvalidParams { name: p.command.clone(), message: "keine Punkte".into() });
    }
    Ok((pts, p.f64_or("size", s.tools.options.brush_size as f64)?.clamp(0.5, 5000.0) as f32))
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("bitmap.paint")
            .label("Pinsel")
            .requires(DOC)
            .param("points", ParamKind::Json, "Punktfolge [[x, y], …]")
            .opt("size", ParamKind::Number { min: Some(0.5), max: Some(5000.0) }, "Durchmesser")
            .opt("color", ParamKind::Color, "Farbe (Vorgabe: Strichfarbe)")
            .opt_default("antialias", ParamKind::Bool, json!(true), "geglättete Kante")
            .opt_default("opacity", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, json!(100), "Deckkraft in %")
            .run(|s, p| {
                let (pts, size) = points_size(p, s)?;
                let color = p.opt_color("color")?.unwrap_or(s.tools.stroke_color);
                let aa = p.bool_or("antialias", true)?;
                let op = (p.f64_or("opacity", 100.0)?.clamp(0.0, 100.0) * 2.55).round() as u8;
                let m = stroke_mask(&pts, size, aa, true).ok_or_else(|| EngineError::Failed("Strich".into()))?;
                with_doc(s, |d| {
                    paint_mask(d, m, color, op)?;
                    Ok(CommandOutput::doc())
                })
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
            .run(|s, p| {
                let (pts, _) = points_size(p, s)?;
                let size = p.f64_or("size", s.tools.options.eraser_size as f64)?.clamp(0.5, 5000.0) as f32;
                let round = p.bool_or("round", true)?;
                let op = p.f64_or("opacity", 100.0)?.clamp(0.0, 100.0) / 100.0;
                let m = stroke_mask(&pts, size, true, round).ok_or_else(|| EngineError::Failed("Strich".into()))?;
                with_doc(s, |d| {
                    let m = clip_to_selection(m, d);
                    let (path, _) = bitmap_for(d, None, &BasicGeometry).ok_or_else(|| sel_err("keine Bitmap ausgewählt"))?;
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_at(doc, &path, |e| {
                        if let ElementKind::Bitmap(b) = &mut e.kind
                            && let Some(mut img) = bitmap_image(b)
                        {
                            let (l, t) = img.origin();
                            let w = img.width().max(1);
                            for (i, px) in img.data_mut().iter_mut().enumerate() {
                                let c = m.get(l + (i % w) as i32, t + (i / w) as i32) as f64 / 255.0 * op;
                                if c > 0.0 {
                                    px[3] = (px[3] as f64 * (1.0 - c)).round() as u8;
                                }
                            }
                            set_bitmap_image(b, &img);
                        }
                    });
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("bitmap.floodFill")
            .label("Füllen")
            .requires(DOC)
            .param("point", ParamKind::Point, "Klickpunkt")
            .opt_default("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, json!(32), "Farbtoleranz")
            .opt("color", ParamKind::Color, "Farbe (Vorgabe: Füllfarbe)")
            .opt_default("fillSelection", ParamKind::Bool, json!(false), "ausgewählten Bereich füllen")
            .run(|s, p| {
                let pt = p.point("point")?;
                let tol = p.i64_or("tolerance", 32)?.clamp(0, 255) as u32;
                let color = p.opt_color("color")?.unwrap_or(s.tools.fill_color);
                let fill_sel = p.bool_or("fillSelection", false)?;
                with_doc(s, |d| {
                    let cov = if fill_sel && d.selection.has_pixels() {
                        d.selection.pixels.clone().unwrap_or_default()
                    } else {
                        let (_, e) = bitmap_for(d, Some(pt), &BasicGeometry).ok_or_else(|| sel_err("keine Bitmap"))?;
                        let ElementKind::Bitmap(b) = &e.kind else { return Err(sel_err("keine Bitmap")) };
                        let img = bitmap_image(b).ok_or_else(|| EngineError::Failed("Bitmap".into()))?;
                        flood_region(&img, pt.x.floor() as i32, pt.y.floor() as i32, tol).ok_or_else(|| sel_err("Punkt außerhalb der Bitmap"))?
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
            .opt("colors", ParamKind::Json, "Farben [#rrggbb, …] (Vorgabe: Füll- → Strichfarbe)")
            .run(|s, p| {
                let (a, b) = (p.point("from")?, p.point("to")?);
                let colors = super::draw::colors_param(p, &[s.tools.fill_color, s.tools.stroke_color])?;
                with_doc(s, |d| {
                    let (path, e) = bitmap_for(d, Some(a), &BasicGeometry).ok_or_else(|| sel_err("keine Bitmap"))?;
                    let region = match &d.selection.pixels {
                        Some(m) if d.selection.has_pixels() => m.clone(),
                        _ => Mask::from_rect(docops::element_bounds(&d.doc, &e).map(|r| r.round_out()).unwrap_or_default())?,
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
