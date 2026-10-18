//! FIX-D: Befunde aus dem Klicktest CT-2 (`docs\funktionstest\ct2_2026-10-13.md`).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::ElementKind;
use serde_json::json;

fn pixel(s: &Session, id: fp_model::ElementId, x: i32, y: i32) -> [u8; 4] {
    let e = el(s, id);
    let ElementKind::Bitmap(b) = &e.kind else { panic!("keine Bitmap") };
    let img = fp_engine::docops::bitmap_image(b).expect("Bild");
    img.get(x, y).expect("Pixel")
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=30 {
        let t = i as f32 / 30.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

/// CT-2 W8: Farbe ersetzen (Größe 13, Toleranz 32, Stärke 200, Grün → Blau) auf grüner Bitmap –
/// beim Ziehen wird der ganze Strich blau, nicht nur der erste Stempel.
#[test]
fn replace_color_drag_replaces_whole_stroke() {
    use fp_engine::prefs::PrefValue::{Bool, Int, Str};
    let mut s = doc_session(300.0, 200.0);
    let b = bitmap(&mut s, 0, 0, 300, 200, [0, 255, 0, 255]);
    select(&mut s, &[b]);
    for (k, v) in [
        ("ReplaceColorToolSrcColor", Str("#00FF00".into())),
        ("ReplaceColorToolDestColor", Str("#0000FF".into())),
        ("ReplaceColorToolTolerance", Int(32)),
        ("ReplaceColorToolStrength", Int(200)),
        ("ReplaceColorToolDiameter", Int(13)),
        ("ReplaceColorToolColorize", Bool(false)),
        ("ReplaceColorToolMode", Bool(false)),
    ] {
        s.prefs.set(k, v);
    }
    s.set_tool(ToolId::ReplaceColor).expect("Werkzeug");
    drag(&mut s, (40.0, 100.0), (240.0, 100.0));
    for x in [40, 80, 140, 200, 239] {
        let p = pixel(&s, b, x, 100);
        assert!(p[1] < 10 && p[2] > 240, "x {x}: {p:?} statt blau");
    }
}

fn textures_installed() -> bool {
    !fp_engine::resources::files_in(&fp_engine::resources::dirs(fp_engine::resources::TEXTURES)).is_empty()
}

fn fill_texture(s: &Session, id: fp_model::ElementId) -> (Option<String>, bool, i32) {
    let e = el(s, id);
    let a = fp_engine::docops::path_attrs_of(&e).expect("Pfad");
    let t = a.fill_texture.as_ref();
    (t.map(|t| t.name.clone()), t.is_some_and(|t| t.mask.is_some()), a.fill.as_ref().map(|f| f.texture_amount).unwrap_or(0))
}

/// CT-2 FV-16: Texturen auch über den angezeigten Namen („Ader“ = Datei „Vein“) – Rechteck-Werkzeug,
/// Füllwerkzeug, `element.setTexture`; im Dokument steht der Dateiname. Unbekannter Name →
/// Fehlermeldung. Ohne Texturordner (anderer Rechner): übersprungen.
#[test]
fn texture_by_display_name_and_unknown_name() {
    if !textures_installed() {
        eprintln!("übersprungen: keine Texturordner");
        return;
    }
    use fp_engine::resources::texture_file;
    let vein = texture_file("Vein").expect("Vein");
    assert_eq!(texture_file("Ader").as_ref(), Some(&vein), "deutscher Anzeigename");
    assert_eq!(texture_file("ader").as_ref(), Some(&vein), "ohne Groß-/Kleinschreibung");
    fp_i18n::with_language("en", || assert_eq!(texture_file("Ader").as_ref(), Some(&vein), "auch bei englischer Oberfläche"));
    assert!(texture_file("gibtsnicht").is_none());

    let mut s = doc_session(300.0, 200.0);
    // element.setTexture mit dem Listennamen
    let r = rect(&mut s, 10.0, 10.0, 90.0, 90.0);
    select(&mut s, &[r]);
    ex(&mut s, "element.setTexture", json!({"name": "Ader", "amount": 50}));
    let (n, mask, _) = fill_texture(&s, r);
    assert_eq!(n.as_deref(), Some("Vein"));
    assert!(mask, "Textur mit Maske (sichtbar)");
    // pi.fill mit Listennamen und mit unbekanntem Namen
    let r2 = rect(&mut s, 100.0, 10.0, 190.0, 90.0);
    ex(&mut s, "pi.fill", json!({"ids": [r2.0], "texture": "Ader", "textureAmount": 50}));
    assert_eq!(fill_texture(&s, r2), (Some("Vein".into()), true, 500));
    let err = s.execute("pi.fill", json!({"ids": [r2.0], "texture": "gibtsnicht"}));
    assert!(err.is_err(), "unbekannter Name → Fehler");
    assert!(s.execute("pi.stroke", json!({"ids": [r2.0], "texture": "gibtsnicht"})).is_err(), "Strich: unbekannter Name → Fehler");
    assert_eq!(fill_texture(&s, r2).0.as_deref(), Some("Vein"), "nichts geändert");

    // Rechteck-Werkzeug mit Textur „Ader“ 50 aus dem Inspektor
    s.tools.options.new_fill = Some(json!({"category": "solid", "edge": 1, "texture": "Ader", "textureAmount": 50, "textureTransparent": false}));
    ex(&mut s, "select.none", json!({}));
    s.set_tool(ToolId::Rectangle).expect("Rechteck");
    drag(&mut s, (20.0, 110.0), (120.0, 190.0));
    let id = fp_model::ElementId(s.active().expect("Dokument").selection.elements[0].0);
    assert_eq!(fill_texture(&s, id), (Some("Vein".into()), true, 500), "neues Rechteck mit Textur");
    // unbekannte Textur in der Werkzeugvorgabe: Zeichnen geht trotzdem
    s.tools.options.new_fill = Some(json!({"category": "solid", "texture": "gibtsnicht", "textureAmount": 50}));
    drag(&mut s, (200.0, 110.0), (280.0, 190.0));

    // Füllwerkzeug auf ein Objekt: Füllung samt Textur
    let r3 = rect(&mut s, 200.0, 10.0, 290.0, 90.0);
    ex(&mut s, "select.none", json!({}));
    s.tools.options.new_fill = Some(json!({"category": "solid", "texture": "Ader", "textureAmount": 50}));
    s.set_tool(ToolId::PaintBucket).expect("Füllwerkzeug");
    events(&mut s, &[ToolEvent::down(245.0, 50.0), ToolEvent::up(245.0, 50.0)]);
    assert_eq!(fill_texture(&s, r3), (Some("Vein".into()), true, 500), "Füllwerkzeug mit Textur");
}
