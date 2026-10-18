//! BA-102: the PSD export writes one layer per object in a group per layer, and with the
//! "editable" options type layers (`TySh`), fill layers with vector masks (`SoCo` + `vmsk`) and
//! layer effects (`lfx2`). Read back with the PSD reader and the importer of fp-io.

use std::sync::Arc;

use fp_color::Argb;
use fp_export::doc_export::{DocExportOptions, FilesStyle, PanelOptions, export_document};
use fp_export::export::EncodeExtras;
use fp_export::psd_objects::PsdEditable;
use fp_export::render::SimpleRenderer;
use fp_io::psd::reader::{LayerKind, read};
use fp_model::effects::EffectKind;
use fp_model::{Document, Effect, EffectList, Element, ElementKind, LayerSlot, PathAttrs, Rect};

fn effect(kind: EffectKind, params: &[(&str, &str)]) -> Effect {
    let mut e = Effect::new(kind.moa_id().expect("moa id"), true);
    for (k, v) in params {
        e.set(k, *v);
    }
    e
}

fn doc() -> Document {
    let mut d = Document::new(200.0, 100.0, Argb(0xFFFF_FFFF));
    let mut ids = std::mem::take(&mut d.ids);
    let mut rect = Element::new_rectangle(
        &mut ids,
        Rect::new(10.0, 10.0, 60.0, 40.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xFFFF_0000)),
    );
    rect.common.name = Some("Box".into());
    rect.common.effects = Some(EffectList {
        effects: vec![
            effect(
                EffectKind::DropShadow,
                &[
                    ("ShadowColor", "#00000080"),
                    ("ShadowBlur", "4"),
                    ("ShadowDistance", "7"),
                    ("ShadowAngle", "315"),
                    ("ShadowType", "0"),
                ],
            ),
            effect(
                EffectKind::BevelFamily,
                &[
                    ("BevelType", "4"),
                    ("GlowWidth", "8"),
                    ("MaskSoftness", "4"),
                    ("BevelContrast", "75"),
                    ("OuterBevelColor", "#ffff00"),
                ],
            ),
        ],
        ..EffectList::default()
    });
    let mut text = Element::new_text(
        &mut ids,
        "Hello",
        "ArialMT",
        24.0,
        Argb(0xFF00_80FF),
        fp_geom::Point::new(20.0, 70.0),
    );
    text.common.effects = Some(EffectList {
        effects: vec![
            effect(EffectKind::InnerShadow, &[("ShadowDistance", "3")]),
            // no PSD counterpart: stays baked into the pixels
            effect(EffectKind::GaussianBlur, &[("blur_radius", "1")]),
        ],
        ..EffectList::default()
    });
    d.ids = ids;
    if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
        let l = Arc::make_mut(l);
        l.name = "Layer A".into();
        // cel order: top first
        Arc::make_mut(&mut l.cels[0]).elements = vec![Arc::new(text), Arc::new(rect)];
    }
    d
}

fn export(d: &Document, ed: PsdEditable) -> Vec<u8> {
    let mut o = DocExportOptions::new("t");
    o.style = FilesStyle::Psd;
    o.panel = Some(PanelOptions {
        psd_editable: ed,
        ..PanelOptions::default()
    });
    let files = export_document(d, &SimpleRenderer, &o, &EncodeExtras::default()).expect("export");
    files
        .into_iter()
        .find(|f| f.path.ends_with(".psd"))
        .expect("psd")
        .bytes
}

#[test]
fn psd_export_one_layer_per_object_with_editable_blocks() {
    let all = PsdEditable {
        effects: true,
        vector: true,
        text: true,
    };
    let psd = read(&export(&doc(), all)).expect("read");
    let kinds: Vec<LayerKind> = psd.layers.iter().map(|l| l.kind()).collect();
    // group end, rectangle (bottom), text, group header
    assert_eq!(psd.layers.len(), 4, "{kinds:?}");
    assert_eq!(kinds[0], LayerKind::SectionDivider);
    assert_eq!(kinds[3], LayerKind::Section { open: true });
    assert_eq!(psd.layers[3].name, "Layer A");

    // rectangle: solid colour fill layer with vector mask and layer effects
    let shape = &psd.layers[1];
    assert_eq!(shape.name, "Box");
    assert_eq!(kinds[1], LayerKind::Content);
    let (key, fill) = shape.fill.as_ref().expect("SoCo");
    assert_eq!(key, "SoCo");
    let c = fill.obj("color").expect("colour");
    assert_eq!(
        (c.num("red"), c.num("green"), c.num("blue")),
        (Some(255.0), Some(0.0), Some(0.0))
    );
    let (subs, _) = shape.vector_mask.as_ref().expect("vmsk");
    assert_eq!(subs.len(), 1);
    assert!(subs[0].closed);
    let xs: Vec<f64> = subs[0].knots.iter().map(|k| k[1].0).collect();
    let ys: Vec<f64> = subs[0].knots.iter().map(|k| k[1].1).collect();
    let rng = |v: &[f64]| {
        (
            v.iter().cloned().fold(f64::MAX, f64::min),
            v.iter().cloned().fold(f64::MIN, f64::max),
        )
    };
    let (x0, x1) = rng(&xs);
    let (y0, y1) = rng(&ys);
    assert!(
        (x0 - 10.0).abs() < 0.01 && (x1 - 60.0).abs() < 0.01,
        "{xs:?}"
    );
    assert!(
        (y0 - 10.0).abs() < 0.01 && (y1 - 40.0).abs() < 0.01,
        "{ys:?}"
    );
    let fx = shape.effects.as_ref().expect("lfx2");
    let ds = fx.obj("dropShadow").expect("drop shadow");
    assert_eq!(ds.bool("enabled"), Some(true));
    assert_eq!(ds.num("distance"), Some(7.0));
    assert_eq!(ds.num("blur"), Some(4.0));
    assert_eq!(ds.num("localLightingAngle"), Some(135.0));
    assert_eq!(ds.num("opacity"), Some(50.0));
    let og = fx.obj("outerGlow").expect("outer glow");
    assert_eq!(og.num("opacity"), Some(75.0));

    // text: type layer with text, font, size and colour
    let text = &psd.layers[2];
    assert_eq!(kinds[2], LayerKind::Text);
    let (m, s, _, engine) = text.text.as_ref().expect("TySh");
    assert_eq!(s, "Hello\r");
    assert_eq!((m[0], m[3]), (1.0, 1.0));
    assert!(engine.contains("/FontSize 24.0000"));
    assert!(engine.contains("/FontSet [ << /Name (\u{fe}\u{ff}\0A\0r\0i\0a\0l\0M\0T)"));
    let ts = text.effects.as_ref().expect("lfx2 on text");
    assert!(ts.obj("innerShadow").is_some());

    // importer: the type layer becomes a text object again, the fill layer a path, the effects
    // live effects
    let data = export(&doc(), all);
    let imp = fp_io::psd::import(&data, &fp_io::ImportOptions::default()).expect("import");
    let mut kinds_in = Vec::new();
    for p in &imp.document.pages {
        for slot in &p.root.layers {
            if let LayerSlot::Own(l) = slot {
                for c in &l.cels {
                    for e in &c.elements {
                        kinds_in.push(match &e.kind {
                            ElementKind::Text(t) => {
                                let s: String = t.runs.iter().map(|r| r.text.as_str()).collect();
                                format!("text:{s}")
                            }
                            ElementKind::Path(_) => "path".to_string(),
                            ElementKind::Bitmap(_) => "bitmap".to_string(),
                            _ => "other".to_string(),
                        });
                    }
                }
            }
        }
    }
    assert!(kinds_in.iter().any(|k| k == "path"), "{kinds_in:?}");
    assert!(
        kinds_in.iter().any(|k| k.starts_with("text:Hello")),
        "{kinds_in:?}"
    );
}

#[test]
fn psd_export_without_editable_options_writes_pixel_layers_only() {
    let psd = read(&export(&doc(), PsdEditable::default())).expect("read");
    assert_eq!(psd.layers.len(), 4);
    for l in &psd.layers[1..3] {
        assert_eq!(l.kind(), LayerKind::Pixel, "{}", l.name);
        assert!(l.effects.is_none() && l.text.is_none() && l.vector_mask.is_none());
    }
}
