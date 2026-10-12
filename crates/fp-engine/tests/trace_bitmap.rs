//! Modifizieren ▸ Bitmap nachzeichnen (`modify.traceBitmap`, Agent VT-1): neue Ebene, Gruppe
//! gefüllter Pfade, Original bleibt bzw. wird gelöscht, ein Verlaufsschritt, Rückgängig/Wiederholen
//! exakt, Optionen, Verfügbarkeit.

use crate::common;

use common::*;
use fp_engine::Session;
use fp_geom::IRect;
use fp_model::element::ElementKind;
use fp_model::ElementId;
use fp_raster::RgbaImage;
use serde_json::json;

/// Zweifarbiges Logo (blauer Kreis, rotes Rechteck auf Weiß) als Bitmap bei (20, 30), 200 × 120.
fn logo_bitmap(s: &mut Session) -> ElementId {
    let mut img = RgbaImage::filled(IRect::from_xywh(20, 30, 200, 120), fp_color::Rgba8::new(255, 255, 255, 255)).expect("Bild");
    let w = img.width();
    for (i, px) in img.data_mut().iter_mut().enumerate() {
        let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
        if (x - 60.0).hypot(y - 60.0) < 40.0 {
            *px = [20, 60, 200, 255];
        } else if (120.0..180.0).contains(&x) && (30.0..100.0).contains(&y) {
            *px = [220, 30, 40, 255];
        }
    }
    s.clipboard.clear();
    s.clipboard.pixels = Some(img);
    let v = ex(s, "edit.paste", json!({}));
    ElementId(v["ids"][0].as_u64().unwrap_or(0))
}

fn model(s: &Session) -> String {
    serde_json::to_string(&*s.active().expect("Dokument").doc).unwrap_or_default()
}

fn layers(s: &Session) -> Vec<(fp_model::LayerId, String, usize)> {
    let d = s.active().expect("Dokument");
    fp_engine::docops::own_layers(&d.doc, d.page).into_iter().map(|(_, l)| (l.id, l.name.clone(), l.cels.first().map(|c| c.elements.len()).unwrap_or(0))).collect()
}

fn undo_count(s: &Session) -> usize {
    s.active().map(|d| d.history.undo_count()).unwrap_or(0)
}

#[test]
fn trace_creates_layer_and_group_and_undo_restores() {
    let mut s = doc_session(300.0, 200.0);
    let bmp = logo_bitmap(&mut s);
    select(&mut s, &[bmp]);
    let before = model(&s);
    let layers_before = layers(&s);
    let steps = undo_count(&s);
    let cur_layer = s.active().and_then(|d| d.layer);

    let v = ex(&mut s, "modify.traceBitmap", json!({}));
    assert_eq!(undo_count(&s), steps + 1, "genau ein Verlaufsschritt");
    let gid = ElementId(v["ids"][0].as_u64().expect("Gruppen-ID"));
    assert_eq!(v["keptOriginal"], json!(true));

    // neue Ebene direkt über der bisherigen, mit genau der Gruppe
    let after = layers(&s);
    assert_eq!(after.len(), layers_before.len() + 1);
    let pos_old = after.iter().position(|l| Some(l.0) == cur_layer).expect("alte Ebene");
    let new = &after[pos_old + 1];
    assert_eq!(new.1, fp_i18n::t("trace-layer-name"));
    assert_eq!(new.2, 1, "eine Gruppe auf der neuen Ebene");
    assert_eq!(s.active().and_then(|d| d.layer), Some(new.0), "neue Ebene ist aktiv");

    // Gruppe gefüllter Pfade ohne Strich, Auswahl = Gruppe
    assert_eq!(s.active().map(|d| d.selection.elements.clone()), Some(vec![gid]));
    let g = el(&s, gid);
    let ElementKind::Group(grp) = &g.kind else { panic!("keine Gruppe: {:?}", g.kind) };
    assert!((3..=8).contains(&grp.children.len()), "{} Pfade", grp.children.len());
    let mut colors = Vec::new();
    for c in &grp.children {
        let ElementKind::Path(p) = &c.kind else { panic!("kein Pfad") };
        assert!(p.attrs.brush.is_none(), "Pfad mit Strich");
        assert!(p.attrs.fill.is_some(), "Pfad ohne Füllung");
        assert!(p.even_odd);
        assert!(p.contours.iter().all(|k| k.closed && k.nodes.len() >= 2));
        colors.push(p.attrs.fill_color.to_rgba8());
    }
    let near = |c: &fp_color::Rgba8, t: (u8, u8, u8)| (c.r as i32 - t.0 as i32).abs() < 16 && (c.g as i32 - t.1 as i32).abs() < 16 && (c.b as i32 - t.2 as i32).abs() < 16;
    assert!(colors.iter().any(|c| near(c, (20, 60, 200))), "blau fehlt: {colors:?}");
    assert!(colors.iter().any(|c| near(c, (220, 30, 40))), "rot fehlt: {colors:?}");
    // Der Hintergrund (weiß, größte Fläche) liegt in der Gruppe zuunterst.
    assert!(near(colors.last().expect("Pfade"), (255, 255, 255)));
    // Lage in Dokumentkoordinaten = Lage der Bitmap
    assert!(rect_approx(bounds(&s, gid), bounds(&s, bmp)), "{:?} vs {:?}", bounds(&s, gid), bounds(&s, bmp));
    // Original bleibt
    assert!(fp_engine::docops::find_on_page(&s.active().expect("Dokument").doc, 0, bmp).is_some());
    let traced = model(&s);

    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(model(&s), before, "Rückgängig stellt exakt her");
    assert_eq!(layers(&s), layers_before);
    ex(&mut s, "edit.redo", json!({}));
    assert_eq!(model(&s), traced, "Wiederholen stellt das Ergebnis her");
}

#[test]
fn trace_without_original_and_options() {
    let mut s = doc_session(300.0, 200.0);
    let bmp = logo_bitmap(&mut s);
    select(&mut s, &[bmp]);
    let before = model(&s);
    let v = ex(&mut s, "modify.traceBitmap", json!({"preset": "blackWhite", "mode": "polygon", "keep_original": false, "layer_name": "Vektoren"}));
    assert_eq!(v["keptOriginal"], json!(false));
    assert!(fp_engine::docops::find_on_page(&s.active().expect("Dokument").doc, 0, bmp).is_none(), "Original nicht gelöscht");
    assert!(layers(&s).iter().any(|l| l.1 == "Vektoren"));
    let gid = ElementId(v["ids"][0].as_u64().expect("Gruppen-ID"));
    let ElementKind::Group(grp) = &el(&s, gid).kind else { panic!("keine Gruppe") };
    for c in &grp.children {
        let ElementKind::Path(p) = &c.kind else { panic!("kein Pfad") };
        assert_eq!(p.attrs.fill_color.to_rgba8(), fp_color::Rgba8::new(0, 0, 0, 255));
        assert!(p.contours.iter().flat_map(|k| &k.nodes).all(|n| n.pred == n.anchor && n.succ == n.anchor), "Polygon mit Henkeln");
    }
    ex(&mut s, "edit.undo", json!({}));
    assert_eq!(model(&s), before);

    // Optionen werden geprüft
    select(&mut s, &[bmp]);
    assert!(s.execute("modify.traceBitmap", json!({"preset": "unbekannt"})).is_err());
    assert!(s.execute("modify.traceBitmap", json!({"colors": 999})).is_err());
    assert_eq!(model(&s), before, "fehlgeschlagener Aufruf ändert nichts");
}

#[test]
fn trace_needs_bitmap_selection() {
    let mut s = doc_session(300.0, 200.0);
    let r = rect(&mut s, 10.0, 10.0, 50.0, 50.0);
    select(&mut s, &[r]);
    assert!(s.execute("modify.traceBitmap", json!({})).is_err());
    let info = s.registry().get("modify.traceBitmap").map(|c| c.info().clone()).expect("registriert");
    assert!(info.params.iter().any(|p| p.name == "keepOriginal"));
    assert!(info.params.iter().any(|p| p.name == "preset"));

    // Mehrere Bitmaps: je eine Gruppe auf derselben neuen Ebene
    let a = logo_bitmap(&mut s);
    let b = logo_bitmap(&mut s);
    ex(&mut s, "edit.move", json!({"ids": [b.0], "dx": 40, "dy": 20}));
    select(&mut s, &[a, b, r]);
    let n_layers = layers(&s).len();
    let v = ex(&mut s, "modify.traceBitmap", json!({"preset": "illustration"}));
    assert_eq!(v["ids"].as_array().map(|x| x.len()), Some(2));
    assert_eq!(layers(&s).len(), n_layers + 1);
}
