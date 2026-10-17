//! Neutralität der Oberfläche (NT, `docs\AGENTEN_REGELN.md` §1a): offscreen mit egui_kittest.
//! Geprüft werden alle Texte, die tatsächlich gezeichnet werden (Text-Formen der Ausgabe), im
//! Hauptfenster, in jedem geöffneten Hauptmenü und im Dialog „Über Firepixel“, dazu der vollständige
//! Menübaum (auch Untermenüs), der Fenstertitel und die Statuszeile. Die verbotenen Wörter kennt nur
//! `fp-neutral` (als Hash).

use egui::epaint::Shape;
use egui::containers::Popup;
use egui_kittest::Harness;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{build_main_menu, Node};
use fp_ui::FirepixelApp;

fn bad(s: &str) -> bool {
    !fp_neutral::find_in_bytes(s.as_bytes()).is_empty()
}

fn texts(shape: &Shape, out: &mut Vec<String>) {
    match shape {
        Shape::Text(t) => out.push(t.galley.text().to_string()),
        Shape::Vec(v) => v.iter().for_each(|s| texts(s, out)),
        _ => {}
    }
}

fn drawn(h: &Harness<'static, FirepixelApp>) -> Vec<String> {
    let mut out = Vec::new();
    for c in &h.output().shapes {
        texts(&c.shape, &mut out);
    }
    out
}

fn walk(nodes: &[Node], path: &str, out: &mut Vec<String>) {
    for n in nodes {
        match n {
            Node::Separator => {}
            Node::Item { label, shortcut, .. } => {
                out.push(format!("{path} ▸ {label}"));
                if let Some(s) = shortcut {
                    out.push(format!("{path} ▸ {label} [{s}]"));
                }
            }
            Node::Sub { label, children, .. } => {
                out.push(format!("{path} ▸ {label}"));
                walk(children, &format!("{path} ▸ {label}"), out);
            }
        }
    }
}

fn app() -> FirepixelApp {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    let _ = app.state.session.execute("file.new", serde_json::json!({"width": 300, "height": 200}));
    app
}

fn harness(app: FirepixelApp) -> Harness<'static, FirepixelApp> {
    Harness::builder().with_size(egui::vec2(1600.0, 1000.0)).with_pixels_per_point(1.0).with_max_steps(8).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app)
}

#[test]
fn oberflaeche_ist_neutral() {
    let mut found: Vec<String> = Vec::new();
    let mut checked = 0usize;
    // Namen installierter Systemschriften (Schriftmenü, Schriftenliste) sind Daten des Rechners,
    // keine Texte von Firepixel; im gemeinsamen Testprogramm ist die Liste meist schon geladen (INF-1)
    let system_font = |s: &str| fp_ui::inspector::fonts::families().iter().any(|f| bad(&f.name) && s.contains(f.name.as_str()));
    let mut check = |what: &str, s: &str, found: &mut Vec<String>| {
        checked += 1;
        if bad(s) && !system_font(s) {
            found.push(format!("{what}: {s}"));
        }
    };

    // Menübaum (alle Ebenen) und Fenstertitel
    let a = app();
    let mut entries = Vec::new();
    let menus = build_main_menu(&a.state);
    for (title, nodes) in &menus {
        entries.push(title.clone());
        walk(nodes, title, &mut entries);
    }
    for e in &entries {
        check("Menü", e, &mut found);
    }
    check("Fenstertitel", &a.state.window_title(), &mut found);

    // Hauptfenster gezeichnet (Anwendungsleiste, Werkzeuge, Bedienfelder, Statuszeile)
    let mut h = harness(a);
    h.run_steps(3);
    let main = drawn(&h);
    assert!(main.iter().any(|t| t.contains("Datei")), "Hauptfenster ohne Menüleiste gezeichnet");
    for t in &main {
        check("Hauptfenster", t, &mut found);
    }
    // jedes Hauptmenü geöffnet
    for (i, menu) in menus.iter().enumerate() {
        Popup::open_id(&h.ctx, egui::Id::new(("fp-main-menu", i)));
        h.run_steps(2);
        for t in drawn(&h) {
            check(&format!("Menü {}", menu.0), &t, &mut found);
        }
        Popup::close_id(&h.ctx, egui::Id::new(("fp-main-menu", i)));
        h.run_steps(1);
    }
    // Dialog „Über Firepixel“
    h.state_mut().state.dialogs.about = true;
    h.run_steps(3);
    let about = drawn(&h);
    assert!(about.iter().any(|t| t.contains("Firepixel")), "Über-Dialog nicht gezeichnet");
    for t in &about {
        check("Über-Dialog", t, &mut found);
    }
    println!("Neutralität der Oberfläche: {checked} Texte geprüft ({} Menüeinträge), {} Fundstellen", entries.len(), found.len());
    assert!(found.is_empty(), "Name des Originals bzw. Herstellers in der Oberfläche:\n{}", found.join("\n"));
}
