//! Photoshop-Deskriptoren (Action Descriptor, „Photoshop File Formats Specification“) mit den
//! String-IDs, die das Original über die PSL-Bibliothek abfragt (12 §3, Terminologie wie `psd.py`).

use std::collections::BTreeMap;

/// Char-ID → String-ID (Auszug wie `psd.py`).
pub fn charid(s: &str) -> &str {
    match s {
        "Scl " => "scale",
        "DrSh" => "dropShadow",
        "IrSh" => "innerShadow",
        "OrGl" => "outerGlow",
        "IrGl" => "innerGlow",
        "ebbl" => "bevelEmboss",
        "SoFi" => "solidFill",
        "GrFl" => "gradientFill",
        "FrFX" => "frameFX",
        "ChFX" => "chromeFX",
        "enab" => "enabled",
        "Md  " => "mode",
        "Clr " => "color",
        "Opct" => "opacity",
        "uglg" => "useGlobalAngle",
        "lagl" => "localLightingAngle",
        "Dstn" => "distance",
        "Ckmt" => "chokeMatte",
        "blur" => "blur",
        "Nose" => "noise",
        "AntA" => "antiAlias",
        "TrnS" => "transparencyShape",
        "GlwT" => "glowTechnique",
        "Inpr" => "inputRange",
        "ShdN" => "shadingNoise",
        "glwS" => "innerGlowSource",
        "bvlT" => "bevelTechnique",
        "bvlS" => "bevelStyle",
        "srgR" => "strengthRatio",
        "bvlD" => "bevelDirection",
        "hglM" => "highlightMode",
        "hglC" => "highlightColor",
        "hglO" => "highlightOpacity",
        "sdwM" => "shadowMode",
        "sdwC" => "shadowColor",
        "sdwO" => "shadowOpacity",
        "Sftn" => "softness",
        "Lald" => "localLightingAltitude",
        "MpgS" => "mappingShape",
        "Grad" => "gradient",
        "Type" => "type",
        "Angl" => "angle",
        "Algn" => "align",
        "Rvrs" => "reverse",
        "Dthr" => "dither",
        "Ofst" => "offset",
        "Hrzn" => "horizontal",
        "Vrtc" => "vertical",
        "GrdF" => "gradientForm",
        "Clrs" => "colors",
        "Trns" => "transparency",
        "Lctn" => "location",
        "Mdpn" => "midpoint",
        "Intr" => "interpolation",
        "Nm  " => "name",
        "Styl" => "style",
        "PntT" => "paintType",
        "Sz  " => "size",
        "Ptrn" => "pattern",
        "Invr" => "invert",
        "Lnkd" => "linked",
        "RGBC" => "RGBColor",
        "Rd  " => "red",
        "Grn " => "green",
        "Bl  " => "blue",
        "Idnt" => "ID",
        "Crv " => "curve",
        "Cntr" => "contour",
        "Txt " => "text",
        "Vsbl" => "visible",
        "Msge" => "message",
        "url " => "url",
        "Trgt" => "target",
        "Orgn" => "origin",
        "Bnds" => "bounds",
        "Top " => "top",
        "Left" => "left",
        "Btom" => "bottom",
        "Rght" => "right",
        "Cyn " => "cyan",
        "Mgnt" => "magenta",
        "Ylw " => "yellowColor",
        "Blck" => "black",
        "H   " => "hue",
        "Strt" => "saturation",
        "Brgh" => "brightness",
        "Gry " => "gray",
        "HSBC" => "HSBColorClass",
        "CMYC" => "CMYKColorClass",
        "Grsc" => "grayscale",
        "LbCl" => "labColor",
        "Lmnc" => "luminance",
        "Usng" => "using",
        "Lyr " => "layer",
        "Phase" => "phase",
        "Clrt" => "colorStopType",
        // spec 79 §6.2: contour corner points, bevel texture, Lab a/b, noise gradients
        "Cnty" => "continuity",
        "InvT" => "invertTexture",
        "A   " => "a",
        "B   " => "b",
        "ShTr" => "showTransparency",
        "VctC" => "vectorColor",
        "ClrS" => "colorSpace",
        "RndS" => "randomSeed",
        "Smth" => "smoothness",
        "Mnm " => "minimum",
        "Mxm " => "maximum",
        other => other,
    }
}

/// Enum-Char-ID → String-ID.
pub fn enumid(s: &str) -> &str {
    match s {
        "Nrml" => "normal",
        "Mltp" => "multiply",
        "Scrn" => "screen",
        "Ovrl" => "overlay",
        "SftL" => "softLight",
        "HrdL" => "hardLight",
        "CDdg" => "colorDodge",
        "CBrn" => "colorBurn",
        "Drkn" => "darken",
        "Lghn" => "lighten",
        "Dfrn" => "difference",
        "Xclu" => "exclusion",
        "H   " => "hue",
        "Strt" => "saturation",
        "Clr " => "color",
        "Lmns" => "luminosity",
        "Dslv" => "dissolve",
        "SfBL" => "softMatte",
        "PrBL" => "preciseMatte",
        "Slmt" => "slopeLimitMatte",
        "InrB" => "innerBevel",
        "OtrB" => "outerBevel",
        "Embs" => "emboss",
        "PlEb" => "pillowEmboss",
        "In  " => "stampIn",
        "Out " => "stampOut",
        "SrcC" => "centerGlow",
        "SrcE" => "edgeGlow",
        "Lnr " => "linear",
        "Rdl " => "radial",
        "Angl" => "angle",
        "Rflc" => "reflected",
        "Dmnd" => "diamond",
        "CstS" => "customStops",
        "ClNs" => "colorNoise",
        "UsrS" => "userStop",
        "BckC" => "backgroundColor",
        "FrgC" => "foregroundColor",
        "OutF" => "outsetFrame",
        "InsF" => "insetFrame",
        "CtrF" => "centeredFrame",
        "SClr" => "solidColor",
        "GrFl" => "gradientFill",
        "Ptrn" => "pattern",
        // spec 79 §3.4: colour space of a noise gradient (`ClrS`)
        "RGBC" => "RGBColor",
        "HSBl" => "HSBColorEnum",
        "LbCl" => "labColor",
        "#Prc" => "percentUnit",
        "#Pxl" => "pixelsUnit",
        "#Ang" => "angleUnit",
        "#Pnt" => "pointsUnit",
        "#Rlt" => "distanceUnit",
        "#Rsl" => "densityUnit",
        "#Nne" => "noneUnit",
        other => other,
    }
}

/// Wert eines Deskriptors.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Object(Descriptor),
    List(Vec<Value>),
    Double(f64),
    Unit {
        unit: String,
        value: f64,
    },
    Text(String),
    Enum(String),
    Long(i32),
    LargeInt(i64),
    Bool(bool),
    Class(String),
    /// Nicht ausgewertete Rohdaten (Länge).
    Raw(usize),
    Reference,
}

impl Value {
    /// Zahl (Double, Long oder Einheitswert).
    pub fn num(&self) -> Option<f64> {
        match self {
            Value::Double(v) => Some(*v),
            Value::Unit { value, .. } => Some(*value),
            Value::Long(v) => Some(*v as f64),
            Value::LargeInt(v) => Some(*v as f64),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(s) | Value::Enum(s) | Value::Class(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_obj(&self) -> Option<&Descriptor> {
        if let Value::Object(d) = self {
            Some(d)
        } else {
            None
        }
    }
}

/// Ein Deskriptor (Name, Klasse, geordnete Einträge).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Descriptor {
    pub name: String,
    pub class: String,
    pub items: Vec<(String, Value)>,
}

impl Descriptor {
    pub fn get(&self, k: &str) -> Option<&Value> {
        self.items.iter().find(|(n, _)| n == k).map(|(_, v)| v)
    }
    pub fn num(&self, k: &str) -> Option<f64> {
        self.get(k)?.num()
    }
    pub fn bool(&self, k: &str) -> Option<bool> {
        self.get(k)?.as_bool()
    }
    pub fn str(&self, k: &str) -> Option<&str> {
        self.get(k)?.as_str()
    }
    pub fn obj(&self, k: &str) -> Option<&Descriptor> {
        self.get(k)?.as_obj()
    }
    /// Setzt bzw. ersetzt einen Eintrag.
    pub fn set(&mut self, k: &str, v: Value) {
        if let Some(e) = self.items.iter_mut().find(|(n, _)| n == k) {
            e.1 = v;
        } else {
            self.items.push((k.to_string(), v));
        }
    }
    /// Als Wörterbuch (Testhilfe).
    pub fn keys(&self) -> BTreeMap<&str, &Value> {
        self.items.iter().map(|(k, v)| (k.as_str(), v)).collect()
    }
}

/// Big-Endian-Lesekopf mit Grenzprüfung.
#[derive(Clone)]
pub struct Reader<'a> {
    pub b: &'a [u8],
    pub o: usize,
    pub end: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Reader<'a> {
        Reader {
            b,
            o: 0,
            end: b.len(),
        }
    }
    pub fn sub(b: &'a [u8], o: usize, end: usize) -> Reader<'a> {
        Reader {
            b,
            o,
            end: end.min(b.len()),
        }
    }
    pub fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.o.checked_add(n)? > self.end {
            return None;
        }
        let v = &self.b[self.o..self.o + n];
        self.o += n;
        Some(v)
    }
    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|v| v[0])
    }
    pub fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|v| u16::from_be_bytes([v[0], v[1]]))
    }
    pub fn i16(&mut self) -> Option<i16> {
        self.u16().map(|v| v as i16)
    }
    pub fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|v| u32::from_be_bytes([v[0], v[1], v[2], v[3]]))
    }
    pub fn i32(&mut self) -> Option<i32> {
        self.u32().map(|v| v as i32)
    }
    pub fn u64(&mut self) -> Option<u64> {
        self.take(8)
            .map(|v| u64::from_be_bytes([v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]]))
    }
    pub fn f64(&mut self) -> Option<f64> {
        self.u64().map(f64::from_bits)
    }
    pub fn key4(&mut self) -> Option<String> {
        self.take(4).map(|v| v.iter().map(|&c| c as char).collect())
    }
    pub fn left(&self) -> usize {
        self.end.saturating_sub(self.o)
    }
}

fn key(r: &mut Reader) -> Option<String> {
    let n = r.u32()? as usize;
    let s: String = r
        .take(if n == 0 { 4 } else { n.min(1 << 20) })?
        .iter()
        .map(|&c| c as char)
        .collect();
    Some(if n == 0 { charid(&s).to_string() } else { s })
}

fn ustr(r: &mut Reader) -> Option<String> {
    let n = r.u32()? as usize;
    let raw = r.take(n.checked_mul(2)?)?;
    let units: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    Some(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_string(),
    )
}

fn value(r: &mut Reader, t: &str, depth: u32) -> Option<Value> {
    if depth > 64 {
        return None;
    }
    Some(match t {
        "Objc" | "GlbO" => Value::Object(descriptor_d(r, depth + 1)?),
        "VlLs" => {
            let n = r.u32()? as usize;
            let mut v = Vec::with_capacity(n.min(4096));
            for _ in 0..n.min(1 << 16) {
                let tt = r.key4()?;
                v.push(value(r, &tt, depth + 1)?);
            }
            Value::List(v)
        }
        "doub" => Value::Double(r.f64()?),
        "UntF" => {
            let u = r.key4()?;
            Value::Unit {
                unit: enumid(&u).to_string(),
                value: r.f64()?,
            }
        }
        "UnFl" => {
            let u = r.key4()?;
            let n = r.u32()? as usize;
            let mut last = 0.0;
            for _ in 0..n.min(1 << 16) {
                last = r.f64()?;
            }
            Value::Unit {
                unit: enumid(&u).to_string(),
                value: last,
            }
        }
        "TEXT" => Value::Text(ustr(r)?),
        "enum" => {
            key(r)?;
            let e = key(r)?;
            Value::Enum(enumid(&e).to_string())
        }
        "long" => Value::Long(r.i32()?),
        "comp" => Value::LargeInt(r.u64()? as i64),
        "bool" => Value::Bool(r.u8()? != 0),
        "type" | "GlbC" => {
            ustr(r)?;
            Value::Class(key(r)?)
        }
        "alis" | "tdta" | "Pth " => {
            let n = r.u32()? as usize;
            r.take(n)?;
            Value::Raw(n)
        }
        "obj " => {
            let n = r.u32()? as usize;
            for _ in 0..n.min(256) {
                let k = r.key4()?;
                match k.as_str() {
                    "prop" => {
                        ustr(r)?;
                        key(r)?;
                        key(r)?;
                    }
                    "Clss" => {
                        ustr(r)?;
                        key(r)?;
                    }
                    "Enmr" => {
                        ustr(r)?;
                        key(r)?;
                        key(r)?;
                        key(r)?;
                    }
                    "rele" => {
                        ustr(r)?;
                        key(r)?;
                        r.u32()?;
                    }
                    "Idnt" | "indx" => {
                        r.u32()?;
                    }
                    "name" => {
                        ustr(r)?;
                        key(r)?;
                        ustr(r)?;
                    }
                    _ => return None,
                }
            }
            Value::Reference
        }
        "ObAr" => {
            r.u32()?;
            ustr(r)?;
            key(r)?;
            let n = r.u32()? as usize;
            let mut d = Descriptor::default();
            for _ in 0..n.min(4096) {
                let k = key(r)?;
                let tt = r.key4()?;
                let v = if tt == "UnFl" {
                    let u = r.key4()?;
                    let m = r.u32()? as usize;
                    let mut vals = Vec::new();
                    for _ in 0..m.min(1 << 16) {
                        vals.push(Value::Double(r.f64()?));
                    }
                    let _ = u;
                    Value::List(vals)
                } else {
                    value(r, &tt, depth + 1)?
                };
                d.items.push((k, v));
            }
            Value::Object(d)
        }
        _ => return None,
    })
}

fn descriptor_d(r: &mut Reader, depth: u32) -> Option<Descriptor> {
    let name = ustr(r)?;
    let class = key(r)?;
    let n = r.u32()? as usize;
    let mut d = Descriptor {
        name,
        class,
        items: Vec::with_capacity(n.min(256)),
    };
    for _ in 0..n.min(1 << 16) {
        let k = key(r)?;
        let t = r.key4()?;
        let v = value(r, &t, depth)?;
        d.items.push((k, v));
    }
    Some(d)
}

/// Deskriptor lesen (`None` bei beschädigten Daten).
pub fn descriptor(r: &mut Reader) -> Option<Descriptor> {
    descriptor_d(r, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w_key(out: &mut Vec<u8>, k: &str) {
        if k.len() == 4 {
            out.extend_from_slice(&0u32.to_be_bytes());
        } else {
            out.extend_from_slice(&(k.len() as u32).to_be_bytes());
        }
        out.extend_from_slice(k.as_bytes());
    }

    #[test]
    fn parse_small_descriptor() {
        let mut b = Vec::new();
        b.extend_from_slice(&1u32.to_be_bytes());
        b.extend_from_slice(&[0, 0]);
        w_key(&mut b, "null");
        b.extend_from_slice(&3u32.to_be_bytes());
        w_key(&mut b, "Opct");
        b.extend_from_slice(b"UntF#Prc");
        b.extend_from_slice(&75.0f64.to_be_bytes());
        w_key(&mut b, "enab");
        b.extend_from_slice(b"bool\x01");
        w_key(&mut b, "Md  ");
        b.extend_from_slice(b"enum");
        w_key(&mut b, "BlnM");
        w_key(&mut b, "Mltp");
        let d = descriptor(&mut Reader::new(&b)).unwrap();
        assert_eq!(d.num("opacity"), Some(75.0));
        assert_eq!(d.bool("enabled"), Some(true));
        assert_eq!(d.str("mode"), Some("multiply"));
        assert!(descriptor(&mut Reader::new(&b[..10])).is_none());
    }
}
