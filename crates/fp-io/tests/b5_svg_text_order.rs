//! B-5 (BA-076): SVG-Import – Text liegt in der Stapelfolge des SVG zwischen den Pfaden (bisher
//! immer oben), auch in Gruppen.

use fp_io::ImportOptions;
use fp_model::{Document, ElementKind, LayerKind, LayerSlot};

/// Elemente der ersten normalen Ebene oben → unten als Kurzform (`T:` Text, `P:` Pfad, `G[…]` Gruppe).
fn order(doc: &Document) -> Vec<String> {
    fn one(e: &fp_model::Element) -> String {
        let n = e.common.name.clone().unwrap_or_default();
        match &e.kind {
            ElementKind::Text(_) => format!("T:{n}"),
            ElementKind::Path(_) => format!("P:{n}"),
            ElementKind::Group(g) => format!(
                "G[{}]",
                g.children
                    .iter()
                    .map(|c| one(c))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            _ => format!("?:{n}"),
        }
    }
    let p = &doc.pages[0];
    for s in &p.root.layers {
        if let LayerSlot::Own(l) = s
            && l.kind == LayerKind::Normal
            && let Some(c) = l.cels.first()
        {
            return c.elements.iter().map(|e| one(e)).collect();
        }
    }
    Vec::new()
}

fn import(svg: &str) -> Document {
    fp_io::open_bytes("t.svg", svg.as_bytes(), &ImportOptions::default())
        .expect("SVG")
        .document
}

#[test]
fn text_between_paths_keeps_svg_order() {
    let d = import(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
          <rect id="unten" x="0" y="0" width="100" height="50" fill="#00f"/>
          <text id="mitte" x="10" y="30">Hallo</text>
          <rect id="oben" x="20" y="20" width="50" height="50" fill="#f00"/>
        </svg>"##,
    );
    // oben → unten
    assert_eq!(order(&d), vec!["P:oben", "T:mitte", "P:unten"]);
}

#[test]
fn text_in_group_and_with_fill_none_parent() {
    // ohne `id`: eine oberste Gruppe mit `id` wäre seit K-9 eine eigene Ebene der Datei
    let d = import(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
          <g opacity="0.5" fill="none">
            <text id="t1" x="10" y="30" fill="#000">A</text>
            <rect id="r1" x="0" y="0" width="10" height="10" fill="#0f0"/>
          </g>
          <rect id="r2" x="50" y="0" width="10" height="10" fill="#f00"/>
          <text id="t2" x="10" y="80">B</text>
        </svg>"##,
    );
    assert_eq!(order(&d), vec!["T:t2", "P:r2", "G[P:r1,T:t1]"]);
}

#[test]
fn only_text_and_hidden_text() {
    let d = import(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
          <text id="a" x="10" y="30">Eins</text>
          <text id="b" x="10" y="60">Zwei</text>
          <text id="c" x="10" y="90" display="none">aus</text>
        </svg>"##,
    );
    assert_eq!(order(&d), vec!["T:b", "T:a"]);
}
