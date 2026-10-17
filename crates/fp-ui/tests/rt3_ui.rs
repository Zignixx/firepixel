//! RT-3 (re 28): Messwerte des Originals für die Oberfläche – L4 Einfügen fremder Pixel, D4
//! Exportvorschau im Dokumentmaßstab, D5 Rasterzelle des Farb-Popups, S3 Zeilenklick einer
//! (Unter-)Ebene, L9 Rastern nur mit dem Filter.

use fp_engine::Session;
use fp_ui::app_state::{AppState, UiSettings};
use serde_json::json;

fn state(w: u32, h: u32) -> AppState {
    fp_engine::shell::disable();
    fp_i18n::profile::use_test_profile();
    let mut s = Session::default();
    s.execute("file.new", json!({"width": w, "height": h, "background": "#ffffff"})).expect("Dokument");
    AppState::new(s, UiSettings::default(), None)
}

/// `re5a_t1_rand.png`: 400 × 300, rotes Quadrat 100,50–200,150, sonst Alpha 0 (RGBA-Bytes).
fn rand_bytes(alpha_outside: u8) -> Vec<u8> {
    let (w, h) = (400usize, 300usize);
    let mut v = Vec::with_capacity(w * h * 4);
    for i in 0..w * h {
        let (x, y) = (i % w, i / w);
        if (100..200).contains(&x) && (50..150).contains(&y) {
            v.extend_from_slice(&[255, 0, 0, 255]);
        } else {
            v.extend_from_slice(&[0, 0, 0, alpha_outside]);
        }
    }
    v
}

/// L4: PNG (mit Alpha) wird auf die deckenden Pixel beschnitten und mittig auf die Leinwand 500 ×
/// 500 gelegt (200,200, 100 × 100), der Versatz im Quellbild entfällt; eine DIB ohne Alpha bleibt
/// ganz (400 × 300 bei 50,100).
#[test]
fn l4_foreign_png_trimmed_and_centered_dib_whole() {
    let st = state(500, 500);
    let png = fp_ui::os_clipboard::prepare_foreign(400, 300, &rand_bytes(0), 1).expect("Bild");
    assert_eq!((png.width(), png.height(), png.origin()), (100, 100, (0, 0)));
    assert_eq!(png.pixel(0, 0).a, 255);
    assert_eq!(fp_ui::os_clipboard::paste_origin(&st, png.width(), png.height()), [200.0, 200.0]);
    // DIB (24 Bit): alle Alphawerte 0 → ganz und deckend
    let mut dib = rand_bytes(0);
    for p in dib.chunks_mut(4) {
        p[3] = 0;
    }
    let dib = fp_ui::os_clipboard::prepare_foreign(400, 300, &dib, 1).expect("DIB");
    assert_eq!((dib.width(), dib.height()), (400, 300));
    assert_eq!(dib.pixel(0, 0).a, 255);
    assert_eq!(fp_ui::os_clipboard::paste_origin(&st, 400, 300), [50.0, 100.0]);
    // ganz durchsichtig: nichts
    assert!(fp_ui::os_clipboard::prepare_foreign(4, 4, &[0u8, 0, 0, 1].repeat(16), 1).is_none());
}

/// D4: Die Exportvorschau im Dokumentfenster rechnet ohne Exportskalierung (deckungsgleich mit
/// „Original“), die Bildvorschau mit.
#[test]
fn d4_document_preview_ignores_export_scale() {
    let mut s = fp_export::options::OptimizeSettings::default();
    s.scale = Some(fp_model::ExportScale { use_scale: true, constrain: true, percent: 50, width: 600, height: 450 });
    let u = fp_ui::preview::unscaled(&s);
    assert_eq!(u.scale, None);
    assert_eq!(fp_ui::preview::unscaled(&u), u);
}

/// D5: Klick in eine Rasterzelle setzt die Farbe mit Deckkraft 100 (#ff000080 → Weiß = #ffffff).
#[test]
fn d5_grid_cell_sets_full_opacity() {
    for (c, r) in [(0, 0), (5, 3), (20, 14)] {
        let g = fp_ui::inspector::color_picker::grid_choice(c, r);
        assert_eq!(g.a(), 255, "Zelle {c},{r}");
        assert_eq!(g.0 & 0x00ff_ffff, fp_ui::inspector::color_picker::grid_color(c, r).0 & 0x00ff_ffff);
    }
}

/// S3: Klick auf „Ebene 1“ mit Unterebene „U“ wählt nur die eigenen Objekte (R1), nicht die
/// Unterebene und deren Objekte.
#[test]
fn s3_layer_row_selects_own_objects_only() {
    let mut st = state(400, 300);
    st.session.execute("draw.rectangle", json!({"rect": [20, 20, 120, 120]})).expect("R1");
    st.session.execute("pi.layer.addSub", json!({"name": "U"})).expect("U");
    st.session.execute("draw.rectangle", json!({"rect": [200, 100, 300, 200]})).expect("R2");
    let d = st.session.active().expect("d");
    let (_, l) = fp_engine::docops::own_layers(&d.doc, d.page).into_iter().find(|(_, l)| l.kind == fp_model::LayerKind::Normal).expect("Ebene 1");
    let cel = l.cel(d.frame).expect("Cel");
    assert!(cel.elements.iter().any(|e| matches!(e.kind, fp_model::ElementKind::SubLayer(_))), "Unterebene in Ebene 1");
    let own = fp_ui::panels::layers::own_objects(&cel.elements);
    assert_eq!(own.len(), 1, "nur R1");
    let sub = cel.elements.iter().find_map(|e| match &e.kind {
        fp_model::ElementKind::SubLayer(sl) => Some(sl.clone()),
        _ => None,
    });
    let sub = sub.expect("U");
    let sub_own = fp_ui::panels::layers::own_objects(&sub.cel(d.frame).expect("Cel U").elements);
    assert_eq!(sub_own.len(), 1, "U: nur R2");
    assert_ne!(own, sub_own);
}

/// L9: Das Rastern nach der Vektor-Rückfrage gilt nur, solange der Filterdialog offen ist.
#[test]
fn l9_rasterize_flag_cleared_when_no_dialog_open() {
    let mut st = state(400, 300);
    st.session.filter_rasterize = true;
    let ctx = egui::Context::default();
    st.process_actions(&ctx);
    assert!(!st.session.filter_rasterize, "kein Dialog offen → zurückgesetzt");
}

/// M6 §4b (re\33): Mit aktivem Malwerkzeug (Pinsel, Bleistift) zeigt der Inspektor nur die
/// Pinseloptionen – kein B/H/X/Y (Quadrant 2 leer, 14080); der Auswahlrahmen hat das Feld (14007).
#[test]
fn m6_paint_tool_inspector_has_no_xywh() {
    use fp_ui::inspector::layouts::select_layout;
    let mut st = state(700, 400);
    st.session.execute("draw.rectangle", json!({"rect": [10, 10, 60, 40]})).expect("Rechteck");
    for (tool, row) in [(fp_engine::ToolId::Brush, 2), (fp_engine::ToolId::Pencil, 13)] {
        st.session.set_tool(tool).expect("Werkzeug");
        let l = select_layout(&st.session).expect("Layout");
        assert_eq!((l.number, l.quads[1]), (row, 14080), "{tool:?}");
    }
    st.session.set_tool(fp_engine::ToolId::Marquee).expect("Werkzeug");
    assert_eq!(select_layout(&st.session).map(|l| l.quads[1]), Some(14007));
}
