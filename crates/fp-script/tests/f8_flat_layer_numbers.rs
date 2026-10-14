//! F8 (Befund 1 aus `docs\berichte\f6.md`): Alle Ebenennummern im Skript-API zählen flach wie
//! `dom.layers` – jede Ebene, direkt danach ihre Unterebenen. Nummer `i` gehört in jeder Methode zu
//! `dom.layers[i]`. Dokumente ohne Unterebenen verhalten sich wie bisher.

use fp_script::{run_source, ScriptOptions};
use serde_json::json;

fn session() -> fp_engine::Session {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 150, "background": "#ffffff"})).unwrap_or_else(|e| panic!("{e}"));
    s
}

fn result(s: &mut fp_engine::Session, src: &str) -> String {
    let o = run_source(s, src, &ScriptOptions::default());
    assert!(o.ok, "Fehler: {:?}\nDiagnose: {:?}\nSkript: {src}", o.error, o.diagnostics);
    o.result.unwrap_or_default()
}

/// Dokument mit Unterebenen: [0 Ebene, 1 S1, 2 S2, 3 Web-Ebene], Rechteck „R“ auf Ebene 0.
fn with_sublayers() -> fp_engine::Session {
    let mut s = session();
    s.execute("draw.rectangle", json!({"rect": [0, 0, 50, 50], "name": "R"})).unwrap_or_else(|e| panic!("{e}"));
    // seit B-5 wird die neue Unterebene aktiv (B-2 BA-030): beide ausdrücklich in Ebene 0
    result(&mut s, "var d=fw.getDocumentDOM();d.addNewSubLayer(0,'S1',false);d.addNewSubLayer(0,'S2',false);d.currentLayerNum=0");
    s.execute("select.none", json!({})).unwrap_or_else(|e| panic!("{e}"));
    s
}

const NAMES: &str = "var d=fw.getDocumentDOM(),r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].name);r.join('|')";
const D: &str = "var d=fw.getDocumentDOM();";

fn names(s: &mut fp_engine::Session) -> Vec<String> {
    result(s, NAMES).split('|').map(str::to_string).collect()
}

#[test]
fn list_layout() {
    let mut s = with_sublayers();
    let n = names(&mut s);
    assert_eq!(n.len(), 4, "{n:?}");
    assert_eq!(&n[1..3], &["S1", "S2"], "{n:?}");
    assert_eq!(result(&mut s, &format!("{D}d.layers[3].layerType")), "web");
}

#[test]
fn current_layer_num_flat() {
    let mut s = with_sublayers();
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "0");
    // Web-Ebene: Nummer 3 (bisher 1), gelesen wie gesetzt
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum=3;[d.currentLayerNum,d.layers[d.currentLayerNum].layerType].join(',')")), "3,web");
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum=0;d.layers[d.currentLayerNum].name")), n0(&mut s));
    // Unterebene: sie selbst wird aktiv und so gelesen (B-2 BA-030, Original liest 2)
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum=3;d.currentLayerNum=2;d.currentLayerNum")), "2");
}

fn n0(s: &mut fp_engine::Session) -> String {
    names(s)[0].clone()
}

#[test]
fn set_layer_visible_and_locked() {
    let mut s = with_sublayers();
    let vis = "var r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].frames[0].visible?1:0);r.join('')";
    assert_eq!(result(&mut s, &format!("{D}d.setLayerVisible(1,-1,false,false);{vis}")), "1011");
    assert_eq!(result(&mut s, &format!("{D}d.setLayerVisible(2,-1,false,false);{vis}")), "1001");
    assert_eq!(result(&mut s, &format!("{D}d.setLayerVisible(0,-1,true,true);{vis}")), "1111", "allLayers: alle Ebenen samt Unterebenen");
    let lock = "var r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].frames[0].locked?1:0);r.join('')";
    assert_eq!(result(&mut s, &format!("{D}d.setLayerLocked(2,-1,true,false);{lock}")), "0010");
    assert_eq!(result(&mut s, &format!("{D}d.setLayerLocked(2,-1,false,false);d.setLayerLocked(3,-1,true,false);{lock}")), "0001");
}

#[test]
fn set_layer_name_and_sharing() {
    let mut s = with_sublayers();
    result(&mut s, &format!("{D}d.setLayerName(1,'Eins');d.setLayerName(2,'Zwei');d.setLayerName(0,'Null')"));
    let n = names(&mut s);
    assert_eq!(&n[0..3], &["Null", "Eins", "Zwei"], "{n:?}");
    assert_eq!(result(&mut s, &format!("{D}d.setLayerSharing(2,'shared',false,false);[d.layers[1].sharing,d.layers[2].sharing].join('|')")), "not shared|shared");
}

#[test]
fn parent_disclosure_master() {
    let mut s = with_sublayers();
    assert_eq!(result(&mut s, &format!("{D}[d.getParentLayerNum(0),d.getParentLayerNum(1),d.getParentLayerNum(2),d.getParentLayerNum(3)].join(',')")), "-1,0,0,-1");
    let disc = "var r=[];for(var i=0;i<d.layers.length;i++)r.push(d.layers[i].disclosure?1:0);r.join('')";
    let before = result(&mut s, &format!("{D}{disc}"));
    let after = result(&mut s, &format!("{D}d.setLayerDisclosure(2,{});{disc}", !before.as_bytes()[2].eq(&b'1')));
    assert_eq!(before[..2], after[..2], "nur Nummer 2 ändert sich: {before} → {after}");
    assert_ne!(before.as_bytes()[2], after.as_bytes()[2], "{before} → {after}");
    assert_eq!(before[3..], after[3..]);
    assert_eq!(result(&mut s, &format!("{D}[d.isMasterPageLayer(0),d.isMasterPageLayer(1),d.isMasterPageLayer(2),d.isMasterPageLayer(3)].join(',')")), "false,false,false,false");
}

#[test]
fn move_select_and_element_flags() {
    let mut s = with_sublayers();
    // Rechteck in Unterebene 1 verschieben
    let r = result(&mut s, &format!("{D}fw.selection=[d.layers[0].frames[0].elements[0]];d.moveSelectionToLayer(1,false,'none',-1);[d.layers[0].frames[0].elements.length,d.layers[1].frames[0].elements.length,d.layers[1].frames[0].elements[0].name].join(',')"));
    assert_eq!(r, "0,1,R");
    // alles auf Ebene 1 auswählen = das Rechteck
    assert_eq!(result(&mut s, &format!("{D}d.selectNone();d.selectAllOnLayer(1,false,false);[fw.selection.length,fw.selection[0].name].join(',')")), "1,R");
    assert_eq!(result(&mut s, &format!("{D}d.selectNone();d.selectAllOnLayer(2,false,false);fw.selection.length")), "0");
    // setElementVisible/Locked(frame, layerNum, objectIndex, …): objectIndex wie dom.layers[l].frames[f].elements
    assert_eq!(result(&mut s, &format!("{D}d.setElementVisible(0,1,0,false);d.layers[1].frames[0].elements[0].visible")), "false");
    assert_eq!(result(&mut s, &format!("{D}d.setElementLocked(0,1,0,true,false,false);d.layers[1].frames[0].elements[0].locked")), "true");
}

#[test]
fn delete_and_add_sublayer() {
    let mut s = with_sublayers();
    result(&mut s, &format!("{D}d.deleteLayer(1)"));
    let n = names(&mut s);
    assert_eq!(n.len(), 3, "{n:?}");
    assert_eq!(n[1], "S2", "{n:?}");
    // Unterebene in Ebene Nummer 0 anlegen: sie erscheint direkt dahinter (oben in der Ebene)
    result(&mut s, &format!("{D}d.addNewSubLayer(0,'S3',false)"));
    let n = names(&mut s);
    assert_eq!(n, vec![n[0].clone(), "S2".into(), "S3".into(), n[3].clone()]);
    assert_eq!(result(&mut s, &format!("{D}d.getParentLayerNum(2)")), "0");
}

#[test]
fn reorder_top_level_with_sublayers() {
    let mut s = with_sublayers();
    // Web-Ebene (Nummer 3) vor die Ebene Nummer 0: die Ebene samt Unterebenen rückt dahinter
    result(&mut s, &format!("{D}d.reorderLayer(3,0,false,0,0)"));
    let n = names(&mut s);
    assert_eq!(result(&mut s, &format!("{D}d.layers[0].layerType")), "web", "{n:?}");
    assert_eq!(&n[2..4], &["S1", "S2"], "{n:?}");
    // Unterebene (BA-075): S2 (3) an die Stelle von S1 (2) – Reihenfolge in der Ebene getauscht
    result(&mut s, &format!("{D}d.reorderLayer(3,2,false,0,0)"));
    assert_eq!(&names(&mut s)[2..4], &["S2", "S1"]);
}

/// Ohne Unterebenen: Nummern wie bisher (oberste Stufe).
#[test]
fn without_sublayers_unchanged() {
    let mut s = session();
    result(&mut s, &format!("{D}d.addNewLayer('B',false)"));
    let n = names(&mut s);
    assert_eq!(n.len(), 3, "{n:?}");
    assert_eq!(n[1], "B");
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum")), "1");
    assert_eq!(result(&mut s, &format!("{D}[d.getParentLayerNum(0),d.getParentLayerNum(1),d.getParentLayerNum(2)].join(',')")), "-1,-1,-1");
    assert_eq!(result(&mut s, &format!("{D}d.setLayerVisible(0,-1,false,false);[d.layers[0].frames[0].visible,d.layers[1].frames[0].visible].join(',')")), "false,true");
    result(&mut s, &format!("{D}d.reorderLayer(1,0,false,0,0)"));
    assert_eq!(names(&mut s)[0], "B");
    result(&mut s, &format!("{D}d.setLayerName(0,'C');d.deleteLayer(1)"));
    let n = names(&mut s);
    assert_eq!(n.len(), 2, "{n:?}");
    assert_eq!(n[0], "C");
    assert_eq!(result(&mut s, &format!("{D}d.currentLayerNum=1;d.layers[d.currentLayerNum].layerType")), "web");
}
