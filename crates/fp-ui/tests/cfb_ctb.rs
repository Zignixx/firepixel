//! CF-B: fixes for the click test CT-B (`docs\funktionstest\ct_b_2026-10-15.md`) in the whole window
//! (egui_kittest): Ctrl+Shift+Tab (CT-B-03), double-click on a guide (CT-B-04), texture of the
//! rectangle tool preset (CT-B-06), tool shortcuts once (CT-B-08), "Edit..." buttons readable
//! (CT-B-10).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use fp_engine::ToolId;
use fp_ui::app_state::UiSettings;
use fp_ui::FirepixelApp;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness_with(s: fp_engine::Session, w: f32, hh: f32) -> H {
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(w, hh)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn new_session(n: usize) -> fp_engine::Session {
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    for _ in 0..n {
        s.execute("file.new", json!({"width": 400, "height": 300, "background": "#ffffff"})).expect("doc");
    }
    s
}

fn screen_of(h: &H, x: f64, y: f64) -> Pos2 {
    let st = &h.state().state;
    let d = st.session.active().expect("doc");
    let area = st.canvases.get(&d.id).and_then(|c| c.area).expect("canvas");
    let (sx, sy) = d.view.doc_to_screen(fp_geom::Point::new(x as f32, y as f32));
    Pos2::new(area.min.x + sx as f32, area.min.y + sy as f32)
}

fn undo_count(h: &H) -> usize {
    h.state().state.session.active().expect("doc").history.undo_count()
}

fn guide_x(h: &H) -> f32 {
    let d = h.state().state.session.active().expect("doc");
    d.doc.pages[d.page].root.guides.as_ref().expect("guides").vertical[0]
}

fn press(h: &mut H, p: Pos2, down: bool) {
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
    h.run_steps(1);
}

/// CT-B-03: Ctrl+Shift+Tab goes back through the documents (A → C → B), Ctrl+Tab forward.
#[test]
fn ctrl_shift_tab_cycles_documents_backwards() {
    let mut h = harness_with(new_session(3), 1400.0, 900.0);
    let ids: Vec<_> = h.state().state.session.documents().iter().map(|d| d.id).collect();
    h.state_mut().state.push(fp_ui::app_state::Action::Activate(ids[0]));
    h.run_steps(3);
    assert_eq!(h.state().state.session.active_id(), Some(ids[0]));
    let back = Modifiers { ctrl: true, command: true, shift: true, ..Default::default() };
    let mut seen = Vec::new();
    for _ in 0..3 {
        h.key_press_modifiers(back, Key::Tab);
        h.run_steps(3);
        seen.push(h.state().state.session.active_id().expect("active"));
    }
    assert_eq!(seen, vec![ids[2], ids[1], ids[0]], "Ctrl+Shift+Tab backwards");
    h.key_press_modifiers(Modifiers { ctrl: true, command: true, ..Default::default() }, Key::Tab);
    h.run_steps(3);
    assert_eq!(h.state().state.session.active_id(), Some(ids[1]), "Ctrl+Tab forwards");
}

/// CT-B-04: a click on a guide leaves no empty step; a double-click opens "Move Guide" (with and,
/// as measured in re 33 §8 b, also without rulers),
/// OK without a change adds no step, a new position moves the guide in one step.
#[test]
fn guide_double_click_opens_move_dialog_without_empty_steps() {
    let mut s = new_session(1);
    s.execute("guide.add", json!({"orientation": "vertical", "position": 200})).expect("guide");
    if let Some(d) = s.active_mut() {
        d.view.show_rulers = true;
        d.view.show_guides = true;
    }
    s.execute("tool.select", json!({"tool": "tool.pointer"})).expect("pointer");
    let mut h = harness_with(s, 1400.0, 900.0);
    let n0 = undo_count(&h);
    let p = screen_of(&h, 200.0, 150.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(2);
    // single click: nothing
    press(&mut h, p, true);
    press(&mut h, p, false);
    h.run_steps(40); // longer than the double-click time
    assert_eq!(undo_count(&h), n0, "click on a guide: no step");
    assert!(h.state().state.dialogs.form.is_none(), "single click: no dialog");
    // double-click
    press(&mut h, p, true);
    press(&mut h, p, false);
    press(&mut h, p, true);
    press(&mut h, p, false);
    h.run_steps(2);
    assert_eq!(undo_count(&h), n0, "double-click: no empty \"Move Guide\" steps");
    let f = h.state().state.dialogs.form.clone().expect("dialog \"Move Guide\"");
    assert_eq!(f.command, "guide.move");
    assert_eq!(f.title, fp_i18n::t("cmd-guide-move"));
    assert_eq!(f.fields[0].value.as_f64(), Some(200.0), "field shows the position");
    // OK without a change is skipped (params equal the opening values)
    assert_eq!(f.initial.as_ref(), Some(&f.params()), "unchanged OK adds no step");
    // new position
    let mut f = f;
    f.fields[0].value = json!(240.0);
    let p2 = f.params();
    h.state_mut().state.dialogs.form = None;
    h.state_mut().state.run("guide.move", p2);
    h.run_steps(2);
    assert_eq!(guide_x(&h), 240.0);
    assert_eq!(undo_count(&h), n0 + 1);
    // without rulers the dialog opens too (re 33 §8 b); cancelling leaves the guide and adds no step
    h.state_mut().state.session.active_mut().expect("doc").view.show_rulers = false;
    h.run_steps(2);
    let p = screen_of(&h, 240.0, 150.0);
    h.event(Event::PointerMoved(p));
    h.run_steps(40);
    for down in [true, false, true, false] {
        press(&mut h, p, down);
    }
    h.run_steps(2);
    let f = h.state().state.dialogs.form.clone().expect("no rulers: dialog \"Move Guide\" as well");
    assert_eq!(f.fields[0].value.as_f64(), Some(240.0), "field shows the position");
    h.state_mut().state.dialogs.form = None; // Cancel
    h.run_steps(2);
    assert_eq!(guide_x(&h), 240.0);
    assert_eq!(undo_count(&h), n0 + 1, "still no empty step");
    // dragging still moves it (one step)
    h.run_steps(40);
    press(&mut h, p, true);
    let p3 = screen_of(&h, 260.0, 150.0);
    for i in 1..=5 {
        h.event(Event::PointerMoved(Pos2::new(p.x + (p3.x - p.x) * i as f32 / 5.0, p.y)));
        h.run_steps(1);
    }
    press(&mut h, p3, false);
    h.run_steps(2);
    assert_eq!(guide_x(&h), 260.0);
    assert_eq!(undo_count(&h), n0 + 2);
}

/// CT-B-06: the texture list shows the default texture ("Ader") without a choice; an amount > 0
/// then applies it to new shapes.
#[test]
fn tool_texture_amount_uses_default_texture() {
    let ta = fp_ui::inspector::obj_quads::ToolAttrs { fill_texture_amount: 50, ..Default::default() };
    let (fill, _) = fp_ui::canvas::overlay::attrs_params(&ta);
    let fill = fill.expect("fill with texture");
    assert_eq!(fill["texture"], json!(fp_ui::inspector::obj_quads::DEFAULT_TEXTURE));
    assert_eq!(fill["textureAmount"], json!(50));
    // amount 0: plain default fill as before
    let (fill, _) = fp_ui::canvas::overlay::attrs_params(&fp_ui::inspector::obj_quads::ToolAttrs::default());
    assert!(fill.is_none());
}

/// CT-B-08: each shortcut once in the tooltips ("Rechteck (U)", "Teilauswahl (A, 1)", "Segment (K)").
#[test]
fn tool_tooltips_list_each_shortcut_once() {
    let h = harness_with(new_session(1), 1400.0, 900.0);
    let st = &h.state().state;
    fp_i18n::with_language("de", || {
        for id in [ToolId::Rectangle, ToolId::Subselection, ToolId::Slice] {
            let tip = fp_ui::tools_panel::tool_tooltip(st, id);
            let inner = tip.rsplit_once('(').map(|(_, r)| r.trim_end_matches(')')).unwrap_or("");
            let keys: Vec<&str> = inner.split(", ").collect();
            let mut uniq = keys.clone();
            uniq.sort();
            uniq.dedup();
            assert_eq!(keys.len(), uniq.len(), "{tip}");
        }
        assert!(fp_ui::tools_panel::tool_tooltip(st, ToolId::Rectangle).ends_with("(U)"));
    });
}

/// Audit entries of the "Edit..." buttons and the width of their text at 85 % of the UI font (the
/// smallest size `button_fit` uses).
fn edit_buttons(lang: &str, setup: fn(&mut fp_engine::Session)) -> (Vec<fp_ui::inspector::fields::TextAudit>, f32) {
    fp_i18n::with_language(lang, || {
        let mut s = new_session(1);
        setup(&mut s);
        let mut h = harness_with(s, 1600.0, 1000.0);
        h.run_steps(2);
        fp_ui::inspector::fields::set_audit(&h.ctx, true);
        h.run_steps(1);
        let log = fp_ui::inspector::fields::take_audit(&h.ctx);
        fp_ui::inspector::fields::set_audit(&h.ctx, false);
        let edit = fp_i18n::t("pi-edit");
        let font = fp_ui::theme::Theme::current(&h.ctx).ui_font();
        let small = egui::FontId::new((font.size * 0.85).max(8.0), font.family);
        let w = h.ctx.fonts_mut(|f| f.layout_no_wrap(edit.clone(), small, egui::Color32::WHITE).size().x);
        (log.into_iter().filter(|e| e.inspector && e.kind == "button" && e.text == edit).collect(), w)
    })
}

/// CT-B-10: "Bearbeiten..." / "Edit..." next to the export preset of a button is shown in full (at
/// most slightly smaller), German and English, window 1600 × 1000. The slice shows the edit icon
/// instead of this button since F17 TW-44 (like the original), so it is no longer checked here.
#[test]
fn edit_buttons_not_cut_off() {
    fn button(s: &mut fp_engine::Session) {
        s.execute("draw.rectangle", json!({"rect": [10, 10, 100, 100], "fillColor": "#ff0000"})).expect("rect");
        s.execute("edit.insert.newButton", json!({})).expect("button");
    }
    for lang in ["de", "en"] {
        for (what, setup) in [("button", button as fn(&mut fp_engine::Session))] {
            let (b, min_w) = edit_buttons(lang, setup);
            assert!(!b.is_empty(), "{lang} {what}: button found");
            for e in b {
                assert!(min_w <= e.avail.width() + 0.5, "{lang} {what}: button {:?} too narrow for the text ({min_w} px)", e.avail);
                assert!(e.fit < 2, "{lang} {what}: \"{}\" cut off ({:?} in {:?})", e.text, e.drawn, e.avail);
                assert!(e.drawn.width() <= e.avail.width() + 0.5, "{lang} {what}: text {:?} wider than {:?}", e.drawn, e.avail);
            }
        }
    }
}

/// CT-B-05: red-eye removal through the whole window with the defaults of the inspector (tolerance
/// 50, strength 50): a rectangle over the eyes darkens the pupil in one step.
#[test]
fn red_eye_through_window() {
    let src = std::path::PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\work\p57_t02.fw.png");
    if !src.is_file() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("fp-cfb-ui-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let p = dir.join("t02.fw.png");
    std::fs::copy(&src, &p).expect("copy");
    fp_engine::shell::disable();
    let mut s = fp_engine::Session::default();
    s.execute("file.open", json!({"path": p.to_string_lossy()})).expect("open");
    let mut h = harness_with(s, 1600.0, 1000.0);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    h.state_mut().state.push(fp_ui::app_state::Action::SetTool(ToolId::RedEyeRemoval));
    h.run_steps(4);
    let before = h.state().state.session.active().unwrap().doc.clone();
    let a = screen_of(&h, 100.0, 250.0);
    let b = screen_of(&h, 225.0, 287.0);
    h.event(Event::PointerMoved(a));
    h.run_steps(1);
    press(&mut h, a, true);
    for i in 1..=10 {
        h.event(Event::PointerMoved(a + (b - a) * (i as f32 / 10.0)));
        h.run_steps(1);
    }
    press(&mut h, b, false);
    h.run_steps(3);
    let st = &h.state().state;
    assert_eq!(st.session.prefs.get_int("RedEyeRemovalToolStrength", -1), 50);
    assert_eq!(st.session.active().expect("doc").history.undo_label().unwrap_or_default(), "Rote Augen entfernen");
    let d = st.session.active().expect("doc");
    let img = fp_engine::render::default_renderer().expect("renderer").render_page(&d.doc, d.page, d.frame).expect("image");
    let c = img.get(120, 268).expect("eye");
    assert!(c[0] < 100 && c[1] < 40, "pupil dark: {c:?}");
    let _ = before;
}

/// CT-B-12 (b): Alt + mouse wheel and releasing Alt does not switch the menu bar into keyboard mode
/// (Alt alone still does).
#[test]
fn alt_wheel_does_not_activate_menu_keyboard_mode() {
    let mut h = harness_with(new_session(1), 1400.0, 900.0);
    let p = screen_of(&h, 200.0, 150.0);
    let ev = |h: &mut H, e: Event| {
        h.event(e);
        h.run_steps(1);
    };
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::ModifiersChanged(Modifiers::ALT));
    ev(&mut h, Event::MouseWheel { unit: egui::MouseWheelUnit::Line, delta: egui::vec2(0.0, -1.0), phase: egui::TouchPhase::Move, modifiers: Modifiers::ALT });
    ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert!(!fp_ui::menu_kbd::state(&h.ctx).active, "Alt+wheel: no menu keyboard mode");
    // Alt alone: menu keyboard mode as before
    ev(&mut h, Event::ModifiersChanged(Modifiers::ALT));
    ev(&mut h, Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    assert!(fp_ui::menu_kbd::state(&h.ctx).active, "Alt alone: menu keyboard mode");
}

fn key_ev(h: &mut H, k: Key, m: Modifiers) {
    for pressed in [true, false] {
        h.event(Event::Key { key: k, physical_key: Some(k), pressed, repeat: false, modifiers: m });
        h.run_steps(1);
    }
    h.run_steps(2);
}

/// Inspector size fields: (B, H) = the text input showing the height "50" (H) and the "100" right
/// above it in the same column (B); the labels are painted, the fields carry the values.
fn width_height_fields(h: &H) -> (egui::Rect, egui::Rect) {
    use egui::accesskit::Role;
    use egui_kittest::kittest::{By, Queryable};
    let inputs: Vec<(egui::Rect, String)> = h.query_all(By::new().role(Role::TextInput)).map(|n| (n.rect(), n.value().unwrap_or_default())).collect();
    let hf = inputs.iter().filter(|(_, v)| v == "50").map(|(r, _)| *r).max_by(|a, b| a.center().y.total_cmp(&b.center().y)).expect("field H");
    let bf = inputs
        .iter()
        .filter(|(r, v)| v == "100" && (r.left() - hf.left()).abs() < 5.0 && r.center().y < hf.center().y)
        .map(|(r, _)| *r)
        .max_by(|a, b| a.center().y.total_cmp(&b.center().y))
        .expect("field B");
    (bf, hf)
}

/// CT-B-12 a (re 33 §8 a): Tab in the inspector field B (width) takes the value over as its own
/// step and moves the focus into H (order B → H, not X); "77" + Enter then sets the height.
#[test]
fn tab_in_width_field_commits_and_moves_to_height() {
    use egui::accesskit::Role;
    use egui_kittest::kittest::{By, NodeT, Queryable};
    let mut s = new_session(1);
    let r = s.execute("draw.rectangle", json!({"rect": [100, 100, 200, 150]})).expect("rect");
    let id = r.value["id"].as_u64().expect("id");
    s.execute("select.set", json!({"ids": [id], "mode": "replace"})).expect("select");
    let mut h = harness_with(s, 1500.0, 950.0);
    let n0 = undo_count(&h);
    let size = |h: &H| {
        let d = h.state().state.session.active().expect("doc");
        let e = &d.doc.pages[d.page].root.layers.iter().find_map(|l| match l {
            fp_model::LayerSlot::Own(l) => l.cels.first().and_then(|c| c.elements.first().cloned()),
            _ => None,
        });
        let b = fp_engine::docops::element_bounds(&d.doc, e.as_ref().expect("element")).expect("bounds");
        (b.right - b.left, b.bottom - b.top)
    };
    assert_eq!(size(&h), (100.0, 50.0));
    let (bf, hf) = width_height_fields(&h);
    let p = bf.center();
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    press(&mut h, p, true);
    press(&mut h, p, false);
    h.run_steps(3);
    assert!(h.ctx.text_edit_focused(), "click focuses field B");
    key_ev(&mut h, Key::A, Modifiers::COMMAND);
    h.event(Event::Text("123".into()));
    h.run_steps(2);
    key_ev(&mut h, Key::Tab, Modifiers::NONE);
    h.run_steps(3);
    assert!(!h.state().state.settings.workspace.panels_hidden, "Tab stays in the inspector");
    assert_eq!(size(&h), (123.0, 50.0), "Tab takes the width over");
    assert_eq!(undo_count(&h), n0 + 1, "own step for B");
    assert!(h.ctx.text_edit_focused(), "focus moved on into a field");
    let focused: Vec<(egui::Rect, String)> = h
        .query_all(By::new().role(Role::TextInput))
        .filter(|n| n.accesskit_node().is_focused())
        .map(|n| (n.rect(), n.value().unwrap_or_default()))
        .collect();
    let (fr, fv) = focused.first().cloned().unwrap_or_else(|| panic!("no focused text input"));
    assert!(fr.center().distance(hf.center()) < 4.0, "focus in H (B → H, not X): {fr:?} vs H {hf:?}");
    assert_eq!(fv.trim(), "50", "H shows the height");
    key_ev(&mut h, Key::A, Modifiers::COMMAND);
    h.event(Event::Text("77".into()));
    h.run_steps(2);
    key_ev(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(3);
    assert_eq!(size(&h), (123.0, 77.0));
    assert_eq!(undo_count(&h), n0 + 2, "own step for H");
}
