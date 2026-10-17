//! Traversieren und Suchen: Elemente aller Wurzeln, Instanz → Symbol (erste Definition gewinnt),
//! effektive Ebenenlisten (geteilte Ebenen und Masterseite eingesetzt), Adressen von Elementen.

use crate::doc::{Document, LayerRoot, Page};
use crate::element::{Element, ElementKind};
use crate::ids::{ElementId, Guid, LayerId, SymbolId, SymbolKey};
use crate::layer::{Cel, ElementState, Layer, LayerKind, LayerSlot, SharedLayerDef};
use crate::symbol::Symbol;
use std::sync::Arc;

/// Wurzel, in der ein Element liegt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RootRef {
    /// Seite `pages[i]`.
    Page(usize),
    /// `MKBv`-Kopie der aktiven Seite.
    RootCopy,
    /// Symbol `symbols[i]`.
    Symbol(SymbolKey),
    /// Definition `shared_layers[i]` des Dokuments.
    SharedLayer(usize),
}

/// Schritt auf dem Weg von einem Cel zu einem Element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PathStep {
    /// Element `i` einer Elementliste (Cel, Gruppenkinder, Operanden).
    Child(usize),
    /// Maske einer Maskengruppe.
    Mask,
    /// Element `i` des `MRMv`-Ersatzes.
    Replacement(usize),
    /// Ergebnis-Pfad eines Compound Shapes.
    Result,
    /// Stellvertreter-Pfad hinter einem Compound Shape.
    Proxy,
    /// Pfad von Text auf Pfad.
    TextPath,
    /// Cel `c` einer Unterebene, danach Element `i` daraus.
    SubLayerCel(usize),
}

/// Adresse eines Elements im Dokumentbaum.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ElementPath {
    pub root: RootRef,
    /// Index in der Ebenenliste der Wurzel (bei `SharedLayer` immer 0).
    pub slot: usize,
    /// Index des Cels in der Ebene.
    pub cel: usize,
    /// Weg durch Gruppen/Unterebenen bis zum Element.
    pub steps: Vec<PathStep>,
}

/// Kontext beim Traversieren.
#[derive(Clone, Copy, Debug)]
pub struct WalkCtx<'a> {
    pub root: RootRef,
    /// Oberste Ebene, in der das Element liegt.
    pub layer: &'a Layer,
    /// Cel-Index der obersten Ebene.
    pub cel: usize,
    /// Verschachtelungstiefe in Gruppen/Unterebenen (0 = direkt im Cel).
    pub depth: usize,
}

/// Herkunft einer Ebene in der effektiven Ebenenliste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerOrigin {
    /// Eigene Ebene der Seite.
    Own,
    /// Über Seiten geteilte Ebene (Elementzustand 2).
    Shared,
    /// Ebene der Masterseite (Elementzustand 1).
    MasterPage,
}

/// Eintrag der effektiven Ebenenliste.
#[derive(Clone, Debug)]
pub struct ResolvedLayer {
    pub layer: Arc<Layer>,
    pub origin: LayerOrigin,
}

impl ResolvedLayer {
    /// Elementzustand der Inhalte (04 §1.6).
    pub fn state(&self) -> ElementState {
        match self.origin {
            LayerOrigin::Own => ElementState::Normal,
            LayerOrigin::Shared => ElementState::SharedLayer,
            LayerOrigin::MasterPage => ElementState::MasterPageContent,
        }
    }
}

impl Document {
    /// Symbol zu einer ID: **erste Definition in Listenreihenfolge** (im Original `@00870e00`, 13 §7.1).
    pub fn symbol_by_id(&self, id: SymbolId) -> Option<(SymbolKey, &Arc<Symbol>)> {
        self.symbols
            .iter()
            .enumerate()
            .find(|(_, s)| s.id == id)
            .map(|(i, s)| (SymbolKey(i), s))
    }

    /// Symbol zum internen Schlüssel.
    pub fn symbol(&self, key: SymbolKey) -> Option<&Arc<Symbol>> {
        self.symbols.get(key.0)
    }

    /// Symbol zum Namen (erstes passendes).
    pub fn symbol_by_name(&self, name: &str) -> Option<(SymbolKey, &Arc<Symbol>)> {
        self.symbols
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == name)
            .map(|(i, s)| (SymbolKey(i), s))
    }

    /// Definition einer geteilten Ebene (Suche in `MKBv/LSMv`, dann in den Seiten).
    pub fn shared_layer(&self, id: &Guid) -> Option<&SharedLayerDef> {
        self.shared_layers
            .iter()
            .find(|d| d.id.same_as(id))
            .or_else(|| {
                self.pages
                    .iter()
                    .filter_map(|p| p.local_shared_layers.as_ref())
                    .flat_map(|v| v.iter())
                    .find(|d| d.id.same_as(id))
            })
    }

    /// Masterseite (höchstens eine).
    pub fn master_page(&self) -> Option<(usize, &Arc<Page>)> {
        self.pages.iter().enumerate().find(|(_, p)| p.master())
    }

    /// Seite zu einer GUID.
    pub fn page_by_id(&self, id: &Guid) -> Option<(usize, &Arc<Page>)> {
        self.pages
            .iter()
            .enumerate()
            .find(|(_, p)| p.id.as_ref().is_some_and(|g| g.same_as(id)))
    }

    /// Effektive Ebenenliste einer Seite unten → oben: Verweise auf geteilte Ebenen aufgelöst, der
    /// Masterseiten-Platzhalter durch die sichtbaren normalen Ebenen der Masterseite ersetzt (04 §2.5).
    pub fn resolved_layers(&self, page: usize) -> Vec<ResolvedLayer> {
        let mut out = Vec::new();
        let Some(p) = self.pages.get(page) else {
            return out;
        };
        for slot in &p.root.layers {
            match slot {
                LayerSlot::Own(l) => out.push(ResolvedLayer {
                    layer: Arc::clone(l),
                    origin: LayerOrigin::Own,
                }),
                LayerSlot::Shared(r) => {
                    if let Some(l) = self.shared_layer(&r.id).and_then(|d| d.layer.as_ref()) {
                        out.push(ResolvedLayer {
                            layer: Arc::clone(l),
                            origin: LayerOrigin::Shared,
                        });
                    }
                }
                LayerSlot::MasterPage(m) => {
                    if !m.is_visible() {
                        continue;
                    }
                    let Some(mid) = m.master_id() else { continue };
                    if let Some((mi, mp)) = self.page_by_id(mid) {
                        if mi == page || !mp.master() {
                            continue;
                        }
                        for ms in &mp.root.layers {
                            if let LayerSlot::Own(l) = ms
                                && l.kind == LayerKind::Normal
                            {
                                out.push(ResolvedLayer {
                                    layer: Arc::clone(l),
                                    origin: LayerOrigin::MasterPage,
                                });
                            }
                        }
                    }
                }
                LayerSlot::Raw(_) => {}
            }
        }
        out
    }

    /// Wurzel zu einer Referenz.
    pub fn root(&self, r: RootRef) -> Option<&LayerRoot> {
        match r {
            RootRef::Page(i) => self.pages.get(i).map(|p| &p.root),
            RootRef::RootCopy => self.root_copy.as_ref().map(|p| &p.root),
            RootRef::Symbol(k) => self.symbols.get(k.0).map(|s| &s.root),
            RootRef::SharedLayer(_) => None,
        }
    }

    /// Ruft `f` für jedes Element aller Seiten, Symbole und geteilten Ebenen auf (ohne die `MKBv`-Kopie),
    /// einschließlich Gruppenkinder, Masken, Unterebenen und Compound-Shape-Teile.
    pub fn walk_elements(&self, mut f: impl FnMut(&WalkCtx, &ElementPath, &Arc<Element>)) {
        for (i, p) in self.pages.iter().enumerate() {
            walk_root(&p.root, RootRef::Page(i), &mut f);
        }
        for (i, s) in self.symbols.iter().enumerate() {
            walk_root(&s.root, RootRef::Symbol(SymbolKey(i)), &mut f);
        }
        for (i, d) in self.shared_layers.iter().enumerate() {
            if let Some(l) = &d.layer {
                walk_layer(l, RootRef::SharedLayer(i), 0, &mut f);
            }
        }
    }

    /// Adresse eines Elements zu seiner Laufzeit-ID.
    pub fn find_element(&self, id: ElementId) -> Option<ElementPath> {
        let mut found = None;
        self.walk_elements(|_, path, e| {
            if found.is_none() && e.id == id {
                found = Some(path.clone());
            }
        });
        found
    }

    /// Element zu einer Adresse.
    pub fn element_at(&self, path: &ElementPath) -> Option<&Arc<Element>> {
        let layer = self.layer_for_path(path)?;
        let cel = layer.cels.get(path.cel)?;
        element_in_cel(cel, &path.steps)
    }

    /// Veränderlicher Zugriff (Copy-on-Write entlang des Pfades). Der Render-Cache bleibt; wer den Inhalt
    /// ändert, ruft [`Element::edit`] bzw. setzt `cache = None`. Bei der aktiven Seite wird die
    /// `MKBv`-Kopie verworfen.
    pub fn element_at_mut(&mut self, path: &ElementPath) -> Option<&mut Element> {
        if path.root == RootRef::Page(self.current_page) {
            self.root_copy = None;
        }
        let layer: &mut Layer = match path.root {
            RootRef::Page(i) => match Arc::make_mut(self.pages.get_mut(i)?)
                .root
                .layers
                .get_mut(path.slot)?
            {
                LayerSlot::Own(l) => Arc::make_mut(l),
                _ => return None,
            },
            RootRef::RootCopy => match Arc::make_mut(self.root_copy.as_mut()?)
                .root
                .layers
                .get_mut(path.slot)?
            {
                LayerSlot::Own(l) => Arc::make_mut(l),
                _ => return None,
            },
            RootRef::Symbol(k) => match Arc::make_mut(self.symbols.get_mut(k.0)?)
                .root
                .layers
                .get_mut(path.slot)?
            {
                LayerSlot::Own(l) => Arc::make_mut(l),
                _ => return None,
            },
            RootRef::SharedLayer(i) => {
                Arc::make_mut(self.shared_layers.get_mut(i)?.layer.as_mut()?)
            }
        };
        let cel = Arc::make_mut(layer.cels.get_mut(path.cel)?);
        element_in_cel_mut(cel, &path.steps)
    }

    fn layer_for_path(&self, path: &ElementPath) -> Option<&Arc<Layer>> {
        match path.root {
            RootRef::SharedLayer(i) => self.shared_layers.get(i)?.layer.as_ref(),
            r => match self.root(r)?.layers.get(path.slot)? {
                LayerSlot::Own(l) => Some(l),
                _ => None,
            },
        }
    }

    /// Alle Instanzen eines Symbols (Adresse und Element).
    pub fn instances_of(&self, id: SymbolId) -> Vec<(ElementPath, Arc<Element>)> {
        let mut v = Vec::new();
        self.walk_elements(|_, path, e| {
            if let ElementKind::Instance(i) = &e.kind
                && i.symbol_id == id
            {
                v.push((path.clone(), Arc::clone(e)));
            }
        });
        v
    }

    /// Ebene zu einer Laufzeit-ID (oberste Ebenen, Unterebenen, geteilte Ebenen, Symbole).
    pub fn find_layer(&self, id: LayerId) -> Option<Arc<Layer>> {
        let mut found: Option<Arc<Layer>> = None;
        let mut check_root = |root: &LayerRoot| {
            for s in &root.layers {
                if let LayerSlot::Own(l) = s
                    && found.is_none()
                {
                    find_layer_in(l, id, &mut found);
                }
            }
        };
        for p in &self.pages {
            check_root(&p.root);
        }
        for s in &self.symbols {
            check_root(&s.root);
        }
        for d in &self.shared_layers {
            if let Some(l) = &d.layer
                && found.is_none()
            {
                find_layer_in(l, id, &mut found);
            }
        }
        found
    }

    /// Größte vergebene Laufzeit-ID im Dokument (für [`crate::ids::IdGen::reserve_up_to`]).
    pub fn max_runtime_id(&self) -> u64 {
        let mut m = 0u64;
        self.walk_elements(|ctx, _, e| {
            m = m.max(e.id.0).max(ctx.layer.id.0);
            if let ElementKind::SubLayer(l) = &e.kind {
                m = m.max(l.id.0);
            }
        });
        m
    }

    /// Prüft Invarianten aus 04 §2.4; liefert Beschreibungen der Verstöße (keine Fehler: Das Original lädt
    /// auch solche Dateien).
    pub fn check_invariants(&self) -> Vec<String> {
        let mut errs = Vec::new();
        if self.pages.is_empty() {
            errs.push("Dokument ohne Seite".into());
        }
        if self.pages.iter().filter(|p| p.master()).count() > 1 {
            errs.push("mehr als eine Masterseite".into());
        }
        let check_root = |r: &LayerRoot, what: &str, errs: &mut Vec<String>| {
            let n = r.frame_count as usize;
            let mut webs = 0;
            for slot in &r.layers {
                if let LayerSlot::Own(l) = slot {
                    if l.kind == LayerKind::Web {
                        webs += 1;
                        if !l.shared_across_frames {
                            errs.push(format!("{what}: Web-Ebene nicht über Status geteilt"));
                        }
                    }
                    let ok = if l.shared_across_frames {
                        !l.cels.is_empty()
                    } else {
                        l.cels.len() == n
                    };
                    if !ok {
                        errs.push(format!(
                            "{what}: Ebene '{}' hat {} Cels (Status: {})",
                            l.name,
                            l.cels.len(),
                            n
                        ));
                    }
                }
            }
            if webs > 1 {
                errs.push(format!("{what}: mehrere Web-Ebenen"));
            }
        };
        for p in &self.pages {
            check_root(&p.root, p.display_name(), &mut errs);
        }
        for s in &self.symbols {
            check_root(&s.root, &s.name, &mut errs);
        }
        let mut unresolved = 0;
        self.walk_elements(|_, _, e| {
            if let ElementKind::Instance(i) = &e.kind
                && self.symbol_by_id(i.symbol_id).is_none()
            {
                unresolved += 1;
            }
        });
        if unresolved > 0 {
            errs.push(format!("{unresolved} Instanz(en) ohne Symbol"));
        }
        errs
    }
}

fn find_layer_in(l: &Arc<Layer>, id: LayerId, found: &mut Option<Arc<Layer>>) {
    if l.id == id {
        *found = Some(Arc::clone(l));
        return;
    }
    for c in &l.cels {
        for e in &c.elements {
            if let ElementKind::SubLayer(s) = &e.kind {
                find_layer_in(s, id, found);
                if found.is_some() {
                    return;
                }
            }
        }
    }
}

/// Traversiert alle Elemente einer Wurzel (eigene Ebenen; Verweise und Masterseite nicht aufgelöst).
pub fn walk_root(
    root: &LayerRoot,
    r: RootRef,
    f: &mut impl FnMut(&WalkCtx, &ElementPath, &Arc<Element>),
) {
    for (si, slot) in root.layers.iter().enumerate() {
        if let LayerSlot::Own(l) = slot {
            walk_layer(l, r, si, f);
        }
    }
}

fn walk_layer(
    l: &Layer,
    r: RootRef,
    slot: usize,
    f: &mut impl FnMut(&WalkCtx, &ElementPath, &Arc<Element>),
) {
    for (ci, cel) in l.cels.iter().enumerate() {
        let ctx = WalkCtx {
            root: r,
            layer: l,
            cel: ci,
            depth: 0,
        };
        let mut path = ElementPath {
            root: r,
            slot,
            cel: ci,
            steps: Vec::new(),
        };
        walk_list(&cel.elements, &ctx, &mut path, f);
    }
}

fn walk_list(
    list: &[Arc<Element>],
    ctx: &WalkCtx,
    path: &mut ElementPath,
    f: &mut impl FnMut(&WalkCtx, &ElementPath, &Arc<Element>),
) {
    for (i, e) in list.iter().enumerate() {
        path.steps.push(PathStep::Child(i));
        walk_one(e, ctx, path, f);
        path.steps.pop();
    }
}

fn walk_one(
    e: &Arc<Element>,
    ctx: &WalkCtx,
    path: &mut ElementPath,
    f: &mut impl FnMut(&WalkCtx, &ElementPath, &Arc<Element>),
) {
    f(ctx, path, e);
    let inner = WalkCtx {
        depth: ctx.depth + 1,
        ..*ctx
    };
    match &e.kind {
        ElementKind::Group(g) => {
            if let Some(m) = &g.mask {
                path.steps.push(PathStep::Mask);
                walk_one(m, &inner, path, f);
                path.steps.pop();
            }
            walk_list(&g.children, &inner, path, f);
            if let Some(r) = &g.replacement {
                for (i, x) in r.iter().enumerate() {
                    path.steps.push(PathStep::Replacement(i));
                    walk_one(x, &inner, path, f);
                    path.steps.pop();
                }
            }
        }
        ElementKind::CompoundShape(c) => {
            if let Some(r) = &c.result {
                path.steps.push(PathStep::Result);
                walk_one(r, &inner, path, f);
                path.steps.pop();
            }
            walk_list(&c.operands, &inner, path, f);
            if let Some(p) = &c.proxy {
                path.steps.push(PathStep::Proxy);
                walk_one(p, &inner, path, f);
                path.steps.pop();
            }
        }
        ElementKind::SubLayer(l) => {
            for (ci, cel) in l.cels.iter().enumerate() {
                path.steps.push(PathStep::SubLayerCel(ci));
                walk_list(&cel.elements, &inner, path, f);
                path.steps.pop();
            }
        }
        ElementKind::Text(t) => {
            if let Some(p) = t.on_path.as_ref().and_then(|p| p.path.as_ref()) {
                path.steps.push(PathStep::TextPath);
                walk_one(p, &inner, path, f);
                path.steps.pop();
            }
        }
        _ => {}
    }
}

fn element_in_cel<'a>(cel: &'a Cel, steps: &[PathStep]) -> Option<&'a Arc<Element>> {
    let mut list: &[Arc<Element>] = &cel.elements;
    let mut cur: Option<&Arc<Element>> = None;
    let mut i = 0;
    while i < steps.len() {
        match steps[i] {
            PathStep::Child(k) => cur = Some(list.get(k)?),
            PathStep::SubLayerCel(c) => {
                let ElementKind::SubLayer(l) = &cur?.kind else {
                    return None;
                };
                list = &l.cels.get(c)?.elements;
                i += 1;
                continue;
            }
            step => {
                let e = cur?;
                cur = Some(match (&e.kind, step) {
                    (ElementKind::Group(g), PathStep::Mask) => g.mask.as_ref()?,
                    (ElementKind::Group(g), PathStep::Replacement(k)) => {
                        g.replacement.as_ref()?.get(k)?
                    }
                    (ElementKind::CompoundShape(c), PathStep::Result) => c.result.as_ref()?,
                    (ElementKind::CompoundShape(c), PathStep::Proxy) => c.proxy.as_ref()?,
                    (ElementKind::Text(t), PathStep::TextPath) => {
                        t.on_path.as_ref()?.path.as_ref()?
                    }
                    _ => return None,
                });
            }
        }
        // Für den nächsten Child-Schritt: Kinderliste des aktuellen Elements.
        if let Some(e) = cur {
            list = e.children();
        }
        i += 1;
    }
    cur
}

fn element_in_cel_mut<'a>(cel: &'a mut Cel, steps: &[PathStep]) -> Option<&'a mut Element> {
    let (first, rest) = steps.split_first()?;
    let PathStep::Child(k) = *first else {
        return None;
    };
    let e = Arc::make_mut(cel.elements.get_mut(k)?);
    descend_mut(e, rest)
}

fn descend_mut<'a>(e: &'a mut Element, steps: &[PathStep]) -> Option<&'a mut Element> {
    let Some((first, rest)) = steps.split_first() else {
        return Some(e);
    };
    let next: &mut Element = match (&mut e.kind, *first) {
        (ElementKind::Group(g), PathStep::Child(k)) => Arc::make_mut(g.children.get_mut(k)?),
        (ElementKind::Group(g), PathStep::Mask) => Arc::make_mut(g.mask.as_mut()?),
        (ElementKind::Group(g), PathStep::Replacement(k)) => {
            Arc::make_mut(g.replacement.as_mut()?.get_mut(k)?)
        }
        (ElementKind::CompoundShape(c), PathStep::Child(k)) => {
            Arc::make_mut(c.operands.get_mut(k)?)
        }
        (ElementKind::CompoundShape(c), PathStep::Result) => Arc::make_mut(c.result.as_mut()?),
        (ElementKind::CompoundShape(c), PathStep::Proxy) => Arc::make_mut(c.proxy.as_mut()?),
        (ElementKind::Text(t), PathStep::TextPath) => {
            Arc::make_mut(t.on_path.as_mut()?.path.as_mut()?)
        }
        (ElementKind::SubLayer(l), PathStep::SubLayerCel(c)) => {
            let (second, rest2) = rest.split_first()?;
            let PathStep::Child(k) = *second else {
                return None;
            };
            let cel = Arc::make_mut(Arc::make_mut(l).cels.get_mut(c)?);
            let e2 = Arc::make_mut(cel.elements.get_mut(k)?);
            return descend_mut(e2, rest2);
        }
        _ => return None,
    };
    descend_mut(next, rest)
}
