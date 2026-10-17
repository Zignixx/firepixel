//! UE-3: Befunde I18N-SHELL – Fenster-Menü-Häkchen der Bedienfeld-Befehle wie im Original nur für
//! das vorderste Bedienfeld seiner sichtbaren Gruppe; Seitenname neuer Dokumente in der Sprache der Oberfläche.

use fp_engine::Session;
use fp_ui::actions::ui_command_state;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::panels::PanelId;

#[test]
fn haekchen_nur_fuer_das_vorderste_bedienfeld() {
    let mut st = AppState::new(Session::default(), UiSettings::default(), None);
    let ws = &st.settings.workspace;
    // eine angedockte Gruppe mit mindestens zwei sichtbaren Bedienfeldern (Reiter)
    let group = ws.dock.iter().find(|g| ws.visible_panels(g).iter().filter(|p| p.engine_command().is_some()).count() >= 2).cloned().expect("Gruppe mit Reitern");
    let panels: Vec<PanelId> = ws.visible_panels(&group).into_iter().filter(|p| p.engine_command().is_some()).collect();
    let front = panels.iter().copied().find(|p| ws.is_front(*p)).expect("vorderes Bedienfeld");
    let back = panels.iter().copied().find(|p| !ws.is_front(*p)).expect("hinteres Bedienfeld");
    let state = |st: &AppState, p: PanelId| ui_command_state(st, p.engine_command().unwrap_or_default());
    assert_eq!(state(&st, front), Some((true, Some(true))), "{front:?} vorn: Häkchen");
    assert_eq!(state(&st, back), Some((true, Some(false))), "{back:?} hinten: kein Häkchen");
    // nach vorn holen (Fenster-Menü) → Häkchen wechselt
    st.settings.workspace.show(back);
    assert_eq!(state(&st, back), Some((true, Some(true))));
    assert_eq!(state(&st, front), Some((true, Some(false))));
    // geschlossen → kein Häkchen
    st.settings.workspace.toggle(back);
    assert_eq!(state(&st, back), Some((true, Some(false))));
}

/// Befund I18N-QA: „Seite 1“ war in der Testsprache unmarkiert. Neue Dokumente und neue Seiten
/// bekommen den Namen in der Sprache der Oberfläche (`msg-default-page`, UE-1); gespeicherte Namen
/// bleiben Dokumentdaten.
#[test]
fn seitenname_neuer_dokumente_folgt_der_sprache() {
    let name_after_new = |lang: &str| {
        fp_i18n::with_language(lang, || {
            let mut s = Session::default();
            s.execute("file.new", serde_json::json!({"width": 100, "height": 80})).expect("file.new");
            s.active().and_then(|d| d.doc.pages.first().map(|p| p.display_name().to_string())).unwrap_or_default()
        })
    };
    assert_eq!(name_after_new("de"), "Seite 1");
    assert_eq!(name_after_new("en"), "Page 1");
    let qps = name_after_new("qps");
    assert!(qps.starts_with("[!!") && qps.contains("Seite 1"), "Testsprache markiert: {qps}");
}
