//! F18 (Tweak-Liste 15.10.): TW-54 – Ebenen-Sichtbarkeit. `layers[i].frames[0].visible` folgt dem
//! Auge im Bedienfeld: Einblenden einer in der Datei über den Cel (`VISb`) ausgeblendeten Ebene,
//! „Alles ausblenden/anzeigen“ einschließlich Unterebenen.

use fp_script::{ScriptOptions, run_source};
use serde_json::json;
use std::sync::Arc;

const VIS: &str =
    "fw.getDocumentDOM().layers.map(function(l){return l.name+':'+l.frames[0].visible}).join('|')";

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

/// Ebene 1, Ebene 2 (wie in der Testdatei über `VIFv` und den Cel ausgeblendet), Ebene 3 mit Unterebene.
fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute(
        "file.new",
        json!({"width": 300, "height": 200, "background": "#ffffff"}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    s.execute("layer.add", json!({"name": "Ebene 2"}))
        .unwrap_or_else(|e| panic!("{e}"));
    s.execute("layer.add", json!({"name": "Ebene 3"}))
        .unwrap_or_else(|e| panic!("{e}"));
    s.execute("pi.layer.addSub", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    let d = s.doc_mut().unwrap_or_else(|e| panic!("{e}"));
    let page = d.page;
    let doc = Arc::make_mut(&mut d.doc);
    let id = fp_engine::docops::own_layers(doc, page)
        .into_iter()
        .find(|(_, l)| l.name == "Ebene 2")
        .map(|(_, l)| l.id)
        .expect("Ebene 2");
    fp_engine::docops::edit_layer_by_id(doc, page, id, |l| {
        l.visibility = vec![false];
        for c in &mut l.cels {
            Arc::make_mut(c).visible = false;
        }
    });
    s
}

fn layer_id(s: &fp_engine::Session, name: &str) -> u64 {
    let d = s.active().expect("doc");
    fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .find(|(_, l)| l.name == name)
        .map(|(_, l)| l.id.0)
        .expect(name)
}

fn part(v: &str, name: &str) -> String {
    v.split('|')
        .find(|x| x.starts_with(&format!("{name}:")))
        .unwrap_or_default()
        .to_string()
}

#[test]
fn eye_on_cel_hidden_layer_shows_it_in_script_and_render() {
    let mut s = session();
    assert_eq!(part(&result(&mut s, VIS), "Ebene 2"), "Ebene 2:false");
    let id = layer_id(&s, "Ebene 2");
    s.execute("layer.setVisible", json!({"layer": id, "visible": true}))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        part(&result(&mut s, VIS), "Ebene 2"),
        "Ebene 2:true",
        "Auge und frames[0].visible gleich"
    );
    let d = s.active().expect("doc");
    let l = fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .find(|(_, l)| l.name == "Ebene 2")
        .expect("l")
        .1;
    assert!(
        l.cels.iter().all(|c| c.visible),
        "Cel-Flag mit eingeblendet (Leinwand zeichnet die Ebene)"
    );
}

#[test]
fn hide_all_and_show_all_include_sublayers() {
    let mut s = session();
    s.execute("panel.layers.hideAll", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    let v = result(&mut s, VIS);
    assert!(
        v.split('|').all(|x| x.ends_with(":false")),
        "alles aus, auch Unterebene: {v}"
    );
    s.execute("panel.layers.showAll", json!({}))
        .unwrap_or_else(|e| panic!("{e}"));
    let v = result(&mut s, VIS);
    assert!(
        v.split('|').all(|x| x.ends_with(":true")),
        "alles an, auch die in der Datei ausgeblendete Ebene 2: {v}"
    );
    assert!(v.split('|').count() >= 5, "Unterebene flach gelistet: {v}");
}

#[test]
fn sublayer_eye_changes_frame_visible() {
    let mut s = session();
    let sub = {
        let d = s.active().expect("doc");
        let mut id = None;
        for (_, l) in fp_engine::docops::own_layers(&d.doc, d.page) {
            for c in &l.cels {
                for e in &c.elements {
                    if let fp_model::ElementKind::SubLayer(sl) = &e.kind {
                        id = Some((sl.id.0, sl.name.clone()));
                    }
                }
            }
        }
        id.expect("Unterebene")
    };
    s.execute(
        "layer.setVisible",
        json!({"layer": sub.0, "visible": false}),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        part(&result(&mut s, VIS), &sub.1),
        format!("{}:false", sub.1)
    );
    s.execute("layer.setVisible", json!({"layer": sub.0, "visible": true}))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        part(&result(&mut s, VIS), &sub.1),
        format!("{}:true", sub.1)
    );
}
