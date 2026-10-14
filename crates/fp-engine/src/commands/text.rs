//! Menü „Text“: Schrift, Größe, Stil, Ausrichtung (`JSTi`, 09 §3.5), Text an/in Pfad, Richtung,
//! sowie Textinhalt und -attribute für Texteditor und Eigenschaften-Inspektor.

use super::{selected_elements, sel_err, target_ids, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ObjectKind, ParamKind, Params, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::session::Session;
use fp_model::element::{Element, ElementKind};
use fp_model::text::{TextAttrs, TextElem, TextOnPath, TextRun};
use fp_model::Extra;
use std::sync::Arc;

const TEXT_SEL: &[Requirement] = &[Requirement::Document, Requirement::AnyOf(ObjectKind::Text)];

/// Menügrößen (Text ▸ Größe).
pub const SIZES: &[f32] = &[8.0, 9.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0, 48.0, 72.0, 96.0, 120.0];

/// Ändert alle ausgewählten Texte (Anfangsattribute und alle Läufe); löst die ATE-Story (09 §4.1).
fn edit_texts(s: &mut Session, f: impl Fn(&mut TextElem)) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let page = d.page;
        let ids: Vec<_> = selected_elements(d).into_iter().filter(|(_, e)| matches!(e.kind, ElementKind::Text(_))).map(|(_, e)| e.id).collect();
        if ids.is_empty() {
            return Err(sel_err("err-sel-no-text-selected"));
        }
        let doc = Arc::make_mut(&mut d.doc);
        for id in ids {
            docops::edit_by_id(doc, page, id, |e| {
                if let ElementKind::Text(t) = &mut e.kind {
                    f(t);
                    t.detach_story();
                }
            });
        }
        Ok(CommandOutput::doc())
    })
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
    s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(e.kind, ElementKind::Text(_))))
}

/// Ist ein Text auf Pfad ausgewählt?
fn has_text_on_path(s: &Session) -> bool {
    s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.on_path.is_some())))
}

/// Mit Textauswahl die Texte ändern, sonst die Vorgaben des Textwerkzeugs (wie das Original).
fn texts_or_defaults(s: &mut Session, on_text: impl Fn(&mut TextElem), on_defaults: impl FnOnce(&mut Session)) -> Result<CommandOutput> {
    if has_text(s) {
        return edit_texts(s, on_text);
    }
    on_defaults(s);
    s.prefs_changed();
    Ok(CommandOutput::session())
}

/// Familienname zu einem Schriftnamen (PostScript- oder Familienname); unbekannt → unverändert.
fn font_family_of(name: &str) -> String {
    let db = fp_text::FontDb::shared();
    match db.find_ps(name).and_then(|i| db.faces().get(i)) {
        Some(f) if !f.names.family.is_empty() => f.names.family.clone(),
        _ => name.to_string(),
    }
}

fn set_default_size(s: &mut Session, v: f32) {
    s.prefs.set("Text_Size", crate::prefs::PrefValue::Real(v as f64));
    s.tools.options.text_size = v;
}

/// Größe des ausgewählten Textes bzw. Vorgabe des Textwerkzeugs (Häkchen im Menü).
fn current_size(s: &Session) -> Option<f32> {
    let d = s.active()?;
    let sel = selected_elements(d).into_iter().find_map(|(_, e)| if let ElementKind::Text(t) = &e.kind { Some(first_size(t)) } else { None });
    Some(sel.unwrap_or(s.tools.options.text_size))
}

fn current_style(s: &Session, key: &str) -> bool {
    s.prefs.get_bool(key, false)
}

fn first_size(t: &TextElem) -> f32 {
    t.resolved_runs().first().and_then(|(a, _)| a.size).or(t.initial.size).unwrap_or(12.0)
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
        alignment: p.opt_i64("alignment")?.map(|v| v as i32),
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
    let t = els.iter().find(|(_, e)| matches!(e.kind, ElementKind::Text(_))).map(|(_, e)| Arc::clone(e));
    let p = els.iter().find(|(_, e)| matches!(e.kind, ElementKind::Path(_))).map(|(_, e)| Arc::clone(e));
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
                tx.on_path = Some(TextOnPath { mode: Some(0), offset: Some(0.0), path: Some(Arc::clone(&path)), extra: Extra::default() });
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
fn detached_text(doc: &fp_model::Document, t: &TextElem, center: Option<(f32, f32)>, r: Option<&dyn crate::render::PageRenderer>) -> TextElem {
    let mut n = t.clone();
    n.on_path = None;
    n.frame_type = Some(1);
    n.detach_story();
    let user_w = t.anchor_rect.map(|a| a.right - a.left).filter(|w| *w >= 1.0);
    let w = user_w.unwrap_or(t.bounds.right - t.bounds.left).max(1.0);
    let a0 = fp_geom::Rect::new(0.0, 0.0, w, 1.0);
    n.anchor_rect = Some(a0);
    // Satz am Ursprung: Höhe und Lage des Satzrahmens relativ zum Benutzerrahmen
    let f = r.and_then(|r| r.text_frame(doc, &n)).unwrap_or(fp_geom::Rect::new(0.0, 0.0, w, t.bounds.bottom - t.bounds.top));
    let h = (f.bottom - f.top).max(1.0);
    let half_up = |v: f32| (v + 0.5).floor();
    let (cx, cy) = center.unwrap_or(((t.bounds.left + t.bounds.right) * 0.5, (t.bounds.top + t.bounds.bottom) * 0.5));
    let (left, top) = (half_up(cx - (f.right - f.left) * 0.5), half_up(cy - h * 0.5));
    let (dx, dy) = (left - f.left, top - f.top);
    n.anchor_rect = Some(fp_geom::Rect::new(a0.left + dx, a0.top + dy, a0.left + dx + w, a0.top + dy + h));
    n.bounds = f.translate(dx, dy);
    n
}

/// Namen der Glättung (JS `antiAliasMode` bzw. „none“ = keine Kantenglättung).
pub(super) fn antialias_names() -> Vec<String> {
    ["none", "crisp", "strong", "smooth", "custom", "system"].iter().map(|s| s.to_string()).collect()
}

/// Namen der Textrichtung (`ORIi` 0–3).
pub(super) fn orientation_names() -> Vec<String> {
    ["horizontalLtr", "verticalRtl", "horizontalRtl", "verticalLtr"].iter().map(|s| s.to_string()).collect()
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
    let aa = if prefs.get_bool("Text_AntiAliasedFill", true) { prefs.get_int("Text_AntiAliasLevel", 0).clamp(0, 4) as i32 } else { -1 };
    (aa, prefs.get_bool("Text_AutoKern", false), prefs.get_int("Text_Orientation", 0).clamp(0, 3) as i32)
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("text.setToolDefaults")
            .label("Textwerkzeug-Vorgaben")
            .opt("antialias", ParamKind::Enum { values: antialias_names() }, "Glättung neuer Texte")
            .opt("autoKern", ParamKind::Bool, "automatische Unterschneidung neuer Texte")
            .opt("orientation", ParamKind::Enum { values: orientation_names() }, "Textrichtung neuer Texte")
            .run(|s, p| {
                use crate::prefs::PrefValue;
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
                    s.prefs.set("Text_Orientation", PrefValue::Int(orientation_code(&n) as i64));
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
    r.add(cmd("text.font").requires(DOC).param("font", ParamKind::String, "Schriftname (PostScript- oder Familienname)").run(|s, p| {
        let f = p.str("font")?;
        let family = font_family_of(&f);
        texts_or_defaults(
            s,
            move |t| {
                let f = f.clone();
                set_attr(t, &move |a| a.font = Some(f.clone()));
                t.initial.ps_font = Some(p_font(&t.initial));
            },
            move |s| {
                // Ausdrücklich speichern, auch wenn es die Werksvorgabe (Arial) ist: `prefs.set`
                // ließe den Schlüssel dann weg, und nach einem Neustart gälte wieder die Vorzugsschrift
                // des Textwerkzeugs statt der gewählten (Befund MN-2, NZ-3)
                s.prefs.values.insert("Text_Font".to_string(), crate::prefs::PrefValue::Str(family.clone()));
                s.tools.options.text_font = family;
            },
        )
    }));
    // Größe/Stil/Ausrichtung sind im Original immer aktiv: ohne ausgewählten Text setzen sie die
    // Vorgaben des Textwerkzeugs (`Text_Size`, `Text_Bold` …).
    r.add(cmd("text.size.other").requires(DOC).param("size", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Größe in px").run(|s, p| {
        let v = p.f64("size")? as f32;
        texts_or_defaults(s, move |t| set_attr(t, &move |a| a.size = Some(v)), move |s| set_default_size(s, v))
    }));
    for &v in SIZES {
        let name = format!("text.size.{}", v as i32);
        r.add(
            cmd(&name)
                .requires(DOC)
                .checked(move |s| current_size(s) == Some(v))
                .run(move |s, _| texts_or_defaults(s, move |t| set_attr(t, &move |a| a.size = Some(v)), move |s| set_default_size(s, v))),
        );
    }
    for (name, up) in [("text.size.larger", true), ("text.size.smaller", false)] {
        r.add(cmd(name).requires(TEXT_SEL).run(move |s, _| {
            edit_texts(s, move |t| {
                let cur = first_size(t);
                let next = if up {
                    SIZES.iter().copied().find(|&x| x > cur).unwrap_or(cur * 1.2)
                } else {
                    SIZES.iter().rev().copied().find(|&x| x < cur).unwrap_or((cur / 1.2).max(1.0))
                };
                set_attr(t, &move |a| a.size = Some(next));
            })
        }));
    }
    r.add(
        cmd("text.style.plain")
            .requires(DOC)
            .checked(|s| ["Text_Bold", "Text_Italic", "Text_Underline"].iter().all(|k| !current_style(s, k)))
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
        ("text.style.fauxBold", "Text_Bold", |a| a.bold, |a, v| a.bold = Some(v)),
        ("text.style.fauxItalic", "Text_Italic", |a| a.italic, |a, v| a.italic = Some(v)),
        ("text.style.underline", "Text_Underline", |a| a.underline, |a, v| a.underline = Some(v)),
    ];
    for (name, key, get, set) in styles {
        r.add(
            cmd(name)
                .requires(DOC)
                .checked(move |s| {
                    if has_text(s) {
                        s.active().is_some_and(|d| {
                            selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.resolved_runs().first().and_then(|(a, _)| get(a)).unwrap_or(false)))
                        })
                    } else {
                        current_style(s, key)
                    }
                })
                .run(move |s, _| {
                    texts_or_defaults(
                        s,
                        move |t| {
                            let on = !t.resolved_runs().first().and_then(|(a, _)| get(a)).unwrap_or(false);
                            set_attr(t, &move |a| set(a, on));
                        },
                        move |s| {
                            let on = !current_style(s, key);
                            s.prefs.set(key, crate::prefs::PrefValue::Bool(on));
                        },
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
                        s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.resolved_runs().first().and_then(|(a, _)| a.alignment).unwrap_or(0) == v)))
                    } else {
                        s.prefs.get_int("Text_Justification", 0) as i32 == v
                    }
                })
                .run(move |s, _| {
                    texts_or_defaults(s, move |t| set_attr(t, &move |a| a.alignment = Some(v)), move |s| s.prefs.set("Text_Justification", crate::prefs::PrefValue::Int(v as i64)))
                }),
        );
    }
    // wie im Original nur bei Text **und** Pfad bzw. bei Text auf Pfad
    r.add(cmd("text.attachToPath").requires(&[Requirement::Document, Requirement::MinSelected(2)]).available(|s| text_and_path(s).is_ok()).run(|s, _| attach(s, false)));
    r.add(cmd("text.attachInPath").requires(&[Requirement::Document, Requirement::MinSelected(2)]).available(|s| text_and_path(s).is_ok()).run(|s, _| attach(s, true)));
    r.add(cmd("text.detachFromPath").requires(TEXT_SEL).available(has_text_on_path).run(|s, _| {
        let renderer = s.renderer().cloned();
        with_doc(s, |d| {
            let page = d.page;
            let texts: Vec<_> = selected_elements(d).into_iter().filter(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.on_path.is_some())).collect();
            if texts.is_empty() {
                return Err(sel_err("err-sel-no-text-on-path"));
            }
            let mut sel = Vec::new();
            for (_, e) in texts {
                let ElementKind::Text(t) = &e.kind else { continue };
                let path = t.on_path.as_ref().and_then(|o| o.path.clone());
                let center = path.as_ref().and_then(|p| docops::element_bounds(&d.doc, p)).map(|b| ((b.left + b.right) * 0.5, (b.top + b.bottom) * 0.5));
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
    }));
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
                    s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.on_path.as_ref().and_then(|o| o.mode) == Some(mode))))
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
    r.add(cmd("text.reverseDirection").requires(TEXT_SEL).available(has_text_on_path).run(|s, _| {
        edit_texts(s, |t| {
            if let Some(o) = &mut t.on_path
                && let Some(pe) = &mut o.path
            {
                let e = Element::edit(pe);
                if let ElementKind::Path(p) = &mut e.kind {
                    p.contours = p.contours.iter().map(|c| crate::geometry::from_bezier(&crate::geometry::to_bezier(c).reversed())).collect();
                }
            }
        })
    }));
    r.add(
        cmd("text.setText")
            .label("Text bearbeiten")
            .requires(crate::commands::DOC)
            .opt("id", ParamKind::Integer { min: Some(0), max: None }, "Textobjekt (Vorgabe: Auswahl)")
            .param("text", ParamKind::String, "neuer Text (Absätze mit \\r)")
            .opt("runs", ParamKind::Json, "formatierte Läufe [{attrs, text}] (Text-Editor); Vorgabe: ein Lauf mit den Attributen des ersten")
            .run(|s, p| {
                let txt = p.str("text")?;
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
                                }
                            }
                        }
                    })
                    .ok_or(EngineError::ElementNotFound(id.0))?;
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("text.convertToPaths").requires(TEXT_SEL).js("convertToPaths").run(|s, _| convert_to_paths(s)));
    r.add(
        cmd("text.editor")
            .label("Texteditor")
            .requires(TEXT_SEL)
            .opt("text", ParamKind::String, "neuer Text (Absätze mit \\r); ohne Parameter: Inhalt lesen")
            .opt("font", ParamKind::String, "Schrift")
            .opt("size", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Größe")
            .opt("bold", ParamKind::Bool, "Faux Fett")
            .opt("italic", ParamKind::Bool, "Faux Kursiv")
            .opt("underline", ParamKind::Bool, "unterstrichen")
            .opt("color", ParamKind::Color, "Textfarbe")
            .run(|s, p| {
                let d = s.doc()?;
                let (_, e) = selected_elements(d).into_iter().find(|(_, e)| matches!(e.kind, ElementKind::Text(_))).ok_or_else(|| sel_err("err-sel-no-text-selected"))?;
                let changes = ["text", "font", "size", "bold", "italic", "underline", "color"].iter().any(|k| p.has(k));
                if !changes {
                    let ElementKind::Text(t) = &e.kind else { return Err(sel_err("err-sel-no-text")) };
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
                                let attrs = t.runs.first().map(|r| r.attrs.clone()).unwrap_or_else(|| t.initial.clone());
                                t.runs = vec![TextRun { attrs, text: txt.clone(), extra: Extra::default() }];
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
                    let info = docops::element_on_page(doc, page, id).and_then(|e| if let ElementKind::Text(t) = &e.kind { Some(text_info(t)) } else { None });
                    Ok(CommandOutput::doc_with(info.unwrap_or_default()).labeled(fp_i18n::t("msg-step-edit-text")))
                })
            }),
    );
    r.add(
        cmd("text.setProperties")
            .label("Texteigenschaften")
            .requires(crate::commands::DOC)
            .opt("ids", ParamKind::Ids, "Textobjekte (Vorgabe: Auswahl)")
            .opt("frameType", ParamKind::Enum { values: vec!["auto".into(), "fixed".into()] }, "Rahmenart: automatisch (Punkttext) oder feste Breite")
            .opt("width", ParamKind::Number { min: Some(1.0), max: None }, "Rahmenbreite bei fester Breite")
            .opt("antialias", ParamKind::Enum { values: antialias_names() }, "Glättung")
            .opt("sharpness", ParamKind::Integer { min: Some(0), max: Some(255) }, "Schärfe (benutzerdefiniert)")
            .opt("strength", ParamKind::Integer { min: Some(0), max: Some(255) }, "Stärke (benutzerdefiniert)")
            .opt("oversample", ParamKind::Integer { min: Some(1), max: Some(64) }, "Überabtastung (benutzerdefiniert)")
            .opt("autoKern", ParamKind::Bool, "automatische Unterschneidung")
            .opt("kerning", ParamKind::Number { min: None, max: None }, "Unterschneidung (Tausendstel em) für alle Zeichen")
            .opt("orientation", ParamKind::Enum { values: orientation_names() }, "Textrichtung (ORIi 0–3)")
            .run(|s, p| {
                let frame = p.opt_str("frameType")?;
                let width = p.opt_f64("width")?;
                let aa = p.opt_str("antialias")?.map(|n| antialias_code(&n));
                let (sharp, strength, over) = (p.opt_i64("sharpness")?, p.opt_i64("strength")?, p.opt_i64("oversample")?);
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
                                        let w = width.map(|w| w as f32).unwrap_or(t.bounds.right - t.bounds.left).max(1.0);
                                        let r = t.anchor_rect.unwrap_or(t.bounds);
                                        t.anchor_rect = Some(fp_geom::Rect::new(r.left, r.top, r.left + w, r.bottom.max(r.top + 1.0)));
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
        cmd("text.setAttributes")
            .label("Textattribute")
            .requires(crate::commands::DOC)
            .opt("ids", ParamKind::Ids, "Textobjekte (Vorgabe: Auswahl)")
            .opt("font", ParamKind::String, "Schrift")
            .opt("size", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Größe")
            .opt("bold", ParamKind::Bool, "Faux Fett")
            .opt("italic", ParamKind::Bool, "Faux Kursiv")
            .opt("underline", ParamKind::Bool, "unterstrichen")
            .opt("color", ParamKind::Color, "Textfarbe")
            .opt("alignment", ParamKind::Integer { min: Some(0), max: Some(0x14) }, "JSTi")
            .opt("leading", ParamKind::Number { min: Some(0.0), max: None }, "Zeilenabstand")
            .opt("leadingMode", ParamKind::Integer { min: Some(0), max: Some(1) }, "0 Prozent, 1 exakt")
            .opt("tracking", ParamKind::Number { min: None, max: None }, "Laufweite")
            .opt("baselineShift", ParamKind::Number { min: None, max: None }, "Grundlinienversatz")
            .opt("horizontalScale", ParamKind::Number { min: Some(0.01), max: None }, "horizontale Skalierung (1 = 100 %)")
            .opt("indent", ParamKind::Number { min: None, max: None }, "Einzug")
            .opt("spaceBefore", ParamKind::Number { min: None, max: None }, "Abstand vor")
            .opt("spaceAfter", ParamKind::Number { min: None, max: None }, "Abstand nach")
            .run(|s, p| {
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
            }),
    );
}

/// Text ▸ In Pfade konvertieren: jedes Textobjekt wird eine Gruppe aus Pfaden (ein Pfad je
/// Zeichen, Unterstreichungen als eigene Pfade) mit der Textfarbe; Name, Deckkraft, Mischmodus und
/// Effekte gehen auf die Gruppe über. Bei nur einer Glyphe entsteht ein einzelner Pfad.
fn convert_to_paths(s: &mut Session) -> Result<CommandOutput> {
    let renderer = s.renderer().cloned().ok_or(EngineError::BackendMissing("err-backend-text-renderer"))?;
    with_doc(s, |d| {
        let page = d.page;
        let texts: Vec<_> = selected_elements(d).into_iter().filter(|(_, e)| matches!(e.kind, ElementKind::Text(_))).collect();
        if texts.is_empty() {
            return Err(sel_err("err-sel-no-text-selected"));
        }
        let mut new_sel: Vec<fp_model::ElementId> = d.selection.elements.iter().copied().filter(|id| !texts.iter().any(|(_, e)| e.id == *id)).collect();
        for (_, e) in texts {
            let ElementKind::Text(t) = &e.kind else { continue };
            let outlines = renderer.text_outlines(&d.doc, t).ok_or(EngineError::BackendMissing("err-backend-text-layout"))?;
            let color = t.resolved_runs().first().and_then(|(a, _)| a.color).map(fp_color::Argb).unwrap_or(t.attrs.fill_color);
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
                paths.push(Arc::new(Element::new_path(&mut ids, contours, attrs.clone())));
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
            let Some(path) = docops::find_on_page(doc, page, e.id) else { continue };
            let Some((list, idx)) = docops::parent_list_mut(doc, &path) else { continue };
            match replacement {
                // Ergebnis mit neuer ID (die Text-ID verschwindet wie im Original)
                Some(r) => {
                    new_sel.push(r.id);
                    list[idx] = Arc::new(r);
                }
                None => {
                    list.remove(idx);
                }
            }
        }
        d.selection.set(new_sel);
        Ok(CommandOutput::doc())
    })
}

/// Menü-/Inspektor-Befehl als Attributänderung für den offenen Text-Editor (`cur` = Attribute an
/// der Markierung): `text.setAttributes` (ohne `ids`), `text.font`, `text.size.*`, `text.style.*`,
/// `text.align.*` (horizontal). `None` = kein Textattribut-Befehl.
pub(crate) fn editor_patch(name: &str, params: &serde_json::Value, cur: &TextAttrs) -> Option<TextAttrs> {
    if params.get("ids").is_some_and(|v| !v.is_null()) {
        return None;
    }
    let p = Params::new(name, params.clone());
    let mut a = TextAttrs::default();
    match name {
        "text.setAttributes" => a = attrs_from_params(&p).ok()?,
        "text.font" => a.font = Some(p.str("font").ok()?),
        "text.size.other" => a.size = Some(p.f64("size").ok()? as f32),
        "text.size.larger" | "text.size.smaller" => {
            let c = cur.size.unwrap_or(12.0);
            a.size = Some(if name.ends_with("larger") {
                SIZES.iter().copied().find(|&x| x > c).unwrap_or(c * 1.2)
            } else {
                SIZES.iter().rev().copied().find(|&x| x < c).unwrap_or((c / 1.2).max(1.0))
            });
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
        "text.align.stretched" => a.alignment = Some(4),
        _ => a.size = Some(name.strip_prefix("text.size.")?.parse::<f32>().ok()?),
    }
    (a != TextAttrs::default()).then_some(a)
}

/// Parameter `runs` (Liste von `TextRun`); der Text der Läufe muss `text` ergeben.
pub(crate) fn parse_runs(p: &Params, text: &str) -> Result<Option<Vec<TextRun>>> {
    let Some(v) = p.get("runs") else { return Ok(None) };
    let bad = |m: String| EngineError::InvalidParams { name: p.command.clone(), message: m };
    let runs: Vec<TextRun> = serde_json::from_value(v.clone()).map_err(|e| bad(e.to_string()))?;
    let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
    if joined != text {
        return Err(bad("runs ≠ text".into()));
    }
    Ok(Some(runs))
}

fn text_info(t: &TextElem) -> serde_json::Value {
    let a = t.resolved_runs().first().map(|(a, _)| a.clone()).unwrap_or_else(|| t.initial.clone());
    serde_json::json!({
        "text": t.plain_text(), "font": a.font, "size": a.size, "bold": a.bold, "italic": a.italic, "underline": a.underline,
        "color": a.color.map(|c| super::argb_hex(fp_color::Argb(c))), "alignment": a.alignment,
        "frameType": t.frame_type, "antialias": t.antialias, "autoKern": t.auto_kern, "orientation": t.orientation,
    })
}

fn p_font(a: &TextAttrs) -> String {
    a.font.clone().unwrap_or_default()
}
