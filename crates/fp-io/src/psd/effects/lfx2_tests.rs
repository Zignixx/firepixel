//! F150: `lfx2` blocks built byte by byte → records of the PSD live effect container (spec 79 §4, §6).

use super::*;
use crate::psd::reader::effects_descriptor;

/// Value of a hand-built descriptor.
enum V {
    Bool(bool),
    Unit(&'static str, f64),
    Doub(f64),
    Long(i32),
    Enum(&'static str, &'static str),
    Text(&'static str),
    Obj(&'static str, Vec<(&'static str, V)>),
    List(Vec<V>),
}

fn w_key(out: &mut Vec<u8>, k: &str) {
    let n = if k.len() == 4 { 0 } else { k.len() as u32 };
    out.extend_from_slice(&n.to_be_bytes());
    out.extend_from_slice(k.as_bytes());
}

fn w_ustr(out: &mut Vec<u8>, s: &str) {
    let u: Vec<u16> = s.encode_utf16().collect();
    out.extend_from_slice(&(u.len() as u32).to_be_bytes());
    for c in u {
        out.extend_from_slice(&c.to_be_bytes());
    }
}

fn w_desc(out: &mut Vec<u8>, class: &str, items: &[(&str, V)]) {
    w_ustr(out, "");
    w_key(out, class);
    out.extend_from_slice(&(items.len() as u32).to_be_bytes());
    for (k, v) in items {
        w_key(out, k);
        w_value(out, v);
    }
}

fn w_value(out: &mut Vec<u8>, v: &V) {
    match v {
        V::Bool(b) => {
            out.extend_from_slice(b"bool");
            out.push(u8::from(*b));
        }
        V::Unit(u, x) => {
            out.extend_from_slice(b"UntF");
            out.extend_from_slice(u.as_bytes());
            out.extend_from_slice(&x.to_be_bytes());
        }
        V::Doub(x) => {
            out.extend_from_slice(b"doub");
            out.extend_from_slice(&x.to_be_bytes());
        }
        V::Long(x) => {
            out.extend_from_slice(b"long");
            out.extend_from_slice(&x.to_be_bytes());
        }
        V::Enum(t, e) => {
            out.extend_from_slice(b"enum");
            w_key(out, t);
            w_key(out, e);
        }
        V::Text(s) => {
            out.extend_from_slice(b"TEXT");
            w_ustr(out, s);
        }
        V::Obj(c, items) => {
            out.extend_from_slice(b"Objc");
            w_desc(out, c, items);
        }
        V::List(vs) => {
            out.extend_from_slice(b"VlLs");
            out.extend_from_slice(&(vs.len() as u32).to_be_bytes());
            for v in vs {
                w_value(out, v);
            }
        }
    }
}

/// `lfx2` block data: version 0, descriptor version 16, `layerEffects` with `Scl `/`masterFXSwitch`.
fn lfx2(scale: f64, master: bool, effects: Vec<(&'static str, V)>) -> Vec<u8> {
    let mut items = vec![
        ("Scl ", V::Unit("#Prc", scale)),
        ("masterFXSwitch", V::Bool(master)),
    ];
    items.extend(effects);
    let mut out = Vec::new();
    out.extend_from_slice(&0u32.to_be_bytes());
    out.extend_from_slice(&16u32.to_be_bytes());
    w_desc(&mut out, "null", &items);
    out
}

fn import_with(data: &[u8], light: GlobalLight) -> EffectList {
    let fx = effects_descriptor(data).expect("lfx2 descriptor");
    effect_list(&fx, false, light)
}

fn import(data: &[u8]) -> EffectList {
    import_with(data, GlobalLight::default())
}

/// Record `name` of the only container of the list.
fn record(list: &EffectList, name: &str) -> String {
    assert_eq!(list.effects.len(), 1, "one container");
    list.effects[0]
        .get(name)
        .unwrap_or_else(|| panic!("record {name}"))
        .to_string()
}

/// Record of a single effect with the given keys (in an `lfx2` at 100 %, master on).
fn one(key: &'static str, live: &str, items: Vec<(&'static str, V)>) -> String {
    record(
        &import(&lfx2(100.0, true, vec![(key, V::Obj(key, items))])),
        live,
    )
}

fn rgb(r: f64, g: f64, b: f64) -> V {
    V::Obj(
        "RGBC",
        vec![
            ("Rd  ", V::Doub(r)),
            ("Grn ", V::Doub(g)),
            ("Bl  ", V::Doub(b)),
        ],
    )
}

fn field(rec: &str, i: usize) -> &str {
    rec.split('&')
        .nth(i)
        .unwrap_or_else(|| panic!("field {i} of {rec}"))
}

const LIN: &str = CONTOUR_LINEAR;
const BLACK: &str = RGB_BLACK;

#[test]
fn defaults_of_all_ten_effects_with_missing_enab() {
    // Every effect object is present but empty: `enab` missing = enabled, the rest from spec 79 §4.
    let expect: [(&str, &str, String); 10] = [
        (
            "DrSh",
            "dropShadow",
            format!(
                "1&75.000000&0&120.000000&0.000000&5.000000&5.000000&0.000000&0&1&multiply&{BLACK}&{LIN}"
            ),
        ),
        (
            "IrSh",
            "innerShadow",
            format!(
                "1&75.000000&0&-135.000000&0.000000&5.000000&5.000000&0.000000&0&0&multiply&{BLACK}&{LIN}"
            ),
        ),
        (
            "OrGl",
            "outerGlow",
            format!(
                "1&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000&softMatte\
                 &RGBColor;255.000000;255.000000;189.996109&nogradient&{LIN}&"
            ),
        ),
        (
            "IrGl",
            "innerGlow",
            format!(
                "1&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000&softMatte\
                 &RGBColor;255.000000;255.000000;189.996109&nogradient&{LIN}&edgeGlow"
            ),
        ),
        (
            "ebbl",
            "bevelEmboss",
            format!(
                "1&0&0&0&0&75.000000&75.000000&120.000000&30.000000&100.000000&5.000000&0.000000\
                 &screen&{RGB_WHITE}&multiply&{BLACK}&softMatte&innerBevel&stampIn&{LIN}&noshape&notexture"
            ),
        ),
        (
            "ChFX",
            "satin",
            format!(
                "1&0&1&50.000000&19.000000&11.000000&14.000000&multiply&{BLACK}&{CONTOUR_GAUSSIAN}"
            ),
        ),
        (
            "SoFi",
            "solidFill",
            "1&100.000000&normal&RGBColor;255.000000;0.000000;0.000000".to_string(),
        ),
        (
            "GrFl",
            "gradientFill",
            "1&0&1&100.000000&90.000000&100.000000&0.000000&0.000000&normal&linear\
             &Two_Color,customStops,4096.000000$2$2,0$50$userStop$RGBColor;0.000000;0.000000;0.000000,\
             4096$50$userStop$RGBColor;255.000000;255.000000;255.000000,100.000000$0$50,100.000000$4096$50"
                .to_string(),
        ),
        (
            "patternFill",
            "patternFill",
            "1&1&100.000000&100.000000&0.000000&0.000000&normal&Bubbles\
             &b7334da0-122f-11d4-8bb5-e27e45023b5f"
                .to_string(),
        ),
        (
            "FrFX",
            "stroke",
            "1&100.000000&3.000000&normal&RGBColor;255.000000;0.000000;0.000000&outsetFrame&solidColor&"
                .to_string(),
        ),
    ];
    for (key, live, want) in &expect {
        assert_eq!(&one(key, live, vec![]), want, "{key}");
    }
    // all ten in one block: one container with ten records
    let all = import(&lfx2(
        100.0,
        true,
        expect
            .iter()
            .map(|(k, _, _)| (*k, V::Obj(k, vec![])))
            .collect(),
    ));
    for (_, live, want) in &expect {
        assert_eq!(&record(&all, live), want);
    }
}

#[test]
fn one_non_default_value_per_effect() {
    let en = || ("enab", V::Bool(true));
    let ds = one(
        "DrSh",
        "dropShadow",
        vec![en(), ("Opct", V::Unit("#Prc", 60.0))],
    );
    assert_eq!(field(&ds, 1), "60.000000");
    let is = one(
        "IrSh",
        "innerShadow",
        vec![en(), ("lagl", V::Unit("#Ang", 33.0))],
    );
    assert_eq!(field(&is, 3), "33.000000");
    let og = one(
        "OrGl",
        "outerGlow",
        vec![en(), ("Inpr", V::Unit("#Prc", 40.0))],
    );
    assert_eq!(
        field(&og, 3),
        "75.000000:0.000000:5.000000:0.000000:0.000000:40.000000"
    );
    let ig = one(
        "IrGl",
        "innerGlow",
        vec![en(), ("glwS", V::Enum("IGSr", "SrcC"))],
    );
    assert_eq!(field(&ig, 8), "centerGlow");
    let bv = one(
        "ebbl",
        "bevelEmboss",
        vec![en(), ("bvlS", V::Enum("BESl", "Embs"))],
    );
    assert_eq!(field(&bv, 17), "emboss");
    let sa = one("ChFX", "satin", vec![en(), ("Invr", V::Bool(false))]);
    assert_eq!(field(&sa, 2), "0");
    let sf = one(
        "SoFi",
        "solidFill",
        vec![en(), ("Clr ", rgb(0.0, 128.0, 255.0))],
    );
    assert_eq!(field(&sf, 3), "RGBColor;0.000000;128.000000;255.000000");
    let gf = one(
        "GrFl",
        "gradientFill",
        vec![en(), ("Type", V::Enum("GrdT", "Rdl "))],
    );
    assert_eq!(field(&gf, 9), "radial");
    let pf = one(
        "patternFill",
        "patternFill",
        vec![en(), ("Algn", V::Bool(false))],
    );
    assert_eq!(field(&pf, 1), "0");
    let st = one("FrFX", "stroke", vec![en(), ("Sz  ", V::Unit("#Pxl", 7.0))]);
    assert_eq!(field(&st, 2), "7.000000");
}

#[test]
fn key_table_gaps_corner_points_texture_lab_and_long_modes() {
    // `Cnty` false = corner point, `InvT`, Lab `A   `/`B   `, long blend mode IDs (spec 79 §6.2)
    let curve = V::Obj(
        "ShpC",
        vec![
            ("Nm  ", V::Text("Linear")),
            (
                "Crv ",
                V::List(vec![
                    V::Obj("CrPt", vec![("Hrzn", V::Doub(0.0)), ("Vrtc", V::Doub(0.0))]),
                    V::Obj(
                        "CrPt",
                        vec![
                            ("Hrzn", V::Doub(128.0)),
                            ("Vrtc", V::Doub(64.0)),
                            ("Cnty", V::Bool(false)),
                        ],
                    ),
                    V::Obj(
                        "CrPt",
                        vec![("Hrzn", V::Doub(255.0)), ("Vrtc", V::Doub(255.0))],
                    ),
                ]),
            ),
        ],
    );
    let lab = V::Obj(
        "LbCl",
        vec![
            ("Lmnc", V::Doub(50.0)),
            ("A   ", V::Doub(-20.0)),
            ("B   ", V::Doub(30.0)),
        ],
    );
    let ds = one(
        "DrSh",
        "dropShadow",
        vec![
            ("Md  ", V::Enum("BlnM", "linearBurn")),
            ("Clr ", lab),
            ("TrnS", curve),
        ],
    );
    assert_eq!(field(&ds, 10), "linearBurn");
    assert_eq!(field(&ds, 11), "labColor;50.000000;-20.000000;30.000000");
    assert_eq!(
        field(&ds, 12),
        "3$$0.000000;0.000000;1$128.000000;64.000000;0$255.000000;255.000000;1"
    );
    let bv = one(
        "ebbl",
        "bevelEmboss",
        vec![
            ("useTexture", V::Bool(true)),
            ("InvT", V::Bool(true)),
            ("Algn", V::Bool(false)),
            ("textureDepth", V::Unit("#Prc", -50.0)),
            (
                "phase",
                V::Obj("Pnt ", vec![("Hrzn", V::Doub(3.0)), ("Vrtc", V::Doub(4.0))]),
            ),
            (
                "Ptrn",
                V::Obj(
                    "Ptrn",
                    vec![("Nm  ", V::Text("Dots")), ("Idnt", V::Text("id-1"))],
                ),
            ),
        ],
    );
    assert_eq!(
        field(&bv, 21),
        "1,0,100.000000,-50.000000,3.000000,4.000000,Dots,id-1"
    );
    assert_eq!(field(&bv, 3), "1");
    let sf = one("SoFi", "solidFill", vec![("Md  ", V::Enum("BlnM", "H   "))]);
    assert_eq!(field(&sf, 2), "hue");
}

#[test]
fn gradients_noise_and_name() {
    // custom stops: name with non-alphanumeric ASCII characters → `_`
    let grad = V::Obj(
        "Grdn",
        vec![
            ("Nm  ", V::Text("Blue, Red & Co.")),
            ("GrdF", V::Enum("GrdF", "CstS")),
            ("Intr", V::Doub(4096.0)),
            (
                "Clrs",
                V::List(vec![
                    V::Obj(
                        "Clrt",
                        vec![
                            ("Clr ", rgb(0.0, 0.0, 255.0)),
                            ("Type", V::Enum("Clry", "UsrS")),
                            ("Lctn", V::Long(0)),
                            ("Mdpn", V::Long(50)),
                        ],
                    ),
                    V::Obj(
                        "Clrt",
                        vec![
                            ("Type", V::Enum("Clry", "BckC")),
                            ("Lctn", V::Long(4096)),
                            ("Mdpn", V::Long(40)),
                        ],
                    ),
                ]),
            ),
            (
                "Trns",
                V::List(vec![V::Obj(
                    "TrnS",
                    vec![
                        ("Opct", V::Unit("#Prc", 80.0)),
                        ("Lctn", V::Long(100)),
                        ("Mdpn", V::Long(50)),
                    ],
                )]),
            ),
        ],
    );
    let gf = one("GrFl", "gradientFill", vec![("Grad", grad)]);
    assert_eq!(
        field(&gf, 10),
        "Blue__Red___Co_,customStops,4096.000000$2$1,0$50$userStop$RGBColor;0.000000;0.000000;255.000000,\
         4096$40$backgroundColor,80.000000$100$50"
    );
    // noise gradient (spec 79 §3.5)
    let noise = V::Obj(
        "Grdn",
        vec![
            ("Nm  ", V::Text("Noise")),
            ("GrdF", V::Enum("GrdF", "ClNs")),
            ("ShTr", V::Bool(true)),
            ("VctC", V::Bool(false)),
            ("ClrS", V::Enum("ClrS", "HSBl")),
            ("RndS", V::Long(1234)),
            ("Smth", V::Long(3000)),
            (
                "Mnm ",
                V::List(vec![V::Long(1), V::Long(2), V::Long(3), V::Long(4)]),
            ),
            (
                "Mxm ",
                V::List(vec![V::Long(90), V::Long(91), V::Long(92), V::Long(93)]),
            ),
        ],
    );
    let og = one("OrGl", "outerGlow", vec![("Grad", noise)]);
    // a glow with a gradient and no colour keeps `nocolor`
    assert_eq!(field(&og, 5), "nocolor");
    assert_eq!(
        field(&og, 6),
        "Noise,colorNoise,1,0,1234,3000,1,2,3,4,90,91,92,93,HSBColorEnum"
    );
}

#[test]
fn stroke_gradient_and_pattern_sub_records() {
    let st = one(
        "FrFX",
        "stroke",
        vec![
            ("PntT", V::Enum("FrFl", "GrFl")),
            ("Angl", V::Unit("#Ang", 45.0)),
            ("Scl ", V::Unit("#Prc", 80.0)),
        ],
    );
    assert!(
        st.starts_with(
            "1&100.000000&3.000000&normal&RGBColor;255.000000;0.000000;0.000000&outsetFrame\
             &gradientFill&0&1&45.000000&80.000000&0.000000&0.000000&linear&Two_Color,"
        ),
        "{st}"
    );
    let st = one(
        "FrFX",
        "stroke",
        vec![
            ("PntT", V::Enum("FrFl", "Ptrn")),
            ("Scl ", V::Unit("#Prc", 50.0)),
        ],
    );
    assert!(
        st.ends_with(
            "&pattern&1&50.000000&0.000000&0.000000&Bubbles&b7334da0-122f-11d4-8bb5-e27e45023b5f"
        ),
        "{st}"
    );
}

#[test]
fn enab_false_is_kept_and_alone_gives_no_container() {
    let data = lfx2(
        100.0,
        true,
        vec![
            ("DrSh", V::Obj("DrSh", vec![("enab", V::Bool(false))])),
            ("SoFi", V::Obj("SoFi", vec![("enab", V::Bool(true))])),
        ],
    );
    let list = import(&data);
    assert_eq!(field(&record(&list, "dropShadow"), 0), "0");
    assert_eq!(field(&record(&list, "solidFill"), 0), "1");
    let off = lfx2(
        100.0,
        true,
        vec![("DrSh", V::Obj("DrSh", vec![("enab", V::Bool(false))]))],
    );
    assert!(import(&off).effects.is_empty());
}

#[test]
fn scale_200_percent_scales_pixel_values_only() {
    let data = lfx2(
        200.0,
        true,
        vec![
            (
                "DrSh",
                V::Obj(
                    "DrSh",
                    vec![
                        ("Dstn", V::Unit("#Pxl", 4.0)),
                        ("blur", V::Unit("#Pxl", 3.0)),
                        ("Ckmt", V::Unit("#Pxl", 20.0)),
                        ("Opct", V::Unit("#Prc", 50.0)),
                    ],
                ),
            ),
            (
                "OrGl",
                V::Obj(
                    "OrGl",
                    vec![
                        ("blur", V::Unit("#Pxl", 6.0)),
                        ("Ckmt", V::Unit("#Pxl", 10.0)),
                        ("Inpr", V::Unit("#Prc", 40.0)),
                    ],
                ),
            ),
            (
                "ebbl",
                V::Obj(
                    "ebbl",
                    vec![
                        ("Sftn", V::Unit("#Pxl", 2.0)),
                        ("useTexture", V::Bool(true)),
                        ("Scl ", V::Unit("#Prc", 30.0)),
                    ],
                ),
            ),
            (
                "GrFl",
                V::Obj("GrFl", vec![("Scl ", V::Unit("#Prc", 80.0))]),
            ),
            (
                "patternFill",
                V::Obj("patternFill", vec![("Scl ", V::Unit("#Prc", 50.0))]),
            ),
            (
                "FrFX",
                V::Obj(
                    "FrFX",
                    vec![
                        ("Sz  ", V::Unit("#Pxl", 3.0)),
                        ("PntT", V::Enum("FrFl", "Ptrn")),
                        ("Scl ", V::Unit("#Prc", 25.0)),
                    ],
                ),
            ),
        ],
    );
    let list = import(&data);
    let ds = record(&list, "dropShadow");
    assert_eq!(
        (field(&ds, 1), field(&ds, 5), field(&ds, 6), field(&ds, 7)),
        ("50.000000", "8.000000", "6.000000", "20.000000")
    );
    assert_eq!(
        field(&record(&list, "outerGlow"), 3),
        "75.000000:10.000000:12.000000:0.000000:0.000000:40.000000"
    );
    let bv = record(&list, "bevelEmboss");
    assert_eq!(field(&bv, 10), "5.000000"); // missing: default, not scaled
    assert_eq!(field(&bv, 11), "4.000000");
    assert!(field(&bv, 21).starts_with("0,1,60.000000,"));
    assert_eq!(field(&record(&list, "gradientFill"), 5), "80.000000");
    assert_eq!(field(&record(&list, "patternFill"), 3), "100.000000");
    let st = record(&list, "stroke");
    assert_eq!(field(&st, 2), "6.000000");
    assert_eq!(field(&st, 8), "50.000000");
}

#[test]
fn master_switch_off_imports_the_effects_switched_off() {
    let data = lfx2(
        100.0,
        false,
        vec![("DrSh", V::Obj("DrSh", vec![("enab", V::Bool(true))]))],
    );
    let list = import(&data);
    assert_eq!(list.effects.len(), 1);
    assert!(!list.effects[0].is_visible());
    assert_eq!(field(&record(&list, "dropShadow"), 0), "1");
    let on = import(&lfx2(100.0, true, vec![("DrSh", V::Obj("DrSh", vec![]))]));
    assert!(on.effects[0].is_visible());
}

#[test]
fn global_angle_and_altitude_for_uglg() {
    let data = lfx2(
        100.0,
        true,
        vec![
            (
                "DrSh",
                V::Obj(
                    "DrSh",
                    vec![("uglg", V::Bool(true)), ("lagl", V::Unit("#Ang", 120.0))],
                ),
            ),
            (
                "IrSh",
                V::Obj(
                    "IrSh",
                    vec![("uglg", V::Bool(false)), ("lagl", V::Unit("#Ang", 10.0))],
                ),
            ),
            ("ebbl", V::Obj("ebbl", vec![("uglg", V::Bool(true))])),
        ],
    );
    let light = GlobalLight {
        angle: Some(45.0),
        altitude: Some(60.0),
    };
    let list = import_with(&data, light);
    let ds = record(&list, "dropShadow");
    assert_eq!((field(&ds, 2), field(&ds, 3)), ("1", "45.000000"));
    assert_eq!(field(&record(&list, "innerShadow"), 3), "10.000000");
    let bv = record(&list, "bevelEmboss");
    assert_eq!((field(&bv, 7), field(&bv, 8)), ("45.000000", "60.000000"));
    // without the resources the effect's own values stay
    let ds = record(&import(&data), "dropShadow");
    assert_eq!(field(&ds, 3), "120.000000");
}

#[test]
fn multi_instance_effects_take_the_first_enabled_entry() {
    let fx = effects_descriptor(&lfx2(
        100.0,
        true,
        vec![(
            "dropShadowMulti",
            V::List(vec![
                V::Obj(
                    "DrSh",
                    vec![("enab", V::Bool(false)), ("Opct", V::Unit("#Prc", 10.0))],
                ),
                V::Obj(
                    "DrSh",
                    vec![("enab", V::Bool(true)), ("Opct", V::Unit("#Prc", 20.0))],
                ),
            ]),
        )],
    ))
    .expect("descriptor");
    assert_eq!(multi_losses(&fx), vec!["dropShadow"]);
    let list = effect_list(&fx, false, GlobalLight::default());
    let ds = record(&list, "dropShadow");
    assert_eq!((field(&ds, 0), field(&ds, 1)), ("1", "20.000000"));
}
