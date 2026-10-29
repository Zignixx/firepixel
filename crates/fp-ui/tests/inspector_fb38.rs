//! FB-38 (Firepixel extensions) in the Properties inspector:
//! - the fill section has a per-cent field with a small checkerboard icon (uniform dither density of
//!   the whole fill) below the texture name; typing a value applies it as one fill history row;
//! - the live filter menu "+" has "Screentone" ("Rasterfolie") under Other; its dialog adds the
//!   effect with all parameters, the edit dialog of an existing Screentone changes the same entry.
//!
//! Realistic input like egui-winit on Windows (one event per frame, typing as `Event::Text`).

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::dialogs::effects;
use serde_json::json;

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn harness_with_rect() -> (H, u64) {
    let mut s = fp_engine::Session::default();
    s.execute("file.new", json!({"width": 500, "height": 300, "background": "#ffffff"})).expect("doc");
    let v = s.execute("draw.rectangle", json!({"rect": [40, 40, 300, 200], "fillColor": "#336699", "strokeWidth": 0})).expect("rect");
    let id = v.value.get("id").and_then(|x| x.as_u64()).expect("id");
    s.execute("select.set", json!({"ids": [id]})).expect("select");
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder().with_size(egui::vec2(W, HH)).with_pixels_per_point(1.0).with_max_steps(20).with_step_dt(1.0 / 60.0).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    (h, id)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn key(h: &mut H, k: Key, m: Modifiers) {
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: true, repeat: false, modifiers: m });
    ev(h, Event::Key { key: k, physical_key: Some(k), pressed: false, repeat: false, modifiers: m });
}

fn click(h: &mut H, p: Pos2) {
    ev(h, Event::PointerMoved(p));
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn element(h: &H, id: u64) -> std::sync::Arc<fp_model::Element> {
    let d = st(h).session.active().expect("doc");
    fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(id)).expect("element")
}

fn density(h: &H, id: u64) -> i32 {
    fp_engine::docops::path_attrs_of(&element(h, id)).and_then(|a| a.fill.as_ref().map(|f| f.dither_density)).expect("fill")
}

fn undo_count(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

/// The dither density field: the inspector text field left of "Transparent" in its row (FB-88:
/// the bare number like the texture amount field).
fn density_field(h: &H) -> (egui::Rect, String, bool) {
    let tr = h.get_by_label(&fp_i18n::t("pi-texture-transparent")).rect();
    h.get_all(By::new().role(egui::accesskit::Role::TextInput))
        .filter(|n| (n.rect().center().y - tr.center().y).abs() < 3.0 && n.rect().right() <= tr.left())
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .map(|n| (n.rect(), n.value().unwrap_or_default(), !n.accesskit_node().is_disabled()))
        .expect("dither density field")
}

#[test]
fn dither_density_field_applies_one_fill_row() {
    let (mut h, id) = harness_with_rect();
    let (f, value, enabled) = density_field(&h);
    assert_eq!(value, "0");
    assert!(enabled);
    let rows = undo_count(&h);
    click(&mut h, f.center());
    key(&mut h, Key::End, Modifiers::NONE);
    key(&mut h, Key::Home, Modifiers::SHIFT);
    for c in "50".chars() {
        ev(&mut h, Event::Text(c.to_string()));
    }
    key(&mut h, Key::Enter, Modifiers::NONE);
    h.run_steps(2);
    assert_eq!(density(&h, id), 50);
    assert_eq!(undo_count(&h), rows + 1, "one history row");
    assert_eq!(density_field(&h).1, "50");
    // the tooltip text exists in both languages
    for lang in ["de", "en"] {
        assert!(fp_i18n::with_language(lang, || fp_i18n::t("pi-fill-dither-density-tip")).contains("Dither"), "{lang}");
    }
}

/// Rests the pointer on `at` for one second and tells whether a tooltip layer is open.
fn rest_and_tooltip_open(h: &mut H, at: Pos2) -> bool {
    ev(h, Event::PointerGone);
    h.run_steps(2);
    ev(h, Event::PointerMoved(at));
    for _ in 0..60 {
        h.step();
    }
    h.ctx.memory(|m| m.areas().visible_layer_ids().iter().any(|l| l.order == egui::Order::Tooltip))
}

/// FB-103: the density field's tooltip has a name line of its own ("Dither density") and the
/// explanation below; with View ▸ Tooltips off only the name stays, with "Hide tool names too"
/// nothing. The old one-line text stayed in full in names-only mode.
#[test]
fn dither_density_tooltip_keeps_only_its_name_with_tooltips_off() {
    fp_i18n::with_language("en", || {
        let (mut h, _) = harness_with_rect();
        let name = fp_i18n::t("pi-fill-dither-density-name");
        let explain = fp_i18n::t("pi-fill-dither-density-tip");
        assert_eq!(name, "Dither density");
        assert!(explain.starts_with("Dither density of the fill in % (0 = off"), "{explain}");
        assert!(explain.contains("Firepixel extension"), "{explain}");
        for lang in ["de", "en"] {
            let n = fp_i18n::with_language(lang, || fp_i18n::t("pi-fill-dither-density-name"));
            assert!(!n.is_empty() && n.chars().count() <= 20, "{lang}: short name {n:?}");
        }
        let full = fp_ui::tips::named(&name, &explain);
        let (f, _, _) = density_field(&h);
        let at = Pos2::new(f.left() - 4.0, f.center().y);
        assert!(rest_and_tooltip_open(&mut h, at), "tooltip on");
        assert!(h.query_by_label(&full).is_some(), "full tooltip with the switch on");
        // View > Tooltips off: only the name line
        h.state_mut().state.run("view.tooltips", json!({}));
        h.run_steps(1);
        assert!(rest_and_tooltip_open(&mut h, at), "name tooltip with the switch off");
        assert!(h.query_by_label(&full).is_none(), "no explanation with the switch off");
        assert!(h.query_by_label(&name).is_some(), "the name line stays");
        // and "hide tool names too": nothing
        h.state_mut()
            .state
            .session
            .prefs
            .set(fp_ui::tips::HIDE_NAMES_PREF, fp_engine::prefs::PrefValue::Bool(true));
        h.run_steps(1);
        assert!(!rest_and_tooltip_open(&mut h, at), "no tooltip with names hidden too");
        h.state_mut().state.run("view.tooltips", json!({}));
        h.run_steps(1);
    });
}

/// FB-109: 0 / 50 / 100 typed into the field give a plain, a checkerboard and an empty solid fill
/// on the canvas render (monotonic share of empty pixels).
#[test]
fn dither_density_0_50_100_change_the_solid_fill() {
    let (mut h, id) = harness_with_rect();
    let mut shares = Vec::new();
    for v in ["0", "50", "100"] {
        let (f, _, enabled) = density_field(&h);
        assert!(enabled, "the field is enabled for a solid fill");
        click(&mut h, f.center());
        key(&mut h, Key::End, Modifiers::NONE);
        key(&mut h, Key::Home, Modifiers::SHIFT);
        for c in v.chars() {
            ev(&mut h, Event::Text(c.to_string()));
        }
        key(&mut h, Key::Enter, Modifiers::NONE);
        h.run_steps(2);
        assert_eq!(density(&h, id).to_string(), v);
        let d = st(&h).session.active().expect("doc");
        let img = fp_render::Renderer::new(fp_render::RenderOptions::default()).render_page(&d.doc, d.page, 0).image;
        // inside the rectangle 40..300 × 40..200: share of pixels showing the fill colour
        let (mut on, mut n) = (0usize, 0usize);
        for y in 60..180 {
            for x in 60..280 {
                n += 1;
                let p = img.pixel(x, y);
                on += ((p.r, p.g, p.b) == (0x33, 0x66, 0x99)) as usize;
            }
        }
        shares.push(on as f64 / n as f64);
    }
    assert_eq!(shares[0], 1.0, "0 = off: {shares:?}");
    assert!((shares[1] - 0.5).abs() <= 0.02, "50 = checkerboard: {shares:?}");
    assert_eq!(shares[2], 0.0, "100 = empty: {shares:?}");
}

fn fx_index(label: &str) -> usize {
    effects::FX.iter().position(|f| f.menu_text() == label).unwrap_or_else(|| panic!("{label}"))
}

#[test]
fn screentone_menu_entry_and_dialog() {
    for (lang, name) in [("de", "Rasterfolie"), ("en", "Screentone")] {
        assert_eq!(fp_i18n::with_language(lang, || fp_i18n::t("effect-screentone")), name);
    }
    let (mut h, id) = harness_with_rect();
    let i = fx_index(&fp_i18n::t("effect-screentone"));
    assert_eq!(effects::FX[i].group, "effect-group-other");
    let rows = undo_count(&h);
    effects::begin_add(&mut h.state_mut().state, i);
    h.run_steps(2);
    {
        let f = h.state_mut().state.dialogs.form.as_mut().expect("dialog open");
        f.field_mut("pattern").expect("pattern").value = json!(2);
        f.field_mut("size").expect("size").value = json!(12.0);
        f.field_mut("keep_color").expect("keep colour").value = json!(true);
    }
    h.run_steps(2);
    let b: Vec<_> = h.get_all(By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
    assert!(st(&h).dialogs.form.is_none(), "dialog closed");
    assert_eq!(undo_count(&h), rows + 1, "one history row");
    let e = element(&h, id);
    let fx = e.common.effects.clone().expect("effects").effects;
    assert_eq!(fx.len(), 1);
    assert_eq!(fx[0].kind(), fp_model::EffectKind::Screentone);
    assert_eq!(fx[0].get("screentone_type"), Some("2"));
    assert_eq!(fx[0].get("screentone_size"), Some("12"));
    assert_eq!(fx[0].get("screentone_angle"), Some("45"));
    assert_eq!(fx[0].get("screentone_keep_color"), Some("true"));
    assert_eq!(fx[0].get("screentone_from_luminance"), Some("true"));
    // editable: the dialog of the existing entry shows its values and changes the same entry
    assert!(effects::editable(&fx[0]));
    assert_eq!(effects::effect_label(&fx[0]), fp_i18n::t("effect-screentone"));
    effects::begin_edit(&mut h.state_mut().state, 0, &fx[0]);
    h.run_steps(2);
    {
        let f = h.state_mut().state.dialogs.form.as_mut().expect("edit dialog");
        assert_eq!(f.get("pattern"), json!(2));
        assert_eq!(f.get("size").as_f64(), Some(12.0));
        f.field_mut("angle").expect("angle").value = json!(0.0);
    }
    h.run_steps(2);
    let b: Vec<_> = h.get_all(By::new().label("OK")).collect();
    b.last().expect("OK").click();
    drop(b);
    h.run_steps(3);
    let fx = element(&h, id).common.effects.clone().expect("effects").effects;
    assert_eq!(fx.len(), 1, "same entry edited");
    assert_eq!(fx[0].get("screentone_angle"), Some("0"));
    assert_eq!(fx[0].get("screentone_type"), Some("2"));
}
