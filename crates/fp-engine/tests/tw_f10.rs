//! F10 (docs\parity\TWEAKS_2026-10-14.md, Nachlauf 14.10.): TW-33 Stapelplatz der Ergebnisse von
//! Pfadoperationen und „Auswahl auf Hintergrund reduzieren“ wie im Original
//! (`menu-modify-b/07,08,11,12`, `menu-modify-a/06`).

use crate::common;

use common::*;
use fp_engine::Session;
use fp_model::ElementId;
use serde_json::json;

fn set_name(s: &mut Session, id: ElementId, n: &str) {
    ex(s, "element.setName", json!({"ids": [id.0], "name": n}));
}

/// Stapel von oben nach unten als (Name, Grenzen links/oben).
fn stack(s: &Session) -> Vec<(Option<String>, (i32, i32))> {
    top_elements(s).iter().map(|e| (e.common.name.clone(), { let b = bounds(s, e.id); (b.left.round() as i32, b.top.round() as i32) })).collect()
}

/// T04 im Kleinen: von oben L1, P1, E1, R2, R1 (R1 zuunterst).
fn t04() -> (Session, [ElementId; 5]) {
    let mut s = doc_session(800.0, 600.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r2 = rect(&mut s, 200.0, 50.0, 300.0, 120.0);
    let e1 = ellipse(&mut s, 50.0, 170.0, 150.0, 240.0);
    let p1 = rect(&mut s, 200.0, 170.0, 420.0, 240.0);
    let l1 = rect(&mut s, 50.0, 300.0, 300.0, 360.0);
    for (id, n) in [(r1, "R1"), (r2, "R2"), (e1, "E1"), (p1, "P1"), (l1, "L1")] {
        set_name(&mut s, id, n);
    }
    (s, [l1, p1, e1, r2, r1])
}

fn names(s: &Session) -> Vec<Option<String>> {
    stack(s).into_iter().map(|(n, _)| n).collect()
}

fn n(v: &[Option<&str>]) -> Vec<Option<String>> {
    v.iter().map(|x| x.map(str::to_string)).collect()
}

/// Vereinigen und Überschneiden: Ergebnis am Platz des oberen Objekts (E1), nicht von R1.
#[test]
fn union_and_intersect_take_the_upper_place() {
    for op in ["union", "intersect"] {
        let (mut s, [_, _, e1, _, r1]) = t04();
        ex(&mut s, "edit.move", json!({"dx": 0, "dy": -100, "ids": [e1.0]}));
        select(&mut s, &[r1, e1]);
        ex(&mut s, &format!("modify.combinePaths.{op}"), json!({}));
        assert_eq!(names(&s), n(&[Some("L1"), Some("P1"), None, Some("R2")]), "{op}");
    }
}

/// Ausstanzen und Zuschneiden bleiben am Platz des unteren Objekts (Original gleich).
#[test]
fn punch_and_crop_stay_at_the_lower_place() {
    for op in ["punch", "crop"] {
        let (mut s, [_, _, e1, _, r1]) = t04();
        ex(&mut s, "edit.move", json!({"dx": 0, "dy": -100, "ids": [e1.0]}));
        select(&mut s, &[r1, e1]);
        ex(&mut s, &format!("modify.combinePaths.{op}"), json!({}));
        assert_eq!(names(&s), n(&[Some("L1"), Some("P1"), Some("R2"), None]), "{op}");
    }
}

/// Verbinden (P1 + L1): am Platz von L1 (oben); Teilen danach: die Teile ganz oben in der Ebene,
/// der erste Teil zuoberst (menu-modify-b/11: E1-Teil 50,170 über R1-Teil 50,50).
#[test]
fn join_takes_upper_place_split_goes_to_top() {
    let (mut s, [l1, p1, ..]) = t04();
    select(&mut s, &[p1, l1]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    assert_eq!(names(&s), n(&[None, Some("E1"), Some("R2"), Some("R1")]));

    let (mut s, [_, _, e1, _, r1]) = t04();
    select(&mut s, &[r1, e1]);
    ex(&mut s, "modify.combinePaths.join", json!({}));
    assert_eq!(names(&s), n(&[Some("L1"), Some("P1"), None, Some("R2")]), "Verbinden am Platz von E1");
    ex(&mut s, "modify.combinePaths.split", json!({}));
    let st = stack(&s);
    assert_eq!(st.len(), 5);
    assert_eq!(st[0], (None, (50, 170)), "{st:?}");
    assert_eq!(st[1], (None, (50, 50)), "{st:?}");
    assert_eq!(st[2].0.as_deref(), Some("L1"), "{st:?}");
}

/// Auswahl auf Hintergrund reduzieren (R1 + E1): Bitmap ohne Namen am Platz von R1 (zuunterst).
#[test]
fn flatten_selection_lands_at_bottom_place_without_name() {
    let (mut s, [_, _, e1, _, r1]) = t04();
    select(&mut s, &[r1, e1]);
    let v = ex(&mut s, "modify.flattenSelection", json!({}));
    let id = ElementId(v["id"].as_u64().expect("id"));
    assert_eq!(names(&s), n(&[Some("L1"), Some("P1"), Some("R2"), None]));
    assert_eq!(top_elements(&s).last().map(|e| e.id), Some(id));
    // ein einzelnes Objekt behält Namen und Platz (TW-27, menu-modify-a/08)
    let (mut s, [_, p1, ..]) = t04();
    select(&mut s, &[p1]);
    ex(&mut s, "modify.flattenSelection", json!({}));
    assert_eq!(names(&s), n(&[Some("L1"), Some("P1"), Some("E1"), Some("R2"), Some("R1")]));
}
