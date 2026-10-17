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
    (d.bool("enabled") == Some(true)).then(|| d.clone())
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
fn f6(v: f64) -> String {
    format!("{v:.6}")
}
fn b01(v: Option<bool>) -> &'static str {
    if v == Some(true) { "1" } else { "0" }
}

/// Farbe `RGBColor;r;g;b` (bzw. `HSBColorClass`, `CMYKColorClass`, `labColor`, `grayscale`).
fn color_str(d: &Descriptor, key: &str, none: &str) -> String {
    let Some(c) = d.obj(key) else {
        return none.to_string();
    };
    let n = |k: &str| f6(c.num(k).unwrap_or(0.0));
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

/// Kontur `N$name$h;v;c$…` (linear, wenn keine Kurve angegeben ist). Das Original schreibt den Namen leer
/// (Laufzeit 12 T5: `2$$…` auch für „Linear“, `9$$…` für „Gaussian“).
fn contour_str(d: &Descriptor, key: &str) -> String {
    let lin = "2$$0.000000;0.000000;1$255.000000;255.000000;1".to_string();
    let Some(c) = d.obj(key) else { return lin };
    let Some(Value::List(pts)) = c.get("curve") else {
        return lin;
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

/// Verlauf `name,form,interp$nC$nT,` + Stopps (05 §9.3; Farbstopps `location$midpoint$type[$farbe]`).
fn gradient_str(d: &Descriptor, key: &str) -> String {
    let Some(g) = d.obj(key) else {
        return "nogradient".to_string();
    };
    let name = g.str("name").unwrap_or("");
    let form = g.str("gradientForm").unwrap_or("customStops");
    let colors = match g.get("colors") {
        Some(Value::List(v)) => v.clone(),
        _ => vec![],
    };
    let trans = match g.get("transparency") {
        Some(Value::List(v)) => v.clone(),
        _ => vec![],
    };
    // Laufzeit 12 T5: Leerzeichen im Namen als `_`, Teile durch Kommas getrennt (ohne Schlusskomma).
    let mut parts = vec![
        name.replace(' ', "_"),
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
/// `&`-Datensatz eines Teil-Effekts (Feldtabellen 05 §9.2); `None` für unbekannte Effekte.
pub fn ps_record(name: &str, d: &Descriptor) -> Option<String> {
    let n = |k: &str, def: f64| f6(d.num(k).unwrap_or(def));
    let mode = d.str("mode").unwrap_or("normal");
    Some(match name {
        "dropShadow" | "innerShadow" => {
            let mut dist = d.num("distance").unwrap_or(0.0);
            if name == "dropShadow" {
                dist = dist.min(250.0);
            }
            format!(
                "{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}",
                b01(d.bool("enabled")),
                n("opacity", 75.0),
                b01(d.bool("useGlobalAngle")),
                n("localLightingAngle", 120.0),
                n("noise", 0.0),
                f6(dist),
                n("blur", 5.0),
                n("chokeMatte", 0.0),
                b01(d.bool("antiAlias")),
                if name == "dropShadow" {
                    b01(d.bool("layerConceals"))
                } else {
                    "0"
                },
                mode,
                color_str(d, "color", "nocolor"),
                contour_str(d, "transparencyShape")
            )
        }
        "outerGlow" | "innerGlow" => format!(
            "{}&{}&{}&{}:{}:{}:{}:{}:{}&{}&{}&{}&{}&{}",
            b01(d.bool("enabled")),
            b01(d.bool("antiAlias")),
            mode,
            n("opacity", 75.0),
            n("chokeMatte", 0.0),
            n("blur", 5.0),
            n("noise", 0.0),
            n("shadingNoise", 0.0),
            n("inputRange", 50.0),
            d.str("glowTechnique").unwrap_or("softMatte"),
            color_str(d, "color", "nocolor"),
            gradient_str(d, "gradient"),
            contour_str(d, "transparencyShape"),
            if name == "innerGlow" {
                d.str("innerGlowSource").unwrap_or("edgeGlow")
            } else {
                ""
            }
        ),
        "bevelEmboss" => format!(
            "{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}",
            b01(d.bool("enabled")),
            b01(d.bool("useGlobalAngle")),
            b01(d.bool("useShape")),
            b01(d.bool("useTexture")),
            b01(d.bool("antialiasGloss")),
            n("highlightOpacity", 75.0),
            n("shadowOpacity", 75.0),
            n("localLightingAngle", 120.0),
            n("localLightingAltitude", 30.0),
            n("strengthRatio", 100.0),
            n("blur", 5.0),
            n("softness", 0.0),
            d.str("highlightMode").unwrap_or("screen"),
            color_str(d, "highlightColor", "nohighlightColor"),
            d.str("shadowMode").unwrap_or("multiply"),
            color_str(d, "shadowColor", "noshadowColor"),
            d.str("bevelTechnique").unwrap_or("softMatte"),
            d.str("bevelStyle").unwrap_or("innerBevel"),
            d.str("bevelDirection").unwrap_or("stampIn"),
            contour_str(d, "transparencyShape"),
            if d.bool("useShape") == Some(true) {
                format!(
                    "{},{},{}",
                    b01(d.bool("antiAlias")),
                    n("inputRange", 50.0),
                    contour_str(d, "mappingShape")
                )
            } else {
                "noshape".into()
            },
            if d.bool("useTexture") == Some(true) {
                let p = d.obj("pattern");
                format!(
                    "{},{},{},{},{},{},{},{}",
                    b01(d.bool("invertTexture")),
                    b01(d.bool("align")),
                    n("scale", 100.0),
                    n("textureDepth", 100.0),
                    f6(d.obj("phase")
                        .and_then(|x| x.num("horizontal"))
                        .unwrap_or(0.0)),
                    f6(d.obj("phase")
                        .and_then(|x| x.num("vertical"))
                        .unwrap_or(0.0)),
                    p.and_then(|x| x.str("name")).unwrap_or(""),
                    p.and_then(|x| x.str("ID")).unwrap_or("")
                )
            } else {
                "notexture".into()
            }
        ),
        "chromeFX" => format!(
            "{}&{}&{}&{}&{}&{}&{}&{}&{}&{}",
            b01(d.bool("enabled")),
            b01(d.bool("antiAlias")),
            b01(d.bool("invert")),
            n("opacity", 50.0),
            n("localLightingAngle", 19.0),
            n("distance", 11.0),
            n("blur", 14.0),
            mode,
            color_str(d, "color", "nocolor"),
            contour_str(d, "mappingShape")
        ),
        "solidFill" => format!(
            "{}&{}&{}&{}",
            b01(d.bool("enabled")),
            n("opacity", 100.0),
            mode,
            color_str(d, "color", "nocolor")
        ),
        "gradientFill" => format!(
            "{}&{}&{}&{}&{}&{}&{}&{}&{}&{}&{}",
            b01(d.bool("enabled")),
            b01(d.bool("reverse")),
            b01(d.bool("align")),
            n("opacity", 100.0),
            n("angle", 90.0),
            n("scale", 100.0),
            f6(d.obj("offset")
                .and_then(|x| x.num("horizontal"))
                .unwrap_or(0.0)),
            f6(d.obj("offset")
                .and_then(|x| x.num("vertical"))
                .unwrap_or(0.0)),
            mode,
            d.str("type").unwrap_or("linear"),
            gradient_str(d, "gradient")
        ),
        "patternFill" => {
            let p = d.obj("pattern");
            format!(
                "{}&{}&{}&{}&{}&{}&{}&{}&{}",
                b01(d.bool("enabled")),
                b01(d.bool("align")),
                n("opacity", 100.0),
                n("scale", 100.0),
                f6(d.obj("phase")
                    .and_then(|x| x.num("horizontal"))
                    .unwrap_or(0.0)),
                f6(d.obj("phase")
                    .and_then(|x| x.num("vertical"))
                    .unwrap_or(0.0)),
                mode,
                p.and_then(|x| x.str("name")).unwrap_or(""),
                p.and_then(|x| x.str("ID")).unwrap_or("")
            )
        }
        "frameFX" => {
            let paint = d.str("paintType").unwrap_or("solidColor");
            let mut s = format!(
                "{}&{}&{}&{}&{}&{}&{}&",
                b01(d.bool("enabled")),
                n("opacity", 100.0),
                n("size", 3.0),
                mode,
                color_str(d, "color", "nocolor"),
                d.str("style").unwrap_or("outsetFrame"),
                paint
            );
            match paint {
                "gradientFill" => s.push_str(&format!(
                    "{}&{}&{}&{}&{}&{}&{}&{}",
                    b01(d.bool("reverse")),
                    b01(d.bool("align")),
                    n("angle", 90.0),
                    n("scale", 100.0),
                    f6(d.obj("offset")
                        .and_then(|x| x.num("horizontal"))
                        .unwrap_or(0.0)),
                    f6(d.obj("offset")
                        .and_then(|x| x.num("vertical"))
                        .unwrap_or(0.0)),
                    d.str("type").unwrap_or("linear"),
                    gradient_str(d, "gradient")
                )),
                "pattern" => {
                    let p = d.obj("pattern");
                    s.push_str(&format!(
                        "{}&{}&{}&{}&{}&{}",
                        b01(d.bool("linked")),
                        n("scale", 100.0),
                        f6(d.obj("phase")
                            .and_then(|x| x.num("horizontal"))
                            .unwrap_or(0.0)),
                        f6(d.obj("phase")
                            .and_then(|x| x.num("vertical"))
                            .unwrap_or(0.0)),
                        p.and_then(|x| x.str("name")).unwrap_or(""),
                        p.and_then(|x| x.str("ID")).unwrap_or("")
                    ))
                }
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

/// Photoshop-Live-Effekt mit allen bekannten Teil-Effekten (`only_enabled`: nur eingeschaltete).
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
            any |= d.bool("enabled") == Some(true);
        }
    }
    any.then_some(e)
}

/// Effektliste eines Elements nach `UseFWFilters`.
pub fn effect_list(fx: &Descriptor, native_first: bool, global_angle: f64) -> EffectList {
    let mut list = EffectList::default();
    if native_first {
        list.effects.extend(native_effects(fx, global_angle));
        // Nativ umgesetzte Effekte werden im PS-Wörterbuch abgeschaltet; Rest als PS-Live-Effekt.
        let rest = |k: &str| {
            PS_ONLY_FX.contains(&k) && fx.obj(k).and_then(|d| d.bool("enabled")) == Some(true)
        };
        if let Some(e) = photoshop_live_effect(fx, &rest) {
            list.effects.push(e);
        }
    } else if let Some(e) = photoshop_live_effect(fx, &|_| true) {
        list.effects.push(e);
    }
    list
}

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
