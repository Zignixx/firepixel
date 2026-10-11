//! Undo/Redo-Verlauf (10 §3, Laufzeit T6).

mod common;
use common::*;
use fp_engine::{PrefValue, Preferences, Session, SessionEvent};
use serde_json::json;

/// Laufzeit T6: `MaxUndos` = 5 → Schrittzahlen 1, 2, 3, 4, 5, 5, 5; die ältesten fallen einzeln weg;
/// 10× Rückgängig nimmt genau 5 Schritte zurück.
#[test]
fn max_undos_trimming_like_t6() {
    let mut p = Preferences::new();
    p.set("MaxUndos", PrefValue::Int(5));
    let mut s = Session::new(p);
    ex(&mut s, "file.new", json!({"width": 100, "height": 100}));
    let mut counts = Vec::new();
    for i in 0..7 {
        rect(&mut s, i as f32, 0.0, i as f32 + 5.0, 5.0);
        counts.push(s.active().map(|d| d.history.undo_count()).unwrap_or(0));
    }
    assert_eq!(counts, vec![1, 2, 3, 4, 5, 5, 5]);
    assert_eq!(s.active().map(|d| d.history.dropped()), Some(2));
    let mut undone = 0;
    for _ in 0..10 {
        if s.execute("edit.undo", json!({})).is_ok() {
            undone += 1;
        }
    }
    assert_eq!(undone, 5);
    assert_eq!(top_elements(&s).len(), 2);
}

#[test]
fn undo_redo_restores_document_and_selection() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    ex(&mut s, "edit.move", json!({"dx": 5, "dy": 0}));
    assert!(approx(bounds(&s, a).left, 5.0));
    let g0 = s.active().map(|d| d.generation).unwrap_or(0);
    ex(&mut s, "edit.undo", json!({}));
    assert!(approx(bounds(&s, a).left, 0.0));
    assert!(s.active().is_some_and(|d| d.generation > g0), "Generation steigt auch beim Rückgängig");
    assert_eq!(s.active().map(|d| d.history.redo_label().map(|l| l.to_string())), Some(Some("Verschieben".to_string())));
    ex(&mut s, "edit.redo", json!({}));
    assert!(approx(bounds(&s, a).left, 5.0));
    ex(&mut s, "edit.undo", json!({}));
    ex(&mut s, "edit.undo", json!({}));
    assert!(top_elements(&s).is_empty());
    assert!(s.active().is_some_and(|d| d.selection.elements.is_empty()));
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![a]));
    // neuer Befehl leert den Redo-Stapel
    rect(&mut s, 50.0, 50.0, 60.0, 60.0);
    assert!(!s.is_available("edit.redo"));
    assert_eq!(s.active().map(|d| d.history.labels()), Some(vec!["Rechteck".to_string(), "Rechteck".to_string()]));
}

#[test]
fn selection_changes_are_steps_view_is_not() {
    let mut s = doc_session(100.0, 100.0);
    rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let n = |s: &Session| s.active().map(|d| d.history.undo_count()).unwrap_or(0);
    let before = n(&s);
    ex(&mut s, "select.none", json!({}));
    assert_eq!(n(&s), before + 1, "Auswahl aufheben ist ein eigener Schritt");
    ex(&mut s, "select.none", json!({}));
    assert_eq!(n(&s), before + 1, "ohne Änderung kein Schritt");
    ex(&mut s, "select.all", json!({}));
    assert_eq!(n(&s), before + 2);
    ex(&mut s, "view.zoomIn", json!({}));
    ex(&mut s, "view.grid.showGrid", json!({}));
    ex(&mut s, "edit.copy", json!({}));
    assert_eq!(n(&s), before + 2, "Ansicht und Kopieren sind keine Schritte");
}

#[test]
fn failed_command_leaves_state() {
    let mut s = doc_session(100.0, 100.0);
    let a = rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    let generation = s.active().map(|d| d.generation);
    let n = s.active().map(|d| d.history.undo_count());
    // swapSymbol mit unbekanntem Symbol: Instanz nötig → Fehler
    assert!(s.execute("modify.symbol.swapSymbol", json!({"symbol": "x"})).is_err());
    assert!(s.execute("select.set", json!({"ids": [999999]})).is_err());
    assert_eq!(s.active().map(|d| d.generation), generation);
    assert_eq!(s.active().map(|d| d.history.undo_count()), n);
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![a]));
}

#[test]
fn export_and_replay_steps() {
    let mut s = doc_session(100.0, 100.0);
    rect(&mut s, 0.0, 0.0, 10.0, 10.0);
    ex(&mut s, "modify.transform.flipVertical", json!({}));
    ex(&mut s, "edit.duplicate", json!({}));
    let v = ex(&mut s, "history.export", json!({}));
    let cmds: Vec<fp_engine::RecordedCommand> = serde_json::from_value(v["commands"].clone()).unwrap_or_default();
    assert_eq!(cmds.len(), 3);
    assert_eq!(cmds[0].command, "draw.rectangle");
    assert!(v["script"].as_str().is_some_and(|t| t.contains("fp.run(\"modify.transform.flipVertical\"")));
    // in neuem Dokument wiederholen: ein einziger Verlaufsschritt
    let mut t = doc_session(100.0, 100.0);
    ex(&mut t, "history.replay", json!({"commands": cmds, "label": "Befehl"}));
    assert_eq!(top_elements(&t).len(), 2);
    assert_eq!(t.active().map(|d| d.history.undo_count()), Some(1));
    assert_eq!(t.active().and_then(|d| d.history.undo_label().map(|l| l.to_string())), Some("Befehl".to_string()));
    ex(&mut t, "edit.undo", json!({}));
    assert!(top_elements(&t).is_empty());
}

#[test]
fn group_is_one_step_and_go_to() {
    let mut s = doc_session(100.0, 100.0);
    s.run_group("Skript", |s| {
        s.execute("draw.rectangle", json!({"rect": [0, 0, 5, 5]}))?;
        s.execute("draw.ellipse", json!({"rect": [10, 0, 15, 5]}))?;
        Ok(())
    })
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s.active().map(|d| d.history.undo_count()), Some(1));
    assert_eq!(top_elements(&s).len(), 2);
    // Fehler in der Gruppe nimmt alles zurück
    let r = s.run_group("kaputt", |s| {
        s.execute("draw.rectangle", json!({"rect": [0, 0, 5, 5]}))?;
        s.execute("gibt.esNicht", json!({}))?;
        Ok(())
    });
    assert!(r.is_err());
    assert_eq!(top_elements(&s).len(), 2);
    rect(&mut s, 20.0, 20.0, 30.0, 30.0);
    rect(&mut s, 40.0, 40.0, 50.0, 50.0);
    ex(&mut s, "history.goTo", json!({"position": 1}));
    assert_eq!(top_elements(&s).len(), 2);
    ex(&mut s, "history.goTo", json!({"position": 3}));
    assert_eq!(top_elements(&s).len(), 4);
}

#[test]
fn events_for_ui() {
    let mut s = Session::default();
    ex(&mut s, "file.new", json!({}));
    let ev = s.drain_events();
    assert!(ev.iter().any(|e| matches!(e, SessionEvent::DocumentOpened(_))));
    rect(&mut s, 0.0, 0.0, 5.0, 5.0);
    let ev = s.drain_events();
    assert!(ev.iter().any(|e| matches!(e, SessionEvent::DocumentChanged { generation: 1, .. })));
    assert!(ev.iter().any(|e| matches!(e, SessionEvent::HistoryChanged(_))));
    assert!(ev.iter().any(|e| matches!(e, SessionEvent::SelectionChanged(_))));
    ex(&mut s, "view.zoomIn", json!({}));
    assert!(s.drain_events().iter().any(|e| matches!(e, SessionEvent::ViewChanged(_))));
    assert!(s.drain_events().is_empty());
}
