//! BATCH-11a: Texte der Auswahlwerte in **einem** Schema (`batch-value-<id>-<param>-<wert>`),
//! Sichtbarkeitsbedingungen der Parameter (`ParamDef::when`) und Vorschau-Maßstab im
//! Schrittkontext (`StepContext::preview_scale`, gesetzt von `pipeline::preview`).

use fp_batch::step::{ParamType, value_key};
use fp_batch::{CancelToken, OutputFormat, Recipe, StepContext, StepRegistry};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Parameter, deren Werte bewusst ohne eigenen Text erscheinen: Formate (Eigennamen), Verhältnisse
/// und Unterabtastung (Zahlen), Befehle des Registers (Text aus `cmd-*`), Endkarte „Dateiname“
/// (eigene Karte von BATCH-3).
fn raw_ok(step: &str, param: &str) -> bool {
    matches!(
        (step, param),
        ("output", "format")
            | ("output", "subsampling")
            | (_, "ratio")
            | ("filter.registry", "command")
    ) || step == "rename"
}

fn locales() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fp-i18n")
        .join("locales")
}

fn ftl_keys(lang: &str) -> Vec<String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(locales().join(lang))
        .expect("Sprachordner")
        .flatten()
    {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.starts_with("batch") && name.ends_with(".ftl")) {
            continue;
        }
        let text = std::fs::read_to_string(e.path()).expect("ftl lesen");
        for l in text.lines() {
            if let Some((k, _)) = l.split_once(" =")
                && !k.starts_with(' ')
                && !k.starts_with('#')
                && !k.is_empty()
            {
                out.push(k.trim().to_string());
            }
        }
    }
    out
}

#[test]
fn jeder_auswahlwert_hat_einen_text_in_de_und_en() {
    let reg = StepRegistry::global();
    let mut missing = Vec::new();
    for lang in ["de", "en"] {
        assert!(fp_i18n::set_language(lang));
        for s in reg.iter() {
            for p in s.params() {
                if raw_ok(s.id(), &p.name) {
                    continue;
                }
                if let ParamType::Enum { values } | ParamType::Multi { values } = &p.ty {
                    for v in values {
                        let k = value_key(s.id(), &p.name, v);
                        if !fp_i18n::has(&k) {
                            missing.push(format!("{lang}: {k}"));
                        }
                    }
                }
            }
        }
    }
    fp_i18n::set_language("de");
    assert!(
        missing.is_empty(),
        "fehlende Texte für Auswahlwerte:\n{}",
        missing.join("\n")
    );
}

#[test]
fn alte_schluesselformen_sind_entfernt() {
    // batch-choice-* (BATCH-6) und Werte als batch-param-<id>-<param>-<wert> (BATCH-1/7/8) gibt es nicht mehr
    let reg = StepRegistry::global();
    let mut old_value_keys = Vec::new();
    for s in reg.iter() {
        for p in s.params() {
            if let ParamType::Enum { values } | ParamType::Multi { values } = &p.ty {
                for v in values {
                    old_value_keys.push(format!(
                        "{}-{v}",
                        fp_batch::step::param_key(s.id(), &p.name)
                    ));
                }
            }
        }
    }
    for lang in ["de", "en"] {
        let keys = ftl_keys(lang);
        let choice: Vec<&String> = keys
            .iter()
            .filter(|k| k.starts_with("batch-choice-"))
            .collect();
        assert!(choice.is_empty(), "{lang}: alte Schlüssel {choice:?}");
        let old: Vec<&String> = keys.iter().filter(|k| old_value_keys.contains(k)).collect();
        assert!(
            old.is_empty(),
            "{lang}: Werte noch als batch-param-…: {old:?}"
        );
        // keine doppelten Schlüssel über die Bereichsdateien hinweg
        let mut sorted = keys.clone();
        sorted.sort();
        let dups: Vec<&String> = sorted
            .windows(2)
            .filter(|w| w[0] == w[1])
            .map(|w| &w[0])
            .collect();
        assert!(dups.is_empty(), "{lang}: doppelte Schlüssel {dups:?}");
    }
}

#[test]
fn katalog_nennt_werte_texte() {
    assert!(fp_i18n::set_language("en"));
    let cat = StepRegistry::global().catalog_json();
    fp_i18n::set_language("de");
    let resize = cat["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "resize")
        .expect("resize");
    let mode = resize["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "mode")
        .expect("mode");
    assert!(
        mode["valueLabels"]["longEdge"]
            .as_str()
            .is_some_and(|s| !s.is_empty()),
        "{mode}"
    );
    let size = resize["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "size")
        .expect("size");
    assert_eq!(size["visibleWhen"]["param"], json!("mode"), "{size}");
}

// ------------------------------------------------------------------ Sichtbarkeitsbedingungen

#[test]
fn bedingungen_zeigen_auf_vorhandene_parameter_und_werte() {
    let reg = StepRegistry::global();
    for s in reg.iter() {
        let defs = s.params();
        for d in &defs {
            let Some(c) = &d.visible_when else { continue };
            let parent = defs.iter().find(|p| p.name == c.param).unwrap_or_else(|| {
                panic!("{}: {} hängt an unbekanntem {}", s.id(), d.name, c.param)
            });
            assert_ne!(
                parent.name,
                d.name,
                "{}: {} hängt an sich selbst",
                s.id(),
                d.name
            );
            let allowed: Vec<String> = match &parent.ty {
                ParamType::Enum { values } => values.clone(),
                ParamType::Bool => vec!["true".into(), "false".into()],
                other => panic!("{}: Bedingung auf {:?}", s.id(), other),
            };
            for v in &c.values {
                assert!(
                    allowed.contains(v),
                    "{}: {} – Wert {v} gibt es bei {} nicht",
                    s.id(),
                    d.name,
                    c.param
                );
            }
            // mit Vorgaben auswerten (keine Endlosschleife)
            let _ = d.is_visible(&defs, &json!({}));
        }
    }
}

fn visible(id: &str, params: Value) -> Vec<String> {
    let reg = StepRegistry::global();
    let defs = reg.get(id).expect("Schritt").params();
    defs.iter()
        .filter(|d| d.is_visible(&defs, &params))
        .map(|d| d.name.clone())
        .collect()
}

#[test]
fn felder_je_modus() {
    // resize: längste Seite → nur „size“; Breite → nur „width“
    let v = visible("resize", json!({}));
    assert!(
        v.contains(&"size".into())
            && !v.contains(&"width".into())
            && !v.contains(&"percent".into()),
        "{v:?}"
    );
    let v = visible("resize", json!({"mode": "width"}));
    assert!(
        v.contains(&"width".into()) && !v.contains(&"height".into()) && !v.contains(&"size".into()),
        "{v:?}"
    );
    let v = visible("resize", json!({"mode": "fit"}));
    assert!(
        v.contains(&"width".into()) && v.contains(&"height".into()),
        "{v:?}"
    );
    let v = visible("resize", json!({"mode": "none"}));
    assert!(
        !v.contains(&"onlyShrink".into()) && v.contains(&"dpi".into()),
        "{v:?}"
    );
    // crop: freies Verhältnis zeigt Breite:Höhe, Rechteck die Koordinaten
    let v = visible("crop", json!({"mode": "aspect", "ratio": "free"}));
    assert!(
        v.contains(&"ratioWidth".into()) && !v.contains(&"x".into()),
        "{v:?}"
    );
    let v = visible("crop", json!({"mode": "rect", "ratio": "free"}));
    assert!(
        !v.contains(&"ratioWidth".into())
            && !v.contains(&"ratio".into())
            && v.contains(&"x".into()),
        "Kette: ratio unsichtbar → ratioWidth auch: {v:?}"
    );
    // Nachzeichnen: Konturverfahren-Werte nur bei potrace
    let v = visible("vector.trace", json!({}));
    assert!(
        !v.contains(&"turdSize".into()) && v.contains(&"filterSpeckle".into()),
        "{v:?}"
    );
    let v = visible("vector.trace", json!({"engine": "potrace"}));
    assert!(
        v.contains(&"turdSize".into()) && !v.contains(&"filterSpeckle".into()),
        "{v:?}"
    );
    // Kästchen als Bedingung
    let v = visible("overlay.text", json!({"outline": true}));
    assert!(v.contains(&"outlineColor".into()), "{v:?}");
    assert!(!visible("overlay.text", json!({})).contains(&"outlineColor".into()));
    // Werte bleiben erhalten, auch wenn ein Feld versteckt ist (Prüfung meldet nichts)
    let reg = StepRegistry::global();
    let issues = reg
        .get("resize")
        .unwrap()
        .validate(&json!({"mode": "width", "size": 800}));
    assert!(issues.is_empty(), "{issues:?}");
}

// ------------------------------------------------------------------ Vorschau-Maßstab

#[test]
fn kontext_maszstab_ausserhalb_und_in_der_vorschau() {
    let info = fp_batch::ItemInfo {
        path: PathBuf::from("x.png"),
        root: None,
        kind: fp_batch::ItemKind::Image,
        format: "png".into(),
        bytes: 0,
        width: 2000,
        height: 1000,
        frames: 1,
        modified: None,
        exif: None,
    };
    let cancel = CancelToken::new();
    let mut ctx = StepContext::new(0, 1, &info, &cancel, OutputFormat::Png);
    ctx.proxy_scale = Some(0.25);
    assert_eq!(
        ctx.preview_scale(),
        Some(1.0),
        "außerhalb der Vorschau immer 1"
    );
    ctx.preview = true;
    assert_eq!(ctx.preview_scale(), Some(0.25));
    ctx.proxy_scale = None;
    assert_eq!(
        ctx.preview_scale(),
        None,
        "unbekannt → Schritte schätzen selbst"
    );
    ctx.proxy_scale = Some(3.0);
    assert_eq!(ctx.preview_scale(), Some(1.0), "nie größer als 1");
}

fn dir(name: &str) -> PathBuf {
    fp_i18n::profile::unique_temp_dir(&format!("batch11a-{name}"))
}

/// PNG: deckendes Rechteck mittig auf transparentem Grund (`w`×`h`).
fn png(path: &Path, w: u32, h: u32) {
    let mut px = vec![0u8; (w * h * 4) as usize];
    for y in h / 4..h * 3 / 4 {
        for x in w / 4..w * 3 / 4 {
            let i = ((y * w + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&[200, 60, 40, 255]);
        }
    }
    let f = std::fs::File::create(path).expect("Datei");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .and_then(|mut wr| wr.write_image_data(&px))
        .expect("PNG");
}

fn recipe(steps: Value) -> Recipe {
    serde_json::from_value(json!({"steps": steps, "output": {"format": "png"}})).expect("Rezept")
}

/// Vorschau und volles Ergebnis: Breite nachher (Proxy) und volle Breite nachher.
fn preview_widths(file: &Path, r: &Recipe) -> (f64, f64, f64) {
    let info = fp_batch::item::probe(file).expect("probe");
    let opts = fp_batch::pipeline::PreviewOptions {
        encode_full: true,
        ..Default::default()
    };
    let res =
        fp_batch::pipeline::preview(r, &info, &StepRegistry::global(), &opts).expect("Vorschau");
    let after = res.after.expect("nachher");
    let scale =
        res.before.width().max(res.before.height()) as f64 / info.width.max(info.height) as f64;
    (after.width() as f64, res.after_size.0 as f64, scale)
}

#[test]
fn rahmen_nach_groessenschritt_stimmt_in_der_vorschau() {
    // Befund BATCH-4/6: ein Größenschritt vor dem Rahmen verfälschte die Schätzung (Rahmen zu dünn)
    let d = dir("border");
    let f = d.join("gross.png");
    png(&f, 2400, 1200);
    let r = recipe(json!([
        {"id": "resize", "params": {"mode": "longEdge", "size": 1200}},
        {"id": "overlay.border", "params": {"width": 60, "unit": "px", "placement": "outer"}}
    ]));
    let (pw, full_w, scale) = preview_widths(&f, &r);
    assert_eq!(full_w, 1320.0, "volles Ergebnis: 1200 + 2 × 60");
    let expect = full_w * scale;
    assert!(
        (pw - expect).abs() <= 2.0,
        "Vorschau {pw} statt ≈ {expect} (Maßstab {scale})"
    );
}

#[test]
fn live_effekt_in_der_vorschau_mitverkleinert() {
    // Befund BATCH-4: Schlagschatten wirkte in der Vorschau großer Bilder relativ größer
    let d = dir("effect");
    let f = d.join("logo.png");
    png(&f, 3000, 1500);
    let r = recipe(
        json!([{"id": "effect.live", "params": {"effect": "dropShadow", "params": {"ShadowDistance": "40", "ShadowBlur": "20"}, "expandCanvas": true}}]),
    );
    let (pw, full_w, scale) = preview_widths(&f, &r);
    // Rechteck füllt nur die Mitte: Leinwand wächst nicht, Schatten liegt innen → gleiche Breite
    assert!(
        (pw - full_w * scale).abs() <= 2.0,
        "Vorschau {pw} vs. {full_w} × {scale}"
    );
    // Logo bis an den Rand: Leinwand wächst um den Schatten, in der Vorschau anteilig
    let f2 = d.join("rand.png");
    {
        let (w, h) = (3000u32, 1500u32);
        let px: Vec<u8> = (0..w * h).flat_map(|_| [30u8, 90, 200, 255]).collect();
        let file = std::fs::File::create(&f2).expect("Datei");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header()
            .and_then(|mut wr| wr.write_image_data(&px))
            .expect("PNG");
    }
    let (pw2, full_w2, scale2) = preview_widths(&f2, &r);
    let grow_full = full_w2 - 3000.0;
    let grow_prev = pw2 - (3000.0 * scale2).round();
    assert!(
        grow_full >= 30.0,
        "Schatten vergrößert die Leinwand: {grow_full}"
    );
    assert!(
        (grow_prev - grow_full * scale2).abs() <= 3.0,
        "Vorschau wächst um {grow_prev} statt ≈ {}",
        grow_full * scale2
    );
}
