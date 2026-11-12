//! FB-149 (TW-295, Firepixel extension): with an object selected and a tool with options active,
//! the Properties panel shows a tool options section below the object properties (icon,
//! "Current tool", name, options). Pointer group and selection tools: the existing Firepixel
//! preferences as check boxes on the same keys; tools with their own tool row after the object
//! rows (Rectangle, Pen, …): that row. The swap button puts the section on top and the choice is
//! kept in the workspace.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fp_engine::prefs::PrefValue;
use fp_engine::tools::{ALT_MARQUEE_DESELECT_PREF, ToolId};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::inspector::tool_section::{PREF_CHECKS, Section, section_for, set_pref};
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

fn session(tool: ToolId, select: bool) -> fp_engine::Session {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 400, "height": 300}))
        .unwrap_or_else(|e| panic!("new: {e}"));
    s.execute("draw.rectangle", json!({"rect": [20, 20, 120, 90]}))
        .unwrap_or_else(|e| panic!("rect: {e}"));
    if !select {
        let _ = s.execute("select.none", json!({}));
    }
    let _ = s.set_tool(tool);
    s
}

fn harness(s: fp_engine::Session) -> H {
    fp_i18n::set_language("en");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    // tall panel: the section below the two rows is visible without scrolling
    app.state.settings.workspace.inspector_height = 330.0;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1600.0, 1000.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(12)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn label() -> String {
    fp_i18n::t("dlg-pref-altmarqueedeselects")
}

#[test]
fn fb149_section_for_the_listed_tools_only_with_a_selection() {
    for t in [ToolId::Pointer, ToolId::Subselection] {
        assert_eq!(
            section_for(&session(t, true)),
            Some(Section::Prefs { marquee: true }),
            "{t:?}"
        );
        assert_eq!(section_for(&session(t, false)), None, "{t:?} without selection");
    }
    for t in [
        ToolId::Marquee,
        ToolId::OvalMarquee,
        ToolId::Lasso,
        ToolId::PolygonLasso,
        ToolId::MagicWand,
    ] {
        // a selection tool deselects vector objects (RE spec 72): a pixel selection
        let mut s = session(t, false);
        s.execute("select.pixels", json!({"shape": "rect", "rect": [20, 20, 60, 60]}))
            .unwrap_or_else(|e| panic!("pixels: {e}"));
        assert_eq!(
            section_for(&s),
            Some(Section::Prefs { marquee: false }),
            "{t:?}"
        );
        assert_eq!(section_for(&session(t, false)), None, "{t:?} without selection");
    }
    // tools with their own tool row after the object rows: that row
    for t in [ToolId::Rectangle, ToolId::Pen, ToolId::Line, ToolId::Ellipse] {
        assert!(
            // without the path buttons (Combine …) of the object panel
            matches!(section_for(&session(t, true)), Some(Section::ToolRow { quads, .. }) if !quads.contains(&14132)),
            "{t:?}"
        );
        assert_eq!(section_for(&session(t, false)), None, "{t:?} without selection");
    }
    // tools without options, and tools whose options already replace the object panel
    for t in [
        ToolId::Hand,
        ToolId::Zoom,
        ToolId::Crop,
        ToolId::Slice,
        ToolId::Brush,
        ToolId::Text,
    ] {
        assert_eq!(section_for(&session(t, true)), None, "{t:?}");
    }
}

#[test]
fn fb149_check_box_and_preference_both_ways() {
    let mut h = harness(session(ToolId::Pointer, true));
    assert!(!h.state().state.session.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false));
    for (_, k) in PREF_CHECKS {
        assert!(h.query_by_label(&fp_i18n::t(k)).is_some(), "{k} shown");
    }
    assert!(
        h.query_by_label(&fp_i18n::t("pi-current-tool")).is_none(),
        "header is painted text"
    );
    h.get_by_label(&label()).click();
    h.run_steps(3);
    assert!(
        h.state().state.session.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false),
        "the check box sets the preference"
    );
    h.get_by_label(&label()).click();
    h.run_steps(3);
    assert!(!h.state().state.session.prefs.get_bool(ALT_MARQUEE_DESELECT_PREF, false));
    // the other way: the preference (as the Preferences dialog sets it) checks the box
    h.state_mut()
        .state
        .session
        .prefs
        .set(ALT_MARQUEE_DESELECT_PREF, PrefValue::Bool(true));
    h.run_steps(3);
    let on = h.get_by_label(&label()).accesskit_node().toggled();
    assert_eq!(on, Some(egui::accesskit::Toggled::True));
    set_pref(&mut h.state_mut().state, ALT_MARQUEE_DESELECT_PREF, false);
    h.run_steps(3);
    let off = h.get_by_label(&label()).accesskit_node().toggled();
    assert_eq!(off, Some(egui::accesskit::Toggled::False));
}

#[test]
fn fb149_swap_puts_the_section_on_top_and_persists() {
    let mut h = harness(session(ToolId::Pointer, true));
    let y_check = |h: &H| h.get_by_label(&label()).rect().center().y;
    let swap = fp_i18n::t("pi-tool-section-swap");
    let before = y_check(&h);
    let swap_y = h.get_by_label(&swap).rect().center().y;
    assert!(before > swap_y, "section below the separator");
    h.get_by_label(&swap).click();
    h.run_steps(3);
    assert!(h.state().state.settings.workspace.inspector_tool_section_on_top);
    let after = y_check(&h);
    let swap_y2 = h.get_by_label(&swap).rect().center().y;
    assert!(after < swap_y2 && after < before, "section on top: {before} -> {after}");
    // kept with the workspace like the other panel settings
    let ws = h.state().state.settings.workspace.clone();
    let text = serde_json::to_string(&ws).unwrap_or_default();
    let back: fp_ui::workspace::Workspace =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert!(back.inspector_tool_section_on_top);
    // older workspace files without the key: below
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
    if let Some(o) = v.as_object_mut() {
        o.remove("inspector_tool_section_on_top");
    }
    let old: fp_ui::workspace::Workspace =
        serde_json::from_value(v).unwrap_or_else(|e| panic!("{e}"));
    assert!(!old.inspector_tool_section_on_top);
    h.get_by_label(&swap).click();
    h.run_steps(3);
    assert!(!h.state().state.settings.workspace.inspector_tool_section_on_top);
}

#[test]
fn fb149_no_section_without_options_or_selection() {
    let h = harness(session(ToolId::Hand, true));
    assert!(h.query_by_label(&label()).is_none());
    assert!(h.query_by_label(&fp_i18n::t("pi-tool-section-swap")).is_none());
    let h = harness(session(ToolId::Pointer, false));
    assert!(h.query_by_label(&label()).is_none(), "nothing selected: as before");
}

#[test]
fn fb149_rectangle_tool_row_below_the_object() {
    let h = harness(session(ToolId::Rectangle, true));
    assert!(h.query_by_label(&fp_i18n::t("pi-tool-section-swap")).is_some());
    assert!(h.query_by_label(&label()).is_none(), "tool row, not the check boxes");
}
