//! Rundreise mit dem echten Original.
//!
//! 1. `cargo test -p fp-render --release --test roundtrip -- --ignored --nocapture write_document`
//!    erzeugt `re\_testdateien\firepixel\x.fw.png` (Rechteck mit Schlagschatten, Ellipse, Text)
//!    mit `fp_fwpng::write` und dem Renderer als `Flattener` (echtes IDAT und prVW).
//! 2. `re\scripts\fwsteuerung\tests\firepixel\rundreise.jsf` öffnet die Datei im Original, zählt
//!    die Elemente und speichert sie als `x_fw.fw.png`.
//! 3. `… -- --ignored --nocapture compare_after_original` vergleicht Struktur und Pixel.

mod common;

use std::sync::Arc;

use fp_color::{Argb, Rgba8};
use fp_effects::params::format_color;
use fp_geom::{ELLIPSE_KAPPA, Point, Rect};
use fp_model::{
    Contour, ContourNode, Document, Effect, EffectList, Element, ElementKind, Extra, LayerSlot,
    PathAttrs,
};
use fp_render::{RenderOptions, Renderer};

fn out_dir() -> std::path::PathBuf {
    common::rea()
        .join("re")
        .join("_testdateien")
        .join("firepixel")
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
    let c = Contour {
        closed: true,
        bsl: None,
        nodes,
        extra: Extra::default(),
    };
    Element::new_path(&mut doc.ids, vec![c], PathAttrs::solid_fill(Argb(color)))
}

fn build() -> Document {
    let mut doc = Document::new(240.0, 160.0, Argb(0xffff_ffff));
    let mut r = Element::new_rectangle(
        &mut doc.ids,
        Rect::new(20.0, 20.0, 110.0, 80.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xff33_66cc)),
    );
    let mut fx = Effect::new("{a7944db8-6ce2-11d1-8c76000502701850}", true);
    for (k, v) in [
        ("ShadowColor", format_color(Rgba8::new(0, 0, 0, 0xa2))),
        ("ShadowBlur", "4".to_string()),
        ("ShadowDistance", "7".to_string()),
        ("ShadowAngle", "315".to_string()),
        ("ShadowType", "0".to_string()),
    ] {
        fx.set(k, v);
    }
    let list = EffectList {
        effects: vec![fx],
        ..EffectList::default()
    };
    // Rechteck-Primitive: Das Original liest die Effekte aus `PRIv/EFLv` (erste Rundreise ohne diese
    // Kopie: Effekt verloren); die Liste am Pfad bleibt für alte Leser.
    if let ElementKind::Path(p) = &mut r.kind
        && let Some(pr) = &mut p.primitive
    {
        pr.effects = Some(list.clone());
    }
    r.common.effects = Some(list);
    r.common.name = Some("Rechteck".into());
    let mut e = ellipse(&mut doc, 170.0, 55.0, 45.0, 30.0, 0xffff_8800);
    e.common.name = Some("Ellipse".into());
    let mut t = Element::new_text(
        &mut doc.ids,
        "Firepixel",
        "ArialMT",
        24.0,
        Argb(0xff20_2020),
        Point::new(20.0, 130.0),
    );
    t.common.name = Some("Text".into());
    // Rahmen wie das Original aus dem Satz übernehmen.
    if let ElementKind::Text(te) = &mut t.kind {
        let block = fp_render::convert::text_block(&doc, te);
        let fonts = fp_text::FontDb::system();
        let lay = fp_text::layout(&block, &fonts, &fp_text::LayoutOptions::default());
        te.bounds = lay.frame;
    }
    let page = Arc::make_mut(&mut doc.pages[0]);
    if let LayerSlot::Own(l) = &mut page.root.layers[0] {
        let cel = Arc::make_mut(&mut Arc::make_mut(l).cels[0]);
        cel.elements = vec![Arc::new(t), Arc::new(e), Arc::new(r)];
    }
    doc
}

#[test]
#[ignore]
fn write_document() {
    let doc = build();
    let r = Renderer::new(RenderOptions::default());
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &r).expect("schreiben");
    std::fs::create_dir_all(out_dir()).expect("Ordner");
    let p = out_dir().join("x.fw.png");
    std::fs::write(&p, &bytes).expect("Datei");
    let back = fp_fwpng::read(&bytes).expect("lesen");
    assert_eq!(back.pages.len(), 1);
    println!("geschrieben: {} ({} Byte)", p.display(), bytes.len());
}

#[test]
#[ignore]
fn compare_after_original() {
    let p = out_dir().join("x_fw.fw.png");
    let Some(doc) = common::load(&p) else {
        println!("{} fehlt (zuerst rundreise.jsf ausführen)", p.display());
        return;
    };
    let mut kinds = Vec::new();
    fp_model::traverse::walk_root(
        &doc.pages[0].root,
        fp_model::traverse::RootRef::Page(0),
        &mut |_, _, e| {
            kinds.push(format!(
                "{}:{}",
                fp_render::kind_tag(e),
                e.common.name.clone().unwrap_or_default()
            ));
        },
    );
    println!("Elemente nach dem Original: {kinds:?}");
    let rect = Renderer::page_rect(&doc, 0).expect("Seite");
    let flat = common::flat(&doc, rect).expect("IDAT");
    for (name, pol) in [
        ("original", fp_render::CachePolicy::Original),
        ("ohne_caches", fp_render::CachePolicy::TextOnly),
    ] {
        let out = Renderer::new(RenderOptions {
            cache_policy: pol,
            ..RenderOptions::default()
        })
        .render_page(&doc, 0, 0);
        let c = out.image.compare(&flat, rect);
        println!(
            "{name}: IDAT des Originals gegen Firepixel {}/{} max {} fehlt {:?}",
            c.equal, c.total, c.max_diff, out.report.missing
        );
    }
    // Eigenes IDAT gegen das des Originals.
    if let Some(mine) = common::load(&out_dir().join("x.fw.png"))
        && let Some(my_flat) = common::flat(&mine, rect)
    {
        let c = my_flat.compare(&flat, rect);
        println!(
            "IDAT Firepixel (x.fw.png) gegen IDAT Original (x_fw.fw.png): {}/{} max {}",
            c.equal, c.total, c.max_diff
        );
        let text = fp_raster::IRect::new(18, 100, 115, 135);
        let ct = my_flat.compare(&flat, text);
        println!(
            "  davon abweichend im Textbereich {text:?}: {}",
            ct.total - ct.equal
        );
    }
}
