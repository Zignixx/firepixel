//! T-4: target rules from the measurement session `re\29b` (M5): inspector values of a masked
//! image (§2, BA-006), trimming rules for menu filters, live filters and copies (§4, BA-009/013),
//! "Open as animation" flattens every state (§5, BA-025).

use crate::common;
use common::*;
use fp_engine::{Session, ToolEvent, ToolId};
use fp_geom::Rect;
use fp_model::ElementKind;
use serde_json::json;
use std::path::PathBuf;

fn dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("fp-t4-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// `m5_rand_fp.fw.png`: bitmap 0,0–400,300 (stored untrimmed), opaque pixels 100,50–200,150.
fn rand_bitmap(name: &str) -> (Session, fp_model::ElementId) {
    let (w, h) = (400usize, 300usize);
    let px: Vec<u32> = (0..w * h).map(|i| if (100..200).contains(&(i % w)) && (50..150).contains(&(i / w)) { 0xffff_0000 } else { 0 }).collect();
    let bytes = fp_export::png_out::write_png_truecolor(&px, w, h, true, &fp_export::png_out::PngOptions::default()).expect("PNG");
    let r = fp_io::png_in::decode_png(&bytes).expect("PNG lesen");
    let doc = fp_io::build::document_from_raster(r, fp_io::FileFormat::Png);
    let fw = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("FW-PNG");
    let p = dir().join(format!("{name}.fw.png"));
    std::fs::write(&p, fw).expect("schreiben");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": p.to_string_lossy()}));
    let id = top_elements(&s)[0].id;
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
    select(&mut s, &[id]);
    (s, id)
}

fn marquee(s: &mut Session, l: f32, t: f32, r: f32, b: f32) {
    s.set_tool(ToolId::Marquee).expect("Werkzeug");
    events(s, &[ToolEvent::down(l, t), ToolEvent::drag((l + r) / 2.0, (t + b) / 2.0), ToolEvent::drag(r, b), ToolEvent::up(r, b)]);
    assert!(s.active().expect("d").selection.has_pixels(), "Auswahlrahmen");
}

// ------------------------------------------------------------------ §4 trimming

/// A0: menu filter without pixel selection trims; A1d: with a pixel selection over the whole
/// canvas the object bounds stay.
#[test]
fn menu_filter_trims_only_without_pixel_selection() {
    let (mut s, id) = rand_bitmap("t4_a0");
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4}));
    let b = bounds(&s, id);
    assert!(b.left > 80.0 && b.top > 30.0 && b.right < 220.0 && b.bottom < 170.0, "A0 beschnitten: {b:?}");

    let (mut s, id) = rand_bitmap("t4_a1d");
    marquee(&mut s, 0.0, 0.0, 400.0, 300.0);
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "A1d: Pixelauswahl über alles beschneidet nicht");
}

/// B: a live filter never trims.
#[test]
fn live_filter_never_trims() {
    let (mut s, id) = rand_bitmap("t4_b");
    ex(&mut s, "filter.blur.gaussian", json!({"radius": 4, "live": true}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0));
}

/// C1: copying a group trims the bitmap inside the copy, the group bounds follow; the original
/// stays untrimmed.
#[test]
fn copy_trims_bitmaps_inside_groups() {
    let (mut s, id) = rand_bitmap("t4_c1");
    let r = rect(&mut s, 150.0, 100.0, 350.0, 250.0);
    select(&mut s, &[id, r]);
    ex(&mut s, "modify.group", json!({}));
    ex(&mut s, "edit.copy", json!({}));
    assert_eq!(bounds(&s, id), Rect::new(0.0, 0.0, 400.0, 300.0), "Original bleibt");
    let copy = s.clipboard.elements[0].clone();
    let ElementKind::Group(g) = &copy.kind else { panic!("Gruppe erwartet") };
    let d = s.active().expect("d");
    let bm = g.children.iter().find(|c| matches!(c.kind, ElementKind::Bitmap(_))).expect("Bild");
    assert_eq!(fp_engine::docops::element_bounds(&d.doc, bm), Some(Rect::new(100.0, 50.0, 200.0, 150.0)));
    assert_eq!(fp_engine::docops::element_bounds(&d.doc, &copy), Some(Rect::new(100.0, 50.0, 350.0, 250.0)), "Gruppengrenzen neu");
}

// ------------------------------------------------------------------ §2 masked image

/// A masked single image: opacity/blend go to the image, B/H/X/Y are the image bounds.
#[test]
fn masked_image_values_belong_to_the_image() {
    let mut s = doc_session(500.0, 400.0);
    let b = bitmap(&mut s, 300, 50, 150, 150, [200, 30, 30, 255]);
    select(&mut s, &[b]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 70}));
    let m = rect(&mut s, 315.0, 65.0, 435.0, 185.0);
    select(&mut s, &[b, m]);
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    let g = top_elements(&s)[0].clone();
    let (child, _) = g.masked_single().expect("maskiertes Einzelbild");
    assert_eq!(child.common.opacity.to_percent().round(), 70.0);
    select(&mut s, &[g.id]);
    ex(&mut s, "element.setOpacity", json!({"opacity": 40}));
    ex(&mut s, "element.setBlendMode", json!({"mode": "multiply"}));
    let g = el(&s, g.id);
    let (child, _) = g.masked_single().expect("bleibt Einzelbild");
    assert_eq!(child.common.opacity.to_percent().round(), 40.0, "Deckkraft am Bild");
    assert_eq!(Some(child.common.blend), fp_color::BlendMode::from_name("multiply"));
    assert_eq!(g.common.opacity.to_percent().round(), 100.0, "Gruppe unverändert");
    let d = s.active().expect("d");
    assert_eq!(fp_engine::docops::ids_info_bounds(&d.doc, d.page, &[g.id]), Some(Rect::new(300.0, 50.0, 450.0, 200.0)), "Bildgrenzen");
}

// ------------------------------------------------------------------ §5 open as animation

fn save(s: &mut Session, name: &str) -> PathBuf {
    let p = dir().join(name);
    ex(s, "file.saveAs", json!({"path": p.to_string_lossy()}));
    p
}

/// Every state of every file becomes one flat image at 0,0 on the single layer, canvas = union,
/// background transparent, no symbols.
#[test]
fn open_as_animation_flattens_each_state() {
    let mut a = doc_session(200.0, 100.0);
    rect(&mut a, 10.0, 10.0, 60.0, 60.0);
    let r = rect(&mut a, 100.0, 20.0, 150.0, 70.0);
    select(&mut a, &[r]);
    ex(&mut a, "modify.symbol.convertToSymbol", json!({"name": "SymA"}));
    let pa = save(&mut a, "t4_anim_a.fw.png");
    let mut c = doc_session(150.0, 150.0);
    ex(&mut c, "panel.states.add", json!({"count": 1, "where": "end"}));
    rect(&mut c, 5.0, 5.0, 45.0, 45.0);
    ex(&mut c, "state.setCurrent", json!({"index": 1}));
    rect(&mut c, 80.0, 5.0, 140.0, 40.0);
    let pc = save(&mut c, "t4_anim_c.fw.png");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": pa.to_string_lossy(), "frames": [pc.to_string_lossy()]}));
    let d = s.active().expect("Dokument");
    let page = &d.doc.pages[d.page];
    assert_eq!((page.canvas.width, page.canvas.height), (200.0, 150.0), "Vereinigung");
    assert_eq!(page.root.background.0 >> 24, 0, "durchsichtig");
    assert!(d.doc.symbols.is_empty(), "keine Symbole");
    let layers = fp_engine::docops::own_layers(&d.doc, d.page);
    let normal: Vec<_> = layers.iter().filter(|(_, l)| l.kind == fp_model::layer::LayerKind::Normal).collect();
    assert_eq!(normal.len(), 1, "nur Ebene 1");
    assert_eq!(d.frame_count(), 3);
    let sizes: Vec<Rect> = (0..3)
        .map(|f| {
            let cel = normal[0].1.cel(f).expect("Cel");
            assert_eq!(cel.elements.len(), 1, "ein Bild je Status");
            assert!(matches!(cel.elements[0].kind, ElementKind::Bitmap(_)));
            fp_engine::docops::element_bounds(&d.doc, &cel.elements[0]).expect("Grenzen")
        })
        .collect();
    assert_eq!(sizes, vec![Rect::new(0.0, 0.0, 200.0, 100.0), Rect::new(0.0, 0.0, 150.0, 150.0), Rect::new(0.0, 0.0, 150.0, 150.0)]);
}

/// M6 (re 33 §7): die neuen Status heißen „Status n“ und haben die Verzögerung 7 – die Vorgaben von
/// `insert_frames` (keine eigenen Statuseinträge in der Exporteinstellung).
#[test]
fn open_as_animation_states_are_named_status_n_with_delay_7() {
    let mut a = doc_session(40.0, 30.0);
    rect(&mut a, 5.0, 5.0, 20.0, 20.0);
    let pa = save(&mut a, "s7_anim_a.fw.png");
    let mut b = doc_session(40.0, 30.0);
    rect(&mut b, 10.0, 10.0, 30.0, 25.0);
    let pb = save(&mut b, "s7_anim_b.fw.png");
    let mut s = doc_session(10.0, 10.0);
    ex(&mut s, "file.open", json!({"path": pa.to_string_lossy(), "frames": [pb.to_string_lossy()]}));
    let d = s.active().expect("Dokument");
    assert_eq!(d.frame_count(), 2);
    let set = fp_export::doc_export::page_settings(&d.doc, d.page);
    for i in 0..2 {
        let f = set.anim.frames.get(i).cloned().unwrap_or_default();
        assert_eq!(f.delay, 7, "Verzögerung 7 (Status {})", i + 1);
        assert!(f.name.as_deref().unwrap_or("").is_empty(), "kein eigener Name");
    }
}

// ------------------------------------------------------------------ §1 pointer marquee, mask group (K-4)

/// re 29b §1 (a): the pointer marquee selects every object it touches or partly covers, in both
/// drag directions (CT-B observation; Firepixel already behaved so, K-4 adds the measured cases).
#[test]
fn k4_pointer_marquee_selects_partly_covered_objects_both_directions() {
    let mut s = doc_session(600.0, 450.0);
    let r1 = rect(&mut s, 50.0, 50.0, 150.0, 120.0);
    let r3 = rect(&mut s, 60.0, 200.0, 100.0, 300.0);
    let r2 = rect(&mut s, 300.0, 150.0, 400.0, 250.0);
    let drag = |s: &mut Session, a: (f32, f32), b: (f32, f32)| {
        select(s, &[]);
        s.set_tool(ToolId::Pointer).expect("Werkzeug");
        events(s, &[ToolEvent::down(a.0, a.1), ToolEvent::drag((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0), ToolEvent::drag(b.0, b.1), ToolEvent::up(b.0, b.1)]);
        let mut v = s.active().expect("d").selection.elements.clone();
        v.sort_by_key(|i| i.0);
        v
    };
    // 30,30 → 110,320: R1 partly (x 50–110 of 50–150), R3 completely
    let mut want = vec![r1, r3];
    want.sort_by_key(|i| i.0);
    assert_eq!(drag(&mut s, (30.0, 30.0), (110.0, 320.0)), want);
    // 480,380 → 350,100 (dragged to the top left): R2 partly (x 350–400)
    assert_eq!(drag(&mut s, (480.0, 380.0), (350.0, 100.0)), vec![r2]);
}

/// re 29b §1 (b): "group as mask" of three objects gives a group of the two lower objects with the
/// top one as mask; its bounds are those of the children (140 × 140 at 50,50), not of the mask.
#[test]
fn k4_mask_group_of_three_counts_two_children() {
    let mut s = doc_session(300.0, 250.0);
    let r4 = rect(&mut s, 50.0, 50.0, 150.0, 150.0);
    let e1 = ellipse(&mut s, 90.0, 90.0, 190.0, 190.0);
    let m1 = rect(&mut s, 70.0, 70.0, 170.0, 170.0);
    select(&mut s, &[r4, e1, m1]);
    ex(&mut s, "modify.mask.groupAsMask", json!({}));
    let top = top_elements(&s);
    assert_eq!(top.len(), 1, "eine Gruppe");
    let ElementKind::Group(g) = &top[0].kind else { panic!("Gruppe erwartet") };
    assert_eq!(g.children.len(), 2, "zwei maskierte Kinder");
    assert!(g.mask.is_some(), "oberstes Objekt ist die Maske");
    // inspector B/H/X/Y: the children, not the mask area 70–170 (`docops::info_bounds`)
    let d = s.active().expect("d");
    assert_eq!(fp_engine::docops::ids_info_bounds(&d.doc, d.page, &[top[0].id]), Some(Rect::new(50.0, 50.0, 190.0, 190.0)));
}
