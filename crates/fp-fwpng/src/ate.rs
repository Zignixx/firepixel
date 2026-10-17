//! `aTXT`: Serialisierung der Text-Engine (ATE) (02 §12.7).
//!
//! Syntax: `chunk := ' ' (name wert)*`, `wert := dict | array | name | zahl | string | true | false`,
//! `dict := '<<' (name wert)* '>>'`, `array := '[' wert* ']'`, `name := '/' Zeichen ohne Leerraum und ohne
//! / < > [ ] ( )`, `string := '(' Bytes ')'` mit `\` als Escape. Das Original trennt alle Tokens mit genau
//! einem Leerzeichen; damit entsteht der Chunk exakt neu. Zahlen behalten ihren Originaltext.
//!
//! Zusätzlich werden die Stories dekodiert (Text, Absatz- und Stil-Läufe mit aufgelösten Attributen),
//! soweit sie für das Layout nötig sind ([`decode`]).

use crate::ate_keys;
use crate::error::{FwError, Result};
use fp_model::text::decode_cos_text;
use fp_model::{AteCharAttrs, AteData, AteParagraphAttrs, AteStory, CosDict, CosValue};

fn err(offset: usize, msg: impl Into<String>) -> FwError {
    FwError::Ate {
        offset,
        msg: msg.into(),
    }
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

fn is_delim(b: u8) -> bool {
    is_ws(b) || matches!(b, b'/' | b'<' | b'>' | b'[' | b']' | b'(' | b')')
}

enum Frame {
    Dict(CosDict, Option<String>),
    Array(Vec<CosValue>),
}

/// Parst den ganzen Chunk; liefert das Wörterbuch der obersten Ebene.
pub fn parse(data: &[u8]) -> Result<CosDict> {
    let n = data.len();
    let mut i = 0usize;
    let mut stack: Vec<Frame> = vec![Frame::Dict(Vec::new(), None)];

    fn put(stack: &mut [Frame], v: CosValue, at: usize) -> Result<()> {
        match stack.last_mut() {
            Some(Frame::Dict(d, pending)) => {
                match pending.take() {
                    None => match v {
                        CosValue::Name(k) => *pending = Some(k),
                        _ => return Err(err(at, "Schlüssel erwartet")),
                    },
                    Some(k) => d.push((k, v)),
                }
                Ok(())
            }
            Some(Frame::Array(a)) => {
                a.push(v);
                Ok(())
            }
            None => Err(err(at, "kein Container")),
        }
    }

    while i < n {
        let c = data[i];
        if is_ws(c) {
            i += 1;
            continue;
        }
        if data[i..].starts_with(b"<<") {
            stack.push(Frame::Dict(Vec::new(), None));
            i += 2;
        } else if data[i..].starts_with(b">>") {
            if stack.len() <= 1 {
                return Err(err(i, "unpassendes >>"));
            }
            match stack.pop() {
                Some(Frame::Dict(d, None)) => put(&mut stack, CosValue::Dict(d), i)?,
                _ => return Err(err(i, "unpassendes >>")),
            }
            i += 2;
        } else if c == b'[' {
            stack.push(Frame::Array(Vec::new()));
            i += 1;
        } else if c == b']' {
            match stack.pop() {
                Some(Frame::Array(a)) => put(&mut stack, CosValue::Array(a), i)?,
                _ => return Err(err(i, "unpassendes ]")),
            }
            i += 1;
        } else if c == b'/' {
            let mut j = i + 1;
            while j < n && !is_delim(data[j]) {
                j += 1;
            }
            let name: String = data[i + 1..j].iter().map(|&b| b as char).collect();
            put(&mut stack, CosValue::Name(name), i)?;
            i = j;
        } else if c == b'(' {
            let mut j = i + 1;
            let mut out = Vec::new();
            loop {
                if j >= n {
                    return Err(err(i, "String ohne Ende"));
                }
                let ch = data[j];
                if ch == b'\\' {
                    let nx = *data.get(j + 1).ok_or_else(|| err(j, "Escape am Ende"))?;
                    out.push(nx);
                    j += 2;
                    continue;
                }
                if ch == b')' {
                    break;
                }
                out.push(ch);
                j += 1;
            }
            put(&mut stack, CosValue::String(out), i)?;
            i = j + 1;
        } else if let Some(len) = number_len(&data[i..]) {
            let t: String = data[i..i + len].iter().map(|&b| b as char).collect();
            put(&mut stack, CosValue::Number(t), i)?;
            i += len;
        } else if data[i..].starts_with(b"true") {
            put(&mut stack, CosValue::Bool(true), i)?;
            i += 4;
        } else if data[i..].starts_with(b"false") {
            put(&mut stack, CosValue::Bool(false), i)?;
            i += 5;
        } else {
            return Err(err(
                i,
                format!(
                    "unbekanntes Token {:?}",
                    String::from_utf8_lossy(&data[i..(i + 20).min(n)])
                ),
            ));
        }
    }
    match stack.pop() {
        Some(Frame::Dict(d, None)) if stack.is_empty() => Ok(d),
        _ => Err(err(n, "unerwartetes Ende")),
    }
}

/// Länge einer Zahl `-?(\d+(\.\d*)?|\.\d+)` am Anfang.
fn number_len(b: &[u8]) -> Option<usize> {
    let mut i = 0;
    if b.first() == Some(&b'-') {
        i = 1;
    }
    let d0 = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i > d0 {
        if i < b.len() && b[i] == b'.' {
            i += 1;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
        return Some(i);
    }
    if i < b.len() && b[i] == b'.' {
        let s = i + 1;
        let mut j = s;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j > s {
            return Some(j);
        }
    }
    None
}

/// Serialisiert in der kanonischen Form (ein Leerzeichen zwischen Tokens, führendes Leerzeichen).
pub fn serialize(top: &CosDict) -> Vec<u8> {
    let mut out = Vec::with_capacity(4096);
    out.push(b' ');
    let mut first = true;
    let mut tok = |out: &mut Vec<u8>, t: &[u8]| {
        if !first {
            out.push(b' ');
        }
        first = false;
        out.extend_from_slice(t);
    };
    // Iterativ: Stapel aus Arbeitsschritten.
    enum W<'a> {
        Val(&'a CosValue),
        Key(&'a str),
        Raw(&'static [u8]),
    }
    let mut work: Vec<W> = Vec::new();
    for (k, v) in top.iter().rev() {
        work.push(W::Val(v));
        work.push(W::Key(k));
    }
    while let Some(w) = work.pop() {
        match w {
            W::Raw(t) => tok(&mut out, t),
            W::Key(k) => {
                let mut t = Vec::with_capacity(k.len() + 1);
                t.push(b'/');
                t.extend(k.chars().map(|c| c as u8));
                tok(&mut out, &t);
            }
            W::Val(v) => match v {
                CosValue::Dict(d) => {
                    tok(&mut out, b"<<");
                    work.push(W::Raw(b">>"));
                    for (k, x) in d.iter().rev() {
                        work.push(W::Val(x));
                        work.push(W::Key(k));
                    }
                }
                CosValue::Array(a) => {
                    tok(&mut out, b"[");
                    work.push(W::Raw(b"]"));
                    for x in a.iter().rev() {
                        work.push(W::Val(x));
                    }
                }
                CosValue::Name(nm) => {
                    let mut t = Vec::with_capacity(nm.len() + 1);
                    t.push(b'/');
                    t.extend(nm.chars().map(|c| c as u8));
                    tok(&mut out, &t);
                }
                CosValue::Number(t) => tok(&mut out, t.as_bytes()),
                CosValue::Bool(b) => tok(&mut out, if *b { b"true" } else { b"false" }),
                CosValue::String(s) => {
                    let mut t = Vec::with_capacity(s.len() + 2);
                    t.push(b'(');
                    for &c in s {
                        if matches!(c, b'\\' | b'(' | b')') {
                            t.push(b'\\');
                        }
                        t.push(c);
                    }
                    t.push(b')');
                    tok(&mut out, &t);
                }
            },
        }
    }
    out
}

fn dict_get<'a>(d: &'a CosDict, key: &str) -> Option<&'a CosValue> {
    d.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn path<'a>(root: &'a CosDict, keys: &[&str]) -> Option<&'a CosValue> {
    let (first, rest) = keys.split_first()?;
    let mut cur = dict_get(root, first)?;
    for k in rest {
        cur = cur.get(k)?;
    }
    Some(cur)
}

/// Wert eines Zahlenschlüssels in einem ATE-Wörterbuch; alte Dateien schreiben manche Schlüssel im
/// Klartext (Tabelle `ate_keys`).
fn attr<'a>(d: &'a CosValue, class: Option<&ate_keys::AteClass>, key: u16) -> Option<&'a CosValue> {
    if let Some(v) = d.get(&key.to_string()) {
        return Some(v);
    }
    let name = ate_keys::key_name(class?, key)?;
    d.get(name)
}

fn as_i64(v: &CosValue) -> Option<i64> {
    v.as_f64().map(|f| f as i64)
}

/// Dekodiert `aTXT`: Rohbaum plus Stories. Bei Syntaxfehlern bleibt der Chunk roh erhalten.
pub fn decode(data: &[u8]) -> AteData {
    let root = match parse(data) {
        Ok(r) => r,
        Err(_) => {
            return AteData {
                root: Vec::new(),
                unparsed: Some(data.to_vec()),
                version: None,
                fonts: Vec::new(),
                stories: Vec::new(),
            };
        }
    };
    let version = path(&root, &["98", "0"]).and_then(as_i64);
    let fonts: Vec<String> = path(&root, &["0", "1", "0"])
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|f| {
                    f.get("0")
                        .and_then(|x| x.get("0"))
                        .and_then(|x| x.get("0"))
                        .and_then(|x| x.as_text())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default();
    let char_class = ate_keys::class("StyleSheetData");
    let para_class = ate_keys::class("ParagraphProperties");
    let style_sheets: Vec<&CosValue> = path(&root, &["0", "5", "0"])
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.get("0")).collect())
        .unwrap_or_default();
    let para_sheets: Vec<&CosValue> = path(&root, &["0", "6", "0"])
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|s| s.get("0")).collect())
        .unwrap_or_default();
    let char_defaults = path(&root, &["1", "2"]);
    let para_defaults = path(&root, &["1", "3"]);

    let mut stories = Vec::new();
    if let Some(list) = path(&root, &["1", "1"]).and_then(|v| v.as_array()) {
        for st in list {
            let model = st.get("0");
            let text = model
                .and_then(|m| m.get("0"))
                .and_then(|t| t.as_text())
                .unwrap_or_default();
            let mut story = AteStory {
                text,
                has_view: st.get("1").is_some(),
                ..AteStory::default()
            };
            story.antialias = model
                .and_then(|m| m.get("10"))
                .and_then(|s| s.get("0"))
                .and_then(as_i64);
            // Absatzläufe.
            if let Some(runs) = model
                .and_then(|m| m.get("5"))
                .and_then(|r| r.get("0"))
                .and_then(|a| a.as_array())
            {
                for r in runs {
                    let len = r.get("1").and_then(as_i64).unwrap_or(0).max(0) as u32;
                    let sheet = r.get("0").and_then(|d| d.get("0"));
                    let mut attrs = AteParagraphAttrs::default();
                    if let Some(d) = para_defaults {
                        apply_para(&mut attrs, d, para_class);
                    }
                    if let Some(s) = sheet {
                        let parent = s.get("6").and_then(as_i64);
                        apply_para_chain(&mut attrs, parent, &para_sheets, para_class, 0);
                        if let Some(f) = s.get("5") {
                            apply_para(&mut attrs, f, para_class);
                        }
                    }
                    story.paragraph_runs.push((len, attrs));
                }
            }
            // Stil-Läufe.
            if let Some(runs) = model
                .and_then(|m| m.get("6"))
                .and_then(|r| r.get("0"))
                .and_then(|a| a.as_array())
            {
                for r in runs {
                    let len = r.get("1").and_then(as_i64).unwrap_or(0).max(0) as u32;
                    let sheet = r.get("0").and_then(|d| d.get("0"));
                    let mut attrs = AteCharAttrs::default();
                    if let Some(d) = char_defaults {
                        apply_char(&mut attrs, d, char_class, &fonts);
                    }
                    if let Some(s) = sheet {
                        let parent = s.get("5").and_then(as_i64);
                        apply_char_chain(&mut attrs, parent, &style_sheets, char_class, &fonts, 0);
                        if let Some(f) = s.get("6") {
                            apply_char(&mut attrs, f, char_class, &fonts);
                        }
                    }
                    story.style_runs.push((len, attrs));
                }
            }
            stories.push(story);
        }
    }
    AteData {
        root,
        unparsed: None,
        version,
        fonts,
        stories,
    }
}

fn apply_char_chain(
    a: &mut AteCharAttrs,
    idx: Option<i64>,
    sheets: &[&CosValue],
    class: Option<&ate_keys::AteClass>,
    fonts: &[String],
    depth: u32,
) {
    let Some(i) = idx else { return };
    if depth > 8 || i < 0 {
        return;
    }
    let Some(s) = sheets.get(i as usize) else {
        return;
    };
    apply_char_chain(
        a,
        s.get("5").and_then(as_i64),
        sheets,
        class,
        fonts,
        depth + 1,
    );
    if let Some(f) = s.get("6") {
        apply_char(a, f, class, fonts);
    }
}

fn apply_para_chain(
    a: &mut AteParagraphAttrs,
    idx: Option<i64>,
    sheets: &[&CosValue],
    class: Option<&ate_keys::AteClass>,
    depth: u32,
) {
    let Some(i) = idx else { return };
    if depth > 8 || i < 0 {
        return;
    }
    let Some(s) = sheets.get(i as usize) else {
        return;
    };
    apply_para_chain(a, s.get("6").and_then(as_i64), sheets, class, depth + 1);
    if let Some(f) = s.get("5") {
        apply_para(a, f, class);
    }
}

fn paint(v: &CosValue) -> Option<Vec<f64>> {
    let inner = v.get("0")?;
    let vals = inner.get("1")?.as_array()?;
    Some(vals.iter().filter_map(|x| x.as_f64()).collect())
}

fn apply_char(
    a: &mut AteCharAttrs,
    f: &CosValue,
    c: Option<&ate_keys::AteClass>,
    fonts: &[String],
) {
    if let Some(i) = attr(f, c, 0).and_then(as_i64)
        && i >= 0
        && let Some(n) = fonts.get(i as usize)
    {
        a.font = Some(n.clone());
    }
    macro_rules! num {
        ($field:ident, $k:expr) => {
            if let Some(v) = attr(f, c, $k).and_then(|v| v.as_f64()) {
                a.$field = Some(v);
            }
        };
    }
    macro_rules! int {
        ($field:ident, $k:expr) => {
            if let Some(v) = attr(f, c, $k).and_then(as_i64) {
                a.$field = Some(v);
            }
        };
    }
    macro_rules! boolean {
        ($field:ident, $k:expr) => {
            if let Some(v) = attr(f, c, $k).and_then(|v| v.as_bool()) {
                a.$field = Some(v);
            }
        };
    }
    num!(font_size, 1);
    boolean!(faux_bold, 2);
    boolean!(faux_italic, 3);
    boolean!(auto_leading, 4);
    num!(leading, 5);
    num!(horizontal_scale, 6);
    num!(vertical_scale, 7);
    num!(tracking, 8);
    num!(baseline_shift, 9);
    int!(auto_kern, 11);
    int!(font_caps, 12);
    int!(font_baseline, 13);
    int!(strikethrough, 15);
    int!(underline, 16);
    boolean!(ligatures, 18);
    int!(language, 38);
    if let Some(p) = attr(f, c, 53).and_then(paint) {
        a.fill_color = Some(p);
    }
    if let Some(p) = attr(f, c, 54).and_then(paint) {
        a.stroke_color = Some(p);
    }
    boolean!(fill_flag, 56);
    boolean!(stroke_flag, 57);
}

fn apply_para(a: &mut AteParagraphAttrs, f: &CosValue, c: Option<&ate_keys::AteClass>) {
    macro_rules! num {
        ($field:ident, $k:expr) => {
            if let Some(v) = attr(f, c, $k).and_then(|v| v.as_f64()) {
                a.$field = Some(v);
            }
        };
    }
    if let Some(v) = attr(f, c, 0).and_then(as_i64) {
        a.justification = Some(v);
    }
    num!(first_line_indent, 1);
    num!(start_indent, 2);
    num!(end_indent, 3);
    num!(space_before, 4);
    num!(space_after, 5);
    num!(auto_leading, 7);
    if let Some(v) = attr(f, c, 8).and_then(as_i64) {
        a.leading_type = Some(v);
    }
    if let Some(v) = attr(f, c, 9).and_then(|v| v.as_bool()) {
        a.auto_hyphenate = Some(v);
    }
    num!(default_tab_width, 31);
    if let Some(tabs) = attr(f, c, 30)
        .and_then(|t| t.get("0"))
        .and_then(|a| a.as_array())
    {
        a.tab_stops = tabs
            .iter()
            .filter_map(|t| {
                Some((
                    t.get("0")?.as_f64()?,
                    t.get("1").and_then(as_i64).unwrap_or(0),
                ))
            })
            .collect();
    }
}

/// Text einer Story als String (Hilfe für Tests und Anzeige).
pub fn story_texts(top: &CosDict) -> Vec<Option<String>> {
    path(top, &["1", "1"])
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|s| {
                    s.get("0")
                        .and_then(|m| m.get("0"))
                        .and_then(|t| t.as_text())
                })
                .collect()
        })
        .unwrap_or_default()
}

/// COS-Bytes als Text (Latin-1/UTF-16), für Diagnose.
pub fn cos_text(b: &[u8]) -> String {
    decode_cos_text(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] = b" /98 << /0 6 >> /0 << /1 << /0 [ << /0 << /99 /CoolTypeFont /0 << /0 (\xfe\xff\x00A\x00r\x00i\x00a\x00l) /2 1 >> >> >> ] >> /5 << /0 [ << /0 << /0 (\xfe\xff\x00N) /6 << /0 0 /1 12.0 /2 false >> >> >> ] >> >> /1 << /1 [ << /0 << /0 (\xfe\xff\x00H\x00i\x00\\)) /6 << /0 [ << /0 << /0 << /0 () /5 0 /6 << /1 10.0 /2 true >> >> >> /1 3 >> ] >> >> /1 << /0 .5 >> >> ] /2 << /0 0 /1 12.0 /11 1 >> >>";

    #[test]
    fn parse_roundtrip_and_decode() {
        let top = parse(SAMPLE).unwrap();
        assert_eq!(serialize(&top), SAMPLE.to_vec());
        let d = decode(SAMPLE);
        assert_eq!(d.version, Some(6));
        assert_eq!(d.fonts, vec!["Arial".to_string()]);
        assert_eq!(d.stories.len(), 1);
        let s = &d.stories[0];
        assert_eq!(s.text, "Hi)");
        assert!(s.has_view);
        let (len, a) = &s.style_runs[0];
        assert_eq!(*len, 3);
        assert_eq!(a.font.as_deref(), Some("Arial"));
        assert_eq!(a.font_size, Some(10.0));
        assert_eq!(a.faux_bold, Some(true));
        assert_eq!(a.auto_kern, Some(1));
    }

    #[test]
    fn errors() {
        assert!(parse(b" /0 << /1 2").is_err());
        assert!(parse(b" /0 ]").is_err());
        assert!(parse(b" /0 (abc").is_err());
        assert!(parse(b" 5").is_err());
        let d = decode(b" /0 ?");
        assert!(d.unparsed.is_some());
        assert_eq!(number_len(b".5 "), Some(2));
        assert_eq!(number_len(b"-12.0/"), Some(5));
        assert_eq!(number_len(b"-x"), None);
    }
}
