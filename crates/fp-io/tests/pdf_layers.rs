//! U-1: PDF-Ebenen aus optionalen Inhalten (OCG). Die Testdateien entstehen hier von Hand
//! (unkomprimierte Inhaltsströme).

use fp_io::ImportOptions;
use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};

/// Minimale PDF-Datei aus Objekten (Nummer = Index + 1); `/Root` ist Objekt 1.
fn build_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn stream(dict: &str, data: &str) -> Vec<u8> {
    format!(
        "<< {dict} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
    .into_bytes()
}

/// Zwei Seiten 200 × 100 pt. Gruppen: 5 „Sky“, 6 „Trees“ (aus), 7 „Leaves“ (in /Order Kind von
/// „Trees“, nur über das /OC eines Form-XObjects benutzt). Seite 1: Inhalt außerhalb (unten und
/// oben), Sky, Trees (zwei Rechtecke), Leaves. Seite 2: außerhalb + Sky.
fn layered_pdf(with_ocgs: bool) -> Vec<u8> {
    let page1 = if with_ocgs {
        "0 0 1 rg 0 0 200 100 re f\n\
         /OC /MC0 BDC 1 0 0 rg 10 10 30 30 re f EMC\n\
         /OC /MC1 BDC 0 1 0 rg 50 10 30 30 re f 90 10 30 30 re f EMC\n\
         q /X0 Do Q\n\
         0 0 0 rg 150 10 20 20 re f"
    } else {
        "0 0 1 rg 0 0 200 100 re f\n\
         1 0 0 rg 10 10 30 30 re f\n\
         0 1 0 rg 50 10 30 30 re f 90 10 30 30 re f\n\
         q /X0 Do Q\n\
         0 0 0 rg 150 10 20 20 re f"
    };
    let page2 = if with_ocgs {
        "0 0 1 rg 0 0 200 100 re f /OC /MC0 BDC 1 0 0 rg 10 10 30 30 re f EMC"
    } else {
        "0 0 1 rg 0 0 200 100 re f 1 0 0 rg 10 10 30 30 re f"
    };
    let catalog = if with_ocgs {
        "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R 6 0 R 7 0 R] \
         /D << /Order [5 0 R 6 0 R [7 0 R]] /OFF [6 0 R] >> >> >>"
    } else {
        "<< /Type /Catalog /Pages 2 0 R >>"
    };
    let res = "/Resources << /Properties << /MC0 5 0 R /MC1 6 0 R >> /XObject << /X0 8 0 R >> >>";
    let form_oc = if with_ocgs { "/OC 7 0 R" } else { "" };
    build_pdf(&[
        catalog.as_bytes().to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] {res} /Contents 4 0 R >>")
            .into_bytes(),
        stream("", page1),
        b"<< /Type /OCG /Name (Sky) >>".to_vec(),
        b"<< /Type /OCG /Name (Trees) >>".to_vec(),
        // UTF-16BE mit BOM: „Leaves“
        b"<< /Type /OCG /Name <FEFF004C00650061007600650073> >>".to_vec(),
        stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 200 100] {form_oc}"),
            "1 1 0 rg 130 60 20 20 re f",
        ),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] {res} /Contents 10 0 R >>")
            .into_bytes(),
        stream("", page2),
    ])
}

/// Pfade (rekursiv) eines Elements.
fn paths(e: &Element) -> usize {
    match &e.kind {
        ElementKind::Path(_) => 1,
        ElementKind::Group(g) => g.children.iter().map(|c| paths(c)).sum(),
        _ => 0,
    }
}

/// (Name, sichtbar, Pfade) je Ebene einer Seite, unten → oben, ohne Web-Ebene.
fn layers(doc: &Document, page: usize) -> Vec<(String, bool, usize)> {
    doc.pages[page]
        .root
        .layers
        .iter()
        .filter_map(|s| match s {
            LayerSlot::Own(l) if l.kind != LayerKind::Web => Some((
                l.name.clone(),
                l.is_visible(),
                l.cels
                    .first()
                    .map_or(0, |c| c.elements.iter().map(|e| paths(e)).sum()),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn pdf_ocgs_become_layers() {
    let data = layered_pdf(true);
    let r = fp_io::open_bytes("layers.pdf", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.document.pages.len(), 2);
    let outside = fp_i18n::tr!("msg-default-layer", n = "1");
    assert_eq!(
        layers(&r.document, 0),
        vec![
            (outside.clone(), true, 2),
            ("Sky".to_string(), true, 1),
            ("Trees".to_string(), false, 2),
            ("Trees / Leaves".to_string(), true, 1),
        ]
    );
    assert_eq!(
        layers(&r.document, 1),
        vec![(outside, true, 1), ("Sky".to_string(), true, 1)]
    );
    // eindeutige Element-IDs über alle Ebenen
    let mut ids = std::collections::HashSet::new();
    for p in &r.document.pages {
        for s in &p.root.layers {
            if let LayerSlot::Own(l) = s {
                for c in &l.cels {
                    for e in &c.elements {
                        assert!(ids.insert(e.id), "doppelte ID");
                    }
                }
            }
        }
    }
}

#[test]
fn pdf_without_ocgs_stays_one_layer() {
    let data = layered_pdf(false);
    let r = fp_io::open_bytes("plain.pdf", &data, &ImportOptions::default()).unwrap();
    assert_eq!(r.document.pages.len(), 2);
    let l = layers(&r.document, 0);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].0, fp_i18n::tr!("msg-default-layer", n = "1"));
    assert!(l[0].1);
    assert_eq!(l[0].2, 6);
    assert_eq!(layers(&r.document, 1).len(), 1);
}

/// Gleiches Bild mit und ohne Gruppen (versteckte Ebene eingeblendet).
#[test]
fn pdf_layers_render_like_flat_page() {
    let flat = fp_io::open_bytes("plain.pdf", &layered_pdf(false), &ImportOptions::default())
        .unwrap()
        .document;
    let mut doc = fp_io::open_bytes("layers.pdf", &layered_pdf(true), &ImportOptions::default())
        .unwrap()
        .document;
    for p in doc.pages.iter_mut() {
        for s in std::sync::Arc::make_mut(p).root.layers.iter_mut() {
            if let LayerSlot::Own(l) = s {
                let l = std::sync::Arc::make_mut(l);
                let n = l.visibility.len().max(1);
                l.visibility = vec![true; n];
            }
        }
    }
    for page in 0..2 {
        let a = fp_render::Renderer::default()
            .render_page(&flat, page, 0)
            .image;
        let b = fp_render::Renderer::default()
            .render_page(&doc, page, 0)
            .image;
        assert_eq!((a.width(), a.height()), (b.width(), b.height()));
        let diff = a
            .data()
            .iter()
            .zip(b.data())
            .filter(|(x, y)| (0..4).any(|k| x[k].abs_diff(y[k]) > 8))
            .count();
        assert!(
            diff * 200 < a.data().len(),
            "Seite {page}: {diff} Pixel abweichend"
        );
    }
}

/// Verschachtelte Klammern (innerste Gruppe zählt), Inline-Bild außerhalb, Gruppe per
/// Inline-Eigenschaften (`<< /OC … >>`), /BaseState /OFF mit /ON.
#[test]
fn pdf_nested_groups_and_inline_image() {
    let content = "q 20 0 0 20 0 0 cm BI /W 2 /H 1 /BPC 8 /CS /RGB ID \u{1}\u{2}\u{3}\u{4}\u{5}\u{6} EI Q\n\
         /OC /A BDC 1 0 0 rg 30 0 20 20 re f\n\
         /OC << /OC 6 0 R >> BDC 0 1 0 rg 60 0 20 20 re f EMC\n\
         0 0 1 rg 90 0 20 20 re f EMC";
    let data = build_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R 6 0 R] \
          /D << /BaseState /OFF /ON [5 0 R] >> >> >>"
            .to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 120 20] \
          /Resources << /Properties << /A 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
        stream("", content),
        b"<< /Type /OCG /Name (Outer) >>".to_vec(),
        b"<< /Type /OCG /Name (Inner) >>".to_vec(),
    ]);
    let r = fp_io::open_bytes("nested.pdf", &data, &ImportOptions::default()).unwrap();
    let l = layers(&r.document, 0);
    let names: Vec<(&str, bool)> = l.iter().map(|(n, v, _)| (n.as_str(), *v)).collect();
    let outside = fp_i18n::tr!("msg-default-layer", n = "1");
    assert_eq!(
        names,
        vec![(outside.as_str(), true), ("Outer", true), ("Inner", false)]
    );
    assert_eq!(l[1].2, 2, "Outer: zwei Rechtecke");
    assert_eq!(l[2].2, 1, "Inner: ein Rechteck");
    // das Inline-Bild liegt in der Ebene außerhalb
    let has_image = |doc: &Document| {
        doc.pages[0].root.layers.iter().any(|s| {
            matches!(s, LayerSlot::Own(lay) if lay.name == outside
                && lay.cels[0].elements.iter().any(|e| !matches!(e.kind, ElementKind::Path(_))))
        })
    };
    assert!(has_image(&r.document), "Inline-Bild außerhalb");
}
