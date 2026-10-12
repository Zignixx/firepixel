//! EB-1: kleine Engine-Befunde aus dem Funktionsvergleich mit dem Original (PM-1, FV-1, FV-3, FV-4).

mod common;
use common::*;
use serde_json::json;

/// PM-1 (N05): R1 wählen, Kopieren, 2× Einfügen, 2× Rückgängig → im Original ist nichts gewählt.
/// Wiederholen wählt die eingefügte Kopie wieder; Rückgängig anderer Befehle stellt die Auswahl her.
#[test]
fn undo_paste_leaves_nothing_selected() {
    let mut s = doc_session(300.0, 200.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    select(&mut s, &[r1]);
    ex(&mut s, "edit.copy", json!({}));
    ex(&mut s, "edit.paste", json!({}));
    ex(&mut s, "edit.paste", json!({}));
    assert_eq!(top_elements(&s).len(), 3);
    let sel = |s: &fp_engine::Session| s.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
    let second = sel(&s);
    assert_eq!(second.len(), 1);
    assert_ne!(second, vec![r1]);
    ex(&mut s, "edit.undo", json!({}));
    assert!(sel(&s).is_empty(), "nach dem ersten Rückgängig nichts gewählt");
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(top_elements(&s).len(), 1);
    assert!(sel(&s).is_empty(), "nach 2× Rückgängig nichts gewählt (Original)");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(top_elements(&s).len(), 2);
    assert_eq!(sel(&s).len(), 1);
    assert_ne!(sel(&s), vec![r1], "Wiederholen wählt die eingefügte Kopie");
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(sel(&s), second);
    // Folge A, B, Rückgängig, C: Verschieben zurücknehmen behält die Auswahl wie bisher.
    select(&mut s, &[r1]);
    ex(&mut s, "edit.move", json!({"dx": 5, "dy": 0}));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(sel(&s), vec![r1]);
    assert!(approx(bounds(&s, r1).left, 50.0));
}

/// FV-4 (UX-B D18): Neue Dokumente und Seiten bekommen das Raster aus den Voreinstellungen
/// (`GridColor` #00ffff, Abstände 36 px); geänderte Voreinstellungen wirken auf das nächste Dokument.
#[test]
fn new_document_grid_from_preferences() {
    use fp_engine::{PrefValue, Preferences, Session};
    let grid = |s: &Session, page: usize| s.active().and_then(|d| d.doc.pages.get(page).and_then(|p| p.root.grid.clone())).expect("Raster gesetzt");
    let mut s = Session::new(Preferences::new());
    ex(&mut s, "file.new", json!({"width": 200, "height": 100}));
    let g = grid(&s, 0);
    assert_eq!(g.color, fp_color::Argb(0xff00_ffff));
    assert_eq!((g.size.x, g.size.y), (36.0, 36.0));
    ex(&mut s, "edit.insert.page", json!({}));
    assert_eq!(grid(&s, 1).color, fp_color::Argb(0xff00_ffff));
    // Rückgängig nimmt die Seite samt Raster zurück, Seite 1 behält ihr Raster.
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(s.active().map(|d| d.doc.pages.len()), Some(1));
    assert_eq!(grid(&s, 0).color, fp_color::Argb(0xff00_ffff));
    let mut p = Preferences::new();
    p.set("GridColor", PrefValue::Str("#ff0000".into()));
    p.set("GridHorizontalDistance", PrefValue::Int(10));
    let mut s = Session::new(p);
    ex(&mut s, "file.new", json!({"width": 200, "height": 100}));
    let g = grid(&s, 0);
    assert_eq!(g.color, fp_color::Argb(0xffff_0000));
    assert_eq!((g.size.x, g.size.y), (10.0, 36.0));
}

/// FV-3 (UX-A O4/O5): Texte zeigen 2 px Rand um den gespeicherten Rahmen (Original: Rahmen
/// 50,170–200,194 → X 48, Y 168, B 154, H 28). Der gespeicherte Rahmen bleibt exakt (re\09 §5.3),
/// Position setzen mit den angezeigten Werten verschiebt nichts, auch 50× nicht.
#[test]
fn text_bounds_show_two_pixel_margin() {
    use fp_model::element::ElementKind;
    let mut s = doc_session(400.0, 300.0);
    let v = ex(&mut s, "draw.text", json!({"text": "Abc", "anchor": [50, 190], "size": 24}));
    let id = fp_model::ElementId(v["id"].as_u64().expect("id"));
    let frame = |s: &fp_engine::Session| match &el(s, id).kind {
        ElementKind::Text(t) => t.bounds,
        _ => panic!("kein Text"),
    };
    let f = frame(&s);
    let shown = visible_bounds(&s, id);
    assert_eq!(shown, fp_geom::Rect::new(f.left - 2.0, f.top - 2.0, f.right + 2.0, f.bottom + 2.0));
    assert_eq!(bounds(&s, id), f, "Testhilfe liefert den Satzrahmen");
    // Inspektor-Werte zurückschreiben: keine Bewegung, keine Drift
    let n = s.active().map(|d| d.history.len()).unwrap_or(0);
    select(&mut s, &[id]);
    for _ in 0..50 {
        ex(&mut s, "element.setBounds", json!({"x": shown.left, "y": shown.top}));
    }
    assert_eq!(frame(&s), f);
    assert_eq!(visible_bounds(&s, id), shown);
    // X = 0 legt den Rahmen auf 2 (sichtbare Kante an 0), Rückgängig stellt her
    ex(&mut s, "element.setBounds", json!({"x": 0}));
    assert!(approx(visible_bounds(&s, id).left, 0.0));
    assert!(approx(frame(&s).left, 2.0));
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(frame(&s), f);
    assert!(s.active().map(|d| d.history.len()).unwrap_or(0) >= n);
    // Gruppe aus Text: Grenzen mit Rand
    let r = rect(&mut s, 300.0, 10.0, 310.0, 20.0);
    select(&mut s, &[id, r]);
    ex(&mut s, "modify.group", json!({}));
    let g = s.active().and_then(|d| d.selection.elements.first().copied()).expect("Gruppe");
    let gb = visible_bounds(&s, g);
    assert!(approx(gb.left, shown.left) && approx(gb.bottom, shown.bottom.max(20.0)), "{gb:?}");
    // Speichern/Laden: Rahmen unverändert
    let dir = std::env::temp_dir().join(format!("fp_eb1_text_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("t.fw.png");
    ex(&mut s, "file.saveAs", json!({"path": path.to_string_lossy()}));
    let mut s2 = fp_engine::Session::default();
    ex(&mut s2, "file.open", json!({"path": path.to_string_lossy()}));
    let mut texts = Vec::new();
    fn walk(e: &fp_model::Element, v: &mut Vec<fp_geom::Rect>) {
        match &e.kind {
            ElementKind::Text(t) => v.push(t.bounds),
            ElementKind::Group(g) => g.children.iter().for_each(|c| walk(c, v)),
            _ => {}
        }
    }
    for e in top_elements(&s2) {
        walk(&e, &mut texts);
    }
    assert_eq!(texts, vec![f]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// FV-1 (UX-A W15/W17, O6): Glättung, Autom. Unterschneidung und Textrichtung sind Vorgaben des
/// Textwerkzeugs. Werksvorgabe wie im Original: glatt, ohne Unterschneidung, waagerecht.
/// `text.setToolDefaults` (Inspektor ohne Auswahl) setzt sie ohne Verlaufsschritt; neue Texte
/// (Werkzeug wie Skript, `draw.text`) bekommen sie, Parameter überschreiben.
#[test]
fn text_tool_defaults_for_new_texts() {
    use fp_model::element::ElementKind;
    let mut s = doc_session(400.0, 300.0);
    let text = |s: &fp_engine::Session, id: fp_model::ElementId| match &el(s, id).kind {
        ElementKind::Text(t) => (t.antialias, t.attrs.fill.as_ref().map(|f| f.edge), t.auto_kern, t.orientation),
        _ => panic!("kein Text"),
    };
    let new_text = |s: &mut fp_engine::Session, extra: serde_json::Value| {
        let mut p = json!({"text": "Ab", "anchor": [20, 50], "size": 24});
        if let (Some(o), Some(e)) = (p.as_object_mut(), extra.as_object()) {
            o.extend(e.clone());
        }
        fp_model::ElementId(ex(s, "draw.text", p)["id"].as_u64().expect("id"))
    };
    let a = new_text(&mut s, json!({}));
    assert_eq!(text(&s, a), (0, Some(1), false, 0), "Werksvorgabe: glatt, weiche Kante, ohne Unterschneidung");
    // Vorgabe setzen: kein Verlaufsschritt, Dokument unverändert; 50× = einmal
    let n = s.active().map(|d| d.history.len()).unwrap_or(0);
    let doc0 = s.active().map(|d| d.doc.clone()).expect("Dokument");
    for i in 0..50 {
        let aa = if i % 2 == 0 { "strong" } else { "crisp" };
        ex(&mut s, "text.setToolDefaults", json!({"antialias": aa, "autoKern": i % 2 == 0, "orientation": "verticalRtl"}));
    }
    let out = ex(&mut s, "text.setToolDefaults", json!({}));
    assert_eq!(out, json!({"antialias": "crisp", "autoKern": false, "orientation": "verticalRtl"}));
    assert_eq!(s.active().map(|d| d.history.len()), Some(n));
    assert!(s.active().is_some_and(|d| std::sync::Arc::ptr_eq(&d.doc, &doc0)));
    assert_eq!(text(&s, a), (0, Some(1), false, 0), "vorhandener Text bleibt");
    ex(&mut s, "text.setToolDefaults", json!({"autoKern": true}));
    let b = new_text(&mut s, json!({}));
    assert_eq!(text(&s, b), (1, Some(1), true, 1));
    assert_eq!(fp_engine::commands::tool_text_defaults(&s.prefs), (1, true, 1));
    // Keine Kantenglättung = harte Füllkante
    ex(&mut s, "text.setToolDefaults", json!({"antialias": "none", "orientation": "horizontalLtr"}));
    let c = new_text(&mut s, json!({}));
    assert_eq!(text(&s, c).1, Some(0));
    assert_eq!(text(&s, c).3, 0);
    // Parameter überschreiben die Vorgabe
    let d = new_text(&mut s, json!({"antialias": "custom", "autoKern": false, "orientation": "horizontalRtl"}));
    assert_eq!(text(&s, d), (4, Some(1), false, 2));
    // Rückgängig/Wiederholen des neuen Textes
    ex(&mut s, "edit.undo", json!({}));
    assert!(s.active().is_some_and(|x| fp_engine::docops::element_on_page(&x.doc, x.page, d).is_none()));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(text(&s, d), (4, Some(1), false, 2));
    // Textwerkzeug (Klick, tippen, Esc) nimmt die Vorgaben
    ex(&mut s, "text.setToolDefaults", json!({"antialias": "strong", "autoKern": true, "orientation": "horizontalLtr"}));
    s.set_tool(fp_engine::ToolId::Text).expect("Textwerkzeug");
    let old: Vec<fp_model::ElementId> = top_elements(&s).iter().map(|e| e.id).collect();
    let before = old.len();
    events(&mut s, &[fp_engine::ToolEvent::down(200.0, 200.0), fp_engine::ToolEvent::up(200.0, 200.0)]);
    let typed = fp_engine::ToolEvent { input: fp_engine::tools::ToolInput::Text("Xy".into()), mods: Default::default(), pressure: 1.0, time_ms: 0 };
    s.tool_event(&typed).expect("tippen");
    s.tool_event(&fp_engine::ToolEvent::key(fp_engine::ToolKey::Escape)).expect("Esc");
    let els = top_elements(&s);
    assert_eq!(els.len(), before + 1);
    let last = els.iter().map(|e| e.id).find(|id| !old.contains(id)).expect("neuer Text");
    assert_eq!(text(&s, last), (2, Some(1), true, 0));
}

/// Während der Bearbeitung zeigen Inspektor/Info die Grenzen mit Rand (`text_edit_bounds`),
/// nach dem Übernehmen gleich den Objektgrenzen.
#[test]
fn text_edit_bounds_with_margin() {
    let mut s = doc_session(400.0, 300.0);
    s.set_tool(fp_engine::ToolId::Text).expect("Textwerkzeug");
    events(&mut s, &[fp_engine::ToolEvent::down(100.0, 100.0), fp_engine::ToolEvent::up(100.0, 100.0)]);
    let typed = fp_engine::ToolEvent { input: fp_engine::tools::ToolInput::Text("Hallo".into()), mods: Default::default(), pressure: 1.0, time_ms: 0 };
    s.tool_event(&typed).expect("tippen");
    let f = s.text_edit_frame().expect("Rahmen");
    let b = s.text_edit_bounds().expect("Grenzen");
    assert_eq!(b, fp_geom::Rect::new(f.left - 2.0, f.top - 2.0, f.right + 2.0, f.bottom + 2.0));
    s.tool_event(&fp_engine::ToolEvent::key(fp_engine::ToolKey::Escape)).expect("Esc");
    let t = top_elements(&s).iter().find(|e| matches!(e.kind, fp_model::element::ElementKind::Text(_))).map(|e| e.id).expect("Text");
    assert_eq!(visible_bounds(&s, t), b);
}
