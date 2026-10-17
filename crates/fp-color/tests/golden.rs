//! Golden-Tests gegen die Python-Referenzen (`re\farbe\farbmodelle.py`, `farbfelddateien.py`,
//! `re\vektor\blend.py`, `stroke.py`, `re\effekte\fx.py`). Testvektoren erzeugt
//! `tools\golden\fp-color\gen_color.py`; große Wertebereiche als CRC-32 über dieselbe Byte-Folge.

use fp_color::blend::{
    OverVariant, behind, blend_pixel, blend_rgb, interpolation_curve, orig_over, orig_over_mmx,
    over_exact, over_place_cs5, overlap_alpha, stamp_opacity_roundtrip,
};
use fp_color::cmyk::{CmykTable, cmyk_to_rgb};
use fp_color::models::*;
use fp_color::swatches::*;
use fp_color::{BlendMode, Opacity, Rgba8};
use serde_json::Value;

fn load() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/color_golden.json");
    let text =
        std::fs::read_to_string(path).expect("color_golden.json fehlt (gen_color.py ausführen)");
    serde_json::from_str(&text).expect("JSON")
}

fn u(v: &Value) -> u64 {
    v.as_u64().expect("Zahl")
}

fn arr(v: &Value) -> &Vec<Value> {
    v.as_array().expect("Liste")
}

fn bytes3(v: &Value) -> (u8, u8, u8) {
    (u(&v[0]) as u8, u(&v[1]) as u8, u(&v[2]) as u8)
}

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xFF) as u8
    }
}

fn crc(f: impl FnOnce(&mut crc32fast::Hasher)) -> u64 {
    let mut h = crc32fast::Hasher::new();
    f(&mut h);
    h.finalize() as u64
}

/// CMYK-Tabelle der Installation (nur lesen): `FP_CMYK_TABLE` oder `..\re\farbe\_scratch\cmyk_1001.bin`.
fn cmyk_table() -> Option<CmykTable> {
    let path = std::env::var("FP_CMYK_TABLE").unwrap_or_else(|_| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../re/farbe/_scratch/cmyk_1001.bin"
        )
        .into()
    });
    let data = std::fs::read(&path).ok()?;
    CmykTable::from_bytes(&data).ok()
}

#[test]
fn models_grid() {
    let d = load();
    let g: Vec<u8> = (0..=255u8).step_by(5).collect();
    let v = crc(|h| {
        for &r in &g {
            for &gg in &g {
                for &b in &g {
                    let (h8, s8, v8) = rgb_to_hsb8(r, gg, b);
                    let mh = rgb_to_mixer_hsb(r, gg, b);
                    let info = info_panel_values(255, r, gg, b, 3);
                    let iv: Vec<i32> = info[..3].iter().map(|s| s.parse().unwrap()).collect();
                    let vals = [
                        hue_degrees(r, gg, b),
                        h8,
                        s8,
                        v8,
                        mh.0,
                        mh.1,
                        mh.2,
                        gray_level(r, gg, b) as i32,
                        rgb_to_mixer_k(r, gg, b),
                        rgb_to_gray_slider(r, gg, b),
                        iv[0],
                        iv[1],
                        iv[2],
                    ];
                    for x in vals {
                        h.update(&x.to_le_bytes());
                    }
                }
            }
        }
    });
    assert_eq!(v, u(&d["models_grid_crc"]), "Farbmodelle Raster 52³");
}

#[test]
fn models_inverse() {
    let d = load();
    let v = crc(|h| {
        for hh in -2..363 {
            for s in (0..=100).step_by(5) {
                for b in (0..=100).step_by(5) {
                    let (r, g, bb) = mixer_hsb_to_rgb(hh as f64, s as f64, b as f64);
                    h.update(&[r, g, bb]);
                }
            }
        }
    });
    assert_eq!(v, u(&d["mixer_hsb_to_rgb_crc"]), "mixer_hsb_to_rgb");
    let v = crc(|h| {
        for hh in -5..366i64 {
            for s in (0..=255i64).step_by(15) {
                for vv in (0..=255i64).step_by(15) {
                    let (r, g, b) = hsb_deg_to_rgb(hh, s, vv);
                    for x in [r, g, b] {
                        h.update(&(x as i32).to_le_bytes());
                    }
                }
            }
        }
    });
    assert_eq!(v, u(&d["hsb_deg_to_rgb_crc"]), "hsb_deg_to_rgb");
    let v = crc(|h| {
        for hh in -3..259 {
            for s in (0..=255).step_by(15) {
                for vv in (0..=255).step_by(15) {
                    let (r, g, b) = hsb8_to_rgb(hh, s, vv);
                    h.update(&[r, g, b]);
                }
            }
        }
    });
    assert_eq!(v, u(&d["hsb8_to_rgb_crc"]), "hsb8_to_rgb");
    for case in arr(&d["mixer_k_to_rgb"]) {
        let k = case[0].as_f64().unwrap();
        assert_eq!(mixer_k_to_rgb(k), bytes3(&case[1]), "K {k}");
    }
}

#[test]
fn gamma_and_bit_depth() {
    let d = load();
    for case in arr(&d["gamma"]) {
        let t = gamma_preview_table(case[0].as_f64().unwrap(), case[1].as_f64().unwrap());
        let r: Vec<u8> = arr(&case[2]).iter().map(|v| u(v) as u8).collect();
        assert_eq!(t.to_vec(), r, "Gamma {:?}", (&case[0], &case[1]));
    }
    for (v, row) in arr(&d["to16_exakt"]).iter().enumerate() {
        for k in 0..4 {
            assert_eq!(to16_float(v as u8, k) as u64, u(&row[k]));
        }
    }
    let c = crc(|h| {
        for v in 0..65536u32 {
            h.update(&[to8_float(v)]);
        }
    });
    assert_eq!(c, u(&d["to8_exakt_crc"]));
}

#[test]
fn info_panel_and_hex() {
    let d = load();
    for case in arr(&d["info_panel"]) {
        let p = &case[0];
        let (a, r, g, b) = (
            u(&p[0]) as u8,
            u(&p[1]) as u8,
            u(&p[2]) as u8,
            u(&p[3]) as u8,
        );
        for m in 0..6 {
            let want: Vec<String> = arr(&case[1][m])
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect();
            assert_eq!(
                info_panel_values(a, r, g, b, m as i64).to_vec(),
                want,
                "Info {m} {:?}",
                p
            );
        }
        let hex: Vec<String> = arr(&case[2])
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        assert_eq!(rgb_to_hex_text(r, g, b).to_vec(), hex);
    }
}

#[test]
fn cmyk_exact() {
    let d = load();
    for case in arr(&d["cmyk_no_table"]) {
        let v = &case[0];
        assert_eq!(
            cmyk_to_rgb(
                u(&v[0]) as u8,
                u(&v[1]) as u8,
                u(&v[2]) as u8,
                u(&v[3]) as u8,
                None
            ),
            bytes3(&case[1])
        );
    }
    let Some(tab) = cmyk_table() else {
        eprintln!("CMYK-Tabelle nicht gefunden (FP_CMYK_TABLE setzen) – Tabellenteil übersprungen");
        return;
    };
    let g: Vec<u8> = (0..=255u8).step_by(15).collect();
    let c = crc(|h| {
        for &cc in &g {
            for &m in &g {
                for &y in &g {
                    for &k in &g {
                        let (r, gg, b) = cmyk_to_rgb(cc, m, y, k, Some(&tab));
                        h.update(&[r, gg, b]);
                    }
                }
            }
        }
    });
    assert_eq!(c, u(&d["cmyk_grid_crc"]), "CMYK-Raster 18⁴ = 104.976");
    for case in arr(&d["cmyk_samples"]) {
        let v = &case[0];
        let got = cmyk_to_rgb(
            u(&v[0]) as u8,
            u(&v[1]) as u8,
            u(&v[2]) as u8,
            u(&v[3]) as u8,
            Some(&tab),
        );
        assert_eq!(got, bytes3(&case[1]), "CMYK {:?}", v);
    }
    // Laufzeitbefund 14: C50/M50 → (84,94,153), C100 → (0,131,215), K100 → (24,21,18)
    assert_eq!(cmyk_to_rgb(128, 128, 0, 0, Some(&tab)), (84, 94, 153));
    assert_eq!(cmyk_to_rgb(255, 0, 0, 0, Some(&tab)), (0, 131, 215));
    assert_eq!(cmyk_to_rgb(0, 0, 0, 255, Some(&tab)), (24, 21, 18));
}

fn modes() -> Vec<BlendMode> {
    BlendMode::ALL.iter().map(|(m, _)| *m).collect()
}

#[test]
fn blend_channel_tables() {
    let d = load();
    let mut ok = 0;
    for m in modes() {
        let c = crc(|h| {
            for s in 0..=255u8 {
                for dd in 0..=255u8 {
                    let r = blend_rgb(m, [s, dd, s ^ dd], [dd, s, s.wrapping_add(dd)]);
                    h.update(&r);
                }
            }
        });
        let want = u(&d["blend_rgb_crc"][m.0.to_string()]);
        if c == want {
            ok += 1;
        } else {
            eprintln!("Mischfarbe abweichend: Modus {} ({:?})", m.0, m.name());
        }
    }
    assert_eq!(
        ok, 46,
        "Mischfarben B(s,d): {ok} von 46 Modi über alle 65.536 Paare gleich"
    );
}

/// Stichprobe: Ziel, Quelle, Deckkraft, Maske, Ergebnis.
type Sample = (Rgba8, Rgba8, u8, u8, Rgba8);

fn pixel_crc(mode: BlendMode, n: u64, seed: u64, variant: OverVariant) -> (u64, Vec<Sample>) {
    let mut rng = Lcg(seed);
    let mut first = Vec::new();
    let c = crc(|h| {
        for i in 0..n {
            let mut dst = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
            let mut src = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
            match rng.next() % 8 {
                0 => dst[3] = 0,
                1 => dst[3] = 255,
                _ => {}
            }
            match rng.next() % 8 {
                0 => src[3] = 0,
                1 => src[3] = 255,
                _ => {}
            }
            let mut op = rng.byte();
            if rng.next().is_multiple_of(3) {
                op = 255;
            }
            let mut m = rng.byte();
            if rng.next().is_multiple_of(2) {
                m = 255;
            }
            let (dp, sp) = (Rgba8::from_array(dst), Rgba8::from_array(src));
            let r = blend_pixel(dp, sp, mode, op, m, variant);
            h.update(&r.to_array());
            if i < 8 {
                first.push((dp, sp, op, m, r));
            }
        }
    });
    (c, first)
}

#[test]
fn blend_pixels() {
    let d = load();
    let n = u(&d["blend_pixel_n"]);
    let mut ok = 0;
    let mut cases: Vec<(String, BlendMode, OverVariant)> = modes()
        .into_iter()
        .map(|m| (m.0.to_string(), m, OverVariant::Mmx))
        .collect();
    cases.push(("0_scalar".into(), BlendMode::NORMAL, OverVariant::Scalar));
    let total = cases.len();
    for (key, m, var) in cases {
        let (c, first) = pixel_crc(m, n, 1000 + m.0 as u64, var);
        let want = &d["blend_pixel"][&key];
        if c == u(&want["crc"]) {
            ok += 1;
            continue;
        }
        for (i, f) in arr(&want["first"]).iter().enumerate() {
            let r = Rgba8::from_array([0, 1, 2, 3].map(|k| u(&f[4][k]) as u8));
            if first[i].4 != r {
                eprintln!(
                    "Modus {key}: {:?} statt {:?} (dst {:?} src {:?} op {} m {})",
                    first[i].4, r, first[i].0, first[i].1, first[i].2, first[i].3
                );
            }
        }
    }
    assert_eq!(
        ok, total,
        "blend_pixel: {ok} von {total} Modusläufen (je {n} Stichproben) gleich"
    );
}

#[test]
fn fx_image_blends() {
    let d = load();
    let n = u(&d["fx_n"]);
    let mut rng = Lcg(5);
    let mut pairs = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let mut t = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
        let mut b = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
        match rng.next() % 6 {
            0 => t[3] = 0,
            1 => t[3] = 255,
            _ => {}
        }
        match rng.next() % 6 {
            0 => b[3] = 0,
            1 => b[3] = 255,
            _ => {}
        }
        pairs.push((Rgba8::from_array(t), Rgba8::from_array(b)));
    }
    type F = fn(Rgba8, Rgba8) -> Rgba8;
    let fns: [(&str, F); 5] = [
        ("over", orig_over),
        ("over_mmx", orig_over_mmx),
        ("behind", behind),
        ("over_exact", over_exact),
        ("over_place", over_place_cs5),
    ];
    for (name, f) in fns {
        let c = crc(|h| {
            for (t, b) in &pairs {
                h.update(&f(*t, *b).to_array());
            }
        });
        if c != u(&d["fx"][name]["crc"]) {
            for (i, row) in arr(&d["fx"][name]["first"]).iter().enumerate() {
                let want = Rgba8::from_array([0, 1, 2, 3].map(|k| u(&row[k]) as u8));
                eprintln!("{name} #{i}: {:?} / {:?}", f(pairs[i].0, pairs[i].1), want);
            }
            panic!("fx {name}: CRC weicht ab");
        }
    }
}

#[test]
fn opacity_overlap_curve() {
    let d = load();
    for (p, v) in arr(&d["opacity_byte"]).iter().enumerate() {
        assert_eq!(Opacity::from_permille(p as u32).0 as u64, u(v), "OPAi {p}");
    }
    let c: Vec<i64> = arr(&d["interpolation_curve"])
        .iter()
        .map(|v| v.as_i64().unwrap())
        .collect();
    assert_eq!(
        interpolation_curve()
            .iter()
            .map(|&x| x as i64)
            .collect::<Vec<_>>(),
        c
    );
    for row in arr(&d["overlap_alpha"]) {
        let n = u(&row[0]) as u32;
        for (k, v) in arr(&row[1]).iter().enumerate() {
            assert_eq!(
                overlap_alpha(n, (k * 5) as u8) as u64,
                u(v),
                "T[{n}][{}]",
                k * 5
            );
        }
    }
    for (o, v) in arr(&d["stamp_promille"]).iter().enumerate() {
        assert_eq!(stamp_opacity_roundtrip(o as u8) as u64, u(v));
    }
}

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn swatch_files() {
    let d = load();
    let sw = &d["swatches"];
    let mut rng = Lcg(9);
    let pal_rand: Vec<Rgba8> = (0..300)
        .map(|_| Rgba8::rgb(rng.byte(), rng.byte(), rng.byte()))
        .collect();
    let pal_gray: Vec<Rgba8> = (0..=255u8).rev().map(|v| Rgba8::rgb(v, v, v)).collect();
    let pal_small = vec![
        Rgba8::rgb(255, 0, 0),
        Rgba8::rgb(0, 128, 255),
        Rgba8::rgb(1, 2, 3),
        Rgba8::rgb(254, 255, 0),
        Rgba8::rgb(17, 17, 17),
    ];
    for (name, pal) in [
        ("small", &pal_small),
        ("gray", &pal_gray),
        ("random", &pal_rand),
    ] {
        let act = write_act_original(pal);
        let ase = write_ase_original(pal);
        let w = &sw[name];
        assert_eq!(act.len() as u64, u(&w["act_len"]));
        assert_eq!(crc32fast::hash(&act) as u64, u(&w["act_crc"]), "ACT {name}");
        assert_eq!(ase.len() as u64, u(&w["ase_len"]));
        assert_eq!(
            crc32fast::hash(&ase) as u64,
            u(&w["ase_crc"]),
            "ASE {name} byte-genau"
        );
        assert!(w["ase_roundtrip"].as_bool().unwrap());
        assert_eq!(
            &ase_to_original_swatches(&read_ase(&ase).unwrap(), None),
            pal
        );
    }
    assert_eq!(
        write_act_original(&pal_small),
        hex(sw["small"]["act_hex"].as_str().unwrap())
    );
    assert_eq!(
        write_ase_original(&pal_small),
        hex(sw["small"]["ase_hex"].as_str().unwrap())
    );
    let mut ps: Vec<u8> = (0u8..48).collect();
    ps.resize(768, 0);
    ps.extend_from_slice(&[0, 16, 0xFF, 0xFF]);
    let fw = read_act_original(&ps).unwrap();
    assert_eq!(fw.len() as u64, u(&sw["photoshop_772"]["original_len"]));
    assert_eq!(fw[256], Rgba8::rgb(0, 16, 255));
    assert_eq!(
        read_act_photoshop(&ps).unwrap().len() as u64,
        u(&sw["photoshop_772"]["photoshop_len"])
    );

    let general = hex(sw["general_ase_hex"].as_str().unwrap());
    let ents = read_ase(&general).unwrap();
    let want = arr(&sw["general_ase_read"]);
    assert_eq!(ents.len(), want.len());
    for (e, w) in ents.iter().zip(want) {
        assert_eq!(e.name, w["name"].as_str().unwrap());
        let model = match e.model {
            AseModel::Rgb => "RGB ",
            AseModel::Cmyk => "CMYK",
            AseModel::Lab => "LAB ",
            AseModel::Gray => "Gray",
            AseModel::Other(_) => "?",
        };
        assert_eq!(model, w["model"].as_str().unwrap());
        let vals: Vec<f32> = arr(&w["values"])
            .iter()
            .map(|v| v.as_f64().unwrap() as f32)
            .collect();
        assert_eq!(e.values, vals);
        assert_eq!(e.color_type.map(|t| t as u64), w["type"].as_u64());
        assert_eq!(e.group.as_deref(), w["group"].as_str());
    }
    let nt: Vec<(u8, u8, u8)> = arr(&sw["general_ase_original_no_table"])
        .iter()
        .map(bytes3)
        .collect();
    let got: Vec<(u8, u8, u8)> = ase_to_original_swatches(&ents, None)
        .iter()
        .map(|c| (c.r, c.g, c.b))
        .collect();
    assert_eq!(got, nt);
    if let Some(tab) = cmyk_table() {
        let wt: Vec<(u8, u8, u8)> = arr(&sw["general_ase_original"])
            .iter()
            .map(bytes3)
            .collect();
        let got: Vec<(u8, u8, u8)> = ase_to_original_swatches(&ents, Some(&tab))
            .iter()
            .map(|c| (c.r, c.g, c.b))
            .collect();
        assert_eq!(got, wt);
    }
}
