//! API-Tests des Renderers mit synthetischen Dokumenten (laufen bei `cargo test`).

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity, Rgba8};
use fp_effects::params::format_color;
use fp_fwpng::Flattener;
use fp_geom::{Matrix3, Point, Rect};
use fp_model::{
    Contour, ContourNode, Document, Effect, EffectList, Element, ElementKind, Extra, LayerSlot,
    NineSlice, PathAttrs, Symbol, SymbolKind,
};
use fp_raster::IRect;
use fp_render::{CachePolicy, HitOptions, PointMap, RenderOptions, Renderer};

fn opts() -> RenderOptions {
    RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    }
}

fn rect(doc: &mut Document, r: Rect, color: u32) -> Element {
    Element::new_rectangle(&mut doc.ids, r, 0.0, PathAttrs::solid_fill(Argb(color)))
}

/// Elemente (oben → unten) in die erste Ebene der aktiven Seite legen.
fn put(doc: &mut Document, elems: Vec<Element>) {
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = elems.into_iter().map(Arc::new).collect();
    }
}

fn first_element(doc: &Document) -> Arc<Element> {
    match &doc.pages[0].root.layers[0] {
        LayerSlot::Own(l) => Arc::clone(&l.cels[0].elements[0]),
        _ => panic!(),
    }
}

#[test]
fn rectangle_on_white_page() {
    let mut doc = Document::new(50.0, 40.0, Argb(0xffff_ffff));
    let r = rect(&mut doc, Rect::new(10.0, 10.0, 30.0, 30.0), 0xffff_0000);
    put(&mut doc, vec![r]);
    let out = Renderer::new(opts()).render_page(&doc, 0, 0);
    assert!(out.report.is_complete(), "{:?}", out.report);
    assert_eq!(out.image.bounds(), IRect::new(0, 0, 50, 40));
    assert_eq!(out.image.pixel(20, 20), Rgba8::rgb(255, 0, 0));
    assert_eq!(out.image.pixel(5, 5), Rgba8::rgb(255, 255, 255));
    assert_eq!(out.report.rasterized_paths, 1);
}

#[test]
fn opacity_blend_and_hidden() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xff00_00ff));
    let mut a = rect(&mut doc, Rect::new(0.0, 0.0, 20.0, 20.0), 0xffff_ffff);
    a.common.opacity = Opacity(128);
    let mut b = rect(&mut doc, Rect::new(20.0, 0.0, 40.0, 20.0), 0xffff_0000);
    b.common.blend = BlendMode(2); // multiply
    let mut hidden = rect(&mut doc, Rect::new(0.0, 20.0, 40.0, 40.0), 0xff00_ff00);
    hidden.common.visible = false;
    let slice = Element::new_slice(&mut doc.ids, Rect::new(0.0, 0.0, 40.0, 40.0));
    put(&mut doc, vec![slice, a, b, hidden]);
    let img = Renderer::new(opts()).render_page(&doc, 0, 0).image;
    let p = img.pixel(5, 5);
    assert!(p.r > 100 && p.r < 140 && p.b == 255, "{p:?}");
    assert_eq!(img.pixel(30, 5), Rgba8::rgb(0, 0, 0)); // Rot multipliziert mit Blau
    assert_eq!(img.pixel(5, 30), Rgba8::rgb(0, 0, 255));
}

#[test]
fn group_opacity_is_applied_to_the_whole() {
    let mut doc = Document::new(30.0, 20.0, Argb(0xffff_ffff));
    let a = rect(&mut doc, Rect::new(0.0, 0.0, 20.0, 20.0), 0xff00_00ff);
    let b = rect(&mut doc, Rect::new(10.0, 0.0, 30.0, 20.0), 0xffff_0000);
    let mut g = Element::new_group(&mut doc.ids, vec![Arc::new(b), Arc::new(a)]);
    g.common.opacity = Opacity(128);
    put(&mut doc, vec![g]);
    let img = Renderer::new(opts()).render_page(&doc, 0, 0).image;
    // Überlappung: nur das obere Rechteck (rot) mit 50 % – nicht rot über blau.
    let p = img.pixel(15, 10);
    assert_eq!(p.b, img.pixel(25, 10).b, "{p:?}");
}

#[test]
fn vector_mask_cuts_content() {
    let mut doc = Document::new(40.0, 40.0, Argb(0x00ff_ffff));
    let content = rect(&mut doc, Rect::new(0.0, 0.0, 40.0, 40.0), 0xff00_ff00);
    let mask = rect(&mut doc, Rect::new(10.0, 10.0, 20.0, 20.0), 0xff00_0000);
    let g = Element::new_mask_group(&mut doc.ids, Arc::new(mask), vec![Arc::new(content)], true);
    put(&mut doc, vec![g]);
    let img = Renderer::new(opts()).render_page(&doc, 0, 0).image;
    assert_eq!(img.pixel(15, 15), Rgba8::rgb(0, 255, 0));
    assert_eq!(img.pixel(30, 30).a, 0);
}

#[test]
fn drop_shadow_effect_is_rendered() {
    let mut doc = Document::new(60.0, 60.0, Argb(0x00ff_ffff));
    let mut r = rect(&mut doc, Rect::new(10.0, 10.0, 30.0, 30.0), 0xffff_0000);
    let mut fx = Effect::new("{a7944db8-6ce2-11d1-8c76000502701850}", true);
    for (k, v) in [
        ("ShadowColor", format_color(Rgba8::new(0, 0, 0, 255))),
        ("ShadowBlur", "0".into()),
        ("ShadowDistance", "7".into()),
        ("ShadowAngle", "315".into()),
        ("ShadowType", "0".into()),
    ] {
        fx.set(k, v);
    }
    r.common.effects = Some(EffectList {
        effects: vec![fx],
        ..EffectList::default()
    });
    put(&mut doc, vec![r]);
    let out = Renderer::new(opts()).render_page(&doc, 0, 0);
    assert_eq!(out.report.effect_chains, 1);
    // Versatz (4, 4) bei 315°/7 px (abgeschnitten): Schatten bei (32, 32), Objekt bleibt rot.
    assert_eq!(out.image.pixel(32, 32), Rgba8::new(0, 0, 0, 255));
    assert_eq!(out.image.pixel(20, 20), Rgba8::rgb(255, 0, 0));
}

fn doc_with_symbol(nine: bool) -> (Document, fp_model::SymbolId) {
    let mut doc = Document::new(300.0, 100.0, Argb(0x00ff_ffff));
    let id = doc.new_symbol_id();
    let mut sym = Symbol::new(&mut doc.ids, "S", id, SymbolKind::Graphic);
    let body = rect(&mut doc, Rect::new(-50.0, -25.0, 50.0, 25.0), 0xff00_80ff);
    let corner = rect(&mut doc, Rect::new(-50.0, -25.0, -40.0, -15.0), 0xffff_0000);
    if let LayerSlot::Own(l) = &mut sym.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = vec![Arc::new(corner), Arc::new(body)];
    }
    if nine {
        sym.nine_slice = Some(NineSlice {
            locked: None,
            color: None,
            guides: vec![-15.0, 15.0, -40.0, 40.0],
            bounds: Some(Rect::new(-50.0, -25.0, 50.0, 25.0)),
            bounds_extra: Extra::default(),
            view: None,
            view_extra: Extra::default(),
            extra: Extra::default(),
        });
    }
    doc.symbols.push(Arc::new(sym));
    (doc, id)
}

#[test]
fn instance_translation_and_nine_slice() {
    let (mut doc, id) = doc_with_symbol(false);
    let inst = Element::new_instance(&mut doc.ids, id, Matrix3::translation(60.0, 50.0));
    put(&mut doc, vec![inst]);
    let img = Renderer::new(opts()).render_page(&doc, 0, 0).image;
    assert_eq!(img.pixel(15, 30), Rgba8::rgb(255, 0, 0));
    assert_eq!(img.pixel(60, 50), Rgba8::rgb(0, 128, 255));

    // 9-Slice: doppelte Breite, die rote Ecke (10×10) bleibt 10 px breit.
    let (mut doc, id) = doc_with_symbol(true);
    let inst = Element::new_instance(
        &mut doc.ids,
        id,
        Matrix3::from_affine(2.0, 0.0, 0.0, 1.0, 150.0, 50.0),
    );
    put(&mut doc, vec![inst]);
    let img = Renderer::new(opts()).render_page(&doc, 0, 0).image;
    assert_eq!(img.pixel(55, 30), Rgba8::rgb(255, 0, 0));
    assert_eq!(img.pixel(61, 30), Rgba8::rgb(0, 128, 255));
    assert_eq!(img.pixel(249, 50), Rgba8::rgb(0, 128, 255));
    assert_eq!(img.pixel(251, 50).a, 0);
}

#[test]
fn hit_test_finds_topmost() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let top = rect(&mut doc, Rect::new(5.0, 5.0, 25.0, 25.0), 0xffff_0000);
    let bottom = rect(&mut doc, Rect::new(15.0, 15.0, 35.0, 35.0), 0xff00_ff00);
    let (tid, bid) = (top.id, bottom.id);
    put(&mut doc, vec![top, bottom]);
    let r = Renderer::new(opts());
    let h = r
        .hit_test(&doc, 0, 0, 20.0, 20.0, &HitOptions::default())
        .expect("Treffer");
    assert_eq!(h.element, tid);
    assert_eq!(
        r.hit_test(&doc, 0, 0, 30.0, 30.0, &HitOptions::default())
            .map(|h| h.element),
        Some(bid)
    );
    assert!(
        r.hit_test(&doc, 0, 0, 1.0, 1.0, &HitOptions::default())
            .is_none()
    );
}

#[test]
fn region_with_zoom_and_cache_invalidation() {
    let mut doc = Document::new(40.0, 40.0, Argb(0xffff_ffff));
    let r0 = rect(&mut doc, Rect::new(10.0, 10.0, 20.0, 20.0), 0xffff_0000);
    put(&mut doc, vec![r0]);
    let r = Renderer::new(RenderOptions {
        cache_policy: CachePolicy::None,
        ..RenderOptions::default()
    });
    let out = r.render_region(&doc, 0, 0, IRect::new(8, 8, 16, 16), 4.0);
    assert_eq!(out.image.bounds(), IRect::new(0, 0, 32, 32));
    assert_eq!(out.image.pixel(0, 0), Rgba8::rgb(255, 255, 255));
    assert_eq!(out.image.pixel(8, 8), Rgba8::rgb(255, 0, 0));
    let _ = r.render_page(&doc, 0, 0);
    let s = r.with_cache(|c| c.stats());
    assert!(s.hits >= 1, "{s:?}");
    // Bearbeitung über Copy-on-Write: neue Farbe wird sichtbar (Cache ungültig über den Zeiger).
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        let e = Element::edit(&mut cel.elements[0]);
        if let ElementKind::Path(p) = &mut e.kind {
            Arc::make_mut(&mut p.attrs).fill_color = Argb(0xff00_00ff);
        }
    }
    let img = r.render_page(&doc, 0, 0).image;
    assert_eq!(img.pixel(15, 15), Rgba8::rgb(0, 0, 255));
}

#[test]
fn flattener_and_writer() {
    let mut doc = Document::new(64.0, 48.0, Argb(0xffff_ffff));
    let r0 = rect(&mut doc, Rect::new(4.0, 4.0, 40.0, 30.0), 0xff33_6699);
    put(&mut doc, vec![r0]);
    let r = Renderer::new(opts());
    let flat = r.flatten(&doc, 0).expect("flach");
    assert_eq!((flat.width, flat.height), (64, 48));
    let small = r.render_scaled(&doc, 0, 0.5, 32, 24).expect("klein");
    assert_eq!(small.rgba.len(), 32 * 24 * 4);
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &r).expect("schreiben");
    let back = fp_fwpng::read(&bytes).expect("lesen");
    let img = back.file_meta.flat_image.clone().expect("IDAT");
    assert_eq!(img.rgba, flat.rgba);
    let e = first_element(&back);
    let look = r.render_element(&back, &e, &PointMap::identity(), 0);
    assert_eq!(look.image.pixel(10, 10), Rgba8::rgb(0x33, 0x66, 0x99));
    let _ = Point::default();
    let _ = ContourNode::corner(Point::default());
    let _: Option<Contour> = None;
}
