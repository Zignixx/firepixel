//! `script.condition` (B-4, `docs/BATCH.md` §3.12): a small expression over the item's metadata
//! that either skips the item or gates the following cards.
//!
//! Expression language (case-insensitive keywords):
//! - values: numbers (`1200`, `1.5`, with `kb`/`mb`/`gb` suffix → bytes), strings (`"…"` or `'…'`),
//!   `true`/`false`;
//! - variables: `width`, `height`, `ratio` (width / height), `megapixels`, `frames`, `format`
//!   (technical source format, `png` …), `ext` (file extension), `name` (file name with extension),
//!   `size` (source bytes), `dpi`, `hasAlpha`, `isAnimation`, `isDocument`;
//! - operators: `||`/`or`, `&&`/`and`, `!`/`not`, `==` `!=` `<` `<=` `>` `>=`, `~`/`matches`
//!   (glob with `*` and `?`, case-insensitive), `+ - * /`, parentheses.
//!
//! Strings compare case-insensitively. Unknown variables are a validation error.

use crate::error::{BatchError, Result};
use crate::item::{Item, ItemInfo, ItemKind};
use crate::step::{Issue, ParamDef, Step, StepContext, StepGroup, StepRegistry, validate_params};
use serde_json::{Value, json};
use std::sync::Arc;

/// ID "If … then".
pub const CONDITION: &str = "script.condition";
/// `Meta::extra`: number of following per-item steps to skip (gate mode).
pub const EXTRA_SKIP_NEXT: &str = "script.skipNext";

/// Variables an expression may use.
pub const VARIABLES: [&str; 13] = [
    "width",
    "height",
    "ratio",
    "megapixels",
    "frames",
    "format",
    "ext",
    "name",
    "size",
    "dpi",
    "hasalpha",
    "isanimation",
    "isdocument",
];

/// Registers the step.
pub fn register(r: &mut StepRegistry) {
    r.register(Arc::new(Condition));
}

/// A value of the expression language.
#[derive(Clone, Debug, PartialEq)]
pub enum Val {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::Num(n) => *n != 0.0,
            Val::Str(s) => !s.is_empty(),
            Val::Bool(b) => *b,
        }
    }
    fn num(&self) -> Option<f64> {
        match self {
            Val::Num(n) => Some(*n),
            Val::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Val::Str(s) => s.trim().parse().ok(),
        }
    }
    fn text(&self) -> String {
        match self {
            Val::Num(n) => format!("{n}"),
            Val::Str(s) => s.clone(),
            Val::Bool(b) => b.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
}

fn err(msg: impl Into<String>) -> String {
    fp_i18n::tr!("batch-issue-condition-syntax", detail = msg.into())
}

fn lex(src: &str) -> std::result::Result<Vec<Tok>, String> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
            continue;
        }
        if ch.is_ascii_digit() || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let s = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            let txt: String = c[s..i].iter().collect();
            let mut n: f64 = txt.parse().map_err(|_| err(txt.clone()))?;
            let us = i;
            while i < c.len() && c[i].is_ascii_alphabetic() {
                i += 1;
            }
            let unit: String = c[us..i].iter().collect::<String>().to_lowercase();
            n *= match unit.as_str() {
                "" | "px" | "b" => 1.0,
                "kb" | "k" => 1024.0,
                "mb" | "m" => 1024.0 * 1024.0,
                "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
                other => return Err(err(format!("{txt}{other}"))),
            };
            out.push(Tok::Num(n));
            continue;
        }
        if ch == '"' || ch == '\'' {
            let q = ch;
            i += 1;
            let s = i;
            while i < c.len() && c[i] != q {
                i += 1;
            }
            if i >= c.len() {
                return Err(err(fp_i18n::t("batch-issue-condition-quote")));
            }
            out.push(Tok::Str(c[s..i].iter().collect()));
            i += 1;
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let s = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            let w: String = c[s..i].iter().collect::<String>().to_lowercase();
            out.push(match w.as_str() {
                "and" => Tok::Op("&&"),
                "or" => Tok::Op("||"),
                "not" => Tok::Op("!"),
                "matches" => Tok::Op("~"),
                _ => Tok::Ident(w),
            });
            continue;
        }
        let two: String = c[i..(i + 2).min(c.len())].iter().collect();
        let op2 = ["&&", "||", "==", "!=", "<=", ">="]
            .into_iter()
            .find(|o| *o == two);
        if let Some(o) = op2 {
            out.push(Tok::Op(o));
            i += 2;
            continue;
        }
        let op1 = match ch {
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '<' => Some(Tok::Op("<")),
            '>' => Some(Tok::Op(">")),
            '!' => Some(Tok::Op("!")),
            '~' => Some(Tok::Op("~")),
            '=' => Some(Tok::Op("==")),
            '+' => Some(Tok::Op("+")),
            '-' => Some(Tok::Op("-")),
            '*' => Some(Tok::Op("*")),
            '/' => Some(Tok::Op("/")),
            _ => None,
        };
        match op1 {
            Some(t) => {
                out.push(t);
                i += 1;
            }
            None => return Err(err(ch.to_string())),
        }
    }
    Ok(out)
}

/// Parsed expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Lit(Val),
    Var(String),
    Not(Box<Expr>),
    Neg(Box<Expr>),
    Bin(&'static str, Box<Expr>, Box<Expr>),
}

struct Parser {
    t: Vec<Tok>,
    i: usize,
}

impl Parser {
    fn peek_op(&self, ops: &[&str]) -> Option<&'static str> {
        match self.t.get(self.i) {
            Some(Tok::Op(o)) if ops.contains(o) => Some(o),
            _ => None,
        }
    }
    fn bin(
        &mut self,
        ops: &[&str],
        next: fn(&mut Parser) -> std::result::Result<Expr, String>,
    ) -> std::result::Result<Expr, String> {
        let mut l = next(self)?;
        while let Some(o) = self.peek_op(ops) {
            self.i += 1;
            let r = next(self)?;
            l = Expr::Bin(o, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn or(&mut self) -> std::result::Result<Expr, String> {
        self.bin(&["||"], Parser::and)
    }
    fn and(&mut self) -> std::result::Result<Expr, String> {
        self.bin(&["&&"], Parser::not)
    }
    fn not(&mut self) -> std::result::Result<Expr, String> {
        if self.peek_op(&["!"]).is_some() {
            self.i += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> std::result::Result<Expr, String> {
        let l = self.sum()?;
        if let Some(o) = self.peek_op(&["==", "!=", "<", "<=", ">", ">=", "~"]) {
            self.i += 1;
            let r = self.sum()?;
            return Ok(Expr::Bin(o, Box::new(l), Box::new(r)));
        }
        Ok(l)
    }
    fn sum(&mut self) -> std::result::Result<Expr, String> {
        self.bin(&["+", "-"], Parser::mul)
    }
    fn mul(&mut self) -> std::result::Result<Expr, String> {
        self.bin(&["*", "/"], Parser::unary)
    }
    fn unary(&mut self) -> std::result::Result<Expr, String> {
        if self.peek_op(&["-"]).is_some() {
            self.i += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }
    fn atom(&mut self) -> std::result::Result<Expr, String> {
        let t = self
            .t
            .get(self.i)
            .cloned()
            .ok_or_else(|| err(fp_i18n::t("batch-issue-condition-end")))?;
        self.i += 1;
        match t {
            Tok::Num(n) => Ok(Expr::Lit(Val::Num(n))),
            Tok::Str(s) => Ok(Expr::Lit(Val::Str(s))),
            Tok::Ident(w) if w == "true" => Ok(Expr::Lit(Val::Bool(true))),
            Tok::Ident(w) if w == "false" => Ok(Expr::Lit(Val::Bool(false))),
            Tok::Ident(w) if VARIABLES.contains(&w.as_str()) => Ok(Expr::Var(w)),
            Tok::Ident(w) => Err(fp_i18n::tr!(
                "batch-issue-condition-variable",
                name = w,
                list = "width, height, ratio, megapixels, frames, format, ext, name, size, dpi, hasAlpha, isAnimation, isDocument"
            )),
            Tok::LParen => {
                let e = self.or()?;
                match self.t.get(self.i) {
                    Some(Tok::RParen) => {
                        self.i += 1;
                        Ok(e)
                    }
                    _ => Err(err(")")),
                }
            }
            Tok::RParen => Err(err(")")),
            Tok::Op(o) => Err(err(o)),
        }
    }
}

/// Parses an expression (message in the current language on error).
pub fn parse(src: &str) -> std::result::Result<Expr, String> {
    if src.trim().is_empty() {
        return Err(fp_i18n::t("batch-issue-condition-empty"));
    }
    let mut p = Parser { t: lex(src)?, i: 0 };
    let e = p.or()?;
    if p.i < p.t.len() {
        return Err(err(format!("{:?}", p.t[p.i])
            .trim_start_matches("Op(")
            .trim_start_matches("Ident(")
            .trim_end_matches(')')
            .trim_matches('"')
            .to_string()));
    }
    Ok(e)
}

/// Glob with `*` and `?` (case-insensitive).
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// Variables of an item (current size and transparency, source name/format/bytes).
pub fn variables(item: &Item, info: &ItemInfo) -> Vec<(&'static str, Val)> {
    let (w, h) = item.dimensions();
    let name = info.file_name();
    let ext = crate::naming::split_extension(&name).1.to_lowercase();
    let frames = match item {
        Item::Animation(a) => a.frames.len(),
        _ => info.frames.max(1) as usize,
    };
    let meta = item.meta();
    vec![
        ("width", Val::Num(w as f64)),
        ("height", Val::Num(h as f64)),
        ("ratio", Val::Num(w as f64 / h.max(1) as f64)),
        ("megapixels", Val::Num(w as f64 * h as f64 / 1_000_000.0)),
        ("frames", Val::Num(frames as f64)),
        ("format", Val::Str(info.format.to_lowercase())),
        ("ext", Val::Str(ext)),
        ("name", Val::Str(name)),
        ("size", Val::Num(info.bytes as f64)),
        (
            "dpi",
            Val::Num(meta.and_then(|m| m.dpi).unwrap_or(72.0) as f64),
        ),
        (
            "hasalpha",
            Val::Bool(meta.is_some_and(|m| m.has_transparency)),
        ),
        ("isanimation", Val::Bool(matches!(item, Item::Animation(_)))),
        ("isdocument", Val::Bool(matches!(item, Item::Document(_)))),
    ]
}

/// Evaluates `e` with the variables `vars`.
pub fn eval(e: &Expr, vars: &[(&'static str, Val)]) -> Val {
    match e {
        Expr::Lit(v) => v.clone(),
        Expr::Var(n) => vars
            .iter()
            .find(|(k, _)| k == n)
            .map(|(_, v)| v.clone())
            .unwrap_or(Val::Bool(false)),
        Expr::Not(x) => Val::Bool(!eval(x, vars).truthy()),
        Expr::Neg(x) => Val::Num(-eval(x, vars).num().unwrap_or(0.0)),
        Expr::Bin(op, l, r) => {
            if *op == "&&" {
                return Val::Bool(eval(l, vars).truthy() && eval(r, vars).truthy());
            }
            if *op == "||" {
                return Val::Bool(eval(l, vars).truthy() || eval(r, vars).truthy());
            }
            let (a, b) = (eval(l, vars), eval(r, vars));
            let nums = match (&a, &b) {
                (Val::Str(_), Val::Str(_)) => None,
                _ => a.num().zip(b.num()),
            };
            match *op {
                "~" => Val::Bool(glob_match(&b.text(), &a.text())),
                "==" | "!=" => {
                    let eq = match nums {
                        Some((x, y)) => (x - y).abs() < 1e-9,
                        None => a.text().to_lowercase() == b.text().to_lowercase(),
                    };
                    Val::Bool(eq == (*op == "=="))
                }
                "<" | "<=" | ">" | ">=" => {
                    let ord = match nums {
                        Some((x, y)) => x.partial_cmp(&y),
                        None => Some(a.text().to_lowercase().cmp(&b.text().to_lowercase())),
                    };
                    let Some(o) = ord else {
                        return Val::Bool(false);
                    };
                    use std::cmp::Ordering::*;
                    Val::Bool(match *op {
                        "<" => o == Less,
                        "<=" => o != Greater,
                        ">" => o == Greater,
                        _ => o != Less,
                    })
                }
                "+" if nums.is_none() => Val::Str(a.text() + &b.text()),
                _ => {
                    let (x, y) = nums.unwrap_or((0.0, 0.0));
                    Val::Num(match *op {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        _ if y == 0.0 => 0.0,
                        _ => x / y,
                    })
                }
            }
        }
    }
}

/// Does the item meet the condition?
pub fn matches(src: &str, item: &Item, info: &ItemInfo) -> std::result::Result<bool, String> {
    let e = parse(src)?;
    Ok(eval(&e, &variables(item, info)).truthy())
}

/// Gate mode: called by the pipeline before every per-item step. `true` = skip this step (the
/// counter in `Meta::extra` goes down by one).
pub fn take_skip(item: &mut Item) -> bool {
    let Some(m) = item.meta_mut() else {
        return false;
    };
    let n = m
        .extra
        .get(EXTRA_SKIP_NEXT)
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if n == 0 {
        return false;
    }
    if n == 1 {
        m.extra.remove(EXTRA_SKIP_NEXT);
    } else {
        m.extra.insert(EXTRA_SKIP_NEXT.into(), json!(n - 1));
    }
    true
}

fn p_str<'a>(p: &'a Value, k: &str, d: &'a str) -> &'a str {
    p.get(k).and_then(Value::as_str).unwrap_or(d)
}

struct Condition;

impl Step for Condition {
    fn id(&self) -> &str {
        CONDITION
    }
    fn group(&self) -> StepGroup {
        StepGroup::Advanced
    }
    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::text("condition", "width > 1000").beginner(),
            ParamDef::choice("mode", &["gate", "skipItem"], "gate").beginner(),
            ParamDef::int("steps", 1, 50, 1).when("mode", &["gate"]),
            ParamDef::bool("invert", false),
            ParamDef::bool("copySkipped", false).when("mode", &["skipItem"]),
        ]
    }
    fn validate(&self, params: &Value) -> Vec<Issue> {
        let mut out = validate_params(&self.params(), params);
        if let Err(m) = parse(p_str(params, "condition", "width > 1000")) {
            out.push(Issue::param("condition", m));
        }
        out
    }
    fn applies_to(&self) -> &[ItemKind] {
        &ItemKind::ALL
    }
    fn apply(&self, item: &mut Item, params: &Value, ctx: &mut StepContext) -> Result<()> {
        let src = p_str(params, "condition", "width > 1000");
        let hit = matches(src, item, ctx.info).map_err(BatchError::Invalid)?;
        let hit = hit
            != params
                .get("invert")
                .and_then(Value::as_bool)
                .unwrap_or(false);
        if hit {
            return Ok(());
        }
        if p_str(params, "mode", "gate") == "skipItem" {
            let reason = fp_i18n::tr!("batch-skip-condition", condition = src.to_string());
            if ctx.preview {
                ctx.warn(fp_i18n::tr!("batch-warn-filter-preview", reason = reason));
                return Ok(());
            }
            let copy = params
                .get("copySkipped")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            return Err(BatchError::filtered(reason, copy));
        }
        let n = params
            .get("steps")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .clamp(1, 50);
        if let Some(m) = item.meta_mut() {
            m.extra.insert(EXTRA_SKIP_NEXT.into(), json!(n));
        }
        Ok(())
    }
    fn summary(&self, params: &Value) -> String {
        p_str(params, "condition", "width > 1000").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vec<(&'static str, Val)> {
        vec![
            ("width", Val::Num(1200.0)),
            ("height", Val::Num(800.0)),
            ("format", Val::Str("png".into())),
            ("name", Val::Str("Photo_01.PNG".into())),
            ("size", Val::Num(600.0 * 1024.0)),
            ("hasalpha", Val::Bool(true)),
        ]
    }
    fn t(src: &str) -> bool {
        eval(&parse(src).expect(src), &vars()).truthy()
    }

    #[test]
    fn expressions() {
        assert!(t("width > 1000 && height <= 800"));
        assert!(t("width > 2000 or hasAlpha"));
        assert!(!t("not hasAlpha"));
        assert!(t("format == 'PNG'"));
        assert!(t("name ~ \"photo_*.png\""));
        assert!(!t("name matches '*.jpg'"));
        assert!(t("size > 500kb && size < 1mb"));
        assert!(t("(width - height) * 2 == 800"));
        assert!(t("width / height > 1.4"));
        assert!(parse("width >").is_err());
        assert!(parse("colour > 3").is_err());
        assert!(parse("name ~ 'x").is_err());
        assert!(parse("").is_err());
        assert!(parse("width > 3 )").is_err());
    }

    #[test]
    fn globs() {
        assert!(glob_match("*", ""));
        assert!(glob_match("a?c*", "abcdef"));
        assert!(!glob_match("a?c", "abcd"));
    }
}
