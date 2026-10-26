//! Jeder Dialog zu einem Menü-/Bedienfeldbefehl öffnet ohne Panik; Abbrechen (Esc) lässt das
//! Dokument exakt unverändert (auch nach Live-Vorschau), OK (Eingabe) schließt ohne Panik.
//! Dateidialoge werden über `FilePickOverride` ersetzt (keine Fenster, AGENTEN_REGELN §4a).

use egui_kittest::Harness;
use fp_engine::Session;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

fn session() -> Session {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 300, "height": 200})).expect("doc");
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 80]})).expect("rect");
    s.execute("draw.text", json!({"text": "Hallo Wlet", "anchor": [30, 150], "size": 20})).expect("text");
    s.execute("draw.ellipse", json!({"rect": [150, 20, 250, 90]})).expect("ellipse");
    s.execute("modify.symbol.convertToSymbol", json!({"name": "Sym"})).expect("symbol");
    s.execute("draw.rectangle", json!({"rect": [160, 120, 260, 180]})).expect("rect2");
    s.execute("pi.style.new", json!({"name": "Stil 1"})).expect("style");
    s
}

fn harness() -> Harness<'static, FirepixelApp> {
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, session(), UiSettings::default(), false);
    a.state.sync_render = true;
    let tmp = fp_i18n::profile::unique_temp_dir("dialogs-open");
    let _ = std::fs::create_dir_all(&tmp);
    a.state.file_override = Some(fp_ui::files::FilePickOverride { open: vec![tmp.join("nicht-vorhanden.png")], save: Some(tmp.join("ausgabe.png")) });
    a.state.dialogs.panel_sel.styles = vec!["Stil 1".into()];
    a.state.dialogs.panel_sel.symbol = Some(0);
    Harness::builder().with_size(egui::vec2(1500.0, 950.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

/// Befehle, deren Menüpunkt im Original einen Dialog öffnet.
const DIALOG_COMMANDS: &[&str] = &[
    "modify.canvas.canvasSize",
    "modify.canvas.imageSize",
    "modify.transform.numericTransform",
    "filter.adjustColor.levels",
    "filter.adjustColor.curves",
    "file.export",
    "file.imagePreview",
    "edit.keyboardShortcuts",
    "text.spellingSetup",
    "filter.blur.gaussian",
    "filter.blur.motion",
    "filter.blur.radial",
    "filter.blur.zoom",
    "filter.sharpen.unsharpMask",
    "filter.noise.addNoise",
    "filter.adjustColor.brightnessContrast",
    "filter.adjustColor.hueSaturation",
    "select.feather",
    "select.expandMarquee",
    "select.contractMarquee",
    "select.borderMarquee",
    "select.smoothMarquee",
    "select.saveBitmapSelection",
    "select.restoreBitmapSelection",
    "select.deleteBitmapSelection",
    "modify.alterPath.simplify",
    "modify.alterPath.insetPath",
    "modify.alterPath.expandStroke",
    "text.size.other",
    "modify.symbol.tweenInstances",
    "modify.symbol.convertToSymbol",
    "modify.animation.animateSelection",
    "modify.animation.settings",
    "modify.canvas.canvasColor",
    "view.grid.editGrid",
    "view.guides.editGuides",
    "edit.insert.layer",
    "edit.insert.state",
    "file.exportWizard",
    "file.htmlSetup",
    "modify.selectiveJpeg.settings",
    "file.pageSetup",
    "file.print",
    "modify.symbol.swapSymbol",
    "edit.insert.newSymbol",
    "edit.insert.newButton",
    "commands.manageSavedCommands",
    "panel.layers.newLayer",
    "panel.layers.newSubLayer",
    "panel.layers.duplicate",
    "panel.layers.shareToPages",
    "panel.layers.thumbnailOptions",
    "panel.pages.thumbnailOptions",
    "panel.states.add",
    "panel.states.duplicate",
    "panel.states.copyTo",
    "panel.states.properties",
    "panel.states.onionCustom",
    "panel.pages.rename",
    "panel.styles.rename",
    "panel.styles.options",
    "panel.library.newSymbol",
    "panel.library.properties",
    "panel.library.saveToCommon",
    "panel.optimize.saveSettings",
    "panel.optimize.optimizeToSize",
    "panel.optimize.exportWizard",
    "panel.optimize.addColorMenu",
    "panel.effects.saveAsStyle",
];

fn doc(h: &Harness<'_, FirepixelApp>) -> Arc<fp_model::Document> {
    Arc::clone(&h.state().state.session.active().expect("doc").doc)
}

#[test]
fn every_dialog_opens_and_cancel_changes_nothing() {
    let mut not_opened = Vec::new();
    for name in DIALOG_COMMANDS {
        let mut h = harness();
        h.run_steps(2);
        let d0 = doc(&h);
        let n0 = h.state().state.session.active().map(|d| d.history.undo_count()).unwrap_or(0);
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        if !h.state().state.dialogs.any_open() {
            not_opened.push(*name);
            continue;
        }
        h.key_press(egui::Key::Escape);
        h.run_steps(3);
        // manche Dialoge schließen erst mit „Abbrechen“; Esc genügt bei allen Modalen
        assert!(!h.state().state.dialogs.any_open(), "{name}: Dialog nach Esc noch offen");
        assert!(*doc(&h) == *d0, "{name}: Abbrechen hat das Dokument geändert");
        assert_eq!(h.state().state.session.active().map(|d| d.history.undo_count()), Some(n0), "{name}: Verlauf nach Abbrechen");
    }
    assert!(not_opened.is_empty(), "ohne Dialog: {not_opened:?}");
}

#[test]
fn every_dialog_ok_closes_without_panic() {
    for name in DIALOG_COMMANDS {
        let mut h = harness();
        h.run_steps(2);
        h.state_mut().state.run(name, json!({}));
        h.run_steps(3);
        h.key_press(egui::Key::Enter);
        h.run_steps(4);
        // Dialoge mit eigener Liste (Tastaturkurzbefehle, gespeicherte Befehle) haben kein Eingabe-OK
        if h.state().state.dialogs.any_open() {
            h.key_press(egui::Key::Escape);
            h.run_steps(2);
        }
        assert!(!h.state().state.dialogs.any_open(), "{name}: bleibt offen");
    }
}

#[test]
fn panel_menu_dialogs_ok_apply() {
    // Neue Ebene: Name + statusübergreifend
    let mut h = harness();
    h.run_steps(2);
    let layers = |h: &Harness<'_, FirepixelApp>| {
        let d = h.state().state.session.active().expect("doc");
        fp_engine::docops::own_layers(&d.doc, d.page).len()
    };
    let n = layers(&h);
    h.state_mut().state.run("panel.layers.newLayer", json!({}));
    h.run_steps(3);
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(layers(&h), n + 1, "Neue Ebene über das Bedienfeldmenü");
    // Ebene duplizieren 3×
    let f = |h: &mut Harness<'_, FirepixelApp>, key: &str, v: serde_json::Value| {
        // Wert direkt setzen: vorher den Fokus des (beim Öffnen fokussierten) Felds abgeben, sonst
        // übernimmt Eingabe dessen Puffer
        h.ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        h.run_steps(1);
        h.state_mut().state.dialogs.form.as_mut().expect("Dialog").field_mut(key).expect("Feld").value = v;
        h.run_steps(1);
    };
    h.state_mut().state.run("panel.layers.duplicate", json!({}));
    h.run_steps(3);
    f(&mut h, "count", json!(3));
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    assert_eq!(layers(&h), n + 4, "Ebene duplizieren mit Anzahl 3");
    // Hilfslinien-Dialog: alle löschen + sperren
    h.state_mut().state.session.execute("guide.add", json!({"orientation": "horizontal", "position": 40})).expect("guide");
    h.state_mut().state.run("view.guides.editGuides", json!({}));
    h.run_steps(3);
    f(&mut h, "clear", json!(true));
    f(&mut h, "locked", json!(true));
    h.key_press(egui::Key::Enter);
    h.run_steps(4);
    let d = h.state().state.session.active().expect("doc");
    assert!(d.view.lock_guides);
    assert!(d.doc.pages[0].root.guides.as_ref().is_none_or(|g| g.horizontal.is_empty()));
}
