//! AU-3: JSF-Textrichtung wie gemessen (`re\09-text.md` T5: `ORIi` 1 = „vertical right to left“,
//! 3 = „vertical left to right“) und Texturnamen in `setFillNColorNTexture`/
//! `setBrushNColorNTexture` (Maske aus der Datei, auch als Vorgabe für neue Formen).

use fp_engine::Session;
use fp_model::element::ElementKind;
use fp_script::{ScriptOptions, run_source};
use serde_json::json;
use std::path::PathBuf;

const NEW: &str = "var dom = fw.createFirepixelDocument({x: 200, y: 200}, {pixelsPerUnit: 72, units: 'inch'}, '#ffffff'); dom = fw.getDocumentDOM();\n";

fn run(s: &mut Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

fn selected(s: &Session) -> std::sync::Arc<fp_model::element::Element> {
    let d = s.active().expect("Dokument");
    let id = *d.selection.elements.last().expect("Auswahl");
    fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element")
}

fn orientation_of_selection(s: &Session) -> u8 {
    match &selected(s).kind {
        ElementKind::Text(t) => t.orientation as u8,
        k => panic!("kein Text: {k:?}"),
    }
}

const TEXT: &str = "dom.addNewText({left: 10, top: 10, right: 120, bottom: 40}, true); dom.setTextRuns({initialAttrs: {}, textRuns: [{changedAttrs: {}, characters: 'ABC'}]});\n";

#[test]
fn set_text_orientation_uses_measured_codes() {
    for (name, code) in [
        ("vertical right to left", 1u8),
        ("horizontal right to left", 2),
        ("vertical left to right", 3),
        ("horizontal left to right", 0),
    ] {
        let mut s = Session::default();
        let got = run(
            &mut s,
            &format!("{NEW}{TEXT} dom.setTextOrientation('{name}'); fw.selection[0].orientation"),
        );
        assert_eq!(orientation_of_selection(&s), code, "{name}");
        assert_eq!(got, name, "Getter liefert denselben Namen");
    }
}

#[test]
fn element_orientation_setter_and_text_flow_use_measured_codes() {
    for (name, code) in [
        ("vertical right to left", 1u8),
        ("vertical left to right", 3),
    ] {
        let mut s = Session::default();
        run(
            &mut s,
            &format!("{NEW}{TEXT} fw.selection[0].orientation = '{name}';"),
        );
        assert_eq!(orientation_of_selection(&s), code, "Setter {name}");
        // setTextFlow: „right to left“ enthält auch „left“ – darf nicht als links→rechts gelten
        let mut s = Session::default();
        run(&mut s, &format!("{NEW}{TEXT} dom.setTextFlow('{name}');"));
        assert_eq!(orientation_of_selection(&s), code, "setTextFlow {name}");
    }
}

#[test]
fn getter_follows_engine_orientation() {
    // Engine-Name `verticalRtl` (Text-Inspektor „Vertikal von rechts nach links“) → JSF wie im Original
    let mut s = Session::default();
    run(&mut s, &format!("{NEW}{TEXT}"));
    s.execute("text.setProperties", json!({"orientation": "verticalRtl"}))
        .expect("Richtung");
    assert_eq!(orientation_of_selection(&s), 1);
    assert_eq!(
        run(&mut s, "fw.selection[0].orientation"),
        "vertical right to left"
    );
    s.execute("text.setProperties", json!({"orientation": "verticalLtr"}))
        .expect("Richtung");
    assert_eq!(
        run(&mut s, "fw.selection[0].orientation"),
        "vertical left to right"
    );
}

/// Kleine Bilddatei als Textur (Schachbrett), außerhalb der Texturordner.
fn texture_png(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fp_au3_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("Ordner");
    let p = dir.join("Au3 Textur.png");
    let mut s = Session::default();
    s.execute(
        "file.new",
        json!({"width": 8, "height": 8, "background": "#ffffff"}),
    )
    .expect("Dokument");
    s.execute(
        "draw.rectangle",
        json!({"rect": [0, 0, 4, 4], "fillColor": "#000000", "strokeWidth": 0}),
    )
    .expect("Rechteck");
    s.execute("file.saveCopyAs", json!({"path": p.to_string_lossy()}))
        .expect("speichern");
    p
}

fn js_path(p: &std::path::Path) -> String {
    p.to_string_lossy().replace('\\', "\\\\")
}

#[test]
fn fill_and_brush_texture_names_are_applied_with_mask() {
    let file = texture_png("sel");
    let mut s = Session::default();
    let src = format!(
        "{NEW} dom.addNewRectangle({{left: 10, top: 10, right: 90, bottom: 60}}, 0);
         dom.setFillNColorNTexture({{category: 'fc_Solid'}}, '#ff0000', '{p}');
         dom.setBrushNColorNTexture({{diameter: 3, category: 'bc_Pencil', name: 'bc_1-Pixel Hard'}}, '#0000ff', '{p}');
         fw.selection[0].pathAttributes.fillTexture.name + '|' + fw.selection[0].pathAttributes.brushTexture.name",
        p = js_path(&file)
    );
    let got = run(&mut s, &src);
    let a = fp_engine::docops::path_attrs_of(&selected(&s)).expect("Pfadattribute");
    let ft = a.fill_texture.as_ref().expect("Fülltextur");
    let bt = a.stroke_texture.as_ref().expect("Strichtextur");
    assert!(
        ft.mask.is_some() && bt.mask.is_some(),
        "Textur aus der Datei hat eine Maske"
    );
    assert_eq!(ft.name, "Au3 Textur", "Name = Dateiname ohne Endung");
    assert_eq!(got, "Au3 Textur|Au3 Textur");
    // leerer Name entfernt die Textur
    run(
        &mut s,
        "fw.getDocumentDOM().setFillNColorNTexture({category: 'fc_Solid'}, '#00ff00', '');",
    );
    let a = fp_engine::docops::path_attrs_of(&selected(&s)).expect("Pfadattribute");
    assert!(a.fill_texture.is_none(), "Textur entfernt");
    assert!(a.stroke_texture.is_some(), "Strichtextur bleibt");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn texture_without_selection_is_default_for_new_shapes() {
    let file = texture_png("def");
    let mut s = Session::default();
    let src = format!(
        "{NEW} dom.selectNone(); dom.setFillNColorNTexture({{category: 'fc_Solid'}}, '#ff0000', '{p}');
         dom.addNewRectangle({{left: 10, top: 10, right: 90, bottom: 60}}, 0);",
        p = js_path(&file)
    );
    run(&mut s, &src);
    let a = fp_engine::docops::path_attrs_of(&selected(&s)).expect("Pfadattribute");
    let ft = a
        .fill_texture
        .as_ref()
        .expect("Fülltextur am neuen Rechteck");
    assert!(ft.mask.is_some());
    assert_eq!(ft.name, "Au3 Textur");
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}

#[test]
fn named_texture_from_texture_folders_gets_mask() {
    let dirs = fp_engine::resources::dirs(fp_engine::resources::TEXTURES);
    let Some((name, _)) = fp_engine::resources::files_in(&dirs).into_iter().next() else {
        return;
    };
    let mut s = Session::default();
    run(
        &mut s,
        &format!(
            "{NEW} dom.addNewRectangle({{left: 10, top: 10, right: 90, bottom: 60}}, 0); dom.setFillNColorNTexture({{category: 'fc_Solid'}}, '#ff0000', '{name}');"
        ),
    );
    let a = fp_engine::docops::path_attrs_of(&selected(&s)).expect("Pfadattribute");
    let ft = a.fill_texture.as_ref().expect("Fülltextur");
    assert_eq!(ft.name, name);
    assert!(ft.mask.is_some(), "Maske aus dem Texturordner");
}
