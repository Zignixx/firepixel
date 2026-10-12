//! Menü „Text“: Schrift, Größe, Stil, Ausrichtung (`JSTi`, 09 §3.5), Text an/in Pfad, Richtung,
//! sowie Textinhalt und -attribute für Texteditor und Eigenschaften-Inspektor.

use super::{selected_elements, sel_err, target_ids, with_doc};
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

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(cmd("text.font").requires(TEXT_SEL).param("font", ParamKind::String, "Schriftname (PostScript- oder Familienname)").run(|s, p| {
        let f = p.str("font")?;
        edit_texts(s, move |t| {
            let f = f.clone();
            set_attr(t, &move |a| a.font = Some(f.clone()));
            t.initial.ps_font = Some(p_font(&t.initial));
        })
    }));
    r.add(cmd("text.size.other").requires(TEXT_SEL).param("size", ParamKind::Number { min: Some(1.0), max: Some(5000.0) }, "Größe in px").run(|s, p| {
        let v = p.f64("size")? as f32;
        edit_texts(s, move |t| set_attr(t, &move |a| a.size = Some(v)))
    }));
    for &v in SIZES {
        let name = format!("text.size.{}", v as i32);
        r.add(cmd(&name).requires(TEXT_SEL).run(move |s, _| edit_texts(s, move |t| set_attr(t, &move |a| a.size = Some(v)))));
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
    r.add(cmd("text.style.plain").requires(TEXT_SEL).run(|s, _| {
        edit_texts(s, |t| {
            set_attr(t, &|a| {
                a.bold = Some(false);
                a.italic = Some(false);
                a.underline = Some(false);
            })
        })
    }));
    type Get = fn(&TextAttrs) -> Option<bool>;
    type Set = fn(&mut TextAttrs, bool);
    let styles: [(&str, Get, Set); 3] = [
        ("text.style.fauxBold", |a| a.bold, |a, v| a.bold = Some(v)),
        ("text.style.fauxItalic", |a| a.italic, |a, v| a.italic = Some(v)),
        ("text.style.underline", |a| a.underline, |a, v| a.underline = Some(v)),
    ];
    for (name, get, set) in styles {
        r.add(
            cmd(name)
                .requires(TEXT_SEL)
                .checked(move |s| {
                    s.active().is_some_and(|d| {
                        selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.resolved_runs().first().and_then(|(a, _)| get(a)).unwrap_or(false)))
                    })
                })
                .run(move |s, _| {
                    edit_texts(s, move |t| {
                        let on = !t.resolved_runs().first().and_then(|(a, _)| get(a)).unwrap_or(false);
                        set_attr(t, &move |a| set(a, on));
                    })
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
                .requires(TEXT_SEL)
                .checked(move |s| {
                    s.active().is_some_and(|d| selected_elements(d).iter().any(|(_, e)| matches!(&e.kind, ElementKind::Text(t) if t.resolved_runs().first().and_then(|(a, _)| a.alignment) == Some(v))))
                })
                .run(move |s, _| edit_texts(s, move |t| set_attr(t, &move |a| a.alignment = Some(v)))),
        );
    }
    r.add(cmd("text.attachToPath").requires(&[Requirement::Document, Requirement::MinSelected(2)]).run(|s, _| attach(s, false)));
    r.add(cmd("text.attachInPath").requires(&[Requirement::Document, Requirement::MinSelected(2)]).run(|s, _| attach(s, true)));
    r.add(cmd("text.detachFromPath").requires(TEXT_SEL).run(|s, _| {
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
                let doc = Arc::make_mut(&mut d.doc);
                docops::edit_by_id(doc, page, e.id, |x| {
                    if let ElementKind::Text(t) = &mut x.kind {
                        t.on_path = None;
                        t.frame_type = Some(0);
                        t.detach_story();
                    }
                });
                sel.push(e.id);
                if let Some(pth) = path {
                    let n = docops::reassign(doc, &pth);
                    sel.push(n.id);
                    if let Some(cur) = docops::find_on_page(doc, page, e.id)
                        && let Some((list, idx)) = docops::parent_list_mut(doc, &cur)
                    {
                        list.insert(idx + 1, Arc::new(n));
                    }
                }
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
    r.add(cmd("text.reverseDirection").requires(TEXT_SEL).run(|s, _| {
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
            .opt("antialias", ParamKind::Enum { values: vec!["none".into(), "crisp".into(), "strong".into(), "smooth".into(), "custom".into(), "system".into()] }, "Glättung")
            .opt("sharpness", ParamKind::Integer { min: Some(0), max: Some(255) }, "Schärfe (benutzerdefiniert)")
            .opt("strength", ParamKind::Integer { min: Some(0), max: Some(255) }, "Stärke (benutzerdefiniert)")
            .opt("oversample", ParamKind::Integer { min: Some(1), max: Some(64) }, "Überabtastung (benutzerdefiniert)")
            .opt("autoKern", ParamKind::Bool, "automatische Unterschneidung")
            .opt("kerning", ParamKind::Number { min: None, max: None }, "Unterschneidung (Tausendstel em) für alle Zeichen")
            .opt("orientation", ParamKind::Enum { values: vec!["horizontalLtr".into(), "verticalRtl".into(), "horizontalRtl".into(), "verticalLtr".into()] }, "Textrichtung (ORIi 0–3)")
            .run(|s, p| {
                let frame = p.opt_str("frameType")?;
                let width = p.opt_f64("width")?;
                let aa = p.opt_str("antialias")?.map(|n| match n.as_str() {
                    "none" => -1,
                    "crisp" => 1,
                    "strong" => 2,
                    "system" => 3,
                    "custom" => 4,
                    _ => 0,
                });
                let (sharp, strength, over) = (p.opt_i64("sharpness")?, p.opt_i64("strength")?, p.opt_i64("oversample")?);
                let auto_kern = p.opt_bool("autoKern")?;
                let kerning = p.opt_f64("kerning")?.map(|v| v as f32);
                let ori = p.opt_str("orientation")?.map(|n| match n.as_str() {
                    "verticalRtl" => 1,
                    "horizontalRtl" => 2,
                    "verticalLtr" => 3,
                    _ => 0,
                });
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
                                    let edge = if v < 0 { 0 } else { 1 };
                                    if v >= 0 {
                                        t.antialias = v;
                                    }
                                    let at = Arc::make_mut(&mut t.attrs);
                                    if let Some(f) = at.fill.as_mut() {
                                        f.edge = edge;
                                    }
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
                    Some(p)
                }
                _ => {
                    let mut g = Element::new_group(&mut ids, paths);
                    g.common = e.common.clone();
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
