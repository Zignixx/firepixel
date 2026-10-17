//! L-3: Symbolursprung in der Mitte des Inhalts (re 28 S2, re 13 T1) – Umrechnung älterer
//! Firepixel-Dateien (Ursprung in Dokumentkoordinaten, `tEXt Software` = „Firepixel“) beim Laden,
//! Dateien des Originals bleiben unberührt und werden gleich geschrieben.

use fp_color::Argb;
use fp_engine::docops;
use fp_engine::symbol_origin::{is_legacy, read_fwpng, symbol_content_bounds, upgrade_legacy};
use fp_geom::{Matrix3, Rect};
use fp_model::element::ElementKind;
use fp_model::layer::LayerSlot;
use fp_model::symbol::{NineSlice, Symbol, SymbolKind};
use fp_model::{Document, Element, ElementId, PathAttrs, SymbolId};
use std::path::PathBuf;
use std::sync::Arc;

fn push_into_symbol(sym: &mut Symbol, e: Element) {
    if let Some(LayerSlot::Own(l)) = sym.root.layers.first_mut()
        && let Some(c) = Arc::make_mut(l).cels.first_mut()
    {
        Arc::make_mut(c).elements.push(Arc::new(e));
    }
}

/// Wie eine Firepixel-Datei vor L-3: „Innen“ (Rechteck 100,100–200,150 in Dokumentkoordinaten,
/// 9-Slice) und „Außen“ (Instanz von „Innen“ mit Einheitsmatrix + Rechteck 300,100–340,140); auf der
/// Seite eine Instanz von „Außen“, nach der Umwandlung um 30,0 verschoben.
fn legacy_doc() -> (Document, ElementId, SymbolId, SymbolId) {
    let mut doc = Document::new(400.0, 300.0, Argb(0xffff_ffff));
    let mut ids = std::mem::take(&mut doc.ids);
    let inner_id = doc.new_symbol_id();
    let mut inner = Symbol::new(&mut ids, "Innen", inner_id, SymbolKind::Graphic);
    push_into_symbol(&mut inner, Element::new_rectangle(&mut ids, Rect::new(100.0, 100.0, 200.0, 150.0), 0.0, PathAttrs::default()));
    inner.nine_slice = Some(NineSlice {
        locked: Some(false),
        color: None,
        guides: vec![110.0, 140.0, 120.0, 180.0],
        bounds: Some(Rect::new(100.0, 100.0, 200.0, 150.0)),
        bounds_extra: Default::default(),
        view: None,
        view_extra: Default::default(),
        extra: Default::default(),
    });
    doc.symbols.push(Arc::new(inner));
    let outer_id = doc.new_symbol_id();
    let mut outer = Symbol::new(&mut ids, "Außen", outer_id, SymbolKind::Graphic);
    push_into_symbol(&mut outer, Element::new_instance(&mut ids, inner_id, Matrix3::IDENTITY));
    push_into_symbol(&mut outer, Element::new_rectangle(&mut ids, Rect::new(300.0, 100.0, 340.0, 140.0), 0.0, PathAttrs::default()));
    doc.symbols.push(Arc::new(outer));
    let inst = Element::new_instance(&mut ids, outer_id, Matrix3::translation(30.0, 0.0));
    let iid = inst.id;
    doc.ids = ids;
    let l = docops::top_normal_layer(&doc, 0);
    docops::insert_element(&mut doc, 0, 0, l, 0, Arc::new(inst)).expect("einfügen");
    (doc, iid, inner_id, outer_id)
}

fn write(doc: &Document, software: &str) -> Vec<u8> {
    let opts = fp_fwpng::WriteOptions { software: software.into(), ..fp_fwpng::WriteOptions::default() };
    fp_fwpng::write(doc, &opts, &fp_fwpng::NoFlattener).expect("schreiben")
}

/// Die (einzige) Instanz auf der Seite; die Laufzeit-IDs werden beim Lesen neu vergeben.
fn page_instance(doc: &Document) -> Arc<Element> {
    let mut found = None;
    doc.walk_elements(|ctx, _, e| {
        if matches!(ctx.root, fp_model::RootRef::Page(0)) && matches!(e.kind, ElementKind::Instance(_)) {
            found = Some(Arc::clone(e));
        }
    });
    found.expect("Instanz auf der Seite")
}

fn inst_bounds(doc: &Document) -> Rect {
    docops::element_bounds(doc, &page_instance(doc)).expect("Grenzen")
}

fn matrix_of(doc: &Document) -> Matrix3 {
    match &page_instance(doc).kind {
        ElementKind::Instance(i) => i.transform,
        _ => panic!("keine Instanz"),
    }
}

fn sym_bounds(doc: &Document, id: SymbolId) -> Rect {
    let (_, s) = doc.symbol_by_id(id).expect("Symbol");
    symbol_content_bounds(doc, s).expect("Inhalt")
}

#[test]
fn legacy_file_is_upgraded_on_load_and_written_as_new() {
    let (doc, _iid, inner, outer) = legacy_doc();
    let before = inst_bounds(&doc);
    assert_eq!(before, Rect::new(130.0, 100.0, 370.0, 150.0));
    let bytes = write(&doc, fp_fwpng::SOFTWARE_LEGACY);

    // alt → neu: gleiche Lage auf der Leinwand, Ursprung in der Mitte
    let up = read_fwpng(&bytes).expect("lesen");
    assert!(!is_legacy(&up), "Kennung umgestellt");
    assert!(up.file_meta.text_chunks.iter().any(|(k, v)| k == "Software" && v == fp_fwpng::SOFTWARE));
    assert_eq!(inst_bounds(&up), before, "Instanz bleibt an ihrer Stelle");
    // Innen: 100,100–200,150 → Mitte 150,125
    assert_eq!(sym_bounds(&up, inner), Rect::new(-50.0, -25.0, 50.0, 25.0));
    let (_, si) = up.symbol_by_id(inner).expect("Innen");
    let ns = si.nine_slice.as_ref().expect("9-Slice");
    assert_eq!(ns.guides, vec![-15.0, 15.0, -30.0, 30.0], "Hilfslinien wandern mit");
    assert_eq!(ns.bounds, Some(Rect::new(-50.0, -25.0, 50.0, 25.0)));
    // Außen: 100,100–340,150 → Mitte 220,125; Seiteninstanz T(220,125)·T(30,0)
    assert_eq!(sym_bounds(&up, outer), Rect::new(-120.0, -25.0, 120.0, 25.0));
    assert_eq!(matrix_of(&up), Matrix3::translation(250.0, 125.0));
    // die Instanz von „Innen“ in „Außen“: T(150,125)·T(−220,−125)
    let (_, so) = up.symbol_by_id(outer).expect("Außen");
    let nested = so.root.layers.iter().find_map(|s| match s {
        LayerSlot::Own(l) => l.cels[0].elements.iter().find_map(|e| match &e.kind {
            ElementKind::Instance(i) => Some(i.transform),
            _ => None,
        }),
        _ => None,
    });
    assert_eq!(nested, Some(Matrix3::translation(-70.0, 0.0)));

    // neu gespeichert (Kennung „r2“) und wieder geladen: keine zweite Umrechnung
    let again = write(&up, fp_fwpng::SOFTWARE);
    let mut re = fp_fwpng::read(&again).expect("lesen");
    assert!(!upgrade_legacy(&mut re), "nicht als alt erkannt");
    assert_eq!(matrix_of(&re), Matrix3::translation(250.0, 125.0));
    assert_eq!(sym_bounds(&re, inner), Rect::new(-50.0, -25.0, 50.0, 25.0));
    assert_eq!(write(&re, fp_fwpng::SOFTWARE), again, "stabil");
}

/// Laden über den Befehlsweg (`load_document`) rechnet ebenfalls um.
#[test]
fn legacy_file_upgraded_by_load_document() {
    let (doc, _, inner, _) = legacy_doc();
    let dir = std::env::temp_dir().join(format!("fp-l3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("Ordner");
    let p = dir.join("alt.fw.png");
    std::fs::write(&p, write(&doc, fp_fwpng::SOFTWARE_LEGACY)).expect("schreiben");
    let d = fp_engine::commands::load_document(&p).expect("laden");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(sym_bounds(&d, inner), Rect::new(-50.0, -25.0, 50.0, 25.0));
    assert_eq!(inst_bounds(&d), Rect::new(130.0, 100.0, 370.0, 150.0));
}

/// Neue Dokumente tragen die neue Kennung.
#[test]
fn new_documents_are_written_with_new_marker() {
    let doc = Document::new(50.0, 50.0, Argb(0xffff_ffff));
    let bytes = fp_fwpng::write(&doc, &fp_fwpng::WriteOptions::default(), &fp_fwpng::NoFlattener).expect("schreiben");
    let d = fp_fwpng::read(&bytes).expect("lesen");
    assert!(d.file_meta.text_chunks.iter().any(|(k, v)| k == "Software" && v == fp_fwpng::SOFTWARE));
    assert!(!is_legacy(&d));
}

/// Treue-Referenz des Originals (nur lesen): mittiger Ursprung wird nicht angefasst, das Modell ist
/// dasselbe wie ohne Umrechnung und wird gleich geschrieben.
#[test]
fn original_file_is_left_untouched() {
    let p = PathBuf::from(r"C:\Users\Administrator\Desktop\REA\re\_testdateien\par\par_symbol_web.fw.png");
    let Ok(bytes) = std::fs::read(&p) else {
        eprintln!("übersprungen: {} fehlt", p.display());
        return;
    };
    let plain = fp_fwpng::read(&bytes).expect("lesen");
    let doc = read_fwpng(&bytes).expect("lesen");
    assert!(!is_legacy(&doc));
    let opts = fp_fwpng::WriteOptions::default();
    let a = fp_fwpng::write(&plain, &opts, &fp_fwpng::NoFlattener).expect("schreiben");
    let b = fp_fwpng::write(&doc, &opts, &fp_fwpng::NoFlattener).expect("schreiben");
    assert_eq!(a, b, "gleich geschrieben");
    // Symbole des Originals: Ursprung in der Mitte des Inhalts
    let mut n = 0;
    for s in &doc.symbols {
        if let Some(b) = symbol_content_bounds(&doc, s) {
            let c = b.center();
            eprintln!("{}: {:?} Mitte {:?}", s.name, b, c);
            assert!(c.x.abs() <= 1.0 && c.y.abs() <= 1.0, "{}: Mitte {c:?}", s.name);
            n += 1;
        }
    }
    assert!(n > 0, "Referenz enthält Symbole");
}
