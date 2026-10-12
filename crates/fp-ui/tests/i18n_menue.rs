//! Mehrsprachigkeit der Menüs, Befehle und Werkzeuge (Phase 6, I18N-MENUE):
//! * Hauptmenü, Kontextmenüs der Leinwand und Bedienfeldmenüs in de, en und qps vollständig, ohne
//!   Rohschlüssel; Deutsch genau wie die Ressource des Originals,
//! * Zugriffstasten je Menü eindeutig (Englisch); Deutsch nur mit den Doppelungen des Originals,
//! * Sprachwechsel zur Laufzeit ändert die Menüs sofort, Alt+Buchstabe in beiden Sprachen,
//! * alle Befehle des Registers und alle Werkzeuge haben Texte in de und en,
//! * englische Menüleiste und Werkzeugbedienfeld bei 1024×768 ohne Überlauf.

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{self, ItemAction, Node};
use fp_ui::FirepixelApp;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// Die Sprache ist global: Tests dieser Datei laufen nacheinander.
static LANG: Mutex<()> = Mutex::new(());

/// Sperre für die Sprache; stellt am Ende (auch nach einem Fehlschlag) Deutsch wieder her.
struct LangGuard(#[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

impl Drop for LangGuard {
    fn drop(&mut self) {
        fp_i18n::set_language("de");
    }
}

fn lock() -> LangGuard {
    let g = LANG.lock().unwrap_or_else(|e| e.into_inner());
    fp_i18n::set_language("de");
    LangGuard(g)
}

fn make_app(doc: bool) -> FirepixelApp {
    let mut session = fp_engine::Session::default();
    if doc {
        let _ = session.execute("file.new", serde_json::json!({ "width": 400, "height": 300 }));
    }
    let ctx = egui::Context::default();
    FirepixelApp::with_session(&ctx, session, UiSettings::default(), false)
}

fn harness(app: FirepixelApp, w: f32, h: f32) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(w, h)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

/// Sieht aus wie ein nicht übersetzter Schlüssel (`cmd-file-open`, `menu-…`, `tool-…`).
fn is_raw_key(s: &str) -> bool {
    let prefixes = ["cmd-", "menu-", "ctx-", "pmenu-", "tool-", "shape-", "panel-", "ui-", "dlg-"];
    prefixes.iter().any(|p| s.starts_with(p)) && !s.contains(' ') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

/// Einträge mit Laufzeitinhalt (Dateinamen, Schriften, Dokumente, Arbeitsbereiche): nicht übersetzt.
fn is_dynamic(n: &Node) -> bool {
    match n {
        Node::Item { action, .. } => match action {
            ItemAction::Ui(fp_ui::app_state::Action::OpenPath(_)) | ItemAction::Ui(fp_ui::app_state::Action::Activate(_)) => true,
            // Sprachnamen sind Eigennamen („Deutsch“, „English“)
            ItemAction::Ui(fp_ui::app_state::Action::Run(name, p)) => name == "commands.runScript" || name == "text.font" || (name == "ui.setLanguage" && p.get("code").and_then(|c| c.as_str()) != Some(fp_i18n::PSEUDO_LANG)),
            _ => false,
        },
        Node::Sub { children, .. } => !children.is_empty() && children.iter().all(|c| matches!(c, Node::Separator) || is_dynamic(c)),
        Node::Separator => false,
    }
}

/// Alle Beschriftungen (Pfad, Text, dynamisch?) eines Menübaums.
fn labels(nodes: &[Node], prefix: &str, skip_sub: &str, out: &mut Vec<(String, String, bool)>) {
    for n in nodes {
        match n {
            Node::Separator => {}
            Node::Item { label, .. } => out.push((format!("{prefix}/{label}"), label.clone(), is_dynamic(n))),
            Node::Sub { label, children, .. } => {
                out.push((format!("{prefix}/{label}"), label.clone(), is_dynamic(n)));
                if label != skip_sub {
                    labels(children, &format!("{prefix}/{label}"), skip_sub, out);
                }
            }
        }
    }
}

fn main_menu_labels(app: &FirepixelApp) -> Vec<(String, String, bool)> {
    let skip = menu::clean_label(&fp_i18n::t("menu-window-workspaceLayouts")).0;
    let mut out = Vec::new();
    for (title, nodes) in menu::build_main_menu(&app.state) {
        out.push((format!("/{title}"), title.clone(), false));
        labels(&nodes, &format!("/{title}"), &skip, &mut out);
    }
    out
}

const CANVAS_MENUS: [&str; 9] = ["6906", "6910", "6912", "6913", "6914", "6915", "6916", "6925", "49120"];

fn context_labels(app: &FirepixelApp) -> Vec<(String, String, bool)> {
    let mut out = Vec::new();
    for id in CANVAS_MENUS {
        let nodes = fp_ui::canvas::context::build(&app.state, fp_ui::canvas::context::entries(id));
        labels(&nodes, &format!("ctx{id}"), "", &mut out);
    }
    for p in fp_ui::panels::PanelId::ALL {
        let nodes = menu::build_panel_menu(&app.state, *p);
        labels(&nodes, &format!("panel{p:?}"), "", &mut out);
    }
    out
}

#[test]
fn menus_complete_in_de_en_qps_without_raw_keys() {
    let _g = lock();
    let app = make_app(true);
    let mut per_lang: BTreeMap<&str, Vec<(String, String, bool)>> = BTreeMap::new();
    for lang in ["de", "en", "qps"] {
        assert!(fp_i18n::set_language(lang));
        let mut all = main_menu_labels(&app);
        all.extend(context_labels(&app));
        let bad: Vec<&(String, String, bool)> = all.iter().filter(|(_, l, _)| l.trim().is_empty() || is_raw_key(l) || l.contains("TODO")).collect();
        assert!(bad.is_empty(), "{lang}: leere Texte oder Rohschlüssel: {bad:#?}");
        per_lang.insert(lang, all);
    }
    fp_i18n::set_language("de");
    let de = &per_lang["de"];
    let en = &per_lang["en"];
    let qps = &per_lang["qps"];
    // gleicher Aufbau in allen Sprachen
    assert_eq!(de.len(), en.len(), "Zahl der Einträge de/en");
    assert_eq!(de.len(), qps.len(), "Zahl der Einträge de/qps");
    // Englisch: übersetzt (gleich nur bei Eigennamen, Zahlen und Fachwörtern, die in beiden Sprachen gleich sind)
    let same: BTreeSet<&str> = de.iter().zip(en.iter()).filter(|(d, e)| !d.2 && d.1 == e.1 && d.1.chars().any(|c| c.is_alphabetic())).map(|(d, _)| d.1.as_str()).collect();
    let allowed: BTreeSet<&str> = [
        "Text", "Filter", "Symbol", "Hotspot", "Info", "URL", "RGB", "CMY", "HSB", "Normal", "Animation", "Polygon", "Zoom", "Firepixel Support Center", "Firepixel Exchange", "OLED", "jQuery", "Gamma 1.8", "Standard",
    ]
    .into_iter()
    .collect();
    let unexpected: Vec<&&str> = same.iter().filter(|s| !allowed.contains(**s)).collect();
    assert!(unexpected.is_empty(), "englisch unverändert deutsch: {unexpected:?}");
    // Testsprache: jeder feste Text markiert (sonst wäre er fest eingebaut)
    let unmarked: Vec<&(String, String, bool)> = qps.iter().filter(|(_, l, dynamic)| !dynamic && l.chars().any(|c| c.is_alphabetic()) && !l.starts_with("[!!")).collect();
    assert!(unmarked.is_empty(), "qps: nicht markierte (fest eingebaute) Texte: {unmarked:#?}");
}

#[test]
fn german_menu_looks_exactly_like_the_resource() {
    let _g = lock();
    fp_i18n::set_language("de");
    let app = make_app(true);
    // Titel und Einträge wie im Original (Stichproben über alle Menüs)
    let titles: Vec<String> = menu::build_main_menu(&app.state).into_iter().map(|(t, _)| t).collect();
    assert_eq!(titles, ["Datei", "Bearbeiten", "Ansicht", "Auswählen", "Modifizieren", "Text", "Befehle", "Filter", "Fenster", "Hilfe"]);
    // jeder Eintrag der Ressource: deutscher Text = Ausgangstext in menu_main.json
    fn walk(es: &[menu::MenuEntry], bad: &mut Vec<String>) {
        for e in es {
            if e.sep {
                continue;
            }
            let de = e.text.split('\t').next().unwrap_or("");
            let shown = e.local_text();
            let shown = shown.split('\t').next().unwrap_or("");
            if de.trim() != shown.trim() {
                bad.push(format!("{de:?} → {shown:?} ({})", e.key));
            }
            if let Some(c) = &e.children {
                walk(c, bad);
            }
        }
    }
    let mut bad = Vec::new();
    walk(menu::main_menu(), &mut bad);
    for id in ["6919", "6920", "6921", "6922", "6923", "6924", "6932", "6933", "6935", "6937", "6939", "6940", "6941", "6942", "7026", "49120"] {
        if let Some(es) = menu::context_menu(id) {
            // Kontext-/Bedienfeldmenüs zeigen keine Zugriffstasten: bereinigt vergleichen
            for e in es {
                if e.sep {
                    continue;
                }
                let a = menu::clean_label(&e.text).0;
                let b = menu::clean_label(&e.local_text()).0;
                if a.trim() != b.trim() {
                    bad.push(format!("{id}: {a:?} → {b:?}"));
                }
            }
        }
    }
    for id in CANVAS_MENUS {
        fn walk_clean(es: &[menu::MenuEntry], id: &str, bad: &mut Vec<String>) {
            for e in es {
                if e.sep {
                    continue;
                }
                let a = menu::clean_label(&e.text).0;
                let b = menu::clean_label(&e.local_text()).0;
                if a.trim() != b.trim() {
                    bad.push(format!("{id}: {a:?} → {b:?}"));
                }
                if let Some(c) = &e.children {
                    walk_clean(c, id, bad);
                }
            }
        }
        walk_clean(fp_ui::canvas::context::entries(id), id, &mut bad);
    }
    assert!(bad.is_empty(), "deutscher Wortlaut weicht ab: {bad:#?}");
    // Werkzeugtipps und Symbolleiste wie bisher
    assert_eq!(fp_ui::tools_panel::tool_label(fp_engine::ToolId::PolygonLasso), "Polygon-Lasso");
    assert_eq!(menu::command_label(&app.state, "modify.align.distributeHeights"), "Höhen verteilen");
    assert_eq!(menu::command_label(&app.state, "file.open"), "Öffnen");
    assert_eq!(menu::command_label(&app.state, "filter.blur.zoom"), "Zoom-Weichzeichner");
    assert_eq!(menu::command_path(&app.state, "modify.align.left"), "Modifizieren ▸ Ausrichten ▸ Links");
    // Laufzeit-Texte genau wie im Original (Parität UX-B, docs\PARITAET_B.md §1)
    let menus = menu::build_main_menu(&app.state);
    let sub = |top: &str, name: &str| -> Vec<String> {
        let nodes = &menus.iter().find(|(t, _)| t == top).expect("Menü").1;
        nodes
            .iter()
            .find_map(|n| match n {
                Node::Sub { label, children, .. } if label == name => Some(children.iter().filter_map(|c| if let Node::Item { label, .. } = c { Some(label.clone()) } else { None }).collect()),
                _ => None,
            })
            .expect("Untermenü")
    };
    assert_eq!(sub("Filter", "Andere"), ["In Alphakanal konv.", "Ränder suchen"]);
    assert_eq!(sub("Filter", "Weichzeichnen"), ["Bewegungsunschärfe…", "Gaußscher Weichz. ...", "Radiales Weichz. ...", "Stärker weichzeichnen", "Weichzeichnen", "Zentriertes Weichz. ..."]);
    let help = sub_items(&menus, "Hilfe");
    assert!(help.contains(&"Firepixel-Programm zur Produktverbesserung...".to_string()) && help.contains(&"Firepixel-Onlineforen".to_string()), "{help:?}");
    let browser = sub("Datei", "Vorschau im Browser");
    assert!(browser[0].starts_with("Vorschau in ") && browser[2].starts_with("Vorschau aller Seiten in ") && browser[2].ends_with(" anzeigen"), "{browser:?}");
    assert_eq!(browser[1], "Vorschau in", "ohne Sekundärbrowser wie im Original");
}

fn sub_items(menus: &[(String, Vec<Node>)], top: &str) -> Vec<String> {
    menus.iter().find(|(t, _)| t == top).map(|(_, ns)| ns.iter().filter_map(|n| if let Node::Item { label, .. } = n { Some(label.clone()) } else { None }).collect()).unwrap_or_default()
}

/// Zugriffstasten je Menüebene: (Pfad, [(Taste, Beschriftung)]).
fn access_levels(app: &FirepixelApp) -> Vec<(String, Vec<(char, String)>)> {
    fn walk(nodes: &[Node], path: &str, out: &mut Vec<(String, Vec<(char, String)>)>) {
        let mut level = Vec::new();
        for n in nodes {
            let (label, children) = match n {
                Node::Item { label, .. } => (label, None),
                Node::Sub { label, children, .. } => (label, Some(children)),
                Node::Separator => continue,
            };
            let p = format!("{path}/{label}");
            if let Some((_, c)) = fp_ui::menu_kbd::access_of(&p) {
                level.push((c, label.clone()));
            }
            if let Some(c) = children {
                walk(c, &p, out);
            }
        }
        out.push((path.to_string(), level));
    }
    let menus = menu::build_main_menu(&app.state);
    let mut out = Vec::new();
    let mut bar = Vec::new();
    for (title, nodes) in &menus {
        if let Some((_, c)) = fp_ui::menu_kbd::access_of(&format!("/{title}")) {
            bar.push((c, title.clone()));
        }
        walk(nodes, &format!("/{title}"), &mut out);
    }
    out.push(("<Menüleiste>".to_string(), bar));
    out
}

fn duplicates(levels: &[(String, Vec<(char, String)>)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, level) in levels {
        let mut by: BTreeMap<char, Vec<&str>> = BTreeMap::new();
        for (c, l) in level {
            by.entry(*c).or_default().push(l);
        }
        for (c, ls) in by {
            if ls.len() > 1 {
                out.push(format!("{path} '{c}': {}", ls.join(" | ")));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn access_keys_unique_per_menu() {
    let _g = lock();
    let app = make_app(true);
    fp_i18n::set_language("en");
    let en = access_levels(&app);
    let titles: Vec<char> = en.iter().find(|(p, _)| p == "<Menüleiste>").map(|(_, l)| l.iter().map(|x| x.0).collect()).unwrap_or_default();
    assert_eq!(titles, ['f', 'e', 'v', 's', 'm', 't', 'c', 'r', 'w', 'h'], "englische Menüleiste: &File &Edit &View &Select &Modify &Text &Commands Filte&rs &Window &Help");
    let dup_en = duplicates(&en);
    assert!(dup_en.is_empty(), "englische Zugriffstasten doppelt: {dup_en:#?}");
    // Deutsch: nur die Doppelungen, die schon das Original hat (Taste springt dann weiter, wie unter Windows)
    fp_i18n::set_language("de");
    let de = access_levels(&app);
    let dup_de = duplicates(&de);
    let known = [
        "/Ansicht 'e': Intelligente Hilfslinien | Ränder",
        "/Ansicht/Raster 'r': Raster anzeigen | Raster bearbeiten...",
        "/Auswählen 'w': Alles auswählen | Weiche Kante…",
        "/Bearbeiten 'e': Einfügen | Innen einfügen",
        "/Bearbeiten/Objekt einfügen 's': Neue Schaltfläche... | Status",
        "/Datei 'l': Alles speichern | HTML-Einrichtung...",
        "/Datei/Vorschau im Browser 'p': Vorschau im Primärbrowser | Primärbrowser festlegen...",
        "/Fenster 's': Stile | Suchen und ersetzen",
        "/Fenster 'u': Status | URL",
        "/Fenster 'z': Fenster duplizieren | Werkzeuge",
        "/Modifizieren/Ausrichten 'v': Vertikal zentrieren | Höhen verteilen",
    ];
    let unknown: Vec<&String> = dup_de.iter().filter(|d| !known.contains(&d.as_str())).collect();
    assert!(unknown.is_empty(), "neue deutsche Doppelungen: {unknown:#?}");
    // Testsprache hat dieselben Tasten wie Deutsch
    fp_i18n::set_language("qps");
    let qps = access_levels(&app);
    let keys = |v: &[(String, Vec<(char, String)>)]| v.iter().map(|(_, l)| l.iter().map(|x| x.0).collect::<String>()).collect::<Vec<_>>();
    assert_eq!(keys(&de), keys(&qps), "qps: Zugriffstasten wie Deutsch");
    fp_i18n::set_language("de");
}

fn ev(h: &mut Harness<'static, FirepixelApp>, e: Event) {
    h.input_mut().events.push(e);
    h.run_steps(1);
}

fn alt_letter(h: &mut Harness<'static, FirepixelApp>, key: Key) {
    ev(h, Event::ModifiersChanged(Modifiers::ALT));
    ev(h, Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: Modifiers::ALT });
    ev(h, Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers: Modifiers::ALT });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
}

fn tap(h: &mut Harness<'static, FirepixelApp>, key: Key) {
    ev(h, Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key, physical_key: Some(key), pressed: false, repeat: false, modifiers: Modifiers::NONE });
}

#[test]
fn language_switch_changes_menus_and_keyboard_at_once() {
    let _g = lock();
    fp_i18n::set_language("de");
    let mut h = harness(make_app(true), 1500.0, 950.0);
    h.run_steps(2);
    let title = |h: &Harness<'static, FirepixelApp>, i: usize| menu::build_main_menu(&h.state().state)[i].0.clone();
    assert_eq!(title(&h, 0), "Datei");
    // Alt+D öffnet „Datei“
    alt_letter(&mut h, Key::D);
    let k = fp_ui::menu_kbd::state(&h.ctx);
    assert!(k.active && k.top == 0 && !k.path.is_empty(), "Alt+D: {k:?}");
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::Escape);
    // Sprachwechsel zur Laufzeit (wie Ansicht ▸ Sprache): nächstes Bild englisch
    assert!(fp_i18n::set_language("en"));
    h.run_steps(1);
    assert_eq!(title(&h, 0), "File");
    assert_eq!(title(&h, 7), "Filters");
    // Alt+F öffnet jetzt „File“, Alt+R „Filters“; Alt+D tut nichts mehr
    alt_letter(&mut h, Key::F);
    let k = fp_ui::menu_kbd::state(&h.ctx);
    assert!(k.active && k.top == 0 && !k.path.is_empty(), "Alt+F: {k:?}");
    // im offenen Menü: Zugriffstaste des ersten aktiven Untermenüs öffnet es (keine Befehle
    // ausführen: „Open…“ würde den Dateidialog des Systems öffnen)
    let menus = menu::build_main_menu(&h.state().state);
    let (sub_label, sub_key) = menus[0]
        .1
        .iter()
        .find_map(|n| match n {
            Node::Sub { label, enabled: true, .. } => fp_ui::menu_kbd::access_of(&format!("/File/{label}")).map(|a| (label.clone(), a.1)),
            _ => None,
        })
        .expect("aktives Untermenü mit Zugriffstaste in File");
    let key = Key::from_name(&sub_key.to_ascii_uppercase().to_string()).expect("Buchstabentaste");
    tap(&mut h, key);
    let k = fp_ui::menu_kbd::state(&h.ctx);
    assert!(k.active && k.path.len() == 2, "{sub_key} in File: Untermenü „{sub_label}“: {k:?}");
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::Escape);
    alt_letter(&mut h, Key::R);
    let k = fp_ui::menu_kbd::state(&h.ctx);
    assert!(k.active && k.top == 7, "Alt+R: {k:?}");
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::Escape);
    alt_letter(&mut h, Key::D);
    let k = fp_ui::menu_kbd::state(&h.ctx);
    assert!(!(k.active && !k.path.is_empty()), "Alt+D im Englischen: {k:?}");
    // zurück auf Deutsch, ohne Neustart
    assert!(fp_i18n::set_language("de"));
    h.run_steps(1);
    assert_eq!(title(&h, 0), "Datei");
    // Werkzeugtipps folgen ebenfalls sofort
    let tip_de = fp_ui::tools_panel::tool_tooltip(&h.state().state, fp_engine::ToolId::Pen);
    fp_i18n::set_language("en");
    let tip_en = fp_ui::tools_panel::tool_tooltip(&h.state().state, fp_engine::ToolId::Pen);
    fp_i18n::set_language("de");
    assert!(tip_de.starts_with("Stift") && tip_en.starts_with("Pen"), "{tip_de} / {tip_en}");
}

#[test]
fn duplicate_german_access_key_cycles_instead_of_running() {
    let _g = lock();
    fp_i18n::set_language("de");
    let mut h = harness(make_app(true), 1500.0, 950.0);
    h.run_steps(2);
    // Datei: „A&lles speichern“ und „HTM&L-Einrichtung…“ teilen sich L
    alt_letter(&mut h, Key::D);
    tap(&mut h, Key::L);
    let a = fp_ui::menu_kbd::state(&h.ctx);
    tap(&mut h, Key::L);
    let b = fp_ui::menu_kbd::state(&h.ctx);
    tap(&mut h, Key::Escape);
    tap(&mut h, Key::Escape);
    assert!(a.active && b.active, "L darf nichts ausführen: {a:?} {b:?}");
    assert_ne!(a.path, b.path, "zweites L springt zum nächsten Eintrag");
}

/// Befund UX-B (Parität): „Rückgängig: <Schritt>“, Löschen zeigt Entf, Alt+B ▸ R bleibt bedienbar.
#[test]
fn undo_shows_step_name_and_delete_key() {
    let _g = lock();
    let mut app = make_app(true);
    let _ = app.state.session.execute("draw.rectangle", serde_json::json!({ "rect": [10, 10, 60, 40] }));
    let find = |app: &FirepixelApp, cmd: &str| -> Option<(String, Option<String>)> {
        fn walk(ns: &[Node], cmd: &str) -> Option<(String, Option<String>)> {
            ns.iter().find_map(|n| match n {
                Node::Item { label, shortcut, action: ItemAction::Command(c), .. } if c == cmd => Some((label.clone(), shortcut.clone())),
                Node::Sub { children, .. } => walk(children, cmd),
                _ => None,
            })
        }
        menu::build_main_menu(&app.state).iter().find_map(|(_, ns)| walk(ns, cmd))
    };
    let step = app.state.session.active().and_then(|d| d.history.undo_label().map(str::to_string)).expect("Verlaufsschritt");
    let (undo, _) = find(&app, "edit.undo").expect("Rückgängig im Menü");
    assert_eq!(undo, format!("Rückgängig: {step}"));
    // Zugriffstaste bleibt die von „&Rückgängig“
    assert_eq!(fp_ui::menu_kbd::access_of(&format!("/Bearbeiten/{undo}")).map(|a| a.1), Some('r'));
    let (_, del) = find(&app, "edit.clear").expect("Löschen im Menü");
    assert_eq!(del, Some(fp_i18n::shortcut_label("Entf")), "Löschen zeigt die Entf-Taste wie das Original");
    let (_, guides) = find(&app, "view.guides.showGuides").expect("Hilfslinien anzeigen");
    assert_eq!(guides.as_deref(), Some("Strg+ü"), "Umlaut-Taste klein wie im Original");
    fp_i18n::set_language("en");
    let (undo_en, _) = find(&app, "edit.undo").expect("Undo");
    assert!(undo_en.starts_with("Undo: "), "{undo_en}");
    assert_eq!(fp_ui::menu_kbd::access_of(&format!("/Edit/{undo_en}")).map(|a| a.1), Some('u'));
    let (_, del_en) = find(&app, "edit.clear").expect("Clear");
    assert_eq!(del_en, Some(fp_i18n::shortcut_label("Del")));
}

#[test]
fn every_command_and_tool_has_texts_in_de_and_en() {
    let _g = lock();
    let app = make_app(false);
    let mut missing = Vec::new();
    for c in app.state.session.registry().iter() {
        let key = fp_i18n::command_key(&c.info().name);
        for lang in ["de", "en"] {
            if !fp_i18n::keys(lang).contains(&key) {
                missing.push(format!("{lang}: {key}"));
            }
        }
        if c.info().description.is_some() && !fp_i18n::keys("en").contains(&format!("{key}.description")) {
            missing.push(format!("en: {key}.description"));
        }
    }
    for t in fp_engine::tools::TOOLS {
        let key = t.name.replace(['.', '_'], "-");
        for lang in ["de", "en"] {
            if !fp_i18n::keys(lang).contains(&key) {
                missing.push(format!("{lang}: {key}"));
            }
        }
    }
    for (de, en) in fp_ui::tools_panel::AUTO_SHAPES {
        let key = fp_ui::tools_panel::shape_key(en);
        if !fp_i18n::keys("de").contains(&key) {
            missing.push(format!("de: {key} ({de})"));
        }
    }
    assert!(missing.is_empty(), "fehlende Texte: {missing:#?}");
    // Deutsch: Werkzeugnamen = Namen der Werkzeugtabelle (fp-engine), Befehle = Beschriftung des Registers
    fp_i18n::set_language("de");
    let tools_bad: Vec<String> = fp_engine::tools::TOOLS.iter().filter(|t| fp_ui::tools_panel::tool_label(t.id) != t.label).map(|t| format!("{} ≠ {}", fp_ui::tools_panel::tool_label(t.id), t.label)).collect();
    assert!(tools_bad.is_empty(), "{tools_bad:?}");
    let cmd_bad: Vec<String> = app
        .state
        .session
        .registry()
        .iter()
        .filter(|c| menu::command_label(&app.state, &c.info().name) != c.info().label)
        .map(|c| format!("{}: {} ≠ {}", c.info().name, menu::command_label(&app.state, &c.info().name), c.info().label))
        .collect();
    assert!(cmd_bad.is_empty(), "deutsche Beschriftung weicht vom Register ab: {cmd_bad:#?}");
}

#[test]
fn english_menu_bar_and_tools_panel_fit_at_1024x768() {
    let _g = lock();
    for lang in ["de", "en"] {
        fp_i18n::set_language(lang);
        let mut h = harness(make_app(true), 1024.0, 768.0);
        h.run_steps(3);
        let fit: Option<(usize, usize)> = h.ctx.data(|d| d.get_temp(egui::Id::new(menu::MENU_FIT_ID)));
        let (fit, total) = fit.expect("Menüleiste gezeichnet");
        assert_eq!(fit, total, "{lang}: Menüleiste läuft bei 1024×768 über ({fit} von {total} Titeln passen)");
        let elided: bool = h.ctx.data(|d| d.get_temp(egui::Id::new(fp_ui::tools_panel::SECTION_ELIDED_ID))).unwrap_or(false);
        assert!(!elided, "{lang}: Abschnittsüberschrift im Werkzeugbedienfeld gekürzt");
    }
    // Testsprache (≈ 40 % länger): Überlauf wird sauber abgefangen (»-Menü, gekürzte Überschriften), keine Panik
    fp_i18n::set_language("qps");
    let mut h = harness(make_app(true), 1024.0, 768.0);
    h.run_steps(3);
    let fit: Option<(usize, usize)> = h.ctx.data(|d| d.get_temp(egui::Id::new(menu::MENU_FIT_ID)));
    assert!(fit.is_some_and(|(f, t)| f <= t && t == 10));
    fp_i18n::set_language("de");
}
