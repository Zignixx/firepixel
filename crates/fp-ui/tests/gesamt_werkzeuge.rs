//! Gesamttest der Werkzeuge (HG, Phase 5): jedes der 49 Werkzeuge wird wie in der Oberfläche
//! gewählt (`Action::SetTool`, Werkzeugbedienfeld/Kürzel) und mit typischen Mausereignissen auf dem
//! Testdokument bedient (`Session::tool_event`, derselbe Weg wie die Leinwand). Geprüft wird die
//! Wirkung (neues/geändertes Objekt mit Verlaufsschritt und exaktem Rückgängig, Pixelauswahl,
//! Auswahl, Farbe, Ansicht) und dass das Werkzeug ein Icon und einen Platz im Werkzeugbedienfeld
//! hat. Ergebnis: `docs\funktionstest\werkzeuge_auto.md`.

use crate::gesamt_common;

use fp_engine::tools::{ToolEvent, ToolKey, TOOLS};
use fp_engine::ToolId;
use fp_ui::app_state::Action;
use gesamt_common::*;
use std::panic::AssertUnwindSafe;

/// Testablauf je Werkzeug: Auswahl vorher (IDs aus dem Testdokument) und Ereignisse.
fn scenario(t: ToolId, f: &Fix) -> (Vec<u64>, Vec<ToolEvent>) {
    use ToolEvent as E;
    let drag = |a: (f32, f32), b: (f32, f32)| vec![E::down(a.0, a.1), E::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), E::drag(b.0, b.1), E::up(b.0, b.1)];
    let click = |x: f32, y: f32| vec![E::down(x, y), E::up(x, y)];
    let empty = vec![];
    match t {
        ToolId::Pointer => (empty, drag((100.0, 100.0), (130.0, 120.0))),
        ToolId::SelectBehind => (empty, click(100.0, 100.0)),
        ToolId::Subselection => (f.group.clone(), click(625.0, 65.0)),
        ToolId::Marquee | ToolId::OvalMarquee => (f.bitmap.clone(), drag((450.0, 50.0), (520.0, 130.0))),
        // Lasso: geschlossene Freihandform (nicht auf einer Geraden)
        ToolId::Lasso => (f.bitmap.clone(), vec![E::down(450.0, 50.0), E::drag(520.0, 60.0), E::drag(510.0, 130.0), E::drag(455.0, 120.0), E::up(455.0, 120.0)]),
        ToolId::PolygonLasso => (f.bitmap.clone(), [click(450.0, 50.0), click(520.0, 60.0), vec![E::double(500.0, 130.0)]].concat()),
        ToolId::Crop | ToolId::ExportArea => (empty, [drag((20.0, 20.0), (420.0, 300.0)), vec![E::key(ToolKey::Enter)]].concat()),
        ToolId::MagicWand => (f.bitmap.clone(), click(500.0, 120.0)),
        ToolId::Line | ToolId::Rectangle | ToolId::Ellipse | ToolId::Polygon | ToolId::AutoShape | ToolId::VectorPath | ToolId::Slice | ToolId::HotspotRect | ToolId::HotspotCircle => {
            (empty, drag((300.0, 280.0), (420.0, 380.0)))
        }
        ToolId::Pen => (empty, [click(300.0, 280.0), click(420.0, 280.0), click(420.0, 380.0), vec![E::key(ToolKey::Enter)]].concat()),
        ToolId::HotspotPolygon | ToolId::PolygonSlice => (empty, [click(300.0, 280.0), click(420.0, 280.0), click(420.0, 380.0), click(300.0, 280.0)].concat()),
        ToolId::Text => (empty, [click(300.0, 300.0), vec![E::text("Hallo"), E::key(ToolKey::Escape)]].concat()),
        ToolId::Pencil | ToolId::Brush | ToolId::Eraser | ToolId::Blur | ToolId::Sharpen | ToolId::Dodge | ToolId::Burn | ToolId::Smudge | ToolId::ReplaceColor | ToolId::RedEyeRemoval => {
            (f.bitmap.clone(), drag((450.0, 60.0), (540.0, 140.0)))
        }
        ToolId::RubberStamp => (f.bitmap.clone(), [vec![E::down(450.0, 60.0).alt(), E::up(450.0, 60.0).alt()], drag((500.0, 100.0), (540.0, 140.0))].concat()),
        ToolId::RedrawPath => (f.path.clone(), drag((40.0, 200.0), (200.0, 300.0))),
        // Transformieren bleibt wie im Original Vorschau, bis Eingabe (bzw. Doppelklick/Werkzeugwechsel) übernimmt
        ToolId::Scale | ToolId::Skew | ToolId::Distort => (f.rect.clone(), [drag((200.0, 160.0), (240.0, 200.0)), vec![E::key(ToolKey::Enter)]].concat()),
        ToolId::NineSliceScale => (f.instance.clone(), [drag((140.0, 480.0), (180.0, 520.0)), vec![E::key(ToolKey::Enter)]].concat()),
        ToolId::Freeform | ToolId::Reshape | ToolId::PathScrubAdd | ToolId::PathScrubSubtract => (f.path.clone(), drag((170.0, 230.0), (180.0, 210.0))),
        ToolId::Knife => (f.rect.clone(), drag((120.0, 20.0), (120.0, 180.0))),
        ToolId::Eyedropper => (empty, click(100.0, 100.0)),
        ToolId::PaintBucket => (f.rect.clone(), click(100.0, 100.0)),
        ToolId::Gradient => (f.rect.clone(), drag((50.0, 100.0), (190.0, 100.0))),
        ToolId::Hand => (empty, drag((300.0, 300.0), (200.0, 200.0))),
        ToolId::Zoom => (empty, click(300.0, 300.0)),
        // KI-Auswahl (Agent KI): Klick auf die Bitmap; die Oberfläche rechnet im Hintergrund
        ToolId::AiObject | ToolId::AiSubject | ToolId::AiRefineEdge => (f.bitmap.clone(), click(500.0, 120.0)),
    }
}

fn run_tool(t: ToolId) -> (Outcome, String) {
    let (mut st, ctx, f) = fixture();
    let (mut sel_ids, mut events) = scenario(t, &f);
    // KI-Motivauswahl/Kante verfeinern: Bitmap mit Motiv (Hund auf Gras) statt der einfarbigen Fläche
    if matches!(t, ToolId::AiSubject | ToolId::AiRefineEdge) {
        let img = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("fp-ai").join("testdata").join("hund_gras.jpg");
        let _ = st.session.execute("file.import", serde_json::json!({ "path": img.to_string_lossy(), "at": [0, 0] }));
        sel_ids = sel(&st);
        let b = sel_ids.first().and_then(|id| {
            let d = st.session.active()?;
            let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(*id))?;
            fp_engine::docops::element_bounds(&d.doc, &e)
        });
        if let Some(b) = b {
            let (cx, cy) = ((b.left + b.right) / 2.0, (b.top + b.bottom) / 2.0);
            if t == ToolId::AiRefineEdge {
                let r = [cx - b.width() * 0.3, cy - b.height() * 0.3, cx + b.width() * 0.3, cy + b.height() * 0.3];
                select(&mut st, &sel_ids);
                let _ = st.session.execute("select.pixels", serde_json::json!({ "shape": "ellipse", "rect": r }));
            }
            events = vec![ToolEvent::down(cx, cy), ToolEvent::up(cx, cy)];
        }
    }
    if t != ToolId::AiRefineEdge {
        select(&mut st, &sel_ids);
    }
    // Farbe des Werkzeugbedienfelds für Pipette/Farbeimer unterscheidbar machen
    st.session.set_colors(Some(fp_color::Argb(0xff00_ff00)), Some(fp_color::Argb(0xff12_3456)));
    st.push(Action::SetTool(t));
    st.process_actions(&ctx);
    let _ = st.session.drain_events();
    if st.session.tools.active != t {
        return (Outcome::Error("Werkzeug nicht wählbar".into()), format!("aktiv: {:?}", st.session.tools.active));
    }
    let before = probe(&st);
    let res = std::panic::catch_unwind(AssertUnwindSafe(move || {
        let mut errs = Vec::new();
        for ev in &events {
            match st.session.tool_event(ev) {
                // Anfragen an die Oberfläche (z. B. Export-Dialog des Exportbereichs) wie die Leinwand weiterreichen
                Ok(r) => {
                    if let Some(req) = r.ui_request {
                        fp_ui::canvas::ui_request(&mut st, &req);
                    }
                }
                Err(e) => errs.push(e.to_string()),
            }
        }
        st.process_actions(&ctx);
        (st, errs)
    }));
    let (mut st, errs) = match res {
        Ok(x) => x,
        Err(e) => return (Outcome::Panic(panic_msg(&e)), String::new()),
    };
    if let Some(e) = errs.first() {
        st.status = Some((e.clone(), 0.0));
    }
    judge(&mut st, &before, "Ziehen/Klicken")
}

#[test]
fn gesamt_werkzeuge() {
    let mut rows = Vec::new();
    for info in TOOLS {
        let icon_ok = std::panic::catch_unwind(|| fp_ui::icons::tool_icon(info.id)).is_ok();
        let (mut o, mut note) = run_tool(info.id);
        if !icon_ok {
            o = Outcome::Error("kein Icon".into());
        }
        if !info.implemented {
            note = format!("{note} (in fp-engine als nicht umgesetzt markiert)");
        }
        rows.push(Row { path: format!("{} ▸ {}", info.section, info.label), command: info.name.to_string(), outcome: o, note });
    }
    let (ok, n) = write_report("werkzeuge_auto.md", "Werkzeuge", "gesamt_werkzeuge.rs", &rows);
    eprintln!("Werkzeuge: {ok}/{n} ✅");
    let panics: Vec<_> = rows.iter().filter(|r| matches!(r.outcome, Outcome::Panic(_))).map(|r| format!("{} {:?}", r.path, r.outcome)).collect();
    assert!(panics.is_empty(), "Paniken: {panics:#?}");
}
