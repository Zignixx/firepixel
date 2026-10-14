//! F2 (14.10.2026): Import-Nachbesserungen TW-15 (PSD Formebene, verschachtelte Gruppen), TW-25 (AI-Text),
//! Q-13 (SVG-`<text>` als Textobjekt). Werte des Originals aus `re\_testdateien\par\run\xs-file-formats\log.jsonl`.

use std::path::PathBuf;

use fp_io::ImportOptions;
use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};

fn par_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("re").join("_testdateien").join("par").join("par_import")
}

fn describe_el(e: &Element, out: &mut Vec<String>, depth: usize) {
    let kind = match &e.kind {
        ElementKind::Bitmap(_) => "Bitmap".to_string(),
        ElementKind::Path(_) => "Pfad".into(),
        ElementKind::Text(_) => "Text".into(),
        ElementKind::Group(g) => format!("Gruppe {:?}", g.kind),
        ElementKind::SubLayer(l) => format!("Unterebene {} (Stufe {})", l.name, l.level),
        other => format!("{:?}", std::mem::discriminant(other)),
    };
    out.push(format!("{}{} [{}]", "  ".repeat(depth), e.common.name.clone().unwrap_or_default(), kind));
    if let ElementKind::SubLayer(l) = &e.kind
        && let Some(c) = l.cels.first()
    {
        for k in &c.elements {
            describe_el(k, out, depth + 1);
        }
    }
}

fn describe(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for slot in doc.pages[0].root.layers.iter().rev() {
        let LayerSlot::Own(l) = slot else { continue };
        if l.kind == LayerKind::Web {
            continue;
        }
        out.push(format!("Ebene {}", l.name));
        if let Some(c) = l.cels.first() {
            for e in &c.elements {
                describe_el(e, &mut out, 1);
            }
        }
    }
    out
}

fn all_elements(doc: &Document) -> Vec<Element> {
    fn walk(e: &Element, out: &mut Vec<Element>) {
        if let ElementKind::SubLayer(l) = &e.kind {
            for c in l.cels.first().map(|c| c.elements.clone()).unwrap_or_default() {
                walk(&c, out);
            }
        } else {
            out.push(e.clone());
        }
    }
    let mut out = Vec::new();
    for slot in &doc.pages[0].root.layers {
        if let LayerSlot::Own(l) = slot
            && let Some(c) = l.cels.first()
        {
            for e in &c.elements {
                walk(e, &mut out);
            }
        }
    }
    out
}

fn open(name: &str) -> Option<Document> {
    let data = std::fs::read(par_dir().join(name)).ok()?;
    Some(fp_io::open_bytes(name, &data, &ImportOptions::default()).expect("öffnen").document)
}

/// TW-15: Formebene „Pfad“ sichtbar (blaues Rechteck 9,9–90,90), Gruppen als Unterebenen in „Ebene“.
#[test]
fn tw15_psd_shape_and_nested_groups() {
    let Some(doc) = open("par.psd") else {
        eprintln!("par.psd fehlt");
        return;
    };
    let d = describe(&doc);
    println!("{d:#?}");
    assert_eq!(
        d,
        vec![
            "Ebene Ebene",
            "   [Unterebene Ebene 2 (Stufe 1)]",
            "    PSD [Text]",
            "   [Unterebene Ebene 1 (Stufe 1)]",
            "    Pfad [Pfad]",
            "  Background [Bitmap]",
        ]
    );
    let els = all_elements(&doc);
    let pfad = els.iter().find(|e| e.common.name.as_deref() == Some("Pfad")).expect("Pfad");
    let ElementKind::Path(p) = &pfad.kind else { panic!("Pfad ist kein Pfad") };
    let b = fp_model::new::contour_bounds(&p.contours);
    println!("Pfad {b:?} {:?} sichtbar={}", p.attrs, pfad.common.visible);
    assert!((b.left - 9.0).abs() < 1.5 && (b.top - 9.0).abs() < 1.5 && (b.right - 91.0).abs() < 1.5 && (b.bottom - 91.0).abs() < 1.5, "{b:?}");
    let img = fp_render::Renderer::default().render_page(&doc, 0, 0).image;
    let w = 200usize;
    let px = |x: usize, y: usize| img.data()[y * w + x];
    let c = px(50, 50);
    assert!(c[2] > 150 && c[0] < 100, "Mitte der Formebene blau: {c:?}");
    assert_eq!(px(150, 120), [255, 255, 255, 255], "außerhalb weiß");
}


/// TW-25: `.ai` aus dem Korpus (Flächentext „Hallo“, Myriad Pro 48) ⇒ ein Textobjekt im Rahmen 8,18–112,147.6.
#[test]
fn tw25_ai_area_text() {
    let Some(doc) = open("par.ai") else {
        eprintln!("par.ai fehlt");
        return;
    };
    let els = all_elements(&doc);
    assert_eq!(els.len(), 1, "genau ein Objekt");
    let ElementKind::Text(t) = &els[0].kind else { panic!("kein Textobjekt") };
    assert_eq!(t.runs.iter().map(|r| r.text.as_str()).collect::<String>(), "Hallo");
    assert_eq!(t.frame_type, Some(1));
    assert_eq!(t.initial.font.as_deref(), Some("Myriad Pro"));
    assert_eq!(t.initial.ps_font.as_deref(), Some("MyriadPro-Regular"));
    assert_eq!(t.initial.size, Some(48.0));
    let r = t.anchor_rect.expect("Rahmen");
    assert_eq!((r.left, r.top, r.right), (8.0, 18.0, 112.0));
    println!("{:?} {:?}", t.bounds, t.runs[0].attrs.color);
}

/// Q-13: `par.svg` ⇒ 3 Pfade + Textobjekt „SVG 1“ (Arial 18, #222222, Grundlinie bei 20,100), Text oben.
#[test]
fn q13_svg_text_object() {
    let Some(doc) = open("par.svg") else {
        eprintln!("par.svg fehlt");
        return;
    };
    let els = all_elements(&doc);
    assert_eq!(els.len(), 4, "3 Pfade + 1 Text");
    let ElementKind::Text(t) = &els[0].kind else { panic!("oberstes Objekt ist kein Text") };
    assert_eq!(t.runs[0].text, "SVG 1");
    assert_eq!(t.initial.font.as_deref(), Some("Arial"));
    assert_eq!(t.initial.size, Some(18.0));
    assert_eq!(t.runs[0].attrs.color, Some(0xFF22_2222));
    let a = t.anchor_rect.expect("Ursprung");
    assert_eq!((a.left, a.top), (20.0, 100.0));
    assert_eq!(els.iter().filter(|e| matches!(e.kind, ElementKind::Path(_))).count(), 3);
}

/// Q-13: geerbte Attribute, `<tspan>`, Transformation, viewBox-Skalierung, Fett/Kursiv, Ausrichtung.
#[test]
fn q13_svg_text_inherited_and_transformed() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100" viewBox="0 0 100 50">
      <g font-family="'Verdana', sans-serif" style="fill:#ff0000;font-size:10px" transform="translate(5,0)">
        <text x="10" y="20" font-weight="bold" font-style="italic" text-anchor="middle"><tspan>Hallo</tspan>
          <tspan fill="blue">Welt</tspan></text>
      </g>
      <defs><text x="0" y="0">versteckt</text></defs>
      <text x="1" y="2" display="none">aus</text>
    </svg>"##;
    let doc = fp_io::open_bytes("t.svg", svg, &ImportOptions::default()).expect("SVG").document;
    let els = all_elements(&doc);
    assert_eq!(els.len(), 1, "nur der sichtbare Text");
    let ElementKind::Text(t) = &els[0].kind else { panic!() };
    assert_eq!(t.runs[0].text, "Hallo Welt");
    assert_eq!(t.initial.font.as_deref(), Some("Verdana"));
    assert_eq!(t.initial.size, Some(20.0), "10 px × viewBox-Faktor 2");
    assert_eq!((t.initial.bold, t.initial.italic, t.initial.alignment), (Some(true), Some(true), Some(1)));
    assert_eq!(t.runs[0].attrs.color, Some(0xFFFF_0000));
    let a = t.anchor_rect.expect("Ursprung");
    assert_eq!((a.left, a.top), (30.0, 40.0));
}

/// TW-25: Punkttext (`To 0`) mit Grundlinie aus `Tp`, Oktal-Umlaut und mehreren Zeilen.
#[test]
fn tw25_ai_point_text() {
    let ai = "%!PS-3.0\n%%BoundingBox: 0 0 100 50\n%AI5_FileFormat 3\n%%EndSetup\n0 To\n1 0 0 1 10 20 0 Tp\nTP\n/_ArialMT 12 Tf\n1 0 0 Xa\n(Gr\\374\\337e\\r) Tx\n(Zeile 2) Tx\nTO\n";
    let r = fp_io::open_bytes("p.ai", ai.as_bytes(), &ImportOptions::default()).expect("AI");
    let els = all_elements(&r.document);
    assert_eq!(els.len(), 1);
    let ElementKind::Text(t) = &els[0].kind else { panic!() };
    assert_eq!(t.runs[0].text, "Grüße\nZeile 2");
    assert_eq!(t.frame_type, Some(0));
    assert_eq!(t.initial.font.as_deref(), Some("Arial"));
    assert_eq!(t.runs[0].attrs.color, Some(0xFFFF_0000));
    let a = t.anchor_rect.expect("Ursprung");
    assert_eq!((a.left, a.top), (10.0, 30.0));
}
