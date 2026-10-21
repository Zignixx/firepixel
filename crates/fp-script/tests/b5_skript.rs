//! B-5: Befunde Skript-API (BA-066, BA-067, BA-068, BA-069, BA-073, BA-074/BA-030, BA-075, BA-031,
//! f13-Nebenbefund `setFillColor`/`setBrushColor` ohne Auswahl).

use fp_script::{ScriptOptions, run_source};
use serde_json::json;

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 400, "height": 300, "background": "#ffffff"}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(
        o.ok,
        "Fehler: {:?}\nDiagnose: {:?}\nSkript: {src}",
        o.error, o.diagnostics
    );
    o.result.unwrap_or_default()
}

const D: &str = "var d=fw.getDocumentDOM();";
const NAMES: &str = "var d=fw.getDocumentDOM(),r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].name+'^'+d.getParentLayerNum(i));r.join('|')";

// ------------------------------------------------------------------ BA-073

#[test]
fn null_and_undefined_are_gaps_in_js_arrays() {
    let mut s = session();
    // Original M2 q18: `[…, a.brush ? … : null].toSource()` → `[…, "fc_Solid/fn_Normal", ]`
    assert_eq!(result(&mut s, "['a', null].toSource()"), "[\"a\", ]");
    assert_eq!(result(&mut s, "[null, undefined, 1].toSource()"), "[, , 1]");
    // unbenanntes Objekt (Original dialogs-d31-d40/02 `[, "[object Path]", …]`)
    assert_eq!(
        result(
            &mut s,
            &format!(
                "{D}d.addNewRectangle({{left:1,top:1,right:20,bottom:20}},0);var e=fw.selection[0];[e.name, 7].toSource()"
            )
        ),
        "[, 7]"
    );
    // d.zoom / docTitle gibt es nicht: undefined, im Array eine Lücke
    assert_eq!(
        result(
            &mut s,
            &format!("{D}[d.zoom, d.docTitle, typeof d.zoom].toSource()")
        ),
        "[, , \"undefined\"]"
    );
    // Wirtobjekte bleiben unverändert (null sichtbar)
    assert_eq!(
        result(
            &mut s,
            // tool stroke "none" (Q-58: `dom.pathAttributes` are the tool attributes)
            &format!(
                "{D}d.selectNone();d.setBrush(null);d.pathAttributes.toSource().indexOf('brush:null') >= 0"
            )
        ),
        "true"
    );
}

// ------------------------------------------------------------------ BA-066

#[test]
fn add_new_hotspot_slice_creates_a_slice() {
    let mut s = session();
    let r = result(
        &mut s,
        &format!(
            "{D}d.addNewHotspot('slice','rectangle',{{left:10,top:10,right:50,bottom:40}});String(fw.selection[0])"
        ),
    );
    assert_eq!(r, "[object SliceHotspot]");
    let r = result(
        &mut s,
        &format!(
            "{D}d.addNewHotspot('hotspot','oval',{{left:60,top:10,right:90,bottom:40}});[String(fw.selection[0]), fw.selection[0].shape].join(',')"
        ),
    );
    assert!(r.starts_with("[object Hotspot],"), "{r}");
    assert!(!r.ends_with(",rectangle"), "ovaler Hotspot: {r}");
}

#[test]
fn add_new_text_defaults_like_original() {
    let mut s = session();
    let q = "var t=fw.selection[0];[t.antiAliased, t.antiAliasMode, t.autoKern, t.textRuns.initialAttrs.leading, t.textRuns.initialAttrs.leadingMode].toSource()";
    // Original UX-A O6: glatt, Unterschneidung aus, 120 %; T-11 (re 43 §4): neuer Text ist neu
    // gesetzt und meldet den Zeilenabstand als 32-Bit-Wert
    let want = "[true, \"smooth\", false, 1.1999999682108562, \"percentage\"]";
    assert_eq!(
        result(
            &mut s,
            &format!("{D}d.addNewText({{left:10,top:10,right:150,bottom:50}}, true);{q}")
        ),
        want
    );
    assert_eq!(
        result(
            &mut s,
            &format!("{D}d.addNewText({{left:10,top:60,right:150,bottom:100}}, false);{q}")
        ),
        want
    );
    // Setter: aus = harte Kante (Modus bleibt), Modus setzen schaltet die Glättung wieder ein
    let r = result(
        &mut s,
        "var t=fw.selection[0]; t.antiAliased=false; var a=[t.antiAliased,t.antiAliasMode]; t.antiAliasMode='crisp'; [a, t.antiAliased, t.antiAliasMode].toSource()",
    );
    assert_eq!(r, "[[false, \"smooth\"], true, \"crisp\"]");
    let r = result(
        &mut s,
        &format!(
            "{D}d.setTextAntiAliasing('none'); var t=fw.selection[0]; [t.antiAliased, t.antiAliasMode].toSource()"
        ),
    );
    assert_eq!(r, "[false, \"crisp\"]");
}

// ------------------------------------------------------------------ BA-067

#[test]
fn text_pixel_rect_is_ink_bounds_like_original() {
    // FX-1: Original meldet für fx_text.fw.png 43,60–148,98
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("re")
        .join("_testdateien")
        .join("par")
        .join("fx")
        .join("base")
        .join("fx_text.fw.png");
    if !p.exists() {
        eprintln!("fx_text.fw.png fehlt – übersprungen");
        return;
    }
    let url = format!(
        "file:///{}",
        p.canonicalize()
            .expect("Pfad")
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .replace('\\', "/")
            .replacen(':', "|", 1)
    );
    let mut s = session();
    let r = result(
        &mut s,
        &format!(
            "var d2=fw.openDocument('{url}'); var el=d2.layers[0].frames[0].elements; var p=el[0].pixelRect; [String(el[0]), p.left, p.top, p.right, p.bottom].join(',')"
        ),
    );
    assert_eq!(r, "[object Text],43,60,148,98");
}

// ------------------------------------------------------------------ BA-068

#[test]
fn set_active_view_scale_sets_zoom_and_center() {
    let mut s = session();
    result(&mut s, "fw.activeViewScale = 4;");
    assert_eq!(
        result(
            &mut s,
            "fw.setActiveViewScale(1, {x: 300, y: 200}); fw.activeViewScale"
        ),
        "1"
    );
    let v = &s.active().expect("Dokument").view;
    let c = (
        v.origin.0 + v.viewport.0 / 2.0 / v.zoom,
        v.origin.1 + v.viewport.1 / 2.0 / v.zoom,
    );
    assert!(
        (c.0 - 300.0).abs() < 1e-6 && (c.1 - 200.0).abs() < 1e-6,
        "Mitte {c:?}"
    );
    assert_eq!(
        result(
            &mut s,
            "fw.setActiveViewScale(2.5, {x: 10, y: 20}); fw.activeViewScale"
        ),
        "2.5"
    );
}

// ------------------------------------------------------------------ BA-069

#[test]
fn builtin_add_shadow_uses_menu_default_a2() {
    let (_, src) = fp_script::builtin_commands::BUILTIN
        .iter()
        .find(|(n, _)| n.ends_with("Schatten hinzufügen.jsf"))
        .expect("Befehl");
    assert!(src.contains("ShadowColor: \"#000000a2\""), "{src}");
}

// ------------------------------------------------------------------ BA-031, BA-074 (= BA-030), BA-075

#[test]
fn add_new_layer_returns_name() {
    let mut s = session();
    assert_eq!(
        result(&mut s, &format!("{D}d.addNewLayer('TestL', false)")),
        "TestL"
    );
}

#[test]
fn sublayer_can_be_active_and_nests() {
    let mut s = session();
    // addNewSubLayer: Rückgabe Name, neue Unterebene aktiv (Original: cur 1 → 2)
    assert_eq!(
        result(&mut s, &format!("{D}d.addNewSubLayer(0,'S1',false)")),
        "S1"
    );
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "1");
    // verschachtelt unter der Unterebene 1
    assert_eq!(
        result(&mut s, &format!("{D}d.addNewSubLayer(1,'S2',false)")),
        "S2"
    );
    let n = result(&mut s, NAMES);
    let parts: Vec<&str> = n.split('|').collect();
    assert_eq!(&parts[1..3], &["S1^0", "S2^1"], "{n}");
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "2");
    // neue Objekte landen in der aktiven Unterebene
    result(
        &mut s,
        &format!("{D}d.addNewRectangle({{left:5,top:5,right:30,bottom:30}},0);"),
    );
    assert_eq!(
        result(&mut s, &format!("{D}d.layers[2].frames[0].elements.length")),
        "1"
    );
    // currentLayerNum = Unterebene liest die Unterebene zurück
    assert_eq!(
        result(
            &mut s,
            &format!("{D}d.currentLayerNum=0;d.currentLayerNum=1;d.currentLayerNum")
        ),
        "1"
    );
    // Seitenwechsel-Prüfung behält die Unterebene
    let page = s.active().expect("Dokument").page;
    let layer = s.active().expect("Dokument").layer;
    assert_eq!(
        fp_engine::docops::layer_on_page(&s.active().expect("Dokument").doc, page, layer),
        layer
    );
}

#[test]
fn duplicate_and_reorder_sublayers() {
    let mut s = session();
    result(
        &mut s,
        &format!(
            "{D}d.addNewSubLayer(0,'S1',false);d.addNewSubLayer(0,'S2',false);d.currentLayerNum=0;"
        ),
    );
    let names = |s: &mut fp_engine::Session| {
        result(s, NAMES)
            .split('|')
            .map(|x| x.split('^').next().unwrap_or("").to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(&names(&mut s)[1..3], &["S1", "S2"]);
    // S2 (2) an die Stelle von S1 (1)
    result(&mut s, &format!("{D}d.reorderLayer(2,1,false,0,0)"));
    assert_eq!(&names(&mut s)[1..3], &["S2", "S1"]);
    // und zurück
    result(&mut s, &format!("{D}d.reorderLayer(1,2,false,0,0)"));
    assert_eq!(&names(&mut s)[1..3], &["S1", "S2"]);
    // duplizieren: Kopie direkt daneben, eigene Ebenen-ID
    result(&mut s, &format!("{D}d.duplicateLayer(1,2)"));
    let n = names(&mut s);
    assert_eq!(n.iter().filter(|x| *x == "S1").count(), 3, "{n:?}");
    let ids = result(
        &mut s,
        &format!(
            "{D}var r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].name=='S1'?1:0);r.join('')"
        ),
    );
    assert_eq!(ids.matches('1').count(), 3);
    // Rückgängig als ein Schritt je Skript
    assert!(s.execute("edit.undo", json!({})).is_ok());
    assert_eq!(names(&mut s).iter().filter(|x| *x == "S1").count(), 1);
}

// ------------------------------------------------------------------ f13: Farben ohne Auswahl

#[test]
fn set_fill_and_brush_color_without_selection_set_tool_colors() {
    let mut s = session();
    result(
        &mut s,
        &format!("{D}d.selectNone();d.setFillColor('#ff0000');d.setBrushColor('#0000ff');"),
    );
    assert_eq!(s.tools.fill_color, fp_model::Argb(0xffff_0000));
    assert_eq!(s.tools.stroke_color, fp_model::Argb(0xff00_00ff));
    assert_eq!(
        result(
            &mut s,
            &format!("{D}var a=d.pathAttributes;[a.fillColor,a.brushColor].join(',')")
        ),
        "#ff0000,#0000ff"
    );
    // with a selection the object and the tool fill change (Q-58, RE spec 58 section 2.1: the next
    // shape gets the colour)
    result(
        &mut s,
        &format!(
            "{D}d.addNewRectangle({{left:1,top:1,right:20,bottom:20}},0);d.setFillColor('#00ff00');"
        ),
    );
    assert_eq!(s.tools.fill_color, fp_model::Argb(0xff00_ff00));
}
