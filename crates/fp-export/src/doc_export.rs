//! Dokument-Export („Exportieren“, `exportFilesStyle`, 08 §1.5): HTML und Bilder, nur Bilder,
//! CSS und Bilder, CSS-Sprites, Ebenen/Status/Seiten in Dateien, Photoshop PSD, PDF, SVG —
//! mit einem [`PageRenderer`] für das flache Rechnen.

use fp_geom::IRect;
use fp_model::{Document, ElementKind, LayerKind, LayerSlot};
use fp_raster::{RgbaImage, Sampler};

use crate::animation::{animated_gif, animated_webp, apng, AnimFrameSrc};
use crate::export::{export_image, EncodeExtras};
use crate::html::{write_css_layers, write_html, CellContent, HtmlOptions, MapArea, Shims};
use crate::options::{ExportFormat, OptimizeSettings};
use crate::pipeline::ExportContext;
use crate::popup::MenuDate;
use crate::render::{page_rect, Background, LayerFilter, PageRenderer, RenderRequest};
use crate::slices::{auto_name, build_grid, collect_hotspots, collect_slices, frame_suffix, NameDate};
use crate::{ExportError, Result};

/// Exportart (`exportFilesStyle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilesStyle {
    /// 1: HTML und Bilder.
    HtmlAndImages,
    /// 2: Nur Bilder.
    ImagesOnly,
    /// 5: CSS und Bilder.
    CssAndImages,
    /// 6: CSS-Sprites.
    CssSprites,
    /// 7: Ebenen in Dateien.
    LayersToFiles,
    /// 8: Status in Dateien.
    FramesToFiles,
    /// 9: Seiten in Dateien.
    PagesToFiles,
    /// 15: Photoshop PSD.
    Psd,
    /// 0: PDF.
    Pdf,
    /// SVG (Firepixel-Erweiterung).
    Svg,
}

impl FilesStyle {
    /// Aus `XFSi` (unbekannte Werte ⇒ HTML und Bilder).
    pub fn from_code(v: i32) -> FilesStyle {
        match v {
            0 => FilesStyle::Pdf,
            2 => FilesStyle::ImagesOnly,
            5 => FilesStyle::CssAndImages,
            6 => FilesStyle::CssSprites,
            7 => FilesStyle::LayersToFiles,
            8 => FilesStyle::FramesToFiles,
            9 => FilesStyle::PagesToFiles,
            15 => FilesStyle::Psd,
            _ => FilesStyle::HtmlAndImages,
        }
    }
}

/// Eine erzeugte Datei (Pfad relativ zum Zielordner).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Textbreitenmessung für Pop-up-Menüs: (Text, Schrift, Größe, fett, kursiv) → Breite in px.
pub type MeasureFn = fn(&str, &str, f64, bool, bool) -> f64;

/// Optionen des Dokument-Exports.
#[derive(Clone, Debug)]
pub struct DocExportOptions {
    pub page: usize,
    pub style: FilesStyle,
    /// Basisname der Dateien (ohne Endung), zugleich Baustein „Dokumentname“ der Slice-Namen.
    pub base_name: String,
    /// Quelldatei (für den `fwtable`-Kommentar).
    pub src_file: String,
    /// Datum für Namensbausteine und Pop-up-Menüs ohne `CDTs`.
    pub date: MenuDate,
    /// Slices verwenden (sonst ganze Seite als ein Bild).
    pub use_slices: bool,
    /// Optimieren-Einstellungen; `None` = die der Seite.
    pub settings: Option<OptimizeSettings>,
    /// Textbreite für Pop-up-Menüs (Text, Schrift, Größe, fett, kursiv); Vorgabe 0,55·Größe je Zeichen.
    pub measure: Option<MeasureFn>,
}

impl DocExportOptions {
    pub fn new(base_name: &str) -> DocExportOptions {
        DocExportOptions {
            page: 0,
            style: FilesStyle::HtmlAndImages,
            base_name: base_name.to_string(),
            src_file: String::new(),
            date: MenuDate { year: 2026, month: 1, day: 1, ..MenuDate::default() },
            use_slices: true,
            settings: None,
            measure: None,
        }
    }
}

fn default_measure(t: &str, _f: &str, size: f64, bold: bool, _i: bool) -> f64 {
    (t.chars().count() as f64 * size.max(1.0) * if bold { 0.62 } else { 0.55 }).round()
}

/// Export-Kontext einer Seite (Leinwandfarbe, Auflösung in dpi).
pub fn page_context(doc: &Document, page: usize) -> ExportContext {
    let mut ctx = ExportContext::default();
    if let Some(p) = doc.pages.get(page) {
        ctx.canvas = p.root.background.0;
        let r = p.canvas.resolution;
        ctx.resolution_dpi = if p.canvas.resolution_unit == 1 { r * 2.54 } else { r };
    }
    ctx
}

/// Optimieren-Einstellungen einer Seite (Matte aus dem HTML-Teil).
pub fn page_settings(doc: &Document, page: usize) -> OptimizeSettings {
    doc.pages
        .get(page)
        .and_then(|p| p.legacy_export.as_ref().or(p.root.export.as_ref()))
        .map(|e| OptimizeSettings::from_model(e, None))
        .unwrap_or_default()
}

fn render(r: &dyn PageRenderer, doc: &Document, page: usize, frame: usize, s: &OptimizeSettings, layers: LayerFilter) -> Result<RgbaImage> {
    let bg = if s.matte.is_some() { Background::Transparent } else { Background::Canvas };
    r.render(doc, &RenderRequest { page, frame, layers, background: bg, area: None }).map_err(ExportError::Render)
}

fn apply_crop_scale(img: RgbaImage, s: &OptimizeSettings) -> RgbaImage {
    let mut img = img;
    if let Some([l, t, rr, b]) = s.crop {
        let r = IRect::new(l, t, rr, b);
        if !r.is_empty() {
            img = img.cropped(r);
        }
    }
    if let Some(sc) = s.scale
        && sc.use_scale
    {
        let (w, h) = if sc.percent > 0 && sc.percent != 100 {
            ((img.width() as i64 * sc.percent as i64 / 100).max(1), (img.height() as i64 * sc.percent as i64 / 100).max(1))
        } else if sc.width > 0 && sc.height > 0 {
            (sc.width as i64, sc.height as i64)
        } else {
            (img.width() as i64, img.height() as i64)
        };
        if w as usize != img.width() || h as usize != img.height() {
            img = fp_raster::resample::resize(&img, w as u32, h as u32, Sampler::Bicubic);
        }
    }
    img
}

fn frame_count(doc: &Document, page: usize) -> usize {
    doc.pages.get(page).map(|p| p.root.frame_count.max(1) as usize).unwrap_or(1)
}

fn frames_of(r: &dyn PageRenderer, doc: &Document, page: usize, s: &OptimizeSettings) -> Result<(usize, usize, Vec<AnimFrameSrc>)> {
    let n = frame_count(doc, page);
    let mut out = Vec::with_capacity(n);
    let (mut w, mut h) = (0, 0);
    for f in 0..n {
        let img = apply_crop_scale(render(r, doc, page, f, s, LayerFilter::All)?, s);
        w = img.width();
        h = img.height();
        out.push(AnimFrameSrc { pixels: crate::pipeline::image_pixels(&img), info: s.anim.frames.get(f).cloned().unwrap_or_default() });
    }
    Ok((w, h, out))
}

/// Exportiert ein ganzes Bild bzw. eine Animation.
pub fn export_whole(r: &dyn PageRenderer, doc: &Document, page: usize, s: &OptimizeSettings, ex: &EncodeExtras) -> Result<Vec<u8>> {
    let ctx = page_context(doc, page);
    match s.format {
        ExportFormat::AnimatedGif | ExportFormat::Apng | ExportFormat::AnimatedWebP => {
            let (w, h, fr) = frames_of(r, doc, page, s)?;
            match s.format {
                ExportFormat::AnimatedGif => animated_gif(&fr, w, h, s, &ctx),
                ExportFormat::Apng => apng(&fr, w, h, &ctx, s.anim.loop_count),
                _ => animated_webp(&fr, w, h, s.webp.lossless, s.webp.quality, s.anim.loop_count),
            }
        }
        _ => {
            let img = apply_crop_scale(render(r, doc, page, 0, s, LayerFilter::All)?, s);
            Ok(export_image(&img, s, &ctx, ex)?.bytes)
        }
    }
}

fn safe_name(s: &str) -> String {
    let v: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' }).collect();
    if v.is_empty() { "unbenannt".into() } else { v }
}

/// Dokument exportieren.
pub fn export_document(doc: &Document, r: &dyn PageRenderer, o: &DocExportOptions, ex: &EncodeExtras) -> Result<Vec<ExportFile>> {
    let page = o.page;
    let p = doc.pages.get(page).ok_or_else(|| ExportError::Encode(format!("Seite {page} fehlt")))?;
    let s = o.settings.clone().unwrap_or_else(|| page_settings(doc, page));
    let ext = s.format.extension();
    let ctx = page_context(doc, page);
    let base = safe_name(&o.base_name);
    let mut files = Vec::new();
    match o.style {
        FilesStyle::FramesToFiles => {
            for f in 0..frame_count(doc, page) {
                let img = apply_crop_scale(render(r, doc, page, f, &s, LayerFilter::All)?, &s);
                files.push(ExportFile { path: format!("{base}_f{}.{ext}", f + 1), bytes: export_image(&img, &s, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::PagesToFiles => {
            for (i, pg) in doc.pages.iter().enumerate() {
                if pg.master() {
                    continue;
                }
                let sp = o.settings.clone().unwrap_or_else(|| page_settings(doc, i));
                let name = safe_name(pg.name.as_deref().unwrap_or(&format!("{base}_{}", i + 1)));
                files.push(ExportFile { path: format!("{name}.{}", sp.format.extension()), bytes: export_whole(r, doc, i, &sp, ex)? });
            }
        }
        FilesStyle::LayersToFiles => {
            for slot in &p.root.layers {
                let LayerSlot::Own(l) = slot else { continue };
                if l.kind == LayerKind::Web {
                    continue;
                }
                let img = apply_crop_scale(render(r, doc, page, 0, &s, LayerFilter::Only(vec![l.id]))?, &s);
                files.push(ExportFile { path: format!("{}.{ext}", safe_name(&l.name)), bytes: export_image(&img, &s, &ctx, ex)?.bytes });
            }
        }
        FilesStyle::Psd => {
            let rect = page_rect(doc, page).unwrap_or_default();
            let mut layers = Vec::new();
            for slot in &p.root.layers {
                let LayerSlot::Own(l) = slot else { continue };
                if l.kind == LayerKind::Web {
                    continue;
                }
                let img = r
                    .render(doc, &RenderRequest { page, frame: 0, layers: LayerFilter::Only(vec![l.id]), background: Background::Transparent, area: Some(rect) })
                    .map_err(ExportError::Render)?;
                let ab = img.alpha_bounds();
                let img = if ab.is_empty() { RgbaImage::empty_at(rect.left, rect.top) } else { img.cropped(ab) };
                let img = img.translated(-rect.left, -rect.top);
                layers.push(crate::psd_out::PsdLayer { name: l.name.clone(), image: img, opacity: l.opacity.0, blend: l.blend.0, visible: l.is_visible() });
            }
            let comp = r.render(doc, &RenderRequest { page, frame: 0, ..Default::default() }).map_err(ExportError::Render)?.translated(-rect.left, -rect.top);
            let guides: Vec<crate::psd_out::PsdGuide> = p
                .root
                .guides
                .as_ref()
                .map(|g| {
                    g.vertical.iter().map(|&v| crate::psd_out::PsdGuide { pos: v - p.canvas.origin.x, vertical: true }).chain(g.horizontal.iter().map(|&v| crate::psd_out::PsdGuide { pos: v - p.canvas.origin.y, vertical: false })).collect()
                })
                .unwrap_or_default();
            files.push(ExportFile { path: format!("{base}.psd"), bytes: crate::psd_out::write_psd(rect.width() as usize, rect.height() as usize, &layers, &comp, ctx.resolution_dpi, &guides)? });
        }
        FilesStyle::Pdf => {
            let mut imgs = Vec::new();
            let mut sizes = Vec::new();
            for (i, pg) in doc.pages.iter().enumerate() {
                if pg.master() {
                    continue;
                }
                let img = r.render(doc, &RenderRequest { page: i, ..Default::default() }).map_err(ExportError::Render)?;
                let c = page_context(doc, i);
                let k = 72.0 / c.resolution_dpi.max(1.0) as f64;
                sizes.push((img.width() as f64 * k, img.height() as f64 * k));
                imgs.push(img);
            }
            let pages: Vec<crate::vector_out::PdfPage> = imgs.iter().zip(&sizes).map(|(i, &(w, h))| crate::vector_out::PdfPage { image: i, width_pt: w, height_pt: h }).collect();
            files.push(ExportFile { path: format!("{base}.pdf"), bytes: crate::vector_out::write_pdf(&pages, &o.base_name)? });
        }
        FilesStyle::Svg => {
            files.push(ExportFile { path: format!("{base}.svg"), bytes: crate::vector_out::write_svg(doc, page, 0, None)?.into_bytes() });
        }
        FilesStyle::HtmlAndImages | FilesStyle::ImagesOnly | FilesStyle::CssAndImages | FilesStyle::CssSprites => {
            let slices = if o.use_slices { collect_slices(doc, page) } else { Vec::new() };
            if slices.is_empty() && o.style == FilesStyle::ImagesOnly {
                files.push(ExportFile { path: format!("{base}.{ext}"), bytes: export_whole(r, doc, page, &s, ex)? });
                return Ok(files);
            }
            let rect = page_rect(doc, page).unwrap_or_default();
            let grid = build_grid([rect.left as f32, rect.top as f32, rect.right as f32, rect.bottom as f32], &slices);
            let html_set = p.root.export.as_ref().and_then(|e| e.html.as_deref()).cloned().unwrap_or_else(fp_model::HtmlSettings::cs6_default);
            let tokens = html_set.slice_naming.clone().unwrap_or_else(|| vec![6, 15, 1, 0, 0, 0]);
            let ftokens = html_set.frame_naming.clone().unwrap_or_else(|| vec![5, 1]);
            let state_names = html_set.state_names.clone().unwrap_or_else(|| [String::new(), "_o".into(), "_d".into(), "_od".into()]);
            let date = NameDate { year: o.date.year, month: o.date.month, day: o.date.day };
            let images_dir = html_set.images_dir.clone().filter(|d| !d.is_empty()).map(|d| if d.ends_with('/') { d } else { format!("{d}/") }).unwrap_or_default();
            let discard = html_set.discard_unspecified_slices.unwrap_or(false);
            let total = grid.cells.iter().flatten().filter(|c| !c.skip).count();
            // Status-Bilder rendern (Grundbild immer, weitere nach Bedarf)
            let mut frame_imgs: Vec<Option<RgbaImage>> = vec![None; frame_count(doc, page)];
            frame_imgs[0] = Some(render(r, doc, page, 0, &s, LayerFilter::All)?);
            let mut cells: Vec<Vec<CellContent>> = Vec::new();
            let mut seq = 0;
            let mut cell_of: std::collections::HashMap<Option<u32>, (usize, usize, String)> = Default::default();
            for (ri, row) in grid.cells.iter().enumerate() {
                let mut out_row = Vec::new();
                for (ci, cell) in row.iter().enumerate() {
                    if cell.skip {
                        out_row.push(CellContent::default());
                        continue;
                    }
                    seq += 1;
                    let spec = cell.slice().and_then(|id| slices.iter().find(|x| x.id == id));
                    let name = spec.and_then(|x| x.base_name.clone()).unwrap_or_else(|| auto_name(&tokens, ri + 1, ci + 1, &base, seq, total, date));
                    let mut cc = CellContent { name: safe_name(&name), image: None, html: None, slice: cell.slice() };
                    if let Some(x) = spec
                        && x.kind == 3
                    {
                        cc.html = Some(x.html.clone());
                    } else if !(cell.undefined && discard) {
                        cc.image = Some(format!("{images_dir}{}.{}", cc.name, ext));
                    }
                    if let Some(id) = cell.slice() {
                        cell_of.entry(Some(id)).or_insert((ri, ci, cc.name.clone()));
                    }
                    out_row.push(cc);
                }
                cells.push(out_row);
            }
            let ext_s = ext.to_string();
            let frame_name = |slice: Option<u32>, frame: i32| -> Option<String> {
                let (_, _, n) = cell_of.get(&slice)?;
                if frame <= 0 {
                    Some(format!("{images_dir}{n}.{ext_s}"))
                } else {
                    Some(format!("{images_dir}{n}{}.{ext_s}", frame_suffix(&ftokens, frame as usize + 1, &state_names)))
                }
            };
            let areas: Vec<MapArea> = collect_hotspots(doc, page)
                .into_iter()
                .map(|(shape, rc, poly, url, alt, target, behaviors)| MapArea { shape, rect: [rc.left, rc.top, rc.right, rc.bottom], polygon: poly, url, alt, target, behaviors })
                .collect();
            let mut ho = HtmlOptions {
                shims: Shims::from_code(html_set.shim_generation.unwrap_or(1)),
                xhtml: html_set.xhtml.unwrap_or(1) == 1,
                charset: ["iso-8859-1", "utf-8", "iso-8859-1", "Shift_JIS", "iso-2022-jp", "EUC-JP", "big5", "gb2312", "euc-kr", "iso-8859-2", "x-mac-ce", "windows-1250", "iso-8859-5", "KOI8-R", "x-mac-cyrillic", "windows-1251", "iso-8859-7", "x-mac-greek", "windows-1253", "windows-1252"]
                    .get(html_set.encoding.unwrap_or(1).clamp(0, 19) as usize)
                    .copied()
                    .unwrap_or("utf-8")
                    .to_string(),
                title: format!("{base}.{ext}"),
                src_file: o.src_file.clone(),
                page_name: p.name.clone().unwrap_or_default(),
                base_name: format!("{base}.{ext}"),
                style_name: html_set.template_name.clone().filter(|t| !t.is_empty()).unwrap_or_else(|| "Firepixel".into()),
                doc_id: p.doc_random.unwrap_or(0),
                bgcolor: format!("#{:06x}", if p.root.background.a() == 0 { 0xFF_FFFF } else { p.root.background.0 & 0xFF_FFFF }),
                css_popups: html_set.css_popups.unwrap_or(true),
                images_dir: images_dir.clone(),
                table_align: html_set.table_alignment.unwrap_or(0),
                now: o.date,
                ..HtmlOptions::default()
            };
            ho.spacer = format!("{images_dir}spacer.gif");
            ho.arrow = "arrows.gif".into();
            let measure = o.measure.unwrap_or(default_measure);
            let out = write_html(&grid, &cells, &slices, &areas, frame_count(doc, page), &frame_name, &measure, &ho);
            // Bilder je Zelle (Grundbild) und benötigte Status

            let mut sprite_parts: Vec<(String, RgbaImage)> = Vec::new();
            for (ri, row) in grid.cells.iter().enumerate() {
                for (ci, cell) in row.iter().enumerate() {
                    let cc = &cells[ri][ci];
                    if cell.skip || cc.image.is_none() {
                        continue;
                    }
                    let (w, h) = grid.span_size(ri, ci);
                    let crect = IRect::new(grid.xs[ci], grid.ys[ri], grid.xs[ci] + w, grid.ys[ri] + h);
                    let frames: Vec<i32> = std::iter::once(0).chain(out.needed_frames.iter().filter(|(sl, _)| *sl == cell.slice() && cell.slice().is_some()).map(|(_, f)| *f)).collect();
                    let spec = cell.slice().and_then(|id| slices.iter().find(|x| x.id == id));
                    let own = spec.and_then(|x| {
                        doc.pages[page].root.layers.iter().find_map(|slot| match slot {
                            LayerSlot::Own(l) if l.kind == LayerKind::Web => l.cels.first().and_then(|c| {
                                c.elements.iter().find_map(|e| match &e.kind {
                                    ElementKind::WebObject(w) => w.slice.as_ref().filter(|sp| sp.slice_id == Some(x.id)).and_then(|sp| sp.export.as_ref()).map(|e| OptimizeSettings::from_model(e, s.matte)),
                                    _ => None,
                                })
                            }),
                            _ => None,
                        })
                    });
                    let cs = own.unwrap_or_else(|| s.clone());
                    for f in frames {
                        let fi = f.max(0) as usize;
                        if fi >= frame_imgs.len() {
                            continue;
                        }
                        if frame_imgs[fi].is_none() {
                            frame_imgs[fi] = Some(render(r, doc, page, fi, &s, LayerFilter::All)?);
                        }
                        let Some(full) = &frame_imgs[fi] else { continue };
                        let part = full.cropped(crect);
                        let path = if f == 0 { cc.image.clone().unwrap_or_default() } else { frame_name(cell.slice(), f).unwrap_or_default() };
                        if o.style == FilesStyle::CssSprites && f == 0 {
                            sprite_parts.push((cc.name.clone(), part.clone()));
                            continue;
                        }
                        let mut cs2 = cs.clone();
                        if cs2.format.is_animated() {
                            cs2.format = match cs2.format {
                                ExportFormat::AnimatedGif => ExportFormat::Gif,
                                ExportFormat::Apng => ExportFormat::Png,
                                _ => ExportFormat::WebPLossless,
                            };
                        }
                        files.push(ExportFile { path, bytes: export_image(&part, &cs2, &ctx, ex)?.bytes });
                    }
                }
            }
            // Grafik-Menübilder der Pop-up-Menüs (mmmenu<n>_<B>x<H>_up/_over)
            if matches!(o.style, FilesStyle::HtmlAndImages | FilesStyle::CssAndImages) {
                for (path, bytes) in crate::popup_images::menu_image_files(r, &out.menu_images, &ctx, ex)? {
                    files.push(ExportFile { path, bytes });
                }
            }
            match o.style {
                FilesStyle::HtmlAndImages => {
                    files.push(ExportFile { path: format!("{base}.htm"), bytes: out.html.into_bytes() });
                    for (n, b) in out.support_files {
                        files.push(ExportFile { path: n, bytes: b });
                    }
                }
                FilesStyle::CssAndImages => {
                    files.push(ExportFile { path: format!("{base}.htm"), bytes: write_css_layers(&grid, &cells, &ho).into_bytes() });
                }
                FilesStyle::CssSprites => {
                    let (img, css) = crate::sprites::pack_sprites(&sprite_parts, &format!("{images_dir}{base}_sprite.png"));
                    let px = crate::pipeline::image_pixels(&img);
                    let png = crate::png_out::write_png_truecolor(&px, img.width().max(1), img.height().max(1), true, &crate::png_out::PngOptions::from_context(&ctx, false))?;
                    files.push(ExportFile { path: format!("{images_dir}{base}_sprite.png"), bytes: png });
                    files.push(ExportFile { path: format!("{base}.css"), bytes: css.into_bytes() });
                }
                _ => {}
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::SimpleRenderer;
    use fp_model::{Element, Point, Rect};
    use std::sync::Arc;

    fn doc() -> Document {
        let mut d = Document::new(40.0, 20.0, fp_color::Argb(0xFFFFFFFF));
        let mut ids = std::mem::take(&mut d.ids);
        let mut tid = 0;
        let bmp = Element::new_bitmap(&mut ids, Point::new(0.0, 0.0), 20, 20, &[255, 0, 0, 255].repeat(400), &mut tid);
        let sl = Element::new_slice(&mut ids, Rect::new(0.0, 0.0, 20.0, 20.0));
        d.ids = ids;
        let pg = d.page_mut(0).unwrap();
        if let LayerSlot::Own(l) = &mut pg.root.layers[0] {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.push(Arc::new(bmp));
        }
        if let LayerSlot::Own(l) = &mut pg.root.layers[1] {
            Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.push(Arc::new(sl));
        }
        d
    }

    #[test]
    fn html_and_images_with_one_slice() {
        let d = doc();
        let o = DocExportOptions::new("test");
        let files = export_document(&d, &SimpleRenderer, &o, &EncodeExtras::standalone()).unwrap();
        let names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(names.contains(&"test.htm"), "{names:?}");
        assert!(names.contains(&"test_r1_c1.gif"));
        assert!(names.contains(&"test_r1_c2.gif"));
        // CS6-Vorgabe LSSi = 3: verschachtelte Tabellen ohne Abstandhalter
        let html = String::from_utf8_lossy(&files.iter().find(|f| f.path == "test.htm").unwrap().bytes).into_owned();
        assert!(html.contains("fwnested=\"1\"") && !names.contains(&"spacer.gif"));
        for style in [FilesStyle::ImagesOnly, FilesStyle::CssAndImages, FilesStyle::CssSprites, FilesStyle::LayersToFiles, FilesStyle::FramesToFiles, FilesStyle::PagesToFiles, FilesStyle::Psd, FilesStyle::Pdf, FilesStyle::Svg] {
            let o = DocExportOptions { style, ..DocExportOptions::new("t") };
            let f = export_document(&d, &SimpleRenderer, &o, &EncodeExtras::standalone());
            assert!(f.as_ref().is_ok_and(|v| !v.is_empty()), "{style:?}: {:?}", f.err());
        }
    }
}
