//! Treffertest: oberstes Element an einem Punkt (für Auswahlwerkzeug und Leinwand der UI).
//!
//! Getestet wird am gerechneten Aussehen (Alpha > 0 im Pixel bzw. in der Toleranz), in derselben
//! Reihenfolge wie gezeichnet, nur umgekehrt: oberste Ebene zuerst, in jeder Ebene das erste
//! Element von `ELMv` zuerst. Gruppen und Instanzen werden als Ganzes getroffen (wie im Original
//! mit dem Zeiger-Werkzeug); [`Hit::leaf`] nennt zusätzlich das innerste getroffene Element einer
//! Gruppe (Unterauswahl). Inhalte der Masterseite sind nicht treffbar (Elementzustand 1, 04 §1.6).

use std::sync::Arc;

use fp_model::{
    Document, Element, ElementId, ElementKind, ElementPath, ElementState, Layer, LayerKind,
    LayerSlot, PathStep, RootRef,
};

use crate::compose::{Renderer, State};
use crate::nine::PointMap;
use crate::options::RenderReport;

/// Einstellungen des Treffertests.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HitOptions {
    /// Toleranz in Dokumentpixeln (0 = nur das Pixel unter dem Punkt).
    pub tolerance: i32,
    /// Slices/Hotspots der Web-Ebene treffen (über ihr Rechteck).
    pub include_web: bool,
    /// Gesperrte Elemente und Ebenen treffen.
    pub include_locked: bool,
}

/// Ergebnis des Treffertests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    /// Adresse des obersten getroffenen Elements der Ebene (bei Unterebenen inklusive Weg).
    pub path: ElementPath,
    /// Laufzeit-ID dieses Elements.
    pub element: ElementId,
    /// Innerstes getroffenes Element (in Gruppen); sonst gleich `element`.
    pub leaf: ElementId,
}

impl Renderer {
    /// Oberstes Element an (`x`, `y`) (Dokumentkoordinaten) auf Seite `page` im Status `frame`.
    pub fn hit_test(
        &self,
        doc: &Document,
        page: usize,
        frame: usize,
        x: f64,
        y: f64,
        opt: &HitOptions,
    ) -> Option<Hit> {
        let p = doc.pages.get(page)?;
        let (px, py) = (x.floor() as i32, y.floor() as i32);
        let st = State {
            map: PointMap::identity(),
            frame,
            symbols: Vec::new(),
        };
        for (slot, s) in p.root.layers.iter().enumerate().rev() {
            let (layer, root, slot_i, state) = match s {
                LayerSlot::Own(l) => (
                    Arc::clone(l),
                    RootRef::Page(page),
                    slot,
                    ElementState::Normal,
                ),
                LayerSlot::Shared(r) => {
                    let Some(i) = doc.shared_layers.iter().position(|d| d.id.same_as(&r.id)) else {
                        continue;
                    };
                    let Some(l) = doc.shared_layers[i].layer.as_ref() else {
                        continue;
                    };
                    (
                        Arc::clone(l),
                        RootRef::SharedLayer(i),
                        0,
                        ElementState::SharedLayer,
                    )
                }
                _ => continue,
            };
            if layer.kind == LayerKind::Web && !opt.include_web {
                continue;
            }
            if let Some(h) =
                self.hit_layer(doc, &layer, root, slot_i, state, &st, px, py, x, y, opt)
            {
                return Some(h);
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn hit_layer(
        &self,
        doc: &Document,
        layer: &Layer,
        root: RootRef,
        slot: usize,
        _state: ElementState,
        st: &State,
        px: i32,
        py: i32,
        x: f64,
        y: f64,
        opt: &HitOptions,
    ) -> Option<Hit> {
        if !layer.is_visible() {
            return None;
        }
        let cel_index = if layer.shared_across_frames {
            0
        } else {
            st.frame.min(layer.cels.len().saturating_sub(1))
        };
        let cel = layer.cels.get(cel_index)?;
        if !cel.visible || (cel.locked && !opt.include_locked) {
            return None;
        }
        let mut path = ElementPath {
            root,
            slot,
            cel: cel_index,
            steps: Vec::new(),
        };
        self.hit_list(
            doc,
            &cel.elements,
            &mut path,
            st,
            px,
            py,
            x,
            y,
            opt,
            layer.kind == LayerKind::Web,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn hit_list(
        &self,
        doc: &Document,
        list: &[Arc<Element>],
        path: &mut ElementPath,
        st: &State,
        px: i32,
        py: i32,
        x: f64,
        y: f64,
        opt: &HitOptions,
        web: bool,
    ) -> Option<Hit> {
        for (i, e) in list.iter().enumerate() {
            if !e.common.visible || (e.common.locked && !opt.include_locked) {
                continue;
            }
            path.steps.push(PathStep::Child(i));
            let found = match &e.kind {
                ElementKind::WebObject(w) => {
                    let b = w.bounds;
                    (web || opt.include_web)
                        && x >= b.left as f64
                        && x < b.right as f64
                        && y >= b.top as f64
                        && y < b.bottom as f64
                }
                ElementKind::SubLayer(l) => {
                    path.steps.push(PathStep::SubLayerCel(0));
                    let r = l.cels.first().and_then(|c| {
                        self.hit_list(doc, &c.elements, path, st, px, py, x, y, opt, web)
                    });
                    path.steps.pop();
                    if let Some(h) = r {
                        return Some(h);
                    }
                    false
                }
                _ => self.hits_pixel(doc, e, st, px, py, opt.tolerance),
            };
            if found {
                let leaf = self
                    .leaf_at(doc, e, st, px, py, opt.tolerance)
                    .unwrap_or(e.id);
                return Some(Hit {
                    path: path.clone(),
                    element: e.id,
                    leaf,
                });
            }
            path.steps.pop();
        }
        None
    }

    fn hits_pixel(
        &self,
        doc: &Document,
        e: &Arc<Element>,
        st: &State,
        px: i32,
        py: i32,
        tol: i32,
    ) -> bool {
        let mut rep = RenderReport::default();
        let Some(look) = self.look(doc, e, st, &mut rep) else {
            return false;
        };
        let t = tol.clamp(0, 64);
        for yy in py - t..=py + t {
            for xx in px - t..=px + t {
                if look.image.get(xx, yy).is_some_and(|p| p[3] > 0) {
                    return true;
                }
            }
        }
        false
    }

    fn leaf_at(
        &self,
        doc: &Document,
        e: &Arc<Element>,
        st: &State,
        px: i32,
        py: i32,
        tol: i32,
    ) -> Option<ElementId> {
        let ElementKind::Group(g) = &e.kind else {
            return None;
        };
        for c in &g.children {
            if c.common.visible && self.hits_pixel(doc, c, st, px, py, tol) {
                return Some(self.leaf_at(doc, c, st, px, py, tol).unwrap_or(c.id));
            }
        }
        None
    }
}
