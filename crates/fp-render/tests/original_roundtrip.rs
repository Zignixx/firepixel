//! Phase 5: Rundreise neu erzeugter Dokumente durch das Original.
//!
//! 1. `cargo test -p fp-render --release --test original_roundtrip -- --ignored --nocapture write_documents`
//!    schreibt `re\_testdateien\phase5\rt\rt_*.fw.png` (Rechteck-Grundform mit Effekten, Ellipse,
//!    Text, Bitmap, Gruppe mit Deckkraft, Mischmodus, Symbol mit zwei Instanzen, zwei Seiten, zwei
//!    Status).
//! 2. `re\scripts\fwsteuerung\tests\phase5\rt_oeffnen.jsf` (Original): öffnet jede Datei, protokolliert
//!    Seiten/Status/Ebenen/Elemente und speichert je Seite/Status eine Kopie `*_o_s<Seite>_f<Status>.fw.png`.
//! 3. `… compare_after_original` vergleicht Struktur und Pixel (Renderer gegen das flache Bild des Originals).

mod common;

use std::sync::Arc;

use fp_color::{Argb, BlendMode, Opacity};
use fp_geom::{Matrix3, Point, Rect, ELLIPSE_KAPPA};
use fp_model::{Cel, Contour, ContourNode, Document, Effect, EffectKind, EffectList, Element, ElementKind, Extra, Layer, LayerKind, LayerSlot, PathAttrs, Symbol, SymbolKind};
use fp_render::{RenderOptions, Renderer};

fn out_dir() -> std::path::PathBuf {
    common::rea().join("re").join("_testdateien").join("phase5").join("rt")
}

fn ellipse(doc: &mut Document, cx: f32, cy: f32, rx: f32, ry: f32, color: u32) -> Element {
    let (kx, ky) = (rx * ELLIPSE_KAPPA, ry * ELLIPSE_KAPPA);
    let n = |x: f32, y: f32, ix: f32, iy: f32, ox: f32, oy: f32| ContourNode {
        anchor: Point::new(x, y),
        pred: Point::new(ix, iy),
        succ: Point::new(ox, oy),
        curve: true,
        random: None,
        segment_length: None,
        pressure: None,
        extra: Extra::default(),
    };
    let nodes = vec![
        n(cx, cy - ry, cx - kx, cy - ry, cx + kx, cy - ry),
        n(cx + rx, cy, cx + rx, cy - ky, cx + rx, cy + ky),
        n(cx, cy + ry, cx + kx, cy + ry, cx - kx, cy + ry),
        n(cx - rx, cy, cx - rx, cy + ky, cx - rx, cy - ky),
    ];
    let c = Contour { closed: true, bsl: None, nodes, extra: Extra::default() };
    Element::new_path(&mut doc.ids, vec![c], PathAttrs::solid_fill(Argb(color)))
}

/// Erste Effektdefinition einer Art aus den Laufzeit-Testdateien (exakte Parametersätze des Originals).
fn effect_template(kind: EffectKind, pred: impl Fn(&Effect) -> bool) -> Option<Effect> {
    let mut files = Vec::new();
    for d in ["05", "02", "07", "13"] {
        if let Ok(rd) = std::fs::read_dir(common::rea().join("re").join("_testdateien").join(d)) {
            files.extend(rd.flatten().map(|e| e.path()));
        }
    }
    files.sort();
    for f in files {
        let Some(doc) = common::load(&f) else { continue };
        let mut found = None;
        for p in 0..doc.pages.len() {
            fp_model::traverse::walk_root(&doc.pages[p].root, fp_model::traverse::RootRef::Page(p), &mut |_, _, e| {
                if found.is_none()
                    && let Some(l) = &e.common.effects
                {
                    found = l.effects.iter().find(|x| x.kind() == kind && pred(x)).cloned();
                }
            });
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

fn drop_shadow() -> Effect {
    let mut fx = Effect::new("{a7944db8-6ce2-11d1-8c76000502701850}", true);
    for (k, v) in [("ShadowColor", "#000000a2"), ("ShadowBlur", "4"), ("ShadowDistance", "7"), ("ShadowAngle", "315"), ("ShadowType", "0")] {
        fx.set(k, v.to_string());
    }
    fx
}

fn with_effects(mut e: Element, fx: Vec<Effect>) -> Element {
    let list = Some(EffectList { effects: fx, ..EffectList::default() });
    if let ElementKind::Path(p) = &mut e.kind
        && let Some(pr) = &mut p.primitive
    {
        pr.effects = list.clone();
    }
    e.common.effects = list;
    e
}

fn named(mut e: Element, n: &str) -> Element {
    e.common.name = Some(n.into());
    e
}

fn text(doc: &mut Document, s: &str, size: f32, at: Point) -> Element {
    let mut t = Element::new_text(&mut doc.ids, s, "ArialMT", size, Argb(0xff20_2020), at);
    if let ElementKind::Text(te) = &mut t.kind {
        let block = fp_render::convert::text_block(doc, te);
        let fonts = fp_text::FontDb::system();
        let lay = fp_text::layout(&block, &fonts, &fp_text::LayoutOptions::default());
        te.bounds = lay.frame;
    }
    t
}

/// Ebenen einer Seite auf `frames` Status bringen.
fn set_frames(doc: &mut Document, page: usize, frames: u32) {
    let p = Arc::make_mut(&mut doc.pages[page]);
    p.root.frame_count = frames;
    for s in &mut p.root.layers {
        if let LayerSlot::Own(l) = s {
            let l = Arc::make_mut(l);
            while l.cels.len() < frames as usize {
                l.cels.push(Arc::new(Cel::empty()));
            }
        }
    }
}

fn put(doc: &mut Document, page: usize, frame: usize, elems: Vec<Element>) {
    let p = Arc::make_mut(&mut doc.pages[page]);
    if let LayerSlot::Own(l) = &mut p.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[frame]);
        cel.elements = elems.into_iter().map(Arc::new).collect();
    }
}

/// Dokument A: alle Grundobjekte, zwei Seiten, zwei Status.
fn build_a() -> Document {
    let mut doc = Document::new(320.0, 220.0, Argb(0xffff_ffff));
    let mut tile = 1u32;
    let rect = with_effects(Element::new_rectangle(&mut doc.ids, Rect::new(20.0, 20.0, 120.0, 90.0), 0.25, PathAttrs::solid_fill(Argb(0xff33_66cc))), vec![drop_shadow()]);
    let mut fx2 = Vec::new();
    if let Some(b) = effect_template(EffectKind::BevelFamily, |_| true) {
        fx2.push(b);
    }
    if let Some(g) = effect_template(EffectKind::InnerGlow, |_| true) {
        fx2.push(g);
    }
    let rect2 = with_effects(Element::new_rectangle(&mut doc.ids, Rect::new(150.0, 20.0, 230.0, 80.0), 0.0, PathAttrs::solid_fill(Argb(0xff22_aa55))), fx2);
    let ell = ellipse(&mut doc, 270.0, 55.0, 35.0, 30.0, 0xffff_8800);
    let mut mult = ellipse(&mut doc, 100.0, 85.0, 30.0, 20.0, 0xffcc_2244);
    mult.common.blend = BlendMode(2);
    let t = text(&mut doc, "Firepixel", 24.0, Point::new(20.0, 140.0));
    let px: Vec<u8> = (0..60 * 40).flat_map(|i| [(i % 60 * 4) as u8, (i / 60 * 6) as u8, 160, 255]).collect();
    let bmp = Element::new_bitmap(&mut doc.ids, Point::new(240.0, 150.0), 60, 40, &px, &mut tile);
    let g1 = Element::new_rectangle(&mut doc.ids, Rect::new(150.0, 110.0, 190.0, 150.0), 0.0, PathAttrs::solid_fill(Argb(0xff88_22cc)));
    let g2 = Element::new_rectangle(&mut doc.ids, Rect::new(170.0, 130.0, 210.0, 170.0), 0.0, PathAttrs::solid_fill(Argb(0xff22_88cc)));
    let mut grp = Element::new_group(&mut doc.ids, vec![Arc::new(g1), Arc::new(g2)]);
    grp.common.opacity = Opacity(128);
    let elems = vec![
        named(t, "Text"),
        named(mult, "Multiplizieren"),
        named(rect, "Rechteck"),
        named(rect2, "Abschraegung"),
        named(ell, "Ellipse"),
        named(bmp, "Bitmap"),
        named(grp, "Gruppe"),
    ];
    set_frames(&mut doc, 0, 2);
    put(&mut doc, 0, 0, elems);
    let moved = with_effects(Element::new_rectangle(&mut doc.ids, Rect::new(60.0, 100.0, 160.0, 170.0), 0.25, PathAttrs::solid_fill(Argb(0xff33_66cc))), vec![drop_shadow()]);
    put(&mut doc, 0, 1, vec![named(moved, "Rechteck Status 2")]);
    let p2 = doc.add_page("Seite 2");
    Arc::make_mut(&mut doc.pages[p2]).root.background = Argb(0xffe0_e8f0);
    let e2 = ellipse(&mut doc, 160.0, 110.0, 80.0, 50.0, 0xff44_aa99);
    let t2 = text(&mut doc, "Seite 2", 18.0, Point::new(120.0, 200.0));
    put(&mut doc, p2, 0, vec![named(t2, "Text 2"), named(e2, "Ellipse 2")]);
    doc
}

/// Dokument B: Grafiksymbol (Rechteck + Ellipse mit Mischmodus) und zwei Instanzen.
fn build_b() -> Document {
    let mut doc = Document::new(300.0, 200.0, Argb(0xffff_ffff));
    let sid = doc.new_symbol_id();
    let mut sym = Symbol::new(&mut doc.ids, "Knopf", sid, SymbolKind::Graphic);
    let r = Element::new_rectangle(&mut doc.ids, Rect::new(0.0, 0.0, 80.0, 50.0), 0.3, PathAttrs::solid_fill(Argb(0xff22_66dd)));
    let mut e = ellipse(&mut doc, 60.0, 25.0, 30.0, 20.0, 0xffee_aa00);
    e.common.blend = BlendMode(2);
    if let LayerSlot::Own(l) = &mut sym.root.layers[0] {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(named(e, "Ellipse")), Arc::new(named(r, "Rechteck"))];
    }
    doc.symbols.push(Arc::new(sym));
    let i1 = Element::new_instance(&mut doc.ids, sid, Matrix3::translation(20.0, 30.0));
    let i2 = Element::new_instance(&mut doc.ids, sid, Matrix3::scaling(1.5, 1.5) * Matrix3::translation(130.0, 90.0));
    put(&mut doc, 0, 0, vec![named(i2, "Instanz 2"), named(i1, "Instanz 1")]);
    let _ = (Layer::new, LayerKind::Normal);
    doc
}

/// Dokument C: eine Vorlage des Korpus (mehrseitig, Masterseite, ATE-Texte, Symbole) **bearbeitet**:
/// Element verschoben (Text und Pfad), Füllfarbe geändert, Element gelöscht, Rechteck mit Schatten
/// ergänzt.
fn build_c() -> Option<Document> {
    let src = common::corpus_files().into_iter().find(|p| p.ends_with("Entertainment.png"))?;
    let mut doc = common::load(&src)?;
    let page = doc.current_page;
    let p = Arc::make_mut(&mut doc.pages[page]);
    let mut done = (false, false, false);
    for s in p.root.layers.iter_mut() {
        let LayerSlot::Own(l) = s else { continue };
        if l.kind == LayerKind::Web {
            continue;
        }
        let l = Arc::make_mut(l);
        let Some(cel) = l.cels.first_mut() else { continue };
        let cel = Arc::make_mut(cel);
        for e in cel.elements.iter_mut() {
            let el = Arc::make_mut(e);
            match &mut el.kind {
                ElementKind::Text(t) if !done.0 => {
                    // Text 5 px nach rechts
                    t.bounds = Rect::new(t.bounds.left + 5.0, t.bounds.top, t.bounds.right + 5.0, t.bounds.bottom);
                    if let Some(a) = &mut t.anchor_rect {
                        *a = Rect::new(a.left + 5.0, a.top, a.right + 5.0, a.bottom);
                    }
                    if let Some(m) = &mut t.transform {
                        m.m[2][0] += 5.0;
                    }
                    el.cache = None;
                    done.0 = true;
                }
                ElementKind::Path(pe) if !done.1 && pe.primitive.is_none() => {
                    let a = Arc::make_mut(&mut pe.attrs);
                    a.fill_color = Argb(0xffcc_3311);
                    for c in &mut pe.contours {
                        for n in &mut c.nodes {
                            n.anchor.y += 3.0;
                            n.pred.y += 3.0;
                            n.succ.y += 3.0;
                        }
                    }
                    pe.polyline = None;
                    el.cache = None;
                    done.1 = true;
                }
                _ => {}
            }
        }
        if !done.2 && cel.elements.len() > 2 {
            cel.elements.remove(1);
            done.2 = true;
        }
    }
    let r = with_effects(Element::new_rectangle(&mut doc.ids, Rect::new(10.0, 10.0, 70.0, 40.0), 0.2, PathAttrs::solid_fill(Argb(0xff22_aa66))), vec![drop_shadow()]);
    let p = Arc::make_mut(&mut doc.pages[page]);
    if let Some(LayerSlot::Own(l)) = p.root.layers.iter_mut().rev().find(|s| matches!(s, LayerSlot::Own(l) if l.kind != LayerKind::Web)) {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements.insert(0, Arc::new(named(r, "Neu")));
    }
    doc.root_copy = None;
    Some(doc)
}

#[test]
#[ignore]
fn write_documents() {
    std::fs::create_dir_all(out_dir()).expect("Ordner");
    let r = Renderer::new(RenderOptions::default());
    let mut docs = vec![("rt_a", build_a()), ("rt_b", build_b())];
    if let Some(c) = build_c() {
        docs.push(("rt_c", c));
    }
    for (name, doc) in docs {
        let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &r).expect("schreiben");
        let p = out_dir().join(format!("{name}.fw.png"));
        std::fs::write(&p, &bytes).expect("Datei");
        let back = fp_fwpng::read(&bytes).expect("lesen");
        assert_eq!(back.pages.len(), doc.pages.len());
        println!("geschrieben: {} ({} Byte)", p.display(), bytes.len());
    }
}

fn describe(doc: &Document, page: usize, frame: usize) -> Vec<String> {
    let mut out = Vec::new();
    for s in &doc.pages[page].root.layers {
        if let LayerSlot::Own(l) = s {
            if l.kind == LayerKind::Web {
                continue;
            }
            let Some(c) = l.cels.get(frame).or_else(|| l.cels.first()) else { continue };
            for e in &c.elements {
                let fx = e.common.effects.as_ref().map(|l| l.effects.len()).unwrap_or(0);
                out.push(format!("{}:{}:fx{}:op{}:bl{}", fp_render::kind_tag(e), e.common.name.clone().unwrap_or_default(), fx, e.common.opacity.0, e.common.blend.0));
            }
        }
    }
    out
}

#[test]
#[ignore]
fn compare_after_original() {
    let mut total = (0u64, 0u64);
    for name in ["rt_a", "rt_b", "rt_c"] {
        let Some(mine) = common::load(&out_dir().join(format!("{name}.fw.png"))) else {
            println!("{name}: eigene Datei fehlt");
            continue;
        };
        for page in 0..mine.pages.len() {
            for frame in 0..mine.pages[page].root.frame_count as usize {
                let p = out_dir().join(format!("{name}_o_s{page}_f{frame}.fw.png"));
                let Some(orig) = common::load(&p) else {
                    println!("{} fehlt (zuerst rt_oeffnen.jsf im Original ausführen)", p.display());
                    continue;
                };
                let a = describe(&mine, page, frame);
                let b = describe(&orig, page, frame);
                println!("{name} Seite {page} Status {frame}: Struktur {}", if a == b { "gleich" } else { "VERSCHIEDEN" });
                if a != b {
                    println!("   Firepixel: {a:?}\n   Original:  {b:?}");
                }
                let rect = Renderer::page_rect(&orig, page).expect("Seite");
                // Referenzbild: PNG-32-Export des Originals für genau diese Seite/diesen Status
                // (das flache Bild der gespeicherten Datei zeigt immer die Seite im Fenster)
                let png_path = out_dir().join(format!("{name}_o_s{page}_f{frame}.png"));
                let Some(flat) = std::fs::read(&png_path).ok().and_then(|b| fp_fwpng::png::read_chunks(&b).ok()).and_then(|c| fp_fwpng::png::decode_image(&c).ok()).and_then(|f| fp_raster::RgbaImage::from_rgba_bytes(rect, &f.rgba).ok()) else {
                    println!("   {} fehlt", png_path.display());
                    continue;
                };
                for (label, pol) in [("Caches", fp_render::CachePolicy::Original), ("neu", fp_render::CachePolicy::TextOnly), ("ohne Caches", fp_render::CachePolicy::None)] {
                    let out = Renderer::new(RenderOptions { cache_policy: pol, ..RenderOptions::default() }).render_page(&orig, page, frame);
                    let c = out.image.compare(&flat, rect);
                    println!("   Bild des Originals gegen Firepixel ({label}): {}/{} gleich, max {}", c.equal, c.total, c.max_diff);
                    if label == "neu" {
                        total.0 += c.equal;
                        total.1 += c.total;
                    }
                }
                if name == "rt_b" {
                    for sbm in [false, true] {
                        let o = RenderOptions { cache_policy: fp_render::CachePolicy::TextOnly, symbol_blend_modes: sbm, ..RenderOptions::default() };
                        let c = Renderer::new(o).render_page(&orig, page, frame).image.compare(&flat, rect);
                        println!("   Mischmodi in Symbolen {}: {}/{} gleich, max {}", if sbm { "an" } else { "aus (Vorgabe)" }, c.equal, c.total, c.max_diff);
                    }
                }
                // eigenes Bild (aus dem eigenen Dokument) gegen das des Originals
                let out = Renderer::new(RenderOptions::default()).render_page(&mine, page, frame);
                let c = out.image.compare(&flat, rect);
                println!("   eigenes Dokument gerendert gegen Original: {}/{} gleich, max {}", c.equal, c.total, c.max_diff);
                if let Some(dir) = std::env::var_os("FP_DUMP_DIR") {
                    // Abweichungen rot auf grau
                    let (w, h) = (rect.width(), rect.height());
                    let mut buf = Vec::with_capacity((w * h * 4) as usize);
                    for (a, b) in out.image.data().iter().zip(flat.data()) {
                        if a == b || (a[3] == 0 && b[3] == 0) {
                            let g = (a[0] as u16 + a[1] as u16 + a[2] as u16) / 6 + 128;
                            buf.extend_from_slice(&[g as u8, g as u8, g as u8, 255]);
                        } else {
                            buf.extend_from_slice(&[255, 0, 0, 255]);
                        }
                    }
                    let f = std::fs::File::create(std::path::Path::new(&dir).join(format!("diff_{name}_s{page}_f{frame}.png"))).expect("Datei");
                    let mut enc = png::Encoder::new(f, w, h);
                    enc.set_color(png::ColorType::Rgba);
                    enc.set_depth(png::BitDepth::Eight);
                    enc.write_header().expect("Kopf").write_image_data(&buf).expect("Daten");
                }
            }
        }
    }
    println!("gesamt (neu gerastert): {}/{}", total.0, total.1);
}
