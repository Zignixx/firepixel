//! TW-225 rest (RE spec 68 §8, parity run 44 `inspector-bitmap/08`): after Modify ▸ Mask ▸
//! Reveal All (and Hide All) on the bitmap B1 the new mask is edited like after a click on its
//! Layers thumbnail: the B1 row keeps its highlight, the mask thumbnail gets the bright green frame,
//! and the inspector shows "Bitmap mask" with Alpha channel / Grayscale (Grayscale on) and the
//! canvas-size bounds 800 × 600 at 0, 0. Undo returns to B1 without mask.

use egui::Color32;
use egui::epaint::Shape;
use egui_kittest::Harness;
use egui_kittest::kittest::{By, NodeT, Queryable};
use fp_ui::FirepixelApp;
use fp_ui::app_state::UiSettings;
use fp_ui::menu::{ItemAction, Node};
use serde_json::{Value, json};

type H = Harness<'static, FirepixelApp>;

const GREEN_FRAME: Color32 = Color32::from_rgb(0x66, 0xFF, 0x00);

fn ex(s: &mut fp_engine::Session, c: &str, p: Value) -> Value {
    s.execute(c, p).unwrap_or_else(|e| panic!("{c}: {e}")).value
}

/// 800 × 600 document with the bitmap "B1" (480 × 320 at 40, 40, like test file T02), selected.
fn harness() -> (H, u64) {
    fp_engine::shell::disable();
    fp_i18n::set_language("de");
    let mut s = fp_engine::Session::default();
    ex(&mut s, "file.new", json!({"width": 800, "height": 600, "background": "#ffffff"}));
    let img = fp_raster::RgbaImage::filled(fp_geom::IRect::from_xywh(40, 40, 480, 320), fp_color::Rgba8::new(40, 120, 200, 255)).expect("image");
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let b = ex(&mut s, "edit.paste", json!({}))["ids"][0].as_u64().expect("bitmap");
    ex(&mut s, "element.setName", json!({"ids": [b], "name": "B1"}));
    ex(&mut s, "select.set", json!({"ids": [b]}));
    let ctx = egui::Context::default();
    let mut app = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    app.state.os_clipboard = false;
    app.state.sync_render = true;
    let mut h = Harness::builder()
        .with_size(egui::vec2(1800.0, 1130.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(20)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), app);
    h.run_steps(5);
    for _ in 0..2 {
        if painted_texts(&h).iter().any(|t| t == "B1") {
            break;
        }
        if let Some(tab) = text_pos(&h, "Ebenen") {
            click(&mut h, tab);
        }
        h.run_steps(30);
    }
    assert!(painted_texts(&h).iter().any(|t| t == "B1"), "Layers panel shows B1");
    (h, b)
}

/// Centre of a painted text in the right part of the window (the panel dock), rightmost match.
fn text_pos(h: &H, label: &str) -> Option<egui::Pos2> {
    fn walk(s: &Shape, label: &str, out: &mut Vec<egui::Pos2>) {
        match s {
            Shape::Text(t) if t.galley.text() == label => out.push(egui::Rect::from_min_size(t.pos, t.galley.size()).center()),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, label, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, label, &mut v);
    }
    v.into_iter().filter(|p| p.x > 900.0).max_by(|a, b| a.x.total_cmp(&b.x))
}

fn click(h: &mut H, p: egui::Pos2) {
    use egui::{Event, Modifiers, PointerButton};
    for e in [
        Event::PointerMoved(p),
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE },
        Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE },
        Event::PointerMoved(egui::Pos2::new(5.0, 1120.0)),
    ] {
        h.event(e);
        h.run_steps(1);
    }
    h.run_steps(3);
}

fn painted_texts(h: &H) -> Vec<String> {
    fn walk(s: &Shape, out: &mut Vec<String>) {
        match s {
            Shape::Text(t) => out.push(t.galley.text().to_string()),
            Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut v = Vec::new();
    for cs in &h.output().shapes {
        walk(&cs.shape, &mut v);
    }
    v
}

fn count_frames(shape: &Shape, col: Color32, n: &mut usize) {
    match shape {
        Shape::Rect(r) if r.stroke.color == col && r.stroke.width > 0.0 => *n += 1,
        Shape::Vec(v) => v.iter().for_each(|s| count_frames(s, col, n)),
        _ => {}
    }
}

fn green_frames(h: &H) -> usize {
    let mut n = 0;
    for cs in &h.output().shapes {
        count_frames(&cs.shape, GREEN_FRAME, &mut n);
    }
    n
}

fn selection(h: &H) -> Vec<u64> {
    h.state().state.session.active().expect("d").selection.elements.iter().map(|e| e.0).collect()
}

fn layout(h: &H) -> Option<u16> {
    fp_ui::inspector::layouts::select_layout(&h.state().state.session).map(|l| l.number)
}

fn radio_on(h: &H, label: &str) -> Option<bool> {
    h.query_all(By::new().role(egui::accesskit::Role::RadioButton).label(label)).next().map(|n| {
        let a = n.accesskit_node();
        a.toggled() == Some(egui::accesskit::Toggled::True) || a.is_selected() == Some(true)
    })
}

fn find_item<'a>(nodes: &'a [Node], cmd: &str) -> Option<&'a Node> {
    for n in nodes {
        match n {
            Node::Item { action: ItemAction::Command(c), .. } if c == cmd => return Some(n),
            Node::Sub { children, .. } => {
                if let Some(x) = find_item(children, cmd) {
                    return Some(x);
                }
            }
            _ => {}
        }
    }
    None
}

/// Runs a Modify ▸ Mask entry the way the main menu does (the entry must be there and enabled).
fn menu_command(h: &mut H, cmd: &str) {
    let menus = fp_ui::menu::build_main_menu(&h.state().state);
    let enabled = menus.iter().find_map(|(_, nodes)| match find_item(nodes, cmd) {
        Some(Node::Item { enabled, .. }) => Some(*enabled),
        _ => None,
    });
    assert_eq!(enabled, Some(true), "{cmd} in the main menu and enabled");
    h.state_mut().state.run(cmd, Value::Object(Default::default()));
    h.run_steps(10);
}

fn masked_group_and_mask(h: &H) -> (u64, u64) {
    let d = h.state().state.session.active().expect("d");
    let m = d.selection.elements[0];
    let owner = fp_engine::docops::mask_owner(&d.doc, d.page, m).expect("a mask is selected");
    (owner.id.0, m.0)
}

#[test]
fn reveal_all_edits_the_new_mask_in_layers_and_inspector() {
    for cmd in ["modify.mask.revealAll", "modify.mask.hideAll"] {
        let (mut h, b) = harness();
        assert_eq!(green_frames(&h), 0, "{cmd}: no mask yet");
        let rows = h.state().state.session.active().expect("d").history.undo_count();
        menu_command(&mut h, cmd);
        let (g, m) = masked_group_and_mask(&h);
        assert_eq!(selection(&h), vec![m], "{cmd}: the new mask is selected, not B1");
        assert!(h.state().state.session.active().expect("d").bitmap_mode, "{cmd}: bitmap editing");
        let d = h.state().state.session.active().expect("d");
        assert_eq!(d.history.undo_count(), rows + 1, "{cmd}: one History row");
        assert_eq!(d.history.undo_label(), Some("Maske hinzufügen"), "{cmd}");
        // Layers panel: the B1 row with the green frame on its mask thumbnail
        let texts = painted_texts(&h);
        assert!(texts.iter().any(|t| t == "B1"), "{cmd}: the B1 row is listed");
        assert!(green_frames(&h) >= 1, "{cmd}: green frame around the mask thumbnail");
        // inspector: bitmap mask, Alpha channel / Grayscale with Grayscale on, canvas bounds
        assert_eq!(layout(&h), Some(21), "{cmd}: bitmap mask layout");
        assert!(texts.iter().any(|t| *t == fp_i18n::t("pi-mask-head-bitmap")), "{cmd}: {texts:?}");
        assert_eq!(radio_on(&h, &fp_i18n::t("pi-mask-alpha")), Some(false), "{cmd}");
        assert_eq!(radio_on(&h, &fp_i18n::t("pi-mask-gray")), Some(true), "{cmd}");
        let d = h.state().state.session.active().expect("d");
        let mb = fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(m))
            .and_then(|e| fp_engine::docops::element_bounds(&d.doc, &e))
            .expect("mask bounds");
        assert_eq!((mb.left, mb.top, mb.width(), mb.height()), (0.0, 0.0, 800.0, 600.0), "{cmd}");
        // undo: B1 again, no mask, no green frame
        h.state_mut().state.run("edit.undo", json!({}));
        h.run_steps(10);
        let d = h.state().state.session.active().expect("d");
        assert!(fp_engine::docops::element_on_page(&d.doc, d.page, fp_model::ElementId(g)).is_none(), "{cmd}: undo removes the mask group");
        assert!(selection(&h).iter().all(|id| *id == b), "{cmd}: {:?}", selection(&h));
        assert_eq!(green_frames(&h), 0, "{cmd}: no green frame after undo");
    }
}
