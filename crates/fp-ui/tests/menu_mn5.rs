//! MN-5 (Nacharbeiten zu MN-4): „Wiederholen: <Filter>“ wie im Original (Text 10722), übersetzbare
//! Texte der mitgelieferten Befehlsskripte (`fp.t`), „Maske deaktivieren“ ohne Kürzel im Menü und
//! die Rückfrage 3169 beim Löschen einer gespeicherten Optimierungseinstellung.

use crate::gesamt_common;

use fp_ui::app_state::{Action, AppState};
use fp_ui::menu::{self, ItemAction, Node};
use gesamt_common::*;
use serde_json::{json, Value};

fn run(st: &mut AppState, ctx: &egui::Context, name: &str, p: Value) {
    st.push(Action::Run(name.into(), p));
    st.process_actions(ctx);
}

fn menu_nodes(st: &AppState, top: &str) -> Vec<Node> {
    menu::build_main_menu(st).into_iter().find(|(l, _)| l == top).unwrap_or_else(|| panic!("Menü {top}")).1
}

fn find<'a>(nodes: &'a [Node], path: &[&str]) -> Option<&'a Node> {
    let (head, rest) = path.split_first()?;
    nodes.iter().find_map(|n| match n {
        Node::Item { label, .. } if rest.is_empty() && label == head => Some(n),
        Node::Sub { label, children, .. } if label == head => if rest.is_empty() { Some(n) } else { find(children, rest) },
        _ => None,
    })
}

// ------------------------------------------------------------------ Filter ▸ Wiederholen

fn repeat_label(st: &AppState, top: &str) -> (String, bool) {
    menu_nodes(st, top)
        .iter()
        .find_map(|n| match n {
            Node::Item { label, action: ItemAction::Command(c), enabled, .. } if c == "filter.repeat" => Some((label.clone(), *enabled)),
            _ => None,
        })
        .expect("Filter wiederholen")
}

#[test]
fn repeat_filter_label_like_original() {
    let (mut st, _ctx, f) = fixture();
    // ohne vorherigen Filter: feste Beschriftung, grau
    assert_eq!(repeat_label(&st, "Filter"), ("Filter wiederholen".to_string(), false));
    let _ = st.session.execute("select.set", json!({ "ids": f.bitmap }));
    st.session.execute("filter.adjustColor.invert", json!({})).expect("Filter");
    assert_eq!(repeat_label(&st, "Filter"), ("Wiederholen: Umkehren".to_string(), true));
    // Filter mit Dialog: Menütext samt „...“ wie im Original („Wiederholen: Farbton/Sättigung...“)
    st.session.execute("filter.adjustColor.hueSaturation", json!({ "hue": 10, "saturation": 0, "lightness": 0 })).expect("Filter");
    assert_eq!(repeat_label(&st, "Filter").0, "Wiederholen: Farbton/Sättigung...");
    // Wiederholen ändert den Namen nicht
    st.session.execute("filter.repeat", json!({})).expect("wiederholen");
    assert_eq!(repeat_label(&st, "Filter").0, "Wiederholen: Farbton/Sättigung...");
    // Englisch
    fp_i18n::set_language("en");
    assert_eq!(repeat_label(&st, "Filters").0, "Repeat: Hue/Saturation...");
    fp_i18n::set_language("de");
}

// ------------------------------------------------------------------ Maske deaktivieren

#[test]
fn disable_mask_shows_no_shortcut() {
    let (st, _ctx, _f) = fixture();
    let nodes = menu_nodes(&st, "Modifizieren");
    match find(&nodes, &["Maske", "Maske deaktivieren"]) {
        Some(Node::Item { shortcut, .. }) => assert_eq!(shortcut.as_deref(), None, "im Original ohne Kürzel (UX-B, PM-7)"),
        other => panic!("{other:?}"),
    }
    // die übrigen Masken-Einträge behalten ihre Anzeige (nur dieser Eintrag ist ausgenommen)
    assert!(matches!(find(&nodes, &["Maske", "Maske löschen"]), Some(Node::Item { .. })));
}

// ------------------------------------------------------------------ Optimieren ▸ Einstellungen löschen

fn presets(st: &AppState) -> Vec<String> {
    fp_ui::ui_commands::export_presets(&st.session.prefs).into_iter().map(|(n, _)| n).collect()
}

#[test]
fn delete_settings_asks_first_and_no_keeps_preset() {
    let (mut st, ctx, _f) = fixture();
    st.session.execute("panel.optimize.saveSettings", json!({ "name": "Klein" })).expect("speichern");
    assert_eq!(presets(&st), ["Klein"]);
    // Weg wie aus dem Auswahldialog: Befehl mit Namen → erst die Rückfrage
    run(&mut st, &ctx, "panel.optimize.deleteSettings", json!({ "name": "Klein" }));
    let c = st.dialogs.confirm.clone().expect("Rückfrage 3169");
    assert_eq!(c.title, "Firepixel");
    assert_eq!(c.text, "Aktuelle Einstellungen löschen? Dies kann nicht rückgängig gemacht werden.");
    assert_eq!(presets(&st), ["Klein"], "vor der Antwort nichts gelöscht");
    // „Nein“ (bzw. Schließen/Esc): nichts gelöscht
    st.dialogs.confirm = None;
    st.process_actions(&ctx);
    assert_eq!(presets(&st), ["Klein"], "Nein löscht nicht");
    // nochmals, „Ja“: gelöscht, keine zweite Rückfrage
    run(&mut st, &ctx, "panel.optimize.deleteSettings", json!({ "name": "Klein" }));
    let c = st.dialogs.confirm.take().expect("Rückfrage");
    run(&mut st, &ctx, &c.command, c.params);
    assert!(st.dialogs.confirm.is_none(), "keine zweite Rückfrage");
    assert!(presets(&st).is_empty(), "Ja löscht");
    // Skripte/MCP (direkt über die Sitzung) löschen ohne Rückfrage wie bisher
    st.session.execute("panel.optimize.saveSettings", json!({ "name": "B" })).expect("speichern");
    st.session.execute("panel.optimize.deleteSettings", json!({ "name": "B" })).expect("löschen");
    assert!(presets(&st).is_empty());
    // Englisch
    fp_i18n::set_language("en");
    st.session.execute("panel.optimize.saveSettings", json!({ "name": "C" })).expect("speichern");
    run(&mut st, &ctx, "panel.optimize.deleteSettings", json!({ "name": "C" }));
    assert_eq!(st.dialogs.confirm.as_ref().map(|c| c.text.as_str()), Some("Delete current settings? This cannot be undone."));
    fp_i18n::set_language("de");
}

#[test]
fn delete_settings_confirm_dialog_buttons() {
    // echte Knöpfe im Rückfragefenster: „Nein“ löscht nicht, „Ja“ löscht
    use egui_kittest::kittest::Queryable;
    for (button, kept) in [("Nein", true), ("Ja", false)] {
        let (mut st, ctx, _f) = fixture();
        st.session.execute("panel.optimize.saveSettings", json!({ "name": "Klein" })).expect("speichern");
        run(&mut st, &ctx, "panel.optimize.deleteSettings", json!({ "name": "Klein" }));
        assert!(st.dialogs.confirm.is_some());
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(900.0, 600.0)).build_ui_state(
            |ui, st: &mut AppState| {
                let ctx = ui.ctx().clone();
                fp_ui::dialogs::misc::show_confirm(&ctx, st);
                st.process_actions(&ctx);
            },
            st,
        );
        h.run_steps(2);
        h.get_by_label(button).click();
        h.run_steps(3);
        let st = h.state();
        assert!(st.dialogs.confirm.is_none(), "{button}: Rückfrage geschlossen");
        assert_eq!(presets(st) == ["Klein"], kept, "{button}");
    }
}

// ------------------------------------------------------------------ Mitgelieferte Befehlsskripte

/// Altes MN-4-Skript „Gemeinsame Palette“ mit festem deutschen Text (wortgleich).
const OLD_SHARED_PALETTE: &str = "// gemeinsame Farbtabelle für mehrere Segmente\n\
alert(\"Gemeinsame Palette: Firepixel legt keine eigene gemeinsame Farbtabelle für mehrere Segmente an. Damit Segmente dieselben Farben verwenden, stellen Sie für jedes Segment im Bedienfeld „Optimieren“ dieselbe Farbtabelle ein.\");\n";

#[test]
fn builtin_scripts_follow_ui_language() {
    use fp_script::builtin_commands::{install_builtin_commands_in, BUILTIN};
    // keine festen deutschen Meldungen mehr in den mitgelieferten Skripten
    for (rel, src) in BUILTIN {
        for bad in ["alert(\"", "prompt(\"", "addNewLayer(\""] {
            assert!(!src.contains(bad), "{rel}: fester Text in {bad}…");
        }
    }
    let dir = temp_dir().join("Commands");
    std::fs::create_dir_all(&dir).unwrap();
    install_builtin_commands_in(&dir, "en").unwrap();
    let script = dir.join("Web").join("Shared Palette.jsf");
    assert!(script.exists());
    for (lang, expect) in [("en", "Shared Palette: Firepixel does not create a shared color table"), ("de", "Gemeinsame Palette: Firepixel legt keine eigene")] {
        fp_i18n::set_language(lang);
        let (mut st, ctx, _f) = fixture();
        fp_i18n::set_language(lang);
        run(&mut st, &ctx, "commands.runScript", json!({ "path": script.to_string_lossy() }));
        let (title, body) = st.dialogs.message.clone().unwrap_or_else(|| panic!("{lang}: Meldung erwartet, Status {:?}", st.status));
        assert_eq!(title, "Firepixel");
        assert!(body.starts_with(expect), "{lang}: {body}");
    }
    fp_i18n::set_language("de");
}

#[test]
fn builtin_script_variables_and_prompts_translated() {
    let (mut st, ctx, f) = fixture();
    let _ = st.session.execute("select.set", json!({ "ids": f.instance }));
    fp_i18n::set_language("en");
    let code = "alert(fp.t(\"builtin-cmd-symbol-script-unsupported\", { name: fw.selection[0].symbolName }));";
    run(&mut st, &ctx, "commands.runScript", json!({ "code": code }));
    let (_, body) = st.dialogs.message.clone().expect("Meldung");
    assert!(body.starts_with("Create Symbol Script: Firepixel cannot create symbol scripts for the symbol “"), "{body}");
    assert!(!body.contains('\u{2068}') && !body.contains("{ $name }"), "Variable eingesetzt: {body:?}");
    fp_i18n::set_language("de");
    // unbekannter Schlüssel: der Schlüssel selbst, kein Abbruch
    st.dialogs.message = None;
    run(&mut st, &ctx, "commands.runScript", json!({ "code": "alert(fp.t(\"builtin-cmd-unknown-key\"));" }));
    assert_eq!(st.dialogs.message.clone().map(|m| m.1).as_deref(), Some("builtin-cmd-unknown-key"));
}

#[test]
fn old_german_scripts_are_replaced_user_edits_kept() {
    use fp_script::builtin_commands::{install_builtin_commands_in, BUILTIN, MARKER_FILE};
    let src = |rel: &str| BUILTIN.iter().find(|(r, _)| *r == rel).map(|(_, s)| *s).unwrap();
    // eingerichteter deutscher Ordner aus MN-4 (Markierung mit set-2)
    let dir = temp_dir().join("Commands");
    std::fs::create_dir_all(dir.join("Web")).unwrap();
    std::fs::write(dir.join("Web").join("Gemeinsame Palette.jsf"), OLD_SHARED_PALETTE).unwrap();
    let user = format!("{OLD_SHARED_PALETTE}// vom Nutzer ergänzt\n");
    std::fs::write(dir.join(MARKER_FILE), "0.1.0\nset-2\n").unwrap();
    install_builtin_commands_in(&dir, "de").unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("Web").join("Gemeinsame Palette.jsf")).unwrap(), src("Web/Gemeinsame Palette.jsf"), "alte Fassung ersetzt");
    // englisch eingerichteter Ordner mit der alten deutschen Fassung → ebenfalls ersetzt
    let en = temp_dir().join("Commands");
    std::fs::create_dir_all(en.join("Web")).unwrap();
    std::fs::write(en.join("Web").join("Shared Palette.jsf"), OLD_SHARED_PALETTE).unwrap();
    std::fs::write(en.join(MARKER_FILE), "0.1.0\nset-2\n").unwrap();
    install_builtin_commands_in(&en, "en").unwrap();
    assert!(std::fs::read_to_string(en.join("Web").join("Shared Palette.jsf")).unwrap().contains("fp.t(\"builtin-cmd-shared-palette\")"));
    // vom Nutzer geänderte Datei bleibt unverändert
    std::fs::write(dir.join("Web").join("Gemeinsame Palette.jsf"), &user).unwrap();
    install_builtin_commands_in(&dir, "de").unwrap();
    assert_eq!(std::fs::read_to_string(dir.join("Web").join("Gemeinsame Palette.jsf")).unwrap(), user, "Nutzerfassung bleibt");
}
