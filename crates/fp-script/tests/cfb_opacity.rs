//! `opacity` reads back like the original's JSF: the stored byte b = round(p·2.55) as b·100/255, not
//! rounded (re 40 §3, M8: field 30 → 30.19607843137255, setOpacity(77) → 76.86274509803921,
//! setOpacity(50.4) and (50.6) → both 129/255). F25 reverts CF-B `8c886bb` (whole percent), which was
//! based on the assumption that the original rounds; only the inspector field shows round(b/2.55).

use fp_script::{ScriptOptions, run_source};

fn js(v: f64) -> String {
    format!("{v}")
}

#[test]
fn opacity_reads_back_as_byte_percentage() {
    let mut s = fp_engine::Session::default();
    let src = "var dom = fw.createFirepixelDocument({x: 200, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n\
               dom.addNewRectangle({left: 10, top: 10, right: 60, bottom: 60}, 0);\n\
               var out = [];\n\
               for (var v = 0; v <= 100; v++) { dom.setOpacity(v); out.push(fw.selection[0].opacity); }\n\
               out.join(',')";
    let o = run_source(&mut s, src, &ScriptOptions::default());
    assert!(o.ok, "{:?} {:?}", o.error, o.diagnostics);
    let got = o.result.unwrap_or_default();
    let want: Vec<String> = (0..=100)
        .map(|v| js((v as f64 * 255.0 / 100.0).round() * 100.0 / 255.0))
        .collect();
    assert_eq!(got, want.join(","));
    let one = |s: &mut fp_engine::Session, src: &str| {
        run_source(s, src, &ScriptOptions::default())
            .result
            .unwrap_or_default()
    };
    assert_eq!(
        one(
            &mut s,
            "fw.getDocumentDOM().setOpacity(30); fw.selection[0].opacity"
        ),
        "30.19607843137255"
    );
    assert_eq!(
        one(
            &mut s,
            "fw.getDocumentDOM().setOpacity(77); fw.selection[0].opacity"
        ),
        "76.86274509803921"
    );
    assert_eq!(
        one(
            &mut s,
            "var d = fw.getDocumentDOM(); d.setOpacity(50.4); var a = fw.selection[0].opacity; d.setOpacity(50.6); a + '|' + fw.selection[0].opacity"
        ),
        "50.588235294117645|50.588235294117645"
    );
    // element.setOpacity (inspector field) 50 → byte 128 → JSF 50.19607843137255
    let d = s.active().expect("doc");
    let id = d.selection.elements[0];
    s.execute(
        "element.setOpacity",
        serde_json::json!({"opacity": 50, "ids": [id.0]}),
    )
    .expect("opacity");
    assert_eq!(one(&mut s, "fw.selection[0].opacity"), "50.19607843137255");
}
