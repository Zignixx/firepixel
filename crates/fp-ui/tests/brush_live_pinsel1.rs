//! PINSEL-1 (Nutzerbefund N31) in der Oberfläche: Ziehen mit dem Pinsel auf der Leinwand über
//! viele Bilder (echte Zeigerereignisse) zeigt den echten Strich live (Kacheln der Vorschau statt
//! Kreis-Umrissen), die Werte des Inspektors (Strichart, Größe, Kante, Deckkraft, Mischmodus)
//! wirken, Loslassen = ein Verlaufsschritt. „Pinselstrich bearbeiten…“ öffnet aus dem
//! Werkzeug-Inspektor auch mit ausgewählter Bitmap den Dialog mit den Registerkarten Optionen, Form
//! und Empfindlichkeit; OK wirkt auf den nächsten Strich, Abbrechen verwirft (AGENTEN_REGELN §4b).

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use fp_engine::tools::ToolId;
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::{ElementId, ElementKind};
use fp_ui::app_state::{AppState, UiSettings};
use fp_ui::FirepixelApp;
use serde_json::json;
use std::sync::Arc;

type H = Harness<'static, FirepixelApp>;

fn app(bitmap: bool) -> (FirepixelApp, Option<ElementId>) {
    let mut s = Session::default();
    s.execute("file.new", json!({"width": 600, "height": 400, "background": "#ffffff"})).expect("Dokument");
    let mut id = None;
    if bitmap {
        let img = fp_raster::RgbaImage::filled(IRect::new(50, 50, 550, 350), fp_color::Rgba8::new(100, 100, 100, 255)).expect("Bild");
        s.clipboard.clear();
        s.clipboard.pixels = Some(img);
        let v = s.execute("edit.paste", json!({})).expect("Bitmap");
        id = Some(ElementId(v.value["ids"][0].as_u64().expect("id")));
    } else {
        s.execute("select.none", json!({})).expect("nichts");
    }
    s.set_colors(Some(fp_color::Argb(0xff00_0000)), None);
    let ctx = egui::Context::default();
    let mut a = FirepixelApp::with_session(&ctx, s, UiSettings::default(), false);
    a.state.sync_render = true;
    (a, id)
}

fn harness(a: FirepixelApp) -> H {
    Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(16).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a)
}

fn st(h: &H) -> &AppState {
    &h.state().state
}

fn doc(h: &H) -> Arc<fp_model::Document> {
    Arc::clone(&st(h).session.active().expect("Dokument").doc)
}

fn steps(h: &H) -> usize {
    st(h).session.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

fn image(h: &H, id: ElementId) -> fp_raster::RgbaImage {
    let d = st(h).session.active().expect("Dokument");
    match &fp_engine::docops::element_on_page(&d.doc, d.page, id).expect("Element").kind {
        ElementKind::Bitmap(b) => fp_engine::docops::bitmap_image(b).expect("Bild"),
        _ => panic!("keine Bitmap"),
    }
}

fn top_id(h: &H) -> ElementId {
    let d = st(h).session.active().expect("Dokument");
    let layers = fp_engine::docops::own_layers(&d.doc, d.page);
    layers.into_iter().rev().find_map(|(_, l)| l.cels[0].elements.first().map(|e| e.id)).expect("Element")
}

/// Dokumentpunkt → Bildschirm (Seitenlage aus dem letzten Bild).
fn screen(h: &H, x: f32, y: f32) -> Pos2 {
    let s = st(h);
    let d = s.session.active().expect("Dokument");
    let pr = s.canvases.get(&d.id).and_then(|c| c.page_screen).expect("Seitenlage");
    let r = fp_render::Renderer::page_rect(&d.doc, d.page).expect("Seite");
    let (w, ht) = ((r.right - r.left) as f32, (r.bottom - r.top) as f32);
    egui::pos2(pr.left() + (x - r.left as f32) / w * pr.width(), pr.top() + (y - r.top as f32) / ht * pr.height())
}

fn preview_tiles(h: &H) -> usize {
    let id = st(h).session.active().expect("Dokument").id;
    fp_ui::canvas::paint_preview::cached_tiles(&h.ctx, id)
}

/// Zieht mit echten Zeigerereignissen über viele Bilder (je Bild ein Schritt von ≈ 4 px) und
/// prüft dabei: Dokument und Verlauf unverändert, Vorschau-Kacheln vorhanden.
fn mouse_drag(h: &mut H, pts: &[(f32, f32)]) {
    let (d0, n0) = (doc(h), steps(h));
    let p0 = screen(h, pts[0].0, pts[0].1);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    let mut frames = 0;
    let mut last = p0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let k = (len / 4.0).ceil().max(1.0) as usize;
        for i in 1..=k {
            let t = i as f32 / k as f32;
            last = screen(h, a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
            h.event(Event::PointerMoved(last));
            h.run_steps(1);
            frames += 1;
            assert!(Arc::ptr_eq(&d0, &doc(h)), "während des Ziehens bleibt das Dokument unverändert (Bild {frames})");
            assert_eq!(steps(h), n0, "kein Verlaufsschritt während des Ziehens");
            if frames > 2 {
                assert!(preview_tiles(h) > 0, "der Strich ist während des Ziehens sichtbar (Bild {frames})");
            }
        }
    }
    assert!(frames >= 30, "realistisch viele Bilder ({frames})");
    h.event(Event::PointerButton { pos: last, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(2);
}

fn preset_index(name: &str) -> usize {
    fp_engine::commands::panel::find_preset(name).and_then(|f| fp_engine::commands::panel::BRUSH_PRESETS.iter().position(|b| b == f)).expect("Vorgabe")
}

fn blend_index(m: i32) -> usize {
    fp_ui::inspector::obj_quads::BLEND_ITEMS.iter().position(|b| b.1 == m).expect("Mischmodus")
}

fn set_tool(h: &mut H, t: ToolId) {
    h.state_mut().state.session.set_tool(t).expect("Werkzeug");
    h.run_steps(2);
}

fn click_label(h: &mut H, label: &str) {
    h.run_steps(1);
    let b: Vec<_> = h.get_all(egui_kittest::kittest::By::new().label(label)).collect();
    b.last().unwrap_or_else(|| panic!("„{label}“ nicht gefunden")).click();
    drop(b);
    h.run_steps(3);
}

/// Klick mit der Maus auf „Pinselstrich bearbeiten“ im Werkzeug-Inspektor (Lage aus dem letzten Bild).
fn click_inspector_edit(h: &mut H) {
    h.run_steps(1);
    let id = egui::Id::new(egui::Id::new(("pi-sedit", true)).with("edit"));
    let r = h.ctx.read_response(id).expect("Knopf „Pinselstrich bearbeiten“ im Inspektor").rect;
    let p = r.center();
    h.event(Event::PointerMoved(p));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
}

#[test]
fn canvas_drag_paints_live_with_inspector_values() {    let (a, _) = app(false);
    let mut h = harness(a);
    set_tool(&mut h, ToolId::Brush);
    // Nutzerbild 8: Strich 35, Kohle „Weich“, Kante 37, Deckkraft 21, „Durchschnitt“
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::edit_tool_attrs(&ctx, |t| {
        t.stroke_preset = Some(preset_index("Einfach/Weich abger."));
        t.stroke_width = 35;
        t.softness = 37;
        t.blend = blend_index(30);
    });
    h.state_mut().state.session.tools.options.opacity = 21;
    h.run_steps(2);
    assert_eq!(st(&h).session.tools.options.paint_blend, 30, "Mischmodus aus dem Inspektor");
    let n0 = steps(&h);
    mouse_drag(&mut h, &[(80.0, 100.0), (300.0, 160.0), (500.0, 120.0)]);
    assert_eq!(steps(&h), n0 + 2, "Loslassen = ein Verlaufsschritt (nach \"Edit Bitmap\", RE spec 71)");
    let id = top_id(&h);
    let img = image(&h, id);
    let max = img.data().iter().map(|p| p[3]).max().unwrap_or(0);
    assert!((52..=56).contains(&max), "Deckkraft 21 % ({max})");
    let partial = img.data().iter().filter(|p| p[3] > 0 && p[3] < max).count();
    assert!(partial > 500, "weiche Kante (Kante 37): viele halbdeckende Pixel ({partial})");
    let covered = (0..400).filter(|y| img.get(190, *y).is_some_and(|p| p[3] > max / 2)).count();
    assert!((25..=42).contains(&covered), "Strich 35 breit ({covered})");
    // nach dem Loslassen: Vorschau weg, sobald das Bild mit dem Strich da ist
    h.run_steps(3);
    assert_eq!(preview_tiles(&h), 0, "Vorschau freigegeben");
    // Rückgängig/Wiederholen
    h.state_mut().state.session.undo().expect("Rückgängig");
    assert_eq!(steps(&h), n0 + 1);
    h.state_mut().state.session.redo().expect("Wiederholen");
    assert_eq!(steps(&h), n0 + 2);
}

#[test]
fn keep_transparency_and_blend_reach_the_bitmap() {
    let (a, b) = app(true);
    let b = b.expect("Bitmap");
    let mut h = harness(a);
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::edit_tool_attrs(&ctx, |t| {
        t.stroke_preset = Some(preset_index("Einfach/Präzise Linie"));
        t.stroke_width = 9;
        t.softness = 0;
        t.blend = blend_index(30);
        t.keep_transparency = true;
    });
    h.state_mut().state.session.set_colors(Some(fp_color::Argb(0xffff_ffff)), None);
    h.run_steps(2);
    assert!(st(&h).session.tools.options.keep_transparency);
    // über den Rand der Bitmap hinaus: außen bleibt es transparent, innen Durchschnitt
    mouse_drag(&mut h, &[(300.0, 200.0), (580.0, 200.0)]);
    let img = image(&h, b);
    assert_eq!(img.get(400, 200), Some([177, 177, 177, 255]), "Durchschnitt Weiß/Grau 100");
    assert_eq!(img.bounds(), IRect::new(50, 50, 550, 350), "Transparenz beibehalten: Bitmap wächst nicht");
}

#[test]
fn edit_stroke_dialog_from_tool_inspector_with_bitmap_selected() {
    let (a, b) = app(true);
    let b = b.expect("Bitmap");
    let mut h = harness(a);
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::edit_tool_attrs(&ctx, |t| {
        t.stroke_preset = Some(preset_index("Einfach/Präzise Linie"));
        t.stroke_width = 6;
        t.softness = 0;
    });
    h.run_steps(2);
    let (d0, n0) = (doc(&h), steps(&h));
    // N31: „Pinselstrich bearbeiten“ war mit ausgewählter Bitmap ohne Funktion
    click_inspector_edit(&mut h);
    let dlg = st(&h).dialogs.brush_edit.as_ref().expect("Dialog offen (kein Hinweis)");
    assert!(dlg.tool, "wirkt auf die Werkzeugvorgabe");
    assert!(st(&h).dialogs.message.is_none());
    // Registerkarten
    click_label(&mut h, &fp_i18n::t("dlg-brushedit-tab-shape"));
    assert_eq!(st(&h).dialogs.brush_edit.as_ref().expect("offen").tab, 1);
    // 50× ändern, dann Abbrechen: nichts übernommen
    for k in 0..50 {
        h.state_mut().state.dialogs.brush_edit.as_mut().expect("offen").brush.diameter = 2 + k % 20;
        h.run_steps(1);
    }
    click_label(&mut h, &fp_i18n::t("btn-cancel"));
    assert!(st(&h).dialogs.brush_edit.is_none());
    assert!(st(&h).session.tools.options.new_brush.is_none(), "Abbrechen verwirft");
    // erneut: Größe 30, eckig → OK
    click_inspector_edit(&mut h);
    click_label(&mut h, &fp_i18n::t("dlg-brushedit-tab-shape"));
    for k in 0..50 {
        let d = h.state_mut().state.dialogs.brush_edit.as_mut().expect("offen");
        d.brush.diameter = 2 + k % 20;
        h.run_steps(1);
    }
    {
        let d = h.state_mut().state.dialogs.brush_edit.as_mut().expect("offen");
        d.brush.diameter = 30;
        d.brush.shape = 0;
    }
    h.run_steps(1);
    click_label(&mut h, &fp_i18n::t("btn-ok"));
    assert!(st(&h).dialogs.brush_edit.is_none());
    let tb = fp_engine::commands::panel::tool_brush(&st(&h).session.tools.options);
    assert_eq!((tb.diameter, tb.shape), (30, 0), "Werte gemerkt");
    assert!(Arc::ptr_eq(&d0, &doc(&h)), "Werkzeugvorgabe ist kein Dokumentschritt");
    assert_eq!(steps(&h), n0);
    // der nächste Strich malt mit Größe 30 in die ausgewählte Bitmap
    mouse_drag(&mut h, &[(150.0, 200.0), (450.0, 200.0)]);
    assert_eq!(steps(&h), n0 + 1);
    let img = image(&h, b);
    let covered = (0..400).filter(|y| img.get(300, *y).is_some_and(|p| p == [0, 0, 0, 255])).count();
    assert!((28..=32).contains(&covered), "Spitzengröße 30 ({covered})");
    // wieder öffnen zeigt die gemerkten Werte
    click_inspector_edit(&mut h);
    assert_eq!(st(&h).dialogs.brush_edit.as_ref().expect("offen").brush.diameter, 30);
    click_label(&mut h, &fp_i18n::t("btn-cancel"));
}

#[test]
fn sensitivity_tab_and_natural_engine() {
    let (a, _) = app(false);
    let mut h = harness(a);
    set_tool(&mut h, ToolId::Brush);
    h.state_mut().state.dialogs.open_stroke_options_for_tool();
    h.run_steps(2);
    let kind = st(&h).dialogs.brush_edit.as_ref().expect("offen").brush.brush_type;
    click_label(&mut h, &fp_i18n::t("dlg-brushedit-tab-sensitivity"));
    assert_eq!(st(&h).dialogs.brush_edit.as_ref().expect("offen").tab, 2);
    {
        let d = h.state_mut().state.dialogs.brush_edit.as_mut().expect("offen");
        d.property = 3; // Streuung
        d.set_sensitivity(b'R', b'R', 50.0);
        d.brush.diameter = 8;
    }
    h.run_steps(1);
    click_label(&mut h, &fp_i18n::t("btn-ok"));
    let tb = fp_engine::commands::panel::tool_brush(&st(&h).session.tools.options);
    assert_eq!(tb.sensitivity(b'R', b'R'), 500);
    // re 27 P1: der Dialog schaltet die Engine nie um, auch nicht durch Empfindlichkeiten
    assert_eq!(tb.brush_type, kind, "Pinselart bleibt");
    if kind == 0 {
        // natürlicher Pinsel: gestreuter Strich, Pixel weit neben der Linie
        mouse_drag(&mut h, &[(100.0, 200.0), (500.0, 200.0)]);
        let id = top_id(&h);
        let bnd = image(&h, id).alpha_bounds();
        assert!(bnd.height() > 12, "Streuung verbreitert den Strich ({bnd:?})");
    }
}

/// Bilder für die Sichtprüfung (Dialog je Registerkarte, Strich mitten im Ziehen), nur mit
/// Grafikadapter: `FP_PINSEL1_SHOTS=<Ordner>` und `--ignored`.
#[test]
#[ignore = "Bilder: $env:FP_PINSEL1_SHOTS=<Ordner>; cargo test -p fp-ui --test all brush_live_pinsel1::shots -- --ignored"]
fn shots() {
    let Ok(dir) = std::env::var("FP_PINSEL1_SHOTS") else { return };
    let dir = std::path::PathBuf::from(dir);
    let (a, _) = app(true);
    let mut h = Harness::builder().renderer(crate::gpu_common::renderer()).with_size(egui::vec2(1400.0, 900.0)).with_pixels_per_point(1.0).with_max_steps(8).renderer(crate::gpu_common::renderer()).build_ui_state(|ui, app: &mut FirepixelApp| app.root_ui(ui), a);
    h.run_steps(3);
    set_tool(&mut h, ToolId::Brush);
    let ctx = h.ctx.clone();
    fp_ui::inspector::obj_quads::edit_tool_attrs(&ctx, |t| {
        t.stroke_preset = Some(preset_index("Kohlestift/Weich"));
        t.stroke_width = 35;
        t.softness = 37;
    });
    h.run_steps(2);
    let save = |h: &mut H, name: &str| {
        h.run_steps(2);
        if let Ok(i) = h.render() {
            let _ = i.save(dir.join(format!("pinsel1_{name}.png")));
        }
    };
    // mitten im Ziehen
    let p0 = screen(&h, 100.0, 120.0);
    h.event(Event::PointerMoved(p0));
    h.run_steps(1);
    h.event(Event::PointerButton { pos: p0, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.run_steps(1);
    for i in 1..=60 {
        let p = screen(&h, 100.0 + i as f32 * 6.0, 120.0 + (i as f32 / 8.0).sin() * 60.0);
        h.event(Event::PointerMoved(p));
        h.run_steps(1);
    }
    save(&mut h, "mid_drag");
    let p = screen(&h, 460.0, 120.0);
    h.event(Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    h.run_steps(3);
    save(&mut h, "after_release");
    click_inspector_edit(&mut h);
    save(&mut h, "dialog_options");
    click_label(&mut h, &fp_i18n::t("dlg-brushedit-tab-shape"));
    save(&mut h, "dialog_shape");
    click_label(&mut h, &fp_i18n::t("dlg-brushedit-tab-sensitivity"));
    save(&mut h, "dialog_sensitivity");
}