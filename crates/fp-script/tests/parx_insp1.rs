//! PAR-X-INSP-1: JSF-Sicht auf Inspektor-Änderungen wie im Original – `dom.exportOptions` folgt dem
//! Exportformat (PAR 16/08), Rundung in px mit `mode` „exact“ (17/05), weiche Kante meldet
//! „antialiased“ + `feather` (18/11), `pixelRect` mit Strich (17/08).

use fp_script::{run_source, ScriptOptions};
use serde_json::json;

fn run(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

fn session() -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    s
}

#[test]
fn export_options_follow_page_settings() {
    let mut s = session();
    // „PNG32“ und „GIF WebSnap 256“ wie aus der Exportformat-Liste des Inspektors
    let mut png = fp_model::ExportSettings::default();
    png.format = 2;
    png.color_mode = 2;
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(&png).expect("json")})).expect("PNG32");
    assert_eq!(run(&mut s, "var o = fw.getDocumentDOM().exportOptions; o.exportFormat + '/' + o.colorMode + '/' + o.name"), "PNG/32 bit/PNG32");
    let mut e = fp_model::ExportSettings::default();
    e.format = 0;
    e.color_mode = 0;
    e.palette_mode = 1;
    e.websnap_adaptive = true;
    e.entries_requested = 256;
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(&e).expect("json")})).expect("Einstellungen");
    let v = run(&mut s, "var o = fw.getDocumentDOM().exportOptions; [o.exportFormat, o.colorMode, o.numEntriesRequested, o.webSnapAdaptive, o.name].join('|')");
    assert_eq!(v, "GIF|indexed|256|true|GIF WebSnap 256");
}

#[test]
fn roundness_exact_mode_and_edge_and_pixel_rect() {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [100, 105, 220, 185], "fillColor": "#808080", "strokeWidth": 0})).expect("Rechteck");
    s.execute("element.setBounds", json!({"width": 150, "height": 100})).expect("Größe");
    s.execute("pi.roundness", json!({"value": 10, "pixels": true})).expect("10 px");
    assert_eq!(run(&mut s, "var e = fw.selection[0]; e.roundness + '|' + e.mode"), "10|exact");
    s.execute("pi.fill", json!({"edge": 2, "feather": 10})).expect("weich");
    assert_eq!(run(&mut s, "var f = fw.selection[0].pathAttributes.fill; f.edgeType + '|' + f.feather"), "antialiased|10");
    s.execute("pi.fill", json!({"edge": 1})).expect("Anti-Alias");
    assert_eq!(run(&mut s, "var f = fw.selection[0].pathAttributes.fill; f.edgeType + '|' + f.feather"), "antialiased|0");
    let id = s.execute("draw.ellipse", json!({"rect": [50, 170, 150, 240]})).expect("Ellipse").value["id"].as_u64().expect("id");
    let _ = id;
    s.execute("pi.stroke", json!({"preset": "Einfach/Präzise abger.", "width": 8, "placement": "outside", "color": "#ff0000"})).expect("Strich");
    assert_eq!(run(&mut s, "var r = fw.selection[0].pixelRect; [r.left, r.top, r.right, r.bottom].join(',')"), "42,162,158,248");
}
