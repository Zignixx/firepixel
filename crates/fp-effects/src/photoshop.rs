//! Photoshop-Live-Effekte `{8eeadf50-…}` (05 §9, Laufzeit T4).
//!
//! Die Parameter stehen als `&`-getrennte Datensätze unter dem PS-Namen (`dropShadow`,
//! `innerShadow`, `outerGlow`, `innerGlow`, `bevelEmboss`, `satin`, `solidFill`, `gradientFill`,
//! `patternFill`, `stroke`) sowie `TopLeft`/`BottomRight`. Der Parser zerlegt sie nach den
//! Feldtabellen 9.2 und behält den Originaltext für das verlustfreie Zurückschreiben.
//!
//! Dargestellt werden (empirisch, B):
//! - **Schlagschatten**: Versatz `round(−d·cos θ)`, `round(d·sin θ)` (θ = Lichtwinkel, ,5 auf),
//!   Dreiecksfilter mit Radius `max(1, Größe − 1)` und **einer** Rundung nach der 2D-Summe,
//!   Deckkraft `round(opacity·2,55)`, Objekt exakt gerundet darüber (nicht Original-over).
//!   Überfüllen: Maske ·100/(100−spread), Größe ·(1−spread/100) (A).
//! - **Farbüberlagerung** „normal“: Farbe mit Deckkraft über das Objekt, Alpha bleibt.
//!
//! Die übrigen Teil-Effekte rechnet das Original in der PSL-Bibliothek. Innerer Schatten/Schein,
//! Kontur und Füllungen sind an T4 vermessen (siehe die einzelnen Funktionen); Satin,
//! Abschrägung/Relief und Musterüberlagerung (eigene Muster) siehe `ps_relief`. Gemeldet wird nur
//! noch das Kontur-Relief (ohne PS-Kontur ohne Wirkung).

use fp_color::Rgba8;
use fp_color::blend::{orig_over, over_exact};
use fp_geom::IRect;
use fp_raster::RgbaImage;

use crate::params::{EffectParams, ParamValue};
use crate::util::{mul, new_image, on_frame, px, union};

/// Bekannte Teil-Effekte in Original-Reihenfolge der Datei.
pub const PS_EFFECT_NAMES: [&str; 10] = [
    "dropShadow",
    "innerShadow",
    "outerGlow",
    "innerGlow",
    "bevelEmboss",
    "satin",
    "solidFill",
    "gradientFill",
    "patternFill",
    "stroke",
];

/// Feldtyp der Tabelle 9.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldType {
    Bool,
    Num,
    Str,
    Color,
    Contour,
    Gradient,
    GlowFloats,
    Rest,
}

fn fields(effect: &str) -> &'static [(&'static str, FieldType)] {
    use FieldType::*;
    const SHADOW: &[(&str, FieldType)] = &[
        ("enabled", Bool),
        ("opacity", Num),
        ("useGlobalAngle", Bool),
        ("localLightingAngle", Num),
        ("noise", Num),
        ("distance", Num),
        ("blur", Num),
        ("chokeMatte", Num),
        ("antiAlias", Bool),
        ("layerConceals", Bool),
        ("mode", Str),
        ("color", Color),
        ("transferSpec", Contour),
    ];
    const GLOW: &[(&str, FieldType)] = &[
        ("enabled", Bool),
        ("antiAlias", Bool),
        ("mode", Str),
        ("params", GlowFloats),
        ("glowTechnique", Str),
        ("color", Color),
        ("gradient", Gradient),
        ("transferSpec", Contour),
        ("innerGlowSource", Str),
    ];
    match effect {
        "dropShadow" | "innerShadow" => SHADOW,
        "outerGlow" | "innerGlow" => GLOW,
        "bevelEmboss" => &[
            ("enabled", Bool),
            ("useGlobalAngle", Bool),
            ("useShape", Bool),
            ("useTexture", Bool),
            ("antialiasGloss", Bool),
            ("highlightOpacity", Num),
            ("shadowOpacity", Num),
            ("localLightingAngle", Num),
            ("localLightingAltitude", Num),
            ("strengthRatio", Num),
            ("blur", Num),
            ("softness", Num),
            ("highlightMode", Str),
            ("highlightColor", Color),
            ("shadowMode", Str),
            ("shadowColor", Color),
            ("bevelTechnique", Str),
            ("bevelStyle", Str),
            ("bevelDirection", Str),
            ("transparencyShape", Contour),
            ("shape", Str),
            ("texture", Str),
        ],
        "satin" => &[
            ("enabled", Bool),
            ("antiAlias", Bool),
            ("invert", Bool),
            ("opacity", Num),
            ("localLightingAngle", Num),
            ("distance", Num),
            ("blur", Num),
            ("mode", Str),
            ("color", Color),
            ("mappingShape", Contour),
        ],
        "solidFill" => &[
            ("enabled", Bool),
            ("opacity", Num),
            ("mode", Str),
            ("color", Color),
        ],
        "gradientFill" => &[
            ("enabled", Bool),
            ("reverse", Bool),
            ("align", Bool),
            ("opacity", Num),
            ("angle", Num),
            ("scale", Num),
            ("horizontal", Num),
            ("vertical", Num),
            ("mode", Str),
            ("type", Str),
            ("gradient", Gradient),
        ],
        "patternFill" => &[
            ("enabled", Bool),
            ("align", Bool),
            ("opacity", Num),
            ("scale", Num),
            ("horizontal", Num),
            ("vertical", Num),
            ("mode", Str),
            ("patternName", Str),
            ("patternID", Str),
        ],
        "stroke" => &[
            ("enabled", Bool),
            ("opacity", Num),
            ("size", Num),
            ("mode", Str),
            ("color", Color),
            ("style", Str),
            ("paintType", Str),
            ("sub", Rest),
        ],
        _ => &[],
    }
}

/// Farbe eines PS-Datensatzes (`RGBColor;r;g;b`, `HSBColorClass;…`, … 05 §9.3).
#[derive(Debug, Clone, PartialEq)]
pub struct PsColor {
    /// Klassenname (`RGBColor`, `HSBColorClass`, `CMYKColorClass`, `labColor`, `grayscale`, `bookColor`).
    pub model: String,
    /// Werte als Text.
    pub values: Vec<String>,
}

impl PsColor {
    fn parse(s: &str) -> Option<PsColor> {
        if s.starts_with("no") || s.is_empty() {
            return None;
        }
        let mut it = s.split(';');
        let model = it.next().unwrap_or("").to_string();
        Some(PsColor {
            model,
            values: it.map(str::to_string).collect(),
        })
    }

    /// RGB (nur `RGBColor`; sonst Schwarz, wie die Referenz).
    pub fn rgb(&self) -> [u8; 3] {
        if self.model != "RGBColor" {
            return [0, 0, 0];
        }
        let v = |i: usize| -> u8 {
            self.values
                .get(i)
                .and_then(|t| t.trim().parse::<f64>().ok())
                .map(|x| (x + 0.5).floor().clamp(0.0, 255.0) as u8)
                .unwrap_or(0)
        };
        [v(0), v(1), v(2)]
    }
}

/// Ein zerlegter Datensatz: Felder in Tabellenreihenfolge (Name, Originaltext).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PsRecord {
    /// Originaltext des Datensatzes (für das Zurückschreiben).
    pub raw: String,
    /// Felder (Name, Text); fehlende Felder am Ende fehlen auch hier.
    pub fields: Vec<(String, String)>,
}

impl PsRecord {
    /// Zerlegt einen `&`-Datensatz nach der Feldtabelle von `effect`.
    pub fn parse(effect: &str, s: &str) -> PsRecord {
        let toks: Vec<&str> = s.split('&').collect();
        let mut out = PsRecord {
            raw: s.to_string(),
            fields: Vec::new(),
        };
        for (i, (name, ty)) in fields(effect).iter().enumerate() {
            if *ty == FieldType::Rest {
                out.fields.push((
                    name.to_string(),
                    toks.get(i..).map(|t| t.join("&")).unwrap_or_default(),
                ));
                break;
            }
            let Some(t) = toks.get(i) else { break };
            out.fields.push((name.to_string(), t.to_string()));
        }
        out
    }

    /// Feldtext.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Wahrheitswert (`1`).
    pub fn flag(&self, name: &str) -> bool {
        self.get(name) == Some("1")
    }

    /// Zahl.
    pub fn num(&self, name: &str, default: f64) -> f64 {
        self.get(name)
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .unwrap_or(default)
    }

    /// Farbe.
    pub fn color(&self, name: &str) -> Option<PsColor> {
        self.get(name).and_then(PsColor::parse)
    }

    /// Glühen: `opacity:chokeMatte:blur:noise:shadingNoise:inputRange`.
    pub fn glow_floats(&self) -> [f64; 6] {
        let mut out = [0.0; 6];
        if let Some(t) = self.get("params") {
            for (o, v) in out.iter_mut().zip(t.split(':')) {
                *o = v.trim().parse().unwrap_or(0.0);
            }
        }
        out
    }
}

/// Alle Teil-Effekte eines PS-Live-Effekts.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PsEffects {
    /// Datensätze je PS-Name in Original-Reihenfolge.
    pub records: Vec<(String, PsRecord)>,
    /// Ergebnisrechteck der letzten PSL-Berechnung (`TopLeft`/`BottomRight`, Dokumentkoordinaten).
    pub result_rect: Option<IRect>,
}

impl PsEffects {
    /// Aus den Datei-Parametern.
    pub fn read(p: &EffectParams) -> PsEffects {
        let mut out = PsEffects::default();
        for name in PS_EFFECT_NAMES {
            if let Some(ParamValue::Text(s)) = p.get(name) {
                out.records
                    .push((name.to_string(), PsRecord::parse(name, s)));
            }
        }
        let pt = |k: &str| -> Option<(i32, i32)> {
            let ParamValue::Text(s) = p.get(k)? else {
                return None;
            };
            let mut it = s
                .split_whitespace()
                .map(|v| v.parse::<f64>().ok().map(|f| f.trunc() as i32));
            Some((it.next()??, it.next()??))
        };
        if let (Some(a), Some(b)) = (pt("TopLeft"), pt("BottomRight")) {
            out.result_rect = Some(IRect::new(
                a.0.min(b.0),
                a.1.min(b.1),
                a.0.max(b.0),
                a.1.max(b.1),
            ));
        }
        out
    }

    /// Datensatz eines Teil-Effekts.
    pub fn get(&self, name: &str) -> Option<&PsRecord> {
        self.records.iter().find(|(k, _)| k == name).map(|(_, r)| r)
    }

    /// Aktive Teil-Effekte, die Firepixel nicht darstellt.
    pub fn unsupported(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|(k, r)| {
                r.flag("enabled")
                    && k == "bevelEmboss"
                    && r.get("bevelStyle") == Some("strokeEmboss")
            })
            .map(|(k, _)| k.clone())
            .collect()
    }
}

/// Position of the PSD-style stroke relative to the object edge (`style` of the `stroke` record,
/// 05 §9.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrokePosition {
    /// `outsetFrame`: the band lies outside the edge (default of the PSD format).
    #[default]
    Outside,
    /// `insetFrame`: the band lies inside the edge.
    Inside,
    /// `centeredFrame`: half inside, half outside.
    Center,
}

impl StrokePosition {
    /// Technical names (`outside`, `inside`, `center`) of the command and batch parameter.
    pub const NAMES: [&'static str; 3] = ["outside", "inside", "center"];

    /// Keyword of the record (`outsetFrame`, `insetFrame`, `centeredFrame`).
    pub fn keyword(self) -> &'static str {
        match self {
            StrokePosition::Outside => "outsetFrame",
            StrokePosition::Inside => "insetFrame",
            StrokePosition::Center => "centeredFrame",
        }
    }

    /// From the record keyword (unknown: outside).
    pub fn from_keyword(s: &str) -> StrokePosition {
        match s {
            "insetFrame" => StrokePosition::Inside,
            "centeredFrame" => StrokePosition::Center,
            _ => StrokePosition::Outside,
        }
    }

    /// From the technical name (`outside`, `inside`, `center`) or the record keyword.
    pub fn from_name(s: &str) -> Option<StrokePosition> {
        match s {
            "outside" | "outsetFrame" => Some(StrokePosition::Outside),
            "inside" | "insetFrame" => Some(StrokePosition::Inside),
            "center" | "centre" | "centeredFrame" => Some(StrokePosition::Center),
            _ => None,
        }
    }

    /// Technical name.
    pub fn name(self) -> &'static str {
        match self {
            StrokePosition::Outside => "outside",
            StrokePosition::Inside => "inside",
            StrokePosition::Center => "center",
        }
    }

    /// Index 0..2 in [`StrokePosition::NAMES`].
    pub fn index(self) -> usize {
        match self {
            StrokePosition::Outside => 0,
            StrokePosition::Inside => 1,
            StrokePosition::Center => 2,
        }
    }

    /// From an index 0..2 (out of range: outside).
    pub fn from_index(i: usize) -> StrokePosition {
        match i {
            1 => StrokePosition::Inside,
            2 => StrokePosition::Center,
            _ => StrokePosition::Outside,
        }
    }
}

/// Blend modes the stroke offers (PSD keywords of 05 §9.4 that the renderer mixes).
pub const STROKE_BLEND_MODES: [&str; 10] = [
    "normal",
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "colorBurn",
    "colorDodge",
    "linearDodge",
    "softLight",
];

/// Parameters of the PSD-style stroke live effect (`stroke` record of the PSD live effect
/// container, 05 §9.2: enabled, opacity %, size px, mode, color, style, paintType + sub-record).
/// Defaults are the defaults of the PSD format (A): 3 px outside, normal, 100 %, red.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeParams {
    /// Sub-effect switched on.
    pub enabled: bool,
    /// Width of the band in pixels (1..250).
    pub size: f64,
    /// Position relative to the edge.
    pub position: StrokePosition,
    /// Blend mode (PSD keyword, see [`STROKE_BLEND_MODES`]).
    pub mode: String,
    /// Opacity 0..100 %.
    pub opacity: f64,
    /// Colour (RGB).
    pub color: [u8; 3],
    /// Fill type and sub-record kept verbatim (`solidColor&` for a solid colour; gradient and
    /// pattern strokes of imported files keep their sub-record).
    pub paint: String,
}

impl Default for StrokeParams {
    fn default() -> Self {
        StrokeParams {
            enabled: true,
            size: 3.0,
            position: StrokePosition::Outside,
            mode: "normal".into(),
            opacity: 100.0,
            color: [255, 0, 0],
            paint: "solidColor&".into(),
        }
    }
}

impl StrokeParams {
    /// Range of the size in pixels (like the PSD format).
    pub const SIZE_RANGE: (f64, f64) = (1.0, 250.0);

    /// Reads a `stroke` record (missing fields: defaults).
    pub fn from_record(s: &str) -> StrokeParams {
        let r = PsRecord::parse("stroke", s);
        let d = StrokeParams::default();
        let paint = match (r.get("paintType"), r.get("sub")) {
            (Some(p), Some(sub)) => format!("{p}&{sub}"),
            (Some(p), None) => format!("{p}&"),
            _ => d.paint.clone(),
        };
        StrokeParams {
            enabled: r.get("enabled").is_none_or(|v| v == "1"),
            size: r.num("size", d.size),
            position: StrokePosition::from_keyword(r.get("style").unwrap_or("outsetFrame")),
            mode: r.get("mode").unwrap_or("normal").to_string(),
            opacity: r.num("opacity", d.opacity),
            color: r.color("color").map(|c| c.rgb()).unwrap_or(d.color),
            paint,
        }
    }

    /// Writes the `stroke` record like the reference (`%d&%f&%f&%s&%s&%s&%s&` + sub-record, numbers
    /// with 6 decimals, colour `RGBColor;r;g;b`).
    pub fn to_record(&self) -> String {
        let (lo, hi) = Self::SIZE_RANGE;
        format!(
            "{}&{:.6}&{:.6}&{}&RGBColor;{:.6};{:.6};{:.6}&{}&{}",
            if self.enabled { 1 } else { 0 },
            self.opacity.clamp(0.0, 100.0),
            self.size.clamp(lo, hi),
            self.mode,
            self.color[0] as f64,
            self.color[1] as f64,
            self.color[2] as f64,
            self.position.keyword(),
            self.paint
        )
    }

    /// Is the fill a solid colour?
    pub fn is_solid(&self) -> bool {
        self.paint.starts_with("solidColor")
    }
}

/// Versatz des PS-Schlagschattens (Lichtrichtung, Schatten gegenüber; ,5 aufgerundet).
pub fn ps_offset(distance: f64, angle: f64) -> (i32, i32) {
    let a = angle.to_radians();
    let f = |v: f64| {
        if v.is_finite() {
            (v + 0.5 + 1e-9).floor().clamp(-1e9, 1e9) as i32
        } else {
            0
        }
    };
    (f(-distance * a.cos()), f(distance * a.sin()))
}

/// Dreiecksfilter der PS-Effekte ohne Zwischenrundung: `round(Σ w_i w_j a / (r+1)⁴)`, Ergebnis um
/// `r` je Seite größer.
pub fn ps_tent(alpha: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let (w2, h2) = (w + 2 * r, h + 2 * r);
    let wt = |k: usize| (k + 1).min(2 * r + 1 - k) as u64;
    // waagerecht (ungerundet), Zeilen der Eingabe
    let mut hs = vec![0u64; w2 * h];
    for y in 0..h {
        for j in 0..w2 {
            let mut s = 0u64;
            for k in 0..=2 * r {
                let i = j as isize + k as isize - 2 * r as isize;
                if i >= 0 && (i as usize) < w {
                    s += wt(k) * alpha[y * w + i as usize] as u64;
                }
            }
            hs[y * w2 + j] = s;
        }
    }
    let n = ((r + 1) as u64).pow(4);
    let mut out = vec![0u8; w2 * h2];
    for x in 0..w2 {
        for jy in 0..h2 {
            let mut s = 0u64;
            for k in 0..=2 * r {
                let i = jy as isize + k as isize - 2 * r as isize;
                if i >= 0 && (i as usize) < h {
                    s += wt(k) * hs[i as usize * w2 + x];
                }
            }
            out[jy * w2 + x] = ((s + n / 2) / n).min(255) as u8;
        }
    }
    out
}

pub(crate) fn opacity255(pct: f64) -> u8 {
    (pct * 255.0 / 100.0 + 0.5).floor().clamp(0.0, 255.0) as u8
}

/// PS-Effekte auf ein Objektbild in der Photoshop-Stapelfolge: Verlaufsüberlagerung,
/// Farbüberlagerung, innerer Schein, innerer Schatten (im Objekt, Alpha bleibt), Kontur, dann
/// äußerer Schein, Schlagschatten und die äußere Abschrägung hinter dem Objekt; davor
/// Musterüberlagerung und Satin, nach dem inneren Schatten die innere Abschrägung.
pub fn render(obj: &RgbaImage, fx: &PsEffects) -> RgbaImage {
    let mut img = obj.clone();
    let on = |k: &str| fx.get(k).filter(|r| r.flag("enabled"));
    if let Some(pf) = on("patternFill")
        && !img.is_empty()
    {
        crate::ps_relief::pattern_fill(&mut img, obj, pf);
    }
    if let Some(gf) = on("gradientFill")
        && !img.is_empty()
    {
        gradient_fill(&mut img, gf);
    }
    if let Some(sf) = on("solidFill") {
        let op = opacity255(sf.num("opacity", 100.0));
        let c = sf.color("color").map(|c| c.rgb()).unwrap_or([0, 0, 0]);
        if sf.get("mode") == Some("normal") {
            for p in img.data_mut() {
                let d = px(*p);
                let r = orig_over(Rgba8::new(c[0], c[1], c[2], op), d);
                *p = [r.r, r.g, r.b, d.a];
            }
        } else {
            let mode = sf.get("mode").unwrap_or("normal").to_string();
            for p in img.data_mut() {
                if p[3] > 0 {
                    *p = inner_mix(*p, c, op, &mode);
                }
            }
        }
    }
    if let Some(sa) = on("satin")
        && !img.is_empty()
    {
        crate::ps_relief::satin(&mut img, obj, sa);
    }
    if let Some(ig) = on("innerGlow")
        && !img.is_empty()
    {
        inner_glow(&mut img, obj, ig);
    }
    if let Some(is) = on("innerShadow")
        && !img.is_empty()
    {
        inner_shadow(&mut img, obj, is);
    }
    let mut relief_out = None;
    if let Some(be) = on("bevelEmboss")
        && !img.is_empty()
    {
        relief_out = crate::ps_relief::bevel(&mut img, obj, be);
    }
    if let Some(st) = on("stroke")
        && !img.is_empty()
    {
        img = stroke(&img, obj, st);
    }
    let og = on("outerGlow").filter(|_| !obj.is_empty());
    let ds = on("dropShadow").filter(|_| !obj.is_empty());
    if og.is_none() && ds.is_none() && relief_out.is_none() {
        return img;
    }
    let mut behind: Vec<RgbaImage> = Vec::new();
    if let Some(ds) = ds {
        behind.push(shadow_layer(obj, ds));
    }
    if let Some(og) = og {
        behind.push(outer_glow_layer(obj, og));
    }
    if let Some(ro) = relief_out {
        behind.push(ro);
    }
    // Ebene hinter dem Objekt (normal übereinander), Objekt exakt gerundet darüber
    let mut frame = img.bounds();
    for b in &behind {
        frame = union(frame, b.bounds());
    }
    let mut base = new_image(frame);
    for b in &behind {
        let bf = on_frame(b, frame);
        for (d, s) in base.data_mut().iter_mut().zip(bf.data()) {
            *d = over_exact(px(*s), px(*d)).to_array();
        }
    }
    let o = on_frame(&img, frame);
    for (d, s) in base.data_mut().iter_mut().zip(o.data()) {
        *d = over_exact(px(*s), px(*d)).to_array();
    }
    base
}

/// Mischung einer Effektfarbe mit Deckung `a` (0–255) in ein Objektpixel; Alpha bleibt.
pub(crate) fn inner_mix(p: [u8; 4], c: [u8; 3], a: u8, mode: &str) -> [u8; 4] {
    let f = |o: u8, s: u8| -> u8 {
        let (o, s) = (o as i32, s as i32);
        let b = match mode {
            "multiply" => (o * s + 127) / 255,
            "screen" => 255 - ((255 - o) * (255 - s) + 127) / 255,
            "overlay" => {
                if o < 128 {
                    (2 * o * s + 127) / 255
                } else {
                    255 - (2 * (255 - o) * (255 - s) + 127) / 255
                }
            }
            "darken" => o.min(s),
            "lighten" => o.max(s),
            "linearDodge" => (o + s).min(255),
            "colorBurn" => {
                if s == 0 {
                    0
                } else {
                    (255 - (255 - o) * 255 / s).max(0)
                }
            }
            "colorDodge" => {
                if s == 255 {
                    255
                } else {
                    (o * 255 / (255 - s)).min(255)
                }
            }
            "softLight" => {
                let (of, sf) = (o as f64 / 255.0, s as f64 / 255.0);
                let r = if sf <= 0.5 {
                    of - (1.0 - 2.0 * sf) * of * (1.0 - of)
                } else {
                    of + (2.0 * sf - 1.0)
                        * ((if of <= 0.25 {
                            ((16.0 * of - 12.0) * of + 4.0) * of
                        } else {
                            of.sqrt()
                        }) - of)
                };
                (r * 255.0).round() as i32
            }
            _ => s,
        };
        let v = o as f64 + (b - o) as f64 * a as f64 / 255.0;
        (v + 0.5).floor().clamp(0.0, 255.0) as u8
    };
    [f(p[0], c[0]), f(p[1], c[1]), f(p[2], c[2]), p[3]]
}

/// Alpha des Objekts in `frame` (außerhalb 0).
fn alpha_in(obj: &RgbaImage, frame: IRect) -> Vec<u8> {
    on_frame(obj, frame).data().iter().map(|p| p[3]).collect()
}

fn blur_radius(size: f64) -> usize {
    ((size.round() as i64) - 1).clamp(1, 512) as usize
}

/// Innerer Schatten (Laufzeit T4 exakt): umgekehrtes Alpha um den Versatz (Schattenrichtung wie
/// beim Schlagschatten) verschoben, Dreiecksfilter `r = max(1, Größe − 1)`, Überfüllen wie beim
/// Schlagschatten; Deckung `round(m·op/255)`, im Objekt mit dem Mischmodus gemischt.
fn inner_shadow(img: &mut RgbaImage, obj: &RgbaImage, rec: &PsRecord) {
    let b = obj.bounds();
    let size = rec.num("blur", 0.0);
    let choke = rec.num("chokeMatte", 0.0);
    let r = if size <= 0.0 {
        0
    } else {
        blur_radius(size * (1.0 - choke / 100.0))
    };
    let (dx, dy) = ps_offset(
        rec.num("distance", 0.0),
        rec.num("localLightingAngle", 120.0),
    );
    // Arbeitsfläche: Objekt + r + Versatz
    let m = r as i32 + dx.abs().max(dy.abs()) + 1;
    let area = IRect::new(b.left - m, b.top - m, b.right + m, b.bottom + m);
    let (w, h) = (area.width() as usize, area.height() as usize);
    let src = alpha_in(obj, area);
    let mut inv = vec![255u8; w * h];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let (sx, sy) = (x - dx, y - dy);
            if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                let mut v = 255 - src[sy as usize * w + sx as usize] as i64;
                if choke > 0.0 {
                    v = (v * 100 / ((100.0 - choke).trunc() as i64).max(1)).min(255);
                }
                inv[y as usize * w + x as usize] = v as u8;
            }
        }
    }
    let mask = if r > 0 {
        ps_tent(&inv, w, h, r)
    } else {
        inv.clone()
    };
    let mw = w + 2 * r;
    let op = opacity255(rec.num("opacity", 75.0));
    let c = rec.color("color").map(|c| c.rgb()).unwrap_or([0, 0, 0]);
    let mode = rec.get("mode").unwrap_or("multiply").to_string();
    let iw = img.width();
    let ib = img.bounds();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = (ib.left + (i % iw) as i32, ib.top + (i / iw) as i32);
        let (mx, my) = ((x - area.left) as usize + r, (y - area.top) as usize + r);
        let mv = mask.get(my * mw + mx).copied().unwrap_or(0);
        let a = mul(mv, op);
        if a > 0 {
            *p = inner_mix(*p, c, a, &mode);
        }
    }
}

/// Innerer Schein (Laufzeit T4 exakt): Kante = Dreiecksfilter des umgekehrten Alphas,
/// Mitte = Dreiecksfilter des Alphas; Kennlinie über den Bereich (`inputRange`):
/// Kante `min(1, m/Bereich)`, Mitte `max(0, (m − (1 − Bereich))/Bereich)`; Deckung ·op.
fn inner_glow(img: &mut RgbaImage, obj: &RgbaImage, rec: &PsRecord) {
    let [opacity, choke, size, _, _, range] = rec.glow_floats();
    let center = rec.get("innerGlowSource") == Some("centerGlow");
    let r = if size <= 0.0 {
        0
    } else {
        blur_radius(size * (1.0 - choke / 100.0))
    };
    let b = obj.bounds();
    let m = r as i32 + 1;
    let area = IRect::new(b.left - m, b.top - m, b.right + m, b.bottom + m);
    let (w, h) = (area.width() as usize, area.height() as usize);
    let a = alpha_in(obj, area);
    let src: Vec<u8> = if center {
        a
    } else {
        a.iter().map(|v| 255 - v).collect()
    };
    let mask = if r > 0 { ps_tent(&src, w, h, r) } else { src };
    let mw = w + 2 * r;
    let range = if range > 0.0 { range / 100.0 } else { 1.0 };
    let op = opacity255(opacity);
    let c = rec
        .color("color")
        .map(|c| c.rgb())
        .unwrap_or([255, 255, 190]);
    let mode = rec.get("mode").unwrap_or("screen").to_string();
    let iw = img.width();
    let ib = img.bounds();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = (ib.left + (i % iw) as i32, ib.top + (i / iw) as i32);
        let (mx, my) = ((x - area.left) as usize + r, (y - area.top) as usize + r);
        let mv = mask.get(my * mw + mx).copied().unwrap_or(0) as f64;
        let mm = if center {
            ((mv - 255.0 * (1.0 - range)) / range).clamp(0.0, 255.0)
        } else {
            (mv / range).min(255.0)
        };
        let al = mul((mm + 0.5).floor() as u8, op);
        if al > 0 {
            *p = inner_mix(*p, c, al, &mode);
        }
    }
}

/// Äußerer Schein (A, Photoshop-Semantik wie der innere Schein): Dreiecksfilter des Alphas
/// (weicher) bzw. Abstandsband (genau), Kennlinie `min(1, m/Bereich)`, Deckung ·op, Farbe; hinter dem
/// Objekt. Laufzeit T4: das Original zeigt dort ein volles Farbband (Breite Größe − 1 bzw. Größe) –
/// Abweichung offen.
fn outer_glow_layer(obj: &RgbaImage, rec: &PsRecord) -> RgbaImage {
    let [opacity, choke, size, _, _, range] = rec.glow_floats();
    let precise = rec.get("glowTechnique") == Some("preciseMatte");
    let b = obj.bounds();
    let (w, h) = (b.width() as usize, b.height() as usize);
    let mut a: Vec<u8> = obj.data().iter().map(|p| p[3]).collect();
    if choke > 0.0 {
        let den = ((100.0 - choke).trunc() as i64).max(1);
        for v in &mut a {
            *v = ((*v as i64 * 100) / den).min(255) as u8;
        }
    }
    let r = if size <= 0.0 {
        0
    } else if precise {
        size.round() as usize
    } else {
        blur_radius(size * (1.0 - choke / 100.0))
    };
    let mask: Vec<u8> = if r == 0 {
        a.clone()
    } else if precise {
        dilate(&a, w, h, r)
    } else {
        ps_tent(&a, w, h, r)
    };
    let (mw, mh) = (w + 2 * r, h + 2 * r);
    let range = if range > 0.0 { range / 100.0 } else { 1.0 };
    let op = opacity255(opacity);
    let c = rec
        .color("color")
        .map(|c| c.rgb())
        .unwrap_or([255, 255, 190]);
    let frame = IRect::from_xywh(b.left - r as i32, b.top - r as i32, mw as i32, mh as i32);
    let mut out = new_image(frame);
    for (i, d) in out.data_mut().iter_mut().enumerate() {
        let mm = (mask[i] as f64 / range).min(255.0);
        let al = mul((mm + 0.5).floor() as u8, op);
        *d = [c[0], c[1], c[2], al];
    }
    out
}

/// Ausdehnung (Kreis mit Radius `r`) eines Alphas, Ergebnis um `r` je Seite größer.
fn dilate(a: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    let (w2, h2) = (w + 2 * r, h + 2 * r);
    let rr = (r * r) as i64;
    let mut out = vec![0u8; w2 * h2];
    for y in 0..h2 as i64 {
        for x in 0..w2 as i64 {
            let mut best = 0u8;
            for oy in -(r as i64)..=r as i64 {
                for ox in -(r as i64)..=r as i64 {
                    if ox * ox + oy * oy > rr {
                        continue;
                    }
                    let (sx, sy) = (x - r as i64 + ox, y - r as i64 + oy);
                    if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                        best = best.max(a[sy as usize * w + sx as usize]);
                    }
                }
            }
            out[y as usize * w2 + x as usize] = best;
        }
    }
    out
}

/// Schlagschatten als eigene Ebene (für das gemeinsame Mischen hinter dem Objekt).
fn shadow_layer(obj: &RgbaImage, ds: &PsRecord) -> RgbaImage {
    let b = obj.bounds();
    let (w, h) = (b.width() as usize, b.height() as usize);
    let choke = ds.num("chokeMatte", 0.0);
    let a: Vec<u8> = obj.data().iter().map(|p| p[3]).collect();
    // Überfüllen (Laufzeit T4, A): volle Größe weichzeichnen, dann Kennlinie m·100/(100 − Überfüllen)
    let size = ds.num("blur", 0.0).round_ties_even();
    let (mut mask, g) = if size <= 0.0 {
        (a, 0usize)
    } else {
        let r = blur_radius(size);
        (ps_tent(&a, w, h, r), r)
    };
    if choke > 0.0 {
        let den = ((100.0 - choke).trunc() as i64).max(1);
        for v in &mut mask {
            *v = ((*v as i64 * 100) / den).min(255) as u8;
        }
    }
    let (dx, dy) = ps_offset(ds.num("distance", 0.0), ds.num("localLightingAngle", 120.0));
    let sb = IRect::from_xywh(
        b.left - g as i32 + dx,
        b.top - g as i32 + dy,
        (w + 2 * g) as i32,
        (h + 2 * g) as i32,
    );
    let op = opacity255(ds.num("opacity", 75.0));
    let c = ds.color("color").map(|c| c.rgb()).unwrap_or([0, 0, 0]);
    let noise = (ds.num("noise", 0.0) / 100.0).clamp(0.0, 1.0);
    let mut out = new_image(sb);
    let sw = (sb.width() as usize).max(1);
    for (i, d) in out.data_mut().iter_mut().enumerate() {
        let mut m = mask[i];
        if noise > 0.0 && m > 0 {
            // Rauschen (A): Deckung je Pixel zufällig abgesenkt, feste Folge je Ort
            let (x, y) = (
                (sb.left + (i % sw) as i32) as u32,
                (sb.top + (i / sw) as i32) as u32,
            );
            let mut hsh = x.wrapping_mul(0x9E37_79B1) ^ y.wrapping_mul(0x85EB_CA77) ^ 0x27D4_EB2F;
            hsh ^= hsh >> 15;
            hsh = hsh.wrapping_mul(0x2C1B_3C6D);
            hsh ^= hsh >> 12;
            let r = (hsh & 0xFFFF) as f64 / 65535.0;
            m = (m as f64 * (1.0 - noise * 2.0 * r).max(0.0) + 0.5) as u8;
        }
        *d = [c[0], c[1], c[2], mul(m, op)];
    }
    out
}

/// Squared Euclidean distance (between pixel centres) of every pixel to the nearest pixel with
/// `feature` set (exact, two 1D passes of the lower-envelope transform). Without any feature
/// pixel the distances stay at `f64::MAX`.
fn sq_distance(feature: &[bool], w: usize, h: usize) -> Vec<f64> {
    const INF: f64 = 1e20;
    fn pass(f: &[f64], d: &mut [f64], v: &mut [usize], z: &mut [f64]) {
        let n = f.len();
        let isect = |q: usize, p: usize| {
            ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q as f64 - p as f64))
        };
        let mut k = 0usize;
        v[0] = 0;
        z[0] = f64::NEG_INFINITY;
        z[1] = f64::INFINITY;
        for q in 1..n {
            let mut s = isect(q, v[k]);
            while s <= z[k] {
                // z[0] is -inf, so k never drops below 0
                k -= 1;
                s = isect(q, v[k]);
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
        }
        k = 0;
        for q in 0..n {
            while z[k + 1] < q as f64 {
                k += 1;
            }
            let p = v[k];
            let dq = q as f64 - p as f64;
            d[q] = dq * dq + f[p];
        }
    }
    let n = w.max(h);
    let (mut v, mut z) = (vec![0usize; n], vec![0.0f64; n + 1]);
    let (mut f, mut d) = (vec![0.0f64; n], vec![0.0f64; n]);
    let mut grid: Vec<f64> = feature.iter().map(|&b| if b { 0.0 } else { INF }).collect();
    for x in 0..w {
        for y in 0..h {
            f[y] = grid[y * w + x];
        }
        pass(&f[..h], &mut d[..h], &mut v, &mut z);
        for y in 0..h {
            grid[y * w + x] = d[y];
        }
    }
    for y in 0..h {
        f[..w].copy_from_slice(&grid[y * w..(y + 1) * w]);
        pass(&f[..w], &mut d[..w], &mut v, &mut z);
        grid[y * w..(y + 1) * w].copy_from_slice(&d[..w]);
    }
    grid.into_iter()
        .map(|v| if v >= INF / 2.0 { f64::MAX } else { v })
        .collect()
}

/// Stroke (A, PSD semantics, `stroke` record 05 §9.2): band of width `size` outside, inside or
/// centred on the object edge (Euclidean distance between pixel centres to the nearest pixel of
/// the other side, anti-aliased by the fraction), colour with opacity on top, mixed with the
/// blend mode where the object has pixels (on transparent pixels every mode acts like "normal").
/// Solid colour only; gradient and pattern strokes are drawn in their colour.
fn stroke(img: &RgbaImage, obj: &RgbaImage, rec: &PsRecord) -> RgbaImage {
    let size = rec.num("size", 3.0).max(0.0);
    let style = rec.get("style").unwrap_or("outsetFrame");
    let (outer, inner) = match style {
        "insetFrame" => (0.0, size),
        "centeredFrame" => (size / 2.0, size / 2.0),
        _ => (size, 0.0),
    };
    let b = obj.bounds();
    let g = outer.ceil() as i32 + 1;
    let frame = IRect::new(b.left - g, b.top - g, b.right + g, b.bottom + g);
    let (w, h) = (frame.width() as usize, frame.height() as usize);
    if w == 0 || h == 0 {
        return img.clone();
    }
    let a = alpha_in(obj, frame);
    let inside: Vec<bool> = a.iter().map(|&v| v >= 128).collect();
    let outside: Vec<bool> = inside.iter().map(|&v| !v).collect();
    // distance of outside pixels to the object, of inside pixels to the outside (the frame has a
    // transparent margin, so every inside pixel has an outside pixel in the frame)
    let to_inside = sq_distance(&inside, w, h);
    let to_outside = sq_distance(&outside, w, h);
    let op = opacity255(rec.num("opacity", 100.0));
    let c = rec.color("color").map(|c| c.rgb()).unwrap_or([255, 0, 0]);
    let mode = rec.get("mode").unwrap_or("normal").to_string();
    let mut out = on_frame(img, frame);
    for i in 0..w * h {
        let (sq, width) = if inside[i] {
            (to_outside[i], inner)
        } else {
            (to_inside[i], outer)
        };
        if sq == f64::MAX || width <= 0.0 {
            continue;
        }
        let d = sq.sqrt() - 0.5; // pixel centre → edge
        let cov = (width - d).clamp(0.0, 1.0);
        if cov <= 0.0 {
            continue;
        }
        let al = mul((cov * 255.0 + 0.5) as u8, op);
        let p = out.data()[i];
        out.data_mut()[i] = if p[3] == 0 {
            [c[0], c[1], c[2], al]
        } else {
            // blend mode against the object colour, then on top with the stroke's coverage
            let m = if mode == "normal" {
                c
            } else {
                let q = inner_mix([p[0], p[1], p[2], 255], c, 255, &mode);
                [q[0], q[1], q[2]]
            };
            over_exact(Rgba8::new(m[0], m[1], m[2], al), px(p)).to_array()
        };
    }
    out
}

/// Farbstopps und Deckkraftstopps eines PS-Verlaufs.
type StopLists = (Vec<(f64, [u8; 3])>, Vec<(f64, u8)>);

/// Stopps eines PS-Verlaufs (`name,customStops,…`, 05 §9.3): Farbstopps (0–1, RGB) und
/// Deckkraftstopps (0–1, 0–255).
fn gradient_stops(s: &str) -> StopLists {
    let parts: Vec<&str> = s.split(',').collect();
    let mut colors = Vec::new();
    let mut alphas = Vec::new();
    if parts.len() < 3 || parts[1] != "customStops" {
        return (
            vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])],
            vec![(0.0, 255), (1.0, 255)],
        );
    }
    let head: Vec<&str> = parts[2].split('$').collect();
    let nc = head
        .get(1)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let na = head
        .get(2)
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    for p in parts.iter().skip(3).take(nc) {
        let f: Vec<&str> = p.split('$').collect();
        let loc = f.first().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) / 4096.0;
        let col = f
            .get(3)
            .and_then(|c| PsColor::parse(c))
            .map(|c| c.rgb())
            .unwrap_or([0, 0, 0]);
        colors.push((loc, col));
    }
    for p in parts.iter().skip(3 + nc).take(na) {
        let f: Vec<&str> = p.split('$').collect();
        let opa = f
            .first()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(100.0);
        let loc = f.get(1).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) / 4096.0;
        alphas.push((loc, opacity255(opa)));
    }
    if colors.is_empty() {
        colors = vec![(0.0, [0, 0, 0]), (1.0, [255, 255, 255])];
    }
    if alphas.is_empty() {
        alphas = vec![(0.0, 255), (1.0, 255)];
    }
    (colors, alphas)
}

/// Glättung der Verlaufsübergänge (A, Laufzeit T4: Mittel aus linear und `smoothstep`).
fn smooth(t: f64) -> f64 {
    0.5 * (t + t * t * (3.0 - 2.0 * t))
}

fn lerp_stops<const N: usize>(st: &[(f64, [u8; N])], t: f64) -> [f64; N] {
    let mut out = [0.0; N];
    let first = st[0];
    let last = st[st.len() - 1];
    let (a, b, f) = if t <= first.0 {
        (first.1, first.1, 0.0)
    } else if t >= last.0 {
        (last.1, last.1, 0.0)
    } else {
        let k = st
            .windows(2)
            .position(|w| t >= w[0].0 && t <= w[1].0)
            .unwrap_or(0);
        let (p, q) = (st[k], st[k + 1]);
        let f = if q.0 > p.0 {
            (t - p.0) / (q.0 - p.0)
        } else {
            0.0
        };
        (p.1, q.1, smooth(f))
    };
    for i in 0..N {
        out[i] = a[i] as f64 + (b[i] as f64 - a[i] as f64) * f;
    }
    out
}

/// Verlaufsüberlagerung (A): linear/gespiegelt/radial/Winkel/Raute über die Objektgrenzen
/// (`align`), Winkel, Skalierung, Umkehren; Deckkraft und Mischmodus im Objekt.
fn gradient_fill(img: &mut RgbaImage, rec: &PsRecord) {
    let (colors, alphas) = gradient_stops(rec.get("gradient").unwrap_or(""));
    let alphas: Vec<(f64, [u8; 1])> = alphas.into_iter().map(|(t, a)| (t, [a])).collect();
    let b = {
        let ab = img.alpha_bounds();
        if ab.is_empty() { img.bounds() } else { ab }
    };
    let (cx, cy) = (
        (b.left + b.right) as f64 / 2.0,
        (b.top + b.bottom) as f64 / 2.0,
    );
    let ang = rec.num("angle", 90.0).to_radians();
    let (ux, uy) = (ang.cos(), -ang.sin());
    let scale = (rec.num("scale", 100.0) / 100.0).max(0.01);
    let (bw, bh) = (b.width() as f64, b.height() as f64);
    // halbe Ausdehnung des Rechtecks in Verlaufsrichtung
    let half = ((bw * ux).abs() + (bh * uy).abs()) / 2.0 * scale;
    let ty = rec.get("type").unwrap_or("linear");
    let rev = rec.flag("reverse");
    let op = opacity255(rec.num("opacity", 100.0));
    let mode = rec.get("mode").unwrap_or("normal").to_string();
    let iw = img.width();
    let ib = img.bounds();
    for (i, p) in img.data_mut().iter_mut().enumerate() {
        if p[3] == 0 {
            continue;
        }
        let (x, y) = (
            (ib.left + (i % iw) as i32) as f64 + 0.5,
            (ib.top + (i / iw) as i32) as f64 + 0.5,
        );
        let (vx, vy) = (x - cx, y - cy);
        let along = vx * ux + vy * uy;
        let mut t = match ty {
            "radial" => (vx * vx + vy * vy).sqrt() / half.max(1e-6),
            "reflected" => along.abs() / half.max(1e-6),
            "diamond" => ((vx * ux + vy * uy).abs() + (vx * -uy + vy * ux).abs()) / half.max(1e-6),
            "angle" => {
                ((vy.atan2(vx) - (-ang)).rem_euclid(std::f64::consts::TAU)) / std::f64::consts::TAU
            }
            _ => (along / half.max(1e-6) + 1.0) / 2.0,
        };
        t = t.clamp(0.0, 1.0);
        if rev {
            t = 1.0 - t;
        }
        let c = lerp_stops(&colors, t);
        let a = lerp_stops(&alphas, t)[0];
        let al = mul((a + 0.5) as u8, op);
        let col = [(c[0] + 0.5) as u8, (c[1] + 0.5) as u8, (c[2] + 0.5) as u8];
        *p = inner_mix(*p, col, al, &mode);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_drop_shadow_record() {
        let s = "1&75.000000&1&120.000000&0.000000&3.000000&5.000000&0.000000&0&1&multiply&RGBColor;0.000000;0.000000;0.000000&2$$0.000000;0.000000;1$255.000000;255.000000;1";
        let r = PsRecord::parse("dropShadow", s);
        assert!(r.flag("enabled"));
        assert_eq!(r.num("blur", 0.0), 5.0);
        assert_eq!(r.get("mode"), Some("multiply"));
        assert_eq!(r.color("color").map(|c| c.rgb()), Some([0, 0, 0]));
        assert_eq!(r.fields.len(), 13);
        // Laufzeit T4: 120°/Distanz 3 → dx = 2.
        assert_eq!(ps_offset(3.0, 120.0).0, 2);
    }

    #[test]
    fn stroke_keeps_subrecord() {
        let r = PsRecord::parse(
            "stroke",
            "1&100.000000&3.000000&normal&RGBColor;1;2;3&outsetFrame&solidColor&",
        );
        assert_eq!(r.get("sub"), Some(""));
        assert_eq!(r.get("style"), Some("outsetFrame"));
    }

    fn stroke_fx(p: &StrokeParams) -> PsEffects {
        PsEffects {
            records: vec![(
                "stroke".to_string(),
                PsRecord::parse("stroke", &p.to_record()),
            )],
            result_rect: None,
        }
    }

    fn gray_box() -> RgbaImage {
        RgbaImage::filled(IRect::new(10, 10, 30, 30), Rgba8::rgb(128, 128, 128)).unwrap()
    }

    #[test]
    fn stroke_record_round_trip() {
        let d = StrokeParams::default();
        assert_eq!(
            d.to_record(),
            "1&100.000000&3.000000&normal&RGBColor;255.000000;0.000000;0.000000&outsetFrame&solidColor&"
        );
        assert_eq!(StrokeParams::from_record(&d.to_record()), d);
        let p = StrokeParams {
            enabled: false,
            size: 7.0,
            position: StrokePosition::Center,
            mode: "multiply".into(),
            opacity: 40.0,
            color: [1, 2, 3],
            paint: "solidColor&".into(),
        };
        assert_eq!(StrokeParams::from_record(&p.to_record()), p);
        // a gradient stroke of an imported file keeps its sub-record
        let g = "1&100.000000&3.000000&normal&nocolor&insetFrame&gradientFill&0&1&90.000000&100.000000&0.000000&0.000000&linear&x,customStops,4096.000000$0$0";
        let gp = StrokeParams::from_record(g);
        assert!(!gp.is_solid());
        assert_eq!(gp.position, StrokePosition::Inside);
        assert_eq!(
            gp.to_record().split_once("&insetFrame&").unwrap().1,
            g.split_once("&insetFrame&").unwrap().1
        );
        assert_eq!(
            StrokePosition::from_name("center"),
            Some(StrokePosition::Center)
        );
        assert_eq!(
            StrokeParams {
                size: 900.0,
                ..d.clone()
            }
            .to_record()
            .split('&')
            .nth(2),
            Some("250.000000")
        );
    }

    #[test]
    fn stroke_outside_inside_center_and_opacity() {
        let obj = gray_box();
        let red = |p: Rgba8| (p.r, p.g, p.b);
        // outside 3 px: two full rows, the third anti-aliased by half (A: coverage width − d)
        let out = render(&obj, &stroke_fx(&StrokeParams::default()));
        assert_eq!(out.bounds(), IRect::new(6, 6, 34, 34));
        assert_eq!(out.pixel(9, 20), Rgba8::new(255, 0, 0, 255));
        assert_eq!(out.pixel(8, 20), Rgba8::new(255, 0, 0, 255));
        assert_eq!(out.pixel(7, 20).a, 128);
        assert_eq!(out.pixel(6, 20).a, 0);
        assert_eq!(out.pixel(10, 20), Rgba8::rgb(128, 128, 128));
        assert_eq!(out.pixel(30, 20), Rgba8::new(255, 0, 0, 255));
        // inside 2 px: band over the object, nothing outside
        let inside = StrokeParams {
            size: 2.0,
            position: StrokePosition::Inside,
            ..StrokeParams::default()
        };
        let out = render(&obj, &stroke_fx(&inside));
        assert_eq!(out.pixel(9, 20).a, 0);
        assert_eq!(red(out.pixel(10, 20)), (255, 0, 0));
        let half = out.pixel(11, 20);
        assert!(
            half.r > 180 && half.r < 200 && half.g > 60 && half.g < 70,
            "{half:?}"
        );
        assert_eq!(out.pixel(12, 20), Rgba8::rgb(128, 128, 128));
        // centre 4 px: 2 outside, 2 inside
        let center = StrokeParams {
            size: 4.0,
            position: StrokePosition::Center,
            ..StrokeParams::default()
        };
        let out = render(&obj, &stroke_fx(&center));
        assert_eq!(out.pixel(9, 20), Rgba8::new(255, 0, 0, 255));
        assert_eq!(out.pixel(8, 20).a, 128);
        assert_eq!(red(out.pixel(10, 20)), (255, 0, 0));
        assert_eq!(out.pixel(12, 20), Rgba8::rgb(128, 128, 128));
        // opacity 50 %: coverage · 128
        let op = StrokeParams {
            opacity: 50.0,
            ..StrokeParams::default()
        };
        let out = render(&obj, &stroke_fx(&op));
        assert_eq!(out.pixel(9, 20), Rgba8::new(255, 0, 0, 128));
        // switched off: the object stays as it is
        let off = StrokeParams {
            enabled: false,
            ..StrokeParams::default()
        };
        assert_eq!(render(&obj, &stroke_fx(&off)), obj);
    }

    #[test]
    fn stroke_blend_mode_and_large_size() {
        let obj = gray_box();
        // multiply inside: red · gray = (128, 0, 0) on the object, plain red on transparency
        let m = StrokeParams {
            size: 4.0,
            position: StrokePosition::Center,
            mode: "multiply".into(),
            ..StrokeParams::default()
        };
        let out = render(&obj, &stroke_fx(&m));
        assert_eq!(out.pixel(10, 20), Rgba8::rgb(128, 0, 0));
        assert_eq!(out.pixel(9, 20), Rgba8::new(255, 0, 0, 255));
        // a big stroke stays fast (distance transform) and round
        let big = StrokeParams {
            size: 120.0,
            ..StrokeParams::default()
        };
        let out = render(&obj, &stroke_fx(&big));
        assert_eq!(out.bounds(), IRect::new(-111, -111, 151, 151));
        // corner: Euclidean distance, the diagonal point at 120 px is outside the band
        assert_eq!(out.pixel(10 - 86, 10 - 86).a, 0);
        assert_eq!(out.pixel(10 - 85, 10 - 85).a, 74);
        assert_eq!(out.pixel(10 - 84, 10 - 84).a, 255);
        assert_eq!(out.pixel(-100, 20).a, 255);
    }
}
