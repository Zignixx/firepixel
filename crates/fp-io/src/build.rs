//! Dokumente aus importierten Inhalten (12 §5): Einzelbild ⇒ Bitmap in der Ebene „Hintergrund“
//! (Laufzeit 7a T7, Leinwand transparent `#ffffff00`), Animation ⇒ ein Status je Einzelbild mit
//! Verzögerung, Löschmethode und Schleife bzw. beim Import ein Animationssymbol, dazu die passenden
//! Exporteinstellungen.

use std::sync::Arc;

use fp_color::Argb;
use fp_model::{Document, Element, ExportSettings, FrameInfo, IdGen, Layer, LayerKind, LayerSlot, Point, Symbol, SymbolKind};
use fp_raster::RgbaImage;

use crate::detect::FileFormat;
use crate::raster::{Animation, RasterImage};
use crate::{ImportError, ImportOptions, Result};

/// Inhaltsart eines Imports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportedContent {
    /// Ein Bild (Bitmap).
    Bitmap,
    /// Animation (Status bzw. Animationssymbol).
    Animation { frames: usize },
    /// Ebenen/Objekte (PSD, SVG, AI, FW-PNG).
    Layered,
}

/// Ergebnis eines Imports: immer ein vollständiges Dokument. Beim Importieren in ein bestehendes
/// Dokument übernimmt der Aufrufer die Ebenen bzw. Elemente der ersten Seite (Symbole aus
/// `document.symbols`).
#[derive(Clone, Debug)]
pub struct ImportResult {
    pub document: Document,
    pub format: FileFormat,
    pub content: ImportedContent,
    /// Hinweise (nicht unterstützte Teile, Abweichungen).
    pub warnings: Vec<String>,
}

/// Bitmap-Element aus einem Bild an seiner Lage.
pub fn bitmap_element(ids: &mut IdGen, img: &RgbaImage, name: Option<&str>, next_tile: &mut u32) -> Element {
    let b = img.bounds();
    let mut e = Element::new_bitmap(ids, Point::new(b.left as f32, b.top as f32), img.width() as u32, img.height() as u32, img.as_bytes(), next_tile);
    e.common.name = name.map(str::to_string);
    e
}

fn set_resolution(doc: &mut Document, dpi: Option<f32>) {
    if let (Some(d), Some(p)) = (dpi, doc.page_mut(0))
        && d.is_finite()
        && d > 0.0
    {
        p.canvas.resolution = d;
        p.canvas.resolution_unit = 0;
    }
}

/// Neues Dokument der Größe `w × h` mit transparenter Leinwand.
pub fn empty_document(w: u32, h: u32) -> Document {
    Document::new(w as f32, h as f32, Argb(0x00FF_FFFF))
}

/// Legt Elemente (oben → unten) in eine Ebene der ersten Seite (Index in `LYLv`, unten → oben).
pub fn push_elements(doc: &mut Document, layer_index: usize, elements: Vec<Element>) {
    if let Some(p) = doc.page_mut(0)
        && let Some(LayerSlot::Own(l)) = p.root.layers.get_mut(layer_index)
        && let Some(c) = Arc::make_mut(l).cels.first_mut()
    {
        Arc::make_mut(c).elements.extend(elements.into_iter().map(Arc::new));
    }
}

/// Exporteinstellungen nach dem Quellformat (12 §5.1/§5.2: GIF ⇒ GIF mit Palette/Transparenz, JPEG ⇒
/// JPEG, PNG ⇒ PNG; sonst Original-Vorgabe).
pub fn export_for(fmt: FileFormat, has_transparency: bool) -> ExportSettings {
    let mut e = ExportSettings::default();
    match fmt {
        FileFormat::Gif => {
            if has_transparency {
                e.transparency = 2;
            }
        }
        FileFormat::Jpeg => {
            e.format = 1;
            e.color_mode = 1;
        }
        FileFormat::Png | FileFormat::Apng => {
            e.format = 2;
            e.color_mode = if has_transparency { 2 } else { 1 };
        }
        _ => {}
    }
    e
}

/// Dokument aus einem Einzelbild: Ebene „Hintergrund“ mit der Bitmap (JPEG zusätzlich eine leere
/// „Ebene 1“ darüber, Laufzeit 7a T7).
pub fn document_from_raster(r: RasterImage, fmt: FileFormat) -> Document {
    let mut doc = empty_document(r.width(), r.height());
    let mut ids = std::mem::take(&mut doc.ids);
    let mut tile = 0u32;
    let e = bitmap_element(&mut ids, &r.image, None, &mut tile);
    let name = r.layer_name.clone().unwrap_or_else(|| "Hintergrund".to_string());
    let jpeg = fmt == FileFormat::Jpeg;
    let extra_layer = if jpeg { Some(Layer::new(&mut ids, "Ebene 1", LayerKind::Normal, 1)) } else { None };
    doc.ids = ids;
    if let Some(p) = doc.page_mut(0) {
        if let Some(LayerSlot::Own(l)) = p.root.layers.get_mut(0) {
            Arc::make_mut(l).name = name;
        }
        if let Some(l) = extra_layer {
            p.root.layers.insert(1, LayerSlot::Own(Arc::new(l)));
        }
        p.root.export = Some(Box::new(export_for(fmt, r.has_transparency)));
    }
    push_elements(&mut doc, 0, vec![e]);
    set_resolution(&mut doc, r.dpi);
    doc
}

/// Exporteinstellungen einer Animation (12 §5.1: GIF-Animation; Index-Alpha bei Transparenz bzw.
/// nicht erweiterten Einzelbildern).
fn anim_export(a: &Animation, fmt: FileFormat, expanded: bool) -> ExportSettings {
    let mut e = ExportSettings::default();
    match fmt {
        FileFormat::Gif => {
            e.format = 4;
            if a.has_transparency || !expanded {
                e.transparency = 2;
            }
        }
        FileFormat::Apng => {
            e.format = 3;
            e.mac_type = Some(u32::from_be_bytes(*b"APNG"));
            e.mac_creator = Some(u32::from_be_bytes(*b"FPXL"));
            e.color_mode = 2;
        }
        _ => {
            e.format = 3;
            e.mac_type = Some(u32::from_be_bytes(*b"WBPA"));
            e.mac_creator = Some(u32::from_be_bytes(*b"FPXL"));
            e.color_mode = 2;
        }
    }
    e.loop_count = a.loop_count;
    e.frames = a.frames.iter().map(|f| FrameInfo { disposal: f.disposal as i32, delay: f.delay_cs.min(65535) as i32, visible: true, name: None, extra: Default::default() }).collect();
    e
}

/// Ebene mit einem Bild je Status.
fn frames_layer(ids: &mut IdGen, a: &Animation, expanded: bool, tile: &mut u32) -> Layer {
    let n = a.frames.len().max(1);
    let mut l = Layer::new(ids, "Ebene 1", LayerKind::Normal, n);
    let composed = if expanded { a.composed_frames() } else { Vec::new() };
    for (k, f) in a.frames.iter().enumerate() {
        let img = if expanded {
            composed.get(k).cloned().unwrap_or_else(|| f.image.clone())
        } else {
            f.image.translated(f.x - f.image.bounds().left, f.y - f.image.bounds().top)
        };
        let e = bitmap_element(ids, &img, None, tile);
        if let Some(c) = l.cels.get_mut(k) {
            Arc::make_mut(c).elements.push(Arc::new(e));
        }
    }
    l
}

/// Erweitern-Entscheidung (12 §5.1, `gif.expand_decision`): Voreinstellung an, erstes Einzelbild bei
/// (0, 0) umschließt alle anderen, Löschmethode des ersten < 2, weniger als 16 Einzelbilder.
pub fn expand_decision(a: &Animation, pref: bool) -> bool {
    let fr = &a.frames;
    if fr.len() < 2 || !pref {
        return false;
    }
    let f0 = &fr[0];
    let (ux0, uy0) = (fr.iter().map(|f| f.x).min().unwrap_or(0), fr.iter().map(|f| f.y).min().unwrap_or(0));
    let ux1 = fr.iter().map(|f| f.x + f.image.width() as i32).max().unwrap_or(0);
    let uy1 = fr.iter().map(|f| f.y + f.image.height() as i32).max().unwrap_or(0);
    f0.x == 0 && f0.y == 0 && f0.image.width() as i32 >= ux1 - ux0 && f0.image.height() as i32 >= uy1 - uy0 && f0.disposal < 2 && fr.len() < 16
}

/// Dokument aus einer Animation. Öffnen: ein Status je Einzelbild. Importieren: Animationssymbol
/// mit diesen Status und eine Instanz in „Ebene 1“ (12 §1.5, A).
pub fn document_from_animation(a: Animation, fmt: FileFormat, opts: &ImportOptions) -> Document {
    let expanded = fmt == FileFormat::Gif && expand_decision(&a, opts.expand_gif_frames) || fmt != FileFormat::Gif;
    let mut doc = empty_document(a.width, a.height);
    let mut ids = std::mem::take(&mut doc.ids);
    let mut tile = 0u32;
    let n = a.frames.len().max(1) as u32;
    let mut frames = a.clone();
    if fmt == FileFormat::Gif && !opts.expand_gif_frames {
        // Ohne Voreinstellung wird „nicht angegeben“ beim ersten Einzelbild zu „nicht entfernen“ (12 §5.1).
        if let Some(f) = frames.frames.first_mut()
            && f.disposal == 0
        {
            f.disposal = 1;
        }
    }
    let layer = frames_layer(&mut ids, &frames, expanded, &mut tile);
    let export = anim_export(&frames, fmt, expanded);
    if opts.importing {
        let sid = doc.new_symbol_id();
        let mut sym = Symbol::new(&mut ids, "Animation", sid, SymbolKind::Animation);
        sym.root.frame_count = n;
        sym.root.layers[0] = LayerSlot::Own(Arc::new(layer));
        if let LayerSlot::Own(w) = &mut sym.root.layers[1] {
            *w = Arc::new(Layer::new(&mut ids, "Webebene", LayerKind::Web, n as usize));
        }
        let mut inst = Element::new_instance(&mut ids, sid, fp_geom::Matrix3::IDENTITY);
        if let fp_model::ElementKind::Instance(i) = &mut inst.kind {
            i.anim.frames = n;
        }
        doc.symbols.push(Arc::new(sym));
        doc.ids = ids;
        push_elements(&mut doc, 0, vec![inst]);
        if let Some(p) = doc.page_mut(0) {
            p.root.export = Some(Box::new(export));
        }
    } else {
        let web = Layer::new(&mut ids, "Webebene", LayerKind::Web, n as usize);
        doc.ids = ids;
        if let Some(p) = doc.page_mut(0) {
            p.root.frame_count = n;
            p.root.layers = vec![LayerSlot::Own(Arc::new(layer)), LayerSlot::Own(Arc::new(web))];
            p.root.export = Some(Box::new(export));
        }
    }
    doc
}

/// Dispatcher nach Format.
pub fn import_format(fmt: FileFormat, name: &str, data: &[u8], opts: &ImportOptions) -> Result<ImportResult> {
    let raster = |r: RasterImage, f: FileFormat| ImportResult { document: document_from_raster(r, f), format: f, content: ImportedContent::Bitmap, warnings: vec![] };
    let anim = |a: Animation, f: FileFormat| {
        let n = a.frames.len();
        if n <= 1 {
            let mut fr = a.frames.into_iter().next();
            let img = fr.take().map(|f| f.image).unwrap_or_else(|| RgbaImage::empty_at(0, 0));
            let r = RasterImage { image: img, dpi: None, palette: a.palette, has_transparency: a.has_transparency, layer_name: None };
            return Ok(raster(r, f));
        }
        Ok(ImportResult { document: document_from_animation(a, f, opts), format: f, content: ImportedContent::Animation { frames: n }, warnings: vec![] })
    };
    match fmt {
        FileFormat::FwPng => Ok(ImportResult { document: fp_fwpng::read(data)?, format: fmt, content: ImportedContent::Layered, warnings: vec![] }),
        FileFormat::Png => Ok(raster(crate::png_in::decode_png(data)?, fmt)),
        FileFormat::Apng => anim(crate::png_in::decode_apng(data)?, fmt),
        FileFormat::Gif => anim(crate::gif::decode(data)?.into_animation(), fmt),
        FileFormat::Jpeg => Ok(raster(crate::jpeg_in::decode(data)?, fmt)),
        FileFormat::Bmp => Ok(raster(crate::bmp::decode(data)?, fmt)),
        FileFormat::Tga => Ok(raster(crate::tga::decode(data)?, fmt)),
        FileFormat::Wbmp => Ok(raster(crate::wbmp::decode(data)?, fmt)),
        FileFormat::Tiff => Ok(raster(crate::tiff_in::decode(data, opts.cmyk_table.as_deref())?, fmt)),
        FileFormat::Qoi => Ok(raster(crate::modern::decode_qoi(data)?, fmt)),
        FileFormat::WebP => anim(crate::modern::decode_webp(data)?, fmt),
        FileFormat::JpegXl => anim(crate::modern::decode_jxl(data)?, fmt),
        FileFormat::Ico => crate::modern::import_ico(data, opts),
        FileFormat::Avif => anim(crate::avif_in::decode_avif(data)?, fmt),
        FileFormat::Heif if crate::avif_in::is_av1_heif(data) => anim(crate::avif_in::decode_avif(data)?, FileFormat::Avif),
        FileFormat::Heif => Ok(raster(crate::heic_in::decode_heic(data)?, fmt)),
        FileFormat::Psd => crate::psd::import(data, opts),
        FileFormat::Svg => crate::vector::import_svg(data, opts),
        FileFormat::Illustrator | FileFormat::Eps | FileFormat::EpsBinary => crate::vector::import_postscript(data, fmt, opts),
        FileFormat::Pdf => crate::vector::import_pdf(data, opts),
        FileFormat::Act => Err(ImportError::Unsupported("Photoshop-Paletten (*.act) sind keine Bilder: im Optimieren-Bedienfeld „Palette laden“ bzw. in den Farbfeldern verwenden.".into())),
        FileFormat::Unknown => {
            let _ = name;
            Err(ImportError::UnknownFormat)
        }
    }
}
