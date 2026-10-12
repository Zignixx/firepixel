//! P-DLG4: Neues Symbol (D14), Gespeicherte Befehle verwalten (D25), Voreinstellungen (D16) und
//! Tastaturkurzbefehle (D17) wie im Original (Befunde PM-5, PARITAET_B §2). Bedienung über echte
//! Tasten und Klicks im Fenster (kittest).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::{general_group_titles, PREFERENCE_CATEGORIES as CATEGORIES};
use fp_ui::dialogs::shortcuts::{rows, tree_lines, TreeLine};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    // keine Fremdprogramme (Hilfe) und nie das echte Nutzerprofil (Befehlsordner)
    fp_engine::shell::disable();
    crate::gesamt_common::test_profile();
    let mut s = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    s.execute("file.new", json!({"width": 500, "height": 400, "background": "#ffffff"})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80], "name": "R1"})).expect("R1");
    s.execute("select.none", json!({})).expect("none");
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

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: Modifiers::NONE });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn run(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(3);
}

fn dialog_rect(h: &H) -> Rect {
    fp_ui::dialogs::dialog_rect(&h.ctx, &fp_ui::dialogs::last_dialog_id(&h.ctx).expect("Dialog")).expect("Rechteck")
}

/// Mittelpunkt des Bedienelements mit dieser Beschriftung im obersten Dialog.
fn in_dialog(h: &H, role: Role, label: &str) -> Pos2 {
    let dr = dialog_rect(h);
    h.query_all(By::new().role(role)).filter(|n| n.accesskit_node().label().as_deref() == Some(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("„{label}“ ({role:?}) nicht im Dialog")).center()
}

/// Wie [`in_dialog`], jede Rolle (Zeilen der Listen).
fn in_dialog_any(h: &H, label: &str) -> Pos2 {
    let dr = dialog_rect(h);
    h.get_all(By::new()).filter(|n| n.accesskit_node().label().as_deref() == Some(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).unwrap_or_else(|| panic!("„{label}“ nicht im Dialog")).center()
}

// ------------------------------------------------------------------ D14 Neues Symbol

#[test]
fn d14_new_symbol_ok_opens_symbol_editing() {
    let mut h = harness();
    let parent = st(&h).session.active_id().expect("Dokument");
    let elems = |h: &H, id| h.state().state.session.document(id).map(|d| fp_engine::docops::own_layers(&d.doc, d.page).iter().map(|(_, l)| l.cels.iter().map(|c| c.elements.len()).sum::<usize>()).sum::<usize>()).unwrap_or(0);
    let n0 = elems(&h, parent);
    run(&mut h, "edit.insert.newSymbol");
    let f = st(&h).dialogs.form.as_ref().expect("Dialog offen");
    // Dialog wie „In Symbol konvertieren“: Name, Typ Grafik/Animation/Schaltfläche, 9-Slice
    assert_eq!(f.title, fp_i18n::t("dlg-title-convert-to-symbol"));
    let keys: Vec<&str> = f.fields.iter().map(|x| x.key.as_str()).collect();
    assert_eq!(keys, ["name", "kind", "nineSlice"]);
    // Typ „Schaltfläche“ per Klick, dann Enter = OK
    let p = in_dialog(&h, Role::RadioButton, &fp_i18n::t("dlg-symbol-button"));
    click(&mut h, p);
    key(&mut h, Key::Enter);
    h.run_steps(4);
    assert!(!st(&h).dialogs.any_open(), "Dialog nach OK zu");
    let view = st(&h).session.active().expect("Ansicht");
    assert_ne!(view.id, parent, "Symbolbearbeitung ist aktiv");
    let link = view.symbol_edit.clone().expect("Symbolbearbeitung");
    assert_eq!(link.parent, parent);
    let doc = &st(&h).session.document(parent).expect("Eltern").doc;
    assert_eq!(doc.symbols.len(), 1, "leeres Symbol in der Bibliothek");
    assert_eq!(doc.symbols[0].name, fp_i18n::t("dlg-symbol-default-name"));
    assert_eq!(doc.symbols[0].kind(), fp_model::SymbolKind::Button);
    assert_eq!(elems(&h, parent), n0, "keine Instanz im Dokument (früher leere Instanz bei 0,0)");
    // Fertig: zurück zum Dokument, ein Rückgängig entfernt das Symbol wieder
    run(&mut h, "modify.symbol.exitEdit");
    assert_eq!(st(&h).session.active_id(), Some(parent));
    run(&mut h, "edit.undo");
    assert!(st(&h).session.active().is_some_and(|d| d.doc.symbols.is_empty()), "Neues Symbol ist ein Verlaufsschritt");
}

#[test]
fn d14_engine_without_edit_keeps_old_behaviour() {
    let mut s = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    s.execute("file.new", json!({"width": 100, "height": 100})).expect("doc");
    s.execute("edit.insert.newSymbol", json!({"name": "A"})).expect("A");
    assert_eq!(s.active().map(|d| (d.doc.symbols.len(), d.symbol_edit.is_some())), Some((1, false)));
    let v = s.execute("edit.insert.newSymbol", json!({"name": "B", "kind": "animation", "edit": true, "nineSlice": true})).expect("B").value;
    assert!(v["doc"].as_u64().is_some());
    assert!(s.active().is_some_and(|d| d.symbol_edit.is_some()), "edit öffnet die Bearbeitung");
}

// ------------------------------------------------------------------ D25 Gespeicherte Befehle

#[test]
fn d25_message_without_own_commands_list_with_own() {
    let mut h = harness();
    // mitgelieferte Befehle-Menü-Skripte liegen im Befehlsordner, zählen aber nicht
    let dir = fp_engine::commands::misc::commands_dir();
    assert!(dir.join(format!("{}.jsf", "Warndialoge zurücksetzen")).exists(), "mitgelieferter Befehl vorhanden");
    let own = format!("DLG4 Testbefehl {}", std::process::id());
    let _ = std::fs::remove_file(dir.join(format!("{own}.jsf")));
    run(&mut h, "commands.manageSavedCommands");
    let (title, text) = st(&h).dialogs.message.clone().expect("Meldung statt Liste");
    assert_eq!(title, "Firepixel");
    assert_eq!(text, "Befehle werden dem Verlauf-Bedienfeld entnommen. Zurzeit sind dort keine gespeicherten Befehle vorhanden.");
    assert!(st(&h).dialogs.saved_commands.is_none());
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert!(!st(&h).dialogs.any_open(), "Enter schließt die Meldung");
    // mit eigenem gespeichertem Befehl: Verwaltungsliste nur mit eigenen Befehlen
    h.state_mut().state.session.execute("commands.manageSavedCommands", json!({"action": "save", "name": own, "code": "fw.getDocumentDOM().selectAll();\n"})).expect("speichern");
    run(&mut h, "commands.manageSavedCommands");
    let names = st(&h).dialogs.saved_commands.as_ref().map(|d| d.names.clone());
    assert!(st(&h).dialogs.message.is_none());
    let _ = std::fs::remove_file(dir.join(format!("{own}.jsf")));
    assert_eq!(names, Some(vec![own.clone()]), "nur eigene Befehle");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(!st(&h).dialogs.any_open());
    // Liste über die Engine (MCP) ebenso ohne mitgelieferte Skripte
    let v = h.state_mut().state.session.execute("commands.manageSavedCommands", json!({})).expect("list").value;
    assert!(v["commands"].as_array().is_some_and(|a| !a.iter().any(|x| x == "Warndialoge zurücksetzen" || x == "Größe ausgewählter Objekte ändern")));
}

// ------------------------------------------------------------------ D16 Voreinstellungen

#[test]
fn d16_categories_groups_and_help() {
    let names: Vec<String> = CATEGORIES.iter().map(|k| fp_i18n::t(k)).collect();
    assert_eq!(names, ["Allgemein", "Bearbeitung", "Hilfslinien und Raster", "Text", "Photoshop-Datei importieren", "Starten und Bearbeiten", "Zusatzmodule"]);
    assert_eq!(general_group_titles(), ["Speichern", "Benutzeroberfläche", "Standardfarben"]);
    let mut h = harness();
    run(&mut h, "edit.preferences");
    assert!(st(&h).dialogs.preferences.is_some());
    // Kategorieliste: Ende wählt „Zusatzmodule“ (letzter Eintrag wie im Original)
    key(&mut h, Key::End);
    assert_eq!(st(&h).dialogs.preferences.as_ref().map(|d| d.category), Some(6));
    key(&mut h, Key::Home);
    h.run_steps(2);
    // Gruppe „Dokumentoptionen“: Kästchen bedienbar
    let p = in_dialog(&h, Role::CheckBox, &fp_i18n::t("dlg-pref-showstartpage"));
    click(&mut h, p);
    assert!(st(&h).dialogs.preferences.as_ref().is_some_and(|d| d.values.contains_key("ShowStartPage")));
    // Hilfe: Knopf vorhanden, Dialog bleibt offen
    let p = in_dialog(&h, Role::Button, "Hilfe");
    click(&mut h, p);
    h.run_steps(2);
    assert!(st(&h).dialogs.preferences.is_some(), "Hilfe schließt den Dialog nicht");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.preferences.is_none());
    // Englisch: Namen des englischen Originals
    fp_i18n::set_language("en");
    let en: Vec<String> = CATEGORIES.iter().map(|k| fp_i18n::t(k)).collect();
    fp_i18n::set_language("de");
    assert_eq!(en, ["General", "Edit", "Guides and Grids", "Type", "Photoshop Import/Open", "Launch and Edit", "Plug-ins"]);
}

// ------------------------------------------------------------------ D17 Tastaturkurzbefehle

#[test]
fn d17_menu_tree_and_description() {
    let mut h = harness();
    run(&mut h, "edit.keyboardShortcuts");
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog offen");
    let rws = rows(st(&h), &d.work, 0, "");
    // zugeklappt: nur die Hauptmenüs, in Menüreihenfolge (Datei, Bearbeiten, Ansicht …)
    let top: Vec<String> = tree_lines(&rws, &d.expanded)
        .into_iter()
        .map(|l| match l {
            TreeLine::Folder { depth: 0, label, open: false, .. } => label,
            other => panic!("zugeklappt nur Menüs: {other:?}"),
        })
        .collect();
    assert_eq!(&top[..3], ["Datei", "Bearbeiten", "Ansicht"]);
    assert!(top.len() >= 8, "{top:?}");
    // Klick auf „Bearbeiten“ klappt auf: Befehle des Menüs erscheinen eingerückt
    let p = in_dialog(&h, Role::Button, "Bearbeiten");
    click(&mut h, p);
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog offen");
    assert!(d.expanded.contains("Bearbeiten"));
    let lines = tree_lines(&rws, &d.expanded);
    assert!(lines.iter().any(|l| matches!(l, TreeLine::Leaf { depth: 1, label, .. } if label == "Rückgängig")), "Bearbeiten ▸ Rückgängig sichtbar");
    assert!(lines.iter().any(|l| matches!(l, TreeLine::Folder { depth: 1, open: false, .. })), "Untermenüs zugeklappt");
    // Befehl wählen: Beschreibung erscheint (Statuszeilentext)
    h.run_steps(2);
    let p = in_dialog_any(&h, "Rückgängig");
    click(&mut h, p);
    let sel = st(&h).dialogs.shortcuts.as_ref().and_then(|d| d.selected.clone());
    assert!(sel.is_some(), "Befehl gewählt");
    let desc = fp_ui::dialogs::shortcuts::selected_description(st(&h), sel.as_deref());
    assert!(desc.is_some_and(|s| !s.is_empty()), "Beschreibung vorhanden");
    // Suche zeigt wieder die flache Liste
    let flat = rows(st(&h), &d.work, 0, "rückgängig");
    assert!(!flat.is_empty() && flat.iter().all(|r| r.1.to_lowercase().contains("rückgängig")));
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.shortcuts.is_none());
}
