//! PAR-X-INSP-2: JSF `pathAttributes.brushPlacement` meldet die Strichposition wie das Original
//! (PAR 17/08b: innen/mittig waren vertauscht).

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

#[test]
fn brush_placement_matches_inspector() {
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .expect("Dokument");
    s.execute("draw.ellipse", json!({"rect": [50, 170, 150, 240]}))
        .expect("Ellipse");
    s.execute(
        "pi.stroke",
        json!({"preset": "Einfach/Präzise abger.", "width": 8, "color": "#ff0000"}),
    )
    .expect("Strich");
    let get = "fw.selection[0].pathAttributes.brushPlacement";
    for (pl, rect) in [
        ("inside", "50,170,150,240"),
        ("center", "46,166,154,244"),
        ("outside", "42,162,158,248"),
    ] {
        // über den Inspektor gesetzt
        s.execute("pi.stroke", json!({"placement": pl}))
            .expect("Position");
        assert_eq!(run(&mut s, get), pl, "Inspektor {pl}");
        assert_eq!(
            run(
                &mut s,
                "var r = fw.selection[0].pixelRect; [r.left, r.top, r.right, r.bottom].join(',')"
            ),
            rect,
            "pixelRect {pl}"
        );
    }
    // per JSF gesetzt, wiederholt (idempotent) und in wechselnder Folge
    for pl in [
        "center", "inside", "inside", "outside", "center", "center", "inside",
    ] {
        run(
            &mut s,
            &format!("fw.getDocumentDOM().setBrushPlacement('{pl}'); 0"),
        );
        assert_eq!(run(&mut s, get), pl, "setBrushPlacement {pl}");
    }
    // Rückgängig stellt die vorige Position wieder her
    s.execute("edit.undo", json!({})).expect("Rückgängig");
    assert_eq!(run(&mut s, get), "center");
}
