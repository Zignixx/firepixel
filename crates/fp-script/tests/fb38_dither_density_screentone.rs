//! FB-38 (Firepixel extensions) in scripts: the uniform dither density of a fill
//! (`pathAttributes.fill.ditherDensity`, `setFill {ditherDensity}`) and the Screentone live filter
//! (`effectList` with EffectMoaID `{firepixel-screentone}` and its parameters).

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "error: {:?}\ndiagnostics: {:?}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

fn session_with_rect() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .expect("document");
    s.execute(
        "draw.rectangle",
        json!({"rect": [40, 40, 200, 150], "fillColor": "#336699", "strokeWidth": 0}),
    )
    .expect("rectangle");
    s
}

fn element(s: &fp_engine::Session) -> std::sync::Arc<fp_model::Element> {
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("element")
}

fn model_density(s: &fp_engine::Session) -> i32 {
    fp_engine::docops::path_attrs_of(&element(s))
        .and_then(|a| a.fill.clone())
        .expect("fill")
        .dither_density
}

const DENSITY: &str = "fw.selection[0].pathAttributes.fill.ditherDensity";

#[test]
fn dither_density_round_trip() {
    let mut s = session_with_rect();
    assert_eq!(run(&mut s, DENSITY), "0");
    s.execute("pi.fill", json!({"ditherDensity": 40}))
        .expect("density");
    assert_eq!(model_density(&s), 40);
    assert_eq!(run(&mut s, DENSITY), "40");
    // setFill with the object read back from pathAttributes keeps it, a new value replaces it
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; fw.getDocumentDOM().setFill(f); 0",
    );
    assert_eq!(model_density(&s), 40);
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.ditherDensity = 75; \
         fw.getDocumentDOM().setFill(f); 0",
    );
    assert_eq!(run(&mut s, DENSITY), "75");
    // together with the dither edge in one setFill
    run(
        &mut s,
        "var f = fw.selection[0].pathAttributes.fill; f.edgeType = 'dither'; f.feather = 4; \
         f.ditherDensity = 0; fw.getDocumentDOM().setFill(f); 0",
    );
    assert_eq!(
        run(
            &mut s,
            "var f = fw.selection[0].pathAttributes.fill; f.edgeType + '|' + f.feather + '|' + f.ditherDensity"
        ),
        "dither|4|0"
    );
}

#[test]
fn screentone_effect_round_trip() {
    let mut s = session_with_rect();
    run(
        &mut s,
        "fw.selection[0].effectList = {category: 'UNUSED', name: 'UNUSED', effects: [{ \
           EffectMoaID: '{firepixel-screentone}', EffectIsVisible: true, screentone_type: 2, \
           screentone_size: 12, screentone_angle: 30, screentone_density: 60, \
           screentone_from_luminance: false, screentone_color: '#ff0000', \
           screentone_keep_color: false }]}; 0",
    );
    let e = element(&s);
    let fx = &e.common.effects.as_ref().expect("effects").effects;
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), fp_model::EffectKind::Screentone);
    assert_eq!(fx[0].get_f64("screentone_size"), Some(12.0));
    assert_eq!(fx[0].get("screentone_from_luminance"), Some("false"));
    let back = run(
        &mut s,
        "var e = fw.selection[0].effectList.effects[0]; \
         [e.EffectMoaID, e.screentone_type, e.screentone_size, e.screentone_angle, \
          e.screentone_density, e.screentone_from_luminance, e.screentone_color].join('|')",
    );
    assert_eq!(back, "{firepixel-screentone}|2|12|30|60|false|#ff0000");
    // the parameters written back unchanged give the same effect
    run(
        &mut s,
        "var l = fw.selection[0].effectList; fw.selection[0].effectList = l; 0",
    );
    let e2 = element(&s);
    assert_eq!(
        e2.common.effects.as_ref().expect("effects").effects,
        e.common.effects.as_ref().expect("effects").effects
    );
}

#[test]
fn screentone_by_name_uses_the_defaults() {
    let mut s = session_with_rect();
    s.execute("element.addEffect", json!({"effect": "screentone"}))
        .expect("screentone");
    let e = element(&s);
    let fx = &e.common.effects.as_ref().expect("effects").effects[0];
    assert_eq!(fx.moa_id(), Some(fp_model::effects::SCREENTONE_MOA_ID));
    assert_eq!(fx.get("screentone_size"), Some("8"));
    assert_eq!(fx.get("screentone_angle"), Some("45"));
    assert_eq!(fx.get("screentone_from_luminance"), Some("true"));
}
