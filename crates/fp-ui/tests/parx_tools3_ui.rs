//! PAR-X-TOOLS-3 (Oberfläche): Doppelklick in den Exportbereich öffnet wie im Original die
//! Bildvorschau mit Register „Datei“ und gesetztem Exportbereich (PAR 08/20); Zuschneiden- und
//! Rechteck-Rahmen erscheinen live im Inspektor (PAR 08/17, 10/15); der Zeiger-Zug eines eben
//! gewählten Objekts lässt den Inspektor beim Dokument (PAR 08/21).
//! Wie alle Testläufe mit `FP_NO_SHELL_OPEN=1` starten (AGENTEN_REGELN §4a).

use fp_engine::{Session, ToolEvent, ToolId};
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

fn state() -> AppState {
    fp_engine::shell::disable();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 800, "height": 600})).expect("neu");
    AppState::new(s, UiSettings::default(), None)
}

fn drag(st: &mut AppState, a: (f32, f32), b: (f32, f32)) -> Option<fp_engine::tools::Invocation> {
    let mut req = None;
    let mut evs = vec![ToolEvent::down(a.0, a.1)];
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        evs.push(ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
    }
    evs.push(ToolEvent::up(b.0, b.1));
    for e in evs {
        if let Ok(r) = st.session.tool_event(&e)
            && r.ui_request.is_some()
        {
            req = r.ui_request;
        }
    }
    req
}

#[test]
fn export_area_double_click_opens_image_preview() {
    let mut st = state();
    st.session.set_tool(ToolId::ExportArea).expect("Werkzeug");
    drag(&mut st, (50.0, 50.0), (300.0, 200.0));
    // live im Inspektor (B 250 H 150 X 50 Y 50)
    assert_eq!(st.session.tool_live_rect().map(|r| (r.left, r.top, r.width(), r.height())), Some((50.0, 50.0, 250.0, 150.0)));
    let r = st.session.tool_event(&ToolEvent::double(100.0, 100.0)).expect("Doppelklick");
    let req = r.ui_request.expect("Anfrage an die Oberfläche");
    fp_ui::canvas::ui_request(&mut st, &req);
    let p = st.dialogs.preview.as_ref().expect("Bildvorschau offen");
    assert_eq!(p.tab, 1, "Register Datei");
    assert_eq!(p.panes[0].settings.crop, Some([50, 50, 300, 200]), "Exportbereich X 50 Y 50 B 250 H 150");
    // Dokument unverändert, kein Verlaufsschritt
    assert_eq!(st.session.active().map(|d| d.history.undo_count()), Some(0));
}

#[test]
fn inspector_holds_document_while_dragging_fresh_selection() {
    let mut st = state();
    st.session.execute("draw.rectangle", json!({"rect": [50, 50, 150, 120]})).expect("R1");
    st.session.execute("select.none", json!({})).expect("abwählen");
    let mut ins = fp_ui::inspector::InspectorState::default();
    ins.update(&st.session);
    let doc_layout = ins.clone();
    st.session.set_tool(ToolId::Pointer).expect("Zeiger");
    for e in [ToolEvent::down(100.0, 85.0), ToolEvent::drag(110.0, 90.0), ToolEvent::drag(130.0, 95.0)] {
        st.session.tool_event(&e).expect("Ereignis");
    }
    ins.update(&st.session);
    assert_eq!(ins, doc_layout, "mitten im Zug noch „Dokument“");
    st.session.tool_event(&ToolEvent::up(130.0, 95.0)).expect("los");
    ins.update(&st.session);
    assert_ne!(ins, doc_layout, "danach das Objekt");
}
