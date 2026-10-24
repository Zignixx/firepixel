//! Befehle der Leinwand-Werkzeuge (Phase 4): Pfadpunkte einfügen/entfernen/löschen, Messer,
//! Verlaufsgriffe und die Retusche-Werkzeuge (`bitmap.retouch`). Wie alle Befehle landen sie im
//! Verlauf und sind für MCP/JSF aufrufbar.

use super::bitmap::{BREAKS_PARAM_TEXT, Dab, dab_points};
use super::select::bitmap_for;
use super::{DOC, sel_err, take_ids, target_ids, with_doc};
use crate::command::{CommandOutput, CommandRegistry, ParamKind, Params, cmd};
use crate::docops::{self, bitmap_image, set_bitmap_image};
use crate::error::{EngineError, Result};
use crate::geometry::{self, BasicGeometry};
use crate::selection::PointRef;
use crate::tools::pathedit::{knife_params, split_at};
use fp_color::Rgba8;
use fp_geom::Point;
use fp_model::element::ElementKind;
use fp_model::{Contour, ElementId};
use fp_raster::RgbaImage;
use serde_json::json;
use std::sync::Arc;

/// Parameterfehler; `m` = Schlüssel `err-param-…` (fremde Texte bleiben unverändert).
fn bad(p: &Params, m: &str) -> EngineError {
    EngineError::InvalidParams {
        name: p.command.clone(),
        message: if m.starts_with("err-") {
            fp_i18n::t(m)
        } else {
            m.into()
        },
    }
}

/// Ändert die Konturen eines Pfads (Grundform wird zum Pfad).
fn edit_contours(
    d: &mut crate::session::DocumentState,
    id: ElementId,
    f: impl FnOnce(&mut Vec<Contour>) -> Result<()>,
) -> Result<()> {
    let page = d.page;
    let mut res = Ok(());
    docops::edit_by_id(Arc::make_mut(&mut d.doc), page, id, |e| {
        if let ElementKind::Path(pe) = &mut e.kind {
            res = f(&mut pe.contours);
            pe.primitive = None;
            pe.polyline = None;
        } else {
            res = Err(EngineError::Selection(fp_i18n::t("err-sel-no-path")));
        }
    })
    .ok_or(EngineError::ElementNotFound(id.0))?;
    res
}

/// Führt einen registrierten Befehl innerhalb eines anderen aus (ohne eigenen Verlaufsschritt).
fn run_inner(
    s: &mut crate::session::Session,
    name: &str,
    params: serde_json::Value,
) -> Result<CommandOutput> {
    let reg = Arc::clone(s.registry());
    let c = reg
        .get(name)
        .ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
    let p = Params::new(name, params);
    crate::command::validate(c.info(), &p)?;
    c.apply(s, &p)
}

/// Legt die Werkzeugattribute (Füllung wie `pi.fill`, Strich wie `pi.stroke`, Pinsel) auf das neue
/// Objekt `id` – ohne eigenen Verlaufsschritt; offene Pfade bekommen keine Füllung.
pub(crate) fn apply_tool_style(
    s: &mut crate::session::Session,
    id: u64,
    open: bool,
    fill: Option<&serde_json::Value>,
    stroke: Option<&serde_json::Value>,
    brush: Option<&serde_json::Value>,
) -> Result<()> {
    if let Some(f) = fill.filter(|v| v.is_object())
        && !open
    {
        let mut f = f.clone();
        f["ids"] = json!([id]);
        drop_unknown_texture(&mut f);
        run_inner(s, "pi.fill", f)?;
    }
    if let Some(st) = stroke.filter(|v| v.is_object()) {
        let mut st = st.clone();
        st["ids"] = json!([id]);
        drop_unknown_texture(&mut st);
        run_inner(s, "pi.stroke", st)?;
    }
    if let Some(bv) = brush.filter(|v| v.is_object()) {
        let brush: fp_model::Brush =
            serde_json::from_value(bv.clone()).map_err(|e| EngineError::InvalidParams {
                name: "brush".into(),
                message: format!("brush: {e}"),
            })?;
        with_doc(s, |d| {
            let page = d.page;
            docops::edit_by_id(Arc::make_mut(&mut d.doc), page, ElementId(id), |e| {
                docops::edit_path_attrs(e, &|a| {
                    if a.stroke_color.a() > 0 || a.brush.is_some() {
                        a.brush = Some(brush.clone());
                    }
                });
            });
            Ok(())
        })?;
    }
    Ok(())
}

/// Zeichenbefehle, die `tool.shape` kapseln darf.
const SHAPE_COMMANDS: [&str; 5] = [
    "draw.rectangle",
    "draw.ellipse",
    "draw.polygon",
    "draw.line",
    "draw.path",
];

/// Auswahlbefehle, die `tool.pick` ohne Verlaufsschritt ausführen darf.
const PICK_COMMANDS: [&str; 5] = [
    "select.set",
    "select.points",
    "select.inRect",
    "select.inPolygon",
    "select.pointAt",
];

pub(super) fn register(r: &mut CommandRegistry) {
    // Auswählen mit der Maus (Zeiger, Teilauswahl, Hintere auswählen, Gummiband, Klick ins Leere):
    // im Original kein Verlaufsschritt – nur die Menübefehle sind welche
    r.add(
        cmd("tool.pick")
            .label("Auswählen (Maus)")
            .describe(
                "Auswahl durch ein Werkzeug ändern, ohne Verlaufsschritt (wie Klicks im Original)",
            )
            .requires(DOC)
            .param(
                "command",
                ParamKind::Enum {
                    values: PICK_COMMANDS.iter().map(|s| s.to_string()).collect(),
                },
                "Auswahlbefehl",
            )
            .opt("params", ParamKind::Json, "Parameter des Auswahlbefehls")
            .run(|s, p| {
                let c = p.str("command")?;
                let before = s.active().map(|d| d.selection.elements.clone());
                let out = run_inner(s, &c, p.get("params").cloned().unwrap_or_else(|| json!({})))?;
                // RE spec 64 §3 rule 4: one selected object makes its layer current, no History row
                let moved = match (before, s.active_mut()) {
                    (Some(b), Some(d)) if b != d.selection.elements => {
                        crate::session::follow_single_selection_now(d)
                    }
                    _ => false,
                };
                if let Some(id) = s.active_id() {
                    s.emit(crate::session::SessionEvent::SelectionChanged(id));
                    if moved {
                        s.emit(crate::session::SessionEvent::LocationChanged(id));
                    }
                }
                Ok(CommandOutput {
                    effect: crate::command::Effect::None,
                    ..out
                })
            }),
    );
    // Firepixel extension: lasso marquee of the Pointer and the Subselection (Properties
    // inspector, tool row of these two tools)
    r.add(
        cmd("tool.pointerLasso")
            .label("Lasso-Auswahlrahmen")
            .describe("Zeiger und Teilauswahl: Ziehen auf leerer Fläche zeichnet statt eines Rechtecks eine freie Lasso-Kontur (Firepixel-Erweiterung, Voreinstellung Firepixel_PointerLasso)")
            .opt("on", ParamKind::Bool, "ein/aus (Vorgabe: umschalten)")
            .checked(|s| s.prefs.get_bool(crate::tools::POINTER_LASSO_PREF, false))
            .run(|s, p| {
                let key = crate::tools::POINTER_LASSO_PREF;
                let v = match p.opt_bool("on")? {
                    Some(v) => v,
                    None => !s.prefs.get_bool(key, false),
                };
                s.prefs.set(key, crate::prefs::PrefValue::Bool(v));
                s.prefs_changed();
                Ok(CommandOutput::session_with(json!({ "on": v })))
            }),
    );
    // Leinwand-Kontextmenüs: Einträge mit eigener ID
    r.add(
        cmd("view.exitFullScreen")
            .label("Vollbildmodus beenden")
            .fw_id(35589)
            .describe("Bildschirmmodus auf Standard zurücksetzen (Kontextmenü der Leinwand)")
            // im Menü immer sichtbar, nur im Vollbildmodus frei (CT-A-13, gemessen re 29 §2: im Normalmodus grau)
            .menu_when(|s| s.ui.screen_mode != 0)
            .run(|s, _| {
                s.ui.screen_mode = 0;
                s.emit(crate::session::SessionEvent::PanelsChanged);
                Ok(CommandOutput::session_with(json!({"mode": 0})))
            }),
    );
    for (id, name, label, op) in [
        (49111u32, "path.combine.union", "Vereinigung", 1),
        (49112, "path.combine.intersect", "Überschneiden", 2),
        (49113, "path.combine.punch", "Ausstanzen", 3),
        (49114, "path.combine.crop", "Zuschneiden", 4),
        (49115, "path.combine.normal", "Normal", 0),
    ] {
        r.add(
            cmd(name)
                .label(label)
                .fw_id(id)
                .describe("Pfad kombinieren (Kontextmenü): Pfadoperation der zusammengesetzten Form wie pi.compound.setOp")
                .requires(&[crate::command::Requirement::Document, crate::command::Requirement::Selection])
                .run(move |s, _| run_inner(s, "pi.compound.setOp", json!({"op": op}))),
        );
    }
    r.add(
        cmd("tool.shape")
            .label("Form zeichnen")
            .describe("Zeichnet mit einem draw.*-Befehl und wendet die Werkzeugattribute des Inspektors (Füllung wie pi.fill, Strich wie pi.stroke) auf das neue Objekt an – ein Verlaufsschritt")
            .requires(DOC)
            .param("draw", ParamKind::Enum { values: SHAPE_COMMANDS.iter().map(|s| s.to_string()).collect() }, "Zeichenbefehl")
            .param("params", ParamKind::Json, "Parameter des Zeichenbefehls")
            .opt("fill", ParamKind::Json, "Parameter für pi.fill (ohne ids)")
            .opt("stroke", ParamKind::Json, "Parameter für pi.stroke (ohne ids)")
            .opt("brush", ParamKind::Json, "vollständiger Pinsel der Werkzeugvorgabe (fp_model::Brush als JSON)")
            .run(|s, p| {
                let draw = p.str("draw")?;
                let params = p.get("params").cloned().unwrap_or_else(|| json!({}));
                let out = run_inner(s, &draw, params)?;
                let id = out.value.get("id").and_then(|v| v.as_u64()).ok_or_else(|| EngineError::Failed(fp_i18n::t("err-no-new-object")))?;
                let open = draw == "draw.line" || (draw == "draw.path" && !p.get("params").and_then(|v| v.get("contours")).and_then(|c| c.as_array()).is_some_and(|a| a.iter().any(|c| c.get("closed").and_then(|b| b.as_bool()) == Some(true))));
                apply_tool_style(s, id, open, p.get("fill"), p.get("stroke"), p.get("brush")).map_err(|e| match e {
                    EngineError::InvalidParams { message, .. } => bad(p, &message),
                    e => e,
                })?;
                // Auswahl bleibt beim neuen Objekt
                with_doc(s, |d| {
                    d.selection.set(vec![ElementId(id)]);
                    Ok(())
                })?;
                // Verlaufsschritt wie im Original nach der Form („Rechteck“, nicht „Form zeichnen“, PAR 18/20b)
                let key = fp_i18n::command_key(&draw);
                let step = format!("{key}.step");
                let out = CommandOutput { value: json!({"id": id}), ..CommandOutput::doc() };
                Ok(if fp_i18n::has(&step) {
                    out.labeled(fp_i18n::t(&step))
                } else if fp_i18n::has(&key) {
                    out.labeled(fp_i18n::t(&key))
                } else {
                    out
                })
            }),
    );
    r.add(
        cmd("path.scrub")
            .label("Pfadausbesserung")
            .describe("Druckwerte der Pfadpunkte entlang eines Strichs erhöhen (amount > 0) bzw. senken (amount < 0)")
            .requires(DOC)
            .param("points", ParamKind::Json, "Strich [[x, y], …]")
            .param("amount", ParamKind::Number { min: Some(-1.0), max: Some(1.0) }, "Änderung des Drucks je Zug")
            .opt_default("size", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, json!(30), "Durchmesser")
            .opt("ids", ParamKind::Ids, "Pfade (Vorgabe: Auswahl)")
            .run(|s, p| {
                let pts = p.points("points")?;
                let amount = p.f64("amount")? as f32;
                let r = (p.f64_or("size", 30.0)? / 2.0) as f32;
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    let mut any = false;
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            if let ElementKind::Path(pe) = &mut e.kind {
                                any |= crate::tools::pathedit::scrub_pressure(&mut pe.contours, &pts, r, amount);
                            }
                        });
                    }
                    if !any {
                        return Err(EngineError::Failed(fp_i18n::t("err-stroke-misses-path")));
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("transform.nineSlice")
            .label("9-Slice-Skalierung")
            .describe("Auswahl mit 9-Slice-Hilfslinien skalieren: Ecken behalten ihre Größe, Kanten und Mitte werden gestreckt (Pfade punktweise, andere Objekte affin)")
            .requires(DOC)
            .param("rect", ParamKind::Rect, "Ausgangsrahmen")
            .param("to", ParamKind::Rect, "Zielrahmen")
            .opt("insets", ParamKind::Json, "feste Ränder [links, oben, rechts, unten] in px (Vorgabe: je ein Drittel)")
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .run(|s, p| {
                let from = p.rect("rect")?.normalized();
                let to = p.rect("to")?.normalized();
                let insets: [f32; 4] = match p.get("insets").and_then(|v| v.as_array()) {
                    Some(a) if a.len() == 4 => {
                        let v: Vec<f32> = a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect();
                        [v[0], v[1], v[2], v[3]]
                    }
                    _ => crate::tools::pathedit::default_insets(&from),
                };
                if from.width() <= 0.0 || from.height() <= 0.0 || to.width() <= 0.0 || to.height() <= 0.0 {
                    return Err(bad(p, "err-param-empty-rect"));
                }
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| nine_slice_element(e, &from, insets, &to, 0)).ok_or(EngineError::ElementNotFound(id.0))?;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.insertNode")
            .label("Punkt einfügen")
            .describe("Stift auf einem Pfadsegment: Punkt einfügen, Form bleibt (de Casteljau)")
            .requires(DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Pfad",
            )
            .param(
                "contour",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Kontur",
            )
            .param(
                "segment",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Segment",
            )
            .param(
                "t",
                ParamKind::Number {
                    min: Some(0.0),
                    max: Some(1.0),
                },
                "Parameter auf dem Segment",
            )
            .run(|s, p| {
                let id = ElementId(p.i64("id")? as u64);
                let (ci, seg, t) = (p.usize("contour")?, p.usize("segment")?, p.f64("t")?);
                with_doc(s, |d| {
                    edit_contours(d, id, |cs| {
                        let c = cs
                            .get_mut(ci)
                            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-contour")))?;
                        let bc = geometry::to_bezier(c)
                            .insert_node(seg, t)
                            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-segment")))?;
                        *c = geometry::from_bezier(&bc);
                        Ok(())
                    })?;
                    d.selection.set(vec![id]);
                    d.selection.points = vec![PointRef {
                        element: id,
                        contour: ci,
                        node: seg + 1,
                    }];
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.removeNode")
            .label("Punkt entfernen")
            .requires(DOC)
            .param(
                "id",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Pfad",
            )
            .param(
                "contour",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Kontur",
            )
            .param(
                "node",
                ParamKind::Integer {
                    min: Some(0),
                    max: None,
                },
                "Punkt",
            )
            .run(|s, p| {
                let id = ElementId(p.i64("id")? as u64);
                let (ci, ni) = (p.usize("contour")?, p.usize("node")?);
                with_doc(s, |d| {
                    let mut empty = false;
                    edit_contours(d, id, |cs| {
                        let c = cs
                            .get_mut(ci)
                            .ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-contour")))?;
                        if ni >= c.nodes.len() {
                            return Err(EngineError::NotFound(fp_i18n::t("err-nf-point")));
                        }
                        c.nodes.remove(ni);
                        if c.nodes.len() < 3 {
                            c.closed = false;
                        }
                        if c.nodes.len() < 2 {
                            cs.remove(ci);
                        }
                        empty = cs.is_empty();
                        Ok(())
                    })?;
                    if empty {
                        docops::remove_ids(Arc::make_mut(&mut d.doc), d.page, &[id]);
                        d.selection.clear();
                    }
                    d.selection.points.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.deletePoints")
            .label("Punkte löschen")
            .describe("Unterausgewählte Pfadpunkte löschen (Teilauswahl + Entf)")
            .requires(DOC)
            .available(|s| s.active().is_some_and(|d| !d.selection.points.is_empty()))
            .run(|s, _| {
                with_doc(s, |d| {
                    let mut pts = d.selection.points.clone();
                    pts.sort();
                    let mut by_el: Vec<(ElementId, Vec<(usize, usize)>)> = Vec::new();
                    for r in pts {
                        match by_el.iter_mut().find(|(e, _)| *e == r.element) {
                            Some((_, v)) => v.push((r.contour, r.node)),
                            None => by_el.push((r.element, vec![(r.contour, r.node)])),
                        }
                    }
                    let mut remove = Vec::new();
                    for (id, mut list) in by_el {
                        list.sort();
                        list.dedup();
                        let mut empty = false;
                        edit_contours(d, id, |cs| {
                            for (ci, ni) in list.into_iter().rev() {
                                if let Some(c) = cs.get_mut(ci)
                                    && ni < c.nodes.len()
                                {
                                    c.nodes.remove(ni);
                                }
                            }
                            for c in cs.iter_mut() {
                                if c.nodes.len() < 3 {
                                    c.closed = false;
                                }
                            }
                            cs.retain(|c| c.nodes.len() >= 2);
                            empty = cs.is_empty();
                            Ok(())
                        })?;
                        if empty {
                            remove.push(id);
                        }
                    }
                    if !remove.is_empty() {
                        let page = d.page;
                        docops::remove_ids(Arc::make_mut(&mut d.doc), page, &remove);
                        d.selection.elements.retain(|e| !remove.contains(e));
                    }
                    d.selection.points.clear();
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.knife")
            .label("Messer")
            .describe(
                "Ausgewählte Pfade entlang einer Schnittlinie teilen (knifeElementsFromPoints)",
            )
            .requires(DOC)
            .param("points", ParamKind::Json, "Schnittlinie [[x, y], …]")
            .opt("ids", ParamKind::Ids, "Pfade (Vorgabe: Auswahl)")
            .run(|s, p| {
                let line = p.points("points")?;
                if line.len() < 2 {
                    return Err(bad(p, "err-param-two-points"));
                }
                with_doc(s, |d| {
                    let ids = target_ids(d, p)?;
                    let page = d.page;
                    let mut new_sel = Vec::new();
                    let mut cut_any = false;
                    for id in ids {
                        let Some(e) = docops::element_on_page(&d.doc, page, id) else {
                            continue;
                        };
                        let ElementKind::Path(pe) = &e.kind else {
                            new_sel.push(id);
                            continue;
                        };
                        // jede Kontur zerschneiden; Stücke werden eigene Pfade
                        let mut pieces: Vec<Contour> = Vec::new();
                        let mut cut = false;
                        for c in &pe.contours {
                            let cuts = knife_params(c, &line);
                            if cuts.is_empty() {
                                pieces.push(c.clone());
                            } else {
                                cut = true;
                                pieces.extend(split_at(c, &cuts));
                            }
                        }
                        if !cut {
                            new_sel.push(id);
                            continue;
                        }
                        cut_any = true;
                        let doc = Arc::make_mut(&mut d.doc);
                        let mut idg = take_ids(doc);
                        let mut elems = Vec::new();
                        for (k, c) in pieces.into_iter().enumerate() {
                            let mut ne = (*e).clone();
                            if k > 0 {
                                ne.id = idg.element();
                            }
                            if let ElementKind::Path(np) = &mut ne.kind {
                                np.contours = vec![c];
                                np.primitive = None;
                                np.polyline = None;
                            }
                            new_sel.push(ne.id);
                            elems.push(Arc::new(ne));
                        }
                        doc.ids = idg;
                        if let Some(path) = docops::find_on_page(doc, page, id)
                            && let Some((list, idx)) = docops::parent_list_mut(doc, &path)
                        {
                            list.splice(idx..=idx, elems);
                        }
                    }
                    if !cut_any {
                        return Err(EngineError::Failed(fp_i18n::t("err-cut-misses-path")));
                    }
                    d.selection.set(new_sel);
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("element.setFillHandles")
            .label("Verlaufsgriffe verschieben")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .param("start", ParamKind::Point, "Anfang")
            .param("end", ParamKind::Point, "Ende")
            .opt(
                "second",
                ParamKind::Point,
                "zweite Achse (Vorgabe: senkrecht zu Anfang→Ende)",
            )
            .run(|s, p| {
                let (a, b) = (p.point("start")?, p.point("end")?);
                let second = p
                    .opt_point("second")?
                    .unwrap_or(Point::new(a.x - (b.y - a.y), a.y + (b.x - a.x)));
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            let h = fp_model::FillHandles {
                                valid: true,
                                start: a,
                                end: b,
                                second,
                            };
                            match &mut e.kind {
                                ElementKind::Path(pe) => pe.fill_handles = h,
                                ElementKind::Text(t) => t.fill_handles = h,
                                _ => {}
                            }
                        })
                        .ok_or(EngineError::ElementNotFound(id.0))?;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("element.rotateFillHandles")
            .label("Verlaufsgriffe drehen")
            .requires(DOC)
            .opt("ids", ParamKind::Ids, "Elemente (Vorgabe: Auswahl)")
            .opt_default(
                "axis",
                ParamKind::Integer {
                    min: Some(1),
                    max: Some(2),
                },
                json!(1),
                "gezogene Achse: 1 = Anfang–Ende, 2 = zweite Achse",
            )
            .param(
                "toward",
                ParamKind::Point,
                "Richtung der Achse relativ zum Anfang",
            )
            .run(|s, p| {
                // RE spec 69 §3: a drag on the line h1–h2 / h1–h3 with the Pointer turns both arms
                // rigidly about h1 (rows "rotate1" / "rotate2" of the reference)
                let toward = p.point("toward")?;
                let axis = p.i64_or("axis", 1)?.clamp(1, 2) as u8;
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            let h = match &mut e.kind {
                                ElementKind::Path(pe) => &mut pe.fill_handles,
                                ElementKind::Text(t) => &mut t.fill_handles,
                                _ => return,
                            };
                            if h.valid {
                                *h = crate::commands::panel::rotate_fill_handles(*h, axis, toward);
                            }
                        })
                        .ok_or(EngineError::ElementNotFound(id.0))?;
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("bitmap.retouch")
            .label("Retusche")
            .describe("Pinselbasierte Bitmapwerkzeuge: Weichzeichner, Scharfzeichner, Abwedler, Nachbelichter, Verwischen, Stempel, Farbe ersetzen, Rote Augen")
            .requires(DOC)
            .param(
                "mode",
                ParamKind::Enum {
                    values: ["blur", "sharpen", "dodge", "burn", "smudge", "stamp", "replaceColor", "redEye"].iter().map(|s| s.to_string()).collect(),
                },
                "Werkzeug",
            )
            .param("points", ParamKind::Json, "Strich [[x, y], …]")
            .opt_default("size", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, json!(13), "Durchmesser")
            .opt_default("round", ParamKind::Bool, json!(true), "runde Spitze")
            .opt_default("soft", ParamKind::Number { min: Some(0.0), max: Some(1.0) }, json!(1.0), "weiche Kante 0..1")
            .opt_default("strength", ParamKind::Number { min: Some(0.0), max: Some(100.0) }, json!(50), "Intensität/Belichtung/Druck in %")
            .opt_default("range", ParamKind::Integer { min: Some(0), max: Some(2) }, json!(1), "Bereich: 0 Tiefen, 1 Mitteltöne, 2 Lichter")
            .opt_default("spacing", ParamKind::Number { min: Some(1.0), max: Some(1000.0) }, json!(20), "Abstand der Abdrücke in % des Durchmessers")
            .opt("offset", ParamKind::Json, "Stempel: Versatz Quelle − Strich [dx, dy]")
            .opt("from", ParamKind::Color, "Farbe ersetzen: Von-Farbe")
            .opt("to", ParamKind::Color, "Farbe ersetzen: In-Farbe")
            .opt_default("colorize", ParamKind::Bool, json!(false), "Farbe ersetzen: Kolorieren")
            .opt_default("tolerance", ParamKind::Integer { min: Some(0), max: Some(255) }, json!(50), "Toleranz")
            .opt_default("fingerPaint", ParamKind::Bool, json!(false), "Verwischen: Farbe verwischen (Farbe `color` einmischen)")
            .opt("color", ParamKind::Color, "Verwischen: Farbe für „Farbe verwischen“ (Vorgabe: Füllfarbe)")
            .opt_default("wholeDocument", ParamKind::Bool, json!(false), "Verwischen: Gesamtes Dokument (alle sichtbaren Objekte und die Leinwand aufnehmen)")
            .opt_default("confineToBounds", ParamKind::Bool, json!(false), "Verwischen: Auf Grenzen beschränken (aus = das Bitmap wächst, wo der Strich über den Rand hinausläuft)")
            .opt("replaceStrength", ParamKind::Integer { min: Some(0), max: Some(255) }, "Farbe ersetzen: Stärke 0..255 (Vorgabe 200)")
            .opt("fromImage", ParamKind::Bool, "Farbe ersetzen: Von-Farbe aus der Grafik unter dem Startpunkt")
            .opt("breaks", ParamKind::Json, BREAKS_PARAM_TEXT)
            .run(|s, p| {
                let mode = p.str("mode")?;
                let pts = p.points("points")?;
                if pts.is_empty() {
                    return Err(bad(p, "err-param-no-points"));
                }
                if mode == "replaceColor" || mode == "redEye" {
                    return replace_or_red_eye(s, p, &mode, &pts);
                }
                let prm = retouch_params(p, &mode, s.tools.fill_color.to_rgba8())?;
                // „Gesamtes Dokument“: Verwischen nimmt die sichtbare Seite samt Leinwand auf
                let whole = mode == "smudge" && p.bool_or("wholeDocument", false)?;
                let rendered = if whole {
                    let d = s.doc()?;
                    s.renderer().and_then(|r| r.render_page(&d.doc, d.page, d.frame))
                } else {
                    None
                };
                // Verlaufsschritt mit dem Namen des Werkzeugs (Original: „Rückgängig: Weichzeichner“ …)
                let label = match mode.as_str() {
                    "blur" => Some("tool-blur"),
                    "sharpen" => Some("tool-sharpen"),
                    "dodge" => Some("tool-dodge"),
                    "burn" => Some("tool-burn"),
                    "smudge" => Some("tool-smudge"),
                    _ => None,
                };
                with_doc(s, |d| {
                    let (path, target) = bitmap_for(d, pts.first().copied(), &BasicGeometry).ok_or_else(|| sel_err("err-sel-no-bitmap-under-stroke"))?;
                    let sel = d.selection.pixels.clone().filter(|_| d.selection.has_pixels());
                    // FB-39: the tool's live preview already computed exactly this stroke
                    let done = if whole { None } else { super::retouch_stroke::take_live_result(&target, &pts, &prm, sel.as_ref()) };
                    // ohne Renderer (headless): Leinwandfarbe mit der Bitmap darüber
                    let page_img = if whole { rendered.clone().or_else(|| canvas_with_bitmap(&d.doc, d.page, &target)) } else { None };
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_at(doc, &path, |e| {
                        if let ElementKind::Bitmap(b) = &mut e.kind {
                            if let Some(img) = done.as_ref() {
                                set_bitmap_image(b, img);
                            } else if let Some(mut img) = bitmap_image(b) {
                                retouch_dispatch(&mut img, &pts, &prm, sel.as_ref(), page_img.as_ref());
                                set_bitmap_image(b, &img);
                            }
                        }
                    });
                    let out = CommandOutput::doc();
                    Ok(match label {
                        Some(k) => out.labeled(fp_i18n::t(k)),
                        None if mode == "stamp" => out.labeled(fp_i18n::t("msg-step-stamp")),
                        None => out,
                    })
                })
            }),
    );
}

/// Werkzeugvorgabe mit einer Textur, die es auf diesem Rechner nicht gibt (z. B. aus einer Datei
/// übernommen): Zeichnen geht trotzdem, nur ohne den unbekannten Namen (`pi.fill` meldet ihn sonst).
fn drop_unknown_texture(v: &mut serde_json::Value) {
    if let Some(o) = v.as_object_mut()
        && o.get("texture")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|t| !super::panel::texture_name_known(t))
    {
        o.remove("texture");
        o.remove("textureAmount");
    }
}

/// Retouch parameters of `bitmap.retouch` (every mode except red eye and replace colour);
/// `fill` is the colour of "Smudge color" when `color` is missing.
fn retouch_params(p: &Params, mode: &str, fill: Rgba8) -> Result<Retouch> {
    Ok(Retouch {
        mode: mode.to_string(),
        size: p.f64_or("size", 13.0)? as f32,
        round: p.bool_or("round", true)?,
        soft: p.f64_or("soft", 1.0)?.clamp(0.0, 1.0) as f32,
        strength: (p.f64_or("strength", 50.0)?.clamp(0.0, 100.0) / 100.0) as f32,
        range: p.i64_or("range", 1)?,
        spacing: p.f64_or("spacing", 20.0)? as f32,
        offset: p
            .get("offset")
            .and_then(crate::command::parse_point)
            .map(|q| (q.x, q.y))
            .unwrap_or((0.0, 0.0)),
        from: p
            .opt_color("from")?
            .map(|c| c.to_rgba8())
            .unwrap_or(Rgba8::new(0, 255, 0, 255)),
        to: p
            .opt_color("to")?
            .map(|c| c.to_rgba8())
            .unwrap_or(Rgba8::new(255, 0, 0, 255)),
        colorize: p.bool_or("colorize", false)?,
        tolerance: p.i64_or("tolerance", 50)?.clamp(0, 255) as f32,
        finger: if p.bool_or("fingerPaint", false)? {
            Some(p.opt_color("color")?.map(|c| c.to_rgba8()).unwrap_or(fill))
        } else {
            None
        },
        confine: p.bool_or("confineToBounds", false)?,
        breaks: crate::paint::parse_breaks(p.get("breaks")),
    })
}

/// Runs one retouch stroke on `img` (the computation of `bitmap.retouch` for the brush modes):
/// Blur, Sharpen, Dodge, Burn and Smudge with the incremental stroke engine (the same code as the
/// live preview, FB-39), the stamp dab by dab.
fn retouch_dispatch(
    img: &mut RgbaImage,
    pts: &[Point],
    prm: &Retouch,
    sel: Option<&fp_raster::Mask>,
    page: Option<&RgbaImage>,
) {
    let base = Arc::new(std::mem::replace(img, RgbaImage::empty_at(0, 0)));
    match super::retouch_stroke::RetouchStroke::new(
        Arc::clone(&base),
        prm.clone(),
        sel.cloned().map(Arc::new),
        page.cloned().map(Arc::new),
    ) {
        Some(mut st) => {
            // FB-31: the engine starts a new part at every break of a resumed stroke (dodge, burn,
            // blur and sharpen work the whole stroke once, smudge starts afresh at each press)
            st.feed(pts, &prm.breaks);
            *img = st.finish();
        }
        None => {
            *img = Arc::unwrap_or_clone(base);
            // FB-31: the stamp carries pixels along the path, each segment starts afresh
            for seg in crate::paint::split_segments(pts, &prm.breaks) {
                retouch(img, seg, prm, sel);
            }
        }
    }
}

/// Live stroke of the retouch tools (FB-39) for the parameters `params` the tool sends on release:
/// Blur, Sharpen, Dodge, Burn and Smudge on the whole target bitmap `base`; `None` for the other
/// modes (they use [`retouch_preview`]) and for "Entire document" without its page.
pub(crate) fn retouch_live_stroke(
    base: Arc<RgbaImage>,
    params: &serde_json::Value,
    sel: Option<Arc<fp_raster::Mask>>,
    page: Option<Arc<RgbaImage>>,
) -> Option<super::retouch_stroke::RetouchStroke> {
    let p = Params::new("bitmap.retouch", params.clone());
    let mode = p.str("mode").ok()?;
    if !matches!(
        mode.as_str(),
        "blur" | "sharpen" | "dodge" | "burn" | "smudge"
    ) {
        return None;
    }
    let whole = mode == "smudge" && p.bool_or("wholeDocument", false).unwrap_or(false);
    if whole && page.is_none() {
        return None;
    }
    let prm = retouch_params(&p, &mode, Rgba8::new(0, 0, 0, 255)).ok()?;
    super::retouch_stroke::RetouchStroke::new(base, prm, sel, page.filter(|_| whole))
}

/// Source page of "Entire document" (smudge): the rendered page with its canvas, or without a
/// renderer (headless) the canvas colour with the target bitmap on top. Rendered once per stroke
/// (by the command, and by the live preview at the press).
pub(crate) fn smudge_page_source(
    doc: &fp_model::Document,
    page: usize,
    frame: usize,
    renderer: Option<&dyn crate::render::PageRenderer>,
    target: &fp_model::Element,
) -> Option<RgbaImage> {
    renderer
        .and_then(|r| r.render_page(doc, page, frame))
        .or_else(|| canvas_with_bitmap(doc, page, target))
}

/// "Replace Color" parameters of `bitmap.retouch` and whether the source colour comes from the
/// image under the start point (`fromImage`).
fn replace_params(p: &Params) -> Result<(super::retouch_px::ReplaceParams, bool)> {
    Ok((
        super::retouch_px::ReplaceParams {
            size: p.f64_or("size", 13.0)? as f32,
            round: p.bool_or("round", true)?,
            from: p
                .opt_color("from")?
                .map(|c| c.to_rgba8())
                .unwrap_or(Rgba8::new(0, 255, 0, 255)),
            to: p
                .opt_color("to")?
                .map(|c| c.to_rgba8())
                .unwrap_or(Rgba8::new(255, 0, 0, 255)),
            tolerance: p.i64_or("tolerance", 32)?.clamp(0, 255) as f32,
            strength: p.i64_or("replaceStrength", 200)?.clamp(0, 255) as f32,
            colorize: p.bool_or("colorize", false)?,
        },
        p.bool_or("fromImage", false)?,
    ))
}

/// "From: Image": the source colour is the pixel under the start point (opaque).
fn replace_from_image(prm: &mut super::retouch_px::ReplaceParams, img: &RgbaImage, start: Point) {
    if let Some(q) = img.get(start.x.floor() as i32, start.y.floor() as i32) {
        prm.from = Rgba8::new(q[0], q[1], q[2], 255);
    }
}

/// Live preview of a retouch stroke without a command (FB-14a, like the stamp preview PAR 05/04):
/// the same computation as `bitmap.retouch` with the parameters `params` the tool sends on release,
/// applied to `img` (a copy of the target bitmap or a part of it around the stroke). `page` = the source page of "Entire document"
/// ([`smudge_page_source`], rendered once at the press). Returns `false` when the mode has no live
/// preview (red eye; smudge over the whole document without a page).
pub(crate) fn retouch_preview(
    img: &mut RgbaImage,
    pts: &[Point],
    params: &serde_json::Value,
    sel: Option<&fp_raster::Mask>,
    page: Option<&RgbaImage>,
) -> bool {
    let p = Params::new("bitmap.retouch", params.clone());
    let Ok(mode) = p.str("mode") else {
        return false;
    };
    let Some(&start) = pts.first() else {
        return false;
    };
    match mode.as_str() {
        "redEye" => false,
        "replaceColor" => {
            let Ok((mut prm, from_image)) = replace_params(&p) else {
                return false;
            };
            if from_image {
                replace_from_image(&mut prm, img, start);
            }
            let breaks = crate::paint::parse_breaks(p.get("breaks"));
            for seg in crate::paint::split_segments(pts, &breaks) {
                super::retouch_px::replace_color(img, seg, &prm, sel);
            }
            true
        }
        _ => {
            let whole = mode == "smudge" && p.bool_or("wholeDocument", false).unwrap_or(false);
            if whole && page.is_none() {
                return false;
            }
            let Ok(prm) = retouch_params(&p, &mode, Rgba8::new(0, 0, 0, 255)) else {
                return false;
            };
            retouch_dispatch(img, pts, &prm, sel, page.filter(|_| whole));
            true
        }
    }
}

/// `bitmap.retouch` für „Farbe ersetzen“ (Strich) und „Rote Augen entfernen“ (Rechteck aus erstem
/// und letztem Punkt), gemessen am Original (`retouch_px`). Verlaufsschritt „Farbe ersetzen“ bzw.
/// „Rote Augen entfernen“; ein Rote-Augen-Rechteck ohne Wirkung legt wie im Original „Bitmap
/// bearbeiten“ an.
fn replace_or_red_eye(
    s: &mut crate::session::Session,
    p: &Params,
    mode: &str,
    pts: &[Point],
) -> Result<CommandOutput> {
    use super::retouch_px::{RedEyeParams, red_eye, replace_color};
    let tol = p
        .i64_or("tolerance", if mode == "redEye" { 50 } else { 32 })?
        .clamp(0, 255) as f32;
    let strength = p.f64_or("strength", 50.0)?.clamp(0.0, 100.0) as f32;
    let replace = if mode == "redEye" {
        None
    } else {
        Some(replace_params(p)?)
    };
    let prefs = s.prefs.clone();
    let breaks = crate::paint::parse_breaks(p.get("breaks"));
    let mut changed = false;
    with_doc(s, |d| {
        let (path, _) = bitmap_for(d, pts.first().copied(), &BasicGeometry)
            .ok_or_else(|| sel_err("err-sel-no-bitmap-under-stroke"))?;
        let sel = d
            .selection
            .pixels
            .clone()
            .filter(|_| d.selection.has_pixels());
        let doc = Arc::make_mut(&mut d.doc);
        docops::edit_at(doc, &path, |e| {
            if let ElementKind::Bitmap(b) = &mut e.kind
                && let Some(mut img) = bitmap_image(b)
            {
                changed = if mode == "redEye" {
                    let (a, z) = (pts[0], pts[pts.len() - 1]);
                    let (l, r) = (a.x.min(z.x).round() as i32, a.x.max(z.x).round() as i32);
                    let (t, bt) = (a.y.min(z.y).round() as i32, a.y.max(z.y).round() as i32);
                    let prm = RedEyeParams {
                        rect: fp_geom::IRect::new(l, t, r, bt),
                        tolerance: tol,
                        strength,
                        min_hue: prefs.get_real("RedEyeRemovalToolMinHueAngle", 300.0) as f32,
                        max_hue: prefs.get_real("RedEyeRemovalToolMaxHueAngle", 5.0) as f32,
                        min_sat: prefs.get_int("RedEyeRemovalToolMinSaturationPercentage", 50)
                            as f32
                            / 100.0,
                        min_lum: prefs.get_int("RedEyeRemovalToolMinLuminancePercentage", 25)
                            as f32
                            / 100.0,
                    };
                    red_eye(&mut img, &prm, sel.as_ref())
                } else if let Some((rp, from_image)) = &replace {
                    let mut prm = rp.clone();
                    if *from_image {
                        replace_from_image(&mut prm, &img, pts[0]);
                    }
                    // FB-31: each segment of a resumed stroke in turn, one history step
                    let mut any = false;
                    for seg in crate::paint::split_segments(pts, &breaks) {
                        any |= replace_color(&mut img, seg, &prm, sel.as_ref());
                    }
                    any
                } else {
                    false
                };
                if changed {
                    set_bitmap_image(b, &img);
                }
            }
        });
        Ok(())
    })?;
    let out = CommandOutput::doc();
    Ok(match mode {
        "redEye" if !changed => out.labeled(fp_i18n::t("msg-step-edit-bitmap")),
        "redEye" => out.labeled(fp_i18n::t("msg-step-red-eye")),
        _ => out.labeled(fp_i18n::t("msg-step-replace-color")),
    })
}

/// 9-Slice eines Elements (Gruppen rekursiv; Pfade punktweise, Rechteck-Grundformen behalten die
/// Rundung in Pixeln; Bitmaps, Text und Symbole affin auf den Zielrahmen).
fn nine_slice_element(
    e: &mut fp_model::Element,
    from: &fp_geom::Rect,
    insets: [f32; 4],
    to: &fp_geom::Rect,
    depth: u32,
) {
    use crate::tools::pathedit::{nine_slice_contours, nine_slice_point};
    e.cache = None;
    match &mut e.kind {
        ElementKind::Path(pe) => {
            // Rechteck-Grundform ohne Drehung/Neigung: bleibt Grundform, der Eckenradius in Pixeln bleibt
            let axis = pe.primitive.as_ref().and_then(|pr| {
                let m = pr.transform.m;
                (m[0][1].abs() < 1e-6 && m[1][0].abs() < 1e-6)
                    .then(|| pr.transform.inverse())
                    .flatten()
            });
            if let (Some(inv), Some(pr)) = (axis, pe.primitive.as_mut()) {
                let m = pr.transform;
                let a = inv.apply(nine_slice_point(
                    m.apply(Point::new(pr.rect.left, pr.rect.top)),
                    from,
                    insets,
                    to,
                ));
                let b = inv.apply(nine_slice_point(
                    m.apply(Point::new(pr.rect.right, pr.rect.bottom)),
                    from,
                    insets,
                    to,
                ));
                let nr = fp_geom::Rect::new(a.x, a.y, b.x, b.y).normalized();
                let px = pr.roundness * pr.rect.width().abs().min(pr.rect.height().abs()) / 2.0;
                let half = (nr.width().min(nr.height()) / 2.0).max(1e-6);
                pr.rect = nr;
                pr.roundness = (px / half).clamp(0.0, 1.0);
                let mut c = geometry::rect_contour(nr, pr.roundness as f64);
                for n in &mut c.nodes {
                    n.anchor = m.apply(n.anchor);
                    n.pred = m.apply(n.pred);
                    n.succ = m.apply(n.succ);
                }
                pe.contours = vec![c];
            } else {
                pe.primitive = None;
                pe.contours = nine_slice_contours(&pe.contours, from, insets, to);
            }
            pe.polyline = None;
            let fh = &mut pe.fill_handles;
            fh.start = nine_slice_point(fh.start, from, insets, to);
            fh.end = nine_slice_point(fh.end, from, insets, to);
            fh.second = nine_slice_point(fh.second, from, insets, to);
        }
        ElementKind::Group(g) if depth < 32 => {
            for c in &mut g.children {
                nine_slice_element(fp_model::Element::edit(c), from, insets, to, depth + 1);
            }
        }
        _ => docops::transform_element(
            e,
            &docops::rect_to_rect(*from, *to),
            fp_raster::Sampler::Bicubic,
        ),
    }
}

/// Parameter der Retusche.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Retouch {
    pub(crate) mode: String,
    pub(crate) size: f32,
    pub(crate) round: bool,
    pub(crate) soft: f32,
    /// 0..1.
    pub(crate) strength: f32,
    pub(crate) range: i64,
    pub(crate) spacing: f32,
    pub(crate) offset: (f32, f32),
    pub(crate) from: Rgba8,
    pub(crate) to: Rgba8,
    pub(crate) colorize: bool,
    pub(crate) tolerance: f32,
    /// Verwischen: „Farbe verwischen“ mit dieser Farbe.
    pub(crate) finger: Option<Rgba8>,
    /// Smudge: "Confine to boundaries" (Firepixel option, `confineToBounds`).
    pub(crate) confine: bool,
    /// FB-31: indices into the points where a paused stroke was resumed (`breaks`).
    pub(crate) breaks: Vec<usize>,
}

fn rgb_to_hsv(p: [u8; 4]) -> (f32, f32, f32) {
    let (r, g, b) = (
        p[0] as f32 / 255.0,
        p[1] as f32 / 255.0,
        p[2] as f32 / 255.0,
    );
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let d = mx - mn;
    let h = if d <= 0.0 {
        0.0
    } else if mx == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if mx == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if mx > 0.0 { d / mx } else { 0.0 }, mx)
}

/// Verstärkung des Weichzeichners: Ein voller Abdruck mischt bei Intensität 100 % 35,3 % des
/// 3×3-Mittels ein (gemessen am Original: Intensität 50 an einer Farbkante → 15/255, an der
/// Bitmapkante → 30/255; FV-11, PARITAET_A L38).
pub(super) const BLUR_GAIN: f32 = 0.353;
/// Abstand der Abdrücke: `Durchmesser × Abstand % × Faktor` (an den Strichmessungen des Originals
/// kalibriert, Vorgabe 20 %: Weichzeichner 13 → ≈ 2 px, Stempel 16 → ≈ 2,1 px).
fn spacing_gain(mode: &str) -> f32 {
    if mode == "stamp" { 0.65 } else { 0.75 }
}
/// Farbe außerhalb der Bitmap beim Weichzeichnen: transparentes Weiß (das Original hellt die
/// Bitmapkante dabei auf und lässt die Bitmap wachsen: 1 px bei einem Klick, 2 px bei einem Zug,
/// RE spec 57 section 3.2).
pub(super) const OUTSIDE: [f32; 4] = [255.0, 255.0, 255.0, 0.0];

pub(super) fn in_rect(r: &fp_geom::IRect, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

/// Scharfzeichner: Verstärkung der Abweichung vom 3×3-Mittel je voller Abdruck bei Intensität
/// 100 % (Original Intensität 50: G 212 neben Schwarz +8, G 160 neben Schwarz +6 bzw. +12 bei
/// zwei Abdrücken; nur an Kanten, gesättigte Kanäle bleiben).
pub(super) const SHARPEN_DAB_GAIN: f32 = 0.226;

/// Gradationskurve von Abwedler (`dodge`) und Nachbelichter je Bereich und Kanal, `a` = Belichtung
/// 0..1. Gemessen am Original bei Belichtung 50 (PAR-X 04/06, 04/07): je Kanal getrennt, also ohne
/// Farbverschiebung. Abwedeln Schatten 0 → 63 (Richtung Weiß um 24,7 %), Mitteltöne als Gamma
/// (133 → 160, Schwarz/Weiß bleiben), Lichter × 1,346 (133 → 179); Nachbelichten spiegelbildlich:
/// Schatten 255 − (255 − c) × 1,346 (133 → 91), Mitteltöne Gamma 1,42 (133 → 101), Lichter × 0,75
/// (Weiß → 191).
pub(super) fn tone_curve(dodge: bool, range: i64, a: f32, c: f32) -> f32 {
    let x = (c / 255.0).clamp(0.0, 1.0);
    let v = match (dodge, range) {
        (true, 0) => x + (1.0 - x) * 0.494 * a,
        (false, 0) => 1.0 - (1.0 - x) * (1.0 + 0.692 * a),
        (true, 2) => x * (1.0 + 0.692 * a),
        (false, 2) => x * (1.0 - 0.5 * a),
        (true, _) => x.powf(1.0 / (1.0 + 0.786 * a)),
        (false, _) => x.powf(1.0 + 0.84 * a),
    };
    v.clamp(0.0, 1.0) * 255.0
}

/// Seite ohne Renderer: Leinwandfarbe mit der Zielbitmap darüber (für „Gesamtes Dokument“).
fn canvas_with_bitmap(
    doc: &fp_model::Document,
    page: usize,
    e: &fp_model::Element,
) -> Option<RgbaImage> {
    let pg = doc.pages.get(page)?;
    let r = fp_geom::IRect::new(
        0,
        0,
        pg.canvas.width.ceil() as i32,
        pg.canvas.height.ceil() as i32,
    );
    let bg = pg.root.background.to_rgba8();
    let mut out = RgbaImage::new(r).ok()?;
    for y in r.top..r.bottom {
        for x in r.left..r.right {
            out.set_pixel(x, y, bg);
        }
    }
    if let ElementKind::Bitmap(b) = &e.kind
        && let Some(img) = bitmap_image(b)
    {
        let opt = fp_raster::CompositeOptions {
            variant: fp_color::blend::OverVariant::Scalar,
            ..fp_raster::CompositeOptions::default()
        };
        fp_raster::composite(&mut out, &img, None, &opt);
    }
    Some(out)
}

/// Colour premultiplied by its alpha (0..255 channels); unchanged for opaque pixels.
pub(super) fn premul(p: [f32; 4]) -> [f32; 4] {
    if p[3] >= 255.0 {
        return p;
    }
    let a = p[3].max(0.0) / 255.0;
    [p[0] * a, p[1] * a, p[2] * a, p[3]]
}

/// Inverse of [`premul`]; fully transparent becomes transparent black.
pub(super) fn unpremul(q: [f32; 4]) -> [f32; 4] {
    if q[3] >= 255.0 {
        return q;
    }
    if q[3] <= 0.0 {
        return [0.0; 4];
    }
    let a = q[3] / 255.0;
    [q[0] / a, q[1] / a, q[2] / a, q[3]]
}

/// Wendet die Retusche entlang des Strichs auf das Bild an (in Dokumentkoordinaten). Jeder Abdruck
/// wirkt auf das Ergebnis der vorigen (wie im Original); gerechnet wird in Gleitkomma, gerundet erst
/// am Ende.
fn retouch(img: &mut RgbaImage, pts: &[Point], prm: &Retouch, sel: Option<&fp_raster::Mask>) {
    let dab = Dab::new(prm.size, prm.round, prm.soft * 100.0);
    let step = (prm.size * prm.spacing / 100.0 * spacing_gain(&prm.mode)).max(1.0);
    let mut stamps = dab_points(pts, step);
    // Stempel: der Zug reicht bis zum Endpunkt (letzter Abdruck dort, nicht einen Abstand davor;
    // gemessen PAR 05/03: Zug 330 → 390 endet im Original 2 px weiter als ohne End-Abdruck)
    if prm.mode == "stamp"
        && let (Some(&end), Some(&last)) = (pts.last(), stamps.last())
        && (end.x - last.x).hypot(end.y - last.y) > 0.01
    {
        stamps.push(end);
    }
    let reach = stamps
        .iter()
        .fold(fp_geom::IRect::default(), |r, s| r.union(&dab.bounds(*s)));
    let orig = img.bounds();
    let grows = prm.mode == "blur";
    if grows && !orig.is_empty() {
        // the blur may grow the bitmap by 2 px, new pixels transparent white (RE spec 57 section
        // 3, RE spec 70 section 1.9; the incremental stroke engine does the same)
        let ext = orig.union(&reach.intersect(&orig.inflate(2, 2)));
        if ext != orig
            && let Ok(mut e) = img.extended_to(ext)
        {
            for y in ext.top..ext.bottom {
                for x in ext.left..ext.right {
                    if !in_rect(&orig, x, y) {
                        e.set_pixel(x, y, Rgba8::new(255, 255, 255, 0));
                    }
                }
            }
            *img = e;
        }
    }
    // RE spec 70 section 1.9: the stamp lays the cloned pixels down also outside the selected
    // bitmap, which grows to what was painted (also a separate strip, the gap transparent); the
    // Firepixel option "Confine to boundaries" keeps it inside (FB-52)
    let stamp_grows = prm.mode == "stamp" && !prm.confine && !orig.is_empty();
    if stamp_grows
        && let Ok(e) = img.extended_to(orig.union(&reach))
    {
        *img = e;
    }
    let ib = img.bounds();
    let work = reach.inflate(1, 1).intersect(&ib);
    if work.is_empty() {
        return;
    }
    let ww = work.width() as usize;
    let idx = |x: i32, y: i32| (y - work.top) as usize * ww + (x - work.left) as usize;
    let mut buf: Vec<[f32; 4]> = Vec::with_capacity(ww * work.height() as usize);
    for y in work.top..work.bottom {
        for x in work.left..work.right {
            buf.push(
                img.get(x, y)
                    .map(|p| p.map(|v| v as f32))
                    .unwrap_or(OUTSIDE),
            );
        }
    }
    let source = img.clone();
    let read = |b: &[[f32; 4]], x: i32, y: i32| -> [f32; 4] {
        if in_rect(&work, x, y) {
            b[idx(x, y)]
        } else {
            source
                .get(x, y)
                .map(|p| p.map(|v| v as f32))
                .unwrap_or(OUTSIDE)
        }
    };
    let mix = |a: [f32; 4], b: [f32; 4], t: f32| -> [f32; 4] {
        [0, 1, 2, 3].map(|k| a[k] + (b[k] - a[k]) * t)
    };
    let lumf = |p: [f32; 4]| 0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2];
    let mut prev: Option<Point> = None;
    for s in stamps {
        let mut changes: Vec<(usize, [f32; 4])> = Vec::new();
        dab.each(s, |x, y, w0| {
            if !in_rect(&work, x, y) {
                return;
            }
            let mut w = w0 * prm.strength;
            if let Some(m) = sel {
                w *= m.get(x, y) as f32 / 255.0;
            }
            if w <= 0.0 {
                return;
            }
            let px = buf[idx(x, y)];
            // Änderungen werden erst nach dem Abdruck übernommen: gelesen wird der Stand davor
            let from: &[[f32; 4]] = &buf;
            let avg = || -> [f32; 4] {
                let mut acc = [0f32; 4];
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let q = read(from, x + dx, y + dy);
                        for k in 0..4 {
                            acc[k] += q[k];
                        }
                    }
                }
                acc.map(|v| v / 9.0)
            };
            let out = match prm.mode.as_str() {
                "blur" => mix(px, avg(), w * BLUR_GAIN),
                "sharpen" => {
                    let a = avg();
                    let target = [0, 1, 2].map(|k| px[k] + (px[k] - a[k]) * 1.5);
                    mix(px, [target[0], target[1], target[2], px[3]], w)
                }
                "dodge" | "burn" => {
                    let l = lumf(px) / 255.0;
                    let band = match prm.range {
                        0 => (1.0 - l).powi(2),
                        2 => l * l,
                        _ => 1.0 - (2.0 * l - 1.0).powi(2),
                    };
                    let t = w * 0.35 * band;
                    if prm.mode == "dodge" {
                        mix(px, [255.0, 255.0, 255.0, px[3]], t)
                    } else {
                        mix(px, [0.0, 0.0, 0.0, px[3]], t)
                    }
                }
                "smudge" => {
                    let Some(pv) = prev else { return };
                    let q = read(
                        from,
                        x - (s.x - pv.x).round() as i32,
                        y - (s.y - pv.y).round() as i32,
                    );
                    mix(px, q, w)
                }
                "stamp" => {
                    // Quelle über das Ziel legen (transparente Quelle lässt das Ziel stehen, gemessen)
                    let (sx, sy) = (
                        (x as f32 + prm.offset.0).floor() as i32,
                        (y as f32 + prm.offset.1).floor() as i32,
                    );
                    let Some(q) = source.get(sx, sy) else { return };
                    let sa = q[3] as f32 / 255.0 * w;
                    let da = px[3] / 255.0;
                    let oa = sa + da * (1.0 - sa);
                    if oa <= 0.0 {
                        return;
                    }
                    let c = |k: usize| (q[k] as f32 * sa + px[k] * da * (1.0 - sa)) / oa;
                    [c(0), c(1), c(2), oa * 255.0]
                }
                "replaceColor" => {
                    let f = prm.from;
                    let dist = ((px[0] - f.r as f32).abs()
                        + (px[1] - f.g as f32).abs()
                        + (px[2] - f.b as f32).abs())
                        / 3.0;
                    let tol = prm.tolerance.max(1.0);
                    if dist > tol {
                        return;
                    }
                    let t = w * (1.0 - dist / tol).max(0.2);
                    let tc = prm.to;
                    let target = if prm.colorize {
                        let l = lumf(px) / 255.0;
                        [
                            tc.r as f32 * l * 1.5,
                            tc.g as f32 * l * 1.5,
                            tc.b as f32 * l * 1.5,
                            px[3],
                        ]
                    } else {
                        // Abweichung von der Von-Farbe bleibt erhalten (Schattierung)
                        let (fr, tr) = ([f.r, f.g, f.b], [tc.r, tc.g, tc.b]);
                        let c = |k: usize| tr[k] as f32 + px[k] - fr[k] as f32;
                        [c(0), c(1), c(2), px[3]]
                    };
                    mix(px, target, t)
                }
                _ => {
                    // Rote Augen: Farbton 300°–5°, Sättigung ≥ 50 %, Helligkeit ≥ 25 % → Rotkanal absenken
                    let p8 = px.map(|v| v.round().clamp(0.0, 255.0) as u8);
                    let (h, sat, v) = rgb_to_hsv(p8);
                    if !(h >= 300.0 || h <= 5.0 + prm.tolerance / 10.0) || sat < 0.5 || v < 0.25 {
                        return;
                    }
                    let rr = (px[1] + px[2]) / 2.0;
                    mix(px, [rr, px[1], px[2], px[3]], (w * 2.0).min(1.0))
                }
            };
            changes.push((idx(x, y), out.map(|v| v.clamp(0.0, 255.0))));
        });
        for (i, v) in changes {
            buf[i] = v;
        }
        prev = Some(s);
    }
    for y in work.top..work.bottom {
        for x in work.left..work.right {
            let mut p = buf[idx(x, y)].map(|v| v.round().clamp(0.0, 255.0) as u8);
            if grows && !in_rect(&orig, x, y) {
                // neue Randpixel: Deckung abrunden, unter 2/255 verwerfen – an den Messungen des
                // Originals kalibriert (Strich 345 → 390 an B2: oben wächst die Bitmap, unten nicht)
                let a = buf[idx(x, y)][3];
                p[3] = if a < 2.0 {
                    0
                } else {
                    a.floor().clamp(0.0, 255.0) as u8
                };
            }
            img.set_pixel(x, y, Rgba8::new(p[0], p[1], p[2], p[3]));
        }
    }
    if (grows || stamp_grows) && !orig.is_empty() {
        // nur Ränder behalten, die wirklich Deckung bekommen haben
        let keep = orig.union(&img.alpha_bounds());
        if keep != img.bounds() {
            *img = img.cropped(keep);
        }
    }
}
