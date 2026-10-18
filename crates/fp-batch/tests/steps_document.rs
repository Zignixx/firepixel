//! Document steps (B-3, `docs/BATCH.md` §3.10): find and replace, show/hide layers, pages and
//! slices — on a small synthetic document built in code.

mod support;

use fp_batch::item::{DocItem, Item, ItemInfo, ItemKind, Meta};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::steps::document::{find_replace, page_name, set_layers, slices_of, web216};
use fp_batch::{
    CancelToken, ItemStatus, OutputFormat, Recipe, RunOptions, Severity, StepContext, StepRegistry,
    executor,
};
use fp_model::{Argb, Document, Element, ElementKind, LayerSlot, PathAttrs, Point, Rect};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use support::*;

const RED: u32 = 0xffff_0000;

/// 40 × 30 document: red rectangle and a text on layer 1, one slice on the web layer; a second
/// page with a blue rectangle.
fn doc() -> Document {
    let mut d = Document::new(40.0, 30.0, Argb(0xffff_ffff));
    let rect = Element::new_rectangle(
        &mut d.ids,
        Rect {
            left: 0.0,
            top: 0.0,
            right: 20.0,
            bottom: 30.0,
        },
        0.0,
        PathAttrs::solid_fill(Argb(RED)),
    );
    let text = Element::new_text(
        &mut d.ids,
        "Hello world, hello",
        "Arial",
        8.0,
        Argb(0xff00_0000),
        Point { x: 2.0, y: 10.0 },
    );
    let slice = Element::new_slice(
        &mut d.ids,
        Rect {
            left: 0.0,
            top: 0.0,
            right: 10.0,
            bottom: 10.0,
        },
    );
    add(&mut d, 0, 0, rect);
    add(&mut d, 0, 0, text);
    add(&mut d, 0, 1, slice);
    let p2 = d.add_page("Second");
    let blue = Element::new_rectangle(
        &mut d.ids,
        Rect {
            left: 0.0,
            top: 0.0,
            right: 40.0,
            bottom: 30.0,
        },
        0.0,
        PathAttrs::solid_fill(Argb(0xff00_00ff)),
    );
    add(&mut d, p2, 0, blue);
    d
}

fn add(d: &mut Document, page: usize, layer: usize, e: Element) {
    let p = Arc::make_mut(&mut d.pages[page]);
    let LayerSlot::Own(l) = &mut p.root.layers[layer] else {
        panic!("own layer");
    };
    let l = Arc::make_mut(l);
    Arc::make_mut(&mut l.cels[0])
        .elements
        .insert(0, Arc::new(e));
}

fn texts(d: &Document) -> Vec<String> {
    let mut out = Vec::new();
    d.walk_elements(|_, _, e| {
        if let ElementKind::Text(t) = &e.kind {
            out.push(t.runs.iter().map(|r| r.text.as_str()).collect());
        }
    });
    out
}

fn fills(d: &Document) -> Vec<u32> {
    let mut out = Vec::new();
    d.walk_elements(|_, _, e| {
        if let ElementKind::Path(p) = &e.kind {
            out.push(p.attrs.fill_color.0);
        }
    });
    out
}

fn info() -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/in/doc.fw.png"),
        root: None,
        kind: ItemKind::Document,
        format: "fwpng".into(),
        bytes: 0,
        width: 40,
        height: 30,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn apply(id: &str, params: Value, item: &mut Item) -> Vec<String> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let i = info();
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &i, &cancel, OutputFormat::Png);
    step.apply(item, &p, &mut ctx).expect("apply");
    ctx.warnings
}

fn doc_item(d: Document) -> Item {
    Item::Document(DocItem {
        doc: Arc::new(d),
        meta: Meta::default(),
    })
}

fn the_doc(item: &Item) -> &Document {
    match item {
        Item::Document(d) => &d.doc,
        _ => panic!("still a document"),
    }
}

fn issues(id: &str, params: Value) -> Vec<String> {
    StepRegistry::global()
        .get(id)
        .expect("step")
        .validate(&params)
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| i.param.unwrap_or_default())
        .collect()
}

#[test]
fn find_replace_text_whole_word_and_case() {
    let mut d = doc();
    let n = find_replace(
        &mut d,
        &json!({"whatToFind": "text", "find": "hello", "replace": "Bye"}),
    )
    .expect("replace");
    assert_eq!(n, 1);
    assert_eq!(texts(&d), vec!["Bye world, Bye"]);
    let mut d = doc();
    find_replace(
        &mut d,
        &json!({"find": "hello", "replace": "Bye", "matchCase": true}),
    )
    .expect("replace");
    assert_eq!(texts(&d), vec!["Hello world, Bye"]);
    let mut d = doc();
    let n = find_replace(
        &mut d,
        &json!({"find": "wor", "replace": "X", "wholeWord": true}),
    )
    .expect("replace");
    assert_eq!(n, 0, "no whole word");
    let mut d = doc();
    find_replace(
        &mut d,
        &json!({"find": "w(or)ld", "replace": "$1", "regex": true}),
    )
    .expect("regex");
    assert_eq!(texts(&d), vec!["Hello or, hello"]);
}

#[test]
fn find_replace_color_font_and_web216() {
    let mut d = doc();
    let n = find_replace(
        &mut d,
        &json!({"whatToFind": "color", "find": "#ff0000", "replace": "#00ff00"}),
    )
    .expect("color");
    assert_eq!(n, 1);
    assert!(fills(&d).contains(&0xff00_ff00) && !fills(&d).contains(&RED));
    let mut d = doc();
    let n = find_replace(
        &mut d,
        &json!({"whatToFind": "font", "find": "arial", "replace": "Verdana"}),
    )
    .expect("font");
    assert_eq!(n, 1);
    assert_eq!(web216([0, 30, 250]), [0, 51, 255]);
    assert_eq!(web216([26, 25, 128]), [51, 0, 153]);
    let mut d = doc();
    find_replace(
        &mut d,
        &json!({"whatToFind": "color", "find": "#ff0000", "replace": "#123456"}),
    )
    .expect("color");
    let n = find_replace(&mut d, &json!({"whatToFind": "nonWeb216"})).expect("web");
    assert!(n >= 1);
    assert!(fills(&d).contains(&0xff00_3366), "{:x?}", fills(&d));
}

#[test]
fn find_replace_step_keeps_document_and_warns_when_nothing_found() {
    let mut item = doc_item(doc());
    let w = apply(
        "document.findReplace",
        json!({"find": "zzz", "replace": "y"}),
        &mut item,
    );
    assert_eq!(w.len(), 1);
    let w = apply(
        "document.findReplace",
        json!({"find": "world", "replace": "there"}),
        &mut item,
    );
    assert!(w.is_empty());
    assert_eq!(texts(the_doc(&item)), vec!["Hello there, hello"]);
}

#[test]
fn layers_only_hides_the_others_and_changes_the_render() {
    let d = doc();
    let red_before = fp_batch::item::render_document(&d).expect("render");
    assert_eq!(red_before.get(5, 25).map(|p| p[0]), Some(255));
    let mut item = doc_item(d);
    let w = apply(
        "document.layers",
        json!({"name": "no such layer", "mode": "only"}),
        &mut item,
    );
    assert_eq!(w.len(), 1, "warning when nothing matches");
    let after = fp_batch::item::render_document(the_doc(&item)).expect("render");
    assert_eq!(after.get(5, 25), Some([255, 255, 255, 255]), "layer hidden");
    // show again by name (layer names are localized: match all)
    let mut d = the_doc(&item).clone();
    let hits = set_layers(&mut d, &|_| true, "show");
    assert!(hits >= 2);
    let again = fp_batch::item::render_document(&d).expect("render");
    assert_eq!(again.get(5, 25).map(|p| p[0]), Some(255));
}

#[test]
fn slices_and_page_names() {
    let d = doc();
    let s = slices_of(&d);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].0, s[0].2.width(), s[0].2.height()), (0, 10, 10));
    assert_eq!(page_name("{name}_{page}", "a", 2, "P", 1), "a_2");
    assert_eq!(page_name("{pageName}-{state}", "a", 2, "Home", 3), "Home-3");
}

fn write_doc(dir: &Path) -> PathBuf {
    let p = dir.join("site.fw.png");
    fp_batch::output::write_fwpng(&doc_item(doc()), &p).expect("fwpng");
    p
}

fn folder_recipe(out: &Path) -> Recipe {
    let mut r = Recipe::named("doc");
    r.output.format = "png".into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

#[test]
fn pages_and_slices_run_end_to_end() {
    let dir = temp("doc-pages");
    let src = write_doc(&dir);
    let other = dir.join("plain.png");
    write_png(&other, 8, 6);
    let list = fp_batch::collect(&[src.clone(), other.clone()], true, false, &[]).items;
    assert_eq!(list[0].kind, ItemKind::Document);
    let out = dir.join("pages");
    let mut r = folder_recipe(&out);
    r.add_step("document.pages", json!({}));
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(dims(&out.join("site_1.png")), (40, 30));
    let p2 = pixels(&out.join("site_2.png"));
    assert_eq!(&p2[..4], &[0, 0, 255, 255], "second page is blue");
    let plain = rep.items.iter().find(|i| i.index == 1).expect("plain");
    assert_eq!(plain.status, ItemStatus::Skipped, "{plain:?}");
    // only page 2
    let out2 = dir.join("pages2");
    let mut r = folder_recipe(&out2);
    r.add_step(
        "document.pages",
        json!({"pages": "2", "pattern": "{name}-p{page}"}),
    );
    executor::run(&r, &list[..1], &RunOptions::default()).expect("run 2");
    assert!(out2.join("site-p2.png").is_file() && !out2.join("site-p1.png").exists());
    // slices
    let out3 = dir.join("slices");
    let mut r = folder_recipe(&out3);
    r.add_step("document.slices", json!({}));
    let rep = executor::run(&r, &list[..1], &RunOptions::default()).expect("slices");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    assert_eq!(dims(&out3.join("site_1.png")), (10, 10));
    assert_eq!(&pixels(&out3.join("site_1.png"))[..4], &[255, 0, 0, 255]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validation_of_the_document_steps() {
    assert_eq!(issues("document.findReplace", json!({})), vec!["find"]);
    assert_eq!(
        issues("document.findReplace", json!({"find": "(", "regex": true})),
        vec!["find"]
    );
    assert_eq!(
        issues(
            "document.findReplace",
            json!({"whatToFind": "color", "find": "red", "replace": "#00ff00"})
        ),
        vec!["find"]
    );
    assert!(issues("document.findReplace", json!({"whatToFind": "nonWeb216"})).is_empty());
    assert_eq!(issues("document.layers", json!({})), vec!["name"]);
    assert_eq!(
        issues("document.pages", json!({"pages": "x-"})),
        vec!["pages"]
    );
}
