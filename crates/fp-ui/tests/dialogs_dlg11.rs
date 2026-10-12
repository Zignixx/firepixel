//! DLG-11: Datei ▸ Datei-Info (D10) in der gemessenen Fenstergröße (außen 680 × 620, Client
//! ≈ 664 × 581) mit dem Dateinamen des Dokuments als Fenstertitel; Felder, Startfokus und
//! Verlauf wie DLG-10. Alle Bedienelemente liegen innerhalb des Fensters, auch in Englisch.

use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    s
}

fn harness(s: Session) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn open(h: &mut H) {
    h.state_mut().state.run("file.fileInfo", json!({}));
    h.run_steps(3);
    assert!(st(h).dialogs.file_info.is_some(), "Datei-Info-Fenster offen");
}

fn doc_name(h: &mut H) -> String {
    h.state_mut().state.session.execute("file.fileInfo", json!({})).expect("info").value["title"].as_str().expect("Titel").to_string()
}

#[test]
fn file_info_has_measured_size() {
    let mut h = harness(session());
    open(&mut h);
    let g = fp_ui::dialogs::dialog_geom(&h.ctx, "fp-file-info").expect("Lage");
    assert_eq!(g.client, egui::vec2(664.0, 581.0), "Client wie gemessen (außen 680 × 620)");
    assert_eq!(fp_ui::dialogs::file_info::SIZE, [664.0, 581.0]);
}

#[test]
fn file_info_title_is_document_name() {
    let mut h = harness(session());
    let name = doc_name(&mut h);
    assert!(!name.is_empty());
    open(&mut h);
    let bar = fp_i18n::tr!("dlg-titlebar", title = name.as_str());
    h.get_by_label(&bar);
    assert_eq!(fp_ui::dialogs::file_info::window_title(&json!({"title": name})), name);
    // ohne Namen der allgemeine Titel
    assert_eq!(fp_ui::dialogs::file_info::window_title(&json!({})), fp_i18n::t("dlg-fileinfo-window-title"));
    assert_eq!(fp_ui::dialogs::file_info::window_title(&json!({"title": "  "})), fp_i18n::t("dlg-fileinfo-window-title"));
}

#[test]
fn file_info_controls_inside_window_de_en() {
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut h = harness(session());
        open(&mut h);
        let g = fp_ui::dialogs::dialog_geom(&h.ctx, "fp-file-info").expect("Lage");
        let win = egui::Rect::from_min_size(g.pos, g.client + egui::vec2(2.0, 2.0 + fp_ui::dialogs::DIALOG_TITLE_H));
        // Eingabefelder des Fensters (Mittelpunkt im Fenster; die Statuszeile darunter zählt nicht)
        let mut n = 0;
        for role in [egui::accesskit::Role::TextInput, egui::accesskit::Role::MultilineTextInput] {
            for node in h.query_all_by_role(role) {
                let r = node.rect();
                if win.contains(r.center()) {
                    n += 1;
                    assert!(win.expand(0.5).contains_rect(r), "{lang}: {role:?} {r:?} ragt aus {win:?}");
                }
            }
        }
        assert_eq!(n, 5, "{lang}: Titel, Autor, Beschreibung, Schlüsselwörter, Copyright");
        for key in ["btn-ok", "btn-cancel"] {
            let r = h.get_by_label(&fp_i18n::t(key)).rect();
            assert!(win.expand(0.5).contains_rect(r), "{lang}: {key} {r:?} ragt aus {win:?}");
        }
    }
}

#[test]
fn file_info_focus_ok_and_single_step_in_new_size() {
    let mut h = harness(session());
    let before = st(&h).session.active().expect("doc").history.undo_count();
    open(&mut h);
    // Startfokus im Titelfeld
    h.event(egui::Event::Text("Plakat".into()));
    h.run_steps(2);
    assert_eq!(st(&h).dialogs.file_info.as_ref().expect("offen").title, "Plakat");
    h.get_by_label(&fp_i18n::t("btn-ok")).click();
    h.run_steps(4);
    assert!(st(&h).dialogs.file_info.is_none());
    assert_eq!(st(&h).session.active().expect("doc").history.undo_count(), before + 1, "ein Verlaufsschritt");
    let m = h.state_mut().state.session.execute("file.fileInfo", json!({})).expect("info").value["metadata"].clone();
    assert_eq!(m["title"], "Plakat");
}
