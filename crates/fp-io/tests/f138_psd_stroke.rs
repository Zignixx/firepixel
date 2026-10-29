//! F138 (BA-101 PSD part): the PSD-style Stroke live effect (a `stroke` record in the PSD live
//! effect container) is written by the PSD export as the stroke layer effect `FrFX` in `lfx2`
//! (with "effects editable") and read back by the PSD import as the same live effect.

use std::sync::Arc;

use fp_color::Argb;
use fp_export::doc_export::{DocExportOptions, FilesStyle, PanelOptions, export_document};
use fp_export::export::EncodeExtras;
use fp_export::psd_objects::PsdEditable;
use fp_export::render::SimpleRenderer;
use fp_io::psd::reader::read;
use fp_model::effects::EffectKind;
use fp_model::{Document, Effect, EffectList, Element, ElementKind, LayerSlot, PathAttrs, Rect};

const STROKE: &str =
    "1&60.000000&4.000000&multiply&RGBColor;0.000000;128.000000;255.000000&insetFrame&solidColor&";

fn doc(record: &str) -> Document {
    let mut d = Document::new(120.0, 80.0, Argb(0xFFFF_FFFF));
    let mut ids = std::mem::take(&mut d.ids);
    let mut rect = Element::new_rectangle(
        &mut ids,
        Rect::new(10.0, 10.0, 60.0, 40.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xFFFF_0000)),
    );
    rect.common.name = Some("Box".into());
    let mut e = Effect::new(EffectKind::PhotoshopLive.moa_id().expect("moa"), true);
    e.set("stroke", record);
    rect.common.effects = Some(EffectList {
        effects: vec![e],
        ..EffectList::default()
    });
    d.ids = ids;
    if let Some(LayerSlot::Own(l)) = d.page_mut(0).map(|p| &mut p.root.layers[0]) {
        Arc::make_mut(&mut Arc::make_mut(l).cels[0]).elements = vec![Arc::new(rect)];
    }
    d
}

fn export(d: &Document, effects: bool) -> Vec<u8> {
    let mut o = DocExportOptions::new("t");
    o.style = FilesStyle::Psd;
    o.panel = Some(PanelOptions {
        psd_editable: PsdEditable {
            effects,
            vector: true,
            text: false,
        },
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
fn stroke_is_written_as_frame_fx_and_imported_back() {
    let data = export(&doc(STROKE), true);
    let psd = read(&data).expect("read");
    let shape = psd
        .layers
        .iter()
        .find(|l| l.name == "Box")
        .expect("layer Box");
    let fx = shape.effects.as_ref().expect("lfx2");
    let fr = fx.obj("frameFX").expect("stroke layer effect");
    assert_eq!(fr.bool("enabled"), Some(true));
    assert_eq!(fr.str("style"), Some("insetFrame"));
    assert_eq!(fr.str("paintType"), Some("solidColor"));
    assert_eq!(fr.str("mode"), Some("multiply"));
    assert_eq!(fr.num("opacity"), Some(60.0));
    assert_eq!(fr.num("size"), Some(4.0));
    let c = fr.obj("color").expect("colour");
    assert_eq!(
        (c.num("red"), c.num("green"), c.num("blue")),
        (Some(0.0), Some(128.0), Some(255.0))
    );
    // the stroke left the pixels: the object is a plain fill layer with a vector mask
    assert!(shape.fill.is_some() && shape.vector_mask.is_some());

    // import: the same PSD live effect with the same stroke record
    let imp = fp_io::psd::import(&data, &fp_io::ImportOptions::default()).expect("import");
    let mut strokes = Vec::new();
    for p in &imp.document.pages {
        for slot in &p.root.layers {
            if let LayerSlot::Own(l) = slot {
                for c in &l.cels {
                    for e in &c.elements {
                        if let (ElementKind::Path(_), Some(list)) = (&e.kind, &e.common.effects) {
                            for x in &list.effects {
                                if x.kind() == EffectKind::PhotoshopLive {
                                    strokes.extend(x.get("stroke").map(str::to_string));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(strokes, vec![STROKE.to_string()]);
}

#[test]
fn stroke_without_editable_effects_stays_in_the_pixels() {
    let psd = read(&export(&doc(STROKE), false)).expect("read");
    let shape = psd
        .layers
        .iter()
        .find(|l| l.name == "Box")
        .expect("layer Box");
    assert!(shape.effects.is_none());
}
