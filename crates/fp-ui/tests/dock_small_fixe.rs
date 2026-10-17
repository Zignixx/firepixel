//! FIX-E (CT-3, niedrig): Bei 900 × 650 blieb die unterste Dock-Gruppe (Dokumentbibliothek /
//! Allgemeine Bibliothek) eine Reiterzeile: Reiterklick, *Fenster ▸ Dokumentbibliothek* klappten sie
//! nicht auf, weil das Dock bei zu wenig Höhe immer die unteren Gruppen einklappt. Jetzt bleibt die
//! zuletzt gezielt geöffnete Gruppe offen, andere klappen dafür zu.

use egui_kittest::Harness;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 200, "height": 150})).expect("doc");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 650.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

/// (Dock-Index, Index im Reiter) der Dokumentbibliothek.
fn library(h: &H) -> (usize, usize) {
    let ws = &h.state().state.settings.workspace;
    ws.dock.iter().enumerate().find_map(|(gi, g)| g.panels.iter().position(|p| *p == PanelId::DocumentLibrary).map(|i| (gi, i))).expect("Dokumentbibliothek im Dock")
}

#[test]
fn library_group_opens_by_tab_click_and_window_menu_at_900x650() {
    let mut h = harness();
    let (gi, idx) = library(&h);
    let shown_collapsed = |h: &H| h.state().state.dock.forced.contains(&gi) || h.state().state.settings.workspace.dock[gi].collapsed;
    assert!(shown_collapsed(&h), "Ausgangslage wie im CT-3: unterste Gruppe eingeklappt");
    // Reiterklick klappt auf (mehrmals geklickt: Wechsel auf/zu wie im Original, nie „hängt“)
    h.state_mut().state.push(Action::Run("ui.dock.tabClicked".into(), json!({"floating": null, "group": gi, "index": idx})));
    h.run_steps(3);
    assert!(!shown_collapsed(&h), "nach Klick offen: {:?}", h.state().state.dock.forced);
    assert!(!h.state().state.dock.forced.is_empty(), "dafür ist eine andere Gruppe eingeklappt");
    h.state_mut().state.push(Action::Run("ui.dock.tabClicked".into(), json!({"floating": null, "group": gi, "index": idx})));
    h.run_steps(3);
    assert!(h.state().state.settings.workspace.dock[gi].collapsed, "zweiter Klick auf den aktiven Reiter klappt zu");
    // Fenster ▸ Dokumentbibliothek öffnet sie wieder
    h.state_mut().state.push(Action::ShowPanel(PanelId::DocumentLibrary));
    h.run_steps(3);
    assert!(!h.state().state.settings.workspace.dock[gi].collapsed);
    assert!(!h.state().state.dock.forced.contains(&gi), "über das Menü offen");
    // eine andere Gruppe gezielt öffnen: die Bibliothek darf wieder zuklappen
    h.state_mut().state.push(Action::ShowPanel(PanelId::Layers));
    h.run_steps(3);
    let layers_gi = h.state().state.settings.workspace.dock.iter().position(|g| g.panels.contains(&PanelId::Layers)).expect("Ebenen im Dock");
    assert!(!h.state().state.dock.forced.contains(&layers_gi));
}
