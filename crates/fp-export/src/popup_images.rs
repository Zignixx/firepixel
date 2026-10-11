//! Grafik-Menübilder der Pop-up-Menüs (15 §3.2): je Menü ein Zellbild „Up“ und „Over“ in Zellgröße
//! (`menuWidth` × `menuHeight`). Wie das Original (B: `@007779a0`) wird die Zelle als Rechteck mit
//! der Zellfarbe (`BCLi` bzw. `BCMi`) und dem Stil (`ZSTv` Up bzw. Over) gerendert: Füllart, Strich
//! und Live-Effekte kommen aus dem Stil, die Füllfarbe ist die Zellfarbe (A, die genaue
//! Pixeldarstellung ist nicht spezifiziert). Text gehört nicht ins Bild (er steht im HTML).

use std::sync::Arc;

use fp_color::Argb;
use fp_geom::Rect;
use fp_model::{Document, Element, ElementKind, LayerSlot, PathAttrs, PopupMenu, Style};
use fp_raster::RgbaImage;

use crate::export::{export_image, EncodeExtras};
use crate::html::MenuImage;
use crate::options::{ExportFormat, OptimizeSettings};
use crate::pipeline::ExportContext;
use crate::render::{Background, PageRenderer, RenderRequest};
use crate::Result;

/// Dokument mit einer Zelle (Rechteck in Zellgröße, Zellfarbe + Stil).
pub fn cell_document(w: u32, h: u32, color: u32, style: Option<&Style>) -> Document {
    let mut doc = Document::new(w as f32, h as f32, Argb(0x00FF_FFFF));
    let mut attrs = match style.and_then(|s| s.attrs.as_deref()) {
        Some(a) => a.clone(),
        None => PathAttrs::solid_fill(Argb(color)),
    };
    attrs.fill_color = Argb(color | 0xFF00_0000);
    if attrs.fill.is_none() {
        attrs.fill = Some(fp_model::Fill::solid());
    }
    if let Some(s) = style
        && !s.use_brush()
    {
        attrs.brush = None;
    }
    let mut e = Element::new_rectangle(&mut doc.ids, Rect::new(0.0, 0.0, w as f32, h as f32), 0.0, attrs);
    if let Some(s) = style {
        if s.use_effects() {
            e.common.effects = s.effects.clone();
        }
        if let ElementKind::Path(p) = &mut e.kind {
            if let Some(sp) = s.stroke_placement {
                p.stroke_placement = sp;
            }
            if let Some(f) = s.fill_on_top {
                p.fill_on_top = f;
            }
        }
    }
    let p = Arc::make_mut(&mut doc.pages[0]);
    if let Some(LayerSlot::Own(l)) = p.root.layers.iter_mut().find(|s| matches!(s, LayerSlot::Own(l) if l.kind != fp_model::LayerKind::Web))
        && let Some(c) = Arc::make_mut(l).cels.first_mut()
    {
        Arc::make_mut(c).elements.push(Arc::new(e));
    }
    doc
}

/// Zellbild (Up = `over == false`).
pub fn cell_image(r: &dyn PageRenderer, menu: &PopupMenu, w: u32, h: u32, over: bool) -> Result<RgbaImage> {
    let (color, style) = if over { (menu.cell_over_color.unwrap_or(0xFFCC_CCCC), menu.styles.get(1)) } else { (menu.cell_up_color.unwrap_or(0xFFFF_FFFF), menu.styles.first()) };
    let doc = cell_document(w, h, color, style);
    r.render(&doc, &RenderRequest { background: Background::Transparent, ..RenderRequest::default() }).map_err(crate::ExportError::Render)
}

/// Kodiert die Menübilder (GIF wie die Dateinamen; Matte = Zellfarbe).
pub fn menu_image_files(r: &dyn PageRenderer, jobs: &[MenuImage], ctx: &ExportContext, ex: &EncodeExtras) -> Result<Vec<(String, Vec<u8>)>> {
    let mut out = Vec::new();
    for j in jobs {
        for (over, path) in [(false, &j.path_up), (true, &j.path_over)] {
            let img = cell_image(r, &j.menu, j.width, j.height, over)?;
            let fmt = if path.ends_with(".png") { ExportFormat::Png } else if path.ends_with(".jpg") { ExportFormat::Jpeg } else { ExportFormat::Gif };
            let s = OptimizeSettings { format: fmt, ..OptimizeSettings::default() };
            out.push((path.clone(), export_image(&img, &s, ctx, ex)?.bytes));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_document_uses_color_and_style_effects() {
        let mut st = Style::new("Knopf");
        st.effects = Some(fp_model::EffectList { effects: vec![fp_model::Effect::new("{a7944db8-6ce2-11d1-8c76000502701850}", true)], ..Default::default() });
        let d = cell_document(80, 20, 0xFF33_6699, Some(&st));
        let mut found = false;
        fp_model::traverse::walk_root(&d.pages[0].root, fp_model::traverse::RootRef::Page(0), &mut |_, _, e| {
            if let ElementKind::Path(p) = &e.kind {
                found = true;
                assert_eq!(p.attrs.fill_color, Argb(0xFF33_6699));
                assert_eq!(e.common.effects.as_ref().map(|l| l.effects.len()), Some(1));
            }
        });
        assert!(found);
    }
}
