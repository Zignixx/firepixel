//! PERF-1: `docops::find_many_on_page` / `elements_on_page` liefern je ID dasselbe wie
//! `find_on_page` / `element_on_page` (auch Gruppenkinder, fehlende und doppelte IDs, Reihenfolge).

use fp_engine::docops;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn session_with_objects() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 800, "height": 600, "background": "#ffffff"})).expect("doc");
    for i in 0..12 {
        let x = 10.0 + i as f64 * 40.0;
        s.execute("draw.rectangle", json!({"rect": [x, 10.0, x + 30.0, 40.0]})).expect("rect");
    }
    // Gruppe aus den letzten beiden, Kinder bleiben über ihre IDs auffindbar
    let ids: Vec<u64> = docops::selectable_ids(&s.active().expect("doc").doc, 0, 0).iter().take(2).map(|i| i.0).collect();
    s.execute("select.set", json!({"ids": ids})).expect("sel");
    s.execute("modify.group", json!({})).expect("group");
    s
}

fn all_ids(s: &Session) -> Vec<ElementId> {
    let d = s.active().expect("doc");
    let mut v = Vec::new();
    d.doc.walk_elements(|_, _, e| v.push(e.id));
    v
}

#[test]
fn find_many_matches_find_on_page() {
    let s = session_with_objects();
    let d = s.active().expect("doc");
    let mut ids = all_ids(&s);
    assert_eq!(ids.len(), 13, "10 Rechtecke, Gruppe, 2 Kinder");
    ids.reverse();
    ids.push(ElementId(999_999)); // fehlt
    ids.push(ids[0]); // doppelt
    let many = docops::find_many_on_page(&d.doc, d.page, &ids);
    let one: Vec<_> = ids.iter().map(|id| docops::find_on_page(&d.doc, d.page, *id)).collect();
    assert_eq!(many, one);
    let els: Vec<u64> = docops::elements_on_page(&d.doc, d.page, &ids).iter().map(|e| e.id.0).collect();
    let els1: Vec<u64> = ids.iter().filter_map(|id| docops::element_on_page(&d.doc, d.page, *id)).map(|e| e.id.0).collect();
    assert_eq!(els, els1);
    // wenige IDs (anderer Weg) und leere Liste
    assert_eq!(docops::find_many_on_page(&d.doc, d.page, &ids[..2]), one[..2].to_vec());
    assert!(docops::find_many_on_page(&d.doc, d.page, &[]).is_empty());
    // falsche Seite: nichts
    assert!(docops::find_many_on_page(&d.doc, 7, &ids).iter().all(Option::is_none));
}

#[test]
fn paths_sorted_and_bounds_unchanged() {
    let s = session_with_objects();
    let d = s.active().expect("doc");
    let ids = all_ids(&s);
    let mut expect: Vec<_> = ids.iter().filter_map(|id| docops::find_on_page(&d.doc, d.page, *id)).collect();
    expect.sort_by_key(docops::z_key);
    assert_eq!(docops::paths_sorted(&d.doc, d.page, &ids), expect);
    let b = docops::ids_bounds(&d.doc, d.page, &ids).expect("bounds");
    assert!((b.left - 10.0).abs() < 1e-3 && (b.right - 480.0).abs() < 1e-3 && (b.top - 10.0).abs() < 1e-3 && (b.bottom - 40.0).abs() < 1e-3, "{b:?}");
}
