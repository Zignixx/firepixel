//! Schritt `vector.trace` „Nachzeichnen → SVG/FW-PNG“ und eingebaute Vorlagen (BATCH-9,
//! SPEC-BATCH §2.5, §3.13): beide Verfahren, Vorgaben, Ausgabe SVG und FW-PNG, Logikfälle §4b.

mod batch4_common;
mod support;

use batch4_common::*;
use fp_batch::executor::{self, RunOptions};
use fp_batch::item::Item;
use fp_batch::recipe::{Recipe, TargetKind, TargetSpec};
use fp_batch::steps::vector::{command_params, encode_svg};
use fp_batch::{Severity, StepRegistry, templates};
use fp_model::{ElementKind, LayerKind, LayerSlot};
use fp_raster::RgbaImage;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Logo 64 × 64: weißer Grund, roter Kreis, blaues Quadrat.
fn logo(transparent: bool) -> RgbaImage {
    image(64, 64, |x, y| {
        let (dx, dy) = (x as i32 - 22, y as i32 - 22);
        if dx * dx + dy * dy <= 14 * 14 {
            [220, 30, 40, 255]
        } else if (38..58).contains(&x) && (36..58).contains(&y) {
            [20, 60, 200, 255]
        } else if transparent {
            [0, 0, 0, 0]
        } else {
            [255, 255, 255, 255]
        }
    })
}

/// Strichzeichnung (schwarz auf weiß): Ring und Balken.
fn line_art() -> RgbaImage {
    image(80, 60, |x, y| {
        let (dx, dy) = (x as i32 - 30, y as i32 - 30);
        let d = dx * dx + dy * dy;
        if (14 * 14..=20 * 20).contains(&d) || ((56..62).contains(&x) && (8..52).contains(&y)) {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    })
}

/// Anteil der Pixel, die (über Weiß gemischt) um mehr als 48 abweichen.
fn mismatch(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()));
    let over_white = |p: [u8; 4]| {
        [0, 1, 2].map(|c| (p[c] as u32 * p[3] as u32 + 255 * (255 - p[3] as u32)) / 255)
    };
    let bad = a
        .data()
        .iter()
        .zip(b.data())
        .filter(|(p, q)| {
            over_white(**p)
                .iter()
                .zip(over_white(**q))
                .any(|(x, y)| x.abs_diff(y) > 48)
        })
        .count();
    bad as f64 / a.data().len() as f64
}

fn doc(item: &Item) -> &fp_model::Document {
    match item {
        Item::Document(d) => &d.doc,
        _ => panic!("Dokument erwartet"),
    }
}

/// (sichtbar, Elementarten) je eigener Ebene der ersten Seite (ohne Web-Ebene).
fn layers(d: &fp_model::Document) -> Vec<(bool, Vec<&'static str>)> {
    d.pages[0]
        .root
        .layers
        .iter()
        .filter_map(|s| match s {
            LayerSlot::Own(l) if l.kind != LayerKind::Web => Some((
                l.is_visible(),
                l.cels
                    .iter()
                    .flat_map(|c| c.elements.iter())
                    .map(|e| match &e.kind {
                        ElementKind::Bitmap(_) => "bitmap",
                        ElementKind::Group(_) => "group",
                        ElementKind::Path(_) => "path",
                        _ => "other",
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn trace_logo_becomes_editable_paths_and_looks_like_the_source() {
    let src = logo(false);
    let mut it = still(src.clone());
    let warn = apply("vector.trace", json!({}), &mut it).expect("trace");
    assert!(warn.is_empty(), "{warn:?}");
    assert_eq!(it.dimensions(), (64, 64));
    let d = doc(&it);
    // ohne „Original behalten“: genau eine Ebene mit einer Gruppe, keine Bitmap
    assert_eq!(layers(d), vec![(true, vec!["group"])]);
    let out = first(&it);
    let m = mismatch(&src, &out);
    assert!(m < 0.03, "Abweichung {m}");
    // Leinwand transparent (Vorgabe), mit transparentCanvas = false wie die Quelle
    assert_eq!(d.pages[0].root.background.0 >> 24, 0);
    let mut it2 = still(src.clone());
    apply(
        "vector.trace",
        json!({"transparentCanvas": false}),
        &mut it2,
    )
    .expect("trace");
    let source_canvas = fp_batch::output::item_document(&still(src.clone()))
        .expect("doc")
        .pages[0]
        .root
        .background;
    assert_eq!(doc(&it2).pages[0].root.background, source_canvas);
}

#[test]
fn keep_original_keeps_a_hidden_bitmap_layer() {
    let src = logo(true);
    let mut plain = still(src.clone());
    apply("vector.trace", json!({}), &mut plain).expect("trace");
    let mut kept = still(src.clone());
    apply("vector.trace", json!({"keepOriginal": true}), &mut kept).expect("trace");
    let l = layers(doc(&kept));
    assert!(l.contains(&(false, vec!["bitmap"])), "{l:?}");
    assert!(l.contains(&(true, vec!["group"])), "{l:?}");
    // die ausgeblendete Bitmap ändert das Ergebnis nicht (Vorschau, flache Formate, SVG)
    assert_eq!(hash(&first(&plain)), hash(&first(&kept)));
    let svg =
        String::from_utf8(encode_svg(&kept, &Default::default()).expect("svg")).expect("utf8");
    assert!(
        svg.contains("<path") && !svg.contains("<image"),
        "nur Pfade"
    );
    // transparente Ränder bleiben transparent
    assert_eq!(px(&first(&plain), 1, 1)[3], 0);
}

#[test]
fn both_engines_and_presets() {
    let art = line_art();
    let mut a = still(art.clone());
    apply(
        "vector.trace",
        json!({"preset": "blackWhite", "engine": "clusters"}),
        &mut a,
    )
    .expect("clusters");
    let mut b = still(art.clone());
    apply(
        "vector.trace",
        json!({"preset": "blackWhite", "engine": "potrace"}),
        &mut b,
    )
    .expect("potrace");
    for it in [&a, &b] {
        let m = mismatch(&art, &first(it));
        assert!(m < 0.04, "Abweichung {m}");
    }
    let svg_a = String::from_utf8(encode_svg(&a, &Default::default()).expect("svg")).expect("utf8");
    let svg_b = String::from_utf8(encode_svg(&b, &Default::default()).expect("svg")).expect("utf8");
    assert_ne!(svg_a, svg_b, "zwei Verfahren, zwei Ergebnisse");
    assert!(svg_b.contains("fill-rule=\"evenodd\""), "Löcher des Rings");
    // Farbe: Vorgaben wirken, einzelne Werte überschreiben
    let mut c = still(logo(false));
    apply("vector.trace", json!({"preset": "photo", "engine": "potrace", "colors": 4, "turdSize": 8, "alphaMax": 0.8}), &mut c).expect("potrace color");
    assert!(mismatch(&logo(false), &first(&c)) < 0.05);
}

#[test]
fn unset_parameters_follow_the_preset() {
    let reg = StepRegistry::global();
    let step = reg.get("vector.trace").expect("registered");
    let p = fp_batch::step::with_defaults(&step.params(), &json!({"preset": "photo"}));
    let c = command_params(&p);
    assert_eq!(c["preset"], "photo");
    assert_eq!(c["engine"], "clusters");
    assert_eq!(c["keepOriginal"], false);
    for k in [
        "colors",
        "filterSpeckle",
        "refine",
        "turdSize",
        "alphaMax",
        "threshold",
    ] {
        assert!(c.get(k).is_none(), "{k} nur, wenn gesetzt");
    }
    let c = command_params(&fp_batch::step::with_defaults(
        &step.params(),
        &json!({"colors": 3, "refine": false, "optTolerance": 0.5}),
    ));
    assert_eq!(
        (
            c["colors"].clone(),
            c["refine"].clone(),
            c["optTolerance"].clone()
        ),
        (json!(3), json!(false), json!(0.5))
    );
}

#[test]
fn validation_of_parameters() {
    assert!(errors("vector.trace", json!({})).is_empty());
    assert!(
        errors(
            "vector.trace",
            json!({"preset": "logo", "engine": "potrace", "alphaMax": 1.0, "turnPolicy": "black"})
        )
        .is_empty()
    );
    for bad in [
        json!({"preset": "comic"}),
        json!({"engine": "magic"}),
        json!({"colors": 999}),
        json!({"alphaMax": 2.0}),
        json!({"turnPolicy": "up"}),
        json!({"keepOriginal": "yes"}),
    ] {
        assert!(!errors("vector.trace", bad.clone()).is_empty(), "{bad}");
    }
}

#[test]
fn repeat_and_retrace_never_duplicate() {
    // §4b: Parameter 50× ändern → ein Schritt im Rezept, Ergebnis = einmal mit dem Endwert
    let mut r = Recipe::named("t");
    let i = r.add_step("vector.trace", json!({}));
    for k in 0..50 {
        r.set_param(i, "colors", json!(2 + k % 7));
        r.set_param(
            i,
            "engine",
            json!(if k % 2 == 0 { "potrace" } else { "clusters" }),
        );
    }
    r.set_param(i, "colors", json!(6));
    r.set_param(i, "engine", json!("potrace"));
    assert_eq!(r.steps.len(), 1);
    let mut a = still(logo(false));
    apply("vector.trace", r.steps[0].params.clone(), &mut a).expect("a");
    let mut b = still(logo(false));
    apply(
        "vector.trace",
        json!({"colors": 6, "engine": "potrace"}),
        &mut b,
    )
    .expect("b");
    assert_eq!(hash(&first(&a)), hash(&first(&b)));
    // dasselbe Element zweimal nachzeichnen: wieder genau eine Ebene mit einer Gruppe
    apply("vector.trace", json!({}), &mut a).expect("again");
    assert_eq!(layers(doc(&a)), vec![(true, vec!["group"])]);
    // gleiches Bild, gleiche Werte → gleiches Ergebnis (deterministisch, auch parallel)
    let (one, many) = single_and_multi(|| {
        let mut it = still(logo(false));
        apply("vector.trace", json!({"engine": "potrace"}), &mut it).expect("t");
        hash(&first(&it))
    });
    assert_eq!(one, many);
}

#[test]
fn animation_uses_first_frame_with_warning_and_empty_image_fails() {
    let mut it = animation(vec![(logo(false), 50), (line_art_64(), 50)]);
    let warn = apply("vector.trace", json!({}), &mut it).expect("trace");
    assert_eq!(warn.len(), 1, "{warn:?}");
    assert!(matches!(it, Item::Document(_)));
    assert!(mismatch(&logo(false), &first(&it)) < 0.03);
    // ganz transparent: nichts nachzuzeichnen → Fehler, Element unverändert
    let empty = image(16, 16, |_, _| [0, 0, 0, 0]);
    let mut it = still(empty.clone());
    assert!(apply("vector.trace", json!({}), &mut it).is_err());
    assert!(matches!(&it, Item::Still(s) if s.image == empty));
}

fn line_art_64() -> RgbaImage {
    image(64, 64, |x, _| {
        if x % 9 < 3 {
            [0, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    })
}

#[test]
fn preview_runs_the_trace() {
    let dir = support::temp("vector-preview");
    let p = dir.join("logo.png");
    std::fs::write(&p, png_bytes(&logo(false))).expect("write");
    let r = templates::recipe("logos-to-svg").expect("template");
    let info = fp_batch::probe(&p).expect("probe");
    let reg = StepRegistry::global();
    let res = fp_batch::pipeline::preview(
        &r,
        &info,
        &reg,
        &fp_batch::PreviewOptions {
            encode_full: true,
            ..Default::default()
        },
    )
    .expect("preview");
    assert_eq!(res.format.name(), "svg");
    assert!(res.bytes_after.is_some_and(|b| b > 100));
    assert!(mismatch(&logo(false), res.after.as_ref().expect("after")) < 0.03);
    let _ = std::fs::remove_dir_all(&dir);
}

fn png_bytes(img: &RgbaImage) -> Vec<u8> {
    fp_batch::output::encode(
        &still(img.clone()),
        fp_batch::OutputFormat::Png,
        &Default::default(),
    )
    .expect("png")
    .0
}

fn run_folder(recipe: &Recipe, src: &[PathBuf], out: &Path) -> fp_batch::Report {
    let mut r = recipe.clone();
    r.output.target = TargetSpec {
        kind: TargetKind::Folder,
        path: Some(out.to_path_buf()),
        ..TargetSpec::default()
    };
    let items = fp_batch::collect(src, false, false, &[]).items;
    executor::run(
        &r,
        &items,
        &RunOptions {
            threads: Some(2),
            ..RunOptions::default()
        },
    )
    .expect("run")
}

#[test]
fn run_writes_svg_and_fwpng() {
    let dir = support::temp("vector-run");
    let src: Vec<PathBuf> = (0..2)
        .map(|k| {
            let p = dir.join(format!("logo {k}.png"));
            std::fs::write(
                &p,
                png_bytes(&if k == 0 { logo(false) } else { logo(true) }),
            )
            .expect("write");
            p
        })
        .collect();
    let sums: Vec<u64> = src.iter().map(|p| support::checksum(p)).collect();
    // SVG (Vorlage)
    let r = templates::recipe("logos-to-svg").expect("template");
    let rep = run_folder(&r, &src, &dir.join("svg"));
    assert_eq!(rep.totals.done, 2, "{:#?}", rep.items);
    for k in 0..2 {
        let svg =
            std::fs::read_to_string(dir.join("svg").join(format!("logo {k}.svg"))).expect("svg");
        assert!(
            svg.starts_with("<?xml") && svg.contains("<svg") && svg.trim_end().ends_with("</svg>")
        );
        assert!(
            svg.contains("<path") && !svg.contains("<image"),
            "Pfade, keine Bitmap"
        );
        assert!(svg.contains("width=\"64\" height=\"64\""));
        assert!(
            svg.contains("fill=\"#dc1e28\"")
                || svg.contains("fill=\"#dc1f28\"")
                || svg.matches("<path").count() >= 2
        );
    }
    // FW-PNG (bearbeitbar): dasselbe Rezept, anderes Ziel → beide Formate
    let mut r2 = r.clone();
    r2.output.format = "fwpng".into();
    r2.set_param(0, "keepOriginal", json!(true));
    let rep = run_folder(&r2, &src, &dir.join("fw"));
    assert_eq!(rep.totals.done, 2, "{:#?}", rep.items);
    let f = dir.join("fw").join("logo 0.fw.png");
    let data = std::fs::read(&f).expect("fw.png");
    let item = fp_batch::item::decode_bytes(&f.to_string_lossy(), &data).expect("decode");
    let l = layers(doc(&item));
    assert!(
        l.contains(&(true, vec!["group"])) && l.contains(&(false, vec!["bitmap"])),
        "{l:?}"
    );
    // Originale unverändert, keine Temporärdateien
    for (k, p) in src.iter().enumerate() {
        assert_eq!(support::checksum(p), sums[k]);
    }
    assert!(!support::has_temp_files(&dir));
    // zweiter Lauf in denselben Ordner: neue Namen, nichts überschrieben
    let rep = run_folder(&r, &src, &dir.join("svg"));
    assert_eq!(rep.totals.done, 2);
    assert!(dir.join("svg").join("logo 0 (2).svg").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn svg_encoder_for_plain_images_embeds_the_bitmap() {
    let svg = String::from_utf8(encode_svg(&still(logo(false)), &Default::default()).expect("svg"))
        .expect("utf8");
    assert!(svg.contains("<image") && svg.contains("data:image/png;base64,"));
    assert!(fp_batch::OutputFormat::Svg.can_encode());
}

// ------------------------------------------------------------------ Vorlagen

#[test]
fn templates_are_complete_valid_and_translated() {
    let reg = StepRegistry::global();
    let ids = templates::ids();
    let all = templates::all();
    assert_eq!(all.len(), ids.len(), "jede Datei lesbar");
    assert!(all.len() >= 10);
    let mut seen = std::collections::HashSet::new();
    for t in &all {
        assert!(seen.insert(t.id), "doppelt {}", t.id);
        for i in t.recipe.validate(&reg) {
            assert!(i.severity != Severity::Error, "{}: {i:?}", t.id);
            assert_ne!(i.code, "unknownStep", "{}: {i:?}", t.id);
        }
        // Parameter jedes Schritts im Schema bekannt (Tippfehler fallen auf)
        for s in &t.recipe.steps {
            let defs = reg.get(&s.id).expect("Schritt").params();
            for k in s
                .params
                .as_object()
                .map(|o| o.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default()
            {
                assert!(defs.iter().any(|d| d.name == k), "{}: {}.{k}", t.id, s.id);
            }
        }
        let de = fp_i18n::with_language("de", || templates::find(t.id).expect("de"));
        let en = fp_i18n::with_language("en", || templates::find(t.id).expect("en"));
        assert!(
            !de.title.is_empty()
                && !en.title.is_empty()
                && !de.description.is_empty()
                && !en.description.is_empty()
        );
        assert!(!de.title.starts_with("batch-template-"));
        assert_eq!(de.recipe.name, de.title, "Rezeptname = Titel");
        assert_eq!(de.recipe.steps, en.recipe.steps, "Sprache ändert nur Texte");
    }
    // jede Datei in data\recipes ist eingebunden
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("recipes");
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .expect("dir")
        .flatten()
        .map(|e| {
            e.file_name()
                .to_string_lossy()
                .trim_end_matches(".fpbatch.json")
                .to_string()
        })
        .collect();
    files.sort();
    let mut list: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    list.sort();
    assert_eq!(files, list);
}

#[test]
fn templates_found_by_id_title_and_host_param() {
    assert_eq!(
        templates::find("logos-to-svg").map(|t| t.id),
        Some("logos-to-svg")
    );
    assert_eq!(
        templates::find("  Logos_To_SVG ").map(|t| t.id),
        Some("logos-to-svg")
    );
    assert_eq!(
        templates::find("web-webp.fpbatch.json").map(|t| t.id),
        Some("web-webp")
    );
    assert_eq!(
        fp_i18n::with_language("en", || templates::find("Fotos für E-Mail").map(|t| t.id)),
        Some("email-photos")
    );
    assert_eq!(
        fp_i18n::with_language("de", || templates::find("Photos for email").map(|t| t.id)),
        Some("email-photos")
    );
    assert!(templates::find("").is_none() && templates::find("does-not-exist").is_none());
    // batch.* nimmt Vorlagennamen; eine vorhandene Datei gleichen Namens hat Vorrang
    let r = fp_batch::host::recipe_from_param(Some(&json!("web-webp"))).expect("template");
    assert_eq!(r.output.format, "webp");
    let dir = support::temp("vector-tplfile");
    let f = dir.join("web-webp");
    let mut own = Recipe::named("own");
    own.output.format = "png".into();
    std::fs::write(&f, own.to_json()).expect("write");
    let r =
        fp_batch::host::recipe_from_param(Some(&Value::String(f.to_string_lossy().to_string())))
            .expect("file");
    assert_eq!(r.output.format, "png");
    let _ = std::fs::remove_dir_all(&dir);
    // Katalog
    let cat = templates::catalog_json();
    assert_eq!(cat["format"], "firepixel-batch-templates");
    assert!(cat["templates"].as_array().is_some_and(|a| {
        a.iter()
            .any(|t| t["id"] == "logos-to-svg" && t["format"] == "svg")
    }));
}

#[test]
fn template_runs_end_to_end() {
    // „Transparenz auf Weiß“ und „Für das Web“ rechnen wirklich
    let dir = support::temp("vector-tplrun");
    let p = dir.join("a.png");
    std::fs::write(&p, png_bytes(&logo(true))).expect("write");
    let rep = run_folder(
        &templates::recipe("transparency-to-white").expect("t"),
        std::slice::from_ref(&p),
        &dir.join("w"),
    );
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    let px0 = support::pixels(&dir.join("w").join("a.png"));
    assert_eq!(&px0[..4], &[255, 255, 255, 255]);
    let rep = run_folder(
        &templates::recipe("web-webp").expect("t"),
        std::slice::from_ref(&p),
        &dir.join("web"),
    );
    assert_eq!(rep.totals.done, 1, "{:#?}", rep.items);
    assert!(dir.join("web").join("a.webp").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
