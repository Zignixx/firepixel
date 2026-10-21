//! FB-13 / TW-184 (RE spec 57 §4): tooltips as measured in the reference application.
//! - Toolbar buttons: the command name only, no shortcut ("New", "Undo").
//! - Tools: "Name (shortcut)" plus a second line with a one-sentence description (Firepixel extension).
//! - Inspector of the retouch tools: descriptive tooltips ("Brush size", "Edge softness", ...), not the labels.
//! - View ▸ Tooltips: a coordinate tag "x:… y:…" next to the pointer over the canvas, only with the
//!   switch on, also while a mouse button is held (dragging).

use egui::{Event, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fp_engine::Session;
use fp_engine::tools::{self, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn harness() -> H {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"}))
        .expect("document");
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.os_clipboard = false;
    a.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(8)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(4);
    h
}

fn ev(h: &mut H, e: Event) {
    h.input_mut().events.push(e);
    h.step();
}

#[test]
fn toolbar_tooltips_show_the_name_without_shortcut() {
    let h = harness();
    let st = &h.state().state;
    fp_i18n::with_language("de", || {
        assert_eq!(fp_ui::toolbar::tooltip(st, "file.new"), "Neu");
        assert_eq!(fp_ui::toolbar::tooltip(st, "edit.undo"), "Rückgängig");
    });
    fp_i18n::with_language("en", || {
        assert_eq!(fp_ui::toolbar::tooltip(st, "file.new"), "New");
        assert_eq!(fp_ui::toolbar::tooltip(st, "edit.undo"), "Undo");
        assert_eq!(fp_ui::toolbar::tooltip(st, "file.save"), "Save");
    });
}

#[test]
fn tool_tooltips_have_name_shortcut_and_a_description_line() {
    let h = harness();
    let st = &h.state().state;
    fp_i18n::with_language("de", || {
        assert_eq!(fp_ui::tools_panel::tool_title(st, ToolId::Pointer), "Zeiger (V, 0)");
        assert_eq!(fp_ui::tools_panel::tool_title(st, ToolId::Smudge), "Verwischen (R)");
    });
    for lang in ["de", "en"] {
        fp_i18n::with_language(lang, || {
            for t in tools::TOOLS {
                let title = fp_ui::tools_panel::tool_title(st, t.id);
                let desc = fp_ui::tools_panel::tool_description(t.id);
                assert!(!desc.is_empty(), "{lang}: no description for {}", t.name);
                assert!(desc.ends_with('.') && !desc.contains('\n'), "{lang}: one sentence: {desc}");
                assert!(desc.chars().count() <= 90, "{lang}: short: {desc}");
                assert_eq!(
                    fp_ui::tools_panel::tool_tooltip(st, t.id),
                    format!("{title}\n{desc}"),
                    "{lang}: {}",
                    t.name
                );
            }
        });
    }
    let de = fp_i18n::with_language("de", || fp_ui::tools_panel::tool_description(ToolId::Smudge));
    let en = fp_i18n::with_language("en", || fp_ui::tools_panel::tool_description(ToolId::Smudge));
    assert_ne!(de, en, "translated");
}

/// Tooltips of the inspector controls of a tool, in drawing order.
fn inspector_tips(h: &mut H, tool: ToolId) -> Vec<(String, egui::Rect)> {
    let _ = h.state_mut().state.session.set_tool(tool);
    h.run_steps(2);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    let _ = fp_ui::inspector::fields::take_tips(&h.ctx);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_tips(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

fn texts(v: &[(String, egui::Rect)]) -> Vec<&str> {
    v.iter().map(|(t, _)| t.as_str()).collect()
}

#[test]
fn retouch_inspector_controls_carry_the_measured_descriptions() {
    let mut h = harness();
    let smudge = fp_i18n::with_language("de", || inspector_tips(&mut h, ToolId::Smudge));
    assert_eq!(
        texts(&smudge),
        [
            "Pinselstärke",
            "Pinselvorschau",
            "Kantenweichheit",
            "Runde Pinselspitze",
            "Quadratische Pinselspitze",
            "Druck",
            "Beim Verwischen Farbe verwenden",
            "Alle sichtbaren Objekte verwischen",
        ]
    );
    let tips_de = |h: &mut H, t: ToolId| fp_i18n::with_language("de", || inspector_tips(h, t));
    let blur = tips_de(&mut h, ToolId::Blur);
    assert!(texts(&blur).starts_with(&["Pinselstärke", "Pinselvorschau", "Kantenweichheit"]));
    assert!(texts(&blur).contains(&"Intensität"), "{blur:?}");
    let burn = tips_de(&mut h, ToolId::Burn);
    assert!(texts(&burn).contains(&"Betroffener Bereich") && texts(&burn).contains(&"Belichtung"));
    let stamp = tips_de(&mut h, ToolId::RubberStamp);
    assert!(texts(&stamp).starts_with(&["Stempelgröße", "Pinselvorschau", "Kantenweichheit"]));
    let eraser = tips_de(&mut h, ToolId::Eraser);
    assert!(texts(&eraser).contains(&"Runder Radiergummi"), "{eraser:?}");
    let smudge_en = fp_i18n::with_language("en", || inspector_tips(&mut h, ToolId::Smudge));
    assert_eq!(texts(&smudge_en)[..3], ["Brush size", "Brush preview", "Edge softness"]);
    assert!(texts(&smudge_en).contains(&"Use color when smudging"));
    assert!(texts(&smudge_en).contains(&"Smudge all visible objects"));

    // the tooltip really appears when the pointer rests on the size field ...
    let size = smudge[0].1;
    fp_i18n::with_language("de", || {
        ev(&mut h, Event::PointerMoved(size.center()));
        for _ in 0..60 {
            h.step();
        }
    });
    assert!(h.query_by_label("Pinselstärke").is_some(), "tooltip shown");
    // ... and the field underneath still takes the click (the tooltip area senses hover only)
    ev(&mut h, Event::PointerButton { pos: size.center(), button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
    ev(&mut h, Event::PointerButton { pos: size.center(), button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
    let te = egui::Id::new(("smudge", "size")).with("te");
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(te), "size field focused");
}

fn page_origin(h: &H) -> Pos2 {
    h.state().state.canvases.values().next().and_then(|c| c.page_screen).expect("page drawn").min
}

fn tag(h: &H) -> Option<String> {
    h.state().state.canvases.values().next().and_then(|c| c.pointer_tag.clone())
}

#[test]
fn view_tooltips_shows_the_pointer_coordinates_on_the_canvas() {
    fp_i18n::with_language("de", view_tooltips_body);
}

fn view_tooltips_body() {
    let mut h = harness();
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    h.run_steps(3);
    assert!(h.state().state.session.ui.tooltips, "default of the switch is kept (on)");
    let o = page_origin(&h);
    ev(&mut h, Event::PointerMoved(o + vec2(120.2, 80.2)));
    h.run_steps(1);
    assert_eq!(tag(&h).as_deref(), Some("x:120 y:80"));
    // also while a mouse button is held (the reference shows it while dragging a bitmap)
    let p = o + vec2(200.2, 150.2);
    ev(&mut h, Event::PointerMoved(p));
    ev(&mut h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
    assert_eq!(tag(&h).as_deref(), Some("x:200 y:150"), "button pressed");
    ev(&mut h, Event::PointerMoved(p + vec2(20.0, 10.0)));
    ev(&mut h, Event::PointerMoved(p + vec2(40.0, 20.0)));
    assert_eq!(tag(&h).as_deref(), Some("x:240 y:170"), "follows the pointer while dragging");
    ev(&mut h, Event::PointerButton { pos: p + vec2(40.0, 20.0), button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
    assert_eq!(tag(&h).as_deref(), Some("x:240 y:170"), "after the release");
    // switched off: no tag
    h.state_mut().state.run("view.tooltips", json!({}));
    h.run_steps(2);
    assert!(!h.state().state.session.ui.tooltips);
    ev(&mut h, Event::PointerMoved(o + vec2(50.2, 40.2)));
    h.run_steps(1);
    assert_eq!(tag(&h), None, "switch off");
    // on again
    h.state_mut().state.run("view.tooltips", json!({}));
    ev(&mut h, Event::PointerMoved(o + vec2(51.2, 41.2)));
    h.run_steps(1);
    assert_eq!(tag(&h).as_deref(), Some("x:51 y:41"));
    // pointer off the canvas: no tag
    ev(&mut h, Event::PointerMoved(egui::pos2(5.0, 5.0)));
    h.run_steps(1);
    assert_eq!(tag(&h), None, "outside the canvas");
}
