//! Additional layer information blocks of the PSD export (BA-102), written from the public PSD
//! format description: action descriptors, layer effects (`lfx2`), type layers (`TySh` with
//! EngineData), vector masks (`vmsk`), solid colour fill (`SoCo`) and group sections (`lsct`).

use fp_color::Argb;
use fp_model::{Contour, Effect, EffectKind};

/// A descriptor value.
#[derive(Clone, Debug, PartialEq)]
pub enum DescValue {
    /// Object: class ID and items.
    Obj(&'static str, Vec<(&'static str, DescValue)>),
    Bool(bool),
    Double(f64),
    /// Unit (`#Prc`, `#Pxl`, `#Ang`, `#Pnt`) and value.
    Unit(&'static str, f64),
    /// Enum type and value.
    Enum(&'static str, &'static str),
    Text(String),
    Long(i32),
    List(Vec<DescValue>),
    /// Raw data (`tdta`).
    Data(Vec<u8>),
}

fn w_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

/// Key: 4-character IDs with length 0, longer string IDs with their length.
fn w_key(out: &mut Vec<u8>, k: &str) {
    if k.len() == 4 {
        w_u32(out, 0);
    } else {
        w_u32(out, k.len() as u32);
    }
    out.extend_from_slice(k.as_bytes());
}

/// Unicode string (length in UTF-16 units including the terminating zero).
fn w_ustr(out: &mut Vec<u8>, s: &str) {
    let u: Vec<u16> = s.encode_utf16().collect();
    w_u32(out, u.len() as u32 + 1);
    for c in u {
        out.extend_from_slice(&c.to_be_bytes());
    }
    out.extend_from_slice(&[0, 0]);
}

fn type_tag(v: &DescValue) -> &'static [u8; 4] {
    match v {
        DescValue::Obj(..) => b"Objc",
        DescValue::Bool(_) => b"bool",
        DescValue::Double(_) => b"doub",
        DescValue::Unit(..) => b"UntF",
        DescValue::Enum(..) => b"enum",
        DescValue::Text(_) => b"TEXT",
        DescValue::Long(_) => b"long",
        DescValue::List(_) => b"VlLs",
        DescValue::Data(_) => b"tdta",
    }
}

fn w_value(out: &mut Vec<u8>, v: &DescValue) {
    match v {
        DescValue::Obj(class, items) => w_desc_body(out, class, items),
        DescValue::Bool(b) => out.push(*b as u8),
        DescValue::Double(d) => out.extend_from_slice(&d.to_be_bytes()),
        DescValue::Unit(u, d) => {
            out.extend_from_slice(&u.as_bytes()[..4]);
            out.extend_from_slice(&d.to_be_bytes());
        }
        DescValue::Enum(t, e) => {
            w_key(out, t);
            w_key(out, e);
        }
        DescValue::Text(s) => w_ustr(out, s),
        DescValue::Long(l) => out.extend_from_slice(&l.to_be_bytes()),
        DescValue::List(vs) => {
            w_u32(out, vs.len() as u32);
            for x in vs {
                out.extend_from_slice(type_tag(x));
                w_value(out, x);
            }
        }
        DescValue::Data(d) => {
            w_u32(out, d.len() as u32);
            out.extend_from_slice(d);
        }
    }
}

fn w_desc_body(out: &mut Vec<u8>, class: &str, items: &[(&'static str, DescValue)]) {
    w_ustr(out, "");
    w_key(out, class);
    w_u32(out, items.len() as u32);
    for (k, v) in items {
        w_key(out, k);
        out.extend_from_slice(type_tag(v));
        w_value(out, v);
    }
}

/// Descriptor with version 16 in front.
pub fn descriptor(class: &str, items: &[(&'static str, DescValue)]) -> Vec<u8> {
    let mut out = Vec::new();
    w_u32(&mut out, 16);
    w_desc_body(&mut out, class, items);
    out
}

fn rgb(c: Argb) -> DescValue {
    DescValue::Obj(
        "RGBC",
        vec![
            ("Rd  ", DescValue::Double(((c.0 >> 16) & 0xFF) as f64)),
            ("Grn ", DescValue::Double(((c.0 >> 8) & 0xFF) as f64)),
            ("Bl  ", DescValue::Double((c.0 & 0xFF) as f64)),
        ],
    )
}

fn alpha_pct(c: Argb) -> f64 {
    ((c.0 >> 24) as f64 / 2.55).round().clamp(0.0, 100.0)
}

fn linear_contour() -> DescValue {
    let pt = |v: f64| {
        DescValue::Obj(
            "CrPt",
            vec![
                ("Hrzn", DescValue::Double(v)),
                ("Vrtc", DescValue::Double(v)),
            ],
        )
    };
    DescValue::Obj(
        "ShpC",
        vec![
            ("Nm  ", DescValue::Text("Linear".into())),
            ("Crv ", DescValue::List(vec![pt(0.0), pt(255.0)])),
        ],
    )
}

/// Blend number of the model → enum value of the descriptor.
fn blend_enum(bldi: i64) -> &'static str {
    match bldi {
        2 => "Mltp",
        3 => "Dfrn",
        4 => "Ovrl",
        5 => "Scrn",
        7 => "Dslv",
        8 => "CBrn",
        9 => "Drkn",
        10 => "Lghn",
        11 => "H   ",
        12 => "Strt",
        13 => "Clr ",
        14 => "Lmns",
        _ => "Nrml",
    }
}

fn num(e: &Effect, k: &str, d: f64) -> f64 {
    e.get_f64(k).filter(|v| v.is_finite()).unwrap_or(d)
}

fn color(e: &Effect, k: &str, d: u32) -> Argb {
    e.get_color(k).unwrap_or(Argb(d))
}

/// Normalises an angle to -180..180.
fn angle(a: f64) -> f64 {
    let mut a = a % 360.0;
    if a > 180.0 {
        a -= 360.0;
    }
    if a <= -180.0 {
        a += 360.0;
    }
    a
}

fn shadow(e: &Effect, class: &'static str, drop: bool) -> DescValue {
    let c = color(e, "ShadowColor", 0xFF00_0000);
    let mut items = vec![
        ("enab", DescValue::Bool(true)),
        ("Md  ", DescValue::Enum("BlnM", "Mltp")),
        ("Clr ", rgb(c)),
        ("Opct", DescValue::Unit("#Prc", alpha_pct(c))),
        ("uglg", DescValue::Bool(false)),
        (
            "lagl",
            // the model stores the direction of the shadow, PSD the light angle
            DescValue::Unit("#Ang", angle(num(e, "ShadowAngle", 315.0) - 180.0)),
        ),
        (
            "Dstn",
            DescValue::Unit("#Pxl", num(e, "ShadowDistance", 7.0).max(0.0)),
        ),
        ("Ckmt", DescValue::Unit("#Pxl", 0.0)),
        (
            "blur",
            DescValue::Unit("#Pxl", num(e, "ShadowBlur", 4.0).max(0.0)),
        ),
        ("Nose", DescValue::Unit("#Prc", 0.0)),
        ("AntA", DescValue::Bool(false)),
        ("TrnS", linear_contour()),
    ];
    if drop {
        items.push(("layerConceals", DescValue::Bool(true)));
    }
    DescValue::Obj(class, items)
}

fn glow(e: &Effect, outer: bool) -> DescValue {
    let c = color(e, "OuterBevelColor", 0xFFFF_FFBE);
    let softness = num(e, "MaskSoftness", 5.0).clamp(0.0, 20.0);
    let choke = (100.0 - softness * 5.0).clamp(0.0, 100.0);
    let spread = (100.0 - choke) / 100.0;
    let width = num(e, "GlowWidth", 5.0).max(0.0);
    let k = if outer { 0.9 } else { 1.0 };
    let size = if spread > 0.0 {
        width / (spread * k)
    } else {
        width
    };
    let mut items = vec![
        ("enab", DescValue::Bool(true)),
        ("Md  ", DescValue::Enum("BlnM", "Scrn")),
        ("Clr ", rgb(c)),
        (
            "Opct",
            DescValue::Unit("#Prc", num(e, "BevelContrast", 75.0).clamp(0.0, 100.0)),
        ),
        (
            "GlwT",
            DescValue::Enum(
                "BETE",
                if num(e, "SlopeType", 0.0) == 1.0 {
                    "SfBL"
                } else {
                    "PrBL"
                },
            ),
        ),
        ("Ckmt", DescValue::Unit("#Pxl", choke)),
        ("blur", DescValue::Unit("#Pxl", size.round().min(250.0))),
        ("Nose", DescValue::Unit("#Prc", 0.0)),
        ("ShdN", DescValue::Unit("#Prc", 0.0)),
        ("AntA", DescValue::Bool(false)),
        ("TrnS", linear_contour()),
        ("Inpr", DescValue::Unit("#Prc", 50.0)),
    ];
    if !outer {
        items.push(("glwS", DescValue::Enum("IGSr", "SrcE")));
    }
    DescValue::Obj(if outer { "OrGl" } else { "IrGl" }, items)
}

fn bevel(e: &Effect, style: &'static str) -> DescValue {
    let hi = color(e, "HiliteColor", 0xFFFF_FFFF);
    let sh = color(e, "ShadowColor", 0xFF00_0000);
    let (hio, sho) = (alpha_pct(hi), alpha_pct(sh));
    let t = (0.25 * hio + 0.75 * sho).trunc().max(1.0);
    let contrast = num(e, "BevelContrast", 75.0);
    let depth = (contrast * 700.0 / t - 250.0).clamp(1.0, 1000.0);
    DescValue::Obj(
        "ebbl",
        vec![
            ("enab", DescValue::Bool(true)),
            ("hglM", DescValue::Enum("BlnM", "Scrn")),
            ("hglC", rgb(hi)),
            ("hglO", DescValue::Unit("#Prc", hio)),
            ("sdwM", DescValue::Enum("BlnM", "Mltp")),
            ("sdwC", rgb(sh)),
            ("sdwO", DescValue::Unit("#Prc", sho)),
            (
                "bvlT",
                DescValue::Enum(
                    "bvlT",
                    if num(e, "SlopeType", 0.0) == 1.0 {
                        "SfBL"
                    } else {
                        "PrBL"
                    },
                ),
            ),
            ("bvlS", DescValue::Enum("BESl", style)),
            ("uglg", DescValue::Bool(false)),
            (
                "lagl",
                DescValue::Unit("#Ang", angle(num(e, "LightAngle", 135.0))),
            ),
            ("Lald", DescValue::Unit("#Ang", 30.0)),
            ("srgR", DescValue::Unit("#Prc", depth.round())),
            (
                "blur",
                DescValue::Unit("#Pxl", num(e, "BevelWidth", 10.0).max(0.0)),
            ),
            ("bvlD", DescValue::Enum("BESs", "In  ")),
            ("TrnS", linear_contour()),
            ("antialiasGloss", DescValue::Bool(false)),
            (
                "Sftn",
                DescValue::Unit("#Pxl", num(e, "AngleSoftness", 0.0).clamp(0.0, 16.0)),
            ),
            ("useShape", DescValue::Bool(false)),
            ("useTexture", DescValue::Bool(false)),
        ],
    )
}

fn color_fill(e: &Effect) -> DescValue {
    let c = color(e, "Color", 0xFFFF_0000);
    DescValue::Obj(
        "SoFi",
        vec![
            ("enab", DescValue::Bool(true)),
            (
                "Md  ",
                DescValue::Enum("BlnM", blend_enum(num(e, "Blendmode", 0.0) as i64)),
            ),
            (
                "Opct",
                DescValue::Unit("#Prc", num(e, "Opacity", 100.0).clamp(0.0, 100.0)),
            ),
            ("Clr ", rgb(c)),
        ],
    )
}

/// Layer effects block (`lfx2`) of the live effects that PSD has natively: drop shadow, inner
/// shadow, outer glow, inner glow, bevel/emboss and colour fill (one of each kind; the model has
/// no stroke effect). Returns the block data and the indices of the effects it contains; the
/// rest stays baked into the pixels. `None` when no effect can be mapped.
pub fn effects_block(effects: &[Effect]) -> Option<(Vec<u8>, Vec<usize>)> {
    let mut used: Vec<usize> = Vec::new();
    let mut items: Vec<(&'static str, DescValue)> = vec![
        ("Scl ", DescValue::Unit("#Prc", 100.0)),
        ("masterFXSwitch", DescValue::Bool(true)),
    ];
    let mut taken: Vec<&'static str> = Vec::new();
    for (i, e) in effects.iter().enumerate() {
        if !e.is_visible() {
            continue;
        }
        let entry = match e.kind() {
            EffectKind::DropShadow => Some(("DrSh", shadow(e, "DrSh", true))),
            EffectKind::InnerShadow => Some(("IrSh", shadow(e, "IrSh", false))),
            EffectKind::InnerGlow => Some(("IrGl", glow(e, false))),
            EffectKind::ColorFill => Some(("SoFi", color_fill(e))),
            EffectKind::BevelFamily => match num(e, "BevelType", 0.0) as i64 {
                0 => Some(("ebbl", bevel(e, "InrB"))),
                1 => Some(("ebbl", bevel(e, "OtrB"))),
                2 => Some(("ebbl", bevel(e, "Embs"))),
                3 => Some(("ebbl", bevel(e, "PlEb"))),
                4 => Some(("OrGl", glow(e, true))),
                _ => None,
            },
            _ => None,
        };
        if let Some((key, v)) = entry
            && !taken.contains(&key)
        {
            taken.push(key);
            items.push((key, v));
            used.push(i);
        }
    }
    if used.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    w_u32(&mut out, 0);
    out.extend(descriptor("null", &items));
    Some((out, used))
}

/// String of the EngineData: UTF-16BE with byte order mark, `(`, `)` and `\` escaped.
fn engine_string(s: &str) -> Vec<u8> {
    let mut out = vec![b'(', 0xFE, 0xFF];
    for u in s.encode_utf16() {
        for b in u.to_be_bytes() {
            if matches!(b, b'(' | b')' | b'\\') {
                out.push(b'\\');
            }
            out.push(b);
        }
    }
    out.push(b')');
    out
}

/// A type layer's content.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeLayer {
    /// Text; paragraphs separated by `\r`.
    pub text: String,
    /// PostScript name of the font.
    pub font: String,
    /// Size in px (= pt at 72 ppi).
    pub size: f64,
    pub color: Argb,
    /// 0 left, 1 right, 2 centre, 3 justified.
    pub justification: u8,
    /// Transformation xx xy yx yy tx ty (tx/ty = origin of the first baseline, layer pixels).
    pub transform: [f64; 6],
    /// Bounds of the text relative to the origin (left, top, right, bottom).
    pub bounds: [f64; 4],
}

fn engine_data(t: &TypeLayer) -> Vec<u8> {
    let mut text = t.text.replace("\r\n", "\r").replace('\n', "\r");
    if !text.ends_with('\r') {
        text.push('\r');
    }
    let n = text.encode_utf16().count();
    let c = t.color.0;
    let ch = |s: u32| ((c >> s) & 0xFF) as f64 / 255.0;
    let fill = format!(
        "<< /Type 1 /Values [ {:.5} {:.5} {:.5} {:.5} ] >>",
        ch(24),
        ch(16),
        ch(8),
        ch(0)
    );
    let mut out: Vec<u8> = Vec::new();
    let mut s = |x: &str| out.extend_from_slice(x.as_bytes());
    s("\n\n<<\n\t/EngineDict\n\t<<\n\t\t/Editor\n\t\t<<\n\t\t\t/Text ");
    let mut out2 = out;
    out2.extend(engine_string(&text));
    let mut out = out2;
    let mut s = |x: &str| out.extend_from_slice(x.as_bytes());
    s(&format!(
        "\n\t\t>>\n\t\t/ParagraphRun\n\t\t<<\n\t\t\t/DefaultRunData\n\t\t\t<<\n\t\t\t\t/ParagraphSheet << /DefaultStyleSheet 0 /Properties << >> >>\n\t\t\t\t/Adjustments << /Axis [ 1.0 0.0 1.0 ] /XY [ 0.0 0.0 ] >>\n\t\t\t>>\n\t\t\t/RunArray [ << /ParagraphSheet << /DefaultStyleSheet 0 /Properties << /Justification {} >> >> /Adjustments << /Axis [ 1.0 0.0 1.0 ] /XY [ 0.0 0.0 ] >> >> ]\n\t\t\t/RunLengthArray [ {n} ]\n\t\t\t/IsJoinable 1\n\t\t>>\n",
        t.justification
    ));
    s(&format!(
        "\t\t/StyleRun\n\t\t<<\n\t\t\t/DefaultRunData << /StyleSheet << /StyleSheetData << >> >> >>\n\t\t\t/RunArray [ << /StyleSheet << /StyleSheetData << /Font 0 /FontSize {:.4} /AutoKerning true /Kerning 0 /FillColor {fill} >> >> >> ]\n\t\t\t/RunLengthArray [ {n} ]\n\t\t\t/IsJoinable 2\n\t\t>>\n",
        t.size
    ));
    s(
        "\t\t/GridInfo << /GridIsOn false /ShowGrid false /GridSize 18.0 /GridLeading 22.0 /GridColor << /Type 1 /Values [ 0.0 0.0 0.0 1.0 ] >> /GridLeadingFillColor << /Type 1 /Values [ 0.0 0.0 0.0 1.0 ] >> /AlignLineHeightToGridFlags false >>\n",
    );
    s("\t\t/AntiAlias 4\n\t\t/UseFractionalGlyphWidths true\n");
    s(
        "\t\t/Rendered << /Version 1 /Shapes << /WritingDirection 0 /Children [ << /ShapeType 0 /Procession 0 /Lines << /WritingDirection 0 /Children [ ] >> /Cookie << /Photoshop << /ShapeType 0 /PointBase [ 0.0 0.0 ] /Base << /ShapeType 0 /TransformPoint0 [ 1.0 0.0 ] /TransformPoint1 [ 0.0 1.0 ] /TransformPoint2 [ 0.0 0.0 ] >> >> >> >> ] >> >>\n\t>>\n",
    );
    s("\t/ResourceDict\n\t<<\n\t\t/FontSet [ << /Name ");
    out.extend(engine_string(&t.font));
    let mut s = |x: &str| out.extend_from_slice(x.as_bytes());
    s(
        " /Script 0 /FontType 0 /Synthetic 0 >> ]\n\t\t/SuperscriptSize 0.583\n\t\t/SuperscriptPosition 0.333\n\t\t/SubscriptSize 0.583\n\t\t/SubscriptPosition 0.333\n\t\t/SmallCapSize 0.7\n\t>>\n>>\n",
    );
    out
}

/// Type tool block (`TySh`).
pub fn type_block(t: &TypeLayer) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1u16.to_be_bytes());
    for v in t.transform {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out.extend_from_slice(&50u16.to_be_bytes());
    let mut text = t.text.replace("\r\n", "\r").replace('\n', "\r");
    if !text.ends_with('\r') {
        text.push('\r');
    }
    let bounds = |c: &'static str| {
        DescValue::Obj(
            c,
            vec![
                ("Left", DescValue::Unit("#Pnt", t.bounds[0])),
                ("Top ", DescValue::Unit("#Pnt", t.bounds[1])),
                ("Rght", DescValue::Unit("#Pnt", t.bounds[2])),
                ("Btom", DescValue::Unit("#Pnt", t.bounds[3])),
            ],
        )
    };
    out.extend(descriptor(
        "TxLr",
        &[
            ("Txt ", DescValue::Text(text)),
            ("textGridding", DescValue::Enum("textGridding", "None")),
            ("Ornt", DescValue::Enum("Ornt", "Hrzn")),
            ("AntA", DescValue::Enum("Annt", "AnCr")),
            ("bounds", bounds("bounds")),
            ("boundingBox", bounds("boundingBox")),
            ("TextIndex", DescValue::Long(0)),
            ("EngineData", DescValue::Data(engine_data(t))),
        ],
    ));
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend(descriptor(
        "warp",
        &[
            ("warpStyle", DescValue::Enum("warpStyle", "warpNone")),
            ("warpValue", DescValue::Double(0.0)),
            ("warpPerspective", DescValue::Double(0.0)),
            ("warpPerspectiveOther", DescValue::Double(0.0)),
            ("warpRotate", DescValue::Enum("Ornt", "Hrzn")),
        ],
    ));
    // left, top, right, bottom (unused by readers)
    out.extend_from_slice(&[0u8; 16]);
    out
}

/// Solid colour fill block (`SoCo`).
pub fn solid_color_block(c: Argb) -> Vec<u8> {
    descriptor("null", &[("Clr ", rgb(c))])
}

fn fixed(v: f64) -> [u8; 4] {
    ((v * (1u32 << 24) as f64)
        .round()
        .clamp(i32::MIN as f64, i32::MAX as f64) as i32)
        .to_be_bytes()
}

/// Vector mask block (`vmsk`) of contours in document pixels; `origin` = canvas top left,
/// `w`/`h` = canvas size (path points are stored relative to it).
pub fn vector_mask_block(contours: &[Contour], origin: (f64, f64), w: f64, h: f64) -> Vec<u8> {
    let mut out = Vec::new();
    w_u32(&mut out, 3);
    w_u32(&mut out, 0);
    let mut rec = |sel: u16, body: &[u8]| {
        let mut r = [0u8; 26];
        r[..2].copy_from_slice(&sel.to_be_bytes());
        r[2..2 + body.len()].copy_from_slice(body);
        out.extend_from_slice(&r);
    };
    rec(6, &[]);
    rec(8, &[0, 0]);
    let (w, h) = (w.max(1.0), h.max(1.0));
    for c in contours.iter().filter(|c| !c.nodes.is_empty()) {
        let n = c.nodes.len().min(u16::MAX as usize) as u16;
        rec(if c.closed { 0 } else { 3 }, &n.to_be_bytes());
        for nd in c.nodes.iter().take(n as usize) {
            let mut body = Vec::with_capacity(24);
            for p in [nd.pred, nd.anchor, nd.succ] {
                body.extend_from_slice(&fixed((p.y as f64 - origin.1) / h));
                body.extend_from_slice(&fixed((p.x as f64 - origin.0) / w));
            }
            rec(if c.closed { 2 } else { 5 }, &body);
        }
    }
    out
}

/// Section divider block (`lsct`): 1 open group, 3 end of group.
pub fn section_block(kind: u32) -> Vec<u8> {
    kind.to_be_bytes().to_vec()
}
