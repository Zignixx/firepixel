//! Photoshop-Import (12 §3) mit den Laufzeitergebnissen 7a:
//!
//! * PS-Ebenen ohne Gruppe werden **Objekte einer gemeinsamen Ebene „Ebene“**; eine Gruppe auf oberster
//!   Stufe wird eine Ebene mit dem Gruppennamen und ihren Objekten (T1/T4); tiefere Gruppen werden
//!   Unterebenen (A).
//! * Schnittmaskenkette: nur die **oberste** beschnittene Ebene liefert ein Bitmap (Bereichs-Composite
//!   Basis … oberste, Einstellungen angewendet); Basis nur mit „Schnittmasken bearbeitbar“ und als
//!   bearbeitbare Ebene, Zwischenebenen nie (T3).
//! * Fülldeckkraft (`iOpa`) wird ignoriert (T2). Einstellungsebenen ohne Schnittmaske entfallen (T4).
//! * Ebeneneffekte: „Native Filter haben Vorrang“ (`UseFWFilters`) ⇒ Original-Live-Effekte nach den
//!   korrigierten Formeln (T5: keine 250-Kappung des Schlagschatten-Abstands, Glühbreite/-weichheit
//!   aus Größe und Überfüllen), Rest als Photoshop-Live-Effekt; sonst ein Photoshop-Live-Effekt mit
//!   allen Effekten (`&`-Datensätze, 05 §9).
//! * Leinwand transparent; Farbmodi Grau/indiziert/CMYK (naiv)/Lab (nicht unterstützt ⇒ Composite).

pub mod descriptor;
pub mod effects;
pub mod reader;

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::IRect;
use fp_model::{
    Contour, ContourNode, Document, Element, ElementKind, IdGen, Layer, LayerKind as FwLayerKind,
    LayerSlot, PathAttrs, Point,
};
use fp_raster::RgbaImage;

use crate::build::{ImportResult, ImportedContent, bitmap_element, empty_document};
use crate::detect::FileFormat;
use crate::{ImportError, ImportOptions, Result};
use reader::{LayerKind, PsdFile, PsdLayer};

/// Optionen des Photoshop-Imports (Felder des Lesers, 12 §3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PsdOptions {
    /// `PSTextEditable` (+0x09).
    pub text_editable: bool,
    /// `PSShapeLayersEditable` (+0x0A).
    pub shape_editable: bool,
    /// `PSShapeLayerEffectsEditable` (+0x0B).
    pub shape_effects_editable: bool,
    /// `PSImageLayersEditable` (+0x0C).
    pub image_editable: bool,
    /// Auf ein Bild reduzieren (+0x0D = !`TryToRetainLayers`).
    pub flatten: bool,
    /// `PSSharedLayers` (+0x0E).
    pub shared_layers: bool,
    /// `PSIncludeGuides` (+0x0F).
    pub include_guides: bool,
    /// `PSLayersAsFrames` (+0x12).
    pub layers_as_frames: bool,
    /// Nur Import: in neue Ebene „Aus PS importiert“ (+0x13).
    pub into_new_layer: bool,
    /// Nur Import: Hintergrundebene einschließen (+0x14).
    pub include_background: bool,
    /// `PSClippingMasksEditable` (+0x15).
    pub clipping_editable: bool,
    /// `PSRetainAdjustmentLayersAppearance` (+0x16).
    pub keep_adjustment_appearance: bool,
    /// `UseFWFilters`: native Filter haben Vorrang vor Photoshop-Live-Effekten.
    pub native_first: bool,
}

impl Default for PsdOptions {
    fn default() -> Self {
        PsdOptions {
            text_editable: true,
            shape_editable: true,
            shape_effects_editable: true,
            image_editable: true,
            flatten: false,
            shared_layers: false,
            include_guides: true,
            layers_as_frames: false,
            into_new_layer: false,
            include_background: true,
            clipping_editable: true,
            keep_adjustment_appearance: false,
            native_first: false,
        }
    }
}

impl PsdOptions {
    /// Vorgaben der Combobox (12 §3.2, `@009bca40`): 0 Bearbeitbarkeit, 1 Erscheinungsbild,
    /// 2 Voreinstellungen, 3 reduzieren.
    pub fn preset(p: u8, prefs: &PsdOptions) -> PsdOptions {
        let mut o = prefs.clone();
        match p {
            0 => {
                o.text_editable = true;
                o.shape_editable = true;
                o.image_editable = true;
                o.clipping_editable = true;
                o.keep_adjustment_appearance = false;
                o.flatten = false;
            }
            1 => {
                o.text_editable = false;
                o.shape_editable = false;
                o.shape_effects_editable = false;
                o.image_editable = false;
                o.clipping_editable = false;
                o.keep_adjustment_appearance = true;
                o.flatten = false;
            }
            3 => o.flatten = true,
            _ => o.flatten = false,
        }
        o
    }
}

/// Mischmodus Photoshop → `BLDi` (12 §3.5; unbekannt ⇒ 0).
pub fn ps_to_fw_blend(name: &str) -> u8 {
    match name {
        "multiply" => 2,
        "difference" => 3,
        "overlay" => 4,
        "screen" => 5,
        "dissolve" => 7,
        "linearBurn" => 8,
        "darken" => 9,
        "lighten" => 10,
        "hue" => 11,
        "saturation" => 12,
        "color" => 13,
        "luminosity" => 14,
        "linearDodge" => 18,
        "vividLight" => 19,
        "linearLight" => 20,
        "pinLight" => 21,
        "hardMix" => 22,
        "exclusion" => 32,
        "hardLight" => 33,
        "softLight" => 34,
        "colorDodge" => 36,
        "colorBurn" => 37,
        _ => 0,
    }
}

fn layer_image(l: &PsdLayer) -> Option<RgbaImage> {
    if l.width() == 0 || l.height() == 0 {
        return None;
    }
    RgbaImage::from_rgba_bytes(
        IRect::new(l.rect[0], l.rect[1], l.rect[2], l.rect[3]),
        &l.rgba,
    )
    .ok()
}

/// Einstellung auf eine Farbe anwenden (unterstützt: Tonwertkorrektur, Umkehren, Schwellenwert,
/// Tontrennung, Helligkeit/Kontrast in der Altform; sonst unverändert).
pub fn adjust(kind: &str, data: &[u8], rgb: [u8; 3]) -> [u8; 3] {
    match kind {
        "levl" if data.len() >= 2 + 10 * 4 => {
            let rec = |k: usize| -> [i32; 5] {
                std::array::from_fn(|j| {
                    i16::from_be_bytes([data[2 + k * 10 + j * 2], data[3 + k * 10 + j * 2]]) as i32
                })
            };
            let apply = |v: f64, r: [i32; 5]| -> f64 {
                let (ilo, ihi, olo, ohi, g) = (
                    r[0] as f64,
                    r[1] as f64,
                    r[2] as f64,
                    r[3] as f64,
                    r[4] as f64 / 100.0,
                );
                let span = (ihi - ilo).max(1.0);
                let mut t = ((v - ilo) / span).clamp(0.0, 1.0);
                if g > 0.0 && (g - 1.0).abs() > 1e-9 {
                    t = t.powf(1.0 / g);
                }
                olo + t * (ohi - olo)
            };
            let master = rec(0);
            std::array::from_fn(|c| {
                let v = apply(rgb[c] as f64, rec(c + 1));
                apply(v, master).round().clamp(0.0, 255.0) as u8
            })
        }
        "nvrt" => rgb.map(|v| 255 - v),
        "thrs" if data.len() >= 2 => {
            let lvl = u16::from_be_bytes([data[0], data[1]]) as u32;
            let l = (rgb[0] as u32 * 299 + rgb[1] as u32 * 587 + rgb[2] as u32 * 114) / 1000;
            let v = if l >= lvl { 255 } else { 0 };
            [v, v, v]
        }
        "post" if data.len() >= 2 => {
            let n = u16::from_be_bytes([data[0], data[1]]).clamp(2, 255) as f64;
            rgb.map(|v| ((v as f64 / 255.0 * (n - 1.0)).round() / (n - 1.0) * 255.0).round() as u8)
        }
        "brit" if data.len() >= 4 => {
            let b = i16::from_be_bytes([data[0], data[1]]) as f64;
            let c = i16::from_be_bytes([data[2], data[3]]) as f64;
            let f = (100.0 + c) / 100.0;
            rgb.map(|v| {
                (((v as f64 + b) - 127.5) * f + 127.5)
                    .round()
                    .clamp(0.0, 255.0) as u8
            })
        }
        _ => rgb,
    }
}

/// Bereichs-Composite einer Schnittmaskenkette `base..=top` (12 §3.6): Ergebnis in den Grenzen der
/// Basis, Alpha der Basis, darüber die beschnittenen Ebenen mit Deckkraft (Normal), Einstellungen auf die
/// bisherigen Farben angewendet.
pub fn range_composite(
    layers: &[PsdLayer],
    base: usize,
    top: usize,
    warnings: &mut Vec<String>,
) -> Option<RgbaImage> {
    let b = layers.get(base)?;
    let mut acc = layer_image(b)?;
    let bop = b.opacity as u32;
    for p in acc.data_mut() {
        p[3] = ((p[3] as u32 * bop + 127) / 255) as u8;
    }
    let rect = acc.bounds();
    for l in layers.get(base + 1..=top)? {
        if !l.visible {
            continue;
        }
        if let Some((k, d)) = &l.adjustment {
            if !matches!(k.as_str(), "levl" | "nvrt" | "thrs" | "post" | "brit") {
                warnings.push(fp_i18n::tr!(
                    "msg-import-warn-psd-adjustment",
                    name = l.name.as_str(),
                    kind = k.as_str()
                ));
            }
            for p in acc.data_mut() {
                let a = adjust(k, d, [p[0], p[1], p[2]]);
                p[0] = a[0];
                p[1] = a[1];
                p[2] = a[2];
            }
            continue;
        }
        let Some(src) = layer_image(l) else { continue };
        let op = l.opacity as u32;
        for y in rect.top.max(src.bounds().top)..rect.bottom.min(src.bounds().bottom) {
            for x in rect.left.max(src.bounds().left)..rect.right.min(src.bounds().right) {
                let s = src.pixel(x, y);
                let sa = (s.a as u32 * op + 127) / 255;
                if sa == 0 {
                    continue;
                }
                let mut d = acc.pixel(x, y);
                let mix =
                    |dv: u8, sv: u8| ((dv as u32 * (255 - sa) + sv as u32 * sa + 127) / 255) as u8;
                d.r = mix(d.r, s.r);
                d.g = mix(d.g, s.g);
                d.b = mix(d.b, s.b);
                acc.set_pixel(x, y, d);
            }
        }
    }
    Some(acc)
}

/// Composite der Datei ohne Weiß-Mattierung (12 §3.9: `C_datei = C·a + 255·(1−a)`).
pub fn unmatte_composite(f: &PsdFile) -> Option<RgbaImage> {
    if f.composite.len() < f.width as usize * f.height as usize * 4 {
        return None;
    }
    let mut px = f.composite.clone();
    for p in px.as_chunks_mut::<4>().0 {
        let a = p[3] as f64 / 255.0;
        if a > 0.0 && a < 1.0 {
            for c in &mut p[..3] {
                *c = ((*c as f64 - 255.0 * (1.0 - a)) / a)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
    RgbaImage::from_rgba_bytes(IRect::new(0, 0, f.width as i32, f.height as i32), &px).ok()
}

fn path_from_subpaths(ids: &mut IdGen, subs: &[reader::SubPath], color: Argb) -> Element {
    let contours: Vec<Contour> = subs
        .iter()
        .map(|s| Contour {
            closed: s.closed,
            bsl: None,
            nodes: s
                .knots
                .iter()
                .map(|k| {
                    let mut n = ContourNode::corner(Point::new(k[1].0 as f32, k[1].1 as f32));
                    n.pred = Point::new(k[0].0 as f32, k[0].1 as f32);
                    n.succ = Point::new(k[2].0 as f32, k[2].1 as f32);
                    n
                })
                .collect(),
            extra: Default::default(),
        })
        .collect();
    let mut e = Element::new_path(ids, contours, PathAttrs::solid_fill(color));
    if let ElementKind::Path(p) = &mut e.kind {
        p.even_odd = false;
    }
    e
}

fn fill_color(desc: &descriptor::Descriptor) -> Argb {
    let c = desc.obj("color").or_else(|| desc.obj("Clr "));
    let rgb = |k: &str| c.and_then(|c| c.num(k)).unwrap_or(0.0).clamp(0.0, 255.0) as u32;
    if desc.obj("gradient").is_some() {
        // Verlaufsfüllung: erste Farbe als Ersatz (A)
        if let Some(descriptor::Value::List(stops)) =
            desc.obj("gradient").and_then(|g| g.get("colors"))
            && let Some(s) = stops.first().and_then(|v| v.as_obj())
            && let Some(cc) = s.obj("color")
        {
            let g = |k: &str| cc.num(k).unwrap_or(0.0).clamp(0.0, 255.0) as u32;
            return Argb(0xFF00_0000 | g("red") << 16 | g("green") << 8 | g("blue"));
        }
    }
    Argb(0xFF00_0000 | rgb("red") << 16 | rgb("green") << 8 | rgb("blue"))
}

/// Text aus `TySh`/EngineData: (Schrift, Größe, Farbe).
fn text_style(engine: &str, m: &[f64; 6]) -> (String, f32, Argb) {
    let find_num = |key: &str| -> Option<f64> {
        let i = engine.find(key)? + key.len();
        let rest = engine[i..].trim_start();
        let end = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .unwrap_or(rest.len());
        rest[..end].parse().ok()
    };
    let size = find_num("/FontSize ").unwrap_or(12.0) * m[3].abs().max(1e-6);
    let mut color = Argb(0xFF00_0000);
    if let Some(i) = engine.find("/FillColor") {
        let tail: String = engine[i..].chars().take(200).collect();
        if let Some(s) = tail.find("/Values [") {
            let vals: Vec<f64> = tail[s + 9..]
                .split(']')
                .next()
                .unwrap_or("")
                .split_whitespace()
                .filter_map(|v| v.parse().ok())
                .collect();
            if vals.len() >= 4 {
                let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
                color = Argb(c(vals[0]) << 24 | c(vals[1]) << 16 | c(vals[2]) << 8 | c(vals[3]));
            }
        }
    }
    // Schrift: erster Eintrag der `/FontSet` (davor stehen gleichnamige Schlüssel der Kinsoku-Tabellen)
    let font = engine
        .find("/FontSet")
        .and_then(|f| engine[f..].find("/Name").map(|n| f + n + 5))
        .and_then(|i| engine_string(engine, i))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Arial".to_string());
    (font, size as f32, color)
}

/// PDF-Zeichenkette `( … )` aus EngineData ab Position `from` (Latin-1, ein Zeichen je Byte); `\xfe\xff` ⇒ UTF-16BE.
fn engine_string(engine: &str, from: usize) -> Option<String> {
    let rest = engine.get(from..)?;
    let start = rest.find('(')?;
    if !rest[..start].trim().is_empty() {
        return None;
    }
    let mut bytes = Vec::new();
    let mut it = rest[start + 1..].chars();
    while let Some(c) = it.next() {
        match c {
            '\\' => bytes.push(it.next()? as u32 as u8),
            ')' => break,
            c => bytes.push(c as u32 as u8),
        }
    }
    Some(if let Some(b) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let units: Vec<u16> = b
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_be_bytes(*c))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    })
}

/// Text des Textobjekts laut EngineData (`/Editor /Text`), Absatzende `\r` ⇒ Zeilenumbruch, letztes entfällt.
/// Das Original zeigt diesen Text, nicht den der Beschreibung (TW-15: `par.psd` hat dort „PSD“, im Editor leer).
fn engine_text(engine: &str) -> Option<String> {
    let e = engine.find("/Editor")?;
    let t = engine[e..].find("/Text")? + e + 5;
    let s = engine_string(engine, t)?;
    let s = s.strip_suffix('\r').unwrap_or(&s).replace('\r', "\n");
    Some(s)
}

/// Knoten des Ebenenbaums.
struct Node {
    layer: Option<usize>,
    children: Vec<Node>,
}

fn build_tree(layers: &[PsdLayer]) -> Node {
    let mut stack: Vec<Node> = vec![Node {
        layer: None,
        children: Vec::new(),
    }];
    for (i, l) in layers.iter().enumerate() {
        match l.kind() {
            LayerKind::SectionDivider => stack.push(Node {
                layer: None,
                children: Vec::new(),
            }),
            LayerKind::Section { .. } => {
                if stack.len() > 1 {
                    let mut n = stack.pop().unwrap_or(Node {
                        layer: None,
                        children: Vec::new(),
                    });
                    n.layer = Some(i);
                    if let Some(top) = stack.last_mut() {
                        top.children.push(n);
                    }
                } else {
                    stack[0].children.push(Node {
                        layer: Some(i),
                        children: Vec::new(),
                    });
                }
            }
            _ => {
                if let Some(top) = stack.last_mut() {
                    top.children.push(Node {
                        layer: Some(i),
                        children: Vec::new(),
                    });
                }
            }
        }
    }
    while stack.len() > 1 {
        if let Some(n) = stack.pop()
            && let Some(top) = stack.last_mut()
        {
            top.children.extend(n.children);
        }
    }
    stack.pop().unwrap_or(Node {
        layer: None,
        children: Vec::new(),
    })
}

struct Ctx<'a> {
    f: &'a PsdFile,
    o: &'a PsdOptions,
    importing: bool,
    ids: IdGen,
    tile: u32,
    top_adj: Option<usize>,
    warnings: Vec<String>,
    light: effects::GlobalLight,
}

impl Ctx<'_> {
    fn element_for(&mut self, i: usize) -> Option<Element> {
        let layers = &self.f.layers;
        let l = &layers[i];
        let kind = l.kind();
        if kind == LayerKind::Background && self.importing && !self.o.include_background {
            return None;
        }
        if kind == LayerKind::Adjustment && !l.clipping {
            return None;
        }
        if l.block_keys.iter().any(|k| k == "vtrk") {
            return None;
        }
        let mut editable = match kind {
            LayerKind::Content => self.o.shape_editable,
            LayerKind::Pixel | LayerKind::Background => self.o.image_editable,
            LayerKind::Text => self.o.text_editable,
            _ => false,
        };
        if self.o.keep_adjustment_appearance && editable && self.top_adj.is_some_and(|t| i < t) {
            editable = false;
        }
        if l.clipping && !self.o.clipping_editable {
            editable = false;
        }
        let clipped_above = layers.get(i + 1).is_some_and(|n| n.clipping);
        let mut el: Element;
        if l.clipping {
            if clipped_above {
                return None; // Zwischenebene der Kette
            }
            let mut base = i;
            while base > 0 && layers[base].clipping {
                base -= 1;
            }
            let img = range_composite(layers, base, i, &mut self.warnings)?;
            el = bitmap_element(&mut self.ids, &img, Some(&l.name), &mut self.tile);
            el.common.visible = l.visible;
            return Some(el);
        }
        if clipped_above && !(editable && self.o.clipping_editable) {
            return None; // Basis ohne eigene Pixel
        }
        match kind {
            LayerKind::Content
                if editable && l.vector_mask.as_ref().is_some_and(|v| !v.0.is_empty()) =>
            {
                let color = l
                    .fill
                    .as_ref()
                    .map(|f| fill_color(&f.1))
                    .unwrap_or(Argb(0xFF00_0000));
                el = path_from_subpaths(
                    &mut self.ids,
                    &l.vector_mask
                        .as_ref()
                        .map(|v| v.0.clone())
                        .unwrap_or_default(),
                    color,
                );
            }
            LayerKind::Text if editable => {
                let Some((m, text, _, engine)) = &l.text else {
                    return None;
                };
                let (font, size, color) = text_style(engine, m);
                let shown = engine_text(engine).unwrap_or_else(|| text.clone());
                el = Element::new_text(
                    &mut self.ids,
                    &shown,
                    &font,
                    size,
                    color,
                    Point::new(m[4] as f32, m[5] as f32),
                );
                self.warnings.push(fp_i18n::tr!(
                    "msg-import-warn-psd-text",
                    name = l.name.as_str()
                ));
            }
            _ => {
                let img = layer_image(l)?;
                el = bitmap_element(&mut self.ids, &img, None, &mut self.tile);
            }
        }
        el.common.name = Some(l.name.clone());
        // Masken
        let mut wrapped = el;
        // Masken-Bit 3: Pixelmaske ist nur die Rasterung anderer Daten (Vektormaske/Form) ⇒ nicht als Maske (TW-15)
        let rendered_mask = l.mask.as_ref().is_some_and(|m| m.flags & 8 != 0)
            && l.vector_mask.as_ref().is_some_and(|v| !v.0.is_empty());
        if let Some(m) = &l.mask
            && !m.disabled()
            && !m.pixels.is_empty()
            && !rendered_mask
        {
            let lr = IRect::new(l.rect[0], l.rect[1], l.rect[2], l.rect[3]);
            let mr = IRect::new(m.rect[0], m.rect[1], m.rect[2], m.rect[3]);
            let u = lr.union(&mr);
            let mut mimg = RgbaImage::filled(
                u,
                fp_color::Rgba8::new(m.default_color, m.default_color, m.default_color, 255),
            )
            .ok()?;
            let mw = (m.rect[2] - m.rect[0]).max(0) as usize;
            for (k, &v) in m.pixels.iter().enumerate() {
                if mw == 0 {
                    break;
                }
                mimg.set_pixel(
                    m.rect[0] + (k % mw) as i32,
                    m.rect[1] + (k / mw) as i32,
                    fp_color::Rgba8::new(v, v, v, 255),
                );
            }
            let me = bitmap_element(&mut self.ids, &mimg, None, &mut self.tile);
            let mut g = Element::new_mask_group(
                &mut self.ids,
                Arc::new(me),
                vec![Arc::new(wrapped)],
                false,
            );
            g.common.name = Some(l.name.clone());
            wrapped = g;
        } else if kind != LayerKind::Content
            && let Some((subs, flags)) = &l.vector_mask
            && !subs.is_empty()
            && flags & 2 == 0
        {
            let me = path_from_subpaths(&mut self.ids, subs, Argb(0xFFFF_FFFF));
            let mut g =
                Element::new_mask_group(&mut self.ids, Arc::new(me), vec![Arc::new(wrapped)], true);
            g.common.name = Some(l.name.clone());
            wrapped = g;
        }
        wrapped.common.opacity = Opacity(l.opacity);
        wrapped.common.blend = BlendMode(ps_to_fw_blend(l.blend));
        wrapped.common.visible = l.visible;
        // Effekte
        if let Some(fx) = &l.effects {
            let effects_editable = (editable
                || (kind == LayerKind::Content && self.o.shape_effects_editable))
                && kind != LayerKind::SmartObject
                && !(l.clipping && !self.o.clipping_editable);
            if effects_editable {
                // F150: all ten layer effects; `masterFXSwitch` = false imports them switched off.
                let list = effects::effect_list(fx, self.o.native_first, self.light);
                for fxname in effects::multi_losses(fx) {
                    self.warnings.push(fp_i18n::tr!(
                        "msg-import-warn-psd-multi-fx",
                        name = l.name.as_str(),
                        effect = fxname
                    ));
                }
                if !list.effects.is_empty() {
                    wrapped.common.effects = Some(list);
                }
            }
        }
        Some(wrapped)
    }

    /// Elemente eines Knotens (oben → unten).
    fn elements_of(&mut self, node: &Node, depth: u32) -> Vec<Element> {
        let mut out = Vec::new();
        for ch in node.children.iter().rev() {
            let Some(li) = ch.layer else { continue };
            let l = &self.f.layers[li];
            if matches!(l.kind(), LayerKind::Section { .. }) {
                let inner = self.elements_of(ch, depth + 1);
                let mut sub = Layer::new(&mut self.ids, &l.name, FwLayerKind::Normal, 1);
                sub.level = (depth + 1).min(2) as i32;
                sub.visibility = vec![l.visible];
                sub.expanded = matches!(l.kind(), LayerKind::Section { open: true });
                sub.opacity = Opacity(l.opacity);
                if let Some(c) = sub.cels.first_mut() {
                    Arc::make_mut(c).elements = inner.into_iter().map(Arc::new).collect();
                }
                let e = Element::new(self.ids.element(), ElementKind::SubLayer(Arc::new(sub)));
                out.push(e);
            } else if let Some(e) = self.element_for(li) {
                out.push(e);
            }
        }
        out
    }
}

/// Unterebenen einer zur obersten Ebene beförderten Gruppe eine Stufe höher setzen (mindestens Stufe 1).
fn lift_sublayers(l: &mut Layer) {
    for c in &mut l.cels {
        for e in &mut Arc::make_mut(c).elements {
            if let ElementKind::SubLayer(s) = &e.kind {
                let mut s2 = (**s).clone();
                s2.level = (s2.level - 1).max(1);
                lift_sublayers(&mut s2);
                Arc::make_mut(e).kind = ElementKind::SubLayer(Arc::new(s2));
            }
        }
    }
}

/// PSD importieren bzw. öffnen.
pub fn import(data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let f = reader::read(data)?;
    let o = &opts.psd;
    let mut doc = empty_document(f.width, f.height);
    let mut warnings = Vec::new();
    if f.mode == 9 {
        warnings.push(fp_i18n::t("msg-import-warn-psd-lab"));
    }
    if f.mode == 4 {
        warnings.push(fp_i18n::t("msg-import-warn-psd-cmyk"));
    }
    let flatten = o.flatten || f.layers.is_empty() || matches!(f.mode, 4 | 9);
    let mut ids = std::mem::take(&mut doc.ids);
    let mut tile = 0u32;
    if flatten {
        let img = unmatte_composite(&f)
            .ok_or_else(|| ImportError::Corrupt(fp_i18n::t("err-import-psd-no-composite")))?;
        let e = bitmap_element(&mut ids, &img, None, &mut tile);
        doc.ids = ids;
        if let Some(p) = doc.page_mut(0)
            && let Some(LayerSlot::Own(l)) = p.root.layers.get_mut(0)
        {
            let l = Arc::make_mut(l);
            l.name = fp_i18n::t("msg-default-background");
            if let Some(c) = l.cels.first_mut() {
                Arc::make_mut(c).elements.push(Arc::new(e));
            }
        }
    } else {
        let top_adj = f.layers.iter().rposition(|l| l.adjustment.is_some());
        let tree = build_tree(&f.layers);
        let mut ctx = Ctx {
            f: &f,
            o,
            importing: opts.importing,
            ids,
            tile,
            top_adj,
            warnings: Vec::new(),
            light: effects::GlobalLight {
                angle: f.resources.global_angle,
                altitude: f.resources.global_altitude,
            },
        };
        // Oberste Stufe: alle Objekte in **einer** Ebene „Ebene“, PS-Gruppen darin als Unterebenen (verschachtelt,
        // TW-15: `par.psd` ⇒ Ebene ▸ Ebene 2 / Ebene 1 / Background). Bleibt nur eine Gruppe übrig, wird sie selbst
        // die Ebene mit ihrem Namen (7a T4).
        let els = ctx.elements_of(&tree, 0);
        let mut fw_layers: Vec<Layer> = Vec::new();
        if let [only] = els.as_slice()
            && let ElementKind::SubLayer(sub) = &only.kind
        {
            let mut l = (**sub).clone();
            l.level = 0;
            lift_sublayers(&mut l);
            fw_layers.push(l);
        } else if !els.is_empty() {
            let mut l = Layer::new(&mut ctx.ids, "Ebene", FwLayerKind::Normal, 1);
            if let Some(c) = l.cels.first_mut() {
                Arc::make_mut(c).elements = els.into_iter().map(Arc::new).collect();
            }
            fw_layers.push(l);
        }
        if fw_layers.is_empty() {
            fw_layers.push(Layer::new(&mut ctx.ids, "Ebene", FwLayerKind::Normal, 1));
        }
        if opts.importing && o.into_new_layer {
            let mut all: Vec<Arc<Element>> = Vec::new();
            for l in fw_layers.iter().rev() {
                let mut sub = l.clone();
                sub.level = 1;
                all.push(Arc::new(Element::new(
                    ctx.ids.element(),
                    ElementKind::SubLayer(Arc::new(sub)),
                )));
            }
            let mut l = Layer::new(&mut ctx.ids, "Aus PS importiert", FwLayerKind::Normal, 1);
            if let Some(c) = l.cels.first_mut() {
                Arc::make_mut(c).elements = all;
            }
            fw_layers = vec![l];
        }
        if o.shared_layers {
            for l in &mut fw_layers {
                l.shared_across_frames = true;
            }
        }
        let mut frames = 1u32;
        if o.layers_as_frames {
            // Je Objekt ein Status in einer Ebene (A)
            let objs: Vec<Arc<Element>> = fw_layers
                .iter()
                .flat_map(|l| {
                    l.cels
                        .first()
                        .map(|c| c.elements.iter().rev().cloned().collect::<Vec<_>>())
                        .unwrap_or_default()
                })
                .collect();
            frames = objs.len().max(1) as u32;
            let mut l = Layer::new(&mut ctx.ids, "Ebene", FwLayerKind::Normal, frames as usize);
            for (k, e) in objs.into_iter().enumerate() {
                if let Some(c) = l.cels.get_mut(k) {
                    Arc::make_mut(c).elements.push(e);
                }
            }
            fw_layers = vec![l];
        }
        warnings.append(&mut ctx.warnings);
        let web = Layer::new(&mut ctx.ids, "Webebene", FwLayerKind::Web, frames as usize);
        doc.ids = ctx.ids;
        if let Some(p) = doc.page_mut(0) {
            p.root.frame_count = frames;
            let mut slots: Vec<LayerSlot> = fw_layers
                .into_iter()
                .map(|l| LayerSlot::Own(Arc::new(l)))
                .collect();
            slots.push(LayerSlot::Own(Arc::new(web)));
            p.root.layers = slots;
        }
    }
    if let Some(p) = doc.page_mut(0) {
        if let Some(d) = f.resources.dpi {
            p.canvas.resolution = d;
        }
        if o.include_guides
            && !f.resources.guides.is_empty()
            && let Some(g) = p.root.guides.as_mut()
        {
            for &(pos, vertical) in &f.resources.guides {
                if vertical {
                    g.vertical.push(pos);
                } else {
                    g.horizontal.push(pos);
                }
            }
        }
    }
    let _ = Document::new;
    Ok(ImportResult {
        document: doc,
        format: FileFormat::Psd,
        content: ImportedContent::Layered,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_and_blend() {
        let p1 = PsdOptions::preset(1, &PsdOptions::default());
        assert!(!p1.image_editable && p1.keep_adjustment_appearance && !p1.clipping_editable);
        assert!(PsdOptions::preset(3, &PsdOptions::default()).flatten);
        assert_eq!(ps_to_fw_blend("softLight"), 34);
        assert_eq!(ps_to_fw_blend("divide"), 0);
        // Tonwert Gamma 2,0: Orange (200,100,50) → (226,160,113) (Laufzeit 7a T3)
        let mut lev = vec![0u8, 2];
        for k in 0..29 {
            let g: i16 = if k == 0 { 200 } else { 100 };
            for v in [0i16, 255, 0, 255, g] {
                lev.extend_from_slice(&v.to_be_bytes());
            }
        }
        assert_eq!(adjust("levl", &lev, [200, 100, 50]), [226, 160, 113]);
    }
}
