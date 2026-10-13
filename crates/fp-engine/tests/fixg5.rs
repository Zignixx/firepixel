//! FIX-G5 (Nutzerentscheid 13.10.2026 21:20, `docs\parity\TWEAKS_2026-10-13.md`):
//! * alt 4: neue Linien, Stiftpfade und Rechteck/Ellipse/Polygon bekommen ab Werk die Strichart
//!   „Präzise abger.“ (`bc_Basic`/`bn_Hard Rounded`, 1 px) wie im Original; eine gewählte Strichart
//!   des Werkzeug-Inspektors gilt weiter.
//! * Q-02: „Maske ▸ Alles einblenden/ausblenden“ auf einer Bitmap ergibt ein maskiertes Einzelelement
//!   (JS `[object Image]` mit `mask`, gemessen im Original).

use crate::common;
use common::*;
use fp_engine::tools::ToolId;
use fp_engine::{Session, ToolEvent};
use fp_model::{Brush, ElementKind};
use serde_json::json;

fn tool(s: &mut Session, t: ToolId) {
    s.set_tool(t).unwrap_or_else(|e| panic!("{e}"));
}

fn drag(s: &mut Session, a: (f32, f32), b: (f32, f32)) {
    events(s, &[ToolEvent::down(a.0, a.1)]);
    for i in 1..=10 {
        let t = i as f32 / 10.0;
        events(s, &[ToolEvent::drag(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)]);
    }
    events(s, &[ToolEvent::up(b.0, b.1)]);
}

/// Pinsel des zuletzt gezeichneten (gewählten) Objekts.
fn brush_of_selection(s: &Session) -> Brush {
    let d = s.active().expect("Dokument");
    let id = *d.selection.elements.first().expect("Auswahl");
    let e = el(s, id);
    match &e.kind {
        ElementKind::Path(p) => p.attrs.brush.clone().expect("Strich"),
        k => panic!("kein Pfad: {k:?}"),
    }
}

fn session() -> Session {
    let mut s = doc_session(400.0, 300.0);
    s.set_colors(Some(fp_color::Argb(0xffc0_4040)), Some(fp_color::Argb(0xff80_8080)));
    s
}

#[test]
fn shapes_and_lines_default_to_hard_rounded() {
    for t in [ToolId::Line, ToolId::Rectangle, ToolId::Ellipse, ToolId::Polygon] {
        let mut s = session();
        tool(&mut s, t);
        drag(&mut s, (50.0, 50.0), (150.0, 120.0));
        let b = brush_of_selection(&s);
        assert_eq!((b.category.as_str(), b.name.as_str(), b.diameter), ("bc_Basic", "bn_Hard Rounded", 1), "{t:?}");
    }
    // Stift: zwei Klicks, Esc beendet
    let mut s = session();
    tool(&mut s, ToolId::Pen);
    events(&mut s, &[ToolEvent::down(40.0, 40.0), ToolEvent::up(40.0, 40.0), ToolEvent::down(140.0, 90.0), ToolEvent::up(140.0, 90.0), ToolEvent::key(fp_engine::ToolKey::Escape)]);
    let b = brush_of_selection(&s);
    assert_eq!((b.category.as_str(), b.name.as_str()), ("bc_Basic", "bn_Hard Rounded"), "Stift");
}

#[test]
fn chosen_tool_stroke_still_wins() {
    let mut s = session();
    s.tools.options.new_stroke = Some(json!({"preset": "Bleistift/1-Pixel hart", "width": 1, "softness": 0, "placement": "center"}));
    tool(&mut s, ToolId::Line);
    drag(&mut s, (50.0, 50.0), (150.0, 120.0));
    let b = brush_of_selection(&s);
    assert_eq!((b.category.as_str(), b.name.as_str()), ("bc_Pencil", "bn_1-Pixel"));
    // Werkzeugvorgabe ohne Strichvorgabe: Werksvorgabe
    assert_eq!(fp_engine::commands::panel::tool_brush_from(None, None).name, "bn_Hard Rounded");
}

#[test]
fn reveal_and_hide_all_keep_a_masked_image() {
    for cmd in ["modify.mask.revealAll", "modify.mask.hideAll"] {
        let mut s = doc_session(400.0, 300.0);
        let id = bitmap(&mut s, 40, 40, 100, 80, [200, 30, 30, 255]);
        ex(&mut s, "element.setName", json!({"ids": [id.0], "name": "B1"}));
        select(&mut s, &[id]);
        ex(&mut s, cmd, json!({}));
        let tops = top_elements(&s);
        assert_eq!(tops.len(), 1, "{cmd}");
        let e = &tops[0];
        let (child, mask) = e.masked_single().unwrap_or_else(|| panic!("{cmd}: maskiertes Einzelelement"));
        assert!(matches!(child.kind, ElementKind::Bitmap(_)) && matches!(mask.kind, ElementKind::Bitmap(_)), "{cmd}");
        assert_eq!(e.js_type(), "Image", "{cmd}: JS sieht ein Bild wie im Original");
        assert_eq!(child.common.name.as_deref(), Some("B1"));
    }
}
