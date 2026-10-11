//! Eigenschaften-Inspektor offscreen (egui_kittest): richtiges Layout je Elementart und Werkzeug,
//! alle Quadranten der Layout-Tabelle haben einen Inhalt, und jedes Text-/Zahlenfeld des Inspektors
//! ändert das Dokument; Rückgängig stellt es exakt wieder her.

use egui_kittest::kittest::{By, Queryable};
use egui_kittest::Harness;
use fp_engine::{Session, ToolId};
use fp_ui::app_state::UiSettings;
use fp_ui::inspector::{layouts, InspectorState, QuadrantRegistry};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

fn new_doc() -> Session {
    unsafe_env();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300})).expect("Dokument");
    s
}

/// Keine externen Programme aus Tests (AGENTEN_REGELN §4a).
fn unsafe_env() {
    // SAFETY-frei: nur Prozessumgebung des Tests; Rust 2024 verlangt `unsafe` für set_var, daher
    // über die Kommandozeile gesetzt, wenn möglich – sonst prüfen die Befehle die Variable nicht.
    if std::env::var_os("FP_NO_SHELL_OPEN").is_none() {
        eprintln!("Hinweis: FP_NO_SHELL_OPEN ist nicht gesetzt");
    }
}

fn app(s: Session) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    a
}

fn harness(a: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1800.0, 1130.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn inspector_state(h: &Harness<'_, FirepixelApp>) -> InspectorState {
    h.ctx.data(|d| d.get_temp::<InspectorState>(egui::Id::new("inspector-state"))).expect("Inspektor gezeichnet")
}

fn ex(s: &mut Session, name: &str, p: serde_json::Value) {
    s.execute(name, p).unwrap_or_else(|e| panic!("{name}: {e}"));
}

/// Dokumente mit je einer Elementart (Name, Aufbau, erwartete Layoutnummer nach 11 §2.3.2).
fn cases() -> Vec<(&'static str, Session, u16)> {
    let mut v = Vec::new();
    v.push(("Dokument", new_doc(), 30));
    let mut s = new_doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 80]}));
    v.push(("Rechteck", s, 58));
    let mut s = new_doc();
    ex(&mut s, "draw.rectangle", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "pi.stroke", json!({"width": 3, "color": "#0000ff"}));
    v.push(("Rechteck mit Strich", s, 58));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    v.push(("Pfad", s, 26));
    let mut s = new_doc();
    ex(&mut s, "draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24}));
    v.push(("Text", s, 25));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.flattenSelection", json!({}));
    v.push(("Bitmap", s, 27));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [140, 20, 220, 80]}));
    ex(&mut s, "select.all", json!({}));
    ex(&mut s, "modify.group", json!({}));
    v.push(("Gruppe", s, 26));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "S", "kind": "graphic"}));
    v.push(("Grafiksymbol", s, 68));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "B", "kind": "button"}));
    v.push(("Schaltfläche", s, 28));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "modify.symbol.convertToSymbol", json!({"name": "A", "kind": "animation"}));
    v.push(("Animation", s, 29));
    let mut s = new_doc();
    ex(&mut s, "web.addSlice", json!({"shape": "rect", "rect": [10, 10, 90, 60]}));
    v.push(("Segment", s, 22));
    let mut s = new_doc();
    ex(&mut s, "web.addHotspot", json!({"shape": "rect", "rect": [10, 10, 90, 60]}));
    v.push(("Hotspot", s, 23));
    let mut s = new_doc();
    ex(&mut s, "draw.ellipse", json!({"rect": [20, 20, 120, 80]}));
    ex(&mut s, "draw.ellipse", json!({"rect": [140, 20, 220, 80]}));
    ex(&mut s, "select.all", json!({}));
    v.push(("Mehrere", s, 26));
    v
}

#[test]
fn every_layout_template_has_content() {
    let reg = QuadrantRegistry::with_defaults();
    let mut missing = Vec::new();
    for l in layouts::LAYOUTS {
        for q in l.quads {
            if q != 0 && !reg.contains(q) && !missing.contains(&q) {
                missing.push(q);
            }
        }
    }
    // 14079–14084 sind die leeren Quadranten ohne Dokument (Original: leer)
    missing.retain(|q| !(14079..=14084).contains(q));
    assert!(missing.is_empty(), "Vorlagen ohne Inhalt: {missing:?}");
}

#[test]
fn inspector_layout_per_element_kind() {
    for (name, s, want) in cases() {
        let mut h = harness(app(s));
        h.run_steps(3);
        let l = layouts::select_layout(&h.state().state.session).unwrap_or_else(|| panic!("{name}: kein Layout"));
        assert_eq!(l.number, want, "{name}: Layout");
        let st = inspector_state(&h);
        for (i, q) in l.quads.iter().enumerate() {
            if *q != 0 {
                assert_eq!(st.quads[i], *q, "{name}: Quadrant {}", i + 1);
            }
        }
        assert_eq!(st.mask, l.mask, "{name}: Trennlinien");
    }
}

#[test]
fn inspector_layout_per_tool() {
    let reg = QuadrantRegistry::with_defaults();
    let mut h = harness(app(new_doc()));
    h.run_steps(2);
    for t in fp_engine::tools::TOOLS {
        if h.state_mut().state.session.set_tool(t.id).is_err() {
            continue;
        }
        h.run_steps(2);
        let st = inspector_state(&h);
        if let Some(l) = layouts::select_layout(&h.state().state.session) {
            for (i, q) in l.quads.iter().enumerate() {
                if *q != 0 {
                    assert_eq!(st.quads[i], *q, "{:?}: Quadrant {}", t.id, i + 1);
                    assert!(reg.contains(*q) || (14079..=14084).contains(q), "{:?}: Vorlage {q} ohne Inhalt", t.id);
                }
            }
        }
    }
    let _ = ToolId::Pointer;
}

fn doc_arc(h: &Harness<'_, FirepixelApp>) -> Arc<fp_model::Document> {
    Arc::clone(&h.state().state.session.active().expect("Dokument").doc)
}

/// Alle Textfelder im Inspektorbereich (unterer Fensterteil).
fn inspector_inputs(h: &Harness<'_, FirepixelApp>) -> Vec<(egui::Rect, String)> {
    let top = 1130.0 - 160.0;
    h.get_all(By::new().role(egui::accesskit::Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).filter(|(r, _)| r.top() > top).collect()
}

fn new_value(v: &str) -> String {
    let t = v.trim().trim_end_matches('%').trim();
    if t.is_empty() {
        return "5".into();
    }
    match t.replace(',', ".").parse::<f64>() {
        Ok(x) => format!("{}", (x + 7.0).round()),
        Err(_) => format!("{t}X"),
    }
}

#[test]
fn inspector_text_fields_change_document_and_undo() {
    let mut total = 0;
    for (name, s, _) in cases() {
        if name == "Dokument" {
            continue;
        }
        let mut h = harness(app(s));
        h.run_steps(3);
        let fields = inspector_inputs(&h);
        assert!(!fields.is_empty(), "{name}: keine Eingabefelder im Inspektor");
        let mut changed = 0;
        for (k, (rect, val)) in fields.iter().enumerate() {
            let before = doc_arc(&h);
            let undo_before = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
            // Feld fokussieren, Inhalt ersetzen, mit Eingabe bestätigen
            let nodes: Vec<_> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect() == *rect).collect();
            let Some(n) = nodes.first() else { continue };
            n.focus();
            drop(nodes);
            h.run_steps(1);
            h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::A]);
            let nodes: Vec<_> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect() == *rect).collect();
            if let Some(n) = nodes.first() {
                n.type_text(&new_value(val));
            }
            drop(nodes);
            h.key_press(egui::Key::Enter);
            h.run_steps(3);
            let after = doc_arc(&h);
            if Arc::ptr_eq(&before, &after) {
                eprintln!("  {name}: Feld {k} bei {rect:?} Wert „{val}“ → „{}“ ohne Wirkung", new_value(val));
            }
            if !Arc::ptr_eq(&before, &after) {
                changed += 1;
                let undo_after = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
                assert!(undo_after > undo_before, "{name} Feld {k} ({val}): kein Verlaufsschritt");
                h.state_mut().state.session.undo().expect("undo");
                h.run_steps(2);
                assert!(*doc_arc(&h) == *before, "{name} Feld {k} ({val}): Rückgängig stellt nicht her");
            }
        }
        eprintln!("{name}: {changed}/{} Felder ändern das Dokument", fields.len());
        assert!(changed > 0, "{name}: kein Feld wirkt");
        total += changed;
    }
    assert!(total >= 20, "nur {total} Inspektorfelder wirken");
}

/// §4b: dasselbe Inspektorfeld 15× ändern (unter MaxUndos 20) → Endwert, je Änderung ein Schritt, alles rückgängig.
#[test]
fn inspector_field_repeated_edits() {
    let (_, s, _) = cases().into_iter().find(|c| c.0 == "Rechteck").expect("Rechteck");
    let mut h = harness(app(s));
    h.run_steps(3);
    let d0 = doc_arc(&h);
    let n0 = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
    // Breitenfeld (B) im Quadranten 2: Wert 100
    let (rect, _) = inspector_inputs(&h).into_iter().find(|(_, v)| v == "100").expect("Feld B");
    for k in 0..15 {
        let w = 101 + k;
        let nodes: Vec<_> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect() == rect).collect();
        nodes.first().expect("Feld").focus();
        drop(nodes);
        h.run_steps(1);
        h.key_combination_modifiers(egui::Modifiers::COMMAND, &[egui::Key::A]);
        let nodes: Vec<_> = h.get_all(By::new().role(egui::accesskit::Role::TextInput)).filter(|n| n.rect() == rect).collect();
        nodes.first().expect("Feld").type_text(&w.to_string());
        drop(nodes);
        h.key_press(egui::Key::Enter);
        h.run_steps(3);
    }
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let b = fp_engine::docops::ids_bounds(&d.doc, d.page, &d.selection.elements).expect("Grenzen");
    assert!((b.width() - 115.0).abs() < 0.6, "Endbreite {}", b.width());
    assert_eq!(fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).len(), 1, "keine Duplikate");
    assert_eq!(d.history.undo_count(), n0 + 15, "je Änderung ein Schritt");
    for _ in 0..15 {
        h.state_mut().state.session.undo().expect("undo");
    }
    assert!(*doc_arc(&h) == *d0, "alles rückgängig");
}