//! MN-3: Bedienfeldmenüs (≡) wie im Original (PARITAET_B §3, Messung PM-7 in Lage S2 = ein
//! Rechteck gewählt, 1 Seite, 1 Status, keine Stile/Verlaufsschritte gewählt). Je Bedienfeld:
//! Einträge, Reihenfolge, Trennlinien, aktiv/grau und Häkchen in den Lagen „nichts gewählt“ und
//! „Objekt gewählt“; dazu das Reiter-Kontextmenü (Schließen) und das Eigenschaften-≡ („Hilfe“).

use fp_engine::Session;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::panels::PanelId;
use fp_ui::FirepixelApp;
use serde_json::json;

fn app(selected: bool) -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    // neue Dokumente exportieren als PNG 32 (BA-033); diese Tests prüfen die GIF-Ansicht
    s.execute("pi.export.settings", json!({"settings": serde_json::to_value(fp_model::ExportSettings::default()).expect("json")})).expect("GIF");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    if !selected {
        s.execute("select.none", json!({})).expect("none");
    }
    FirepixelApp::with_session(&ctx, s, UiSettings::default(), false)
}

/// Zeile je Eintrag wie in der Messung des Originals: „---“ für Trennlinien, sonst der Text mit
/// „ (grau)“ und „ (✓)“.
fn rows(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Separator => "---".to_string(),
            Node::Item { label, enabled, checked, .. } => {
                let mut s = label.clone();
                if !*enabled {
                    s.push_str(" (grau)");
                }
                if *checked == Some(true) {
                    s.push_str(" (✓)");
                }
                s
            }
            Node::Sub { label, .. } => format!("{label} ▸"),
        })
        .collect()
}

fn menu(a: &FirepixelApp, p: PanelId) -> Vec<String> {
    rows(&fp_ui::menu::build_panel_menu(&a.state, p))
}

fn check(a: &FirepixelApp, p: PanelId, want: &[&str]) {
    assert_eq!(menu(a, p), want, "Bedienfeldmenü {p:?}");
}

const HISTORY: &[&str] = &["Ausgewählte Schritte wiederholen (grau)", "Schritte kopieren (grau)", "Als Befehl speichern... (grau)", "---", "Verlauf löschen", "---", "Hilfe"];

#[test]
fn history_greys_step_commands_without_selected_steps() {
    for sel in [false, true] {
        check(&app(sel), PanelId::History, HISTORY);
    }
    // mit gewähltem Verlaufsschritt aktiv
    let mut a = app(true);
    a.state.dialogs.panel_sel.history.insert(0);
    check(&a, PanelId::History, &["Ausgewählte Schritte wiederholen", "Schritte kopieren", "Als Befehl speichern...", "---", "Verlauf löschen", "---", "Hilfe"]);
}

#[test]
fn styles_grey_without_selected_style() {
    // Original (S2): nur Neuer Stil, Nicht verwendete Stile auswählen, Stil laden,
    // Stilbibliothek importieren und Große Symbole aktiv
    let s2 = [
        "Neuer Stil...",
        "Stil duplizieren... (grau)",
        "Stile löschen (grau)",
        "Stil umbenennen... (grau)",
        "---",
        "Stil neu definieren (grau)",
        "---",
        "Abweichungen löschen (grau)",
        "Verknüpfung mit Stil aufheben (grau)",
        "Nicht verwendete Stile auswählen",
        "Stiloptionen... (grau)",
        "---",
        "Stil laden...",
        "---",
        "Stilbibliothek importieren...",
        "Stilbibliothek speichern (grau)",
        "---",
        "Große Symbole",
        "---",
        "Hilfe",
    ];
    check(&app(true), PanelId::Styles, &s2);
    // nichts gewählt: zusätzlich „Neuer Stil“ grau (braucht ein Objekt)
    let mut s0 = s2.to_vec();
    s0[0] = "Neuer Stil... (grau)";
    check(&app(false), PanelId::Styles, &s0);
    // mit gewähltem Stil: Duplizieren, Löschen, Umbenennen, Optionen, Bibliothek speichern aktiv
    let mut a = app(false);
    a.state.dialogs.panel_sel.styles = vec!["Stil 1".into()];
    let m = menu(&a, PanelId::Styles);
    for e in ["Stil duplizieren...", "Stile löschen", "Stil umbenennen...", "Stiloptionen...", "Stilbibliothek speichern"] {
        assert!(m.iter().any(|r| r == e), "{e} aktiv mit gewähltem Stil: {m:#?}");
    }
}

#[test]
fn states_with_one_state() {
    // Original (S2, 1 Status): Status löschen und Auf Status verteilen aktiv, In Status kopieren grau
    let s2 = [
        "Status hinzufügen...",
        "Status duplizieren...",
        "Status löschen",
        "---",
        "In Status kopieren... (grau)",
        "Auf Status verteilen",
        "---",
        "Autom. zuschneiden (✓)",
        "Nummerierung (✓)",
        "Auto-Differenz (✓)",
        "---",
        "Eigenschaften...",
        "---",
        "Hilfe",
    ];
    check(&app(true), PanelId::States, &s2);
    // nichts gewählt: nichts zu verteilen
    let mut s0 = s2.to_vec();
    s0[5] = "Auf Status verteilen (grau)";
    check(&app(false), PanelId::States, &s0);
    // mit zwei Status und gewähltem Objekt: In Status kopieren aktiv
    let mut a = app(true);
    a.state.session.execute("panel.states.duplicate", json!({"count": 1, "where": "end"})).expect("status");
    a.state.session.execute("select.all", json!({})).expect("all");
    assert!(menu(&a, PanelId::States).iter().any(|r| r == "In Status kopieren..."), "{:#?}", menu(&a, PanelId::States));
}

#[test]
fn last_state_and_page_delete_is_enabled_and_keeps_the_last_one() {
    let mut a = app(true);
    a.state.session.execute("panel.states.delete", json!({})).expect("Status löschen");
    a.state.session.execute("panel.pages.delete", json!({})).expect("Seite löschen");
    let d = a.state.session.active().expect("doc");
    assert_eq!(d.frame_count(), 1);
    assert_eq!(d.doc.pages.len(), 1);
    // mit zwei Status löscht der Befehl wie bisher
    a.state.session.execute("panel.states.add", json!({"count": 1, "where": "end"})).expect("status");
    assert_eq!(a.state.session.active().expect("doc").frame_count(), 2);
    a.state.session.execute("panel.states.delete", json!({})).expect("Status löschen");
    assert_eq!(a.state.session.active().expect("doc").frame_count(), 1);
}

#[test]
fn pages_with_one_page() {
    // Original (S2, 1 Seite): Seite löschen aktiv, Masterseiten-Befehle grau, ✓ Nummerierung
    let want = [
        "Neue Seite",
        "Seite duplizieren",
        "Seite löschen",
        "Seite umbenennen",
        "Mit Masterseite verknüpfen (grau)",
        "---",
        "Als Masterseite festlegen",
        "Masterseite zurücksetzen (grau)",
        "---",
        "Ausgewählte Seite(n) exportieren...",
        "---",
        "Nummerierung (✓)",
        "Optionen für Miniaturansicht...",
        "---",
        "Hilfe",
    ];
    for sel in [false, true] {
        check(&app(sel), PanelId::Pages, &want);
    }
}

#[test]
fn layers_edit_mask_needs_a_mask() {
    // Original (S2): 27 Einträge, Maske bearbeiten/deaktivieren/löschen grau ohne Maske
    let s2 = [
        "Neues Bild",
        "---",
        "Neue Ebene...",
        "Neue Unterebene...",
        "Ebene duplizieren...",
        "Ebene statusübergreifend freigeben",
        "Einzelebene bearbeiten",
        "Ebene löschen",
        "---",
        "Masterseitenebene hinzufügen (grau)",
        "Ebene für Seiten freigeben...",
        "Ebene für alle Seiten freigeben",
        "Freigegebene Ebene ausschließen (grau)",
        "Freigegebene Ebene lösen (grau)",
        "---",
        "Auswahl auf Hintergrund reduzieren",
        "Mit darunter liegender auf eine Ebene reduzieren (grau)",
        "---",
        "Alles ausblenden",
        "Alles anzeigen",
        "Alles sperren",
        "Alle Sperrungen aufheben",
        "---",
        "Maske hinzufügen",
        "Maske bearbeiten (grau)",
        "Maske deaktivieren (grau)",
        "Maske löschen (grau)",
        "---",
        "Miniaturoptionen...",
        "---",
        "Hilfe",
    ];
    check(&app(true), PanelId::Layers, &s2);
    // nichts gewählt: zusätzlich Reduzieren und Maske hinzufügen grau
    let mut s0 = s2.to_vec();
    s0[15] = "Auswahl auf Hintergrund reduzieren (grau)";
    s0[23] = "Maske hinzufügen (grau)";
    check(&app(false), PanelId::Layers, &s0);
    // mit Maske: Maske bearbeiten aktiv
    let mut a = app(true);
    a.state.session.execute("panel.layers.addMask", json!({})).expect("Maske");
    let m = menu(&a, PanelId::Layers);
    assert!(m.iter().any(|r| r == "Maske bearbeiten"), "{m:#?}");
}

fn optimize_png32(selected: bool) -> FirepixelApp {
    let mut a = app(selected);
    set_format(&mut a, "png32");
    a
}

#[test]
fn optimize_like_the_original_with_png32() {
    // Original (S2, PNG32): JPEG- und Palettenbefehle grau, „Farbfeld-Rückmeldung anzeigen“ ✓
    let want = [
        "Einstellungen speichern...",
        "Einstellungen löschen",
        "---",
        "Optimieren für Dateigröße...",
        "Exportassistent...",
        "---",
        "Mit Zeilensprung",
        "Progressives JPEG (grau)",
        "JPEG-Ränder scharfzeichnen (grau)",
        "---",
        "Sperrung aller Farben aufheben (grau)",
        "Nicht verwendete Farben entfernen (grau)",
        "Paletteneintrag ersetzen... (grau)",
        "Transparent machen (grau)",
        "---",
        "Farbfeld-Rückmeldung anzeigen (✓)",
        "---",
        "Bearbeitung entfernen (grau)",
        "Alle Bearbeitungen entfernen (grau)",
        "---",
        "Palette laden...",
        "Palette speichern...",
        "---",
        "Hilfe",
    ];
    for sel in [false, true] {
        check(&optimize_png32(sel), PanelId::Optimize, &want);
    }
}

#[test]
fn optimize_jpeg_and_palette_entries_follow_the_format() {
    // GIF (Vorgabe): JPEG-Einträge grau und ohne Häkchen, Palettenbefehle aktiv; Farbbefehle erst
    // mit gewählter Palettenfarbe
    let mut a = app(true);
    let m = menu(&a, PanelId::Optimize);
    for e in ["Progressives JPEG (grau)", "JPEG-Ränder scharfzeichnen (grau)", "Sperrung aller Farben aufheben", "Nicht verwendete Farben entfernen (✓)", "Paletteneintrag ersetzen... (grau)", "Transparent machen (grau)", "Bearbeitung entfernen (grau)"] {
        assert!(m.iter().any(|r| r == e), "GIF: {e} erwartet in {m:#?}");
    }
    a.state.dialogs.panel_sel.palette = vec![0xff00_0000];
    let m = menu(&a, PanelId::Optimize);
    for e in ["Paletteneintrag ersetzen...", "Transparent machen", "Bearbeitung entfernen"] {
        assert!(m.iter().any(|r| r == e), "GIF mit Farbe: {e} erwartet in {m:#?}");
    }
    // JPEG: beide JPEG-Einträge aktiv, Palettenbefehle grau
    set_format(&mut a, "jpeg");
    let m = menu(&a, PanelId::Optimize);
    assert!(m.iter().any(|r| r.starts_with("Progressives JPEG") && !r.contains("grau")), "{m:#?}");
    assert!(m.iter().any(|r| r.starts_with("JPEG-Ränder scharfzeichnen") && !r.contains("grau")), "{m:#?}");
    assert!(m.iter().any(|r| r == "Nicht verwendete Farben entfernen (grau)"), "{m:#?}");
    // Skripte dürfen die JPEG-Option weiter setzen, auch bei anderem Format
    set_format(&mut a, "png32");
    a.state.session.execute("panel.optimize.progressive", json!({"on": true})).expect("progressive per Skript");
}

#[test]
fn panel_menus_end_with_help_and_close_is_in_the_tab_menu() {
    let a = app(true);
    for p in PanelId::ALL {
        let m = menu(&a, *p);
        assert_eq!(m.last().map(String::as_str), Some("Hilfe"), "{p:?}: {m:#?}");
        assert!(!m.iter().any(|r| r == "Schließen" || r == "Registerkartengruppe schließen"), "{p:?}: Schließen gehört ins Reiter-Kontextmenü");
        let tab = fp_ui::menu::build_tab_menu(&a.state, *p);
        assert_eq!(rows(&tab), ["Schließen", "Registerkartengruppe schließen"], "{p:?}");
        let Node::Item { action: ItemAction::Ui(Action::TogglePanel(q)), .. } = &tab[0] else { panic!("Schließen ohne Aktion") };
        assert_eq!(q, p);
    }
}

#[test]
fn properties_menu_is_only_help() {
    let a = app(true);
    assert_eq!(rows(&fp_ui::menu::build_inspector_menu(&a.state)), ["Hilfe"]);
    assert_eq!(rows(&fp_ui::menu::build_inspector_tab_menu(&a.state)), ["Volle Höhe (✓)", "Halbe Höhe", "---", "Schließen"]);
}

#[test]
fn english_labels() {
    fp_i18n::set_language("en");
    let a = app(true);
    let m = menu(&a, PanelId::History);
    assert_eq!(m.last().map(String::as_str), Some("Help"));
    assert_eq!(rows(&fp_ui::menu::build_tab_menu(&a.state, PanelId::Layers)), ["Close", "Close Tab Group"]);
    fp_i18n::set_language("de");
}

/// Exportformat der Seite setzen (wie „Übernehmen“ in der Bildvorschau).
fn set_format(a: &mut FirepixelApp, format: &str) {
    a.state.session.execute("file.imagePreview", json!({"settings": {"format": format}, "apply": true})).expect(format);
}
