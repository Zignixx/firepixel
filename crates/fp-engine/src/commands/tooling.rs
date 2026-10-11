//! Befehle der Leinwand-Werkzeuge (Phase 4): Pfadpunkte einfügen/entfernen/löschen, Messer,
//! Verlaufsgriffe und die Retusche-Werkzeuge (`bitmap.retouch`). Wie alle Befehle landen sie im
//! Verlauf und sind für MCP/JSF aufrufbar.

use super::select::bitmap_for;
use super::{sel_err, take_ids, target_ids, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Params};
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
    EngineError::InvalidParams { name: p.command.clone(), message: if m.starts_with("err-") { fp_i18n::t(m) } else { m.into() } }
}

/// Ändert die Konturen eines Pfads (Grundform wird zum Pfad).
fn edit_contours(d: &mut crate::session::DocumentState, id: ElementId, f: impl FnOnce(&mut Vec<Contour>) -> Result<()>) -> Result<()> {
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
fn run_inner(s: &mut crate::session::Session, name: &str, params: serde_json::Value) -> Result<CommandOutput> {
    let reg = Arc::clone(s.registry());
    let c = reg.get(name).ok_or_else(|| EngineError::UnknownCommand(name.into()))?;
    let p = Params::new(name, params);
    crate::command::validate(c.info(), &p)?;
    c.apply(s, &p)
}

/// Zeichenbefehle, die `tool.shape` kapseln darf.
const SHAPE_COMMANDS: [&str; 5] = ["draw.rectangle", "draw.ellipse", "draw.polygon", "draw.line", "draw.path"];

/// Auswahlbefehle, die `tool.pick` ohne Verlaufsschritt ausführen darf.
const PICK_COMMANDS: [&str; 4] = ["select.set", "select.points", "select.inRect", "select.pointAt"];

pub(super) fn register(r: &mut CommandRegistry) {
    // Auswählen mit der Maus (Zeiger, Teilauswahl, Hintere auswählen, Gummiband, Klick ins Leere):
    // im Original kein Verlaufsschritt – nur die Menübefehle sind welche
    r.add(
        cmd("tool.pick")
            .label("Auswählen (Maus)")
            .describe("Auswahl durch ein Werkzeug ändern, ohne Verlaufsschritt (wie Klicks im Original)")
            .requires(DOC)
            .param("command", ParamKind::Enum { values: PICK_COMMANDS.iter().map(|s| s.to_string()).collect() }, "Auswahlbefehl")
            .opt("params", ParamKind::Json, "Parameter des Auswahlbefehls")
            .run(|s, p| {
                let c = p.str("command")?;
                let out = run_inner(s, &c, p.get("params").cloned().unwrap_or_else(|| json!({})))?;
                if let Some(id) = s.active_id() {
                    s.emit(crate::session::SessionEvent::SelectionChanged(id));
                }
                Ok(CommandOutput { effect: crate::command::Effect::None, ..out })
            }),
    );
    // Leinwand-Kontextmenüs: Einträge mit eigener ID
    r.add(cmd("view.exitFullScreen").label("Vollbildmodus beenden").fw_id(35589).describe("Bildschirmmodus auf Standard zurücksetzen (Kontextmenü der Leinwand)").run(|s, _| {
        s.ui.screen_mode = 0;
        s.emit(crate::session::SessionEvent::PanelsChanged);
        Ok(CommandOutput::session_with(json!({"mode": 0})))
    }));
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
                if let Some(f) = p.get("fill").filter(|v| v.is_object())
                    && !open
                {
                    let mut f = f.clone();
                    f["ids"] = json!([id]);
                    run_inner(s, "pi.fill", f)?;
                }
                if let Some(st) = p.get("stroke").filter(|v| v.is_object()) {
                    let mut st = st.clone();
                    st["ids"] = json!([id]);
                    run_inner(s, "pi.stroke", st)?;
                }
                if let Some(bv) = p.get("brush").filter(|v| v.is_object()) {
                    let brush: fp_model::Brush = serde_json::from_value(bv.clone()).map_err(|e| bad(p, &format!("brush: {e}")))?;
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
                // Auswahl bleibt beim neuen Objekt
                with_doc(s, |d| {
                    d.selection.set(vec![ElementId(id)]);
                    Ok(())
                })?;
                Ok(CommandOutput { value: json!({"id": id}), ..CommandOutput::doc() })
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
            .param("id", ParamKind::Integer { min: Some(0), max: None }, "Pfad")
            .param("contour", ParamKind::Integer { min: Some(0), max: None }, "Kontur")
            .param("segment", ParamKind::Integer { min: Some(0), max: None }, "Segment")
            .param("t", ParamKind::Number { min: Some(0.0), max: Some(1.0) }, "Parameter auf dem Segment")
            .run(|s, p| {
                let id = ElementId(p.i64("id")? as u64);
                let (ci, seg, t) = (p.usize("contour")?, p.usize("segment")?, p.f64("t")?);
                with_doc(s, |d| {
                    edit_contours(d, id, |cs| {
                        let c = cs.get_mut(ci).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-contour")))?;
                        let bc = geometry::to_bezier(c).insert_node(seg, t).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-segment")))?;
                        *c = geometry::from_bezier(&bc);
                        Ok(())
                    })?;
                    d.selection.set(vec![id]);
                    d.selection.points = vec![PointRef { element: id, contour: ci, node: seg + 1 }];
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("path.removeNode")
            .label("Punkt entfernen")
            .requires(DOC)
            .param("id", ParamKind::Integer { min: Some(0), max: None }, "Pfad")
            .param("contour", ParamKind::Integer { min: Some(0), max: None }, "Kontur")
            .param("node", ParamKind::Integer { min: Some(0), max: None }, "Punkt")
            .run(|s, p| {
                let id = ElementId(p.i64("id")? as u64);
                let (ci, ni) = (p.usize("contour")?, p.usize("node")?);
                with_doc(s, |d| {
                    let mut empty = false;
                    edit_contours(d, id, |cs| {
                        let c = cs.get_mut(ci).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-contour")))?;
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
            .describe("Ausgewählte Pfade entlang einer Schnittlinie teilen (knifeElementsFromPoints)")
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
                        let Some(e) = docops::element_on_page(&d.doc, page, id) else { continue };
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
            .opt("second", ParamKind::Point, "zweite Achse (Vorgabe: senkrecht zu Anfang→Ende)")
            .run(|s, p| {
                let (a, b) = (p.point("start")?, p.point("end")?);
                let second = p.opt_point("second")?.unwrap_or(Point::new(a.x - (b.y - a.y), a.y + (b.x - a.x)));
                with_doc(s, |d| {
                    let (page, ids) = (d.page, target_ids(d, p)?);
                    let doc = Arc::make_mut(&mut d.doc);
                    for id in ids {
                        docops::edit_by_id(doc, page, id, |e| {
                            let h = fp_model::FillHandles { valid: true, start: a, end: b, second };
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
            .run(|s, p| {
                let mode = p.str("mode")?;
                let pts = p.points("points")?;
                if pts.is_empty() {
                    return Err(bad(p, "err-param-no-points"));
                }
                let prm = Retouch {
                    mode: mode.clone(),
                    size: p.f64_or("size", 13.0)? as f32,
                    round: p.bool_or("round", true)?,
                    soft: p.f64_or("soft", 1.0)?.clamp(0.0, 1.0) as f32,
                    strength: (p.f64_or("strength", 50.0)?.clamp(0.0, 100.0) / 100.0) as f32,
                    range: p.i64_or("range", 1)?,
                    spacing: p.f64_or("spacing", 20.0)? as f32,
                    offset: p.get("offset").and_then(crate::command::parse_point).map(|q| (q.x, q.y)).unwrap_or((0.0, 0.0)),
                    from: p.opt_color("from")?.map(|c| c.to_rgba8()).unwrap_or(Rgba8::new(0, 255, 0, 255)),
                    to: p.opt_color("to")?.map(|c| c.to_rgba8()).unwrap_or(Rgba8::new(255, 0, 0, 255)),
                    colorize: p.bool_or("colorize", false)?,
                    tolerance: p.i64_or("tolerance", 50)?.clamp(0, 255) as f32,
                };
                with_doc(s, |d| {
                    let (path, _) = bitmap_for(d, pts.first().copied(), &BasicGeometry).ok_or_else(|| sel_err("err-sel-no-bitmap-under-stroke"))?;
                    let sel = d.selection.pixels.clone().filter(|_| d.selection.has_pixels());
                    let doc = Arc::make_mut(&mut d.doc);
                    docops::edit_at(doc, &path, |e| {
                        if let ElementKind::Bitmap(b) = &mut e.kind
                            && let Some(mut img) = bitmap_image(b)
                        {
                            retouch(&mut img, &pts, &prm, sel.as_ref());
                            set_bitmap_image(b, &img);
                        }
                    });
                    Ok(CommandOutput::doc())
                })
            }),
    );
}

/// 9-Slice eines Elements (Gruppen rekursiv; Pfade punktweise, Rechteck-Grundformen behalten die
/// Rundung in Pixeln; Bitmaps, Text und Symbole affin auf den Zielrahmen).
fn nine_slice_element(e: &mut fp_model::Element, from: &fp_geom::Rect, insets: [f32; 4], to: &fp_geom::Rect, depth: u32) {
    use crate::tools::pathedit::{nine_slice_contours, nine_slice_point};
    e.cache = None;
    match &mut e.kind {
        ElementKind::Path(pe) => {
            // Rechteck-Grundform ohne Drehung/Neigung: bleibt Grundform, der Eckenradius in Pixeln bleibt
            let axis = pe.primitive.as_ref().and_then(|pr| {
                let m = pr.transform.m;
                (m[0][1].abs() < 1e-6 && m[1][0].abs() < 1e-6).then(|| pr.transform.inverse()).flatten()
            });
            if let (Some(inv), Some(pr)) = (axis, pe.primitive.as_mut()) {
                let m = pr.transform;
                let a = inv.apply(nine_slice_point(m.apply(Point::new(pr.rect.left, pr.rect.top)), from, insets, to));
                let b = inv.apply(nine_slice_point(m.apply(Point::new(pr.rect.right, pr.rect.bottom)), from, insets, to));
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
        _ => docops::transform_element(e, &docops::rect_to_rect(*from, *to), fp_raster::Sampler::Bicubic),
    }
}

/// Parameter der Retusche.
struct Retouch {
    mode: String,
    size: f32,
    round: bool,
    soft: f32,
    /// 0..1.
    strength: f32,
    range: i64,
    spacing: f32,
    offset: (f32, f32),
    from: Rgba8,
    to: Rgba8,
    colorize: bool,
    tolerance: f32,
}

fn lum(p: [u8; 4]) -> f32 {
    0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32
}

fn rgb_to_hsv(p: [u8; 4]) -> (f32, f32, f32) {
    let (r, g, b) = (p[0] as f32 / 255.0, p[1] as f32 / 255.0, p[2] as f32 / 255.0);
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

/// Wendet die Retusche entlang des Strichs auf das Bild an (in Dokumentkoordinaten).
fn retouch(img: &mut RgbaImage, pts: &[Point], prm: &Retouch, sel: Option<&fp_raster::Mask>) {
    let r = (prm.size / 2.0).max(0.5);
    let step = (prm.size * prm.spacing / 100.0).max(1.0);
    // Abdrücke im Abstand `step`
    let mut stamps = vec![pts[0]];
    let mut carry = 0.0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b.x - a.x).hypot(b.y - a.y);
        let mut t = step - carry;
        while t <= len {
            let f = t / len.max(1e-6);
            stamps.push(Point::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f));
            t += step;
        }
        carry = (carry + len) % step;
    }
    let source = img.clone();
    let inner = r * (1.0 - prm.soft);
    let mut prev: Option<Point> = None;
    for s in stamps {
        let (x0, y0, x1, y1) = ((s.x - r).floor() as i32, (s.y - r).floor() as i32, (s.x + r).ceil() as i32, (s.y + r).ceil() as i32);
        let snapshot = if prm.mode == "smudge" || prm.mode == "blur" || prm.mode == "sharpen" { Some(img.clone()) } else { None };
        for y in y0..y1 {
            for x in x0..x1 {
                let (cx, cy) = (x as f32 + 0.5 - s.x, y as f32 + 0.5 - s.y);
                let d = if prm.round { cx.hypot(cy) } else { cx.abs().max(cy.abs()) };
                if d > r {
                    continue;
                }
                let mut w = if d <= inner || r <= inner { 1.0 } else { 1.0 - (d - inner) / (r - inner) };
                w *= prm.strength;
                if let Some(m) = sel {
                    w *= m.get(x, y) as f32 / 255.0;
                }
                if w <= 0.0 {
                    continue;
                }
                let Some(px) = img.get(x, y) else { continue };
                let get = |im: &RgbaImage, xx: i32, yy: i32| im.get(xx, yy).unwrap_or(px);
                let avg = |im: &RgbaImage| -> [f32; 4] {
                    let mut acc = [0f32; 4];
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            let q = get(im, x + dx, y + dy);
                            for k in 0..4 {
                                acc[k] += q[k] as f32;
                            }
                        }
                    }
                    acc.map(|v| v / 9.0)
                };
                let mix = |a: [u8; 4], b: [f32; 4], t: f32| -> [u8; 4] {
                    let mut o = a;
                    for k in 0..4 {
                        o[k] = (a[k] as f32 + (b[k] - a[k] as f32) * t).round().clamp(0.0, 255.0) as u8;
                    }
                    o
                };
                let out = match prm.mode.as_str() {
                    "blur" => mix(px, avg(snapshot.as_ref().unwrap_or(img)), w),
                    "sharpen" => {
                        let a = avg(snapshot.as_ref().unwrap_or(img));
                        let target = [0, 1, 2].map(|k| px[k] as f32 + (px[k] as f32 - a[k]) * 1.5);
                        mix(px, [target[0], target[1], target[2], px[3] as f32], w)
                    }
                    "dodge" | "burn" => {
                        let l = lum(px) / 255.0;
                        let band = match prm.range {
                            0 => (1.0 - l).powi(2),
                            2 => l * l,
                            _ => 1.0 - (2.0 * l - 1.0).powi(2),
                        };
                        let t = w * 0.35 * band;
                        if prm.mode == "dodge" {
                            mix(px, [255.0, 255.0, 255.0, px[3] as f32], t)
                        } else {
                            mix(px, [0.0, 0.0, 0.0, px[3] as f32], t)
                        }
                    }
                    "smudge" => {
                        let Some(pv) = prev else { continue };
                        let q = get(snapshot.as_ref().unwrap_or(img), x - (s.x - pv.x).round() as i32, y - (s.y - pv.y).round() as i32);
                        mix(px, q.map(|v| v as f32), w)
                    }
                    "stamp" => {
                        let (sx, sy) = ((x as f32 + prm.offset.0).floor() as i32, (y as f32 + prm.offset.1).floor() as i32);
                        let Some(q) = source.get(sx, sy) else { continue };
                        mix(px, q.map(|v| v as f32), w)
                    }
                    "replaceColor" => {
                        let f = prm.from;
                        let dist = ((px[0] as f32 - f.r as f32).abs() + (px[1] as f32 - f.g as f32).abs() + (px[2] as f32 - f.b as f32).abs()) / 3.0;
                        let tol = prm.tolerance.max(1.0);
                        if dist > tol {
                            continue;
                        }
                        let t = w * (1.0 - dist / tol).max(0.2);
                        let tc = prm.to;
                        let target = if prm.colorize {
                            let l = lum(px) / 255.0;
                            [tc.r as f32 * l * 1.5, tc.g as f32 * l * 1.5, tc.b as f32 * l * 1.5, px[3] as f32]
                        } else {
                            // Abweichung von der Von-Farbe bleibt erhalten (Schattierung)
                            let (fr, tr) = ([f.r, f.g, f.b], [tc.r, tc.g, tc.b]);
                            let c = |k: usize| tr[k] as f32 + px[k] as f32 - fr[k] as f32;
                            [c(0), c(1), c(2), px[3] as f32]
                        };
                        mix(px, target, t)
                    }
                    _ => {
                        // Rote Augen: Farbton 300°–5°, Sättigung ≥ 50 %, Helligkeit ≥ 25 % → Rotkanal absenken
                        let (h, sat, v) = rgb_to_hsv(px);
                        if !(h >= 300.0 || h <= 5.0 + prm.tolerance / 10.0) || sat < 0.5 || v < 0.25 {
                            continue;
                        }
                        let rr = (px[1] as f32 + px[2] as f32) / 2.0;
                        mix(px, [rr, px[1] as f32, px[2] as f32, px[3] as f32], (w * 2.0).min(1.0))
                    }
                };
                img.set_pixel(x, y, Rgba8::new(out[0], out[1], out[2], out[3]));
            }
        }
        prev = Some(s);
    }
}
