//! TW-131: the URL panel menu (≡) has the nine entries of the original, with "Edit URL..." and
//! "Delete URL" grey without a selected URL, and its actions work on the URL library.

use fp_engine::Session;
use fp_engine::url_library::UrlLibrary;
use fp_ui::FirepixelApp;
use fp_ui::app_state::{Action, UiSettings};
use fp_ui::menu::{ItemAction, Node};
use fp_ui::panels::PanelId;
use serde_json::json;

fn app() -> (egui::Context, FirepixelApp) {
    let ctx = egui::Context::default();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("edit.insert.slice", json!({})).expect("slice");
    s.execute("pi.web", json!({"url": "http://used.example/"})).expect("url");
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.dialogs.url.ensure_loaded();
    a.state.dialogs.url.libs = vec![UrlLibrary::new("URLs.htm")];
    a.state.dialogs.url.current = 0;
    a.state.dialogs.url.sel = None;
    (ctx, a)
}

fn rows(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .map(|n| match n {
            Node::Separator => "---".to_string(),
            Node::Item { label, enabled, .. } => {
                if *enabled { label.clone() } else { format!("{label} (grau)") }
            }
            Node::Sub { label, .. } => format!("{label} ▸"),
        })
        .collect()
}

fn menu(a: &FirepixelApp) -> Vec<String> {
    rows(&fp_ui::menu::build_panel_menu(&a.state, PanelId::Url))
}

fn urls(a: &FirepixelApp) -> Vec<String> {
    a.state.dialogs.url.lib().expect("lib").urls.clone()
}

fn run(ctx: &egui::Context, a: &mut FirepixelApp, name: &str, p: serde_json::Value) {
    fp_ui::actions::execute(&mut a.state, ctx, Action::Run(name.into(), p));
}

#[test]
fn url_menu_lists_nine_entries_with_enable_states() {
    let (_ctx, mut a) = app();
    assert_eq!(
        menu(&a),
        [
            "Verwendete URLs in Bibliothek aufnehmen",
            "Nicht verwendete URLs löschen",
            "---",
            "URL hinzufügen...",
            "URL bearbeiten... (grau)",
            "URL löschen (grau)",
            "---",
            "Neue URL-Bibliothek...",
            "URLs importieren...",
            "URLs exportieren...",
            "---",
            "Hilfe",
        ]
    );
    a.state.dialogs.url.libs[0].add("http://x.example/");
    a.state.dialogs.url.sel = Some(0);
    let m = menu(&a);
    assert_eq!(m[4], "URL bearbeiten...");
    assert_eq!(m[5], "URL löschen");
    // every entry runs a UI action
    for n in fp_ui::menu::build_panel_menu(&a.state, PanelId::Url) {
        if let Node::Item { action, .. } = n {
            assert!(matches!(action, ItemAction::Ui(Action::Run(..))));
        }
    }
}

#[test]
fn url_menu_actions_change_the_library() {
    let (ctx, mut a) = app();
    run(&ctx, &mut a, "ui.url.add", json!({"url": "http://unused.example/"}));
    run(&ctx, &mut a, "ui.url.addUsed", json!({}));
    assert_eq!(urls(&a), ["http://unused.example/", "http://used.example/"]);
    run(&ctx, &mut a, "ui.url.deleteUnused", json!({}));
    assert_eq!(urls(&a), ["http://used.example/"]);
    run(&ctx, &mut a, "ui.url.edit", json!({"index": 0, "url": "http://edited.example/"}));
    assert_eq!(urls(&a), ["http://edited.example/"]);
    a.state.dialogs.url.sel = Some(0);
    run(&ctx, &mut a, "ui.url.delete", json!({}));
    assert!(urls(&a).is_empty());
    assert_eq!(a.state.dialogs.url.selected(), None);
    // without parameters, add/edit/new library open a dialog
    run(&ctx, &mut a, "ui.url.add", json!({}));
    assert!(a.state.dialogs.form.take().is_some());
    run(&ctx, &mut a, "ui.url.newLibrary", json!({}));
    assert!(a.state.dialogs.form.take().is_some());
    // export and import through the file dialogs (test override)
    let dir = std::env::temp_dir().join(format!("fp-ui-url-tw131-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let file = dir.join("list.htm");
    a.state.dialogs.url.libs[0].add("http://a.example/");
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![file.clone()], save: Some(file.clone()) });
    run(&ctx, &mut a, "ui.url.export", json!({}));
    assert!(std::fs::read_to_string(&file).expect("exported").contains("<a href=\"http://a.example/\">"));
    // new library: empty, current, then import fills it
    run(&ctx, &mut a, "ui.url.newLibrary", json!({"name": "Second"}));
    assert_eq!(a.state.dialogs.url.lib().expect("lib").name, "Second.htm");
    assert!(urls(&a).is_empty());
    run(&ctx, &mut a, "ui.url.import", json!({}));
    assert_eq!(urls(&a), ["http://a.example/"]);
    let _ = std::fs::remove_dir_all(&dir);
}
