//! Gesamttest der Tastenkürzel (HG, Phase 5): alle mitgelieferten Kürzelsätze.
//!
//! * Jedes Menükürzel eines Satzes ist auf der Tastatur erreichbar (egui-Taste → VK), zeigt auf
//!   einen registrierten Befehl, und `command_for_shortcut` liefert genau diesen Befehl zurück
//!   (Mehrfachbelegungen werden als Hinweis gezählt).
//! * Jedes Werkzeugkürzel wählt über `tool_shortcut` ein Werkzeug.
//! * Durchstich durch die echte Tastaturverarbeitung (egui_kittest): wichtige Kürzel lösen im
//!   Hauptfenster die erwartete Wirkung aus.
//!
//! Ergebnis: `docs\funktionstest\kuerzel_auto.md`.

mod gesamt_common;

use egui_kittest::Harness;
use fp_engine::{PrefValue, Shortcut};
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use gesamt_common::*;

fn egui_key_for_vk(vk: u16) -> Option<egui::Key> {
    // Ziffernblock: egui meldet dieselben Tasten wie die Ziffernreihe (input.rs probiert beides)
    let vk = match vk {
        0x60..=0x69 => 0x30 + (vk - 0x60),
        0x6B => return Some(egui::Key::Plus),
        0x6D => return Some(egui::Key::Minus),
        v => v,
    };
    egui::Key::ALL.iter().copied().find(|k| fp_ui::input::vk_of(*k) == Some(vk))
}

/// Drückt das Kürzel im Hauptfenster (egui_kittest) und prüft, ob sich irgendein Zustand ändert
/// (für Kürzel des Werkzeugbedienfelds, die kein Werkzeug wählen: Farben, Bildschirmmodus, Segmente).
fn key_has_effect(set: &str, sc: Shortcut) -> bool {
    let Some(key) = egui_key_for_vk(sc.vk) else { return false };
    let (mut st, _ctx, f) = fixture();
    st.session.prefs.set("CurrentShortcutSet", PrefValue::Str(set.to_string()));
    // Objekte mit verschiedener Lage (Ausrichten wirkt), Zoom 100 % (Vergrößerungskürzel wirken)
    let ids: Vec<u64> = [&f.ellipse, &f.path2, &f.text].iter().filter_map(|v| v.first().copied()).collect();
    select(&mut st, &ids);
    let _ = st.session.execute("view.setZoom", serde_json::json!({ "zoom": 0.33 }));
    st.session.set_colors(Some(fp_color::Argb(0xff00_ff00)), Some(fp_color::Argb(0xff12_3456)));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
    app.state = st;
    let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(2);
    let before = probe(&h.state().state);
    let mods = egui::Modifiers { alt: sc.alt, ctrl: sc.ctrl, shift: sc.shift, command: sc.ctrl, mac_cmd: false };
    h.key_press_modifiers(mods, key);
    h.run_steps(2);
    let after = probe(&h.state().state);
    before.ui != after.ui || before.doc != after.doc
}

#[test]
fn gesamt_kuerzelsaetze() {
    let mut rows = Vec::new();
    for set in fp_engine::shortcuts::builtin_sets() {
        let (mut st, _ctx, _f) = fixture();
        st.session.prefs.set("CurrentShortcutSet", PrefValue::Str(set.name.clone()));
        let mut total = 0;
        let mut ok = 0;
        let mut problems: Vec<String> = Vec::new();
        let mut shared = 0;
        for (fw_cmd, scs) in &set.menu {
            let Some(cmd) = st.session.registry().by_fw_cmd(fw_cmd).map(|c| c.info().name.clone()) else {
                problems.push(format!("{fw_cmd}: kein Befehl im Register"));
                continue;
            };
            for sc in scs {
                total += 1;
                if egui_key_for_vk(sc.vk).is_none() {
                    problems.push(format!("{} ({cmd}): Taste nicht über egui erreichbar", sc.display_de()));
                    continue;
                }
                let numpad = matches!(sc.vk, 0x60..=0x69 | 0x6B | 0x6D);
                if numpad {
                    // Durchstich über die echte Tastaturverarbeitung (Ziffernblock-Rückfall)
                    if key_has_effect(&set.name, *sc) {
                        ok += 1;
                    } else {
                        problems.push(format!("{} ({cmd}): ohne Wirkung", sc.display_de()));
                    }
                    continue;
                }
                match st.session.command_for_shortcut(sc) {
                    Some(c) if c == cmd => ok += 1,
                    Some(_) => {
                        // mehrfach belegt (das Original löst den ersten Eintrag aus)
                        shared += 1;
                        ok += 1;
                    }
                    None => problems.push(format!("{} ({cmd}): löst nichts aus", sc.display_de())),
                }
            }
        }
        let mut tools_total = 0;
        let mut tools_ok = 0;
        for scs in set.tools.values() {
            for sc in scs {
                tools_total += 1;
                let sc: Shortcut = *sc;
                if st.session.tool_shortcut(&sc).ok().flatten().is_some() || key_has_effect(&set.name, sc) {
                    tools_ok += 1;
                } else {
                    problems.push(format!("Werkzeugkürzel {} ohne Wirkung", sc.display_de()));
                }
            }
        }
        let o = if problems.is_empty() { Outcome::Ui } else { Outcome::Error(format!("{} Fund(e)", problems.len())) };
        rows.push(Row {
            path: format!("Kürzelsatz „{}“", set.name),
            command: format!("{total} Menükürzel, {tools_total} Werkzeugkürzel"),
            outcome: o,
            note: format!("Menü {ok}/{total} (davon {shared} mehrfach belegt), Werkzeuge {tools_ok}/{tools_total}{}", if problems.is_empty() { String::new() } else { format!("; {}", problems.join("; ")) }),
        });
    }
    // Durchstich: echte Tasten im Hauptfenster (Standardsatz)
    type Check = fn(&fp_ui::app_state::AppState) -> bool;
    let cases: Vec<(&str, egui::Modifiers, egui::Key, Check)> = vec![
        ("Strg+N → Neues Dokument", egui::Modifiers::COMMAND, egui::Key::N, |st| st.dialogs.new_document.is_some()),
        ("Strg+A → Alles auswählen", egui::Modifiers::COMMAND, egui::Key::A, |st| st.session.active().is_some_and(|d| d.selection.elements.len() > 3)),
        ("Strg+G → Gruppieren", egui::Modifiers::COMMAND, egui::Key::G, |st| st.session.active().is_some_and(|d| d.history.undo_count() > 0)),
        ("Strg++ → Vergrößern", egui::Modifiers::COMMAND, egui::Key::Plus, |st| st.session.active().is_some_and(|d| d.view.zoom > 0.5)),
        ("U → Rechteck-Werkzeug", egui::Modifiers::NONE, egui::Key::U, |st| st.session.tools.active != fp_engine::ToolId::Pointer),
        ("V → Zeiger", egui::Modifiers::NONE, egui::Key::V, |st| st.session.tools.active == fp_engine::ToolId::Pointer),
        ("Tab → Bedienfelder ausblenden", egui::Modifiers::NONE, egui::Key::Tab, |st| st.settings.workspace.panels_hidden),
        ("D → Standardfarben", egui::Modifiers::NONE, egui::Key::D, |st| st.session.tools.fill_color != fp_color::Argb(0xff12_3456)),
        ("X → Farben tauschen", egui::Modifiers::NONE, egui::Key::X, |st| st.session.tools.fill_color == fp_color::Argb(0xff00_ff00)),
        ("F → Bildschirmmodus", egui::Modifiers::NONE, egui::Key::F, |st| st.session.ui.screen_mode != 0),
        ("Strg+Num 1 → 100 %", egui::Modifiers::COMMAND, egui::Key::Num1, |st| st.session.active().is_some_and(|d| (d.view.zoom - 1.0).abs() < 1e-6)),
    ];
    for (label, mods, key, check) in cases {
        let (mut st, _ctx, _f) = fixture();
        if label.starts_with("Strg+G") {
            let _ = st.session.execute("select.all", serde_json::json!({}));
        }
        if label.starts_with("V ") {
            st.push(fp_ui::app_state::Action::SetTool(fp_engine::ToolId::Rectangle));
        }
        st.session.set_colors(Some(fp_color::Argb(0xff00_ff00)), Some(fp_color::Argb(0xff12_3456)));
        let ctx = egui::Context::default();
        let mut app = FirepixelApp::with_session(&ctx, fp_engine::Session::default(), UiSettings::default(), false);
        app.state = st;
        let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
        h.run_steps(2);
        h.key_press_modifiers(mods, key);
        h.run_steps(2);
        let ok = check(&h.state().state);
        rows.push(Row { path: format!("Tastatur ▸ {label}"), command: format!("{key:?}"), outcome: if ok { Outcome::Ui } else { Outcome::Nothing(String::new()) }, note: if ok { "Wirkung erkannt".into() } else { "keine Wirkung".into() } });
    }
    let (ok, n) = write_report("kuerzel_auto.md", "Tastenkürzel", "gesamt_kuerzel.rs", &rows);
    eprintln!("Kürzel: {ok}/{n} ✅");
}
