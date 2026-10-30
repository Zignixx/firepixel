//! Photoshop-Ebenenstile → Original-Live-Effekte (12 §3.7, Laufzeit 7a T5; Parameterformat 05 §9).
//!
//! * Nativ (Tabelle `@01b4fd14`): `solidFill`, `bevelEmboss`, `innerGlow`, `outerGlow`, `innerShadow`,
//!   `dropShadow` — in dieser Reihenfolge wie im Laufzeittest T5. Abschneiden statt Runden (`cvttsd2si`),
//!   Farb-Alpha mit Banker's Rounding, Schattenwinkel + 180, **keine** Kappung des Schlagschatten-Abstands
//!   (T5: 300 bleibt 300); Glühen: `MaskSoftness = (100 − Überfüllen) div 5` (0…20) und
//!   `GlowWidth = trunc(Größe·(100 − Überfüllen)/100)` beim inneren bzw. `·0,9` beim äußeren Glühen
//!   (A: trifft beide T5-Messpunkte 10/50 → 5/10 und 40/25 → 27/15).
//! * Sonst bzw. für den Rest (`gradientFill`, `patternFill`, `frameFX`, `chromeFX`): ein
//!   Photoshop-Live-Effekt `{8eeadf50-…}` mit `&`-Datensätzen je Teil-Effekt; die 250-Kappung des
//!   Schlagschatten-Abstands gilt hier (zurückgeschriebenes PS-Wörterbuch).

use fp_model::effects::format_hex_color;
use fp_model::{Effect, EffectList};

use super::descriptor::{Descriptor, Value};
use super::ps_to_fw_blend;

const MOA_COLOR_FILL: &str = "{dd54adc0-a279-11d3-b92a000502f3fdbe}";
const MOA_DROP_SHADOW: &str = "{a7944db8-6ce2-11d1-8c76000502701850}";
const MOA_INNER_SHADOW: &str = "{5600f702-774c-11d3-baad0000861f4d01}";
const MOA_INNER_GLOW: &str = "{2ba87123-8220-11d3-baad0000861f4d01}";
const MOA_BEVEL: &str = "{7fe61102-6ce2-11d1-8c76000502701850}";
const MOA_PHOTOSHOP: &str = "{8eeadf50-9efe-11da-a7460800200c9a66}";

/// Native Effekte und reine PS-Effekte.
pub const NATIVE_FX: [&str; 6] = [
    "dropShadow",
    "innerShadow",
    "outerGlow",
    "solidFill",
    "innerGlow",
    "bevelEmboss",
];
pub const PS_ONLY_FX: [&str; 4] = ["gradientFill", "patternFill", "frameFX", "chromeFX"];

fn trunc(x: f64) -> i64 {
    if x.is_finite() { x.trunc() as i64 } else { 0 }
}

/// Farbe aus Farbdeskriptor und Deckkraft (`@00aedf20`): Alpha = ROUND(int(opacity)·2,55) (half-even).
pub fn ps_color(d: &Descriptor, ckey: &str, okey: &str) -> fp_color::Argb {
    let Some(c) = d.obj(ckey) else {
        return fp_color::Argb(0xFFFF_FFFF);
    };
    let a = match d.num(okey) {
        Some(op) => ((trunc(op) as f64) * 2.55).round_ties_even() as i64 & 0xFF,
        None => 0xFF,
    } as u32;
    let ch = |k: &str| trunc(c.num(k).unwrap_or(0.0)).clamp(0, 255) as u32;
    fp_color::Argb(a << 24 | ch("red") << 16 | ch("green") << 8 | ch("blue"))
}

fn enabled(fx: &Descriptor, k: &str) -> Option<Descriptor> {
    let d = fx.obj(k)?;
    // spec 79 §3.6: a missing `enab` counts as enabled
    (d.bool("enabled") != Some(false)).then(|| d.clone())
}

/// Native Original-Effekte aus einem `layerEffects`-Deskriptor.
pub fn native_effects(fx: &Descriptor, global_angle: f64) -> Vec<Effect> {
    let mut out = Vec::new();
    let angle = |d: &Descriptor| -> i64 {
        trunc(if d.bool("useGlobalAngle") == Some(true) {
            global_angle
        } else {
            d.num("localLightingAngle").unwrap_or(0.0)
        })
    };
    if let Some(d) = enabled(fx, "solidFill") {
        let mut e = Effect::new(MOA_COLOR_FILL, true);
        e.set("Color", format_hex_color(ps_color(&d, "color", "opacity")));
        e.set(
            "Opacity",
            trunc(d.num("opacity").unwrap_or(100.0)).to_string(),
        );
        e.set(
            "Blendmode",
            ps_to_fw_blend(d.str("mode").unwrap_or("normal")).to_string(),
        );
        out.push(e);
    }
    if let Some(d) = enabled(fx, "bevelEmboss") {
        let style = d.str("bevelStyle").unwrap_or("innerBevel").to_string();
        let hi = trunc(d.num("highlightOpacity").unwrap_or(75.0));
        let sh = trunc(d.num("shadowOpacity").unwrap_or(75.0));
        let mut base = Effect::new(MOA_BEVEL, true);
        base.set(
            "BevelWidth",
            trunc(d.num("blur").unwrap_or(0.0)).to_string(),
        );
        base.set("EdgeThreshold", "0");
        base.set(
            "AngleSoftness",
            trunc(d.num("softness").unwrap_or(0.0)).to_string(),
        );
        base.set(
            "LightAngle",
            (angle(&d)
                + if d.str("bevelDirection") == Some("stampOut") {
                    180
                } else {
                    0
                })
            .to_string(),
        );
        if let Some(t) = d.str("bevelTechnique") {
            base.set("SlopeType", if t == "softMatte" { "1" } else { "0" });
        }
        base.set("SlopeMultiplier", "1");
        base.set(
            "HiliteColor",
            format_hex_color(ps_color(&d, "highlightColor", "highlightOpacity")),
        );
        base.set(
            "ShadowColor",
            format_hex_color(ps_color(&d, "shadowColor", "shadowOpacity")),
        );
        base.set("ButtonState", "0");
        let sr = d.num("strengthRatio");
        let contrast = match sr {
            None => 75,
            Some(_) if style == "outerBevel" => 85,
            Some(s) => {
                let t = trunc(0.25 * hi as f64 + 0.75 * sh as f64);
                trunc((s + 250.0) * t as f64 / 700.0).clamp(0, 100)
            }
        };
        base.set("BevelContrast", contrast.to_string());
        match style.as_str() {
            "innerBevel" => {
                base.set("BevelType", "0");
                out.push(base);
            }
            "outerBevel" => {
                let v = match sr {
                    None => 255,
                    Some(s) => trunc((1.0 - s / 1000.0) * 255.0).clamp(0, 255),
                } as u32;
                let a = (trunc(sh as f64 * 2.55) & 0xFF) as u32;
                base.set("BevelType", "1");
                base.set(
                    "OuterBevelColor",
                    format_hex_color(fp_color::Argb(a << 24 | v << 16 | v << 8 | v)),
                );
                out.push(base);
            }
            "emboss" | "pillowEmboss" => {
                base.set("ShowObject", "1");
                base.set(
                    "MaskSoftness",
                    trunc(d.num("softness").unwrap_or(0.0)).to_string(),
                );
                let mut first = base.clone();
                first.set("BevelType", "0");
                let mut second = base;
                second.set("BevelType", if style == "emboss" { "2" } else { "3" });
                out.push(first);
                out.push(second);
            }
            _ => {}
        }
    }
    for (k, outer) in [("innerGlow", false), ("outerGlow", true)] {
        let Some(d) = enabled(fx, k) else { continue };
        let mut e = Effect::new(if outer { MOA_BEVEL } else { MOA_INNER_GLOW }, true);
        e.set(
            "BevelContrast",
            trunc(d.num("opacity").unwrap_or(0.0)).to_string(),
        );
        if outer {
            // Laufzeit T5: das innere Glühen hat keinen EdgeThreshold.
            e.set("EdgeThreshold", "0");
        }
        e.set(
            "OuterBevelColor",
            format_hex_color(ps_color(&d, "color", "opacity")),
        );
        let choke = d.num("chokeMatte");
        e.set(
            "MaskSoftness",
            choke
                .map(|c| ((100 - trunc(c)) / 5).clamp(0, 20))
                .unwrap_or(5)
                .to_string(),
        );
        let width = match d.num("blur") {
            None => 5,
            Some(b) => {
                let spread = (100.0 - choke.map(|c| trunc(c) as f64).unwrap_or(0.0)) / 100.0;
                trunc(b * spread * if outer { 0.9 } else { 1.0 }).clamp(0, 35)
            }
        };
        e.set("GlowWidth", width.to_string());
        if let Some(t) = d.str("bevelTechnique") {
            e.set("SlopeType", if t == "softMatte" { "1" } else { "0" });
        }
        if outer {
            e.set("ButtonState", "0");
            e.set("BevelType", "4");
        } else {
            e.set("GlowStartDistance", "0");
        }
        out.push(e);
    }
    for k in ["innerShadow", "dropShadow"] {
        let Some(d) = enabled(fx, k) else { continue };
        let mut e = Effect::new(
            if k == "dropShadow" {
                MOA_DROP_SHADOW
            } else {
                MOA_INNER_SHADOW
            },
            true,
        );
        e.set("ShadowType", "0");
        e.set(
            "ShadowColor",
            format_hex_color(ps_color(&d, "color", "opacity")),
        );
        e.set(
            "ShadowBlur",
            trunc(d.num("blur").unwrap_or(0.0)).to_string(),
        );
        e.set(
            "ShadowDistance",
            trunc(d.num("distance").unwrap_or(0.0)).to_string(),
        );
        e.set("ShadowAngle", (angle(&d) + 180).to_string());
        out.push(e);
    }
    for e in out.iter_mut().filter(|e| e.moa_id() == Some(MOA_BEVEL)) {
        for (k, v) in BEVEL_DEFAULTS {
            if e.params.get(k).is_none() {
                e.set(k, v);
            }
        }
    }
    out
}

/// Vorgabewerte, die das Original beim PSD-Import für nicht aus dem Ebenenstil abgeleitete Parameter des
/// Kanten-/Glüh-Effekts schreibt (Laufzeit T5, `T5_effekte_nativ.fw.png`).
const BEVEL_DEFAULTS: [(&str, &str); 16] = [
    ("SlopeType", "0"),
    ("HiliteColor", "#ffffff"),
    ("ShowObject", "false"),
    ("LightAngle", "135"),
    ("BevelWidth", "10"),
    ("ShadowColor", "#000000"),
    ("GlowWidth", "0"),
    ("GlowStartDistance", "0"),
    ("EmbossFaceColor", "#ffffff00"),
    ("HitBlendColor", "#ffffff3f"),
    ("MaskSoftness", "0"),
    ("DownBlendColor", "#0000003f"),
    ("AngleSoftness", "25"),
    ("LightDistance", "100"),
    ("SlopeMultiplier", "1"),
    ("OuterBevelColor", "#df0000"),
];

// ---------------------------------------------------------------- Photoshop-Live-Effekt (05 §9)
//
// F150 (spec 79 §4, §6): every `lfx2` layer effect becomes a record of the PSD live effect container.
// Missing or unreadable values take the defaults of spec 79 §4 (the values of the PSD application's own
// default layer style); a missing `enab` in a present effect counts as enabled (§3.6).

fn f6(v: f64) -> String {
    format!("{v:.6}")
}

/// Linear contour (default of every contour key except the satin's, spec 79 §3.5).
const CONTOUR_LINEAR: &str = "2$$0.000000;0.000000;1$255.000000;255.000000;1";
/// Gaussian contour (default of the satin's `MpgS`, spec 79 §3.5).
const CONTOUR_GAUSSIAN: &str = "9$$0.000000;0.000000;1$32.000000;7.000000;1$64.000000;38.000000;1\
$96.000000;101.000000;1$128.000000;166.000000;1$159.000000;209.000000;1$191.000000;235.000000;1\
$223.000000;248.000000;1$255.000000;255.000000;1";
const RGB_BLACK: &str = "RGBColor;0.000000;0.000000;0.000000";
const RGB_WHITE: &str = "RGBColor;255.000000;255.000000;255.000000";
const RGB_RED: &str = "RGBColor;255.000000;0.000000;0.000000";
/// Glow colour 255/255/189.99610894941634 (spec 79 §4.3, value of the default file).
const RGB_GLOW: &str = "RGBColor;255.000000;255.000000;189.996109";
/// Gradient "Two Color" black → white (default of `GrFl` and of a gradient `FrFX`, spec 79 §4.8).
const GRADIENT_TWO_COLOR: &str = "Two_Color,customStops,4096.000000$2$2,\
0$50$userStop$RGBColor;0.000000;0.000000;0.000000,\
4096$50$userStop$RGBColor;255.000000;255.000000;255.000000,100.000000$0$50,100.000000$4096$50";
/// Default pattern (spec 79 §4.9).
const PATTERN_NAME: &str = "Bubbles";
const PATTERN_ID: &str = "b7334da0-122f-11d4-8bb5-e27e45023b5f";

/// The ten layer effects (descriptor string IDs) in the order of spec 79 §3.2.
pub const LAYER_EFFECTS: [&str; 10] = [
    "dropShadow",
    "innerShadow",
    "outerGlow",
    "solidFill",
    "gradientFill",
    "patternFill",
    "frameFX",
    "innerGlow",
    "bevelEmboss",
    "chromeFX",
];

/// Multi-instance lists of newer PSD versions → single effect (spec 79 §6.6).
const MULTI_FX: [(&str, &str); 5] = [
    ("dropShadowMulti", "dropShadow"),
    ("innerShadowMulti", "innerShadow"),
    ("solidFillMulti", "solidFill"),
    ("gradientFillMulti", "gradientFill"),
    ("frameFXMulti", "frameFX"),
];

/// Global light of the file: image resources 1037 (angle) and 1049 (altitude), spec 79 §6.4.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlobalLight {
    pub angle: Option<f64>,
    pub altitude: Option<f64>,
}

fn num(d: &Descriptor, k: &str, def: f64) -> String {
    f6(d.num(k).filter(|v| v.is_finite()).unwrap_or(def))
}
fn flag(d: &Descriptor, k: &str, def: bool) -> &'static str {
    if d.bool(k).unwrap_or(def) { "1" } else { "0" }
}
fn text<'a>(d: &'a Descriptor, k: &str, def: &'a str) -> &'a str {
    d.str(k).unwrap_or(def)
}
/// Coordinate of a point object (`Ofst`, `phase`).
fn point(d: &Descriptor, obj: &str, k: &str) -> String {
    f6(d.obj(obj)
        .and_then(|x| x.num(k))
        .filter(|v| v.is_finite())
        .unwrap_or(0.0))
}
/// Name or ID of the pattern object (`Ptrn`).
fn pattern<'a>(d: &'a Descriptor, k: &str, def: &'a str) -> &'a str {
    d.obj("pattern").and_then(|p| p.str(k)).unwrap_or(def)
}

/// Farbe `RGBColor;r;g;b` (bzw. `HSBColorClass`, `CMYKColorClass`, `labColor`, `grayscale`); `none`, wenn
/// der Schlüssel fehlt.
fn color_str(d: &Descriptor, key: &str, none: &str) -> String {
    let Some(c) = d.obj(key) else {
        return none.to_string();
    };
    let n = |k: &str| f6(c.num(k).filter(|v| v.is_finite()).unwrap_or(0.0));
    match c.class.as_str() {
        "HSBColorClass" | "HSBC" => format!(
            "HSBColorClass;{};{};{}",
            n("hue"),
            n("saturation"),
            n("brightness")
        ),
        "CMYKColorClass" | "CMYC" => format!(
            "CMYKColorClass;{};{};{};{}",
            n("cyan"),
            n("magenta"),
            n("yellowColor"),
            n("black")
        ),
        "labColor" | "LbCl" => format!("labColor;{};{};{}", n("luminance"), n("a"), n("b")),
        "grayscale" | "Grsc" => format!("grayscale;{}", n("gray")),
        _ => format!("RGBColor;{};{};{}", n("red"), n("green"), n("blue")),
    }
}

/// Kontur `N$name$h;v;c$…` (`def`, wenn keine Kurve angegeben ist). Das Original schreibt den Namen leer
/// (Laufzeit 12 T5: `2$$…` auch für „Linear“, `9$$…` für „Gaussian“). `Cnty` fehlt = Kurvenpunkt (1).
fn contour_str(d: &Descriptor, key: &str, def: &str) -> String {
    let Some(c) = d.obj(key) else {
        return def.to_string();
    };
    let Some(Value::List(pts)) = c.get("curve") else {
        return def.to_string();
    };
    let mut s = format!("{}$", pts.len());
    for p in pts {
        let Some(p) = p.as_obj() else { continue };
        let cont = p
            .bool("continuity")
            .map(|v| if v { 1 } else { 0 })
            .unwrap_or(1);
        s.push_str(&format!(
            "${};{};{}",
            f6(p.num("horizontal").unwrap_or(0.0)),
            f6(p.num("vertical").unwrap_or(0.0)),
            cont
        ));
    }
    s
}

/// Gradient name as the reference reads it: every non-alphanumeric ASCII character → `_`
/// (spec 79 §6.2).
pub fn gradient_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_alphanumeric() {
                '_'
            } else {
                c
            }
        })
        .collect()
}

/// Verlauf `name,form,interp$nC$nT,` + Stopps (05 §9.3; Farbstopps `location$midpoint$type[$farbe]`);
/// Rauschverlauf `name,colorNoise,ShTr,VctC,RndS,Smth,mn×4,mx×4,ClrS` (spec 79 §3.5). `none`, wenn der
/// Schlüssel fehlt.
fn gradient_str(d: &Descriptor, key: &str, none: &str) -> String {
    let Some(g) = d.obj(key) else {
        return none.to_string();
    };
    let name = gradient_name(g.str("name").unwrap_or(""));
    let form = g.str("gradientForm").unwrap_or("customStops");
    if form == "colorNoise" {
        let int = |v: Option<f64>, def: i64| {
            v.filter(|v| v.is_finite())
                .map(|v| v.trunc() as i64)
                .unwrap_or(def)
        };
        let four = |k: &str, def: i64| -> Vec<String> {
            let vals: Vec<Option<f64>> = match g.get(k) {
                Some(Value::List(v)) => v.iter().map(Value::num).collect(),
                _ => Vec::new(),
            };
            (0..4)
                .map(|i| int(vals.get(i).copied().flatten(), def).to_string())
                .collect()
        };
        let mut parts = vec![
            name,
            "colorNoise".to_string(),
            flag(g, "showTransparency", false).to_string(),
            flag(g, "vectorColor", true).to_string(),
            int(g.num("randomSeed"), 0).to_string(),
            int(g.num("smoothness"), 2048).to_string(),
        ];
        parts.extend(four("minimum", 0));
        parts.extend(four("maximum", 100));
        parts.push(text(g, "colorSpace", "RGBColor").to_string());
        return parts.join(",");
    }
    let colors = match g.get("colors") {
        Some(Value::List(v)) => v.clone(),
        _ => vec![],
    };
    let trans = match g.get("transparency") {
        Some(Value::List(v)) => v.clone(),
        _ => vec![],
    };
    // Teile durch Kommas getrennt (ohne Schlusskomma).
    let mut parts = vec![
        name,
        form.to_string(),
        format!(
            "{}${}${}",
            f6(g.num("interpolation").unwrap_or(4096.0)),
            colors.len(),
            trans.len()
        ),
    ];
    for c in &colors {
        let Some(c) = c.as_obj() else { continue };
        let ty = c.str("type").unwrap_or("userStop");
        let mut s = format!(
            "{}${}${ty}",
            c.num("location").map(|v| v as i64).unwrap_or(0),
            c.num("midpoint").map(|v| v as i64).unwrap_or(50)
        );
        if ty == "userStop" {
            s.push('$');
            s.push_str(&color_str(c, "color", "nocolor"));
        }
        parts.push(s);
    }
    for t in &trans {
        let Some(t) = t.as_obj() else { continue };
        parts.push(format!(
            "{}${}${}",
            f6(t.num("opacity").unwrap_or(100.0)),
            t.num("location").map(|v| v as i64).unwrap_or(0),
            t.num("midpoint").map(|v| v as i64).unwrap_or(50)
        ));
    }
    parts.join(",")
}

/// `&`-Datensatz eines Teil-Effekts (Feldtabellen 05 §9.2, Vorgaben spec 79 §4); `None` für unbekannte
/// Effekte.
pub fn ps_record(name: &str, d: &Descriptor) -> Option<String> {
    let en = flag(d, "enabled", true);
    let contour = |k: &str| contour_str(d, k, CONTOUR_LINEAR);
    Some(match name {
        "dropShadow" | "innerShadow" => {
            let drop = name == "dropShadow";
            let mut dist = d.num("distance").filter(|v| v.is_finite()).unwrap_or(5.0);
            if drop {
                dist = dist.min(250.0);
            }
            [
                en.to_string(),
                num(d, "opacity", 75.0),
                flag(d, "useGlobalAngle", false).to_string(),
                num(d, "localLightingAngle", if drop { 120.0 } else { -135.0 }),
                num(d, "noise", 0.0),
                f6(dist),
                num(d, "blur", 5.0),
                num(d, "chokeMatte", 0.0),
                flag(d, "antiAlias", false).to_string(),
                if drop {
                    flag(d, "layerConceals", true)
                } else {
                    "0"
                }
                .to_string(),
                text(d, "mode", "multiply").to_string(),
                color_str(d, "color", RGB_BLACK),
                contour("transparencyShape"),
            ]
            .join("&")
        }
        "outerGlow" | "innerGlow" => {
            // A glow with a gradient and no colour keeps `nocolor` (the export leaves `Clr ` out for it).
            let color = if d.get("color").is_none() && d.get("gradient").is_some() {
                "nocolor".to_string()
            } else {
                color_str(d, "color", RGB_GLOW)
            };
            [
                en.to_string(),
                flag(d, "antiAlias", false).to_string(),
                text(d, "mode", "screen").to_string(),
                [
                    num(d, "opacity", 75.0),
                    num(d, "chokeMatte", 0.0),
                    num(d, "blur", 5.0),
                    num(d, "noise", 0.0),
                    num(d, "shadingNoise", 0.0),
                    num(d, "inputRange", 50.0),
                ]
                .join(":"),
                text(d, "glowTechnique", "softMatte").to_string(),
                color,
                gradient_str(d, "gradient", "nogradient"),
                contour("transparencyShape"),
                if name == "innerGlow" {
                    text(d, "innerGlowSource", "edgeGlow")
                } else {
                    ""
                }
                .to_string(),
            ]
            .join("&")
        }
        "bevelEmboss" => {
            let use_shape = d.bool("useShape") == Some(true);
            let use_texture = d.bool("useTexture") == Some(true);
            // The shape and texture blocks are kept whenever their keys are present (spec 79 §4.5).
            let shape = if use_shape || d.get("mappingShape").is_some() {
                format!(
                    "{},{},{}",
                    flag(d, "antiAlias", false),
                    num(d, "inputRange", 50.0),
                    contour("mappingShape")
                )
            } else {
                "noshape".into()
            };
            let texture = if use_texture || d.get("pattern").is_some() {
                format!(
                    "{},{},{},{},{},{},{},{}",
                    flag(d, "invertTexture", false),
                    flag(d, "align", true),
                    num(d, "scale", 100.0),
                    num(d, "textureDepth", 100.0),
                    point(d, "phase", "horizontal"),
                    point(d, "phase", "vertical"),
                    pattern(d, "name", PATTERN_NAME),
                    pattern(d, "ID", PATTERN_ID)
                )
            } else {
                "notexture".into()
            };
            [
                en.to_string(),
                flag(d, "useGlobalAngle", false).to_string(),
                flag(d, "useShape", false).to_string(),
                flag(d, "useTexture", false).to_string(),
                flag(d, "antialiasGloss", false).to_string(),
                num(d, "highlightOpacity", 75.0),
                num(d, "shadowOpacity", 75.0),
                num(d, "localLightingAngle", 120.0),
                num(d, "localLightingAltitude", 30.0),
                num(d, "strengthRatio", 100.0),
                num(d, "blur", 5.0),
                num(d, "softness", 0.0),
                text(d, "highlightMode", "screen").to_string(),
                color_str(d, "highlightColor", RGB_WHITE),
                text(d, "shadowMode", "multiply").to_string(),
                color_str(d, "shadowColor", RGB_BLACK),
                text(d, "bevelTechnique", "softMatte").to_string(),
                text(d, "bevelStyle", "innerBevel").to_string(),
                text(d, "bevelDirection", "stampIn").to_string(),
                contour("transparencyShape"),
                shape,
                texture,
            ]
            .join("&")
        }
        "chromeFX" => [
            en.to_string(),
            flag(d, "antiAlias", false).to_string(),
            flag(d, "invert", true).to_string(),
            num(d, "opacity", 50.0),
            num(d, "localLightingAngle", 19.0),
            num(d, "distance", 11.0),
            num(d, "blur", 14.0),
            text(d, "mode", "multiply").to_string(),
            color_str(d, "color", RGB_BLACK),
            contour_str(d, "mappingShape", CONTOUR_GAUSSIAN),
        ]
        .join("&"),
        "solidFill" => [
            en.to_string(),
            num(d, "opacity", 100.0),
            text(d, "mode", "normal").to_string(),
            color_str(d, "color", RGB_RED),
        ]
        .join("&"),
        "gradientFill" => [
            en.to_string(),
            flag(d, "reverse", false).to_string(),
            flag(d, "align", true).to_string(),
            num(d, "opacity", 100.0),
            num(d, "angle", 90.0),
            num(d, "scale", 100.0),
            point(d, "offset", "horizontal"),
            point(d, "offset", "vertical"),
            text(d, "mode", "normal").to_string(),
            text(d, "type", "linear").to_string(),
            gradient_str(d, "gradient", GRADIENT_TWO_COLOR),
        ]
        .join("&"),
        "patternFill" => [
            en.to_string(),
            flag(d, "align", true).to_string(),
            num(d, "opacity", 100.0),
            num(d, "scale", 100.0),
            point(d, "phase", "horizontal"),
            point(d, "phase", "vertical"),
            text(d, "mode", "normal").to_string(),
            pattern(d, "name", PATTERN_NAME).to_string(),
            pattern(d, "ID", PATTERN_ID).to_string(),
        ]
        .join("&"),
        "frameFX" => {
            let paint = text(d, "paintType", "solidColor");
            let mut s = [
                en.to_string(),
                num(d, "opacity", 100.0),
                num(d, "size", 3.0),
                text(d, "mode", "normal").to_string(),
                color_str(d, "color", RGB_RED),
                text(d, "style", "outsetFrame").to_string(),
                paint.to_string(),
                String::new(),
            ]
            .join("&");
            match paint {
                "gradientFill" => s.push_str(
                    &[
                        flag(d, "reverse", false).to_string(),
                        flag(d, "align", true).to_string(),
                        num(d, "angle", 90.0),
                        num(d, "scale", 100.0),
                        point(d, "offset", "horizontal"),
                        point(d, "offset", "vertical"),
                        text(d, "type", "linear").to_string(),
                        gradient_str(d, "gradient", GRADIENT_TWO_COLOR),
                    ]
                    .join("&"),
                ),
                "pattern" => s.push_str(
                    &[
                        flag(d, "linked", true).to_string(),
                        num(d, "scale", 100.0),
                        point(d, "phase", "horizontal"),
                        point(d, "phase", "vertical"),
                        pattern(d, "name", PATTERN_NAME).to_string(),
                        pattern(d, "ID", PATTERN_ID).to_string(),
                    ]
                    .join("&"),
                ),
                _ => {}
            }
            s
        }
        _ => return None,
    })
}

/// PS-Effektname (Deskriptor) → Name im Live-Effekt (05 §9.1).
fn live_name(k: &str) -> &str {
    match k {
        "chromeFX" => "satin",
        "frameFX" => "stroke",
        other => other,
    }
}

/// Multiplies a number value in place (unit kept).
fn scale_value(d: &mut Descriptor, k: &str, f: f64) {
    if let Some((_, v)) = d.items.iter_mut().find(|(n, _)| n == k) {
        match v {
            Value::Unit { value, .. } | Value::Double(value) => *value *= f,
            Value::Long(l) => *v = Value::Double(*l as f64 * f),
            _ => {}
        }
    }
}

/// Layer effects whose further instances (`…Multi` lists with more than one entry) are dropped by the
/// import (spec 79 §6.6), as descriptor string IDs.
pub fn multi_losses(fx: &Descriptor) -> Vec<&'static str> {
    MULTI_FX
        .iter()
        .filter(|(m, _)| matches!(fx.get(m), Some(Value::List(v)) if v.len() > 1))
        .map(|(_, s)| *s)
        .collect()
}

/// Normalised `layerEffects` descriptor (spec 79 §6.3, §6.4, §6.6): a `…Multi` list gives its first
/// enabled entry (else the first) when the single effect is missing; `Scl ` ≠ 100 % is applied to the
/// pixel values (`Dstn`, `blur`, `Sftn`, `Sz  `) and the pattern/texture scale of `patternFill`, `ebbl`
/// and a pattern `FrFX` (never to `Ckmt`, `Inpr`, opacities, angles or a gradient scale) and then set to
/// 100 %; an effect with `uglg` takes the global angle 1037 (and the bevel the global altitude 1049) when
/// the file has the resource.
pub fn normalize(fx: &Descriptor, light: GlobalLight) -> Descriptor {
    let mut fx = fx.clone();
    for (m, s) in MULTI_FX {
        if fx.get(s).is_some() {
            continue;
        }
        let Some(Value::List(v)) = fx.get(m) else {
            continue;
        };
        let objs: Vec<&Descriptor> = v.iter().filter_map(Value::as_obj).collect();
        let pick = objs
            .iter()
            .find(|d| d.bool("enabled") != Some(false))
            .or(objs.first())
            .map(|d| (*d).clone());
        if let Some(p) = pick {
            fx.set(s, Value::Object(p));
        }
    }
    let scale = fx
        .num("scale")
        .filter(|v| v.is_finite() && *v > 0.0)
        .unwrap_or(100.0);
    let f = scale / 100.0;
    for k in LAYER_EFFECTS {
        let Some((_, Value::Object(d))) = fx.items.iter_mut().find(|(n, _)| n == k) else {
            continue;
        };
        if f != 1.0 {
            for key in ["distance", "blur", "softness", "size"] {
                scale_value(d, key, f);
            }
            let pattern_scale = match k {
                "patternFill" | "bevelEmboss" => true,
                "frameFX" => d.str("paintType") == Some("pattern"),
                _ => false,
            };
            if pattern_scale {
                scale_value(d, "scale", f);
            }
        }
        if d.bool("useGlobalAngle") == Some(true) {
            if let Some(a) = light.angle {
                d.set(
                    "localLightingAngle",
                    Value::Unit {
                        unit: "angleUnit".into(),
                        value: a,
                    },
                );
            }
            if k == "bevelEmboss"
                && let Some(h) = light.altitude
            {
                d.set(
                    "localLightingAltitude",
                    Value::Unit {
                        unit: "angleUnit".into(),
                        value: h,
                    },
                );
            }
        }
    }
    if fx.get("scale").is_some() {
        fx.set(
            "scale",
            Value::Unit {
                unit: "percentUnit".into(),
                value: 100.0,
            },
        );
    }
    fx
}

/// Photoshop-Live-Effekt mit allen bekannten Teil-Effekten, für die `include` gilt (abgeschaltete werden
/// mitgenommen); `None`, wenn keiner eingeschaltet ist. Erwartet einen normalisierten Deskriptor
/// ([`normalize`]).
pub fn photoshop_live_effect(fx: &Descriptor, include: &dyn Fn(&str) -> bool) -> Option<Effect> {
    let mut e = Effect::new(MOA_PHOTOSHOP, true);
    let mut any = false;
    for k in [
        "dropShadow",
        "innerShadow",
        "outerGlow",
        "innerGlow",
        "bevelEmboss",
        "chromeFX",
        "solidFill",
        "gradientFill",
        "patternFill",
        "frameFX",
    ] {
        let Some(d) = fx.obj(k) else { continue };
        if !include(k) {
            continue;
        }
        if let Some(rec) = ps_record(k, d) {
            e.set(live_name(k), rec);
            any |= d.bool("enabled") != Some(false);
        }
    }
    any.then_some(e)
}

/// Effektliste eines Elements nach `UseFWFilters` (spec 79 §6). `masterFXSwitch` = false: the effects are
/// imported switched off (spec 79 §10 question 8, safe default: nothing is lost, nothing is drawn).
pub fn effect_list(fx: &Descriptor, native_first: bool, light: GlobalLight) -> EffectList {
    let fx = normalize(fx, light);
    let mut list = EffectList::default();
    if native_first {
        list.effects
            .extend(native_effects(&fx, light.angle.unwrap_or(30.0)));
        // Nativ umgesetzte Effekte werden im PS-Wörterbuch abgeschaltet; Rest als PS-Live-Effekt.
        let rest = |k: &str| {
            PS_ONLY_FX.contains(&k) && fx.obj(k).and_then(|d| d.bool("enabled")) != Some(false)
        };
        if let Some(e) = photoshop_live_effect(&fx, &rest) {
            list.effects.push(e);
        }
    } else if let Some(e) = photoshop_live_effect(&fx, &|_| true) {
        list.effects.push(e);
    }
    if fx.bool("masterFXSwitch") == Some(false) {
        for e in &mut list.effects {
            e.set("EffectIsVisible", "false");
        }
    }
    list
}

#[cfg(test)]
mod lfx2_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(items: Vec<(&str, Value)>) -> Descriptor {
        Descriptor {
            name: String::new(),
            class: "null".into(),
            items: items.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        }
    }

    #[test]
    fn drop_shadow_like_t5() {
        let color = obj(vec![
            ("red", Value::Double(0.0)),
            ("green", Value::Double(0.0)),
            ("blue", Value::Double(0.0)),
        ]);
        let ds = obj(vec![
            ("enabled", Value::Bool(true)),
            ("color", Value::Object(color)),
            (
                "opacity",
                Value::Unit {
                    unit: "percentUnit".into(),
                    value: 60.0,
                },
            ),
            ("useGlobalAngle", Value::Bool(false)),
            (
                "localLightingAngle",
                Value::Unit {
                    unit: "angleUnit".into(),
                    value: 120.0,
                },
            ),
            (
                "distance",
                Value::Unit {
                    unit: "pixelsUnit".into(),
                    value: 300.0,
                },
            ),
            (
                "blur",
                Value::Unit {
                    unit: "pixelsUnit".into(),
                    value: 7.0,
                },
            ),
        ]);
        let fx = obj(vec![("dropShadow", Value::Object(ds))]);
        let n = native_effects(&fx, 30.0);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].get("ShadowAngle"), Some("300"));
        assert_eq!(n[0].get("ShadowDistance"), Some("300"));
        assert_eq!(n[0].get("ShadowColor"), Some("#00000099"));
        assert_eq!(n[0].get("ShadowBlur"), Some("7"));
        let live = photoshop_live_effect(&fx, &|_| true).unwrap();
        assert!(
            live.get("dropShadow")
                .unwrap()
                .starts_with("1&60.000000&0&120.000000&0.000000&250.000000&7.000000")
        );
    }
}
