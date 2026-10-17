//! S-4: Zugriffstasten (Alt+Buchstabe) der Formulardialoge nach `re\32` – D14 N/G/A/S, D18 P/X
//! (Einheit „Pixel“ rechts vom Feld), D21 I/T/B, D22 E/G/T/B (G → Farbknopf, grau → OK), D23 G.
//! OK/Abbrechen ohne Buchstaben; Alt+Buchstabe schreibt nichts ins Feld (CF-A).
//!
//! Ereignisse wie egui-winit unter Windows: Taste + Text je Zeichen (auch bei Alt).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::access;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200, "background": "#ffffff"})).expect("doc");
    let id = s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 100], "name": "R1"})).expect("rect").value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("Auswahl");
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

fn get(h: &H, key: &str) -> serde_json::Value {
    st(h).dialogs.form.as_ref().expect("Formular offen").get(key)
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn alt() -> Modifiers {
    Modifiers { alt: true, ..Default::default() }
}

/// Alt+Buchstabe wie egui-winit unter Windows (WM_SYSKEYDOWN + WM_SYSCHAR): Taste **und** Text.
fn alt_letter(h: &mut H, c: char) {
    let k = Key::from_name(&c.to_ascii_uppercase().to_string()).expect("Taste");
    h.event(Event::ModifiersChanged(alt()));
    h.event(Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: alt() });
    ev(h, Event::Text(c.to_string()));
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: alt() });
    ev(h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(3);
}

fn tab(h: &mut H) {
    ev(h, Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: true, repeat: false, modifiers: Modifiers::NONE });
    ev(h, Event::Key { key: Key::Tab, physical_key: Some(Key::Tab), pressed: false, repeat: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn open(h: &mut H, cmd: &str) {
    h.state_mut().state.run(cmd, json!({}));
    h.run_steps(4);
    assert!(st(h).dialogs.any_open(), "{cmd}: Dialog offen");
}

/// Rolle und Name (bzw. Wert) des fokussierten Bedienelements.
fn focused(h: &H) -> Option<(Role, String)> {
    h.ctx.memory(|m| m.focused())?;
    h.get_all(By::new()).find(|n| n.accesskit_node().is_focused()).map(|n| {
        let node = n.accesskit_node();
        (node.role(), node.label().map(|s| s.to_string()).or_else(|| node.value().map(|s| s.to_string())).unwrap_or_default())
    })
}

fn t(k: &str) -> String {
    fp_i18n::t(k)
}

/// Wert eines Schlüssels in einer ftl-Datei (einzeilig).
fn ftl(src: &str, key: &str) -> String {
    src.lines()
        .find_map(|l| l.strip_prefix(key).and_then(|r| r.trim_start().strip_prefix('=')).map(|v| v.trim().to_string()))
        .unwrap_or_else(|| panic!("{key} fehlt"))
}

fn letter(raw: &str) -> char {
    access::split_access(raw).map(|(_, c)| c.to_ascii_uppercase()).unwrap_or_else(|| panic!("keine Taste in {raw}"))
}

type Keys = &'static [(&'static str, char, char)];

/// Buchstaben nach `re\32` (DE) und die gewählten EN-Buchstaben; je Dialog keine Doppelbelegung.
#[test]
fn s4_letters_de_en() {
    let de = include_str!("../../fp-i18n/locales/de/dialogs.ftl");
    let en = include_str!("../../fp-i18n/locales/en/dialogs.ftl");
    let dialogs: [(&str, Keys); 5] = [
        ("D14", &[("dlg-lbl-name-ak", 'N', 'N'), ("dlg-symbol-graphic-ak", 'G', 'G'), ("dlg-symbol-animation-ak", 'A', 'A'), ("dlg-symbol-button-ak", 'S', 'B')]),
        ("D18", &[("dlg-grid-unit-h-ak", 'P', 'P'), ("dlg-grid-unit-v-ak", 'X', 'X')]),
        ("D21", &[("dlg-ccolor-white-ak", 'I', 'I'), ("dlg-ccolor-transparent-ak", 'T', 'T'), ("dlg-ccolor-custom-ak", 'B', 'C')]),
        ("D22", &[("dlg-md-enable-selective-ak", 'E', 'E'), ("dlg-md-overlay-color-ak", 'G', 'O'), ("dlg-md-preserve-text-ak", 'T', 'T'), ("dlg-md-preserve-buttons-ak", 'B', 'B')]),
        ("D23", &[("dlg-lbl-size-ak", 'G', 'S')]),
    ];
    for (dlg, keys) in dialogs {
        let mut seen_de = Vec::new();
        let mut seen_en = Vec::new();
        for (k, d, e) in keys {
            let (rd, re) = (ftl(de, k), ftl(en, k));
            assert_eq!(letter(&rd), *d, "{dlg} DE {k} = {rd}");
            assert_eq!(letter(&re), *e, "{dlg} EN {k} = {re}");
            assert!(!seen_de.contains(d) && !seen_en.contains(e), "{dlg}: doppelt belegt");
            seen_de.push(*d);
            seen_en.push(*e);
        }
    }
    // die Beschriftung ohne „&“ ist die bisherige (Namen für Tests und Barrierefreiheit unverändert)
    assert_eq!(ftl(de, "dlg-ccolor-white-ak"), "We&iß");
    assert_eq!(access::plain(&ftl(de, "dlg-md-overlay-color-ak")), ftl(de, "dlg-md-overlay-color"));
    assert_eq!(access::plain(&ftl(de, "dlg-ccolor-custom-ak")), ftl(de, "dlg-canvas-custom"));
    assert_eq!(access::plain(&ftl(en, "dlg-symbol-button-ak")), ftl(en, "dlg-symbol-button"));
    // OK/Abbrechen ohne Buchstaben
    assert!(!t("btn-ok").contains('&') && !t("btn-cancel").contains('&'));
}

/// D14: Alt+N → Namensfeld (Inhalt bleibt, kein „n“), Alt+A/S/G wählen und fokussieren das
/// Optionsfeld; Tab vom Optionsfeld → OK (ein Tab-Stopp für die Gruppe).
#[test]
fn s4_d14_name_and_type() {
    let mut h = harness();
    open(&mut h, "modify.symbol.convertToSymbol");
    let name0 = get(&h, "name");
    alt_letter(&mut h, 'a');
    assert_eq!(get(&h, "kind"), json!("animation"), "Alt+A: Animation");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-symbol-animation"))));
    alt_letter(&mut h, 's');
    assert_eq!(get(&h, "kind"), json!("button"), "Alt+S: Schaltfläche");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-symbol-button"))));
    alt_letter(&mut h, 'n');
    assert!(access::field_has_focus(&h.ctx, ("19104", 0usize)), "Alt+N: Namensfeld");
    alt_letter(&mut h, 'g');
    assert_eq!(get(&h, "kind"), json!("graphic"), "Alt+G: Grafik");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-symbol-graphic"))));
    assert_eq!(get(&h, "name"), name0, "kein Buchstabe im Namensfeld (CF-A)");
    tab(&mut h);
    assert_eq!(focused(&h), Some((Role::Button, t("btn-ok"))), "Tab: Grafik › OK");
}

/// D18: „Pixel“ rechts vom Feld; Alt+X → senkrechter Abstand, Alt+P → waagrechter; Werte bleiben.
#[test]
fn s4_d18_grid_units() {
    let mut h = harness();
    open(&mut h, "view.grid.editGrid");
    let (w0, h0) = (get(&h, "width"), get(&h, "height"));
    alt_letter(&mut h, 'x');
    assert!(access::field_has_focus(&h.ctx, ("5984", 2usize)), "Alt+X: Feld ↕");
    alt_letter(&mut h, 'p');
    assert!(access::field_has_focus(&h.ctx, ("5984", 1usize)), "Alt+P: Feld ↔");
    assert_eq!((get(&h, "width"), get(&h, "height")), (w0, h0), "nichts getippt (CF-A)");
    tab(&mut h);
    assert!(access::field_has_focus(&h.ctx, ("5984", 2usize)), "Tab: ↔ › ↕");
}

/// D21: Alt+T/B/I wählen Transparent/Benutzerdefiniert/Weiß und geben ihnen den Fokus.
#[test]
fn s4_d21_canvas_color() {
    let mut h = harness();
    open(&mut h, "modify.canvas.canvasColor");
    alt_letter(&mut h, 't');
    assert_eq!(get(&h, "_mode"), json!(1), "Alt+T");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-canvas-transparent"))));
    alt_letter(&mut h, 'b');
    assert_eq!(get(&h, "_mode"), json!(2), "Alt+B");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-canvas-custom"))));
    alt_letter(&mut h, 'i');
    assert_eq!(get(&h, "_mode"), json!(0), "Alt+I (nicht W)");
    assert_eq!(focused(&h), Some((Role::RadioButton, t("dlg-canvas-white"))));
    alt_letter(&mut h, 'w');
    assert_eq!(get(&h, "_mode"), json!(0), "Alt+W wirkt nicht");
    tab(&mut h);
    assert_eq!(focused(&h), Some((Role::Button, t("btn-ok"))), "Tab: Gruppe › OK");
}

/// D22: gesperrt wirken T/B nicht, Alt+G springt zu OK; Alt+E schaltet frei, dann T/B umschalten
/// und Alt+G fokussiert den Farbknopf (ohne Farbwahl).
#[test]
fn s4_d22_selective_jpeg() {
    let mut h = harness();
    open(&mut h, "modify.selectiveJpeg.settings");
    assert_eq!(get(&h, "enabled"), json!(false));
    let (pt, pb) = (get(&h, "preserveText"), get(&h, "preserveButtons"));
    alt_letter(&mut h, 't');
    alt_letter(&mut h, 'b');
    assert_eq!((get(&h, "preserveText"), get(&h, "preserveButtons")), (pt.clone(), pb.clone()), "gesperrt: keine Wirkung");
    alt_letter(&mut h, 'g');
    h.run_steps(3);
    assert_eq!(focused(&h), Some((Role::Button, t("btn-ok"))), "Alt+G bei grauem Farbknopf → OK");
    alt_letter(&mut h, 'e');
    assert_eq!(get(&h, "enabled"), json!(true), "Alt+E");
    alt_letter(&mut h, 't');
    assert_eq!(get(&h, "preserveText"), json!(!pt.as_bool().unwrap()), "Alt+T");
    alt_letter(&mut h, 'b');
    assert_eq!(get(&h, "preserveButtons"), json!(!pb.as_bool().unwrap()), "Alt+B");
    alt_letter(&mut h, 'g');
    assert!(h.ctx.memory(|m| m.has_focus(access::color_button_id(("19147", 2usize)))), "Alt+G: Farbknopf");
    assert!(st(&h).dialogs.any_open(), "Dialog bleibt offen");
}

/// D23: Alt+G → Feld, Wert unverändert.
#[test]
fn s4_d23_text_size() {
    let mut h = harness();
    open(&mut h, "text.size.other");
    let v0 = get(&h, "size");
    tab(&mut h);
    alt_letter(&mut h, 'g');
    assert!(access::field_has_focus(&h.ctx, ("6073", 0usize)), "Alt+G: Feld Größe");
    assert_eq!(get(&h, "size"), v0, "nichts getippt (CF-A)");
}
