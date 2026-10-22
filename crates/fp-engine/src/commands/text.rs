//! Menü „Text“: Schrift, Größe, Stil, Ausrichtung (`JSTi`, 09 §3.5), Text an/in Pfad, Richtung,
//! sowie Textinhalt und -attribute für Texteditor und Eigenschaften-Inspektor.

use super::{DOC, any_selected, sel_err, selected_elements, target_ids, with_doc};
use crate::command::{
    CommandOutput, CommandRegistry, ObjectKind, ParamKind, Params, Requirement, cmd,
};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::session::Session;
use fp_model::Extra;
use fp_model::element::{Element, ElementKind};
use fp_model::text::{TextAttrs, TextElem, TextOnPath, TextRun};
use std::sync::Arc;

const TEXT_SEL: &[Requirement] = &[Requirement::Document, Requirement::AnyOf(ObjectKind::Text)];

/// Menügrößen (Text ▸ Größe).
pub const SIZES: &[f32] = &[
    8.0, 9.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0, 48.0, 72.0, 96.0, 120.0,
];

/// Ändert alle ausgewählten Texte (Anfangsattribute und alle Läufe); löst die ATE-Story (09 §4.1).
fn edit_texts(s: &mut Session, f: impl Fn(&mut TextElem)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let ids: Vec<_> = selected_elements(d)
            .into_iter()
            .filter(|(_, e)| matches!(e.kind, ElementKind::Text(_)))
            .map(|(_, e)| e.id)
            .collect();
        if ids.is_empty() {
            return Err(sel_err("err-sel-no-text-selected"));
        }
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| {
                if let ElementKind::Text(t) = &mut e.kind {
                    f(t);
                    t.detach_story();
                    t.relaid = true;
                }
            });
        }
        Ok(CommandOutput::doc())
    })
}

/// Text ▸ Ausrichten setzt eine gemischte Auswahl auf die Textobjekte zurück, per Maus wie per
/// Tastatur (TW-57, re 29 §5: Text + Rechteck → nur der Text bleibt gewählt).
fn reduce_selection_to_texts(s: &mut Session) {
    let Some(d) = s.active_mut() else { return };
    let texts: Vec<_> = selected_elements(d)
        .into_iter()
        .filter(|(_, e)| matches!(e.kind, ElementKind::Text(_)))
        .map(|(_, e)| e.id)
        .collect();
    let keep: Vec<_> = d
        .selection
        .elements
        .iter()
        .copied()
        .filter(|id| texts.contains(id))
        .collect();
    if keep.is_empty() || keep.len() == d.selection.elements.len() {
        return;
    }
    d.selection.elements = keep;
    d.selection.points.clear();
    if let Some(id) = s.active_id() {
        s.emit(crate::session::SessionEvent::SelectionChanged(id));
    }
}

/// Setzt ein Attribut in den Anfangsattributen und allen Läufen.
fn set_attr(t: &mut TextElem, f: &dyn Fn(&mut TextAttrs)) {
    f(&mut t.initial);
    for r in &mut t.runs {
        f(&mut r.attrs);
    }
}

/// Ist ein Text ausgewählt?
fn has_text(s: &Session) -> bool {
    s.active()
        .is_some_and(|d| any_selected(d, |e| matches!(e.kind, ElementKind::Text(_))))
}

/// Do all selected texts carry the style on every run? `false` without selected text.
fn all_texts_have(s: &Session, get: fn(&TextAttrs) -> Option<bool>) -> bool {
    s.active().is_some_and(|d| {
        let texts: Vec<Vec<bool>> = selected_elements(d)
            .into_iter()
            .filter_map(|(_, e)| match &e.kind {
                ElementKind::Text(t) => Some(
                    t.resolved_runs()
                        .iter()
                        .map(|(a, _)| get(a).unwrap_or(false))
                        .collect(),
                ),
                _ => None,
            })
            .collect();
        !texts.is_empty()
            && texts
                .iter()
                .all(|runs| !runs.is_empty() && runs.iter().all(|&b| b))
    })
}

/// Ist ein Text auf Pfad ausgewählt?
fn has_text_on_path(s: &Session) -> bool {
    s.active().is_some_and(|d| {
        any_selected(
            d,
            |e| matches!(&e.kind, ElementKind::Text(t) if t.on_path.is_some()),
        )
    })
}

/// Text attribute command without open editor: with selected text change those texts; the
/// defaults of the text tool (`Text_*`) are written in every case (TW-138, re 50 section 2.2).
fn texts_or_defaults(
    s: &mut Session,
    on_text: impl Fn(&mut TextElem),
    on_defaults: impl FnOnce(&mut Session),
) -> Result<CommandOutput> {
    let out = if has_text(s) {
        edit_texts(s, on_text)?
    } else {
        CommandOutput::session()
    };
    on_defaults(s);
    s.prefs_changed();
    Ok(out)
}

/// Familienname zu einem Schriftnamen (PostScript- oder Familienname); unbekannt → unverändert.
fn font_family_of(name: &str) -> String {
    let db = fp_text::FontDb::shared();
    match db.find_ps(name).and_then(|i| db.faces().get(i)) {
        Some(f) if !f.names.family.is_empty() => f.names.family.clone(),
        _ => name.to_string(),
    }
}

/// PostScript name for a font name (PostScript or family name; a family gives its regular face);
/// unknown names stay as they are.
fn ps_name_of(name: &str) -> String {
    let db = fp_text::FontDb::shared();
    let id = db
        .find_ps(name)
        .or_else(|| db.find_family(name, false, false).map(|(i, _)| i));
    match id.and_then(|i| db.faces().get(i)) {
        Some(f) if !f.names.ps_name.is_empty() => f.names.ps_name.clone(),
        _ => name.to_string(),
    }
}

/// A font change becomes the default of new text (TW-138, re 53 section 2.2): `Text_CoolType_Font`
/// gets the PostScript name, `Text_Font` keeps the family (read by new text). Both are stored
/// explicitly, also for the factory font (`prefs.set` would drop the key, MN-2/NZ-3).
pub(crate) fn store_font_default(s: &mut Session, font: &str) {
    use crate::prefs::PrefValue;
    let family = font_family_of(font);
    s.prefs
        .values
        .insert("Text_Font".to_string(), PrefValue::Str(family.clone()));
    s.prefs.values.insert(
        "Text_CoolType_Font".to_string(),
        PrefValue::Str(ps_name_of(font)),
    );
    s.tools.options.text_font = family;
}

/// Puts the family of `font` at the front of the recently used fonts (`RecentFontList`, form
/// "(A)(B)…", at most `RecentFontLimit` entries) – on every font change from the Text menu, the
/// Properties panel or the editor (TW-138).
pub(crate) fn note_recent_font(s: &mut Session, font: &str) {
    if font.trim().is_empty() {
        return;
    }
    let family = font_family_of(font);
    let limit = s.prefs.get_int("RecentFontLimit", 5).clamp(1, 50) as usize;
    let mut list: Vec<String> = s
        .prefs
        .get_str("RecentFontList", "")
        .split(')')
        .filter_map(|p| p.trim().strip_prefix('('))
        .filter(|n| !n.is_empty() && *n != family)
        .map(str::to_string)
        .collect();
    list.insert(0, family);
    list.truncate(limit);
    let v: String = list.iter().map(|n| format!("({n})")).collect();
    s.prefs
        .set("RecentFontList", crate::prefs::PrefValue::Str(v));
}

fn set_default_size(s: &mut Session, v: f32) {
    s.prefs
        .set("Text_Size", crate::prefs::PrefValue::Real(v as f64));
    s.tools.options.text_size = v;
}

/// Größe des ausgewählten Textes bzw. Vorgabe des Textwerkzeugs (Häkchen im Menü).
fn current_size(s: &Session) -> Option<f32> {
    let d = s.active()?;
    let sel = selected_elements(d).into_iter().find_map(|(_, e)| {
        if let ElementKind::Text(t) = &e.kind {
            Some(first_size(t))
        } else {
            None
        }
    });
    Some(sel.unwrap_or(s.tools.options.text_size))
}

fn current_style(s: &Session, key: &str) -> bool {
    s.prefs.get_bool(key, false)
}

/// Text size after Text ▸ Size ▸ Larger/Smaller: `cur + delta` pt, at least 1 pt.
pub(crate) fn step_size(cur: f32, delta: f32) -> f32 {
    (cur + delta).max(1.0)
}

/// Alignment the text engine actually stores (TW-133, re 50 section 1.4): "stretched" (4, or
/// 0x14 vertical) cannot be represented and becomes left (0 / 0x10) on every text, point or
/// fixed-width; 0-3 (and their vertical variants) are kept.
pub fn effective_alignment(code: i32) -> i32 {
    if code & 0x0f >= 4 { code & 0x10 } else { code }
}

/// Attributes new text takes from the `Text_*` defaults (text tool, `draw.text`, `addNewText`),
/// only where they differ from the factory values (re 50 section 2.3). The alignment is the one
/// the engine can store ("stretched" becomes left).
pub fn new_text_pref_attrs(prefs: &crate::prefs::Preferences) -> TextAttrs {
    let mut a = TextAttrs::default();
    let just = effective_alignment(prefs.get_int("Text_Justification", 0).clamp(0, 4) as i32);
    if just != 0 {
        a.alignment = Some(just);
    }
    let on = |k: &str| prefs.get_bool(k, false).then_some(true);
    a.bold = on("Text_Bold");
    a.italic = on("Text_Italic");
    a.underline = on("Text_Underline");
    let real = |k: &str, d: f64| {
        let v = prefs.get_real(k, d);
        ((v - d).abs() > 1e-6).then_some(v as f32)
    };
    // R6 (re 56 section 3): new text never takes `Text_RangeKerning` as its tracking (range
    // kerning 0). The original puts it on the first typed character as pair kerning; the
    // Firepixel text layout does not apply per-character kerning, so it is not applied at all.
    if prefs.get_int("Text_LeadingMode", 0) == 1 {
        a.leading_mode = Some(1);
    }
    a.indent = real("Text_ParagraphIndent", 0.0);
    a.space_before = real("Text_ParagraphSpacingBefore", 0.0);
    a.space_after = real("Text_ParagraphSpacingAfter", 0.0);
    a.horizontal_scale = real("Text_HorizScale", 1.0);
    a.baseline_shift = real("Text_BaselineShift", 0.0);
    a
}

/// Text ▸ Align: stores the defaults like the original (re 50 section 1.2): `Text_Justification`
/// gets the horizontal code as given (also 4 = stretched), `Text_Orientation` horizontal/vertical.
fn store_alignment_prefs(s: &mut Session, code: i32) {
    use crate::prefs::PrefValue;
    s.prefs
        .set("Text_Justification", PrefValue::Int((code & !0x10) as i64));
    let vertical = code & 0x10 != 0;
    let ori = s.prefs.get_int("Text_Orientation", 0);
    // keep the direction (left/right to left) and switch only between horizontal and vertical
    let now_vertical = ori == 1 || ori == 3;
    if vertical != now_vertical {
        let v = match (vertical, ori) {
            (true, 2) => 3,
            (true, _) => 1,
            (false, 3) => 2,
            (false, _) => 0,
        };
        s.prefs.set("Text_Orientation", PrefValue::Int(v));
    }
}

fn first_size(t: &TextElem) -> f32 {
    t.resolved_runs()
        .first()
        .and_then(|(a, _)| a.size)
        .or(t.initial.size)
        .unwrap_or(12.0)
}

pub(crate) fn attrs_from_params(p: &Params) -> Result<TextAttrs> {
    Ok(TextAttrs {
        font: p.opt_str("font")?,
        ps_font: None,
        size: p.opt_f64("size")?.map(|v| v as f32),
        bold: p.opt_bool("bold")?,
        italic: p.opt_bool("italic")?,
        underline: p.opt_bool("underline")?,
        horizontal_scale: p.opt_f64("horizontalScale")?.map(|v| v as f32),
        kerning: None,
        tracking: p.opt_f64("tracking")?.map(|v| v as f32),
        baseline_shift: p.opt_f64("baselineShift")?.map(|v| v as f32),
        leading: p.opt_f64("leading")?.map(|v| v as f32),
        leading_mode: p.opt_i64("leadingMode")?.map(|v| v as i32),
        alignment: p
            .opt_i64("alignment")?
            .map(|v| effective_alignment(v as i32)),
        color: p.opt_color("color")?.map(|c| c.0),
        indent: p.opt_f64("indent")?.map(|v| v as f32),
        space_before: p.opt_f64("spaceBefore")?.map(|v| v as f32),
        space_after: p.opt_f64("spaceAfter")?.map(|v| v as f32),
    })
}

fn text_and_path(s: &Session) -> Result<(Arc<Element>, Arc<Element>)> {
    // (auch Verfügbarkeit von „An/In Pfad anfügen“)
    let d = s.doc()?;
    let els = selected_elements(d);
    let t = els
        .iter()
        .find(|(_, e)| matches!(e.kind, ElementKind::Text(_)))
        .map(|(_, e)| Arc::clone(e));
    let p = els
        .iter()
        .find(|(_, e)| matches!(e.kind, ElementKind::Path(_)))
        .map(|(_, e)| Arc::clone(e));
    match (t, p) {
        (Some(t), Some(p)) if els.len() == 2 => Ok((t, p)),
        _ => Err(sel_err("err-sel-text-and-path")),
    }
}

fn attach(s: &mut Session, inside: bool) -> Result<CommandOutput> {
    let (t, path) = text_and_path(s)?;
    with_doc(s, |d| {
        let page = d.page;
        let doc = Arc::make_mut(&mut d.doc);
        docops::remove_ids(doc, page, &[path.id]);
        docops::edit_by_id(doc, page, t.id, |e| {
            if let ElementKind::Text(tx) = &mut e.kind {
                tx.on_path = Some(TextOnPath {
                    mode: Some(0),
                    offset: Some(0.0),
                    path: Some(Arc::clone(&path)),
                    extra: Extra::default(),
                });
                tx.frame_type = Some(if inside { 3 } else { 2 });
                tx.detach_story();
            }
        })
        .ok_or(EngineError::ElementNotFound(t.id.0))?;
        d.selection.set(vec![t.id]);
        Ok(CommandOutput::doc())
    })
}

/// Text nach „Von Pfad lösen“ wie im Original (Q-16, `docs\berichte\m1.md`): Rahmen mit fester
/// Breite (Breite des Rahmens vor dem Anfügen: Benutzerrahmen, sonst gespeicherter Satzrahmen), Text
/// neu umbrochen, Rahmen auf die Mitte `center` der geometrischen Pfadgrenzen zentriert (ganze Pixel,
/// .5 aufwärts).
fn detached_text(
    doc: &fp_model::Document,
    t: &TextElem,
    center: Option<(f32, f32)>,
    r: Option<&dyn crate::render::PageRenderer>,
) -> TextElem {
    let mut n = t.clone();
    n.on_path = None;
    n.frame_type = Some(1);
    n.detach_story();
    let user_w = t
        .anchor_rect
        .map(|a| a.right - a.left)
        .filter(|w| *w >= 1.0);
    let w = user_w.unwrap_or(t.bounds.right - t.bounds.left).max(1.0);
    let a0 = fp_geom::Rect::new(0.0, 0.0, w, 1.0);
    n.anchor_rect = Some(a0);
    // Satz am Ursprung: Höhe und Lage des Satzrahmens relativ zum Benutzerrahmen
    let f = r
        .and_then(|r| r.text_frame(doc, &n))
        .unwrap_or(fp_geom::Rect::new(
            0.0,
            0.0,
            w,
            t.bounds.bottom - t.bounds.top,
        ));
    let h = (f.bottom - f.top).max(1.0);
    let half_up = |v: f32| (v + 0.5).floor();
    let (cx, cy) = center.unwrap_or((
        (t.bounds.left + t.bounds.right) * 0.5,
        (t.bounds.top + t.bounds.bottom) * 0.5,
    ));
    let (left, top) = (
        half_up(cx - (f.right - f.left) * 0.5),
        half_up(cy - h * 0.5),
    );
    let (dx, dy) = (left - f.left, top - f.top);
    n.anchor_rect = Some(fp_geom::Rect::new(
        a0.left + dx,
        a0.top + dy,
        a0.left + dx + w,
        a0.top + dy + h,
    ));
    n.bounds = f.translate(dx, dy);
    n
}

/// Namen der Glättung (JS `antiAliasMode` bzw. „none“ = keine Kantenglättung).
pub(super) fn antialias_names() -> Vec<String> {
    ["none", "crisp", "strong", "smooth", "custom", "system"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Namen der Textrichtung (`ORIi` 0–3).
pub(super) fn orientation_names() -> Vec<String> {
    [
        "horizontalLtr",
        "verticalRtl",
        "horizontalRtl",
        "verticalLtr",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Glättungsname → `TAAi` (re\09 §3.5: 0 glatt, 1 scharf, 2 stark, 3 System, 4 benutzerdefiniert);
/// „none“ = −1 (harte Füllkante `FETi 0`, kein `TAAi`-Wert).
pub(super) fn antialias_code(name: &str) -> i32 {
    match name {
        "none" => -1,
        "crisp" => 1,
        "strong" => 2,
        "system" => 3,
        "custom" => 4,
        _ => 0,
    }
}

/// Richtungsname → `ORIi`.
pub(super) fn orientation_code(name: &str) -> i32 {
    match name {
        "verticalRtl" => 1,
        "horizontalRtl" => 2,
        "verticalLtr" => 3,
        _ => 0,
    }
}

/// Setzt die Glättung eines Textobjekts: −1 = harte Füllkante, sonst `TAAi` mit weicher Kante.
pub(super) fn apply_antialias(t: &mut TextElem, v: i32) {
    let edge = if v < 0 { 0 } else { 1 };
    if v >= 0 {
        t.antialias = v;
    }
    let at = Arc::make_mut(&mut t.attrs);
    if let Some(f) = at.fill.as_mut() {
        f.edge = edge;
    }
}

/// Vorgaben des Textwerkzeugs für neue Texte aus den Voreinstellungen wie im Original:
/// Glättung (`Text_AntiAliasLevel` = `TAAi`, `Text_AntiAliasedFill` aus = keine Kantenglättung → −1),
/// `Text_AutoKern`, `Text_Orientation`. Werksvorgabe: glatt, ohne Unterschneidung, waagerecht (O6).
pub fn tool_text_defaults(prefs: &crate::prefs::Preferences) -> (i32, bool, i32) {
    let aa = if prefs.get_bool("Text_AntiAliasedFill", true) {
        prefs.get_int("Text_AntiAliasLevel", 0).clamp(0, 4) as i32
    } else {
        -1
    };
    (
        aa,
        prefs.get_bool("Text_AutoKern", false),
        prefs.get_int("Text_Orientation", 0).clamp(0, 3) as i32,
    )
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        text_attr_opts(cmd("text.setToolDefaults").label("Textwerkzeug-Vorgaben"))
            .opt(
                "antialias",
                ParamKind::Enum {
                    values: antialias_names(),
                },
                "Glättung neuer Texte",
            )
            .opt(
                "autoKern",
                ParamKind::Bool,
                "automatische Unterschneidung neuer Texte",
            )
            .opt(
                "orientation",
                ParamKind::Enum {
                    values: orientation_names(),
                },
                "Textrichtung neuer Texte",
            )
            .run(|s, p| {
                use crate::prefs::PrefValue;
                store_text_tool_defaults(s, p, true, true)?;
                if let Some(n) = p.opt_str("antialias")? {
                    let v = antialias_code(&n);
                    s.prefs.set("Text_AntiAliasedFill", PrefValue::Bool(v >= 0));
                    if v >= 0 {
                        s.prefs.set("Text_AntiAliasLevel", PrefValue::Int(v as i64));
                    }
                }
                if let Some(v) = p.opt_bool("autoKern")? {
                    s.prefs.set("Text_AutoKern", PrefValue::Bool(v));
                }
                if let Some(n) = p.opt_str("orientation")? {
                    s.prefs.set(
                        "Text_Orientation",
                        PrefValue::Int(orientation_code(&n) as i64),
                    );
                }
                let (aa, kern, ori) = tool_text_defaults(&s.prefs);
                let aa_name = match aa {
                    -1 => "none",
                    1 => "crisp",
                    2 => "strong",
                    3 => "system",
                    4 => "custom",
                    _ => "smooth",
                };
                Ok(CommandOutput::session_with(serde_json::json!({
                    "antialias": aa_name,
                    "autoKern": kern,
                    "orientation": orientation_names()[ori as usize],
                })))
            }),
    );
    // Im Original immer aktiv: ohne ausgewählten Text setzt die Wahl die Vorgabe des Textwerkzeugs
    // (`Text_Font`, PM-7/MN-2).
    r.add(
        cmd("text.font")
            .requires(DOC)
            .param(
                "font",
                ParamKind::String,
                "Schriftname (PostScript- oder Familienname)",
            )
            .run(|s, p| {
                let f = p.str("font")?;
                let font = f.clone();
                texts_or_defaults(
                    s,
                    move |t| {
                        let f = f.clone();
                        set_attr(t, &move |a| a.font = Some(f.clone()));
                        t.initial.ps_font = Some(p_font(&t.initial));
                    },
                    move |s| {
                        // TW-138: `Text_CoolType_Font` (PostScript name) + recently used fonts
                        store_font_default(s, &font);
                        note_recent_font(s, &font);
                    },
                )
            }),
    );
    // Größe/Stil/Ausrichtung sind im Original immer aktiv: ohne ausgewählten Text setzen sie die
    // Vorgaben des Textwerkzeugs (`Text_Size`, `Text_Bold` …).
    r.add(
        cmd("text.size.other")
            .requires(DOC)
            .param(
                "size",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(5000.0),
                },
                "Größe in px",
            )
            .run(|s, p| {
                let v = p.f64("size")? as f32;
                texts_or_defaults(
                    s,
                    move |t| set_attr(t, &move |a| a.size = Some(v)),
                    move |s| set_default_size(s, v),
                )
            }),
    );
    for &v in SIZES {
        let name = format!("text.size.{}", v as i32);
        r.add(
            cmd(&name)
                .requires(DOC)
                .checked(move |s| current_size(s) == Some(v))
                .run(move |s, _| {
                    texts_or_defaults(
                        s,
                        move |t| set_attr(t, &move |a| a.size = Some(v)),
                        move |s| set_default_size(s, v),
                    )
                }),
        );
    }
    // Larger/smaller step every run by exactly 1 pt (relative, no size list; TW-133).
    for (name, delta) in [("text.size.larger", 1.0), ("text.size.smaller", -1.0)] {
        r.add(cmd(name).requires(TEXT_SEL).run(move |s, _| {
            edit_texts(s, move |t| {
                let fallback = first_size(t);
                set_attr(t, &move |a| {
                    a.size = Some(step_size(a.size.unwrap_or(fallback), delta));
                });
            })
        }));
    }
    r.add(
        cmd("text.style.plain")
            .requires(DOC)
            .checked(|s| {
                ["Text_Bold", "Text_Italic", "Text_Underline"]
                    .iter()
                    .all(|k| !current_style(s, k))
            })
            .run(|s, _| {
                texts_or_defaults(
                    s,
                    |t| {
                        set_attr(t, &|a| {
                            a.bold = Some(false);
                            a.italic = Some(false);
                            a.underline = Some(false);
                        })
                    },
                    |s| {
                        for k in ["Text_Bold", "Text_Italic", "Text_Underline"] {
                            s.prefs.set(k, crate::prefs::PrefValue::Bool(false));
                        }
                    },
                )
            }),
    );
    type Get = fn(&TextAttrs) -> Option<bool>;
    type Set = fn(&mut TextAttrs, bool);
    let styles: [(&str, &str, Get, Set); 3] = [
        (
            "text.style.fauxBold",
            "Text_Bold",
            |a| a.bold,
            |a, v| a.bold = Some(v),
        ),
        (
            "text.style.fauxItalic",
            "Text_Italic",
            |a| a.italic,
            |a, v| a.italic = Some(v),
        ),
        (
            "text.style.underline",
            "Text_Underline",
            |a| a.underline,
            |a, v| a.underline = Some(v),
        ),
    ];
    for (name, key, get, set) in styles {
        r.add(
            cmd(name)
                .requires(DOC)
                .checked(move |s| {
                    if has_text(s) {
                        all_texts_have(s, get)
                    } else {
                        current_style(s, key)
                    }
                })
                .run(move |s, _| {
                    // Toggle over the whole pointer selection (re 46 §1.3): only when every
                    // selected text already has the style everywhere is it removed, else set on
                    // all runs of all selected texts.
                    // Without selected text the default toggles.
                    let on = if has_text(s) {
                        !all_texts_have(s, get)
                    } else {
                        !current_style(s, key)
                    };
                    texts_or_defaults(
                        s,
                        move |t| set_attr(t, &move |a| set(a, on)),
                        move |s| s.prefs.set(key, crate::prefs::PrefValue::Bool(on)),
                    )
                }),
        );
    }
    for (name, v) in [
        ("text.align.left", 0),
        ("text.align.centeredHorizontally", 1),
        ("text.align.right", 2),
        ("text.align.justified", 3),
        ("text.align.stretched", 4),
        ("text.align.top", 0x10),
        ("text.align.centeredVertically", 0x11),
        ("text.align.bottom", 0x12),
        ("text.align.justifiedVertically", 0x13),
    ] {
        r.add(
            cmd(name)
                .requires(DOC)
                .checked(move |s| {
                    if has_text(s) {
                        s.active().is_some_and(|d| any_selected(d, |e| matches!(&e.kind, ElementKind::Text(t) if t.resolved_runs().first().and_then(|(a, _)| a.alignment).unwrap_or(0) == v)))
                    } else {
                        let ori = s.prefs.get_int("Text_Orientation", 0);
                        let vertical = if ori == 1 || ori == 3 { 0x10 } else { 0 };
                        let just = s.prefs.get_int("Text_Justification", 0) as i32;
                        effective_alignment(just | vertical) == v
                    }
                })
                .run(move |s, _| {
                    // TW-133 (re 50 section 1): the defaults are always written (also 4 =
                    // stretched); selected texts get the alignment the engine can store
                    // ("stretched" becomes left on every text), are laid out again and the
                    // step is named "Text change"
                    let eff = effective_alignment(v);
                    store_alignment_prefs(s, v);
                    s.prefs_changed();
                    if !has_text(s) {
                        return Ok(CommandOutput::session());
                    }
                    let out = edit_texts(s, move |t| set_attr(t, &move |a| a.alignment = Some(eff)))?;
                    reduce_selection_to_texts(s);
                    Ok(out.labeled(fp_i18n::t("msg-step-text-change")))
                }),
        );
    }
    // wie im Original nur bei Text **und** Pfad bzw. bei Text auf Pfad
    r.add(
        cmd("text.attachToPath")
            .requires(&[Requirement::Document, Requirement::MinSelected(2)])
            .available(|s| text_and_path(s).is_ok())
            .run(|s, _| attach(s, false)),
    );
    r.add(
        cmd("text.attachInPath")
            .requires(&[Requirement::Document, Requirement::MinSelected(2)])
            .available(|s| text_and_path(s).is_ok())
            .run(|s, _| attach(s, true)),
    );
    r.add(
        cmd("text.detachFromPath")
            .requires(TEXT_SEL)
            .available(has_text_on_path)
            .run(|s, _| {
                let renderer = s.renderer().cloned();
                with_doc(s, |d| {
                    let page = d.page;
                    let texts: Vec<_> = selected_elements(d)
                        .into_iter()
                        .filter(
                            |(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.on_path.is_some()),
                        )
                        .collect();
                    if texts.is_empty() {
                        return Err(sel_err("err-sel-no-text-on-path"));
                    }
                    let mut sel = Vec::new();
                    for (_, e) in texts {
                        let ElementKind::Text(t) = &e.kind else {
                            continue;
                        };
                        let path = t.on_path.as_ref().and_then(|o| o.path.clone());
                        let center = path
                            .as_ref()
                            .and_then(|p| docops::element_bounds(&d.doc, p))
                            .map(|b| ((b.left + b.right) * 0.5, (b.top + b.bottom) * 0.5));
                        let detached = detached_text(&d.doc, t, center, renderer.as_deref());
                        let doc = Arc::make_mut(&mut d.doc);
                        docops::edit_by_id(doc, page, e.id, |x| {
                            if let ElementKind::Text(t) = &mut x.kind {
                                **t = detached;
                            }
                        });
                        if let Some(pth) = path {
                            // wie im Original: der Pfad kommt als oberstes Objekt der Ebene zurück (Q-16)
                            let n = docops::reassign(doc, &pth);
                            sel.push(n.id);
                            if let Some(cur) = docops::find_on_page(doc, page, e.id)
                                && let Some((list, _)) = docops::parent_list_mut(doc, &cur)
                            {
                                list.insert(0, Arc::new(n));
                            }
                        }
                        sel.push(e.id);
                    }
                    d.selection.set(sel);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    for (name, mode) in [
        ("text.orientation.rotateAroundPath", 0),
        ("text.orientation.vertical", 1),
        ("text.orientation.skewVertical", 2),
        ("text.orientation.skewHorizontal", 3),
    ] {
        r.add(
            cmd(name)
                .requires(TEXT_SEL)
                .available(has_text_on_path)
                .checked(move |s| {
                    s.active().is_some_and(|d| any_selected(d, |e| matches!(&e.kind, ElementKind::Text(t) if t.on_path.as_ref().and_then(|o| o.mode) == Some(mode))))
                })
                .run(move |s, _| {
                    edit_texts(s, move |t| {
                        if let Some(o) = &mut t.on_path {
                            o.mode = Some(mode);
                        }
                    })
                }),
        );
    }
    /// Pfad eines Texts am Pfad umkehren wie im Original (FP1-03, Vergleichslauf 7 `menu-text/10`):
    /// die Knotenfolge rückwärts, Henkel getauscht – auch bei geschlossenen Konturen **ohne** Drehung,
    /// der bisher letzte Knoten wird Startpunkt. TX3 am Kreis (Start links, im Uhrzeigersinn) läuft
    /// danach vom unteren Knoten gegen den Uhrzeigersinn über den rechten Bogen und bleibt am Kreis.
    fn reverse_text_path(c: &fp_model::Contour) -> fp_model::Contour {
        let b = crate::geometry::to_bezier(c);
        let nodes = b
            .nodes
            .iter()
            .rev()
            .map(fp_geom::BezierNode::swapped)
            .collect();
        crate::geometry::from_bezier(&fp_geom::BezierContour::new(nodes, b.closed))
    }

    r.add(
        cmd("text.reverseDirection")
            .requires(TEXT_SEL)
            .available(has_text_on_path)
            .run(|s, _| {
                edit_texts(s, |t| {
                    if let Some(o) = &mut t.on_path
                        && let Some(pe) = &mut o.path
                    {
                        let e = Element::edit(pe);
                        if let ElementKind::Path(p) = &mut e.kind {
                            p.contours = p.contours.iter().map(reverse_text_path).collect();
                        }
                    }
                })
            }),
    );
    r.add(
        cmd("text.setText")
            .label("Text bearbeiten")
            .requires(crate::commands::DOC)
            .opt("id", ParamKind::Integer { min: Some(0), max: None }, "Textobjekt (Vorgabe: Auswahl)")
            .param("text", ParamKind::String, "neuer Text (Absätze mit \\r)")
            .opt("runs", ParamKind::Json, "formatierte Läufe [{attrs, text}] (Text-Editor); Vorgabe: ein Lauf mit den Attributen des ersten")
            .run(|s, p| {
                let txt = normalize_breaks(&p.str("text")?);
                let runs = parse_runs(p, &txt)?;
                with_doc(s, |d| {
                    let id = match p.opt_i64("id")? {
                        Some(v) => fp_model::ElementId(v as u64),
                        None => *d.selection.elements.first().ok_or_else(|| sel_err("err-sel-no-text"))?,
                    };
                    let page = d.page;
                    let doc = Arc::make_mut(&mut d.doc);
                    if txt.is_empty() {
                        docops::remove_ids(doc, page, &[id]);
                        d.selection.clear();
                        return Ok(CommandOutput::doc());
                    }
                    docops::edit_by_id(doc, page, id, |e| {
                        if let ElementKind::Text(t) = &mut e.kind {
                            match runs {
                                Some(r) => crate::tools::text::apply_runs(t, r),
                                None => {
                                    let attrs = t.runs.first().map(|r| r.attrs.clone()).unwrap_or_else(|| t.initial.clone());
                                    t.runs = vec![TextRun { attrs, text: txt.clone(), extra: Extra::default() }];
                                    t.detach_story();
                                    // wie eine Bearbeitung im Editor: das Original setzt den Text neu (M10, re 43 §4)
                                    t.relaid = true;
                                }
                            }
                        }
                    })
                    .ok_or(EngineError::ElementNotFound(id.0))?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("text.convertToPaths")
            .requires(TEXT_SEL)
            .js("convertToPaths")
            .run(|s, _| convert_to_paths(s)),
    );
    r.add(
        cmd("text.editor")
            .label("Texteditor")
            .requires(TEXT_SEL)
            .opt(
                "text",
                ParamKind::String,
                "neuer Text (Absätze mit \\r); ohne Parameter: Inhalt lesen",
            )
            .opt("font", ParamKind::String, "Schrift")
            .opt(
                "size",
                ParamKind::Number {
                    min: Some(1.0),
                    max: Some(5000.0),
                },
                "Größe",
            )
            .opt("bold", ParamKind::Bool, "Faux Fett")
            .opt("italic", ParamKind::Bool, "Faux Kursiv")
            .opt("underline", ParamKind::Bool, "unterstrichen")
            .opt("color", ParamKind::Color, "Textfarbe")
            .run(|s, p| {
                let d = s.doc()?;
                let (_, e) = selected_elements(d)
                    .into_iter()
                    .find(|(_, e)| matches!(e.kind, ElementKind::Text(_)))
                    .ok_or_else(|| sel_err("err-sel-no-text-selected"))?;
                let changes = [
                    "text",
                    "font",
                    "size",
                    "bold",
                    "italic",
                    "underline",
                    "color",
                ]
                .iter()
                .any(|k| p.has(k));
                if !changes {
                    let ElementKind::Text(t) = &e.kind else {
                        return Err(sel_err("err-sel-no-text"));
                    };
                    return Ok(CommandOutput::value(text_info(t)));
                }
                let a = attrs_from_params(p)?;
                let color = p.opt_color("color")?;
                let txt = p.opt_str("text")?;
                let id = e.id;
                with_doc(s, |d| {
                    let page = d.page;
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_by_id(doc, page, id, |e| {
                        if let ElementKind::Text(t) = &mut e.kind {
                            if let Some(txt) = &txt {
                                let attrs = t
                                    .runs
                                    .first()
                                    .map(|r| r.attrs.clone())
                                    .unwrap_or_else(|| t.initial.clone());
                                t.runs = vec![TextRun {
                                    attrs,
                                    text: txt.clone(),
                                    extra: Extra::default(),
                                }];
                            }
                            let a2 = a.clone();
                            set_attr(t, &move |x| x.apply(&a2));
                            if let Some(c) = color {
                                Arc::make_mut(&mut t.attrs).fill_color = c;
                            }
                            t.detach_story();
                        }
                    })
                    .ok_or(EngineError::ElementNotFound(id.0))?;
                    let info = docops::element_on_page(doc, page, id).and_then(|e| {
                        if let ElementKind::Text(t) = &e.kind {
                            Some(text_info(t))
                        } else {
                            None
                        }
                    });
                    Ok(CommandOutput::doc_with(info.unwrap_or_default())
                        .labeled(fp_i18n::t("msg-step-edit-text")))
                })
            }),
    );
    r.add(
        cmd("text.setProperties")
            .label("Texteigenschaften")
            .requires(crate::commands::DOC)
            .opt("ids", ParamKind::Ids, "Textobjekte (Vorgabe: Auswahl)")
            .opt(
                "frameType",
                ParamKind::Enum {
                    values: vec!["auto".into(), "fixed".into()],
                },
                "Rahmenart: automatisch (Punkttext) oder feste Breite",
            )
            .opt(
                "width",
                ParamKind::Number {
                    min: Some(1.0),
                    max: None,
                },
                "Rahmenbreite bei fester Breite",
            )
            .opt(
                "antialias",
                ParamKind::Enum {
                    values: antialias_names(),
                },
                "Glättung",
            )
            .opt(
                "sharpness",
                ParamKind::Integer {
                    min: Some(0),
                    max: Some(255),
                },
                "Schärfe (benutzerdefiniert)",
            )
            .opt(
                "strength",
                ParamKind::Integer {
                    min: Some(0),
                    max: Some(255),
                },
                "Stärke (benutzerdefiniert)",
            )
            .opt(
                "oversample",
                ParamKind::Integer {
                    min: Some(1),
                    max: Some(64),
                },
                "Überabtastung (benutzerdefiniert)",
            )
            .opt("autoKern", ParamKind::Bool, "automatische Unterschneidung")
            .opt(
                "kerning",
                ParamKind::Number {
                    min: None,
                    max: None,
                },
                "Unterschneidung (Tausendstel em) für alle Zeichen",
            )
            .opt(
                "orientation",
                ParamKind::Enum {
                    values: orientation_names(),
                },
                "Textrichtung (ORIi 0–3)",
            )
            .run(|s, p| {
                let frame = p.opt_str("frameType")?;
                let width = p.opt_f64("width")?;
                let aa = p.opt_str("antialias")?.map(|n| antialias_code(&n));
                let (sharp, strength, over) = (
                    p.opt_i64("sharpness")?,
                    p.opt_i64("strength")?,
                    p.opt_i64("oversample")?,
                );
                let auto_kern = p.opt_bool("autoKern")?;
                let kerning = p.opt_f64("kerning")?.map(|v| v as f32);
                let ori = p.opt_str("orientation")?.map(|n| orientation_code(&n));
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut n = 0;
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            if let ElementKind::Text(t) = &mut e.kind {
                                n += 1;
                                match frame.as_deref() {
                                    Some("auto") => t.frame_type = Some(0),
                                    Some("fixed") => {
                                        t.frame_type = Some(1);
                                        let w = width
                                            .map(|w| w as f32)
                                            .unwrap_or(t.bounds.right - t.bounds.left)
                                            .max(1.0);
                                        let r = t.anchor_rect.unwrap_or(t.bounds);
                                        t.anchor_rect = Some(fp_geom::Rect::new(
                                            r.left,
                                            r.top,
                                            r.left + w,
                                            r.bottom.max(r.top + 1.0),
                                        ));
                                    }
                                    _ => {}
                                }
                                if let Some(v) = aa {
                                    // „Keine Kantenglättung“ = harte Füllkante (FETi 0), sonst Modus TAAi
                                    apply_antialias(t, v);
                                }
                                if let Some(v) = sharp {
                                    t.aa_sharpness = v as i32;
                                }
                                if let Some(v) = strength {
                                    t.aa_strength = v as i32;
                                }
                                if let Some(v) = over {
                                    t.aa_oversample = v as i32;
                                }
                                if let Some(v) = auto_kern {
                                    t.auto_kern = v;
                                }
                                if let Some(k) = kerning {
                                    set_attr(t, &move |a| a.kerning = Some(k));
                                }
                                if let Some(o) = ori {
                                    t.orientation = o;
                                }
                                t.detach_story();
                            }
                        });
                    }
                    if n == 0 {
                        return Err(sel_err("err-sel-no-text"));
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        text_attr_opts(
            cmd("text.setAttributes")
                .label("Textattribute")
                .requires(crate::commands::DOC)
                .opt("ids", ParamKind::Ids, "Textobjekte (Vorgabe: Auswahl)"),
        )
        .run(set_attributes),
    );
    // Inspector entry (TW-138, re 50 section 2): with the text editor open the attributes go to
    // the marked text (routed by `Session::execute`, see `editor_patch`), with text objects
    // selected to those texts; in every case they become the defaults (`Text_*`). With nothing
    // selected they only set the defaults.
    r.add(
        text_attr_opts(
            cmd("text.applyAttributes")
                .label("Textattribute anwenden")
                .requires(crate::commands::DOC),
        )
        .run(|s, p| {
            // R7 (re 56 section 3): every field change is its own "Text change" row, also
            // with nothing selected (then a step without a document change)
            if has_text(s) {
                let out = set_attributes(s, p)?;
                store_text_tool_defaults(s, p, true, false)?;
                s.prefs_changed();
                return Ok(out.labeled(fp_i18n::t("msg-step-text-change")));
            }
            store_text_tool_defaults(s, p, true, false)?;
            s.prefs_changed();
            s.force_history_step();
            Ok(CommandOutput::doc().labeled(fp_i18n::t("msg-step-text-change")))
        }),
    );
}

/// Text ▸ In Pfade konvertieren: jedes Textobjekt wird eine Gruppe aus Pfaden (ein Pfad je
/// Zeichen, Unterstreichungen als eigene Pfade) mit der Textfarbe; Name, Deckkraft, Mischmodus und
/// Effekte gehen auf die Gruppe über. Bei nur einer Glyphe entsteht ein einzelner Pfad.
fn convert_to_paths(s: &mut Session) -> Result<CommandOutput> {
    let renderer = s
        .renderer()
        .cloned()
        .ok_or(EngineError::BackendMissing("err-backend-text-renderer"))?;
    with_doc(s, |d| {
        let page = d.page;
        let texts: Vec<_> = selected_elements(d)
            .into_iter()
            .filter(|(_, e)| matches!(e.kind, ElementKind::Text(_)))
            .collect();
        if texts.is_empty() {
            return Err(sel_err("err-sel-no-text-selected"));
        }
        let mut new_sel: Vec<fp_model::ElementId> = d
            .selection
            .elements
            .iter()
            .copied()
            .filter(|id| !texts.iter().any(|(_, e)| e.id == *id))
            .collect();
        for (_, e) in texts {
            let ElementKind::Text(t) = &e.kind else {
                continue;
            };
            let outlines = renderer
                .text_outlines(&d.doc, t)
                .ok_or(EngineError::BackendMissing("err-backend-text-layout"))?;
            let color = t
                .resolved_runs()
                .first()
                .and_then(|(a, _)| a.color)
                .map(fp_color::Argb)
                .unwrap_or(t.attrs.fill_color);
            let mut attrs = (*t.attrs).clone();
            attrs.fill_color = color;
            if attrs.fill.is_none() {
                attrs.fill = Some(fp_model::Fill::solid());
            }
            let doc = Arc::make_mut(&mut d.doc);
            let mut ids = super::take_ids(doc);
            let mut paths: Vec<Arc<Element>> = Vec::new();
            for glyph in outlines.iter().filter(|g| !g.is_empty()) {
                let contours: Vec<_> = glyph.iter().map(crate::geometry::from_bezier).collect();
                paths.push(Arc::new(Element::new_path(
                    &mut ids,
                    contours,
                    attrs.clone(),
                )));
            }
            let replacement = match paths.len() {
                0 => None,
                1 => {
                    let mut p = (*paths[0]).clone();
                    p.common = e.common.clone();
                    p.common.name = None; // TW-26: Ergebnis im Original namenlos
                    Some(p)
                }
                _ => {
                    let mut g = Element::new_group(&mut ids, paths);
                    g.common = e.common.clone();
                    g.common.name = None; // TW-26: Ergebnis im Original namenlos
                    Some(g)
                }
            };
            doc.ids = ids;
            let Some(path) = docops::find_on_page(doc, page, e.id) else {
                continue;
            };
            let Some((list, idx)) = docops::parent_list_mut(doc, &path) else {
                continue;
            };
            match replacement {
                // Ergebnis mit neuer ID (die Text-ID verschwindet wie im Original)
                Some(r) => {
                    new_sel.push(r.id);
                    list[idx] = Arc::new(r);
                }
                // TW-40: ohne gesetzte Glyphen (z. B. Rahmen schmaler als ein Zeichen) bleibt der Text
                // wie im Original unverändert stehen und gewählt
                None => {
                    new_sel.push(e.id);
                }
            }
        }
        d.selection.set(new_sel);
        Ok(CommandOutput::doc())
    })
}

/// Parameters shared by `text.setAttributes`, `text.applyAttributes` and `text.setToolDefaults`.
fn text_attr_opts(b: crate::command::CommandBuilder) -> crate::command::CommandBuilder {
    b.opt("font", ParamKind::String, "Schrift")
        .opt(
            "size",
            ParamKind::Number {
                min: Some(1.0),
                max: Some(5000.0),
            },
            "Größe",
        )
        .opt("bold", ParamKind::Bool, "Faux Fett")
        .opt("italic", ParamKind::Bool, "Faux Kursiv")
        .opt("underline", ParamKind::Bool, "unterstrichen")
        .opt("color", ParamKind::Color, "Textfarbe")
        .opt(
            "alignment",
            ParamKind::Integer {
                min: Some(0),
                max: Some(0x14),
            },
            "JSTi",
        )
        .opt(
            "leading",
            ParamKind::Number {
                min: Some(0.0),
                max: None,
            },
            "Zeilenabstand",
        )
        .opt(
            "leadingMode",
            ParamKind::Integer {
                min: Some(0),
                max: Some(1),
            },
            "0 Prozent, 1 exakt",
        )
        .opt(
            "tracking",
            ParamKind::Number {
                min: None,
                max: None,
            },
            "Laufweite",
        )
        .opt(
            "baselineShift",
            ParamKind::Number {
                min: None,
                max: None,
            },
            "Grundlinienversatz",
        )
        .opt(
            "horizontalScale",
            ParamKind::Number {
                min: Some(0.01),
                max: None,
            },
            "horizontale Skalierung (1 = 100 %)",
        )
        .opt(
            "indent",
            ParamKind::Number {
                min: None,
                max: None,
            },
            "Einzug",
        )
        .opt(
            "spaceBefore",
            ParamKind::Number {
                min: None,
                max: None,
            },
            "Abstand vor",
        )
        .opt(
            "spaceAfter",
            ParamKind::Number {
                min: None,
                max: None,
            },
            "Abstand nach",
        )
}

/// `text.setAttributes` on the given or selected text objects.
fn set_attributes(s: &mut Session, p: &Params) -> Result<CommandOutput> {
    let a = attrs_from_params(p)?;
    let color = p.opt_color("color")?;
    with_doc(s, |d| {
        let (page, ids) = (d.page, target_ids(d, p)?);
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| {
                if let ElementKind::Text(t) = &mut e.kind {
                    let a2 = a.clone();
                    set_attr(t, &move |x| x.apply(&a2));
                    if let Some(c) = color {
                        Arc::make_mut(&mut t.attrs).fill_color = c;
                    }
                    t.detach_story();
                }
            });
        }
        Ok(CommandOutput::doc())
    })
}

/// Live preview of a Properties inspector text slider (FB-21/FB-22): applies the attributes of a
/// `text.applyAttributes` parameter object to the text objects `ids` on page `page` of `doc` (a copy
/// of the document the user interface renders), exactly as the command does for selected text
/// objects, but without a history step and without touching the `Text_*` defaults. Other elements
/// are left as they are.
pub fn preview_text_attributes(
    doc: &mut fp_model::Document,
    page: usize,
    ids: &[fp_model::ElementId],
    params: &serde_json::Value,
) -> Result<()> {
    let p = Params::new("text.applyAttributes", params.clone());
    let a = attrs_from_params(&p)?;
    let color = p.opt_color("color")?;
    for &id in ids {
        docops::edit_by_id(doc, page, id, |e| {
            if let ElementKind::Text(t) = &mut e.kind {
                let a2 = a.clone();
                set_attr(t, &move |x| x.apply(&a2));
                if let Some(c) = color {
                    Arc::make_mut(&mut t.attrs).fill_color = c;
                }
                t.detach_story();
            }
        });
    }
    Ok(())
}

/// Stores text attributes as the defaults of the text tool (prefs `Text_*`, app-wide; TW-138).
/// New text never reads `Text_FillColor` (re 50 section 2.3): its colour is the current fill colour
/// of the text tool (`tools.options.text_color`), which `tool_color` also sets - not for a
/// partial selection in the open editor.
///
/// `baseline_shift`: only the explicit default setter (`text.setToolDefaults`) writes
/// `Text_BaselineShift`; the Properties panel never does (re 53 section 2.2, R1).
fn store_text_tool_defaults(
    s: &mut Session,
    p: &Params,
    tool_color: bool,
    baseline_shift: bool,
) -> Result<()> {
    use crate::prefs::PrefValue;
    if let Some(f) = p.opt_str("font")? {
        store_font_default(s, &f);
        note_recent_font(s, &f);
    }
    if let Some(v) = p.opt_f64("size")? {
        set_default_size(s, v as f32);
    }
    if let Some(c) = p.opt_color("color")? {
        let hex = fp_color::Rgba8::from(c).to_hex_rgba();
        s.prefs.set("Text_FillColor", PrefValue::Str(hex));
        if tool_color {
            s.tools.options.text_color = c;
        }
    }
    for (k, pref) in [
        ("bold", "Text_Bold"),
        ("italic", "Text_Italic"),
        ("underline", "Text_Underline"),
    ] {
        if let Some(v) = p.opt_bool(k)? {
            s.prefs.set(pref, PrefValue::Bool(v));
        }
    }
    for (k, pref) in [
        ("alignment", "Text_Justification"),
        ("leadingMode", "Text_LeadingMode"),
    ] {
        if let Some(v) = p.opt_i64(k)? {
            s.prefs.set(pref, PrefValue::Int(v));
        }
    }
    for (k, pref) in TEXT_TOOL_REAL_PREFS {
        if k == "baselineShift" && !baseline_shift {
            continue;
        }
        if let Some(v) = p.opt_f64(k)? {
            // the tracking field value is stored divided by 100 (field 15 -> 0.15, re 56 section 0)
            let v = if k == "tracking" { v / 100.0 } else { v };
            s.prefs.set(pref, PrefValue::Real(v));
        }
    }
    Ok(())
}

/// R5 (re 56 section 2.4/3): activating the text tool resets the paragraph defaults (indent,
/// spacing before/after, leading to `DefaultLeading` in % mode, tracking, horizontal scale,
/// baseline shift to `DefaultBaselineShift`, measured re 56 section 5.3, TW-178) unless `TextEditor_RememberALLSettings` is on. Size, style, font and alignment
/// stay. Firepixel has no text-tool-only opacity/blend option (the tool opacity/blend fields are
/// shared with other tools and not applied to new text), so that part of the reset is omitted.
pub(crate) fn reset_text_tool_paragraph_defaults(s: &mut Session) {
    use crate::prefs::PrefValue;
    if s.prefs.get_bool("TextEditor_RememberALLSettings", false) {
        return;
    }
    let leading = s.prefs.get_real("DefaultLeading", 1.2);
    let shift = s.prefs.get_real("DefaultBaselineShift", 0.0);
    let before = s.prefs.clone();
    s.prefs.set("Text_Leading", PrefValue::Real(leading));
    s.prefs.set("Text_LeadingMode", PrefValue::Int(0));
    for (pref, v) in [
        ("Text_RangeKerning", 0.0),
        ("Text_HorizScale", 1.0),
        ("Text_BaselineShift", shift),
        ("Text_ParagraphIndent", 0.0),
        ("Text_ParagraphSpacingBefore", 0.0),
        ("Text_ParagraphSpacingAfter", 0.0),
    ] {
        s.prefs.set(pref, PrefValue::Real(v));
    }
    if s.prefs != before {
        s.prefs_changed();
    }
}

/// Leading value after switching the Properties panel unit (measured, re 56 section 5.4,
/// TW-178): the conversion starts from the displayed field value, rounded half up, and stores
/// the result unrounded. % -> px is `round(factor * 100) * size / 100` (123 % at 30 pt -> 36.9,
/// 125 % -> 37.5), px -> % is `round(px) / size` (37.5 shown as 38 -> 1.2667), with the size
/// limited to 1000 (observed once); same unit unchanged.
pub fn convert_leading(leading: f32, size: f32, from_px: bool, to_px: bool) -> f32 {
    let (leading, size) = (leading as f64, size as f64);
    match (from_px, to_px) {
        (false, true) => ((leading * 100.0).round() * size / 100.0) as f32,
        (true, false) => (leading.round() / size.clamp(1.0, 1000.0)) as f32,
        _ => leading as f32,
    }
}

/// Number shown in the Properties panel leading field: whole px or whole percent, rounded half
/// up like the original (37.5 px -> 38, 34.5 px -> 35, 1.2333 -> 123 %; re 56 section 5.4).
pub fn leading_field_value(leading: f32, px: bool) -> f64 {
    let v = leading as f64;
    if px { v.round() } else { (v * 100.0).round() }
}

/// Number attributes of the text tool defaults: parameter and preference key.
const TEXT_TOOL_REAL_PREFS: [(&str, &str); 7] = [
    ("leading", "Text_Leading"),
    ("tracking", "Text_RangeKerning"),
    ("indent", "Text_ParagraphIndent"),
    ("spaceBefore", "Text_ParagraphSpacingBefore"),
    ("spaceAfter", "Text_ParagraphSpacingAfter"),
    ("horizontalScale", "Text_HorizScale"),
    ("baselineShift", "Text_BaselineShift"),
];

/// Text attribute command routed to the open text editor: writes the `Text_*` defaults like
/// without editor (TW-138, re 50 section 2.2) - the size, font, style and alignment of a partial
/// selection become the defaults of new text. Larger/Smaller (1 pt steps) and the script setter
/// `text.setAttributes` write nothing; a colour is stored in `Text_FillColor` only (new text
/// keeps the fill colour of the text tool).
pub(crate) fn store_editor_prefs(
    s: &mut Session,
    name: &str,
    params: &serde_json::Value,
    patch: &TextAttrs,
) -> Result<()> {
    use crate::prefs::PrefValue;
    match name {
        "text.size.larger" | "text.size.smaller" | "text.setAttributes" => return Ok(()),
        "text.applyAttributes" => {
            store_text_tool_defaults(s, &Params::new(name, params.clone()), false, false)?;
        }
        _ => {
            if let Some(code) = name
                .strip_prefix("text.align.")
                .and_then(horizontal_align_code)
            {
                // editor: only `Text_Justification` (re 50 section 2.2), as given (also 4)
                s.prefs
                    .set("Text_Justification", PrefValue::Int(code as i64));
            }
            if let Some(f) = &patch.font {
                store_font_default(s, f);
                note_recent_font(s, f);
            }
            if let Some(v) = patch.size {
                set_default_size(s, v);
            }
            for (v, key) in [
                (patch.bold, "Text_Bold"),
                (patch.italic, "Text_Italic"),
                (patch.underline, "Text_Underline"),
            ] {
                if let Some(v) = v {
                    s.prefs.set(key, PrefValue::Bool(v));
                }
            }
        }
    }
    s.prefs_changed();
    Ok(())
}

/// Code of a horizontal Text > Align command (`text.align.<name>`).
fn horizontal_align_code(name: &str) -> Option<i32> {
    match name {
        "left" => Some(0),
        "centeredHorizontally" => Some(1),
        "right" => Some(2),
        "justified" => Some(3),
        "stretched" => Some(4),
        _ => None,
    }
}

/// Menü-/Inspektor-Befehl als Attributänderung für den offenen Text-Editor (`cur` = Attribute an
/// der Markierung): `text.setAttributes` (ohne `ids`), `text.font`, `text.size.*`, `text.style.*`,
/// `text.align.*` (horizontal). `None` = kein Textattribut-Befehl.
pub(crate) fn editor_patch(
    name: &str,
    params: &serde_json::Value,
    cur: &TextAttrs,
) -> Option<TextAttrs> {
    if params.get("ids").is_some_and(|v| !v.is_null()) {
        return None;
    }
    let p = Params::new(name, params.clone());
    let mut a = TextAttrs::default();
    match name {
        "text.setAttributes" | "text.applyAttributes" => a = attrs_from_params(&p).ok()?,
        "text.font" => a.font = Some(p.str("font").ok()?),
        "text.size.other" => a.size = Some(p.f64("size").ok()? as f32),
        "text.size.larger" | "text.size.smaller" => {
            let c = cur.size.unwrap_or(12.0);
            let delta = if name.ends_with("larger") { 1.0 } else { -1.0 };
            a.size = Some(step_size(c, delta));
        }
        "text.style.plain" => {
            a.bold = Some(false);
            a.italic = Some(false);
            a.underline = Some(false);
        }
        "text.style.fauxBold" => a.bold = Some(!cur.bold.unwrap_or(false)),
        "text.style.fauxItalic" => a.italic = Some(!cur.italic.unwrap_or(false)),
        "text.style.underline" => a.underline = Some(!cur.underline.unwrap_or(false)),
        "text.align.left" => a.alignment = Some(0),
        "text.align.centeredHorizontally" => a.alignment = Some(1),
        "text.align.right" => a.alignment = Some(2),
        "text.align.justified" => a.alignment = Some(3),
        "text.align.stretched" => a.alignment = Some(effective_alignment(4)),
        _ => a.size = Some(name.strip_prefix("text.size.")?.parse::<f32>().ok()?),
    }
    (a != TextAttrs::default()).then_some(a)
}

/// Paragraph separators of text given by scripts, MCP or CLI (TW-144, re 50 section 3.4): CRLF and
/// a single LF become CR (the separator stored in the text and in `TRNs`), like pasted text.
pub fn normalize_breaks(s: &str) -> String {
    if s.contains('\n') {
        s.replace("\r\n", "\r").replace('\n', "\r")
    } else {
        s.to_string()
    }
}

/// Parameter `runs` (Liste von `TextRun`); der Text der Läufe muss `text` ergeben. Paragraph
/// separators are normalized like `text` (see [`normalize_breaks`]).
pub(crate) fn parse_runs(p: &Params, text: &str) -> Result<Option<Vec<TextRun>>> {
    let Some(v) = p.get("runs") else {
        return Ok(None);
    };
    let bad = |m: String| EngineError::InvalidParams {
        name: p.command.clone(),
        message: m,
    };
    let mut runs: Vec<TextRun> =
        serde_json::from_value(v.clone()).map_err(|e| bad(e.to_string()))?;
    for r in &mut runs {
        r.text = normalize_breaks(&r.text);
    }
    let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
    if joined != text {
        return Err(bad("runs ≠ text".into()));
    }
    Ok(Some(runs))
}

fn text_info(t: &TextElem) -> serde_json::Value {
    let a = t
        .resolved_runs()
        .first()
        .map(|(a, _)| a.clone())
        .unwrap_or_else(|| t.initial.clone());
    serde_json::json!({
        "text": t.plain_text(), "font": a.font, "size": a.size, "bold": a.bold, "italic": a.italic, "underline": a.underline,
        "color": a.color.map(|c| super::argb_hex(fp_color::Argb(c))), "alignment": a.alignment,
        "frameType": t.frame_type, "antialias": t.antialias, "autoKern": t.auto_kern, "orientation": t.orientation,
    })
}

fn p_font(a: &TextAttrs) -> String {
    a.font.clone().unwrap_or_default()
}
