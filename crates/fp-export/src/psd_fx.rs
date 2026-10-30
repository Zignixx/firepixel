//! PSD live effect container → editable PSD layer effects (`lfx2`), spec 79 §3–§5 (F149).
//!
//! The PSD live effect container `{8eeadf50-…}` keeps its ten effects as `&`-separated records
//! of PSD descriptors in text form (05 §9). The export turns every record of the container
//! (disabled ones included, `enab` false) into the matching `lfx2` entry with the keys, types,
//! units and defaults of spec 79 §4; the entries are written in the order of §3.2.
//!
//! Tables copied word for word from spec 79 (§3.2 order, §3.4 enums, §3.5 sub-objects); the
//! import side (`fp-io`) keeps its own copy (spec 79 §8).

use fp_effects::photoshop::PsRecord;
use fp_model::{Effect, EffectKind};

use crate::psd_blocks::DescValue;

/// Order of the effects in the descriptor (spec 79 §3.2): `lfx2` key / class and the record of
/// the container (05 §9).
pub const LFX2_ORDER: [(&str, &str); 10] = [
    ("DrSh", "dropShadow"),
    ("IrSh", "innerShadow"),
    ("OrGl", "outerGlow"),
    ("SoFi", "solidFill"),
    ("GrFl", "gradientFill"),
    ("patternFill", "patternFill"),
    ("FrFX", "stroke"),
    ("IrGl", "innerGlow"),
    ("ebbl", "bevelEmboss"),
    ("ChFX", "satin"),
];

/// Position of an `lfx2` key in [`LFX2_ORDER`] (unknown keys last).
pub fn order_of(key: &str) -> usize {
    LFX2_ORDER
        .iter()
        .position(|(k, _)| *k == key)
        .unwrap_or(LFX2_ORDER.len())
}

/// Blend mode table (enum type `BlnM`, spec 79 §3.4): string ID of the record → char ID.
const BLEND_MODES: [(&str, &str); 29] = [
    ("normal", "Nrml"),
    ("dissolve", "Dslv"),
    ("darken", "Drkn"),
    ("multiply", "Mltp"),
    ("colorBurn", "CBrn"),
    ("linearBurn", "linearBurn"),
    ("darkerColor", "darkerColor"),
    ("lighten", "Lghn"),
    ("screen", "Scrn"),
    ("colorDodge", "CDdg"),
    ("linearDodge", "linearDodge"),
    ("lighterColor", "lighterColor"),
    ("overlay", "Ovrl"),
    ("softLight", "SftL"),
    ("hardLight", "HrdL"),
    ("vividLight", "vividLight"),
    ("linearLight", "linearLight"),
    ("pinLight", "pinLight"),
    ("hardMix", "hardMix"),
    ("difference", "Dfrn"),
    ("exclusion", "Xclu"),
    ("blendSubtraction", "blendSubtraction"),
    ("subtract", "blendSubtraction"),
    ("blendDivide", "blendDivide"),
    ("divide", "blendDivide"),
    ("hue", "H   "),
    ("saturation", "Strt"),
    ("color", "Clr "),
    ("luminosity", "Lmns"),
];

/// Blend mode string ID of a record → char ID of the descriptor (unknown: `Nrml`).
pub fn blend_char_id(mode: &str) -> &'static str {
    let m = mode.trim();
    BLEND_MODES
        .iter()
        .find(|(s, _)| *s == m)
        .map(|(_, c)| *c)
        .unwrap_or("Nrml")
}

/// The other enums of spec 79 §3.4 (enum type, [(string ID, char ID)]); the first value is the
/// default of the PSD format.
const ENUMS: [(&str, &[(&str, &str)]); 11] = [
    ("BETE", &[("softMatte", "SfBL"), ("preciseMatte", "PrBL")]),
    (
        "bvlT",
        &[
            ("softMatte", "SfBL"),
            ("preciseMatte", "PrBL"),
            ("slopeLimitMatte", "Slmt"),
        ],
    ),
    (
        "BESl",
        &[
            ("innerBevel", "InrB"),
            ("outerBevel", "OtrB"),
            ("emboss", "Embs"),
            ("pillowEmboss", "PlEb"),
            ("strokeEmboss", "strokeEmboss"),
        ],
    ),
    ("BESs", &[("stampIn", "In  "), ("stampOut", "Out ")]),
    ("IGSr", &[("edgeGlow", "SrcE"), ("centerGlow", "SrcC")]),
    (
        "GrdT",
        &[
            ("linear", "Lnr "),
            ("radial", "Rdl "),
            ("angle", "Angl"),
            ("reflected", "Rflc"),
            ("diamond", "Dmnd"),
        ],
    ),
    ("GrdF", &[("customStops", "CstS"), ("colorNoise", "ClNs")]),
    (
        "Clry",
        &[
            ("userStop", "UsrS"),
            ("backgroundColor", "BckC"),
            ("foregroundColor", "FrgC"),
        ],
    ),
    (
        "FStl",
        &[
            ("outsetFrame", "OutF"),
            ("insetFrame", "InsF"),
            ("centeredFrame", "CtrF"),
        ],
    ),
    (
        "FrFl",
        &[
            ("solidColor", "SClr"),
            ("gradientFill", "GrFl"),
            ("pattern", "Ptrn"),
        ],
    ),
    (
        "ClrS",
        &[
            ("RGBColor", "RGBC"),
            ("HSBColorEnum", "HSBl"),
            ("labColor", "LbCl"),
        ],
    ),
];

/// String ID of a record → char ID of the enum type `typ` (spec 79 §3.4). A char ID is passed
/// through; an unknown value gives the default of the type (its first value), an unknown type
/// `"Nrml"` via [`blend_char_id`].
pub fn enum_char_id(typ: &str, string_id: &str) -> &'static str {
    if typ == "BlnM" {
        return blend_char_id(string_id);
    }
    let s = string_id.trim();
    let Some((_, vals)) = ENUMS.iter().find(|(t, _)| *t == typ) else {
        return "Nrml";
    };
    vals.iter()
        .find(|(sid, cid)| *sid == s || *cid == s)
        .or(vals.first())
        .map(|(_, c)| *c)
        .unwrap_or("Nrml")
}

fn en(typ: &'static str, string_id: &str) -> DescValue {
    DescValue::Enum(typ, enum_char_id(typ, string_id))
}

fn num_text(t: &str) -> Option<f64> {
    t.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn rgb(r: f64, g: f64, b: f64) -> DescValue {
    DescValue::Obj(
        "RGBC",
        vec![
            ("Rd  ", DescValue::Double(r)),
            ("Grn ", DescValue::Double(g)),
            ("Bl  ", DescValue::Double(b)),
        ],
    )
}

/// Colour of a record (05 §9.3) → `Objc` (spec 79 §3.5). `None` for `nocolor`, `no…` and an
/// empty text (the key is left out). A colour book colour or an unknown class gives RGB 0/0/0.
pub fn parse_color(s: &str) -> Option<DescValue> {
    let s = s.trim();
    if s.is_empty() || s.starts_with("no") {
        return None;
    }
    let mut it = s.split(';');
    let class = it.next().unwrap_or("");
    let v: Vec<f64> = it.map(|t| num_text(t).unwrap_or(0.0)).collect();
    let g = |i: usize| v.get(i).copied().unwrap_or(0.0);
    let d = DescValue::Double;
    Some(match class {
        "RGBColor" => rgb(g(0), g(1), g(2)),
        "HSBColorClass" => DescValue::Obj(
            "HSBC",
            vec![
                ("H   ", DescValue::Unit("#Ang", g(0))),
                ("Strt", d(g(1))),
                ("Brgh", d(g(2))),
            ],
        ),
        "CMYKColorClass" => DescValue::Obj(
            "CMYC",
            vec![
                ("Cyn ", d(g(0))),
                ("Mgnt", d(g(1))),
                ("Ylw ", d(g(2))),
                ("Blck", d(g(3))),
            ],
        ),
        "labColor" => DescValue::Obj(
            "LbCl",
            vec![("Lmnc", d(g(0))), ("A   ", d(g(1))), ("B   ", d(g(2)))],
        ),
        "grayscale" => DescValue::Obj("Grsc", vec![("Gry ", d(g(0)))]),
        // `bookColor` cannot be mapped (spec 79 §3.5)
        _ => rgb(0.0, 0.0, 0.0),
    })
}

fn contour_point(h: f64, v: f64, corner: bool) -> DescValue {
    let mut items = vec![
        ("Hrzn", DescValue::Double(h)),
        ("Vrtc", DescValue::Double(v)),
    ];
    if corner {
        items.push(("Cnty", DescValue::Bool(false)));
    }
    DescValue::Obj("CrPt", items)
}

fn contour(name: &str, pts: &[(f64, f64)]) -> DescValue {
    DescValue::Obj(
        "ShpC",
        vec![
            ("Nm  ", DescValue::Text(name.into())),
            (
                "Crv ",
                DescValue::List(
                    pts.iter()
                        .map(|&(h, v)| contour_point(h, v, false))
                        .collect(),
                ),
            ),
        ],
    )
}

/// Linear contour (default of the PSD format, spec 79 §3.5).
pub fn linear_contour() -> DescValue {
    contour("Linear", &[(0.0, 0.0), (255.0, 255.0)])
}

/// Gaussian contour (default of the satin, spec 79 §3.5).
pub fn gaussian_contour() -> DescValue {
    contour(
        "Gaussian",
        &[
            (0.0, 0.0),
            (32.0, 7.0),
            (64.0, 38.0),
            (96.0, 101.0),
            (128.0, 166.0),
            (159.0, 209.0),
            (191.0, 235.0),
            (223.0, 248.0),
            (255.0, 255.0),
        ],
    )
}

fn parse_contour_or(s: &str, default: fn() -> DescValue) -> DescValue {
    let mut parts = s.split('$');
    let _count = parts.next();
    let Some(name) = parts.next() else {
        return default();
    };
    let mut pts = Vec::new();
    for p in parts {
        let f: Vec<&str> = p.split(';').collect();
        let (Some(h), Some(v)) = (
            f.first().and_then(|t| num_text(t)),
            f.get(1).and_then(|t| num_text(t)),
        ) else {
            return default();
        };
        let corner = f.get(2).is_some_and(|c| c.trim() == "0");
        pts.push(contour_point(h, v, corner));
    }
    if pts.len() < 2 {
        return default();
    }
    DescValue::Obj(
        "ShpC",
        vec![
            ("Nm  ", DescValue::Text(name.into())),
            ("Crv ", DescValue::List(pts)),
        ],
    )
}

/// Contour of a record (`N$name$h;v;c$…`, 05 §9.3) → `ShpC` (spec 79 §3.5): curve points
/// `Hrzn`/`Vrtc`, corner points (`c` = 0) also `Cnty` false. Empty or unreadable: Linear.
pub fn parse_contour(s: &str) -> DescValue {
    parse_contour_or(s, linear_contour)
}

fn long(t: &str, d: i32) -> i32 {
    num_text(t).map(|v| v.round() as i32).unwrap_or(d)
}

/// "Two Color" black → white (default gradient of the gradient overlay, spec 79 §4.8).
pub fn default_gradient() -> DescValue {
    let cstop = |c: f64, l: i32| {
        DescValue::Obj(
            "Clrt",
            vec![
                ("Clr ", rgb(c, c, c)),
                ("Type", DescValue::Enum("Clry", "UsrS")),
                ("Lctn", DescValue::Long(l)),
                ("Mdpn", DescValue::Long(50)),
            ],
        )
    };
    let tstop = |l: i32| {
        DescValue::Obj(
            "TrnS",
            vec![
                ("Opct", DescValue::Unit("#Prc", 100.0)),
                ("Lctn", DescValue::Long(l)),
                ("Mdpn", DescValue::Long(50)),
            ],
        )
    };
    DescValue::Obj(
        "Grdn",
        vec![
            ("Nm  ", DescValue::Text("Two Color".into())),
            ("GrdF", DescValue::Enum("GrdF", "CstS")),
            ("Intr", DescValue::Double(4096.0)),
            (
                "Clrs",
                DescValue::List(vec![cstop(0.0, 0), cstop(255.0, 4096)]),
            ),
            ("Trns", DescValue::List(vec![tstop(0), tstop(4096)])),
        ],
    )
}

/// Gradient of a record (`name,form,interp$nC$nT,stops…`, 05 §9.3) → `Grdn` (spec 79 §3.5).
/// `None` for an empty text or `nogradient`.
pub fn parse_gradient(s: &str) -> Option<DescValue> {
    let s = s.trim();
    if s.is_empty() || s.starts_with("no") {
        return None;
    }
    let mut parts: Vec<&str> = s.split(',').collect();
    if parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    let name = parts.first().copied().unwrap_or("");
    let form = parts.get(1).copied().unwrap_or("customStops").trim();
    let mut items = vec![("Nm  ", DescValue::Text(name.into()))];
    if form == "colorNoise" {
        // name,colorNoise,ShTr,VctC,RndS,Smth,mn0..3,mx0..3,ClrS
        let p = |i: usize| parts.get(i).copied().unwrap_or("");
        items.push(("GrdF", DescValue::Enum("GrdF", "ClNs")));
        items.push(("ShTr", DescValue::Bool(p(2).trim() == "1")));
        items.push(("VctC", DescValue::Bool(p(3).trim() == "1")));
        items.push(("ClrS", en("ClrS", p(14))));
        items.push(("RndS", DescValue::Long(long(p(4), 0))));
        items.push(("Smth", DescValue::Long(long(p(5), 2048))));
        items.push((
            "Mnm ",
            DescValue::List((6..10).map(|i| DescValue::Long(long(p(i), 0))).collect()),
        ));
        items.push((
            "Mxm ",
            DescValue::List((10..14).map(|i| DescValue::Long(long(p(i), 100))).collect()),
        ));
        return Some(DescValue::Obj("Grdn", items));
    }
    let head: Vec<&str> = parts.get(2).copied().unwrap_or("").split('$').collect();
    let interp = head.first().and_then(|t| num_text(t)).unwrap_or(4096.0);
    let nc = head
        .get(1)
        .and_then(|t| t.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let nt = head
        .get(2)
        .and_then(|t| t.trim().parse::<usize>().ok())
        .unwrap_or(0);
    let stops = parts.get(3..).unwrap_or(&[]);
    let mut clrs = Vec::new();
    for st in stops.iter().take(nc) {
        // location$midpoint$type[$colour]
        let f: Vec<&str> = st.splitn(4, '$').collect();
        let ty = f.get(2).copied().unwrap_or("userStop").trim();
        let mut it = Vec::new();
        if ty == "userStop" {
            it.push((
                "Clr ",
                f.get(3)
                    .and_then(|c| parse_color(c))
                    .unwrap_or_else(|| rgb(0.0, 0.0, 0.0)),
            ));
        }
        it.push(("Type", en("Clry", ty)));
        it.push((
            "Lctn",
            DescValue::Long(long(f.first().copied().unwrap_or(""), 0)),
        ));
        it.push((
            "Mdpn",
            DescValue::Long(long(f.get(1).copied().unwrap_or(""), 50)),
        ));
        clrs.push(DescValue::Obj("Clrt", it));
    }
    let mut trns = Vec::new();
    for st in stops.iter().skip(nc).take(nt) {
        // opacity$location$midpoint
        let f: Vec<&str> = st.split('$').collect();
        trns.push(DescValue::Obj(
            "TrnS",
            vec![
                (
                    "Opct",
                    DescValue::Unit("#Prc", f.first().and_then(|t| num_text(t)).unwrap_or(100.0)),
                ),
                (
                    "Lctn",
                    DescValue::Long(long(f.get(1).copied().unwrap_or(""), 0)),
                ),
                (
                    "Mdpn",
                    DescValue::Long(long(f.get(2).copied().unwrap_or(""), 50)),
                ),
            ],
        ));
    }
    items.push(("GrdF", DescValue::Enum("GrdF", "CstS")));
    items.push(("Intr", DescValue::Double(interp)));
    items.push(("Clrs", DescValue::List(clrs)));
    items.push(("Trns", DescValue::List(trns)));
    Some(DescValue::Obj("Grdn", items))
}

/// Default pattern of the PSD format (spec 79 §4.9).
const DEFAULT_PATTERN: (&str, &str) = ("Bubbles", "b7334da0-122f-11d4-8bb5-e27e45023b5f");

fn pattern(name: Option<&str>, id: Option<&str>) -> DescValue {
    DescValue::Obj(
        "Ptrn",
        vec![
            (
                "Nm  ",
                DescValue::Text(name.unwrap_or(DEFAULT_PATTERN.0).into()),
            ),
            (
                "Idnt",
                DescValue::Text(id.unwrap_or(DEFAULT_PATTERN.1).into()),
            ),
        ],
    )
}

/// Point with `#Prc` coordinates (gradient offset `Ofst`).
fn point_pct(h: f64, v: f64) -> DescValue {
    DescValue::Obj(
        "Pnt ",
        vec![
            ("Hrzn", DescValue::Unit("#Prc", h)),
            ("Vrtc", DescValue::Unit("#Prc", v)),
        ],
    )
}

/// Point with plain doubles (pattern `phase`).
fn point_doub(h: f64, v: f64) -> DescValue {
    DescValue::Obj(
        "Pnt ",
        vec![
            ("Hrzn", DescValue::Double(h)),
            ("Vrtc", DescValue::Double(v)),
        ],
    )
}

/// Field access of a record with the defaults of spec 79 §4.
struct Rec<'a>(&'a PsRecord);

impl Rec<'_> {
    fn get(&self, k: &str) -> Option<&str> {
        self.0.get(k)
    }
    /// `1` → true, other text false, missing field the default.
    fn flag(&self, k: &str, d: bool) -> DescValue {
        DescValue::Bool(self.get(k).map(|t| t.trim() == "1").unwrap_or(d))
    }
    fn num(&self, k: &str, d: f64) -> f64 {
        self.get(k).and_then(num_text).unwrap_or(d)
    }
    fn unit(&self, k: &str, unit: &'static str, d: f64) -> DescValue {
        DescValue::Unit(unit, self.num(k, d))
    }
    /// Enum with a default string ID for a missing or empty field.
    fn en(&self, k: &str, typ: &'static str, d: &str) -> DescValue {
        let v = self.get(k).map(str::trim).filter(|t| !t.is_empty());
        en(typ, v.unwrap_or(d))
    }
    fn mode(&self, k: &str, d: &str) -> DescValue {
        self.en(k, "BlnM", d)
    }
    /// Colour that is always written (missing or `nocolor`: the default).
    fn color(&self, k: &str, d: (f64, f64, f64)) -> DescValue {
        self.get(k)
            .and_then(parse_color)
            .unwrap_or_else(|| rgb(d.0, d.1, d.2))
    }
    /// Optional colour: missing field the default, `nocolor`/empty left out.
    fn opt_color(&self, k: &str, d: (f64, f64, f64)) -> Option<DescValue> {
        match self.get(k) {
            None => Some(rgb(d.0, d.1, d.2)),
            Some(t) => parse_color(t),
        }
    }
    fn contour(&self, k: &str) -> DescValue {
        parse_contour(self.get(k).unwrap_or(""))
    }
}

/// Default glow colour (spec 79 §4.3, B file).
const GLOW_COLOR: (f64, f64, f64) = (255.0, 255.0, 189.99610894941634);

/// Drop shadow (§4.1) and inner shadow (§4.2).
fn desc_shadow(rec: &PsRecord, drop: bool) -> DescValue {
    let r = Rec(rec);
    let mut items = vec![
        ("enab", r.flag("enabled", false)),
        ("Md  ", r.mode("mode", "multiply")),
        ("Clr ", r.color("color", (0.0, 0.0, 0.0))),
        ("Opct", r.unit("opacity", "#Prc", 75.0)),
        ("uglg", r.flag("useGlobalAngle", false)),
        (
            "lagl",
            r.unit(
                "localLightingAngle",
                "#Ang",
                if drop { 120.0 } else { -135.0 },
            ),
        ),
        ("Dstn", r.unit("distance", "#Pxl", 5.0)),
        // `Ckmt` is a percentage written as `#Pxl` (B file)
        ("Ckmt", r.unit("chokeMatte", "#Pxl", 0.0)),
        ("blur", r.unit("blur", "#Pxl", 5.0)),
        ("Nose", r.unit("noise", "#Prc", 0.0)),
        ("AntA", r.flag("antiAlias", false)),
        ("TrnS", r.contour("transferSpec")),
    ];
    if drop {
        items.push(("layerConceals", r.flag("layerConceals", true)));
    }
    DescValue::Obj(if drop { "DrSh" } else { "IrSh" }, items)
}

/// Outer glow (§4.3) and inner glow (§4.4, other key order).
fn desc_glow(rec: &PsRecord, outer: bool) -> DescValue {
    let r = Rec(rec);
    // opacity:chokeMatte:blur:noise:shadingNoise:inputRange
    let mut f = [75.0, 0.0, 5.0, 0.0, 0.0, 50.0];
    if let Some(t) = r.get("params") {
        for (o, v) in f.iter_mut().zip(t.split(':')) {
            if let Some(x) = num_text(v) {
                *o = x;
            }
        }
    }
    let mut items = vec![
        ("enab", r.flag("enabled", false)),
        ("Md  ", r.mode("mode", "screen")),
    ];
    if let Some(c) = r.opt_color("color", GLOW_COLOR) {
        items.push(("Clr ", c));
    }
    if let Some(g) = r.get("gradient").and_then(parse_gradient) {
        items.push(("Grad", g));
    }
    items.push(("Opct", DescValue::Unit("#Prc", f[0])));
    items.push(("GlwT", r.en("glowTechnique", "BETE", "softMatte")));
    items.push(("Ckmt", DescValue::Unit("#Pxl", f[1])));
    items.push(("blur", DescValue::Unit("#Pxl", f[2])));
    let noise = ("Nose", DescValue::Unit("#Prc", f[3]));
    let jitter = ("ShdN", DescValue::Unit("#Prc", f[4]));
    if outer {
        items.push(noise);
        items.push(jitter);
        items.push(("AntA", r.flag("antiAlias", false)));
    } else {
        items.push(jitter);
        items.push(noise);
        items.push(("AntA", r.flag("antiAlias", false)));
        items.push(("glwS", r.en("innerGlowSource", "IGSr", "edgeGlow")));
    }
    items.push(("TrnS", r.contour("transferSpec")));
    items.push(("Inpr", DescValue::Unit("#Prc", f[5])));
    DescValue::Obj(if outer { "OrGl" } else { "IrGl" }, items)
}

/// Bevel and emboss (§4.5).
fn desc_bevel(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    let mut items = vec![
        ("enab", r.flag("enabled", false)),
        ("hglM", r.mode("highlightMode", "screen")),
    ];
    if let Some(c) = r.opt_color("highlightColor", (255.0, 255.0, 255.0)) {
        items.push(("hglC", c));
    }
    items.push(("hglO", r.unit("highlightOpacity", "#Prc", 75.0)));
    items.push(("sdwM", r.mode("shadowMode", "multiply")));
    if let Some(c) = r.opt_color("shadowColor", (0.0, 0.0, 0.0)) {
        items.push(("sdwC", c));
    }
    items.extend([
        ("sdwO", r.unit("shadowOpacity", "#Prc", 75.0)),
        ("bvlT", r.en("bevelTechnique", "bvlT", "softMatte")),
        ("bvlS", r.en("bevelStyle", "BESl", "innerBevel")),
        ("uglg", r.flag("useGlobalAngle", false)),
        ("lagl", r.unit("localLightingAngle", "#Ang", 120.0)),
        ("Lald", r.unit("localLightingAltitude", "#Ang", 30.0)),
        ("srgR", r.unit("strengthRatio", "#Prc", 100.0)),
        ("blur", r.unit("blur", "#Pxl", 5.0)),
        ("bvlD", r.en("bevelDirection", "BESs", "stampIn")),
        ("TrnS", r.contour("transparencyShape")),
        ("antialiasGloss", r.flag("antialiasGloss", false)),
        ("Sftn", r.unit("softness", "#Pxl", 0.0)),
        ("useShape", r.flag("useShape", false)),
    ]);
    // field 20: antiAlias,inputRange,contour
    if let Some(sh) = r.get("shape").map(str::trim)
        && !sh.is_empty()
        && !sh.starts_with("no")
    {
        let f: Vec<&str> = sh.splitn(3, ',').collect();
        items.push(("MpgS", parse_contour(f.get(2).copied().unwrap_or(""))));
        items.push((
            "AntA",
            DescValue::Bool(f.first().is_some_and(|t| t.trim() == "1")),
        ));
        items.push((
            "Inpr",
            DescValue::Unit("#Prc", f.get(1).and_then(|t| num_text(t)).unwrap_or(50.0)),
        ));
    }
    items.push(("useTexture", r.flag("useTexture", false)));
    // field 21: invert,align,scale,depth,h,v,name,ID (a comma in the name stays in the name)
    if let Some(tx) = r.get("texture").map(str::trim)
        && !tx.is_empty()
        && !tx.starts_with("no")
    {
        let f: Vec<&str> = tx.split(',').collect();
        let n = |i: usize, d: f64| f.get(i).and_then(|t| num_text(t)).unwrap_or(d);
        let (name, id) = if f.len() >= 8 {
            (Some(f[6..f.len() - 1].join(",")), f.last().copied())
        } else {
            (f.get(6).map(|s| s.to_string()), None)
        };
        items.extend([
            ("Ptrn", pattern(name.as_deref(), id)),
            ("Scl ", DescValue::Unit("#Prc", n(2, 100.0))),
            ("textureDepth", DescValue::Unit("#Prc", n(3, 100.0))),
            (
                "InvT",
                DescValue::Bool(f.first().is_some_and(|t| t.trim() == "1")),
            ),
            (
                "Algn",
                DescValue::Bool(f.get(1).is_none_or(|t| t.trim() == "1")),
            ),
            ("phase", point_doub(n(4, 0.0), n(5, 0.0))),
        ]);
    }
    DescValue::Obj("ebbl", items)
}

/// Satin (§4.6, no `uglg`).
fn desc_satin(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    DescValue::Obj(
        "ChFX",
        vec![
            ("enab", r.flag("enabled", false)),
            ("Md  ", r.mode("mode", "multiply")),
            ("Clr ", r.color("color", (0.0, 0.0, 0.0))),
            ("AntA", r.flag("antiAlias", false)),
            ("Invr", r.flag("invert", true)),
            ("Opct", r.unit("opacity", "#Prc", 50.0)),
            ("lagl", r.unit("localLightingAngle", "#Ang", 19.0)),
            ("Dstn", r.unit("distance", "#Pxl", 11.0)),
            ("blur", r.unit("blur", "#Pxl", 14.0)),
            (
                "MpgS",
                parse_contour_or(r.get("mappingShape").unwrap_or(""), gaussian_contour),
            ),
        ],
    )
}

/// Colour overlay (§4.7).
fn desc_solid_fill(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    DescValue::Obj(
        "SoFi",
        vec![
            ("enab", r.flag("enabled", false)),
            ("Md  ", r.mode("mode", "normal")),
            ("Opct", r.unit("opacity", "#Prc", 100.0)),
            ("Clr ", r.color("color", (255.0, 0.0, 0.0))),
        ],
    )
}

fn gradient_or_default(s: Option<&str>) -> DescValue {
    s.and_then(parse_gradient).unwrap_or_else(default_gradient)
}

/// Gradient overlay (§4.8; `Algn` before `Scl `).
fn desc_gradient_fill(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    DescValue::Obj(
        "GrFl",
        vec![
            ("enab", r.flag("enabled", false)),
            ("Md  ", r.mode("mode", "normal")),
            ("Opct", r.unit("opacity", "#Prc", 100.0)),
            ("Grad", gradient_or_default(r.get("gradient"))),
            ("Angl", r.unit("angle", "#Ang", 90.0)),
            ("Type", r.en("type", "GrdT", "linear")),
            ("Rvrs", r.flag("reverse", false)),
            ("Algn", r.flag("align", true)),
            ("Scl ", r.unit("scale", "#Prc", 100.0)),
            (
                "Ofst",
                point_pct(r.num("horizontal", 0.0), r.num("vertical", 0.0)),
            ),
        ],
    )
}

/// Pattern overlay (§4.9, long key and class).
fn desc_pattern_fill(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    DescValue::Obj(
        "patternFill",
        vec![
            ("enab", r.flag("enabled", false)),
            ("Md  ", r.mode("mode", "normal")),
            ("Opct", r.unit("opacity", "#Prc", 100.0)),
            ("Ptrn", pattern(r.get("patternName"), r.get("patternID"))),
            ("Scl ", r.unit("scale", "#Prc", 100.0)),
            ("Algn", r.flag("align", true)),
            (
                "phase",
                point_doub(r.num("horizontal", 0.0), r.num("vertical", 0.0)),
            ),
        ],
    )
}

/// Stroke (§4.10): solid colour, gradient (`Scl ` before `Algn`) and pattern.
fn desc_stroke(rec: &PsRecord) -> DescValue {
    let r = Rec(rec);
    let paint = r
        .get("paintType")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .unwrap_or("solidColor");
    let mut items = vec![
        ("enab", r.flag("enabled", false)),
        ("Styl", r.en("style", "FStl", "outsetFrame")),
        ("PntT", en("FrFl", paint)),
        ("Md  ", r.mode("mode", "normal")),
        ("Opct", r.unit("opacity", "#Prc", 100.0)),
        ("Sz  ", r.unit("size", "#Pxl", 3.0)),
        // always written, also for gradient and pattern (B file)
        ("Clr ", r.color("color", (255.0, 0.0, 0.0))),
    ];
    let sub: Vec<&str> = r
        .get("sub")
        .map(|s| s.split('&').collect())
        .unwrap_or_default();
    let s = |i: usize| sub.get(i).copied();
    let n = |i: usize, d: f64| s(i).and_then(num_text).unwrap_or(d);
    let b = |i: usize, d: bool| DescValue::Bool(s(i).map(|t| t.trim() == "1").unwrap_or(d));
    match enum_char_id("FrFl", paint) {
        "GrFl" => {
            // reverse&align&angle&scale&horizontal&vertical&type&gradient
            items.extend([
                ("Grad", gradient_or_default(s(7))),
                ("Angl", DescValue::Unit("#Ang", n(2, 90.0))),
                (
                    "Type",
                    en(
                        "GrdT",
                        s(6).map(str::trim)
                            .filter(|t| !t.is_empty())
                            .unwrap_or("linear"),
                    ),
                ),
                ("Rvrs", b(0, false)),
                ("Scl ", DescValue::Unit("#Prc", n(3, 100.0))),
                ("Algn", b(1, true)),
                ("Ofst", point_pct(n(4, 0.0), n(5, 0.0))),
            ]);
        }
        "Ptrn" => {
            // linked&scale&horizontal&vertical&name&ID
            items.extend([
                ("Ptrn", pattern(s(4), s(5))),
                ("Scl ", DescValue::Unit("#Prc", n(1, 100.0))),
                ("Lnkd", b(0, true)),
                ("phase", point_doub(n(2, 0.0), n(3, 0.0))),
            ]);
        }
        _ => {}
    }
    DescValue::Obj("FrFX", items)
}

/// Stroke layer effect (`FrFX`) of a PSD-style stroke (F138; now with gradient and pattern).
pub fn frame_fx(sp: &fp_effects::StrokeParams) -> DescValue {
    desc_stroke(&PsRecord::parse("stroke", &sp.to_record()))
}

/// `lfx2` entry of one record of the container (`name` = record name of 05 §9).
pub fn record_entry(name: &str, text: &str) -> Option<(&'static str, DescValue)> {
    let rec = PsRecord::parse(name, text);
    let key = LFX2_ORDER.iter().find(|(_, r)| *r == name)?.0;
    let v = match name {
        "dropShadow" => desc_shadow(&rec, true),
        "innerShadow" => desc_shadow(&rec, false),
        "outerGlow" => desc_glow(&rec, true),
        "innerGlow" => desc_glow(&rec, false),
        "bevelEmboss" => desc_bevel(&rec),
        "satin" => desc_satin(&rec),
        "solidFill" => desc_solid_fill(&rec),
        "gradientFill" => desc_gradient_fill(&rec),
        "patternFill" => desc_pattern_fill(&rec),
        "stroke" => desc_stroke(&rec),
        _ => return None,
    };
    Some((key, v))
}

/// All `lfx2` entries of a PSD live effect container (every record, disabled ones with `enab`
/// false), in the order of [`LFX2_ORDER`]; empty for other effects.
pub fn container_entries(e: &Effect) -> Vec<(&'static str, DescValue)> {
    if e.kind() != EffectKind::PhotoshopLive {
        return Vec::new();
    }
    let mut out: Vec<(&'static str, DescValue)> = e
        .psd_sub_effects()
        .into_iter()
        .filter_map(|n| record_entry(n, e.get(n)?))
        .collect();
    out.sort_by_key(|(k, _)| order_of(k));
    out
}

/// Global light of the layer effects: (angle, altitude) in whole degrees (image resources
/// 1037/1049, spec 79 §5.4).
pub type GlobalLight = (i32, i32);

/// Record name of an `lfx2` key.
pub fn record_of(key: &str) -> Option<&'static str> {
    LFX2_ORDER.iter().find(|(k, _)| *k == key).map(|(_, r)| *r)
}

/// Global light (angle, altitude) of an entry that uses the global angle (`uglg` true): `lagl`
/// and `Lald` (default 30), rounded to whole degrees (spec 79 §5.4).
pub fn global_light(entry: &DescValue) -> Option<GlobalLight> {
    let DescValue::Obj(_, items) = entry else {
        return None;
    };
    let get = |k: &str| items.iter().find(|(n, _)| *n == k).map(|(_, v)| v);
    if get("uglg") != Some(&DescValue::Bool(true)) {
        return None;
    }
    let deg = |k: &str, d: f64| match get(k) {
        Some(DescValue::Unit(_, v)) => *v,
        _ => d,
    };
    Some((
        deg("lagl", 120.0).round() as i32,
        deg("Lald", 30.0).round() as i32,
    ))
}

/// Switches a record off (`enabled` = 0, the rest unchanged).
pub fn record_off(text: &str) -> String {
    match text.split_once('&') {
        Some((_, rest)) => format!("0&{rest}"),
        None => "0".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psd_blocks::{effects_block_with_light, layer_effects};

    const LIN: &str = "2$Linear$0.000000;0.000000;1$255.000000;255.000000;1";
    const GAUSS: &str = "9$Gaussian$0.000000;0.000000;1$32.000000;7.000000;1$64.000000;38.000000;1$96.000000;101.000000;1$128.000000;166.000000;1$159.000000;209.000000;1$191.000000;235.000000;1$223.000000;248.000000;1$255.000000;255.000000;1";
    const TWO_COLOR: &str = "Two Color,customStops,4096.000000$2$2,0$50$userStop$RGBColor;0.000000;0.000000;0.000000,4096$50$userStop$RGBColor;255.000000;255.000000;255.000000,100.000000$0$50,100.000000$4096$50,";

    /// The ten records with the defaults of spec 79 §4 (all switched off, like layer 1 of the
    /// reference file).
    fn default_records() -> Vec<(&'static str, String)> {
        vec![
            ("dropShadow", format!("0&75.000000&0&120.000000&0.000000&5.000000&5.000000&0.000000&0&1&multiply&RGBColor;0.000000;0.000000;0.000000&{LIN}")),
            ("innerShadow", format!("0&75.000000&0&-135.000000&0.000000&5.000000&5.000000&0.000000&0&0&multiply&RGBColor;0.000000;0.000000;0.000000&{LIN}")),
            ("outerGlow", format!("0&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000&softMatte&RGBColor;255.000000;255.000000;189.996109&nogradient&{LIN}&")),
            ("innerGlow", format!("0&0&screen&75.000000:0.000000:5.000000:0.000000:0.000000:50.000000&softMatte&RGBColor;255.000000;255.000000;189.996109&nogradient&{LIN}&edgeGlow")),
            ("bevelEmboss", format!("0&0&0&0&0&75.000000&75.000000&120.000000&30.000000&100.000000&5.000000&0.000000&screen&RGBColor;255.000000;255.000000;255.000000&multiply&RGBColor;0.000000;0.000000;0.000000&softMatte&innerBevel&stampIn&{LIN}&noshape&notexture")),
            ("satin", format!("0&0&1&50.000000&19.000000&11.000000&14.000000&multiply&RGBColor;0.000000;0.000000;0.000000&{GAUSS}")),
            ("solidFill", "0&100.000000&normal&RGBColor;255.000000;0.000000;0.000000".into()),
            ("gradientFill", format!("0&0&1&100.000000&90.000000&100.000000&0.000000&0.000000&normal&linear&{TWO_COLOR}")),
            ("patternFill", "0&1&100.000000&100.000000&0.000000&0.000000&normal&Bubbles&b7334da0-122f-11d4-8bb5-e27e45023b5f".into()),
            ("stroke", "0&100.000000&3.000000&normal&RGBColor;255.000000;0.000000;0.000000&outsetFrame&solidColor&".into()),
        ]
    }

    fn container(records: &[(&str, String)]) -> Effect {
        let mut e = Effect::new(EffectKind::PhotoshopLive.moa_id().expect("moa"), true);
        for (k, v) in records {
            e.set(k, v.as_str());
        }
        e
    }

    fn items(v: &DescValue) -> &[(&'static str, DescValue)] {
        match v {
            DescValue::Obj(_, i) => i,
            _ => panic!("object expected"),
        }
    }

    fn keys(v: &DescValue) -> Vec<&'static str> {
        items(v).iter().map(|(k, _)| *k).collect()
    }

    fn get<'a>(v: &'a DescValue, k: &str) -> &'a DescValue {
        &items(v)
            .iter()
            .find(|(n, _)| *n == k)
            .unwrap_or_else(|| panic!("key {k}"))
            .1
    }

    fn entry(name: &str, rec: &str) -> DescValue {
        record_entry(name, rec).expect("entry").1
    }

    fn unit(v: &DescValue) -> (&'static str, f64) {
        match v {
            DescValue::Unit(u, x) => (u, *x),
            _ => panic!("unit expected: {v:?}"),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    fn rec(n: &str) -> (&'static str, String) {
        default_records()
            .into_iter()
            .find(|(k, _)| *k == n)
            .expect("record")
    }

    #[test]
    fn default_container_matches_the_spec_tables() {
        let e = container(&default_records());
        let all = container_entries(&e);
        let order: Vec<&str> = all.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            order,
            [
                "DrSh",
                "IrSh",
                "OrGl",
                "SoFi",
                "GrFl",
                "patternFill",
                "FrFX",
                "IrGl",
                "ebbl",
                "ChFX"
            ]
        );
        let by = |k: &str| &all.iter().find(|(n, _)| *n == k).expect("entry").1;
        let sh = [
            "enab", "Md  ", "Clr ", "Opct", "uglg", "lagl", "Dstn", "Ckmt", "blur", "Nose", "AntA",
            "TrnS",
        ];
        let mut dr = sh.to_vec();
        dr.push("layerConceals");
        assert_eq!(keys(by("DrSh")), dr);
        assert_eq!(keys(by("IrSh")), sh);
        assert_eq!(
            keys(by("OrGl")),
            [
                "enab", "Md  ", "Clr ", "Opct", "GlwT", "Ckmt", "blur", "Nose", "ShdN", "AntA",
                "TrnS", "Inpr"
            ]
        );
        assert_eq!(
            keys(by("IrGl")),
            [
                "enab", "Md  ", "Clr ", "Opct", "GlwT", "Ckmt", "blur", "ShdN", "Nose", "AntA",
                "glwS", "TrnS", "Inpr"
            ]
        );
        assert_eq!(
            keys(by("ebbl")),
            [
                "enab",
                "hglM",
                "hglC",
                "hglO",
                "sdwM",
                "sdwC",
                "sdwO",
                "bvlT",
                "bvlS",
                "uglg",
                "lagl",
                "Lald",
                "srgR",
                "blur",
                "bvlD",
                "TrnS",
                "antialiasGloss",
                "Sftn",
                "useShape",
                "useTexture"
            ]
        );
        assert_eq!(
            keys(by("ChFX")),
            [
                "enab", "Md  ", "Clr ", "AntA", "Invr", "Opct", "lagl", "Dstn", "blur", "MpgS"
            ]
        );
        assert_eq!(keys(by("SoFi")), ["enab", "Md  ", "Opct", "Clr "]);
        assert_eq!(
            keys(by("GrFl")),
            [
                "enab", "Md  ", "Opct", "Grad", "Angl", "Type", "Rvrs", "Algn", "Scl ", "Ofst"
            ]
        );
        assert_eq!(
            keys(by("patternFill")),
            ["enab", "Md  ", "Opct", "Ptrn", "Scl ", "Algn", "phase"]
        );
        assert_eq!(
            keys(by("FrFX")),
            ["enab", "Styl", "PntT", "Md  ", "Opct", "Sz  ", "Clr "]
        );
        // every record is switched off
        for (_, v) in &all {
            assert_eq!(get(v, "enab"), &DescValue::Bool(false));
        }
        // defaults, types and units (spec 79 §4)
        let d = by("DrSh");
        assert_eq!(get(d, "Md  "), &DescValue::Enum("BlnM", "Mltp"));
        assert_eq!(unit(get(d, "Opct")), ("#Prc", 75.0));
        assert_eq!(unit(get(d, "lagl")), ("#Ang", 120.0));
        assert_eq!(unit(get(d, "Dstn")), ("#Pxl", 5.0));
        assert_eq!(unit(get(d, "Ckmt")), ("#Pxl", 0.0), "Ckmt in #Pxl");
        assert_eq!(unit(get(d, "Nose")), ("#Prc", 0.0));
        assert_eq!(get(d, "layerConceals"), &DescValue::Bool(true));
        assert_eq!(get(d, "TrnS"), &linear_contour());
        assert_eq!(unit(get(by("IrSh"), "lagl")), ("#Ang", -135.0));
        let og = by("OrGl");
        assert_eq!(get(og, "Md  "), &DescValue::Enum("BlnM", "Scrn"));
        assert_eq!(get(og, "GlwT"), &DescValue::Enum("BETE", "SfBL"));
        assert_eq!(unit(get(og, "Inpr")), ("#Prc", 50.0));
        let DescValue::Double(b) = get(get(og, "Clr "), "Bl  ") else {
            panic!("blue")
        };
        assert!((b - 189.99610894941634).abs() < 1e-5);
        assert_eq!(get(by("IrGl"), "glwS"), &DescValue::Enum("IGSr", "SrcE"));
        let bv = by("ebbl");
        assert_eq!(get(bv, "hglM"), &DescValue::Enum("BlnM", "Scrn"));
        assert_eq!(get(bv, "sdwM"), &DescValue::Enum("BlnM", "Mltp"));
        assert_eq!(get(bv, "bvlT"), &DescValue::Enum("bvlT", "SfBL"));
        assert_eq!(get(bv, "bvlS"), &DescValue::Enum("BESl", "InrB"));
        assert_eq!(get(bv, "bvlD"), &DescValue::Enum("BESs", "In  "));
        assert_eq!(unit(get(bv, "Lald")), ("#Ang", 30.0));
        assert_eq!(unit(get(bv, "srgR")), ("#Prc", 100.0));
        assert_eq!(unit(get(bv, "Sftn")), ("#Pxl", 0.0));
        let sa = by("ChFX");
        assert_eq!(get(sa, "Invr"), &DescValue::Bool(true));
        assert_eq!(unit(get(sa, "Opct")), ("#Prc", 50.0));
        assert_eq!(unit(get(sa, "lagl")), ("#Ang", 19.0));
        assert_eq!(unit(get(sa, "Dstn")), ("#Pxl", 11.0));
        assert_eq!(unit(get(sa, "blur")), ("#Pxl", 14.0));
        assert_eq!(get(sa, "MpgS"), &gaussian_contour());
        let so = by("SoFi");
        assert_eq!(get(so, "Md  "), &DescValue::Enum("BlnM", "Nrml"));
        assert_eq!(get(so, "Clr "), &rgb(255.0, 0.0, 0.0));
        let gf = by("GrFl");
        assert_eq!(get(gf, "Algn"), &DescValue::Bool(true));
        assert_eq!(get(gf, "Type"), &DescValue::Enum("GrdT", "Lnr "));
        assert_eq!(unit(get(gf, "Angl")), ("#Ang", 90.0));
        assert_eq!(get(gf, "Ofst"), &point_pct(0.0, 0.0));
        assert_eq!(get(gf, "Grad"), &default_gradient());
        let pf = by("patternFill");
        assert_eq!(get(pf, "Algn"), &DescValue::Bool(true));
        assert_eq!(get(pf, "Ptrn"), &pattern(None, None));
        assert_eq!(get(pf, "phase"), &point_doub(0.0, 0.0), "phase in doubles");
        let fr = by("FrFX");
        assert_eq!(get(fr, "Styl"), &DescValue::Enum("FStl", "OutF"));
        assert_eq!(get(fr, "PntT"), &DescValue::Enum("FrFl", "SClr"));
        assert_eq!(unit(get(fr, "Sz  ")), ("#Pxl", 3.0));
    }

    #[test]
    fn missing_fields_give_the_spec_defaults() {
        // a record with only `enabled`: the keys of the default record (spec 79 §4)
        for (name, full) in default_records() {
            assert_eq!(keys(&entry(name, "0")), keys(&entry(name, &full)), "{name}");
            // the glow colour of a record has 6 decimals, the default keeps all of them
            if !name.ends_with("Glow") {
                assert_eq!(entry(name, "0"), entry(name, &full), "{name}");
            }
        }
        assert_eq!(unit(get(&entry("innerShadow", "1"), "lagl")).1, -135.0);
        assert_eq!(get(&entry("satin", "1"), "MpgS"), &gaussian_contour());
        assert_eq!(get(&entry("satin", "1"), "Invr"), &DescValue::Bool(true));
        assert_eq!(get(&entry("satin", "1"), "enab"), &DescValue::Bool(true));
        // unreadable numbers: default
        let d = entry("dropShadow", "1&abc&0&nan");
        assert_eq!(unit(get(&d, "Opct")).1, 75.0);
        assert_eq!(unit(get(&d, "lagl")).1, 120.0);
    }

    #[test]
    fn non_default_values_are_written_unchanged() {
        let d = entry(
            "dropShadow",
            &format!(
                "1&60.000000&1&33.000000&10.000000&7.000000&4.000000&20.000000&1&0&linearBurn&RGBColor;10.500000;20.000000;30.000000&{GAUSS}"
            ),
        );
        assert_eq!(get(&d, "enab"), &DescValue::Bool(true));
        assert_eq!(get(&d, "Md  "), &DescValue::Enum("BlnM", "linearBurn"));
        assert_eq!(get(&d, "Clr "), &rgb(10.5, 20.0, 30.0));
        assert_eq!(unit(get(&d, "Opct")).1, 60.0);
        assert_eq!(get(&d, "uglg"), &DescValue::Bool(true));
        assert_eq!(unit(get(&d, "lagl")).1, 33.0);
        assert_eq!(unit(get(&d, "Nose")).1, 10.0);
        assert_eq!(unit(get(&d, "Dstn")).1, 7.0);
        assert_eq!(unit(get(&d, "blur")).1, 4.0);
        assert_eq!(unit(get(&d, "Ckmt")), ("#Pxl", 20.0));
        assert_eq!(get(&d, "AntA"), &DescValue::Bool(true));
        assert_eq!(get(&d, "layerConceals"), &DescValue::Bool(false));
        assert_eq!(get(&d, "TrnS"), &gaussian_contour());
        let i = entry("innerShadow", "1&50.000000&0&45.000000");
        assert_eq!(unit(get(&i, "lagl")).1, 45.0);
        let g = entry(
            "outerGlow",
            &format!(
                "1&1&hue&40.000000:10.000000:12.000000:3.000000:4.000000:60.000000&preciseMatte&nocolor&{TWO_COLOR}&{LIN}&"
            ),
        );
        assert_eq!(
            keys(&g),
            [
                "enab", "Md  ", "Grad", "Opct", "GlwT", "Ckmt", "blur", "Nose", "ShdN", "AntA",
                "TrnS", "Inpr"
            ],
            "nocolor leaves out Clr, a gradient follows directly"
        );
        assert_eq!(get(&g, "Md  "), &DescValue::Enum("BlnM", "H   "));
        assert_eq!(get(&g, "GlwT"), &DescValue::Enum("BETE", "PrBL"));
        assert_eq!(unit(get(&g, "Opct")).1, 40.0);
        assert_eq!(unit(get(&g, "Ckmt")), ("#Pxl", 10.0));
        assert_eq!(unit(get(&g, "blur")).1, 12.0);
        assert_eq!(unit(get(&g, "Nose")).1, 3.0);
        assert_eq!(unit(get(&g, "ShdN")).1, 4.0);
        assert_eq!(unit(get(&g, "Inpr")).1, 60.0);
        let ig = entry(
            "innerGlow",
            &format!(
                "1&0&screen&75:0:5:0:0:50&softMatte&RGBColor;1;2;3&nogradient&{LIN}&centerGlow"
            ),
        );
        assert_eq!(get(&ig, "glwS"), &DescValue::Enum("IGSr", "SrcC"));
        assert_eq!(get(&ig, "Clr "), &rgb(1.0, 2.0, 3.0));
        let s = entry(
            "satin",
            &format!("1&1&0&70.000000&-30.000000&20.000000&8.000000&screen&RGBColor;0;0;255&{LIN}"),
        );
        assert_eq!(get(&s, "Invr"), &DescValue::Bool(false));
        assert_eq!(unit(get(&s, "lagl")).1, -30.0);
        assert_eq!(get(&s, "MpgS"), &linear_contour());
        let f = entry("solidFill", "1&40.000000&multiply&RGBColor;0;255;0");
        assert_eq!(unit(get(&f, "Opct")).1, 40.0);
        assert_eq!(get(&f, "Md  "), &DescValue::Enum("BlnM", "Mltp"));
        let gf = entry(
            "gradientFill",
            "1&1&0&80.000000&45.000000&120.000000&10.000000&-5.000000&overlay&reflected&nogradient",
        );
        assert_eq!(get(&gf, "Rvrs"), &DescValue::Bool(true));
        assert_eq!(get(&gf, "Algn"), &DescValue::Bool(false));
        assert_eq!(get(&gf, "Type"), &DescValue::Enum("GrdT", "Rflc"));
        assert_eq!(unit(get(&gf, "Scl ")).1, 120.0);
        assert_eq!(get(&gf, "Ofst"), &point_pct(10.0, -5.0));
        let pf = entry(
            "patternFill",
            "1&0&50.000000&200.000000&3.000000&4.000000&normal&Dots&abc-1",
        );
        assert_eq!(get(&pf, "Ptrn"), &pattern(Some("Dots"), Some("abc-1")));
        assert_eq!(get(&pf, "Algn"), &DescValue::Bool(false));
        assert_eq!(unit(get(&pf, "Scl ")).1, 200.0);
        assert_eq!(get(&pf, "phase"), &point_doub(3.0, 4.0));
        let b = entry(
            "bevelEmboss",
            &format!(
                "1&1&1&1&1&60&50&100&45&300&10&2&overlay&nohighlightColor&darken&RGBColor;1;1;1&slopeLimitMatte&strokeEmboss&stampOut&{LIN}&1,70.000000,{GAUSS}&1,0,150.000000,-200.000000,5.000000,6.000000,Bubbles,b7334da0-122f-11d4-8bb5-e27e45023b5f"
            ),
        );
        assert_eq!(
            keys(&b),
            [
                "enab",
                "hglM",
                "hglO",
                "sdwM",
                "sdwC",
                "sdwO",
                "bvlT",
                "bvlS",
                "uglg",
                "lagl",
                "Lald",
                "srgR",
                "blur",
                "bvlD",
                "TrnS",
                "antialiasGloss",
                "Sftn",
                "useShape",
                "MpgS",
                "AntA",
                "Inpr",
                "useTexture",
                "Ptrn",
                "Scl ",
                "textureDepth",
                "InvT",
                "Algn",
                "phase"
            ]
        );
        assert_eq!(get(&b, "bvlT"), &DescValue::Enum("bvlT", "Slmt"));
        assert_eq!(get(&b, "bvlS"), &DescValue::Enum("BESl", "strokeEmboss"));
        assert_eq!(get(&b, "bvlD"), &DescValue::Enum("BESs", "Out "));
        assert_eq!(unit(get(&b, "Lald")).1, 45.0);
        assert_eq!(unit(get(&b, "srgR")).1, 300.0);
        assert_eq!(get(&b, "MpgS"), &gaussian_contour());
        assert_eq!(get(&b, "AntA"), &DescValue::Bool(true));
        assert_eq!(unit(get(&b, "Inpr")).1, 70.0);
        assert_eq!(get(&b, "InvT"), &DescValue::Bool(true));
        assert_eq!(get(&b, "Algn"), &DescValue::Bool(false));
        assert_eq!(unit(get(&b, "textureDepth")).1, -200.0);
        assert_eq!(get(&b, "phase"), &point_doub(5.0, 6.0));
        assert_eq!(get(&b, "Ptrn"), &pattern(None, None));
    }

    #[test]
    fn stroke_with_gradient_and_pattern() {
        let g = entry(
            "stroke",
            &format!(
                "1&100.000000&5.000000&normal&RGBColor;255;0;0&centeredFrame&gradientFill&1&0&30.000000&80.000000&5.000000&6.000000&radial&{TWO_COLOR}"
            ),
        );
        assert_eq!(
            keys(&g),
            [
                "enab", "Styl", "PntT", "Md  ", "Opct", "Sz  ", "Clr ", "Grad", "Angl", "Type",
                "Rvrs", "Scl ", "Algn", "Ofst"
            ],
            "Scl before Algn in FrFX"
        );
        assert_eq!(get(&g, "PntT"), &DescValue::Enum("FrFl", "GrFl"));
        assert_eq!(get(&g, "Styl"), &DescValue::Enum("FStl", "CtrF"));
        assert_eq!(get(&g, "Type"), &DescValue::Enum("GrdT", "Rdl "));
        assert_eq!(get(&g, "Rvrs"), &DescValue::Bool(true));
        assert_eq!(get(&g, "Algn"), &DescValue::Bool(false));
        assert_eq!(unit(get(&g, "Angl")), ("#Ang", 30.0));
        assert_eq!(get(&g, "Ofst"), &point_pct(5.0, 6.0));
        assert_eq!(get(&g, "Grad"), &default_gradient());
        let p = entry(
            "stroke",
            "1&100.000000&5.000000&normal&RGBColor;255;0;0&insetFrame&pattern&0&150.000000&1.000000&2.000000&Dots&id-7",
        );
        assert_eq!(
            keys(&p),
            [
                "enab", "Styl", "PntT", "Md  ", "Opct", "Sz  ", "Clr ", "Ptrn", "Scl ", "Lnkd",
                "phase"
            ]
        );
        assert_eq!(get(&p, "Ptrn"), &pattern(Some("Dots"), Some("id-7")));
        assert_eq!(get(&p, "Lnkd"), &DescValue::Bool(false));
        assert_eq!(get(&p, "phase"), &point_doub(1.0, 2.0));
        assert_eq!(unit(get(&p, "Scl ")).1, 150.0);
    }

    #[test]
    fn special_values() {
        // colour classes
        assert_eq!(
            parse_color("labColor;50;-10;20"),
            Some(DescValue::Obj(
                "LbCl",
                vec![
                    ("Lmnc", DescValue::Double(50.0)),
                    ("A   ", DescValue::Double(-10.0)),
                    ("B   ", DescValue::Double(20.0)),
                ]
            ))
        );
        assert_eq!(
            keys(&parse_color("CMYKColorClass;1;2;3;4").expect("cmyk")),
            ["Cyn ", "Mgnt", "Ylw ", "Blck"]
        );
        let h = parse_color("HSBColorClass;90;50;25").expect("hsb");
        assert_eq!(get(&h, "H   "), &DescValue::Unit("#Ang", 90.0));
        assert_eq!(keys(&h), ["H   ", "Strt", "Brgh"]);
        assert_eq!(
            parse_color("grayscale;40"),
            Some(DescValue::Obj(
                "Grsc",
                vec![("Gry ", DescValue::Double(40.0))]
            ))
        );
        assert_eq!(
            parse_color("bookColor;1;book;name;key"),
            Some(rgb(0.0, 0.0, 0.0))
        );
        assert_eq!(parse_color("nocolor"), None);
        assert_eq!(parse_color(""), None);
        // corner point in a contour
        let c = parse_contour(
            "3$Mine$0.000000;0.000000;1$128.000000;20.000000;0$255.000000;255.000000;1",
        );
        assert_eq!(get(&c, "Nm  "), &DescValue::Text("Mine".into()));
        let DescValue::List(pts) = get(&c, "Crv ") else {
            panic!("list")
        };
        assert_eq!(keys(&pts[0]), ["Hrzn", "Vrtc"]);
        assert_eq!(keys(&pts[1]), ["Hrzn", "Vrtc", "Cnty"]);
        assert_eq!(get(&pts[1], "Cnty"), &DescValue::Bool(false));
        assert_eq!(parse_contour(""), linear_contour());
        assert_eq!(parse_contour("x"), linear_contour());
        // gradient stops
        let g = parse_gradient(TWO_COLOR).expect("gradient");
        assert_eq!(keys(&g), ["Nm  ", "GrdF", "Intr", "Clrs", "Trns"]);
        assert_eq!(g, default_gradient());
        let fg = parse_gradient("FG,customStops,4096$1$1,0$50$foregroundColor,100$0$50")
            .expect("gradient");
        let DescValue::List(cs) = get(&fg, "Clrs") else {
            panic!("list")
        };
        assert_eq!(
            keys(&cs[0]),
            ["Type", "Lctn", "Mdpn"],
            "no colour for a foreground stop"
        );
        assert_eq!(get(&cs[0], "Type"), &DescValue::Enum("Clry", "FrgC"));
        // noise gradient
        let n = parse_gradient("Noise,colorNoise,1,0,7,2048,0,0,0,0,100,100,100,100,RGBColor")
            .expect("noise");
        assert_eq!(
            keys(&n),
            [
                "Nm  ", "GrdF", "ShTr", "VctC", "ClrS", "RndS", "Smth", "Mnm ", "Mxm "
            ]
        );
        assert_eq!(get(&n, "GrdF"), &DescValue::Enum("GrdF", "ClNs"));
        assert_eq!(get(&n, "ClrS"), &DescValue::Enum("ClrS", "RGBC"));
        assert_eq!(get(&n, "RndS"), &DescValue::Long(7));
        assert_eq!(parse_gradient("nogradient"), None);
        // blend modes and enums
        assert_eq!(blend_char_id("linearBurn"), "linearBurn");
        assert_eq!(blend_char_id("hue"), "H   ");
        assert_eq!(blend_char_id("subtract"), "blendSubtraction");
        assert_eq!(blend_char_id("divide"), "blendDivide");
        assert_eq!(blend_char_id("whatever"), "Nrml");
        assert_eq!(enum_char_id("BESl", "pillowEmboss"), "PlEb");
        assert_eq!(enum_char_id("BESl", "unknown"), "InrB");
        // noshape / notexture / nohighlightColor / noshadowColor
        let b = entry(
            "bevelEmboss",
            &format!(
                "1&0&1&1&0&75&75&120&30&100&5&0&screen&nohighlightColor&multiply&noshadowColor&softMatte&innerBevel&stampIn&{LIN}&noshape&notexture"
            ),
        );
        let k = keys(&b);
        for absent in ["hglC", "sdwC", "MpgS", "Ptrn", "phase"] {
            assert!(!k.contains(&absent), "{absent}");
        }
    }

    #[test]
    fn block_order_and_occupancy() {
        // satin and drop shadow set in reverse order: `DrSh` before `ChFX`
        let mut fx = vec![container(&[rec("satin"), rec("dropShadow")])];
        let e = layer_effects(&mut fx).expect("entries");
        assert_eq!(
            e.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
            ["DrSh", "ChFX"]
        );
        assert!(fx.is_empty());
        // a native drop shadow and a container drop shadow: the container wins, the native one
        // stays baked
        let native = Effect::new(EffectKind::DropShadow.moa_id().expect("moa"), true);
        let ds = format!("1&60.000000&0&33.000000&0&5&5&0&0&1&multiply&RGBColor;0;0;0&{LIN}");
        let mut fx = vec![native, container(&[("dropShadow", ds)])];
        let e = layer_effects(&mut fx).expect("entries");
        assert_eq!(e.len(), 1);
        assert_eq!(unit(get(&e[0].1, "lagl")).1, 33.0);
        assert_eq!(fx.len(), 1);
        assert_eq!(fx[0].kind(), EffectKind::DropShadow);
        // two containers with overlapping records: the second stays with its written records
        // switched off
        let on = |n: &str| {
            let (k, v) = rec(n);
            (k, format!("1{}", &v[1..]))
        };
        let a = container(&[rec("dropShadow")]);
        let b = container(&[on("dropShadow"), on("satin")]);
        let mut fx = vec![a, b];
        let e = layer_effects(&mut fx).expect("entries");
        assert_eq!(
            e.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
            ["DrSh", "ChFX"]
        );
        assert_eq!(
            get(&e[0].1, "enab"),
            &DescValue::Bool(false),
            "first container"
        );
        assert_eq!(fx.len(), 1);
        assert!(fx[0].get("dropShadow").expect("ds").starts_with("1&"));
        let satin = fx[0].get("satin").expect("satin");
        assert!(satin.starts_with("0&"));
        assert_eq!(&satin[1..], &on("satin").1[1..]);
    }

    #[test]
    fn global_light_from_the_first_entry() {
        let ds = format!("1&60&1&33&0&5&5&0&0&1&multiply&RGBColor;0;0;0&{LIN}");
        let bv = format!(
            "1&1&0&0&0&75&75&100&45&100&5&0&screen&RGBColor;255;255;255&multiply&RGBColor;0;0;0&softMatte&innerBevel&stampIn&{LIN}&noshape&notexture"
        );
        // the bevel comes later in §3.2 than the drop shadow
        let mut fx = vec![container(&[
            ("bevelEmboss", bv.clone()),
            ("dropShadow", ds),
        ])];
        let (_, light) = effects_block_with_light(&mut fx).expect("block");
        assert_eq!(light, Some((33, 30)));
        let mut fx = vec![container(&[("bevelEmboss", bv)])];
        assert_eq!(
            effects_block_with_light(&mut fx).expect("block").1,
            Some((100, 45))
        );
        let mut fx = vec![container(&default_records())];
        assert_eq!(effects_block_with_light(&mut fx).expect("block").1, None);
        // the resources are written only with a global light
        let comp = fp_raster::RgbaImage::filled(
            fp_geom::IRect::new(0, 0, 2, 2),
            fp_color::Rgba8::new(255, 255, 255, 255),
        )
        .expect("image");
        let res = |light| {
            crate::psd_out::write_psd_with_light(2, 2, &[], &comp, 72.0, &[], light).expect("psd")
        };
        let find = |d: &[u8], id: u16| {
            let mut k = b"8BIM".to_vec();
            k.extend(id.to_be_bytes());
            d.windows(6).position(|w| w == k.as_slice())
        };
        let with = res(Some((33, 30)));
        let p = find(&with, 1037).expect("1037");
        assert_eq!(&with[p + 8..p + 16], &[0, 0, 0, 4, 0, 0, 0, 33]);
        let p = find(&with, 1049).expect("1049");
        assert_eq!(&with[p + 8..p + 16], &[0, 0, 0, 4, 0, 0, 0, 30]);
        let without = res(None);
        assert!(find(&without, 1037).is_none() && find(&without, 1049).is_none());
    }

    /// Minimal descriptor reader for the reference comparison.
    #[derive(Debug, PartialEq)]
    enum Node {
        Obj(String, Vec<(String, Node)>),
        List(Vec<Node>),
        Num(String, f64),
        Other(String, String),
    }

    struct Rd<'a>(&'a [u8], usize);

    impl Rd<'_> {
        fn take(&mut self, n: usize) -> Option<&[u8]> {
            let s = self.0.get(self.1..self.1 + n)?;
            self.1 += n;
            Some(s)
        }
        fn u32(&mut self) -> Option<u32> {
            Some(u32::from_be_bytes(self.take(4)?.try_into().ok()?))
        }
        fn f64(&mut self) -> Option<f64> {
            Some(f64::from_be_bytes(self.take(8)?.try_into().ok()?))
        }
        fn key(&mut self) -> Option<String> {
            let n = self.u32()? as usize;
            let n = if n == 0 { 4 } else { n };
            Some(String::from_utf8_lossy(self.take(n)?).into_owned())
        }
        fn ustr(&mut self) -> Option<String> {
            let n = self.u32()? as usize;
            let b = self.take(n * 2)?;
            let u: Vec<u16> = b
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .take_while(|c| *c != 0)
                .collect();
            Some(String::from_utf16_lossy(&u))
        }
        fn desc(&mut self) -> Option<Node> {
            self.ustr()?;
            let class = self.key()?;
            let n = self.u32()?;
            let mut items = Vec::new();
            for _ in 0..n {
                let k = self.key()?;
                items.push((k, self.value()?));
            }
            Some(Node::Obj(class, items))
        }
        fn value(&mut self) -> Option<Node> {
            let t = String::from_utf8_lossy(self.take(4)?).into_owned();
            Some(match t.as_str() {
                "Objc" | "GlbO" => self.desc()?,
                "VlLs" => {
                    let n = self.u32()?;
                    Node::List((0..n).map(|_| self.value()).collect::<Option<_>>()?)
                }
                "doub" => Node::Num(t, self.f64()?),
                "UntF" => {
                    let u = String::from_utf8_lossy(self.take(4)?).into_owned();
                    Node::Num(format!("UntF {u}"), self.f64()?)
                }
                "long" => Node::Num(t, i32::from_be_bytes(self.take(4)?.try_into().ok()?) as f64),
                "bool" => Node::Other(t, self.take(1)?[0].to_string()),
                "enum" => {
                    let a = self.key()?;
                    let b = self.key()?;
                    Node::Other(t, format!("{a}.{b}"))
                }
                "TEXT" => Node::Other(t, self.ustr()?),
                _ => return None,
            })
        }
    }

    fn same(a: &Node, b: &Node, path: &str, out: &mut Vec<String>) {
        match (a, b) {
            (Node::Obj(ca, ia), Node::Obj(cb, ib)) => {
                if ca != cb {
                    out.push(format!("{path}: class {ca} / {cb}"));
                }
                let ka: Vec<&String> = ia.iter().map(|(k, _)| k).collect();
                let kb: Vec<&String> = ib.iter().map(|(k, _)| k).collect();
                if ka != kb {
                    out.push(format!("{path}: keys {ka:?} / {kb:?}"));
                    return;
                }
                for ((k, x), (_, y)) in ia.iter().zip(ib) {
                    same(x, y, &format!("{path}/{k}"), out);
                }
            }
            (Node::List(la), Node::List(lb)) if la.len() == lb.len() => {
                for (i, (x, y)) in la.iter().zip(lb).enumerate() {
                    same(x, y, &format!("{path}[{i}]"), out);
                }
            }
            (Node::Num(ta, va), Node::Num(tb, vb)) if ta == tb && close(*va, *vb) => {}
            // contour names may differ (spec 79 §9.3)
            (Node::Other(ta, _), Node::Other(tb, _)) if ta == tb && path.ends_with("/Nm  ") => {}
            _ if a == b => {}
            _ => out.push(format!("{path}: {a:?} / {b:?}")),
        }
    }

    /// Compares the `lfx2` of the first layer of a reference PSD (`FP_PSD_REF`, not in the
    /// repository) with what Firepixel writes for the default container: keys, types, units,
    /// order and values (numbers with a tolerance of 1e-6, names of contours excepted).
    #[test]
    #[ignore = "needs a reference PSD in FP_PSD_REF"]
    fn reference_psd_key_order() {
        let Ok(path) = std::env::var("FP_PSD_REF") else {
            eprintln!("FP_PSD_REF not set, skipped");
            return;
        };
        let data = std::fs::read(&path).expect("read FP_PSD_REF");
        let at = data
            .windows(8)
            .position(|w| w == b"8BIMlfx2")
            .expect("an lfx2 block");
        let mut r = Rd(&data, at + 8);
        let _len = r.u32();
        assert_eq!(r.u32(), Some(0), "object effects version");
        assert_eq!(r.u32(), Some(16), "descriptor version");
        let reference = r.desc().expect("reference descriptor");
        let mut fx = vec![container(&default_records())];
        let block = crate::psd_blocks::effects_block(&mut fx).expect("block");
        let mut w = Rd(&block, 8);
        let ours = w.desc().expect("our descriptor");
        let mut diffs = Vec::new();
        same(&reference, &ours, "lfx2", &mut diffs);
        for d in &diffs {
            eprintln!("{d}");
        }
        assert!(diffs.is_empty(), "{} differences", diffs.len());
    }
}
