//! FB-13 / TW-184 (RE spec 57 §4): tooltips as measured in the reference application.
//! - Toolbar buttons: the command name only, no shortcut ("New", "Undo").
//! - Tools: "Name (shortcut)" plus a second line with a one-sentence description (Firepixel extension).
//! - Inspector of the retouch tools: descriptive tooltips ("Brush size", "Edge softness", ...), not the labels.
//! - Text inspector and the shared name, W/H/X/Y, filter and style controls (FB-22, TW-201, RE spec 60 §4).
//! - Coordinate tag (re 58 §4) next to the pointer over the canvas, on its own switch View ▸
//!   Coordinate Tag since FB-41 (default on): shape tools show "x:… y:…" on hover and "w:… h:…"
//!   while drawing, a Pointer drag shows the dragged object's new top-left corner, Pointer/Pen/Text
//!   show nothing on hover.
//! - View ▸ Tooltips (FB-37) switches the tooltips of buttons and fields at once (default on).

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
fn coordinate_tag_shows_the_pointer_coordinates_on_the_canvas() {
    fp_i18n::with_language("de", coordinate_tag_body);
}

fn press(h: &mut H, p: Pos2, pressed: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed, modifiers: Default::default() });
}

fn tag_on(h: &H) -> bool {
    h.state().state.session.prefs.get_bool(fp_engine::session::COORDINATE_TAG_PREF, true)
}

/// The coordinate tag per tool (re 58 §4, measured): no tag on hover with Pointer, Pen and Text;
/// shape tools show the pointer position on hover and the size while drawing; a Pointer drag
/// shows the dragged object's new top-left corner; the tag goes away on release. Its switch is
/// View ▸ Coordinate Tag (FB-41, default on), independent of View ▸ Tooltips.
fn coordinate_tag_body() {
    let mut h = harness();
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    let _ = h.state_mut().state.session.execute("view.setZoom", json!({"zoom": 1.0}));
    h.run_steps(3);
    assert!(tag_on(&h), "View > Coordinate Tag is on by default (FB-41)");
    assert!(h.state().state.session.ui.tooltips, "View > Tooltips is on by default (FB-37)");
    let o = page_origin(&h);
    // Pointer, Pen, Text: nothing on hover
    for t in [ToolId::Pointer, ToolId::Pen, ToolId::Text] {
        let _ = h.state_mut().state.session.set_tool(t);
        ev(&mut h, Event::PointerMoved(o + vec2(120.2, 80.2)));
        h.run_steps(1);
        assert_eq!(tag(&h), None, "{t:?}: no tag on hover");
    }
    // Rectangle (and the other shape tools): pointer position on hover
    for t in [ToolId::Ellipse, ToolId::Line, ToolId::Rectangle] {
        let _ = h.state_mut().state.session.set_tool(t);
        ev(&mut h, Event::PointerMoved(o + vec2(53.2, 37.2)));
        h.run_steps(1);
        assert_eq!(tag(&h).as_deref(), Some("x:53 y:37"), "{t:?}: hover");
    }
    // View > Tooltips off does not hide the tag (own switch since FB-41)
    h.state_mut().state.run("view.tooltips", json!({}));
    h.run_steps(2);
    assert!(!h.state().state.session.ui.tooltips);
    ev(&mut h, Event::PointerMoved(o + vec2(54.2, 38.2)));
    h.run_steps(1);
    assert_eq!(tag(&h).as_deref(), Some("x:54 y:38"), "tag independent of View > Tooltips");
    h.state_mut().state.run("view.tooltips", json!({}));
    h.run_steps(2);
    // drawing a rectangle: its size
    let a = o + vec2(100.0, 100.0);
    ev(&mut h, Event::PointerMoved(a));
    press(&mut h, a, true);
    ev(&mut h, Event::PointerMoved(a + vec2(40.0, 25.0)));
    ev(&mut h, Event::PointerMoved(a + vec2(80.0, 50.0)));
    assert_eq!(tag(&h).as_deref(), Some("w:80 h:50"), "size while drawing");
    press(&mut h, a + vec2(80.0, 50.0), false);
    h.run_steps(2);
    assert!(tag(&h).is_none_or(|t| !t.starts_with("w:")), "size tag gone after the release");
    assert_eq!(h.state().state.session.active().map(|d| d.selection.elements.len()), Some(1), "rectangle drawn");
    // Pointer: pressing without movement shows nothing, dragging the object shows its new
    // top-left corner (rectangle 100,100 moved by +30/+20 → x:130 y:120)
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    let g = o + vec2(140.2, 125.2);
    ev(&mut h, Event::PointerMoved(g));
    h.run_steps(1);
    assert_eq!(tag(&h), None, "Pointer hover over the object");
    press(&mut h, g, true);
    assert_eq!(tag(&h), None, "pressed without movement");
    ev(&mut h, Event::PointerMoved(g + vec2(15.0, 10.0)));
    ev(&mut h, Event::PointerMoved(g + vec2(30.0, 20.0)));
    h.run_steps(1);
    assert_eq!(tag(&h).as_deref(), Some("x:130 y:120"), "object top-left while dragging");
    press(&mut h, g + vec2(30.0, 20.0), false);
    h.run_steps(2);
    assert_eq!(tag(&h), None, "gone after the release");
    // View > Coordinate Tag off: no tag on hover nor while drawing; stored as a preference
    let _ = h.state_mut().state.session.set_tool(ToolId::Rectangle);
    h.state_mut().state.run("view.coordinateTag", json!({}));
    h.run_steps(2);
    assert!(!tag_on(&h), "preference Firepixel_CoordinateTag off");
    assert!(h.state().state.session.ui.tooltips, "View > Tooltips untouched");
    ev(&mut h, Event::PointerMoved(o + vec2(50.2, 40.2)));
    h.run_steps(1);
    assert_eq!(tag(&h), None, "switch off: no tag on hover");
    let b = o + vec2(300.0, 200.0);
    ev(&mut h, Event::PointerMoved(b));
    press(&mut h, b, true);
    ev(&mut h, Event::PointerMoved(b + vec2(30.0, 20.0)));
    ev(&mut h, Event::PointerMoved(b + vec2(60.0, 40.0)));
    assert_eq!(tag(&h), None, "switch off: no tag while drawing");
    press(&mut h, b + vec2(60.0, 40.0), false);
    h.run_steps(2);
    // on again
    h.state_mut().state.run("view.coordinateTag", json!({}));
    h.run_steps(2);
    assert!(tag_on(&h), "preference Firepixel_CoordinateTag on");
    ev(&mut h, Event::PointerMoved(o + vec2(51.2, 41.2)));
    h.run_steps(1);
    assert_eq!(tag(&h).as_deref(), Some("x:51 y:41"));
    let c = o + vec2(400.0, 100.0);
    ev(&mut h, Event::PointerMoved(c));
    press(&mut h, c, true);
    ev(&mut h, Event::PointerMoved(c + vec2(20.0, 10.0)));
    ev(&mut h, Event::PointerMoved(c + vec2(40.0, 30.0)));
    assert_eq!(tag(&h).as_deref(), Some("w:40 h:30"), "switch on: size while drawing");
    press(&mut h, c + vec2(40.0, 30.0), false);
    h.run_steps(2);
    // pointer off the canvas: no tag
    ev(&mut h, Event::PointerMoved(egui::pos2(5.0, 5.0)));
    h.run_steps(1);
    assert_eq!(tag(&h), None, "outside the canvas");
    // a new session reads the stored preferences
    let mut prefs = fp_engine::Preferences::new();
    prefs.set(fp_engine::session::COORDINATE_TAG_PREF, fp_engine::prefs::PrefValue::Bool(false));
    prefs.set("Tooltips", fp_engine::prefs::PrefValue::Bool(false));
    let s = Session::new(prefs);
    assert!(!fp_engine::session::coordinate_tag(&s.prefs), "stored tag off is read back");
    assert!(!s.ui.tooltips, "stored tooltips off is read back");
    assert_eq!(s.command_state("view.coordinateTag").1, Some(false));
    assert_eq!(s.command_state("view.tooltips").1, Some(false));
    let s = Session::new(fp_engine::Preferences::new());
    assert!(s.ui.tooltips && fp_engine::session::coordinate_tag(&s.prefs), "defaults on");
}

/// Commands of View ▸ Tooltips and View ▸ Coordinate Tag in the built View menu, in menu order,
/// with their label (current language) and check mark.
fn view_switches(h: &H) -> Vec<(String, String, Option<bool>)> {
    use fp_ui::menu::{ItemAction, Node};
    let menu = fp_ui::menu::build_main_menu(&h.state().state);
    let view = &menu[2].1;
    view.iter()
        .filter_map(|n| match n {
            Node::Item { label, checked, action: ItemAction::Command(c), .. }
                if c == "view.tooltips" || c == "view.coordinateTag" =>
            {
                Some((c.clone(), label.clone(), *checked))
            }
            _ => None,
        })
        .collect()
}

fn view_checks(h: &H) -> Vec<(String, Option<bool>)> {
    view_switches(h).into_iter().map(|(c, _, k)| (c, k)).collect()
}

/// Rests the pointer on `at` for one second (tooltip delay 0.6 s) and tells whether a tooltip layer
/// is open.
fn rest_and_tooltip_open(h: &mut H, at: Pos2) -> bool {
    ev(h, Event::PointerGone);
    h.run_steps(2);
    ev(h, Event::PointerMoved(at));
    for _ in 0..60 {
        h.step();
    }
    h.ctx.memory(|m| m.areas().visible_layer_ids().iter().any(|l| l.order == egui::Order::Tooltip))
}

/// FB-37: View ▸ Tooltips switches the tooltips of the toolbox buttons (and every other widget) at
/// once; stored as the preference `Tooltips`; the menu's check mark follows.
#[test]
fn view_tooltips_switches_widget_tooltips_at_once() {
    fp_i18n::with_language("de", || {
        let mut h = harness();
        let tip = fp_ui::tools_panel::tool_tooltip(&h.state().state, ToolId::Pointer);
        let label = tip.lines().next().unwrap_or_default().to_string();
        let button = h.query_all_by_label(&label).next().map(|n| n.rect()).expect("Pointer button");
        let on_on = vec![("view.tooltips".to_string(), Some(true)), ("view.coordinateTag".to_string(), Some(true))];
        assert_eq!(view_checks(&h), on_on, "both checked by default, Coordinate Tag right after Tooltips");
        // on (default): resting on the Pointer button shows its tooltip
        assert!(rest_and_tooltip_open(&mut h, button.center()), "tooltip with the switch on");
        assert!(h.query_by_label(&tip).is_some(), "the tool's tooltip text: {tip:?}");
        // off: no tooltip, preference stored, check mark gone; the tag switch stays
        h.state_mut().state.run("view.tooltips", json!({}));
        h.run_steps(1);
        assert!(!h.state().state.session.prefs.get_bool("Tooltips", true), "preference Tooltips off");
        assert_eq!(
            view_checks(&h),
            vec![("view.tooltips".to_string(), Some(false)), ("view.coordinateTag".to_string(), Some(true))]
        );
        assert!(!rest_and_tooltip_open(&mut h, button.center()), "no tooltip with the switch off");
        assert!(h.query_by_label(&tip).is_none(), "no tool tooltip text");
        // on again: the tooltip comes back with the normal delay
        h.state_mut().state.run("view.tooltips", json!({}));
        h.run_steps(1);
        assert!(h.state().state.session.prefs.get_bool("Tooltips", false), "preference Tooltips on");
        assert!(h.ctx.global_style().interaction.tooltip_delay.is_finite(), "delay restored");
        assert!(rest_and_tooltip_open(&mut h, button.center()), "tooltip back");
        assert!(h.query_by_label(&tip).is_some(), "the tool's tooltip text again");
        // View > Coordinate Tag off: its check mark follows, Tooltips stays checked
        h.state_mut().state.run("view.coordinateTag", json!({}));
        h.run_steps(1);
        assert_eq!(
            view_checks(&h),
            vec![("view.tooltips".to_string(), Some(true)), ("view.coordinateTag".to_string(), Some(false))]
        );
        assert!(!h.state().state.session.prefs.get_bool(fp_engine::session::COORDINATE_TAG_PREF, true));
    });
}

/// The two menu entries are translated (DE/EN).
#[test]
fn view_switch_labels_de_en() {
    let h = harness();
    let labels = |lang: &str| {
        fp_i18n::with_language(lang, || view_switches(&h).into_iter().map(|(_, l, _)| l).collect::<Vec<_>>())
    };
    assert_eq!(labels("de"), ["QuickInfos", "Koordinaten-Tag"]);
    assert_eq!(labels("en"), ["Tooltips", "Coordinate Tag"]);
}

/// Tooltips of the Property Inspector with a selected text object, in drawing order.
fn text_inspector_tips(h: &mut H) -> Vec<(String, egui::Rect)> {
    h.run_steps(2);
    fp_ui::inspector::fields::set_audit(&h.ctx, true);
    let _ = fp_ui::inspector::fields::take_tips(&h.ctx);
    h.run_steps(1);
    let v = fp_ui::inspector::fields::take_tips(&h.ctx);
    fp_ui::inspector::fields::set_audit(&h.ctx, false);
    v
}

/// FB-22 / TW-201 (RE spec 60 §4): the text inspector and the shared controls (name, W/H/X/Y,
/// filter, style) carry the measured descriptions; the colour wells show their hex value, a
/// stroke "none" "#FFFFFF00"; nothing on the arrows, glyphs, labels and the style menu.
#[test]
fn text_inspector_controls_carry_the_measured_descriptions() {
    let mut h = harness();
    let _ = h.state_mut().state.session.set_tool(ToolId::Pointer);
    h.state_mut()
        .state
        .session
        .execute("draw.text", json!({"text": "Schieber Test", "anchor": [40, 60], "size": 24, "color": "#000000"}))
        .expect("text");
    let de = fp_i18n::with_language("de", || text_inspector_tips(&mut h));
    assert_eq!(
        texts(&de),
        [
            // object quadrant: name field, then lock and W/H/X/Y
            "Objektnamen bearbeiten",
            "Proportionen beibehalten",
            "Auswahlbreite",
            "Auswahlhöhe",
            "X-Position der Auswahl",
            "Y-Position der Auswahl",
            // text row 1 (no tip on the size, AV and leading arrows nor on the glyphs)
            "Schriftfamilie",
            "Schriftstil",
            "Größe",
            "#000000",
            "Unterschneidung oder Laufweite",
            "Zeilenabstand",
            "Einheit für Zeilenabstand",
            "Faux Fett",
            "Faux Kursiv",
            "Unterstrichen",
            "Textausrichtung festlegen",
            "Linksbündig",
            "Zentrieren",
            "Rechtsbündig",
            "Blocksatz",
            // opacity, blend mode, filters
            "Deckkraft",
            "Mischmodus",
            "Live-Filter hinzufügen oder Vorgabe wählen",
            "Aktuell ausgewählten Live-Filter löschen",
            // text row 2; the stroke well without stroke shows transparent white
            "Absatzeinzug",
            "Leerraum vor Absatz",
            "Horizontale Skalierung",
            "Leerraum nach Absatz",
            "#FFFFFF00",
            "Anti-Aliasing-Stufe",
            "Unterschneidung von Schriftarten verwenden",
            // style buttons (no tip on the "No Style" menu); the last two are Firepixel extras
            "Neuer Stil",
            "Stil neu definieren",
            "Abweichungen löschen",
            "Verknüpfung zu Stil aufheben",
            "Stil löschen",
        ]
    );
    let en = fp_i18n::with_language("en", || text_inspector_tips(&mut h));
    assert_eq!(
        texts(&en),
        [
            "Edit object name",
            "Constrain proportions",
            "Selection width",
            "Selection height",
            "Selection X position",
            "Selection Y position",
            "Font family",
            "Font style",
            "Size",
            "#000000",
            "Kerning or tracking",
            "Leading",
            "Leading unit",
            "Faux Bold",
            "Faux Italic",
            "Underline",
            "Set text orientation",
            "Left align",
            "Center",
            "Right align",
            "Justify",
            "Opacity",
            "Blending mode",
            "Add Live Filters or choose a preset",
            "Delete the selected Live Filter",
            "Paragraph indent",
            "Space before paragraph",
            "Horizontal scale",
            "Space after paragraph",
            "#FFFFFF00",
            "Anti-aliasing level",
            "Use font kerning",
            "New Style",
            "Redefine Style",
            "Clear Overrides",
            "Break Link to Style",
            "Delete Style",
        ]
    );
    // the tip areas cover the fields only, not the slider arrows right of them
    let av = de.iter().find(|(t, _)| t == "Unterschneidung oder Laufweite").expect("AV tip").1;
    let size = de.iter().find(|(t, _)| t == "Größe").expect("size tip").1;
    // (field 27 x 13 DLU; with the 9 DLU arrow it would be wider than 2.5 : 1)
    for r in [av, size] {
        assert!(r.width() / r.height() < 2.2, "field only: {r:?}");
    }

    // a real hover on the AV field shows the description ...
    fp_i18n::with_language("de", || {
        ev(&mut h, Event::PointerMoved(av.center()));
        for _ in 0..60 {
            h.step();
        }
    });
    assert!(h.query_by_label("Unterschneidung oder Laufweite").is_some(), "tooltip shown");
    // ... and the field underneath still takes the click
    ev(&mut h, Event::PointerButton { pos: av.center(), button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
    ev(&mut h, Event::PointerButton { pos: av.center(), button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
    h.run_steps(2);
    let te = egui::Id::new(egui::Id::new(("pi-text", false)).with("track")).with("te");
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(te), "AV field focused");
}
