//! MN-4 (PM-7, PARITAET_B §1): Befehle-Menü wie im Original (fehlende mitgelieferte Befehle, Namen,
//! „Warnmeldungen zurücksetzen“ nur einmal), Kürzeltexte „Strg+AKUT“/„Strg+ZIRKUMFLEX“,
//! „Filter wiederholen: <Name>“ und der neutrale Eintrag „Im Dateibrowser durchsuchen...“.

use crate::gesamt_common;

use fp_ui::app_state::{Action, AppState};
use fp_ui::menu::{self, ItemAction, Node};
use gesamt_common::*;
use serde_json::{json, Value};

/// Testdokument mit eigenem Befehlsordner je Test: parallel laufende Tests, die Befehle speichern,
/// ändern das Befehle-Menü dieses Tests nicht (FIX-F).
fn fixture() -> (AppState, egui::Context, Fix) {
    own_commands_dir();
    gesamt_common::fixture()
}

fn run(st: &mut AppState, ctx: &egui::Context, name: &str, p: Value) {
    st.push(Action::Run(name.into(), p));
    st.process_actions(ctx);
}

fn doc_json(st: &AppState) -> Value {
    serde_json::to_value(&*st.session.active().expect("Dokument").doc).unwrap()
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

fn is_script(n: &Node) -> bool {
    matches!(n, Node::Item { action: ItemAction::Ui(Action::Run(c, p)), .. } if c == "commands.runScript" && p.get("path").is_some())
}

/// Beschriftungen der Befehle aus dem Befehlsordner (Skripte und ihre Unterordner) in Menüfolge.
fn script_labels(nodes: &[Node]) -> Vec<String> {
    nodes
        .iter()
        .filter_map(|n| match n {
            Node::Sub { label, children, .. } if !children.is_empty() && children.iter().all(is_script) => Some(label.clone()),
            Node::Item { label, .. } if is_script(n) => Some(label.clone()),
            _ => None,
        })
        .collect()
}

fn sub_labels(nodes: &[Node], path: &[&str]) -> Vec<String> {
    match find(nodes, path) {
        Some(Node::Sub { children, .. }) => children.iter().filter_map(|n| if let Node::Item { label, .. } = n { Some(label.clone()) } else { None }).collect(),
        other => panic!("{path:?}: {other:?}"),
    }
}

fn script_params(st: &AppState, path: &[&str]) -> Value {
    match find(&menu_nodes(st, "Befehle"), path) {
        Some(Node::Item { enabled, action: ItemAction::Ui(Action::Run(c, p)), .. }) => {
            assert!(*enabled, "{path:?} aktiv");
            assert_eq!(c, "commands.runScript");
            p.clone()
        }
        other => panic!("{path:?}: {other:?}"),
    }
}

fn shortcut(nodes: &[Node], path: &[&str]) -> Option<String> {
    match find(nodes, path) {
        Some(Node::Item { shortcut, .. }) => shortcut.clone(),
        other => panic!("{path:?}: {other:?}"),
    }
}

// ------------------------------------------------------------------ Befehle-Menü

#[test]
fn commands_menu_names_and_order_like_original() {
    let (st, _ctx, _f) = fixture();
    let nodes = menu_nodes(&st, "Befehle");
    assert_eq!(
        script_labels(&nodes),
        [
            "Auswahl",
            "Batch-Befehle",
            "Bildschirmfoto",
            "Demo - aktuelles Dokument",
            "Dokument",
            "Größe ausgewählter Objekte ändern",
            "Kreativ",
            "Symbolskript erstellen",
            "Text",
            "Warnmeldungen zurücksetzen",
            "Web",
        ]
    );
    assert_eq!(sub_labels(&nodes, &["Auswahl"]), ["Früheres Vektorrendern verwenden", "Verbessertes Vektorrendern verwenden"]);
    assert_eq!(
        sub_labels(&nodes, &["Batch-Befehle"]),
        [
            "180°-Drehung",
            "270°-Drehung",
            "90°-Drehung",
            "Alles auswählen",
            "Auswahl scharfzeichnen",
            "Auswahl um 180 Grad drehen",
            "Auswahl um 270 Grad drehen",
            "Auswahl um 90 Grad drehen",
            "Auswahl weichzeichnen",
            "Auswahlfarbe umkehren",
            "Auto-Tonwertauswahl",
        ]
    );
    assert_eq!(sub_labels(&nodes, &["Dokument"]), ["Andere Ebenen ausblenden", "Andere Ebenen sperren", "Status umkehren", "Verteilung auf Ebenen"]);
    assert_eq!(
        sub_labels(&nodes, &["Kreativ"]),
        ["Autom. Vektormaske", "Bilderrahmen hinzufügen", "Drehen und Ausblenden", "Graustufenkonvertierung", "Pfeilspitzen hinzufügen", "Schatten hinzufügen", "Sepia-Konvertierung"]
    );
    assert_eq!(
        sub_labels(&nodes, &["Text"]),
        [
            "Als Großbuchstaben",
            "Als Kleinbuchstaben",
            "Als Pfade wiedergeben",
            "Als Pixel wiedergeben",
            "Breite festlegen",
            "Erster Buchstabe Großbuchstabe",
            "Feste Breite ein/aus",
            "Lorem ipsum",
            "Verfestigen",
        ]
    );
    assert_eq!(sub_labels(&nodes, &["Web"]), ["ALT-Text festlegen", "Gemeinsame Palette", "Leeren ALT-Text auswählen"]);
}

#[test]
fn reset_warnings_only_once() {
    let (st, _ctx, _f) = fixture();
    let nodes = menu_nodes(&st, "Befehle");
    let n = nodes.iter().filter(|n| matches!(n, Node::Item { label, .. } if label.starts_with("Warnmeldungen") || label.starts_with("Warndialoge"))).count();
    assert_eq!(n, 1, "„Warnmeldungen zurücksetzen“ genau einmal");
    // der verbliebene Eintrag wirkt: abgeschaltete Hinweise kommen wieder
    let (mut st, ctx, _f) = fixture();
    st.session.execute("edit.preferences", json!({ "key": "DisableNoticeCombinePaths", "value": true })).unwrap();
    let p = script_params(&st, &["Warnmeldungen zurücksetzen"]);
    run(&mut st, &ctx, "commands.runScript", p);
    assert!(!st.session.prefs.get_bool("DisableNoticeCombinePaths", false));
}

#[test]
fn new_commands_run_and_explain_missing_functions() {
    for (path, expect) in [
        (&["Bildschirmfoto"][..], Some("Bildschirmfoto: Firepixel kann den Bildschirm nicht selbst aufnehmen.")),
        (&["Symbolskript erstellen"], Some("Symbolskript erstellen: Wählen Sie zuerst eine Instanz eines Grafiksymbols aus.")),
        (&["Kreativ", "Autom. Vektormaske"], Some("Autom. Vektormaske: Wählen Sie zuerst ein oder mehrere Objekte aus.")),
        (&["Web", "Gemeinsame Palette"], Some("Gemeinsame Palette: Firepixel legt keine eigene gemeinsame Farbtabelle")),
        (&["Demo - aktuelles Dokument"], None),
    ] {
        let (mut st, ctx, _f) = fixture();
        let _ = st.session.execute("select.set", json!({ "ids": [] }));
        let before = doc_json(&st);
        let p = script_params(&st, path);
        run(&mut st, &ctx, "commands.runScript", p);
        let msg = st.dialogs.message.clone();
        match expect {
            Some(text) => {
                let (title, body) = msg.unwrap_or_else(|| panic!("{path:?}: Meldung erwartet, Status {:?}", st.status));
                assert_eq!(title, "Firepixel");
                assert!(body.starts_with(text), "{path:?}: {body}");
            }
            None => assert!(msg.is_none(), "{path:?}: keine Meldung: {msg:?}"),
        }
        assert_eq!(doc_json(&st), before, "{path:?}: Dokument unverändert");
    }
}

#[test]
fn symbol_script_names_selected_symbol() {
    let (mut st, ctx, f) = fixture();
    let _ = st.session.execute("select.set", json!({ "ids": f.instance }));
    let p = script_params(&st, &["Symbolskript erstellen"]);
    run(&mut st, &ctx, "commands.runScript", p);
    let (_, body) = st.dialogs.message.clone().expect("Meldung");
    assert!(body.contains("Für das Symbol „"), "{body}");
}

#[test]
fn auto_vector_mask_masks_selection_one_undo_step() {
    let (mut st, ctx, f) = fixture();
    let _ = st.session.execute("select.set", json!({ "ids": f.rect }));
    let before = doc_json(&st);
    let n = st.session.active().unwrap().history.undo_count();
    let p = script_params(&st, &["Kreativ", "Autom. Vektormaske"]);
    run(&mut st, &ctx, "commands.runScript", p);
    assert!(st.dialogs.message.is_none(), "{:?}", st.dialogs.message);
    let d = st.session.active().unwrap();
    let sel = d.selection.elements.clone();
    assert_eq!(sel.len(), 1, "eine Maskengruppe ausgewählt");
    let e = fp_engine::docops::element_on_page(&d.doc, d.page, sel[0]).expect("Element");
    match &e.kind {
        fp_model::ElementKind::Group(g) => {
            assert!(matches!(g.kind, fp_model::element::GroupKind::VectorMask), "Vektormaske: {:?}", g.kind);
            assert!(g.mask.is_some(), "Maske vorhanden");
        }
        other => panic!("Gruppe erwartet: {other:?}"),
    }
    assert_eq!(d.history.undo_count(), n + 1, "genau ein Verlaufsschritt");
    st.session.undo().expect("Rückgängig");
    assert_eq!(doc_json(&st), before, "Rückgängig stellt exakt her");
}

// ------------------------------------------------------------------ Bestehende Befehlsordner

#[test]
fn existing_folder_renames_unchanged_and_adds_new_commands_once() {
    use fp_script::builtin_commands::{install_builtin_commands_in, BUILTIN, MARKER_FILE};
    let dir = temp_dir().join("Commands");
    std::fs::create_dir_all(dir.join("Text")).unwrap();
    std::fs::create_dir_all(dir.join("Kreativ")).unwrap();
    let src = |rel: &str| BUILTIN.iter().find(|(r, _)| *r == rel).map(|(_, s)| *s).unwrap();
    // eingerichteter Ordner einer früheren Version: alte Namen, ein Befehl vom Nutzer geändert
    std::fs::write(dir.join("Warndialoge zurücksetzen.jsf"), src("Warnmeldungen zurücksetzen.jsf")).unwrap();
    std::fs::write(dir.join("Text").join("Blindtext.jsf"), src("Text/Lorem ipsum.jsf")).unwrap();
    std::fs::write(dir.join("Kreativ").join("Verdrehen und ausblenden.jsf"), "// vom Nutzer geändert\n").unwrap();
    std::fs::write(dir.join(MARKER_FILE), "0.1.0").unwrap();
    install_builtin_commands_in(&dir, "de").unwrap();
    assert!(dir.join("Warnmeldungen zurücksetzen.jsf").exists() && !dir.join("Warndialoge zurücksetzen.jsf").exists(), "umbenannt");
    assert!(dir.join("Text").join("Lorem ipsum.jsf").exists() && !dir.join("Text").join("Blindtext.jsf").exists(), "umbenannt");
    assert_eq!(std::fs::read_to_string(dir.join("Kreativ").join("Verdrehen und ausblenden.jsf")).unwrap(), "// vom Nutzer geändert\n", "Nutzeränderung bleibt");
    assert!(!dir.join("Kreativ").join("Drehen und Ausblenden.jsf").exists(), "kein Doppel neben der Nutzerfassung");
    for rel in ["Bildschirmfoto.jsf", "Demo - aktuelles Dokument.jsf", "Symbolskript erstellen.jsf", "Kreativ/Autom. Vektormaske.jsf", "Web/Gemeinsame Palette.jsf"] {
        assert!(dir.join(rel).exists(), "{rel} nachgetragen");
    }
    // nur alte Befehle wurden nicht neu installiert (die Markierung bleibt maßgeblich)
    assert!(!dir.join("Text").join("Verfestigen.jsf").exists());
    // gelöschte neue Befehle kommen nicht wieder
    std::fs::remove_file(dir.join("Bildschirmfoto.jsf")).unwrap();
    assert_eq!(install_builtin_commands_in(&dir, "de").unwrap(), 0);
    assert!(!dir.join("Bildschirmfoto.jsf").exists());
}

// ------------------------------------------------------------------ Kürzeltexte

#[test]
fn shortcut_texts_acute_and_circumflex() {
    let (st, _ctx, _f) = fixture();
    let view = menu_nodes(&st, "Ansicht");
    assert_eq!(shortcut(&view, &["QuickInfos"]).as_deref(), Some("Strg+AKUT"));
    assert_eq!(shortcut(&view, &["Intelligente Hilfslinien", "Intelligente Hilfslinien anzeigen"]).as_deref(), Some("Strg+ZIRKUMFLEX"));
    assert_eq!(shortcut(&view, &["Intelligente Hilfslinien", "An intelligenten Hilfslinien ausrichten"]).as_deref(), Some("Strg+Umschalt+ZIRKUMFLEX"));
    // die Anzeige lässt sich wieder lesen
    let p = fp_engine::shortcuts::Shortcut::parse("Strg+AKUT").expect("lesbar");
    assert_eq!((p.ctrl, p.vk), (true, 221));
    assert_eq!(fp_engine::shortcuts::Shortcut::parse("Ctrl+Shift+Circumflex").map(|s| (s.shift, s.vk)), Some((true, 220)));
    // Englisch
    fp_i18n::set_language("en");
    let view = menu_nodes(&st, "View");
    assert_eq!(shortcut(&view, &["Tooltips"]).as_deref(), Some("Ctrl+Acute"));
    fp_i18n::set_language("de");
}

// ------------------------------------------------------------------ Filter wiederholen

fn repeat_label(st: &AppState) -> (String, bool) {
    let nodes = menu_nodes(st, "Filter");
    nodes
        .iter()
        .find_map(|n| match n {
            Node::Item { label, action: ItemAction::Command(c), enabled, .. } if c == "filter.repeat" => Some((label.clone(), *enabled)),
            _ => None,
        })
        .expect("Filter wiederholen")
}

#[test]
fn repeat_filter_shows_last_filter_name() {
    let (mut st, _ctx, f) = fixture();
    assert_eq!(repeat_label(&st).0, "Filter wiederholen");
    let _ = st.session.execute("select.set", json!({ "ids": f.bitmap }));
    st.session.execute("filter.adjustColor.invert", json!({})).expect("Filter");
    let (label, enabled) = repeat_label(&st);
    assert_eq!(label, "Wiederholen: Umkehren"); // Wortlaut des Originals seit MN-5
    assert!(enabled);
    st.session.execute("filter.blur.blur", json!({})).expect("Filter");
    let name = menu::clean_label(&fp_i18n::t(&fp_i18n::command_key("filter.blur.blur"))).0;
    assert_eq!(repeat_label(&st).0, format!("Wiederholen: {name}"));
    // Wiederholen selbst ändert den Namen nicht
    st.session.execute("filter.repeat", json!({})).expect("wiederholen");
    assert_eq!(repeat_label(&st).0, format!("Wiederholen: {name}"));
}

// ------------------------------------------------------------------ Datei ▸ Im Dateibrowser durchsuchen

#[test]
fn browse_entry_is_neutral_and_works() {
    let (st, _ctx, _f) = fixture();
    let nodes = menu_nodes(&st, "Datei");
    match find(&nodes, &["Im Dateibrowser durchsuchen..."]) {
        Some(Node::Item { enabled, shortcut, action: ItemAction::Command(c), .. }) => {
            assert!(*enabled);
            assert_eq!(c, "file.browseInBridge");
            assert_eq!(shortcut.as_deref(), Some("Strg+Alt+O"));
        }
        other => panic!("{other:?}"),
    }
    assert!(find(&nodes, &["Bridge durchsuchen..."]).is_none());
}
