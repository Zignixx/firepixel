//! Der getaggte Textbaum von `mkTS`/`mkBS` (Spezifikation 02 §3).
//!
//! Grammatik: `node := TAG TYP '{' payload '}'` mit TAG = 3 Zeichen, TYP ∈ {v,i,f,b,s,y}. `v` enthält
//! Kindknoten, `i` eine Hexzahl (32 Bit), `f` eine Dezimalzahl, `b` 0/1, `s` einen UTF-16BE-String mit
//! u16-Längenpräfix, `y` einen Binärblock mit u32-Längenpräfix. Zwischen Knoten stehen keine Trenner;
//! wie der Original-Leser tolerieren wir TAB/LF/CR/Leerzeichen vor einem Tag und vor `{`.
//!
//! Parsen und Serialisieren sind iterativ (die Bäume sind tief) und exakt umkehrbar: Mit der
//! Zahlformatierung aus [`format_float`] entstehen alle Korpusbäume byte-identisch neu.

use crate::error::{FwError, Result};
use fp_model::{RawNode, RawValue, TagKey};

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn err(offset: usize, msg: impl Into<String>) -> FwError {
    FwError::Tree { offset, msg: msg.into() }
}

/// Parst einen kompletten Strom; liefert die Wurzelknoten.
pub fn parse(data: &[u8]) -> Result<Vec<RawNode>> {
    let mut stack: Vec<(TagKey, Vec<RawNode>)> = Vec::new();
    let mut roots: Vec<RawNode> = Vec::new();
    let mut i = 0usize;
    let n = data.len();
    while i < n {
        let c = data[i];
        if is_ws(c) {
            i += 1;
            continue;
        }
        if c == b'}' {
            let (key, kids) = stack.pop().ok_or_else(|| err(i, "überzähliges '}'"))?;
            let node = RawNode::list(key, kids);
            match stack.last_mut() {
                Some((_, v)) => v.push(node),
                None => roots.push(node),
            }
            i += 1;
            continue;
        }
        if i + 4 > n {
            return Err(err(i, "Knoten abgeschnitten"));
        }
        let key = TagKey([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        if !key.0.iter().all(|b| b.is_ascii_graphic()) {
            return Err(err(i, format!("ungültiger Tag {:?}", &data[i..i + 4])));
        }
        let mut k = i + 4;
        while k < n && is_ws(data[k]) {
            k += 1;
        }
        if k >= n || data[k] != b'{' {
            return Err(err(i, format!("kein '{{' nach Tag {key}")));
        }
        let start = k + 1;
        let value = match key.kind() {
            b'v' => {
                stack.push((key, Vec::new()));
                i = start;
                continue;
            }
            b's' => {
                if start + 2 > n {
                    return Err(err(start, "String-Länge abgeschnitten"));
                }
                let len = u16::from_be_bytes([data[start], data[start + 1]]) as usize;
                let body = start + 2;
                let end = body + 2 * len;
                if end >= n || data[end] != b'}' {
                    return Err(err(start, format!("String {key} nicht mit '}}' beendet")));
                }
                let units: Vec<u16> =
                    data[body..end].as_chunks::<2>().0.iter().map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                i = end + 1;
                match String::from_utf16(&units) {
                    Ok(s) => RawValue::Str(s),
                    Err(_) => RawValue::Utf16(units),
                }
            }
            b'y' => {
                if start + 4 > n {
                    return Err(err(start, "Blocklänge abgeschnitten"));
                }
                let len = u32::from_be_bytes([data[start], data[start + 1], data[start + 2], data[start + 3]]) as usize;
                let body = start + 4;
                let end = body.checked_add(len).ok_or_else(|| err(start, "Blocklänge ungültig"))?;
                if end >= n || data[end] != b'}' {
                    return Err(err(start, format!("Binärblock {key} nicht mit '}}' beendet")));
                }
                i = end + 1;
                RawValue::Bytes(data[body..end].to_vec())
            }
            b'i' | b'f' | b'b' => {
                let rel = data[start..].iter().position(|&b| b == b'}').ok_or_else(|| err(start, "fehlendes '}'"))?;
                let txt = std::str::from_utf8(&data[start..start + rel]).map_err(|_| err(start, "Zahl ist kein ASCII"))?;
                i = start + rel + 1;
                match key.kind() {
                    b'i' => {
                        let t = txt.trim();
                        if t.is_empty() {
                            return Err(err(start, format!("leere Zahl bei {key}")));
                        }
                        let v = u64::from_str_radix(t, 16).map_err(|_| err(start, format!("ungültige Hexzahl {t:?}")))?;
                        if v > u32::MAX as u64 {
                            return Err(err(start, format!("Hexzahl {t:?} größer als 32 Bit")));
                        }
                        RawValue::Int(v as u32)
                    }
                    b'f' => {
                        let v: f64 = txt.trim().parse().map_err(|_| err(start, format!("ungültige Zahl {txt:?}")))?;
                        RawValue::Num(v)
                    }
                    _ => RawValue::Bool(txt != "0"),
                }
            }
            other => return Err(err(i, format!("unbekannter Typ {:?} bei Tag {key}", other as char))),
        };
        let node = RawNode::new(key, value);
        match stack.last_mut() {
            Some((_, v)) => v.push(node),
            None => roots.push(node),
        }
    }
    if !stack.is_empty() {
        return Err(err(n, format!("unerwartetes Ende, {} offene Listen", stack.len())));
    }
    Ok(roots)
}

/// Zahlformat wie in Original-Dateien: C-`%f` (6 Nachkommastellen), danach Nullen und Punkt am Ende
/// abschneiden; negative Null bleibt `-0`.
///
/// Gerundet wird wie die C-Laufzeit des Originals: zuerst auf 17 signifikante Stellen, dann auf
/// 6 Nachkommastellen mit „halb weg von null“. Für Werte, die aus Dateitext gelesen wurden, ist das
/// identisch mit korrekter Rundung; bei aus `f32` stammenden Werten mit exakt 7 Nachkommastellen
/// (z. B. 16,0078125) entspricht es dem Verhalten der alten C-Laufzeit.
pub fn format_float(v: f64) -> String {
    if v.is_nan() {
        return "nan".to_string();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".to_string() } else { "-inf".to_string() };
    }
    let neg = v.is_sign_negative();
    let a = v.abs();
    
    let digits_int: Vec<u8> = if a == 0.0 {
        vec![0]
    } else {
        let sci = format!("{:.16e}", a);
        let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
        let exp: i32 = exp.parse().unwrap_or(0);
        let d: Vec<u8> = mant.bytes().filter(|b| b.is_ascii_digit()).map(|b| b - b'0').collect();
        // Anzahl der zu behaltenden Ziffern = Ganzzahlstellen + 6.
        let k = exp + 1 + 6;
        let mut n: Vec<u8> = if k <= 0 {
            if k == 0 && d.first().copied().unwrap_or(0) >= 5 { vec![1] } else { vec![0] }
        } else if (k as usize) >= d.len() {
            let mut x = d.clone();
            x.resize(k as usize, 0);
            x
        } else {
            let mut x = d[..k as usize].to_vec();
            if d[k as usize] >= 5 {
                // Aufrunden mit Übertrag.
                let mut j = x.len();
                loop {
                    if j == 0 {
                        x.insert(0, 1);
                        break;
                    }
                    j -= 1;
                    if x[j] == 9 {
                        x[j] = 0;
                    } else {
                        x[j] += 1;
                        break;
                    }
                }
            }
            x
        };
        while n.len() > 1 && n[0] == 0 {
            n.remove(0);
        }
        n
    };
    // digits_int = Wert · 10^6 als Ziffernfolge.
    let mut s: Vec<u8> = digits_int.iter().map(|d| d + b'0').collect();
    while s.len() < 7 {
        s.insert(0, b'0');
    }
    let split = s.len() - 6;
    let int_part = std::str::from_utf8(&s[..split]).unwrap_or("0");
    let frac = std::str::from_utf8(&s[split..]).unwrap_or("").trim_end_matches('0');
    let mut out = String::with_capacity(s.len() + 2);
    if neg {
        out.push('-');
    }
    out.push_str(int_part);
    if !frac.is_empty() {
        out.push('.');
        out.push_str(frac);
    }
    out
}

/// Serialisiert Knoten (Umkehrung von [`parse`]).
pub fn serialize(nodes: &[RawNode]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4096);
    serialize_into(nodes, &mut out);
    out
}

/// Serialisiert in einen vorhandenen Puffer.
pub fn serialize_into(nodes: &[RawNode], out: &mut Vec<u8>) {
    let mut stack: Vec<std::slice::Iter<'_, RawNode>> = vec![nodes.iter()];
    while let Some(top) = stack.last_mut() {
        let Some(n) = top.next() else {
            stack.pop();
            if !stack.is_empty() {
                out.push(b'}');
            }
            continue;
        };
        out.extend_from_slice(&n.key.0);
        out.push(b'{');
        match &n.value {
            RawValue::List(kids) => {
                stack.push(kids.iter());
                continue;
            }
            RawValue::Int(v) => out.extend_from_slice(format!("{v:x}").as_bytes()),
            RawValue::Num(v) => out.extend_from_slice(format_float(*v).as_bytes()),
            RawValue::Bool(b) => out.push(if *b { b'1' } else { b'0' }),
            RawValue::Str(s) => {
                let units: Vec<u16> = s.encode_utf16().collect();
                write_utf16(out, &units);
            }
            RawValue::Utf16(units) => write_utf16(out, units),
            RawValue::Bytes(b) => {
                out.extend_from_slice(&(b.len() as u32).to_be_bytes());
                out.extend_from_slice(b);
            }
        }
        out.push(b'}');
    }
}

fn write_utf16(out: &mut Vec<u8>, units: &[u16]) {
    // Längenfeld ist u16; längere Strings werden abgeschnitten (das Original teilt lange Skripte selbst).
    let n = units.len().min(u16::MAX as usize);
    out.extend_from_slice(&(n as u16).to_be_bytes());
    for u in &units[..n] {
        out.extend_from_slice(&u.to_be_bytes());
    }
}

/// Lesbare, eingerückte Darstellung (wie `fwpng.py tree`).
pub fn pretty(nodes: &[RawNode], max_str: usize) -> String {
    let mut out = String::new();
    let mut stack: Vec<(std::slice::Iter<'_, RawNode>, usize)> = vec![(nodes.iter(), 0)];
    while let Some((it, depth)) = stack.last_mut() {
        let d = *depth;
        let Some(n) = it.next() else {
            stack.pop();
            if !stack.is_empty() {
                out.push_str(&"  ".repeat(d - 1));
                out.push_str("}\n");
            }
            continue;
        };
        let pad = "  ".repeat(d);
        match &n.value {
            RawValue::List(k) if k.is_empty() => out.push_str(&format!("{pad}{} {{}}\n", n.key)),
            RawValue::List(k) => {
                out.push_str(&format!("{pad}{} {{\n", n.key));
                stack.push((k.iter(), d + 1));
            }
            RawValue::Int(v) => {
                let s = *v as i32;
                if s < 0 || *v > 9 {
                    out.push_str(&format!("{pad}{} {v:x}  (={s})\n", n.key));
                } else {
                    out.push_str(&format!("{pad}{} {v:x}\n", n.key));
                }
            }
            RawValue::Num(v) => out.push_str(&format!("{pad}{} {}\n", n.key, format_float(*v))),
            RawValue::Bool(b) => out.push_str(&format!("{pad}{} {}\n", n.key, *b as u8)),
            RawValue::Str(s) => {
                let shown: String = s.chars().take(max_str).collect();
                let more = if s.chars().count() > max_str { format!("...({} Zeichen)", s.chars().count()) } else { String::new() };
                out.push_str(&format!("{pad}{} {:?}{more}\n", n.key, shown));
            }
            RawValue::Utf16(u) => out.push_str(&format!("{pad}{} <UTF-16, {} Einheiten>\n", n.key, u.len())),
            RawValue::Bytes(b) => out.push_str(&format!("{pad}{} <{} Byte binär>\n", n.key, b.len())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_like_original() {
        assert_eq!(format_float(0.0), "0");
        assert_eq!(format_float(-0.0), "-0");
        assert_eq!(format_float(994.599976), "994.599976");
        assert_eq!(format_float(0.5), "0.5");
        assert_eq!(format_float(-30.285004), "-30.285004");
        assert_eq!(format_float(72.0), "72");
        assert_eq!(format_float(-1e-9), "-0");
        assert_eq!(format_float(0.9999996), "1");
        assert_eq!(format_float(123456.5), "123456.5");
        // f32-Werte: 17 Stellen, dann halb weg von null.
        assert_eq!(format_float(0.1f32 as f64), "0.1");
        assert_eq!(format_float(16.0078125), "16.007813");
        assert_eq!(format_float(0.70710677f32 as f64), "0.707107");
        assert_eq!(format_float(31999.998046875), "31999.998047");
        assert_eq!(format_float(1e7), "10000000");
    }

    #[test]
    fn parse_serialize_roundtrip() {
        let src = b"MKBv{VRSi{a}FRCi{1}XLCf{-40}WIDf{80.5}LCKb{0}PGNs{\x00\x02\x00A\x00}}PTRy{\x00\x00\x00\x03abc}}";
        // Achtung: PTRy steht innerhalb von MKBv, die zweite '}' schließt MKBv.
        let nodes = parse(src).unwrap();
        assert_eq!(nodes.len(), 1);
        let mkb = &nodes[0];
        assert_eq!(mkb.children().len(), 7);
        assert_eq!(mkb.child(TagKey::new(b"PGNs")).and_then(|c| c.as_str()), Some("A}"));
        assert_eq!(serialize(&nodes), src.to_vec());
    }

    #[test]
    fn tolerant_whitespace_and_hex() {
        let nodes = parse(b" MKBv {\n VRSi{0A}\r\n}").unwrap();
        assert_eq!(nodes[0].children()[0].as_int(), Some(10));
        assert_eq!(serialize(&nodes), b"MKBv{VRSi{a}}".to_vec());
    }

    #[test]
    fn errors_not_panics() {
        for bad in [&b"MKBv{"[..], b"}", b"MKBx{1}", b"MKBs{\x00\x05ab}", b"VRSi{zz}", b"VRSi{1ffffffff}", b"AB"] {
            assert!(parse(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
        // Ungültiges UTF-16 bleibt erhalten.
        let src = b"NAMs{\x00\x01\xd8\x00}";
        let nodes = parse(src).unwrap();
        assert!(matches!(nodes[0].value, RawValue::Utf16(_)));
        assert_eq!(serialize(&nodes), src.to_vec());
    }
}
