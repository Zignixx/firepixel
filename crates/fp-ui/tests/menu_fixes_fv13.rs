//! FV-13 (P-MENU1): Menüeinträge, die im Gesamttest durchfielen. Je Eintrag: freigegeben wie im
//! Original (grau ohne Ziel, aktiv mit Ziel), Wirkung am Dokument bzw. an den Voreinstellungen,
//! Rückgängig stellt exakt her; der Gesamttest liest nie das echte Nutzerprofil.

mod gesamt_common;

use fp_ui::app_state::{Action, AppState};
use fp_ui::menu::{self, ItemAction, Node};
use gesamt_common::*;
use serde_json::{json, Value};

fn run(st: &mut AppState, ctx: &egui::Context, name: &str, p: Value) {
    st.push(Action::Run(name.into(), p));
    st.process_actions(ctx);
}

fn doc_json(st: &AppState) -> Value {
    serde_json::to_value(&*st.session.active().expect("Dokument").doc).unwrap()
}

/// Rückgängig nach genau einem Verlaufsschritt stellt `before` exakt her.
fn undo_restores(st: &mut AppState, before: &Value, undo_before: usize) {
    let undo = st.session.active().unwrap().history.undo_count();
    assert_eq!(undo, undo_before + 1, "genau ein Verlaufsschritt");
    st.session.undo().expect("Rückgängig");
    assert_eq!(&doc_json(st), before, "Rückgängig stellt exakt her");
}

fn undo_count(st: &AppState) -> usize {
    st.session.active().unwrap().history.undo_count()
}

fn find_item<'a>(nodes: &'a [Node], path: &[&str]) -> Option<&'a Node> {
    let (head, rest) = path.split_first()?;
    for n in nodes {
        match n {
            Node::Item { label, .. } if rest.is_empty() && label == head => return Some(n),
            Node::Sub { label, children, .. } if label == head => return if rest.is_empty() { Some(n) } else { find_item(children, rest) },
            _ => {}
        }
    }
    None
}

fn menu_item(st: &AppState, path: &[&str]) -> (bool, ItemAction) {
    let menus = menu::build_main_menu(st);
    let (_, nodes) = menus.iter().find(|(l, _)| l == path[0]).unwrap_or_else(|| panic!("Menü {}", path[0]));
    match find_item(nodes, &path[1..]) {
        Some(Node::Item { enabled, action, .. }) => (*enabled, action.clone()),
        other => panic!("Eintrag {path:?} nicht gefunden: {other:?}"),
    }
}

fn layers(st: &AppState) -> Vec<(String, bool, bool, fp_model::LayerKind)> {
    let d = st.session.active().unwrap();
    fp_engine::docops::own_layers(&d.doc, d.page)
        .into_iter()
        .map(|(_, l)| (l.name.clone(), l.visibility.first().copied().unwrap_or(true), l.cels.iter().any(|c| c.locked), l.kind))
        .collect()
}

// ------------------------------------------------------------------ Bitmap-Auswahl

#[test]
fn bitmap_selection_restore_and_delete_need_saved_selection() {
    let (mut st, ctx, _f) = fixture();
    for c in ["select.restoreBitmapSelection", "select.deleteBitmapSelection"] {
        assert!(!available(&st, c), "{c}: ohne gespeicherte Auswahl grau wie im Original");
    }
    let _ = st.session.execute("select.pixels", json!({ "shape": "rect", "rect": [40, 40, 120, 120], "mode": "replace" }));
    let mask = st.session.active().unwrap().selection.pixels.clone().expect("Pixelauswahl");
    st.session.execute("select.saveBitmapSelection", json!({ "name": "Testauswahl" })).expect("speichern");
    let _ = st.session.execute("select.pixels", json!({ "shape": "none" }));
    assert!(st.session.active().unwrap().selection.pixels.is_none());
    for c in ["select.restoreBitmapSelection", "select.deleteBitmapSelection"] {
        assert!(available(&st, c), "{c}: mit gespeicherter Auswahl aktiv");
    }
    // Menü öffnet den Dialog; OK (= Befehl mit den Dialogwerten) stellt die Maske wieder her
    run(&mut st, &ctx, "select.restoreBitmapSelection", json!({}));
    assert!(st.dialogs.any_open() || st.dialogs.pending_form.is_some(), "Dialog „Auswahl wiederherstellen“");
    st.dialogs = Default::default();
    st.session.execute("select.restoreBitmapSelection", json!({ "index": 0, "mode": "replace" })).expect("wiederherstellen");
    assert_eq!(st.session.active().unwrap().selection.pixels.as_ref(), Some(&mask), "gleiche Maske wie gespeichert");
    // Löschen entfernt die gespeicherte Auswahl (ein Verlaufsschritt, Rückgängig exakt)
    let before = doc_json(&st);
    let n = undo_count(&st);
    st.session.execute("select.deleteBitmapSelection", json!({ "index": 0 })).expect("löschen");
    assert!(!available(&st, "select.restoreBitmapSelection"), "nach dem Löschen wieder grau");
    undo_restores(&mut st, &before, n);
    assert!(available(&st, "select.restoreBitmapSelection"));
}

#[test]
fn bitmap_selection_entries_pass_in_menu_test() {
    for c in ["select.restoreBitmapSelection", "select.deleteBitmapSelection"] {
        let (o, note) = run_menu_command(c);
        assert!(o.ok(), "{c}: {o:?} {note}");
        assert!(note.contains("gespeicherte Bitmap-Auswahl"), "{c}: {note}");
    }
}

// ------------------------------------------------------------------ Hilfslinien

#[test]
fn clear_guides_needs_guides_and_removes_them() {
    let (mut st, _ctx, _f) = fixture();
    assert!(!available(&st, "view.guides.clearGuides"), "ohne Hilfslinien grau wie im Original");
    st.session.execute("view.guides.editGuides", json!({ "horizontal": [100.0, 150.0], "vertical": [200.0] })).unwrap();
    assert!(available(&st, "view.guides.clearGuides"), "mit Hilfslinien aktiv");
    let before = doc_json(&st);
    let n = undo_count(&st);
    st.session.execute("view.guides.clearGuides", json!({})).unwrap();
    let d = st.session.active().unwrap();
    let g = d.doc.pages[d.page].root.guides.as_ref().expect("Hilfslinien");
    assert!(g.horizontal.is_empty() && g.vertical.is_empty());
    assert!(!available(&st, "view.guides.clearGuides"), "danach wieder grau");
    undo_restores(&mut st, &before, n);
    let (o, note) = run_menu_command("view.guides.clearGuides");
    assert!(o.ok(), "{o:?} {note}");
}

// ------------------------------------------------------------------ Pfade kombinieren ▸ Teilen

#[test]
fn split_needs_composite_path_and_splits_it() {
    let (mut st, _ctx, f) = fixture();
    select(&mut st, &f.path);
    assert!(!available(&st, "modify.combinePaths.split"), "einfacher Pfad: grau wie im Original");
    select(&mut st, &[f.path[0], f.path2[0]]);
    assert!(!available(&st, "modify.combinePaths.split"), "zwei einfache Pfade: grau");
    st.session.execute("modify.combinePaths.join", json!({})).unwrap();
    let joined = sel(&st);
    assert_eq!(joined.len(), 1, "ein zusammengesetzter Pfad");
    assert!(available(&st, "modify.combinePaths.split"), "zusammengesetzter Pfad: aktiv");
    let before = doc_json(&st);
    let n = undo_count(&st);
    st.session.execute("modify.combinePaths.split", json!({})).unwrap();
    assert_eq!(sel(&st).len(), 2, "wieder zwei Pfade");
    undo_restores(&mut st, &before, n);
    let (o, note) = run_menu_command("modify.combinePaths.split");
    assert!(o.ok(), "{o:?} {note}");
}

// ------------------------------------------------------------------ Vorschau im zweiten Browser

#[test]
fn secondary_browser_preview_needs_configured_browser() {
    let (mut st, _ctx, f) = fixture();
    let path = ["Datei", "Vorschau im Browser", "Vorschau in"];
    let (en, _) = menu_item(&st, &path);
    assert!(!en, "ohne zweiten Browser grau wie im Original („Vorschau in“ ohne Namen)");
    let exe = f.file.with_file_name("browser.exe");
    st.session.execute("file.previewInBrowser.setSecondaryBrowser", json!({ "path": exe.to_string_lossy() })).unwrap();
    let (en, _) = menu_item(&st, &["Datei", "Vorschau im Browser", "Vorschau in browser.exe"]);
    assert!(en, "mit zweitem Browser aktiv und mit dessen Programmnamen");
    // Wirkung: HTML-Vorschau geschrieben (Start des Browsers ist im Test abgeschaltet)
    let out = st.session.execute("file.previewInBrowser.previewInSecondaryBrowser", json!({})).expect("Vorschau").value;
    let html = out["html"].as_str().expect("html");
    assert!(std::path::Path::new(html).exists(), "Vorschauseite {html}");
    let (o, note) = run_menu_command("file.previewInBrowser.previewInSecondaryBrowser");
    assert!(o.ok(), "{o:?} {note}");
}

// ------------------------------------------------------------------ Befehlsskripte

/// Der Befehlsordner des Tests ist nie der des echten Nutzerprofils und enthält die
/// mitgelieferten Befehle.
#[test]
fn menu_test_uses_test_commands_folder() {
    let (st, _ctx, _f) = fixture();
    let dir = fp_engine::commands::misc::commands_dir();
    assert!(dir.starts_with(test_profile()), "Befehlsordner {dir:?}");
    if let Some(app) = std::env::var_os("APPDATA") {
        assert!(!dir.starts_with(app), "nie %APPDATA%: {dir:?}");
    }
    assert!(fp_engine::commands::misc::extensions_dir().starts_with(test_profile()));
    for p in [&["Befehle", "Dokument", "Andere Ebenen ausblenden"][..], &["Befehle", "Dokument", "Andere Ebenen sperren"], &["Befehle", "Warndialoge zurücksetzen"]] {
        let (en, action) = menu_item(&st, p);
        assert!(en, "{p:?}");
        let ItemAction::Ui(Action::Run(name, params)) = action else { panic!("{p:?}: {action:?}") };
        assert_eq!(name, "commands.runScript");
        let file = params["path"].as_str().unwrap_or_default();
        assert!(std::path::Path::new(file).starts_with(&dir), "{p:?}: {file}");
    }
}

fn script_path(st: &AppState, p: &[&str]) -> Value {
    match menu_item(st, p).1 {
        ItemAction::Ui(Action::Run(_, params)) => params,
        other => panic!("{other:?}"),
    }
}

#[test]
fn hide_and_lock_other_layers_scripts_work() {
    for (entry, hidden) in [("Andere Ebenen ausblenden", true), ("Andere Ebenen sperren", false)] {
        let (mut st, ctx, _f) = fixture();
        // ohne zweite Ebene (nur Ebene 1 und Web-Ebene) gibt es nichts zu tun
        let before = doc_json(&st);
        let params = script_path(&st, &["Befehle", "Dokument", entry]);
        run(&mut st, &ctx, "commands.runScript", params.clone());
        assert_eq!(doc_json(&st), before, "{entry}: ohne andere Ebenen keine Änderung");
        // zwei Ebenen: die neue ist aktiv, die alte wird aus-/gesperrt, die Web-Ebene bleibt
        st.session.execute("layer.add", json!({ "name": "Oben" })).unwrap();
        let before = doc_json(&st);
        let n = undo_count(&st);
        run(&mut st, &ctx, "commands.runScript", params);
        let l = layers(&st);
        let other = l.iter().find(|x| x.0 != "Oben" && x.3 != fp_model::LayerKind::Web).expect("andere Ebene");
        let cur = l.iter().find(|x| x.0 == "Oben").expect("aktive Ebene");
        let web = l.iter().find(|x| x.3 == fp_model::LayerKind::Web).expect("Web-Ebene");
        if hidden {
            assert!(!other.1, "{entry}: andere Ebene ausgeblendet: {l:?}");
            assert!(cur.1 && web.1, "{entry}: aktive Ebene und Web-Ebene bleiben sichtbar: {l:?}");
        } else {
            assert!(other.2, "{entry}: andere Ebene gesperrt: {l:?}");
            assert!(!cur.2 && !web.2, "{entry}: aktive Ebene und Web-Ebene bleiben frei: {l:?}");
        }
        undo_restores(&mut st, &before, n);
    }
}

#[test]
fn reset_warning_dialogs_script_reenables_notices() {
    let (mut st, ctx, _f) = fixture();
    st.session.execute("edit.preferences", json!({ "key": "DisableNoticeCombinePaths", "value": true })).unwrap();
    st.session.execute("edit.preferences", json!({ "key": "DisableConfirmDeleteLayer", "value": true })).unwrap();
    assert!(st.session.prefs.get_bool("DisableNoticeCombinePaths", false));
    let params = script_path(&st, &["Befehle", "Warndialoge zurücksetzen"]);
    let before = doc_json(&st);
    run(&mut st, &ctx, "commands.runScript", params);
    assert!(!st.session.prefs.get_bool("DisableNoticeCombinePaths", false), "Hinweis wieder eingeschaltet");
    assert!(!st.session.prefs.get_bool("DisableConfirmDeleteLayer", false), "Rückfrage wieder eingeschaltet");
    assert_eq!(doc_json(&st), before, "Dokument unverändert");
}

#[test]
fn command_scripts_pass_in_menu_test() {
    let (st, _ctx, _f) = fixture();
    for p in [&["Befehle", "Dokument", "Andere Ebenen ausblenden"][..], &["Befehle", "Dokument", "Andere Ebenen sperren"], &["Befehle", "Warndialoge zurücksetzen"]] {
        let ItemAction::Ui(a) = menu_item(&st, p).1 else { panic!() };
        let (o, note) = run_item(a, Some("commands.runScript"));
        assert!(o.ok(), "{p:?}: {o:?} {note}");
    }
}
