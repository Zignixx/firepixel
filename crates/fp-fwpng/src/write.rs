//! Abbildung Dokumentmodell → Tag-Baum (`mkTS`).
//!
//! Neu erzeugte Objekte werden in der Reihenfolge und mit den Pflicht-Tags geschrieben, die das Original
//! schreibt (Reihenfolgen aus den CS6-Dateien des Korpus). Gelesene Objekte werden über ihre
//! Rundreise-Daten unverändert wiederhergestellt.

use crate::k;
use crate::schema::Wr;
use fp_model::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Was mit Render-Caches beim Schreiben geschieht.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Alle Caches des Modells schreiben (verlustfrei).
    #[default]
    Keep,
    /// Wie das Original (02 §12.9, Laufzeit T5): Pfad- und Compound-Shape-Caches weglassen, Caches nur für
    /// sichtbare Elemente; Gruppen- und Text-Caches nach den Schaltern.
    Original {
        save_group_caches: bool,
        save_text_caches: bool,
    },
    /// Keine Caches.
    None,
}

/// Schreibkontext: Kachelvergabe und Optionen.
pub struct WriteCtx {
    /// Kacheln in Vergabereihenfolge.
    pub tiles: Vec<(u32, Arc<Tile>)>,
    by_ptr: HashMap<usize, u32>,
    used: HashSet<u32>,
    next_id: u32,
    /// Caches unterdrücken (beim Erzeugen der `MKBv`-Kopie wie das Original).
    pub suppress_caches: bool,
    pub caches: CachePolicy,
    /// Statuszahl der gerade geschriebenen Wurzel: VIFv hat im Original je Status einen Eintrag
    /// (fehlen Einträge, verwirft das Original die Inhalte der übrigen Status).
    pub frames: u32,
}

impl WriteCtx {
    pub fn new(caches: CachePolicy, reserved: impl IntoIterator<Item = u32>) -> Self {
        let used: HashSet<u32> = reserved.into_iter().collect();
        let next_id = used
            .iter()
            .copied()
            .max()
            .unwrap_or(0x0073_4BC0)
            .max(0x0073_4BC0);
        WriteCtx {
            tiles: Vec::new(),
            by_ptr: HashMap::new(),
            used,
            next_id,
            suppress_caches: false,
            caches,
            frames: 1,
        }
    }

    fn tile_id(&mut self, t: &Arc<Tile>) -> u32 {
        let p = Arc::as_ptr(t) as *const u8 as usize;
        if let Some(id) = self.by_ptr.get(&p) {
            return *id;
        }
        let id = if t.id != 0 && !self.used.contains(&t.id) {
            t.id
        } else {
            loop {
                self.next_id = self.next_id.wrapping_add(1);
                if self.next_id != 0 && !self.used.contains(&self.next_id) {
                    break self.next_id;
                }
            }
        };
        self.used.insert(id);
        self.by_ptr.insert(p, id);
        self.tiles.push((id, Arc::clone(t)));
        id
    }

    fn cache_allowed(&self, e: &Element) -> bool {
        if self.suppress_caches {
            return false;
        }
        match self.caches {
            CachePolicy::Keep => true,
            CachePolicy::None => false,
            CachePolicy::Original {
                save_group_caches,
                save_text_caches,
            } => match &e.kind {
                ElementKind::Text(_) => save_text_caches,
                ElementKind::Group(_) => save_group_caches && e.common.visible,
                ElementKind::Instance(_) => e.common.visible,
                ElementKind::Path(_) | ElementKind::CompoundShape(_) => false,
                _ => e.common.visible,
            },
        }
    }
}

const LTRB: [TagKey; 4] = [k!(b"LFTf"), k!(b"TOPf"), k!(b"RITf"), k!(b"BOTf")];

/// Schreibt das Dokument als Wurzeln von `mkTS`.
pub fn write_document(doc: &Document, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut roots = Vec::new();
    if doc.single_root {
        if let Some(p) = doc.pages.first() {
            roots.push(RawNode::list(k!(b"MKBv"), write_page(p, ctx, Some(doc))));
        }
    } else {
        match &doc.root_copy {
            Some(copy) => roots.push(RawNode::list(k!(b"MKBv"), write_page(copy, ctx, Some(doc)))),
            None => {
                let copy = make_root_copy(doc);
                let prev = ctx.suppress_caches;
                ctx.suppress_caches = true;
                roots.push(RawNode::list(
                    k!(b"MKBv"),
                    write_page(&copy, ctx, Some(doc)),
                ));
                ctx.suppress_caches = prev;
            }
        }
        for p in &doc.pages {
            roots.push(RawNode::list(k!(b"PDCv"), write_page(p, ctx, None)));
        }
    }
    roots.extend(doc.extra_roots.iter().cloned());
    roots
}

/// Erzeugt die `MKBv`-Kompatibilitätskopie aus der aktiven Seite (02 §3.2): Web-Ebene zuerst, geteilte
/// Ebenen und Masterseiten-Ebenen als gewöhnliche Ebenen ausgerollt, `ILLb{0}`, GUID der Hauptwurzel.
pub fn make_root_copy(doc: &Document) -> Page {
    let mut ids = IdGen::new();
    let Some(active) = doc.active_page() else {
        return Page::new(&mut ids, "", 500.0, 500.0, Argb(0xffff_ffff));
    };
    let mut p = (**active).clone();
    p.id = doc.root_id.clone().or_else(|| Some(Guid::new_random()));
    p.compat_layer_copy = Some(false);
    p.last_export_dir = None;
    p.last_export_file = None;
    p.local_shared_layers = None;
    let mut layers: Vec<LayerSlot> = Vec::new();
    for slot in &active.root.layers {
        match slot {
            LayerSlot::Shared(r) => {
                if let Some(l) = doc.shared_layer(&r.id).and_then(|d| d.layer.as_ref()) {
                    layers.push(LayerSlot::Own(Arc::clone(l)));
                }
            }
            LayerSlot::MasterPage(m) => {
                let mut cel = Cel::empty();
                if let Some((_, mp)) = m.master_id().and_then(|id| doc.page_by_id(id)) {
                    for ms in mp.root.layers.iter().rev() {
                        if let LayerSlot::Own(l) = ms
                            && l.kind == LayerKind::Normal
                            && let Some(c) = l.cels.first()
                        {
                            cel.elements.extend(c.elements.iter().cloned());
                        }
                    }
                }
                let mut l = Layer::new(&mut ids, &m.name, LayerKind::Normal, 1);
                l.cels = vec![Arc::new(cel)];
                l.shared_across_frames = true;
                layers.push(LayerSlot::Own(Arc::new(l)));
            }
            other => layers.push(other.clone()),
        }
    }
    if let Some(pos) = layers
        .iter()
        .position(|s| matches!(s, LayerSlot::Own(l) if l.kind == LayerKind::Web))
    {
        let web = layers.remove(pos);
        layers.insert(0, web);
    }
    p.root.layers = layers;
    p.root.layers_extra = Extra::default();
    p
}

/// Kinder einer Seite (`MKBv` mit `globals`, sonst `PDCv`).
pub fn write_page(p: &Page, ctx: &mut WriteCtx, globals: Option<&Document>) -> Vec<RawNode> {
    let mut w = Wr::new(&p.extra);
    w.opt_int(k!(b"VRSi"), p.format_version);
    w.opt_int(k!(b"VRMi"), p.format_minor);
    let n = p.root.frame_count_repeat.unwrap_or(2);
    write_frc(&mut w, &p.root, n);
    w.opt_int(k!(b"RQNi"), p.doc_random);
    let c = &p.canvas;
    w.f32_def(k!(b"XLCf"), c.origin.x, 0.0, true);
    w.f32_def(k!(b"YLCf"), c.origin.y, 0.0, true);
    w.f32_def(k!(b"WIDf"), c.width, 0.0, true);
    w.f32_def(k!(b"HITf"), c.height, 0.0, true);
    w.f32_def(k!(b"RESf"), c.resolution, 72.0, true);
    w.i32_def(k!(b"RSUi"), c.resolution_unit, 0, true);
    w.int_def(k!(b"BGCi"), p.root.background.0, 0xffff_ffff, true);
    write_presets(&mut w, &p.presets, ctx);
    if let Some(g) = globals {
        let syms: Vec<RawNode> = g
            .symbols
            .iter()
            .map(|s| RawNode::list(k!(b"MSTv"), write_symbol(s, ctx)))
            .collect();
        w.list(
            k!(b"MSLv"),
            crate::schema::arrange_list(syms, &g.symbols_extra),
        );
        if let Some(st) = &g.styles {
            let v: Vec<RawNode> = st
                .iter()
                .map(|s| RawNode::list(k!(b"ZSTv"), write_style(s, ctx)))
                .collect();
            w.list(k!(b"ZSLv"), crate::schema::arrange(v, &g.styles_extra));
        }
    }
    if let Some(a) = &p.tool_attrs {
        w.list(k!(b"PATv"), write_attrs(a, ctx));
    }
    if let Some(e) = &p.root.export {
        w.list(k!(b"EXPv"), write_export(e));
    }
    if let Some(m) = &p.image_map {
        let mut s = Wr::new(&m.extra);
        s.opt_str(k!(b"MNDs"), m.name.as_deref());
        w.list(k!(b"MNFv"), s.finish());
    }
    w.opt_str(k!(b"PGNs"), p.name.as_deref());
    w.opt_bool(k!(b"IMPb"), p.is_master);
    w.opt_bool(k!(b"PILb"), p.linked_to_master);
    if let Some(id) = &p.id {
        w.str(k!(b"PIDs"), &id.0);
    }
    match globals {
        Some(g) => {
            if g.has_shared_layer_list || !g.shared_layers.is_empty() {
                let v: Vec<RawNode> = g
                    .shared_layers
                    .iter()
                    .map(|d| write_shared_def(d, ctx))
                    .collect();
                w.list(
                    k!(b"LSMv"),
                    crate::schema::arrange(v, &g.shared_layers_extra),
                );
            }
        }
        None => {
            if let Some(defs) = &p.local_shared_layers {
                let v: Vec<RawNode> = defs.iter().map(|d| write_shared_def(d, ctx)).collect();
                w.list(
                    k!(b"LSMv"),
                    crate::schema::arrange(v, &p.local_shared_layers_extra),
                );
            }
        }
    }
    w.opt_bool(k!(b"ILLb"), p.compat_layer_copy);
    if p.root.has_layer_list || w.is_new() {
        ctx.frames = p.root.frame_count;
        w.list(
            k!(b"LYLv"),
            write_layer_list(&p.root.layers, &p.root.layers_extra, ctx),
        );
    }
    if let Some(g) = &p.root.grid {
        w.list(k!(b"GRDv"), write_grid(g));
    }
    if let Some(g) = &p.root.guides {
        w.list(k!(b"GDSv"), write_guides(g));
    }
    w.opt_str(k!(b"LXDs"), p.last_export_dir.as_deref());
    w.opt_str(k!(b"LXFs"), p.last_export_file.as_deref());
    w.i32_def(k!(b"OSBi"), p.root.onion_before, 0, true);
    w.i32_def(k!(b"OSAi"), p.root.onion_after, 0, true);
    if let Some(s) = &p.selection {
        w.list(k!(b"MSVv"), write_selection(s, ctx));
    }
    if let Some(v) = &p.saved_selections {
        let kids: Vec<RawNode> = v
            .iter()
            .map(|s| RawNode::list(k!(b"OSSv"), write_selection(s, ctx)))
            .collect();
        w.list(
            k!(b"MSSv"),
            crate::schema::arrange(kids, &p.saved_selections_extra),
        );
    }
    if let Some(s) = &p.jpeg_mask {
        w.list(k!(b"MJPv"), write_selection(s, ctx));
    }
    if let Some(b) = &p.printer_data {
        w.node(RawNode::new(k!(b"PTRy"), RawValue::Bytes(b.clone())));
    }
    w.opt_i32(k!(b"FLPi"), p.legacy_loop);
    if let Some(e) = &p.legacy_export {
        w.list(k!(b"EXSv"), write_export(e));
    }
    w.finish()
}

/// `FRCi` so oft wie gelesen bzw. wie das Original (`n`).
fn write_frc(w: &mut Wr, root: &LayerRoot, n: u32) {
    let count = if w.is_new() {
        n as usize
    } else {
        w.had_count(k!(b"FRCi"))
    };
    for _ in 0..count.max(if w.is_new() { 1 } else { 0 }) {
        w.int(k!(b"FRCi"), root.frame_count);
    }
}

fn write_presets(w: &mut Wr, p: &Presets, ctx: &mut WriteCtx) {
    if let Some(v) = &p.brushes {
        let kids = v
            .iter()
            .map(|b| RawNode::list(k!(b"BPLv"), write_brush(b)))
            .collect();
        w.list(k!(b"PLLv"), crate::schema::arrange(kids, &p.brushes_extra));
    }
    if let Some(v) = &p.fills {
        let kids = v
            .iter()
            .map(|f| RawNode::list(k!(b"FPLv"), write_fill(f, ctx)))
            .collect();
        w.list(k!(b"FLDv"), crate::schema::arrange(kids, &p.fills_extra));
    }
    if let Some(v) = &p.gradients {
        let kids = v
            .iter()
            .map(|g| RawNode::list(k!(b"FGLv"), write_gradient(g)))
            .collect();
        w.list(
            k!(b"GLDv"),
            crate::schema::arrange(kids, &p.gradients_extra),
        );
    }
    if let Some(v) = &p.effects {
        let kids = v
            .iter()
            .map(|e| RawNode::list(k!(b"EFLv"), write_effect_list(e)))
            .collect();
        w.list(k!(b"EFDv"), crate::schema::arrange(kids, &p.effects_extra));
    }
}

fn write_grid(g: &Grid) -> Vec<RawNode> {
    let mut w = Wr::new(&g.extra);
    w.f32_def(k!(b"GOXf"), g.origin.x, 0.0, true);
    w.f32_def(k!(b"GOYf"), g.origin.y, 0.0, true);
    w.f32_def(k!(b"GSXf"), g.size.x, 36.0, true);
    w.f32_def(k!(b"GSYf"), g.size.y, 36.0, true);
    w.int_def(k!(b"GCLi"), g.color.0, 0xff66_6666, true);
    w.finish()
}

fn write_guides(g: &Guides) -> Vec<RawNode> {
    let mut w = Wr::new(&g.extra);
    w.opt_bool(k!(b"GDLb"), g.locked);
    w.opt_int(k!(b"GDKi"), g.color.map(|c| c.0));
    w.list(
        k!(b"GDHv"),
        write_float_list(&g.horizontal, k!(b"GDCf"), &g.horizontal_extra),
    );
    w.list(
        k!(b"GDVv"),
        write_float_list(&g.vertical, k!(b"GDCf"), &g.vertical_extra),
    );
    w.finish()
}

fn write_float_list(v: &[f32], key: TagKey, extra: &Extra) -> Vec<RawNode> {
    let mut used = vec![false; extra.shadow.len()];
    let kids: Vec<RawNode> = v
        .iter()
        .map(|x| {
            for (i, s) in extra.shadow.iter().enumerate() {
                if !used[i] && s.key == key && s.as_num().map(|f| f as f32) == Some(*x) {
                    used[i] = true;
                    return s.clone();
                }
            }
            RawNode::num(key, *x as f64)
        })
        .collect();
    let clean = Extra {
        nodes: extra.nodes.clone(),
        order: extra.order.clone(),
        shadow: Vec::new(),
    };
    crate::schema::arrange(kids, &clean)
}

fn write_selection(s: &SavedSelection, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&s.extra);
    if let Some(m) = &s.mask {
        w.list(k!(b"MSKv"), write_mask(m, ctx));
    }
    w.f32_def(k!(b"LFTf"), s.left, 0.0, true);
    w.f32_def(k!(b"TOPf"), s.top, 0.0, true);
    w.opt_str(k!(b"NMSs"), s.name.as_deref());
    w.finish()
}

fn write_shared_def(d: &SharedLayerDef, ctx: &mut WriteCtx) -> RawNode {
    let mut w = Wr::new(&d.extra);
    w.str_def(k!(b"UIDs"), &d.id.0, "", true);
    w.bool_def(k!(b"ISPb"), d.to_all_pages, false, true);
    if let Some(l) = &d.layer {
        w.list(k!(b"LAYv"), write_layer(l, ctx));
    }
    RawNode::list(k!(b"LSEv"), w.finish())
}

/// Ebenenliste `LYLv`.
pub fn write_layer_list(slots: &[LayerSlot], extra: &Extra, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let kids: Vec<RawNode> = slots
        .iter()
        .map(|s| match s {
            LayerSlot::Own(l) => RawNode::list(k!(b"LAYv"), write_layer(l, ctx)),
            LayerSlot::Shared(r) => {
                let mut w = Wr::new(&r.extra);
                w.str_def(k!(b"UIDs"), &r.id.0, "", true);
                w.bool_def(k!(b"ISPb"), r.to_all_pages, false, true);
                RawNode::list(k!(b"LSAv"), w.finish())
            }
            LayerSlot::MasterPage(m) => {
                let mut w = Wr::new(&m.extra);
                w.str_def(k!(b"PIDs"), &m.master.0, "", true);
                w.bool_def(k!(b"DISb"), m.expanded, true, true);
                w.str_def(k!(b"LNMs"), &m.name, "", true);
                if m.has_visibility_node || w.is_new() {
                    w.list(
                        k!(b"VIFv"),
                        write_visibility(&padded(&m.visibility, ctx.frames), &m.visibility_extra),
                    );
                }
                RawNode::list(k!(b"MPLv"), w.finish())
            }
            LayerSlot::Raw(n) => n.clone(),
        })
        .collect();
    crate::schema::arrange_list(kids, extra)
}

/// Sichtbarkeit je Status auf die Statuszahl auffüllen (letzter Wert bzw. sichtbar).
fn padded(v: &[bool], frames: u32) -> Vec<bool> {
    let mut out = v.to_vec();
    if out.len() < frames as usize {
        let last = out.last().copied().unwrap_or(true);
        out.resize(frames as usize, last);
    }
    out
}

fn write_visibility(v: &[bool], extra: &Extra) -> Vec<RawNode> {
    let kids: Vec<RawNode> = if v.is_empty() && extra.order.is_none() {
        vec![RawNode::bool(k!(b"VISb"), true)]
    } else {
        v.iter().map(|b| RawNode::bool(k!(b"VISb"), *b)).collect()
    };
    crate::schema::arrange(kids, extra)
}

/// Ebene `LAYv`/`LSLv` (Kinder).
pub fn write_layer(l: &Layer, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&l.extra);
    w.bool_def(k!(b"BKGb"), l.background, false, true);
    w.bool_def(k!(b"URLb"), l.kind == LayerKind::Web, false, true);
    w.str_def(k!(b"LNMs"), &l.name, "", true);
    w.i32_def(k!(b"LLLi"), l.level, 0, true);
    w.bool_def(k!(b"MSLb"), l.legacy_shared, false, true);
    let msl_eff = if w.had(k!(b"MSLb")) || w.is_new() {
        l.legacy_shared
    } else {
        true
    };
    let mcl = !l.shared_across_frames;
    w.bool_def(k!(b"MCLb"), mcl, !msl_eff, true);
    w.bool_def(k!(b"DISb"), l.expanded, true, true);
    w.opacity(k!(b"OPAi"), l.opacity);
    w.blend(k!(b"BLDi"), l.blend);
    let cels: Vec<RawNode> = l
        .cels
        .iter()
        .map(|c| RawNode::list(k!(b"CELv"), write_cel(c, ctx)))
        .collect();
    w.list(
        k!(b"CLLv"),
        crate::schema::arrange_list(cels, &l.cels_extra),
    );
    if l.has_visibility_node || w.is_new() {
        w.list(
            k!(b"VIFv"),
            write_visibility(&padded(&l.visibility, ctx.frames), &l.visibility_extra),
        );
    }
    w.finish()
}

fn write_cel(c: &Cel, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&c.extra);
    w.bool_def(k!(b"VISb"), c.visible, true, false);
    w.bool_def(k!(b"LCKb"), c.locked, false, true);
    w.list(
        k!(b"ELMv"),
        write_elements(&c.elements, &c.elements_extra, ctx),
    );
    w.finish()
}

/// Elementliste `ELMv` (inklusive Stellvertreter-Pfade hinter Compound Shapes).
pub fn write_elements(list: &[Arc<Element>], extra: &Extra, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut kids = Vec::with_capacity(list.len());
    for e in list {
        // Ganz leere Bitmaps (0×0) verwirft das Original beim Speichern; das Dokument behält sie.
        if let ElementKind::Bitmap(b) = &e.kind
            && (b.image.grid.width == 0 || b.image.grid.height == 0)
        {
            continue;
        }
        kids.push(write_element(e, ctx));
        if let ElementKind::CompoundShape(c) = &e.kind {
            match &c.proxy {
                Some(p) => {
                    let mut n = write_element(p, ctx);
                    set_proxy_flag(&mut n);
                    kids.push(n);
                }
                None => {
                    if let Some(r) = &c.result {
                        let mut n = write_element(r, ctx);
                        set_proxy_flag(&mut n);
                        kids.push(n);
                    }
                }
            }
        }
    }
    crate::schema::arrange_list(kids, extra)
}

/// Setzt `PRPb{1}` in einem `PTHv` (hinter `RNDi`, wie das Original), falls noch nicht vorhanden.
fn set_proxy_flag(n: &mut RawNode) {
    if let RawValue::List(kids) = &mut n.value {
        if kids.iter().any(|c| c.key == k!(b"PRPb")) {
            for c in kids.iter_mut() {
                if c.key == k!(b"PRPb") {
                    c.value = RawValue::Bool(true);
                }
            }
            return;
        }
        let pos = kids
            .iter()
            .position(|c| c.key == k!(b"RNDi"))
            .map(|p| p + 1)
            .unwrap_or(kids.len());
        kids.insert(pos, RawNode::bool(k!(b"PRPb"), true));
    }
}

fn w_vis(w: &mut Wr, c: &CommonAttrs) {
    w.bool_def(k!(b"VISb"), c.visible, true, false);
}
fn w_lck(w: &mut Wr, c: &CommonAttrs) {
    w.bool_def(k!(b"LCKb"), c.locked, false, true);
}
fn w_efl(w: &mut Wr, c: &CommonAttrs) {
    if let Some(e) = &c.effects {
        w.list(k!(b"EFLv"), write_effect_list(e));
    }
}
fn w_names(w: &mut Wr, c: &CommonAttrs) {
    w.opt_str(k!(b"OBNs"), c.name.as_deref());
    w.opt_str(k!(b"ZSNs"), c.style.as_deref());
}
fn w_dta(w: &mut Wr, c: &CommonAttrs) {
    if w.want(
        k!(b"DTAv"),
        c.custom_data.entries.is_empty() && c.custom_data.extra.nodes.is_empty(),
        true,
    ) {
        w.list(k!(b"DTAv"), write_dict(&c.custom_data));
    }
}
fn w_cache(w: &mut Wr, e: &Element, ctx: &mut WriteCtx) {
    if let Some(c) = &e.cache
        && ctx.cache_allowed(e)
    {
        w.list(
            k!(b"IMGv"),
            write_bitmap_node(&c.common, &c.bitmap, &c.extra, ctx),
        );
    }
}

/// Ein Element als Knoten.
pub fn write_element(e: &Element, ctx: &mut WriteCtx) -> RawNode {
    let c = &e.common;
    match &e.kind {
        ElementKind::Raw(n) => n.clone(),
        ElementKind::SubLayer(l) => RawNode::list(k!(b"LSLv"), write_layer(l, ctx)),
        ElementKind::Bitmap(b) => {
            RawNode::list(k!(b"IMGv"), write_bitmap_node(c, b, &e.extra, ctx))
        }
        ElementKind::Path(p) => {
            let mut w = Wr::new(&e.extra);
            w_vis(&mut w, c);
            w_lck(&mut w, c);
            w.list(k!(b"PATv"), write_attrs(&p.attrs, ctx));
            w.i32_def(k!(b"BRPi"), p.stroke_placement, 1, true);
            w.i32_def(k!(b"PTTi"), p.ptt, 0, true);
            w.bool_def(k!(b"FOTb"), p.fill_on_top, false, true);
            w_efl(&mut w, c);
            w.int_def(k!(b"RNDi"), p.random_seed, 0, true);
            w.bool_def(k!(b"PRPb"), p.compat_proxy, false, false);
            w.point_def(
                k!(b"TOXf"),
                k!(b"TOYf"),
                p.texture_offset,
                Point::default(),
                true,
            );
            w.blend(k!(b"BLDi"), c.blend);
            w.opacity(k!(b"OPAi"), c.opacity);
            write_fill_handles(&mut w, &p.fill_handles, true);
            let cs: Vec<RawNode> = p
                .contours
                .iter()
                .map(|c| RawNode::list(k!(b"PBPv"), write_contour(c)))
                .collect();
            w.list(
                k!(b"PBLv"),
                crate::schema::arrange_list(cs, &p.contours_extra),
            );
            if let Some(pl) = &p.polyline {
                let kids: Vec<RawNode> = pl
                    .iter()
                    .map(|x| RawNode::list(k!(b"PPLv"), write_polyline(x)))
                    .collect();
                w.list(
                    k!(b"PCLv"),
                    crate::schema::arrange_list(kids, &p.polyline_extra),
                );
            }
            if let Some(pr) = &p.primitive {
                w.list(k!(b"PRIv"), write_primitive(pr, c.effects.as_ref()));
            }
            w_names(&mut w, c);
            w_dta(&mut w, c);
            w.int_def(k!(b"POPi"), p.path_op, 0, false);
            w.bool_def(k!(b"EOFb"), p.even_odd, false, true);
            w.bool_def(k!(b"AGMb"), p.agm, false, true);
            w_cache(&mut w, e, ctx);
            RawNode::list(k!(b"PTHv"), w.finish())
        }
        ElementKind::Text(t) => RawNode::list(k!(b"TXTv"), write_text(e, t, ctx)),
        ElementKind::Group(g) => {
            let mut w = Wr::new(&e.extra);
            w_vis(&mut w, c);
            w_lck(&mut w, c);
            let mf = &g.mask_flags;
            w.bool_def(k!(b"LNKb"), mf.linked, true, false);
            w.bool_def(k!(b"MENb"), mf.enabled, true, false);
            w.bool_def(k!(b"MNAb"), mf.hide_attrs, false, false);
            w.i32_def(k!(b"GRTi"), g.kind.code(), 0, true);
            w.bool_def(k!(b"MRXb"), mf.auto_expand, false, false);
            let mut all: Vec<Arc<Element>> = Vec::with_capacity(g.children.len() + 1);
            if let Some(m) = &g.mask {
                all.push(Arc::clone(m));
            }
            all.extend(g.children.iter().cloned());
            w.list(k!(b"ELMv"), write_elements(&all, &g.children_extra, ctx));
            w.blend(k!(b"BLDi"), c.blend);
            w.opacity(k!(b"OPAi"), c.opacity);
            if let Some(r) = &g.replacement {
                w.list(k!(b"MRMv"), write_elements(r, &g.replacement_extra, ctx));
            }
            w_efl(&mut w, c);
            w_names(&mut w, c);
            if let Some(s) = &g.smart_shape {
                w.list(k!(b"ISSv"), write_smart_shape(s));
            }
            w_dta(&mut w, c);
            w_cache(&mut w, e, ctx);
            RawNode::list(k!(b"GRPv"), w.finish())
        }
        ElementKind::CompoundShape(cs) => {
            let mut w = Wr::new(&e.extra);
            w_vis(&mut w, c);
            w_lck(&mut w, c);
            w.i32_def(k!(b"GRTi"), cs.group_type, 0, true);
            if let Some(r) = &cs.result {
                let n = write_element(r, ctx);
                w.list(k!(b"PTHv"), n.children().to_vec());
            }
            w.list(
                k!(b"ELMv"),
                write_elements(&cs.operands, &cs.operands_extra, ctx),
            );
            w.blend(k!(b"BLDi"), c.blend);
            w.opacity(k!(b"OPAi"), c.opacity);
            w_efl(&mut w, c);
            w_names(&mut w, c);
            w_dta(&mut w, c);
            w_cache(&mut w, e, ctx);
            RawNode::list(k!(b"CSGv"), w.finish())
        }
        ElementKind::Instance(i) => {
            let mut w = Wr::new(&e.extra);
            w_vis(&mut w, c);
            w.int(k!(b"MSNi"), i.symbol_id.0);
            w.opt_int(k!(b"ALNi"), i.instance_id);
            w.matrix(k!(b"MTXv"), &i.transform);
            w_lck(&mut w, c);
            w.blend(k!(b"BLDi"), c.blend);
            w.opacity(k!(b"OPAi"), c.opacity);
            w.i32_def(k!(b"XFMi"), i.transform_mode, 1, true);
            w_efl(&mut w, c);
            w_names(&mut w, c);
            w_dta(&mut w, c);
            let a = &i.anim;
            w.int_def(k!(b"ANNi"), a.frames, 5, true);
            w.f32_def(k!(b"ANXf"), a.offset.x, 0.0, true);
            w.f32_def(k!(b"ANYf"), a.offset.y, 0.0, true);
            w.i32_def(k!(b"ANRi"), a.rotation, 0, false);
            w.f32_def(k!(b"ASCf"), a.scale, 100.0, true);
            w.int_def(k!(b"ASOi"), a.start_opacity, 1000, false);
            w.int_def(k!(b"AEOi"), a.end_opacity, 1000, false);
            w.opt_str(k!(b"WPVs"), i.widget_params.as_deref());
            w.opt_str(k!(b"FILs"), i.button.file_name.as_deref());
            w.opt_str(k!(b"BUTs"), i.button.text.as_deref());
            w.opt_str(k!(b"ALTs"), i.button.target.as_deref());
            w.opt_str(k!(b"A2Ts"), i.button.alt.as_deref());
            w.opt_str(k!(b"INMs"), i.button.url.as_deref());
            w.opt_bool(k!(b"DVLb"), i.show_down_on_load);
            if let Some(b) = &i.behaviors {
                w.list(k!(b"BEVv"), write_behaviors(b, ctx));
            }
            if let Some(m) = &i.alt_transform {
                w.matrix(k!(b"ATMv"), m);
            }
            w_cache(&mut w, e, ctx);
            RawNode::list(k!(b"ALSv"), w.finish())
        }
        ElementKind::WebObject(u) => {
            let mut w = Wr::new(&e.extra);
            w.opt_str(k!(b"INMs"), u.url.as_deref());
            w.opt_str(k!(b"A2Ts"), u.alt_text.as_deref());
            w.opt_str(k!(b"ALTs"), u.target.as_deref());
            w.int_def(k!(b"FCLi"), u.color.0, 0xff00_0000, true);
            w.i32_def(k!(b"URSi"), u.shape.code(), 0, true);
            w.rect4_def(LTRB, u.bounds, Rect::default(), true);
            if let Some(p) = &u.polygon {
                w.list(k!(b"PBPv"), write_contour(p));
            }
            if let Some(b) = &u.behaviors {
                w.list(k!(b"BEVv"), write_behaviors(b, ctx));
            }
            w.opt_str(k!(b"FILs"), u.file_name.as_deref());
            if let Some(s) = &u.slice {
                w.opt_str(k!(b"HTMs"), s.html.as_deref());
                w.opt_str(k!(b"TDTs"), s.td_text.as_deref());
                w.i32(k!(b"TSLi"), s.slice_type);
                w.opt_int(k!(b"MSNi"), s.slice_id);
                if let Some(x) = &s.export {
                    w.list(k!(b"EXPv"), write_export(x));
                }
                if let Some(b) = &s.background {
                    w.bool(k!(b"CBTb"), b.is_background);
                    w.i32(k!(b"CBRi"), b.repeat);
                    w.i32(k!(b"CBAi"), b.attachment);
                    w.i32(k!(b"CBHi"), b.h_align);
                    w.i32(k!(b"CHVi"), b.h_value);
                    w.i32(k!(b"CBVi"), b.v_align);
                    w.i32(k!(b"CVVi"), b.v_value);
                }
            }
            w.blend(k!(b"BLDi"), c.blend);
            w.opacity(k!(b"OPAi"), c.opacity);
            w_efl(&mut w, c);
            w_names(&mut w, c);
            w_dta(&mut w, c);
            w_vis(&mut w, c);
            w_lck(&mut w, c);
            w.opt_str(k!(b"UNMs"), u.legacy_status.as_deref());
            RawNode::list(k!(b"URLv"), w.finish())
        }
    }
}

/// `IMGv`-Kinder (Bitmap-Element, Cache, Effekt-Ergebnis, Musterbild).
pub fn write_bitmap_node(
    c: &CommonAttrs,
    b: &BitmapElem,
    extra: &Extra,
    ctx: &mut WriteCtx,
) -> Vec<RawNode> {
    let mut w = Wr::new(extra);
    w_vis(&mut w, c);
    w_lck(&mut w, c);
    let img = &b.image;
    w.f32_def(k!(b"XOFf"), img.offset.x, 0.0, true);
    w.f32_def(k!(b"YOFf"), img.offset.y, 0.0, true);
    w.f32_def(k!(b"XLCf"), img.origin.x, 0.0, true);
    w.f32_def(k!(b"YLCf"), img.origin.y, 0.0, true);
    write_tile_grid(&mut w, &img.grid, ctx, 128);
    w_efl(&mut w, c);
    w.blend(k!(b"BLDi"), c.blend);
    w.opacity(k!(b"OPAi"), c.opacity);
    w_names(&mut w, c);
    w_dta(&mut w, c);
    if let Some(r) = &b.effect_result
        && !ctx.suppress_caches
    {
        w.list(
            k!(b"IMGv"),
            write_bitmap_node(&r.common, &r.bitmap, &r.extra, ctx),
        );
    }
    w.finish()
}

fn write_tile_grid(w: &mut Wr, g: &TileGrid, ctx: &mut WriteCtx, default_ts: u32) {
    w.int_def(k!(b"WPXi"), g.width, 0, true);
    w.int_def(k!(b"HPXi"), g.height, 0, true);
    w.int_def(k!(b"TSZi"), g.tile_size, default_ts, true);
    let kids: Vec<RawNode> = g
        .tiles
        .iter()
        .map(|t| match t {
            TileEntry::Tile(t) => RawNode::int(k!(b"TIDi"), ctx.tile_id(t)),
            TileEntry::Missing(id) => {
                ctx.used.insert(*id);
                RawNode::int(k!(b"TIDi"), *id)
            }
            TileEntry::Solid(c) => RawNode::int(k!(b"TMCi"), c.0),
            TileEntry::SolidMask(v) => RawNode::int(k!(b"TMMi"), *v),
        })
        .collect();
    w.list(k!(b"TILv"), crate::schema::arrange(kids, &g.tiles_extra));
}

fn write_mask(m: &MaskData, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&m.extra);
    write_tile_grid(&mut w, &m.grid, ctx, 256);
    w.finish()
}

fn write_fill_handles(w: &mut Wr, h: &FillHandles, with_pcf: bool) {
    if with_pcf {
        w.bool_def(k!(b"PCFb"), h.valid, false, true);
    }
    w.point_def(k!(b"PSXf"), k!(b"PSYf"), h.start, Point::default(), true);
    w.point_def(k!(b"PEXf"), k!(b"PEYf"), h.end, Point::default(), true);
    w.point_def(k!(b"PFXf"), k!(b"PFYf"), h.second, Point::default(), true);
}

/// Kontur `PBPv` (Kinder).
pub fn write_contour(c: &Contour) -> Vec<RawNode> {
    let mut w = Wr::new(&c.extra);
    w.bool_def(k!(b"ISCb"), c.closed, false, true);
    w.opt_f32(k!(b"BSLf"), c.bsl);
    w.int(k!(b"PPCi"), c.nodes.len() as u32);
    for n in &c.nodes {
        w.list(k!(b"PBTv"), write_contour_node(n));
    }
    w.finish()
}

fn write_contour_node(n: &ContourNode) -> Vec<RawNode> {
    let mut w = Wr::new(&n.extra);
    w.f32_def(k!(b"XLCf"), n.anchor.x, 0.0, true);
    w.f32_def(k!(b"YLCf"), n.anchor.y, 0.0, true);
    w.f32_def(k!(b"XPCf"), n.pred.x, n.anchor.x, false);
    w.f32_def(k!(b"YPCf"), n.pred.y, n.anchor.y, false);
    w.f32_def(k!(b"XSCf"), n.succ.x, n.anchor.x, false);
    w.f32_def(k!(b"YSCf"), n.succ.y, n.anchor.y, false);
    w.bool_def(k!(b"CRVb"), n.curve, false, false);
    w.opt_int(k!(b"RNDi"), n.random);
    w.opt_f32(k!(b"NSDf"), n.segment_length);
    if let Some(p) = &n.pressure {
        let mut s = Wr::new(&p.extra);
        s.opt_int(k!(b"BZCi"), p.count);
        for z in &p.samples {
            let mut q = Wr::new(&z.extra);
            q.opt_f32(k!(b"PRSf"), z.pressure);
            q.opt_f32(k!(b"VELf"), z.velocity);
            s.list(k!(b"BAZv"), q.finish());
        }
        w.list(k!(b"BZLv"), s.finish());
    }
    w.finish()
}

fn write_polyline(p: &Polyline) -> Vec<RawNode> {
    let mut w = Wr::new(&p.extra);
    w.bool_def(k!(b"ISCb"), p.closed, false, true);
    w.int(k!(b"PPCi"), p.points.len() as u32);
    for pt in &p.points {
        let mut q = Wr::new(&pt.extra);
        q.f32_def(k!(b"XLCf"), pt.pos.x, 0.0, true);
        q.f32_def(k!(b"YLCf"), pt.pos.y, 0.0, true);
        q.f32_def(k!(b"PRSf"), pt.pressure, 0.0, true);
        q.f32_def(k!(b"VELf"), pt.velocity, 0.0, true);
        q.int_def(k!(b"DURi"), pt.duration, 0, true);
        q.int_def(k!(b"RNDi"), pt.random, 0, true);
        q.opt_bool(k!(b"ONCb"), pt.on_node);
        w.list(k!(b"PPTv"), q.finish());
    }
    w.finish()
}

/// Rechteck-Grundform. Das Original liest die Effekte aus PRIv/EFLv; im Korpus ist diese Liste
/// immer gleich der des Pfads (6.729/6.729), daher wird die Pfadliste geschrieben (eine veraltete
/// Kopie im Modell kann so den Effekt nicht verlieren).
fn write_primitive(p: &RectPrimitive, path_effects: Option<&fp_model::EffectList>) -> Vec<RawNode> {
    let mut w = Wr::new(&p.extra);
    write_fill_handles(&mut w, &p.fill_handles, true);
    w.matrix(k!(b"MTXv"), &p.transform);
    if let Some(e) = path_effects {
        w.list(k!(b"EFLv"), write_effect_list(e));
    }
    let mut r = Wr::new(&p.rect_extra);
    r.f32_def(k!(b"RDSf"), p.roundness, 0.0, true);
    r.rect4_def(LTRB, p.rect, Rect::default(), true);
    w.list(k!(b"RCTv"), r.finish());
    w.opt_bool(k!(b"RDMb"), p.roundness_pixels);
    w.finish()
}

/// Attribute `PATv` (Kinder).
pub fn write_attrs(a: &PathAttrs, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&a.extra);
    w.int_def(k!(b"BCLi"), a.stroke_color.0, 0xff00_0000, true);
    w.int_def(k!(b"FCLi"), a.fill_color.0, 0xff00_00ff, true);
    if let Some(b) = &a.brush {
        w.list(k!(b"BPLv"), write_brush(b));
    }
    if let Some(f) = &a.fill {
        w.list(k!(b"FPLv"), write_fill(f, ctx));
    }
    if let Some(t) = &a.stroke_texture {
        w.list(k!(b"TXBv"), write_texture(t, ctx));
    }
    if let Some(t) = &a.fill_texture {
        w.list(k!(b"TXFv"), write_texture(t, ctx));
    }
    w.finish()
}

fn write_texture(t: &Texture, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&t.extra);
    w.str_def(k!(b"INMs"), &t.name, "", true);
    w.opt_str(k!(b"UNMs"), t.user_name.as_deref());
    if let Some(m) = &t.mask {
        w.list(k!(b"MSKv"), write_mask(m, ctx));
    }
    w.finish()
}

/// Firepixel-own marker in a fill `FPLv`: `fpEi` & 3 = 3 marks the dither edge (FB-23), which
/// the file otherwise stores as a feather (`FETi` 2, `FEFi` = width; `FETi` 0 at width 0); bit 4
/// ([`Fill::EXT_LOCK_TO_DOCUMENT`]) marks a pattern or gradient locked to the document (FB-44).
/// Without either the tag is not written. Like
/// [`SPRITE_EXT`] the tag starts with a lower-case letter and cannot clash with a tag of the
/// reference format; readers that do not know it keep / drop it and show the feather.
pub const FILL_EDGE_EXT: TagKey = k!(b"fpEi");

/// Füllung `FPLv` (Kinder).
pub fn write_fill(f: &Fill, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&f.extra);
    w.str_def(k!(b"CATs"), &f.category, "", true);
    w.str_def(k!(b"INMs"), &f.name, "", true);
    w.opt_str(k!(b"UNMs"), f.user_name.as_deref());
    w.bool_def(k!(b"RDOb"), f.read_only, false, true);
    w.i32_def(k!(b"FTBi"), f.texture_amount, 0, true);
    w.i32_def(k!(b"FEFi"), f.feather, 0, true);
    w.i32_def(k!(b"FSMi"), f.stamping_mode, 1, true);
    // dither edge (Firepixel extension): a feather (hard at width 0) for every other reader plus
    // the own marker `fpEi` (see `FILL_EDGE_EXT`)
    let dither = f.edge == Fill::EDGE_DITHER;
    let feti = match f.edge {
        Fill::EDGE_DITHER if f.feather > 0 => Fill::EDGE_FEATHER,
        Fill::EDGE_DITHER => Fill::EDGE_HARD,
        e => e,
    };
    w.i32_def(k!(b"FETi"), feti, 1, true);
    w.bool_def(k!(b"FRDb"), f.dither, false, true);
    w.i32_def(k!(b"FRRi"), f.dither_amount, 0, true);
    w.i32_def(k!(b"FSHi"), f.shape, 0, true);
    w.i32_def(k!(b"FSXi"), f.shape_ext, 0, true);
    if let Some(d) = &f.dither_colors {
        w.int(k!(b"FD1i"), d[0]);
        w.int(k!(b"FD2i"), d[1]);
        w.int(k!(b"FD3i"), d[2]);
        w.int(k!(b"FD4i"), d[3]);
    }
    w.opt_bool(k!(b"FDTb"), f.transparent_dither);
    // Firepixel-own bits: 3 = dither edge, 4 = lock pattern to document (FB-44); absent at default
    let ext = if dither { Fill::EDGE_DITHER } else { 0 }
        | if f.lock_to_document {
            Fill::EXT_LOCK_TO_DOCUMENT
        } else {
            0
        };
    if ext != 0 {
        w.i32(FILL_EDGE_EXT, ext);
    }
    if let Some(g) = &f.gradient {
        w.list(k!(b"FGLv"), write_gradient(g));
    }
    if let Some(p) = &f.pattern {
        let mut s = Wr::new(&p.extra);
        s.str_def(k!(b"INMs"), &p.name, "", true);
        s.opt_str(k!(b"UNMs"), p.user_name.as_deref());
        if let Some(i) = &p.image {
            s.list(
                k!(b"IMGv"),
                write_bitmap_node(&i.common, &i.bitmap, &i.extra, ctx),
            );
        }
        w.list(k!(b"FPTv"), s.finish());
    }
    w.finish()
}

/// Verlauf `FGLv` (Kinder).
pub fn write_gradient(g: &Gradient) -> Vec<RawNode> {
    let mut w = Wr::new(&g.extra);
    w.str_def(k!(b"CATs"), &g.category, "", true);
    w.str_def(k!(b"INMs"), &g.name, "", true);
    w.opt_str(k!(b"UNMs"), g.user_name.as_deref());
    if let Some(s) = &g.legacy_stops {
        w.list(k!(b"FGVv"), write_stops(s));
    }
    if let Some(s) = &g.stops {
        let mut y = Wr::new(&s.extra);
        y.list(k!(b"FG0v"), write_stops(&s.colors));
        y.list(k!(b"FG1v"), write_stops(&s.opacities));
        y.opt_bool(k!(b"FGDb"), s.dither);
        y.opt_i32(k!(b"FGDi"), s.dither_value);
        w.list(k!(b"FGYv"), y.finish());
    }
    w.finish()
}

fn write_stops(s: &StopList) -> Vec<RawNode> {
    let mut w = Wr::new(&s.extra);
    w.int(k!(b"FNCi"), s.stops.len() as u32);
    for st in &s.stops {
        let mut q = Wr::new(&st.extra);
        q.f32_def(k!(b"FGPf"), st.position, 0.0, true);
        q.int_def(k!(b"FGCi"), st.color.0, 0xff00_0000, true);
        w.list(k!(b"FGIv"), q.finish());
    }
    w.finish()
}

/// Pinsel `BPLv` (Kinder).
pub fn write_brush(b: &Brush) -> Vec<RawNode> {
    let mut w = Wr::new(&b.extra);
    w.str_def(k!(b"CATs"), &b.category, "", true);
    w.str_def(k!(b"INMs"), &b.name, "", true);
    w.opt_str(k!(b"UNMs"), b.user_name.as_deref());
    w.bool_def(k!(b"RDOb"), b.read_only, false, true);
    let vals: [(TagKey, i32); 26] = [
        (k!(b"BANi"), b.angle),
        (k!(b"BASi"), b.aspect),
        (k!(b"BDIi"), b.diameter),
        (k!(b"BMMi"), b.max_count),
        (k!(b"BMSi"), b.min_size),
        (k!(b"BSEi"), b.softness),
        (k!(b"BSFi"), b.soften_mode),
        (k!(b"BSHi"), b.shape),
        (k!(b"BBLi"), b.bbl),
        (k!(b"BBKi"), b.blackness),
        (k!(b"BCNi"), b.concentration),
        (k!(b"BEFi"), b.alpha_remap),
        (k!(b"BRTi"), b.brush_type),
        (k!(b"BFBi"), b.feedback),
        (k!(b"BFRi"), b.flow_rate),
        (k!(b"BNTi"), b.tip_count),
        (k!(b"BSPi"), b.spacing),
        (k!(b"BTBi"), b.texture_blend),
        (k!(b"BTEi"), b.texture_edge),
        (k!(b"BTSi"), b.tip_spacing),
        (k!(b"BSMi"), b.tip_spacing_mode),
        (k!(b"BCMi"), b.tip_coloring_mode),
        (k!(b"NDIi"), b.dash_count),
        (k!(b"DO1i"), b.dash_on[0]),
        (k!(b"DO2i"), b.dash_on[1]),
        (k!(b"DO3i"), b.dash_on[2]),
    ];
    for (key, v) in &vals[..22] {
        w.i32_def(*key, *v, 0, true);
    }
    w.bool_def(k!(b"BIAb"), b.antialiased, false, true);
    for (key, v) in &vals[22..] {
        w.i32_def(*key, *v, 0, true);
    }
    w.i32_def(k!(b"DF1i"), b.dash_off[0], 0, true);
    w.i32_def(k!(b"DF2i"), b.dash_off[1], 0, true);
    w.i32_def(k!(b"DF3i"), b.dash_off[2], 0, true);
    for s in &b.sensitivities {
        w.i32(TagKey([b'S', s.source, s.target, b'i']), s.value);
    }
    w.finish()
}

/// Effektliste `EFLv` (Kinder).
pub fn write_effect_list(e: &EffectList) -> Vec<RawNode> {
    let mut w = Wr::new(&e.extra);
    w.str_def(k!(b"CATs"), &e.category, "", true);
    w.str_def(k!(b"INMs"), &e.name, "", true);
    w.opt_str(k!(b"UNMs"), e.user_name.as_deref());
    for x in &e.effects {
        let key = match x.tag {
            EffectTag::Eps => k!(b"EPSv"),
            EffectTag::Epd => k!(b"EPDv"),
        };
        w.list(key, write_dict(&x.params));
    }
    w.finish()
}

/// Wörterbuch (`DTAv`, `EPSv`, `GDTv`; Kinder).
pub fn write_dict(d: &DataDict) -> Vec<RawNode> {
    let kids: Vec<RawNode> = d.entries.iter().map(write_entry).collect();
    crate::schema::arrange(kids, &d.extra)
}

fn write_entry(e: &DataEntry) -> RawNode {
    let mut w = Wr::new(&e.extra);
    w.str(k!(b"DCKs"), &e.key);
    match &e.value {
        DataValue::Points(p) => w.list(k!(b"GPLv"), write_points(p)),
        DataValue::List(l) => w.list(k!(b"GVRv"), write_data_list(l)),
        DataValue::Dict(d) => w.list(k!(b"GDTv"), write_dict(d)),
        DataValue::Scalar(s) => w.str(k!(b"DCVs"), s),
        DataValue::None => {}
    }
    if !matches!(e.value, DataValue::Scalar(_)) {
        w.opt_str(k!(b"DCVs"), e.text.as_deref());
    }
    let key = match e.tag {
        EntryTag::Dce => k!(b"DCEv"),
        EntryTag::Dcf => k!(b"DCFv"),
    };
    RawNode::list(key, w.finish())
}

fn write_data_list(l: &DataList) -> Vec<RawNode> {
    let kids: Vec<RawNode> = l
        .items
        .iter()
        .map(|v| match v {
            DataValue::Scalar(s) => RawNode::str(k!(b"VVLs"), s.as_str()),
            DataValue::Dict(d) => RawNode::list(k!(b"GDTv"), write_dict(d)),
            DataValue::List(x) => RawNode::list(k!(b"GVRv"), write_data_list(x)),
            DataValue::Points(p) => RawNode::list(k!(b"GPLv"), write_points(p)),
            DataValue::None => RawNode::str(k!(b"VVLs"), ""),
        })
        .collect();
    crate::schema::arrange(kids, &l.extra)
}

fn write_points(p: &PointList) -> Vec<RawNode> {
    let empty = Extra::default();
    let kids: Vec<RawNode> = p
        .points
        .iter()
        .enumerate()
        .map(|(i, pt)| {
            let mut q = Wr::new(p.point_extra.get(i).unwrap_or(&empty));
            q.f32_def(k!(b"XLCf"), pt.x, 0.0, true);
            q.f32_def(k!(b"YLCf"), pt.y, 0.0, true);
            RawNode::list(k!(b"GPTv"), q.finish())
        })
        .collect();
    crate::schema::arrange(kids, &p.extra)
}

fn write_text_attrs_run(w: &mut Wr, a: &TextAttrs) {
    w.opt_str(k!(b"FONs"), a.font.as_deref());
    w.opt_str(k!(b"CFTs"), a.ps_font.as_deref());
    w.opt_f32(k!(b"PTSf"), a.size);
    w.opt_bool(k!(b"BOLb"), a.bold);
    w.opt_bool(k!(b"ITLb"), a.italic);
    w.opt_bool(k!(b"UNDb"), a.underline);
    w.opt_f32(k!(b"HSCf"), a.horizontal_scale);
    w.opt_f32(k!(b"KRNf"), a.kerning);
    w.opt_f32(k!(b"RKNf"), a.tracking);
    w.opt_f32(k!(b"BLSf"), a.baseline_shift);
    w.opt_f32(k!(b"LEDf"), a.leading);
    w.opt_i32(k!(b"LDMi"), a.leading_mode);
    w.opt_i32(k!(b"JSTi"), a.alignment);
    w.opt_int(k!(b"FCLi"), a.color);
    w.opt_f32(k!(b"PINf"), a.indent);
    w.opt_f32(k!(b"PSBf"), a.space_before);
    w.opt_f32(k!(b"PSAf"), a.space_after);
}

fn write_text(e: &Element, t: &TextElem, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let c = &e.common;
    let mut w = Wr::new(&e.extra);
    w.rect4_def(LTRB, t.bounds, Rect::new(0.0, 0.0, 250.0, 250.0), true);
    if let Some(r) = t.anchor_rect {
        w.rect4([k!(b"LFAf"), k!(b"TOAf"), k!(b"RIAf"), k!(b"BOAf")], r);
    }
    w.opt_i32(k!(b"TFTi"), t.frame_type);
    w.opt_int(k!(b"AIDi"), t.story_index);
    w.opt_bool(k!(b"FOAb"), t.from_story);
    w_vis(&mut w, c);
    w_lck(&mut w, c);
    w.list(k!(b"PATv"), write_attrs(&t.attrs, ctx));
    w.i32_def(k!(b"BRPi"), t.stroke_placement, 1, true);
    w.i32_def(k!(b"PTTi"), t.ptt, 0, true);
    w.bool_def(k!(b"FOTb"), t.fill_on_top, false, true);
    w_efl(&mut w, c);
    w.int_def(k!(b"RNDi"), t.random_seed, 0, true);
    w.point_def(
        k!(b"TOXf"),
        k!(b"TOYf"),
        t.texture_offset,
        Point::default(),
        true,
    );
    w.opacity(k!(b"OPAi"), c.opacity);
    w.blend(k!(b"BLDi"), c.blend);
    w.i32_def(k!(b"ORIi"), t.orientation, 0, true);
    write_fill_handles(&mut w, &t.fill_handles, true);
    if let Some(m) = &t.transform {
        w.matrix(k!(b"MTXv"), m);
    }
    w.i32_def(k!(b"XFMi"), t.transform_mode, 1, true);
    w.i32_def(k!(b"TAAi"), t.antialias, 0, true);
    w.i32_def(k!(b"TA1i"), t.aa_sharpness, 192, true);
    w.i32_def(k!(b"TA2i"), t.aa_strength, 64, true);
    w.i32_def(k!(b"TOSi"), t.aa_oversample, 8, true);
    w.opt_bool(k!(b"ATEb"), t.ate);
    w.bool_def(k!(b"ATKb"), t.auto_kern, true, true);
    let a = &t.initial;
    w.opt_str(k!(b"FONs"), a.font.as_deref());
    w.opt_str(k!(b"CFTs"), a.ps_font.as_deref());
    w.opt_f32(k!(b"PTSf"), a.size);
    w.opt_bool(k!(b"BOLb"), a.bold);
    w.opt_bool(k!(b"ITLb"), a.italic);
    w.opt_bool(k!(b"UNDb"), a.underline);
    w.opt_f32(k!(b"HSCf"), a.horizontal_scale);
    w.opt_f32(k!(b"KRNf"), a.kerning);
    w.opt_f32(k!(b"RKNf"), a.tracking);
    w.opt_f32(k!(b"BLSf"), a.baseline_shift);
    w.opt_f32(k!(b"LEDf"), a.leading);
    w.opt_i32(k!(b"LDMi"), a.leading_mode);
    w.opt_int(k!(b"FCLi"), a.color);
    if let Some(p) = &t.on_path {
        let mut s = Wr::new(&p.extra);
        s.opt_i32(k!(b"TAMi"), p.mode);
        s.opt_f32(k!(b"CHOf"), p.offset);
        if let Some(e) = &p.path {
            let n = write_element(e, ctx);
            s.list(k!(b"PTHv"), n.children().to_vec());
        }
        w.list(k!(b"TAPv"), s.finish());
    }
    w.opt_i32(k!(b"JSTi"), a.alignment);
    w.opt_f32(k!(b"PINf"), a.indent);
    w.opt_f32(k!(b"PSBf"), a.space_before);
    w.opt_f32(k!(b"PSAf"), a.space_after);
    if t.has_runs_node || w.is_new() {
        let mut kids = Vec::new();
        for r in &t.runs {
            let mut b = Wr::new(&r.extra);
            write_text_attrs_run(&mut b, &r.attrs);
            kids.extend(b.finish());
            kids.push(RawNode::str(k!(b"TRNs"), r.text.as_str()));
        }
        if let Some(r) = &t.trailing_attrs {
            let mut b = Wr::new(&r.extra);
            write_text_attrs_run(&mut b, &r.attrs);
            kids.extend(b.finish());
        }
        kids.extend(t.runs_extra.nodes.iter().cloned());
        w.list(k!(b"TFSv"), kids);
    }
    w_names(&mut w, c);
    w_dta(&mut w, c);
    w_cache(&mut w, e, ctx);
    w.finish()
}

fn write_smart_shape(s: &SmartShape) -> Vec<RawNode> {
    let mut w = Wr::new(&s.extra);
    w.str_def(k!(b"FILs"), &s.name, "", true);
    let chunks: Vec<RawNode> = s
        .script_chunks
        .iter()
        .map(|c| RawNode::str(k!(b"JSSs"), c.as_str()))
        .collect();
    w.list(k!(b"JSVv"), crate::schema::arrange(chunks, &s.script_extra));
    if let Some(m) = &s.transform {
        w.matrix(k!(b"MTXv"), m);
    }
    if let Some(cp) = &s.control_points {
        let kids: Vec<RawNode> = cp
            .iter()
            .map(|p| {
                let mut q = Wr::new(&p.extra);
                q.f32_def(k!(b"XLCf"), p.position.x, 0.0, true);
                q.f32_def(k!(b"YLCf"), p.position.y, 0.0, true);
                q.i32_def(k!(b"CTYi"), p.kind, 0, true);
                q.opt_str(k!(b"OBNs"), p.name.as_deref());
                q.opt_str(k!(b"TTPs"), p.tooltip.as_deref());
                q.bool_def(k!(b"TTDb"), p.tooltip_tracks_drag, false, true);
                q.bool_def(k!(b"VISb"), p.visible, true, true);
                RawNode::list(k!(b"CPTv"), q.finish())
            })
            .collect();
        w.list(
            k!(b"CPSv"),
            crate::schema::arrange(kids, &s.control_points_extra),
        );
    }
    if let Some(ev) = &s.events {
        let kids: Vec<RawNode> = ev
            .iter()
            .map(|e| RawNode::str(k!(b"EVTs"), e.as_str()))
            .collect();
        w.list(k!(b"EVVv"), crate::schema::arrange(kids, &s.events_extra));
    }
    w.finish()
}

fn write_behaviors(b: &Behaviors, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let kids: Vec<RawNode> = b
        .items
        .iter()
        .map(|x| RawNode::list(k!(b"BEHv"), write_behavior(x, ctx)))
        .collect();
    crate::schema::arrange(kids, &b.extra)
}

fn write_behavior(b: &Behavior, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&b.extra);
    w.i32(k!(b"BETi"), b.action);
    w.i32(k!(b"BEEi"), b.event);
    w.opt_i32(k!(b"ROLi"), b.flags);
    w.opt_bool(k!(b"CHKb"), b.checked);
    w.opt_str(k!(b"UNMs"), b.status_text.as_deref());
    if let Some(p) = &b.popup {
        w.opt_str(k!(b"CDTs"), p.creation_date.as_deref());
        w.opt_f32(k!(b"XLCf"), p.horz_offset);
        w.opt_f32(k!(b"YLCf"), p.vert_offset);
        w.opt_str(k!(b"TRNs"), p.items_text.as_deref());
        w.opt_int(k!(b"FCLi"), p.text_up_color);
        w.opt_int(k!(b"FETi"), p.text_over_color);
        w.opt_i32(k!(b"JSTi"), p.text_alignment);
        w.opt_i32(k!(b"VERi"), p.vert_alignment);
        w.opt_i32(k!(b"TMOi"), p.hide_timeout);
        w.opt_f32(k!(b"XSLf"), p.horz_submenu_offset);
        w.opt_f32(k!(b"YSLf"), p.vert_submenu_offset);
        w.opt_bool(k!(b"RELb"), p.submenu_relative);
        w.opt_i32(k!(b"CSPi"), p.item_spacing);
        w.opt_i32(k!(b"CPDi"), p.item_padding);
        w.opt_bool(k!(b"OPQb"), p.opaque);
        w.opt_int(k!(b"HICi"), p.hilite_color);
        w.opt_int(k!(b"CSHi"), p.shadow_color);
        w.opt_int(k!(b"BOCi"), p.border_color);
        w.opt_i32(k!(b"BOSi"), p.border_size);
        w.opt_bool(k!(b"OVWb"), p.override_width);
        w.opt_bool(k!(b"OVHb"), p.override_height);
        w.opt_i32(k!(b"MITi"), p.text_indent);
        w.opt_bool(k!(b"MIVb"), p.vertical);
        w.opt_i32(k!(b"MIDi"), p.menu_id);
        w.opt_i32(k!(b"MIWi"), p.menu_width);
        w.opt_i32(k!(b"MIHi"), p.menu_height);
        w.opt_bool(k!(b"HIMb"), p.hide_on_mouseout);
        w.opt_bool(k!(b"BOLb"), p.bold);
        w.opt_bool(k!(b"ITLb"), p.italic);
        w.opt_str(k!(b"FONs"), p.font.as_deref());
        w.opt_f32(k!(b"PTSf"), p.size);
        w.opt_bool(k!(b"ATEb"), p.text_only);
        w.opt_int(k!(b"BCLi"), p.cell_up_color);
        w.opt_int(k!(b"BCMi"), p.cell_over_color);
        for s in &p.styles {
            w.list(k!(b"ZSTv"), write_style(s, ctx));
        }
    }
    w.opt_int(k!(b"MSNi"), b.target_slice);
    w.opt_i32(k!(b"FRCi"), b.frame);
    w.opt_bool(k!(b"RMOb"), b.restore);
    w.opt_bool(k!(b"PREb"), b.preload);
    w.opt_i32(k!(b"FRTi"), b.down_frame);
    w.opt_str(k!(b"FILs"), b.file.as_deref());
    w.opt_str(k!(b"INMs"), b.down_file.as_deref());
    w.opt_bool(k!(b"ENDb"), b.end);
    w.finish()
}

/// `PALv`-Kinder einer Palette (gleiche Form beim Lesen geprüft).
pub fn encode_palette(p: &[PaletteEntry]) -> Vec<RawNode> {
    let mut v = Vec::with_capacity(p.len());
    for e in p {
        v.push(RawNode::int(k!(b"FCLi"), e.color));
        if let Some(b) = e.locked {
            v.push(RawNode::bool(k!(b"CLLb"), b));
        }
        if let Some(b) = e.transparent {
            v.push(RawNode::bool(k!(b"CLTb"), b));
        }
        if let Some(b) = e.modified {
            v.push(RawNode::bool(k!(b"CLMb"), b));
        }
        if let Some(c) = e.new_color {
            v.push(RawNode::int(k!(b"BCLi"), c));
        }
    }
    v
}

/// Exporteinstellungen `EXPv` (Kinder).
pub fn write_export(e: &ExportSettings) -> Vec<RawNode> {
    let mut w = Wr::new(&e.extra);
    let d = ExportSettings::options_only();
    w.i32_def(k!(b"FMTi"), e.format, d.format, true);
    w.opt_int(k!(b"MCTi"), e.mac_type);
    w.opt_int(k!(b"MCCi"), e.mac_creator);
    w.i32_def(k!(b"CLRi"), e.color_mode, d.color_mode, true);
    w.i32_def(k!(b"PLTi"), e.palette_mode, d.palette_mode, true);
    w.list(k!(b"PALv"), encode_palette(&e.palette));
    w.i32_def(k!(b"PLNi"), e.palette_count, 0, true);
    w.i32_def(k!(b"PLRi"), e.entries_requested, d.entries_requested, true);
    w.i32_def(k!(b"PTNi"), e.transparency, 0, true);
    w.bool_def(k!(b"PLOb"), e.optimized, d.optimized, true);
    w.bool_def(k!(b"APLb"), e.local_adaptive, d.local_adaptive, true);
    w.bool_def(k!(b"APSb"), e.websnap_adaptive, d.websnap_adaptive, true);
    w.bool_def(k!(b"AACb"), e.anim_auto_crop, d.anim_auto_crop, true);
    w.bool_def(
        k!(b"AADb"),
        e.anim_auto_difference,
        d.anim_auto_difference,
        true,
    );
    w.i32_def(k!(b"ASTi"), e.websnap_tolerance, d.websnap_tolerance, true);
    w.i32_def(k!(b"PLDi"), e.dither, 0, true);
    w.i32_def(k!(b"GLTi"), e.lossy, 0, true);
    w.i32_def(k!(b"JPQi"), e.jpeg_quality, d.jpeg_quality, true);
    w.i32_def(k!(b"JPSi"), e.jpeg_smoothness, 0, true);
    w.i32_def(k!(b"JPOi"), e.jpeg_subsampling, 0, true);
    w.bool_def(k!(b"JPPb"), e.jpeg_progressive, false, true);
    w.bool_def(k!(b"GFIb"), e.interlaced, false, true);
    w.i32_def(k!(b"JSQi"), e.jpeg_sel_quality, d.jpeg_sel_quality, true);
    w.bool_def(k!(b"JSEb"), e.jpeg_sel_enabled, false, true);
    w.bool_def(k!(b"JSTb"), e.jpeg_sel_preserve_text, true, true);
    w.bool_def(k!(b"JSBb"), e.jpeg_sel_preserve_buttons, false, true);
    w.i32_def(k!(b"FLPi"), e.loop_count, 0, true);
    let frames: Vec<RawNode> = e
        .frames
        .iter()
        .map(|f| {
            let mut q = Wr::new(&f.extra);
            q.i32_def(k!(b"FNPi"), f.disposal, 0, true);
            q.i32_def(k!(b"FNDi"), f.delay, 7, true);
            q.bool_def(k!(b"VISb"), f.visible, true, true);
            q.opt_str(k!(b"OBNs"), f.name.as_deref());
            RawNode::list(k!(b"FNMv"), q.finish())
        })
        .collect();
    w.list(k!(b"FNLv"), crate::schema::arrange(frames, &e.frames_extra));
    if let Some(c) = &e.crop {
        w.i32(k!(b"LRLi"), c[0]);
        w.i32(k!(b"LRTi"), c[1]);
        w.i32(k!(b"LRRi"), c[2]);
        w.i32(k!(b"LRBi"), c[3]);
    }
    if let Some(s) = &e.scale {
        w.bool(k!(b"CLSb"), s.use_scale);
        w.bool(k!(b"CNSb"), s.constrain);
        w.i32(k!(b"CLPi"), s.percent);
        w.i32(k!(b"XSZi"), s.width);
        w.i32(k!(b"YSZi"), s.height);
    }
    if let Some(h) = &e.html {
        w.opt_bool(k!(b"GSLb"), h.slice_along_guides);
        w.opt_bool(k!(b"LSUb"), h.slice_using_urls);
        w.opt_bool(k!(b"DSUb"), h.discard_unspecified_slices);
        w.opt_bool(k!(b"LSDb"), h.generate_demo_html);
        w.opt_bool(k!(b"CSMb"), h.css_popups);
        w.opt_i32(k!(b"ECSi"), h.external_css);
        w.opt_bool(k!(b"NYSb"), h.not_yet_set);
        w.opt_i32(k!(b"LSSi"), h.shim_generation);
        w.opt_i32(k!(b"HTMi"), h.html_destination);
        w.opt_str(k!(b"HTTs"), h.template_name.as_deref());
        w.opt_i32(k!(b"XFSi"), h.export_files_style);
        w.opt_str(k!(b"IMDs"), h.images_dir.as_deref());
        w.opt_str(k!(b"LXDs"), h.last_export_dir.as_deref());
        w.opt_i32(k!(b"XMDi"), h.multiple_files_mode);
        w.opt_bool(k!(b"TRMb"), h.trm);
        w.opt_bool(k!(b"HTMb"), h.htm);
        w.opt_bool(k!(b"XDAb"), h.xda);
        w.opt_str(k!(b"EFXs"), h.file_extension.as_deref());
        w.opt_str(k!(b"EFBs"), h.base_name.as_deref());
        if let Some(v) = &h.slice_naming {
            let kids: Vec<RawNode> = v
                .iter()
                .map(|x| RawNode::int(k!(b"NM4i"), *x as u32))
                .collect();
            w.list(
                k!(b"NM4v"),
                crate::schema::arrange(kids, &h.slice_naming_extra),
            );
        }
        if let Some(v) = &h.frame_naming {
            let kids: Vec<RawNode> = v
                .iter()
                .map(|x| RawNode::int(k!(b"NMFi"), *x as u32))
                .collect();
            w.list(
                k!(b"NMFv"),
                crate::schema::arrange(kids, &h.frame_naming_extra),
            );
        }
        if let Some(ns) = &h.state_names {
            w.str(k!(b"NS0s"), &ns[0]);
            w.str(k!(b"NS1s"), &ns[1]);
            w.str(k!(b"NS2s"), &ns[2]);
            w.str(k!(b"NS3s"), &ns[3]);
        }
        w.opt_i32(k!(b"ENCi"), h.encoding);
        w.opt_i32(k!(b"TALi"), h.table_alignment);
        w.opt_i32(k!(b"TPWi"), h.table_pct_width);
        w.opt_i32(k!(b"XTMi"), h.xhtml);
        w.opt_str(k!(b"CBUs"), h.background_url.as_deref());
        w.opt_i32(k!(b"CBRi"), h.background_repeat);
        w.opt_i32(k!(b"CBAi"), h.background_attachment);
        w.opt_i32(k!(b"CDAi"), h.page_alignment);
        if let Some(m) = &h.margins {
            let mut q = Wr::new(&h.margins_extra);
            for (i, key) in LTRB.iter().enumerate() {
                q.f32_def(*key, m[i], 0.0, true);
            }
            w.list(k!(b"CDMv"), q.finish());
        }
        w.opt_bool(k!(b"CXBb"), h.external_css_for_doc);
        w.opt_str(k!(b"CXFs"), h.external_css_filename.as_deref());
        w.opt_str(k!(b"CBDs"), h.css_dir.as_deref());
        w.opt_int(k!(b"FCLi"), h.matte_color);
        if let Some(kids) = write_sprite_ext(h) {
            w.list(SPRITE_EXT, kids);
        }
    }
    w.finish()
}

/// Firepixel-eigene Erweiterung im `EXPv`: Exportoptionen für CSS-Sprites und der
/// „Standard-ALT-Tag“ der HTML-Einrichtung (`DALs`, FIX-E). Das Original hält diese
/// Felder im Dokument, ihre Tags sind aber nicht bekannt (`re\08` §1.5 „Tags ?“, nicht im
/// Vokabular). Der Container beginnt mit Kleinbuchstaben und kann daher mit keinem Tag des
/// Originals (immer drei Großbuchstaben/Ziffern) zusammenfallen.
pub const SPRITE_EXT: TagKey = k!(b"fpSv");

fn write_sprite_ext(h: &HtmlSettings) -> Option<Vec<RawNode>> {
    if h.sprite_selector_is_class.is_none()
        && h.sprite_css_prefix.is_none()
        && h.sprite_css_suffix.is_none()
        && h.sprite_padding.is_none()
        && h.sprite_orientation.is_none()
        && h.sprite_preset.is_none()
        && h.default_alt.is_none()
    {
        return None;
    }
    let empty = Extra::default();
    let mut w = Wr::new(&empty);
    w.opt_bool(k!(b"SSCb"), h.sprite_selector_is_class);
    w.opt_str(k!(b"SPXs"), h.sprite_css_prefix.as_deref());
    w.opt_str(k!(b"SSXs"), h.sprite_css_suffix.as_deref());
    w.opt_i32(k!(b"SPDi"), h.sprite_padding);
    w.opt_i32(k!(b"SORi"), h.sprite_orientation);
    w.opt_str(k!(b"SPRs"), h.sprite_preset.as_deref());
    w.opt_str(k!(b"DALs"), h.default_alt.as_deref());
    Some(w.finish())
}

/// Symbol `MSTv` (Kinder).
pub fn write_symbol(s: &Symbol, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&s.extra);
    w.str_def(k!(b"UNMs"), &s.name, "", true);
    w.int_def(k!(b"MSNi"), s.id.0, 0, true);
    w.opt_i32(k!(b"MSYi"), s.kind_code);
    w.f32_def(k!(b"MOXf"), s.origin_x, 0.0, true);
    w.f32_def(k!(b"MOYf"), s.origin_y, 0.0, true);
    w.opt_str(k!(b"LFNs"), s.library_file.as_deref());
    w.opt_int(k!(b"LMNi"), s.library_symbol);
    w.opt_str(k!(b"LUTs"), s.modified.as_deref());
    if s.root.has_layer_list || w.is_new() {
        ctx.frames = s.root.frame_count;
        w.list(
            k!(b"LYLv"),
            write_layer_list(&s.root.layers, &s.root.layers_extra, ctx),
        );
    }
    if let Some(g) = &s.root.grid {
        w.list(k!(b"GRDv"), write_grid(g));
    }
    if let Some(g) = &s.root.guides {
        w.list(k!(b"GDSv"), write_guides(g));
    }
    if let Some(n) = &s.nine_slice {
        let mut q = Wr::new(&n.extra);
        q.opt_bool(k!(b"NGLb"), n.locked);
        q.opt_int(k!(b"NGKi"), n.color.map(|c| c.0));
        for g in &n.guides {
            q.f64(k!(b"NGCf"), *g);
        }
        if let Some(b) = n.bounds {
            let mut r = Wr::new(&n.bounds_extra);
            r.rect4(LTRB, b);
            q.list(k!(b"NGMv"), r.finish());
        }
        if let Some(b) = n.view {
            let mut r = Wr::new(&n.view_extra);
            r.rect4(LTRB, b);
            q.list(k!(b"NGVv"), r.finish());
        }
        w.list(k!(b"NGSv"), q.finish());
    }
    w.opt_str(k!(b"WJSs"), s.widget_script.as_deref());
    let n = s.root.frame_count_repeat.unwrap_or(1);
    write_frc(&mut w, &s.root, n);
    w.i32_def(k!(b"OSBi"), s.root.onion_before, 0, true);
    w.i32_def(k!(b"OSAi"), s.root.onion_after, 0, true);
    if let Some(e) = &s.root.export {
        w.list(k!(b"EXPv"), write_export(e));
    }
    w.int_def(k!(b"BGCi"), s.root.background.0, 0xffff_ffff, true);
    w.opt_str(k!(b"INMs"), s.button.url.as_deref());
    w.opt_str(k!(b"A2Ts"), s.button.alt.as_deref());
    w.opt_str(k!(b"ALTs"), s.button.target.as_deref());
    w.opt_str(k!(b"FILs"), s.button.file_name.as_deref());
    w.opt_str(k!(b"BUTs"), s.button.text.as_deref());
    w.opt_str(k!(b"MSGs"), s.status_message.as_deref());
    if let Some(b) = &s.behaviors {
        w.list(k!(b"BEVv"), write_behaviors(b, ctx));
    }
    let o = &s.button_options;
    w.bool_def(k!(b"ATSb"), o.auto_slice, true, true);
    w.bool_def(k!(b"IDSb"), o.include_down, false, true);
    w.bool_def(k!(b"DVLb"), o.show_down_on_load, false, true);
    w.bool_def(k!(b"IODb"), o.include_over_down, false, true);
    w.finish()
}

/// Stil `ZSTv` (Kinder).
pub fn write_style(s: &Style, ctx: &mut WriteCtx) -> Vec<RawNode> {
    let mut w = Wr::new(&s.extra);
    w.str_def(k!(b"ZSNs"), &s.name, "", true);
    w.int_def(k!(b"ZSFi"), s.flags, 0, true);
    w.int_def(k!(b"ZSOi"), s.other_flags, 0, true);
    if let Some(a) = &s.attrs {
        w.list(k!(b"PATv"), write_attrs(a, ctx));
    }
    if let Some(h) = &s.fill_handles {
        write_fill_handles(&mut w, h, false);
    }
    w.opt_i32(k!(b"BRPi"), s.stroke_placement);
    w.opt_bool(k!(b"FOTb"), s.fill_on_top);
    if let Some(r) = &s.preview_rect {
        w.i32(k!(b"FALi"), r[0]);
        w.i32(k!(b"FATi"), r[1]);
        w.i32(k!(b"FARi"), r[2]);
        w.i32(k!(b"FABi"), r[3]);
    }
    if let Some(e) = &s.effects {
        w.list(k!(b"EFLv"), write_effect_list(e));
    }
    let a = &s.text;
    w.opt_str(k!(b"FONs"), a.font.as_deref());
    w.opt_str(k!(b"CFTs"), a.ps_font.as_deref());
    w.opt_f32(k!(b"PTSf"), a.size);
    w.opt_bool(k!(b"BOLb"), a.bold);
    w.opt_bool(k!(b"ITLb"), a.italic);
    w.opt_bool(k!(b"UNDb"), a.underline);
    w.opt_i32(k!(b"JSTi"), a.alignment);
    w.opt_f32(k!(b"HSCf"), a.horizontal_scale);
    w.opt_f32(k!(b"KRNf"), a.kerning);
    w.opt_f32(k!(b"RKNf"), a.tracking);
    w.opt_f32(k!(b"BLSf"), a.baseline_shift);
    w.opt_f32(k!(b"LEDf"), a.leading);
    w.opt_i32(k!(b"LDMi"), a.leading_mode);
    w.opt_int(k!(b"FCLi"), a.color);
    w.opt_f32(k!(b"PINf"), a.indent);
    w.opt_f32(k!(b"PSBf"), a.space_before);
    w.opt_f32(k!(b"PSAf"), a.space_after);
    w.opt_bool(k!(b"ATKb"), s.auto_kern);
    w.opt_bool(k!(b"TIAb"), s.tia);
    w.opt_i32(k!(b"TAAi"), s.antialias);
    w.opt_i32(k!(b"TA1i"), s.aa_sharpness);
    w.opt_i32(k!(b"TA2i"), s.aa_strength);
    w.opt_i32(k!(b"TOSi"), s.aa_oversample);
    for p in &s.previews {
        w.list(
            k!(b"IMGv"),
            write_bitmap_node(&p.common, &p.bitmap, &p.extra, ctx),
        );
    }
    w.finish()
}
