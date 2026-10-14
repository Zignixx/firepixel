//! F12 (14.10.2026): TW-37 (PSD „Background“ wie im Original) und TW-38 (Lage des AI-Flächentexts).
//! Werte des Originals per JSF gemessen (Bericht `docs\berichte\f12.md`).

use std::path::PathBuf;

use fp_io::ImportOptions;
use fp_model::{Document, ElementKind, LayerSlot};

fn par_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("re").join("_testdateien").join("par").join("par_import")
}

/// Tintengrenzen (dunkle Pixel) der gerenderten Seite, rechts/unten exklusiv wie `pixelRect`.
fn ink(doc: &Document, w: usize) -> Option<(usize, usize, usize, usize)> {
    let img = fp_render::Renderer::default().render_page(doc, 0, 0).image;
    let data = img.data();
    let h = data.len() / w;
    let (mut l, mut t, mut r, mut b) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..h {
        for x in 0..w {
            let p = data[y * w + x];
            if p[3] > 0 && (p[0] as u32 + p[1] as u32 + p[2] as u32) < 600 {
                l = l.min(x);
                t = t.min(y);
                r = r.max(x + 1);
                b = b.max(y + 1);
            }
        }
    }
    (l != usize::MAX).then_some((l, t, r, b))
}

fn text_anchor(doc: &Document) -> fp_geom::Rect {
    let LayerSlot::Own(l) = &doc.pages[0].root.layers[0] else { panic!("Ebene") };
    let e = &l.cels[0].elements[0];
    let ElementKind::Text(t) = &e.kind else { panic!("kein Text") };
    t.anchor_rect.expect("Rahmen")
}

/// TW-38: `par.ai` ⇒ Tinte 9,20–107,55 wie im Original (vorher 11,18–109,53).
#[test]
fn tw38_ai_area_text_position() {
    let Ok(data) = std::fs::read(par_dir().join("par.ai")) else {
        eprintln!("par.ai fehlt");
        return;
    };
    let doc = fp_io::open_bytes("par.ai", &data, &ImportOptions::default()).expect("AI").document;
    let a = text_anchor(&doc);
    assert_eq!((a.left, a.top), (6.0, 20.0), "rawLeft/rawTop des Originals");
    let Some((l, t, r, b)) = ink(&doc, 120) else { panic!("keine Tinte") };
    println!("Tinte {l},{t}–{r},{b}");
    // Schriftraster Myriad Pro: ±1 px
    assert!(l.abs_diff(9) <= 1 && t.abs_diff(20) <= 1 && r.abs_diff(107) <= 1 && b.abs_diff(55) <= 1, "Tinte {l},{t}–{r},{b}");
}

/// TW-38: Versatz −2/+2 gilt für jede Lage (Original: Pfad 30,50 ⇒ raw 28,42; Pfad 20,30 ⇒ raw 18,62).
#[test]
fn tw38_ai_area_text_offset_other_positions() {
    for (x, y, rl, rt) in [(30.0, 50.0, 28.0, 42.0), (20.0, 30.0, 18.0, 62.0)] {
        let ai = format!(
            "%!PS-3.0\n%%BoundingBox: 0 0 120 90\n%AI5_FileFormat 3\n%%EndSetup\n1 To\n1 0 0 1 {x} {y} 0 Tp\n{x} {y} m\n{r} {y} l\n{r} {b} l\n{x} {b} l\n{x} {y} l\nn\nTP\n/_MyriadPro-Regular 24 Tf\n0 0 0 Xa\n(Hallo) Tj\nTO\n",
            r = x + 104.0,
            b = y - 129.6
        );
        let doc = fp_io::open_bytes("t.ai", ai.as_bytes(), &ImportOptions::default()).expect("AI").document;
        let a = text_anchor(&doc);
        assert_eq!((a.left, a.top), (rl, rt), "Pfad {x},{y}");
        assert_eq!(a.right - a.left, 104.0, "Breite des Pfads bleibt");
    }
}

/// TW-37: `par.psd` ⇒ „Background“ liegt direkt in „Ebene“ (nicht in „Ebene 1“), wie im Original: JSF
/// `dom.layers` = [Ebene: Background] [Ebene 1: Pfad] [Ebene 2: PSD] [Webebene].
#[test]
fn tw37_psd_background_in_top_layer() {
    let Ok(data) = std::fs::read(par_dir().join("par.psd")) else {
        eprintln!("par.psd fehlt");
        return;
    };
    let doc = fp_io::open_bytes("par.psd", &data, &ImportOptions::default()).expect("PSD").document;
    let LayerSlot::Own(top) = &doc.pages[0].root.layers[0] else { panic!("Ebene") };
    assert_eq!(top.name, "Ebene");
    let els = &top.cels[0].elements;
    let bg: Vec<_> = els.iter().filter(|e| e.common.name.as_deref() == Some("Background")).collect();
    assert_eq!(bg.len(), 1, "genau ein Background direkt in „Ebene“");
    assert!(matches!(bg[0].kind, ElementKind::Bitmap(_)));
    for e in els {
        if let ElementKind::SubLayer(s) = &e.kind {
            let names: Vec<_> = s.cels[0].elements.iter().filter_map(|k| k.common.name.clone()).collect();
            assert!(!names.iter().any(|n| n == "Background"), "{}: {names:?}", s.name);
            if s.name == "Ebene 1" {
                assert_eq!(names, vec!["Pfad".to_string()], "Ebene 1 enthält nur „Pfad“");
            }
        }
    }
}
