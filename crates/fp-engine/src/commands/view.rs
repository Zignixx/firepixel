//! Menü „Ansicht“: Zoom (10 §4.4), Einpassen, Raster, Hilfslinien, Lineale, Ausblenden, Seiten- und
//! Statuswechsel, Hand-Werkzeug und Hilfslinien-Werkzeuge.

use super::{canvas_rect, with_doc, DOC};
use crate::command::{cmd, CommandOutput, CommandRegistry, ParamKind, Requirement};
use crate::docops;
use crate::error::{EngineError, Result};
use crate::session::{DocumentState, Session};
use crate::view::ViewState;
use fp_model::{Grid, Guides};
use std::sync::Arc;

fn view_cmd(s: &mut Session, f: impl FnOnce(&mut ViewState)) -> Result<CommandOutput> {
    let d = s.doc_mut()?;
    f(&mut d.view);
    Ok(CommandOutput::view())
}

fn anchor_for(d: &DocumentState, p: &crate::command::Params) -> Result<Option<(f64, f64)>> {
    Ok(p.opt_point("at")?.map(|pt| d.view.doc_to_screen(pt)))
}

/// Menüwerte der Vergrößerungsbefehle → exakter Tabellenwert.
const MAGNIFICATIONS: &[(&str, f64)] = &[
    ("6", 0.0625),
    ("12", 0.125),
    ("25", 0.25),
    ("50", 0.5),
    ("66", 0.66),
    ("100", 1.0),
    ("150", 1.5),
    ("200", 2.0),
    ("300", 3.0),
    ("400", 4.0),
    ("800", 8.0),
    ("1600", 16.0),
    ("3200", 32.0),
    ("6400", 64.0),
];

fn page_guides(d: &mut DocumentState) -> Result<&mut Guides> {
    let page = d.page;
    let p = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
    Ok(p.root.guides.get_or_insert_with(Guides::default))
}

macro_rules! toggle {
    ($r:expr, $name:expr, $field:ident) => {
        $r.add(cmd($name).requires(DOC).checked(|s| s.active().is_some_and(|d| d.view.$field)).run(|s, _| view_cmd(s, |v| v.$field = !v.$field)));
    };
}

pub(super) fn register(r: &mut CommandRegistry) {
    r.add(
        cmd("view.zoomIn")
            .requires(DOC)
            .opt("at", ParamKind::Point, "Dokumentpunkt, der an seiner Bildschirmstelle bleibt")
            .available(|s| s.active().is_some_and(|d| d.view.zoom < crate::view::MAX_ZOOM))
            .run(|s, p| {
                let d = s.doc_mut()?;
                let a = anchor_for(d, p)?;
                d.view.zoom_in(a);
                Ok(CommandOutput::view())
            }),
    );
    r.add(
        cmd("view.zoomOut")
            .requires(DOC)
            .opt("at", ParamKind::Point, "Dokumentpunkt, der an seiner Bildschirmstelle bleibt")
            .available(|s| s.active().is_some_and(|d| d.view.zoom > crate::view::MIN_ZOOM))
            .run(|s, p| {
                let d = s.doc_mut()?;
                let a = anchor_for(d, p)?;
                d.view.zoom_out(a);
                Ok(CommandOutput::view())
            }),
    );
    for &(suffix, z) in MAGNIFICATIONS {
        let name = format!("view.magnification.{suffix}");
        r.add(cmd(&name).requires(DOC).checked(move |s| s.active().is_some_and(|d| d.view.zoom == z)).run(move |s, _| view_cmd(s, |v| v.set_zoom(z, None))));
    }
    r.add(
        cmd("view.setZoom")
            .label("Zoom setzen")
            .requires(DOC)
            .param("zoom", ParamKind::Number { min: Some(0.0625), max: Some(64.0) }, "Zoomfaktor (1 = 100 %)")
            .opt("at", ParamKind::Point, "Ankerpunkt (Dokumentkoordinaten)")
            .run(|s, p| {
                let z = p.f64("zoom")?;
                let d = s.doc_mut()?;
                let a = anchor_for(d, p)?;
                d.view.set_zoom(z, a);
                Ok(CommandOutput::view())
            }),
    );
    r.add(cmd("view.zoomToRect").label("Auf Bereich zoomen").requires(DOC).param("rect", ParamKind::Rect, "Bereich").run(|s, p| {
        let rr = p.rect("rect")?;
        view_cmd(s, |v| v.fit_rect(rr))
    }));
    r.add(
        cmd("view.pan")
            .label("Ansicht verschieben")
            .requires(DOC)
            .param("dx", ParamKind::Number { min: None, max: None }, "Δx in Bildschirmpixeln")
            .param("dy", ParamKind::Number { min: None, max: None }, "Δy in Bildschirmpixeln")
            .run(|s, p| {
                let (dx, dy) = (p.f64("dx")?, p.f64("dy")?);
                view_cmd(s, |v| v.pan(dx, dy))
            }),
    );
    r.add(
        cmd("view.setViewport")
            .label("Sichtbereich melden")
            .requires(DOC)
            .param("width", ParamKind::Number { min: Some(1.0), max: None }, "Breite in Bildschirmpixeln")
            .param("height", ParamKind::Number { min: Some(1.0), max: None }, "Höhe in Bildschirmpixeln")
            .run(|s, p| {
                let (w, h) = (p.f64("width")?, p.f64("height")?);
                view_cmd(s, |v| v.viewport = (w, h))
            }),
    );
    r.add(cmd("view.fitAll").requires(DOC).run(|s, _| {
        let d = s.doc_mut()?;
        let r = canvas_rect(&d.doc, d.page);
        // stufenlos mit 15 px Rand wie im Original (nicht die nächste Tabellenstufe)
        d.view.fit_rect_exact(r);
        Ok(CommandOutput::view())
    }));
    // Menü wie im Original nur mit Auswahl (Kontextmenü/Werkzeug zoomen sonst auf die Leinwand)
    r.add(cmd("view.fitSelection").requires(DOC).menu_when(|s| s.active().is_some_and(|d| !d.selection.elements.is_empty())).run(|s, _| {
        let d = s.doc_mut()?;
        let r = docops::ids_bounds(&d.doc, d.page, &d.selection.elements).unwrap_or_else(|| canvas_rect(&d.doc, d.page));
        d.view.fit_rect_exact(r);
        Ok(CommandOutput::view())
    }));
    toggle!(r, "view.macintoshGamma", gamma_preview);
    toggle!(r, "view.rulers", show_rulers);
    toggle!(r, "view.grid.showGrid", show_grid);
    toggle!(r, "view.grid.snapToGrid", snap_to_grid);
    toggle!(r, "view.guides.showGuides", show_guides);
    toggle!(r, "view.guides.lockGuides", lock_guides);
    toggle!(r, "view.guides.snapToGuides", snap_to_guides);
    toggle!(r, "view.smartGuides.showSmartGuides", show_smart_guides);
    toggle!(r, "view.smartGuides.snapToSmartGuides", snap_to_smart_guides);
    toggle!(r, "view.nineSliceScalingGuides.lockGuides", lock_nine_slice_guides);
    toggle!(r, "view.sliceGuides", show_slice_guides);
    toggle!(r, "view.sliceOverlay", show_slice_overlay);
    toggle!(r, "view.edges", show_edges);
    r.add(cmd("view.toggleSlices").label("Segmente ein-/ausblenden").describe("Pseudo-Werkzeug 35579").requires(DOC).checked(|s| s.active().is_some_and(|d| !d.view.hide_slices)).run(|s, _| view_cmd(s, |v| v.hide_slices = !v.hide_slices)));
    r.add(cmd("view.tooltips").checked(|s| s.ui.tooltips).run(|s, _| {
        s.ui.tooltips = !s.ui.tooltips;
        Ok(CommandOutput::session())
    }));
    r.add(cmd("view.hideSelection").requires(&[Requirement::Document, Requirement::Selection]).run(|s, _| {
        with_doc(s, |d| {
            let (page, ids) = (d.page, d.selection.elements.clone());
            let doc = Arc::make_mut(&mut d.doc);
            for id in ids {
                docops::edit_by_id(doc, page, id, |e| e.common.visible = false);
            }
            d.selection.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(cmd("view.showAll").requires(DOC).run(|s, _| {
        with_doc(s, |d| {
            let page = d.page;
            let mut hidden = Vec::new();
            if let Some(p) = d.doc.pages.get(page) {
                fp_model::traverse::walk_root(&p.root, fp_model::RootRef::Page(page), &mut |_, path, e| {
                    if !e.common.visible {
                        hidden.push(path.clone());
                    }
                });
            }
            let doc = Arc::make_mut(&mut d.doc);
            for path in hidden {
                docops::edit_at(doc, &path, |e| e.common.visible = true);
            }
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("view.grid.editGrid")
            .requires(DOC)
            .opt("width", ParamKind::Number { min: Some(1.0), max: None }, "Rasterabstand horizontal")
            .opt("height", ParamKind::Number { min: Some(1.0), max: None }, "Rasterabstand vertikal")
            .opt("color", ParamKind::Color, "Rasterfarbe")
            .opt("origin", ParamKind::Point, "Rasterursprung")
            .opt("show", ParamKind::Bool, "Raster anzeigen")
            .opt("snap", ParamKind::Bool, "Am Raster ausrichten")
            .run(|s, p| {
                let (w, h, c, o) = (p.opt_f64("width")?, p.opt_f64("height")?, p.opt_color("color")?, p.opt_point("origin")?);
                let (show, snap) = (p.opt_bool("show")?, p.opt_bool("snap")?);
                with_doc(s, |d| {
                    if let Some(v) = show {
                        d.view.show_grid = v;
                    }
                    if let Some(v) = snap {
                        d.view.snap_to_grid = v;
                    }
                    if w.is_some() || h.is_some() || c.is_some() || o.is_some() {
                        let page = d.page;
                        let pg = Arc::make_mut(&mut d.doc).page_mut(page).ok_or_else(|| EngineError::NotFound(fp_i18n::t("err-nf-page")))?;
                        let g = pg.root.grid.get_or_insert_with(Grid::default);
                        if let Some(w) = w {
                            g.size.x = w as f32;
                        }
                        if let Some(h) = h {
                            g.size.y = h as f32;
                        }
                        if let Some(c) = c {
                            g.color = c;
                        }
                        if let Some(o) = o {
                            g.origin = o;
                        }
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("view.guides.editGuides")
            .requires(DOC)
            .opt("horizontal", ParamKind::Json, "y-Positionen der horizontalen Hilfslinien")
            .opt("vertical", ParamKind::Json, "x-Positionen der vertikalen Hilfslinien")
            .opt("color", ParamKind::Color, "Farbe")
            .run(|s, p| {
                let hz: Option<Vec<f32>> = p.parse("horizontal")?;
                let vt: Option<Vec<f32>> = p.parse("vertical")?;
                let c = p.opt_color("color")?;
                with_doc(s, |d| {
                    let g = page_guides(d)?;
                    if let Some(h) = hz {
                        g.horizontal = h;
                    }
                    if let Some(v) = vt {
                        g.vertical = v;
                    }
                    if let Some(c) = c {
                        g.color = Some(c);
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(cmd("view.guides.clearGuides").requires(DOC).menu_when(|s| s.active().is_some_and(|d| d.doc.pages.get(d.page).and_then(|p| p.root.guides.as_ref()).is_some_and(|g| !g.horizontal.is_empty() || !g.vertical.is_empty()))).run(|s, _| {
        with_doc(s, |d| {
            let g = page_guides(d)?;
            g.horizontal.clear();
            g.vertical.clear();
            Ok(CommandOutput::doc())
        })
    }));
    r.add(
        cmd("guide.add")
            .label("Hilfslinie hinzufügen")
            .describe("Hilfslinie aus dem Lineal ziehen (CAddGuideTool)")
            .requires(DOC)
            .param("orientation", ParamKind::Enum { values: vec!["horizontal".into(), "vertical".into()] }, "Ausrichtung")
            .param("position", ParamKind::Number { min: None, max: None }, "Position in Pixel")
            .run(|s, p| {
                let horiz = p.str("orientation")? == "horizontal";
                let pos = p.f64("position")? as f32;
                with_doc(s, |d| {
                    if d.view.lock_guides {
                        return Err(EngineError::Failed(fp_i18n::t("err-guides-locked")));
                    }
                    let g = page_guides(d)?;
                    if horiz { g.horizontal.push(pos) } else { g.vertical.push(pos) }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    r.add(
        cmd("guide.move")
            .label("Hilfslinie verschieben")
            .requires(DOC)
            .param("orientation", ParamKind::Enum { values: vec!["horizontal".into(), "vertical".into()] }, "Ausrichtung")
            .param("index", ParamKind::Integer { min: Some(0), max: None }, "Index")
            .opt("position", ParamKind::Number { min: None, max: None }, "neue Position; fehlt = entfernen")
            .run(|s, p| {
                let horiz = p.str("orientation")? == "horizontal";
                let i = p.usize("index")?;
                let pos = p.opt_f64("position")?;
                with_doc(s, |d| {
                    let g = page_guides(d)?;
                    let v = if horiz { &mut g.horizontal } else { &mut g.vertical };
                    if i >= v.len() {
                        return Err(EngineError::NotFound(fp_i18n::t("err-nf-guide")));
                    }
                    match pos {
                        Some(x) => v[i] = x as f32,
                        None => {
                            v.remove(i);
                        }
                    }
                    Ok(CommandOutput::doc())
                })
            }),
    );
    // Seiten- und Statuswechsel
    r.add(cmd("view.state.next").requires(DOC).run(|s, _| step_frame(s, 1)));
    r.add(cmd("view.state.previous").requires(DOC).run(|s, _| step_frame(s, -1)));
    r.add(cmd("view.page.next").requires(&[Requirement::Document, Requirement::MultiplePages]).run(|s, _| step_page(s, 1)));
    r.add(cmd("view.page.previous").requires(&[Requirement::Document, Requirement::MultiplePages]).run(|s, _| step_page(s, -1)));
}

fn step_frame(s: &mut Session, dir: i64) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let n = d.frame_count() as i64;
        d.frame = ((d.frame as i64 + dir).rem_euclid(n)) as usize;
        d.selection.clear();
        Ok(CommandOutput::doc())
    })
}

fn step_page(s: &mut Session, dir: i64) -> Result<CommandOutput> {
    with_doc(s, |d| {
        let n = d.doc.pages.len() as i64;
        d.page = ((d.page as i64 + dir).rem_euclid(n.max(1))) as usize;
        d.frame = 0;
        d.layer = docops::layer_on_page(&d.doc, d.page, d.layer);
        d.selection.clear();
        Ok(CommandOutput::doc())
    })
}
