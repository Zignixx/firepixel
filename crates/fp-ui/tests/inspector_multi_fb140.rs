//! FB-140: the Properties panel on a mixed multi-selection (Path + Bitmap, also right after
//! Ungroup of a mask) shows the multi-selection panel "n objects" (row 20 of the layout table,
//! 11 §2.3.2) instead of the document panel; mixed opacity and blend mode are blank but usable
//! (a value applies to all); the filter list shows only when all objects agree; a selected mask
//! element row of the Layers panel shows that element's own panel; the Layers opacity field
//! follows the Properties slider live and the other way round.

use egui_kittest::Harness;
use egui::{Event, Id, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::{By, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::inspector::layouts::select_layout;
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

const W: f32 = 1800.0;
const HH: f32 = 1130.0;

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

fn id(v: Value) -> u64 {
    v["id"].as_u64().expect("id")
}

fn harness(setup: impl FnOnce(&mut fp_engine::Session)) -> H {
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 500, "height": 300, "background": "#ffffff"}));
    setup(&mut s);
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.sync_render = true;
    app.state.os_clipboard = false;
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, HH))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(4);
    h
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

/// A bitmap of 60 × 40 px at (x, y), selected.
fn bitmap(s: &mut fp_engine::Session, x: i32, y: i32) -> u64 {
    let img = fp_raster::RgbaImage::filled(
        fp_geom::IRect::from_xywh(x, y, 60, 40),
        fp_color::Rgba8::new(200, 30, 30, 255),
    )
    .ok();
    s.clipboard.clear();
    s.clipboard.pixels = img;
    ex(s, "edit.paste", json!({}));
    s.active().expect("doc").selection.elements[0].0
}

fn rect(s: &mut fp_engine::Session, r: [f64; 4]) -> u64 {
    id(ex(s, "draw.rectangle", json!({"rect": r, "fillColor": "#3366cc", "strokeWidth": 0})))
}

fn layout(h: &H) -> Option<u16> {
    select_layout(&st(h).session).map(|l| l.number)
}

fn quads(h: &H) -> [u16; 6] {
    select_layout(&st(h).session).map(|l| l.quads).expect("layout")
}

fn shape_texts(shape: &egui::epaint::Shape, out: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Text(t) => out.push(t.galley.job.text.clone()),
        egui::epaint::Shape::Vec(v) => v.iter().for_each(|s| shape_texts(s, out)),
        _ => {}
    }
}

/// Texts painted in the Properties panel (bottom of the window) and its accessible labels.
fn texts(h: &H) -> Vec<String> {
    let mut out = Vec::new();
    for cs in &h.output().shapes {
        if cs.clip_rect.bottom() > HH - 170.0 && cs.clip_rect.top() > HH - 400.0 {
            shape_texts(&cs.shape, &mut out);
        }
    }
    out
}

fn inputs(h: &H) -> Vec<String> {
    h.get_all(By::new().role(egui::accesskit::Role::TextInput))
        .map(|n| n.value().unwrap_or_default())
        .collect()
}

/// Path + Bitmap (both selection orders) and Group + Bitmap: the multi-selection panel with the
/// path quadrants (row 20), header "2 Objekte", W/H/X/Y of the union; no document buttons.
#[test]
fn path_and_bitmap_show_the_multi_selection_panel() {
    for order in [false, true] {
        let mut ids = (0, 0);
        let mut h = harness(|s| {
            ids.0 = bitmap(s, 100, 100);
            ids.1 = rect(s, [40.0, 40.0, 140.0, 120.0]);
            let sel = if order { json!([ids.1, ids.0]) } else { json!([ids.0, ids.1]) };
            ex(s, "select.set", json!({ "ids": sel }));
        });
        h.run_steps(3);
        assert_eq!(layout(&h), Some(20), "mixed multi-selection with paths");
        assert_eq!(quads(&h)[0], 14102, "header n objects");
        assert_eq!(quads(&h)[2], 14129, "fill and stroke of the path layout");
        let t = texts(&h);
        assert!(t.iter().any(|x| x == "2 Objekte"), "{t:?}");
        assert!(!t.iter().any(|x| x.starts_with("Leinwandgröße")), "no document panel: {t:?}");
        let v = inputs(&h);
        // union 40..160 × 40..140
        for want in ["120", "100", "40"] {
            assert!(v.iter().any(|x| x == want), "W/H/X/Y {want} in {v:?}");
        }
    }
}

/// Bitmap + Slice (no object with fill and stroke): the multi-selection panel with opacity,
/// blend mode and filters only.
#[test]
fn bitmap_and_slice_show_the_multi_selection_panel() {
    let mut h = harness(|s| {
        let b = bitmap(s, 100, 100);
        let sl = id(ex(s, "web.addSlice", json!({"shape": "rect", "rect": [10, 10, 60, 60]})));
        ex(s, "select.set", json!({"ids": [b, sl]}));
    });
    h.run_steps(3);
    assert_eq!(layout(&h), Some(20));
    assert_eq!(quads(&h)[0], 14102);
    assert_eq!(quads(&h)[2], 14037, "opacity and filters only");
    assert!(texts(&h).iter().any(|x| x == "2 Objekte"));
}

/// Typed multi-selections keep their rows (two bitmaps = 27, two paths = 26 or 58).
#[test]
fn typed_multi_selections_keep_their_rows() {
    let h = harness(|s| {
        let a = bitmap(s, 10, 10);
        let b = bitmap(s, 100, 100);
        ex(s, "select.set", json!({"ids": [a, b]}));
    });
    assert_eq!(layout(&h), Some(27));
    let h = harness(|s| {
        let a = rect(s, [10.0, 10.0, 50.0, 50.0]);
        let b = rect(s, [100.0, 100.0, 150.0, 150.0]);
        ex(s, "select.set", json!({"ids": [a, b]}));
    });
    assert!(matches!(layout(&h), Some(58 | 26)), "{:?}", layout(&h));
    // one object of each kind alone: its own panel
    let h = harness(|s| {
        bitmap(s, 10, 10);
    });
    assert_eq!(layout(&h), Some(27));
}

/// Mixed opacity and blend mode: blank fields that still apply a value to all objects.
#[test]
fn mixed_opacity_and_blend_apply_to_all() {
    let mut ids = (0, 0);
    let mut h = harness(|s| {
        ids.0 = bitmap(s, 100, 100);
        ex(s, "element.setOpacity", json!({"opacity": 47}));
        ex(s, "element.setBlendMode", json!({"mode": "multiply"}));
        ids.1 = rect(s, [40.0, 40.0, 140.0, 120.0]);
        ex(s, "select.set", json!({"ids": [ids.0, ids.1]}));
    });
    h.run_steps(3);
    let shown = st(&h).session.active().and_then(|d| {
        fp_engine::docops::shown_opacity_blend(&d.doc, d.page, &d.selection.elements)
    });
    assert_eq!(shown.map(|v| (v.0.is_none(), v.1.is_none())), Some((true, true)), "both mixed");
    assert_eq!(layout(&h), Some(20));
    // the inspector's field is usable: the value goes to every selected object
    h.state_mut().state.run("element.setOpacity", json!({"opacity": 30}));
    h.state_mut().state.run("element.setBlendMode", json!({"mode": "screen"}));
    h.run_steps(3);
    let d = st(&h).session.active().expect("doc");
    for i in [ids.0, ids.1] {
        let e = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(i)).expect("el");
        let (op, bl) = e.placement();
        assert_eq!(op.to_percent().round(), 30.0);
        assert_eq!(bl.name(), Some("screen"));
    }
}

/// Filters: the list shows only when all selected objects carry the same filters.
#[test]
fn filter_list_only_when_all_agree() {
    let mut ids = (0, 0);
    let mut h = harness(|s| {
        ids.0 = bitmap(s, 100, 100);
        ids.1 = rect(s, [40.0, 40.0, 140.0, 120.0]);
        ex(s, "element.addEffect", json!({"ids": [ids.1], "effect": "gaussianBlur"}));
        ex(s, "select.set", json!({"ids": [ids.0, ids.1]}));
    });
    h.run_steps(3);
    assert!(fp_ui::inspector::obj_quads::effects_shown(st(&h)).is_none(), "different filters: empty list");
    h.state_mut().state.run("element.addEffect", json!({"ids": [ids.0], "effect": "gaussianBlur"}));
    h.run_steps(3);
    let e = fp_ui::inspector::obj_quads::effects_shown(st(&h)).expect("same filters: shown");
    assert_eq!(e.common.effects.as_ref().map(|l| l.effects.len()), Some(1));
}

/// How the test builds the mask of two paths over a picture.
#[derive(Clone, Copy, Debug)]
enum MaskWay {
    /// Modify ▸ Mask ▸ Group as Mask with a group of the two paths on top.
    GroupAsMask,
    /// Cut the bitmap, select the group of the two paths, Edit ▸ Paste Inside (the tester's way).
    PasteInside,
    /// Copy the two paths, select the bitmap, Edit ▸ Paste as Mask.
    PasteAsMask,
}

/// A mask element row (an object of a group used as mask) shows the object's own panel, not the
/// document panel; for every way of building the mask and a bitmap or path content.
#[test]
fn mask_element_row_shows_its_own_panel() {
    for way in [MaskWay::GroupAsMask, MaskWay::PasteInside, MaskWay::PasteAsMask] {
        for content_bitmap in [false, true] {
            let mut ids = (0, 0, 0);
            let mut h = harness(|s| {
                let y = if content_bitmap { bitmap(s, 40, 40) } else { rect(s, [40.0, 40.0, 140.0, 120.0]) };
                ids.1 = rect(s, [45.0, 45.0, 90.0, 90.0]);
                ids.2 = rect(s, [80.0, 60.0, 130.0, 110.0]);
                ex(s, "select.set", json!({"ids": [ids.1, ids.2]}));
                let out = match way {
                    MaskWay::GroupAsMask => {
                        let gm = id(ex(s, "modify.group", json!({})));
                        ex(s, "select.set", json!({"ids": [y, gm]}));
                        ex(s, "modify.mask.groupAsMask", json!({}))
                    }
                    MaskWay::PasteInside => {
                        let gm = id(ex(s, "modify.group", json!({})));
                        ex(s, "select.set", json!({"ids": [y]}));
                        ex(s, "edit.cut", json!({}));
                        ex(s, "select.set", json!({"ids": [gm]}));
                        ex(s, "edit.pasteInside", json!({}))
                    }
                    MaskWay::PasteAsMask => {
                        ex(s, "edit.cut", json!({}));
                        ex(s, "select.set", json!({"ids": [y]}));
                        ex(s, "edit.pasteAsMask", json!({}))
                    }
                };
                ids.0 = out["id"].as_u64().unwrap_or_else(|| {
                    // Paste as Mask selects the mask: its owner is the masked object
                    let d = s.active().expect("doc");
                    let sel = d.selection.elements[0];
                    fp_engine::docops::mask_owner(&d.doc, d.page, sel).map(|o| o.id.0).unwrap_or(sel.0)
                });
                ex(s, "select.none", json!({}));
            });
            h.run_steps(3);
            // the paths of the mask as the Layers panel lists them (the rows' element ids)
            let (doc, page) = st(&h).session.active().map(|d| (std::sync::Arc::clone(&d.doc), d.page)).expect("doc");
            let owner = fp_engine::docops::element_on_page(&doc, page, fp_model::ElementId(ids.0)).expect("owner");
            let members: Vec<u64> = match fp_engine::docops::mask_members(&owner) {
                Some(fp_engine::docops::MaskMembers::Group(cs)) => cs.iter().map(|c| c.id.0).collect(),
                Some(fp_engine::docops::MaskMembers::Single(x)) => vec![x.id.0],
                _ => Vec::new(),
            };
            assert!(!members.is_empty(), "{way:?}: mask rows");
            // the actions of a click on the first mask element row (Layers panel, `tree_click_actions`)
            h.state_mut().state.run("select.set", json!({"ids": [ids.0]}));
            h.state_mut().state.run("panel.layers.editMask", json!({}));
            h.state_mut().state.run(
                "tool.pick",
                json!({"command": "select.set", "params": {"ids": [members[0]], "mode": "replace"}}),
            );
            h.run_steps(4);
            let sel = st(&h).session.active().map(|d| d.selection.elements.clone()).unwrap_or_default();
            assert_eq!(sel, vec![fp_model::ElementId(members[0])], "{way:?} bitmap content {content_bitmap}");
            let d = st(&h).session.active().expect("doc");
            assert!(
                fp_engine::docops::element_on_page(&d.doc, d.page, sel[0]).is_some(),
                "{way:?}: the selected mask element is found on the page"
            );
            let l = layout(&h);
            assert!(
                matches!(l, Some(58 | 26 | 19)),
                "{way:?}: the path's own panel, got {l:?} (bitmap content {content_bitmap})"
            );
            let t = texts(&h);
            assert!(!t.iter().any(|x| x.starts_with("Leinwandgröße")), "{way:?}: no document panel: {t:?}");
        }
    }
}

fn ev(h: &mut H, e: Event) {
    h.event(e);
    h.run_steps(1);
}

fn press(h: &mut H, p: Pos2, down: bool) {
    ev(h, Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: down, modifiers: Modifiers::NONE });
}

/// Arrow rect of the opacity field `id` (the Layers tab is brought to the front if needed).
fn arrow(h: &mut H, id: &str) -> egui::Rect {
    let key = Id::new(id).with("arrow");
    for _ in 0..2 {
        if let Some(r) = h.ctx.read_response(key).map(|r| r.rect) {
            return r;
        }
        let mut all = Vec::new();
        for cs in &h.output().shapes {
            if let egui::epaint::Shape::Text(t) = &cs.shape {
                all.push((t.galley.job.text.clone(), t.pos));
            }
        }
        let tab = all.iter().filter(|(t, p)| t == "Ebenen" && p.x > W / 2.0).map(|(_, p)| *p).next().expect("Layers tab");
        let p = tab + egui::vec2(5.0, 5.0);
        ev(h, Event::PointerMoved(p));
        press(h, p, true);
        press(h, p, false);
        h.run_steps(10);
    }
    h.ctx.read_response(key).map(|r| r.rect).expect("opacity arrow drawn")
}

/// The Layers opacity field follows a drag of the Properties opacity slider live, and the
/// Properties field follows the Layers slider; both show the value after the release.
#[test]
fn layers_opacity_follows_the_properties_slider() {
    let mut h = harness(|s| {
        rect(s, [40.0, 40.0, 140.0, 120.0]);
    });
    h.run_steps(3);
    let count = |h: &H, v: &str| inputs(h).iter().filter(|x| *x == v).count();
    for (id, want) in [("pi-opacity", "90"), ("layers-opacity", "70")] {
        let a = arrow(&mut h, id).center();
        ev(&mut h, Event::PointerMoved(a));
        press(&mut h, a, true);
        for i in 1..=6 {
            ev(&mut h, Event::PointerMoved(Pos2::new(a.x, a.y + 4.0 * i as f32)));
        }
        // 2 px per per cent below the thumb of 100 % (the top end of the track)
        let start = h
            .query_all(By::new().role(egui::accesskit::Role::Slider))
            .next()
            .map(|n| n.rect().top() + fp_ui::inspector::fields::VTrack::PERCENT.pad)
            .expect("pop-up slider open");
        let to = Pos2::new(a.x, start + if want == "90" { 20.0 } else { 60.0 });
        for _ in 0..3 {
            ev(&mut h, Event::PointerMoved(to));
        }
        assert_eq!(count(&h, want), 2, "{id}: both fields show the dragged value: {:?}", inputs(&h));
        press(&mut h, to, false);
        h.run_steps(4);
        assert_eq!(count(&h, want), 2, "{id}: both show it after the release: {:?}", inputs(&h));
    }
}
