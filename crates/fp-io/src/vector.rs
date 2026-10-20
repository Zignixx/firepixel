//! Vektorformate: SVG (über usvg → Pfade, Gruppen, Bilder; ARCHITEKTUR §5) sowie Illustrator ≤ 8 und
//! EPS, soweit 12 §4 sie beschreibt (PostScript-Teilmenge der AI-Formate).
//!
//! * **SVG:** Pfade mit Vollton-Füllung/-Strich (Deckkraft als Farb-Alpha, Füllregel), Gruppen mit
//!   Deckkraft, eingebettete PNG/JPEG-Bilder; lineare/radiale Verläufe als Verlaufsfüllung (Form 2/3/9 mit
//!   Griffen und Farb-/Deckkraftstopps), Verlaufsstriche als erste Farbe (A),
//!   Muster, Filter und Masken entfallen mit Hinweis; `<text>` wird Textobjekt (eigener Leser, Q-13).
//! * **Illustrator ≤ 8 / AI-EPS:** Leinwand = `%%BoundingBox`, Pfadoperatoren `m l L c C v V y Y h H`,
//!   Malen `f F s S b B n N`, Farben `g G k K x X Xa XA`, Strichbreite `w`. CMYK über die
//!   CMYK-Tabelle des Originals bzw. naiv. Text (`To` … `TO`) wird Textobjekt (Punkt-/Flächentext, TW-25);
//!   platzierte Bilder und Muster entfallen mit Hinweis.
//! * **EPS ohne Illustrator-Daten:** nur die TIFF-Vorschau eines DOS-EPS (das Original rastert mit MPS.dll,
//!   die nicht nachgebaut wird); sonst klare Fehlermeldung.
//! * **PDF / AI ≥ 9:** über den reinen Rust-PDF-Interpreter hayro nach SVG und dann wie SVG
//!   ([`import_pdf`]); das Original nutzt dafür sein Illustrator-Modul (12 §4.5), Ergebnis daher
//!   strukturell abweichend (A): Text als Glyphenpfade, eine Seite je PDF-Seite.

use std::sync::Arc;

use fp_color::Argb;
use fp_color::cmyk::{cmyk_to_rgb, postscript_cmyk_byte};
use fp_model::{Brush, Contour, ContourNode, Element, ElementKind, IdGen, PathAttrs, Point};
use usvg::roxmltree;

use crate::build::{
    ImportResult, ImportedContent, bitmap_element, document_from_raster, empty_document,
    push_elements,
};
use crate::detect::FileFormat;
use crate::{ImportError, ImportOptions, Result, check_size};

/// Früher: Begründung für den fehlenden PDF-Import (bleibt für ältere Aufrufer erhalten).
#[deprecated(note = "PDF wird jetzt über import_pdf gelesen")]
pub const PDF_NOTE: &str =
    "PDF und AI ab Version 9 werden über einen PDF-Interpreter als Vektoren gelesen.";

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
        self.contours.push(Contour {
            closed,
            bsl: None,
            nodes,
            extra: Default::default(),
        });
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
        usvg::Paint::Color(c) => Some(Argb(
            a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32,
        )),
        usvg::Paint::LinearGradient(g) => stop_color(g.stops(), opacity, warnings),
        usvg::Paint::RadialGradient(g) => stop_color(g.stops(), opacity, warnings),
        usvg::Paint::Pattern(_) => {
            warnings.push(fp_i18n::t("msg-import-warn-svg-pattern"));
            None
        }
    }
}

/// Nur noch für Striche (das Original kennt keine Verlaufsstriche).
fn stop_color(stops: &[usvg::Stop], opacity: f32, warnings: &mut Vec<String>) -> Option<Argb> {
    warnings.push(fp_i18n::t("msg-import-warn-svg-gradient-stroke"));
    let s = stops.first()?;
    let c = s.color();
    let a = ((opacity * s.opacity().get()).clamp(0.0, 1.0) * 255.0).round() as u32;
    Some(Argb(
        a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32,
    ))
}

fn argb_of(c: usvg::Color, a: f32) -> Argb {
    let a = (a.clamp(0.0, 1.0) * 255.0).round() as u32;
    Argb(a << 24 | (c.red as u32) << 16 | (c.green as u32) << 8 | c.blue as u32)
}

/// Verlaufsfüllung aus einem SVG-Verlauf: linear ⇒ Form 2, radial ⇒ Form 3 (kreisrund) bzw. 9
/// (elliptisch, bei nicht gleichmäßiger Verzerrung). Griffe in Dokumentkoordinaten (Verlaufs- und
/// Pfadtransformation). Farbstopps aus `stop-color`, Deckkraftstopps aus `stop-opacity` × Deckkraft.
/// Nicht abbildbar (A): Brennpunkt ≠ Mittelpunkt, `spreadMethod` reflect/repeat (Original: nur „pad“).
fn svg_gradient(
    p: &usvg::Paint,
    opacity: f32,
    path_t: &usvg::Transform,
    warnings: &mut Vec<String>,
) -> Option<(fp_model::Fill, fp_model::FillHandles)> {
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
                warnings.push(fp_i18n::t("msg-import-warn-svg-gradient-repeat"));
            }
            let (a, b) = (tf(&t, g.x1(), g.y1()), tf(&t, g.x2(), g.y2()));
            let perp = Point::new(a.x - (b.y - a.y), a.y + (b.x - a.x));
            (2, "fc_Linear", "Linear", a, b, perp)
        }
        usvg::Paint::RadialGradient(g) => {
            if (g.fx() - g.cx()).abs() > 1e-3 || (g.fy() - g.cy()).abs() > 1e-3 {
                warnings.push(fp_i18n::t("msg-import-warn-svg-gradient-focus"));
            }
            let r = g.r().get();
            let c = tf(&t, g.cx(), g.cy());
            let ex = tf(&t, g.cx() + r, g.cy());
            let ey = tf(&t, g.cx(), g.cy() + r);
            let (lx, ly) = (
                (ex.x - c.x).hypot(ex.y - c.y),
                (ey.x - c.x).hypot(ey.y - c.y),
            );
            let dot = (ex.x - c.x) * (ey.x - c.x) + (ex.y - c.y) * (ey.y - c.y);
            let round = (lx - ly).abs() < 1e-3 * lx.max(1.0) && dot.abs() < 1e-3 * lx * ly.max(1.0);
            if round {
                (3, "fc_Circular", "Radial", c, ex, ey)
            } else {
                (9, "fc_Elliptical", "Elliptical", c, ex, ey)
            }
        }
        _ => return None,
    };
    let _ = kind;
    let colors: Vec<GradientStop> = stops
        .iter()
        .map(|s| GradientStop {
            position: s.offset().get(),
            color: argb_of(s.color(), 1.0),
            extra: Default::default(),
        })
        .collect();
    let opac: Vec<GradientStop> = stops
        .iter()
        .map(|s| GradientStop {
            position: s.offset().get(),
            color: Argb(
                (((s.opacity().get() * opacity).clamp(0.0, 1.0) * 255.0).round() as u32) << 24
                    | 0x00FF_FFFF,
            ),
            extra: Default::default(),
        })
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
        stops: Some(GradientStops {
            colors: StopList {
                stops: colors,
                extra: Default::default(),
            },
            opacities: StopList {
                stops: opac,
                extra: Default::default(),
            },
            dither: None,
            dither_value: None,
            extra: Default::default(),
        }),
        extra: Default::default(),
    });
    Some((
        f,
        FillHandles {
            valid: true,
            start,
            end,
            second,
        },
    ))
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
                let c1 = Point::new(
                    p0.x + 2.0 / 3.0 * (q.x - p0.x),
                    p0.y + 2.0 / 3.0 * (q.y - p0.y),
                );
                let c2 = Point::new(a.x + 2.0 / 3.0 * (q.x - a.x), a.y + 2.0 / 3.0 * (q.y - a.y));
                b.cubic_to(c1, c2, a);
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(c1, c2, a) => {
                b.cubic_to(tf(&t, c1.x, c1.y), tf(&t, c2.x, c2.y), tf(&t, a.x, a.y))
            }
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
        attrs.brush = Some(Brush::basic(
            (s.width().get() * scale).round().max(1.0) as i32
        ));
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

fn svg_image(
    ids: &mut IdGen,
    img: &usvg::Image,
    tile: &mut u32,
    warnings: &mut Vec<String>,
) -> Option<Element> {
    let data = match img.kind() {
        usvg::ImageKind::PNG(d) | usvg::ImageKind::JPEG(d) => d.clone(),
        _ => {
            warnings.push(fp_i18n::t("msg-import-warn-svg-image"));
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
    let (w, h) = (
        ((size.width() * t.sx).abs().round() as u32).max(1),
        ((size.height() * t.sy).abs().round() as u32).max(1),
    );
    let scaled = if w != r.width() || h != r.height() {
        fp_raster::resample::resize(&r.image, w, h, fp_raster::Sampler::Bicubic)
    } else {
        r.image.clone()
    };
    let placed = scaled.translated(
        t.tx.round() as i32 - scaled.bounds().left,
        t.ty.round() as i32 - scaled.bounds().top,
    );
    Some(bitmap_element(ids, &placed, None, tile))
}

/// Platzhalter-ID eines `<text>` im vorbereiteten SVG (Stapelfolge, BA-076).
const TEXT_MARK: &str = "__fp_text_";

fn svg_group(
    ids: &mut IdGen,
    g: &usvg::Group,
    tile: &mut u32,
    warnings: &mut Vec<String>,
    texts: &mut std::collections::HashMap<usize, Element>,
    depth: u32,
) -> Vec<Element> {
    if depth > 64 {
        return Vec::new();
    }
    if g.clip_path().is_some() || g.mask().is_some() || !g.filters().is_empty() {
        warnings.push(fp_i18n::t("msg-import-warn-svg-clip"));
    }
    let mut out = Vec::new();
    // SVG: spätere Kinder liegen oben ⇒ Reihenfolge des Originals oben → unten umkehren
    for n in g.children().iter().rev() {
        svg_node(ids, n, tile, warnings, texts, depth, &mut out);
    }
    out
}

/// Ein Knoten eines SVG-Baums als Elemente (oben → unten) an `out` anhängen.
fn svg_node(
    ids: &mut IdGen,
    n: &usvg::Node,
    tile: &mut u32,
    warnings: &mut Vec<String>,
    texts: &mut std::collections::HashMap<usize, Element>,
    depth: u32,
    out: &mut Vec<Element>,
) {
    {
        match n {
            usvg::Node::Path(p) => {
                // Platzhalter eines Texts: das Textobjekt an dieser Stelle der Stapelfolge
                if let Some(k) = p
                    .id()
                    .strip_prefix(TEXT_MARK)
                    .and_then(|k| k.parse::<usize>().ok())
                {
                    if let Some(t) = texts.remove(&k) {
                        out.push(t);
                    }
                    return;
                }
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
                let kids = svg_group(ids, c, tile, warnings, texts, depth + 1);
                if kids.is_empty() {
                    return;
                }
                let op = c.opacity().get();
                if kids.len() == 1 && op >= 1.0 {
                    out.extend(kids);
                } else {
                    let mut ge = Element::new_group(ids, kids.into_iter().map(Arc::new).collect());
                    ge.common.opacity =
                        fp_color::Opacity((op.clamp(0.0, 1.0) * 255.0).round() as u8);
                    if !c.id().is_empty() {
                        ge.common.name = Some(c.id().to_string());
                    }
                    out.push(ge);
                }
            }
            // Text liest svg_texts selbst (usvg ohne Textmodul liefert keine Textknoten)
            usvg::Node::Text(_) => {}
        }
    }
}

/// Ebenen einer SVG-Datei (K-9, re 37 §6): die obersten `<g>` mit `id` unter `<svg>`, Name aus
/// `inkscape:label`, sonst die `id`. Ergebnis: `id` → Ebenenname.
fn svg_layer_names(data: &[u8]) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let unz;
    let data = if data.starts_with(&[0x1f, 0x8b]) {
        let Ok(d) = usvg::decompress_svgz(data) else {
            return out;
        };
        unz = d;
        &unz[..]
    } else {
        data
    };
    let Ok(s) = std::str::from_utf8(data) else {
        return out;
    };
    let opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let Ok(xml) = roxmltree::Document::parse_with_options(s, opt) else {
        return out;
    };
    for g in xml
        .root_element()
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "g")
    {
        let Some(id) = g.attribute("id").filter(|i| !i.is_empty()) else {
            continue;
        };
        let label = g
            .attribute(("http://www.inkscape.org/namespaces/inkscape", "label"))
            .filter(|l| !l.trim().is_empty())
            .unwrap_or(id);
        out.insert(id.to_string(), label.to_string());
    }
    out
}

/// Ein Ebenen-Abschnitt: Name aus der Datei (unbenannt `None`) und Elemente oben → unten.
type LayerChunk = (Option<String>, Vec<Element>);

/// Ebenen-Abschnitte eines SVG-Baums (unten → oben): jede oberste benannte Gruppe
/// ([`svg_layer_names`]) ist eine Ebene, Objekte dazwischen bilden unbenannte Abschnitte.
/// `None`, wenn es keine benannte Gruppe gibt.
fn svg_layer_chunks(
    ids: &mut IdGen,
    root: &usvg::Group,
    names: &std::collections::HashMap<String, String>,
    tile: &mut u32,
    warnings: &mut Vec<String>,
    texts: &mut std::collections::HashMap<usize, Element>,
) -> Option<Vec<LayerChunk>> {
    if names.is_empty() {
        return None;
    }
    // usvg legt bei viewBox eine unbenannte Hülle um den Inhalt
    let mut g = root;
    while let [usvg::Node::Group(c)] = g.children()
        && c.id().is_empty()
        && c.opacity().get() >= 1.0
        && c.clip_path().is_none()
        && c.mask().is_none()
        && c.filters().is_empty()
    {
        g = c;
    }
    if !g
        .children()
        .iter()
        .any(|n| matches!(n, usvg::Node::Group(c) if names.contains_key(c.id())))
    {
        return None;
    }
    let mut chunks: Vec<LayerChunk> = Vec::new();
    for n in g.children() {
        if let usvg::Node::Group(c) = n
            && let Some(name) = names.get(c.id())
        {
            if c.clip_path().is_some() || c.mask().is_some() || !c.filters().is_empty() {
                warnings.push(fp_i18n::t("msg-import-warn-svg-clip"));
            }
            let mut kids = svg_group(ids, c, tile, warnings, texts, 1);
            let op = c.opacity().get();
            if op < 1.0 && !kids.is_empty() {
                let mut ge = Element::new_group(ids, kids.into_iter().map(Arc::new).collect());
                ge.common.opacity = fp_color::Opacity((op.clamp(0.0, 1.0) * 255.0).round() as u8);
                kids = vec![ge];
            }
            chunks.push((Some(name.clone()), kids));
        } else {
            let mut els = Vec::new();
            svg_node(ids, n, tile, warnings, texts, 1, &mut els);
            match chunks.last_mut() {
                Some((None, v)) => {
                    els.append(v);
                    *v = els;
                }
                _ => chunks.push((None, els)),
            }
        }
    }
    Some(chunks)
}

/// Ebenen-Abschnitte (unten → oben) als Ebenen der ersten Seite: bei mehreren je eine Ebene mit
/// dem Namen aus der Datei (unbenannt „Ebene n“), sonst alles in „Ebene 1“ (AI wie SVG, U-1/K-9).
fn push_layer_chunks(doc: &mut fp_model::Document, mut ids: IdGen, mut chunks: Vec<LayerChunk>) {
    chunks.retain(|(_, e)| !e.is_empty());
    if chunks.len() > 1 {
        let mut at = 0;
        for (k, (name, els)) in chunks.into_iter().enumerate() {
            let label = name
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| fp_i18n::tr!("msg-default-layer", n = (k + 1).to_string()));
            if k == 0 {
                if let Some(p) = doc.page_mut(0)
                    && let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0)
                {
                    Arc::make_mut(l).name = label;
                }
            } else if let Some(p) = doc.page_mut(0) {
                at += 1;
                let l = fp_model::Layer::new(&mut ids, &label, fp_model::LayerKind::Normal, 1);
                p.root
                    .layers
                    .insert(at, fp_model::LayerSlot::Own(Arc::new(l)));
            }
            doc.ids = std::mem::take(&mut ids);
            push_elements(doc, at, els);
            ids = std::mem::take(&mut doc.ids);
        }
        doc.ids = ids;
    } else {
        // ANNAHME (ungemessen, wie der AI-Leser, U-1): eine einzige Ebene der Datei legt keine
        // eigene, benannte Ebene an; der Inhalt landet wie ohne Ebenen in „Ebene 1“
        let elements: Vec<Element> = chunks.into_iter().flat_map(|(_, e)| e).collect();
        doc.ids = ids;
        push_elements(doc, 0, elements);
        if let Some(p) = doc.page_mut(0)
            && let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0)
        {
            Arc::make_mut(l).name = fp_i18n::tr!("msg-default-layer", n = "1");
        }
    }
}

/// Geerbte Darstellungsattribute für SVG-Text.
#[derive(Clone)]
struct SvgTextStyle {
    t: svgtypes::Transform,
    fill: Option<Argb>,
    family: String,
    size: f64,
    bold: bool,
    italic: bool,
    anchor: i32,
}

/// Wert einer Eigenschaft aus Attribut oder `style="…"`.
fn svg_prop<'a>(n: &roxmltree::Node<'a, 'a>, name: &str) -> Option<String> {
    if let Some(style) = n.attribute("style") {
        for decl in style.split(';') {
            if let Some((k, v)) = decl.split_once(':')
                && k.trim() == name
            {
                return Some(v.trim().to_string());
            }
        }
    }
    n.attribute(name).map(|v| v.trim().to_string())
}

/// Erste Zahl einer Längenangabe in Pixeln (`18`, `18px`, `12pt`, `1em` relativ zu `em`).
fn svg_len(v: &str, em: f64) -> Option<f64> {
    let v = v.split([' ', ',']).find(|s| !s.is_empty())?;
    let num_end = v
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e'))
        .unwrap_or(v.len());
    let x: f64 = v[..num_end].parse().ok()?;
    Some(match &v[num_end..] {
        "pt" => x * 4.0 / 3.0,
        "pc" => x * 16.0,
        "mm" => x * 96.0 / 25.4,
        "cm" => x * 96.0 / 2.54,
        "in" => x * 96.0,
        "em" => x * em,
        "%" => x / 100.0 * em,
        _ => x,
    })
}

/// SVG-`<text>` als Textobjekte (Q-13): Inhalt (inkl. `<tspan>`), Schrift, Größe, Fett/Kursiv, Füllfarbe,
/// Ausrichtung und Lage (Grundlinie bei `x`/`y`, alle Transformationen bis zur Leinwand). Oben → unten.
/// Texte in Dokumentreihenfolge mit dem Bytebereich ihres `<text>` im (entpackten) Quelltext,
/// dazu der entpackte Quelltext selbst (für die Platzhalter, BA-076).
fn svg_texts(
    data: &[u8],
    ids: &mut IdGen,
    warnings: &mut Vec<String>,
) -> (Vec<(std::ops::Range<usize>, Element)>, Option<String>) {
    let unz;
    let data = if data.starts_with(&[0x1f, 0x8b]) {
        let Ok(d) = usvg::decompress_svgz(data) else {
            return (Vec::new(), None);
        };
        unz = d;
        &unz[..]
    } else {
        data
    };
    let Ok(s) = std::str::from_utf8(data) else {
        return (Vec::new(), None);
    };
    let opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };
    let Ok(xml) = roxmltree::Document::parse_with_options(s, opt) else {
        return (Vec::new(), None);
    };
    let root = xml.root_element();
    // viewBox ⇒ Leinwand (gleichmäßig wie usvg bei preserveAspectRatio „xMidYMid meet“)
    let mut t0 = svgtypes::Transform::default();
    if let Some(vb) = root
        .attribute("viewBox")
        .and_then(|v| v.parse::<svgtypes::ViewBox>().ok())
    {
        let w = root
            .attribute("width")
            .and_then(|v| svg_len(v, 16.0))
            .unwrap_or(vb.w);
        let h = root
            .attribute("height")
            .and_then(|v| svg_len(v, 16.0))
            .unwrap_or(vb.h);
        if vb.w > 0.0 && vb.h > 0.0 {
            let sc = (w / vb.w).min(h / vb.h);
            let (tx, ty) = (
                (w - vb.w * sc) / 2.0 - vb.x * sc,
                (h - vb.h * sc) / 2.0 - vb.y * sc,
            );
            t0 = svgtypes::Transform::new(sc, 0.0, 0.0, sc, tx, ty);
        }
    }
    let style0 = SvgTextStyle {
        t: t0,
        fill: Some(Argb(0xFF00_0000)),
        family: "Times New Roman".into(),
        size: 16.0,
        bold: false,
        italic: false,
        anchor: 0,
    };
    let mut out = Vec::new();
    svg_text_walk(root, &style0, ids, warnings, &mut out, 0);
    (out, Some(s.to_string()))
}

fn svg_text_style(n: &roxmltree::Node, parent: &SvgTextStyle) -> SvgTextStyle {
    let mut st = parent.clone();
    if let Some(tr) = n
        .attribute("transform")
        .and_then(|v| v.parse::<svgtypes::Transform>().ok())
    {
        let p = parent.t;
        st.t = svgtypes::Transform::new(
            p.a * tr.a + p.c * tr.b,
            p.b * tr.a + p.d * tr.b,
            p.a * tr.c + p.c * tr.d,
            p.b * tr.c + p.d * tr.d,
            p.a * tr.e + p.c * tr.f + p.e,
            p.b * tr.e + p.d * tr.f + p.f,
        );
    }
    if let Some(f) = svg_prop(n, "fill") {
        st.fill = if f == "none" {
            None
        } else {
            f.parse::<svgtypes::Color>()
                .ok()
                .map(|c| {
                    Argb(
                        (c.alpha as u32) << 24
                            | (c.red as u32) << 16
                            | (c.green as u32) << 8
                            | c.blue as u32,
                    )
                })
                .or(st.fill)
        };
    }
    if let Some(f) = svg_prop(n, "font-family")
        && let Some(first) = f.split(',').next()
    {
        let fam = first.trim().trim_matches(|c| c == '\'' || c == '"');
        st.family = match fam {
            "sans-serif" => "Arial".into(),
            "serif" => "Times New Roman".into(),
            "monospace" => "Courier New".into(),
            "" => st.family,
            f => f.to_string(),
        };
    }
    if let Some(v) = svg_prop(n, "font-size").and_then(|v| svg_len(&v, parent.size)) {
        st.size = v;
    }
    if let Some(w) = svg_prop(n, "font-weight") {
        st.bold = w == "bold" || w == "bolder" || w.parse::<u32>().is_ok_and(|x| x >= 600);
    }
    if let Some(s) = svg_prop(n, "font-style") {
        st.italic = s == "italic" || s == "oblique";
    }
    if let Some(a) = svg_prop(n, "text-anchor") {
        st.anchor = match a.as_str() {
            "middle" => 1,
            "end" => 2,
            _ => 0,
        };
    }
    st
}

fn svg_text_walk(
    n: roxmltree::Node,
    parent: &SvgTextStyle,
    ids: &mut IdGen,
    warnings: &mut Vec<String>,
    out: &mut Vec<(std::ops::Range<usize>, Element)>,
    depth: u32,
) {
    if depth > 64 || svg_prop(&n, "display").as_deref() == Some("none") {
        return;
    }
    let name = n.tag_name().name();
    if matches!(
        name,
        "defs" | "symbol" | "clipPath" | "mask" | "pattern" | "marker" | "style" | "script"
    ) {
        return;
    }
    let st = svg_text_style(&n, parent);
    if name != "text" {
        for c in n.children().filter(|c| c.is_element()) {
            svg_text_walk(c, &st, ids, warnings, out, depth + 1);
        }
        return;
    }
    // Inhalt: Textknoten aller Nachfahren, Leerraum wie `xml:space="default"` zusammengefasst
    let mut raw = String::new();
    for d in n.descendants() {
        if d.is_text() {
            raw.push_str(d.text().unwrap_or(""));
        }
        if d.is_element() && d.tag_name().name() == "textPath" {
            warnings.push(fp_i18n::t("msg-import-warn-svg-text"));
        }
    }
    let text = raw
        .replace(['\n', '\r', '\t'], " ")
        .split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return;
    }
    // erster gesetzter Ursprung (auch aus dem ersten `<tspan>`), Stil des ersten `<tspan>` mit Text
    let first_span = n.descendants().find(|d| {
        d.is_element()
            && d.tag_name().name() == "tspan"
            && d.text().is_some_and(|t| !t.trim().is_empty())
    });
    let span_st = first_span.map(|s| {
        let mut chain: Vec<roxmltree::Node> = s.ancestors().take_while(|a| *a != n).collect();
        chain.reverse();
        chain
            .iter()
            .fold(st.clone(), |acc, a| svg_text_style(a, &acc))
    });
    let st2 = span_st.as_ref().unwrap_or(&st);
    let pick = |k: &str| {
        n.attribute(k)
            .or_else(|| first_span.and_then(|s| s.attribute(k)))
            .and_then(|v| svg_len(v, st.size))
            .unwrap_or(0.0)
    };
    let (x, y) = (pick("x"), pick("y"));
    let t = st.t;
    let p = Point::new(
        (t.a * x + t.c * y + t.e) as f32,
        (t.b * x + t.d * y + t.f) as f32,
    );
    let scale = (t.a * t.d - t.b * t.c).abs().sqrt();
    let size = (st2.size * if scale > 0.0 { scale } else { 1.0 }).max(1.0) as f32;
    let color = st2.fill.unwrap_or(Argb(0xFF00_0000));
    let mut e = Element::new_text(ids, &text, &st2.family, size, color, p);
    if let ElementKind::Text(te) = &mut e.kind {
        for a in std::iter::once(&mut te.initial).chain(te.runs.iter_mut().map(|r| &mut r.attrs)) {
            a.bold = Some(st2.bold);
            a.italic = Some(st2.italic);
            a.alignment = Some(st2.anchor);
        }
    }
    if let Some(id) = n.attribute("id") {
        e.common.name = Some(id.to_string());
    }
    out.push((n.range(), e));
}

/// SVG importieren (Pfade ins Modell); oberste benannte `<g>` werden Ebenen (K-9).
pub fn import_svg(data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    import_svg_with(data, opts, true)
}

/// SVG importieren; `layers` = oberste benannte Gruppen als Ebenen (aus beim PDF-Weg über hayro,
/// dessen SVG keine Ebenen der Datei trägt).
fn import_svg_with(data: &[u8], _opts: &ImportOptions, layers: bool) -> Result<ImportResult> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default())
        .map_err(|e| ImportError::Decode("SVG", e.to_string()))?;
    let size = tree.size();
    let (w, h) = (
        size.width().ceil().max(1.0) as u32,
        size.height().ceil().max(1.0) as u32,
    );
    check_size(w, h)?;
    let mut doc = empty_document(w, h);
    let mut ids = std::mem::take(&mut doc.ids);
    let mut warnings = Vec::new();
    let mut tile = 0;
    // Texte liest svg_texts (usvg ohne Textmodul liefert keine Textknoten). Für die Stapelfolge
    // ersetzt ein unsichtbarer Platzhalterpfad mit eigener ID jedes `<text>`; usvg ordnet ihn wie
    // den Text ein, svg_group setzt dort das Textobjekt ein (BA-076). Nicht eingesetzte Texte
    // (Platzhalter verworfen) liegen wie bisher oben.
    let (found, src) = svg_texts(data, &mut ids, &mut warnings);
    let marked = src.filter(|_| !found.is_empty()).map(|s| {
        let mut m = String::with_capacity(s.len());
        let mut at = 0;
        for (k, (r, _)) in found.iter().enumerate() {
            if r.start < at {
                continue;
            }
            m.push_str(&s[at..r.start]);
            m.push_str(&format!("<path id=\"{TEXT_MARK}{k}\" d=\"M0 0h1v1h-1z\" fill=\"#000\" stroke=\"none\" opacity=\"1\"/>"));
            at = r.end;
        }
        m.push_str(&s[at..]);
        m
    });
    let mut texts: std::collections::HashMap<usize, Element> =
        found.into_iter().map(|(_, e)| e).enumerate().collect();
    let marked_tree =
        marked.and_then(|m| usvg::Tree::from_data(m.as_bytes(), &usvg::Options::default()).ok());
    let root = marked_tree.as_ref().unwrap_or(&tree).root();
    let names = if layers {
        svg_layer_names(data)
    } else {
        Default::default()
    };
    let mut chunks =
        match svg_layer_chunks(&mut ids, root, &names, &mut tile, &mut warnings, &mut texts) {
            Some(c) => c,
            None => vec![(
                None,
                svg_group(&mut ids, root, &mut tile, &mut warnings, &mut texts, 0),
            )],
        };
    let mut rest: Vec<(usize, Element)> = texts.into_iter().collect();
    rest.sort_by_key(|(k, _)| std::cmp::Reverse(*k));
    // nicht eingesetzte Texte liegen oben (oberster Abschnitt)
    if let Some((_, top)) = chunks.last_mut() {
        top.splice(0..0, rest.into_iter().map(|(_, e)| e));
    }
    push_layer_chunks(&mut doc, ids, chunks);
    warnings.sort();
    warnings.dedup();
    Ok(ImportResult {
        document: doc,
        format: FileFormat::Svg,
        content: ImportedContent::Layered,
        warnings,
    })
}

// ---------------------------------------------------------------- PDF / Illustrator ≥ 9
/// Systemschrift für eine der 14 PDF-Standardschriften (zur Laufzeit aus `%WINDIR%\Fonts`, nichts
/// eingebettet); nicht eingebettete sonstige Schriften über die nächstliegende Standardschrift.
fn standard_font_file(f: hayro_svg::hayro_interpret::font::StandardFont) -> Option<Vec<u8>> {
    use hayro_svg::hayro_interpret::font::StandardFont as S;
    let names: &[&str] = match f {
        S::Helvetica => &["arial.ttf", "LiberationSans-Regular.ttf", "Arial.ttf"],
        S::HelveticaBold => &["arialbd.ttf", "LiberationSans-Bold.ttf", "Arial Bold.ttf"],
        S::HelveticaOblique => &[
            "ariali.ttf",
            "LiberationSans-Italic.ttf",
            "Arial Italic.ttf",
        ],
        S::HelveticaBoldOblique => &[
            "arialbi.ttf",
            "LiberationSans-BoldItalic.ttf",
            "Arial Bold Italic.ttf",
        ],
        S::Courier => &["cour.ttf", "LiberationMono-Regular.ttf", "Courier New.ttf"],
        S::CourierBold => &[
            "courbd.ttf",
            "LiberationMono-Bold.ttf",
            "Courier New Bold.ttf",
        ],
        S::CourierOblique => &[
            "couri.ttf",
            "LiberationMono-Italic.ttf",
            "Courier New Italic.ttf",
        ],
        S::CourierBoldOblique => &[
            "courbi.ttf",
            "LiberationMono-BoldItalic.ttf",
            "Courier New Bold Italic.ttf",
        ],
        S::TimesRoman => &[
            "times.ttf",
            "LiberationSerif-Regular.ttf",
            "Times New Roman.ttf",
        ],
        S::TimesBold => &[
            "timesbd.ttf",
            "LiberationSerif-Bold.ttf",
            "Times New Roman Bold.ttf",
        ],
        S::TimesItalic => &[
            "timesi.ttf",
            "LiberationSerif-Italic.ttf",
            "Times New Roman Italic.ttf",
        ],
        S::TimesBoldItalic => &[
            "timesbi.ttf",
            "LiberationSerif-BoldItalic.ttf",
            "Times New Roman Bold Italic.ttf",
        ],
        S::Symbol => &["symbol.ttf", "Symbol.ttf"],
        S::ZapfDingBats => &["wingding.ttf", "Wingdings.ttf"],
    };
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(w) = std::env::var_os("WINDIR") {
        dirs.push(std::path::PathBuf::from(w).join("Fonts"));
    }
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(
            std::path::PathBuf::from(l)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    dirs.push("/usr/share/fonts/truetype/liberation".into());
    // macOS: Arial, Courier New, Times New Roman, Symbol liegen unter Supplemental
    dirs.push("/System/Library/Fonts/Supplemental".into());
    dirs.push("/System/Library/Fonts".into());
    names
        .iter()
        .flat_map(|n| dirs.iter().map(move |d| d.join(n)))
        .find_map(|p| std::fs::read(p).ok())
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
            let e = c.entry(key).or_insert_with(|| {
                standard_font_file(f)
                    .map(|v| Arc::new(v) as hayro_svg::hayro_interpret::font::FontData)
            });
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
            hayro_svg::hayro_syntax::LoadPdfError::Decryption(_) => {
                ImportError::Unsupported(fp_i18n::t("err-import-pdf-encrypted"))
            }
            _ => ImportError::Corrupt(fp_i18n::t("err-import-pdf-unreadable")),
        })?;
        let settings = pdf_settings();
        let cache = hayro_svg::RenderCache::new();
        let mut result: Option<ImportResult> = None;
        // optionale Inhalte (OCG): je Gruppe eine Ebene (U-1); ohne Gruppen wie bisher eine Ebene
        let oc = crate::pdf_layers::OcInfo::read(&pdf, data);
        for (i, page) in pdf.pages().iter().enumerate().take(64) {
            if let Some(oc) = &oc
                && let Some(plan) = oc.plan(page, &fp_i18n::tr!("msg-default-layer", n = "1"))
                // misslingt ein Durchgang, wird die Seite wie bisher als eine Ebene gelesen
                && let Ok((base, chunks)) = pdf_layer_passes(data, oc, &plan, i, opts)
            {
                match &mut result {
                    None => {
                        let mut r = base;
                        r.format = FileFormat::Pdf;
                        if let Some(p) = r.document.page_mut(0) {
                            for s in p.root.layers.iter_mut() {
                                if let fp_model::LayerSlot::Own(l) = s {
                                    for c in Arc::make_mut(l).cels.iter_mut() {
                                        Arc::make_mut(c).elements.clear();
                                    }
                                }
                            }
                        }
                        fill_page_layers(&mut r.document, 0, chunks);
                        result = Some(r);
                    }
                    Some(acc) => {
                        let src = &base.document.pages[0];
                        let idx = acc
                            .document
                            .add_page(&fp_i18n::tr!("msg-default-page", n = (i + 1).to_string()));
                        if let Some(p) = acc.document.page_mut(idx) {
                            p.canvas.width = src.canvas.width;
                            p.canvas.height = src.canvas.height;
                            p.root.background = src.root.background;
                        }
                        fill_page_layers(&mut acc.document, idx, chunks);
                        acc.warnings.extend(base.warnings);
                    }
                }
                continue;
            }
            let svg = hayro_svg::convert(
                page,
                &cache,
                &settings,
                &hayro_svg::SvgRenderSettings::default(),
            );
            let r = import_svg_with(svg.as_bytes(), opts, false)?;
            match &mut result {
                None => {
                    let mut r = r;
                    r.format = FileFormat::Pdf;
                    result = Some(r);
                }
                Some(acc) => {
                    let src = &r.document.pages[0];
                    let idx = acc
                        .document
                        .add_page(&fp_i18n::tr!("msg-default-page", n = (i + 1).to_string()));
                    let els: Vec<Arc<Element>> = src
                        .root
                        .layers
                        .iter()
                        .filter_map(|s| {
                            if let fp_model::LayerSlot::Own(l) = s {
                                l.cels.first().map(|c| c.elements.clone())
                            } else {
                                None
                            }
                        })
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
        let mut r =
            result.ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-pdf-no-page")))?;
        r.warnings.sort();
        r.warnings.dedup();
        Ok(r)
    };
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run)).unwrap_or_else(|_| {
        Err(ImportError::Decode(
            "PDF",
            fp_i18n::t("err-import-pdf-aborted"),
        ))
    })
}

/// Ebene einer PDF-Seite: Name (`None` ⇒ „Ebene n“), ausgeblendet, Elemente oben → unten
/// (IDs noch aus dem Durchgang, [`fill_page_layers`] vergibt sie neu).
type PdfLayerChunk = (Option<String>, bool, Vec<Element>);

/// Je Ebene der geplanten Seite einen Durchgang rendern (U-1). Liefert das Ergebnis des ersten
/// Durchgangs (Leinwand, Warnungen) und die Ebenen unten → oben.
fn pdf_layer_passes(
    data: &[u8],
    oc: &crate::pdf_layers::OcInfo,
    plan: &crate::pdf_layers::PagePlan,
    page: usize,
    opts: &ImportOptions,
) -> Result<(ImportResult, Vec<PdfLayerChunk>)> {
    let mut base: Option<ImportResult> = None;
    let mut chunks = Vec::new();
    for l in &plan.layers {
        let bytes = oc.pass_pdf(data, plan, l.target);
        let pdf = hayro_svg::hayro_syntax::Pdf::new(bytes)
            .map_err(|_| ImportError::Corrupt(fp_i18n::t("err-import-pdf-unreadable")))?;
        let p = pdf
            .pages()
            .get(page)
            .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-pdf-no-page")))?;
        let mut settings = pdf_settings();
        // Anmerkungen gehören zu keiner Gruppe: nur einmal, mit dem Inhalt außerhalb
        settings.render_annotations = l.target.is_none();
        let svg = hayro_svg::convert(
            p,
            &hayro_svg::RenderCache::new(),
            &settings,
            &hayro_svg::SvgRenderSettings::default(),
        );
        let mut r = import_svg_with(svg.as_bytes(), opts, false)?;
        let els: Vec<Element> = r.document.pages[0]
            .root
            .layers
            .iter()
            .filter_map(|s| {
                if let fp_model::LayerSlot::Own(l) = s {
                    l.cels.first().map(|c| c.elements.clone())
                } else {
                    None
                }
            })
            .flatten()
            .map(|e| (*e).clone())
            .collect();
        chunks.push((l.name.clone(), l.hidden, els));
        match &mut base {
            None => base = Some(r),
            Some(b) => b.warnings.append(&mut r.warnings),
        }
    }
    let base = base.ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-pdf-no-page")))?;
    Ok((base, chunks))
}

/// PDF-Ebenen (unten → oben) auf Seite `page` legen: die erste in die vorhandene Ebene, die
/// übrigen darüber; leere entfallen, ausgeblendete Gruppen werden ausgeblendete Ebenen.
fn fill_page_layers(doc: &mut fp_model::Document, page: usize, mut chunks: Vec<PdfLayerChunk>) {
    chunks.retain(|(_, _, e)| !e.is_empty());
    let mut ids = std::mem::take(&mut doc.ids);
    if let Some(p) = doc.page_mut(page)
        && let Some(first) = p.root.layers.iter().position(
            |s| matches!(s, fp_model::LayerSlot::Own(l) if l.kind != fp_model::LayerKind::Web),
        )
    {
        let frames = match &p.root.layers[first] {
            fp_model::LayerSlot::Own(l) => l.cels.len().max(1),
            _ => 1,
        };
        for (k, (name, hidden, els)) in chunks.into_iter().enumerate() {
            let label = name
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| fp_i18n::tr!("msg-default-layer", n = (k + 1).to_string()));
            if k > 0 {
                let l = fp_model::Layer::new(&mut ids, &label, fp_model::LayerKind::Normal, frames);
                p.root
                    .layers
                    .insert(first + k, fp_model::LayerSlot::Own(Arc::new(l)));
            }
            let els: Vec<Arc<Element>> = els
                .iter()
                .map(|e| Arc::new(reassign_ids(&mut ids, e)))
                .collect();
            if let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(first + k) {
                let l = Arc::make_mut(l);
                l.name = label;
                if hidden {
                    let n = l.visibility.len().max(1);
                    l.visibility = vec![false; n];
                }
                if let Some(c) = l.cels.first_mut() {
                    Arc::make_mut(c).elements.extend(els);
                }
            }
        }
    }
    doc.ids = ids;
}

/// Element (rekursiv) mit neuen IDs aus `ids`.
fn reassign_ids(ids: &mut IdGen, e: &Element) -> Element {
    let mut n = e.clone();
    n.id = ids.element();
    if let ElementKind::Group(g) = &mut n.kind {
        g.children = g
            .children
            .iter()
            .map(|c| Arc::new(reassign_ids(ids, c)))
            .collect();
    }
    n
}

// ---------------------------------------------------------------- PostScript / Illustrator ≤ 8
fn bounding_box(text: &str) -> Option<[f64; 4]> {
    for line in text.lines().take(400) {
        if let Some(rest) = line
            .strip_prefix("%%HiResBoundingBox:")
            .or_else(|| line.strip_prefix("%%BoundingBox:"))
        {
            let v: Vec<f64> = rest
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if v.len() == 4 {
                return Some([v[0], v[1], v[2], v[3]]);
            }
        }
    }
    None
}

fn is_illustrator(text: &str) -> bool {
    text.lines().take(60).any(|l| {
        (l.starts_with("%%Creator:") && l.contains("Illustrator"))
            || l.starts_with("%%AI")
            || l.starts_with("%AI")
            || l.contains("AI3_")
            || l.contains("AI5_")
    })
}

/// PostScript-Zeile in Wörter zerlegen; `( … )` (mit `\\`-Maskierung und Klammerpaaren) bleibt ein Wort.
fn ps_tokens(line: &str) -> Vec<&str> {
    let b = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if b[i] == b'(' {
            let mut depth = 0;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 1,
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else {
            while i < b.len() && !b[i].is_ascii_whitespace() && (i == start || b[i] != b'(') {
                i += 1;
            }
        }
        if let Some(t) = line.get(start..i.min(b.len())) {
            out.push(t);
        }
    }
    out
}

/// Inhalt einer PostScript-Zeichenkette `( … )`; `\r` bzw. `\n` ⇒ Zeilenumbruch, Oktal `\ddd` ⇒ Latin-1.
fn ps_string(tok: &str) -> String {
    let inner = tok.strip_prefix('(').unwrap_or(tok);
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    let mut out = String::new();
    let mut it = inner.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('r' | 'n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(d @ '0'..='7') => {
                let mut v = d.to_digit(8).unwrap_or(0);
                for _ in 0..2 {
                    if let Some(&n) = it.peek()
                        && let Some(x) = n.to_digit(8)
                    {
                        v = v * 8 + x;
                        it.next();
                    }
                }
                out.push(char::from_u32(v).unwrap_or('?'));
            }
            Some(o) => out.push(o),
            None => {}
        }
    }
    out
}

/// Textobjekt einer Illustrator-Datei (`To` … `TO`).
#[derive(Default)]
struct AiText {
    /// 0 Punkttext, 1 Flächentext, 2 Pfadtext.
    kind: i32,
    /// Ursprung aus `Tp` (Grundlinie des Punkttexts), Illustrator-Koordinaten.
    origin: Option<(f64, f64)>,
    /// Rahmen eines Flächentexts in Dokumentkoordinaten.
    frame: Option<fp_geom::Rect>,
    font: String,
    size: f64,
    color: Argb,
    text: String,
}

impl AiText {
    /// Schriftfamilie aus dem PostScript-Namen: `_MyriadPro-Regular` ⇒ „Myriad Pro“.
    fn family(&self) -> (String, String) {
        let ps = self.font.trim_start_matches('_').to_string();
        // `ArialMT`, `TimesNewRomanPSMT` ⇒ ohne Herstellerkürzel
        let base = ps
            .split('-')
            .next()
            .unwrap_or(&ps)
            .trim_end_matches("MT")
            .trim_end_matches("PS");
        let mut fam = String::new();
        let mut prev: Option<char> = None;
        for c in base.chars() {
            if c.is_ascii_uppercase() && prev.is_some_and(|p| p.is_ascii_lowercase()) {
                fam.push(' ');
            }
            fam.push(c);
            prev = Some(c);
        }
        if fam.is_empty() {
            fam = "Arial".into();
        }
        (fam, if ps.is_empty() { "ArialMT".into() } else { ps })
    }

    fn element(
        mut self,
        ids: &mut IdGen,
        map: &dyn Fn(f64, f64) -> Point,
        warnings: &mut Vec<String>,
    ) -> Option<Element> {
        // Absatzende des letzten Absatzes ist kein Text
        while self.text.ends_with('\n') {
            self.text.pop();
        }
        if self.text.is_empty() {
            return None;
        }
        let (family, ps) = self.family();
        let size = if self.size > 0.0 {
            self.size as f32
        } else {
            12.0
        };
        let origin = self.origin.map(|(x, y)| map(x, y)).unwrap_or_default();
        let mut e = Element::new_text(ids, &self.text, &family, size, self.color, origin);
        if let ElementKind::Text(t) = &mut e.kind {
            t.initial.ps_font = Some(ps.clone());
            if let Some(r) = t.runs.first_mut() {
                r.attrs.font = Some(ps);
            }
            match (self.kind, self.frame) {
                (1, Some(f)) => {
                    // Flächentext: Rahmen fester Breite wie im Original (Rahmentext, `frame_type` 1). Das Original
                    // legt den Rahmen 2 px links und 2 px tiefer an als den Pfad zwischen `Tp` und `TP` (TW-38,
                    // gemessen 14.10.2026: `rawLeft/rawTop` = Pfad −2/+2 bei 3 Lagen, Myriad Pro und Arial,
                    // 24/48 pt; `par.ai` ⇒ Tinte 9,20–107,55)
                    let f = fp_geom::Rect::new(
                        f.left - 2.0,
                        f.top + 2.0,
                        f.right - 2.0,
                        f.bottom + 2.0,
                    );
                    t.frame_type = Some(1);
                    t.anchor_rect = Some(f);
                    t.bounds = f;
                    t.fill_handles = fp_model::new::default_fill_handles(f);
                }
                (2, _) => warnings.push(fp_i18n::t("msg-import-warn-ai-text")),
                _ => {}
            }
        }
        Some(e)
    }
}

/// Illustrator-PostScript (≤ 8) bzw. AI-EPS lesen.
fn import_ai_text(text: &str, opts: &ImportOptions) -> Result<ImportResult> {
    let bb = bounding_box(text)
        .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-eps-no-bbox")))?;
    let (w, h) = ((bb[2] - bb[0]).ceil(), (bb[3] - bb[1]).ceil());
    if !(w > 0.0 && h > 0.0) {
        return Err(ImportError::Corrupt(fp_i18n::t("err-import-bbox-empty")));
    }
    check_size(w as u32, h as u32)?;
    let body_start = ["%%EndSetup", "%%EndProlog"]
        .iter()
        .filter_map(|m| text.find(m))
        .min()
        .unwrap_or(0);
    let body = &text[body_start..];
    let map = |x: f64, y: f64| Point::new((x - bb[0]) as f32, (bb[3] - y) as f32);
    let cmyk = |c: f64, m: f64, y: f64, k: f64| -> Argb {
        let (r, g, b) = cmyk_to_rgb(
            postscript_cmyk_byte(c),
            postscript_cmyk_byte(m),
            postscript_cmyk_byte(y),
            postscript_cmyk_byte(k),
            opts.cmyk_table.as_deref(),
        );
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
    let mut txt: Option<AiText> = None;
    let (mut last_str, mut last_name) = (String::new(), String::new());
    // Ebenen der Datei (`Lb` … `(Name) Ln` … `LB`), unten → oben, je Elemente in Malreihenfolge
    let mut ai_layers: Vec<(Option<String>, Vec<Element>)> = Vec::new();
    let mut layer_name: Option<String> = None;
    for raw in body.lines() {
        let line = raw.trim();
        if line.starts_with('%') {
            continue;
        }
        for tok in ps_tokens(line) {
            if tok.starts_with('(') {
                last_str = ps_string(tok);
                stack.clear();
                continue;
            }
            if let Some(name) = tok.strip_prefix('/') {
                last_name = name.to_string();
                stack.clear();
                continue;
            }
            if tok.starts_with('[') || tok.starts_with('{') {
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
                "Lb" | "LB" => {
                    if !elements.is_empty() || layer_name.is_some() {
                        ai_layers.push((layer_name.take(), std::mem::take(&mut elements)));
                    }
                }
                "Ln" => layer_name = Some(last_str.clone()),
                "To" => {
                    in_text = true;
                    txt = Some(AiText {
                        kind: if n >= 1 { arg(1) as i32 } else { 0 },
                        ..AiText::default()
                    });
                }
                "TO" => {
                    in_text = false;
                    if let Some(t) = txt.take()
                        && let Some(e) = t.element(&mut ids, &map, &mut warnings)
                    {
                        elements.push(e);
                    }
                }
                "Tp" if n >= 7 => {
                    if let Some(t) = txt.as_mut() {
                        t.origin = Some((arg(3), arg(2)));
                    }
                }
                "Tf" if n >= 1 => {
                    if let Some(t) = txt.as_mut() {
                        t.font = last_name.clone();
                        t.size = arg(1);
                    }
                }
                "Tx" | "Tj" => {
                    if let Some(t) = txt.as_mut() {
                        if t.text.is_empty() {
                            t.color = fill;
                        }
                        t.text.push_str(&last_str);
                    }
                }
                "m" if n >= 2 => b.move_to(map(arg(2), arg(1))),
                "l" | "L" if n >= 2 => b.line_to(map(arg(2), arg(1))),
                "c" | "C" if n >= 6 => b.cubic_to(
                    map(arg(6), arg(5)),
                    map(arg(4), arg(3)),
                    map(arg(2), arg(1)),
                ),
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
                    // Rahmen eines Flächentexts (Pfad zwischen `Tp` und `TP`)
                    if in_text
                        && let Some(t) = txt.as_mut()
                        && t.frame.is_none()
                        && !contours.is_empty()
                    {
                        t.frame = Some(fp_model::new::contour_bounds(&contours));
                    }
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
                "x" if n >= 5 => {
                    fill = cmyk(
                        arg(5) * arg(1),
                        arg(4) * arg(1),
                        arg(3) * arg(1),
                        arg(2) * arg(1),
                    )
                }
                "X" if n >= 5 => {
                    stroke = cmyk(
                        arg(5) * arg(1),
                        arg(4) * arg(1),
                        arg(3) * arg(1),
                        arg(2) * arg(1),
                    )
                }
                "Xa" if n >= 3 => fill = rgb(arg(3), arg(2), arg(1)),
                "XA" if n >= 3 => stroke = rgb(arg(3), arg(2), arg(1)),
                "w" if n >= 1 => width = arg(1),
                "XI" | "XG" | "%%BeginRaster" => {
                    warnings.push(fp_i18n::t("msg-import-warn-ai-raster"))
                }
                _ => {}
            }
            if !matches!(tok, "Xa" | "XA") || n >= 3 {
                stack.clear();
            }
        }
    }
    if !elements.is_empty() || layer_name.is_some() {
        ai_layers.push((layer_name.take(), std::mem::take(&mut elements)));
    }
    // je Ebene der Datei eine Ebene mit ihrem Namen (unten → oben); Elemente oben → unten
    let chunks = ai_layers
        .into_iter()
        .map(|(n, mut e)| {
            e.reverse();
            (n, e)
        })
        .collect();
    push_layer_chunks(&mut doc, ids, chunks);
    warnings.sort();
    warnings.dedup();
    Ok(ImportResult {
        document: doc,
        format: FileFormat::Illustrator,
        content: ImportedContent::Layered,
        warnings,
    })
}

/// Illustrator ≤ 8, AI-EPS oder DOS-EPS mit TIFF-Vorschau.
pub fn import_postscript(
    data: &[u8],
    fmt: FileFormat,
    opts: &ImportOptions,
) -> Result<ImportResult> {
    let (ps, tiff) = if fmt == FileFormat::EpsBinary && data.len() >= 30 {
        let u = |o: usize| {
            u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize
        };
        let (po, pl, to, tl) = (u(4), u(8), u(20), u(24));
        (
            data.get(po..po.saturating_add(pl)).unwrap_or(&[]),
            if tl > 0 {
                data.get(to..to.saturating_add(tl))
            } else {
                None
            },
        )
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
        let mut r = ImportResult {
            document: document_from_raster(img, FileFormat::Eps),
            format: FileFormat::Eps,
            content: ImportedContent::Bitmap,
            warnings: vec![],
        };
        r.warnings.push(fp_i18n::t("msg-import-warn-eps-preview"));
        if let Some(p) = r.document.page_mut(0)
            && let Some(fp_model::LayerSlot::Own(l)) = p.root.layers.get_mut(0)
        {
            Arc::make_mut(l).name = fp_i18n::tr!("msg-default-layer", n = "1");
        }
        return Ok(r);
    }
    Err(ImportError::Unsupported(fp_i18n::t(
        "err-import-eps-no-ai-data",
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai8_rectangle() {
        let ai = "%!PS-3.0\n%%Creator: Illustrator(R) 8.0\n%%BoundingBox: 0 0 100 50\n%%EndSetup\n0 0 0 1 k\n10 10 m\n90 10 L\n90 40 L\n10 40 L\nf\n";
        let r = import_postscript(
            ai.as_bytes(),
            FileFormat::Illustrator,
            &ImportOptions::default(),
        )
        .unwrap();
        let p = &r.document.pages[0];
        assert_eq!((p.canvas.width, p.canvas.height), (100.0, 50.0));
        let fp_model::LayerSlot::Own(l) = &p.root.layers[0] else {
            panic!()
        };
        let e = &l.cels[0].elements[0];
        let ElementKind::Path(pe) = &e.kind else {
            panic!()
        };
        assert_eq!(pe.contours[0].nodes.len(), 4);
        assert!(pe.contours[0].closed);
        assert_eq!(pe.contours[0].nodes[0].anchor, Point::new(10.0, 40.0));
        assert!(
            import_postscript(
                b"%!PS-3.0 EPSF-3.0\n%%BoundingBox: 0 0 1 1\n",
                FileFormat::Eps,
                &ImportOptions::default()
            )
            .is_err()
        );
    }

    #[test]
    fn svg_paths() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect x="2" y="3" width="10" height="5" fill="#ff0000"/><g opacity="0.5"><circle cx="30" cy="10" r="5" fill="blue" stroke="black" stroke-width="2"/><path d="M0 0 L5 5"/></g></svg>"##;
        let r = import_svg(svg, &ImportOptions::default()).unwrap();
        let p = &r.document.pages[0];
        assert_eq!((p.canvas.width, p.canvas.height), (40.0, 20.0));
        let fp_model::LayerSlot::Own(l) = &p.root.layers[0] else {
            panic!()
        };
        assert_eq!(l.cels[0].elements.len(), 2);
        let ElementKind::Path(rect) = &l.cels[0].elements[1].kind else {
            panic!()
        };
        assert_eq!(rect.attrs.fill_color, Argb(0xFFFF_0000));
        assert!(import_svg(b"<svg", &ImportOptions::default()).is_err());
    }
}
