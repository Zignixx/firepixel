//! MN-2 (Messung PM-7, PARITAET_B §1): Menüfreigabe wie im Original.
//! - Pixelauswahl (S6): die Objektbefehle sind grau, Ausschneiden/Kopieren/Löschen bleiben aktiv.
//! - Gruppe (S7) und Segment (S9): Transformationen entfernen, Verbinden, An Pixeln ausrichten,
//!   Popupmenü bearbeiten/löschen, Maske, Als Vektoren kopieren, In Symbol konvertieren …
//! - Untermenü-Köpfe sind immer aktiv, auch wenn alles darin grau ist.
//! - Text ▸ Schrift: auch ohne Text aktiv (setzt die Vorgabe des Textwerkzeugs), ✓ an der aktuellen
//!   Schrift, zuletzt benutzte Schriften oben (`RecentFontList`), Familien flach.

use crate::full_common;

use fp_ui::app_state::{Action, AppState};
use fp_ui::menu::{self, ItemAction, Node};
use full_common::*;
use serde_json::{json, Value};

/// Die 12 Objektbefehle, die das Original bei einer Pixelauswahl sperrt.
const PIXEL_GREY: [&str; 12] = [
    "edit.duplicate",
    "edit.clone",
    "edit.copyAsVectors",
    "select.superselect",
    "modify.animation.animateSelection",
    "modify.symbol.convertToSymbol",
    "modify.flattenSelection",
    "modify.arrange.bringToFront",
    "modify.arrange.bringForward",
    "modify.arrange.sendBackward",
    "modify.arrange.sendToBack",
    "modify.group",
];

/// Freigabe eines Befehls im aufgebauten Hauptmenü (sucht den Eintrag über den Befehlsnamen).
fn menu_enabled(st: &AppState, cmd: &str) -> bool {
    fn find(nodes: &[Node], cmd: &str) -> Option<bool> {
        for n in nodes {
            match n {
                Node::Item { action: ItemAction::Command(c), enabled, .. } if c == cmd => return Some(*enabled),
                Node::Sub { children, .. } => {
                    if let Some(e) = find(children, cmd) {
                        return Some(e);
                    }
                }
                _ => {}
            }
        }
        None
    }
    menu::build_main_menu(st).iter().find_map(|(_, nodes)| find(nodes, cmd)).unwrap_or_else(|| panic!("{cmd} nicht im Hauptmenü"))
}

fn assert_state(st: &AppState, lage: &str, cmds: &[&str], want: bool) {
    for c in cmds {
        assert_eq!(available(st, c), want, "{lage}: {c} soll {}", if want { "aktiv sein" } else { "grau sein" });
        assert_eq!(menu_enabled(st, c), want, "{lage}: Menüeintrag {c}");
    }
}

fn pixel_selection(st: &mut AppState, ids: &[u64]) {
    select(st, ids);
    let _ = st.session.execute("select.pixels", json!({ "shape": "rect", "rect": [450, 50, 520, 120], "mode": "replace" }));
    if !ids.is_empty() {
        let _ = st.session.execute("select.set", json!({ "ids": ids, "mode": "add" }));
    }
    assert!(st.session.active().unwrap().selection.has_pixels(), "Pixelauswahl vorhanden");
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().unwrap().history.undo_count()
}

fn doc_json(st: &AppState) -> Value {
    serde_json::to_value(&*st.session.active().expect("Dokument").doc).unwrap()
}

// ------------------------------------------------------------------ Lagen

#[test]
fn pixelauswahl_sperrt_die_objektbefehle() {
    let (mut st, _ctx, f) = fixture();
    // S5 Bitmap ohne Pixelauswahl: alle aktiv
    select(&mut st, &f.bitmap);
    assert_state(&st, "Bitmap", &PIXEL_GREY, true);
    // S6 Bitmap mit Pixelauswahl: grau wie im Original
    pixel_selection(&mut st, &f.bitmap);
    assert_state(&st, "Bitmap + Pixelauswahl", &PIXEL_GREY, false);
    // Ausschneiden/Kopieren/Löschen bleiben aktiv
    assert_state(&st, "Bitmap + Pixelauswahl", &["edit.cut", "edit.copy", "edit.clear"], true);
    // Pixelauswahl aufheben → wieder aktiv (Wiederholung: zweimal hin und her)
    for _ in 0..2 {
        let _ = st.session.execute("select.pixels", json!({ "shape": "none" }));
        select(&mut st, &f.bitmap);
        assert_state(&st, "Bitmap nach Aufheben", &PIXEL_GREY, true);
        pixel_selection(&mut st, &f.bitmap);
        assert_state(&st, "Bitmap + Pixelauswahl erneut", &PIXEL_GREY, false);
    }
    // Pixelauswahl auch bei einem Vektorobjekt in der Auswahl: weiter grau
    pixel_selection(&mut st, &[f.bitmap[0], f.rect[0]]);
    assert_state(&st, "Rechteck + Pixelauswahl", &PIXEL_GREY, false);
}

#[test]
fn gruppe_freigabe_wie_im_original() {
    let (mut st, ctx, f) = fixture();
    select(&mut st, &f.group);
    // ○ im Original
    assert_state(&st, "Gruppe", &["modify.transform.removeTransformations"], false);
    // ● im Original
    assert_state(&st, "Gruppe", &["modify.combinePaths.join", "modify.snapToPixel", "modify.group", "edit.duplicate", "modify.ungroup"], true);

    // „Verbinden“ ohne Pfad in der Auswahl: Meldung statt Wirkung, kein Verlaufsschritt
    let before = doc_json(&st);
    let n = undo_count(&st);
    let r = st.session.execute("modify.combinePaths.join", json!({}));
    assert!(r.is_err(), "Verbinden bei Gruppe meldet, statt zu wirken");
    assert_eq!(undo_count(&st), n);
    assert_eq!(doc_json(&st), before);

    // „An Pixeln ausrichten“ richtet die Gruppe als Ganzes aus: genau ein Schritt, Rückgängig exakt
    select(&mut st, &f.group);
    st.push(Action::Run("edit.move".into(), json!({ "dx": 0.4, "dy": 0.3 })));
    st.process_actions(&ctx);
    let b0 = bounds(&st, f.group[0]);
    assert!(b0.left.fract().abs() > 0.01, "Gruppe liegt zwischen den Pixeln: {b0:?}");
    let before = doc_json(&st);
    let n = undo_count(&st);
    st.session.execute("modify.snapToPixel", json!({})).expect("An Pixeln ausrichten");
    let b1 = bounds(&st, f.group[0]);
    for v in [b1.left, b1.top, b1.right, b1.bottom] {
        assert!((v - v.round()).abs() < 1e-3, "Grenzen auf ganzen Pixeln: {b1:?}");
    }
    assert_eq!(undo_count(&st), n + 1, "genau ein Verlaufsschritt");
    // zweimal ausrichten ändert nichts mehr (Idempotenz)
    let after = doc_json(&st);
    let _ = st.session.execute("modify.snapToPixel", json!({}));
    assert_eq!(doc_json(&st), after, "zweites Ausrichten ohne Wirkung");
    // zurück bis vor das erste Ausrichten
    while undo_count(&st) > n {
        st.session.undo().expect("Rückgängig");
    }
    assert_eq!(doc_json(&st), before, "Rückgängig stellt exakt her");
}

fn bounds(st: &AppState, id: u64) -> fp_geom::Rect {
    let d = st.session.active().unwrap();
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("Element");
    fp_engine::docops::element_bounds(&d.doc, &e).expect("Grenzen")
}

#[test]
fn segment_freigabe_wie_im_original() {
    let (mut st, _ctx, f) = fixture();
    select(&mut st, &f.slice);
    assert_state(
        &st,
        "Segment",
        &[
            "edit.copyAsVectors",
            "modify.symbol.convertToSymbol",
            "modify.flattenSelection",
            "modify.mask.revealAll",
            "modify.mask.hideAll",
            "modify.transform.removeTransformations",
            "modify.popUpMenu.editPopUpMenu",
            "modify.popUpMenu.deletePopUpMenu",
        ],
        false,
    );
    assert_state(&st, "Segment", &["modify.combinePaths.join", "modify.popUpMenu.addPopUpMenu", "edit.duplicate", "modify.group"], true);
    // Segment mit Popupmenü: Bearbeiten/Löschen aktiv
    st.session.execute("modify.popUpMenu.addPopUpMenu", json!({ "items": [{ "text": "Eins", "url": "eins.htm" }] })).expect("Popupmenü");
    select(&mut st, &f.slice);
    assert_state(&st, "Segment mit Popupmenü", &["modify.popUpMenu.editPopUpMenu", "modify.popUpMenu.deletePopUpMenu"], true);
    // Löschen → wieder grau
    st.session.execute("modify.popUpMenu.deletePopUpMenu", json!({})).expect("Popupmenü löschen");
    select(&mut st, &f.slice);
    assert_state(&st, "Segment nach Löschen", &["modify.popUpMenu.editPopUpMenu", "modify.popUpMenu.deletePopUpMenu"], false);
    // Segment + Rechteck: nicht nur Web-Objekte → wieder aktiv
    select(&mut st, &[f.slice[0], f.rect[0]]);
    assert_state(&st, "Segment + Rechteck", &["edit.copyAsVectors", "modify.symbol.convertToSymbol", "modify.transform.removeTransformations"], true);
}

#[test]
fn rechteck_und_nichts_gewaehlt() {
    let (mut st, _ctx, f) = fixture();
    select(&mut st, &f.rect);
    let mut all: Vec<&str> = PIXEL_GREY.to_vec();
    all.extend(["modify.transform.removeTransformations", "modify.combinePaths.join", "modify.snapToPixel"]);
    assert_state(&st, "Rechteck", &all, true);
    select(&mut st, &[]);
    assert_state(&st, "ohne Auswahl", &all, false);
    assert_state(&st, "ohne Auswahl", &["modify.popUpMenu.editPopUpMenu", "modify.popUpMenu.deletePopUpMenu", "modify.mask.revealAll"], false);
}

// ------------------------------------------------------------------ Untermenü-Köpfe

fn subs(nodes: &[Node], path: &str, out: &mut Vec<(String, bool, bool)>) {
    for n in nodes {
        if let Node::Sub { label, children, enabled } = n {
            let any = children.iter().any(|k| matches!(k, Node::Item { enabled: true, .. } | Node::Sub { .. }));
            out.push((format!("{path} > {label}"), *enabled, any));
            subs(children, &format!("{path} > {label}"), out);
        }
    }
}

#[test]
fn untermenue_koepfe_immer_aktiv() {
    let (mut st, _ctx, f) = fixture();
    for (lage, ids) in [("ohne Auswahl", vec![]), ("Rechteck", f.rect.clone()), ("Text", f.text.clone()), ("Segment", f.slice.clone())] {
        select(&mut st, &ids);
        let mut out = Vec::new();
        for (top, nodes) in menu::build_main_menu(&st) {
            subs(&nodes, &top, &mut out);
        }
        assert!(out.len() > 20, "Untermenüs gefunden: {}", out.len());
        let grey: Vec<_> = out.iter().filter(|(_, en, _)| !en).map(|(p, ..)| p.clone()).collect();
        assert!(grey.is_empty(), "{lage}: graue Untermenü-Köpfe {grey:?}");
        // mindestens ein Kopf, dessen Einträge alle grau sind (z. B. Ausrichten, Text ▸ Richtung)
        if lage == "ohne Auswahl" {
            assert!(out.iter().any(|(_, en, any)| *en && !any), "{lage}: Kopf aktiv trotz grauer Einträge");
            for p in ["Modifizieren > Animation", "Modifizieren > Maske", "Modifizieren > Transformieren", "Modifizieren > Anordnen", "Modifizieren > Ausrichten", "Text > Richtung", "Filter > Weichzeichnen", "Filter > Andere"] {
                assert!(out.iter().any(|(q, en, _)| q == p && *en), "{lage}: {p} aktiv");
            }
        }
    }
}

// ------------------------------------------------------------------ Text ▸ Schrift

/// Wartet (höchstens 60 s), bis der Schriftindex aufgebaut ist.
fn families() -> std::sync::Arc<Vec<fp_ui::inspector::fonts::Family>> {
    let t0 = std::time::Instant::now();
    loop {
        let f = fp_ui::inspector::fonts::families();
        if f.len() > 5 || t0.elapsed().as_secs() > 60 {
            return f;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn font_nodes(st: &AppState) -> Vec<Node> {
    let menus = menu::build_main_menu(st);
    let (_, text) = menus.iter().find(|(l, _)| l == "Text").expect("Menü Text");
    for n in text {
        if let Node::Sub { label, children, enabled } = n
            && label == "Schrift"
        {
            assert!(*enabled, "Kopf Schrift aktiv");
            return children.clone();
        }
    }
    panic!("Text ▸ Schrift fehlt");
}

fn label_of(n: &Node) -> Option<&str> {
    match n {
        Node::Item { label, .. } | Node::Sub { label, .. } => Some(label),
        Node::Separator => None,
    }
}

/// Familien mit Häkchen (Eintrag selbst oder ein Schnitt darunter).
fn checked_families(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for n in nodes {
        match n {
            Node::Item { label, checked: Some(true), .. } => out.push(label.clone()),
            Node::Sub { label, children, .. } if children.iter().any(|k| matches!(k, Node::Item { checked: Some(true), .. })) => out.push(label.clone()),
            _ => {}
        }
    }
    out
}

fn pick_font(st: &mut AppState, ctx: &egui::Context, ps: &str) {
    let a = Action::Run("text.font".into(), json!({ "font": ps }));
    menu::note_font_choice(st, &a);
    st.push(a);
    st.process_actions(ctx);
}

fn first_item_action(n: &Node) -> Option<Action> {
    match n {
        Node::Item { action: ItemAction::Ui(a), enabled, .. } => {
            assert!(*enabled, "Schrift-Eintrag aktiv");
            Some(a.clone())
        }
        Node::Sub { children, .. } => children.iter().find_map(first_item_action),
        _ => None,
    }
}

#[test]
fn schrift_ohne_text_aktiv_setzt_vorgabe() {
    let fams = families();
    let (mut st, ctx, _f) = fixture();
    select(&mut st, &[]);
    assert!(available(&st, "text.font"), "Schrift ohne Text aktiv");
    let nodes = font_nodes(&st);
    // alle Einträge aktiv
    fn all_enabled(nodes: &[Node]) -> bool {
        nodes.iter().all(|n| match n {
            Node::Item { enabled, .. } => *enabled,
            Node::Sub { enabled, children, .. } => *enabled && all_enabled(children),
            Node::Separator => true,
        })
    }
    assert!(all_enabled(&nodes), "alle Schrift-Einträge aktiv ohne Text");
    // eine andere Familie als die aktuelle wählen (über die Aktion des Menüeintrags)
    let cur = st.session.tools.options.text_font.clone();
    let target = fams.iter().find(|f| !f.name.eq_ignore_ascii_case(&cur)).expect("zweite Familie").clone();
    let node = nodes.iter().rev().find(|n| label_of(n) == Some(target.name.as_str())).expect("Eintrag der Familie");
    let Some(Action::Run(name, p)) = first_item_action(node) else { panic!("Aktion") };
    assert_eq!(name, "text.font");
    let before = doc_json(&st);
    let n = undo_count(&st);
    // 5× dieselbe Wahl: Vorgabe gesetzt, kein Verlaufsschritt, Dokument unverändert, einmal in der Liste
    for _ in 0..5 {
        pick_font(&mut st, &ctx, p["font"].as_str().unwrap());
    }
    assert_eq!(undo_count(&st), n, "Vorgabe setzen ist kein Verlaufsschritt");
    assert_eq!(doc_json(&st), before, "Dokument unverändert");
    let tf = st.session.tools.options.text_font.clone();
    assert!(
        tf.eq_ignore_ascii_case(&target.name) || target.styles.iter().any(|(_, ps)| ps.eq_ignore_ascii_case(&tf)),
        "Vorgabe des Textwerkzeugs = {} (ist {tf})",
        target.name
    );
    assert_eq!(st.session.prefs.get_str("Text_Font", ""), tf, "Voreinstellung Text_Font (englischer Schlüssel)");
    // ✓ an der gewählten Schrift, sonst nirgends (außer derselben Familie oben in „zuletzt benutzt“)
    let nodes = font_nodes(&st);
    let checked = checked_families(&nodes);
    assert!(!checked.is_empty() && checked.iter().all(|c| *c == target.name), "✓ nur an {}: {checked:?}", target.name);
    // zuletzt benutzt: oben, genau einmal, gefolgt von einer Trennlinie
    assert_eq!(label_of(&nodes[0]), Some(target.name.as_str()), "zuletzt benutzte Schrift oben");
    let list = st.session.prefs.get_str("RecentFontList", "");
    assert_eq!(list.matches(&format!("({})", target.name)).count(), 1, "einmal in RecentFontList: {list}");
    let sep = nodes.iter().position(|n| matches!(n, Node::Separator)).expect("Trennlinie nach den zuletzt benutzten");
    // neuer Text übernimmt die Vorgabe
    let ids = ex(&mut st, "draw.text", json!({ "text": "Neu", "anchor": [100, 560], "size": 20 }));
    assert!(!ids.is_empty());
    let nodes = font_nodes(&st);
    assert_eq!(checked_families(&nodes[sep + 1..]), vec![target.name.clone()], "✓ am neuen Text");
}

#[test]
fn schrift_haekchen_am_ausgewaehlten_text() {
    let fams = families();
    let (mut st, ctx, f) = fixture();
    select(&mut st, &f.text);
    let target = fams.iter().rev().find(|x| x.styles.len() == 1).or(fams.last()).expect("Familie").clone();
    let ps = target.styles[0].1.clone();
    let n = undo_count(&st);
    pick_font(&mut st, &ctx, &ps);
    assert_eq!(undo_count(&st), n + 1, "Schrift am Text = ein Verlaufsschritt");
    select(&mut st, &f.text);
    let nodes = font_nodes(&st);
    let sep = nodes.iter().position(|n| matches!(n, Node::Separator)).unwrap_or(0);
    assert_eq!(checked_families(&nodes[sep + 1..]), vec![target.name.clone()], "✓ an der Schrift des Texts");
    // Vorgabe des Werkzeugs bleibt unberührt
    assert_ne!(st.session.prefs.values.get("Text_Font").and_then(|v| v.as_str()), Some(ps.clone()));
    st.session.undo().expect("Rückgängig");
}

#[test]
fn schrift_familien_flach() {
    let fams = families();
    let (mut st, _ctx, _f) = fixture();
    st.session.prefs.set("RecentFontList", fp_engine::prefs::PrefValue::Str(String::new()));
    let nodes = font_nodes(&st);
    let names: Vec<&str> = nodes.iter().filter_map(label_of).collect();
    let want: Vec<&str> = fams.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, want, "alle Familien flach in Indexreihenfolge (keine Buchstaben-Untermenüs)");
    // Familien mit mehreren Schnitten: Untermenü mit den Schnitten
    for (n, f) in nodes.iter().zip(fams.iter()) {
        match n {
            Node::Sub { children, .. } => assert_eq!(children.len(), f.styles.len(), "{}: Schnitte", f.name),
            Node::Item { .. } => assert_eq!(f.styles.len(), 1, "{}: ein Schnitt", f.name),
            Node::Separator => unreachable!(),
        }
    }
}

#[test]
fn schrift_menue_mit_bildlauf_im_fenster() {
    // Das lange flache Untermenü bekommt einen Bildlauf und bleibt im Fenster (600 px hoch).
    use egui_kittest::Harness;
    use std::sync::{Arc, Mutex};
    let _ = families();
    let (st, _ctx, _f) = fixture();
    let mut nodes = font_nodes(&st);
    // mindestens 300 Einträge (wie das Original mit 439 Familien), unabhängig von den installierten Schriften
    let mut k = 0;
    while nodes.len() < 300 {
        nodes.push(Node::Item { label: format!("Familie {k}"), shortcut: None, action: ItemAction::None, enabled: true, checked: Some(false) });
        k += 1;
    }
    let height = Arc::new(Mutex::new(0.0f32));
    let h2 = Arc::clone(&height);
    let mut h = Harness::builder().with_size(egui::vec2(400.0, 600.0)).build_ui(move |ui| {
        let mut out = Vec::new();
        let r = ui.scope(|ui| menu::show_submenu_body(ui, &nodes, &mut out)).response.rect;
        *h2.lock().unwrap() = r.height();
    });
    h.run();
    let hgt = *height.lock().unwrap();
    assert!(hgt > 100.0 && hgt <= 600.0, "Untermenü passt mit Bildlauf ins Fenster: {hgt}");
    // kurze Untermenüs bleiben ohne Bildlauf in voller Höhe
    const { assert!(menu::LONG_SUBMENU >= 30) };
}
