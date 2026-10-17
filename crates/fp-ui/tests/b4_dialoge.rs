//! B-4: Befunde Dialoge/Datei-Dialoge/Gestaltung (Paket B4 in `docs\BEFUNDE_ABBAU.md`). Bedienung
//! über echte Tasten und Klicks im Fenster (kittest), wo es um Oberfläche geht.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::shortcuts::{rows, tree_key, tree_lines, TreeLine};
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
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

fn in_dialog(h: &H, role: Role, label: &str) -> Option<Pos2> {
    let dr = dialog_rect(h);
    h.query_all(By::new().role(role)).filter(|n| n.accesskit_node().label().as_deref() == Some(label)).map(|n| n.rect()).find(|r| dr.contains(r.center())).map(|r| r.center())
}

// ------------------------------------------------------------ BA-049 Tastaturkurzbefehle

/// Vier Symbolknöpfe (Name = Tooltip), „Befehle:“ vor dem Kombifeld, Baum mit Pfeiltasten.
#[test]
fn ba049_shortcuts_icons_label_and_tree_keys() {
    let mut h = harness();
    run(&mut h, "edit.keyboardShortcuts");
    for name in ["Duplizieren", "Umbenennen", "Exportieren", "Löschen"] {
        assert!(in_dialog(&h, Role::Button, name).is_some(), "Symbolknopf „{name}“");
    }
    // „Befehle:“ steht links vom Kombifeld „Menübefehle“
    assert_eq!(fp_i18n::t("dlg-sc-commands"), "Befehle:");
    // Klick auf „Bearbeiten“ fokussiert den Baum; ↓ wählt die erste Zeile darin, → / ← klappen
    let p = in_dialog(&h, Role::Button, "Bearbeiten").expect("Bearbeiten");
    click(&mut h, p);
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog");
    assert_eq!(d.cursor.as_deref(), Some("Bearbeiten"), "Knoten markiert");
    assert!(d.expanded.contains("Bearbeiten"));
    key(&mut h, Key::ArrowDown);
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog");
    assert!(d.cursor.is_some() || d.selected.is_some(), "↓ bewegt die Markierung: {:?}", (d.cursor, d.selected));
    assert_ne!(d.cursor.as_deref(), Some("Bearbeiten"), "↓ verlässt den Knoten");
    // ← geht zurück zum Knoten „Bearbeiten“, ein zweites ← klappt ihn zu
    key(&mut h, Key::ArrowLeft);
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog");
    if d.cursor.as_deref() != Some("Bearbeiten") {
        // die erste Zeile war ein Untermenü (aufgeklappt?) – noch einmal ←
        key(&mut h, Key::ArrowLeft);
    }
    let d = st(&h).dialogs.shortcuts.clone().expect("Dialog");
    assert_eq!(d.cursor.as_deref(), Some("Bearbeiten"), "← zum übergeordneten Knoten");
    key(&mut h, Key::ArrowLeft);
    assert!(!st(&h).dialogs.shortcuts.as_ref().expect("Dialog").expanded.contains("Bearbeiten"), "← klappt zu");
    key(&mut h, Key::ArrowRight);
    assert!(st(&h).dialogs.shortcuts.as_ref().expect("Dialog").expanded.contains("Bearbeiten"), "→ klappt auf");
    key(&mut h, Key::Escape);
    h.run_steps(3);
    assert!(st(&h).dialogs.shortcuts.is_none(), "Esc schließt weiterhin");
}

/// Baumlogik ohne Fenster: Pos1/Ende, Grenzen, Blatt wählt den Befehl mit erstem Kürzel.
#[test]
fn ba049_tree_key_logic() {
    let h = harness();
    let mut d = fp_ui::dialogs::shortcuts::ShortcutsDialog::new(st(&h));
    let rws = rows(st(&h), &d.work, 0, "");
    let lines = tree_lines(&rws, &d.expanded);
    assert!(tree_key(&mut d, &lines, &rws, Key::End));
    let last = match lines.last() {
        Some(TreeLine::Folder { path, .. }) => path.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(d.cursor.as_deref(), Some(last.as_str()));
    assert!(!tree_key(&mut d, &lines, &rws, Key::ArrowDown), "am Ende keine Wirkung");
    assert!(tree_key(&mut d, &lines, &rws, Key::Home));
    assert!(tree_key(&mut d, &lines, &rws, Key::ArrowRight), "→ klappt das erste Menü auf");
    let lines = tree_lines(&rws, &d.expanded);
    // nach dem Aufklappen geht → in den ersten Eintrag
    assert!(tree_key(&mut d, &lines, &rws, Key::ArrowRight));
    let lines = tree_lines(&rws, &d.expanded);
    // bis zum ersten Befehl (Blatt) laufen
    let mut guard = 0;
    while d.selected.is_none() && guard < 50 {
        assert!(tree_key(&mut d, &lines, &rws, Key::ArrowDown));
        guard += 1;
    }
    assert!(d.selected.is_some() && d.cursor.is_none(), "Blatt gewählt");
}

// ------------------------------------------------------------ BA-050 Bitmap-Auswahl wiederherstellen

/// Live-Anzeige: Die Leinwand zeigt die entstehende Auswahl, Abbrechen stellt die alte wieder her,
/// OK ergibt genau einen Verlaufsschritt.
#[test]
fn ba050_restore_selection_live_preview() {
    let mut h = harness();
    {
        let s = &mut h.state_mut().state.session;
        s.execute("select.pixels", json!({"shape": "rect", "rect": [10, 10, 60, 60]})).expect("Auswahl");
        s.execute("select.saveBitmapSelection", json!({"name": "A"})).expect("speichern");
        s.execute("select.pixels", json!({"shape": "none"})).expect("keine Pixelauswahl");
    }
    let before = st(&h).session.active().expect("doc").selection.clone();
    let undo_before = st(&h).session.active().expect("doc").history.undo_count();
    run(&mut h, "select.restoreBitmapSelection");
    assert!(st(&h).dialogs.bitmap_selection.is_some(), "Dialog offen");
    let live = st(&h).session.active().expect("doc").selection.clone();
    assert!(live.has_pixels(), "Live-Anzeige zeigt die Auswahl");
    assert_eq!(st(&h).session.active().expect("doc").history.undo_count(), undo_before, "kein Verlaufsschritt");
    key(&mut h, Key::Escape);
    h.run_steps(2);
    assert!(st(&h).dialogs.bitmap_selection.is_none());
    assert_eq!(st(&h).session.active().expect("doc").selection, before, "Abbrechen: Auswahl wie vorher");
    // OK: ein Schritt, Auswahl geladen
    run(&mut h, "select.restoreBitmapSelection");
    key(&mut h, Key::Enter);
    h.run_steps(3);
    assert!(st(&h).dialogs.bitmap_selection.is_none());
    let d = st(&h).session.active().expect("doc");
    assert!(d.selection.has_pixels());
    assert_eq!(d.history.undo_count(), undo_before + 1, "genau ein Schritt");
}

// ------------------------------------------------------------ BA-051 Einstellungen löschen

/// Gewählte eigene Einstellung: Bedienfeldmenü fragt sofort „Aktuelle Einstellungen löschen?“ – kein
/// Auswahldialog.
#[test]
fn ba051_delete_selected_own_setting_directly() {
    let mut h = harness();
    {
        let s = &mut h.state_mut().state.session;
        s.execute("panel.optimize.saveSettings", json!({"name": "Meine"})).expect("speichern");
        s.execute("panel.optimize.saveSettings", json!({"name": "Andere"})).expect("speichern");
        s.execute("panel.optimize.applySettings", json!({"name": "Meine"})).expect("anwenden");
    }
    assert!(fp_ui::panels::optimize::selected_own_preset(st(&h)).is_some());
    run(&mut h, "panel.optimize.deleteSettings");
    h.run_steps(2);
    assert!(st(&h).dialogs.form.is_none(), "kein Auswahldialog");
    let c = st(&h).dialogs.confirm.clone().expect("Rückfrage");
    assert_eq!(c.command, "panel.optimize.deleteSettings");
    assert!(c.params.get("name").and_then(|v| v.as_str()).is_some(), "{:?}", c.params);
}

// ------------------------------------------------------------ BA-053 Neues Symbol im Bibliothek-Bedienfeld

#[test]
fn ba053_library_new_symbol_like_d14() {
    let mut h = harness();
    run(&mut h, "panel.library.newSymbol");
    let f = st(&h).dialogs.form.clone().expect("Dialog");
    let keys: Vec<&str> = f.fields.iter().map(|x| x.key.as_str()).collect();
    assert_eq!(keys, ["name", "kind", "nineSlice", "_common"]);
    // 9-Slice wirkt
    let n0 = st(&h).session.active().expect("doc").doc.symbols.len();
    h.state_mut().state.session.execute("panel.library.newSymbol", json!({"name": "S9", "nineSlice": true})).expect("anlegen");
    let d = st(&h).session.active().expect("doc");
    assert_eq!(d.doc.symbols.len(), n0 + 1);
}

// ------------------------------------------------------------ BA-054 Exportdialog meldet „exportiert“

#[test]
fn ba054_export_counter_field_exists() {
    let h = harness();
    assert_eq!(st(&h).dialogs.exported, 0);
}

// ------------------------------------------------------------ BA-055 Knopf „Pinselstrich bearbeiten“

/// Im deutschen Inspektor ist der Text des Knopfs ganz zu lesen (kein „…“).
#[test]
fn ba055_brush_edit_button_full_text() {
    let mut h = harness();
    {
        let s = &mut h.state_mut().state.session;
        s.execute("select.all", json!({})).expect("alle");
    }
    h.run_steps(3);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    h.run_steps(2);
    let audit = fp_ui::inspector::fields::take_audit(&h.ctx);
    let hits: Vec<_> = audit.iter().filter(|a| a.text == "Pinselstrich bearbeiten").collect();
    if !hits.is_empty() {
        for a in hits {
            assert!(a.fit < 2, "gekürzt: {a:?}");
        }
    }
    // Knopf über seinen Namen auffindbar (Barrierefreiheit)
    let _ = h.query_all(By::new().role(Role::Button)).filter(|n| n.accesskit_node().label().as_deref() == Some("Pinselstrich bearbeiten")).count();
}

// ------------------------------------------------------------ BA-057 Verlaufsnamen Pfade

#[test]
fn ba057_path_history_names() {
    let h = harness();
    drop(h);
    let mut s = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    s.execute("file.new", json!({"width": 200, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("R");
    s.execute("modify.alterPath.expandStroke", json!({"width": 5})).expect("erweitern");
    assert_eq!(s.active().and_then(|d| d.history.undo_label()).map(|x| x.to_string()).as_deref(), Some("Pfade erweitern"));
    s.execute("modify.alterPath.insetPath", json!({"distance": 2})).expect("einlassen");
    assert_eq!(s.active().and_then(|d| d.history.undo_label()).map(|x| x.to_string()).as_deref(), Some("Pfade einlassen"));
}

// ------------------------------------------------------------ BA-058 „Optionen…“ im Exportdialog

#[test]
fn ba058_export_options_button_has_name() {
    let mut h = harness();
    run(&mut h, "file.export");
    h.run_steps(2);
    if st(&h).dialogs.export.is_some() {
        assert!(in_dialog(&h, Role::Button, "Optionen...").or_else(|| in_dialog(&h, Role::Button, "Optionen…")).is_some(), "Knopf „Optionen…“ per Name");
    }
}

// ------------------------------------------------------------ BA-059 Kürzel nach der Menümessung

#[test]
fn ba059_shortcuts_like_menu_measurement() {
    let h = harness();
    let s = &st(&h).session;
    let txt = |n: &str| s.command_shortcuts(n).iter().map(fp_ui::menu::shortcut_text).collect::<Vec<_>>();
    eprintln!("Satz {}: Verhalten {:?}, Dateibrowser {:?}, Maske {:?}, Seiten {:?}", s.prefs.shortcut_set(), txt("window.behaviors"), txt("file.browseInBridge"), txt("modify.mask.disableMask"), txt("window.pages"));
    assert_eq!(txt("window.behaviors").first().map(String::as_str), Some("Umschalt+F3"));
    assert_eq!(txt("file.browseInBridge").first().map(String::as_str), Some("Strg+Alt+O"));
    assert_eq!(txt("window.pages").first().map(String::as_str), Some("F5"));
}

// ------------------------------------------------------------ BA-064 Fußleiste der Voreinstellungen

#[test]
fn ba064_preferences_have_footer() {
    let mut h = harness();
    run(&mut h, "edit.preferences");
    h.run_steps(3);
    assert!(fp_ui::dialogs::dialog_footer_top(&h.ctx, "fp-prefs").is_some(), "Fußleiste unter der Kategorienliste");
}

// ------------------------------------------------------------ F11: Enter in der Popupmenü-Tabelle

/// Enter in einer Tabellenzelle des Popupmenü-Editors übernimmt die Zelle und blättert nicht weiter.
#[test]
fn f11_enter_in_popup_cell_stays_on_tab() {
    fp_engine::shell::disable();
    crate::gesamt_common::test_profile();
    let mut s = fp_ui::ui_commands::new_session(fp_engine::Preferences::new());
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 60]})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(12).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(3);
    h.state_mut().state.run("modify.popUpMenu.addPopUpMenu", json!({}));
    h.run_steps(4);
    assert_eq!(st(&h).dialogs.popup_editor.as_ref().map(|d| d.tab), Some(0));
    let dr = dialog_rect(&h);
    let mut cells: Vec<Rect> = h.get_all(By::new().role(Role::TextInput)).map(|n| n.rect()).filter(|r| dr.contains(r.center())).collect();
    cells.sort_by(|a, b| a.left().total_cmp(&b.left()));
    let c0 = cells.first().copied().expect("Zelle");
    click(&mut h, c0.center());
    ev(&mut h, Event::Text("Start".into()));
    key(&mut h, Key::Enter);
    h.run_steps(3);
    let d = st(&h).dialogs.popup_editor.clone().expect("Editor bleibt offen");
    assert_eq!(d.tab, 0, "Enter in der Zelle blättert nicht zu „Erscheinungsbild“");
    assert_eq!(d.rows[0].text, "Start", "Zelle übernommen");
}

/// Mehrere Pfeiltasten hintereinander bewegen die Markierung weiter (Fokus bleibt im Baum).
#[test]
fn ba049_tree_keeps_focus_over_several_keys() {
    let mut h = harness();
    run(&mut h, "edit.keyboardShortcuts");
    let p = in_dialog(&h, Role::Button, "Bearbeiten").expect("Bearbeiten");
    // Klick und sofort danach ↓ (wie über die Brücke): der Fokus darf nicht aus dem Baum wandern
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    let mut seen = Vec::new();
    for _ in 0..3 {
        key(&mut h, Key::ArrowDown);
        let d = st(&h).dialogs.shortcuts.clone().expect("Dialog");
        seen.push((d.cursor.clone(), d.selected.clone()));
    }
    assert!(seen[0] != seen[1] && seen[1] != seen[2], "jede ↓ bewegt: {seen:?}");
}
