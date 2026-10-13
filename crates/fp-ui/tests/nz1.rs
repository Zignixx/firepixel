//! Nachzügler NZ-1 (Befunde von PM-4): Abbrechen in Dialogen mit Vorschau hinterlässt weder einen
//! Rückgängig- noch einen Wiederholen-Schritt (NUTZERBEFUNDE N01, §4b.2), Werkzeugname im
//! Inspektorkopf bricht nur zwischen Wörtern um (PARITAET_A W28), Textur- und Musterliste so breit
//! wie der längste Name (W21), Symbolknöpfe mit Namen im Barrierefreiheits-Baum (für `ui_find`).

use egui::accesskit::{Role, Toggled};
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, NodeT, Queryable};
use egui_kittest::Harness;
use fp_engine::{Session, ToolId};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use fp_ui::inspector::obj_quads::{self, DEFAULT_TEXTURE};
use fp_ui::inspector::popups;
use fp_ui::FirepixelApp;
use serde_json::{json, Value};
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness(setup: impl FnOnce(&mut Session)) -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("Dokument");
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn rect(s: &mut Session, r: [i32; 4]) {
    s.execute("draw.rectangle", json!({"rect": r, "fillColor": "#3366cc", "strokeWidth": 0})).expect("Rechteck");
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
}

/// (Rückgängig-Schritte, Wiederholen-Schritte, nächster Wiederholen-Schritt)
fn history(h: &H) -> (usize, usize, Option<String>) {
    let d = st(h).session.active().expect("Dokument");
    (d.history.undo_count(), d.history.redo_count(), d.history.redo_label().map(str::to_string))
}

fn element_count(h: &H) -> usize {
    let d = st(h).session.active().expect("Dokument");
    fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).len()
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

/// Strg+Y wie egui-winit unter Windows (Drücken und Loslassen getrennt).
fn ctrl_y(h: &mut H) {
    for pressed in [true, false] {
        ev(h, Event::Key { key: Key::Y, physical_key: Some(Key::Y), pressed, repeat: false, modifiers: Modifiers::CTRL });
    }
    h.run_steps(2);
}

fn set_field(h: &mut H, key: &str, v: Value) {
    let f = h.state_mut().state.dialogs.form.as_mut().expect("Dialog offen");
    f.field_mut(key).unwrap_or_else(|| panic!("Feld {key}")).value = v;
    h.run_steps(1);
}

fn press(h: &mut H, label: &str) {
    h.run_steps(1);
    let r = h.get_all(By::new().label(label)).last().unwrap_or_else(|| panic!("Knopf {label}")).rect();
    click(h, r.center());
    h.run_steps(2);
}

fn fx_index(label: &str) -> usize {
    effects::FX.iter().position(|f| f.menu_text() == label).unwrap_or_else(|| panic!("{label}"))
}

/// Ein Rechteck gewählt und ein wiederholbarer Schritt (zweites Rechteck, zurückgenommen).
fn rect_with_redo() -> H {
    let mut h = harness(|s| {
        rect(s, [20, 20, 140, 100]);
        rect(s, [200, 40, 300, 120]);
        s.undo().expect("undo");
    });
    h.run_steps(2);
    h
}

// ------------------------------------------------------------------ N01: Abbrechen mit Vorschau

const LIVE: &[(&str, &str)] = &[("Glühen", "width"), ("Schlagschatten", "dist"), ("Innerer Schatten", "dist"), ("Abschrägung innen", "width"), ("Farbfüllung", "opacity"), ("Gaußscher Weichz. ...", "radius"), ("Farbton/Sättigung...", "hue")];

/// N01: Live-Effekt bzw. Live-Filter neu, Wert vielfach geändert (Vorschau), Abbrechen → Dokument
/// und Verlauf exakt wie vorher: kein Rückgängig-, kein Wiederholen-Schritt, der vorher vorhandene
/// Wiederholen-Schritt bleibt; Strg+Y bringt ihn (nicht den Effekt) zurück.
#[test]
fn n01_cancel_new_live_effect_leaves_no_history() {
    for (label, field) in LIVE {
        let mut h = rect_with_redo();
        let (d0, h0) = (doc(&h), history(&h));
        assert_eq!(h0.1, 1, "Ausgang: ein Wiederholen-Schritt");
        effects::begin_add(&mut h.state_mut().state, fx_index(label));
        h.run_steps(2);
        for k in 1..=12 {
            set_field(&mut h, field, json!(k + 2));
        }
        press(&mut h, "Abbrechen");
        assert!(st(&h).dialogs.form.is_none(), "{label}: Dialog zu");
        assert!(*doc(&h) == *d0, "{label}: Dokument wie vorher");
        assert_eq!(history(&h), h0, "{label}: Verlauf (Rückgängig, Wiederholen) wie vorher");
        ctrl_y(&mut h);
        assert_eq!(element_count(&h), 2, "{label}: Strg+Y bringt das zweite Rechteck zurück");
        let d = st(&h).session.active().expect("Dokument");
        let fx: usize = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, id)).map(|e| e.common.effects.as_ref().map(|l| l.effects.len()).unwrap_or(0)).sum();
        assert_eq!(fx, 0, "{label}: Strg+Y bringt keinen Effekt");
    }
}

/// Ohne vorherigen Wiederholen-Stapel: nach Abbrechen ist kein Schritt wiederholbar, und der
/// „letzte Befehl“ (Strg+Y ohne Wiederholen-Stapel wiederholt ihn wie im Original) ist wieder der
/// vor dem Dialog, nicht die Vorschau – vorher fügte Strg+Y das Glühen wieder ein.
#[test]
fn n01_cancel_glow_ctrl_y_does_not_bring_glow_back() {
    let mut h = harness(|s| rect(s, [20, 20, 140, 100]));
    let h0 = history(&h);
    let last0 = st(&h).session.last_command.clone();
    assert_eq!(h0.1, 0);
    effects::begin_add(&mut h.state_mut().state, fx_index("Glühen"));
    h.run_steps(2);
    for k in 1..=20 {
        set_field(&mut h, "width", json!(k));
    }
    press(&mut h, "Abbrechen");
    assert_eq!(history(&h), h0);
    assert_eq!(st(&h).session.last_command, last0, "letzter Befehl wie vor dem Dialog");
    ctrl_y(&mut h);
    let d = st(&h).session.active().expect("Dokument");
    let fx: usize = fp_engine::docops::selectable_ids(&d.doc, d.page, d.frame).into_iter().filter_map(|id| fp_engine::docops::element_on_page(&d.doc, d.page, id)).map(|e| e.common.effects.as_ref().map(|l| l.effects.len()).unwrap_or(0)).sum();
    assert_eq!(fx, 0, "Strg+Y bringt das Glühen nicht zurück");
}

/// Bearbeiten eines vorhandenen Effekts, Vorschau aus- und wieder eingeschaltet, Abbrechen; und
/// OK bleibt genau ein Schritt (der Wiederholen-Stapel wird dann wie bei jedem Befehl geleert).
#[test]
fn n01_edit_effect_cancel_and_ok() {
    let mut h = harness(|s| {
        rect(s, [20, 20, 140, 100]);
        s.execute("element.addEffect", json!({"effect": "glow", "params": {"BevelType": "4", "GlowWidth": "4"}})).expect("Glühen");
        rect(s, [200, 40, 300, 120]);
        s.undo().expect("undo");
    });
    let (d0, h0) = (doc(&h), history(&h));
    let e = {
        let d = st(&h).session.active().expect("Dokument");
        let id = *d.selection.elements.first().expect("Auswahl");
        fp_engine::docops::element_on_page(&d.doc, d.page, id).and_then(|e| e.common.effects.clone()).expect("Effekte").effects[0].clone()
    };
    effects::begin_edit(&mut h.state_mut().state, 0, &e);
    h.run_steps(2);
    for k in 1..=8 {
        set_field(&mut h, "width", json!(k + 5));
    }
    h.state_mut().state.dialogs.form.as_mut().expect("Dialog").preview_on = false;
    h.run_steps(2);
    h.state_mut().state.dialogs.form.as_mut().expect("Dialog").preview_on = true;
    set_field(&mut h, "width", json!(30));
    press(&mut h, "Abbrechen");
    assert!(*doc(&h) == *d0, "Bearbeiten abbrechen: Dokument");
    assert_eq!(history(&h), h0, "Bearbeiten abbrechen: Verlauf");
    // OK: genau ein Schritt
    effects::begin_edit(&mut h.state_mut().state, 0, &e);
    h.run_steps(2);
    for k in 1..=8 {
        set_field(&mut h, "width", json!(k + 5));
    }
    press(&mut h, "OK");
    assert_eq!(history(&h).0, h0.0 + 1, "OK = ein Schritt");
    assert_eq!(history(&h).1, 0, "nach OK nichts zu wiederholen");
}

/// Filter auf eine Bitmap (Gaußscher Weichzeichner, Unscharf maskieren über das Menü) und die
/// Tonwertkorrektur: Abbrechen nach mehreren Vorschauen stellt Dokument und Verlauf her.
#[test]
fn n01_cancel_destructive_filters_and_levels() {
    for (cmd, field) in [("filter.blur.gaussian", Some("radius")), ("filter.sharpen.unsharpMask", Some("amount")), ("filter.adjustColor.levels", None)] {
        let mut h = harness(|s| {
            rect(s, [20, 20, 140, 100]);
            s.execute("modify.flattenSelection", json!({})).expect("Bitmap");
            rect(s, [200, 40, 300, 120]);
            s.undo().expect("undo");
        });
        let (d0, h0) = (doc(&h), history(&h));
        assert!(fp_ui::dialogs::open_for_command(&mut h.state_mut().state, cmd), "{cmd}: Dialog");
        h.run_steps(3);
        for k in 1..=6 {
            match field {
                Some(f) => set_field(&mut h, f, json!(k as f64 * 2.0)),
                None => {
                    h.state_mut().state.dialogs.levels.as_mut().expect("Tonwert").params.rgb.gamma = 1.0 + k as f64 * 0.2;
                    h.run_steps(2);
                }
            }
        }
        assert!(history(&h).0 >= h0.0, "{cmd}: Vorschau angewandt");
        press(&mut h, "Abbrechen");
        assert!(*doc(&h) == *d0, "{cmd}: Dokument wie vorher");
        assert_eq!(history(&h), h0, "{cmd}: Verlauf wie vorher");
    }
}

/// Unverändertes Dokument bleibt nach Vorschau und Abbrechen unverändert (kein Stern im Titel).
#[test]
fn n01_cancel_keeps_document_clean() {
    let mut h = harness(|s| rect(s, [20, 20, 140, 100]));
    {
        let d = h.state_mut().state.session.active_mut().expect("Dokument");
        d.saved_generation = d.generation;
    }
    effects::begin_add(&mut h.state_mut().state, fx_index("Glühen"));
    h.run_steps(2);
    for k in 1..=5 {
        set_field(&mut h, "width", json!(k + 3));
    }
    press(&mut h, "Abbrechen");
    assert!(!st(&h).session.active().expect("Dokument").is_dirty(), "nach Abbrechen ungeändert");
}

// ------------------------------------------------------------------ W28: Umbruch nur zwischen Wörtern

/// Zeilen ergeben den Namen; jeder Umbruch liegt an einem Leerzeichen oder nach einem Bindestrich.
fn wraps_between_words(label: &str, lines: &[String]) -> bool {
    if lines.last().is_some_and(|l| l.ends_with('…')) {
        return true;
    }
    let mut rest = label;
    for (k, l) in lines.iter().enumerate() {
        let Some(r) = rest.strip_prefix(l.as_str()) else { return false };
        rest = r;
        if k + 1 < lines.len() {
            if let Some(r) = rest.strip_prefix(' ') {
                rest = r.trim_start();
            } else if !l.ends_with('-') {
                return false;
            }
        }
    }
    rest.trim().is_empty()
}

fn header_lines(h: &mut H, tool: ToolId) -> Vec<String> {
    // Zeilen des vorigen Werkzeugs verwerfen (Werkzeuge ohne Werkzeugkopf zeichnen keine)
    h.ctx.data_mut(|d| d.remove_temp::<Vec<String>>(egui::Id::new("pi-tool-q1-lines")));
    let _ = h.state_mut().state.session.set_tool(tool);
    h.run_steps(3);
    fp_ui::inspector::tool_header_lines(&h.ctx)
}

/// Alle Werkzeugnamen in Deutsch und Englisch: höchstens zwei Zeilen, nie mitten im Wort
/// umbrochen; „Pfadausbesserung / - additiv“ wie im Original.
#[test]
fn w28_tool_names_wrap_between_words_de_en() {
    let mut h = harness(|_| {});
    let mut bad = Vec::new();
    let mut checked = 0;
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            for info in fp_engine::tools::TOOLS {
                if info.id == ToolId::AutoShape {
                    continue;
                }
                let want = fp_ui::inspector::texts::tool_label(info);
                let lines = header_lines(&mut h, info.id);
                // nur Werkzeuge, die sich ohne weiteren Zustand wählen lassen (Kopf zeigt sie)
                if lines.is_empty() || h.state().state.session.tools.active != info.id {
                    continue;
                }
                checked += 1;
                if lines.len() > 2 || !wraps_between_words(&want, &lines) {
                    bad.push(format!("{lang}: {want:?} → {lines:?}"));
                }
            }
        });
    }
    assert!(bad.is_empty(), "Umbruch mitten im Wort:\n{}", bad.join("\n"));
    assert!(checked >= 60, "Werkzeuge geprüft: {checked}");
    let lines = header_lines(&mut h, ToolId::PathScrubAdd);
    assert_eq!(lines, vec!["Pfadausbesserung".to_string(), "- additiv".to_string()]);
    let lines = header_lines(&mut h, ToolId::RedEyeRemoval);
    assert_eq!(lines, vec!["Rote Augen".to_string(), "entfernen".to_string()]);
    assert_eq!(header_lines(&mut h, ToolId::Text), vec!["Text".to_string()]);
}

// ------------------------------------------------------------------ W21: Breite der Listen

fn text_w(h: &H, s: &str) -> f32 {
    let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
    h.ctx.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font, egui::Color32::WHITE).size().x)
}

fn texture_combos(h: &H) -> Vec<egui::Rect> {
    let vein = obj_quads::texture_label(DEFAULT_TEXTURE);
    let mut v: Vec<egui::Rect> = h.get_all(By::new().role(Role::ComboBox)).filter(|n| n.value().unwrap_or_default() == vein).map(|n| n.rect()).filter(|r| r.top() > HH - 160.0).collect();
    v.sort_by(|a, b| a.left().total_cmp(&b.left()));
    v
}

/// Aufgeklappte Texturliste (Rechteck-Werkzeug) so breit wie der längste Name, nicht nur wie das
/// Feld (Windows-Kombifeld im Original); jeder Name passt ungekürzt hinein.
#[test]
fn w21_texture_list_as_wide_as_longest_name() {
    let mut h = harness(|_| {});
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.run_steps(3);
    let c = texture_combos(&h);
    assert!(!c.is_empty(), "Texturfeld");
    click(&mut h, c[0].center());
    let other = h.get_by_label("Andere...").rect();
    let (_, labels, _) = obj_quads::texture_list_labels("");
    let longest = labels.iter().map(|l| text_w(&h, l)).fold(0.0_f32, f32::max);
    assert!(other.width() >= c[0].width() - 1.0, "mindestens so breit wie das Feld: {other:?} / {:?}", c[0]);
    assert!(other.width() >= longest + 10.0 - 0.5, "längster Name {longest} passt in die Liste {other:?}");
}

/// Musterliste: ebenso so breit wie der längste Mustername.
#[test]
fn w21_pattern_list_as_wide_as_longest_name() {
    if popups::pattern_files().is_empty() {
        return;
    }
    let mut h = harness(|s| rect(s, [50, 50, 150, 120]));
    let tex = texture_combos(&h)[0];
    click(&mut h, egui::pos2(tex.left() + 119.0, tex.top() - 38.0));
    let other = h.get_by_label("Andere...").rect();
    let longest = popups::pattern_files().iter().map(|(n, _)| text_w(&h, n)).fold(0.0_f32, f32::max);
    assert!(other.width() >= longest + 10.0 - 0.5, "längster Mustername {longest} passt: {other:?}");
}

// ------------------------------------------------------------------ Barrierefreiheit der Symbolknöpfe

fn button(h: &H, name: &str) -> Option<(egui::Rect, Option<Toggled>)> {
    h.query_all(By::new().role(Role::Button).label(name)).next().map(|n| (n.rect(), n.accesskit_node().toggled()))
}

/// „+“/„−“ im Filter-Kopf, F/K/U, „ab/cd“, Ausrichtungen und Werkzeugknöpfe sind Knöpfe mit
/// übersetztem Namen; Umschalter melden ihren Zustand. In Deutsch und Englisch.
#[test]
fn a11y_icon_buttons_have_translated_names() {
    let mut h = harness(|s| {
        s.execute("draw.text", json!({"text": "Hallo", "anchor": [30, 60], "size": 24})).expect("Text");
    });
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            h.run_steps(3);
            for key in ["pi-fx-add", "pi-fx-delete", "pi-text-italic", "pi-text-underline", "pi-text-orientation", "pi-text-align-left", "pi-style-new"] {
                let name = fp_i18n::t(key);
                assert!(button(&h, &name).is_some(), "{lang}: Knopf {key} „{name}“ fehlt im Barrierefreiheits-Baum");
            }
            let left = button(&h, &fp_i18n::t("pi-text-align-left")).expect("Linksbündig");
            assert_eq!(left.1, Some(Toggled::True), "{lang}: Linksbündig gedrückt");
            let bold = button(&h, &fp_i18n::t("pi-text-bold")).expect("Fett");
            assert_eq!(bold.1, Some(Toggled::False), "{lang}: F/B nicht gedrückt");
            let italic = button(&h, &fp_i18n::t("pi-text-italic")).expect("Kursiv");
            assert_eq!(italic.1, Some(Toggled::False), "{lang}: K/I nicht gedrückt");
            let n = h.get_all(By::new().role(Role::Button)).filter(|n| n.accesskit_node().label().is_some_and(|l| !l.is_empty())).count();
            assert!(n > 40, "{lang}: viele benannte Knöpfe (Werkzeuge, Fenster, Bedienfelder): {n}");
        });
    }
    // „+“ über den Namen finden und anklicken öffnet das Live-Filter-Menü
    h.run_steps(2);
    let (plus, _) = button(&h, &fp_i18n::t("pi-fx-add")).expect("+");
    click(&mut h, plus.center());
    assert!(h.query_all(By::new().label_contains("Glühen")).next().is_some(), "Menü der Live-Filter offen");
}
