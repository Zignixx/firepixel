//! Umwandlungen zwischen JS-Werten und Rust/JSON nach den Typregeln von 03 §10.2/§10.11
//! (lockere String↔Zahl-Konvertierung, boolean aus Strings, `null` je nach Typ).

use crate::bindings::{hr, TypeCode};
use crate::state::FwErr;
use boa_engine::object::builtins::JsArray;
use boa_engine::property::PropertyKey;
use boa_engine::{js_string, Context, JsResult, JsString, JsValue};
use serde_json::{Map, Number, Value};

/// Name der versteckten Handle-Eigenschaft der Wrapper.
pub const HANDLE_KEY: &str = "__fp";

/// Name der versteckten Markierung für Wirtobjekte/-Arrays (von Firepixel erzeugte Werte). `toSource`
/// gibt sie wie die Wirtobjekte des Originals aus (`{ x:1 }`, `[ 1 ]`, leer `[  ]`), reine JS-Werte
/// dagegen wie SpiderMonkey (`({x:1})`, `[1, , 2]`).
pub const HOST_KEY: &str = "__fpHost";

/// Markiert ein Objekt/Array als Wirtwert (nicht aufzählbar, unsichtbar für `for…in`/`Object.keys`).
pub fn mark_host(o: &boa_engine::JsObject, ctx: &mut Context) -> JsResult<()> {
    use boa_engine::property::PropertyDescriptor;
    o.define_property_or_throw(
        JsString::from(HOST_KEY),
        PropertyDescriptor::builder().value(true).writable(false).enumerable(false).configurable(true),
        ctx,
    )?;
    Ok(())
}

/// Ist der Wert ein Wirtobjekt/-array (markiert oder FwArray)?
pub fn is_host(v: &JsValue, ctx: &mut Context) -> bool {
    let Some(o) = v.as_object() else { return false };
    o.has_own_property(JsString::from(HOST_KEY), ctx).unwrap_or(false) || is_fwarray(v, ctx)
}

/// String als JS-Wert.
pub fn s(v: &str) -> JsValue {
    JsValue::from(JsString::from(v))
}

/// Zahl als JS-Wert.
pub fn n(v: f64) -> JsValue {
    if v.fract() == 0.0 && v.abs() < 2_147_483_647.0 {
        JsValue::from(v as i32)
    } else {
        JsValue::from(v)
    }
}

/// Handle-Index eines Wrapper-Objekts.
pub fn handle_of(v: &JsValue, ctx: &mut Context) -> Option<usize> {
    let o = v.as_object()?;
    let h = o.get(js_string!("__fp"), ctx).ok()?;
    h.as_number().filter(|x| *x >= 0.0).map(|x| x as usize)
}

/// Tiefe Umwandlung JS → JSON. Wrapper werden zu `{"__h": index}`, Funktionen zu `null`.
pub fn js_to_json(v: &JsValue, ctx: &mut Context, depth: u32) -> JsResult<Value> {
    if depth > 24 {
        return Ok(Value::Null);
    }
    if v.is_null_or_undefined() {
        return Ok(Value::Null);
    }
    if let Some(b) = v.as_boolean() {
        return Ok(Value::Bool(b));
    }
    if let Some(x) = v.as_number() {
        if x.fract() == 0.0 && x.abs() < 9.0e15 {
            return Ok(Value::Number(Number::from(x as i64)));
        }
        return Ok(Number::from_f64(x).map(Value::Number).unwrap_or(Value::Null));
    }
    if let Some(st) = v.as_string() {
        return Ok(Value::String(st.to_std_string_escaped()));
    }
    let Some(o) = v.as_object() else { return Ok(Value::Null) };
    if o.is_callable() {
        return Ok(Value::Null);
    }
    if let Some(h) = handle_of(v, ctx) {
        return Ok(serde_json::json!({ "__h": h }));
    }
    let len_v = o.get(js_string!("length"), ctx)?;
    if o.is_array() || (len_v.as_number().is_some() && is_fwarray(v, ctx)) {
        let len = len_v.as_number().unwrap_or(0.0).clamp(0.0, 1.0e6) as u32;
        let mut out = Vec::with_capacity(len as usize);
        for i in 0..len {
            let e = o.get(i, ctx)?;
            out.push(js_to_json(&e, ctx, depth + 1)?);
        }
        return Ok(Value::Array(out));
    }
    let keys = o.own_property_keys(ctx)?;
    let mut m = Map::new();
    for k in keys {
        let name = match &k {
            PropertyKey::String(s) => s.to_std_string_escaped(),
            PropertyKey::Index(i) => i.get().to_string(),
            PropertyKey::Symbol(_) => continue,
        };
        if name == HANDLE_KEY || name == HOST_KEY {
            continue;
        }
        let e = o.get(k, ctx)?;
        if e.is_undefined() || e.as_object().is_some_and(|x| x.is_callable()) {
            continue;
        }
        m.insert(name, js_to_json(&e, ctx, depth + 1)?);
    }
    Ok(Value::Object(m))
}

/// Ist der Wert ein FwArray (Prototyp `FwArray.prototype`)?
pub fn is_fwarray(v: &JsValue, ctx: &mut Context) -> bool {
    let Some(o) = v.as_object() else { return false };
    let Ok(ctor) = ctx.global_object().get(js_string!("FwArray"), ctx) else { return false };
    let Some(c) = ctor.as_object() else { return false };
    let Ok(proto) = c.get(js_string!("prototype"), ctx) else { return false };
    match (o.prototype(), proto.as_object()) {
        (Some(p), Some(q)) => p == q,
        _ => false,
    }
}

/// JSON → JS (Objekte und Arrays als normale JS-Objekte, als Wirtwerte markiert).
pub fn json_to_js(v: &Value, ctx: &mut Context) -> JsResult<JsValue> {
    Ok(match v {
        Value::Null => JsValue::null(),
        Value::Bool(b) => JsValue::from(*b),
        Value::Number(x) => n(x.as_f64().unwrap_or(0.0)),
        Value::String(t) => s(t),
        Value::Array(a) => {
            let mut items = Vec::with_capacity(a.len());
            for e in a {
                items.push(json_to_js(e, ctx)?);
            }
            let arr = JsArray::from_iter(items, ctx);
            mark_host(&arr, ctx)?;
            arr.into()
        }
        Value::Object(m) => {
            let o = boa_engine::JsObject::with_object_proto(ctx.intrinsics());
            for (k, e) in m {
                let jv = json_to_js(e, ctx)?;
                o.create_data_property_or_throw(JsString::from(k.as_str()), jv, ctx)?;
            }
            mark_host(&o, ctx)?;
            o.into()
        }
    })
}

/// Normales JS-Array (als Wirtwert markiert).
pub fn array(items: Vec<JsValue>, ctx: &mut Context) -> JsValue {
    let arr = JsArray::from_iter(items, ctx);
    let _ = mark_host(&arr, ctx);
    arr.into()
}

/// Objekt aus Schlüssel-Wert-Paaren.
pub fn object(pairs: Vec<(&str, JsValue)>, ctx: &mut Context) -> JsResult<JsValue> {
    let o = boa_engine::JsObject::with_object_proto(ctx.intrinsics());
    for (k, v) in pairs {
        o.create_data_property_or_throw(JsString::from(k), v, ctx)?;
    }
    mark_host(&o, ctx)?;
    Ok(o.into())
}

/// Rechteck `{left, top, right, bottom}`.
pub fn rect_js(r: [f64; 4], ctx: &mut Context) -> JsResult<JsValue> {
    object(vec![("left", n(r[0])), ("top", n(r[1])), ("right", n(r[2])), ("bottom", n(r[3]))], ctx)
}

/// Punkt `{x, y}`.
pub fn point_js(x: f64, y: f64, ctx: &mut Context) -> JsResult<JsValue> {
    object(vec![("x", n(x)), ("y", n(y))], ctx)
}

// ------------------------------------------------------------------ Argumente

/// Ein umgewandeltes Argument.
#[derive(Clone, Debug)]
pub enum Arg {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    /// Strukturierter Wert (Punkt, Rechteck, Wörterbuch, Liste, Objekt); bei Wrappern `{"__h": i}`.
    Json(Value),
}

impl Arg {
    pub fn is_null(&self) -> bool {
        matches!(self, Arg::Null)
    }
    pub fn bool(&self) -> bool {
        match self {
            Arg::Bool(b) => *b,
            Arg::Num(x) => *x != 0.0 && !x.is_nan(),
            Arg::Str(t) => t == "true",
            Arg::Json(_) => true,
            Arg::Null => false,
        }
    }
    pub fn f64(&self) -> f64 {
        match self {
            Arg::Num(x) => *x,
            Arg::Bool(b) => f64::from(u8::from(*b)),
            Arg::Str(t) => t.trim().parse().unwrap_or(0.0),
            _ => 0.0,
        }
    }
    pub fn i64(&self) -> i64 {
        let x = self.f64();
        if x.is_finite() { x.trunc() as i64 } else { 0 }
    }
    pub fn str(&self) -> Option<String> {
        match self {
            Arg::Null => None,
            Arg::Str(t) => Some(t.clone()),
            Arg::Num(x) => Some(fmt_num(*x)),
            Arg::Bool(b) => Some(b.to_string()),
            Arg::Json(v) => Some(v.to_string()),
        }
    }
    pub fn json(&self) -> Value {
        match self {
            Arg::Null => Value::Null,
            Arg::Bool(b) => Value::Bool(*b),
            Arg::Num(x) => Number::from_f64(*x).map(Value::Number).unwrap_or(Value::Null),
            Arg::Str(t) => Value::String(t.clone()),
            Arg::Json(v) => v.clone(),
        }
    }
    /// Handle eines übergebenen Wrappers.
    pub fn handle(&self) -> Option<usize> {
        match self {
            Arg::Json(v) => v.get("__h").and_then(|h| h.as_u64()).map(|h| h as usize),
            _ => None,
        }
    }
    /// Punkt `{x, y}`.
    pub fn point(&self) -> Option<(f64, f64)> {
        point_of(&self.json())
    }
    /// Rechteck `{left, top, right, bottom}`.
    pub fn rect(&self) -> Option<[f64; 4]> {
        rect_of(&self.json())
    }
}

/// Zahl wie JS `String(x)`.
pub fn fmt_num(x: f64) -> String {
    if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "Infinity".into() } else { "-Infinity".into() }
    } else if x.fract() == 0.0 && x.abs() < 1e21 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

fn num_of(v: &Value) -> Option<f64> {
    match v {
        Value::Number(x) => x.as_f64(),
        Value::String(t) => t.trim().parse().ok(),
        Value::Bool(b) => Some(f64::from(u8::from(*b))),
        _ => None,
    }
}

/// Punkt aus `{x, y}` oder `[x, y]`.
pub fn point_of(v: &Value) -> Option<(f64, f64)> {
    match v {
        Value::Object(m) => Some((num_of(m.get("x")?)?, num_of(m.get("y")?)?)),
        Value::Array(a) if a.len() >= 2 => Some((num_of(&a[0])?, num_of(&a[1])?)),
        _ => None,
    }
}

/// Rechteck aus `{left, top, right, bottom}`.
pub fn rect_of(v: &Value) -> Option<[f64; 4]> {
    match v {
        Value::Object(m) => Some([num_of(m.get("left")?)?, num_of(m.get("top")?)?, num_of(m.get("right")?)?, num_of(m.get("bottom")?)?]),
        Value::Array(a) if a.len() >= 4 => Some([num_of(&a[0])?, num_of(&a[1])?, num_of(&a[2])?, num_of(&a[3])?]),
        _ => None,
    }
}

/// Wandelt ein JS-Argument nach Typcode (03 §10.2, §10.11).
pub fn convert_arg(v: &JsValue, code: TypeCode, ctx: &mut Context) -> Result<Arg, FwErr> {
    let bad = || FwErr::Hr(hr::E_BAD_PARAM_TYPE, format!("falscher Parametertyp ({code:?})"));
    let js = |e: boa_engine::JsError| FwErr::Js(e.to_string());
    if v.is_null_or_undefined() {
        return if code.null_ok() { Ok(Arg::Null) } else { Err(bad()) };
    }
    match code {
        TypeCode::Bool => {
            if let Some(b) = v.as_boolean() {
                Ok(Arg::Bool(b))
            } else if let Some(x) = v.as_number() {
                Ok(Arg::Bool(x != 0.0 && !x.is_nan()))
            } else if let Some(t) = v.as_string() {
                Ok(Arg::Bool(t.to_std_string_escaped() == "true"))
            } else {
                Ok(Arg::Bool(true))
            }
        }
        TypeCode::Int | TypeCode::Float => {
            if let Some(x) = v.as_number() {
                Ok(Arg::Num(if code == TypeCode::Int && x.is_finite() { x.trunc() } else { x }))
            } else if let Some(b) = v.as_boolean() {
                Ok(Arg::Num(f64::from(u8::from(b))))
            } else if let Some(t) = v.as_string() {
                // lockere Umwandlung wie das Original (10.11 T5; 08 T3: "hard edge" für int wird angenommen):
                // führende Zahl, sonst 0
                let x = leading_number(&t.to_std_string_escaped());
                Ok(Arg::Num(if code == TypeCode::Int { x.trunc() } else { x }))
            } else {
                Err(bad())
            }
        }
        TypeCode::Str | TypeCode::Url | TypeCode::Any | TypeCode::Color => {
            if let Some(t) = v.as_string() {
                Ok(Arg::Str(t.to_std_string_escaped()))
            } else if let Some(x) = v.as_number() {
                Ok(Arg::Str(fmt_num(x)))
            } else if let Some(b) = v.as_boolean() {
                Ok(Arg::Str(b.to_string()))
            } else if code == TypeCode::Any {
                Ok(Arg::Json(js_to_json(v, ctx, 0).map_err(js)?))
            } else {
                let t = v.to_string(ctx).map_err(js)?;
                Ok(Arg::Str(t.to_std_string_escaped()))
            }
        }
        TypeCode::Point => {
            let j = js_to_json(v, ctx, 0).map_err(js)?;
            point_of(&j).map(|_| Arg::Json(j)).ok_or_else(bad)
        }
        TypeCode::Rect => {
            let j = js_to_json(v, ctx, 0).map_err(js)?;
            rect_of(&j).map(|_| Arg::Json(j)).ok_or_else(bad)
        }
        TypeCode::Array => {
            let j = js_to_json(v, ctx, 0).map_err(js)?;
            if j.is_array() { Ok(Arg::Json(j)) } else { Err(bad()) }
        }
        _ => {
            if let Some(t) = v.as_string() {
                return Ok(Arg::Str(t.to_std_string_escaped()));
            }
            if let Some(x) = v.as_number() {
                return Ok(Arg::Num(x));
            }
            if let Some(b) = v.as_boolean() {
                return Ok(Arg::Bool(b));
            }
            Ok(Arg::Json(js_to_json(v, ctx, 0).map_err(js)?))
        }
    }
}

/// Führende Dezimalzahl eines Texts (wie `atof`), sonst 0.
pub fn leading_number(t: &str) -> f64 {
    let s = t.trim_start();
    let b = s.as_bytes();
    let mut end = 0;
    if end < b.len() && (b[end] == b'-' || b[end] == b'+') {
        end += 1;
    }
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    while end < b.len() {
        let c = b[end];
        if c.is_ascii_digit() {
            seen_digit = true;
        } else if c == b'.' && !seen_dot && !seen_exp {
            seen_dot = true;
        } else if (c == b'e' || c == b'E') && seen_digit && !seen_exp && end + 1 < b.len() && (b[end + 1].is_ascii_digit() || ((b[end + 1] == b'-' || b[end + 1] == b'+') && end + 2 < b.len() && b[end + 2].is_ascii_digit())) {
            seen_exp = true;
            end += 1;
        } else {
            break;
        }
        end += 1;
    }
    if !seen_digit {
        return 0.0;
    }
    s[..end].parse().unwrap_or(0.0)
}

/// Farbe normalisieren: `#rgb`, `#rrggbb`, `#rrggbbaa` → `#rrggbb[aa]` (Kleinbuchstaben).
pub fn norm_color(c: &str) -> Option<String> {
    let t = c.trim().trim_start_matches('#');
    if !t.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return None;
    }
    match t.len() {
        3 => Some(format!("#{}", t.chars().flat_map(|ch| [ch, ch]).collect::<String>().to_lowercase())),
        6 | 8 => Some(format!("#{}", t.to_lowercase())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_colors() {
        assert_eq!(fmt_num(3.0), "3");
        assert_eq!(fmt_num(1.5), "1.5");
        assert_eq!(norm_color("#FFF"), Some("#ffffff".into()));
        assert_eq!(norm_color("#000000a2"), Some("#000000a2".into()));
        assert_eq!(norm_color("xyz"), None);
        assert_eq!(leading_number("-1"), -1.0);
        assert_eq!(leading_number(" 12px"), 12.0);
        assert_eq!(leading_number("1.5e2x"), 150.0);
        assert_eq!(leading_number("hard edge"), 0.0);
        assert_eq!(rect_of(&serde_json::json!({"left": 1, "top": "2", "right": 3, "bottom": 4})), Some([1.0, 2.0, 3.0, 4.0]));
    }
}
