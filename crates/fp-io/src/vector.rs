//! Vektorformate: SVG (über usvg → Pfade, Gruppen, Bilder; ARCHITEKTUR §5) sowie Illustrator ≤ 8 und
//! EPS, soweit 12 §4 sie beschreibt (PostScript-Teilmenge der AI-Formate).
//!
//! * **SVG:** Pfade mit Vollton-Füllung/-Strich (Deckkraft als Farb-Alpha, Füllregel), Gruppen mit
//!   Deckkraft, eingebettete PNG/JPEG-Bilder; lineare/radiale Verläufe als Verlaufsfüllung (Form 2/3/9 mit
//!   Griffen und Farb-/Deckkraftstopps), Verlaufsstriche als erste Farbe (A),
//!   Muster, Filter, Masken und Text (ohne Textmodul) entfallen mit Hinweis.
//! * **Illustrator ≤ 8 / AI-EPS:** Leinwand = `%%BoundingBox`, Pfadoperatoren `m l L c C v V y Y h H`,
//!   Malen `f F s S b B n N`, Farben `g G k K x X Xa XA`, Strichbreite `w`. CMYK über die
//!   CMYK-Tabelle des Originals bzw. naiv. Text, platzierte Bilder und Muster entfallen mit Hinweis.
//! * **EPS ohne Illustrator-Daten:** nur die TIFF-Vorschau eines DOS-EPS (das Original rastert mit MPS.dll,
//!   die nicht nachgebaut wird); sonst klare Fehlermeldung.
//! * **PDF / AI ≥ 9:** über den reinen Rust-PDF-Interpreter hayro nach SVG und dann wie SVG
//!   ([`import_pdf`]); das Original nutzt dafür sein Illustrator-Modul (12 §4.5), Ergebnis daher
//!   strukturell abweichend (A): Text als Glyphenpfade, eine Seite je PDF-Seite.

use std::sync::Arc;

use fp_color::cmyk::{cmyk_to_rgb, postscript_cmyk_byte};
use fp_color::Argb;
use fp_model::{Brush, Contour, ContourNode, Element, ElementKind, IdGen, PathAttrs, Point};

use crate::build::{bitmap_element, document_from_raster, empty_document, push_elements, ImportResult, ImportedContent};
use crate::detect::FileFormat;
use crate::{check_size, ImportError, ImportOptions, Result};

/// Früher: Begründung für den fehlenden PDF-Import (bleibt für ältere Aufrufer erhalten).
#[deprecated(note = "PDF wird jetzt über import_pdf gelesen")]
pub const PDF_NOTE: &str = "PDF und Illustrator ab Version 9 werden über einen PDF-Interpreter als Vektoren gelesen.";

#[derive(Default)]
struct PathBuilder {
    contours: Vec<Contour>,
    cur: Vec<ContourNode>,
    start: Option<Point>,
}

impl PathBuilder {
    fn flush(&mut self, closed: bool) {
        if self.cur.is_empty() {
            return;
        }
        let mut nodes = std::mem::take(&mut self.cur);
        if closed && nodes.len() > 1 {
            let (first, last) = (nodes[0].anchor, nodes[nodes.len() - 1].anchor);
            if (first.x - last.x).abs() < 1e-4 && (first.y - last.y).abs() < 1e-4 {
                let l = nodes.pop().unwrap_or_else(|| ContourNode::corner(first));
                nodes[0].pred = l.pred;
            }
        }
        self.contours.push(Contour { closed, bsl: None, nodes, extra: Default::default() });
        self.start = None;
    }
    fn move_to(&mut self, p: Point) {
        self.flush(false);
        self.cur.push(ContourNode::corner(p));
        self.start = Some(p);
    }
    fn line_to(&mut self, p: Point) {
        if self.cur.is_empty() {
            self.move_to(p);
            return;
        }
        self.cur.push(ContourNode::corner(p));
    }
    fn cubic_to(&mut self, c1: Point, c2: Point, p: Point) {
        if self.cur.is_empty() {
            self.move_to(c1);
        }
        if let Some(last) = self.cur.last_mut() {
            last.succ = c1;
            if c1 != last.anchor {
                last.curve = true;
            }
        }
        let mut n = ContourNode::corner(p);
        n.pred = c2;
        n.curve = c2 != p;
        self.cur.push(n);
    }
    fn current(&self) -> Option<Point> {
        self.cur.last().map(|n| n.anchor)
    }
}

// ---------------------------------------------------------------- SVG
fn tf(t: &usvg::Transform, x: f32, y: f32) -> Point {
    Point::new(t.sx * x + t.kx * y + t.tx, t.ky * x + t.sy * y + t.ty)
}

fn paint_color(p: &usvg::Paint, opacity: f32, warnings: &mut Vec<String>) -> Option<Argb> {
    let a = (opacity.clamp(0.0, 1.0) * 255.0).round() as u32;
    match p {
        usvg::Paint::Color(c) => Some(Argb(a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32)),
        usvg::Paint::LinearGradient(g) => stop_color(g.stops(), opacity, warnings),
        usvg::Paint::RadialGradient(g) => stop_color(g.stops(), opacity, warnings),
        usvg::Paint::Pattern(_) => {
            warnings.push("SVG-Muster werden nicht übernommen".into());
            None
        }
    }
}

/// Nur noch für Striche (das Original kennt keine Verlaufsstriche).
fn stop_color(stops: &[usvg::Stop], opacity: f32, warnings: &mut Vec<String>) -> Option<Argb> {
    warnings.push("SVG-Verlaufsstriche werden durch ihre erste Farbe ersetzt (A)".into());
    let s = stops.first()?;
    let c = s.color();
    let a = ((opacity * s.opacity().get()).clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(Argb(a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32))
}

fn argb_of(c: usvg::Color, a: f32) -> Argb {
    let a = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
    Argb(a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32)
}

/// Verlaufsfüllung aus einem SVG-Verlauf: linear ⇒ Form 2, radial ⇒ Form 3 (kreisrund) bzw. 9
/// (elliptisch, bei nicht gleichmäßiger Verzerrung). Griffe in Dokumentkoordinaten (Verlaufs- und
/// Pfadtransformation). Farbstopps aus `stop-color`, Deckkraftstopps aus `stop-opacity` × Deckkraft.
/// Nicht abbildbar (A): Brennpunkt ≠ Mittelpunkt, `spreadMethod` reflect/repeat (Original: nur „pad“).
fn svg_gradient(p: &usvg::Paint, opacity: f32, path_t: &usvg::Transform, warnings: &mut Vec<String>) -> Option<(fp_model::Fill, fp_model::FillHandles)> {
    use fp_model::{Fill, FillHandles, Gradient, GradientStop, GradientStops, StopList};
    let (stops, gt, kind) = match p {
        usvg::Paint::LinearGradient(g) => (g.stops(), g.transform(), 0),
        usvg::Paint::RadialGradient(g) => (g.stops(), g.transform(), 1),
        _ => return None,
    };
    if stops.is_empty() {
        return None;
    }
    let t = path_t.pre_concat(gt);
    let (shape, cat, name, start, end, second) = match p {
        usvg::Paint::LinearGradient(g) => {
            if g.spread_method() != usvg::SpreadMethod::Pad {
                warnings.push("SVG-Verläufe mit Wiederholung werden ohne Wiederholung übernommen".into());
            }
            let (a, b) = (tf(&t, g.x1(), g.y1()), tf(&t, g.x2(), g.y2()));
            let perp = Point::new(a.x - (b.y - a.y), a.y + (b.x - a.x));
            (2, "fc_Linear", "Linear", a, b, perp)
        }
        usvg::Paint::RadialGradient(g) => {
            if (g.fx() - g.cx()).abs() > 1e-3 || (g.fy() - g.cy()).abs() > 1e-3 {
                warnings.push("SVG-Verläufe mit verschobenem Brennpunkt werden zentriert übernommen (A)".into());
            }
            let r = g.r().get();
            let c = tf(&t, g.cx(), g.cy());
            let ex = tf(&t, g.cx() + r, g.cy());
            let ey = tf(&t, g.cx(), g.cy() + r);
            let (lx, ly) = ((ex.x - c.x).hypot(ex.y - c.y), (ey.x - c.x).hypot(ey.y - c.y));
            let dot = (ex.x - c.x) * (ey.x - c.x) + (ex.y - c.y) * (ey.y - c.y);
            let round = (lx - ly).abs() < 1e-3 * lx.max(1.0) && dot.abs() < 1e-3 * lx * ly.max(1.0);
            if round { (3, "fc_Circular", "Radial", c, ex, ey) } else { (9, "fc_Elliptical", "Elliptical", c, ex, ey) }
        }
        _ => return None,
    };
    let _ = kind;
    let colors: Vec<GradientStop> = stops.iter().map(|s| GradientStop { position: s.offset().get(), color: argb_of(s.color(), 1.0), extra: Default::default() }).collect();
    let opac: Vec<GradientStop> = stops
        .iter()
        .map(|s| GradientStop { position: s.offset().get(), color: Argb((((s.opacity().get() * opacity).clamp(0.0, 1.0) * 255.0).round() as u32) << 24 | 0x00FF_FFFF), extra: Default::default() })
        .collect();
    let mut f = Fill::solid();
    f.category = cat.into();
    f.name = name.into();
    f.user_name = Some(name.into());
    f.shape = shape;
    f.gradient = Some(Gradient {
        category: cat.into(),
        name: name.into(),
        user_name: None,
        legacy_stops: None,
        stops: Some(GradientStops { colors: StopList { stops: colors, extra: Default::default() }, opacities: StopList { stops: opac, extra: Default::default() }, dither: None, dither_value: None, extra: Default::default() }),
        extra: Default::default(),
    });
    Some((f, FillHandles { valid: true, start, end, second }))
}

fn svg_path(ids: &mut IdGen, p: &usvg::Path, warnings: &mut Vec<String>) -> Option<Element> {
    if !p.is_visible() {
        return None;
    }
    let t = p.abs_transform();
    let mut b = PathBuilder::default();
    for seg in p.data().segments() {
        match seg {
            usvg::tiny_skia_path::PathSegment::MoveTo(a) => b.move_to(tf(&t, a.x, a.y)),
            usvg::tiny_skia_path::PathSegment::LineTo(a) => b.line_to(tf(&t, a.x, a.y)),
            usvg::tiny_skia_path::PathSegment::QuadTo(q, a) => {
                let p0 = b.current().unwrap_or_else(|| tf(&t, q.x, q.y));
                let (q, a) = (tf(&t, q.x, q.y), tf(&t, a.x, a.y));
                let c1 = Point::new(p0.x + 2.0 / 3.0 * (q.x - p0.x), p0.y + 2.0 / 3.0 * (q.y - p0.y));
                let c2 = Point::new(a.x + 2.0 / 3.0 * (q.x - a.x), a.y + 2.0 / 3.0 * (q.y - a.y));
                b.cubic_to(c1, c2, a);
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(c1, c2, a) => b.cubic_to(tf(&t, c1.x, c1.y), tf(&t, c2.x, c2.y), tf(&t, a.x, a.y)),
            usvg::tiny_skia_path::PathSegment::Close => b.flush(true),
        }
    }
    b.flush(false);
    if b.contours.is_empty() {
        return None;
    }
    let mut attrs = PathAttrs::default();
    let mut even_odd = false;
    let mut handles = None;
    if let Some(f) = p.fill() {
        even_odd = f.rule() == usvg::FillRule::EvenOdd;
        if let Some((fill, h)) = svg_gradient(f.paint(), f.opacity().get(), &t, warnings) {
            if let Some(s) = fill.gradient.as_ref().and_then(|g| g.color_stops().first()) {
                attrs.fill_color = s.color;
            }
            attrs.fill = Some(fill);
            handles = Some(h);
        } else if let Some(c) = paint_color(f.paint(), f.opacity().get(), warnings) {
            attrs.fill_color = c;
            attrs.fill = Some(fp_model::Fill::solid());
        }
    }
    if let Some(s) = p.stroke()
        && let Some(c) = paint_color(s.paint(), s.opacity().get(), warnings)
    {
        let scale = ((t.sx * t.sx + t.ky * t.ky).sqrt() + (t.kx * t.kx + t.sy * t.sy).sqrt()) / 2.0;
        attrs.stroke_color = c;
        attrs.brush = Some(Brush::basic((s.width().get() * scale).round().max(1.0) as i32));
    }
    let mut e = Element::new_path(ids, b.contours, attrs);
    if let ElementKind::Path(pe) = &mut e.kind {
        pe.even_odd = even_odd;
        if let Some(h) = handles {
            pe.fill_handles = h;
        }
    }
    if !p.id().is_empty() {
        e.common.name = Some(p.id().to_string());
    }
    Some(e)
}

fn svg_image(ids: &mut IdGen, img: &usvg::Image, tile: &mut u32, warnings: &mut Vec<String>) -> Option<Element> {
    let data = match img.kind() {
        usvg::ImageKind::PNG(d) | usvg::ImageKind::JPEG(d) => d.clone(),
        _ => {
            warnings.push("eingebettete SVG-Bilder dieses Typs werden nicht übernommen".into());
            return None;
        }
    };
    let r = match crate::detect::detect_format(&data, "") {
        FileFormat::Png => crate::png_in::decode_png(&data).ok()?,
        FileFormat::Jpeg => crate::jpeg_in::decode(&data).ok()?,
        _ => return None,
    };
    let t = img.abs_transform();
    let size = img.size();
    let (w, h) = (((size.width() * t.sx).abs().round() as u32).max(1), ((size.height() * t.sy).abs().round() as u32).max(1));
    let scaled = if w != r.width() || h != r.height() { fp_raster::resample::resize(&r.image, w, h, fp_raster::Sampler::Bicubic) } else { r.image.clone() };
    let placed = scaled.translated(t.tx.round() as i32 - scaled.bounds().left, t.ty.round() as i32 - scaled.bounds().top);
    Some(bitmap_element(ids, &placed, None, tile))
}

fn svg_group(ids: &mut IdGen, g: &usvg::Group, tile: &mut u32, warnings: &mut Vec<String>, depth: u32) -> Vec<Element> {
    if depth > 64 {
        return Vec::new();
    }
    if g.clip_path().is_some() || g.mask().is_some() || !g.filters().is_empty() {
        warnings.push("SVG-Schnittpfade, Masken und Filter werden nicht übernommen".into());
    }
    let mut out = Vec::new();
    // SVG: spätere Kinder liegen oben ⇒ Reihenfolge des Originals oben → unten umkehren
    for n in g.children().iter().rev() {
        match n {
            usvg::Node::Path(p) => {
                if let Some(e) = svg_path(ids, p, warnings) {
                    out.push(e);
                }
            }
            usvg::Node::Image(i) => {
                if let Some(e) = svg_image(ids, i, tile, warnings) {
                    out.push(e);
                }
            }
            usvg::Node::Group(c) => {
                let kids = svg_group(ids, c, tile, warnings, depth + 1);
                if kids.is_empty() {
                    continue;
                }
                let op = c.opacity().get();
                if kids.len() == 1 && op >= 1.0 {
                    out.extend(kids);
                } else {
                    let mut ge = Element::new_group(ids, kids.into_iter().map(Arc::new).collect());
                    ge.common.opacity = fp_color::Opacity((op.clamp(0.0, 1.0) * 255.0).round() as u8);
                    if !c.id().is_empty() {
                        ge.common.name = Some(c.id().to_string());
                    }
                    out.push(ge);
                }
            }
            usvg::Node::Text(_) => warnings.push("SVG-Text wird nicht übernommen (ohne Textmodul)".into()),
        }
    }
    out
}

/// SVG importieren (Pfade ins Modell).
pub fn import_svg(data: &[u8], _opts: &ImportOptions) -> Result<ImportResult> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default()).map_err(|e| ImportError::Decode("SVG", e.to_string()))?;
    let size = tree.size();
    let (w, h) = (size.width().ceil().max(1.0) as u32, size.height().ceil().max(1.0) as u32);
    check_size(w, h)?;
    let mut doc = empty_document(w, h);
    let mut ids = std::mem::take(&mut doc.ids);
    let mut warnings = Vec::new();
    let mut tile = 0;
    let els = svg_group(&mut ids, tree.root(), &mut tile, &mut warnings, 0);
    doc.ids = ids;
    push_elements(&mut doc, 0, els);
    warnings.sort();
    warnings.dedup();
    Ok(ImportResult { document: doc, format: FileFormat::Svg, content: ImportedContent::Layered, warnings })
}

// ---------------------------------------------------------------- PDF / Illustrator ≥ 9
/// Systemschrift für eine der 14 PDF-Standardschriften (zur Laufzeit aus `%WINDIR%\Fonts`, nichts
/// eingebettet); nicht eingebettete sonstige Schriften über die nächstliegende Standardschrift.
fn standard_font_file(f: hayro_svg::hayro_interpret::font::StandardFont) -> Option<Vec<u8>> {
    use hayro_svg::hayro_interpret::font::StandardFont as S;
    let names: &[&str] = match f {
        S::Helvetica => &["arial.ttf", "LiberationSans-Regular.ttf"],
        S::HelveticaBold => &["arialbd.ttf", "LiberationSans-Bold.ttf"],
        S::HelveticaOblique => &["ariali.ttf", "LiberationSans-Italic.ttf"],
        S::HelveticaBoldOblique => &["arialbi.ttf", "LiberationSans-BoldItalic.ttf"],
        S::Courier => &["cour.ttf", "LiberationMono-Regular.ttf"],
        S::CourierBold => &["courbd.ttf", "LiberationMono-Bold.ttf"],
        S::CourierOblique => &["couri.ttf", "LiberationMono-Italic.ttf"],
        S::CourierBoldOblique => &["courbi.ttf", "LiberationMono-BoldItalic.ttf"],
        S::TimesRoman => &["times.ttf", "LiberationSerif-Regular.ttf"],
        S::TimesBold => &["timesbd.ttf", "LiberationSerif-Bold.ttf"],
        S::TimesItalic => &["timesi.ttf", "LiberationSerif-Italic.ttf"],
        S::TimesBoldItalic => &["timesbi.ttf", "LiberationSerif-BoldItalic.ttf"],
        S::Symbol => &["symbol.ttf"],
        S::ZapfDingBats => &["wingding.ttf"],
    };
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(w) = std::env::var_os("WINDIR") {
        dirs.push(std::path::PathBuf::from(w).join("Fonts"));
    }
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(std::path::PathBuf::from(l).join("Microsoft").join("Windows").join("Fonts"));
    }
    dirs.push("/usr/share/fonts/truetype/liberation".into());
    names.iter().flat_map(|n| dirs.iter().map(move |d| d.join(n))).find_map(|p| std::fs::read(p).ok())
}

fn pdf_settings() -> hayro_svg::hayro_interpret::InterpreterSettings {
    use hayro_svg::hayro_interpret::font::FontQuery;
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Cache = Mutex<HashMap<String, Option<hayro_svg::hayro_interpret::font::FontData>>>;
    let cache: Arc<Cache> = Arc::new(Mutex::new(HashMap::new()));
    hayro_svg::hayro_interpret::InterpreterSettings {
        font_resolver: Arc::new(move |q: &FontQuery| {
            let f = match q {
                FontQuery::Standard(s) => *s,
                FontQuery::Fallback(f) => f.pick_standard_font(),
            };
            let key = format!("{f:?}");
            let mut c = cache.lock().ok()?;
            let e = c.entry(key).or_insert_with(|| standard_font_file(f).map(|v| Arc::new(v) as hayro_svg::hayro_interpret::font::FontData));
            e.clone().map(|d| (d, 0))
        }),
        cmap_resolver: Arc::new(|_| None),
        warning_sink: Arc::new(|_| {}),
        render_annotations: true,
    }
}

/// PDF bzw. Illustrator ab Version 9 importieren: jede PDF-Seite wird über einen reinen
/// Rust-PDF-Interpreter (hayro) nach SVG übersetzt und wie SVG eingelesen (Pfade, Verläufe,
/// Bilder; Text als Glyphenpfade). Mehrseitige Dateien ⇒ eine Seite je PDF-Seite (höchstens 64).
pub fn import_pdf(data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let run = || -> Result<ImportResult> {
        let pdf = hayro_svg::hayro_syntax::Pdf::new(data.to_vec()).map_err(|e| match e {
            hayro_svg::hayro_syntax::LoadPdfError::Decryption(_) => ImportError::Unsupported("verschlüsselte PDF-Dateien werden nicht unterstützt".into()),
            _ => ImportError::Corrupt("PDF nicht lesbar".into()),
        })?;
        let settings = pdf_settings();
        let cache = hayro_svg::RenderCache::new();
        let mut result: Option<ImportResult> = None;
        for (i, page) in pdf.pages().iter().enumerate().take(64) {
            let svg = hayro_svg::convert(page, &cache, &settings, &hayro_svg::SvgRenderSettings::default());
            let r = import_svg(svg.as_bytes(), opts)?;
            match &mut result {
                None => {
                    let mut r = r;
                    r.format = FileFormat::Pdf;
                    result = Some(r);
                }
                Some(acc) => {
                    let src = &r.document.pages[0];
                    let idx = acc.document.add_page(&format!("Seite {}", i + 1));
                    let els: Vec<Arc<Element>> = src
                        .root
                        .layers
                        .iter()
                        .filter_map(|s| if let fp_model::LayerSlot::Own(l) = s { l.cels.first().map(|c| c.elements.clone()) } else { None })
                        .flatten()
                        .collect();
                    // IDs im Zieldokument neu vergeben
                    let mut ids = std::mem::take(&mut acc.document.ids);
                    let els: Vec<Element> = els.iter().map(|e| reassign_ids(&mut ids, e)).collect();
                    acc.document.ids = ids;
                    if let Some(p) = acc.document.page_mut(idx) {
                        p.canvas.width = src.canvas.width;
                        p.canvas.height = src.canvas.height;
                        p.root.background = src.root.background;
                        if let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.iter_mut().find(|s| matches!(s, fp_model::LayerSlot::Own(l) if l.kind != fp_model::LayerKind::Web))
                            && let Some(c) = Arc::make_mut(l).cels.first_mut()
                        {
                            Arc::make_mut(c).elements.extend(els.into_iter().map(Arc::new));
                        }
                    }
                    acc.warnings.extend(r.warnings);
                }
            }
        }
        let mut r = result.ok_or_else(|| ImportError::Corrupt("PDF ohne Seite".into()))?;
        r.warnings.sort();
        r.warnings.dedup();
        Ok(r)
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| Err(ImportError::Decode("PDF", "PDF-Interpreter abgebrochen (beschädigte Daten)".into())))
}

/// Element (rekursiv) mit neuen IDs aus `ids`.
fn reassign_ids(ids: &mut IdGen, e: &Element) -> Element {
    let mut n = e.clone();
    n.id = ids.element();
    if let ElementKind::Group(g) = &mut n.kind {
        g.children = g.children.iter().map(|c| Arc::new(reassign_ids(ids, c))).collect();
    }
    n
}

// ---------------------------------------------------------------- PostScript / Illustrator ≤ 8
fn bounding_box(text: &str) -> Option<[f64; 4]> {
    for line in text.lines().take(400) {
        if let Some(rest) = line.strip_prefix("%%HiResBoundingBox:").or_else(|| line.strip_prefix("%%BoundingBox:")) {
            let v: Vec<f64> = rest.split_whitespace().filter_map(|s| s.parse().ok()).collect();
            if v.len() == 4 {
                return Some([v[0], v[1], v[2], v[3]]);
            }
        }
    }
    None
}

fn is_illustrator(text: &str) -> bool {
    text.lines().take(60).any(|l| (l.starts_with("%%Creator:") && l.contains("Illustrator")) || l.starts_with("%%AI") || l.starts_with("%AI") || l.contains("AI3_") || l.contains("AI5_"))
}

/// Illustrator-PostScript (≤ 8) bzw. AI-EPS lesen.
fn import_ai_text(text: &str, opts: &ImportOptions) -> Result<ImportResult> {
    let bb = bounding_box(text).ok_or_else(|| ImportError::Corrupt("AI/EPS ohne %%BoundingBox".into()))?;
    let (w, h) = ((bb[2] - bb[0]).ceil(), (bb[3] - bb[1]).ceil());
    if !(w > 0.0 && h > 0.0) {
        return Err(ImportError::Corrupt("Begrenzungsrahmen mit Breite oder Höhe ≤ 0".into()));
    }
    check_size(w as u32, h as u32)?;
    let body_start = ["%%EndSetup", "%%EndProlog"].iter().filter_map(|m| text.find(m)).min().unwrap_or(0);
    let body = &text[body_start..];
    let map = |x: f64, y: f64| Point::new((x - bb[0]) as f32, (bb[3] - y) as f32);
    let cmyk = |c: f64, m: f64, y: f64, k: f64| -> Argb {
        let (r, g, b) = cmyk_to_rgb(postscript_cmyk_byte(c), postscript_cmyk_byte(m), postscript_cmyk_byte(y), postscript_cmyk_byte(k), opts.cmyk_table.as_deref());
        Argb(0xFF00_0000 | (r as u32) << 16 | (g as u32) << 8 | b as u32)
    };
    let gray = |g: f64| {
        let v = (g.clamp(0.0, 1.0) * 255.0).round() as u32;
        Argb(0xFF00_0000 | v << 16 | v << 8 | v)
    };
    let rgb = |r: f64, g: f64, b: f64| {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        Argb(0xFF00_0000 | c(r) << 16 | c(g) << 8 | c(b))
    };
    let mut doc = empty_document(w as u32, h as u32);
    let mut ids = std::mem::take(&mut doc.ids);
    let (mut fill, mut stroke, mut width) = (Argb(0xFF00_0000), Argb(0xFF00_0000), 1.0f64);
    let mut stack: Vec<f64> = Vec::new();
    let mut b = PathBuilder::default();
    let mut elements: Vec<Element> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut in_text = false;
    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with('%') {
            continue;
        }
        for tok in line.split_whitespace() {
            if tok.starts_with('(') || tok.starts_with('[') || tok.starts_with('/') || tok.starts_with('{') {
                stack.clear();
                continue;
            }
            if let Ok(v) = tok.parse::<f64>() {
                stack.push(v);
                continue;
            }
            let n = stack.len();
            let arg = |k: usize| stack.get(n.wrapping_sub(k)).copied().unwrap_or(0.0);
            match tok {
                "To" | "TO" => in_text = !in_text || tok == "To",
                "m" if n >= 2 => b.move_to(map(arg(2), arg(1))),
                "l" | "L" if n >= 2 => b.line_to(map(arg(2), arg(1))),
                "c" | "C" if n >= 6 => b.cubic_to(map(arg(6), arg(5)), map(arg(4), arg(3)), map(arg(2), arg(1))),
                "v" | "V" if n >= 4 => {
                    let p0 = b.current().unwrap_or_else(|| map(arg(4), arg(3)));
                    b.cubic_to(p0, map(arg(4), arg(3)), map(arg(2), arg(1)));
                }
                "y" | "Y" if n >= 4 => {
                    let end = map(arg(2), arg(1));
                    b.cubic_to(map(arg(4), arg(3)), end, end);
                }
                "h" | "H" => b.flush(true),
                "f" | "F" | "s" | "S" | "b" | "B" | "n" | "N" | "B*" | "b*" | "f*" | "F*" => {
                    let close = tok.chars().next().is_some_and(|c| c.is_ascii_lowercase());
                    b.flush(close);
                    let contours = std::mem::take(&mut b.contours);
                    let do_fill = matches!(tok.chars().next(), Some('f' | 'F' | 'b' | 'B'));
                    let do_stroke = matches!(tok.chars().next(), Some('s' | 'S' | 'b' | 'B'));
                    if !contours.is_empty() && (do_fill || do_stroke) && !in_text {
                        let mut attrs = PathAttrs::default();
                        if do_fill {
                            attrs.fill_color = fill;
                            attrs.fill = Some(fp_model::Fill::solid());
                        }
                        if do_stroke {
                            attrs.stroke_color = stroke;
                            attrs.brush = Some(Brush::basic(width.round().max(1.0) as i32));
                        }
                        let mut e = Element::new_path(&mut ids, contours, attrs);
                        if let ElementKind::Path(p) = &mut e.kind {
                            p.even_odd = tok.ends_with('*');
                        }
                        elements.push(e);
                    }
                }
                "g" if n >= 1 => fill = gray(arg(1)),
                "G" if n >= 1 => stroke = gray(arg(1)),
                "k" if n >= 4 => fill = cmyk(arg(4), arg(3), arg(2), arg(1)),
                "K" if n >= 4 => stroke = cmyk(arg(4), arg(3), arg(2), arg(1)),
                "x" if n >= 5 => fill = cmyk(arg(5) * arg(1), arg(4) * arg(1), arg(3) * arg(1), arg(2) * arg(1)),
                "X" if n >= 5 => stroke = cmyk(arg(5) * arg(1), arg(4) * arg(1), arg(3) * arg(1), arg(2) * arg(1)),
                "Xa" if n >= 3 => fill = rgb(arg(3), arg(2), arg(1)),
                "XA" if n >= 3 => stroke = rgb(arg(3), arg(2), arg(1)),
                "w" if n >= 1 => width = arg(1),
                "Tx" | "Tj" | "TX" => warnings.push("Illustrator-Text wird nicht übernommen".into()),
                "XI" | "XG" | "%%BeginRaster" => warnings.push("platzierte Bilder/Raster werden nicht übernommen".into()),
                _ => {}
            }
            if !matches!(tok, "Xa" | "XA") || n >= 3 {
                stack.clear();
            }
        }
    }
    elements.reverse();
    doc.ids = ids;
    push_elements(&mut doc, 0, elements);
    if let Some(p) = doc.page_mut(0)
        && let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0)
    {
        Arc::make_mut(l).name = "Ebene 1".into();
    }
    warnings.sort();
    warnings.dedup();
    Ok(ImportResult { document: doc, format: FileFormat::Illustrator, content: ImportedContent::Layered, warnings })
}

/// Illustrator ≤ 8, AI-EPS oder DOS-EPS mit TIFF-Vorschau.
pub fn import_postscript(data: &[u8], fmt: FileFormat, opts: &ImportOptions) -> Result<ImportResult> {
    let (ps, tiff) = if fmt == FileFormat::EpsBinary && data.len() >= 30 {
        let u = |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize;
        let (po, pl, to, tl) = (u(4), u(8), u(20), u(24));
        (data.get(po..po.saturating_add(pl)).unwrap_or(&[]), if tl > 0 { data.get(to..to.saturating_add(tl)) } else { None })
    } else {
        (data, None)
    };
    if ps.starts_with(b"%PDF") {
        return import_pdf(ps, opts);
    }
    let text = String::from_utf8_lossy(ps);
    if is_illustrator(&text) || fmt == FileFormat::Illustrator {
        let mut r = import_ai_text(&text, opts)?;
        r.format = fmt;
        return Ok(r);
    }
    if let Some(t) = tiff {
        let img = crate::tiff_in::decode(t, opts.cmyk_table.as_deref())?;
        let mut r = ImportResult { document: document_from_raster(img, FileFormat::Eps), format: FileFormat::Eps, content: ImportedContent::Bitmap, warnings: vec![] };
        r.warnings.push("EPS ohne Illustrator-Daten: nur die eingebettete TIFF-Vorschau wurde übernommen (kein PostScript-Interpreter)".into());
        if let Some(p) = r.document.page_mut(0)
            && let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0)
        {
            Arc::make_mut(l).name = "Ebene 1".into();
        }
        return Ok(r);
    }
    Err(ImportError::Unsupported("EPS ohne Illustrator-Daten kann nicht gelesen werden: Das Original rastert sie mit seinem PostScript-Modul (MPS.dll), das in Firepixel nicht enthalten ist. Bitte als Illustrator-8-EPS, SVG oder Pixelbild speichern.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai8_rectangle() {
        let ai = "%!PS-3.0\n%%Creator: Illustrator(R) 8.0\n%%BoundingBox: 0 0 100 50\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n";
        let r = import_postscript(ai.as_bytes(), FileFormat::Illustrator, &ImportOptions::default()).unwrap();
        let p = &r.document.pages[0];
        assert_eq!((p.canvas.width, p.canvas.height), (100.0, 50.0));
        let fp_model::LayerSlot::Own(l) = &p.root.layers[0] else { panic!() };
        let e = &l.cels[0].elements[0];
        let ElementKind::Path(pe) = &e.kind else { panic!() };
        assert_eq!(pe.contours[0].nodes.len(), 4);
        assert!(pe.contours[0].closed);
        assert_eq!(pe.contours[0].nodes[0].anchor, Point::new(10.0, 40.0));
        assert!(import_postscript(b"%!PS-3.0 EPSF-3.0\n%%BoundingBox: 0 0 1 1\n", FileFormat::Eps, &ImportOptions::default()).is_err());
    }

    #[test]
    fn svg_paths() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect x="2" y="3" width="10" height="5" fill="#ff0000"/><g opacity="0.5"><circle cx="30" cy="10" r="5" fill="blue" stroke="black" stroke-width="2"/><path d="M0 0 L5 5"/></g></svg>"##;
        let r = import_svg(svg, &ImportOptions::default()).unwrap();
        let p = &r.document.pages[0];
        assert_eq!((p.canvas.width, p.canvas.height), (40.0, 20.0));
        let fp_model::LayerSlot::Own(l) = &p.root.layers[0] else { panic!() };
        assert_eq!(l.cels[0].elements.len(), 2);
        let ElementKind::Path(rect) = &l.cels[0].elements[1].kind else { panic!() };
        assert_eq!(rect.attrs.fill_color, Argb(0xFFFF_0000));
        assert!(import_svg(b"<svg", &ImportOptions::default()).is_err());
    }
}
