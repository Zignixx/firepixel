//! F150 (spec 79 §9.2, §9.3): round trip Firepixel → PSD → Firepixel of the ten records of the PSD live
//! effect container. Each document carries a rectangle with one container holding one enabled record
//! with non-default values; after the PSD export (effects editable) and the PSD import (default options)
//! the object carries exactly one container whose record equals the original field by field (numbers
//! with tolerance 1e-6; the records below are already in the normalised form of spec 79 §9.2: contour
//! names empty, gradient names with `_`, no trailing comma).
//!
//! The current exporter writes only the stroke (`FrFX`, F138); the nine other effects follow with F149
//! and are ignored until then ("needs F149").

use std::sync::Arc;

use fp_color::Argb;
use fp_export::doc_export::{DocExportOptions, FilesStyle, PanelOptions, export_document};
use fp_export::export::EncodeExtras;
use fp_export::psd_objects::PsdEditable;
use fp_export::render::SimpleRenderer;
use fp_model::effects::EffectKind;
use fp_model::{Document, Effect, EffectList, Element, LayerSlot, PathAttrs, Rect};

const LIN: &str = "2$$0.000000;0.000000;1$255.000000;255.000000;1";
const GAUSS: &str = "9$$0.000000;0.000000;1$32.000000;7.000000;1$64.000000;38.000000;1\
$96.000000;101.000000;1$128.000000;166.000000;1$159.000000;209.000000;1$191.000000;235.000000;1\
$223.000000;248.000000;1$255.000000;255.000000;1";
const BUBBLES: &str = "Bubbles&b7334da0-122f-11d4-8bb5-e27e45023b5f";

/// (record name in the container, record with non-default values).
fn records() -> Vec<(&'static str, String)> {
    vec![
        (
            "dropShadow",
            format!(
                "1&60.000000&0&33.000000&10.000000&7.000000&4.000000&20.000000&1&0&multiply\
                 &RGBColor;10.000000;20.000000;30.000000&{GAUSS}"
            ),
        ),
        (
            "innerShadow",
            format!(
                "1&55.000000&0&-60.000000&5.000000&3.000000&6.000000&10.000000&0&0&screen\
                 &RGBColor;200.000000;0.000000;0.000000&{LIN}"
            ),
        ),
        (
            "outerGlow",
            format!(
                "1&1&linearDodge&80.000000:10.000000:9.000000:5.000000:0.000000:60.000000&preciseMatte\
                 &RGBColor;0.000000;255.000000;0.000000&nogradient&{LIN}&"
            ),
        ),
        (
            "innerGlow",
            format!(
                "1&0&overlay&65.000000:5.000000:7.000000:0.000000:10.000000:45.000000&softMatte\
                 &RGBColor;255.000000;255.000000;0.000000&nogradient&{LIN}&centerGlow"
            ),
        ),
        (
            "bevelEmboss",
            format!(
                "1&1&1&1&1&60.000000&50.000000&100.000000&40.000000&250.000000&8.000000&2.000000\
                 &colorDodge&RGBColor;255.000000;250.000000;240.000000&multiply\
                 &RGBColor;20.000000;0.000000;40.000000&preciseMatte&emboss&stampOut&{LIN}\
                 &1,40.000000,{GAUSS}&1,0,150.000000,-50.000000,3.000000,4.000000,Bubbles,\
                 b7334da0-122f-11d4-8bb5-e27e45023b5f"
            ),
        ),
        (
            "satin",
            format!(
                "1&1&0&40.000000&45.000000&9.000000&6.000000&screen\
                 &RGBColor;0.000000;0.000000;255.000000&{LIN}"
            ),
        ),
        (
            "solidFill",
            "1&70.000000&multiply&RGBColor;0.000000;128.000000;255.000000".to_string(),
        ),
        (
            "gradientFill",
            "1&1&0&85.000000&30.000000&120.000000&10.000000&-5.000000&normal&radial\
             &My_Grad,customStops,4096.000000$3$2,0$50$userStop$RGBColor;255.000000;0.000000;0.000000,\
             2048$40$userStop$RGBColor;0.000000;255.000000;0.000000,\
             4096$50$userStop$RGBColor;0.000000;0.000000;255.000000,100.000000$0$50,30.000000$4096$60"
                .to_string(),
        ),
        (
            "patternFill",
            format!("1&0&75.000000&150.000000&5.000000&7.000000&screen&{BUBBLES}"),
        ),
        (
            "stroke",
            "1&60.000000&4.000000&multiply&RGBColor;0.000000;128.000000;255.000000&insetFrame\
             &solidColor&"
                .to_string(),
        ),
    ]
}

fn record(name: &str) -> String {
    records()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, r)| r)
        .expect("record")
}

fn doc(recs: &[(&str, String)]) -> Document {
    let mut d = Document::new(120.0, 80.0, Argb(0xFFFF_FFFF));
    let mut ids = std::mem::take(&mut d.ids);
    let mut rect = Element::new_rectangle(
        &mut ids,
        Rect::new(20.0, 20.0, 60.0, 40.0),
        0.0,
        PathAttrs::solid_fill(Argb(0xFFFF_0000)),
    );
    rect.common.name = Some("Box".into());
    let mut e = Effect::new(EffectKind::PhotoshopLive.moa_id().expect("moa"), true);
    for (k, r) in recs {
        e.set(k, r.as_str());
    }
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

fn export(d: &Document) -> Vec<u8> {
    let mut o = DocExportOptions::new("t");
    o.style = FilesStyle::Psd;
    o.panel = Some(PanelOptions {
        psd_editable: PsdEditable {
            effects: true,
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

/// All PSD live effect containers of the imported document.
fn containers(data: &[u8]) -> Vec<Effect> {
    let imp = fp_io::psd::import(data, &fp_io::ImportOptions::default()).expect("import");
    let mut out = Vec::new();
    for p in &imp.document.pages {
        for slot in &p.root.layers {
            if let LayerSlot::Own(l) = slot {
                for c in &l.cels {
                    for e in &c.elements {
                        if let Some(list) = &e.common.effects {
                            out.extend(
                                list.effects
                                    .iter()
                                    .filter(|x| x.kind() == EffectKind::PhotoshopLive)
                                    .cloned(),
                            );
                        }
                    }
                }
            }
        }
    }
    out
}

/// Field-by-field comparison: tokens split at the record separators, numbers with tolerance 1e-6.
fn assert_same_record(name: &str, got: &str, want: &str) {
    let split = |s: &str| -> Vec<String> {
        s.split(['&', ',', ':', '$', ';'])
            .map(str::to_string)
            .collect()
    };
    let (g, w) = (split(got), split(want));
    assert_eq!(
        g.len(),
        w.len(),
        "{name}: field count\n got {got}\nwant {want}"
    );
    for (i, (a, b)) in g.iter().zip(&w).enumerate() {
        let same = match (a.parse::<f64>(), b.parse::<f64>()) {
            (Ok(x), Ok(y)) => (x - y).abs() <= 1e-6,
            _ => a == b,
        };
        assert!(
            same,
            "{name}: token {i} {a:?} != {b:?}\n got {got}\nwant {want}"
        );
    }
}

fn round_trip(names: &[&str]) {
    let recs: Vec<(&str, String)> = names.iter().map(|n| (*n, record(n))).collect();
    let data = export(&doc(&recs));
    let cs = containers(&data);
    assert_eq!(cs.len(), 1, "exactly one container");
    let c = &cs[0];
    assert_eq!(
        c.psd_sub_effects().len(),
        names.len(),
        "records {:?}",
        c.psd_sub_effects()
    );
    for (n, want) in &recs {
        let got = c.get(n).unwrap_or_else(|| panic!("record {n} missing"));
        assert_same_record(n, got, want);
    }
}

#[test]
fn round_trip_stroke() {
    round_trip(&["stroke"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_drop_shadow() {
    round_trip(&["dropShadow"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_inner_shadow() {
    round_trip(&["innerShadow"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_outer_glow() {
    round_trip(&["outerGlow"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_inner_glow() {
    round_trip(&["innerGlow"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_bevel_emboss() {
    round_trip(&["bevelEmboss"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_satin() {
    round_trip(&["satin"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_solid_fill() {
    round_trip(&["solidFill"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_gradient_fill() {
    round_trip(&["gradientFill"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_pattern_fill() {
    round_trip(&["patternFill"]);
}

#[test]
#[ignore = "needs F149"]
fn round_trip_stroke_gradient_and_pattern() {
    for (paint, sub) in [
        (
            "gradientFill",
            "1&0&45.000000&80.000000&5.000000&6.000000&reflected&My_Grad,customStops,\
             4096.000000$2$2,0$50$userStop$RGBColor;0.000000;0.000000;0.000000,\
             4096$50$foregroundColor,100.000000$0$50,0.000000$4096$50",
        ),
        (
            "pattern",
            "0&50.000000&1.000000&2.000000&Bubbles&b7334da0-122f-11d4-8bb5-e27e45023b5f",
        ),
    ] {
        let rec = format!(
            "1&80.000000&5.000000&normal&RGBColor;255.000000;0.000000;0.000000&centeredFrame&{paint}&{sub}"
        );
        let data = export(&doc(&[("stroke", rec.clone())]));
        let cs = containers(&data);
        assert_eq!(cs.len(), 1);
        assert_same_record(paint, cs[0].get("stroke").expect("stroke"), &rec);
    }
}

#[test]
#[ignore = "needs F149"]
fn round_trip_all_ten_at_once() {
    let names: Vec<&str> = records().iter().map(|(n, _)| *n).collect();
    round_trip(&names);
}

/// Spec 79 §9.3 (import part): the first layer of the reference PSD named by `FP_PSD_REF` carries the ten
/// effects with the defaults of the PSD application, all switched off; each imports as the record built
/// from the spec 79 §4 defaults with `enabled` = 0. The file is not in the repository.
#[test]
#[ignore = "needs FP_PSD_REF (reference PSD, run by the lead)"]
fn reference_psd_first_layer_has_the_spec_defaults() {
    use fp_io::psd::descriptor::{Descriptor, Value};
    use fp_io::psd::effects::{GlobalLight, LAYER_EFFECTS, normalize, ps_record};
    let Ok(path) = std::env::var("FP_PSD_REF") else {
        eprintln!("FP_PSD_REF not set: skipped");
        return;
    };
    let data = std::fs::read(&path).expect("read FP_PSD_REF");
    let psd = fp_io::psd::reader::read(&data).expect("parse PSD");
    let fx = psd
        .layers
        .iter()
        .find_map(|l| l.effects.as_ref())
        .expect("a layer with lfx2");
    let fx = normalize(fx, GlobalLight::default());
    let off = Descriptor {
        name: String::new(),
        class: "null".into(),
        items: vec![("enabled".into(), Value::Bool(false))],
    };
    for k in LAYER_EFFECTS {
        let d = fx.obj(k).unwrap_or_else(|| panic!("{k} missing"));
        let got = ps_record(k, d).expect("record");
        let want = ps_record(k, &off).expect("default record");
        if k == "bevelEmboss" {
            // The default file may carry the contour and texture keys with `useShape`/`useTexture` off;
            // the import keeps those blocks (spec 79 §4.5), so compare the first 20 fields only.
            let cut = |s: &str| s.split('&').take(20).collect::<Vec<_>>().join("&");
            assert_same_record(k, &cut(&got), &cut(&want));
            continue;
        }
        assert_same_record(k, &got, &want);
    }
}
