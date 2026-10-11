//! Import gegen die Ergebnisse des Originals aus der Testrunde 11.10.2026 (`..\re\_testdateien\12`) für die
//! synthetischen Dateien `..\re\import\_scratch\testpsd`.
//! Aufruf: `cargo test -p fp-io --test testfiles -- --ignored --nocapture`.

use std::path::PathBuf;
use std::sync::Arc;

use fp_export::render::{PageRenderer, RenderRequest, SimpleRenderer};
use fp_io::psd::PsdOptions;
use fp_io::ImportOptions;
use fp_model::{Document, Element, ElementKind, LayerKind, LayerSlot};

fn re_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("re")
}

fn read_png(path: &std::path::Path) -> Option<(usize, usize, Vec<[u8; 4]>)> {
    let f = std::fs::File::open(path).ok()?;
    let mut d = png::Decoder::new(std::io::BufReader::new(f));
    d.set_transformations(png::Transformations::normalize_to_color8());
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()?];
    let info = r.next_frame(&mut buf).ok()?;
    let ch = info.color_type.samples();
    let (w, h) = (info.width as usize, info.height as usize);
    let mut px = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let p = &buf[y * info.line_size + x * ch..y * info.line_size + x * ch + ch];
            px.push(match ch {
                1 => [p[0], p[0], p[0], 255],
                2 => [p[0], p[0], p[0], p[1]],
                3 => [p[0], p[1], p[2], 255],
                _ => [p[0], p[1], p[2], p[3]],
            });
        }
    }
    Some((w, h, px))
}

fn describe_el(e: &Element, out: &mut Vec<String>, depth: usize) {
    let kind = match &e.kind {
        ElementKind::Bitmap(b) => format!("Bitmap {}x{}@{},{}", b.image.grid.width, b.image.grid.height, b.image.position().x, b.image.position().y),
        ElementKind::Path(_) => "Pfad".into(),
        ElementKind::Text(_) => "Text".into(),
        ElementKind::Group(g) => format!("Gruppe {:?}", g.kind),
        ElementKind::SubLayer(l) => format!("Unterebene {}", l.name),
        other => format!("{:?}", std::mem::discriminant(other)),
    };
    out.push(format!("{}{} [{}] op={}", "  ".repeat(depth), e.common.name.clone().unwrap_or_default(), kind, e.common.opacity.0));
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
        out.push(format!("Ebene „{}“", l.name));
        if let Some(c) = l.cels.first() {
            for e in &c.elements {
                describe_el(e, &mut out, 1);
            }
        }
    }
    out
}

fn render(doc: &Document) -> Vec<[u8; 4]> {
    let img = SimpleRenderer.render(doc, &RenderRequest::default()).expect("render");
    img.data().to_vec()
}

/// T1–T4 × Vorgabe 0/1: Ebenen/Objekte wie im Original, Pixel gegen den PNG32-Export des Originals.
#[test]
#[ignore]
fn psd_t1_t4_like_original() {
    let src = re_dir().join("import").join("_scratch").join("testpsd");
    let fwd = re_dir().join("_testdateien").join("12");
    let cases = [("T1_drei_pixelebenen", "T1_drei_pixelebenen"), ("T2_fuelldeckkraft", "T2_fuelldeckkraft"), ("T3_schnittmaske_einstellung", "T3_schnittmaske_einstellung"), ("T4_gruppe_einstellung", "T4_gruppe_einstellung")];
    let (mut n, mut struct_ok, mut px_ok) = (0, 0, 0);
    for (psd, fw) in cases {
        let Ok(data) = std::fs::read(src.join(format!("{psd}.psd"))) else {
            eprintln!("{psd}.psd fehlt");
            return;
        };
        for v in [0u8, 1] {
            let opts = ImportOptions { psd: PsdOptions::preset(v, &PsdOptions::default()), ..ImportOptions::default() };
            let ours = fp_io::open_bytes("x.psd", &data, &opts).expect("PSD").document;
            let Ok(fwbytes) = std::fs::read(fwd.join(format!("{fw}_vorgabe{v}.fw.png"))) else { continue };
            let fwdoc = fp_fwpng::read(&fwbytes).expect("FW-Datei");
            let (a, b) = (describe(&ours), describe(&fwdoc));
            let same_struct = a == b;
            n += 1;
            struct_ok += usize::from(same_struct);
            let (w, h, fwpx) = read_png(&fwd.join(format!("{fw}_vorgabe{v}_export.png"))).expect("Export");
            let ourpx = render(&ours);
            let same = ourpx.iter().zip(&fwpx).filter(|(x, y)| x == y).count();
            px_ok += usize::from(same == w * h);
            println!("{psd} Vorgabe {v}: Struktur gleich={same_struct}, Pixel {same}/{}", w * h);
            if !same_struct {
                println!("  Rust: {a:?}\n  FW  : {b:?}");
            }
        }
    }
    println!("PSD T1–T4: Struktur {struct_ok}/{n}, Pixel {px_ok}/{n}");
    assert_eq!(struct_ok, n);
    assert_eq!(px_ok, n);
}

/// T5: sechs Ebeneneffekte, `UseFWFilters` an (nativ) bzw. aus (Photoshop-Live-Effekt).
#[test]
#[ignore]
fn psd_t5_effects() {
    let src = re_dir().join("import").join("_scratch").join("testpsd").join("T5_effekte.psd");
    let Ok(data) = std::fs::read(&src) else {
        eprintln!("T5 fehlt");
        return;
    };
    let fwd = re_dir().join("_testdateien").join("12");
    let effects_of = |doc: &Document| -> Vec<fp_model::Effect> {
        let mut out = Vec::new();
        for slot in &doc.pages[0].root.layers {
            if let LayerSlot::Own(l) = slot
                && let Some(c) = l.cels.first()
            {
                for e in &c.elements {
                    if let Some(fx) = &e.common.effects {
                        out.extend(fx.effects.iter().cloned());
                    }
                }
            }
        }
        out
    };
    let mut total = 0;
    let mut same = 0;
    for (native, file) in [(true, "T5_effekte_nativ.fw.png"), (false, "T5_effekte_pslive.fw.png")] {
        let mut opts = ImportOptions::default();
        opts.psd.native_first = native;
        let ours = effects_of(&fp_io::open_bytes("x.psd", &data, &opts).unwrap().document);
        let fw = effects_of(&fp_fwpng::read(&std::fs::read(fwd.join(file)).unwrap()).unwrap());
        println!("{file}: Rust {} Effekte, FW {}", ours.len(), fw.len());
        for (k, f) in fw.iter().enumerate() {
            let Some(o) = ours.get(k) else {
                println!("  fehlt: {:?}", f.moa_id());
                continue;
            };
            let mut diffs = Vec::new();
            for (key, val) in o.params.entries.iter().map(|e| (e.key.clone(), e.value.clone())) {
                let fv = f.params.get(&key).map(|e| e.value.clone());
                total += 1;
                if fv.as_ref() == Some(&val) {
                    same += 1;
                } else {
                    diffs.push(format!("{key}: Rust {val:?} FW {fv:?}"));
                }
            }
            // Nur schreibgeschützte UI-Namen bzw. die PS-Wörterbuchversion dürfen fehlen.
            let fw_only: Vec<String> = f
                .params
                .entries
                .iter()
                .filter(|e| o.params.get(&e.key).is_none() && e.key != "mkbFile_WriteOnly_TemporaryEffectUiName" && e.key != "Dict")
                .map(|e| format!("{}={:?}", e.key, e.value))
                .collect();
            total += fw_only.len();
            println!("  {} {:?}: {} (nur FW: {fw_only:?})", k, f.moa_id(), if diffs.is_empty() { "alle Parameter gleich".to_string() } else { diffs.join("; ") });
        }
    }
    println!("T5: {same}/{total} Effektparameter gleich");
    assert_eq!(same, total);
}

/// T7: CMYK-TIFF über die CMYK-Tabelle des Originals (Spezifikation 4.096/4.096) und CMYK-JPEG
/// multiplikativ ohne Tabelle (4.096/4.096 mit dem IJG-Dekoder).
#[test]
#[ignore]
fn cmyk_tiff_and_jpeg() {
    let src = re_dir().join("import").join("_scratch").join("testpsd");
    let fwd = re_dir().join("_testdateien").join("12");
    let table = fp_io::pe::load_cmyk_table(&re_dir().join("farbe").join("_scratch").join("cmyk_1001.bin"))
        .or_else(|| {
            let corpus = std::env::var_os("FP_CORPUS_DIR").map(PathBuf::from).unwrap_or_else(|| re_dir().join("..").join("korpus"));
            fp_color::install::cmyk_resource_file(&corpus).and_then(|f| fp_io::pe::load_cmyk_table(&f))
        });
    let Some(table) = table else {
        eprintln!("CMYK-Tabelle fehlt");
        return;
    };
    let opts = ImportOptions { cmyk_table: Some(Arc::new(table)), ..ImportOptions::default() };
    let tif = fp_io::open_bytes("T7.tif", &std::fs::read(src.join("T7_cmyk.tif")).unwrap(), &opts).unwrap();
    let (w, h, fw) = read_png(&fwd.join("T7_cmyk_tif_export.png")).unwrap();
    let ours = render(&tif.document);
    let same = ours.iter().zip(&fw).filter(|(a, b)| a == b).count();
    println!("CMYK-TIFF: {same}/{} Pixel gleich", w * h);
    let LayerSlot::Own(l) = &tif.document.pages[0].root.layers[0] else { panic!() };
    assert_eq!(l.name, "Hintergrund");
    assert_eq!(same, w * h);
    let jpg = fp_io::open_bytes("T7.jpg", &std::fs::read(src.join("T7_cmyk.jpg")).unwrap(), &opts).unwrap();
    let (w2, _, fw2) = read_png(&fwd.join("T7_cmyk_jpg_export.png")).unwrap();
    let ours2 = render(&jpg.document);
    let same2 = ours2.iter().zip(&fw2).filter(|(a, b)| a == b).count();
    let maxd = ours2.iter().zip(&fw2).map(|(a, b)| (0..3).map(|k| a[k].abs_diff(b[k])).max().unwrap_or(0)).max().unwrap_or(0);
    // Farbfelder (je 8×8 oben links): C, M, Y, K, C50M50
    let fields: Vec<[u8; 4]> = (0..5).map(|i| ours2[4 * w2 + 8 * i + 4]).collect();
    let fwf: Vec<[u8; 4]> = (0..5).map(|i| fw2[4 * w2 + 8 * i + 4]).collect();
    println!("CMYK-JPEG: {same2}/{} Pixel gleich (max. Abweichung {maxd}), Farbfelder Rust {fields:?} FW {fwf:?}", fw2.len());
    assert_eq!(same2, fw2.len(), "CMYK-JPEG pixelgenau (multiplikativ, IJG-Dekoder)");
    assert_eq!(fields, fwf);
}
