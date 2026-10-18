//! Geometry, colour and optimize steps (B-3, `docs/BATCH.md` §3.3, §3.4, §3.11): perspective,
//! tiles, noise, LUT, style, JPEG optimization, web-safe colours — on small synthetic images.

mod support;

use fp_batch::item::{Item, ItemInfo, ItemKind, still_from_rgba};
use fp_batch::recipe::{TargetKind, TargetSpec};
use fp_batch::steps::pixel::{parse_cube, perspective, tile_rects};
use fp_batch::{
    CancelToken, OutputFormat, Recipe, RunOptions, Severity, StepContext, StepRegistry, executor,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

fn info(w: u32, h: u32) -> ItemInfo {
    ItemInfo {
        path: PathBuf::from("C:/in/a.png"),
        root: None,
        kind: ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: w,
        height: h,
        frames: 1,
        modified: None,
        exif: None,
    }
}

fn run_step(
    id: &str,
    params: Value,
    item: &mut Item,
    fmt: OutputFormat,
) -> fp_batch::Result<Vec<String>> {
    let reg = StepRegistry::global();
    let step = reg.get(id).expect("step");
    let p = fp_batch::step::with_defaults(&step.params(), &params);
    let (w, h) = item.dimensions();
    let i = info(w, h);
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &i, &cancel, fmt);
    step.apply(item, &p, &mut ctx)?;
    Ok(ctx.warnings)
}

fn apply(id: &str, params: Value, item: &mut Item) -> Vec<String> {
    run_step(id, params, item, OutputFormat::Png).expect("apply")
}

fn px(item: &Item) -> Vec<[u8; 4]> {
    item.preview_image().expect("image").data().to_vec()
}

fn grad(w: u32, h: u32) -> Item {
    still_from_rgba(w, h, &rgba(w, h, false)).expect("item")
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
fn perspective_identity_and_half_quad_are_exact() {
    let mut item = grad(20, 10);
    let before = px(&item);
    apply("perspective", json!({}), &mut item);
    assert_eq!(item.dimensions(), (20, 10));
    assert_eq!(px(&item), before, "identity");
    let mut half = grad(20, 10);
    apply("perspective", json!({"tlX": 50, "blX": 50}), &mut half);
    assert_eq!(half.dimensions(), (10, 10));
    let got = px(&half);
    for y in 0..10 {
        for x in 0..10 {
            assert_eq!(got[y * 10 + x], before[y * 20 + 10 + x], "{x},{y}");
        }
    }
    // a real trapezoid keeps the size with `keep` and stays inside the colour range
    let mut t = grad(20, 10);
    let img = t.preview_image().expect("img");
    let out = perspective(
        &img,
        [(4.0, 0.0), (16.0, 0.0), (20.0, 10.0), (0.0, 10.0)],
        20,
        10,
    )
    .expect("warp");
    assert_eq!((out.width(), out.height()), (20, 10));
    apply(
        "perspective",
        json!({"tlX": 20, "trX": 80, "size": "keep"}),
        &mut t,
    );
    assert_eq!(t.dimensions(), (20, 10));
}

#[test]
fn tile_rects_grid_and_size() {
    let r = tile_rects(5, 3, Some((2, 2)), (0, 0));
    let sizes: Vec<(usize, usize, u32, u32)> = r
        .iter()
        .map(|(row, col, x)| (*row, *col, x.width(), x.height()))
        .collect();
    assert_eq!(
        sizes,
        vec![(1, 1, 2, 1), (1, 2, 3, 1), (2, 1, 2, 2), (2, 2, 3, 2)]
    );
    let r = tile_rects(5, 3, None, (2, 2));
    assert_eq!(r.len(), 6);
    assert_eq!((r[5].2.width(), r[5].2.height()), (1, 1));
}

fn folder_recipe(out: &Path, format: &str) -> Recipe {
    let mut r = Recipe::named("px");
    r.output.format = format.into();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    r
}

#[test]
fn tile_split_writes_one_file_per_tile() {
    let dir = temp("tiles");
    let src = dir.join("pic.png");
    write_png(&src, 8, 6);
    let out = dir.join("out");
    let mut r = folder_recipe(&out, "png");
    r.add_step("tile.split", json!({"columns": 2, "rows": 3}));
    let list = fp_batch::collect(std::slice::from_ref(&src), true, false, &[]).items;
    let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    for row in 1..=3 {
        for col in 1..=2 {
            assert_eq!(dims(&out.join(format!("pic_{row}_{col}.png"))), (4, 2));
        }
    }
    let src_px = pixels(&src);
    let tile = pixels(&out.join("pic_2_2.png"));
    assert_eq!(&tile[..4], &src_px[(2 * 8 + 4) * 4..(2 * 8 + 4) * 4 + 4]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn noise_is_reproducible_and_median_removes_specks() {
    let mut a = grad(16, 16);
    let mut b = grad(16, 16);
    let before = px(&a);
    apply("filter.noise", json!({"amount": 30, "seed": 7}), &mut a);
    apply("filter.noise", json!({"amount": 30, "seed": 7}), &mut b);
    assert_eq!(px(&a), px(&b), "same seed, same noise");
    assert_ne!(px(&a), before);
    assert!(px(&a).iter().all(|p| p[3] == 255), "alpha unchanged");
    let mut mono = grad(16, 16);
    apply(
        "filter.noise",
        json!({"amount": 50, "monochrome": true, "distribution": "uniform"}),
        &mut mono,
    );
    let changed = px(&mono)
        .iter()
        .zip(&before)
        .filter(|(n, o)| n[0] as i16 - o[0] as i16 != n[2] as i16 - o[2] as i16)
        .count();
    assert!(
        changed < 40,
        "monochrome: mostly the same offset ({changed})"
    );
    // a single white speck on black disappears with the median
    let mut data = vec![0u8; 9 * 9 * 4];
    for p in data.chunks_mut(4) {
        p[3] = 255;
    }
    data[(4 * 9 + 4) * 4..(4 * 9 + 4) * 4 + 3].copy_from_slice(&[255, 255, 255]);
    let mut speck = still_from_rgba(9, 9, &data).expect("item");
    apply(
        "filter.noise",
        json!({"mode": "remove", "radius": 1}),
        &mut speck,
    );
    assert!(px(&speck).iter().all(|p| p[..3] == [0, 0, 0]));
}

fn cube(invert: bool) -> String {
    let mut s = String::from("TITLE \"t\"\nLUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                let f = |v: i32| if invert { 1 - v } else { v };
                s.push_str(&format!("{} {} {}\n", f(r), f(g), f(b)));
            }
        }
    }
    s
}

#[test]
fn lut_identity_invert_and_strength() {
    let dir = temp("lut");
    let id = dir.join("id.cube");
    let inv = dir.join("inv.cube");
    std::fs::write(&id, cube(false)).expect("cube");
    std::fs::write(&inv, cube(true)).expect("cube");
    let mut a = grad(8, 8);
    let before = px(&a);
    apply("adjust.lut", json!({"file": id.to_string_lossy()}), &mut a);
    assert_eq!(px(&a), before);
    let mut b = grad(8, 8);
    apply("adjust.lut", json!({"file": inv.to_string_lossy()}), &mut b);
    for (n, o) in px(&b).iter().zip(&before) {
        for c in 0..3 {
            assert!((n[c] as i16 - (255 - o[c] as i16)).abs() <= 1);
        }
    }
    let mut c = grad(8, 8);
    apply(
        "adjust.lut",
        json!({"file": inv.to_string_lossy(), "strength": 50}),
        &mut c,
    );
    assert!(
        px(&c).iter().all(|p| (p[2] as i16 - 128).abs() <= 1),
        "half way to 127"
    );
    assert!(parse_cube("LUT_1D_SIZE 4\n").is_err());
    assert!(parse_cube("LUT_3D_SIZE 2\n0 0 0\n").is_err());
    assert_eq!(issues("adjust.lut", json!({})), vec!["file"]);
    assert_eq!(
        issues(
            "adjust.lut",
            json!({"file": dir.join("missing.cube").to_string_lossy()})
        ),
        vec!["file"]
    );
    assert!(issues("adjust.lut", json!({"file": id.to_string_lossy()})).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn style_needs_a_name_and_an_existing_style() {
    assert_eq!(issues("effect.style", json!({})), vec!["name"]);
    let mut a = grad(4, 4);
    let err = run_step(
        "effect.style",
        json!({"name": "Nope"}),
        &mut a,
        OutputFormat::Png,
    )
    .expect_err("missing style");
    assert!(err.to_string().contains("Nope"), "{err}");
}

#[test]
fn optimize_jpeg_sets_quality_and_warns_for_other_formats() {
    let mut a = grad(8, 8);
    let w = run_step("optimize.jpeg", json!({}), &mut a, OutputFormat::Png).expect("png");
    assert_eq!(w.len(), 1);
    let dir = temp("optjpeg");
    let src = dir.join("photo.png");
    write_png(&src, 64, 48);
    let list = fp_batch::collect(std::slice::from_ref(&src), true, false, &[]).items;
    let mut sizes = Vec::new();
    for q in [20, 95] {
        let out = dir.join(format!("q{q}"));
        let mut r = folder_recipe(&out, "jpeg");
        r.add_step("optimize.jpeg", json!({"quality": q, "subsampling": "444"}));
        let rep = executor::run(&r, &list, &RunOptions::default()).expect("run");
        assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
        sizes.push(std::fs::metadata(out.join("photo.jpg")).expect("jpg").len());
    }
    assert!(sizes[0] < sizes[1], "{sizes:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn web_safe_snaps_to_multiples_of_51() {
    for dither in ["none", "ordered"] {
        let mut a = grad(16, 16);
        apply("optimize.webSafe", json!({"dither": dither}), &mut a);
        assert!(
            px(&a)
                .iter()
                .all(|p| p[..3].iter().all(|c| c % 51 == 0) && p[3] == 255),
            "{dither}"
        );
    }
}

#[test]
fn style_from_the_document_recolors_a_rectangle() {
    use fp_model::{Argb, Document, Element, LayerSlot, PathAttrs, Rect, Style};
    use std::sync::Arc;
    let mut d = Document::new(10.0, 10.0, Argb(0xffff_ffff));
    let rect = Element::new_rectangle(
        &mut d.ids,
        Rect {
            left: 0.0,
            top: 0.0,
            right: 10.0,
            bottom: 10.0,
        },
        0.0,
        PathAttrs::solid_fill(Argb(0xffff_0000)),
    );
    {
        let p = Arc::make_mut(&mut d.pages[0]);
        let LayerSlot::Own(l) = &mut p.root.layers[0] else {
            panic!("layer");
        };
        Arc::make_mut(&mut Arc::make_mut(l).cels[0])
            .elements
            .push(Arc::new(rect));
    }
    let mut st = Style::new("Green");
    st.attrs = Some(Arc::new(PathAttrs::solid_fill(Argb(0xff00_ff00))));
    d.styles = Some(vec![Arc::new(st)]);
    let mut item = Item::Document(fp_batch::item::DocItem {
        doc: Arc::new(d),
        meta: Default::default(),
    });
    let before = fp_batch::item::render_document(match &item {
        Item::Document(d) => &d.doc,
        _ => panic!("doc"),
    })
    .expect("render");
    assert_eq!(before.get(5, 5), Some([255, 0, 0, 255]));
    apply("effect.style", json!({"name": "Green"}), &mut item);
    let Item::Document(d) = &item else {
        panic!("stays a document");
    };
    let after = fp_batch::item::render_document(&d.doc).expect("render");
    assert_eq!(after.get(5, 5), Some([0, 255, 0, 255]));
}
